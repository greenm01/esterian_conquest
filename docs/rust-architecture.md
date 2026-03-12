# Rust Architecture

The Rust workspace should follow a pragmatic data-oriented design.

## Principles

- keep binary layout explicit
- prefer plain data records plus small focused functions
- avoid deep object hierarchies and abstraction layers
- split by data domain and command family, not by arbitrary utility classes
- treat preserved fixtures and original binaries as the acceptance oracle

## Module Direction

For `ec-data`:

- keep record/file layout code close to the bytes it represents
- keep parsing and serialization deterministic
- prefer stable typed accessors over semantic guesses
- keep tests in `tests/`, not inline in source files

For `ec-cli`:

- keep `main.rs` as thin dispatch
- group commands by feature area in submodules
- keep shared parsing/path helpers in `support/`
- prefer explicit command functions over framework-style indirection

## Current Structure

`ec-cli` is now split into:

- `src/commands/guard_starbase.rs`
- `src/commands/ipbm.rs`
- `src/support/parse.rs`
- `src/support/paths.rs`

`ec-data` and `ec-tui` tests now live under crate `tests/` directories instead
of source-file `#[cfg(test)]` modules.

## KDL Timing

If a KDL scenario/order layer is added later, it should sit on top of the
internal Rust gamestate/order model after that model stabilizes. It should not
drive the low-level layout design prematurely.
