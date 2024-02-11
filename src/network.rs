use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{ggrs::UdpNonBlockingSocket, prelude::*, LocalInputs, LocalPlayers};
use bevy_oxr::{
    xr::sys::SpaceUserFB,
    xr_input::{
        hands::{common::HandsResource, HandBone},
        trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker},
    },
    XrEvents,
};
use bevy_xpbd_3d::prelude::*;
use std::{
    io::{self, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    str::FromStr,
};

use crate::{
    player, spell_control::QueuedSpell, xr::scene::SceneState, PhysLayer, PlayerInput,
    WizGgrsConfig, FPS,
};

use self::multicast::{MulticastEmitter, MulticastListener};

mod multicast;

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone)]
enum NetworkingState {
    #[default]
    Uninitialized,
    HostWaiting,
    ClientEstablishConnection,
    ClientWaitForData,
    InitGgrs,
    Done,
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone)]
enum AwaitingAnchor {
    #[default]
    Uninitialized,
    Awaiting,
    Done,
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone)]
enum AwaitingIps {
    #[default]
    Uninitialized,
    Awaiting,
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

#[derive(Resource)]
struct ConnectionArgs {
    local_port: u16,
    remote_players: Vec<IpAddr>,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GgrsPlugin::<WizGgrsConfig>::default())
            // define frequency of rollback game logic update
            .set_rollback_schedule_fps(FPS)
            // TODO add components that need rollback
            .rollback_component_with_clone::<Transform>()
            .insert_resource(RemoteAddresses(Vec::new()))
            .init_state::<NetworkingState>()
            .init_state::<AwaitingAnchor>()
            .init_state::<AwaitingIps>()
            // On startup we need to allow a user to choose whether they want to host or join
            .add_systems(Startup, init)
            // If the host enters a waiting state, they need to scan the room and open a listener
            .add_systems(OnEnter(NetworkingState::HostWaiting), host_init)
            // We loop, creating TCP streams to clients that want to join and recording addresses
            .add_systems(
                Update,
                host_wait.run_if(in_state(NetworkingState::HostWaiting)),
            )
            // When all clients are joined, we need to tell each client the IPs of all clients
            .add_systems(OnExit(NetworkingState::HostWaiting), host_inform_clients)
            // When all clients are joined, we need to tell each client the IPs of all clients
            .add_systems(OnExit(NetworkingState::HostWaiting), host_share_anchor)
            // If we choose to join, then we start sending multicast packets for host discovery
            .add_systems(
                OnEnter(NetworkingState::ClientEstablishConnection),
                client_init,
            )
            // We wait for our tcp listener to accept an incoming connection from the host
            .add_systems(
                Update,
                client_establish_tcp.run_if(in_state(NetworkingState::ClientEstablishConnection)),
            )
            // Start waiting for an anchor and a list of IPs
            .add_systems(
                OnEnter(NetworkingState::ClientWaitForData),
                client_start_await_information,
            )
            // Await an anchor
            .add_systems(
                Update,
                client_await_anchor.run_if(in_state(AwaitingAnchor::Awaiting)),
            )
            // Await a list of IPs
            .add_systems(
                Update,
                client_await_ips.run_if(in_state(AwaitingIps::Awaiting)),
            )
            // If we get an anchor we may be done, check to see
            .add_systems(
                OnEnter(AwaitingAnchor::Done),
                client_check_end_await_information,
            )
            // If we get a list of IPs we may be done, check to see
            .add_systems(
                OnEnter(AwaitingIps::Done),
                client_check_end_await_information,
            )
            .add_systems(OnEnter(NetworkingState::InitGgrs), init_ggrs)
            .add_systems(OnEnter(NetworkingState::Done), spawn_networked_player_objs)
            .add_systems(ReadInputs, read_local_inputs)
            .add_systems(GgrsSchedule, move_networked_player_objs);
    }
}

fn init(mut state: ResMut<NextState<NetworkingState>>) {
    // Here we'll need to create some prompt on startup
    // This will allow users to select whether they're going to be acting
    // as the host or a client that will be joining the game
    #[cfg(target_os = "android")]
    {
        state.0 = Some(NetworkingState::ClientEstablishConnection);
    }

    // Devices that aren't the quest 3 should *only* be able to act as clients
    #[cfg(not(target_os = "android"))]
    {
        state.0 = Some(NetworkingState::ClientEstablishConnection);
    }
}

#[derive(Resource)]
struct MulticastListenerRes(MulticastListener);

#[derive(Resource)]
struct RemoteAddresses(Vec<IpAddr>);

#[derive(Resource)]
struct ClientConnections(Vec<TcpStream>);

