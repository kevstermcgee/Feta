# Feta validation — 2026-09-24

First playable build on Linux, based on BlueEngine 8a667e7.

- Full `python3 tools/be2.py check` passed: 165 default-feature tests and examples,
  142 headless tests and examples, four Python authoring tests, formatting,
  Clippy and rustdoc with warnings denied in both feature configurations, and
  the headless dependency boundary.
- Nine Feta-specific tests cover role readiness/settings revisions, hiding freeze
  and state privacy, survival/rematch reset, shooting and walls, delayed melee,
  disconnect cancellation, invalid/replayed input, bounded lag compensation,
  packet size, UDP authentication/capacity/commands, and rat-only map clearance.
- Live release server: two real UDP clients completed a 5-second hide, 30-second hunt,
  Feta survival result and rematch lobby in 40.78 seconds; each received 816
  snapshots. Server exited cleanly after 3,300 ticks.
- Linux release client and isolated headless server built successfully.
- Actual rendered menu, lobby, pause, settings, result, under-table rat camera and
  garden captures inspected. The upside-down UI discovered in the first capture
  was fixed before publication. Captures use an off-screen Linux X display.

Recorded check report: `.be2-work/check-20260924T212659754778Z/report.json`.
Visual evidence: `previews/feta/` (scripted fixture states, not manual gameplay).

Windows builds/tests are performed by GitHub Actions. The first downloadable
release is a friends' playtest: physical Windows input/cursor capture, audio
listening, cross-machine Tailscale access, and human map balance still need real
PC playtesting. Automated Linux rendering does not establish those results.
There is no claim of production availability or measured internet latency.
