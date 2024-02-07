use std::default;

use bevy::{input::keyboard::KeyboardInput, prelude::*, render::mesh, transform};
use bevy_xpbd_3d::prelude::*;

#[derive(Debug, Default)]
enum ProjectileMovement {
    #[default]
    Linear,
    Static,
}
#[derive(Debug)]
enum ProjectileEffect {
    Damage(u16)
}
 impl Default for ProjectileEffect {
    fn default() -> Self {
        ProjectileEffect::Damage((10))
    }
 }

#[derive(Debug, Default)]
enum ProjectileVisual {
    #[default]
    None
}

#[derive(Component, Debug, Default)]
struct Projectile {
    movement: ProjectileMovement,
    effect: ProjectileEffect,
    visual: ProjectileVisual
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
        let mesh = meshes.add(Mesh::from(shape::UVSphere{radius: 0.05, ..default()}));
        let material = materials.add(Color::rgb(1.0, 0., 0.).into());
        let collider = Collider::ball(0.05);
        let transform = Transform::from_xyz(0., 1., 0.);
        let direction = Vec3::new(0., -1., 0.);
        let speed = 5.;

        spawn_projectile(commands, mesh, material, transform, collider, direction, speed, Default::default());
    }

}

fn update_projectiles(time: Res<Time>, mut projectiles: Query<(&mut Transform, &Velocity, &Projectile)>) {
    for mut p in &mut projectiles {
        match p.2.movement {
            ProjectileMovement::Linear => p.0.translation += p.1.0 * time.delta_seconds(),
            ProjectileMovement::Static => (),
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
 
fn spawn_projectile(mut commands: Commands,
                    mesh: Handle<Mesh>, 
                    material: Handle<StandardMaterial>,
                    transform: Transform,
                    collider: Collider,
                    direction: Vec3,
                    speed: f32,
                    projectile: Projectile) {
    commands.spawn((
        projectile,
        collider,
        PbrBundle {
            mesh: mesh,
            material: material,
            transform: transform,
            ..default()
        },
        Velocity(direction.normalize() * speed)));
}