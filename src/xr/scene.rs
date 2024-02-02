use crate::oxr;
use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    input::XrInput,
    resources::{XrFrameState, XrInstance, XrSession},
    xr::{
        self,
        sys::{
            self, SpaceComponentFilterInfoFB, SpaceComponentStatusFB, SpaceLocation,
            SpaceQueryInfoFB, SpaceQueryResultsFB,
        },
        AsyncRequestIdFB, Duration, Event, Posef, SpaceComponentTypeFB, SpaceLocationFlags,
        SpaceQueryActionFB, StructureType, Time, Vector3f, ViewConfigurationType,
    },
    xr_input::{
        xr_camera::{XRProjection, XrCameraType},
        QuatConv, Vec3Conv,
    },
    XrEvents,
};
use std::{
    mem::MaybeUninit,
    os::unix::raw::off_t,
    ptr::{null, null_mut},
    sync::Arc,
};

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
enum SceneState {
    #[default]
    Uninit,
    Scanning,
    ScanComplete,
    QueryingScene,
    Done,
}

pub struct QuestScene;

impl Plugin for QuestScene {
    fn build(&self, app: &mut App) {
        app.add_state::<SceneState>()
            .add_systems(Startup, capture_scene)
            .add_systems(
                Update,
                wait_scan_complete.run_if(in_state(SceneState::Scanning)),
            )
            .add_systems(OnEnter(SceneState::ScanComplete), query_scene)
            .add_systems(
                Update,
                wait_query_complete.run_if(in_state(SceneState::QueryingScene)),
            )
            .add_systems(OnEnter(SceneState::Done), init_world_mesh);
    }
}

// This prompts the user to do a scene setup
fn capture_scene(
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    mut state: ResMut<NextState<SceneState>>,
) {
    let vtable = instance.exts().fb_scene_capture.unwrap();
    let info = sys::SceneCaptureRequestInfoFB {
        ty: sys::SceneCaptureRequestInfoFB::TYPE,
        next: null(),
        request_byte_count: 0,
        request: null(),
    };
    state.0 = Some(SceneState::Scanning);
    let mut request: MaybeUninit<sys::AsyncRequestIdFB> = MaybeUninit::uninit();
    oxr!((vtable.request_scene_capture)(
        session.as_raw(),
        &info,
        request.as_mut_ptr()
    ));
}

fn wait_scan_complete(events: NonSend<XrEvents>, mut state: ResMut<NextState<SceneState>>) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&(*event).inner) }.unwrap();
        if let Event::SceneCaptureCompleteFB(_) = event {
            state.0 = Some(SceneState::ScanComplete)
        };
    }
}

fn query_scene(
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    mut state: ResMut<NextState<SceneState>>,
) {
    // TODO: Fix the filter
    // currently adding it gives an error "insight_QueryAnchorSpaces invalid filter flags provided: 24"
    let filter = Box::leak(Box::new(SpaceComponentFilterInfoFB {
        ty: SpaceComponentFilterInfoFB::TYPE,
        next: null(),
        component_type: SpaceComponentTypeFB::TRIANGLE_MESH_M,
    }));

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
    let mut request_id: AsyncRequestIdFB = AsyncRequestIdFB::from_raw(0);

    state.0 = Some(SceneState::QueryingScene);
    oxr!((vtable.query_spaces)(
        session.as_raw(),
        query as *const _ as *const _,
        &mut request_id,
    ));
}

#[derive(Resource)]
struct MeshSpace(xr::Space);

