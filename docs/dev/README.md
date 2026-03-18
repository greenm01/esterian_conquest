# Developer Documentation

This directory holds the current engineering docs for Esterian Conquest.

Use this file as the entrypoint. It tells you which docs are authoritative for
Rust implementation, which ones are workflow guides, and which ones are
reference/background only.

## Canonical Rust Implementation Docs

These are the docs that should drive engine/client behavior.

- [ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md)
  - canonical oracle-backed yearly maintenance ordering
- [rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md)
  - implementation-facing Rust companion for the turn cycle
- [ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-combat-spec.md)
  - canonical Rust combat and hostile world-resolution mechanics
- [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
  - weekly scheduler and `Stardate` behavior
- [economics.md](/home/mag/dev/esterian_conquest/docs/dev/economics.md)
  - economy/build policy and post-loop world/player updates
- [ec-movement-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-movement-spec.md)
  - movement, contact, and planner-facing movement rules
- [ec-setup-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-setup-spec.md)
  - setup/generator expectations and compatibility boundaries
- [ec-reports.md](/home/mag/dev/esterian_conquest/docs/dev/ec-reports.md)
  - canonical player-facing report wording, narrative style, and classic
    `RESULTS.DAT` layout contract

## Architecture And Workflow Docs

These docs explain how to work in the repo and how to structure the Rust side.

- [next-session.md](/home/mag/dev/esterian_conquest/docs/dev/next-session.md)
  - short restart brief for the next development session
- [approach.md](/home/mag/dev/esterian_conquest/docs/dev/approach.md)
  - project preservation strategy and evidence policy
- [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)
  - repository-wide Rust architecture and DOD rules
- [ghidra-workflow.md](/home/mag/dev/esterian_conquest/docs/dev/ghidra-workflow.md)
  - Ghidra investigation workflow
- [dosbox-workflow.md](/home/mag/dev/esterian_conquest/docs/dev/dosbox-workflow.md)
  - DOSBox-based oracle/dynamic investigation workflow
- [fixtures.md](/home/mag/dev/esterian_conquest/docs/dev/fixtures.md)
  - fixture layout and intended usage

## Reference And Background Docs

These docs are useful inputs, but they are not the source of truth for Rust
behavior by themselves.

- [ecmaint-combat-reference.md](/home/mag/dev/esterian_conquest/docs/dev/ecmaint-combat-reference.md)
  - historical combat-oriented oracle/reference notes
- [planet-report-reference.md](/home/mag/dev/esterian_conquest/docs/dev/planet-report-reference.md)
  - report-side target-world reference profiles
- [bbs_door_client_rust.md](/home/mag/dev/esterian_conquest/docs/dev/bbs_door_client_rust.md)
  - client/delivery direction for the Rust player-side stack
- [config-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/config-architecture.md)
  - KDL/config extraction boundaries and sequencing

## Reading Order

For gameplay/engine work:

1. [next-session.md](/home/mag/dev/esterian_conquest/docs/dev/next-session.md)
2. [approach.md](/home/mag/dev/esterian_conquest/docs/dev/approach.md)
3. [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)
4. the canonical spec docs listed above for the subsystem you are touching

For client/UI work:

1. [next-session.md](/home/mag/dev/esterian_conquest/docs/dev/next-session.md)
2. [bbs_door_client_rust.md](/home/mag/dev/esterian_conquest/docs/dev/bbs_door_client_rust.md)
3. [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)

## Doc Policy

- keep authoritative Rust behavior in the canonical spec docs
- keep `next-session.md` short and current
- keep historical notebooks and bulky RE detail in `archive/`
- keep reference docs clearly labeled when they are useful but non-canonical
