# ECMAINT Canonical Turn Cycle

This document records the current best recovery of the original
`ECMAINT.EXE` turn cycle.

The goal is to specify the oracle's phase order, not Rust policy. Where the
original ordering is not yet fully decoded, this document marks the gap
explicitly instead of filling it with guessed semantics.

For the implementation-facing companion that describes the same process as a
Rust engine/state-machine problem, use
[rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md).

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

The yearly simulation core (step `4`) is now substantially recovered:

- **movement is annual** — fleet positions are updated once per year
  (one-time advance), not per-week. Fractional travel state is stored in
  the fleet tuple_c field (Real48) for multi-year journeys.
- **the 52-week fleet loop is event scheduling, not physics** — it
  schedules encounter detection, combat resolution, and report emission
  from post-movement positions. Stardates come from timing codes, not
  physical arrival time.
- fleet visit order is **PRNG-shuffled** per game state (LCG confirmed
  as Borland Pascal `$08088405`; exact shuffle algorithm unknown)
- **mission resolution requires start-of-year position** — bombard,
  colonize, invade resolve only when the fleet is at its target at the
  start of the year
- combat reports are emitted **inline** during the weekly loop
- combat is triggered by the first co-located hostile fleet processed
- fleet destruction and capture are dynamic mid-pass
- a **pre-loop fleet setup phase** handles captures/reassignments before
  the 52-week loop (non-combat scenarios skip it entirely)
- economy/autopilot processing is **gated by `player[0] = 0xFF`** (rogue
  mode), runs **after** the fleet loop, and reads post-combat fleet state
- colonization is **atomic on arrival** (ownership, armies, name, status,
  production all set in one pass)
- **timing-window constants** are fully recovered: 8 codes with fixed
  week offsets (+2/+7/+21/+0/+0/+0/+0/+30), minimum week floors, and
  scheduling priorities. Kind-1 producer assigns codes 3-6 by fleet
  composition (starbase/BS/CA-TT-army/scout-DD).

Remaining unresolved areas: exact PRNG shuffle algorithm, timing codes 7
and 8 producer assignment, exact target-world aftermath predicates, and
production completion timing.

## Practical Rust Consequences

For implementation guidance, use the companion
[rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md).

Key points:

- movement is annual (pre-loop), not per-week
- the 52-week loop is event scheduling, not physics simulation
- economy/autopilot runs after the fleet loop, gated by `player[0]`
- the late tail (`8652 → 1da6 / 0c06 / 2db3 / 56be`) is output/report
  generation, not simulation — do not shape Rust gameplay order around it
- the durable event pool (`0x2f72 / 0x2f76`) has two layers:
  - transient validation scratch from `5ee4` (cleared before return)
  - durable entries from `1000:dddb` (kind-1) and `1000:e31b` (kind-2),
    later consumed by the canonicalizer and weekly report scheduler

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
- the first recovered ordering between those durable producers is now better
  bounded:
  - sibling drivers `1000:00e8` and `1000:024d` both call `1000:f71d` first
  - `1000:f71d` reaches the kind-`1` writer via `1000:f8a9 -> 1000:dddb`
  - only after that do those same drivers call `1000:e31b` for kind-`2`
    emission
  - practical reading: at least this durable event family is not built by one
    unordered bulk pass; producer order matters before the later matcher pairs
    kind `2` against kind `1`
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

#### 4a. Movement is an annual pre-loop phase

Confidence: `High`

- the crash marker is literally `Move.Tok`
- the restore message says the previous maintenance halted during the movement
  phase
- movement updates fleet positions once per year (not per-week)
- fractional travel state is stored in fleet tuple_c (Real48) for multi-year
  journeys
- this is the strongest recovered named phase boundary in the binary so far
- see also 4b (event scheduling model) and 4p (position-first evidence)

#### 4b. The 52-week fleet loop is event scheduling, not physics simulation

Confidence: `High`

Source: stardate analysis, tuple field analysis, and fixture diffs
(Phase C, 2026-03-17).

Key evidence:

- fleet-battle: fleet 8 (speed 3, 1 sector from (9,10) to (10,10))
  produces a contact report at stardate **50/3010**. If movement were
  per-week at speed 3 (arrival at week ~17), the contact would be at
  week ~19 (code 1 offset +2). Week 50 is inconsistent with per-week
  movement — the 52-week loop is scheduling report emission, not
  simulating fleet physics.

