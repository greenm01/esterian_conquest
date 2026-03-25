# Rust Architecture

This document defines the standing Rust-side architecture rules for this repo.
It is the code-ownership and module-boundary companion to the behavior specs.

Read it together with:

- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  for yearly maintenance phase ordering
- [ec-combat-spec.md](ec-combat-spec.md)
  for combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  for weekly timing and report `Stardate` behavior
- [economics.md](economics.md)
  for post-loop build/economy policy
- [bbs_door_client_rust.md](bbs_door_client_rust.md)
  for player-client direction

Authority boundary:

- the subsystem spec docs own gameplay behavior and turn ordering
- this document owns Rust crate boundaries, data flow, and module structure
- if a behavior spec and this doc ever disagree on rules, the behavior spec
  wins; this doc should then be updated to match

## Core Rules

The Rust workspace should follow pragmatic data-oriented design:

- keep binary layout explicit
- prefer plain records plus focused free functions or small impl blocks
- keep logic DRY by centralizing shared validation, normalization, and report
  helpers
- keep source files lean; when a file starts feeling crowded, split it before
  growing it further
- avoid deep object hierarchies, framework-style indirection, and giant
  monolithic modules
- keep unknown classic fields raw until the semantics are supported by docs,
  oracle diffs, or repeated observation
- treat preserved fixtures, original manuals, and the original binaries as the
  acceptance oracle

## Workspace Ownership

### `ec-data`

`ec-data` is the shared runtime/store/model crate.

It is responsible for:

- explicit runtime record/file layouts under `records/`
- `CoreGameData` and shared multi-file directory semantics
- SQLite runtime persistence and snapshot/history loading
- shared validators, normalizers, report/mail/intel state, and typed helpers
  used by more than one frontend

It should stay focused on runtime/state semantics rather than owning the normal
classic import/export workflow or the external engine API surface.

### `ec-engine`

`ec-engine` is the public Rust engine/rules boundary.

It is responsible for:

- yearly maintenance behavior
- economy, movement/pathfinding, setup/map generation, and combat-facing rule
  surfaces
- the stable public API that CLI/client/harness code should use for gameplay
  logic

Engine callers should depend on `ec-engine` rather than reaching directly into
`ec-data` for rules.

### `ec-classic`

`ec-classic` is the low-level classic-file support crate.

It is responsible for:

- raw classic record types that are still shared across the compat boundary
- classic byte-level parse/encode helpers
- keeping classic byte mechanics out of the runtime/store API surface

It should stay small and dumb:

- explicit layouts
- minimal parsing/encoding helpers
- no runtime-store policy
- no gameplay rules

### `ec-compat`

`ec-compat` owns the explicit classic compatibility boundary.

It is responsible for:

- importing classic directories into normalized SQLite runtime state
- exporting normalized SQLite snapshots back to classic `.DAT` directories
- classic report/database projection helpers used only for oracle and hybrid
  workflows
- keeping classic file handling out of normal engine/client code

### `ec-sysop`

`ec-sysop` is the public sysop/admin surface.

It is responsible for:

- new-game creation and setup/admin flows
- yearly maintenance execution
- live campaign operator actions that belong in normal play

It should stay thin:

- `main.rs` is process boundary only
- dispatch owns top-level routing
- command modules orchestrate workflows but do not own rules

Game rules should not be reimplemented in command modules. If a command needs a
shared rule, that rule belongs in `ec-engine` (backed by shared runtime/store
types from `ec-data`).

### `ec-cli`

`ec-cli` is the internal developer/oracle/inspection surface.

It is responsible for:

- command dispatch and workflow orchestration
- oracle and compliance sweeps
- import/export and runtime/storage bridge commands
- inspection, reporting, and scenario materialization helpers
- temporary compatibility shims during public CLI transitions

It should stay thin:

- `main.rs` is process boundary only
- `dispatch.rs` owns top-level routing
- `commands/` is grouped by workflow or command family
- `support/` holds shared parsing/path helpers
- `workspace.rs` owns shared directory/bootstrap helpers

Game rules should not be reimplemented in command modules. If a command needs a
shared rule, that rule belongs in `ec-engine` (backed by shared runtime/store
types from `ec-data`).

### `ec-client`

`ec-client` is the player-facing application layer and currently ships as the
`ec-game` binary.

It is responsible for:

- rendering, input, startup flow, and screen/domain navigation
- player-facing review/edit flows over runtime state
- terminal/layout/theme concerns
- player-report presentation

It should not duplicate game rules. The client consumes `ec-data` runtime/store
types and `ec-engine` rule surfaces instead of re-deriving combat, movement,
build, or maintenance semantics locally.
It also should not own classic `.DAT` projection; if a workflow needs classic
files, that belongs in `ec-cli` export/materialization code.

Keep the client structure similarly explicit:

- `src/app/` is the thin shell:
  - root app state
  - top-level action enum
  - reducer/update loop
  - shell-wide helpers
