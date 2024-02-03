use bevy::{input::keyboard::KeyboardInput, prelude::*, render::mesh};
use bevy_xpbd_3d::prelude::*;

#[derive(Debug)]
enum MovementType {
    Linear,
    Static,
}

#[derive(Component, Debug)]
struct Projectile {
    movement_type: MovementType,
}

#[derive(Component)]
struct Velocity(Vec3);


pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, mut app: &mut App) {
        app.add_systems(Update, (update_projectiles, detect_projectile_collisions).chain())
        .add_systems(Update, spawn_projectiles);
    }
}

fn spawn_projectiles(keys: Res<Input<KeyCode>>, mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    if keys.just_pressed(KeyCode::P) {
        commands.spawn((
            Projectile{ movement_type : MovementType::Linear},
            Collider::ball(0.1),
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {radius : 0.1, ..default()})),
                material: materials.add(Color::rgb(0., 1., 0.).into()),
                transform: Transform::from_xyz(0., 1., 0.5),
                ..default()
            },
            Velocity(Vec3::new(0., -0.2, 0.))
        ));
    }

}

fn update_projectiles(time: Res<Time>, mut projectiles: Query<(&mut Transform, &Velocity, &Projectile)>) {
    for mut p in &mut projectiles {
        match p.2.movement_type {
            MovementType::Linear => p.0.translation += p.1.0 * time.delta_seconds(),
            MovementType::Static => (),
        }
    }
}

fn detect_projectile_collisions(mut commands: Commands, mut collisions: EventReader<CollisionStarted>, projectiles: Query<&Projectile>) {
    for CollisionStarted(e1, e2) in collisions.read() {
        match projectiles.get(*e1) {
            Ok(p) => handle_projectile_collision(&mut commands, p, e1, e2),
            Err(_) => (),
        }
        match projectiles.get(*e2) {
            Ok(p) => handle_projectile_collision(&mut commands, p, e2, e1),
            Err(_) => (),
        }
    }
}

fn handle_projectile_collision(commands: &mut Commands, projectile: &Projectile, p_entity: &Entity, contact: &Entity) {
    println!("Collision with projectile {:#?}", projectile);
    commands.entity(*p_entity).despawn();
}
