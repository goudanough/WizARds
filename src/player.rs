use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_xpbd_3d::{math::*, prelude::*, PhysicsSchedule, PhysicsStepSet};

// Set camera move speed.
const CAMERA_ROTATE_SPEED: Vec2 = Vec2::new(50.0, 50.0);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                PhysicsSchedule,
                (move_camera, movement.before(move_camera)).before(PhysicsStepSet::BroadPhase),
            )
            .add_systems(Update, cursor_movement_system);
    }
}

#[derive(Component)]
pub struct Player;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                radius: 0.4,
                ..default()
            })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        },
        // Transform::from_xyz(0.0, 1.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(1.0, 0.4),
        // Prevent the player from falling over
        LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        // Cast the player shape downwards to detect when the player is grounded
        ShapeCaster::new(
            Collider::capsule(0.9, 0.35),
            Vector::NEG_Y * 0.05,
            Quaternion::default(),
            Vector::NEG_Y,
        )
        .with_max_time_of_impact(0.2)
        .with_max_hits(1),
        Restitution::new(0.0).with_combine_rule(CoefficientCombine::Min),
        Player,
    ));
}

// w,a,s,d move position
fn movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut players: Query<(&mut LinearVelocity, &ShapeHits), With<Player>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Player>)>,
) {
    if let Ok((mut linear_velocity, ground_hits)) = players.get_single_mut() {
        let camera_transform = camera_query.single();
        let mut forward = camera_transform.forward();
        forward.y = 0.0;
        forward = forward.normalize();
        let mut right = camera_transform.right();
        right.y = 0.0;
        right = right.normalize();
        // Directional movement
        if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
            linear_velocity.0 += forward * 1.2;
        }
        if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
            linear_velocity.0 += forward * -1.2;
        }
        if keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left) {
            linear_velocity.0 += right * -1.2;
        }
        if keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right) {
            linear_velocity.0 += right * 1.2;
        }

        // Jump if space pressed and the player is close enough to the ground
        if keyboard_input.just_pressed(KeyCode::Space) && !ground_hits.is_empty() {
            linear_velocity.y += 4.0;
        }

        // Slow player down on the x and y axes
        linear_velocity.x *= 0.8;
        linear_velocity.z *= 0.8;
    }
}

// move camera eq player position
fn move_camera(
    mut players: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    if let Ok(transform) = players.get_single_mut() {
        camera_query.single_mut().translation = transform.translation;
    }
}

// rotate camera
fn cursor_movement_system(
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    let mut delta: Vec2 = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta * time.delta_seconds();
    }

    if delta == Vec2::ZERO {
        return;
    }

    for mut transform in query.iter_mut() {

        let yaw = Quat::from_rotation_y(-delta.x * CAMERA_ROTATE_SPEED.x.to_radians());

        let pitch = Quat::from_rotation_x(-delta.y * CAMERA_ROTATE_SPEED.y.to_radians());

        let target_rotation = yaw * transform.rotation * pitch;

        transform.rotation = transform.rotation.lerp(target_rotation, 0.1);
    }
}