- `src/domains/<domain>/` owns domain-specific:
  - screen state
  - render/update logic
  - any `App` methods specific to that domain
- `src/screen/` owns shared rendering primitives and screen IDs

Do not leave large domain controllers parked under `src/app/` once a real
domain module exists for them.

## Current Structural Direction

The current workspace shape is:

```text
rust/
├── ec-classic
│   └── src/          # low-level classic record/codecs
├── ec-data
│   ├── src/records/   # explicit binary/runtime record layouts
│   ├── src/storage.rs # SQLite campaign store + snapshot bridge
│   └── shared runtime/support modules
├── ec-engine
│   └── src/          # public engine/rules surface
├── ec-compat
│   └── src/          # classic import/export and compat projections
├── ec-sysop
│   └── src/          # public sysop/admin/maintenance workflows
├── ec-cli
│   ├── src/commands/  # developer/oracle/runtime/compat workflows
│   └── src/support/   # shared CLI helpers
└── ec-client
    ├── src/domains/   # feature/domain slices + domain controllers
    ├── src/app/       # thin app shell/state/update/action seams
    ├── src/screen/    # screen/layout primitives
    └── terminal/startup/theme helpers
```

This doc intentionally describes the module families rather than a frozen file
inventory. The structure should keep evolving when a cleaner split buys clarity,
but the ownership boundaries above should remain stable.

## Boundary Sketch

```text
+--------------------------------------------------------------+
|                         Frontends                            |
|--------------------------------------------------------------|
|  ec-client      ec-sysop         ec-cli         ec-harness   |
|  player TUI   sysop/admin    dev/oracle/compat  scenarios/tests |
+--------------------------------------------------------------+
                |          |                |
                | normal   | normal         | explicit classic/oracle
                | runtime  | runtime        |
                v          v                v
+--------------------------------+   +-------------------------+
|           ec-engine            |   |        ec-compat        |
|--------------------------------|   |-------------------------|
| gameplay rules                 |   | classic DAT workflows   |
| maintenance                    |   | import/export bridge    |
| mapgen / movement / economy    |   | oracle materialization  |
+--------------------------------+   +-------------------------+
                 |                              |
                 v                              v
+--------------------------------+   +-------------------------+
|            ec-data             |   |       ec-classic        |
|--------------------------------|   |-------------------------|
| runtime store                  |   | raw classic records     |
| shared model                   |   | byte codecs             |
| snapshots / reports / mail     |   | DAT parsing/encoding    |
| fog of war                     |   +-------------------------+
+--------------------------------+               |
                 |                               v
                 v                    classic directories / DOS oracles
         SQLite / ecgame.db          DATABASE.DAT / RESULTS.DAT / MESSAGES.DAT
         authoritative state         ECGAME / ECMAINT / DOSBox-X
```

The key visual idea is:

- left side = normal runtime stack
- right side = compat/oracle stack

Even simpler:

`NORMAL PLAY / RUNTIME`

`frontend -> ec-engine -> ec-data -> SQLite`

`CLASSIC / ORACLE`

`frontend -> ec-compat -> ec-classic -> .DAT files / DOS binaries`

Read this sketch with the ownership rules above:

- `ec-client` does not parse classic `.DAT` files
- `ec-engine` owns gameplay rules, not classic file workflows
- `ec-data` owns shared runtime/store/model state
- `ec-classic` owns low-level classic byte/record helpers only
- `ec-sysop` owns public maintenance/setup flows
- `ec-cli` orchestrates explicit compat flows through `ec-compat`
- SQLite is authoritative; `.DAT` is the compatibility/oracle edge

## Maintenance Engine Structure

The Rust yearly maintenance engine is exposed and implemented through
`ec-engine/src/maint/`.

Shared maintenance result payloads remain in `ec-data::maintenance_types` so
multiple crates can consume the same plain event data without duplicating the
rule code.

