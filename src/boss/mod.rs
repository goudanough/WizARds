mod boss_attack;
mod boss_state;

use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use bevy_ggrs::AddRollbackCommandExtension;
use bevy_xpbd_3d::prelude::*;

use self::{
    boss_attack::{boss_attack, AttackTimer},
    boss_state::{boss_action, boss_move, BossState},
};
use crate::{
    assets::{AssetHandles, MatName, MeshName},
    player::Player,
    projectile::DamageMask,
    PhysLayer,
};

#[derive(Component)]
pub struct BossHealth {
    pub max: f32,
    pub current: f32,
    pub damage_mask: DamageMask,
}

// This implementation of phases is gross.
#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default, Copy)]
pub enum BossPhase {
    #[default]
    Phase1,
    Phase2,
    Phase3,
    TwoPylon,
    Dead,
    Reset,
}

impl BossPhase {
    pub fn max_health(self) -> f32 {
        match self {
            Self::Phase1 => 50.,
            Self::Phase2 => 50.,
            Self::Phase3 => 50.,
            Self::TwoPylon => 10.,
            Self::Dead => 0.,
            Self::Reset => 0.,
        }
    }

    pub fn next_phase(self) -> Self {
        match self {
            BossPhase::Phase1 => Self::TwoPylon,
            BossPhase::Phase2 => Self::Phase3,
            BossPhase::Phase3 => Self::Dead,
            BossPhase::TwoPylon => Self::Phase2,
            BossPhase::Dead => Self::Dead,
            BossPhase::Reset => Self::Reset,
        }
    }
}

#[derive(Component)]
pub struct Pylon {
    pub destroyed: bool,
    pub respawn_timer: Timer,
    pub damage_mask: DamageMask,
}

#[derive(Resource)]
pub struct CurrentPhase(pub BossPhase);

impl BossHealth {
    pub fn normalized_value(&self) -> f32 {
        self.current / self.max
    }
}

#[derive(Component)]
pub struct Boss;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BossState>()
            .init_state::<BossPhase>()
            .insert_resource(CurrentPhase(BossPhase::Phase1))
            .add_systems(Startup, setup)
            .insert_resource(AttackTimer(Timer::from_seconds(5.0, TimerMode::Repeating)))
            .add_systems(
                Update,
                (
                    update_boss,
                    boss_action,
                    boss_attack.run_if(in_state(BossState::Attack)),
                    boss_move.run_if(in_state(BossState::MoveTowardsPlayer)),
                    check_phase,
                    handle_pylons.run_if(in_state(BossPhase::TwoPylon)),
                ),
            )
            .add_systems(OnEnter(BossPhase::Phase2), init_phase2)
            .add_systems(OnEnter(BossPhase::Phase3), init_phase3)
            .add_systems(OnEnter(BossPhase::TwoPylon), init_two_pylon_phase)
            .add_systems(OnEnter(BossPhase::Reset), reset_phase)
            .add_systems(OnEnter(BossPhase::Dead), despawn_boss);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let model = asset_server.load("white bear.glb#Scene0");

    let initial_mask: DamageMask = DamageMask(DamageMask::FIRE.0 | DamageMask::LIGHTNING.0);

    commands.spawn((
        SceneBundle {
            scene: model,
            transform: Transform::from_xyz(0.0, 0.4, 0.0).with_scale(Vec3::new(1.0, 2.5, 1.0)),
            ..default()
        },
        RigidBody::Kinematic,
        Collider::cuboid(0.25, 0.25, 0.25),
        CollisionLayers::new(PhysLayer::Boss, LayerMask::ALL ^ PhysLayer::BossProjectile),
        Boss,
        BossHealth {
            max: BossPhase::Phase1.max_health(),
            current: BossPhase::Phase1.max_health(),
            damage_mask: initial_mask,
        },
    ));
}

// boss look at player
fn update_boss(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    if let Some(player_transform) = player_query.iter().next() {
        let Ok(mut boss_transform) = query.get_single_mut() else {
            return;
        };
        let mut player_pos_flat = player_transform.translation;
        player_pos_flat.y = boss_transform.translation.y;

        let direction = player_pos_flat - boss_transform.translation;
        if direction != Vec3::ZERO {
            let look_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

            let left_rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);

            boss_transform.rotation = look_rotation * left_rotation;
        }
    }
}

fn init_phase2(mut boss_health: Query<&mut BossHealth>, mut current_phase: ResMut<CurrentPhase>) {
    println!("Enter Phase 2.");
    let Ok(mut health) = boss_health.get_single_mut() else {
        return;
    };

    health.current = current_phase.0.max_health();
    health.max = current_phase.0.max_health();

    // TODO This is dumb, but I don't have time to think of a better way to do this, do better in future.
    current_phase.0 = BossPhase::Phase2;
}

