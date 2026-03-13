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

The initialized/post-maint fleet baseline now also includes exact mission
semantics, not just per-block consistency:
- every fleet in those preserved four-fleet blocks uses standing order `5`
  (`Guard/blockade world`)
- every fleet in a block targets that empire's homeworld-seed coordinates
- every fleet in a block also keeps mission aux bytes `[1, 0]`, so the full
  preserved mission pattern is `[5, x, y, 1, 0]`

The preserved initialized/post-maint control-file baseline is now explicit too:
- `SETUP.DAT.version_tag()` stays `EC151`
- `SETUP.DAT.option_prefix()` stays `[4, 3, 4, 3, 1, 1, 1, 1]`
- snoop and remote timeout stay enabled
- local timeout stays disabled
- max-time-between-keys stays `10`
- minimum granted time, purge-after-turns, and autopilot-inactive-turns stay `0`
- `CONQUEST.DAT.player_count()` stays `4`
- `CONQUEST.DAT.maintenance_schedule_bytes()` stays `[1, 1, 1, 1, 1, 1, 1]`
- `CONQUEST.DAT.game_year()` stays within the preserved initialized/post-maint
  pair `3000` / `3001`

Those control/count rules are now also repairable through the shared model:
- `CoreGameData::sync_current_known_baseline_controls_and_counts()` repairs
  current-known starbase/IPBM count words
- rewrites `SETUP.DAT` baseline control fields
- rewrites `CONQUEST.DAT` player-count and maintenance-schedule baseline fields
- normalizes invalid `CONQUEST.DAT` years back into the preserved
  initialized/post-maint pair by setting `3001`
- this is intentionally narrower than full baseline reconstruction; it only
  touches deterministic fields the current RE actually supports

The same now applies to the deterministic initialized fleet baseline:
- `CoreGameData::sync_current_known_initialized_fleet_baseline()` rebuilds the
  four-fleet-per-player preserved baseline from the current homeworld-seed
  coordinates
- it rewrites fleet count/ordering topology, payload/loadout bytes, mission
  bytes, and current locations
- this is still current-known and intentionally bounded: it reconstructs the
  preserved initialized/post-maint fleet blocks, not arbitrary live fleet state

The same pattern now exists for deterministic initialized/post-maint planet
payloads:
- `CoreGameData::sync_current_known_initialized_planet_payloads()` rewrites the
  current-known payload fields for:
  - owned homeworld seeds
  - unowned planets
- it repairs name/summary bytes, tax, stored goods, factories, build queue,
  stardock, population, developed value, likely-army count, and ownership
  status for those already-classified records
- this is intentionally bounded too: it does not guess new homeworld ownership
  or rebuild arbitrary planet topology, it only normalizes the payload fields
  the current model already validates

Those bounded repair surfaces now compose into one shared baseline synchronizer:
- `CoreGameData::sync_current_known_initialized_post_maint_baseline()`
- this composes:
  - empty auxiliary-state repair
  - control/count repair
  - initialized fleet baseline repair
  - initialized planet payload repair
- the CLI-facing goal is practical baseline construction/repair for current-known
  compliant directories, while still staying explicit about scope and avoiding
  guesses beyond the validated model

That composed synchronizer now also has a directory-constructor wrapper:
- `ec-cli core-init-current-known-baseline [source_dir] <target_dir>`
- it copies a source tree, applies the shared current-known baseline
  synchronizer, and writes a ready-to-validate output directory
- this is the preferred CLI path when the task is “materialize a baseline
  experiment directory” rather than “repair the directory I already have”

There is now also an explicit drift-reporting surface for the current-known
baseline:
- `CoreGameData::current_known_baseline_diff_counts()`
- `CoreGameData::current_known_baseline_diff_offsets()`
- CLI: `ec-cli core-diff-current-known-baseline [dir]`
- CLI: `ec-cli core-diff-current-known-baseline-offsets [dir]`
- this compares a directory against the Rust-generated current-known baseline
  normalization and reports either per-file differing-byte counts or exact byte
  offsets for the core `.DAT` set
- this answers: "what would the current-known normalizer still change?"
- this is still useful because the current-known normalizer is only guaranteed
  byte-complete for the preserved initialized -> post-maint baseline, not for
  noisier shipped/sample states like `original/v1.5`
- practical milestone:
  - `ec-cli core-init-current-known-baseline fixtures/ecutil-init/v1.5 <dir>`
    now yields a directory whose canonical diff against
    `fixtures/ecmaint-post/v1.5` is empty for every core `.DAT` file
- for the noisier shipped sample, use:
  - `ec-cli core-init-current-known-baseline original/v1.5 /tmp/ec-from-original`
  - `ec-cli core-report-canonical-transition-clusters /tmp/ec-from-original`
  - this groups the remaining canonical drift by record for `PLAYER.DAT`,
    `PLANETS.DAT`, and `FLEETS.DAT`
  - after the `CONQUEST.DAT` header sync milestone, those are now the only
    remaining core-file drift surfaces for the normalized shipped sample
  - this is now the preferred queue-building surface for the next
    transition-rule RE phase
