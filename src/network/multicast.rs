use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    str::FromStr,
};

use bevy_oxr::xr::sys::SpaceUserFB;

// Administratively scoped multicast
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(239, 2, 2, 2);
const MULTICAST_PORT: u16 = 18860;
const MULTICAST_SOCKET: (Ipv4Addr, u16) = (MULTICAST_ADDRESS, MULTICAST_PORT);

const MULTICAST_MTU: usize = 1500;

pub fn decode(msg: Box<[u8]>) -> Option<(u16, u64)> {
    let msg_parts = msg.split(|b| *b == b':').collect::<Vec<&[u8]>>();
    if msg_parts.len() != 3 {
        // Should contain three segments
        return None;
    }
    if msg_parts[0] != b"GOUDA" {
        // Failed magic bytes check
        return None;
    }
    // Get port number
    let port: u16 = FromStr::from_str(std::str::from_utf8(msg_parts[1]).ok()?).ok()?;
    // Get XrSpaceUserFB ID
    let fb_id: u64 = FromStr::from_str(std::str::from_utf8(msg_parts[1]).ok()?).ok()?;
    Some((port, fb_id))
}

pub struct MulticastListener {
    pub socket: UdpSocket,
}

impl MulticastListener {
    pub fn new() -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, MULTICAST_PORT)).unwrap();
        socket
            .join_multicast_v4(&MULTICAST_ADDRESS, &Ipv4Addr::UNSPECIFIED)
            .unwrap();
        socket.set_nonblocking(true).unwrap();
        Self { socket }
    }

    pub fn get_buf(&self) -> Option<(Box<[u8]>, SocketAddr)> {
        let mut buf = vec![0u8; MULTICAST_MTU];

        match self.socket.recv_from(buf.as_mut()) {
            Ok((len, addr)) => Some((Box::<[u8]>::from(&buf[..len]), addr)),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => None,
            Err(err) if err.kind() == io::ErrorKind::ConnectionReset => None,
            Err(err) => panic!("{err:?} on {:?}", self.socket),
        }
    }
}

pub struct MulticastEmitter {
    pub socket: UdpSocket,
    listener: TcpListener,
    fb_id: u64,
}

impl MulticastEmitter {
    pub fn new(fb_id: SpaceUserFB) -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            socket,
            listener,
            fb_id: fb_id.into_raw(),
        }
    }

    pub fn emit(&self) {
        use std::fmt::Write;
        let mut msg = String::new();
        write!(
            msg,
            "GOUDA:{}:{}",
            self.listener.local_addr().unwrap().port(),
            self.fb_id
        )
        .unwrap();
        self.socket
            .send_to(&msg.as_bytes(), MULTICAST_SOCKET)
            .unwrap();
    }

    pub fn accept(&self) -> Option<(TcpStream, SocketAddr)> {
        match self.listener.accept() {
            Ok((stream, socket)) => {
                stream.set_nonblocking(true).unwrap();
                Some((stream, socket))
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => None,
            Err(err) => panic!("{err:?} on {:?}", self.socket),
        }
    }
}
