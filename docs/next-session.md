## Accomplished
- Completed the reverse engineering of the token gate mechanism and the `16A4` integrity bypass flag.
- Exhaustively proved that `DS:16A4` is never set to 1 due to a likely developer typo (command line `/B` sets `16A2`, but the integrity check tests `16A4`).
- Discovered the true reason `.TOK` files "bypass" the crash: the presence of `Move.Tok` triggers an automatic restore of `.SAV` backups over the `.DAT` files prior to the integrity check, causing the repaired files to pass naturally.
- Confirmed the planet stardock capacity: it is a 10-slot array of ship counts (`0x38..0x4B`) and corresponding ship kinds (`0x4C..0x55`), matching the 10-slot build queue.
- Parameterized the Guard Starbase generator to dynamically copy `tuple_a`, `tuple_b`, and `tuple_c` payloads directly from the guarding fleet, completely removing hardcoded byte arrays.
- Documented findings in `token-investigation.md`.
- Rust scenario-writer milestone:
  - `ec-cli` is now split further by feature area:
    - `src/commands/compare.rs`
    - `src/commands/compliance.rs`
    - `src/commands/fleet_order.rs`
    - `src/commands/planet_build.rs`
    - `src/commands/guard_starbase.rs`
    - `src/commands/inspect.rs`
    - `src/commands/ipbm.rs`
    - `src/commands/scenario.rs`
    - `src/commands/setup.rs`
  - `fleet-order` and `planet-build` no longer live as one-off logic inside
    `main.rs`; they now follow the same module/report/init pattern as the
    starbase/IPBM workflows
  - scenario catalog / init / preserved-validation logic also now lives in its
    own command module instead of being embedded directly in `main.rs`
  - compliance/batch-report, compare/preserved-diff, inspect/header dumping,
    and setup/config editing are also now extracted from `main.rs`
  - current refactor checkpoint:
    - `rust/ec-cli/src/main.rs` is now `767` lines, down from the original
      multi-thousand-line monolith
    - full `cargo test` is green after the extraction
  - `ec-data` now parses `BASES.DAT` (`35`-byte records) and `IPBM.DAT`
    (`0x20`-byte records) to the current structural level
  - `ec-cli fleet-order <dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
    rewrites decoded fleet-order fields in place
  - `ec-cli planet-build <dir> <planet_record> <build_slot_raw> <build_kind_raw>`
    rewrites decoded build-queue bytes in place
  - latest Rust scenario CLI milestone:
    - `ec-cli scenario <dir> fleet-order`
    - `ec-cli scenario <dir> planet-build`
    - `ec-cli validate <dir> fleet-order`
    - `ec-cli validate <dir> planet-build`
    - `ec-cli validate <dir> all`
    - `ec-cli validate-preserved <dir> <scenario>`
    - `ec-cli validate-preserved <dir> all`
    - `ec-cli compare-preserved <dir> <scenario>`
    - `ec-cli compare-preserved <dir> all`
    - `ec-cli scenario <dir> list`
    - `ec-cli scenario <dir> show <scenario>`
    - `ec-cli scenario-init-all [source_dir] <target_root>`
    - `ec-cli scenario-init [source_dir] <target_dir> fleet-order`
    - `ec-cli scenario-init [source_dir] <target_dir> planet-build`
    - these now turn the two already-preserved accepted fleet/build rewrites
      into named runnable scenarios rather than only low-level mutator calls
    - latest parser/usability correction:
      - `ec-cli init <target_dir>` now correctly uses the documented default
        source `original/v1.5`
      - `ec-cli scenario-init <target_dir> <scenario>` now correctly uses the
        documented scenario baseline `fixtures/ecmaint-post/v1.5`
  - `cargo test -p ec-cli` now proves two exact fixture recreations from the
    compliant `fixtures/ecmaint-post/v1.5` baseline:
    - `FLEETS.DAT` for `fixtures/ecmaint-fleet-pre/v1.5`
    - `PLANETS.DAT` for `fixtures/ecmaint-build-pre/v1.5`
  - new accepted starbase scenario command:
    - `ec-cli scenario <dir> guard-starbase`
    - verified against `fixtures/ecmaint-starbase-pre/v1.5` for:
      - `PLAYER.DAT`
      - `FLEETS.DAT`
      - `BASES.DAT`
    - latest Rust milestone:
      - `BASES.DAT` is no longer emitted from a raw 35-byte template constant
      - `ec-data::BaseRecord` now exposes named setters for the currently
        mapped integrity-critical fields:
        - local slot
        - active flag
        - base ID
        - link word
        - chain word
        - coords
        - tuple A/B/C payload blocks
        - trailing coords
        - owner empire
      - `cargo test` is green for the full Rust workspace after this change
  - `ec-cli validate <dir> guard-starbase` now checks the currently-known
    accepted one-base Guard Starbase invariants across `PLAYER.DAT`,
    `FLEETS.DAT`, and `BASES.DAT`
    - latest refinement:
      - it now validates the one-base linkage keys directly instead of only
        comparing a preserved accepted base blob
      - current explicit checks cover:
        - player starbase-count word
        - fleet local-slot word
        - fleet ID word
        - base chain word
        - guard starbase index/enable bytes
        - base/fleet coordinate agreement
      - Rust-side explicitly linked Guard Starbase encoder milestone:
        - Replaced the hard-coded single-base template with an explicit encoder that generates `BASES.DAT` linkage fields directly from the `PLAYER.DAT` and `FLEETS.DAT` states
        - Enforced strict linkage-key validation checking that `base.summary_word_raw() == fleet.local_slot_word_raw()` and matching `base.chain_word_raw() == fleet.fleet_id_word_raw()`
        - Demonstrated that the kind-1 / kind-2 linkage semantics described by the `3558/355A` decode logic fully align with the preserved one-base fixture values
      - Rust-side next-step milestone already landed:
        - `ec-cli guard-starbase-onebase <dir> <target_x> <target_y>`
        - this emits a parameterized one-base guard-starbase directory while
          keeping the currently-known accepted linkage keys fixed
        - `ec-cli guard-starbase-report <dir>`
        - this prints the current player/fleet/base linkage words and the
          current validator verdict, which is now the quickest Rust-side
          inspection tool before handing a directory to `ECMAINT`
        - `ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>`
        - this now materializes a coordinate-parameterized one-base Guard
          Starbase directory in one command, defaulting to
          `fixtures/ecmaint-post/v1.5`
        - `ec-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...`
        - this now materializes multiple coordinate-parameterized one-base
          Guard Starbase directories plus a manifest in one run
        - `ec-cli ipbm-report <dir>`
        - `ec-cli ipbm-zero <dir> <count>`
        - `ec-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>`
        - `ec-cli ipbm-validate <dir>`
        - `ec-cli ipbm-init [source_dir] <target_dir> <count>`
        - `ec-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...`
        - these now cover the first practical Rust-side `IPBM.DAT` workflow:
          inspect the current count/length state, then materialize a zero-filled
          valid record family that satisfies the known `PLAYER[0x48]` gate,
          then start populating the decoded record prefix fields from Rust
        - latest refinement:
          - `ec-data::IpbmRecord` now exposes the currently mapped tuple tags,
            tuple payload groups, and trailing control bytes
          - `ec-cli ipbm-report` now prints those groups directly, so Rust-side
            scenario shaping no longer needs raw hex inspection for the mapped
            parts of the record
        - `ec-cli compliance-report <dir>`
        - `ec-cli compliance-batch-report <root>`
        - this now gives the quickest Rust-side preflight for the currently
          mapped integrity-sensitive areas:
          - Fleet Order
          - Planet Build
          - Guard Starbase linkage
          - `IPBM.DAT` count/length state
        - the batch form now gives a compact per-directory verdict for roots
          produced by the Rust scenario generators
        - `ec-cli fleet-order-report [dir] [fleet_record]`
        - `ec-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
        - `ec-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>...`
        - `ec-cli planet-build-report [dir] [planet_record]`
        - `ec-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>`
        - `ec-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>...`
        - these now make fleet/build experiment directories as easy to
          materialize and inspect as the existing starbase/IPBM flows
        - the new batch forms now make it practical to generate multiple
          parameterized fleet/build experiment roots without hand-copying the
          same baseline directory repeatedly
  - `ec-cli scenario-init [source_dir] <target_dir> guard-starbase`
    materializes a runnable scenario directory in one command from a
    compliant baseline
  - practical meaning:
    - Rust can now emit accepted pre-maint scenario files from decoded fields,
      not just copy preserved fixture trees wholesale
    - Rust can now generate and validate accepted fleet/build/starbase
      scenarios through a consistent scenario-oriented CLI
    - Rust can now also materialize and inspect parameterized fleet/build
      experiment directories directly from the compliant post-maint baseline
    - Rust can now classify a directory against all currently-known accepted
      scenarios with one command
    - Rust can now also distinguish "matches the currently-known rule shape"
      from "exactly matches the preserved accepted fixture bytes"
    - Rust can now diff a generated scenario directly against the preserved
      accepted fixture files for the same scenario
    - Rust now has a first-class scenario catalog and can materialize all
      currently-known accepted scenarios under one target root in one run
    - Rust can also reject obviously non-compliant starbase snapshots before
      running the original engine
    - Rust can now generate a ready-to-run preserved scenario directory, not
      just mutate an existing one in place
    - the next Rust-side milestone should build on this by replacing more
      scenario-specific raw blobs with explicit field encoders/validators

