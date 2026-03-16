# ECMAINT Investigation Plan

`ECMAINT` is the highest-value target for recovering the actual game engine.

## Progress Summary

Phase 1 (build queue) and subsequent investigation cycles are **complete**. The
black-box workflow is established and has been used successfully across multiple
scenario families:

- **Build queue**: single planet build queue transition captured and diffed
- **Economics**: production, tax, population maintenance pass captured
- **Fleet movement**: fleet coordinate deltas across maintenance observed
- **Bombardment**: extensive bombardment matrix (army counts, development
  levels, battleship counts) with 12+ pre/post fixture pairs
- **Invasion**: heavy invasion scenario captured
- **Fleet combat**: fleet-vs-fleet battle scenario captured
- **Starbases**: Guard Starbase order (0x04) fully decoded — three essential
  fields identified (`FLEETS.DAT[0x1F]`, `FLEETS.DAT[0x23]`, `PLAYER.DAT[0x44]`)
- **Cross-file integrity**: ECMAINT performs cross-file integrity checks;
  methodology accounts for this

The DOSBox-X testing workflow is documented in `docs/dosbox-workflow.md`.

## Original Phase-1 Goal (Completed)

The original goal was to establish a repeatable black-box workflow, centered on
one build-queue scenario, that produces:

- one preserved pre-maint fixture
- one preserved post-maint fixture
- one file-diff summary tied to a known gameplay change
- at least one new confirmed maintenance-driven field or transition
- at least one new fixture-backed Rust test

All of these were achieved. The workflow is now standard practice.

## First Scenario

Center phase 1 on a single planet build queue transition.

Why this scenario comes first:

- build queues were already observed to land in `PLANETS.DAT`
- build completion is likely easier to observe than combat resolution
- it should expose whether maintenance materializes queued production into fleets, planet-state updates, or both
- it is a more controlled first transform than a battle-producing setup

The first scenario should introduce exactly one known build order on one known homeworld-style planet and nothing else.

## Working Model

Treat `ECMAINT` as a deterministic transform from:

- `PLAYER.DAT`
- `PLANETS.DAT`
- `FLEETS.DAT`
- `CONQUEST.DAT`
- support files in the game directory

into:

- updated core state
- derived database/report outputs
- player-visible message/result files

For phase 1, treat the binary as a black box. Static binary mapping is explicitly out of scope until the first transform is captured cleanly.

## Phase-1 Procedure

### 1. Choose the baseline fixture

Start from the cleanest pre-maint state that already matches the current Rust assumptions.

Default choice:

- `fixtures/ecutil-init/v1.5/`

Use `fixtures/ecmaint-post/v1.5/` only if the scenario requires a post-maint-derived file to exist first, and record that exception explicitly in `RE_NOTES.md`.

### 2. Create one controlled pre-maint change

Introduce exactly one build queue change:

- one planet
- one build order
- no other player, fleet, maintenance-day, or ownership changes

Before running maintenance, record:

- which planet was changed
- what was ordered
- which file offsets changed in the pre-maint state

### 3. Preserve the pre-maint fixture

Create:

- `fixtures/ecmaint-build-pre/v1.5/`

Copy the full scenario directory there, not only the changed files.

The fixture note in `RE_NOTES.md` must include:

- baseline fixture used
- exact planet/build order
- whether the scenario was generated through original DOS tools or direct file editing

### 4. Run ECMAINT

Run the original `ECMAINT` against the pre-maint scenario directory and preserve the exact post-run state.

Create:

- `fixtures/ecmaint-build-post/v1.5/`

Again, copy the full directory, not only changed files.

### 5. Diff the full output set

Diff these files first:

- `PLANETS.DAT`
- `FLEETS.DAT`
- `DATABASE.DAT`
- `MESSAGES.DAT`
- `RESULTS.DAT`
- `CONQUEST.DAT`

Also check:

- `PLAYER.DAT`

For each changed file, classify it as one of:

- core persistent state
- derived/indexed state
- report/message output

