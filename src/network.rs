use self::{
    client_init::{client_await_data, client_establish_tcp, client_sync_anchor},
    host_init::{
        host_establish_connections, host_inform_clients, host_share_anchor, host_wait_share_anchor,
    },
};
#[cfg(target_os = "android")]
use crate::speech::{fetch_recogniser, SpeechRecognizer};
use crate::{
    player,
    speech::{RecognizedWord, RecordingStatus},
    spell_control::QueuedSpell,
    PhysLayer, PlayerInput, WizGgrsConfig, FPS,
};
use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{ggrs::UdpNonBlockingSocket, prelude::*, LocalInputs, LocalPlayers};
use bevy_oxr::xr_input::{
    hands::{common::HandsResource, HandBone},
    trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker},
};
use bevy_xpbd_3d::prelude::*;
use std::net::{IpAddr, SocketAddr};

mod client_init;
mod host_init;
mod multicast;

// This series of states is used to represent what stage of device discovery we're in
#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone)]
pub enum NetworkingState {
    #[default]
    HostClientMenu,
    HostEstablishConnections,
    HostSendData,
    ClientEstablishConnection,
    ClientWaitForData,
    InitGgrs,
    Done,
}

pub const LOCAL_PLAYER_HNDL: usize = 0;

#[derive(Component)]
pub struct PlayerID {
    // TODO currently this is not rollbacked to support local player always being zero, hopefully will not cause issues
    pub handle: usize,
}

#[derive(Component)]
pub struct PlayerHead;
#[derive(Component)]
pub struct PlayerLeftPalm;
#[derive(Component)]
pub struct PlayerRightPalm;

// Why this number? I asked a random number generator!
const GGRS_PORT: u16 = 47511;

// Needs to be configured by both host and client during initialization for use in init_ggrs
#[derive(Resource)]
struct RemoteAddresses(Vec<IpAddr>);

#[cfg(target_os = "android")]
const MENU_GRAMMAR: [&str; 2] = ["host", "join"];

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GgrsPlugin::<WizGgrsConfig>::default())
            // define frequency of rollback game logic update
            .set_rollback_schedule_fps(FPS)
            .rollback_component_with_clone::<Transform>()
            .insert_resource(RemoteAddresses(Vec::new()))
            .init_state::<NetworkingState>()
            // On startup we need to allow a user to choose whether they want to host or join
            .add_systems(Startup, init)
            .add_systems(
                OnEnter(RecordingStatus::Success),
                menu_select.run_if(in_state(NetworkingState::HostClientMenu)),
            )
            // If the player chooses to host, we need to scan the room and open a multicast listener
            .add_systems(
                OnEnter(NetworkingState::HostEstablishConnections),
                host_init::host_init,
            )
            // We loop, creating TCP streams to clients that want to join, and recording addresses
            .add_systems(
                Update,
                host_establish_connections
                    .run_if(in_state(NetworkingState::HostEstablishConnections)),
            )
            // We tell the host to share the anchor with the clients
            .add_systems(OnEnter(NetworkingState::HostSendData), host_share_anchor)
            // We wait until the host has finished sharing the anchor with the clients
            .add_systems(
                Update,
                host_wait_share_anchor.run_if(in_state(NetworkingState::HostSendData)),
            )
            // When all clients are joined, we need to tell each client the IPs of all clients
            .add_systems(OnExit(NetworkingState::HostSendData), host_inform_clients)
            // If we choose to join, initialize networking ready to send multicast packets for device discovery
            .add_systems(
                OnEnter(NetworkingState::ClientEstablishConnection),
                client_init::client_init,
            )
            // We loop, sending multicast packets for device discovery and waiting for our tcp listener
            // to accept an incoming connection from the host
            .add_systems(
                Update,
                client_establish_tcp.run_if(in_state(NetworkingState::ClientEstablishConnection)),
            )
            // Waiting for an anchor ID and a list of IPs
            .add_systems(
                Update,
                client_await_data.run_if(in_state(NetworkingState::ClientWaitForData)),
            )
            // Await an anchor
            .add_systems(
                OnExit(NetworkingState::ClientWaitForData),
                client_sync_anchor,
            )
            // All setup is done. Initialize the GGRS session
            .add_systems(OnEnter(NetworkingState::InitGgrs), init_ggrs)
            .add_systems(OnEnter(NetworkingState::Done), spawn_networked_player_objs)
            .add_systems(ReadInputs, read_local_inputs)
            .add_systems(GgrsSchedule, move_networked_player_objs);
    }
}