## Next Steps
The token-gate investigation is complete. The next work should return to the
main maintenance engine and focus on the remaining port-critical unknowns.

Primary project goal:

- reverse engineer enough of the original formats and engine rules to generate
  100% compliant gamestate files from Rust
- treat the original game and `ECMAINT` as the acceptance oracle for that
  milestone
- use that milestone as the first concrete step toward a full Rust port

Later serialization-layer milestone:

- add a KDL-backed scenario/order format only after the internal Rust
  gamestate/order model stabilizes
- treat that as a layer on top of the compliant generator, not as the next RE
  milestone

Rust architecture note:

- recent refactor milestone:
  - `ec-cli` Guard Starbase/IPBM logic now lives in command submodules
  - shared argument/path helpers now live under `ec-cli/src/support/`
  - `ec-data` and `ec-tui` tests now live in crate `tests/` directories
- preserve this direction:
  - data-oriented layout
  - feature/domain submodules
  - thin `main.rs`

Milestone ladder:

1. Known accepted scenarios
   - current state: active and productive
   - Rust can already emit, validate, and materialize preserved accepted
     `fleet-order`, `planet-build`, and `guard-starbase` scenarios

2. Parameterized scenario generation
   - current state: partially started
   - next work here is to replace more accepted-value blobs and
     scenario-specific assumptions with explicit field encoders/validators