- fleet tuple_c field (`+0x19..+0x1E`) is a Borland Pascal Real48 that
  changes from `1.0` to `~0.9999` during movement tick and reverts to
  `1.0` when the mission resolves. Pre/post fixture comparison shows
  tuples are identical — they are **scratch state** used during yearly
  processing, not persistent fleet data.

- `Move.Tok` exists as a separate crash-recovery phase boundary, distinct
  from the 52-week loop.

- PLANETS.DAT is never accessed during the fleet loop — planet-side
  effects are annual, not per-week.

Structural conclusion:

- **movement is an annual position update**, not per-week incremental
  advance. Fleet positions are updated once per year (storing fractional
  travel state in tuple_c for multi-year journeys).
- **the 52-week loop processes encounter detection, combat resolution,
  and report scheduling** from post-movement positions.
- the weekly stardate assigned to each event comes from the timing-code
  system (codes 1-8 with their offsets and minimum-week floors), not
  from the physical arrival time.

Per-fleet-per-week inner body:

1. Read fleet record
2. Check if this fleet has events to emit this week (timing-window test)
3. If co-located hostile: resolve combat + emit RESULTS.DAT inline
4. Update weekly event state in fleet record
5. Write fleet record

This replaces the earlier "4b. Arrival and assault resolution" section
with a stronger structural model.

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

#### 4e. Producer pass internal ordering (partially recovered)

Confidence: `Medium`

What is now settled:

- sibling drivers `1000:00e8` and `1000:024d` belong to the yearly producer
  family, not the late report tail
- both call `1000:f71d` first
- `1000:f71d` reaches the durable kind-`1` writer through
  `1000:f8a9 -> 1000:dddb`
- only after that do those same drivers call `1000:e31b` for durable
  kind-`2` emission
- `1000:024d` continues into the partially recovered `1000:03ff..0d53`
  interior, while `1000:00e8` stops earlier
- practical reading:
  this is not one flat unordered event dump; at least one real producer family
  inside step `4` has internal ordering and sibling specialization

#### 4f. `1000:024d` is a mixed planet-state producer pass

Confidence: `Medium`

Settled structure:

- `1000:024d` starts with the already known producer/event front half:
  - `cce7`
  - `f71d`
  - `d5d2`
  - `b6d8`
  - optional `db04(arg=0x0a)` when current planet `+0x5a > 0`
  - `f2c7`
  - `e31b`
  - `e1c0`
  - `f9ff`
  - `f914`
  - `c025`
  - `c9a0`
  - `fe73`
- it then enters a deeper owned-planet loop at `1000:03ff`
- that interior:
  - iterates staged planets from `0x1712`
  - skips `planet[+0x5d] == 0`
  - advances `planet[+0x5c]` through a small state ladder
  - gates on owner-player state
  - scans durable kind-`2` entries
  - folds `entry[+0x22]` into running planet-side accumulators
  - writes results into `planet[+0x58/+0x5a]`
  - directly transforms planet numeric fields at
    `+0x03/+0x05/+0x07` and `+0x09/+0x0b/+0x0d`
  - branches on `planet[+0x60]`

Practical meaning:

- `024d` is not just a late summary helper
- it is a genuine step-`4` bridge between planet-side state mutation and
  durable event creation
- some of that work is silent:
  - direct oracle probes can change `PLANETS.DAT`, `DATABASE.DAT`, and
    `RANKINGS.TXT`
  - while leaving `RESULTS.DAT`, `MESSAGES.DAT`, and `ERRORS.TXT` empty

#### 4g. Deep `024d` planet mutation is gated by more than ownership

Confidence: `Medium`

Current strongest black-box constraints:

- forcing `planet[+0x60] = 1` on an owned world consistently activates the
  deeper same-world rewrite at `+0x03..+0x0e`
- forcing nearby visible fields without `+0x60` does not reproduce that path:
  - `+0x0e`
  - `+0x58`
  - `+0x5a`
  - simple combinations
- forcing `+0x60 = 1` on an unowned world does not reproduce the broad rewrite
- forcing `+0x5c = 0` or `1` together with `+0x60 = 1` still triggers the
  rewrite and then normalizes `+0x5c` back to `2`

Payload bisect result:

- ownership plus `+0x60 != 0` is still not sufficient by itself
- the branch also depends on a richer established-world payload in
  `planet[+0x03..+0x0d]`
