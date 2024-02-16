use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_oxr::xr::PerformanceMetricsCounterFlagsMETA;
use bevy_xpbd_3d::prelude::*;

use crate::{
    network::{debug_move_networked_player_objs, PlayerObj},
    spell_control::Spell,
    PhysLayer,
};

#[derive(Debug, Default, Component)]
struct LinearMovement(f32);

#[derive(Debug, Component)]
enum ProjectileHitEffect {
    Damage(DamageMask, f32),
}
impl Default for ProjectileHitEffect {
    fn default() -> Self {
        ProjectileHitEffect::Damage(DamageMask::FIRE, 10.)
    }
}

#[derive(Component)]
struct Health(DamageMask, f32);

#[derive(Component, Debug, Default)]
pub struct Projectile;

#[derive(Debug)]
pub struct DamageMask(u8);

impl DamageMask {
    const FIRE: Self = DamageMask(1 << 0);
    const LIGHTNING: Self = DamageMask(1 << 1);

    fn intersect(&self, other: &Self) -> bool {
        self.0 & other.0 > 0
    }
}

impl From<DamageMask> for u8 {
    fn from(val: DamageMask) -> Self {
        val.0
    }
}
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GgrsSchedule,
            (
                update_linear_movement.ambiguous_with(debug_move_networked_player_objs), // TODO this is a hack, make it work without the hack.
                detect_projectile_collisions,
            )
                .chain(),
        );
    }
}

fn update_linear_movement(
    time: Res<Time>,
    mut projectiles: Query<(&mut Transform, &LinearMovement), Without<PlayerObj>>,
) {
    for (mut t, mut s) in &mut projectiles {
        t.translation += t.forward() * s.0 * time.delta_seconds();
    }
}

fn detect_projectile_collisions(
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    projectiles: Query<&ProjectileHitEffect>,
    mut healths: Query<&mut Health>,
) {
    for CollisionStarted(e1, e2) in collisions.read() {
        if let Ok(p) = projectiles.get(*e1) {
            handle_projectile_collision(&mut commands, p, e1, healths.get_mut(*e2));
        }
        if let Ok(p) = projectiles.get(*e2) {
            handle_projectile_collision(&mut commands, p, e2, healths.get_mut(*e1));
        }
    }
}

fn handle_projectile_collision(
    commands: &mut Commands,
    hit_effect: &ProjectileHitEffect,
    p_entity: &Entity,
    health: Result<Mut<Health>, QueryEntityError>,
) {
    commands.entity(*p_entity).despawn();
    if let ProjectileHitEffect::Damage(m, a) = &hit_effect {
        if let Ok(mut h) = health {
            if h.0.intersect(&m) {
                h.1 -= a;
            }
        }
    }
}

pub fn spawn_spell_projectile(commands: &mut Commands, spell: &Spell) {
    // TODO add PbrBundle for projectiles, this will need:
    //    - a mesh and material, meaning this function will need access to mesh and material resources.
    //    - transform information for the projectile, this will also have to be passed in.
    match spell {
        Spell::Fireball => commands
            .spawn((
                Projectile,
                LinearMovement(5.),
                ProjectileHitEffect::Damage(DamageMask::FIRE, 10.),
                CollisionLayers::all_masks::<PhysLayer>()
                    .add_group(PhysLayer::PlayerProjectile)
                    .remove_mask(PhysLayer::Player),
                Collider::ball(0.03),
                RigidBody::Kinematic,
            ))
            .add_rollback(),
        Spell::Lightning => commands
            .spawn((
                Projectile,
                LinearMovement(5.),
                ProjectileHitEffect::Damage(DamageMask::LIGHTNING, 10.),
                CollisionLayers::all_masks::<PhysLayer>()
                    .add_group(PhysLayer::PlayerProjectile)
                    .remove_mask(PhysLayer::Player),
                Collider::ball(0.03),
                RigidBody::Kinematic,
            ))
            .add_rollback(),
    }
}