3. General compliant gamestate generation
   - blocked on the remaining `ECMAINT` linkage semantics, especially
     `3502` / `3558` / `355A`

4. Full Rust `ECMAINT` replacement
   - deferred until the first three milestones are substantially complete

1. `2000:5EE4` / integrity validator completion
   - Fully map the accepted and rejected structure rules in the early
     cross-file validator, especially the `PLAYER -> BASES -> FLEETS` path.
   - Treat the token question as closed; the remaining task here is the actual
     validation logic, not more `.TOK` experiments.
   - Save additional function names and branch notes in the live Ghidra project
     as the validator becomes clearer.
   - Current correction:
     - artifact: `artifacts/ghidra/ecmaint-live/5ee4-fleet-branch.txt`
     - script: `tools/ghidra_scripts_tmp/Report5EE4FleetBranch.java`
     - confirmed:
       - `2000:6040..6368` is the `FLEETS.DAT` validator branch, not the
         direct `BASES.DAT` loader
       - it opens stream `0x3178` with record size `0x36` and copies the
         active fleet record into local scratch at `[BP+0xFF3E]`
       - it emits kind-`1` summary entries through `0x2F72` / `0x2F76`
       - the early synthetic two-base integrity abort is therefore distinct
         from the later `Fleet assigned to an unknown starbase` behavior
   - Immediate next target:
     - recover the remaining kind-`1` / kind-`2` linkage semantics around
       `3502`, `3558`, and `355A`
     - goal: replace the current accepted single-base scenario encoder with a
       more general Rust-side base/fleet linkage validator instead of
       scenario-specific accepted values
   - Current kind-`1` follow-up:
     - artifact: `artifacts/ghidra/ecmaint-live/kind1-scratch-function.txt`
     - script: `tools/ghidra_scripts_tmp/ReportKind1ScratchFunction.java`
     - confirmed:
       - `0000:02ED..03D5` is the kind-`1` mirror of the generic summary
         loader, using scratch block `0x3502`
       - it consumes:
         - `350D`, `350F..3513`
         - `350E`, `3515..3519`
         - `3522`, `3523`
         - `351B..351F`
         - capped byte `3524`
         - selector/count byte `350C`
       - in the initial kind-`1` load path, the only explicit summary input
         passed to the `3502` loader is `ES:[DI+0x06]`
       - summary bytes `+0x01` / `+0x02` emitted by the fleet branch are not
         read there; they are overwritten later by the shared canonicalization
         stage
       - `5EE4` fleet-scratch offsets now correlate cleanly to the known fleet
         layout:
         - `[BP+0xFF40]` -> `record[0x02]`
         - `[BP+0xFF41]` -> `record[0x03..0x04]`
         - `[BP+0xFF43]` -> `record[0x05..0x06]`
         - `[BP+0xFF49]` -> `record[0x0B]`
         - `[BP+0xFF4A]` -> `record[0x0C]`
       - the second sub-branch is therefore following the per-empire
         `next fleet` link, not selecting another starbase-side record
       - working hypothesis:
         - summary `+0x06` is carrying a fleet-chain identifier
         - `player[0x40]` in the first sub-branch is likely the empire's
           head-of-chain fleet ID, not a count
     - correction:
       - raw-import entry `2000:C067` is not yet a trustworthy semantic
         function start; it decodes as a fragment inside a larger helper region
       - treat `3502` field correlation, not `C067` naming, as the next
         productive task
     - milestone:
       - the kind-`2` path at `0000:03DF..06AE` actively scans the summary
         table for a matching active kind-`1` entry before it finalizes the
         current base-side summary
       - candidate summary requirements:
         - same summary `+0x00`
         - kind `+0x04 == 1`
         - active/status `+0x03 != 0`
         - and then either:
           - direct word match `candidate +0x0A == [0x3558]`
           - or same `+0x01`, `+0x02`, `+0x05` plus helper-decoded `+0x06`
             matching `[0x355A]` with decoded kind `4` and flag `0`
       - practical consequence:
         - the later `unknown starbase` behavior is now best modeled as a
           failed kind-`1` / kind-`2` summary pairing problem
         - Rust-side compliant gamestate generation will need fleet/base
           linkage values that survive this summary pairing, not just
           individually plausible records
     - new base-side emitter mapping:
       - artifact: `artifacts/ghidra/ecmaint-live/5ee4-base-branch.txt`
       - script: `tools/ghidra_scripts_tmp/Report5EE4BaseBranch.java`
       - confirmed:
         - `2000:63D3..6759` is the `BASES.DAT` validator / kind-`2` summary
           emitter
         - kind-`2` summary `+0x01` / `+0x02` come from base coords
           `0x0B` / `0x0C`
         - kind-`2` summary `+0x0A` comes from base `0x02..0x03`
         - kind-`2` summary `+0x06` comes from:
           - `player[0x44]` in the primary branch
           - base `0x07..0x08` in the follow-on linked-base branch
       - practical consequence:
         - the remaining unknown pairing keys around `3558/355A` are now
           narrowed to values derived from `player[0x44]` or base `0x07..0x08`,
           not the coordinate or flag fields already mapped
     - new matcher decode milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-matcher.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2Matcher.java`
       - confirmed:
         - the kind-`2` matcher at `0000:03DF..06AE` does not compare `3558`
           and `355A` as raw base fields
         - it first pushes base-side summary `+0x06`, then decodes it through
           helper `0x2000:c09a` into scratch rooted at `3558`
         - the direct accept path compares candidate kind-`1` summary `+0x0A`
           against decoded `[0x3558]`
         - the structural accept path decodes candidate kind-`1` summary `+0x06`
           through helper `0x2000:c067` and compares decoded word against
           `[0x355A]` with decoded kind `4` and flag `0`
       - practical consequence:
         - `3558` / `355A` are now best modeled as helper-decoded linkage keys
           derived from summary `+0x06`, not as raw persistent fields
     - correction after helper dump:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-decode-helpers.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2DecodeHelpers.java`
       - `0x2000:c067` and `0x2000:c09a` are still not trustworthy semantic
         function starts in the raw live import
       - they currently sit inside a dense helper island that includes
         arithmetic and counted-string helpers
     - caller-pattern milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-helper-callers.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2HelperCallers.java`
       - confirmed shared contract:
         - callers push source summary word `ES:[DI + 0x06]`
         - then push a destination far pointer
         - then call either `0x2000:c067` or `0x2000:c09a`
       - concrete high-value cases:
         - `0000:0307` decodes summary `+0x06` into `DS:3502`
         - `0000:03fe` decodes summary `+0x06` into `DS:3558`
         - `0000:0681` decodes candidate kind-`1` summary `+0x06` into a
           local buffer and then checks:
           - decoded `+0x1f` as kind byte
           - decoded `+0x23` as word matched against `[0x355A]`
           - decoded `+0x0a` as flag byte
     - decoded-field-layout milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-decoded-field-uses.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2DecodedFieldUses.java`
       - confirmed:
         - `3502` and `3558` are sibling decoded summary-`+0x06` structures
           with the same tuple-oriented layout
         - shared shape:
           - tag byte `+0x0b` with payload words `+0x0d/+0x0f/+0x11`
           - tag byte `+0x0c` with payload words `+0x13/+0x15/+0x17`
           - control/scalar group rooted at `+0x20`
         - the local structural-match buffer from `0000:0681` confirms:
           - decoded kind byte at local `+0x1f`
           - decoded word at local `+0x23`
           - decoded flag byte at local `+0x0a`