fn wait_query_complete(
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    events: NonSend<XrEvents>,
    mut state: ResMut<NextState<SceneState>>,
) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&(*event).inner) }.unwrap();
        match event {
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
                let vtable = instance.exts().fb_spatial_entity_query.unwrap();
                let mut query_results = SpaceQueryResultsFB {
                    ty: SpaceQueryResultsFB::TYPE,
                    next: null_mut(),
                    result_capacity_input: 0,
                    result_count_output: 0,
                    results: null_mut(),
                };
                oxr!((vtable.retrieve_space_query_results)(
                    session.as_raw(),
                    resultsAvailable.request_id(),
                    &mut query_results
                ));
                let size = query_results.result_count_output;

                oxr!((vtable.retrieve_space_query_results)(
                    session.as_raw(),
                    resultsAvailable.request_id(),
                    &mut query_results
                ));
                query_results.result_capacity_input = size;
                let mut results = Vec::with_capacity(size as usize);
                query_results.results = results.as_mut_ptr();

                oxr!((vtable.retrieve_space_query_results)(
                    session.as_raw(),
                    resultsAvailable.request_id(),
                    &mut query_results
                ));
                unsafe { results.set_len(size as usize) };

                for result in &results {
                    let space = result.space;

                    let vtable = instance.exts().fb_spatial_entity.unwrap();
                    let mut cnt = 0;
                    oxr!((vtable.enumerate_space_supported_components)(
                        space,
                        0,
                        &mut cnt,
                        null_mut()
                    ));
                    let size = cnt as usize;
                    let mut exts: Vec<SpaceComponentTypeFB> = Vec::with_capacity(size);
                    oxr!((vtable.enumerate_space_supported_components)(
                        space,
                        size as _,
                        &mut cnt,
                        exts.as_mut_ptr()
                    ));
                    unsafe { exts.set_len(size) };

                    info!("{:?} supports components: {:?}", space, exts);

                    // exts contains an array of supported components
                    // the important one is to make sure it has TRIANGLE_MESH_M

                    if !exts.contains(&SpaceComponentTypeFB::TRIANGLE_MESH_M) {
                        continue;
                    }

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

                    if !bool::from(status.enabled) {
                        continue;
                    }

                    commands.insert_resource(MeshSpace(unsafe {
                        xr::Space::reference_from_raw(session.0.clone(), space)
                    }));
                    state.0 = Some(SceneState::Done)
                }
            }
            _ => {}
        }
    }
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    xr_frame_state: Res<XrFrameState>,
    space: Res<MeshSpace>,
    mut input: ResMut<XrInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(&mut Transform, &XrCameraType, &mut XRProjection)>,
) {
    if let Some(vtable) = instance.exts().meta_spatial_entity_mesh {
        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let space = &space.0;
        let space_raw = space.as_raw();

        let (flags, views) = session
            .0
            .locate_views(
                ViewConfigurationType::PRIMARY_STEREO,
                instance.now().unwrap(),
                &space,
            )
            .unwrap();

        for (mut transform, camera_type, mut xr_projection) in query.iter_mut() {
            let view_idx = match camera_type {
                XrCameraType::Xr(eye) => *eye as usize,
                XrCameraType::Flatscreen => continue,
            };
            let view = views.get(view_idx).unwrap();
            xr_projection.fov = view.fov;
            transform.rotation = view.pose.orientation.to_quat();
            transform.translation = view.pose.position.to_vec3();
        }

        // panic!("{:?}", views.iter().map(|v| v.pose).collect::<Vec<_>>());

        let info = sys::SpaceTriangleMeshGetInfoMETA {
            ty: StructureType::SPACE_TRIANGLE_MESH_GET_INFO_META,
            next: null(),
        };
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
            space_raw, &info, &mut mesh
        ));

        let v_size = mesh.vertex_count_output as usize;
        let i_size = mesh.index_count_output as usize;
        let mut vertices: Vec<Vector3f> = Vec::with_capacity(v_size);
        let mut indices: Vec<u32> = Vec::with_capacity(i_size);

        mesh.vertex_capacity_input = v_size as _;
        mesh.index_capacity_input = i_size as _;
        mesh.vertices = vertices.as_mut_ptr();
        mesh.indices = indices.as_mut_ptr();

        oxr!((vtable.get_space_triangle_mesh)(
            space_raw, &info, &mut mesh
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

        oxr!((instance.fp().locate_space)(
            input.stage.as_raw(),
            space_raw,
            xr_frame_state.lock().unwrap().predicted_display_time,
            &mut location,
        ));

        let translation = location.pose.position;
        let rotation = location.pose.orientation;

        // They define their orientation differently and it hurts me
        let new_vertices: Vec<Vec3> = vertices
            .into_iter()
            .map(|Vector3f { x, y, z }| Vec3 {
                x: y,
                y: z,
                z: x,
            })
            .collect();

        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_vertices);
        let indices = mesh::Indices::U32(indices);
        bevy_mesh.set_indices(Some(indices));

        commands
            .spawn(PbrBundle {
                mesh: meshes.add(bevy_mesh),
                material: materials.add(Color::WHITE.into()),
                transform: Transform {
                    translation: Vec3 {
                        x: -translation.y,
                        y: -translation.z,
                        z: -translation.x,
                    },
                    rotation: Quat::from_array([rotation.y, rotation.x, rotation.z, rotation.w]),
                    scale: Vec3 {
                        x: 1.,
                        y: 1.,
                        z: 1.,
                    },
                },
                ..default()
            })
            .insert(Wireframe);
    } else {
        todo!("Fall back to regular scene API when XR_META_spatial_entity_mesh not available")
    }
}
