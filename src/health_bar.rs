use bevy::prelude::*;
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye};

pub struct HealthBarPlugin;

const HEALTHBAR_HEIGHT: f32 = 0.2; // TODO have these consts be decided at runtime
const HEALTHBAR_DISTANCE: f32 = 0.5;

impl Plugin for HealthBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_health_bar)
            .add_systems(Update, update_health_bar);
    }
}

#[derive(Component, Reflect)]
struct Health {
    max: f32,
    current: f32,
}

impl Health {
    fn value(&self) -> f32 {
        self.current / self.max
    }
}

#[derive(Component)]
struct HealthBarBackground;

#[derive(Component)]
struct HealthBar;

fn spawn_health_bar(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(0.6, 0.1)))),
                material: materials.add(StandardMaterial {
                    base_color: Color::BLACK,
                    depth_bias: 0.1,
                    ..default()
                }),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
            Health {
                current: 0.7,
                max: 1.0,
            },
            HealthBarBackground,
        ))
        .with_children(|parent| {
            parent.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(0.6, 0.1)))),
                    material: materials.add(StandardMaterial {
                        base_color: Color::RED,
                        depth_bias: 1000.0,
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                },
                HealthBar,
            ));
        });
}

fn update_health_bar(
    health_query: Query<&Health, With<Health>>,
    mut health_bar_bg_query: Query<&mut Transform, (With<HealthBarBackground>, Without<HealthBar>)>,

    mut health_bar_query: Query<&mut Transform, (Without<HealthBarBackground>, With<HealthBar>)>,
    left_eye: Query<
        &Transform,
        (
            With<OpenXRLeftEye>,
            Without<HealthBarBackground>,
            Without<HealthBar>,
        ),
    >,
    right_eye: Query<
        &Transform,
        (
            With<OpenXRRightEye>,
            Without<HealthBarBackground>,
            Without<HealthBar>,
        ),
    >,
) {
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();

    let head_pos = left_eye.translation.lerp(right_eye.translation, 0.5);
    let head_rot = left_eye.rotation;

    let health = health_query.get_single().unwrap();
    let mut health_bar_bg_transform = health_bar_bg_query.get_single_mut().unwrap();

    let yaw = head_rot.to_euler(EulerRot::XYZ).2;
    health_bar_bg_transform.translation = Transform::from_xyz(
        head_pos.x - yaw.sin() * HEALTHBAR_DISTANCE,
        HEALTHBAR_HEIGHT,
        head_pos.z - yaw.cos() * HEALTHBAR_DISTANCE,
    )
    .translation;

    health_bar_bg_transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.0, yaw, 0.0);

    let mut health_bar_transform = health_bar_query.get_single_mut().unwrap();

    health_bar_transform.scale = Vec3::new(health.value() * health.max, 0.6, 1.0);
}
