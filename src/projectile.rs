use bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_xpbd_3d::prelude::*;

use crate::{
    assets::{AssetHandles, MatName, MeshName},
    boss::{BossHealth, BossPhase},
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
struct DamageHit(DamageMask, f32);

#[derive(Component, Debug, Clone)]
struct BossHit;
#[derive(Debug, Default, Component)]
pub struct LinearMovement(f32);

#[derive(Debug, Component, Clone)]
enum ProjectileHitEffect {
    Damage(DamageHit),
    ResetPhase(BossHit),
}
impl Default for ProjectileHitEffect {
    fn default() -> Self {
        ProjectileHitEffect::Damage(DamageHit(DamageMask::FIRE, 10.))
    }
}

#[derive(Component, Debug, Default)]
pub struct Projectile;

#[derive(Debug, Clone)]
pub struct DamageMask(pub u8);

impl DamageMask {
    pub const FIRE: Self = DamageMask(1 << 0);
    pub const LIGHTNING: Self = DamageMask(1 << 1);

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
                update_linear_movement.ambiguous_with(move_networked_player_objs), // TODO this might be a hack, but also might be how bevy_ggrs works
                detect_projectile_collisions,
                handle_damage_hits,
                handle_boss_hits,
            )
                .chain(),
        );
    }
}

pub fn update_linear_movement(
    time: Res<Time>,
    mut projectiles: Query<(&mut Transform, &LinearMovement), Without<PlayerID>>,
) {
    for (mut t, s) in projectiles.iter_mut() {
        let forward = t.forward();
        t.translation += forward * s.0 * time.delta_seconds();
    }
}

fn detect_projectile_collisions(
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    projectiles: Query<(&ProjectileHitEffect, &Transform)>,
) {
    for CollisionStarted(e1, e2) in collisions.read() {
        if let Ok((p, t)) = projectiles.get(*e1) {
            match p {
                ProjectileHitEffect::Damage(damage_hit) => {
                    commands
                        .spawn((ProjectileHit(*e2), *t, damage_hit.clone()))
                        .add_rollback();
                }
                ProjectileHitEffect::ResetPhase(boss_hit) => {
                    commands
                        .spawn((ProjectileHit(*e2), *t, boss_hit.clone()))
                        .add_rollback();
                }
            }

            commands.entity(*e1).despawn();
        }
        if let Ok((p, t)) = projectiles.get(*e2) {
            match p {
                ProjectileHitEffect::Damage(damage_hit) => {
                    commands
                        .spawn((ProjectileHit(*e1), *t, damage_hit.clone()))
                        .add_rollback();
                }
                ProjectileHitEffect::ResetPhase(boss_hit) => {
                    commands
                        .spawn((ProjectileHit(*e1), *t, boss_hit.clone()))
                        .add_rollback();
                }
            }
            commands.entity(*e2).despawn();
        }
    }
}

fn handle_damage_hits(
    mut commands: Commands,
    hits: Query<(&Transform, &ProjectileHit, Entity, &DamageHit)>,
    mut boss_health: Query<&mut BossHealth>,
) {
    for (_transform, p_hit, e, d) in hits.iter() {
        if let Ok(mut h) = boss_health.get_mut(p_hit.0) {
            if h.damage_mask.intersect(&d.0) {
                h.current -= d.1;
            }
        }
        commands.entity(e).despawn();
    }
}

fn handle_boss_hits(
    mut commands: Commands,
    hits: Query<(&Transform, &ProjectileHit, Entity), With<BossHit>>,
    mut next_phase: ResMut<NextState<BossPhase>>,
    players: Query<&PlayerID>,
) {
    for (_transform, p_hit, e) in hits.iter() {
        if players.get(p_hit.0).is_ok() {
            next_phase.set(BossPhase::Reset)
        }
        commands.entity(e).despawn();
    }
}

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
                    (LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile,
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
                    (LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile,
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
                ProjectileHitEffect::ResetPhase(BossHit),
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
