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

use super::{Boss, BossPhase, CurrentPhase};

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
    phase: Res<CurrentPhase>,
) {
    let Some(player_transform) = player_query.iter().next() else {
        return;
    };
    let Ok(boss_transform) = query.get_single_mut() else {
        return;
    };

    let distance = player_transform
        .translation
        .distance(boss_transform.translation);

    // change boss state depend on distance
    // TODO this is bad, i should collpase boss state and boss phase behaviour together, but time.
    if distance > 10.0 {
        state.set(BossState::Idle);
    } else if distance > 7.0 {
        if phase.0 == BossPhase::Phase1 {
            state.set(BossState::Idle);
        } else {
            state.set(BossState::MoveTowardsPlayer);
        }
    } else if distance >= 0.0 {
        if phase.0 == BossPhase::Phase1 {
            state.set(BossState::Idle);
        } else {
            state.set(BossState::Attack);
        }
    }
}

pub fn boss_move(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    time: Res<Time>,
) {
    let Some(player_transform) = player_query.iter().next() else {
        return;
    };
    let Ok(mut boss_transform) = query.get_single_mut() else {
        return;
    };

    let direction = player_transform.translation - boss_transform.translation;
    let direction = direction.normalize();

    boss_transform.translation += direction * 1.0 * time.delta_seconds();
}
