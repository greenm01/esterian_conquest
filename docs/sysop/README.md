# Sysop Documentation

This section is for EC operators running the Rust game stack, staging
player-facing assets, and administering live campaigns.

Use these docs in roughly this order:

- [sysop-map-exports.md](sysop-map-exports.md)
  - player map export and queue/download staging for the Rust client
- [enigma-bbs-setup.md](enigma-bbs-setup.md)
  - legacy DOS door setup under Enigma BBS; useful for compatibility hosting,
    but not the main EC direction

Practical posture:

- prefer the Rust-native `ec-game` / `ec-sysop` stack for new deployments
- treat original DOS `ECGAME` hosting as a compatibility bridge, not the long-
  term operating model
- use `config.kdl` as the only public sysop config file
- schedule `ec-sysop maint` with host tooling such as `systemd`, `cron`, or
  BBS event hooks instead of trying to schedule maintenance inside EC config
