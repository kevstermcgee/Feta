#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! Feta desktop client. The map, character artwork and sounds are embedded.
mod character;
mod feta_input;
mod impact_audio;
mod wrench_view;
use macroquad::{
    input::utils::{register_input_subscriber, repeat_all_miniquad_input},
    prelude::*,
};
use std::{collections::VecDeque, net::SocketAddr};
use vesper3d::{
    math::V,
    viewer::{
        camera::{CameraRig, Perspective},
        controller::{CharacterKind, Movement},
        feta::{self, Action, Input, Match, Outcome, Phase, State},
        feta_net::Client,
        mesh,
        weapons::Pistol,
        wrench::Wrench,
    },
};
const INK: Color = Color::new(0.92, 0.94, 0.91, 1.);
const MUTED: Color = Color::new(0.59, 0.65, 0.66, 1.);
const ACCENT: Color = Color::new(0.46, 0.78, 0.64, 1.);
fn icon() -> Option<miniquad::conf::Icon> {
    let image =
        Image::from_file_with_format(include_bytes!("../../assets/branding/blueengine.png"), None)
            .ok()?;
    fn sample<const N: usize>(image: &Image, side: usize) -> [u8; N] {
        let mut bytes = [0; N];
        for y in 0..side {
            for x in 0..side {
                let src = ((y * image.height as usize / side) * image.width as usize
                    + x * image.width as usize / side)
                    * 4;
                bytes[(y * side + x) * 4..(y * side + x) * 4 + 4]
                    .copy_from_slice(&image.bytes[src..src + 4]);
            }
        }
        bytes
    }
    Some(miniquad::conf::Icon {
        small: sample(&image, 16),
        medium: sample(&image, 32),
        big: sample(&image, 64),
    })
}
fn config() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Feta".into(),
            icon: icon(),
            window_width: 1100,
            window_height: 720,
            high_dpi: true,
            sample_count: 4,
            window_resizable: true,
            ..Default::default()
        },
        draw_call_vertex_capacity: 30000,
        draw_call_index_capacity: 30000,
        ..Default::default()
    }
}
fn label(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(s, x, y, size, color);
}
fn ui_camera() {
    let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 960., 640.));
    camera.zoom.y = -camera.zoom.y;
    set_camera(&camera);
}
fn mouse() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x * 960. / screen_width(), y * 640. / screen_height())
}
fn button(s: &str, x: f32, y: f32, w: f32, enabled: bool) -> bool {
    let r = Rect::new(x, y, w, 42.);
    let hover = r.contains(mouse());
    draw_rectangle(
        x,
        y,
        w,
        42.,
        if enabled && hover {
            Color::new(0.23, 0.36, 0.32, 1.)
        } else {
            Color::new(0.12, 0.18, 0.18, 1.)
        },
    );
    draw_rectangle_lines(x, y, w, 42., 1., if enabled { ACCENT } else { MUTED });
    let size = 22.;
    let width = measure_text(s, None, size as u16, 1.).width;
    label(
        s,
        x + (w - width) / 2.,
        y + 28.,
        size,
        if enabled { INK } else { MUTED },
    );
    enabled && hover && is_mouse_button_pressed(MouseButton::Left)
}
fn panel(title: &str, subtitle: &str) {
    draw_rectangle(220., 52., 520., 536., Color::new(0.045, 0.07, 0.075, 0.97));
    label(title, 260., 112., 48., INK);
    label(subtitle, 260., 145., 20., MUTED);
}
fn field(value: &mut String, x: f32, y: f32, active: bool, secret: bool) -> bool {
    let r = Rect::new(x, y, 440., 42.);
    draw_rectangle(x, y, 440., 42., Color::new(0.09, 0.13, 0.14, 1.));
    draw_rectangle_lines(x, y, 440., 42., 1., if active { ACCENT } else { MUTED });
    if active {
        let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if control && is_key_pressed(KeyCode::V) {
            if let Some(paste) = miniquad::window::clipboard_get() {
                *value = paste
                    .trim()
                    .chars()
                    .filter(|c| c.is_ascii() && !c.is_control())
                    .take(128)
                    .collect();
            }
        }
        while let Some(c) = get_char_pressed() {
            if !control && c.is_ascii() && !c.is_control() && value.len() < 128 {
                value.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            value.pop();
        }
    }
    let shown = if secret {
        "*".repeat(value.len().min(40))
    } else {
        value.clone()
    };
    label(&shown, x + 12., y + 28., 22., INK);
    r.contains(mouse()) && is_mouse_button_pressed(MouseButton::Left)
}
fn time(ticks: u64) -> String {
    let seconds = ticks.div_ceil(60);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn outcome(o: Option<Outcome>) -> &'static str {
    match o {
        Some(Outcome::Feta) => "Feta escaped!",
        Some(Outcome::Scientist) => "Scientist wins!",
        _ => "Player disconnected",
    }
}
fn own(state: &State, slot: usize) -> Option<&vesper3d::viewer::feta::PlayerView> {
    state.players.iter().find(|p| p.slot == slot)
}

#[macroquad::main(config)]
async fn main() {
    if let Err(e) = run().await {
        let message = e.to_string();
        feta_input::capture(false);
        loop {
            clear_background(BLACK);
            draw_text("Feta could not start", 25., 60., 30., WHITE);
            for (i, part) in message.as_bytes().chunks(85).enumerate() {
                draw_text(
                    &String::from_utf8_lossy(part),
                    25.,
                    110. + i as f32 * 25.,
                    20.,
                    WHITE,
                );
            }
            draw_text(
                "Press Escape to close",
                25.,
                screen_height() - 30.,
                20.,
                WHITE,
            );
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
            next_frame().await;
        }
    }
}
async fn run() -> vesper3d::Result<()> {
    clear_background(Color::new(0.045, 0.07, 0.075, 1.));
    draw_text("Feta / preparing Briar House...", 40., 80., 30., INK);
    next_frame().await;
    let room = feta::map()?;
    let hash = feta::content_id();
    let meshes = mesh::bake_tagged(&room.world, &room.render_tags());
    let material = mesh::material()?;
    let mut tool = wrench_view::View::new();
    let mut pistol_view = wrench_view::View::pistol();
    let mut pistol = Pistol::default();
    let mut wrench = Wrench::default();
    let mut shot_audio = impact_audio::ImpactAudio::pistol().await;
    let mut hit_audio = impact_audio::ImpactAudio::new().await;
    let mut body = character::Character::default();
    let mut remote_body = character::Character::default();
    let mut controller = feta::spawn(CharacterKind::Feta);
    let mut previous = controller.clone();
    let mut camera = CameraRig::default();
    let mut perspective = Perspective::First;
    let mut client: Option<Client> = None;
    let mut solo = None;
    let mut solo_select = false;
    let mut solo_jump = 0;
    let mut server = feta::DEFAULT_SERVER.to_string();
    let mut key = String::new();
    let mut active_field = 1;
    let mut message = String::new();
    let mut paused = false;
    let mut grabbed = false;
    let mut sensitivity = 0.0025;
    let mut fov = 75_f32;
    let mut settings_open = false;
    let mut keys = feta_input::Keys::default();
    let subscriber = register_input_subscriber();
    let mut input = Input::default();
    let mut accumulator = 0_f32;
    let mut history: VecDeque<Input> = VecDeque::new();
    let mut snapshots: VecDeque<State> = VecDeque::new();
    let mut received_at = get_time();
    let mut last_round = 0;
    let mut last_phase = Phase::Lobby;
    let mut last_role = None;
    let mut correction = V::ZERO;
    let mut visual_shots = 0;
    let mut visual_swings = 0;
    let args: Vec<_> = std::env::args().collect();
    let capture_dir = args
        .windows(2)
        .find(|a| a[0] == "--capture")
        .map(|a| std::path::PathBuf::from(&a[1]));
    if let Some(dir) = &capture_dir {
        std::fs::create_dir(dir)?;
    }
    let mut frame = 0;
    loop {
        frame += 1;
        let dt = get_frame_time().clamp(0., 0.133);
        repeat_all_miniquad_input(&mut keys, subscriber);
        keys.poll(feta_input::foreground());
        if let Some(c) = &mut client {
            match c.poll() {
                Ok(updates) => {
                    for state in updates {
                        if let Some(me) = own(&state, c.slot.unwrap_or(0)) {
                            let reset = state.round != last_round
                                || state.phase != last_phase
                                || me.role != last_role;
                            let old_position = controller.position;
                            let look = (controller.yaw, controller.pitch);
                            if reset {
                                history.clear();
                                correction = V::ZERO;
                                if state.round != last_round || me.role != last_role {
                                    input.shot = 0;
                                    input.melee = 0;
                                    input.jump = 0;
                                }
                                controller =
                                    feta::spawn(me.role.unwrap_or(CharacterKind::Scientist));
                                perspective = Perspective::First;
                                visual_shots = me.shots;
                                visual_swings = me.swings;
                                paused = false;
                                settings_open = false;
                            }
                            controller.restore_network_state(&me.pose);
                            history.retain(|i| i.sequence > me.ack);
                            if Match::can_move(state.phase, me.role) {
                                for replay in &history {
                                    controller.yaw = replay.yaw;
                                    controller.pitch = replay.pitch;
                                    controller.update(replay.movement, feta::DT, &room.colliders);
                                }
                            }
                            if !reset {
                                let error = old_position - controller.position;
                                if error.length() < 1. {
                                    correction = correction + error;
                                } else {
                                    correction = V::ZERO;
                                }
                                controller.yaw = look.0;
                                controller.pitch = look.1;
                            }
                            if me.shots > visual_shots {
                                let _ = pistol.fire(true, &room, controller.ray());
                                visual_shots = me.shots;
                            }
                            if me.swings > visual_swings {
                                wrench.start(true, true);
                                visual_swings = me.swings;
                            }
                            previous = controller.clone();
                            last_round = state.round;
                            last_phase = state.phase;
                            last_role = me.role;
                        }
                        snapshots.push_back(state);
                        while snapshots.len() > 12 {
                            snapshots.pop_front();
                        }
                        received_at = get_time();
                    }
                }
                Err(e) => c.error = Some(e.to_string()),
            }
            if let Some(error) = &c.error {
                message = error.clone();
                client = None;
                history.clear();
                snapshots.clear();
            }
        }
        let mut state = client.as_ref().and_then(|c| c.state.clone());
        if capture_dir.is_some() && (135..250).contains(&frame) {
            let mut fixture = Match::new()?;
            fixture.join(0);
            fixture.join(1);
            fixture.action(0, Action::Select(CharacterKind::Feta));
            fixture.action(1, Action::Select(CharacterKind::Scientist));
            if frame >= 230 {
                let revision = fixture.revision;
                fixture.action(0, Action::Ready { revision });
                fixture.action(1, Action::Ready { revision });
                fixture.leave(1);
            }
            state = Some(fixture.state(0));
            paused = (170..230).contains(&frame);
            settings_open = (200..230).contains(&frame);
        }
        // Local exploration uses the real controller/render path, without a network session.
        if solo.is_some() {
            state = Some(State {
                tick: 0,
                round: 0,
                revision: 0,
                phase: Phase::Hunting,
                remaining: 0,
                outcome: None,
                settings: Default::default(),
                players: vec![feta::PlayerView {
                    slot: 0,
                    role: solo,
                    ready: false,
                    pose: controller.network_state(),
                    ack: 0,
                    shots: 0,
                    swings: 0,
                }],
            });
        }
        let slot = client.as_ref().and_then(|c| c.slot).unwrap_or(0);
        let phase = state.as_ref().map_or(Phase::Lobby, |s| s.phase);
        let role = state
            .as_ref()
            .and_then(|s| own(s, slot))
            .and_then(|p| p.role);
        let playing = matches!(phase, Phase::Hiding | Phase::Hunting);
        if keys.pressed(KeyCode::Escape) && (client.is_some() || solo.is_some()) {
            paused = !paused;
            settings_open = false;
        }
        if !feta_input::foreground() && playing {
            paused = true;
        }
        let active = capture_dir.is_none()
            && playing
            && !paused
            && Match::can_move(phase, role)
            && feta_input::foreground();
        if active != grabbed {
            feta_input::capture(active);
            grabbed = active;
        }
        if active {
            let delta = mouse_delta_position();
            controller.look(
                -delta.x * screen_width() / 2.,
                -delta.y * screen_height() / 2.,
                sensitivity,
                false,
            );
            if keys.pressed(KeyCode::Q) {
                perspective.toggle();
            }
            if keys.pressed(KeyCode::Space) {
                input.jump += 1;
            }
            if phase == Phase::Hunting && role == Some(CharacterKind::Scientist) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    input.shot += 1;
                    if solo.is_some() {
                        let _ = pistol.fire(true, &room, controller.ray());
                    }
                }
                if is_mouse_button_pressed(MouseButton::Right) {
                    input.melee += 1;
                    if solo.is_some() {
                        wrench.start(true, true);
                    }
                }
            }
        }
        let (forward, right) = if active {
            feta_input::axes(&keys.down)
        } else {
            (0., 0.)
        };
        let movement = Movement {
            forward,
            right,
            sprint: active
                && (keys.down.contains(&KeyCode::LeftShift)
                    || keys.down.contains(&KeyCode::RightShift)),
            crouch: active
                && (keys.down.contains(&KeyCode::C)
                    || keys.down.contains(&KeyCode::LeftControl)
                    || keys.down.contains(&KeyCode::RightControl)),
            jump: false,
        };
        accumulator += dt;
        let mut jump_applied = if solo.is_some() {
            solo_jump
        } else {
            history.back().map_or(input.jump, |h| h.jump)
        };
        while accumulator >= feta::DT {
            accumulator -= feta::DT;
            previous = controller.clone();
            input.sequence += 1;
            input.round = last_round;
            input.aim_tick = state.as_ref().map_or(0, |s| {
                (s.tick as f64 + (get_time() - received_at) * 60. - 6.).max(0.) as u64
            });
            input.yaw = controller.yaw;
            input.pitch = controller.pitch;
            input.movement = movement;
            input.movement.jump = input.jump > jump_applied;
            jump_applied = input.jump;
            solo_jump = input.jump;
            if let Some(c) = &client {
                if let Err(e) = c.send_input(input.clone()) {
                    message = e.to_string();
                }
            }
            if active {
                controller.update(input.movement, feta::DT, &room.colliders);
            }
            if client.is_some() {
                history.push_back(input.clone());
                while history.len() > 180 {
                    history.pop_front();
                }
            }
        }
        correction = correction * (-12. * dt).exp();
        let mut render = controller.interpolated(&previous, accumulator / feta::DT);
        render.set_physics_state(
            render.position + correction,
            render.vertical_velocity(),
            render.is_grounded(),
        );
        let distance = (controller.position - previous.position).length();
        body.update(distance, dt, active && distance > 0.0001);
        pistol.tick(dt);
        wrench.tick(dt, &room, controller.ray());
        shot_audio.update(pistol.shots);
        hit_audio.update(wrench.hits);
        clear_background(Color::new(0.48, 0.70, 0.86, 1.));
        // Capture mode exercises actual scene, menu and character paths without a network connection.
        let capture_scene = capture_dir.is_some() && (60..135).contains(&frame);
        let blind = phase == Phase::Hiding && role == Some(CharacterKind::Scientist);
        let view = if playing && !blind || capture_scene {
            if capture_scene {
                render = feta::spawn(CharacterKind::Feta);
                if frame < 100 {
                    render.set_physics_state(V(-2.2, 0.22, -3.4), 0., true);
                    render.yaw = 0.7;
                } else {
                    render.set_physics_state(V(0., 0.22, -9.), 0., true);
                    render.yaw = 0.;
                }
            }
            camera.advance(perspective, &render, &room, dt);
            camera.view(perspective, &render, &room)
        } else {
            vesper3d::viewer::camera::View {
                eye: V(8.5, 7.8, 11.),
                target: V(0., 2., 0.),
                show_body: false,
            }
        };
        set_camera(&Camera3D {
            position: mesh::vec(view.eye),
            target: mesh::vec(view.target),
            up: Vec3::Y,
            fovy: fov.to_radians(),
            z_near: 0.025,
            z_far: 80.,
            ..Default::default()
        });
        material.set_uniform("Eye", mesh::vec(view.eye));
        material.set_uniform("ObjectStates", vec2(1., 0.));
        gl_use_material(&material);
        for m in &meshes {
            draw_mesh(m);
        }
        gl_use_default_material();
        if view.show_body {
            body.draw(&render, &wrench, &pistol_view, false);
        }
        if playing && !blind {
            if let Some(latest) = snapshots.back() {
                let tick = latest.tick as f64 + (get_time() - received_at) * 60. - 6.;
                let pair = snapshots
                    .iter()
                    .zip(snapshots.iter().skip(1))
                    .find(|(a, b)| tick >= a.tick as f64 && tick <= b.tick as f64);
                let (a, b, t) = pair.map_or((latest, latest, 1.), |(a, b)| {
                    (
                        a,
                        b,
                        ((tick - a.tick as f64) / (b.tick - a.tick) as f64) as f32,
                    )
                });
                if let Some(p) = own(b, 1 - slot) {
                    let mut remote = feta::spawn(p.role.unwrap_or(CharacterKind::Scientist));
                    remote.restore_network_state(&p.pose);
                    if let Some(old) = own(a, 1 - slot) {
                        let mut before = remote.clone();
                        before.restore_network_state(&old.pose);
                        remote = remote.interpolated(&before, t);
                    }
                    remote_body.update(0.03, dt, true);
                    remote_body.draw(&remote, &Wrench::default(), &pistol_view, false);
                }
            }
        }
        set_default_camera();
        if playing && !blind && role == Some(CharacterKind::Scientist) {
            if wrench.phase().is_some() {
                tool.draw(&wrench);
            } else {
                pistol_view.draw_pistol(&pistol);
            }
        }
        ui_camera();
        let mut disconnect = false;
        let mut quit = false;
        let mut start_solo = None;
        if capture_dir.is_some() {
            match frame {
                250 => {
                    solo_select = true;
                    paused = false;
                    settings_open = false;
                }
                270 => start_solo = Some(CharacterKind::Feta),
                310 => start_solo = Some(CharacterKind::Scientist),
                340 => paused = true,
                _ => {}
            }
        }
        if capture_scene && frame < 135 {
            label("BRIAR HOUSE", 30., 40., 20., INK);
        } else if solo_select {
            panel(
                "Explore solo",
                "Briar House / no timer or connection needed",
            );
            if button("Explore as Feta", 260., 220., 440., true) {
                start_solo = Some(CharacterKind::Feta);
            }
            if button("Explore as Scientist", 260., 282., 440., true) {
                start_solo = Some(CharacterKind::Scientist);
            }
            label(
                "Try movement, hiding spots and Scientist tools.",
                260.,
                380.,
                19.,
                MUTED,
            );
            label(
                "Esc opens settings or lets you change character.",
                260.,
                411.,
                19.,
                MUTED,
            );
            if button("Back", 260., 526., 440., true) || keys.pressed(KeyCode::Escape) {
                solo_select = false;
            }
        } else if client.is_none()
            && solo.is_none()
            && (capture_dir.is_none() || !(135..250).contains(&frame))
        {
            panel("Feta", "A small rat. A big head start.");
            label("Server", 260., 188., 20., MUTED);
            if field(&mut server, 260., 202., active_field == 0, false) {
                active_field = 0;
            }
            label("Join key", 260., 280., 20., MUTED);
            if field(&mut key, 260., 294., active_field == 1, true) {
                active_field = 1;
            }
            if keys.pressed(KeyCode::Tab) {
                active_field = 1 - active_field;
            }
            if button("Connect", 260., 366., 440., true) || keys.pressed(KeyCode::Enter) {
                match server
                    .trim()
                    .parse::<SocketAddr>()
                    .map_err(|_| {
                        "Use an IP address and port, such as 192.168.0.109:4000".to_string()
                    })
                    .and_then(|addr| {
                        Client::connect(addr, key.clone(), hash).map_err(|e| e.to_string())
                    }) {
                    Ok(c) => {
                        client = Some(c);
                        message.clear();
                        key.clear();
                        paused = false;
                        history.clear();
                        snapshots.clear();
                        last_role = None;
                    }
                    Err(e) => message = e,
                }
            }
            if button("Explore solo", 260., 420., 440., true) {
                solo_select = true;
                message.clear();
            }
            for (i, line) in message.as_bytes().chunks(49).take(2).enumerate() {
                label(
                    &String::from_utf8_lossy(line),
                    260.,
                    485. + i as f32 * 23.,
                    18.,
                    ACCENT,
                );
            }
            if button("Quit", 260., 526., 440., true) {
                quit = true;
            }
        } else if let Some(state) = state.as_ref() {
            if paused {
                panel(
                    if settings_open { "Settings" } else { "Paused" },
                    if solo.is_some() {
                        "Solo exploration / take your time."
                    } else {
                        "Online rounds keep running while this menu is open."
                    },
                );
                if settings_open {
                    label(
                        &format!("Sensitivity  {:.1}", sensitivity * 1000.),
                        260.,
                        218.,
                        24.,
                        INK,
                    );
                    if button("-", 550., 187., 65., true) {
                        sensitivity = (sensitivity - 0.0005_f32).max(0.0005);
                    }
                    if button("+", 635., 187., 65., true) {
                        sensitivity = (sensitivity + 0.0005_f32).min(0.008);
                    }
                    label(&format!("Field of view  {fov:.0}"), 260., 285., 24., INK);
                    if button("-", 550., 254., 65., true) {
                        fov = (fov - 5.).max(55.);
                    }
                    if button("+", 635., 254., 65., true) {
                        fov = (fov + 5.).min(100.);
                    }
                    label(
                        "WASD / arrows   Move    Shift   Sprint",
                        260.,
                        352.,
                        20.,
                        MUTED,
                    );
                    label("Space   Jump    Ctrl / C   Crouch", 260., 383., 20., MUTED);
                    label(
                        "Left click   Shoot    Right click   Wrench",
                        260.,
                        414.,
                        20.,
                        MUTED,
                    );
                    label("Q   Toggle camera    Esc   Menu", 260., 445., 20., MUTED);
                    if button("Back", 260., 500., 440., true) {
                        settings_open = false;
                    }
                } else {
                    if button("Resume", 260., 220., 440., true) {
                        paused = false;
                    }
                    if button("Settings", 260., 282., 440., true) {
                        settings_open = true;
                    }
                    if solo.is_some() && button("Change character", 260., 344., 440., true) {
                        solo = None;
                        solo_select = true;
                        paused = false;
                    }
                    if button(
                        if solo.is_some() || solo_select {
                            "Main menu"
                        } else {
                            "Disconnect"
                        },
                        260.,
                        406.,
                        440.,
                        true,
                    ) {
                        disconnect = true;
                    }
                    if button("Quit", 260., 468., 440., true) {
                        quit = true;
                    }
                }
            } else if phase == Phase::Lobby {
                let me = own(state, slot);
                let busy = client.as_ref().is_some_and(|c| c.pending());
                panel("Feta", "Briar House / choose your side");
                let taken = |r| {
                    state
                        .players
                        .iter()
                        .any(|p| p.slot != slot && p.role == Some(r))
                };
                if button(
                    if role == Some(CharacterKind::Feta) {
                        "Feta / selected"
                    } else {
                        "Feta"
                    },
                    260.,
                    180.,
                    210.,
                    !busy && !taken(CharacterKind::Feta),
                ) {
                    client
                        .as_mut()
                        .unwrap()
                        .command(Action::Select(CharacterKind::Feta));
                }
                if button(
                    if role == Some(CharacterKind::Scientist) {
                        "Scientist / selected"
                    } else {
                        "Scientist"
                    },
                    490.,
                    180.,
                    210.,
                    !busy && !taken(CharacterKind::Scientist),
                ) {
                    client
                        .as_mut()
                        .unwrap()
                        .command(Action::Select(CharacterKind::Scientist));
                }
                let host = state.players.iter().map(|p| p.slot).min() == Some(slot);
                let mut settings = state.settings;
                label(
                    &format!("Hunt   {}", time(settings.hunt_seconds as u64 * 60)),
                    260.,
                    283.,
                    26.,
                    INK,
                );
                if button("-", 550., 252., 65., host && !busy) {
                    settings.hunt_seconds = settings.hunt_seconds.saturating_sub(30).max(30);
                }
                if button("+", 635., 252., 65., host && !busy) {
                    settings.hunt_seconds = (settings.hunt_seconds + 30).min(1800);
                }
                label(
                    &format!("Head start   {}", time(settings.hide_seconds as u64 * 60)),
                    260.,
                    351.,
                    26.,
                    INK,
                );
                if button("-", 550., 320., 65., host && !busy) {
                    settings.hide_seconds = settings.hide_seconds.saturating_sub(5).max(5);
                }
                if button("+", 635., 320., 65., host && !busy) {
                    settings.hide_seconds = (settings.hide_seconds + 5).min(300);
                }
                if settings != state.settings {
                    client
                        .as_mut()
                        .unwrap()
                        .command(Action::Configure(settings));
                }
                label(
                    if host {
                        "You set the times. Both players confirm Ready."
                    } else {
                        "The first player sets the times. Confirm Ready."
                    },
                    260.,
                    397.,
                    18.,
                    MUTED,
                );
                let ready = me.is_some_and(|p| p.ready);
                if button(
                    if ready { "Ready / waiting" } else { "Ready" },
                    260.,
                    423.,
                    440.,
                    role.is_some() && !ready && !busy && settings == state.settings,
                ) {
                    client.as_mut().unwrap().command(Action::Ready {
                        revision: state.revision,
                    });
                }
                label(
                    if state.players.len() < 2 {
                        "Waiting for the second player..."
                    } else {
                        "One Feta + one Scientist. Both ready to begin."
                    },
                    260.,
                    495.,
                    19.,
                    MUTED,
                );
                if button("Disconnect", 260., 526., 440., true) {
                    disconnect = true;
                }
            } else if phase == Phase::Finished {
                panel(
                    outcome(state.outcome),
                    "Choose your characters again for the next round.",
                );
                label(
                    &format!("Back to lobby in {}", time(state.remaining)),
                    260.,
                    270.,
                    28.,
                    ACCENT,
                );
            } else if blind {
                draw_rectangle(0., 0., 960., 640., Color::new(0.045, 0.07, 0.075, 1.));
                label("Let Feta find a hiding place.", 260., 302., 32., INK);
                label(
                    "You will be released when the head start ends.",
                    260.,
                    338.,
                    20.,
                    MUTED,
                );
                label(&time(state.remaining), 866., 34., 20., MUTED);
            } else {
                let s = state;
                label(
                    if solo == Some(CharacterKind::Feta) {
                        "Exploring as Feta"
                    } else if solo == Some(CharacterKind::Scientist) {
                        "Exploring as Scientist"
                    } else if phase == Phase::Hiding {
                        "Find a hiding place"
                    } else {
                        "The hunt is on"
                    },
                    25.,
                    34.,
                    19.,
                    INK,
                );
                if solo.is_none() {
                    label(&time(s.remaining), 866., 34., 20., INK);
                }
                if role == Some(CharacterKind::Scientist) {
                    draw_circle(480., 320., 2., Color::new(1., 1., 1., 0.8));
                }
                label("Esc  Menu", 25., 614., 17., MUTED);
            }
        } else {
            panel("Connecting...", "Finding your private game server.");
            if button("Cancel", 260., 480., 440., true) {
                disconnect = true;
            }
        }
        set_default_camera();
        if let Some(dir) = &capture_dir {
            let name = match frame {
                45 => Some("menu"),
                90 => Some("under-table"),
                120 => Some("garden"),
                150 => Some("lobby"),
                190 => Some("pause"),
                220 => Some("settings"),
                245 => Some("result"),
                260 => Some("solo-select"),
                290 => Some("solo-feta"),
                330 => Some("solo-scientist"),
                350 => Some("solo-pause"),
                _ => None,
            };
            if let Some(name) = name {
                get_screen_data().export_png(dir.join(format!("{name}.png")).to_str().unwrap());
            }
            if frame >= 351 {
                break;
            }
        }
        if let Some(role) = start_solo {
            client = None;
            solo = Some(role);
            solo_select = false;
            paused = false;
            settings_open = false;
            controller = feta::spawn(role);
            previous = controller.clone();
            camera = CameraRig::default();
            perspective = Perspective::First;
            input = Input::default();
            solo_jump = 0;
            accumulator = 0.;
            correction = V::ZERO;
            snapshots.clear();
            history.clear();
            // Preserve audio event counters while clearing weapon animations/cooldowns.
            let shots = pistol.shots;
            let hits = wrench.hits;
            pistol = Pistol::default();
            pistol.shots = shots;
            wrench = Wrench::default();
            wrench.hits = hits;
        }
        if disconnect {
            solo = None;
            solo_select = false;
            client = None;
            snapshots.clear();
            history.clear();
            message.clear();
            paused = false;
        }
        if quit {
            break;
        }
        next_frame().await;
    }
    feta_input::capture(false);
    Ok(())
}
