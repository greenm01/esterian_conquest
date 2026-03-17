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
