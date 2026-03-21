# Sysop Documentation

This section is for `v1.6` operators running the Rust game stack, staging
player-facing assets, and preparing campaign setups.

Use these docs in roughly this order:

- [setup-kdl-schema.md](/home/niltempus/dev/esterian_conquest/docs/sysop/setup-kdl-schema.md)
  - declarative campaign setup for `ec-cli sysop new-game --config ...`
- [sysop-map-exports.md](/home/niltempus/dev/esterian_conquest/docs/sysop/sysop-map-exports.md)
  - player map export and queue/download staging for the Rust client
- [enigma-bbs-setup.md](/home/niltempus/dev/esterian_conquest/docs/sysop/enigma-bbs-setup.md)
  - legacy DOS door setup under Enigma BBS; useful for compatibility hosting,
    but not the main `v1.6` direction

Practical posture:

- prefer the Rust-native `ec-client` / `maint-rust` stack for new deployments
- treat original DOS `ECGAME` hosting as a compatibility bridge, not the long-
  term operating model
- keep classic `.DAT` import/export at the edge of the system instead of as the
  main runtime path
