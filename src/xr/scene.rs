use std::ptr::{null, null_mut};

use super::{SceneState, SpatialAnchors};
use crate::{oxr, PhysLayer};
use bevy::{
    prelude::*,
    render::{mesh, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    input::XrInput,
    resources::{XrFrameState, XrInstance, XrSession},
    xr::{
        sys::{
            self, RoomLayoutFB, Session, Space, SpaceComponentFilterInfoFB, SpaceComponentStatusFB,
            SpaceComponentStatusSetInfoFB, SpaceLocation, SpaceQueryInfoFB, SpaceQueryResultFB,
            SpaceQueryResultsFB, UUID_SIZE_EXT,
        },
        Duration, Event, InstanceExtensions, Offset2Df, Posef, Rect2Df, SpaceComponentTypeFB,
        SpaceLocationFlags, SpaceQueryActionFB, SpaceQueryResultsAvailableFB, StructureType,
        UuidEXT, Vector3f,
    },
    XrEvents,
};
use bevy_xpbd_3d::prelude::*;

pub struct QuestScene;

impl Plugin for QuestScene {
    fn build(&self, app: &mut App) {
        app.init_state::<SceneState>()
            // When this state machine is set to scanning, we begin our scan
            .add_systems(OnEnter(SceneState::Scanning), capture_scene)
            // While we're scanning, we wait for an event that signifies that the scan is complete
            .add_systems(
                Update,
                wait_scan_complete.run_if(in_state(SceneState::Scanning)),
            )
            // When we're done scanning we emit a query to the captured data
            .add_systems(OnExit(SceneState::Scanning), query_scene)
            // We wait for the query to complete
            .add_systems(
                Update,
                wait_query_complete.run_if(in_state(SceneState::QueryingScene)),
            )
            // .add_systems(Update, dbg_mesh_gizmos)
            .add_systems(OnEnter(SceneState::Done), init_world_mesh)
            .add_systems(OnEnter(SceneState::Done), init_room_layout);
    }
}

// This prompts the user to do a scene setup
// This relies on the extension listed at
// https://registry.khronos.org/OpenXR/specs/1.0/html/xrspec.html#XR_FB_scene_capture
fn capture_scene(instance: Res<XrInstance>, session: Res<XrSession>) {
    let vtable = instance.exts().fb_scene_capture.unwrap();
    let info = sys::SceneCaptureRequestInfoFB {
        ty: sys::SceneCaptureRequestInfoFB::TYPE,
        next: null(),
        request_byte_count: 0,
        request: null(),
    };
    let mut request = sys::AsyncRequestIdFB::default();
    oxr!((vtable.request_scene_capture)(
        session.as_raw(),
        &info,
        &mut request
    ));
}

// We wait for an XrEventDataSceneCaptureCompleteFB event
fn wait_scan_complete(events: NonSend<XrEvents>, mut state: ResMut<NextState<SceneState>>) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&event.inner) }.unwrap();
        if let Event::SceneCaptureCompleteFB(_) = event {
            state.set(SceneState::QueryingScene)
        };
    }
}

// Emit a query to get entities with the TRIANGLE_MESH_META component
// This relies on the extension listed at
// https://registry.khronos.org/OpenXR/specs/1.0/html/xrspec.html#XR_FB_spatial_entity_query
fn query_scene(instance: Res<XrInstance>, session: Res<XrSession>) {
    // TODO: Fix the filter
    // currently adding it gives an error "insight_QueryAnchorSpaces invalid filter flags provided: 24"
    let _filter = Box::leak(Box::new(SpaceComponentFilterInfoFB {
        ty: SpaceComponentFilterInfoFB::TYPE,
        next: null(),
        component_type: SpaceComponentTypeFB::TRIANGLE_MESH_M,
    }));

    // TODO: Is it safe to use these without leaking them?
    let query = Box::leak(Box::new(SpaceQueryInfoFB {
        ty: SpaceQueryInfoFB::TYPE,
        next: null(),
        query_action: SpaceQueryActionFB::LOAD,
        max_result_count: 20u32,
        timeout: Duration::NONE,
        filter: null(),
        exclude_filter: null(),
    }));

    let vtable = instance.exts().fb_spatial_entity_query.unwrap();
    let mut request = sys::AsyncRequestIdFB::default();

    oxr!((vtable.query_spaces)(
        session.as_raw(),
        query as *const _ as *const _,
        &mut request
    ));
}

