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

Current expectation:

- `fleet-order`, `planet-build`, `guard-starbase`, `ipbm`, and
  `compliance-report` paths should use the shared directory model where they
  touch multiple files or participate in scenario-composition workflows

## KDL Timing

If a KDL scenario/order layer is added later, it should sit on top of the
internal Rust gamestate/order model after that model stabilizes. It should not
drive the low-level layout design prematurely.
