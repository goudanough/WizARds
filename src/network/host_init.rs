use std::{io::Write, net::TcpStream, ptr::null};

use bevy::ecs::{
    schedule::NextState,
    system::{Commands, NonSend, Res, ResMut, Resource},
};
use bevy_oxr::{
    resources::{XrInstance, XrSession},
    xr::{
        self,
        sys::{SpaceShareInfoFB, SpaceUserCreateInfoFB, SpaceUserFB, UUID_SIZE_EXT},
        AsyncRequestIdFB, UuidEXT,
    },
    XrEvents,
};

use crate::{
    oxr,
    xr::{scene::get_supported_components, SceneState, SpatialAnchors},
};

use super::{
    multicast::{self, MulticastListener},
    NetworkingState, RemoteAddresses,
};

// Used for listening to incoming multicast packets
// created: host_init | dropped: host_wait
#[derive(Resource)]
pub(super) struct MulticastListenerRes(MulticastListener);

// Used for communicating addresses back to clients in host_inform_clients
// created: host_init | dropped: host_inform_clients
#[derive(Resource)]
pub(super) struct ClientConnections(Vec<TcpStream>);

// Used for sharing an anchor with other quest clients in host_share_anchor
// created: host_init | dropped: host_share_anchor
#[derive(Resource)]
pub(super) struct FbIds(Vec<SpaceUserFB>);

// Create a multicast listener and insert it as a resource
pub(super) fn host_init(mut commands: Commands, mut scan_state: ResMut<NextState<SceneState>>) {
    // Request a scene capture on hosting
    scan_state.set(SceneState::Scanning);

    let listener = MulticastListener::new();
    commands.insert_resource(MulticastListenerRes(listener));
    commands.insert_resource(ClientConnections(Vec::new()));
    commands.insert_resource(FbIds(Vec::new()));
}

// Handle any incoming UDP packets that have reached us through multicast
pub(super) fn host_establish_connections(
    mut state: ResMut<NextState<NetworkingState>>,
    mut commands: Commands,
    instance: Res<XrInstance>,
    session: Res<XrSession>,
    listener: Res<MulticastListenerRes>,
    mut addresses: ResMut<RemoteAddresses>,
    mut connections: ResMut<ClientConnections>,
    mut fb_ids: ResMut<FbIds>,
) {
    let (addresses, connections, fb_ids) = (&mut addresses.0, &mut connections.0, &mut fb_ids.0);
    let listener = &listener.0;

    // Loop over all the multicast packets that we've recieved
    while let Some((msg, addr)) = listener.get_buf() {
        // Ignore known addresses
        if addresses.contains(&addr.ip()) {
            continue;
        }

        // Attempt to decode the message into a TCP port and ID
        let Some((port, fb_id)) = multicast::decode(&msg) else {
            continue;
        };
        println!(
            "Got message {:?}, which decoded into (port: {port}, fb_id: {fb_id})",
            std::str::from_utf8(&msg)
        );

        //Log the IP of the incoming connection
        addresses.push(addr.ip());

        // Initialize a TCP connection to the client
        let stream = TcpStream::connect((addr.ip(), port)).unwrap();
        stream.set_nonblocking(true).unwrap();
        connections.push(stream);
        if fb_id != 0 {
            let vtable = instance.exts().fb_spatial_entity_user.unwrap();
            let info = SpaceUserCreateInfoFB {
                ty: SpaceUserCreateInfoFB::TYPE,
                next: null(),
                user_id: fb_id,
            };
            let mut user = SpaceUserFB::NULL;
            oxr!((vtable.create_space_user)(session.as_raw(), &info, &mut user));
            fb_ids.push(user);
        }

        // Currently hardcoded to exit on 1 remote client
        if addresses.len() == 1 {
            // Drop the listener, we don't need it anymore
            commands.remove_resource::<MulticastListenerRes>();
            state.set(NetworkingState::HostSendData);
            return;
        }
    }
}

// Runs once all connections are complete
// Allows clients to synchronise their game space
pub(super) fn host_share_anchor(
    mut commands: Commands,
    instance: Option<Res<XrInstance>>,
    session: Option<Res<XrSession>>,
    mut fb_ids: ResMut<FbIds>,
    anchors: Res<SpatialAnchors>,
) {
    let (Some(instance), Some(session)) = (instance, session) else {
        return;
    };
    let anchor = anchors.position;
    let fb_ids = &mut fb_ids.0;
    let vtable = instance.exts().fb_spatial_entity_sharing.unwrap();
    let mut anchors = [anchor];
    let info = SpaceShareInfoFB {
        ty: SpaceShareInfoFB::TYPE,
        next: null(),
        space_count: 1,
        spaces: anchors.as_mut_ptr(),
        user_count: fb_ids.len() as u32,
        users: fb_ids.as_mut_ptr(),
    };
    let mut request = AsyncRequestIdFB::default();
    oxr!((vtable.share_spaces)(session.as_raw(), &info, &mut request));
    println!("Sharing space {anchor:?} with users {fb_ids:?}");

    // Drop all the other user IDs
    commands.remove_resource::<FbIds>();
}

pub(super) fn host_wait_share_anchor(
    events: NonSend<XrEvents>,
    mut state: ResMut<NextState<NetworkingState>>,
) {
    for event in &events.0 {
        let event = unsafe { xr::Event::from_raw(&event.inner) }.unwrap();
        if let xr::Event::SpaceShareCompleteFB(_) = event {
            println!("Finished sharing anchor");
            state.set(NetworkingState::InitGgrs)
        }
    }
}

// Runs once all connections are complete
// Informs all clients about every IP that will be participating
pub(super) fn host_inform_clients(
    mut commands: Commands,
    addresses: Res<RemoteAddresses>,
    connections: Res<ClientConnections>,
    instance: Res<XrInstance>,
    anchors: Res<SpatialAnchors>,
) {
    let (addresses, connections) = (&addresses.0, &connections.0);

    let vtable = instance.exts().fb_spatial_entity.unwrap();
    let mut uuid = UuidEXT {
        data: <[u8; UUID_SIZE_EXT]>::default(),
    };
    oxr!((vtable.get_space_uuid)(anchors.mesh, &mut uuid));
    let uuid_num = u128::from_be_bytes(uuid.data);
    let msg_begin = format!("{uuid_num}");

    // Loop over each connection and send them the list of IPs
    for mut conn in connections {
        use std::fmt::Write;
        let mut buf = msg_begin.clone();
        for addr in addresses {
            // Make sure we don't tell any client about itself
            if conn.peer_addr().unwrap().ip() != *addr {
                // For the sake of simplicity all IPs are sent as null-seperated strings
                write!(buf, "\0{addr}").unwrap();
            }
        }

        conn.write_all(buf.as_bytes()).unwrap();
    }

    // Drop all the open tcp streams
    commands.remove_resource::<ClientConnections>();
}

pub fn host_wait_anchor_store(instance: Res<XrInstance>, events: NonSend<XrEvents>) {
    for event in &events.0 {
        let event = unsafe { xr::Event::from_raw(&event.inner) }.unwrap();
        if let xr::Event::SpaceSaveCompleteFB(res) = event {
            let enabled = get_supported_components(res.space(), instance.exts());
            panic!("Finished sharing anchor. Components are {enabled:?}");
        }
    }
}
