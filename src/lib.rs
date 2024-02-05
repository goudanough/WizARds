#![allow(non_snake_case)]

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::transform::components::Transform;
use bevy_ggrs::GgrsConfig;
use bevy_oxr::graphics::extensions::XrExtensions;
use bevy_oxr::graphics::{XrAppInfo, XrPreferdBlendMode};
use bevy_oxr::xr_input::debug_gizmos::OpenXrDebugRenderer;
use bevy_oxr::xr_input::hands::common::{
    HandInputDebugRenderer, HandResource, HandsResource, IndexResource, LittleResource,
    MiddleResource, OpenXrHandInput, RingResource, ThumbResource,
};
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker};
use bevy_oxr::xr_input::xr_camera::XrCameraType;
use bevy_oxr::DefaultXrPlugins;
use bytemuck::{Pod, Zeroable};
use network::NetworkPlugin;

mod network;

const FPS: usize = 72;

pub type WizGgrsConfig = GgrsConfig<PlayerInput>;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable, Default)]
pub struct PlayerInput {
    head_pos: Vec3,
    spell: u32, // TODO change to be an enum type equivalent to a u32
    left_hand_pos: Vec3,
    _padding0: u32,
    right_hand_pos: Vec3,
    _padding1: u32,
    head_rot: Quat,
    left_hand_rot: Quat,
    right_hand_rot: Quat,
}

#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_systems(Startup, setup)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(NetworkPlugin);

    #[cfg(target_os = "android")]
    {
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
        .add_plugins(OpenXrHandInput)
        .add_plugins(OpenXrDebugRenderer)
        .add_plugins(HandInputDebugRenderer)
    }

    #[cfg(not(target_os = "android"))]
    {
        app.add_plugins(DefaultPlugins)
            .add_systems(Startup, spawn_camera)
            .add_systems(Startup, spoof_xr_components);
    }

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

fn spoof_xr_components(mut commands: Commands) {
    commands.spawn((Transform::default(), OpenXRLeftEye));
    commands.spawn((Transform::default(), OpenXRRightEye));

    let mut define = |ty: HandBone| {
        commands
            .spawn((Transform::default(), OpenXRTracker, ty))
            .id()
    };

    let hands = HandsResource {
        left: HandResource {
            palm: define(HandBone::Palm),
            wrist: define(HandBone::Wrist),
            thumb: ThumbResource {
                metacarpal: define(HandBone::ThumbMetacarpal),
                proximal: define(HandBone::ThumbProximal),
                distal: define(HandBone::ThumbDistal),
                tip: define(HandBone::ThumbTip),
            },
            index: IndexResource {
                metacarpal: define(HandBone::IndexMetacarpal),
                proximal: define(HandBone::IndexProximal),
                intermediate: define(HandBone::IndexProximal),
                distal: define(HandBone::IndexDistal),
                tip: define(HandBone::IndexTip),
            },
            middle: MiddleResource {
                metacarpal: define(HandBone::MiddleMetacarpal),
                proximal: define(HandBone::MiddleProximal),
                intermediate: define(HandBone::MiddleProximal),
                distal: define(HandBone::MiddleDistal),
                tip: define(HandBone::MiddleTip),
            },
            ring: RingResource {
                metacarpal: define(HandBone::RingMetacarpal),
                proximal: define(HandBone::RingProximal),
                intermediate: define(HandBone::RingProximal),
                distal: define(HandBone::RingDistal),
                tip: define(HandBone::RingTip),
            },
            little: LittleResource {
                metacarpal: define(HandBone::LittleMetacarpal),
                proximal: define(HandBone::LittleProximal),
                intermediate: define(HandBone::LittleProximal),
                distal: define(HandBone::LittleDistal),
                tip: define(HandBone::LittleTip),
            },
        },
        right: HandResource {
            palm: define(HandBone::Palm),
            wrist: define(HandBone::Wrist),
            thumb: ThumbResource {
                metacarpal: define(HandBone::ThumbMetacarpal),
                proximal: define(HandBone::ThumbProximal),
                distal: define(HandBone::ThumbDistal),
                tip: define(HandBone::ThumbTip),
            },
            index: IndexResource {
                metacarpal: define(HandBone::IndexMetacarpal),
                proximal: define(HandBone::IndexProximal),
                intermediate: define(HandBone::IndexProximal),
                distal: define(HandBone::IndexDistal),
                tip: define(HandBone::IndexTip),
            },
            middle: MiddleResource {
                metacarpal: define(HandBone::MiddleMetacarpal),
                proximal: define(HandBone::MiddleProximal),
                intermediate: define(HandBone::MiddleProximal),
                distal: define(HandBone::MiddleDistal),
                tip: define(HandBone::MiddleTip),
            },
            ring: RingResource {
                metacarpal: define(HandBone::RingMetacarpal),
                proximal: define(HandBone::RingProximal),
                intermediate: define(HandBone::RingProximal),
                distal: define(HandBone::RingDistal),
                tip: define(HandBone::RingTip),
            },
            little: LittleResource {
                metacarpal: define(HandBone::LittleMetacarpal),
                proximal: define(HandBone::LittleProximal),
                intermediate: define(HandBone::LittleProximal),
                distal: define(HandBone::LittleDistal),
                tip: define(HandBone::LittleTip),
            },
        }
    };

    commands.insert_resource(hands);
}