fn init_phase3(mut boss_health: Query<&mut BossHealth>, mut current_phase: ResMut<CurrentPhase>) {
    println!("Enter Phase 3.");
    let Ok(mut health) = boss_health.get_single_mut() else {
        return;
    };

    current_phase.0 = BossPhase::Phase3;

    health.damage_mask = DamageMask::LIGHTNING;
    health.current = current_phase.0.max_health();
    health.max = current_phase.0.max_health();

    // TODO This is dumb, but I don't have time to think of a better way to do this, do better in future.
}

fn init_two_pylon_phase(
    mut commands: Commands,
    mut boss_health: Query<&mut BossHealth>,
    mut current_phase: ResMut<CurrentPhase>,
    asset_handles: Res<AssetHandles>,
) {
    let Ok(mut health) = boss_health.get_single_mut() else {
        return;
    };
    current_phase.0 = BossPhase::TwoPylon;

    health.current = current_phase.0.max_health();
    health.max = current_phase.0.max_health();
    health.damage_mask = DamageMask::IMMUNE;

    commands
        .spawn((
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Green as usize].clone(),
                transform: Transform::from_translation(Vec3::new(1.0, 0.0, 1.0))
                    .with_scale(2.0 * Vec3::ONE),
                ..Default::default()
            },
            Collider::sphere(1.0),
            CollisionLayers::new(PhysLayer::Boss, LayerMask::ALL ^ PhysLayer::BossProjectile),
            Pylon {
                destroyed: false,
                respawn_timer: Timer::new(Duration::from_secs(3), TimerMode::Repeating),
                damage_mask: DamageMask::FIRE,
            },
        ))
        .add_rollback();

    commands
        .spawn((
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Green as usize].clone(),
                transform: Transform::from_translation(Vec3::new(-1.0, 0.0, -1.0))
                    .with_scale(2.0 * Vec3::ONE),
                ..Default::default()
            },
            Collider::sphere(1.0),
            CollisionLayers::new(PhysLayer::Boss, LayerMask::ALL ^ PhysLayer::BossProjectile),
            Pylon {
                destroyed: false,
                respawn_timer: Timer::new(Duration::from_secs(3), TimerMode::Repeating),
                damage_mask: DamageMask::FIRE,
            },
        ))
        .add_rollback();
}

fn handle_pylons(
    mut commands: Commands,
    mut boss_health: Query<&mut BossHealth>,
    time: Res<Time>,
    mut pylon_query: Query<(Entity, &mut Visibility, &mut Pylon)>,
) {
    let mut pylon_entites_to_despawn = Vec::new();

    for (pylon_entity, mut pylon_visibility, mut pylon_status) in pylon_query.iter_mut() {
        if pylon_status.destroyed {
            pylon_status.respawn_timer.tick(time.delta());
            if pylon_status.respawn_timer.just_finished() {
                *pylon_visibility = Visibility::Visible;
                pylon_status.destroyed = false;
            } else {
                pylon_entites_to_despawn.push(pylon_entity);
            }
        }
    }

    // only despawn if both are defeated at the same time
    if pylon_entites_to_despawn.len() == 2 {
        for pylon_entity in pylon_entites_to_despawn.iter() {
            commands.entity(*pylon_entity).despawn();
        }

        if let Ok(mut health) = boss_health.get_single_mut() {
            health.damage_mask = DamageMask(DamageMask::FIRE.0 | DamageMask::LIGHTNING.0);
        }
    }
}

// TODO A phase that just goes back to the start of the current phase seems dumb, do it in a better way.
fn reset_phase(current_phase: Res<CurrentPhase>, mut next_phase: ResMut<NextState<BossPhase>>) {
    println!("Resetting Phase.");
    next_phase.set(current_phase.0);
}

fn check_phase(
    current_phase: Res<CurrentPhase>,
    mut next_phase: ResMut<NextState<BossPhase>>,
    boss_health: Query<&BossHealth>,
) {
    let Ok(health) = boss_health.get_single() else {
        return;
    };
    if health.current <= 0.0 {
        next_phase.set(current_phase.0.next_phase());
    }
}

fn despawn_boss(mut commands: Commands, mut boss: Query<(Entity, &Transform), With<Boss>>) {
    let Ok((boss_e, t)) = boss.get_single_mut() else {
        return;
    };

    commands.spawn(TransformBundle {
        local: Transform::clone(t).with_rotation(Quat::from_axis_angle(Vec3::Y, (2.0 * PI) / 2.0)),
        ..default()
    });

    commands.entity(boss_e).despawn_recursive();
}
