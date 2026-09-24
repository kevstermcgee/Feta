use vesper3d::{
    math::V,
    viewer::{
        controller::{CharacterKind as Role, Movement},
        feta::{self, Action, Input, Match, Outcome, Phase, Settings},
        feta_net::{self, Client, Server, Wire},
    },
};
fn lobby() -> Match {
    let mut g = Match::new().unwrap();
    g.join(0);
    g.join(1);
    g.action(0, Action::Select(Role::Scientist));
    g.action(1, Action::Select(Role::Feta));
    g
}
fn start(g: &mut Match) {
    g.action(
        0,
        Action::Configure(Settings {
            hunt_seconds: 30,
            hide_seconds: 5,
        }),
    );
    let revision = g.revision;
    g.action(0, Action::Ready { revision });
    g.action(1, Action::Ready { revision });
    assert_eq!(g.phase, Phase::Hiding);
}
fn hunt(g: &mut Match) {
    start(g);
    for _ in 0..300 {
        g.step();
    }
    assert_eq!(g.phase, Phase::Hunting);
}
fn aim(g: &mut Match, shot: u64, melee: u64) {
    let target = g.players[1].as_ref().unwrap().controller.position;
    let shooter = g.players[0].as_ref().unwrap().controller.position;
    let d = (target - shooter).norm();
    g.input(
        0,
        Input {
            sequence: g.tick + 1,
            round: g.round,
            yaw: d.0.atan2(-d.2),
            pitch: d.1.asin(),
            aim_tick: g.tick + 1,
            shot,
            melee,
            ..Default::default()
        },
    );
}
#[test]
fn lobby_requires_distinct_roles_and_current_settings_confirmation() {
    let mut g = Match::new().unwrap();
    assert_eq!(g.settings, Settings::default());
    g.join(0);
    g.action(0, Action::Select(Role::Feta));
    g.action(
        0,
        Action::Ready {
            revision: g.revision,
        },
    );
    assert_eq!(g.phase, Phase::Lobby);
    g.join(1);
    g.action(1, Action::Select(Role::Feta));
    assert_eq!(g.players[1].as_ref().unwrap().role, None);
    g.action(1, Action::Select(Role::Scientist));
    let stale = g.revision;
    g.action(0, Action::Ready { revision: stale });
    g.action(
        0,
        Action::Configure(Settings {
            hunt_seconds: 60,
            hide_seconds: 10,
        }),
    );
    assert!(!g.players[0].as_ref().unwrap().ready);
    g.action(1, Action::Ready { revision: stale });
    assert!(!g.players[1].as_ref().unwrap().ready);
    g.action(1, Action::Configure(Settings::default()));
    assert_eq!(g.settings.hunt_seconds, 60);
    g.action(
        0,
        Action::Configure(Settings {
            hunt_seconds: 0,
            hide_seconds: 0,
        }),
    );
    assert_eq!(g.settings.hunt_seconds, 60);
}
#[test]
fn headstart_freezes_scientist_hides_rat_and_survival_resets_roles() {
    let mut g = lobby();
    start(&mut g);
    let before = g.players[0].as_ref().unwrap().controller.position;
    g.input(
        0,
        Input {
            sequence: 1,
            round: g.round,
            movement: Movement {
                forward: 1.,
                ..Default::default()
            },
            shot: 1,
            ..Default::default()
        },
    );
    g.input(
        1,
        Input {
            sequence: 1,
            round: g.round,
            movement: Movement {
                forward: 1.,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    for _ in 0..299 {
        g.step();
    }
    assert_eq!(g.players[0].as_ref().unwrap().controller.position, before);
    assert_eq!(g.state(0).players.len(), 1);
    assert_eq!(g.state(1).players.len(), 2);
    g.step();
    assert_eq!(g.phase, Phase::Hunting);
    assert_eq!(g.state(0).remaining, 1800);
    for _ in 0..1800 {
        g.step();
    }
    assert_eq!(g.outcome, Some(Outcome::Feta));
    for _ in 0..300 {
        g.step();
    }
    assert_eq!(g.phase, Phase::Lobby);
    assert!(g
        .players
        .iter()
        .flatten()
        .all(|p| p.role.is_none() && !p.ready));
}
#[test]
fn visible_shot_wins_but_wall_and_wrong_role_do_not() {
    let mut g = lobby();
    hunt(&mut g);
    g.players[0]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 1.68, 4.), 0., true);
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 0.22, 2.), 0., true);
    aim(&mut g, 1, 0);
    g.step();
    assert_eq!(g.outcome, Some(Outcome::Scientist));
    let mut g = lobby();
    hunt(&mut g);
    g.players[0]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(-4.8, 1.68, 1.), 0., true);
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(-4.8, 0.22, -1.), 0., true);
    aim(&mut g, 1, 0);
    g.step();
    assert_eq!(g.phase, Phase::Hunting);
    g.input(
        1,
        Input {
            sequence: 1,
            round: g.round,
            shot: 100,
            ..Default::default()
        },
    );
    g.step();
    assert_eq!(g.phase, Phase::Hunting);
}
#[test]
fn melee_uses_delayed_contact_and_disconnect_cancels_round() {
    let mut g = lobby();
    hunt(&mut g);
    g.players[0]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 1.68, 3.), 0., true);
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 0.22, 2.5), 0., true);
    aim(&mut g, 0, 1);
    g.step();
    assert_eq!(g.phase, Phase::Hunting);
    for _ in 0..12 {
        aim(&mut g, 0, 1);
        g.step();
    }
    assert_eq!(g.outcome, Some(Outcome::Scientist));
    let mut g = lobby();
    hunt(&mut g);
    g.leave(1);
    assert_eq!(g.outcome, Some(Outcome::Disconnected));
    for _ in 0..300 {
        g.step();
    }
    assert_eq!(g.phase, Phase::Lobby);
}
#[test]
fn replay_invalid_input_and_old_round_are_ignored() {
    let mut g = lobby();
    hunt(&mut g);
    let before = g.players[0].as_ref().unwrap().controller.position;
    g.input(
        0,
        Input {
            sequence: 1,
            round: g.round,
            yaw: f32::NAN,
            ..Default::default()
        },
    );
    g.step();
    assert_eq!(g.players[0].as_ref().unwrap().controller.position, before);
    g.input(
        0,
        Input {
            sequence: 2,
            round: g.round - 1,
            movement: Movement {
                forward: 1.,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    g.step();
    assert_eq!(g.players[0].as_ref().unwrap().controller.position, before);
    aim(&mut g, 1, 0);
    g.step();
    let shots = g.players[0].as_ref().unwrap().shots;
    for _ in 0..60 {
        g.step();
    }
    assert_eq!(g.players[0].as_ref().unwrap().shots, shots);
}
#[test]
fn snapshots_stay_under_mtu_in_every_phase() {
    let mut g = lobby();
    for _ in 0..2 {
        for slot in 0..2 {
            let bytes = feta_net::encode(&Wire::Snapshot {
                token: [u64::MAX; 2],
                command_ack: u64::MAX,
                state: g.state(slot),
            })
            .unwrap();
            assert!(bytes.len() <= feta_net::MTU);
        }
        if g.phase == Phase::Lobby {
            hunt(&mut g);
        }
    }
}
#[test]
fn actual_encrypted_authentication_capacity_commands_and_disconnect() {
    use vesper3d::viewer::feta_secure::Identity;
    let identity = rcgen::generate_simple_self_signed(vec!["feta.local".into()]).unwrap();
    let certificate = identity.cert.der().to_vec();
    let mut server = Server::bind_with_identity(
        "127.0.0.1:0",
        "test-key-only".into(),
        Identity {
            certificate: certificate.clone(),
            private_key: identity.signing_key.serialize_der(),
        },
    )
    .unwrap();
    let addr = server.address().unwrap();
    let hash = feta::content_id();
    let connect = |key: &str, hash| {
        Client::connect_with_certificate(addr, key.into(), hash, certificate.clone()).unwrap()
    };
    fn pump(server: &mut Server, clients: &mut [&mut Client]) {
        for _ in 0..50 {
            for c in clients.iter_mut() {
                if let Err(e) = c.poll() {
                    c.error = Some(e.to_string());
                }
            }
            server.step().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        for c in clients.iter_mut() {
            if let Err(e) = c.poll() {
                c.error = Some(e.to_string());
            }
        }
    }
    let mut bad = connect("wrong-key", hash);
    pump(&mut server, &mut [&mut bad]);
    assert!(bad.error.as_ref().unwrap().contains("Incorrect"));
    assert_eq!(server.peers(), 0);
    drop(bad);
    let mut mismatch = connect("test-key-only", hash + 1);
    pump(&mut server, &mut [&mut mismatch]);
    assert!(mismatch.error.is_some());
    drop(mismatch);
    let impostor = rcgen::generate_simple_self_signed(vec!["feta.local".into()]).unwrap();
    let mut untrusted = Client::connect_with_certificate(
        addr,
        "test-key-only".into(),
        hash,
        impostor.cert.der().to_vec(),
    )
    .unwrap();
    pump(&mut server, &mut [&mut untrusted]);
    assert!(untrusted
        .error
        .as_ref()
        .unwrap()
        .contains("Secure connection failed"));
    assert_eq!(server.peers(), 0);
    drop(untrusted);
    let mut a = connect("test-key-only", hash);
    let mut b = connect("test-key-only", hash);
    pump(&mut server, &mut [&mut a, &mut b]);
    assert!(a.slot.is_some() && b.slot.is_some());
    let mut third = connect("test-key-only", hash);
    pump(&mut server, &mut [&mut a, &mut b, &mut third]);
    assert!(third.error.is_some());
    drop(third);
    assert_eq!(a.state.as_ref().unwrap().players.len(), 2);
    let rogue = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    rogue.send_to(&vec![b'x'; 2000], addr).unwrap();
    rogue.send_to(b"{garbage", addr).unwrap();
    rogue
        .send_to(
            &feta_net::encode(&Wire::Command {
                token: [0; 2],
                sequence: 1,
                round: 0,
                action: Action::Select(Role::Feta),
            })
            .unwrap(),
            addr,
        )
        .unwrap();
    pump(&mut server, &mut [&mut a, &mut b]);
    assert!(server
        .game
        .players
        .iter()
        .flatten()
        .all(|p| p.role.is_none()));
    a.command(Action::Select(Role::Scientist));
    b.command(Action::Select(Role::Feta));
    pump(&mut server, &mut [&mut a, &mut b]);
    assert_eq!(
        server.game.players[a.slot.unwrap()].as_ref().unwrap().role,
        Some(Role::Scientist)
    );
    assert_eq!(
        server.game.players[b.slot.unwrap()].as_ref().unwrap().role,
        Some(Role::Feta)
    );
    a.disconnect();
    pump(&mut server, &mut [&mut b]);
    assert_eq!(server.peers(), 1);
}
#[test]
fn map_spawn_clearance_and_rat_shortcut_are_real() {
    let room = feta::map().unwrap();
    assert_eq!(room.name, "Briar House");
    for role in [Role::Feta, Role::Scientist] {
        let mut c = feta::spawn(role);
        let before = c.position;
        c.update(Movement::default(), feta::DT, &room.colliders);
        assert!((c.position - before).length() < 0.01);
    }
    let mut rat = feta::spawn(Role::Feta);
    rat.set_physics_state(V(-1.3, 3.42, -2.7), 0., true);
    rat.yaw = std::f32::consts::FRAC_PI_2;
    for _ in 0..20 {
        rat.update(
            Movement {
                forward: 1.,
                ..Default::default()
            },
            feta::DT,
            &room.colliders,
        );
    }
    assert!(
        rat.position.0 > -0.3,
        "rat shortcut blocked: {:?}",
        rat.position
    );
    let mut human = feta::spawn(Role::Scientist);
    human.set_physics_state(V(-1.3, 4.88, -2.7), 0., true);
    human.yaw = std::f32::consts::FRAC_PI_2;
    for _ in 0..30 {
        human.update(
            Movement {
                forward: 1.,
                ..Default::default()
            },
            feta::DT,
            &room.colliders,
        );
    }
    assert!(human.position.0 < -0.7);
}

#[test]
fn lag_compensation_uses_bounded_server_history() {
    let mut g = lobby();
    hunt(&mut g);
    g.players[0]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 1.68, 4.), 0., true);
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(0., 0.22, 2.), 0., true);
    g.step();
    let viewed = g.tick;
    let target = g.players[1].as_ref().unwrap().controller.position;
    let shooter = g.players[0].as_ref().unwrap().controller.position;
    let d = (target - shooter).norm();
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(2., 0.22, 2.), 0., true);
    for _ in 0..5 {
        g.step();
    }
    g.input(
        0,
        Input {
            sequence: 1,
            round: g.round,
            aim_tick: viewed,
            yaw: d.0.atan2(-d.2),
            pitch: d.1.asin(),
            shot: 1,
            ..Default::default()
        },
    );
    g.step();
    assert_eq!(g.outcome, Some(Outcome::Scientist));
    let mut g = lobby();
    hunt(&mut g);
    g.players[0]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(shooter, 0., true);
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(target, 0., true);
    g.step();
    let old = g.tick;
    g.players[1]
        .as_mut()
        .unwrap()
        .controller
        .set_physics_state(V(2., 0.22, 2.), 0., true);
    for _ in 0..30 {
        g.step();
    }
    g.input(
        0,
        Input {
            sequence: 1,
            round: g.round,
            aim_tick: old,
            yaw: d.0.atan2(-d.2),
            pitch: d.1.asin(),
            shot: 1,
            ..Default::default()
        },
    );
    g.step();
    assert_eq!(g.phase, Phase::Hunting);
}
