# Feta runtime

Feta is a game fork of BlueEngine commit 8a667e7. The original package/library names
remain `be2`/`vesper3d`; shipping binaries are `feta` and `feta-server`.

- `viewer/feta.rs`: two-slot match, ready/settings revision, hide/hunt/result timers,
  authoritative attacks, map selection, bounded hit history.
- `viewer/feta_net.rs`: independent versioned Feta packets, OS-random session tokens,
  join key, endpoint binding, input/command sequencing, repeated state/commands,
  bounded receives and datagrams. The old BlueEngine UDP protocol is not used by Feta. Bundled content IDs hash normalized source, avoiding platform-specific floating-point geometry differences.
- `bin/feta-server.rs`: encrypted UDP listener, 60 Hz scheduling, metrics.
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
Both roles use BlueEngine collision/movement profiles and begin each round in
first-person mode by default (toggled with Q). Feta retains the low smooth
third-person camera option, and Scientist uses first-person aiming to align the
server ray with the crosshair. Remote players interpolate roughly 100 ms behind; own movement predicts
and replays inputs after authoritative correction.

Security uses Quinn/Rustls QUIC datagrams with TLS 1.3 and the bundled server
certificate as the sole trust anchor. There is no insecure verifier or plaintext
fallback. `viewer/feta_secure.rs` runs one async network thread per endpoint behind
bounded queues; the simulation/render thread does not block on network I/O. Server
private keys stay outside the repository. CI uses ephemeral test identities.

At most eight handshake/transport tasks exist, with stateless address validation
before TLS allocation, handshake/data timeouts, 240 datagrams/second per connection,
no application streams and bounded queues. Only two authenticated game sessions may
exist. Game Hello processing remains limited to 12/second. JSON payloads are at most
1100 bytes, fitting QUIC's minimum-path datagram budget. Endpoint-bound random session
tokens and command sequences remain an additional defense inside encrypted transport.

A private generated join code gates game admission. The public executable contains
only the server certificate, never the join code or signing key. This is not a
competitive anti-cheat or DDoS mitigation system. Read HOSTING.md for router setup,
key storage, certificate rotation and the outside-network validation requirement.

## Local exploration

The desktop client's Explore solo mode uses the bundled map, fixed-step controller,
camera and weapon presentation directly, without constructing a network client or
changing Match readiness rules. Both characters are selectable; pause offers role
selection and return to the main menu. No opponent, round timer or score is created.
Solo jump edges persist until consumed by a simulation step. Entry resets movement,
camera, prediction history and weapons; online content/protocol IDs are unchanged.
