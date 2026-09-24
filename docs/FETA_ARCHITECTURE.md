# Feta runtime

Feta is a game fork of BlueEngine commit 8a667e7. The original package/library names
remain `be2`/`vesper3d`; shipping binaries are `feta` and `feta-server`.

- `viewer/feta.rs`: two-slot match, ready/settings revision, hide/hunt/result timers,
  authoritative attacks, map selection, bounded hit history.
- `viewer/feta_net.rs`: independent versioned Feta packets, OS-random session tokens,
  join key, endpoint binding, input/command sequencing, repeated state/commands,
  bounded receives and datagrams. The old BlueEngine UDP protocol is not used by Feta. Bundled content IDs hash normalized source, avoiding platform-specific floating-point geometry differences.
- `bin/feta-server.rs`: private-interface listener, 60 Hz scheduling, metrics.
- `bin/feta.rs`: menu, connection, prediction/reconciliation, remote interpolation,
  actual game renderer and visual captures.
- `bin/feta_input/mod.rs`: inherited keyboard layouts and Windows focus checks.
- `viewer/house.rs`: Briar House variant; the original engine reference map is preserved.
- `tests/feta_game.rs`: game and UDP regression evidence.

Only two sessions may exist. The first occupied slot owns settings. Configuration
or selection changes invalidate Ready; Ready names the settings revision it confirms.
Round IDs reject delayed input/commands from prior rounds. The match starts only
with both slots ready and distinct non-empty roles. Five seconds after a result,
roles and Ready are cleared. Losing either session cancels an active round. Rejoining
returns to the lobby flow. Input older than 150 ms stops driving movement; sessions
expire after five seconds without fresh authenticated input/commands.

Movement and authoritative time use fixed ticks. Snapshots repeat at 20 Hz; no
unreliable delta baseline exists. Client actions retry until acknowledged. Fire and
jump counters repeat in input; loss does not require guessing an edge. Pistol cooldown
is 400 ms; wrench contact is 11 ticks after windup and recovery is 32 ticks. Rat hits
use a body box, closest scene geometry and collision-only barriers. Client view time
is clamped to 12 ticks (200 ms) of server-owned pose history. A shot at the exact hunt
deadline loses to survival. No client can submit a hit, position, timer, or winner.

The map is static, including props; there is no physics-prop replication or pickup.
Both roles use BlueEngine collision/movement profiles. Feta keeps the low smooth
third-person camera. Scientist uses first-person aiming to align the server ray with
the crosshair. Remote players interpolate roughly 100 ms behind; own movement predicts
and replays inputs after authoritative correction.

Security relies on Tailscale for encrypted authenticated peers. The server binary
rejects wildcard/public bind addresses. A private environment join key gates entry;
128-bit OS-random session tokens plus source endpoints gate later messages. JSON
packets are capped at 1400 bytes; receive work is bounded at 128 datagrams per poll,
Hello work at 12 requests per second. This is a trusted friends' game, not a public
competitive anti-cheat service. Join keys are not logged, embedded, or saved by the client.
See HOSTING.md for the access-policy requirement.
