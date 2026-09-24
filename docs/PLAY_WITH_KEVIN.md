# Play Feta with Kevin — Windows setup

You need a Windows 10/11 64-bit PC, a mouse and keyboard, and an internet connection.
**No Tailscale, VPN, game account, or developer tools are needed.** Encryption and
server verification are built into Feta.

## 1. Get the game

1. Open https://github.com/kevstermcgee/Feta/releases.
2. Open the release Kevin is running. If it is marked **Pre-release**, this is the
   friends' playtest build. Both players and the server must use the same release.
3. Under **Assets**, download **Feta-Windows-x64.zip**, or **Feta.exe** by itself.
   Do not download the automatically generated source-code archives.
4. For the ZIP, right-click it, choose **Extract All**, and open the extracted folder.
5. Double-click **Feta.exe**. No installation or administrator launch is needed.

This small project is not code-signed. Windows may show an unknown-publisher warning.
Verify that the download came from the repository above before deciding whether to
run it. Do not disable antivirus or general Windows protections. The release also
includes `SHA256SUMS.txt` for checking download integrity.

## Explore alone first

In playtest 2 or newer, click **Explore solo** on the main menu, then choose
**Explore as Feta** or **Explore as Scientist**. No server, join code, second player
or timer is involved. Walk through the same Briar House map, try hiding places,
and test the Scientist's pistol and wrench against the scenery.

Use **Esc → Change character** to try the other role (you restart at its spawn),
**Esc → Settings** for sensitivity/FOV, or **Esc → Main menu** to return and connect
online. There is no opponent, scoring or simulated online round in exploration.

## 2. Ask Kevin for two things

- The current **server address**, including `:4000`.
- Your **join code**, sent privately. This is not a GitHub, email, or router password.

The Server box is prefilled, but Kevin's home internet address can change. Use the
address he gives you if it differs. There are only two player slots.

## 3. Connect

1. Open Feta.
2. Check the Server address. Click a field and press **Ctrl+V** to replace its contents
   with copied text, or use Backspace and type it.
3. Click **Join key**, then paste Kevin's join code with **Ctrl+V**. It is case-sensitive.
4. Click **Connect**.
5. Wait in the lobby for Kevin if he has not connected yet.

You do not need to change your router, accept a VPN invitation, or install anything
else. The join code is not saved to disk or published in this public guide.

## 4. Choose characters and play

1. One player chooses **Feta**, the other chooses **Scientist**.
2. The first player connected sets **Hunt** and **Head start**. Defaults are **5:00**
   and **1:00**. Changing settings clears Ready so both players confirm the new times.
3. Both click **Ready**. The game cannot start with two rats, two Scientists, or one player.
4. Feta runs off to hide. The Scientist sees a waiting screen during the head start.
5. The Scientist is released when the hunt begins. The discreet upper-right timer
   shows the time left. One shot or wrench hit ends the round for the Scientist;
   surviving the hunt wins for Feta. The five-minute hunt is separate from hiding time.
6. After five seconds on the result screen, choose again. Switch sides whenever you
   like. A disconnect cancels the round without awarding a win.

## Controls

| Key / mouse | Action |
|---|---|
| WASD or arrow keys | Move |
| Mouse | Look |
| Shift | Sprint |
| Space | Jump |
| Ctrl or C | Crouch |
| Left mouse | Scientist shoots |
| Right mouse | Scientist swings wrench |
| Q | Change Feta's camera |
| Esc | Open menu / release mouse |

**Esc → Settings** adjusts mouse sensitivity and field of view. Online time keeps
running while the menu is open. Use **Disconnect** when finished.

Briar House has two floors and a fenced garden. Feta fits under tables and chairs
and through a low upstairs shortcut. Garden crates and the potting shelter provide
cover with multiple escape routes. Props stay in place; this is not prop hunt.

## Troubleshooting

- **Cannot reach server / secure connection timeout:** ask Kevin to confirm the
  current address, running server and UDP port-forwarding rule. You do not need to
  forward a port on your own router. Ordinary TCP-only port-check websites cannot
  reliably test this game's QUIC/UDP listener.
- **Secure connection / certificate error:** download the same release Kevin is
  hosting and check your PC's date/time. Feta will not skip server verification.
- **Incorrect join key:** ask Kevin for the current code and paste it again. Never
  enter your account passwords into Feta.
- **Different game version:** both download the release Kevin is hosting.
- **Two players already connected:** wait about five seconds after a crash or
  disconnect, then retry.
- **Round won't start:** choose opposite characters and both click Ready again after
  a settings change.
- **Black window / startup graphics error:** update the graphics driver. Tell Kevin
  the exact message and your Windows/GPU details.
- **Movement jumps:** pause large downloads and tell Kevin what you were doing.

For a problem report, send the release name, what happened, and a screenshot if
useful. Do not include your join code or any account credentials.
