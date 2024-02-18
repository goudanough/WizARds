use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use crate::{
    assets::AssetHandles,
    player::Player,
    projectile::{spawn_projectile, ProjectileType},
};

use super::Boss;

#[derive(Resource)]
pub struct AttackTimer(pub Timer);

pub fn boss_attack(
    mut timer: ResMut<AttackTimer>,
    mut commands: Commands,
    assets: Res<AssetHandles>,
    boss_query: Query<&Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    time: Res<Time>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let boss_transform = boss_query.single();
        let Some(player_transform) = player_query.iter().next() else {
            return;
        };

        let player_pos = Vec3::new(
            player_transform.translation.x,
            0.0,
            player_transform.translation.z,
        );
        let boss_pos = Vec3::new(
            boss_transform.translation.x,
            0.0,
            boss_transform.translation.z,
        );
        let projectile_direction = (player_pos - boss_pos).normalize();
        let projectile_start = boss_transform.translation - boss_transform.right() * 2.0;

        let transform = Transform {
            translation: projectile_start,
            rotation: Quat::from_rotation_arc(-Vec3::Z, projectile_direction),
            ..default()
        };

        spawn_projectile(
            &mut commands,
            ProjectileType::BossAttack,
            &transform,
            &assets,
        )
    }
}
