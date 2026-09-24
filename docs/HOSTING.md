# Hosting Feta on Kevin's Linux machine

The server listens only on its Tailscale address, `100.92.249.99:4000/udp`.
No public router forwarding is required. GitHub stores the code and downloads; it
does not run the live game server.

## Before sharing with your cousin

1. Sign into https://login.tailscale.com/admin/acls as the tailnet owner.
2. Review `deploy/tailscale-policy.example.json`. It preserves a member's access to
   their own machines while allowing externally shared users only UDP 4000 on this
   server. Remove the default allow-everything rule: adding a narrow grant alongside
   a broad grant does **not** revoke the broad access. Preserve any intentional rules
   your existing network needs. This project cannot assume your cloud policy is saved.
3. In the policy editor, verify your cousin can reach this server on UDP 4000 and
   cannot reach TCP 22 or other services. Save the policy before sending invitations.
4. At https://login.tailscale.com/admin/machines, open **debian-pc → Share**. Create
   an invitation for your cousin's own account and send the link privately. Sharing
   one machine avoids granting membership of your whole network.
5. Give him `docs/PLAY_WITH_KEVIN.md` (also in the Windows ZIP) and the join key privately.
6. Install/sign into Tailscale on your own gaming PC too. Both players then follow
   the same in-game connection steps.

Reference: [Tailscale sharing](https://tailscale.com/docs/features/sharing) and
[grant syntax](https://tailscale.com/docs/reference/syntax/grants).

## Build and service

```sh
cargo build --release --locked --no-default-features --bin feta-server
install -Dm755 target/release/feta-server ~/.local/lib/feta/feta-server
install -Dm644 deploy/feta-server.service ~/.config/systemd/user/feta-server.service
```

Store `FETA_JOIN_KEY=your-private-key` in `~/.config/feta/server.env` with file mode
600 and directory mode 700. The key must be 8..128 bytes. Do not put it in argv,
Git, a release asset, or a public issue. For special characters, use systemd's
EnvironmentFile quoting rules. Restart the service after rotating it.

```sh
systemctl --user daemon-reload
systemctl --user enable --now feta-server
systemctl --user status feta-server --no-pager
journalctl --user -u feta-server -n 30 --no-pager
```

To survive logout and start at boot, enable user lingering once:

```sh
sudo loginctl enable-linger kevin
```

Tailscale must remain logged in and the machine must stay awake. The service retries
if the Tailscale address is unavailable during startup. The restart policy handles
process failure; this is not a guarantee of uninterrupted hosting.

Stop with `systemctl --user stop feta-server`. Before updating the binary, stop the
service, install the new binary, then start it again. Notify both players to use the
matching GitHub release. Logs print connection counts, phase and simulation timing,
not join keys or session tokens.

## Validation and diagnosis

```sh
cargo test --locked --no-default-features --test feta_game
cargo clippy --all-targets --locked --no-default-features -- -D warnings
python3 tools/check_headless.py
tailscale status
ss -lun | rg ':4000'
```

For a bounded local run, provide FETA_JOIN_KEY in the environment and use
`feta-server --listen 127.0.0.1:4000 --ticks 600`. Do not launch a second instance
on the live port. UDP testing needs permission to bind local sockets; the agent's
restricted sandbox blocks these, although the host supports them.
