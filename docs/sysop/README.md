# Sysop Documentation

This section is for EC operators running the Rust game stack, staging
player-facing assets, and administering live campaigns.

Use these docs in roughly this order:

- [turn-kdl.md](turn-kdl.md)
  - KDL turn file format reference for file-based turn submission
- [mystic-rust-setup.md](mystic-rust-setup.md)
  - validated local-door BBS setup for the Rust-native `nc-game`
- [enigma-rust-setup.md](enigma-rust-setup.md)
  - validated ENiGMA½ setup notes for the Rust-native `nc-game` door
- [sysop-map-exports.md](sysop-map-exports.md)
  - player map export and queue/download staging for the Rust client
- [enigma-bbs-setup.md](enigma-bbs-setup.md)
  - legacy DOS door setup under Enigma BBS; useful for compatibility hosting,
    but not the main EC direction

Practical posture:

- prefer the Rust-native `nc-connect` / `nc-game` / `nc-sysop` stack for new
  deployments
- use the public Linux x64 `nc-sysop` package for localhost or BBS hosting
  when you want a no-Cargo operator handoff
- use the matching Windows x64 `nc-sysop` package when you have built and
  validated it on a native Windows host
- treat hosted Rust campaigns as DB-only: one `ncgame.db` per game directory
- Mystic and ENiGMA are both now verified Rust-door hosts
- on Unix-like hosts, use `tools/bbs/run_nc_rust.sh`; on native Windows
  hosts, use `tools/bbs/run_nc_rust.cmd`
- for BBS play, treat `HJKL` as the primary door navigation contract and
  `^U` / `^D` as the primary paging keys
- treat original DOS `ECGAME` hosting as a compatibility bridge, not the long-
  term operating model
- VPS/Nostr hosting remains a tagged-source Cargo workflow with
  `scripts/install_vps.sh`; the public sysop package is for localhost and BBS,
  not VPS
- hosted/Nostr campaigns use `ncgame.db` for per-game runtime policy, while
  BBS door campaigns use a minimal per-game `config.kdl`; `/etc/nc-gate/config.kdl`
  remains the global daemon game list
- schedule `nc-sysop maint` or `nc-sysop maint-all` with host tooling such as
  `systemd`, `cron`, or BBS event hooks instead of trying to schedule
  maintenance inside the campaign
