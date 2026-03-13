# Config Architecture

This document defines the intended boundary between Rust code, human-facing
specs, and machine-readable config in the Esterian Conquest port.

## Purpose

The Rust port should eventually use config files extensively for:

- stable combat/entity constants
- setup and baseline presets
- oracle scenario definitions

KDL is the preferred format for those machine-readable config surfaces.

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

The first expected config extraction is combat constants and oracle scenarios,
not a full generalized scenario DSL.
