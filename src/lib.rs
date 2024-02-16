mod network;
mod projectile;
mod speech;
mod spell_control;
mod boss;
mod player;

use crate::speech::SpeechPlugin;
use crate::spell_control::SpellControlPlugin;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::transform::components::Transform;
use bevy_ggrs::GgrsConfig;
#[cfg(target_os = "android")]
use bevy_oxr::graphics::{extensions::XrExtensions, XrAppInfo, XrPreferdBlendMode};
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::debug_gizmos::OpenXrDebugRenderer;
#[cfg(target_os = "android")]
use bevy_oxr::xr_input::hands::common::{HandInputDebugRenderer, OpenXrHandInput};
use bevy_oxr::xr_input::hands::common::{
    HandResource, HandsResource, IndexResource, LittleResource, MiddleResource, RingResource,
    ThumbResource,
};
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker};
#[cfg(target_os = "android")]
use bevy_oxr::DefaultXrPlugins;
use bevy_xpbd_3d::prelude::*;
use bytemuck::{Pod, Zeroable};
use network::NetworkPlugin;
use projectile::ProjectilePlugin;

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

#[derive(PhysicsLayer)]
enum PhysLayer {
    Player,
    PlayerProjectile,
}

#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_systems(Startup, setup)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(NetworkPlugin)
        // boss and player plugins(not use)
        .add_plugins(boss::BossPlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(ProjectilePlugin)
        .add_plugins(SpeechPlugin)
        .add_plugins(SpellControlPlugin);

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
        .add_plugins(HandInputDebugRenderer);
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
            transform: Transform::from_xyz(5.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
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
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane::from_size(128.0))),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(128.0, 0.005, 128.0),
    ));
    // cube
    commands.spawn((PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(2.0, 0.5, 4.0),
        ..default()
    },RigidBody::Dynamic, Collider::cuboid(0.5, 0.5, 0.5),ColliderDensity(100.0),));
    // cube
    commands.spawn((PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        material: materials.add(Color::rgb(0.8, 0.0, 0.0)),
        transform: Transform::from_xyz(3.0, 0.5, 1.0),
        ..default()
    },RigidBody::Dynamic, Collider::cuboid(0.5, 0.5, 0.5),ColliderDensity(100.0)),);
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

fn spoof_xr_components(mut commands: Commands) {
    commands.spawn((Transform::from_xyz(1.0, 2.0, 1.1), OpenXRLeftEye));
    commands.spawn((Transform::from_xyz(1.0, 2.0, 0.9), OpenXRRightEye));

    let mut define_l = |ty: HandBone| {
        commands
            .spawn((Transform::from_xyz(1.0, 1.0, 1.5), OpenXRTracker, ty))
            .id()
    };

    let left = HandResource {
        palm: define_l(HandBone::Palm),
        wrist: define_l(HandBone::Wrist),
        thumb: ThumbResource {
            metacarpal: define_l(HandBone::ThumbMetacarpal),
            proximal: define_l(HandBone::ThumbProximal),
            distal: define_l(HandBone::ThumbDistal),
            tip: define_l(HandBone::ThumbTip),
        },
        index: IndexResource {
            metacarpal: define_l(HandBone::IndexMetacarpal),
            proximal: define_l(HandBone::IndexProximal),
            intermediate: define_l(HandBone::IndexProximal),
            distal: define_l(HandBone::IndexDistal),
            tip: define_l(HandBone::IndexTip),
        },
        middle: MiddleResource {
            metacarpal: define_l(HandBone::MiddleMetacarpal),
            proximal: define_l(HandBone::MiddleProximal),
            intermediate: define_l(HandBone::MiddleProximal),
            distal: define_l(HandBone::MiddleDistal),
            tip: define_l(HandBone::MiddleTip),
        },
        ring: RingResource {
            metacarpal: define_l(HandBone::RingMetacarpal),
            proximal: define_l(HandBone::RingProximal),
            intermediate: define_l(HandBone::RingProximal),
            distal: define_l(HandBone::RingDistal),
            tip: define_l(HandBone::RingTip),
        },
        little: LittleResource {
            metacarpal: define_l(HandBone::LittleMetacarpal),
            proximal: define_l(HandBone::LittleProximal),
            intermediate: define_l(HandBone::LittleProximal),
            distal: define_l(HandBone::LittleDistal),
            tip: define_l(HandBone::LittleTip),
        },
    };

    let mut define_r = |ty: HandBone| {
        commands
            .spawn((Transform::from_xyz(1.0, 1.0, 0.5), OpenXRTracker, ty))
            .id()
    };

    let right = HandResource {
        palm: define_r(HandBone::Palm),
        wrist: define_r(HandBone::Wrist),
        thumb: ThumbResource {
            metacarpal: define_r(HandBone::ThumbMetacarpal),
            proximal: define_r(HandBone::ThumbProximal),
            distal: define_r(HandBone::ThumbDistal),
            tip: define_r(HandBone::ThumbTip),
        },
        index: IndexResource {
            metacarpal: define_r(HandBone::IndexMetacarpal),
            proximal: define_r(HandBone::IndexProximal),
            intermediate: define_r(HandBone::IndexProximal),
            distal: define_r(HandBone::IndexDistal),
            tip: define_r(HandBone::IndexTip),
        },
        middle: MiddleResource {
            metacarpal: define_r(HandBone::MiddleMetacarpal),
            proximal: define_r(HandBone::MiddleProximal),
            intermediate: define_r(HandBone::MiddleProximal),
            distal: define_r(HandBone::MiddleDistal),
            tip: define_r(HandBone::MiddleTip),
        },
        ring: RingResource {
            metacarpal: define_r(HandBone::RingMetacarpal),
            proximal: define_r(HandBone::RingProximal),
            intermediate: define_r(HandBone::RingProximal),
            distal: define_r(HandBone::RingDistal),
            tip: define_r(HandBone::RingTip),
        },
        little: LittleResource {
            metacarpal: define_r(HandBone::LittleMetacarpal),
            proximal: define_r(HandBone::LittleProximal),
            intermediate: define_r(HandBone::LittleProximal),
            distal: define_r(HandBone::LittleDistal),
            tip: define_r(HandBone::LittleTip),
        },
    };

    commands.insert_resource(HandsResource { left, right });
}
