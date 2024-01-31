#![allow(non_snake_case)]

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::transform::components::Transform;
use bevy_ggrs::GgrsConfig;
use bevy_oxr::graphics::extensions::XrExtensions;
use bevy_oxr::graphics::{XrAppInfo, XrPreferdBlendMode};
use bevy_oxr::xr_input::debug_gizmos::OpenXrDebugRenderer;
use bevy_oxr::xr_input::hands::common::{HandInputDebugRenderer, OpenXrHandInput};
use bevy_oxr::xr_input::trackers::{
    OpenXRController, OpenXRLeftController, OpenXRRightController, OpenXRTracker,
};
use bevy_oxr::DefaultXrPlugins;
use bytemuck::{Pod, Zeroable};
use rollback::RollbackPlugin;

mod rollback;

const FPS: usize = 72;
const NUM_PLAYERS: usize = 2;

pub type RollbackConfig = GgrsConfig<PlayerInput>;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable, Default)]
pub struct PlayerInput {
    head_pos: Vec3,
    _padding: u32,
    head_rot: Quat,
}

#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_systems(Startup, setup)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin);
    let mut reqeusted_extensions = XrExtensions::default();
    reqeusted_extensions
        .enable_fb_passthrough()
        .enable_hand_tracking();

    app.add_plugins(DefaultXrPlugins {
        reqeusted_extensions,
        prefered_blend_mode: XrPreferdBlendMode::AlphaBlend,
        app_info: XrAppInfo {
            name: "wizARds".to_string(),
        },
    })
    .add_plugins(OpenXrDebugRenderer)
    .add_plugins(HandInputDebugRenderer)
    .add_plugins(OpenXrHandInput)
    .add_plugins(RollbackPlugin)
    // .add_systems(Update, proto_locomotion)
    .add_systems(Startup, spawn_controllers_example);
    // .insert_resource(PrototypeLocomotionConfig::default());
    // app.add_plugins(DefaultPlugins)
    //     .add_systems(Startup, spawn_camera);

    app.run()
}

#[derive(Component)]
struct PancakeCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5.0, 6.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PancakeCamera,
    ));
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::Rgba {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.0, 0.0).into()),
        transform: Transform::from_xyz(0.0, 0.5, 1.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

fn spawn_controllers_example(mut commands: Commands) {
    //left hand
    commands.spawn((
        OpenXRLeftController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
    ));
    //right hand
    commands.spawn((
        OpenXRRightController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
    ));
}
