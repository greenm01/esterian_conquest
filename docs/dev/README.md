# Developer Documentation

This directory holds the current engineering docs for Esterian Conquest.

Use this file as the entrypoint. It tells you which docs are authoritative for
Rust implementation, which ones are workflow guides, and which ones are
reference/background only.

## Canonical Rust Implementation Docs

These are the docs that should drive engine/client behavior.

- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
  - canonical oracle-backed yearly maintenance ordering
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  - implementation-facing Rust companion for the turn cycle
- [ec-combat-spec.md](ec-combat-spec.md)
  - canonical Rust combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  - weekly scheduler and `Stardate` behavior
- [economics.md](economics.md)
  - economy/build policy and post-loop world/player updates
- [ec-movement-spec.md](ec-movement-spec.md)
  - movement, contact, and planner-facing movement rules
- [ec-setup-spec.md](ec-setup-spec.md)
  - setup/generator expectations and compatibility boundaries
- [ec-reports.md](ec-reports.md)
  - canonical player-facing report wording, narrative style, and classic
    `RESULTS.DAT` layout contract

## Architecture And Workflow Docs

These docs explain how to work in the repo and how to structure the Rust side.

- [next-session.md](next-session.md)
  - short restart brief for the next development session
- [approach.md](approach.md)
  - project preservation strategy and evidence policy
- [rust-architecture.md](rust-architecture.md)
  - repository-wide Rust architecture and DOD rules
- [ghidra-workflow.md](ghidra-workflow.md)
  - Ghidra investigation workflow
- [dosbox-workflow.md](dosbox-workflow.md)
  - DOSBox-based oracle/dynamic investigation workflow
- [fixtures.md](fixtures.md)
  - fixture layout and intended usage
- [harness/README.md](harness/README.md)
  - typed KDL and CLI workflow for runtime scenarios, combat scenarios, and combat sweeps

## Reference And Background Docs

These docs are useful inputs, but they are not the source of truth for Rust
behavior by themselves.

- [ecmaint-combat-reference.md](ecmaint-combat-reference.md)
  - historical combat-oriented oracle/reference notes
- [planet-report-reference.md](planet-report-reference.md)
  - report-side target-world reference profiles
- [bbs_door_client_rust.md](bbs_door_client_rust.md)
  - client/delivery direction for the SQLite-native Rust player-side stack
- [config-architecture.md](config-architecture.md)
  - KDL/config extraction boundaries and sequencing

## Reading Order

For gameplay/engine work:

1. [next-session.md](next-session.md)
2. [approach.md](approach.md)
3. [rust-architecture.md](rust-architecture.md)
4. the canonical spec docs listed above for the subsystem you are touching

For client/UI work:

1. [next-session.md](next-session.md)
2. [bbs_door_client_rust.md](bbs_door_client_rust.md)
3. [rust-architecture.md](rust-architecture.md)

## Doc Policy

- keep authoritative Rust behavior in the canonical spec docs
- keep `next-session.md` short and current
- keep historical notebooks and bulky RE detail in `archive/`
- keep reference docs clearly labeled when they are useful but non-canonical
