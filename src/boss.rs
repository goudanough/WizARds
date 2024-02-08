mod boss_attack;
mod boss_state;
use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use crate::player::Player;

use self::{
    boss_attack::{dog_update, init_dog},
    boss_state::{boss_action, BossState},
};

const BOSS_MAX_HEALTH: f32 = 100.0;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BossState>().add_systems(Startup, (setup, init_dog)).add_systems(
            Update,
            (
                update_boss,
                boss_action,
                dog_update,
                boss_attack::spawn_and_launch_dog.run_if(in_state(BossState::Attack)),
                boss_state::boss_move.run_if(in_state(BossState::MoveTowardsPlayer)),
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
    let font = asset_server.load("fonts/fira_mono.ttf");

    commands.spawn((
        SceneBundle {
            scene: model,
            transform: Transform::from_xyz(0.0, 1.0, 15.0).with_scale(Vec3::new(2.0, 2.0, 2.0)),

            ..default()
        },
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        Boss,
        Health(BOSS_MAX_HEALTH),
    ));

    commands.spawn(TextBundle {
        text: Text {
            sections: vec![TextSection {
                value: "Boss Health: 100/100".to_string(),
                style: TextStyle {
                    font,
                    font_size: 32.0,
                    color: Color::RED,
                },
            }],
            ..default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        ..default()
    });
}

// boss look at player
fn update_boss(
    mut query: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    if let Ok(player_transform) = player_query.get_single() {
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