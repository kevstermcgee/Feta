# Feta

<img src="assets/branding/blueengine.png" alt="Feta the white rat" width="112">

A small, private, two-player cat-and-mouse game built on BlueEngine.
One Scientist. One very quick rat. One furnished house and garden.

**[Download for Windows](https://github.com/kevstermcgee/Feta/releases)** ·
**[Player setup, step by step](docs/PLAY_WITH_KEVIN.md)** ·
**[Host setup](docs/HOSTING.md)**

Choose opposite characters and both press Ready. Feta gets 60 seconds to hide;
the Scientist then has five minutes to find and hit Feta. A shot or wrench hit wins
for the Scientist. Survive the hunt and Feta wins. After the result, choose again.
The first connected player can change both times in the lobby. Time changes clear
both Ready confirmations. The hunt duration does not include the head start.

Briar House has two floors, furnished bedrooms, a kitchen, living room, bathroom,
a fenced garden, a potting shelter, and low furniture/shortcuts for the rat. Props
are static cover. This is not prop hunt: there is no disguising or prop carrying.

| Control | Action |
|---|---|
| WASD or arrows | Move |
| Mouse | Look |
| Shift | Sprint |
| Space | Jump |
| Ctrl or C | Crouch |
| Left click | Scientist's pistol |
| Right click | Scientist's wrench |
| Q | Feta's first/third-person camera |
| Esc | Resume / Settings / Disconnect / Quit |

The server is authoritative at 60 Hz and sends small snapshots at 20 Hz. Client
movement prediction and interpolation keep presentation responsive. Hit validation
uses a bounded 200 ms history window and checks walls. During the head start the
Scientist receives no rat position. Disconnects cancel the round without awarding
a win. Pausing does not stop an online round.

## Connect directly

Players download Feta.exe and enter the server address and private join code.
QUIC/TLS encryption and pinned server verification are built in: no Tailscale,
VPN account or extra app is needed. The host maps one UDP port on the router (automatically where supported);
players do not configure their routers. See the step-by-step guides above.

## Development

```sh
cargo run --locked --no-default-features --bin feta-server -- --help
cargo run --locked --bin feta
python3 tools/be2.py check
```

The release server uses `cargo build --release --locked --no-default-features --bin feta-server`.
The client uses `cargo build --release --locked --bin feta`.
No map, audio, font or texture downloads are needed by the executable.

See [Feta architecture](docs/FETA_ARCHITECTURE.md) and [validation](docs/FETA_VALIDATION.md).
The original BlueEngine tools/binaries remain available for development; see
[the inherited README](README_BLUEENGINE.md).

## Credits and license

Game direction: Kevin Ward. Feta implementation and tooling: OpenAI Codex, working
with Kevin. Built on BlueEngine, retaining its original MIT license and history,
including the earlier Google DeepMind Antigravity contributions. The approved white
rat artwork is preserved. See [LICENSE](LICENSE).