#[cfg(target_os = "android")]
fn init(mut commands: Commands) {
    commands.insert_resource(SpeechRecognizer(fetch_recogniser(&MENU_GRAMMAR)));
}

// Devices that aren't the quest 3 should *only* be able to act as clients
#[cfg(not(target_os = "android"))]

fn init(mut state: ResMut<NextState<NetworkingState>>) {
    state.set(NetworkingState::ClientEstablishConnection);
}

fn menu_select(word: Res<RecognizedWord>, mut state: ResMut<NextState<NetworkingState>>) {
    match &*word.0 {
        "host" => {
            println!("Hosting session");
            state.set(NetworkingState::HostEstablishConnections)
        }
        "join" => {
            println!("Joining session");
            state.set(NetworkingState::ClientEstablishConnection)
        }
        _ => {}
    };
}

fn init_ggrs(
    mut commands: Commands,
    mut state: ResMut<NextState<NetworkingState>>,
    addresses: Res<RemoteAddresses>,
) {
    // Once everyone has information about the clients that are going to be playing
    // We can go ahead and configure and start our Ggrs session

    let addresses = &addresses.0;

    // create a GGRS session
    let mut sess_build = SessionBuilder::<WizGgrsConfig>::new()
        .with_num_players(addresses.len() + 1)
        .add_player(PlayerType::Local, LOCAL_PLAYER_HNDL)
        .unwrap();
    // .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 10 }) // (optional) set how often to exchange state checksums
    // .with_max_prediction_window(12).expect("prediction window can't be 0") // (optional) set max prediction window
    // .with_input_delay(2); // (optional) set input delay for the local player

    // add players
    for (i, player_addr) in addresses.iter().enumerate() {
        // remote players
        let remote_addr: SocketAddr = (*player_addr, GGRS_PORT).into();
        sess_build = sess_build
            .add_player(PlayerType::Remote(remote_addr), i + 1)
            .unwrap();
    }

    // start the GGRS session
    let socket = UdpNonBlockingSocket::bind_to_port(GGRS_PORT).unwrap();
    let sess = sess_build.start_p2p_session(socket).unwrap();

    // add your GGRS session
    commands.insert_resource(Session::P2P(sess));
    state.set(NetworkingState::Done);
}

pub fn read_local_inputs(
    mut commands: Commands,
    left_eye: Query<&Transform, With<OpenXRLeftEye>>,
    right_eye: Query<&Transform, With<OpenXRRightEye>>,
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    local_player: Res<LocalPlayers>,
    mut queued_spell: ResMut<QueuedSpell>,
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
            spell: queued_spell.0.map(|s| s as u32).unwrap_or(0),
            ..Default::default()
        },
    );
    commands.insert_resource(LocalInputs::<WizGgrsConfig>(local_inputs));
    queued_spell.0 = None;
}

fn spawn_networked_player_objs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    addresses: Res<RemoteAddresses>,
) {
    // Add one cube on each player's head
    for i in 0..addresses.0.len() + 1 {
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::sphere(0.1),
                CollisionLayers::new(
                    PhysLayer::Player,
                    LayerMask::ALL ^ PhysLayer::PlayerProjectile,
                ),
                // TransformBundle { ..default() },
                PlayerID { handle: i },
                PlayerHead,
                player::Player,
                PbrBundle {
                    mesh: meshes.add(Cuboid::new(0.2, 0.2, 0.2)),
                    material: materials.add(Color::SILVER),
                    ..default()
                },
            ))
            .add_rollback();
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::sphere(0.1),
                CollisionLayers::new(
                    PhysLayer::Player,
                    LayerMask::ALL ^ PhysLayer::PlayerProjectile,
                ),
                TransformBundle { ..default() },
                PlayerID { handle: i },
                PlayerLeftPalm,
            ))
            .add_rollback();
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::sphere(0.1),
                CollisionLayers::new(
                    PhysLayer::Player,
                    LayerMask::ALL ^ PhysLayer::PlayerProjectile,
                ),
                TransformBundle { ..default() },
                PlayerID { handle: i },
                PlayerRightPalm,
            ))
            .add_rollback();
    }
}

pub fn move_networked_player_objs(
    mut player_heads: Query<
        (&mut Transform, &PlayerID),
        (
            With<PlayerHead>,
            Without<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    mut player_left_palms: Query<
        (&mut Transform, &PlayerID),
        (
            Without<PlayerHead>,
            With<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    mut player_right_palms: Query<
        (&mut Transform, &PlayerID),
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