Use [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
as the ordering spec, then reflect that ordering in code by phase-oriented
submodules rather than one giant driver file.

Current direction:

- `maint/mod.rs` owns orchestration and shared turn context only
- focused submodules own stable mechanic families such as:
  - sanitize
  - retarget
  - movement
  - merging
  - combat
  - economics
  - campaign
  - events

Guidelines for maint code:

- keep phase ordering explicit
- keep per-phase read/write scope clear
- return typed events from phase helpers instead of burying report-side facts in
  ad hoc string assembly
- move reusable rule logic into focused helpers when two phases or frontends
  need the same behavior
- do not let UI/client concerns leak into maint code

If `maint/mod.rs` starts accumulating mechanic-specific detail again, split that
detail into submodules instead of extending the driver further.

## Shared Model Boundary

`CoreGameData` remains the canonical gameplay-state snapshot model inside
`ec-data`.
`CampaignStore` is the persisted source of truth for active campaigns.

Use it when the code needs:

- shared cross-file validation
- the canonical state snapshot that engine code mutates
- classic directory load/save
- reusable scenario/setup/report helpers
- plain shared event/result payloads

If a transform expresses shared game-directory/runtime-store semantics rather
than one frontend's interaction policy, it should live on `CoreGameData` or in
a closely related `ec-data` helper module.

Examples:

- cross-file validators
- build/fleet/player input validation
- maintenance events
- shared report/intel projections
- raw record-layout helpers such as fleet motion scratch-field access/reset

The CLI and client should orchestrate those helpers, not replace them.

## Storage Boundary

The runtime storage direction is now active, not deferred:

- `CampaignStore` / `CampaignRuntimeState` in `ecgame.db` are the runtime
  source of truth for active campaigns
- `CoreGameData` is the canonical snapshot shape carried through storage and
  shared engine helpers
- classic `.DAT` files remain the compatibility boundary and oracle artifact set
- runtime-facing state should stay structured:
  - `CoreGameData`
  - report blocks
  - queued mail
  - per-player intel
  - campaign seed
- classic compatibility projections such as `DATABASE.DAT` belong to explicit
  import/export helpers, not to the normal runtime/client API surface

Practical rule:

- `ec-client` and normal Rust maintenance/mutator paths read and write SQLite
  runtime state
- explicit compatibility paths such as `db-export`, scenario materialization,
  and oracle setup are the only places that should intentionally write classic
  `.DAT` outputs, normally through `ec-compat`
- explicit import paths such as `db-import` are the only places that should
  rebuild runtime state from a classic directory, normally through `ec-compat`
- read-only inspection/report commands must not create or update `ecgame.db`
  as a side effect
- SQLite is the runtime source of truth; classic files remain the compatibility
  and oracle projection boundary

Do not bypass classic compatibility just because Rust-native storage exists, and
do not couple client/runtime logic directly to `DATABASE.DAT` or other classic
report artifacts. `CampaignRuntimeState` should not require compat-shaped
fields like `DatabaseDat`, `RESULTS.DAT` bytes, or `MESSAGES.DAT` bytes just to
drive the Rust engine or player client.

## Command And Report Ownership

Keep ownership clear:

- rule calculation belongs in `ec-engine`
- operator command selection / argument parsing belongs in `ec-sysop`
- developer/oracle command selection / argument parsing belongs in `ec-cli`
- screen flow / interaction belongs in `ec-client`
- player-visible report timing and header rules belong to the dedicated specs,
  then to shared `ec-engine` / `ec-data` helpers, not to CLI or client-only
  string logic

If the same report/intel/business rule is needed in more than one frontend,
promote it into `ec-engine` or a shared `ec-data` helper, depending on whether
the code is rule logic or runtime/store structure.

## Setup, Movement, Economy, And Combat Boundaries

Subsystem behavior should follow the companion specs:

- setup/map-generation constraints:
  [ec-setup-spec.md](ec-setup-spec.md)
- movement/pathfinding behavior:
  [ec-movement-spec.md](ec-movement-spec.md)
- build/economy behavior:
  [economics.md](economics.md)
- combat/hostile world resolution:
  [ec-combat-spec.md](ec-combat-spec.md)

The architecture consequence is straightforward:

- keep gameplay/rule execution in `ec-engine`
- keep shared models, invariants, and plain event/result payloads in `ec-data`
- keep the specs authoritative
- keep CLI/client layers as consumers of those rules

## Testing Direction

Testing rules:

- keep tests in crate `tests/` directories by default
- prefer regression tests tied to preserved fixtures and oracle-accepted
  directories
- split tests by concern rather than growing one giant integration test file
- after meaningful Rust changes, run `cargo test -q` from `rust/`
- for maint-sensitive behavior, keep using the oracle sweep and Rust maint sweep
  documented in [next-session.md](next-session.md)

The test split should mirror the code split. If a feature area deserves its own
module, it usually deserves its own test surface too.

## Avoid

Do not:

- grow giant `main.rs`, `mod.rs`, or catch-all utility files
- duplicate rules between `ec-engine`, `ec-data`, `ec-cli`, and `ec-client`
- bury classic byte semantics in UI or command code
- treat scenario-specific scripts as the long-term home for shared mechanics
- collapse maint ordering, combat rules, timing rules, and economy rules into
  one mega module
- use speculative semantic names in typed APIs before the evidence supports
  them

## Practical Heuristic

When adding or refactoring behavior, ask:

1. is this gameplay/rule execution, a shared data invariant, or a plain shared
   payload type?
2. is this a frontend workflow over an existing rule?
3. is this a presentation concern only?

Then place it accordingly:

- gameplay/rule execution -> `ec-engine`
- shared model/invariant/plain payload -> `ec-data`
- public sysop/admin workflow -> `ec-sysop`
- developer/oracle/compat workflow -> `ec-cli`
- player interaction/rendering -> `ec-client`

That placement rule matters more than preserving any one historical file tree.