fn get_query_results(
    resultsAvailable: SpaceQueryResultsAvailableFB,
    session: Session,
    exts: &InstanceExtensions,
) -> Vec<SpaceQueryResultFB> {
    let vtable = exts.fb_spatial_entity_query.unwrap();
    // To figure out how many results were generated, we call xrRetrieveSpaceQueryResultsFB
    // with result_capacity_input set to 0
    let mut query_results = SpaceQueryResultsFB {
        ty: SpaceQueryResultsFB::TYPE,
        next: null_mut(),
        result_capacity_input: 0,
        result_count_output: 0,
        results: null_mut(),
    };
    oxr!((vtable.retrieve_space_query_results)(
        session,
        resultsAvailable.request_id(),
        &mut query_results
    ));
    // The number of results available to us are written into result_count_output
    let size = query_results.result_count_output;

    // On our next query we want to retrieve all the results that are available to us
    query_results.result_capacity_input = size;
    // Create a vector large enough to contain the number of available results
    let mut results = Vec::with_capacity(size as usize);
    query_results.results = results.as_mut_ptr();

    // Populate our vector with all of the results that are available
    oxr!((vtable.retrieve_space_query_results)(
        session,
        resultsAvailable.request_id(),
        &mut query_results
    ));
    unsafe { results.set_len(size as usize) }

    results
}

fn get_supported_components(space: Space, exts: &InstanceExtensions) -> Vec<SpaceComponentTypeFB> {
    let vtable = exts.fb_spatial_entity.unwrap();
    let mut cnt = 0;
    // This call populates cnt with the number of supported components
    oxr!((vtable.enumerate_space_supported_components)(
        space,
        0,
        &mut cnt,
        null_mut()
    ));
    let size = cnt as usize;
    // Create a vector that can fit all of the supported components
    let mut exts: Vec<SpaceComponentTypeFB> = Vec::with_capacity(size);
    // Populate the vector with all of the supported component types
    oxr!((vtable.enumerate_space_supported_components)(
        space,
        cnt,
        &mut cnt,
        exts.as_mut_ptr()
    ));
    unsafe { exts.set_len(size) };

    exts
}

fn check_component_enabled(
    space: sys::Space,
    component: SpaceComponentTypeFB,
    exts: &InstanceExtensions,
) -> bool {
    let vtable = exts.fb_spatial_entity.unwrap();

    // Check whether the component is enabled for this entity
    // Note: Being enabled is different to being supported
    let mut status = SpaceComponentStatusFB {
        ty: SpaceComponentStatusFB::TYPE,
        next: null_mut(),
        enabled: false.into(),
        change_pending: false.into(),
    };
    oxr!((vtable.get_space_component_status)(
        space,
        component,
        &mut status
    ));
    status.enabled.into()
}

// Making anchors LOCATABLE means that we can use functions like XrLocateSpace on them
fn make_space_locatable(space: sys::Space, exts: &InstanceExtensions) {
    let vtable = exts.fb_spatial_entity.unwrap();

    let mut status = SpaceComponentStatusSetInfoFB {
        ty: SpaceComponentStatusSetInfoFB::TYPE,
        next: null(),
        component_type: SpaceComponentTypeFB::LOCATABLE,
        enabled: true.into(),
        timeout: Duration::NONE,
    };
    let mut request = sys::AsyncRequestIdFB::default();
    // TODO: Actually handle this async request.
    oxr!((vtable.set_space_component_status)(
        space,
        &mut status,
        &mut request
    ));
}

