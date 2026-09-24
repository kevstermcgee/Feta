# Feta validation — 2026-09-24

Friends playtest based on BlueEngine 8a667e7, using QUIC/TLS 1.3.

- Full `python3 tools/be2.py check` passed for the encrypted revision: default and
  headless tests, Python authoring tests, formatting, Clippy and rustdoc with
  warnings denied in both configurations, and the headless dependency boundary.
- Nine Feta tests cover role/settings readiness, hiding freeze and state privacy,
  survival/rematch, shooting and walls, melee, disconnect, replayed input,
  bounded lag compensation, packet size, authentication/capacity/commands,
  wrong certificate and plaintext rejection, and rat-only map clearance.
- Two real QUIC clients connected to a release Linux server using its production
  certificate and completed a 5-second hide, 30-second hunt, survival result and
  rematch lobby in 40.78 seconds, receiving 816 snapshots each.
- Linux release client and isolated headless server built successfully.
- Rendered menu, lobby, pause, settings, result, under-table rat camera and garden
  captures inspected using an off-screen Linux display.
- Router confirmed a renewable one-hour UDP 4000 mapping. This alone does not
  establish outside-network reachability; the Windows external test is pending.

Check report: `.be2-work/check-20260924T215106504224Z/report.json`.
Visual evidence: `previews/feta/` (scripted fixture states, not manual gameplay).

Windows builds/tests are performed by GitHub Actions. Physical Windows input,
cursor capture, audio listening and human map balance still need real PC
playtesting. Automated Linux rendering does not establish those results.
There is no claim of production availability or measured internet latency.