2. `IPBM.DAT` resolution
   - Practical status: structurally complete enough for Rust-side compliant
     file generation.
   - Confirmed:
     - record size `0x20`
     - `PLAYER[0x48]` is the record count gate
     - the raw record copies contiguously into `DS:3538`
     - the overlapping field map is documented in `RE_NOTES.md`
   - Remaining work is semantic naming of some tuple payloads, not file-layout
     discovery or integrity-critical structure.
     - script: `tools/ghidra_scripts_tmp/ReportIPBMTailTransition.java`
     - confirmed:
       - common writeback always updates summary offsets `+0x01`, `+0x02`,
         `+0x05`
       - kind `2` has an extra side path through helper `0x2000:c100`
       - kind `3` writes finalized tuples back into:
         - tuple A -> `3541`, `3543..3547`
         - tuple B -> `3542`, `3549..354d`
         - tuple C -> `354f..3553`
     - practical implication:
       - `3555..3557` are outside the main tuple A/B/C writeback and should be
         treated as a separate trailing control group
   - Current boundary result:
     - `353D` is only consumed by the second `5EE4` `IPBM` branch
     - `3555..3557` are only visible inside the kind-`3` path in `0000:02c0`
     - practical implication:
       - kind `3` has a primary normalized field group
         (`3541`, `3543..3547`, `3542`, `3549..354d`, `354f..3553`)
       - plus a trailing group (`3555..3557`)
       - while `353B/353D` likely belong to the second-branch follow-on result
         path rather than the generic trailing group
   - First live baseline capture:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/registers-6870.txt`
       - `artifacts/ecmaint-ipbm-debug/scratch-3538-6870.txt`
     - setup:
       - valid one-record baseline with `PLAYER[0x48] = 1`
       - zero-filled `IPBM.DAT` of length `0x20`
       - breakpoint at live `2814:6870`
     - key observed normalized values:
       - `353D = 0x0001`
       - `3543 = 0x0080`
       - `3549 = 0x0080`
       - observed `3541`, `3542`, and `354F..3557` bytes/words are zero
     - practical implication:
       - this is the baseline shape to compare against future mutated `IPBM`
         records
   - First mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x00] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0001`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM` offset `0x00` definitely feeds tuple C / the summary-`+0x0A`
         word path
       - it also suppresses the zero-record default normalization that
         previously produced `353D = 1` and the paired `0x0080` defaults
   - Second mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x01] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0100`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM[0x00..0x01]` maps directly into `3538` as a little-endian
         word
       - tuple C / early summary `+0x0A` is therefore confirmed to derive from
         the first `u16` in the raw record
   - Expanded prefix mapping:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_02_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_03_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_04_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_05_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_06_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_07_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_09_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_0a_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x02]` copies to scratch `353A`
       - `IPBM[0x03]` copies to scratch `353B`
       - `IPBM[0x04]` copies to scratch `353C`
       - `IPBM[0x05]` copies to scratch `353D`
       - `IPBM[0x06]` copies to scratch `353E`
       - `IPBM[0x07]` copies to scratch `353F`
       - `IPBM[0x09]` copies to scratch `3541`
       - `IPBM[0x0A]` copies to scratch `3542`
     - semantic read with `2000:5EE4`:
       - `353A` is the player / empire byte copied to summary `+0x00`
       - `353B..353C` is the non-aligned `u16` that gates the second `IPBM`
         branch
       - `353D..353E` is the non-aligned `u16` later written to summary `+0x06`
       - `3541` and `3542` are the kind-`3` tag bytes written to summary
         `+0x01` and `+0x02`
     - practical implication:
       - the front of the raw `IPBM` record copies contiguously into scratch,
         then `ECMAINT` interprets overlapping byte/word fields over that copy
       - baseline all-zero defaults like `353D = 1` and `3543 = 3549 = 0x0080`
         are derived normalization, not the raw on-disk values
   - Group-start confirmation:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_0b_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_11_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_17_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_1d_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x0B]` copies to scratch `3543` (tuple-A payload block start)
       - `IPBM[0x11]` copies to scratch `3549` (tuple-B payload block start)
       - `IPBM[0x17]` copies to scratch `354F` (tuple-C payload block start)
       - `IPBM[0x1D]` copies to scratch `3555` (trailing control group start)
     - static follow-up from `0000:0723..0797`:
       - `3555` and `3556` are read as scalar bytes and expanded through
         helper `0x3000:486B`
       - `3557` is clamped to at most `1`, so raw `0x1F` behaves like a
         boolean / capped mode byte
     - dynamic clarification:
       - `artifacts/ecmaint-ipbm-debug/off_1e_val_01-scratch.txt` confirms
         `IPBM[0x1E]` copies to `3556`
       - `artifacts/ecmaint-ipbm-debug/off_1f_val_02-scratch.txt` shows the
         first `5EE4` stop still sees raw `3557 = 0x02`
       - the clamp to `1` therefore happens later in shared summary logic,
         not during the initial record-to-scratch copy
     - practical implication:
       - the coarse full-record layout is now stable enough for Rust-side
         binary encoding even though several gameplay semantics remain unnamed

3. Build queue / stardock encoding
   - Continue the partially solved `PLANETS.DAT[0x38]` / `0x4C` work.
   - Determine how completed production is stored in the stardock, including
     multiple ship types/counts and the exact handoff to later commissioning.
   - Goal: enough format/behavior detail to reproduce build completion without
     depending on the original executable.

4. Maintenance phase ordering around `Move.Tok`
   - Now that `Move.Tok` is understood as the crash-recovery marker, map the
     broader maintenance phase boundaries around it.
   - Determine what runs before movement, what runs after movement, when
     backups are written, and when token files are created/deleted.
   - Goal: recover the high-level phase order for faithful reimplementation of
     the maintenance engine.

Suggested execution order:
- First finish `2000:5EE4` branch mapping with emphasis on `BASES.DAT` and
  `IPBM.DAT`.
- Then move to build queue / stardock mechanics.
- Finally map movement-phase ordering and backup/token lifecycle around
  `Move.Tok`.
