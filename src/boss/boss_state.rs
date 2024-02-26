use bevy::{
    ecs::{
        entity::Entity, query::{With, Without}, 
        schedule::{NextState, States}, 
        system::{In, Query, Res, ResMut}
    },
    time::Time,
    transform::components::Transform,
};

use super::{Boss, BossPhase, CurrentPhase, Follow};
use crate::player::Player;


pub fn boss_action(
    boss_query: Query<&Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    phase: Res<CurrentPhase>,
)->Option<bool> {
    let player_transform = player_query.iter().next()?;
    
    let boss_transform= boss_query.get_single().ok()?;

    let distance = player_transform
        .translation
        .distance(boss_transform.translation);

    // change boss state depend on distance
    // TODO this is bad, i should collpase boss state and boss phase behaviour together, but time.
    if distance > 10.0 {
        None
    } else if distance > 7.0 {
        if phase.0 == BossPhase::Phase1 {
            None
        } else {
            Some(false)
        }
    } else if distance >= 0.0 {
        if phase.0 == BossPhase::Phase1 {
            None
        } else {
            Some(true)
        }
    }else{
        None
    
    }
}

pub fn boss_move(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    time: Res<Time>,
    follow:Query<&Follow>,
) {
    for _ in &follow{
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

}
