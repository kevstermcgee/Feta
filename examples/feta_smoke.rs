//! End-to-end live-server smoke: two actual clients, lobby, ready, hide, hunt, result, rematch lobby.
//! Run only on an empty test server; reads FETA_JOIN_KEY from the environment.
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use vesper3d::viewer::{
    controller::CharacterKind,
    feta::{self, Action, Input, Outcome, Phase, Settings},
    feta_net::Client,
};
fn main() -> vesper3d::Result<()> {
    let address: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:4001".into())
        .parse()?;
    let key = std::env::var("FETA_JOIN_KEY")?;
    let mut clients = [
        Client::connect(address, key.clone(), feta::content_id())?,
        Client::connect(address, key, feta::content_id())?,
    ];
    let start = Instant::now();
    let mut seq = 0;
    let mut saw_hide = false;
    let mut saw_hunt = false;
    let mut saw_win = false;
    let mut previous = [0, 0];
    let mut received = [0, 0];
    while start.elapsed() < Duration::from_secs(55) {
        seq += 1;
        for (i, c) in clients.iter_mut().enumerate() {
            c.poll()?;
            if let Some(error) = &c.error {
                return Err(error.clone().into());
            }
            let Some(state) = c.state.clone() else {
                continue;
            };
            if state.tick != previous[i] {
                received[i] += 1;
                previous[i] = state.tick;
            }
            c.send_input(Input {
                sequence: seq,
                round: state.round,
                ..Default::default()
            })?;
            let slot = c.slot.unwrap();
            let me = state.players.iter().find(|p| p.slot == slot).unwrap();
            if state.phase == Phase::Lobby && !saw_win && !c.pending() {
                if slot == 0
                    && state.settings
                        != (Settings {
                            hunt_seconds: 30,
                            hide_seconds: 5,
                        })
                {
                    c.command(Action::Configure(Settings {
                        hunt_seconds: 30,
                        hide_seconds: 5,
                    }));
                } else if me.role.is_none() {
                    c.command(Action::Select(if i == 0 {
                        CharacterKind::Scientist
                    } else {
                        CharacterKind::Feta
                    }));
                } else if !me.ready
                    && state.players.len() == 2
                    && state.players.iter().all(|p| p.role.is_some())
                {
                    c.command(Action::Ready {
                        revision: state.revision,
                    });
                }
            }
            saw_hide |= state.phase == Phase::Hiding;
            saw_hunt |= state.phase == Phase::Hunting;
            if state.phase == Phase::Finished {
                if state.outcome != Some(Outcome::Feta) {
                    return Err("Expected rat survival".into());
                }
                saw_win = true;
            }
        }
        if saw_win
            && clients.iter().all(|c| {
                c.state.as_ref().is_some_and(|s| {
                    s.phase == Phase::Lobby
                        && s.players.iter().all(|p| p.role.is_none() && !p.ready)
                })
            })
        {
            if !saw_hide || !saw_hunt {
                return Err("Missing phase transition".into());
            }
            println!("PASS: authenticated two clients, opposite roles, configured 5s hide/30s hunt, Feta victory, cleared rematch lobby; snapshots={received:?}, elapsed={:.2}s",start.elapsed().as_secs_f64());
            // Restore public defaults for the next human session.
            let host = clients.iter().position(|c| c.slot == Some(0)).unwrap();
            clients[host].command(Action::Configure(Settings::default()));
            for _ in 0..25 {
                for c in &mut clients {
                    c.poll()?;
                    seq += 1;
                    c.send_input(Input {
                        sequence: seq,
                        round: c.state.as_ref().unwrap().round,
                        ..Default::default()
                    })?;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs_f64(1. / 60.));
    }
    Err("Live server round timed out".into())
}
