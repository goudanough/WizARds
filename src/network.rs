use std::net::SocketAddr;

use bevy::{prelude::*, utils::HashMap};
use bevy_ggrs::{ggrs::UdpNonBlockingSocket, prelude::*, LocalInputs, LocalPlayers};
use bevy_oxr::xr_input::{
    hands::{common::HandsResource, HandBone},
    trackers::{OpenXRLeftEye, OpenXRRightEye, OpenXRTracker},
};
use bevy_xpbd_3d::prelude::*;
use bevy_hanabi::prelude::*;
use crate::{player, spell_control::QueuedSpell, PhysLayer, PlayerInput, WizGgrsConfig, FPS};

#[derive(States, Debug, Hash, Eq, PartialEq, Clone)]
enum NetworkingState {
    Uninitialized,
    HostWaiting,
    ClientWaiting,
    InitGgrs,
    Done,
}

#[derive(Component)]
pub struct PlayerID {
    pub handle: usize,
}

#[derive(Resource)]
pub struct LocalPlayerID {
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
        app.add_plugins(GgrsPlugin::<WizGgrsConfig>::default())
            // define frequency of rollback game logic update
            .set_rollback_schedule_fps(FPS)
            .rollback_component_with_clone::<Transform>()
            // TODO add components that need rollback
            // TODO remove these systems and have players be instantiated in a different plugin
            .insert_state(NetworkingState::Uninitialized)
            .add_systems(Startup, init)
            .add_systems(
                Update,
                host_wait.run_if(in_state(NetworkingState::HostWaiting)),
            )
            .add_systems(OnExit(NetworkingState::HostWaiting), host_inform_clients)
            .add_systems(
                Update,
                client_wait.run_if(in_state(NetworkingState::ClientWaiting)),
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
    state.0 = Some(NetworkingState::HostWaiting);
}

fn host_wait(mut state: ResMut<NextState<NetworkingState>>) {
    // Here we'll need to create some multicast address and listen for
    // clients that want to join the game.
    // Ideally we establish TCP connections to each client.
    state.0 = Some(NetworkingState::InitGgrs);
}

fn host_inform_clients() {
    // Here we'll need to send some information back to every client over our
    // established TCP connection. This involves:
    // - The IP + port of every client
    // - The anchor point that all clients are coordinate themselves around
}

fn client_wait(mut state: ResMut<NextState<NetworkingState>>) {
    // Here we'll need to send packets to some multicast address
    // and wait for the host to attempt to establish TCP connection
    state.0 = Some(NetworkingState::InitGgrs);
}

fn init_ggrs(mut commands: Commands, mut state: ResMut<NextState<NetworkingState>>) {
    // Once everyone has information about the clients that are going to be playing
    // We can go ahead and configure and start our Ggrs session

    // TODO currently networking is hard coded, need to be able to select ips and port after game starts
    let args = ConnectionArgs {
        local_port: 8000,
        players: vec![
            "localhost".to_owned(), /*"192.168.66.202:8000".to_owned()*/
        ],
    };
    assert!(!args.players.is_empty());

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
            commands.insert_resource(LocalPlayerID { handle: i });
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

fn spawn_networked_player_objs(mut commands: Commands, args: Res<ConnectionArgs>,mut effects: ResMut<Assets<EffectAsset>>) {
    // Add one cube on each player's head
    for i in 0..args.players.len() {
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
                PlayerHead,
                player::Player,
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
        let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(1.0, Vec2::splat(0.));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.5)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.8).uniform(writer.lit(1.2)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add constant downward acceleration to simulate gravity
    let accel = writer.lit(Vec3::Y * -8.).expr();
    let update_accel = AccelModifier::new(accel);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(5.).expr();
    let update_drag = LinearDragModifier::new(drag);

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(2.).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Give a bit of variation by randomizing the initial speed
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: (writer.rand(ScalarType::Float) * writer.lit(0.0002)).expr(),
    };

    let effect = EffectAsset::new(
        32768,
        Spawner::new(20000.0.into(), 0.5.into(), 0.5.into()),
        writer.finish(),
    )
    .with_name("firework")
    .init(init_pos)
    .init(init_vel)
    .init(init_age)
    .init(init_lifetime)
    .update(update_drag)
    // .update(update_accel)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient1,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient1,
        screen_space_size: false,
    });

    let effect1 = effects.add(effect);
        
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
                PlayerRightPalm,
            ))
            .add_rollback();
        
            commands.spawn((
                Name::new("firework"),
                ParticleEffectBundle {
                    effect: ParticleEffect::new(effect1),
                    transform: Transform::IDENTITY,
                    ..Default::default()
                },
            ));
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
