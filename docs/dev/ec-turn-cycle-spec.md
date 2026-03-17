# ECMAINT Canonical Turn Cycle

This document records the current best recovery of the original
`ECMAINT.EXE` turn cycle.

The goal is to specify the oracle's phase order, not Rust policy. Where the
original ordering is not yet fully decoded, this document marks the gap
explicitly instead of filling it with guessed semantics.

## Confidence Levels

- `High`: directly supported by static RE, black-box probes, or both
- `Medium`: strongly suggested by multiple clues, but still missing one key
  link in code or fixtures
- `Low`: useful working hypothesis only; do not promote into Rust as settled

## High-Level Result

Current best model:

1. maintenance schedule / token wait gate
2. `Move.Tok` crash recovery and `.SAV` restore, if needed
3. cross-file integrity validation and file-count/link checks
4. yearly maintenance simulation over the loaded game state
5. late summary canonicalization / sorting
6. report emission across an internal `1..52` weekly timeline
7. final report flush / cleanup / token cleanup

The major unresolved area is the internal ordering inside step `4`: we now know
that movement, hostile contact, delayed mission resolution, and later weekly
report stamping all exist, but economy / production / command-side transforms
are not yet fully placed relative to one another.

## Practical Rust Consequences

Current practical guidance for `rust-maint`:

- keep cross-file validation as a distinct early phase, not mixed into yearly
  state mutation
- keep summary/event generation as a real intermediate boundary inside the
  engine; the original binary clearly builds outcomes first and only later
  canonicalizes, coalesces, and emits report text from them
- do not assume every writer to the `0x2f72 / 0x2f76` workspace belongs to the
  final late-report event pool:
  - `5ee4` appends `0x0c` records there during validation
  - but `6ac3` zeroes `0x2f76` before `5ee4` returns
  - current best practical reading is that `5ee4` uses the shared workspace as
    temporary validation scratch, not as the final persistent report-event set
- do treat the durable report-event pool as a later producer phase:
  - first confirmed non-`5ee4` durable writers are `1000:dddb` and `1000:e31b`
  - they allocate fresh `0x0c` records after `5ee4` has already cleared the
    scratch count
  - they write the later-consumed kind bytes directly:
    - `1000:dddb` -> kind `1`
    - `1000:e31b` -> kind `2`
- do not shape Rust gameplay order around the already-recovered late helpers:
  - `9e1e` is startup summary-workspace plumbing
  - `6d9b` is restore/integrity wrapper logic
  - `5ee4` is staged validation plus known fleet/base/IPBM summary emission
  - `8652 -> 1da6 / 0c06 / 2db3 / 56be` is late output/database/report side
  - `87f4 -> 8b15` is late summary coalescing/report prep
- this means the remaining Rust turn-order risk is concentrated in the still
  unresolved earlier simulation helpers, not in the late report pipeline

## Evidence Backbone

This spec is built from four independent evidence sources:

- token and restore-path RE in
  [token-investigation.md](/home/mag/dev/esterian_conquest/docs/dev/token-investigation.md)
