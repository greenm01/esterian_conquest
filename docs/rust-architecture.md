# Rust Architecture

The Rust workspace should follow a pragmatic data-oriented design. This is a
standing project rule, not a temporary preference.

## Principles

- keep binary layout explicit
- prefer plain data records plus small focused functions
- optimize for data flow and deterministic transforms, not object lifecycles
- avoid deep object hierarchies and abstraction layers
- split by data domain and command family, not by arbitrary utility classes
- keep logic DRY by centralizing shared field validation and report helpers
- avoid copy-pasted scenario logic when one record-level helper can express it
- treat preserved fixtures and original binaries as the acceptance oracle

## Module Direction

For `ec-data`:

- keep record/file layout code close to the bytes it represents
- keep parsing and serialization deterministic
- prefer stable typed accessors over semantic guesses
- prefer explicit record structs and free functions over sprawling impl blocks
- keep tests in `tests/`, not inline in source files

For `ec-cli`:

- keep `main.rs` as thin dispatch
- group commands by feature area in submodules
- keep shared parsing/path helpers in `support/`
- prefer explicit command functions over framework-style indirection
- keep batch/report paths built on shared pure validators instead of ad hoc
  command-specific checks
- keep multi-step experiment setup in shared scenario workflows instead of
  re-implementing chained per-command shell sequences

## Current Structure

`ec-cli` is now split into:

- `src/commands/compare.rs`
- `src/commands/compliance.rs`
- `src/commands/fleet_order.rs`
- `src/commands/planet_build.rs`
- `src/commands/guard_starbase.rs`
- `src/commands/inspect.rs`
- `src/commands/ipbm.rs`
- `src/commands/scenario.rs`
- `src/commands/setup.rs`
- `src/support/parse.rs`
- `src/support/paths.rs`

The current intended split is:

- `main.rs`: process boundary only
- `dispatch.rs`: top-level command routing
- `commands/`: feature- and workflow-oriented command families
- `support/`: shared parsing and path helpers
- `usage.rs`: top-level CLI usage/help text
- `workspace.rs`: shared fixture copying, initialization, and directory
  matching helpers

Scenario composition should stay centralized in `commands/scenario.rs`. When a
new experiment can be expressed as an ordered combination of known scenario
transforms, prefer extending that workflow instead of adding a one-off wrapper
command.

`ec-data` and `ec-tui` tests now live under crate `tests/` directories instead
of source-file `#[cfg(test)]` modules.

`ec-data` integration tests are now also split by concern instead of one large
`regression.rs` file:

- `tests/formats.rs`
- `tests/mutators.rs`
- `tests/ecmaint.rs`
- `tests/common/mod.rs`

`ec-cli` integration tests are now also split by command family instead of one
large `commands.rs` file:

- `tests/basic.rs`
- `tests/setup.rs`
- `tests/fleet.rs`
- `tests/planet.rs`
- `tests/starbase.rs`
- `tests/ipbm.rs`
- `tests/compliance.rs`
- `tests/common/mod.rs`

`ec-data` is now split into domain modules instead of one large `lib.rs`:

- `src/directory.rs`
- `src/records/player.rs`
- `src/records/planet.rs`
- `src/records/fleet.rs`
- `src/records/base.rs`
- `src/records/ipbm.rs`
- `src/records/setup.rs`
- `src/records/conquest.rs`
- `src/support.rs`

The intended split is:

- `lib.rs`: constants, module wiring, crate-root reexports only
- `directory.rs`: shared typed multi-file game-directory loading/saving for
  core `.DAT` workflows
- `records/`: record/file layouts and deterministic byte transforms
- `support.rs`: shared parse helpers and errors

This shared directory layer is the preferred way for multi-file compliance and
scenario commands to operate. Commands shall not reimplement ad hoc
`PLAYER.DAT`/`FLEETS.DAT`/`BASES.DAT`/`IPBM.DAT` load-save choreography when the
same workflow can use `ec_data::CoreGameData`.