#[derive(Resource)]
struct FbIds(Vec<u64>);

// Create a multicast listener and insert it as a resource
fn host_init(mut commands: Commands, mut state: ResMut<NextState<SceneState>>) {
    println!("host_init");
    state.0 = Some(SceneState::Scanning);
    let listener = MulticastListener::new();
    commands.insert_resource(MulticastListenerRes(listener));
    commands.insert_resource(ClientConnections(Vec::new()));
    commands.insert_resource(FbIds(Vec::new()));
}

// Handle any incoming UDP packets that have reached us through multicast
fn host_wait(
    mut state: ResMut<NextState<NetworkingState>>,
    mut commands: Commands,
    listener: Res<MulticastListenerRes>,
    mut addresses: ResMut<RemoteAddresses>,
    mut connections: ResMut<ClientConnections>,
    mut fb_ids: ResMut<FbIds>,
) {
    println!("host_wait");
    while let Some((msg, addr)) = listener.0.get_buf() {
        if addresses.0.contains(&addr.ip()) {
            continue;
        }
        println!("got packet: {}", std::str::from_utf8(&msg).unwrap());
        let Some((port, fb_id)) = multicast::decode(msg) else {
            continue;
        };
        let stream = TcpStream::connect((addr.ip(), port)).unwrap();
        stream.set_nonblocking(true).unwrap();
        fb_ids.0.push(fb_id);
        addresses.0.push(addr.ip());
        connections.0.push(stream);

        // Currently hardcoded to exit on 1 remote client
        if addresses.0.len() == 1 {
            // Drop the listener, we don't need it anymore
            commands.remove_resource::<MulticastListenerRes>();
            state.0 = Some(NetworkingState::InitGgrs);
            return;
        }
    }
}

fn host_share_anchor() {
    println!("host_share_anchor");
}

// Runs once all connections are complete
// Informs all clients about every IP that will be participating
fn host_inform_clients(
    mut commands: Commands,
    addresses: Res<RemoteAddresses>,
    connections: Res<ClientConnections>,
) {
    println!("host_inform_clients");

    for mut conn in &connections.0 {
        use std::fmt::Write;
        let mut buf = String::new();
        for addr in &addresses.0 {
            // Make sure we don't tell any client about itself
            if conn.peer_addr().unwrap().ip() != *addr {
                // For the sake of simplicity all IPs are sent as null-seperated strings
                write!(buf, "{addr}\0").unwrap();
            }
        }

        conn.write(buf.as_bytes()).unwrap();
    }

    // Drop all the open tcp streams
    commands.remove_resource::<ClientConnections>();
}

#[derive(Resource)]
struct MulticastEmitterRes(MulticastEmitter);

fn client_init(mut commands: Commands) {
    println!("client_init");
    let emitter = MulticastEmitter::new(SpaceUserFB::NULL);
    commands.insert_resource(MulticastEmitterRes(emitter));
}

#[derive(Resource)]
struct HostConnection(TcpStream);

fn client_establish_tcp(
    mut state: ResMut<NextState<NetworkingState>>,
    mut commands: Commands,
    emit: Res<MulticastEmitterRes>,
) {
    println!("client_establish_tcp");
    let emit = &emit.0;

    // First we do a listen to see if we've got any incoming connections
    if let Some((stream, _)) = emit.accept() {
        commands.insert_resource(HostConnection(stream));
        state.0 = Some(NetworkingState::ClientWaitForData);
        return;
    } else {
        // If we there are no incoming requests then we emit a new multicast message
        // TODO: put this on a timer
        emit.emit();
    }
}

// Start awaiting both anchors and IPs
fn client_start_await_information(
    mut anchors: ResMut<NextState<AwaitingAnchor>>,
    mut ips: ResMut<NextState<AwaitingIps>>,
) {
    println!("client_start_await_information");
    anchors.0 = Some(AwaitingAnchor::Awaiting);
    ips.0 = Some(AwaitingIps::Awaiting);
}

#[cfg(target_os = "android")]
fn client_await_anchor(mut anchors: ResMut<NextState<AwaitingAnchor>>, events: NonSend<XrEvents>) {
    println!("client_await_anchor");
    // for event in &events.0 {
    //     let event = unsafe { bevy_oxr::xr::Event::from_raw(&(*event).inner) }.unwrap();
    //     if let bevy_oxr::xr::Event::SpaceShareCompleteFB(res) = event {
    //         // TODO: whatever the fuck is supposed to go here
    //         anchors.0 = Some(AwaitingAnchor::Done);
    //         return;
    //     }
    // }
    anchors.0 = Some(AwaitingAnchor::Done);
}

