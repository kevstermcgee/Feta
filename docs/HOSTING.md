# Hosting Feta on Kevin's Linux machine

Feta uses QUIC datagrams encrypted with TLS 1.3. The client trusts the bundled
server certificate, not arbitrary public servers. Players only need Feta.exe.
The server runs here; GitHub distributes the code and Windows download.

## Automatic router mapping on this machine

The Archer A7 already enables UPnP. `tools/feta_portmap.py` requests only UDP 4000
with a one-hour lease. A user timer renews it every 30 minutes and discovers the
current LAN address each time. No web-admin password, router reset, address
reservation, VPN, or client-side installation is needed. Ethernet remains useful
for reliable hosting, but is not required for the mapping.

```sh
python3 tools/feta_portmap.py status
systemctl --user status feta-portmap.timer --no-pager
```

The helper refuses to overwrite an unrelated mapping or leave a permanent lease.
To withdraw internet access, stop the renewal timer and remove this rule:

```sh
systemctl --user disable --now feta-portmap.timer
python3 tools/feta_portmap.py remove
```

Stopping the renewal timer alone lets the current lease expire within one hour.
Stopping the game service alone leaves no game listening, but the timer can start
it again; stop both when intentionally shutting down hosting. If the router loses
its mappings after a reboot, run the portmap service or wait for renewal.
An outside-network connection test is still required to verify ISP reachability.

## Manual alternative, if automatic mapping is unavailable

From a computer connected to your home network:

1. Open http://192.168.0.1 (or http://tplinkwifi.net) and sign in with the router's
   administrator password. This is not necessarily your Wi-Fi password.
2. Open **Advanced → Network → DHCP Server → Address Reservation**. Reserve
   **192.168.0.109** for **debian-pc**, the Linux game server. Select the existing
   device from the DHCP client list if the router offers it. Save.
3. Open **Advanced → NAT Forwarding → Virtual Servers → Add** and set:

   | Field | Value |
   |---|---|
   | Service/name | Feta |
   | External port | 4000 |
   | Internal IP | 192.168.0.109 |
   | Internal port | 4000 |
   | Protocol | UDP |
   | Status | Enabled |

4. Save. Leave DMZ and remote router administration disabled. Only this game port
   needs forwarding; there is no reason to expose SSH or all ports.
5. Test from outside your home network, for example your cousin's PC or a PC using
   a phone hotspot. A connection from this server to its own address does not prove
   that the router accepts outside traffic.

Official reference: [Archer C7/A7 v5 manual](https://static.tp-link.com/2021/202103/20210325/1910012976_Archer%20C7%26A7_UG_REV5.2.0.pdf),
[TP-Link port forwarding](https://www.tp-link.com/us/support/faq/1379/).

Your main PC on the same home network can enter **192.168.0.109:4000**. Your cousin
uses your current public IPv4 plus **:4000**. The client initially uses the public
address observed during setup; if your ISP changes it, send the new address privately.
If the public address does not work from inside the house, use the LAN address:
some routers do not support NAT loopback. A future dynamic-DNS name can avoid manual
address updates. Double NAT/CGNAT can require ISP help; do not open extra ports to
try to work around it.

## Server service

```sh
cargo build --release --locked --no-default-features --bin feta-server
systemctl --user stop feta-server
install -Dm755 target/release/feta-server ~/.local/lib/feta/feta-server
install -Dm644 deploy/feta-server.service ~/.config/systemd/user/feta-server.service
systemctl --user daemon-reload
systemctl --user enable --now feta-server
systemctl --user status feta-server --no-pager
journalctl --user -u feta-server -n 30 --no-pager
```

The service listens on **0.0.0.0:4000/udp**, with encryption always required. No
plaintext compatibility listener exists in the Feta server. The original BlueEngine
binaries are development tools and must not be substituted for feta-server.

User lingering is enabled on this machine so the service survives SSH logout and
starts at boot. Elsewhere enable it once with `sudo loginctl enable-linger USER`.
The machine still needs to stay awake. The service restarts after failure and has
bounded memory/tasks. If a host firewall is enabled, permit only inbound UDP 4000
for the game. This project does not disable the firewall.

## Private files and join code

`~/.config/feta/` has directory mode 700. These files have mode 600:

- `server.env`: FETA_JOIN_KEY and FETA_TLS_KEY_FILE for the service.
- `join-code.txt`: the generated join code, for you to share privately with your cousin.
- `server-key.der`: the private TLS signing key. Never share it.

Read the player code in your own terminal:

```sh
cat ~/.config/feta/join-code.txt
```

The generated code has a memorable prefix plus 96 random bits. A short dictionary
word alone is not the internet-facing secret. To rotate it, update the env file and
join-code file, then restart the service. Existing clients reconnect with the new code.

The certificate at `assets/network/server-cert.der` is public and intentionally
bundled in the client. Its private key is not in Git, CI, downloads or logs. Keep a
private backup of the key. Rotating the certificate requires rebuilding both server
and clients. Tests use ephemeral generated identities; GitHub never receives the
production private key. The certificate's hostname is an internal identity label,
`feta.local`; players connect to an IP address and still verify that identity.

## Diagnostics

```sh
ss -lun | rg ':4000'
cargo test --locked --no-default-features --test feta_game
python3 tools/check_headless.py
```

`examples/feta_smoke.rs` connects two clients and completes a short whole round;
use it only on an empty server. It reads FETA_JOIN_KEY from the environment and
verifies the bundled certificate. Do not copy private env values into public logs.

The protocol bounds packet/queue sizes and handshake/session counts, rejects
unauthenticated data and rate-limits client datagrams. It is a small friends' server,
not a DDoS-protected hosting service. Public reachability and real Windows input
still need the outside-PC playtest described above.