// This function waits for our query to complete
fn wait_query_complete(
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    events: NonSend<XrEvents>,
    mut state: ResMut<NextState<SceneState>>,
) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&event.inner) }.unwrap();
        match event {
            // Report once the event is complete, and warn if it's failed
            Event::SpaceQueryCompleteFB(query) => {
                let result = query.result();
                if result == bevy_oxr::xr::sys::Result::SUCCESS {
                    info!("Space Query Completed Successfully");
                } else {
                    warn!(
                        r#"Space Query Completed {:?} Failed With Error "{}""#,
                        query.request_id(),
                        result
                    )
                }
            }
            Event::SpaceQueryResultsAvailableFB(resultsAvailable) => {
                let exts = instance.exts();
                let mut room_layout = SpatialAnchors {
                    mesh: sys::Space::NULL,
                    floor: sys::Space::NULL,
                    walls: Vec::new(),
                    ceiling: sys::Space::NULL,
                };
                let results = get_query_results(resultsAvailable, session.as_raw(), exts);
                // I wish I could do this with a hashmap or something...
                // Lets just assume it stays small
                let mut mapping: Vec<(UuidEXT, sys::Space)> = Vec::new();

                // This lets us build up a mapping from XrUuidExts to XrSpace handles
                for result in &results {
                    let space = result.space;
                    let vtable = exts.fb_spatial_entity.unwrap();
                    let mut id = UuidEXT {
                        data: [0; UUID_SIZE_EXT],
                    };
                    oxr!((vtable.get_space_uuid)(space, &mut id));
                    mapping.push((id, space));
                }

                for result in results {
                    let space = result.space;

                    let components = get_supported_components(space, exts);
                    info!("{space:?} supports components: {components:?}");

                    // Check if the space is the mesh
                    let ty = SpaceComponentTypeFB::TRIANGLE_MESH_M;
                    if components.contains(&ty) && check_component_enabled(space, ty, exts) {
                        make_space_locatable(space, exts);
                        room_layout.mesh = space;
                        continue;
                    }

                    let ty = SpaceComponentTypeFB::ROOM_LAYOUT;
                    if components.contains(&ty) && check_component_enabled(space, ty, exts) {
                        let vtable = exts.fb_scene.unwrap();
                        // I really really wish these would impl Default
                        let mut layout = RoomLayoutFB {
                            ty: RoomLayoutFB::TYPE,
                            next: null(),
                            floor_uuid: UuidEXT {
                                data: [0; UUID_SIZE_EXT],
                            },
                            ceiling_uuid: UuidEXT {
                                data: [0; UUID_SIZE_EXT],
                            },
                            wall_uuid_capacity_input: 0,
                            wall_uuid_count_output: 0,
                            wall_uuids: null_mut(),
                        };
                        oxr!((vtable.get_space_room_layout)(
                            session.as_raw(),
                            space,
                            &mut layout
                        ));
                        let capacity = layout.wall_uuid_count_output;
                        layout.wall_uuid_capacity_input = capacity;
                        let mut walls = Vec::with_capacity(capacity as usize);
                        layout.wall_uuids = walls.as_mut_ptr();
                        oxr!((vtable.get_space_room_layout)(
                            session.as_raw(),
                            space,
                            &mut layout
                        ));
                        unsafe { walls.set_len(capacity as usize) };

                        let floor = layout.floor_uuid;
                        let ceiling = layout.ceiling_uuid;

                        for (id, space) in &mapping {
                            if id.data == floor.data {
                                // Is this the floor?
                                make_space_locatable(*space, exts);
                                room_layout.floor = *space;
                            } else if id.data == ceiling.data {
                                // Is this the ceiling?
                                make_space_locatable(*space, exts);
                                room_layout.ceiling = *space;
                            } else {
                                // Is this a wall?
                                for wall_id in &walls {
                                    if wall_id.data == id.data {
                                        make_space_locatable(*space, exts);
                                        room_layout.walls.push(*space)
                                    }
                                }
                            }
                        }
                    }
                }
                // Set the XrSpace handle as the one we'll use in init_world_mesh
                commands.insert_resource(room_layout);
                state.0 = Some(SceneState::Done);
            }
            _ => {}
        }
    }
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    xr_frame_state: Res<XrFrameState>,
    room_layout: Res<SpatialAnchors>,
    input: Res<XrInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let vtable = instance.exts().meta_spatial_entity_mesh.unwrap();
    // Create a new mesh to be used in bevy
    let mut bevy_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let info = sys::SpaceTriangleMeshGetInfoMETA {
        ty: StructureType::SPACE_TRIANGLE_MESH_GET_INFO_META,
        next: null(),
    };
    // By setting vertex_capacity_input and index_capacity_input both to 0
    // the runtime will update vertex_count_output and index_count_output
    // to indicate how many indices and vertices are available to us
    let mut mesh = sys::SpaceTriangleMeshMETA {
        ty: StructureType::SPACE_TRIANGLE_MESH_META,
        next: null_mut(),
        vertex_capacity_input: 0,
        vertex_count_output: 0,
        vertices: null_mut(),
        index_capacity_input: 0,
        index_count_output: 0,
        indices: null_mut(),
    };
    oxr!((vtable.get_space_triangle_mesh)(
        room_layout.mesh,
        &info,
        &mut mesh
    ));

    // Create vectors with capacity to store all of the vertices and indices
    let v_size = mesh.vertex_count_output as usize;
    let i_size = mesh.index_count_output as usize;
    let mut vertices: Vec<Vector3f> = Vec::with_capacity(v_size);
    let mut indices: Vec<u32> = Vec::with_capacity(i_size);

    // Set the mesh struct to recieve all of the vertex and index data
    mesh.vertex_capacity_input = v_size as _;
    mesh.index_capacity_input = i_size as _;
    mesh.vertices = vertices.as_mut_ptr() as *mut _;
    mesh.indices = indices.as_mut_ptr();

    // Populate the mesh struct with all of the mesh data
    oxr!((vtable.get_space_triangle_mesh)(
        room_layout.mesh,
        &info,
        &mut mesh
    ));

    unsafe {
        vertices.set_len(v_size);
        indices.set_len(i_size)
    }

    let mut location = SpaceLocation {
        ty: SpaceLocation::TYPE,
        next: null_mut(),
        location_flags: SpaceLocationFlags::EMPTY,
        pose: Posef::IDENTITY,
    };

    // Get the position of the user relative to the scene mesh anchor
    oxr!((instance.fp().locate_space)(
        room_layout.mesh,
        input.stage.as_raw(),
        xr_frame_state.lock().unwrap().predicted_display_time,
        &mut location,
    ));
    let translation = location.pose.position;
    let rotation = location.pose.orientation;

    // We need to map between Vector3f and Vec3 because Vector3f is repr(C) and Vec3 is not
    // This means they could potentially have different layouts
    bevy_mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices
            .into_iter()
            .map(|Vector3f { x, y, z }| Vec3 { x, y, z })
            .collect::<Vec<_>>(),
    );
    let indices = mesh::Indices::U32(indices);
    bevy_mesh.insert_indices(indices);

    // Here we spawn our mesh that represents the room
    commands.spawn((
        AsyncCollider(ComputedCollider::TriMesh),
        PbrBundle {
            mesh: meshes.add(bevy_mesh),
            material: materials.add(Color::rgba(0.0, 0.0, 0.0, 0.0)),
            transform: Transform {
                translation: Vec3 {
                    x: translation.x,
                    y: translation.y,
                    z: translation.z,
                },
                rotation: Quat::from_array([-rotation.x, -rotation.z, -rotation.y, -rotation.w]),
                ..default()
            },
            ..default()
        },
        CollisionLayers::new(PhysLayer::Terrain, LayerMask::ALL),
        RigidBody::Static,
    ));
}