- that prerequisite is not one opaque blob:
  - lower block `+0x03..+0x08` can drive the lower half of the rewrite
  - upper block keyed by `+0x09..+0x0d` can drive the upper half
  - byte `+0x09` alone is already enough to activate the upper-half rewrite
    shape at `+0x09..+0x0e`
  - copying `+0x03..+0x09` together reproduces the full broad rewrite

Practical meaning:

- the current best raw gate is:
  owned world plus `planet[+0x60] != 0` plus an established-world numeric
  payload
- for Rust modeling, treat this as evidence for at least two coupled
  planet-state numeric groups inside step `4`, not one undifferentiated world
  blob

#### 4h. First ordering signal: some `024d` planet mutation precedes visible delayed consequences

Confidence: `Medium`

Current direct timing evidence:

- in preserved invasion and bombardment pre-fixtures, forcing target-world
  `+0x60 = 1` causes the deep `+0x03..+0x0e` world rewrite to begin on tick
  `1`
- in those same forced probes, `RESULTS.DAT` is still empty on tick `1`
- in a fleet-battle probe, the same kind of world rewrite also lands on tick
  `1`, but `RESULTS.DAT` is non-empty that tick

Practical meaning:

- at least some `024d`-side planet mutation can occur earlier in step `4`
  than later visible mission/combat consequences, especially for delayed
  families like invasion and bombardment
- this is enough to reject the old "producer passes are only late aftermath"
  model
- it is not enough to claim the full canonical order among economy, movement,
  combat resolution, and these producer passes

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

#### 4i. The yearly simulation is a weekly fleet-processing loop

Confidence: `High`

Source: enriched file-I/O trace analysis across 6 scenarios (bombard, econ,
fleet-order, fleet-battle, invade, planet-build).

Settled structure:

- the fleet write block inside step `4` consists of **exactly 52 passes**
  through the fleet table in scenarios without fleet destruction (bombard,
  econ, fleet-order, planet-build all show `832 writes / 16 records = 52`)
- each pass reads-then-writes every active fleet record once
- 52 passes = 52 simulated weeks
- the simulation phase always starts at event index `507`, consistent across
  all 6 scenarios tested

Fleet visit order is **data-dependent, not sequential**:

- bombard:     `[11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12]`
- econ:        `[11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10]`
- fleet-order: `[6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8]`
- planet-build: `[15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5]`
- fleet-battle: `[1,14,7,4,9,8,12,15,0,13,10,5,11,6]` (14 records)
- invade:       not a fixed sequence (variable due to destruction mid-pass)

Fleet visit order is stable within a scenario: all 52 passes visit the
same records in the same order (confirmed by pass-order consistency check
in bombard, econ, fleet-order, and planet-build).

Fleet destruction reduces record count and pass count:

- fleet-battle: 15 records × 49 passes = 735 writes (one fleet absent)
- invade: 15 records × ~45.7 passes = 685 writes (non-integer ratio
  implies fleet destruction mid-pass, reducing write count for later
  passes)

#### 4j. Combat reports are emitted inside the weekly fleet loop, not after

Confidence: `High`

Source: fleet-battle file-I/O trace interleave analysis.

Key finding:

- in the fleet-battle scenario, **RESULTS.DAT writes are interleaved inside
  fleet write pass 7** (events 640-677)
- 11 RESULTS.DAT records (84 bytes each) are written in two bursts:
  - first burst: 5 records (events 648-652)
  - second burst: 6 records (events 663-676), with a seek-read-rewrite
    pattern suggesting report insertion/reordering
- this happens between fleet record writes within the same weekly pass

Practical meaning:

- report generation is not deferred to a separate post-simulation phase
- the weekly fleet-processing loop itself generates combat reports inline
- the Rust engine should allow report emission during fleet processing, not
  only after all fleet passes complete
- the RESULTS.DAT write at pass 7 correlates with the Stardate `1/3` seen
  in the report text (7th weekly pass maps to week 3 via some scheduling
  offset or initialization)

#### 4k. Fleet-battle has a pre-loop fleet setup phase before the 52-week loop

Confidence: `High`

Source: fleet-battle trace pass-by-pass record indices vs non-combat traces.

Non-combat scenarios (bombard, econ, fleet-order, planet-build) all show
exactly **52 passes** with all 16 fleet records from pass 1. Fleet-battle
shows **57 passes**: 5 pre-loop passes with varying fleet subsets, then 52
passes with a stable 14-fleet set.

