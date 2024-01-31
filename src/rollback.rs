use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{ggrs::UdpNonBlockingSocket, prelude::*, LocalInputs, LocalPlayers};
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye};
// use clap::Parser;
use std::net::SocketAddr;

use crate::{PlayerInput, RollbackConfig, FPS, NUM_PLAYERS};

#[derive(Component)]
struct PlayerObj {
    handle: usize,
}

// #[derive(Parser, Resource)]
struct ConnectionArgs {
    // #[clap(short, long)]
    local_port: u16,
    // #[clap(short, long, num_args = 1..)]
    players: Vec<String>,
}
pub struct RollbackPlugin;

impl Plugin for RollbackPlugin {
    fn build(&self, app: &mut App) {
        let args = ConnectionArgs {
            local_port: 8001,
            players: vec!["localhost".to_owned(), "10.42.0.95:8000".to_owned()],
        };
        // create a GGRS session
        let mut sess_build = SessionBuilder::<RollbackConfig>::new().with_num_players(NUM_PLAYERS);
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

        app.add_plugins(GgrsPlugin::<RollbackConfig>::default())
            // define frequency of rollback game logic update
            .set_rollback_schedule_fps(FPS)
            // this system will be executed as part of input reading
            .add_systems(ReadInputs, read_local_inputs)
            .rollback_component_with_clone::<Transform>()
            // .insert_resource(args)
            .add_systems(Startup, setup_system)
            // these systems will be executed as part of the advance frame update
            .add_systems(GgrsSchedule, move_models_system)
            // add your GGRS session
            .insert_resource(Session::P2P(sess));
    }
}

fn setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add one cube on each player's head
    for i in 0..NUM_PLAYERS {
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                    material: materials.add(Color::WHITE.into()),
                    ..Default::default()
                },
                PlayerObj { handle: i },
            ))
            .add_rollback();
    }
}

fn read_local_inputs(
    mut commands: Commands,
    left_eye: Query<&Transform, With<OpenXRLeftEye>>,
    right_eye: Query<&Transform, With<OpenXRRightEye>>,
    // left_hand: Query<
    //     &Transform,
    //     (
    //         With<OpenXRLeftController>,
    //         With<OpenXRController>,
    //         With<OpenXRTracker>,
    //     ),
    // >,
    // right_hand: Query<
    //     &Transform,
    //     (
    //         With<OpenXRRightController>,
    //         With<OpenXRController>,
    //         With<OpenXRTracker>,
    //     ),
    // >,
    local_player: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();
    // let left_hand = left_hand.get_single().unwrap();
    // let right_hand = right_hand.get_single().unwrap();
    let player = local_player.0.first().unwrap();
    let head_pos = left_eye.translation.lerp(right_eye.translation, 0.5);
    let head_rot = left_eye.rotation;
    local_inputs.insert(
        *player,
        PlayerInput {
            head_pos,
            head_rot,
            ..Default::default()
        },
    );
    commands.insert_resource(LocalInputs::<RollbackConfig>(local_inputs));
}

fn move_models_system(
    mut player_heads: Query<(&mut Transform, &PlayerObj), With<Rollback>>,
    inputs: Res<PlayerInputs<RollbackConfig>>,
) {
    for (mut t, p) in player_heads.iter_mut() {
        let input = inputs[p.handle].0;
        t.translation = input.head_pos;
        t.rotation = input.head_rot;
    }
}