fn init_room_layout(
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    xr_frame_state: Res<XrFrameState>,
    room_layout: Res<SpatialAnchors>,
    input: Res<XrInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Do fucky stuff with walls and shit

    let vtable = instance.exts().fb_scene.unwrap();

    let mut create_surface = |space| {
        let mut location = SpaceLocation {
            ty: SpaceLocation::TYPE,
            next: null_mut(),
            location_flags: SpaceLocationFlags::EMPTY,
            pose: Posef::IDENTITY,
        };

        oxr!((instance.fp().locate_space)(
            space,
            input.stage.as_raw(),
            xr_frame_state.lock().unwrap().predicted_display_time,
            &mut location,
        ));
        let translation = location.pose.position;
        let rotation = location.pose.orientation;

        let mut floor_rect = Rect2Df::default();
        oxr!((vtable.get_space_bounding_box2_d)(
            session.as_raw(),
            space,
            &mut floor_rect
        ));
        let Offset2Df { x, y } = floor_rect.offset;
        commands.spawn(PbrBundle {
            mesh: meshes.add(Rectangle {
                half_size: Vec2 { x, y },
            }),
            material: materials.add(Color::SILVER),
            transform: Transform {
                translation: Vec3 {
                    x: translation.x,
                    y: translation.y,
                    z: translation.z,
                },
                rotation: Quat::from_array([-rotation.x, -rotation.y, -rotation.z, -rotation.w]),
                ..default()
            },
            ..Default::default()
        });
    };

    create_surface(room_layout.floor);
    create_surface(room_layout.ceiling);

    for wall in &room_layout.walls {
        create_surface(*wall);
    }
}
