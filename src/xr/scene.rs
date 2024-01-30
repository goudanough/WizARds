use std::{mem::MaybeUninit, ptr::{null, null_mut}};

use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    input::XrInput, resources::{XrInstance, XrSession}, xr::{sys, Event, StructureType}, XrEvents
};

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
enum SceneState {
    #[default]
    Uninit,
    Scanning,
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
            .add_systems(OnEnter(SceneState::Done), init_world_mesh);
    }
}


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
    // let vtable = instance.exts().fb_spatial_entity_query.unwrap();

    // let mut results: SpaceQueryResultsFB = SpaceQueryResultsFB {
    //     ty: SpaceQueryResultsFB::TYPE,
    //     next: null_mut(),
    //     result_capacity_input: 0,
    //     result_count_output: 0,
    //     results: null_mut(),
    // };
    // oxr!((vtable.retrieve_space_query_results)(
    //     session.as_raw(),
    //     request.0,
    //     &mut results
    // ));

    // dbg!(results);

    // if results.result_count_output != 0 {
    //     state.0 = Some(SceneState::Done)
    // }

    for event in &events.0 {
        let event = unsafe { Event::from_raw(&(*event).inner) }.unwrap();
        if let Event::SceneCaptureCompleteFB(_) = event {
            state.0 = Some(SceneState::Done);
        }
    }
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    input: Res<XrInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(vtable) = instance.exts().meta_spatial_entity_mesh {
        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);

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
            input.stage.as_raw(),
            &info,
            &mut mesh
        ));

        let vertices = unsafe {
            Vec::from_raw_parts(
                mesh.vertices as *mut Vec3,
                mesh.vertex_count_output as usize,
                mesh.vertex_capacity_input as usize,
            )
        };
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

        let indices = unsafe {
            Vec::from_raw_parts(
                mesh.indices,
                mesh.index_count_output as usize,
                mesh.index_capacity_input as usize,
            )
        };
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
