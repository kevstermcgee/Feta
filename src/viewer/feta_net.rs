//! Bounded Feta UDP transport. Deploy only on loopback or a private Tailscale interface.
//! Tailscale supplies encryption and peer authentication; the join key is an extra gate.
use super::feta::{self, Action, Input, Match, State};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};
pub const MTU: usize = 1400;
pub type Token = [u64; 2];
pub fn random_token() -> crate::Result<Token> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| format!("OS randomness unavailable: {e}"))?;
    Ok([
        u64::from_le_bytes(bytes[..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..].try_into().unwrap()),
    ])
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Wire {
    Hello {
        version: u32,
        content: u64,
        nonce: Token,
        key: String,
    },
    Welcome {
        nonce: Token,
        token: Token,
        slot: usize,
    },
    Reject {
        nonce: Token,
        reason: String,
    },
    Input {
        token: Token,
        input: Input,
    },
    Command {
        token: Token,
        sequence: u64,
        round: u64,
        action: Action,
    },
    Snapshot {
        token: Token,
        command_ack: u64,
        state: State,
    },
}
pub fn encode(packet: &Wire) -> crate::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(packet)?;
    if bytes.len() > MTU {
        return Err("Feta packet exceeds MTU".into());
    }
    Ok(bytes)
}
fn send(socket: &UdpSocket, address: SocketAddr, packet: &Wire) -> crate::Result<()> {
    socket.send_to(&encode(packet)?, address)?;
    Ok(())
}
fn receive(socket: &UdpSocket, buffer: &mut [u8]) -> crate::Result<Vec<(SocketAddr, Wire)>> {
    let mut packets = Vec::new();
    for _ in 0..128 {
        match socket.recv_from(buffer) {
            Ok((len, addr)) if len <= MTU => {
                if let Ok(packet) = serde_json::from_slice(&buffer[..len]) {
                    packets.push((addr, packet));
                }
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            // Windows may report ICMP port-unreachable after the peer exits.
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                continue
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(packets)
}
struct Session {
    address: SocketAddr,
    nonce: Token,
    token: Token,
    seen: Instant,
    command: u64,
    input: u64,
}
pub struct Server {
    pub game: Match,
    socket: UdpSocket,
    sessions: [Option<Session>; 2],
    buffer: Box<[u8]>,
    key: String,
    content: u64,
    hello_window: Instant,
    hellos: usize,
}
impl Server {
    pub fn bind(address: &str, key: String) -> crate::Result<Self> {
        if !(8..=128).contains(&key.len()) {
            return Err("Join key must contain 8..128 bytes".into());
        }
        let socket = UdpSocket::bind(address)?;
        socket.set_nonblocking(true)?;
        let game = Match::new()?;
        let content = feta::content_id();
        Ok(Self {
            game,
            socket,
            sessions: [None, None],
            buffer: vec![0; 65536].into_boxed_slice(),
            key,
            content,
            hello_window: Instant::now(),
            hellos: 0,
        })
    }
    pub fn address(&self) -> crate::Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
    pub fn peers(&self) -> usize {
        self.sessions.iter().flatten().count()
    }
    pub fn poll(&mut self) -> crate::Result<()> {
        let now = Instant::now();
        if now.duration_since(self.hello_window) >= Duration::from_secs(1) {
            self.hello_window = now;
            self.hellos = 0;
        }
        for (address, packet) in receive(&self.socket, &mut self.buffer)? {
            if let Wire::Hello {
                version,
                content,
                nonce,
                key,
            } = packet
            {
                if self.hellos >= 12 {
                    continue;
                }
                self.hellos += 1;
                let reason = if version != feta::VERSION || content != self.content {
                    Some("Different game version. Download the same release.")
                } else if key != self.key {
                    Some("Incorrect join key.")
                } else {
                    None
                };
                if let Some(reason) = reason {
                    let _ = send(
                        &self.socket,
                        address,
                        &Wire::Reject {
                            nonce,
                            reason: reason.into(),
                        },
                    );
                    continue;
                }
                if let Some((slot, s)) = self.sessions.iter().enumerate().find_map(|(i, s)| {
                    s.as_ref()
                        .filter(|s| s.address == address && s.nonce == nonce)
                        .map(|s| (i, s))
                }) {
                    let _ = send(
                        &self.socket,
                        address,
                        &Wire::Welcome {
                            nonce,
                            token: s.token,
                            slot,
                        },
                    );
                    continue;
                }
                if let Some(slot) = self.sessions.iter().position(Option::is_none) {
                    let token = random_token()?;
                    self.sessions[slot] = Some(Session {
                        address,
                        nonce,
                        token,
                        seen: now,
                        command: 0,
                        input: 0,
                    });
                    self.game.join(slot);
                    let _ = send(&self.socket, address, &Wire::Welcome { nonce, token, slot });
                } else {
                    let _ = send(
                        &self.socket,
                        address,
                        &Wire::Reject {
                            nonce,
                            reason: "Two players are already connected.".into(),
                        },
                    );
                }
                continue;
            }
            let Some(slot) = self
                .sessions
                .iter()
                .position(|s| s.as_ref().is_some_and(|s| s.address == address))
            else {
                continue;
            };
            let session = self.sessions[slot].as_mut().unwrap();
            match packet {
                Wire::Input { token, input }
                    if token == session.token
                        && input.valid()
                        && input.sequence > session.input =>
                {
                    session.seen = now;
                    session.input = input.sequence;
                    self.game.input(slot, input);
                }
                Wire::Command {
                    token,
                    sequence,
                    round,
                    action,
                } if token == session.token && sequence > session.command => {
                    session.seen = now;
                    session.command = sequence;
                    if matches!(action, Action::Leave) {
                        self.sessions[slot] = None;
                        self.game.leave(slot);
                    } else if round == self.game.round {
                        self.game.action(slot, action);
                    }
                }
                _ => {}
            }
        }
        for slot in 0..2 {
            if self.sessions[slot]
                .as_ref()
                .is_some_and(|s| now.duration_since(s.seen) > Duration::from_secs(5))
            {
                self.sessions[slot] = None;
                self.game.leave(slot);
            }
        }
        Ok(())
    }
    pub fn step(&mut self) -> crate::Result<()> {
        self.poll()?;
        self.game.step();
        if self.game.tick.is_multiple_of(3) {
            for (slot, session) in self.sessions.iter().enumerate() {
                if let Some(s) = session {
                    // Encoding failures are a local bug; surface them instead of silently losing state.
                    let bytes = encode(&Wire::Snapshot {
                        token: s.token,
                        command_ack: s.command,
                        state: self.game.state(slot),
                    })?;
                    let _ = self.socket.send_to(&bytes, s.address);
                }
            }
        }
        Ok(())
    }
}
/// Client socket accepts packets only from the configured endpoint and current random session.
pub struct Client {
    socket: UdpSocket,
    server: SocketAddr,
    buffer: Box<[u8]>,
    nonce: Token,
    token: Option<Token>,
    pub slot: Option<usize>,
    pub state: Option<State>,
    pub error: Option<String>,
    key: String,
    content: u64,
    started: Instant,
    last_receive: Instant,
    last_send: Instant,
    sequence: u64,
    pending: VecDeque<(u64, u64, Action)>,
}
impl Client {
    pub fn connect(server: SocketAddr, key: String, content: u64) -> crate::Result<Self> {
        if !(8..=128).contains(&key.len()) {
            return Err("Enter the join key (8..128 characters).".into());
        }
        let socket = UdpSocket::bind(if server.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })?;
        socket.set_nonblocking(true)?;
        let now = Instant::now();
        Ok(Self {
            socket,
            server,
            buffer: vec![0; 65536].into_boxed_slice(),
            nonce: random_token()?,
            token: None,
            slot: None,
            state: None,
            error: None,
            key,
            content,
            started: now,
            last_receive: now,
            last_send: now - Duration::from_secs(2),
            sequence: 0,
            pending: VecDeque::new(),
        })
    }
    pub fn command(&mut self, action: Action) {
        if self.pending.len() >= 8 {
            return;
        }
        self.sequence += 1;
        self.pending.push_back((
            self.sequence,
            self.state.as_ref().map_or(0, |s| s.round),
            action,
        ));
    }
    pub fn pending(&self) -> bool {
        !self.pending.is_empty()
    }
    pub fn send_input(&self, input: Input) -> crate::Result<()> {
        if let Some(token) = self.token {
            send(&self.socket, self.server, &Wire::Input { token, input })?;
        }
        Ok(())
    }
    pub fn poll(&mut self) -> crate::Result<Vec<State>> {
        let now = Instant::now();
        let mut states = Vec::new();
        for (address, packet) in receive(&self.socket, &mut self.buffer)? {
            if address != self.server {
                continue;
            }
            match packet {
                Wire::Welcome { nonce, token, slot }
                    if nonce == self.nonce && self.token.is_none() && slot < 2 =>
                {
                    self.token = Some(token);
                    self.slot = Some(slot);
                    self.key.clear();
                    self.last_receive = now;
                }
                Wire::Reject { nonce, reason } if nonce == self.nonce && self.token.is_none() => {
                    self.error = Some(reason);
                }
                Wire::Snapshot {
                    token,
                    command_ack,
                    state,
                } if Some(token) == self.token => {
                    if self.state.as_ref().is_some_and(|s| state.tick <= s.tick) {
                        continue;
                    }
                    self.last_receive = now;
                    self.pending.retain(|(seq, _, _)| *seq > command_ack);
                    self.state = Some(state.clone());
                    states.push(state);
                }
                _ => {}
            }
        }
        if self.token.is_none() && now.duration_since(self.started) > Duration::from_secs(12) {
            self.error.get_or_insert(
                "Cannot reach server. Check Tailscale and the server address.".into(),
            );
        } else if self.token.is_some()
            && now.duration_since(self.last_receive) > Duration::from_secs(6)
        {
            self.error
                .get_or_insert("Connection lost. Reconnect to return to the lobby.".into());
        }
        if self.error.is_none() && now.duration_since(self.last_send) > Duration::from_millis(250) {
            self.last_send = now;
            if let Some(token) = self.token {
                if let Some((sequence, round, action)) = self.pending.front() {
                    send(
                        &self.socket,
                        self.server,
                        &Wire::Command {
                            token,
                            sequence: *sequence,
                            round: *round,
                            action: action.clone(),
                        },
                    )?;
                }
            } else {
                send(
                    &self.socket,
                    self.server,
                    &Wire::Hello {
                        version: feta::VERSION,
                        content: self.content,
                        nonce: self.nonce,
                        key: self.key.clone(),
                    },
                )?;
            }
        }
        Ok(states)
    }
    pub fn disconnect(&mut self) {
        if let Some(token) = self.token {
            self.sequence += 1;
            for _ in 0..3 {
                let _ = send(
                    &self.socket,
                    self.server,
                    &Wire::Command {
                        token,
                        sequence: self.sequence,
                        round: 0,
                        action: Action::Leave,
                    },
                );
            }
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.disconnect();
    }
}
