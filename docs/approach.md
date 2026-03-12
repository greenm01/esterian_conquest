# Preservation Approach

This repository is not trying to recover the original Pascal source code verbatim.

The goal is:

- preserve Esterian Conquest v1.5 as a working historical artifact
- reverse engineer its file formats and rules
- build Rust tooling that can generate 100% compliant gamestate files accepted
  by the original game and `ECMAINT`
- use that compliance target as the first concrete milestone toward a faithful
  modern reimplementation in Rust
- keep the original DOS binaries and data as the reference implementation

## Principles

1. Treat the DOS binaries as the spec

- `ECGAME.EXE` is the player-facing command UI
- `ECUTIL.EXE` is the sysop/configuration utility
- `ECMAINT.EXE` is the yearly maintenance and simulation engine

2. Prefer confirmed behavior over guessed structure

- only name fields after they are supported by diffs, screenshots, docs, or repeated observation
- keep unknown bytes raw until they are mapped with confidence

3. Separate stable docs from lab notes

- `RE_NOTES.md` is the chronological investigation notebook
- `docs/` holds stable, reusable engineering docs

4. Keep the architecture layered

- `ec-data`: binary formats and typed accessors
- `ec-cli`: std-only scripting and verification interface
- `ec-tui`: user-facing terminal UI

5. Use fixtures to lock in behavior

- original shipped state
- initialized state
- post-maintenance state
- targeted scenario snapshots for specific features

6. Prefer engine outputs over UI playback

- `ECMAINT` writes the underlying state and generated report data
- `ECGAME` is still useful, but mainly as a viewer/validation layer for those outputs
- when possible, decode changes in `.DAT` files first and use live report viewing second
- historical text captures are reference evidence when live playback is unavailable or flaky

## What Counts As Success

Short term:

- decode the important on-disk formats
- reproduce `ECUTIL` behavior faithfully
- understand `ECMAINT` as a deterministic state transformer
- define the cross-file invariants required for original-engine acceptance
- generate fully compliant gamestate files from Rust

Long term:

- reimplement the real turn engine in Rust
- build a usable player client and admin client
- support classic-compatible saves and reproducible results

## Milestone Ladder

1. Known accepted scenarios

- Rust can emit preserved accepted pre-maint scenarios from decoded fields
- the original binaries and preserved fixtures are the acceptance oracle
- current examples:
  - `fleet-order`
  - `planet-build`
  - `guard-starbase`

2. Parameterized scenario generation

- replace scenario-specific constants with explicit field builders and
  validators
- move from "recreate this one accepted shape" toward "generate families of
  accepted shapes within known-safe constraints"

3. General compliant gamestate generation

- Rust can write a full arbitrary gamestate directory that `ECMAINT` accepts
  without integrity failures
- this requires the remaining cross-file linkage rules, especially the
  starbase/fleet summary-pairing semantics in `ECMAINT`

4. Full Rust maintenance replacement

- reimplement `ECMAINT` behavior in Rust with reproducible outputs
- preserve compatibility with original save directories and reports

Near-term acceptance rule:

- a format/mechanic is not "done" until Rust can emit the relevant state and
  the original binaries accept it without integrity failures or unexpected
  normalization

Current concrete Rust milestone:

- start from a known-good preserved snapshot such as
  `fixtures/ecmaint-post/v1.5`
- rewrite only decoded fields in Rust
- verify the rewritten `.DAT` file matches a preserved accepted pre-maint
  scenario exactly
