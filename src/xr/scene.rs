use std::ptr::{null, null_mut};

use bevy::{
    prelude::*,
    render::{mesh, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    input::XrInput,
    resources::{XrFrameState, XrInstance, XrSession},
    xr::{
        sys::{
            self, Session, Space, SpaceComponentFilterInfoFB, SpaceComponentStatusFB,
            SpaceComponentStatusSetInfoFB, SpaceLocation, SpaceQueryInfoFB, SpaceQueryResultFB,
            SpaceQueryResultsFB,
        },
        Duration, Event, InstanceExtensions, Posef, SpaceComponentTypeFB, SpaceLocationFlags,
        SpaceQueryActionFB, SpaceQueryResultsAvailableFB, StructureType, Vector3f,
    },
    XrEvents,
};
use bevy_xpbd_3d::prelude::*;

use crate::{oxr, PhysLayer};

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
enum SceneState {
    #[default]
    Uninit,
    Scanning,
    QueryingScene,
    Done,
}

pub struct QuestScene;

impl Plugin for QuestScene {
    fn build(&self, app: &mut App) {
        app.init_state::<SceneState>()
            .add_systems(Startup, capture_scene_startup)
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
            .add_systems(OnEnter(SceneState::Done), init_world_mesh);
    }
}

// This function will be removed in future, and only serves the purpose of
// performing scene capture on startup. Eventually this will be managed
// by a menu system in another plugin instead
fn capture_scene_startup(mut state: ResMut<NextState<SceneState>>) {
    state.0 = Some(SceneState::Scanning);
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
            state.0 = Some(SceneState::QueryingScene)
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

// This struct is to retain the XrSpace handle representing the mesh of the room
#[derive(Resource)]
struct MeshSpace(sys::Space);

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
                for result in get_query_results(resultsAvailable, session.as_raw(), instance.exts())
                {
                    let space = result.space;

                    let exts = get_supported_components(space, instance.exts());
                    info!("{:?} supports components: {:?}", space, exts);

                    // The component that we care about for the scene mesh is TRIANGLE_MESH_M
                    // Continue if this entity doesn't support it

                    // TODO: Remove the check for this component once the filter in
                    // query_scene is fixed
                    if !exts.contains(&SpaceComponentTypeFB::TRIANGLE_MESH_M) {
                        continue;
                    }

                    let vtable = instance.exts().fb_spatial_entity.unwrap();

                    // Check whether TRIANGLE_MESH_M is enabled for this entity
                    // Note: Being enabled is different to being supported
                    let mut status = SpaceComponentStatusFB {
                        ty: SpaceComponentStatusFB::TYPE,
                        next: null_mut(),
                        enabled: false.into(),
                        change_pending: false.into(),
                    };
                    oxr!((vtable.get_space_component_status)(
                        space,
                        SpaceComponentTypeFB::TRIANGLE_MESH_M,
                        &mut status
                    ));
                    info!(
                        "TRIANGLE_MESH_M enabled for {:?}: {}",
                        space, status.enabled
                    );

                    // If TRIANGLE_MESH_M isn't enabled then this entity isn't the scene mesh
                    if !bool::from(status.enabled) {
                        continue;
                    }

                    // Once we've found our scene mesh, we need to make it LOCATABLE so we can
                    // apply transformations between the real world and bevy
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

                    info!("Setting {space:?} as XrHandle for scene mesh");

                    // Set the XrSpace handle as the one we'll use in init_world_mesh
                    commands.insert_resource(MeshSpace(space));
                    state.0 = Some(SceneState::Done);
                    break;
                }
            }
            _ => {}
        }
    }
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    xr_frame_state: Res<XrFrameState>,
    space: Res<MeshSpace>,
    input: Res<XrInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(vtable) = instance.exts().meta_spatial_entity_mesh {
        // Create a new mesh to be used in bevy
        let mut bevy_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );

        let MeshSpace(space) = *space;

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
        oxr!((vtable.get_space_triangle_mesh)(space, &info, &mut mesh));

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
        oxr!((vtable.get_space_triangle_mesh)(space, &info, &mut mesh));

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
            space,
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
                    rotation: Quat::from_array([
                        -rotation.x,
                        -rotation.z,
                        -rotation.y,
                        -rotation.w,
                    ]),
                    ..default()
                },
                ..default()
            },
            CollisionLayers::new(PhysLayer::Terrain, LayerMask::ALL),
            RigidBody::Static,
        ));
    } else {
        todo!("Fall back to regular scene API when XR_META_spatial_entity_mesh not available")
    }
}
