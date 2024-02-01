use std::{
    mem::MaybeUninit,
    ptr::{null, null_mut},
};

use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    resources::{XrInstance, XrSession},
    xr::{
        sys::{self, SpaceComponentFilterInfoFB, SpaceQueryInfoFB},
        AsyncRequestIdFB, Duration, Event, SpaceComponentTypeFB, SpaceQueryActionFB, StructureType,
    },
    XrEvents,
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

use crate::oxr;

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

fn wait_scan_complete(
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    events: NonSend<XrEvents>,
    mut state: ResMut<NextState<SceneState>>,
) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&(*event).inner) }.unwrap();
        if let Event::SceneCaptureCompleteFB(_) = event {
            state.0 = Some(SceneState::ScanComplete)
        };
        // match event {
        //     Event::SceneCaptureCompleteFB(_) => state.0 = Some(SceneState::Done),
        //     Event::SpaceSetStatusCompleteFB(setStatusComplete) => {
        //         commands.insert_resource(Space(setStatusComplete.space()));
        //     }
        //     Event::SpaceQueryResultsAvailableFB(resultsAvailable) => {
        //         let vtable = instance.exts().fb_spatial_entity_query.unwrap();
        //         let mut query_results = SpaceQueryResultsFB {
        //             ty: SpaceQueryResultsFB::TYPE,
        //             next: null_mut(),
        //             result_capacity_input: 0,
        //             result_count_output: 0,
        //             results: null_mut(),
        //         };
        //         oxr!((vtable.retrieve_space_query_results)(
        //             session.as_raw(),
        //             resultsAvailable.request_id(),
        //             &mut query_results
        //         ));
        //         let size = query_results.result_count_output;

        //         oxr!((vtable.retrieve_space_query_results)(
        //             session.as_raw(),
        //             resultsAvailable.request_id(),
        //             &mut query_results
        //         ));
        //         query_results.result_capacity_input = size;
        //         let mut results = Vec::with_capacity(size as usize);
        //         query_results.results = results.as_mut_ptr();

        //         oxr!((vtable.retrieve_space_query_results)(
        //             session.as_raw(),
        //             resultsAvailable.request_id(),
        //             &mut query_results
        //         ));
        //         unsafe { results.set_len(size as usize) };

        //         for result in results {
        //             commands.insert_resource(Space(result.space));
        //         }
        //     }
        //     _ => {}
        // }
    }
}

fn query_scene(
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    mut state: ResMut<NextState<SceneState>>,
) {
    let filter = SpaceComponentFilterInfoFB {
        ty: SpaceComponentFilterInfoFB::TYPE,
        next: null(),
        component_type: SpaceComponentTypeFB::TRIANGLE_MESH_M,
    };

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

fn wait_query_complete(events: NonSend<XrEvents>, mut state: ResMut<NextState<SceneState>>) {
    for event in &events.0 {
        let event = unsafe { Event::from_raw(&(*event).inner) }.unwrap();
        if let Event::SpaceQueryCompleteFB(query) = event {
            oxr!(query.result());
            state.0 = Some(SceneState::ScanComplete);
        };
    }
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(vtable) = instance.exts().meta_spatial_entity_mesh {
        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);

        panic!("init_world_mesh");

        let space = sys::Space::NULL;

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
        oxr!((vtable.get_space_triangle_mesh)(space, &info, &mut mesh));

        let v_size = mesh.vertex_count_output as usize;
        let i_size = mesh.index_count_output as usize;
        let mut vertices: Vec<Vec3> = Vec::with_capacity(v_size);
        let mut indices: Vec<u32> = Vec::with_capacity(i_size);

        mesh.vertex_capacity_input = v_size as _;
        mesh.index_capacity_input = i_size as _;
        mesh.vertices = vertices.as_mut_ptr() as *mut _;
        mesh.indices = indices.as_mut_ptr();

        oxr!((vtable.get_space_triangle_mesh)(space, &info, &mut mesh));

        unsafe {
            vertices.set_len(v_size);
            indices.set_len(i_size)
        }

        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        let indices = mesh::Indices::U32(indices);
        bevy_mesh.set_indices(Some(indices));

        commands
            .spawn(PbrBundle {
                mesh: meshes.add(bevy_mesh),
                material: materials.add(Color::rgb(0., 0.867, 1.).into()),
                ..default()
            })
            .insert(Wireframe);
    } else {
        todo!("Fall back to regular scene API when XR_META_spatial_entity_mesh not available")
    }
}
