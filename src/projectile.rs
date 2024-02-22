use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_xpbd_3d::prelude::*;

use crate::{
    assets::{AssetHandles, MatName, MeshName},
    boss::{BossHealth, BossPhase, CurrentPhase},
    network::{move_networked_player_objs, PlayerID},
    PhysLayer,
};

pub enum ProjectileType {
    Fireball,
    LightningBolt,
    BossAttack,
}

#[derive(Debug, Default, Component)]
pub struct LinearMovement(f32);

#[derive(Debug, Component)]
enum ProjectileHitEffect {
    Damage(DamageMask, f32),
    ResetPhase,
}
impl Default for ProjectileHitEffect {
    fn default() -> Self {
        ProjectileHitEffect::Damage(DamageMask::FIRE, 10.)
    }
}

#[derive(Component, Debug, Default)]
pub struct Projectile;

#[derive(Debug)]
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

// TODO Make this better - function signature changes as we add more types of projectile which is bad.
// instead of this we should create an bundle representing each collision and have these be processed by different systems.
fn detect_projectile_collisions(
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    projectiles: Query<(&ProjectileHitEffect, &Transform)>,
    mut healths: Query<&mut BossHealth>,
    players: Query<&PlayerID>,
    mut next_phase: ResMut<NextState<BossPhase>>,
    current_phase: Res<CurrentPhase>,
) {
    for CollisionStarted(e1, e2) in collisions.read() {
        if let Ok((p, _)) = projectiles.get(*e1) {
            handle_projectile_collision(
                &mut commands,
                p,
                e1,
                healths.get_mut(*e2),
                players.get(*e2),
                &mut next_phase,
                &current_phase,
            );
        }
        if let Ok((p, _)) = projectiles.get(*e2) {
            handle_projectile_collision(
                &mut commands,
                p,
                e2,
                healths.get_mut(*e1),
                players.get(*e1),
                &mut next_phase,
                &current_phase,
            );
        }
    }
}

// TODO see previous TODO
fn handle_projectile_collision(
    commands: &mut Commands,
    hit_effect: &ProjectileHitEffect,
    p_entity: &Entity,
    health: Result<Mut<BossHealth>, QueryEntityError>,
    player: Result<&PlayerID, QueryEntityError>,
    next_phase: &mut ResMut<NextState<BossPhase>>,
    current_phase: &Res<CurrentPhase>,
) {
    commands.entity(*p_entity).despawn();
    match &hit_effect {
        ProjectileHitEffect::Damage(m, a) => {
            if let Ok(mut h) = health {
                if h.damage_mask.intersect(m) {
                    h.current -= a;
                    if h.current <= 0.0 {
                        println!("Change phase");
                        next_phase.set(current_phase.0.next_phase());
                    }
                }
            }
        }
        ProjectileHitEffect::ResetPhase => {
            if player.is_ok() {
                println!("Reset phase");
                next_phase.set(BossPhase::Reset);
            }
        }
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
                ProjectileHitEffect::Damage(DamageMask::FIRE, 25.0),
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
                ProjectileHitEffect::Damage(DamageMask::LIGHTNING, 25.0),
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
                ProjectileHitEffect::ResetPhase,
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