- current confirmed examples:
  - fleet-order rewrite reproducing `fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT`
  - planet build-queue rewrite reproducing
    `fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT`
  - named fleet/build scenario commands and validators over those same
    preserved accepted rewrites:
    - `ec-cli scenario <dir> fleet-order`
    - `ec-cli scenario <dir> planet-build`
    - `ec-cli validate <dir> fleet-order`
    - `ec-cli validate <dir> planet-build`
    - `ec-cli scenario-init [source_dir] <target_dir> fleet-order`
    - `ec-cli scenario-init [source_dir] <target_dir> planet-build`
  - accepted Guard Starbase scenario rewrite reproducing the core gamestate
    files from `fixtures/ecmaint-starbase-pre/v1.5`
  - the Guard Starbase base record is now emitted from named Rust field
    setters over `BaseRecord`, not from a preserved raw byte blob
  - Rust can also validate the currently-known accepted one-base Guard
    Starbase shape with `ec-cli validate <dir> guard-starbase`
  - that Guard Starbase validator is now linkage-shaped:
    - player starbase-count word
    - fleet local-slot word
    - fleet ID word
    - base chain word
    - guard target/base coords
    - guard starbase index/enable bytes
  - Rust also now has a first safe parameterized starbase writer:
    - `ec-cli guard-starbase-onebase <dir> <target_x> <target_y>`
    - this keeps the currently-known linkage keys fixed in the accepted
      one-base shape while varying the sector coordinates
  - Rust also now has a direct linkage inspection command:
    - `ec-cli guard-starbase-report <dir>`
    - use this to print the currently relevant player/fleet/base key words
      before and after running `ECMAINT`
  - Rust also now has a one-command parameterized directory initializer:
    - `ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>`
    - this is the quickest current path from a known-good baseline to a new
      ECMAINT-ready one-base Guard Starbase experiment
  - Rust also now has the first practical `IPBM.DAT` control surface:
    - `ec-cli ipbm-report <dir>`
    - `ec-cli ipbm-zero <dir> <count>`
    - this is enough to keep generated scenarios on the correct side of the
      known `PLAYER[0x48]` / `IPBM.DAT` count gate without hand-editing bytes
  - `ec-cli validate <dir> all` now gives a quick classification pass across
    the current known accepted scenarios
  - the known accepted scenarios are now centralized behind one Rust-side
    catalog:
    - `ec-cli scenario <dir> list`
    - `ec-cli scenario <dir> show <scenario>`
    - `ec-cli scenario-init-all [source_dir] <target_root>`
  - scenario validation now has two useful levels:
    - rule-shaped validators: `ec-cli validate <dir> ...`
    - preserved exact-match validators: `ec-cli validate-preserved <dir> ...`
  - preserved scenario drift can now be inspected directly with
    `ec-cli compare-preserved <dir> ...`
  - Rust can now materialize a runnable Guard Starbase directory from a
    compliant baseline with
    `ec-cli scenario-init [source_dir] <target_dir> guard-starbase`
  - the documented optional-source forms now work as intended:
    - `ec-cli init <target_dir>` defaults to `original/v1.5`
    - `ec-cli scenario-init <target_dir> <scenario>` defaults to
      `fixtures/ecmaint-post/v1.5`

This is intentionally narrower than a full arbitrary save generator, but it is
the first real proof that the Rust layer can emit accepted gamestate files from
decoded state rather than only copy fixture trees.

## Current Strategy

Near-term effort should prioritize `ECMAINT`.

Why:

- `ECUTIL` is mostly configuration/state setup
- `ECGAME` is mainly command entry and presentation
- `ECMAINT` appears to be the core simulation engine:
  - movement
  - battles
  - build completion
  - AI / rogue empire behavior
  - database and report generation

That makes `ECMAINT` the highest-value target for recovering the actual rules of the game.
It is also the main acceptance oracle for the first milestone: Rust-generated
gamestate files that are 100% compliant with the original engine.

## ECMAINT Investigation Model

The current `ECMAINT` workflow is:

1. create one controlled pre-maint scenario
2. run original `ECMAINT`
3. diff the resulting `.DAT` files
4. preserve pre/post fixtures
5. encode the confirmed transform in Rust tests
6. optionally read the generated reports through `ECGAME` as a validation step

This keeps the preservation work grounded in deterministic engine behavior rather
than in UI rendering.

## Session Handoff

When pausing work, keep the immediate restart point in:

- `docs/next-session.md`

That file should be updated with:

- the latest high-confidence combat model
- the most recent commits worth resuming from
- the exact next controlled experiment
