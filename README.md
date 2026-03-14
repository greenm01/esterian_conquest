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
- replacing original RNG combat with a documented deterministic combat model in
  Rust while keeping classic save compatibility

Current milestone ladder:
- `known accepted scenarios`: in progress and already useful
- `parameterized scenario generation`: started, but incomplete
- `general compliant gamestate generation`: not there yet
- `full Rust ECMAINT replacement`: later milestone after the above
- `scenario DSL / KDL layer`: defer until the internal Rust gamestate/order
  model stabilizes; add it as a serialization layer on top of the compliant
  generator rather than as the next RE milestone

Current top-level contents:
- `original/`: local snapshot of original EC 1.5 files used for preservation and testing
- `docs/`: stable project docs for approach, fixtures, and ECMAINT planning
- `RE_NOTES.md`: working reverse-engineering notes
- `rust/`: preservation-oriented Rust workspace
- `tools/`: unpacking and analysis helpers used during investigation

Docs:
- `docs/approach.md`: preservation and porting strategy
- `docs/setup-kdl-schema.md`: first schema for declarative sysop/setup config
- `docs/fixtures.md`: fixture creation and usage workflow
- `docs/ecmaint-plan.md`: current plan for reverse engineering the maintenance engine
- `docs/ec-combat-spec.md`: canonical deterministic combat rules for the Rust port
- `docs/config-architecture.md`: Rust-vs-KDL ownership and future config layering
- `docs/ecmaint-combat-reference.md`: combat-oriented historical validation references
- `docs/ec-setup-spec.md`: manual-driven setup and starmap rules for Rust
- `docs/starmap-generation-spec.md`: fair homeworld placement and planet distribution algorithm
- `docs/ec-movement-spec.md`: classic movement semantics and canonical routing policy
- `docs/ghidra-workflow.md`: headless Ghidra install and ECMAINT analysis workflow
- `docs/planet-report-reference.md`: coordinate-linked scouting/world stat references
- `docs/rust-architecture.md`: Rust module layout and data-oriented design notes
- `docs/next-session.md`: exact restart point for the next ECMAINT experiment

Default black-box oracle loop for new mechanics:
- `python3 tools/ecmaint_oracle.py prepare /tmp/ecmaint-oracle [source_dir]`
- submit one controlled order family or mutate one narrow field family
- `python3 tools/ecmaint_oracle.py run /tmp/ecmaint-oracle`
- inspect `.oracle/` snapshots plus the reported `.DAT`/report diff clusters

Sysop/admin setup surface:
- `ec-cli sysop new-game <target_dir> [--players <1-4>] [--seed <u64>]`
- `ec-cli sysop new-game <target_dir> --config rust/ec-data/config/setup.example.kdl`
- `ec-cli sysop generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]`
- `ec-cli sysop setup-programs [dir]`
- `ec-cli sysop port-setup [dir]`
- `ec-cli sysop snoop <dir> <on|off>`
- `ec-cli sysop maintenance-days <dir> set <sun|mon|...>`

The older flat setup commands remain available as compatibility aliases, but
new admin/setup work should prefer the `sysop` command family and, over time,
declarative KDL config.

Known scenario replay:
- `python3 tools/ecmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
- `python3 tools/ecmaint_oracle.py replay-known planet-build /tmp/ecmaint-build-oracle`
- `python3 tools/ecmaint_oracle.py replay-known guard-starbase /tmp/ecmaint-starbase-oracle`

Exact replayable pre-maint directory init:
- `ec-cli scenario-init-replayable [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase>`
- this overlays the shared preserved pre-maint `CONQUEST.DAT` / `DATABASE.DAT`
  replay context on top of the existing scenario generator so the output
  matches the preserved pre-maint fixture exactly for the known scenarios

Preserved fixture replay validation:
- `python3 tools/ecmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved planet-build /tmp/ecmaint-build-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved guard-starbase /tmp/ecmaint-starbase-pre-direct`

Current Rust milestone:
- `ec-cli` is now split by feature area instead of letting `main.rs` keep
  growing:
  - `src/commands/fleet_order.rs`
  - `src/commands/planet_build.rs`
  - `src/commands/guard_starbase.rs`
  - `src/commands/ipbm.rs`
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
- Rust now also has a safe parameterized one-base Guard Starbase writer:
  - `ec-cli guard-starbase-onebase <dir> <target_x> <target_y>`
  - keeps the current known-good linkage keys fixed while varying the guard
    target/base coordinates
- Rust now also has a starbase-linkage inspection command:
  - `ec-cli guard-starbase-report <dir>`
  - prints the current player/fleet/base linkage words and the validator
    verdict for quick ECMAINT experiment setup
- Rust now also has a one-command parameterized directory initializer:
  - `ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>`
  - defaults to the compliant `fixtures/ecmaint-post/v1.5` baseline when
    `source_dir` is omitted
- Rust now also has a batch coordinate-variant starbase initializer:
  - `ec-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...`
  - writes multiple ECMAINT-ready one-base Guard Starbase directories plus a
    manifest in one run
- Rust now also has the first practical `IPBM.DAT` controls:
  - `ec-cli ipbm-report <dir>`
  - `ec-cli ipbm-zero <dir> <count>`
  - `ec-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>`
  - `ec-cli ipbm-validate <dir>`
  - `ec-cli ipbm-init [source_dir] <target_dir> <count>`
  - `ec-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...`
  - these are enough to inspect and satisfy the currently-known
    `PLAYER[0x48]` / `IPBM.DAT` count-length gate from Rust and to start
    emitting non-zero structural record prefixes
  - `ec-data::IpbmRecord` now also exposes the currently mapped tuple tags,
    tuple payload groups, and trailing control bytes, and `ec-cli ipbm-report`
    prints them
- Rust now also has a combined integrity-focused inspection command:
  - `ec-cli compliance-report <dir>`
  - `ec-cli compliance-batch-report <root>`
  - this summarizes the current Guard Starbase linkage verdict, the current
    `IPBM` count/length verdict, and the most relevant key words in one pass
  - the batch form scans a directory of generated scenarios and prints a
    concise per-directory compliance summary
  - the batch form now covers `fleet-order`, `planet-build`,
    `guard-starbase`, and `ipbm`
- `ec-cli validate <dir> all` now classifies a directory against all currently
  known accepted scenarios and reports which ones match
- Rust now also has parameterized fleet/build inspection and init commands:
  - `ec-cli fleet-order-report [dir] [fleet_record]`
  - `ec-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
  - `ec-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>...`
  - `ec-cli planet-build-report [dir] [planet_record]`
  - `ec-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>`
  - `ec-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>...`
  - these default to the compliant `fixtures/ecmaint-post/v1.5` baseline and
    make the fleet/build paths consistent with the existing starbase/IPBM
    report/init workflow
  - the new batch forms materialize multiple parameterized directories plus
    manifests in one run
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
