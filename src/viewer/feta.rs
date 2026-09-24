//! Authoritative two-player Feta rules. All durations count fixed 60 Hz server ticks.
use super::{
    controller::{CharacterKind, Controller, KinematicState, Movement},
    room::Room,
};
use crate::math::{Ray, V};
use serde::{Deserialize, Serialize};

pub const HZ: u64 = 60;
pub const DT: f32 = 1. / HZ as f32;
pub const VERSION: u32 = 2;
pub const DEFAULT_SERVER: &str = "73.157.115.243:4000";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub hunt_seconds: u32,
    pub hide_seconds: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hunt_seconds: 300,
            hide_seconds: 60,
        }
    }
}
impl Settings {
    pub fn valid(self) -> bool {
        (30..=1800).contains(&self.hunt_seconds) && (5..=300).contains(&self.hide_seconds)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Lobby,
    Hiding,
    Hunting,
    Finished,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Feta,
    Scientist,
    Disconnected,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Select(CharacterKind),
    Configure(Settings),
    Ready { revision: u64 },
    Leave,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Input {
    pub sequence: u64,
    pub round: u64,
    pub movement: Movement,
    pub yaw: f32,
    pub pitch: f32,
    pub aim_tick: u64,
    /// Increasing counters are repeated until acknowledged, so a lost edge is harmless.
    pub shot: u64,
    pub melee: u64,
    pub jump: u64,
}
impl Input {
    pub fn valid(&self) -> bool {
        self.yaw.is_finite()
            && self.pitch.is_finite()
            && self.yaw.abs() < 1.0e6
            && self.pitch.abs() <= 1.51
            && self.movement.forward.is_finite()
            && self.movement.right.is_finite()
            && self.movement.forward.abs() <= 1.
            && self.movement.right.abs() <= 1.
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub slot: usize,
    pub role: Option<CharacterKind>,
    pub ready: bool,
    pub pose: KinematicState,
    pub ack: u64,
    pub shots: u64,
    pub swings: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub tick: u64,
    pub round: u64,
    pub revision: u64,
    pub phase: Phase,
    pub remaining: u64,
    pub outcome: Option<Outcome>,
    pub settings: Settings,
    pub players: Vec<PlayerView>,
}
pub struct Player {
    pub role: Option<CharacterKind>,
    pub ready: bool,
    pub controller: Controller,
    input: Input,
    applied: u64,
    last_input: u64,
    shot: u64,
    melee: u64,
    jump: u64,
    cooldown: u64,
    windup: Option<u64>,
    pub shots: u64,
    pub swings: u64,
}
impl Default for Player {
    fn default() -> Self {
        Self {
            role: None,
            ready: false,
            controller: spawn(CharacterKind::Scientist),
            input: Input::default(),
            applied: 0,
            last_input: 0,
            shot: 0,
            melee: 0,
            jump: 0,
            cooldown: 0,
            windup: None,
            shots: 0,
            swings: 0,
        }
    }
}
/// Fixed bundled content ID, derived from normalized source to avoid platform libm differences.
/// Compatibility only: session authentication is independent of this non-cryptographic hash.
pub fn content_id() -> u64 {
    let sources = [
        include_str!("feta.rs"),
        include_str!("feta_net.rs"),
        include_str!("feta_secure.rs"),
        include_str!("house.rs"),
        include_str!("controller.rs"),
        include_str!("profile.rs"),
        include_str!("room.rs"),
        include_str!("props.rs"),
        include_str!("props/accessories.rs"),
        include_str!("props/decor_library.rs"),
        include_str!("landscaping.rs"),
    ];
    sources
        .iter()
        .flat_map(|s| s.bytes().filter(|b| *b != b'\r'))
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}
pub fn map() -> crate::Result<Room> {
    super::house::build_feta()
}
pub fn spawn(role: CharacterKind) -> Controller {
    let mut c = Controller::for_character(role);
    let (x, z) = if role == CharacterKind::Feta {
        (0., 2.8)
    } else {
        (0., 7.3)
    };
    c.set_physics_state(V(x, c.position.1, z), 0., true);
    c.yaw = 0.;
    c.pitch = 0.;
    c
}
pub struct Match {
    pub room: Room,
    pub players: [Option<Player>; 2],
    pub phase: Phase,
    pub tick: u64,
    pub round: u64,
    pub revision: u64,
    pub settings: Settings,
    pub outcome: Option<Outcome>,
    deadline: u64,
    history: std::collections::VecDeque<(u64, [Option<Controller>; 2])>,
}
impl Match {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            room: map()?,
            players: [None, None],
            phase: Phase::Lobby,
            tick: 0,
            round: 0,
            revision: 0,
            settings: Settings::default(),
            outcome: None,
            deadline: 0,
            history: std::collections::VecDeque::new(),
        })
    }
    pub fn join(&mut self, slot: usize) {
        self.players[slot] = Some(Player::default());
        self.invalidate_ready();
    }
    fn invalidate_ready(&mut self) {
        self.revision += 1;
        for p in self.players.iter_mut().flatten() {
            p.ready = false;
        }
    }
    pub fn leave(&mut self, slot: usize) {
        self.players[slot] = None;
        if matches!(self.phase, Phase::Hiding | Phase::Hunting) {
            self.finish(Outcome::Disconnected);
        }
        self.invalidate_ready();
    }
    pub fn action(&mut self, slot: usize, action: Action) {
        if slot >= 2 || self.players[slot].is_none() {
            return;
        }
        if matches!(action, Action::Leave) {
            self.leave(slot);
            return;
        }
        if self.phase != Phase::Lobby {
            return;
        }
        match action {
            Action::Select(role) => {
                if self
                    .players
                    .iter()
                    .enumerate()
                    .any(|(i, p)| i != slot && p.as_ref().is_some_and(|p| p.role == Some(role)))
                {
                    return;
                }
                self.players[slot].as_mut().unwrap().role = Some(role);
                self.players[slot].as_mut().unwrap().controller = spawn(role);
                self.invalidate_ready();
            }
            Action::Configure(settings) => {
                let host = self.players.iter().position(Option::is_some);
                if host == Some(slot) && settings.valid() && self.settings != settings {
                    self.settings = settings;
                    self.invalidate_ready();
                }
            }
            Action::Ready { revision } if revision == self.revision => {
                let p = self.players[slot].as_mut().unwrap();
                if p.role.is_some() {
                    p.ready = true;
                }
                if self
                    .players
                    .iter()
                    .all(|p| p.as_ref().is_some_and(|p| p.ready))
                    && self.players[0].as_ref().unwrap().role
                        != self.players[1].as_ref().unwrap().role
                {
                    self.history.clear();
                    self.round += 1;
                    self.phase = Phase::Hiding;
                    self.outcome = None;
                    self.deadline = self.tick + self.settings.hide_seconds as u64 * HZ;
                    for p in self.players.iter_mut().flatten() {
                        p.controller = spawn(p.role.unwrap());
                        p.input = Input::default();
                        p.applied = 0;
                        p.shot = 0;
                        p.melee = 0;
                        p.jump = 0;
                        p.cooldown = 0;
                        p.windup = None;
                        p.shots = 0;
                        p.swings = 0;
                    }
                }
            }
            _ => {}
        }
    }
    pub fn input(&mut self, slot: usize, mut input: Input) {
        if slot >= 2 || !input.valid() || input.round != self.round {
            return;
        }
        if let Some(p) = &mut self.players[slot] {
            if input.sequence <= p.input.sequence {
                return;
            }
            input.movement.jump = false; // Only the reliable jump counter creates an edge.
            p.input = input;
            p.last_input = self.tick;
        }
    }
    pub fn can_move(phase: Phase, role: Option<CharacterKind>) -> bool {
        phase == Phase::Hunting || (phase == Phase::Hiding && role == Some(CharacterKind::Feta))
    }
    pub fn step(&mut self) {
        self.tick += 1;
        if self.phase == Phase::Finished && self.tick >= self.deadline {
            self.phase = Phase::Lobby;
            self.invalidate_ready();
            for p in self.players.iter_mut().flatten() {
                p.role = None;
                p.controller.stop();
            }
        }
        if self.phase == Phase::Hiding && self.tick >= self.deadline {
            self.phase = Phase::Hunting;
            self.deadline = self.tick + self.settings.hunt_seconds as u64 * HZ;
        }
        // At the exact deadline survival wins; no late fire can reverse the result.
        if self.phase == Phase::Hunting && self.tick >= self.deadline {
            self.finish(Outcome::Feta);
        }
        let mut attacks = Vec::new();
        for (slot, p) in self.players.iter_mut().enumerate() {
            let Some(p) = p else {
                continue;
            };
            let fresh = self.tick.saturating_sub(p.last_input) <= 9;
            let mut movement = if fresh {
                p.input.movement
            } else {
                Movement::default()
            };
            movement.jump = fresh && p.input.jump > p.jump;
            p.jump = p.input.jump;
            if Self::can_move(self.phase, p.role) {
                p.controller.yaw = p.input.yaw;
                p.controller.pitch = p.input.pitch;
                p.controller.update(movement, DT, &self.room.colliders);
            } else {
                p.controller.stop();
            }
            p.applied = p.input.sequence;
            let shoot = p.input.shot > p.shot;
            let melee = p.input.melee > p.melee;
            p.shot = p.input.shot;
            p.melee = p.input.melee;
            if self.phase == Phase::Hunting && p.role == Some(CharacterKind::Scientist) && fresh {
                if let Some(at) = p.windup {
                    if self.tick >= at {
                        attacks.push((slot, 1.65, p.input.aim_tick));
                        p.windup = None;
                    }
                }
                if self.tick >= p.cooldown {
                    if shoot {
                        p.shots += 1;
                        p.cooldown = self.tick + 24;
                        attacks.push((slot, 40., p.input.aim_tick));
                    } else if melee {
                        p.swings += 1;
                        p.cooldown = self.tick + 32;
                        p.windup = Some(self.tick + 11);
                    }
                }
            } else {
                p.windup = None;
            }
        }
        self.history.push_back((
            self.tick,
            std::array::from_fn(|i| self.players[i].as_ref().map(|p| p.controller.clone())),
        ));
        while self.history.len() > 13 {
            self.history.pop_front();
        }
        for (slot, range, aim_tick) in attacks {
            if self.phase != Phase::Hunting {
                break;
            }
            let attacker = &self.players[slot].as_ref().unwrap().controller;
            if let Some(target) = self.players[1 - slot].as_ref() {
                if target.role == Some(CharacterKind::Feta) && {
                    let aim_tick = aim_tick.clamp(self.tick.saturating_sub(12), self.tick);
                    let historical = self
                        .history
                        .iter()
                        .rev()
                        .find(|(tick, _)| *tick <= aim_tick)
                        .and_then(|(_, players)| players[1 - slot].as_ref())
                        .unwrap_or(&target.controller);
                    hit(&self.room, attacker.ray(), historical, range)
                } {
                    self.finish(Outcome::Scientist);
                }
            }
        }
    }
    fn finish(&mut self, outcome: Outcome) {
        self.phase = Phase::Finished;
        self.outcome = Some(outcome);
        self.deadline = self.tick + 5 * HZ;
        for p in self.players.iter_mut().flatten() {
            p.controller.stop();
            p.windup = None;
        }
    }
    /// During the head start the Scientist receives no rat position at all.
    pub fn state(&self, recipient: usize) -> State {
        let blind = self.phase == Phase::Hiding
            && self.players[recipient]
                .as_ref()
                .is_some_and(|p| p.role == Some(CharacterKind::Scientist));
        State {
            tick: self.tick,
            round: self.round,
            revision: self.revision,
            phase: self.phase,
            remaining: self.deadline.saturating_sub(self.tick),
            outcome: self.outcome,
            settings: self.settings,
            players: self
                .players
                .iter()
                .enumerate()
                .filter_map(|(slot, p)| {
                    let p = p.as_ref()?;
                    if blind && slot != recipient {
                        return None;
                    }
                    Some(PlayerView {
                        slot,
                        role: p.role,
                        ready: p.ready,
                        pose: p.controller.network_state(),
                        ack: p.applied,
                        shots: p.shots,
                        swings: p.swings,
                    })
                })
                .collect(),
        }
    }
}
/// Ray versus the rat's body box, occluded by both visible geometry and collision-only barriers.
pub fn hit(room: &Room, ray: Ray, target: &Controller, range: f32) -> bool {
    let r = target.character_kind().radius();
    let lo = V(
        target.position.0 - r,
        target.feet_height(),
        target.position.2 - r,
    );
    let hi = V(
        target.position.0 + r,
        target.feet_height() + target.body_height(),
        target.position.2 + r,
    );
    let Some(distance) = ray_box(ray, lo, hi, range) else {
        return false;
    };
    if room.hit(ray, distance + 0.001).is_some() {
        return false;
    }
    !room
        .colliders
        .iter()
        .any(|c| ray_box(ray, c.min, c.max, distance).is_some())
}
fn ray_box(ray: Ray, lo: V, hi: V, limit: f32) -> Option<f32> {
    let mut near = 0_f32;
    let mut far = limit;
    for axis in 0..3 {
        let o = ray.o.axis(axis);
        let d = ray.d.axis(axis);
        if d.abs() < 1e-7 {
            if o < lo.axis(axis) || o > hi.axis(axis) {
                return None;
            }
        } else {
            let a = (lo.axis(axis) - o) / d;
            let b = (hi.axis(axis) - o) / d;
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}
