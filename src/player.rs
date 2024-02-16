use bevy::{/*input::mouse::MouseMotion,*/ prelude::*};
use bevy_xpbd_3d::{math::*, prelude::*, /*PhysicsSchedule, PhysicsStepSet*/};

// Set camera move speed.
//const CAMERA_ROTATE_SPEED: Vec2 = Vec2::new(50.0, 50.0);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            /*.add_systems(
                PhysicsSchedule,
                (move_camera, movement.before(move_camera)).before(PhysicsStepSet::BroadPhase)
            )*/;
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
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
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