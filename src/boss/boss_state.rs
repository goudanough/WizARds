use bevy::{
    ecs::{
        query::{With, Without},
        schedule::{NextState, States},
        system::{Query, Res, ResMut},
    },
    time::Time,
    transform::components::Transform,
};

use crate::player::Player;

use super::Boss;

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum BossState {
    #[default]
    Idle,
    MoveTowardsPlayer,
    Attack,
}

pub fn boss_action(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    mut state: ResMut<NextState<BossState>>,
) {
    let player_transform = player_query.single();
    let boss_transform = query.single_mut();

    let distance = player_transform
        .translation
        .distance(boss_transform.translation);

    // change boss state depend on distance
    if distance > 20.0 {
        state.set(BossState::Idle);
    } else if distance > 10.0 {
        state.set(BossState::MoveTowardsPlayer);
    } else if distance >= 0.0 {
        state.set(BossState::Attack);
    }
}

pub fn boss_move(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    time: Res<Time>,
) {
    let player_transform = player_query.single();
    let mut boss_transform = query.single_mut();

    let direction = player_transform.translation - boss_transform.translation;
    let direction = direction.normalize();

    boss_transform.translation += direction * 1.0 * time.delta_seconds();
}