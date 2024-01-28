use bevy::prelude::*;

const BOSS_MAX_HEALTH: f32 = 100.0;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup);
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