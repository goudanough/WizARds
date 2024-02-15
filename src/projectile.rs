use bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_xpbd_3d::prelude::*;

use crate::network::{debug_move_networked_player_objs, PlayerObj};

#[derive(Debug, Default)]
enum ProjectileMovement {
    #[default]
    Linear,
}
#[derive(Debug)]
enum ProjectileEffect {
    Damage(u16),
}
impl Default for ProjectileEffect {
    fn default() -> Self {
        ProjectileEffect::Damage(10)
    }
}

#[derive(Debug, Default)]
enum ProjectileVisual {
    #[default]
    None,
}

#[allow(dead_code)]
#[derive(Component, Debug, Default)]
pub struct Projectile {
    movement: ProjectileMovement,
    effect: ProjectileEffect,
    visual: ProjectileVisual,
}

#[derive(Component)]
struct Velocity(Vec3);

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GgrsSchedule,
            (
                update_projectiles.ambiguous_with(debug_move_networked_player_objs), // TODO this is a hack, make it work without the hack.
                detect_projectile_collisions,
            )
                .chain(),
        );
    }
}

fn update_projectiles(
    time: Res<Time>,
    mut projectiles: Query<(&mut Transform, &Velocity, &Projectile)>,
) {
    for mut p in &mut projectiles {
        match p.2.movement {
            ProjectileMovement::Linear => p.0.translation += p.1 .0 * time.delta_seconds(),
        }
    }
}

fn detect_projectile_collisions(
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    projectiles: Query<&Projectile>,
) {
    for CollisionStarted(e1, e2) in collisions.read() {
        if let Ok(p) = projectiles.get(*e1) {
            handle_projectile_collision(&mut commands, p, e1, e2);
        }
        if let Ok(p) = projectiles.get(*e2) {
            handle_projectile_collision(&mut commands, p, e2, e1);
        }
    }
}

fn handle_projectile_collision(
    commands: &mut Commands,
    projectile: &Projectile,
    p_entity: &Entity,
    contact: &Entity,
) {
    println!("Collision with projectile {:#?}", projectile);
    println!("{:?} {:?}", p_entity, contact);
    commands.entity(*p_entity).despawn();
}

pub fn spawn_projectile(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
    collider: Collider,
    direction: Vec3,
    speed: f32,
    projectile: Projectile,
) {
    commands
        .spawn((
            projectile,
            collider,
            PbrBundle {
                mesh,
                material,
                transform,
                ..default()
            },
            RigidBody::Kinematic,
            Velocity(direction.normalize() * speed),
        ))
        .add_rollback();
}
