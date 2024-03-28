use bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_xpbd_3d::prelude::*;

use crate::{
    assets::{AssetHandles, MatName, MeshName},
    boss::{BossHealth, BossPhase, Pylon},
    network::{move_networked_player_objs, PlayerID},
    PhysLayer,
};

pub enum ProjectileType {
    Fireball,
    LightningBolt,
    BossAttack,
}

#[derive(Component)]
struct ProjectileHit(Entity);

#[derive(Component, Debug, Clone)]
pub struct DamageHit(pub DamageMask, pub f32);

#[derive(Component, Debug, Clone)]
pub struct ResetPhaseHit;
#[derive(Debug, Default, Component)]
pub struct LinearMovement(f32);

#[derive(Debug, Component, Clone)]
pub enum ProjectileHitEffect {
    Damage(DamageHit),
    ResetPhase(ResetPhaseHit),
}
impl Default for ProjectileHitEffect {
    fn default() -> Self {
        ProjectileHitEffect::Damage(DamageHit(DamageMask::FIRE, 10.))
    }
}

#[derive(Component, Debug, Default)]
pub struct Projectile;

// DamageMask struct used for handling damage types.
// Each bit is a damage type, a bit is set to 1 if that type is enabled.
// Things that deal damage should have a damage mask with the damage types they deal enabled.
// Things that take damage should have a damage mask with the damage types they can take enabled.
#[derive(Debug, Clone)]
pub struct DamageMask(pub u8);

impl DamageMask {
    pub const FIRE: Self = DamageMask(1 << 0);
    pub const LIGHTNING: Self = DamageMask(1 << 1);
    pub const IMMUNE: Self = DamageMask(1 << 1);

    pub fn intersect(&self, other: &Self) -> bool {
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
        // All Projectile code needs to run on the GgrsSchedule.
        app.add_systems(
            GgrsSchedule,
            (
                update_linear_movement.ambiguous_with(move_networked_player_objs), // TODO this might be a hack, but also might be how bevy_ggrs works
                detect_projectile_collisions,
                handle_damage_hits,
                handle_reset_phase_hits,
            )
                .chain(),
        );
    }
}

// Move projectiles forward manually.
pub fn update_linear_movement(
    time: Res<Time>,
    mut projectiles: Query<(&mut Transform, &LinearMovement), Without<PlayerID>>,
) {
    for (mut t, s) in projectiles.iter_mut() {
        let forward = t.forward();
        t.translation += forward * s.0 * time.delta_seconds();
    }
}

// Check for collisions between projectiles and other objects, and emit entities to represent these "hits".
fn detect_projectile_collisions(
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    projectiles: Query<(&ProjectileHitEffect, &Transform)>,
) {
    let mut entities_to_despawn: Vec<Entity> = Vec::new();
    for CollisionStarted(e1, e2) in collisions.read() {
        if let Ok((p, t)) = projectiles.get(*e1) {
            match p {
                ProjectileHitEffect::Damage(damage_hit) => {
                    commands
                        .spawn((ProjectileHit(*e2), *t, damage_hit.clone()))
                        .add_rollback();
                }
                ProjectileHitEffect::ResetPhase(reset_phase_hit) => {
                    commands
                        .spawn((ProjectileHit(*e2), *t, reset_phase_hit.clone()))
                        .add_rollback();
                }
            }
            entities_to_despawn.push(*e1);
        }
        if let Ok((p, t)) = projectiles.get(*e2) {
            match p {
                ProjectileHitEffect::Damage(damage_hit) => {
                    commands
                        .spawn((ProjectileHit(*e1), *t, damage_hit.clone()))
                        .add_rollback();
                }
                ProjectileHitEffect::ResetPhase(reset_phase_hit) => {
                    commands
                        .spawn((ProjectileHit(*e1), *t, reset_phase_hit.clone()))
                        .add_rollback();
                }
            }
            entities_to_despawn.push(*e2);
        }
    }

    for entity in entities_to_despawn {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).despawn();
        }
    }
}

