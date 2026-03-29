# Sysop Documentation

This section is for EC operators running the Rust game stack, staging
player-facing assets, and administering live campaigns.

Use these docs in roughly this order:

- [turn-kdl.md](turn-kdl.md)
  - KDL turn file format reference for file-based turn submission
- [mystic-rust-setup.md](mystic-rust-setup.md)
  - validated local-door BBS setup for the Rust-native `ec-game`
- [enigma-rust-setup.md](enigma-rust-setup.md)
  - validated ENiGMA½ setup notes for the Rust-native `ec-game` door
- [sysop-map-exports.md](sysop-map-exports.md)
  - player map export and queue/download staging for the Rust client
- [enigma-bbs-setup.md](enigma-bbs-setup.md)
  - legacy DOS door setup under Enigma BBS; useful for compatibility hosting,
    but not the main EC direction

Practical posture:

- prefer the Rust-native `ec-connect` / `ec-game` / `ec-sysop` stack for new
  deployments
- during the current beta, Rust sysops should expect tagged-source Cargo
  installs rather than public Rust binary downloads
- treat hosted Rust campaigns as DB-only: one `ecgame.db` per game directory
- Mystic and ENiGMA are both now verified Rust-door hosts
- for BBS play, treat `HJKL` as the primary door navigation contract and
  `^U` / `^D` as the primary paging keys
- treat original DOS `ECGAME` hosting as a compatibility bridge, not the long-
  term operating model
- a public Linux x64 BBS door package is planned later; it should carry
  `ec-game`, `ec-sysop`, and both manuals
- use `ec-sysop settings` for per-game runtime policy and `/etc/ec-gate/config.kdl`
  for the global daemon game list
- schedule `ec-sysop maint` or `ec-sysop maint-all` with host tooling such as
  `systemd`, `cron`, or BBS event hooks instead of trying to schedule
  maintenance inside the campaign
