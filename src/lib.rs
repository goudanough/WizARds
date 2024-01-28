#![allow(non_snake_case)]
mod boss;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::transform::components::Transform;
use bevy_xpbd_3d::prelude::*;
#[cfg(target_os = "android")]
use bevy_oxr::graphics::extensions::XrExtensions;
#[cfg(target_os = "android")]
use bevy_oxr::graphics::{XrAppInfo, XrPreferdBlendMode};
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::debug_gizmos::OpenXrDebugRenderer;
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::hands::common::{HandInputDebugRenderer, OpenXrHandInput};
#[cfg(target_os = "android")]
use bevy_oxr::DefaultXrPlugins;
// #[cfg(target_os = "android")]
// use bevy_oxr::xr_input::prototype_locomotion::{proto_locomotion, PrototypeLocomotionConfig};
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::trackers::{
    OpenXRController, OpenXRLeftController, OpenXRRightController, OpenXRTracker,
};

#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_systems(Startup, global_setup)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin);

    #[cfg(target_os = "android")]
    {
        let mut reqeusted_extensions = XrExtensions::default();
        reqeusted_extensions.enable_fb_passthrough().enable_hand_tracking();

        app.add_plugins(DefaultXrPlugins {
            reqeusted_extensions,
            prefered_blend_mode: XrPreferdBlendMode::AlphaBlend,
            app_info: XrAppInfo {
                name: "wizARds".to_string(),
            },
        })
        .add_plugins(OpenXrDebugRenderer)
        .add_plugins(HandInputDebugRenderer)
        .add_plugins(OpenXrHandInput);
        // .add_systems(Update, proto_locomotion)
        // .add_systems(Startup, spawn_controllers_example);
        // .insert_resource(PrototypeLocomotionConfig::default());
    }

    #[cfg(not(target_os = "android"))]
    {
        app.add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            boss::BossPlugin,
        ))
        .add_systems(Startup, spawn_camera);
    }

    app.run()
}

#[derive(Component)]
struct PancakeCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.0, 0.0).looking_at(Vec3::new(0.0, 1.0, 1.0), Vec3::Y),
        ..default()
        },
    PancakeCamera,
    ));
}
fn global_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ambient_light: ResMut<AmbientLight>,
    mut materials: ResMut<Assets<StandardMaterial>>,
)
    {commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane::from_size(128.0))),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(128.0, 0.005, 128.0),
    ));
    // commands.spawn(DirectionalLightBundle {
    //     directional_light: DirectionalLight {
    //         color: Color::WHITE,
    //         illuminance: 100000.0,
    //         ..Default::default()
    //     },
    //     transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
    //     ..Default::default()
    // });
    ambient_light.color = Color::WHITE;
    ambient_light.brightness = 1.0;
}

#[cfg(target_os = "android")]
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