`CoreGameData` is now more than a load/save container. Current-known structural
validator/normalizer logic should live on the data model when it reflects shared
directory semantics rather than command-specific UI behavior.

The same rule now applies to current-known multi-file scenario mutations. If a
transform expresses shared directory semantics rather than CLI interaction
policy, it should live on `CoreGameData` and the CLI should only load, invoke,
save, and report.

Current-known cross-file validation semantics should follow the same rule. The
CLI shall not be the source of truth for fleet/build/starbase/IPBM rule checks
once those checks are stable enough to live on `CoreGameData`.

The same requirement applies to current-known `IPBM` mutation helpers. Zero-fill
record shaping and mapped prefix-field updates shall live on `CoreGameData`,
with the CLI acting only as load/save/report orchestration.

Current-known compliance aggregation and key-word summaries shall follow the
same rule. If multiple CLI reports are reusing the same rule set and field
summary, that status aggregation belongs on `CoreGameData`.

Current-known starbase linkage checks shall also live on the shared model. When
the code needs to follow a guarded base by index and compare decoded fleet/base
fields, that linkage logic belongs in `ec-data`, even if the CLI still exposes
one-base scenario workflows on top of it.

When a current-known rule naturally applies to all matching records, the shared
model shall expose the aggregate form too. Guard-starbase compliance should not
be implicitly hardcoded to fleet 1 when the rule is really “all guarding
fleets”.

Reports built on those rules shall prefer the shared aggregate too. If the CLI
needs counts or summaries of guarding fleets, that should come from the shared
model view rather than from ad hoc command-local scans.

When the next phase needs richer per-record starbase analysis, the starting
point shall be shared guarding-fleet linkage summaries from `CoreGameData`, not
new command-local byte inspection.

Current-known core count rules shall prefer per-empire owned-record semantics
where the RE supports them. For player 1, starbase count normalization and
validation shall be based on player-1-owned base records, not blindly on total
`BASES.DAT` length.

The same pattern shall extend across players when the field semantics are clear.
Starbase-count normalization is now a per-player owned-base rule, even though
`IPBM` count normalization remains intentionally player-1-scoped for now.

Current-known fleet topology shall be modeled from the fleet table itself, not
from speculative player-side ownership words. The initialized/post-maint
preserved `FLEETS.DAT` shape supports a deterministic four-fleet-per-player
block rule (IDs, local slots, prev/next links), and that topology is now part
of shared validation. By contrast, non-player-1 `PLAYER.DAT.fleet_chain_head`
words in the current 88-byte model are not stable enough to treat as generic
cross-file truth.

That initialized/post-maint fleet model now also includes the current-known
slot/block payload rules proven by preserved fixtures:
- slots `1` and `2` carry the cruiser+ETAC loadout with max speed `3`
- slots `3` and `4` carry the destroyer loadout with max speed `6`
- current speed is `0` in the preserved baseline
- location and mission bytes are constant within each four-fleet empire block

Current-known ownership semantics shall also prefer explicit owner fields where
the file format is stable. `PLANETS.DAT.owner_empire_slot_raw()` and
`BASES.DAT.owner_empire_raw()` now feed shared per-player ownership counts and
range validation, so broader multi-player compliance work should build on those
fields before leaning on weaker inferred relationships.

Current expectation:

- `fleet-order`, `planet-build`, `guard-starbase`, `ipbm`, and
  `compliance-report` paths should use the shared directory model where they
  touch multiple files or participate in scenario-composition workflows
- full directory inspection should also use the shared directory model; keep
  narrower header-only commands on the lighter targeted file reads they need
- directory-level structural reports/validators should be layered on top of the
  shared model, but they shall be explicit about scope when they encode only
  current-known invariants rather than full engine behavior
- if a current-known invariant is repairable without guessing at unknown engine
  semantics, prefer adding an explicit normalization command on top of the
  shared model rather than forcing users to hand-edit raw bytes

## KDL Timing

If a KDL scenario/order layer is added later, it should sit on top of the
internal Rust gamestate/order model after that model stabilizes. It should not
drive the low-level layout design prematurely.
