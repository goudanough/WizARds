use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use crate::{player::Player, projectile::spawn_projectile};

use super::Boss;

#[derive(Resource)]
pub struct AttackTimer(pub Timer);

pub fn boss_attack(
    mut timer: ResMut<AttackTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

        let boss_forward = boss_transform.forward();
        let boss_left = Vec3::new(-boss_forward.z, 0.0, boss_forward.x);
        let projectile_start = boss_transform.translation + boss_left * 2.0;

        let mesh = meshes.add(Mesh::from(shape::UVSphere {
            radius: 0.3,
            ..default()
        }));
        let material = materials.add(Color::PURPLE);
        let transform =
            Transform::from_xyz(projectile_start.x, projectile_start.y, projectile_start.z);
        let collider = Collider::ball(0.3);
        let direction = projectile_direction;
        let speed = 2.;

        spawn_projectile(
            &mut commands,
            mesh,
            material,
            transform,
            collider,
            direction,
            speed,
            default(),
        )
    }
}
