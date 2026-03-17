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

## Best Recovery Workflow For Step 4

Current best method for uncovering the yearly simulation core:

1. build controlled one-mechanic oracle scenarios first
2. run classic `ECMAINT` and diff persistent state plus durable summary outputs
3. promote repeated field-level mutations into a step-4 evidence matrix
4. use static RE only on the specific helpers that those scenario diffs keep
   pointing at
5. stop once the ordering boundary is explicit enough to guide Rust

Practical rule:

- a full linear assembly dump is possible, but it is not the highest-yield tool
  for this problem by itself
- the hard part is trustworthy structure:
  - code vs data separation
  - real function boundaries
  - cross-segment control flow
  - deciding which routines mutate durable game state and which only format
    late reports
- the current late-summary work recovered around `5ee4`, `861d`, `87f4`,
  `f319`, and `f34a` is the cautionary example:
  - broad static coverage alone can spend a lot of time on important-looking
    code that is still only report/output plumbing

Current highest-yield step-4 target:

- keep pairing controlled oracle diffs with the partially recovered
  `1000:024d` owned-planet interior
- the strongest current seam is now the `1000:03ff..0d53` body *inside*
  `1000:024d`, because it:
  - iterates owned planets directly
  - gates on owner/player state
  - reads durable kind-`2` entries
  - folds derived values back into planet fields
- treat `024d` as the current leading seam for practical Rust modeling until a
  stronger earlier simulation driver is recovered
- when running direct oracle probes for that seam, always inspect both:
  - persistent state/output drift (`*.DAT`, `RANKINGS.TXT`)
  - player-visible report/error channels (`RESULTS.DAT`, `MESSAGES.DAT`,
    `ERRORS.TXT`)
- practical reason:
  step-4 placement depends not only on which records mutate, but also on
  whether a probe produces visible report traffic or only silent state /
  derived-database changes

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

#### 4k. Early fleet-battle passes show incremental fleet activation

Confidence: `High`

Source: fleet-battle trace pass-by-pass record indices.

The first 5 passes show fleet records appearing incrementally:

- pass 1: `[1]` (1 record)
- pass 2: `[1,3]` (2 records)
- pass 3: `[1]` (1 record)
- pass 4: `[1,0]` (2 records)
- pass 5: `[1,11,6]` (3 records)
- pass 6+: stable 14-record set `[1,14,7,4,9,8,12,15,0,13,10,5,11,6]`

The final pass (57) shows 12 records: `[1,14,7,4,9,8,12,15,0,13,10,5]` —
fleets 11 and 6 were dropped (destroyed in combat).

Practical meaning:

- the weekly loop does not iterate a static full-size fleet table from the
  start; fleet records enter the active write set dynamically
- the incremental startup may reflect movement arrival: fleets reaching
  their destinations and becoming "active" in the simulation sense
- fleet 2 (empire 1's bombard fleet targeting planet 14 from (16,13)) is
  never in the write set — its mission may have resolved differently or it
  was absorbed before the main loop started

#### 4l. Fleet visit order is PRNG-shuffled, seeded from game state

Confidence: `High`

Source: cross-fixture comparison. Four scenarios with identical
CONQUEST.DAT, SETUP.DAT, and nearly identical FLEETS.DAT (only fleet 2's
ship counts differ) but different PLANETS.DAT produce completely different
visit orders.

Evidence:

- bombard:     `[11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12]`
- econ:        `[11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10]`
- fleet-order: `[6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8]`
- planet-build:`[15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5]`

The linked list traversal (`next_fleet_link`) does NOT match any of these
orderings. The fleet data is nearly identical but the orderings are
completely different, ruling out all simple fleet-field-based sort keys.

The only varying file among these four fixtures is PLANETS.DAT (planet 13's
name and numeric fields differ). Small planet data changes cascade into
completely different fleet orderings — characteristic of PRNG seeding.

Practical meaning:

- the fleet visit order is determined by a shuffle or sort using a PRNG
  seeded from game state (likely including planet data)
- the PRNG is probably Borland Pascal's `Random` function
- for Rust oracle parity, either replicate the exact PRNG or accept that
  visit order is deterministic-per-state but not trivially reproducible
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
- first coarse file-I/O tracing on a classic `bombard` run supports a broad
  phase split:
  - a long `FLEETS.DAT` write burst lands before later writes to
    `DATABASE.DAT`, `PLAYER.DAT`, `PLANETS.DAT`, `CONQUEST.DAT`, and
    `RANKINGS.TXT`
  - practical meaning:
    heavy fleet-state mutation clearly precedes the late derived-output tail,
    but this still does not prove the precise middle ordering of movement,
    economy, combat, or producer passes

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
