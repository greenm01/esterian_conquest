# Config Architecture

This document defines the intended boundary between Rust code, human-facing
specs, and machine-readable config in the Esterian Conquest port.

## Purpose

The Rust port should eventually use config files extensively for:

- stable combat/entity constants
- setup and baseline presets
- oracle scenario definitions
- sysop/admin game setup

KDL is the preferred format for those machine-readable config surfaces.

This does not mean every new Rust-only piece of persistent state should become
another KDL sidecar. The intended long-term split is:

- KDL for authored setup/config/scenario input
- classic `.DAT` files for compatibility with the original game
- a future per-game SQLite database for Rust-native campaign metadata,
  historical turn state, and policy extensions that do not belong in classic
  files

Until that SQLite layer exists, Rust-only campaign policy such as turn limits
should remain deferred rather than forcing more live runtime sidecars into KDL.

## Ownership Boundaries

The layers are:

1. `docs/`
- normative human-readable engineering and rules docs
- the source of truth for architecture and canonical mechanic intent

2. Rust code
- execution engine
- save compatibility authority
- maintenance phase ordering
- cross-file mutation choreography
- validation and writeback into classic `.DAT` files

3. KDL config
- machine-readable stable data tables and presets
- loaded by Rust after the internal model for that area is stable
- preferred long-term source for durable sysop/setup intent

If there is drift between docs and KDL, the docs win until both config and code
are updated together.

## Good KDL Targets

Approved long-term KDL categories:

- combat constants
  - unit `AS` / `DS`
  - fresh-step thresholds
  - ROE thresholds
  - CER tables
  - bombard / invade / blitz weights
  - target priority tables
- setup presets
  - initialized baselines
  - test scenario overlays
  - combat fixture seeds
- sysop setup
  - player count
  - game year
  - maintenance days
  - setup/program options
  - optional map-generation seed
  - startup map-generation policy
  - startup presets and map-generation choices
- oracle scenarios
  - named scenario definitions
  - source baseline
  - overlay mutations
  - turn counts
  - expected comparison policy

## Bad Early KDL Targets

Do not push these into config prematurely:

- maintenance control flow
- byte-level compatibility choreography
- cross-file repair semantics
- unresolved reverse-engineering guesses
- low-level layout rules that must stay explicit in Rust record code

## Adoption Sequence

The intended sequence is:

1. implement and stabilize the mechanic in Rust
2. centralize constants in typed Rust tables
3. extract stable tables into KDL
4. add parse/validation tests proving KDL matches the intended canonical values

This keeps config from driving low-level design too early.

## Recommended File Layout

When KDL is introduced, prefer:

- `rust/ec-data/config/combat.kdl`
- `rust/ec-data/config/setup.kdl`
- `rust/ec-data/config/scenarios/*.kdl`
- schema and examples documented in `docs/`

For sysop/admin setup, prefer the ownership split:

- KDL stores durable setup intent
- `ec-cli sysop` validates and materializes that config into classic `.DAT`
  directories
- a future TUI may edit the same config/model, but should not become the only
  place where setup exists

The first expected config extraction is combat constants and oracle scenarios,
not a full generalized scenario DSL.

The first concrete sysop/setup target is now documented in
[setup-kdl-schema.md](docs/setup-kdl-schema.md)
with a matching sample file at
[`rust/ec-data/config/setup.example.kdl`](rust/ec-data/config/setup.example.kdl).