- startup integrity/load RE in [RE_NOTES.md](/home/mag/dev/esterian_conquest/RE_NOTES.md)
- timing/log analysis in
  [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
- late summary/report pipeline RE in
  `artifacts/ghidra/ecmaint-live/summary-post-canonical.txt` and
  `artifacts/ghidra/ecmaint-live/late-report-pipeline.txt`

## Canonical Phase Order

### 1. Schedule Gate And Token Wait

Confidence: `High`

Settled facts:

- `ECMAINT` has a real schedule/date gate
- the clearest current static anchor is the string cluster at `2000:6fc6`:
  `Today is ... maintenance is not scheduled to run.`
- before doing real work, the engine checks the token-file set
- the master token loop at `2000:997C` walks:
  - `Planets.Tok`
  - `Fleets.Tok`
  - `Player.Tok`
  - `IPBMs.Tok`
  - `Conquest.Tok`
  - `Message.Tok`
  - `Results.Tok`
  - `Database.Tok`
- this is an active wait/cleanup gate, not a passive existence check

Practical meaning:

- maintenance begins with node-coordination and schedule eligibility, before
  simulation or validation

### 2. Crash Recovery From `Move.Tok`

Confidence: `High`

Settled facts:

- `Move.Tok` is the crash marker for a previous run that halted during the
  movement phase
- after the token wait logic, `ECMAINT` checks for `Move.Tok`
- if present, it prints the recovery messages at the block around `2000:6DAA`
- it restores `.SAV` backups over the working `.DAT` files before continuing
- this restore happens before the main integrity check

Practical meaning:

- the engine treats movement as a distinct critical phase
- backup/rollback exists specifically to recover from interruption during that
  phase
- `Move.Tok` is evidence that movement is not just one tiny helper call inside
  a flat pass; it is a named crash boundary in the real engine

### 3. Cross-File Integrity Validation And Input Loading

Confidence: `High`

Settled facts:

- after recovery, `ECMAINT` runs a broad cross-file integrity validator
- helper `0x25EE4` already has recovered passes over:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `IPBM.DAT`
- the validator checks linkage/count relationships, not just file lengths
- recovered examples:
  - `PLAYER[0x44]` links into `BASES.DAT`
  - `PLAYER[0x48]` is the `IPBM.DAT` record count
- the nearby startup/status string cluster at `2000:841b..855a`
  (`main.tok`, `Performing integrity check of game files...`,
  `Creating main work file...`, `Merging joint fleets and setting required
  speeds...`) currently has no direct scalar xrefs in the live dump
  - current best interpretation is that this outer startup/status path is
    reached indirectly, likely through a table/pointer-driven emitter

Practical meaning:

- the engine refuses to simulate from structurally inconsistent state
- canonical turn processing starts only after this validation/load stage

Additional static tightening:

- startup helper `2000:9e1e` initializes the shared summary workspace before
  the later phases run:
  - stores a startup time tuple at `0x34fa/0x34fc`
  - zeroes summary count `0x2f76`
  - allocates `0xfa00` bytes through `2000:9b13`
  - stores the resulting far pointer at `0x2f72/0x2f74`

Practical meaning:

- the summary/event table later used by canonicalization and weekly report
  emission is not lazily invented at the end; it is seeded up front in the
  startup/token-side path
- that still does not place the missing gameplay-core ordering, but it narrows
  the boundary between startup plumbing and later summary/report processing
- helper `2000:5ee4` now has a firmer internal shape:
  - zeroes `0x16ae`, `0x1714`, and `0x190a`
  - loads `0x3278` records of size `0x6e` into the far-pointer table rooted at
    `0x16ac`, with count byte `0x16ae`
  - loads `0x2f78` records of size `0x61` into the far-pointer table rooted at
    `0x1712`, with count byte `0x1714`
  - then runs the already-recovered summary emitters over:
    - `0x3178` fleet records
    - `0x2ff8` base records
    - `0x31f8` IPBM records
  - finally frees the staged `0x3278` / `0x2f78` buffers before returning

Practical meaning:

- current best reading is that `0x3278` is the player-side staging collection
  and `0x2f78` is the planet-side staging collection
- within the currently recovered `5ee4` body, those collections act as inputs
  to the fleet/base/IPBM validation and summary-emission paths; they are not
  yet supported as separate direct summary producers
- wrapper `2000:6d9b` is now better bounded as restore/validation scaffolding:
  - `arg = 0` jumps into `0x6f20`, calls `5ee4`, and on failure emits
    recovery/error text before recursively re-entering `6d9b` with `arg = 1`
  - `arg = 1` brackets `5ee4` with two `0x3000:4f4c` registration waves over
    the stream anchors rooted at `0x2f78`, `0x2ff8`, `0x3078`, `0x30f8`,
    `0x3178`, `0x31f8`, `0x3278`, `0x32f8`, and `0x3478`
- this strengthens the reading that `6d9b` is integrity/restore framing around
  `5ee4`, not another hidden yearly gameplay-core stage
- `5ee4`'s writes into `0x2f72 / 0x2f76` now look transient rather than final:
  - fleet/base/IPBM branches inside `5ee4` do increment `0x2f76` and allocate
    `0x0c` entry records in the shared workspace
  - but tail `0x6ac3` immediately zeroes `0x2f76` before `5ee4` returns
  - practical reading: those validation-time entries are temporary scratch for
    integrity/link checks, not the durable late-report summary set consumed by
    later canonicalization/coalescing passes
- the first confirmed durable event emitters now sit later in segment `1000`:
  - `1000:dddb` / probe point `1000:e09d` appends kind-`1` `0x0c` entries
  - `1000:e31b` / probe point `1000:e569` appends kind-`2` `0x0c` entries
  - both write owner/coords/common key words into the exact pool later read by
    the `87f4 -> 8b15` matcher/coalescer
- this further narrows the unresolved gameplay-core search away from the
  already-recovered `5ee4` tail exits

### 4. Yearly Simulation Core

Confidence: `Medium`

This is the most important remaining unresolved block.

Settled facts:

- this stage exists after validation and before late report emission
- it includes a real movement phase
- it includes hostile contact / battle / mission resolution behavior that can
  alter standing orders and fleet destinations
- it is not a single end-of-year text formatter, because later reports reflect
  different intra-year weeks for different outcomes

What is currently settled inside this core:

#### 4a. Movement is an explicit engine phase

Confidence: `High`

- the crash marker is literally `Move.Tok`
- the restore message says the previous maintenance halted during the movement
  phase
- this is the strongest recovered named phase boundary in the binary so far

#### 4b. Arrival and assault resolution are not always the same yearly step

Confidence: `High`

Black-box proof:

- controlled bombardment fixtures showed:
  - year 1: fleet moves into bombard position
  - year 2: bombard order is consumed and losses are applied
- this means "reach target orbit" and "execute bombardment" can be separate
  yearly maintenance steps

Practical meaning:

- the simulation core includes at least:
  - movement/arrival
  - later mission execution for fleets already in position

#### 4c. A real intra-year weekly scheduler participates in outcomes

Confidence: `High`

Black-box proof:

- shipped logs show report stardates in `1..52`
- logs are ordered nondecreasing by `(year, week)`
- the same fleet can generate multiple reports in one year at different weeks
- example from `ec2.txt`:
  - `3rd Fleet` reports at `12/3002`
  - then again at `21/3002`

Practical meaning:

- the yearly simulation core is not modeled as one atomic instant
- the engine assigns event times inside the year

#### 4d. Hostile contact/combat and administrative summaries feed the same timing stream

Confidence: `Medium`

Evidence:

- same-week bundles are common in historical logs, especially:
  - sensor contact
  - identification
  - interception
- same-week ordering is stable in the corpus, not just co-timestamped noise
  - repeated ordered `sensor contact -> identification` pairs are common
  - longer `sensor contact -> identification -> interception` chains also recur
- adjacent report transitions are dominated by gap `0` and gap `1` weeks
  - this fits a shared ordered weekly event stream
- Fleet Command Center loss summaries are interleaved into the same weekly
  ordering rather than appended as a separate yearly summary
- targeted recurring transitions now show that administrative summaries and
  follow-on mission consequences share that same stream
  - `identified -> fleet-lost` same week: `4x`
  - `attacked -> fleet-lost` next week: `2x`
  - `fleet-lost -> join-retarget` same week: `2x`
  - `fleet-lost -> planet-bombarded` same week: `4x`
  - `intercepted -> planet-bombarded` next week: `3x`

Practical meaning:

- combat/contact/admin summaries are likely produced from one shared event
  stream inside the yearly simulation
- at least some post-combat administrative and retargeting consequences are
  emitted immediately or on the very next weekly tick, not in a detached
  end-of-year appendix

#### 4e. Internal ordering of economy, production, tax growth, and command application remains open

Confidence: `Low`

What is not yet settled:

- whether economic growth runs before or after movement/combat
- when production completion is applied relative to movement/combat
- when player-order sanitation/normalization happens relative to economic
  updates
- whether some command effects are expanded before movement and others after
- exactly how each mission family maps combat outcomes onto the weekly
  scheduler

What is now constrained:

- the weekly aftermath delay is not one universal rule across all missions
- current corpus evidence shows:
  - bombardment `attacked -> bombing-run` at gaps `0`, `5`, `6`, and `7`
  - invasion `attacked -> invaded` at gap `7`
  - colonization `attacked -> arrived-target` at gap `2`
  - guard/blockade `arrived-world -> intercepted` at gaps `1`, `6`, `27`, and
    `43`

Current rule:

- do not claim a full canonical middle-cycle order yet
- do not model combat aftermath in Rust as one uniform post-combat delay; it
  is increasingly clear that mission family matters

### 5. Late Summary Canonicalization And Sort

Confidence: `High`

Recovered structure from
`artifacts/ghidra/ecmaint-live/summary-post-canonical.txt`:

- `0000:1104..123E` performs generic post-processing over summary entries
- it seeds summary word `+0x08`
- it sorts/swaps the 12-byte summary records by that derived key
- `0000:123E..12FD` then emits generic report/header staging

Practical meaning:

- major simulation outcomes are first collapsed into a summary table
- only after that summary table is canonicalized and sorted does the later
  report/timing pipeline consume it
- the post-validate call chain recovered so far now supports that separation
  more strongly:
  - `1da6`, `0c06`, and especially `56be` are increasingly report/message
    oriented rather than core simulation routines
  - `2db3` still looks more like derived-output regeneration than combat or
    movement logic
  - the newly probed helper `33f7` inside `2db3` is tied to
    `Backing up intelligence database...`, which pushes `2db3` even further
    toward derived database handling rather than simulation-core work

### 6. Weekly Report Emission Loop

Confidence: `High`

Recovered structure from
`artifacts/ghidra/ecmaint-live/late-report-pipeline.txt`:

- `0000:127A..1361` runs an outer loop controlled by `[BP-0x2]`
- that loop starts at `1`
- it repeats until `0x34`
- `0x34` is decimal `52`
- inside each outer iteration, `0000:1302..1356` scans active summary entries
- each active entry runs through:
  - `0000:02C0`
  - `1000:a26e`
  - `2000:c057`
  - `1000:0b51`
  - `2000:c057`
- after the `52` boundary, the loop flushes through `0x3000:32e0`

Practical meaning:

- the binary contains a real late-stage `1..52` loop
- this is the first strong static confirmation that the weekly structure is in
  engine code, not only in report text
- current best interpretation:
  - the yearly simulation has already produced canonicalized summary entries
  - the later report pipeline then walks those summaries across a 52-step
    weekly presentation / emission schedule

Important caution:

- this does not yet prove that every gameplay mechanic is itself simulated
  week-by-week in the same loop
- what is proven is that the late report/timing stage explicitly iterates over
  `1..52`

### 6a. Fixed post-validate report/output tail

Confidence: `Medium`

New static anchor:

- a larger driver region around `2000:861d` now shows a fixed late call order
  after successful restore/validation:
  - `2000:1da6`
  - `2000:0c06`
  - `2000:2db3`
  - `2000:56be`
  - conditional `2000:7659` when `0x169a != 0`

What is already clear:

- this is a late-phase tail, not the whole gameplay simulation core
- `2000:56be` is strongly report-oriented:
  - it references mission-report text families including invasion,
    colonization, scouting, seek-home, and starbase/guard-blockade reports
- `2000:0c06` also looks report/output-oriented:
  - it references player-facing starbase/crew loss text
- `2000:2db3` is the strongest current `DATABASE.DAT` rebuild candidate:
  - it sizes work by `planet_count * 100`
  - that matches the already recovered `DATABASE.DAT` `100`-byte slot model

Practical meaning:

- after validation, `ECMAINT` enters a structured late-output tail before the
  explicit weekly `1..52` emission pass completes
- the still-missing "core simulation" is now more likely earlier than this
  `861d` tail, or partly hidden behind helpers that feed it

### 6b. Kind-`2` summary coalescing happens in the late weekly/report side

Confidence: `Medium`

New static tightening from `2000:87f4 -> 2000:8b15`:

- this region iterates over the summary pointer table at `0x2f72` / `0x2f76`
- it selects summary kind `2` entries and scans for matching kind `1` entries
- the structural match keys are currently:
  - owner byte `+0x00`
  - X byte `+0x01`
  - Y byte `+0x02`
  - flag/mode byte `+0x05`
- after that match, it decodes summary payload word `+0x06` through helper
  pair `2000:3f5a` / `2000:3f27`
- matched entries then feed more late text/output helpers rather than obvious
  movement/combat/economy code

Practical meaning:

- this is another late summary coalescing / report-prep stage, not the missing
  yearly gameplay simulation core
- the unresolved middle ordering is therefore even less likely to live in the
  `861d -> 8b3d` region and more likely earlier in the run or behind helpers
  that populate the summary table before this pass

### 7. Final Flush, Writes, And Cleanup

Confidence: `Medium`

Settled facts:

- after the weekly loop, the pipeline flushes report output
- token deletion / cleanup exists elsewhere in the run
- `Conquest.Tok` has explicit management code in the live image

What remains open:

- exact ordering of final file writes for `RESULTS.DAT`, `MESSAGES.DAT`,
  `DATABASE.DAT`, and ranking outputs
- exact cleanup order for all token files

## What The Oracle Already Proves About Turn Structure

These are the most important practical conclusions:

- `ECMAINT` does have a more sophisticated internal turn structure than "one
  instant per year"
- movement is a named crash-sensitive phase
- some missions resolve in delayed follow-up years after arrival
- reports are emitted on a real internal `1..52` weekly scale
- contact, interception, and command-center summaries participate in that same
  timing stream
- the engine performs a late summary sort/canonicalization pass before at least
  one major weekly report loop

## What Is Still Missing

To finish the canonical cycle, we still need:

- the exact middle ordering of:
  - economy
  - build completion
  - movement
  - hostile contact
  - orbital combat
  - bombard / invade / blitz
  - retreat / seek-home rewrites
- the direct code path that formats player-visible `Stardate: D/YYYY`
- the exact write/flush ordering for all output files

## Current Working Canonical Spec

This is the tightest safe statement today:

1. `ECMAINT` first performs schedule/token gating.
2. If `Move.Tok` exists, it restores `.SAV` backups before validation.
3. It validates and loads the linked `.DAT` state.
4. It runs the yearly gameplay simulation, including at least an explicit
   movement phase and later mission/combat consequences.
5. It canonicalizes and sorts summary entries from those outcomes.
6. It performs a late `1..52` weekly report/timing loop over the active
   summaries.
7. It flushes outputs and performs final cleanup.

That is the current oracle-backed canonical turn cycle.