- first concrete payoff:
  - the repeated `FLEETS.DAT` drift clusters were traced to missing initialized
    fleet owner-byte and tuple-marker semantics
  - the shared initialized fleet baseline now includes:
    - `owner_empire` at fleet offset `0x02`
    - tuple A `[0x80, 0, 0, 0, 0]`
    - tuple B `[0x80, 0, 0, 0, 0]`
    - tuple C `[0x81, 0, 0, 0, 0]`
  - for the preserved `fixtures/ecmaint-post/v1.5` baseline, `FLEETS.DAT` is
    now byte-complete under the current-known normalizer, leaving `PLAYER.DAT`
    and `PLANETS.DAT` as the next byte-completion targets
- second concrete payoff:
  - the repeated `PLAYER.DAT` drift words at offsets `156/157`, `244/245`, and
    `332/333` were traced to overreaching normalization of non-player-1 count
    words
  - the current 88-byte player-record model only treats player 1's starbase and
    `IPBM` count words as validated semantics
  - the shared baseline now preserves player 2..5 count words as raw /
    uninterpreted bytes instead of rewriting them
  - for the preserved `fixtures/ecmaint-post/v1.5` baseline, `PLAYER.DAT` is
    now byte-complete under the current-known normalizer, leaving `PLANETS.DAT`
    as the main remaining drift target
- third concrete payoff:
  - the remaining `PLANETS.DAT` drift clusters were the six hidden bytes after
    the visible `"Unowned"` name prefix in non-homeworld records
  - those bytes are preserved per-planet payload, not generic string padding
  - the shared baseline now uses a prefix-only setter for `"Unowned"` records
    so it preserves that hidden tail instead of zeroing it
  - for the preserved `fixtures/ecmaint-post/v1.5` baseline, the current-known
    normalizer is now byte-complete across the full core `.DAT` set

There is now a second drift-reporting surface for the canonical preserved
post-maint oracle:
- CLI: `ec-cli core-diff-canonical-current-known-baseline [dir]`
- CLI: `ec-cli core-diff-canonical-current-known-baseline-offsets [dir]`
- these compare a directory directly against the canonical preserved core
  baseline in `fixtures/ecmaint-post/v1.5`
- this answers: "how far is this directory from the preserved oracle bytes?"
- this is intentionally different from normalizer drift:
  - a directory can be a current-known fixed point of the Rust normalizer
    without matching the canonical preserved post-maint core bytes
  - for example, normalizing `original/v1.5` currently yields a valid
    current-known state, but the canonical diff still reports remaining gaps
    in `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `CONQUEST.DAT`

That byte-complete baseline now supports an exact-match oracle:
- CLI: `ec-cli core-validate-current-known-baseline [dir]`
- this is stricter than `ec-cli core-validate [dir]`
  - `core-validate` checks the current-known structural and semantic rules
  - `core-validate-current-known-baseline` checks byte-identical equality to
    the canonical preserved post-maint core baseline in
    `fixtures/ecmaint-post/v1.5`
- that exact oracle now also feeds normal directory classification:
  - `ec-cli match <dir>` prints `MATCH current-known-post-maint-baseline-core`
    when the directory's core `.DAT` set is an exact byte match for that
    canonical preserved post-maint core baseline
- important boundary:
  - a directory can be a valid current-known fixed point of the Rust
    normalizer without being canonical
  - for example, normalizing `original/v1.5` directly currently yields a valid
    current-known state, but not the canonical preserved post-maint core bytes

The CLI now also has exact canonical constructor/synchronizer paths:
- `ec-cli core-sync-canonical-current-known-baseline [dir]`
- `ec-cli core-init-canonical-current-known-baseline [source_dir] <target_dir>`
- these overlay the canonical preserved post-maint core `.DAT` set from
  `fixtures/ecmaint-post/v1.5`
- practical meaning:
  - use `core-sync-current-known-baseline` when you want the bounded
    current-known normalizer
  - use `core-sync-canonical-current-known-baseline` when you want exact
    preserved oracle bytes for the core `.DAT` set
  - use `core-init-canonical-current-known-baseline` when you want to preserve
    top-level programs/non-core files from some source tree while materializing
    the exact preserved post-maint core baseline on top

The shared model also now treats homeworld-seed alignment as a current-known
multi-player invariant for initialized/post-maint state:
- each active empire has exactly one owned `Not Named Yet` planet
- that planet's coordinates match the owning empire's four-fleet block location
  and mission target bytes

The initialized/post-maint planet baseline is now explicit too:
- owned planets are restricted to those homeworld seeds
- non-homeworld planets remain unowned in the preserved baseline
- owned homeworld seeds carry ownership status `2`

The owned homeworld-seed payload is also now part of the current-known baseline:
- header bytes carry the preserved `100 / 135` seed markers
- `developed_value_raw()` is `10`
- `likely_army_count_raw()` is `4`
- build queue, stardock, and population payloads remain zeroed

The complementary unowned-planet payload baseline is now explicit too:
- non-homeworld planets remain `Unowned`
- owner slot and ownership status stay `0`
- developed value and likely-army markers stay `0`
- build queue, stardock, and population payloads remain zeroed

That planet baseline now includes the current-known economic payload too:
- homeworld seeds keep tax rate `12`
- homeworld seeds keep factories payload `[0, 0, 0, 0, 72, 134]`
- homeworld seeds keep stored goods at `0`
- unowned planets keep tax rate `0`
- unowned planets keep factories all zero
- unowned planets keep stored goods at `0`

The initialized/post-maint baseline also now treats auxiliary state as
intentionally empty:
- `BASES.DAT` is empty
- `IPBM.DAT` is empty
- guarding-fleet count is `0`
- this is a preserved baseline rule, distinct from later scenario-specific
  valid directories

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
