# Sysop Documentation

This section is for EC operators running the Rust game stack, staging
player-facing assets, and administering live campaigns.

Use these docs in roughly this order:

- [campaign-settings.md](campaign-settings.md)
  - non-BBS campaign settings and raw `nc-sysop settings show` reference
- [turn-kdl.md](turn-kdl.md)
  - KDL turn file format reference for file-based turn submission
- [mystic-rust-setup.md](mystic-rust-setup.md)
  - validated Mystic setup for the Rust-native `nc-door`, including the
    native Windows `D3` / `DOOR32` path
- [synchronet-rust-setup.md](synchronet-rust-setup.md)
  - validated native Windows Synchronet setup for the Rust-native `nc-door`
- [enigma-rust-setup.md](enigma-rust-setup.md)
  - validated ENiGMA½ setup notes for the Rust-native `nc-door`
- [sysop-map-exports.md](sysop-map-exports.md)
  - player map export and queue/download staging for the Rust client
- [enigma-bbs-setup.md](enigma-bbs-setup.md)
  - legacy DOS door setup under Enigma BBS; useful for compatibility hosting,
    but not the main EC direction

Practical posture:

For new deployments, prefer the Rust-native `nc-connect`, `nc-game`,
`nc-door`, and `nc-sysop` stack. Keep the roles straight. `nc-game` is the
direct localhost and SSH/VPS session client. `nc-door` is the BBS entrypoint on
both Windows and Linux. `nc-sysop` creates campaigns, edits settings, and runs
maintenance.

The public Windows x64 and Linux x64 `nc-sysop` archives are the BBS/sysop
packages. Use them when you want a normal door-host handoff without a Cargo
toolchain. Localhost play on Windows, Linux, and macOS remains a source-build
workflow. VPS/Nostr hosting also remains a source-build workflow with
`scripts/install_vps.sh`.

Hosted Rust campaigns are DB-only: one `ncgame.db` per game directory. BBS
door campaigns keep a minimal per-game `config.kdl` beside `ncgame.db`.
Hosted/Nostr game registry data remains global in `/etc/nc-gate/config.kdl`.

ENiGMA½ is the validated `abracadabra` socket-mode Rust-door host. Native
Windows Mystic and native Windows Synchronet are both verified `DOOR32` hosts
for `nc-door`.

Treat original DOS `ECGAME` hosting as a compatibility bridge, not the main
Rust operating model. Schedule `nc-sysop maint` or `nc-sysop maint-all` with
real host tooling such as `systemd`, `cron`, Task Scheduler, or BBS event
hooks rather than trying to schedule maintenance inside the campaign itself.