The phase-1 expectation is:

- `PLANETS.DAT` should show the queue consuming or changing state
- `FLEETS.DAT` may show newly materialized fleets
- `DATABASE.DAT` should reflect derived player-facing database output
- `MESSAGES.DAT` and `RESULTS.DAT` should expose player-visible maintenance consequences
- `CONQUEST.DAT` should show global turn/year/summary movement

If `ECGAME` can be launched reliably against the post-maint fixture, use it to
view the generated reports and confirm that the rendered text matches the raw
file changes. That live report viewing is useful, but it is not the primary
artifact. The primary artifact is still the preserved post-maint `.DAT` state.

### 6. Record one concrete maintenance outcome

Phase 1 is only successful if at least one specific post-maint transition is named and grounded.

Examples of acceptable outcomes:

- a build queue byte cleared in `PLANETS.DAT`
- a new fleet record created in `FLEETS.DAT`
- a production/result message generated in `MESSAGES.DAT`
- a global counter advanced in `CONQUEST.DAT`

Do not promote fields based on guesswork. Record raw bytes if semantics are still unclear.

### 7. Feed the result back into Rust

After the diff is understood enough to be actionable:

- add or refine the corresponding accessors in `rust/ec-data`
- preserve the new fixtures in repo
- add at least one fixture-backed test in `ec-data` or `ec-cli`

If the result is still too ambiguous for a named field, add a conservative raw accessor instead of inventing semantics.

## Files and Repo Artifacts To Use

This phase should explicitly use:

- `fixtures/ecutil-init/v1.5/`
- `fixtures/ecmaint-post/v1.5/`
- `docs/fixtures.md`
- `RE_NOTES.md`
- `rust/ec-data`
- `rust/ec-cli`

New phase-1 fixture names are fixed:

- `fixtures/ecmaint-build-pre/v1.5/`
- `fixtures/ecmaint-build-post/v1.5/`

## Acceptance Criteria

Phase 1 is complete only when all of the following exist:

1. a preserved pre-maint build fixture
2. a preserved post-maint build fixture
3. a diff summary recorded in `RE_NOTES.md`
4. at least one confirmed maintenance-driven field or record transition
5. at least one new fixture-backed Rust test

If a run produces only noisy derived outputs and no interpretable core-state change, the scenario is not complete and should be repeated with a simpler or clearer build order.

## Validation Sources

Prefer these sources in this order:

1. controlled pre/post `.DAT` fixture diff
2. repeated observation across multiple preserved states
3. live `ECGAME` report viewing against the post-maint fixture
4. historical text captures that show the same class of event

This order matters. `ECGAME` and old text reports help interpret the result
files, but they do not replace the raw engine outputs generated by `ECMAINT`.

## After Phase 1 (Status)

The bombardment investigation has moved well beyond the initial seed-shell
target. Multiple fixture pairs exist with varying army counts, development
levels, and attacker compositions. The key finding about friendly-vs-hostile
targeting (where a mature colony owned by the attacker's empire was treated as
friendly and caused order conversion to guard/blockade) was addressed by
ensuring proper enemy ownership on target worlds.

The invasion investigation has one heavy-invasion fixture pair preserved.

The starbase investigation decoded the Guard Starbase order completely,
identifying three essential fields and the failure/error behavior.

## Open Investigation Targets

These are the next candidate scenarios, in rough priority order:

1. **Movement math**: set a fleet to Move Only (order `1`) with known speed and
   observe coordinate deltas across maintenance passes to recover the movement
   formula.
2. **FLEETS.DAT[0x23] semantics**: create a second starbase and cross-reference
   values to determine if this is an empire ID, a base index, or something else.
3. **Rogue/AI empire behavior**: observe what ECMAINT does for non-player empires.
4. **IPBM resolution**: planetary bombardment missiles — untouched so far.
5. **Build queue mechanics**: deeper investigation of queued production
   materialization into fleets.

These should be planned one at a time, following the established fixture workflow.
