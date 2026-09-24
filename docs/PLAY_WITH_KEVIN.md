# Play Feta with Kevin — Windows setup

You need a Windows 10/11 64-bit PC, a mouse and keyboard, and an internet connection.
There is no Rust, Python, command-line setup, or game account to install.

## 1. Install Tailscale (once)

1. Download the Windows installer from https://tailscale.com/download/windows.
2. Install it and open Tailscale from the system tray near the clock.
3. Choose **Log in**, and sign in with your own account. Tell Kevin which email
   address you used. Do not use Kevin's GitHub login.
4. Leave Tailscale connected while playing. You do not need an exit node or VPN
   routing changes.

## 2. Accept Kevin's invitation (once)

1. Kevin will send you a private Tailscale sharing link for **debian-pc**.
2. Open it in your browser while signed into your own Tailscale account, then accept.
3. Confirm **debian-pc** appears as a shared machine in Tailscale. Kevin configures
   access to the game port; you do not need SSH or access to his files.

If you have not received the link yet, ask Kevin for it. Installing Tailscale alone
does not grant access to the server.

## 3. Download the game

1. Open https://github.com/kevstermcgee/Feta/releases.
2. Open the release Kevin is running. If it is marked **Pre-release**, this is the
   friends' playtest build. Both players and the server must use that same release.
3. Under **Assets**, download **Feta-Windows-x64.zip**, or **Feta.exe** by itself.
   Do not download the automatically generated source-code archives.
4. If you downloaded the ZIP, right-click it, choose **Extract All**, and open the
   extracted folder. Double-click **Feta.exe**. No installation or admin launch is needed.
5. This small project is not code-signed. Windows may show an unknown-publisher
   warning. Verify that your download came from the repository above before deciding
   whether to run it; do not disable antivirus or general Windows protections.

Optional: the release includes `SHA256SUMS.txt` for checking download integrity.

## 4. Connect

1. Confirm Tailscale says **Connected**.
2. Open Feta. The Server box is prefilled with **100.92.249.99:4000**.
3. Enter the join key Kevin gives you privately. It is case-sensitive.
4. Click **Connect**.
5. The lobby should show that both players are connected. If Kevin is not in yet,
   leave the lobby open and wait.

The join key is intentionally not printed in this public guide. There is no account
password to enter into Feta, and Feta does not save the key to disk.

## 5. Start a round

1. One player chooses **Feta**, the other chooses **Scientist**.
2. The first player connected sets **Hunt** and **Head start**. Defaults are **5:00**
   and **1:00**. Changing times clears Ready so both players confirm the new settings.
3. Both click **Ready**. The game cannot start with two rats, two Scientists, or one player.
4. Feta runs off to hide. The Scientist sees a waiting screen for the head start.
5. When the hunt begins, the Scientist is released. The small upper-right timer shows
   the time left. One shot or wrench hit ends the round; surviving the hunt wins for Feta.
6. After five seconds on the result screen, choose characters again. Switch sides
   whenever you like. A disconnect cancels the round, rather than awarding a win.

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
running while the menu is open. Use **Disconnect** when finished playing.

## If something does not work

- **Cannot reach server:** check Tailscale is connected, the invitation was accepted,
  and the address includes `:4000`. Ask Kevin whether the server is running and whether
  his game-only access rule is saved. Router port forwarding is not needed.
- **Incorrect join key:** ask Kevin for the current key; do not enter your Tailscale
  or GitHub password.
- **Different game version:** both download the release Kevin is hosting.
- **Two players already connected:** there are only two slots. A crashed/disconnected
  client is removed after about five seconds; retry then.
- **Round won't start:** select opposite characters and both click Ready again after
  any settings or character change.
- **Black window / startup graphics error:** update the PC's graphics driver. Tell
  Kevin the exact message and your Windows/GPU details.
- **Poor connection:** keep Tailscale connected, pause large downloads, and tell Kevin
  if movement jumps. A relayed Tailscale connection can have higher latency.

For a problem report, send Kevin the release name, what you were doing, and a
screenshot if useful. Do not include passwords, login tokens, or your join key.