// Handle hits from player projectiles that deal damage.
fn handle_damage_hits(
    mut commands: Commands,
    hits: Query<(&Transform, &ProjectileHit, Entity, &DamageHit)>,
    mut pylons: Query<(Entity, &mut Visibility, &mut Pylon)>,
    mut boss_health: Query<&mut BossHealth>,
) {
    for (_transform, p_hit, e, d) in hits.iter() {
        // Check if the collided entity is the boss.
        if let Ok(mut h) = boss_health.get_mut(p_hit.0) {
            // If the damage masks intersect, then the boss doesn't resist the attack.
            if h.damage_mask.intersect(&d.0) {
                h.current -= d.1;
            }
        }
        for (pylon_entity, mut pylon_visibility, mut pylon_status) in pylons.iter_mut() {
            if pylon_entity == p_hit.0
                && pylon_status.damage_mask.intersect(&d.0)
                && !pylon_status.destroyed
            {
                pylon_status.destroyed = true;
                *pylon_visibility = Visibility::Hidden;
            }
        }
        commands.entity(e).despawn();
    }
}

// Handle hits from boss projectiles.
fn handle_reset_phase_hits(
    mut commands: Commands,
    hits: Query<(&Transform, &ProjectileHit, Entity), With<ResetPhaseHit>>,
    mut next_phase: ResMut<NextState<BossPhase>>,
    players: Query<&PlayerID>,
) {
    for (_transform, p_hit, e) in hits.iter() {
        // Check if the collided entity is part of a player.
        if players.get(p_hit.0).is_ok() {
            next_phase.set(BossPhase::Reset)
        }
        commands.entity(e).despawn();
    }
}

// Given a projectile type, spawn a corresponding projectile. Sort of prefabing.
pub fn spawn_projectile(
    commands: &mut Commands,
    projectile_type: ProjectileType,
    spell_transform: &Transform,
    asset_handles: &Res<AssetHandles>,
) {
    match projectile_type {
        ProjectileType::Fireball => commands
            .spawn((
                Projectile,
                PbrBundle {
                    mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                    material: asset_handles.mats[MatName::Red as usize].clone(),
                    transform: *spell_transform,
                    ..Default::default()
                },
                LinearMovement(3.0),
                ProjectileHitEffect::Damage(DamageHit(DamageMask::FIRE, 25.0)),
                CollisionLayers::new(
                    PhysLayer::PlayerProjectile,
                    ((LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile)
                        ^ PhysLayer::ParryObject,
                ),
                Collider::sphere(0.1),
                RigidBody::Kinematic,
            ))
            .add_rollback(),
        ProjectileType::LightningBolt => commands
            .spawn((
                Projectile,
                PbrBundle {
                    mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                    material: asset_handles.mats[MatName::Blue as usize].clone(),
                    transform: *spell_transform,
                    ..Default::default()
                },
                LinearMovement(6.0),
                ProjectileHitEffect::Damage(DamageHit(DamageMask::LIGHTNING, 25.0)),
                CollisionLayers::new(
                    PhysLayer::PlayerProjectile,
                    ((LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile)
                        ^ PhysLayer::ParryObject,
                ),
                Collider::sphere(0.1),
                RigidBody::Kinematic,
            ))
            .add_rollback(),
        ProjectileType::BossAttack => commands
            .spawn((
                Projectile,
                PbrBundle {
                    mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                    material: asset_handles.mats[MatName::Purple as usize].clone(),
                    transform: spell_transform.with_scale(1.2 * Vec3::ONE),
                    ..Default::default()
                },
                LinearMovement(1.0),
                ProjectileHitEffect::ResetPhase(ResetPhaseHit),
                CollisionLayers::new(
                    PhysLayer::BossProjectile,
                    (LayerMask::ALL ^ PhysLayer::Boss) ^ PhysLayer::PlayerProjectile, // ugh
                ),
                Collider::sphere(0.2),
                RigidBody::Kinematic,
            ))
            .add_rollback(),
    };
}