Pre-loop passes (fleet-battle only):

- pass 1: `[1]` (1 record)
- pass 2: `[1,3]` (2 records)
- pass 3: `[1]` (1 record)
- pass 4: `[1,0]` (2 records)
- pass 5: `[1,11,6]` (3 records)
- pass 6-57: stable 14-record set `[1,14,7,4,9,8,12,15,0,13,10,5,11,6]`

The final pass (57) shows 12 records — fleets 11 and 6 were dropped
(destroyed during combat in pass 7, which is weekly pass 2).

Fleet 2 (empire 1's bombard fleet, speed 3) and fleet 3 (captured in
pre-loop) are absent from the stable 52-week set.

Practical meaning:

- passes 6-57 = exactly 52 weekly passes = the main simulation loop
- passes 1-5 are a **pre-loop fleet setup phase** that handles fleet
  captures, reassignments, and initial fleet-set construction
- this phase only produces fleet writes when there are fleets to
  capture/reassign (non-combat scenarios skip it entirely)
- the "incremental activation" previously attributed to movement arrival
  is better understood as a capture/setup phase that runs before the weekly
  loop, not as gradual fleet awakening during the loop itself

#### 4l. Fleet visit order is PRNG-shuffled, seeded from accumulated game-state processing

Confidence: `High`

Source: cross-fixture comparison plus PRNG reverse-engineering
(Phase C/D, 2026-03-17).

