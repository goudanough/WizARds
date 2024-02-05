use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{ggrs::UdpNonBlockingSocket, prelude::*, LocalInputs, LocalPlayers};
use bevy_oxr::xr_input::{
    hands::{common::HandsResource, HandBone},
    trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker},
};
use std::net::SocketAddr;

use crate::{PlayerInput, WizGgrsConfig, FPS};

#[derive(Component)]
pub struct PlayerObj {
    pub handle: usize,
}

#[derive(Component)]
pub struct PlayerHead;
#[derive(Component)]
pub struct PlayerLeftPalm;
#[derive(Component)]
pub struct PlayerRightPalm;

#[derive(Resource)]
struct ConnectionArgs {
    local_port: u16,
    players: Vec<String>,
}
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        // TODO currently networking is hard coded, need to be able to select ips and port after game starts
        let args = ConnectionArgs {
            local_port: 8000,
            players: vec!["localhost".to_owned(), "192.168.137.195:8001".to_owned()],
        };
        assert!(args.players.len() > 0);
        // create a GGRS session
        let mut sess_build =
            SessionBuilder::<WizGgrsConfig>::new().with_num_players(args.players.len());
        // .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 10 }) // (optional) set how often to exchange state checksums
        // .with_max_prediction_window(12).expect("prediction window can't be 0") // (optional) set max prediction window
        // .with_input_delay(2); // (optional) set input delay for the local player

        // add players
        for (i, player_addr) in args.players.iter().enumerate() {
            // local player
            if player_addr == "localhost" {
                sess_build = sess_build.add_player(PlayerType::Local, i).unwrap();
            } else {
                // remote players
                let remote_addr: SocketAddr = player_addr.parse().unwrap();
                sess_build = sess_build
                    .add_player(PlayerType::Remote(remote_addr), i)
                    .unwrap();
            }
        }

        // start the GGRS session
        let socket = UdpNonBlockingSocket::bind_to_port(args.local_port).unwrap();
        let sess = sess_build.start_p2p_session(socket).unwrap();

        app.add_plugins(GgrsPlugin::<WizGgrsConfig>::default())
            // add network info as a bevy resource
            .insert_resource(args)
            // define frequency of rollback game logic update
            .set_rollback_schedule_fps(FPS)
            .add_systems(ReadInputs, read_local_inputs)
            .rollback_component_with_clone::<Transform>()
            // TODO add further components that need rollback
            // add your GGRS session
            .insert_resource(Session::P2P(sess))
            // TODO remove these systems and have players be instantiated in a different plugin
            .add_systems(Startup, debug_spawn_networked_player_objs)
            .add_systems(GgrsSchedule, debug_move_networked_player_objs);
    }
}

fn read_local_inputs(
    mut commands: Commands,
    left_eye: Query<&Transform, With<OpenXRLeftEye>>,
    right_eye: Query<&Transform, With<OpenXRRightEye>>,
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    local_player: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();
    let left_hand = hand_bones.get(hands_resource.left.palm).unwrap();
    let right_hand = hand_bones.get(hands_resource.right.palm).unwrap();
    let player = local_player.0.first().unwrap();
    local_inputs.insert(
        *player,
        PlayerInput {
            head_pos: left_eye.translation.lerp(right_eye.translation, 0.5),
            head_rot: left_eye.rotation,
            left_hand_pos: left_hand.translation,
            right_hand_pos: right_hand.translation,
            left_hand_rot: left_hand.rotation,
            right_hand_rot: right_hand.rotation,
            spell: 0, // TODO set spell using spell system
            ..Default::default()
        },
    );
    commands.insert_resource(LocalInputs::<WizGgrsConfig>(local_inputs));
}

fn debug_spawn_networked_player_objs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    args: Res<ConnectionArgs>,
) {
    // Add one cube on each player's head
    for i in 0..args.players.len() {
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
                    material: materials.add(Color::WHITE.into()),
                    ..Default::default()
                },
                PlayerObj { handle: i },
                PlayerHead,
            ))
            .add_rollback();
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                    material: materials.add(Color::WHITE.into()),
                    ..Default::default()
                },
                PlayerObj { handle: i },
                PlayerLeftPalm,
            ))
            .add_rollback();
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                    material: materials.add(Color::WHITE.into()),
                    ..Default::default()
                },
                PlayerObj { handle: i },
                PlayerRightPalm,
            ))
            .add_rollback();
    }
}

fn debug_move_networked_player_objs(
    mut player_heads: Query<
        (&mut Transform, &PlayerObj),
        (
            With<PlayerHead>,
            Without<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    mut player_left_palms: Query<
        (&mut Transform, &PlayerObj),
        (
            Without<PlayerHead>,
            With<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    mut player_right_palms: Query<
        (&mut Transform, &PlayerObj),
        (
            Without<PlayerHead>,
            Without<PlayerLeftPalm>,
            With<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
) {
    for (mut t, p) in player_heads.iter_mut() {
        let input = inputs[p.handle].0;
        t.translation = input.head_pos;
        t.rotation = input.head_rot;
    }
    for (mut t, p) in player_left_palms.iter_mut() {
        let input = inputs[p.handle].0;
        t.translation = input.left_hand_pos;
        t.rotation = input.left_hand_rot;
    }
    for (mut t, p) in player_right_palms.iter_mut() {
        let input = inputs[p.handle].0;
        t.translation = input.right_hand_pos;
        t.rotation = input.right_hand_rot;
    }
}