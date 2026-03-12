# esterian_conquest

Preservation and reverse-engineering workspace for Esterian Conquest v1.5.

Current focus:
- documenting the original DOS game behavior and file formats
- preserving confirmed reverse-engineering findings
- building a Rust compatibility/preservation toolchain
- making Rust-generated gamestate files 100% compliant with the original game
  and `ECMAINT`
- expanding Rust from fixture copying toward scenario generation from decoded
  fields

Current milestone ladder:
- `known accepted scenarios`: in progress and already useful
- `parameterized scenario generation`: started, but incomplete
- `general compliant gamestate generation`: not there yet
- `full Rust ECMAINT replacement`: later milestone after the above

Current top-level contents:
- `original/`: local snapshot of original EC 1.5 files used for preservation and testing
- `docs/`: stable project docs for approach, fixtures, and ECMAINT planning
- `RE_NOTES.md`: working reverse-engineering notes
- `rust/`: preservation-oriented Rust workspace
- `tools/`: unpacking and analysis helpers used during investigation

Docs:
- `docs/approach.md`: preservation and porting strategy
- `docs/fixtures.md`: fixture creation and usage workflow
- `docs/ecmaint-plan.md`: current plan for reverse engineering the maintenance engine
- `docs/ecmaint-combat-reference.md`: combat-oriented historical validation references
- `docs/ghidra-workflow.md`: headless Ghidra install and ECMAINT analysis workflow
- `docs/planet-report-reference.md`: coordinate-linked scouting/world stat references
- `docs/next-session.md`: exact restart point for the next ECMAINT experiment

Current Rust milestone:
- `cargo test -p ec-cli` now verifies that Rust can rewrite a compliant
  `fixtures/ecmaint-post/v1.5` snapshot into two preserved accepted pre-maint
  scenarios using decoded fields instead of wholesale fixture replacement:
  - `ec-cli fleet-order ...` reproduces `fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT`
  - `ec-cli planet-build ...` reproduces `fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT`
  - `ec-cli scenario <dir> fleet-order` and `ec-cli scenario <dir> planet-build`
    now expose those same accepted rewrites as named scenario commands
  - `ec-cli validate <dir> fleet-order` and `ec-cli validate <dir> planet-build`
    check the currently-known accepted field values for those preserved
    scenarios
  - `ec-cli scenario-init [source_dir] <target_dir> fleet-order`
    and `... planet-build` materialize runnable scenario directories from a
    compliant baseline in one command
- Rust can also now emit an accepted Guard Starbase scenario from the same
  compliant baseline:
  - `ec-cli scenario <dir> guard-starbase`
  - verified against `fixtures/ecmaint-starbase-pre/v1.5` for `PLAYER.DAT`,
    `FLEETS.DAT`, and `BASES.DAT`
  - the `BASES.DAT` output now comes from named `BaseRecord` field setters,
    not a raw 35-byte template constant
- Rust can now also validate the currently-known accepted one-base Guard
  Starbase shape directly:
  - `ec-cli validate <dir> guard-starbase`
  - this validator now checks the explicit one-base linkage keys across
    `PLAYER.DAT`, `FLEETS.DAT`, and `BASES.DAT` instead of only comparing one
    accepted base record byte-for-byte
- `ec-cli validate <dir> all` now classifies a directory against all currently
  known accepted scenarios and reports which ones match
- `ec-cli validate-preserved <dir> <scenario>` now checks exact byte-for-byte
  agreement with the preserved accepted fixture files for that scenario
- `ec-cli compare-preserved <dir> <scenario>` now reports scenario-focused
  byte diffs against the preserved accepted fixture files
- `ec-cli scenario original/v1.5 list` now prints the current Rust-side
  scenario catalog
- `ec-cli scenario original/v1.5 show <scenario>` now prints the preserved
  fixture path and exact-match files for a known scenario
- `ec-cli scenario-init-all [source_dir] <target_root>` now materializes all
  currently-known accepted scenario directories in one run
- Rust can now materialize a runnable Guard Starbase scenario directory from a
  compliant baseline in one command:
  - `ec-cli scenario-init [source_dir] <target_dir> guard-starbase`
  - if `source_dir` is omitted, `scenario-init` now correctly defaults to the
    compliant `fixtures/ecmaint-post/v1.5` baseline