Four scenarios with identical CONQUEST.DAT, SETUP.DAT, and nearly identical
FLEETS.DAT (only fleet 2's ship counts differ) but different PLANETS.DAT
produce completely different visit orders:

- bombard:     `[11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12]`
- econ:        `[11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10]`
- fleet-order: `[6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8]`
- planet-build:`[15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5]`

The linked list traversal (`next_fleet_link`) does NOT match any of these
orderings. Fleet data is identical across all 4 fixtures. No simple
fleet-field sort produces any of the orderings.

The only varying file is PLANETS.DAT (planet 13's name and numeric fields
differ). Small planet data changes — even just 2 bytes — cascade into
completely different fleet orderings, characteristic of PRNG sensitivity.

PRNG identification (from binary analysis of the memdump):

- the LCG is confirmed as standard Borland Pascal:
  `RandSeed = RandSeed * $08088405 + 1`
- the Random function was located via its shift-and-add multiply pattern
  in the memdump (the 32-bit multiply by `$08088405` is decomposed into
  16-bit partial products with shifts)
- `RandSeed` is stored at `DS:0x03A6` (32-bit, low word at `0x03A6`,
  high word at `0x03A8`)
- the `Randomize` function (seeds from DOS `INT 21h/2Ch` Get Time) sits
  immediately after the Random function body

Runtime values captured via DOSBox-X debugger (bombard scenario):

- `DS = 0x3529` at runtime
- `RandSeed = 0x000E000E` at the post-load bridge point (`96c4`),
  before the validation/simulation phases have run

Shuffle algorithm investigation:

Exhaustive black-box search ruled out all standard approaches across
the full 2^32 seed space:

- Fisher-Yates reverse shuffle (for i=N-1 downto 1): no seed in 0..50M
  matches for any Random extraction variant (multiplicative, shift, TP7)
- Fisher-Yates forward shuffle (for i=0 to N-2): same result
- Simple swap shuffle (for i=0 to N-1): same result
- Sort-by-Random-key (assign key[i]=Random() for each fleet, sort by key):
  no match in full 2^32 seed space (4.3 billion seeds tested)
- Sort-by-Random(10000)-key: no match in 10M seeds
- TP7 16-bit Random(Range) extraction `((seed >> 16) * Range) >> 16`:
  full seed_1 range searched for both Fisher-Yates variants, no match

The conclusion is that the seed at shuffle time is the **accumulated
RandSeed after all Random() calls during validation and loading** (step 3).
Processing different planet data takes different branch paths during
validation, each advancing the PRNG state differently. By the time the
shuffle runs, the seed is completely different for each fixture.

This means the shuffle algorithm cannot be cracked from black-box data
alone — it requires either:

1. a DOSBox breakpoint at `DS:0x03A6` write to capture the pre-shuffle
   seed value, or
2. Ghidra RE of the shuffle call site inside the fleet loop setup

With the actual seed in hand, the algorithm (likely standard Fisher-Yates)
should be identifiable from a single test.

Practical meaning:

- for Rust oracle parity, exact PRNG replication requires knowing both
  the shuffle algorithm and the full PRNG call chain from program start
  through validation to the shuffle point
- for practical Rust implementation, use a deterministic visit order
  (e.g., slot order or a seeded shuffle from a Rust-native PRNG) and
  accept that visit order will differ from the original
- the visit order affects combat timing within weekly passes: when two
  hostile fleets are co-located, the first-visited fleet triggers combat
  resolution and report emission

#### 4m. Combat resolution is triggered by the first co-located hostile fleet processed

Confidence: `High`

Source: fleet-battle pass 7 inner event analysis.

When fleet 4 (empire 2, PatrolSector, 100 BS at (10,10)) is processed in
weekly pass 7:

1. read fleet 4
2. read fleet 0 (empire 1, co-located hostile, 50 BS/CA/DD at (10,10))
3. open RESULTS.DAT
4. write 5 report records (84 bytes each)
5. close RESULTS.DAT
6. read fleet 0 again
7. open RESULTS.DAT, read 1 record back, write 7 more records
8. close RESULTS.DAT
9. write fleet 4

The engine reads the opposing fleet's state during the processing fleet's
iteration, resolves combat, emits reports inline, then writes the
processing fleet's updated state. Fleet 0's state update happens later in
the same pass when it reaches its own position in the visit order.

#### 4n. Fleet slots can be reassigned between empires during the simulation

Confidence: `High`

Source: fleet-battle preserved pre/post fixture comparison.

Observed fleet ownership changes:

- fleet 2: empire 1 → empire 2 (reassigned)
- fleet 3: empire 1 → empire 2 (reassigned)
- fleet 8: empire 3 → empire 4 (reassigned)

Fleet 2 (empire 1's bombard fleet) is absent from the entire weekly visit
set (only 14 of 16 fleet slots participate in the 52-pass loop). This
suggests fleet slots that are being reassigned/captured are excluded from
the weekly processing loop, or they are processed through a separate path.

#### 4o. File write ordering is stable across scenarios

Confidence: `High`

Source: cross-scenario first-write ordering comparison.

All 6 scenarios follow the same first-write file ordering:

```
FLEETS.DAT -> [RESULTS.DAT in combat scenarios] -> DATABASE.DAT ->
PLAYER.DAT -> PLANETS.DAT -> CONQUEST.DAT -> RANKINGS.TXT
```

RESULTS.DAT appears only in fleet-battle (the only single-tick scenario
with active combat at the start). The remaining files always appear in
the same order in the flush phase.

DATABASE.DAT writes correlate with specific planet record indices:

- bombard/econ: planets `[44, 65, 32, 33, 14, 13]`
- fleet-battle: planets `[44, 65, 32, 33, 14]`
- fleet-order: planets `[44, 65, 32, 13, 14]`
- invade: planets `[44, 65, 32, 33, 14]`
- planet-build: planets `[44, 65, 32, 14]`

Planets 44, 65, and 32 appear in every scenario — they are likely
homeworld or structurally significant planets whose database entries are
always refreshed.

#### 4p. Movement is position-first, mission-resolution-next-year

Confidence: `High`

Source: black-box ordering probes across econ, bombard, and colonization
fixtures (Phase C probes, 2026-03-17).

A speed-3 fleet traveling 1 sector updates its position in tick N but does
not resolve its mission (bombardment, colonization) until tick N+1:

- econ fixture: fleet 2 (bombard, speed 3) at (16,13) targeting (15,13)
  - tick 1: location_x changes 16→15, but speed and order remain set
  - tick 2: speed clears to 0, order clears, planet damage applied
- colonization probe: fleet 2 (colonize, speed 3) at (16,13) targeting (18,15)
  - tick 1: location moves to (17,14) — one sector of two
  - tick 2: location reaches (18,15), order clears, planet colonized

Contrast with co-located fleets (fleet-battle fixture): fleets already at
the same sector at the start of a tick resolve combat within that tick.

Practical implication: the 52-week loop processes movement and resolves
missions within the same yearly pass, but a fleet must already be at its
target at the start of the year for its mission to resolve that year.
Position updates within the 52-week loop are visible at end-of-year but
mission resolution uses start-of-year position.

#### 4q. Colonization is atomic on arrival

Confidence: `High`

Source: ETAC colonize probe on econ fixture (Phase C, 2026-03-17).

When a colonize fleet arrives at an unowned planet, all colonization
effects happen in the same tick:

- planet ownership status: 0→2 (homeworld-style)
- planet owner empire: 0→colonizer empire
- planet army count: 0→1
- planet name: updated to "Not Named Yet"
- planet potential_prod_hi: set to 0x81 (129)
- planet stardock/build fields: unchanged

Economy starts on the newly colonized planet in the following tick
(factories_raw initialized on tick 3).

#### 4r. Economy/autopilot processing gated by player mode byte

Confidence: `High`

Source: direct PLAYER.DAT byte-0 mutation probes (Phase C, 2026-03-17).

Planet economy changes (army growth, battery growth, econ_marker updates,
factories_raw adjustments) only occur for empires whose PLAYER.DAT
`byte[0]` is `0xFF` (rogue mode). Empires in civil disorder (`byte[0] =
0x00`) are economically frozen: their owned planets show no growth across
ECMAINT ticks.

Verified by direct mutation: patching only `player[0]` from `0x00` to
`0xFF` in the econ fixture causes planet 14 (empire 1 homeworld) to show
armies 10→27, econ_marker 12→4, and factories exponent adjustment on
tick 1 — matching the natural behavior in fleet-battle/invade fixtures
where player 1 was already `0xFF`.

The `autopilot_flag` at `player[0x6D]` is the companion that drives
army/battery building within the rogue pass, per existing Rust RE.

#### 4s. Economy/autopilot runs after the fleet loop and reads combat outcomes

Confidence: `High`

Source: controlled fleet-battle comparison with and without combat
(Phase C, 2026-03-17).

Comparing the same fleet-battle fixture with and without hostile fleets at
(10,10):

- **with combat**: planet 14 (empire 1 homeworld at 16,13) → armies 10→27,
  econ_marker 12→4
- **without combat** (fleet 4 and fleet 8 moved away): planet 14 → armies
  unchanged (10), econ_marker 12→2, stardock fields change instead

The economy/autopilot pass reads post-combat fleet state to determine
its behavior. With fleet losses at (10,10), the rogue AI builds armies on
the homeworld. Without losses, it does not.

Combined with the file-I/O evidence (PLANETS.DAT is never accessed during
the 52-pass fleet loop), this confirms:

- economy/autopilot runs **after** the fleet loop, not before or during
- it reads post-combat game state and adjusts planet production accordingly
- the econ_marker value depends on the full post-fleet-loop game state, not
  just the planet's own pre-existing economy

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
- the late timing side is now better split into internal layers:
  - `0000:02c0` decodes kind-`1` summary entries through `2000:c067` into
    stack-resident local timing state
  - `1000:9fa1 / 1000:a26e` derives timing windows from a local `0x0a`-byte
    code table using fixed offsets like `+2`, `+7`, `+0x15`, and `+0x1e`
  - `1000:c102 / 1000:9c0e` then score/test the current weekly slot against
    those windows and raise a rejection flag when the candidate falls outside
    the acceptable range

#### 6-table. Recovered timing-window constants

Confidence: `High`

Source: full disassembly of the `1000:a26e` switch-case (2026-03-17).

The scheduler reads a code byte from each `0x0a`-byte timing entry and
applies these fixed parameters:

| Code | Week Offset | Min Week | Priority | Corpus Match |
| ---: | ---: | ---: | ---: | :--- |
| 1 | +2 | ≥10 | 6 (low) | sensor contact |
| 2 | +7 | ≥15 | 5 | fleet identification |
| 3 | +21 | ≥20 | 4 | interception / engagement |
| 4 | +0 | ≥0 | 6 (low) | immediate, low priority |
| 5 | +0 | ≥0 | 5 | immediate, medium priority |
| 6 | +0 | ≥0 | 5 | immediate, medium priority |
| 7 | +0 | ≥0 | 3 (high) | immediate, high priority |
| 8 | +30 | ≥25 | 1 (highest) | late resolution (bombard/invade) |

Key relationships:

- contact → identification: +7 − +2 = **5-week gap**
- identification → interception: +21 − +7 = **14-week gap**
- these match the dominant corpus transition gaps of ~5 and ~14 weeks
- codes 4–7 are all immediate (offset +0) with varying priority for
  scheduling conflicts
- code 8 is the latest placement (+30 weeks from base, earliest week 25)
  with highest scheduling priority — likely bombardment/invasion resolution

The `min_week` floor prevents early events from being scheduled before
their minimum week. The priority value resolves conflicts when multiple
events compete for the same weekly slot (lower number = higher priority).

Important caution:

- this does not yet prove that every gameplay mechanic is itself simulated
  week-by-week in the same loop
- what is proven is that the late report/timing stage explicitly iterates over
  `1..52`
Code-to-source mapping (recovered from kind-1 producer `dddb`):

The kind-1 producer at `1000:dddb` assigns entry `[+0x09]` (the timing
code) based on fleet composition at the source record:

| Condition | Code | Meaning |
| :--- | ---: | :--- |
| `source[+0x30] > 0` (starbase present) | 3 | starbase fleet (+21 weeks) |
| `source[+0x26] > 0` (battleships) | 4 | battleship fleet (immediate) |
| `source[+0x28] > 0` or `[+0x2c] > 0` or `[+0x2e] > 0` | 5 | cruiser/transport/army fleet (immediate) |
| else (scouts/destroyers only) | 6 | light fleet (immediate) |

Codes 1 and 2 (sensor contact +2, identification +7) are not assigned by
the `dddb` producer. They are likely generated by the `02c0` decoder
during the weekly emission loop — one kind-1 summary entry may produce
multiple report events at different weeks (contact first, identification
later).

Codes 7 and 8 (immediate high-priority and +30 week late resolution)
are not assigned by `dddb` either. Code 8 likely comes from the kind-2
producer family or a separate late-resolution producer.

The kind-2 producer at `1000:e31b` writes `entry[+0x04] = 2` but does not
explicitly write `entry[+0x09]` in the visible code path — the timing
code for kind-2 entries may be filled by a downstream helper or defaulted

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

## What Is Still Missing

To complete the canonical cycle to full oracle parity, we still need:

- the exact PRNG shuffle algorithm for fleet visit order (LCG confirmed as
  Borland Pascal `$08088405`, but the shuffle variant and seed-at-shuffle-time
  are unknown — full 2^32 search ruled out all standard Fisher-Yates and
  sort-by-key variants)
- the producer assignment for timing codes 7 and 8 (codes 3-6 are mapped from
  `dddb` by fleet composition; 1,2 come from the `02c0` decoder)
- exact target-world-state predicates that choose one aftermath shape over
  another
- production completion timing relative to other subphases

None of these block Rust implementation. They affect oracle parity and report
fidelity, not engine structure.

## Current Working Canonical Spec

This is the tightest oracle-backed statement today:

1. `ECMAINT` first performs schedule/token gating.
2. If `Move.Tok` exists, it restores `.SAV` backups before validation.
3. It validates and loads the linked `.DAT` state.
4. It runs the yearly simulation core:
   a. Prepare transient workspaces.
   b. Annual movement update (one-time position advance for all fleets;
      fractional travel state stored in fleet tuple_c Real48 field for
      multi-year journeys).
   c. Pre-loop fleet setup phase (captures/reassignments; skipped when no
      fleets need reassignment).
   d. Compute fleet visit order (PRNG shuffle seeded from game state).
   e. Run a 52-iteration weekly event scheduling loop (NOT physics sim):
      - for each week 1..52:
        - for each fleet in visit order:
          - read fleet record
          - timing-window check: events to emit this week?
          - if co-located hostile: resolve combat, emit RESULTS.DAT inline
          - update weekly event state in fleet record
          - write fleet record
        - remove destroyed/captured fleets from active set
   f. Post-loop fleet summary scan (2 sequential reads of all fleet records).
   g. Economy/autopilot pass over owned planets (rogue empires only;
      reads post-combat fleet state).
   h. Producer/mutator passes on planet state (`024d` interior).
   i. DATABASE.DAT planet-specific updates.
5. It canonicalizes and sorts summary entries from those outcomes.
6. It performs a late `1..52` weekly report/timing loop over the active
   summaries, using the recovered timing-window constants (8 codes with
   offsets +2/+7/+21/+0/+0/+0/+0/+30 and minimum-week floors).
7. It flushes outputs (`PLAYER.DAT`, `PLANETS.DAT`, `CONQUEST.DAT`,
   `RANKINGS.TXT`) and performs final cleanup.

That is the current oracle-backed canonical turn cycle.