// We don't generate XrEvents in pancake mode. Move along swiftly.
#[cfg(not(target_os = "android"))]
fn client_await_anchor(mut anchors: ResMut<NextState<AwaitingAnchor>>) {
    println!("client_await_anchor");
    anchors.0 = Some(AwaitingAnchor::Done);
}

fn client_await_ips(
    mut commands: Commands,
    mut state: ResMut<NextState<AwaitingIps>>,
    mut stream: ResMut<HostConnection>,
) {
    println!("client_await_ips");
    let mut buf = Vec::new();

    match stream.0.read_to_end(&mut buf) {
        Ok(len) => {
            let mut ips = buf[..len]
                .split(|chr| *chr == 0)
                .filter_map(|slice| IpAddr::from_str(std::str::from_utf8(slice).ok()?).ok())
                .collect::<Vec<_>>();
            ips.push(stream.0.peer_addr().unwrap().ip());
            commands.insert_resource(RemoteAddresses(ips));
            commands.remove_resource::<HostConnection>();
            state.0 = Some(AwaitingIps::Done);
        }
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => return,
        Err(err) if err.kind() == io::ErrorKind::ConnectionReset => return,
        Err(err) => panic!("{err:?} on {:?}", stream.0),
    };
}

// Start awaiting both anchors and IPs
fn client_check_end_await_information(
    mut state: ResMut<NextState<NetworkingState>>,
    anchors: ResMut<State<AwaitingAnchor>>,
    ips: ResMut<State<AwaitingIps>>,
) {
    println!("client_check_end_await_information");
    if (*anchors.get() == AwaitingAnchor::Done) && (*ips.get() == AwaitingIps::Done) {
        state.0 = Some(NetworkingState::InitGgrs);
    }
}

fn init_ggrs(
    mut commands: Commands,
    mut state: ResMut<NextState<NetworkingState>>,
    addresses: Res<RemoteAddresses>,
) {
    println!("init_ggrs");
    // Once everyone has information about the clients that are going to be playing
    // We can go ahead and configure and start our Ggrs session

    // TODO currently networking is hard coded, need to be able to select ips and port after game starts
    let args = ConnectionArgs {
        local_port: 8000,
        remote_players: addresses.0.clone(),
    };
    assert!(args.remote_players.len() > 0);

    // create a GGRS session
    let mut sess_build = SessionBuilder::<WizGgrsConfig>::new()
        .with_num_players(args.remote_players.len() + 1)
        .add_player(PlayerType::Local, 0)
        .unwrap();
    // .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 10 }) // (optional) set how often to exchange state checksums
    // .with_max_prediction_window(12).expect("prediction window can't be 0") // (optional) set max prediction window
    // .with_input_delay(2); // (optional) set input delay for the local player

    // add players
    for (i, player_addr) in args.remote_players.iter().enumerate() {
        // remote players
        let remote_addr: SocketAddr = (*player_addr, 8000).into();
        sess_build = sess_build
            .add_player(PlayerType::Remote(remote_addr), i + 1)
            .unwrap();
    }

    // start the GGRS session
    let socket = UdpNonBlockingSocket::bind_to_port(args.local_port).unwrap();
    let sess = sess_build.start_p2p_session(socket).unwrap();

    // add network info as a bevy resource
    commands.insert_resource(args);

    // add your GGRS session
    commands.insert_resource(Session::P2P(sess));
    state.0 = Some(NetworkingState::Done);
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
    args: Res<ConnectionArgs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add one cube on each player's head
    for i in 0..args.remote_players.len() + 1 {
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::ball(0.1),
                CollisionLayers::all_masks::<PhysLayer>()
                    .add_group(PhysLayer::Player)
                    .remove_mask(PhysLayer::PlayerProjectile),
                // TransformBundle { ..default() },
                PlayerID { handle: i },
                PlayerHead,
                player::Player,
                PbrBundle {
                    mesh: meshes.add(shape::Box::new(0.2, 0.2, 0.2)),
                    material: materials.add(Color::SILVER),
                    ..default()
                },
            ))
            .add_rollback();
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::ball(0.1),
                CollisionLayers::all_masks::<PhysLayer>()
                    .add_group(PhysLayer::Player)
                    .remove_mask(PhysLayer::PlayerProjectile),
                TransformBundle { ..default() },
                PlayerID { handle: i },
                PlayerLeftPalm,
            ))
            .add_rollback();
        commands
            .spawn((
                RigidBody::Kinematic,
                Collider::ball(0.1),
                CollisionLayers::all_masks::<PhysLayer>()
                    .add_group(PhysLayer::Player)
                    .remove_mask(PhysLayer::PlayerProjectile),
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
