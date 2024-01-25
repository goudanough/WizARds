use std::{mem::MaybeUninit, ptr::null};

use bevy::{
    ecs::world,
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, render_resource::PrimitiveTopology},
};
use bevy_oxr::{
    resources::{XrInstance, XrSession},
    xr::{self, raw, sys},
};

use crate::oxr;

pub struct QuestScene;

impl Plugin for QuestScene {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, capture_scene)
            .add_systems(Startup, init_world_mesh.after(capture_scene));
    }
}

fn capture_scene(instance: Res<XrInstance>, session: Res<XrSession>) {
    let vtable = instance.exts().fb_scene_capture.unwrap();
    let info = sys::SceneCaptureRequestInfoFB {
        ty: sys::SceneCaptureRequestInfoFB::TYPE,
        next: null(),
        request_byte_count: 0,
        request: null(),
    };
    let mut request: MaybeUninit<sys::AsyncRequestIdFB> = MaybeUninit::uninit();
    oxr!((vtable.request_scene_capture)(session.as_raw(), &info, request.as_mut_ptr()));
    let request = unsafe { request.assume_init() };
    // TODO, Find a way to block until request is complete
}

fn init_world_mesh(
    mut commands: Commands,
    instance: Res<XrInstance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(vtable) = instance.exts().meta_spatial_entity_mesh {
        let bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let info = sys::SpaceTriangleMeshGetInfoMETA {
            ty: sys::StructureType::SPACE_TRIANGLE_MESH_GET_INFO_META,
            next: null(),
        };
        let mut mesh: MaybeUninit<sys::SpaceTriangleMeshMETA> = MaybeUninit::uninit();
        oxr!((vtable.get_space_triangle_mesh)(
            sys::Space::NULL,
            &info,
            mesh.as_mut_ptr()
        ));

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
