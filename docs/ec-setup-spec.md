# EC Setup And Starmap Spec

This document defines the intended Esterian Conquest setup model for the Rust
port.

The original manuals are the semantic authority here. Exact byte-for-byte map
recreation from one historical game is not required. What matters is faithful
adherence to the documented setup rules while preserving classic-compatible
save directories.

## Source Authority

Primary source:

- [ECPLAYER.DOC](/home/mag/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC)

Relevant manual statements:

- the galaxy is a square grid
- map size depends on player count
- solar systems are randomly generated at game start
- each solar system contains one colonizable planet
- the total number of solar systems is `5 * player_count`
- each player begins with one planet producing `100` points per year
- each player begins with four fleets:
  - two fleets containing `1 cruiser + 1 ETAC`
  - two fleets containing `1 destroyer`

## Canonical Setup Goals

Rust setup should:

- follow the player-count and map-size rules from the manuals
- generate a valid star map and initial empire state without violating classic
  file compatibility
- preserve interchangeability with the original DOS binaries at the `.DAT`
  directory boundary
- avoid treating the current generalized builder as the final gameplay setup
  model
- keep sysop setup flows distinct from the player client workflow

## Player Count Tiers

The original game supports these canonical player-count tiers:

- `4 players -> 18 x 18` map
- `9 players -> 27 x 27` map
- `16 players -> 36 x 36` map
- `25 players -> 45 x 45` map

The intended long-term Rust initializer should treat those as the primary
supported setup modes.

The current generalized builder may continue to support narrower experimental
counts for oracle and compatibility work, but that flexibility should not be
confused with the documented gameplay setup rules.

## Star Map Rules

The map is a square sector grid. Each sector is either:

- empty deep space
- one solar system containing one planet

The total number of solar systems is:

- `5 * player_count`

The positions of solar systems are randomly generated at game start.

This implies:

- star-system count is a first-class gameplay rule, not a scenario convenience
- a faithful Rust setup flow needs explicit map-generation rules, not just
  hand-placed known scenarios

## Planet And Empire Start Rules

Each player begins with:

- one owned homeworld
- current production `100`
- four initial fleets:
  - two `CA + ETAC`
  - two `DD`

The long-term setup model should make the following explicit:

- homeworld placement constraints
- homeworld separation constraints
- initial fleet placement relative to the homeworld
- initial ownership and development fields required for classic compatibility

## Compatibility Rule

The Rust setup flow does not need to reproduce the original game’s hidden map
RNG sequence exactly.

It does need to:

- generate manual-faithful map sizes and system counts
- generate sane starting empires and planets
- emit classic-compatible `.DAT` directories accepted by original tools

## Sysop Workflow Boundary

Game setup is a sysop/admin responsibility, not a normal player action.

For the Rust port, that means:

- shared setup logic may live in `ec-data`
- admin-facing setup commands may live in `ec-cli` or a future sysop UI
- the player client should not be the primary place where a new game is
  initialized or rewritten

This follows the original `ECUTIL` / `ECGAME` separation even if the Rust
implementation later shares more code under the hood.

## Implementation Direction

Refine setup in this order:

1. codify manual-driven setup invariants
2. audit the current builder against those invariants
3. separate "compatibility builder" from "faithful game initializer"
4. add a dedicated Rust initializer for canonical EC game start
5. validate generated starts against original-tool acceptance and gameplay
   sanity rather than byte-perfect RNG recreation

## Current Rust Audit

The current Rust setup/builder layer is a compatibility-oriented baseline
constructor, not yet a faithful full EC game initializer.

Current hard boundaries in `ec-data`:

- [`lib.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/lib.rs)
  defines:
  - `PLAYER_RECORD_COUNT = 4`
  - `PLANET_RECORD_COUNT = 20`
- [`builder.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/builder.rs)
  clamps `with_player_count()` to `1..=4`
- [`directory.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/directory.rs)
  rejects `CONQUEST.DAT.player_count > 4` in current preflight validation

Those choices are consistent with the current milestone:

- reconstruct and validate the known 4-player preserved baseline
- generate classic-compatible directories accepted by original maintenance
- port mechanics incrementally from that known-good footing

They are not yet sufficient for the manuals' full setup model.

So the current code should be understood as:

- good compatibility infrastructure
- good test harness infrastructure
- not yet the final manual-faithful game-start implementation

## Audit Consequences

The setup roadmap should separate two deliverables:

1. preserve and stabilize the current 4-player compatibility builder
2. expand the data model and initializer toward the manual-driven player-count
   tiers and star-map rules

That keeps the current compliant workflow intact while making room for a more
faithful initializer later.

## Non-Goals

This spec does not yet define:

- the exact procedural RNG used by the DOS binaries
- the full planet-distribution algorithm
- exact homeworld placement heuristics used by `ECUTIL`

Those may be recovered later, but they are not prerequisites for a faithful
manual-driven setup implementation.
