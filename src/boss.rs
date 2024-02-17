mod boss_attack;
mod boss_state;
use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use crate::player::Player;

use self::{
    boss_attack::{boss_attack, AttackTimer},
    boss_state::{boss_action, boss_move, BossState},
};

const BOSS_MAX_HEALTH: f32 = 100.0;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BossState>()
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
            );
    }
}

#[derive(Component)]
struct Boss;

#[derive(Component)]
struct Health(f32);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let model = asset_server.load("white bear.glb#Scene0");

    commands.spawn((
        SceneBundle {
            scene: model,
            transform: Transform::from_xyz(0.0, 1.0, 9.0).with_scale(Vec3::new(2.0, 2.0, 2.0)),

            ..default()
        },
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        Boss,
        Health(BOSS_MAX_HEALTH),
    ));
}

// boss look at player
fn update_boss(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    if let Some(player_transform) = player_query.iter().next() {
        let mut boss_transform = query.single_mut();

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
