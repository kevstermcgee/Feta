//! Private dedicated Feta server. Join key is read from FETA_JOIN_KEY, never from argv.
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use vesper3d::viewer::{feta, feta_net::Server};
fn main() -> vesper3d::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut address = "127.0.0.1:4000".to_string();
    let mut ticks = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                i += 1;
                address = args.get(i).ok_or("--listen requires IP:port")?.clone();
            }
            "--ticks" => {
                i += 1;
                ticks = Some(
                    args.get(i)
                        .ok_or("--ticks requires a positive count")?
                        .parse::<u64>()?,
                );
            }
            "--help" => {
                println!("feta-server [--listen IP:4000] [--ticks N]\nSet FETA_JOIN_KEY in a private environment file. QUIC/TLS encryption is built in. Set FETA_TLS_KEY_FILE to the server private key.");
                return Ok(());
            }
            other => return Err(format!("Unknown argument: {other}").into()),
        }
        i += 1;
    }
    let _: SocketAddr = address.parse()?;
    if ticks == Some(0) {
        return Err("--ticks must be positive".into());
    }
    let key = std::env::var("FETA_JOIN_KEY")
        .map_err(|_| "Set FETA_JOIN_KEY before starting the server")?;
    let mut server = Server::bind(&address, key)?;
    println!(
        "Feta {} | {} | {}",
        env!("CARGO_PKG_VERSION"),
        server.address()?,
        server.game.room.name
    );
    let mut next = Instant::now();
    let mut max_us = 0;
    let mut total_us = 0u128;
    loop {
        let started = Instant::now();
        server.step()?;
        let elapsed = started.elapsed().as_micros();
        max_us = max_us.max(elapsed);
        total_us += elapsed;
        if server.game.tick.is_multiple_of(3600) {
            println!(
                "tick={} peers={} phase={:?} mean_us={} max_us={}",
                server.game.tick,
                server.peers(),
                server.game.phase,
                total_us / 3600,
                max_us
            );
            max_us = 0;
            total_us = 0;
        }
        if ticks.is_some_and(|n| server.game.tick >= n) {
            break;
        }
        next += Duration::from_secs_f64(1. / feta::HZ as f64);
        // Do not burst indefinitely after suspension or heavy load.
        if next + Duration::from_millis(133) < Instant::now() {
            next = Instant::now();
        }
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    println!("Clean exit after {} ticks", server.game.tick);
    Ok(())
}
