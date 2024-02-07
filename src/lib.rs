#![allow(non_snake_case)]

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
use bevy_oxr::xr_input::hands::common::{HandInputDebugRenderer, OpenXrHandInput, HandResource, HandsResource};
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::hands::hand_tracking::HandTrackingData;
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::hands::HandBone;

#[cfg(target_os = "android")]
use bevy_oxr::DefaultXrPlugins;
#[cfg(target_os = "android")]
use bevy_oxr::input::XrInput;
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::{actions::XrActionSets,
                        oculus_touch::OculusController,
                        Hand};
#[cfg(target_os = "android")]
use bevy_oxr::resources::{XrFrameState, XrInstance, XrSession};
// #[cfg(target_os = "android")]
// use bevy_oxr::xr_input::prototype_locomotion::{proto_locomotion, PrototypeLocomotionConfig};
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::trackers::{
    OpenXRController, OpenXRLeftController, OpenXRRightController, OpenXRTracker,OpenXRLeftEye, OpenXRRightEye
};
use projectile::ProjectilePlugin;

mod projectile;

use crate::speech::SpeechPlugin;
use crate::spell_control::SpellControlPlugin;

mod speech;
mod spell_control;


#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_systems(Startup, setup)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(ProjectilePlugin)
        .add_plugins(SpeechPlugin)
        .add_plugins(SpellControlPlugin);

    #[cfg(target_os = "android")]
    {   

        //println!("{}", Path::new("/storage/emulated/0/Android/data/org.goudanough.wizARds/files/vosk-model").exists());
       
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
        .add_plugins(OpenXrHandInput)
        // .add_systems(Update, proto_locomotion)
        .add_systems(Startup, (spawn_controllers_example, spawn_vr_camera));
        // .insert_resource(PrototypeLocomotionConfig::default());
    }

    #[cfg(not(target_os = "android"))]
    {
        app.add_plugins(DefaultPlugins)
            .add_systems(Startup, spawn_camera);
    }

    app.run()
}

#[derive(Component)]
struct VRCamera;
fn spawn_vr_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5.0, 6.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        VRCamera,
    ));
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
    let plane_mesh = Mesh::from(shape::Plane::from_size(5.0));
    commands.spawn((Collider::trimesh_from_mesh(&plane_mesh).unwrap(), 
        PbrBundle {
            mesh: meshes.add(plane_mesh),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        }));
    // cube
    commands.spawn((PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
        },
        Collider::cuboid(0.1, 0.1, 0.1)));
    // cube
    commands.spawn((PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.0, 0.0).into()),
        transform: Transform::from_xyz(0.0, 0.5, 1.0),
        ..default()
    },
    Collider::cuboid(0.1, 0.1, 0.1)));
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

#[cfg(target_os = "android")]
fn spawn_controllers_example(mut commands: Commands) {
    //left hand
    commands.spawn((
        OpenXRLeftController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
        Hand::Left,
    ));
    //right hand
    commands.spawn((
        OpenXRRightController,
        OpenXRController,
        OpenXRTracker,
        SpatialBundle::default(),
        Hand::Right,
    ));
}






  


