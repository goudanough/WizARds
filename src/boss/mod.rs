mod boss_attack;
mod boss_state;

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use self::{
    boss_attack::{boss_attack, AttackTimer},
    boss_state::{boss_action, boss_move, BossState},
};
use crate::{player::Player, projectile::DamageMask, PhysLayer};

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
    Dead,
    Reset,
}

impl BossPhase {
    pub fn max_health(self) -> f32 {
        match self {
            Self::Phase1 => 50.,
            Self::Phase2 => 50.,
            Self::Phase3 => 50.,
            Self::Dead => 0.,
            Self::Reset => 0.,
        }
    }

    pub fn next_phase(self) -> Self {
        match self {
            BossPhase::Phase1 => Self::Phase2,
            BossPhase::Phase2 => Self::Phase3,
            BossPhase::Phase3 => Self::Dead,
            BossPhase::Dead => Self::Dead,
            BossPhase::Reset => Self::Reset,
        }
    }
}

#[derive(Resource)]
pub struct CurrentPhase(pub BossPhase);

impl BossHealth {
    pub fn normalized_value(&self) -> f32 {
        self.current / self.max
    }
}

#[derive(Component)]
struct Boss;

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
                ),
            )
            .add_systems(OnEnter(BossPhase::Phase2), init_phase2)
            .add_systems(OnEnter(BossPhase::Phase3), init_phase3)
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
        Sensor,
        Collider::cuboid(0.25, 0.25, 0.25),
        ActiveEvents::COLLISION_EVENTS,
        ActiveCollisionTypes::STATIC_STATIC,
        CollisionGroups {
            memberships: PhysLayer::Boss.into(),
            filters: Group::all().difference(PhysLayer::BossProj.into()),
        },
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

// TODO A phase that just goes back to the start of the current phase seems dumb, do it in a better way.
fn reset_phase(current_phase: Res<CurrentPhase>, mut next_phase: ResMut<NextState<BossPhase>>) {
    println!("Resetting Phase.");
    next_phase.set(current_phase.0);
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
