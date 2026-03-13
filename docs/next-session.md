# Next Session

Use this as the restart brief. Historical detail lives in
[next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md).

## Current Goal

Primary milestone:

- generate 100% `ECMAINT`-compliant gamestate files from Rust
- use the original DOS binaries as the acceptance oracle
- use that compliant generator as the bridge toward a Rust `ECMAINT`
  replacement

## Milestone 3: General Compliant Gamestate Generation

**Status:** In Progress (Phase 1: DATABASE.DAT)

**Definition:** Rust can write an arbitrary full gamestate directory (not just one of the 9 known scenarios) that `ECMAINT` accepts without integrity failures or unexpected normalization. "Arbitrary" means: any valid combination of player count, fleet configurations, starbase presence, IPBM counts, planet states, and orders — within the rules ECMAINT enforces at `2000:5EE4`.

### Phase 1: DATABASE.DAT — Close the Derived Cache Gap

**Goal:** Model and generate DATABASE.DAT (8000 bytes, 80 × 100-byte intel cache records) derived from PLANETS.DAT + CONQUEST.DAT. This closes the remaining 12–15 byte replay drift seen in all `replay-known` runs.

**Work Items:**

1.1 Model DATABASE.DAT in `ec-data`
- Add `DatabaseDat` type: 80 records × 100 bytes each = 8000 bytes
- Add `DatabaseRecord`: raw 100-byte wrapper with accessors for:
  - Display name string slots (Pascal-style copy/trim helpers)
  - Embedded CONQUEST.DAT year word in homeworld records
- Implement `parse()` / `to_bytes()` round-trip
- Does NOT join `CoreGameData` — it's a derived file, not authoritative state

1.2 Build DATABASE.DAT generator from PLANETS.DAT + CONQUEST.DAT
- Map which PLANETS.DAT fields land in which DATABASE.DAT record offsets via fixture comparison
- For each of the 20 planets × 4 per-player slots, populate the 100-byte record:
  - Copy planet display name from PLANETS.DAT
  - Embed CONQUEST.DAT year in appropriate homeworld offsets
  - Use `ecutil-init/v1.5/DATABASE.DAT` as template for unknown bytes

1.3 Wire into workspace/scenario init
- Replace raw file copy of DATABASE.DAT with generation call
- Remove DATABASE.DAT from `PRE_MAINT_REPLAY_CONTEXT_FILES` (should be generated, not copied)

1.4 Validate with oracle
- Target: zero DATABASE.DAT drift in `replay-known` runs
- Regression test: generated DATABASE.DAT matches preserved fixture bytes

### Phase 2: Expand Cross-File Compliance Validator

**Goal:** Promote all known ECMAINT integrity rules (from `2000:5EE4` RE) into explicit `CoreGameData` validators. This is the Rust equivalent of the ECMAINT preflight check.

**Rules to Implement:**

| Rule | Files | Check | ECMAINT Location |
|------|-------|-------|------------------|
| CONQUEST header | CONQUEST.DAT | year in range, player_count matches | Early |
| SETUP header | SETUP.DAT | version_tag == "EC151" | Early |
| PLAYER starbase_count ↔ BASES.DAT | PLAYER[0x44] ↔ BASES count | Count match + BASES[0x04] identity | 2000:5EE4 |
| PLAYER ipbm_count ↔ IPBM.DAT | PLAYER[0x48] ↔ IPBM.DAT length/32 | Exact count match | 2000:5EE4 |
| Fleet owner validation | FLEETS[0x02] | Owner byte matches expected player | 2000:6040..6368 |
| Fleet block structure | FLEETS | id/local_slot/prev/next chain | 2000:6040..6368 |
| Planet owner bounds | PLANETS[0x5D] ↔ CONQUEST[0x02] | owner_slot <= player_count | 2000:5EE4 |
| Base link word validity | BASES[0x05..0x06] | Valid index or 0x0000; no 0x0001/0x0101 | 2000:26582 |
| Guard starbase resolution | FLEET[0x1F]=4, [0x23]=1 | Full fleet↔base linkage | 0000:3fcf..41a0 |

Aggregate into `ecmaint_preflight_errors()` method and wire into `compliance-report` CLI.

### Phase 3: General Gamestate Builder

**Goal:** Enable arbitrary valid gamestate generation via a builder API.

**Work Items:**

3.1 Design the builder API
- `GameStateBuilder` or extend `CoreGameData` with builder pattern
- Parameters: player_count (1-4), per-player fleet configs, per-player starbase configs, per-player IPBM configs, planet assignments, orders
- Builder ensures all cross-file linkage rules satisfied by construction

3.2 Implement initialized baseline builder
- Generalize `sync_current_known_initialized_post_maint_baseline()` to accept parameters
- Support variable player_count, homeworld coordinates, fleet compositions

3.3 Implement order overlay
- Generalize `set_fleet_order`, `set_planet_build`, `set_guard_starbase` pattern
- Support composing multiple order types in one directory

3.4 Generate DATABASE.DAT from built state
- Use Phase 1 generator

3.5 CLI: `generate-gamestate` command
- Accept spec (initially positional args)
- Run `ecmaint_preflight_errors()` before writing

### Phase 4: Oracle Validation Loop

**Goal:** Achieve 100% ECMAINT acceptance for diverse generated gamestates.

**Work Items:**

4.1 Automated oracle sweep
- Generate N diverse gamestates (vary: coords, fleet counts, base presence, IPBM counts, orders)
- Run each through `ECMAINT` oracle
- Collect pass/fail + diff reports

4.2 Regression test suite
- Integration tests that generate diverse gamestates and verify `ecmaint_preflight_errors()` returns empty
- Lock in new rules discovered during oracle sweep

### Open Questions / Risks

1. **DATABASE.DAT record layout** — Full 100-byte structure not decoded; some bytes may come from unmapped fields. Mitigation: start from template and incrementally map via oracle diffing.

2. **Multi-base starbase configurations** — Only single-base is well-understood. Multi-base linkage via `BASES[0x05..0x06]` is partially probed. Mitigation: acceptable to restrict builder to 0-1 bases for Milestone 3.

3. **Variable player_count** — All current work assumes 4 players. ECMAINT behavior with fewer players may have edge cases. Mitigation: start with player_count=4, extend later.

4. **IPBM content normalization** — ECMAINT copies IPBM fields into scratch and builds kind-3 summaries. Non-trivial IPBM must satisfy this normalization. Mitigation: start with IPBM count=0 (simplest accepted state).

### Execution Order

Current: **Phase 1 (DATABASE.DAT)** — Most concrete, closes known gap, prerequisite for Phase 3.

Next: Phase 2 (validators) → Phase 3 (builder) → Phase 4 (oracle sweep)

---

## Working Method

Default method:

- black-box first
- initialize or materialize a controlled directory
- submit one narrow order family or field mutation
- run the original binary oracle
- diff `.DAT` files and reports
- promote deterministic rule into `CoreGameData`

Default harness:

- `python3 tools/ecmaint_oracle.py prepare <target_dir> [source_dir]`
- submit orders or mutate one narrow field family
- `python3 tools/ecmaint_oracle.py run <target_dir>`

Known replay harness:

- `python3 tools/ecmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
- `python3 tools/ecmaint_oracle.py replay-known planet-build /tmp/ecmaint-build-oracle`
- `python3 tools/ecmaint_oracle.py replay-known guard-starbase /tmp/ecmaint-starbase-oracle`
- `python3 tools/ecmaint_oracle.py replay-known move /tmp/ecmaint-move-oracle`

Preserved replay harness:

- `python3 tools/ecmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved planet-build /tmp/ecmaint-build-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved guard-starbase /tmp/ecmaint-starbase-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved move /tmp/ecmaint-move-pre-direct`

### Replay coverage status (confirmed)

| Scenario | ticks | replay-preserved compare | notes |
|----------|-------|--------------------------|-------|
| fleet-order | 1 | **zero diff** ✅ | |
| planet-build | 1 | **zero diff** ✅ | |
| guard-starbase | 1 | **zero diff** ✅ | |
| move | 3 | **zero diff** ✅ | |
| econ | 2 | **non-deterministic** ⚠️ | all remaining diffs non-det |
| bombard | 2 | **non-deterministic** ⚠️ | CA/DD losses random |
| fleet-battle | 2 | **non-deterministic** ⚠️ | battle outcome random |
| invade-heavy | 2 | **non-deterministic** ⚠️ | invasion outcome random |

Non-deterministic diffs in econ: army count growth (rec14 off 0x58), stardock
build queue residual (off 0x3c), fleet CA/DD losses (fleet 2 bombards rec13).
These are all random. There are no deterministic compliance gaps in the econ
scenario.

Field notes from econ investigation:
- `PLANETS.DAT` `0x38..0x4b`: stardock build queue counts (u16_le per slot)
- `PLANETS.DAT` `0x4c..0x4f`: stardock build queue kinds (u8 per slot)
- `PLANETS.DAT` `0x50`: meaning unknown but set after 2 ticks of economy activity
  (present in econ-post, fleet-battle-post, invade-heavy-post rec14); not
  related to stardock queue
- `PLANETS.DAT` `0x38+` gets populated by ECMAINT during tick processing (not
  just from pre-existing build orders); cleared as ships are built
- Build scenario confirms: pre `0x24[slot]`/`0x2e[slot]` (build order) →
  post `0x38[slot*2]`/`0x4c[slot]` (stardock queue entry)

First concrete replay result:

- `python3 tools/ecmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
  runs cleanly through `ECMAINT`, but does **not** land exactly on the
  preserved `fixtures/ecmaint-fleet-post/v1.5` directory
- residual drift after the replay:
  - `PLAYER.DAT`: `2` bytes
  - `PLANETS.DAT`: `18` bytes
  - `FLEETS.DAT`: `9` bytes
  - `DATABASE.DAT`: `29` bytes
- practical implication:
  - our accepted pre-maint `fleet-order` generator is sufficient for the known
    scenario validator, but it is not yet a full exact replay of the preserved
    campaign-state transition
  - use the oracle replay diffs as the next rule-discovery queue instead of
    assuming the current pre-maint shape is exact

Replay queue update:

- `planet-build` replay is now clean for core `.DAT` files
  - `python3 tools/ecmaint_oracle.py replay-known planet-build /tmp/ecmaint-build-oracle`
  - `PLANETS.DAT`: zero diff against preserved post fixture
  - only residual drift is the shared context gap: `CONQUEST.DAT` (year) + `DATABASE.DAT`
    (year embedded in homeworld records) — see below

- `guard-starbase` replay is now fully clean (zero diff against preserved fixture)

Residual `replay-known` shared context gap:

- All three `replay-known` runs still show residual drift in:
  - `CONQUEST.DAT`: 1 byte (offset 0, the year word lo-byte)
  - `DATABASE.DAT`: 12–15 bytes (same year word embedded in homeworld planet records)
- Root cause: `replay-known` seeds from `ecmaint-post/v1.5` which has `game_year=3001`;
  the preserved pre-maint fixtures have `game_year=3000`, so ECMAINT advances to 3001.
  Our generated pre has year 3001, so ECMAINT advances to 3002.
- `replay-preserved` produces zero diff for all three scenarios — confirming the
  oracle harness is correct; the gap is entirely in the Rust-generated year value.
- This is the already-documented shared context gap (`CONQUEST.DAT` + `DATABASE.DAT`).
  No new per-scenario rules are blocked by it.

Preserved pre/post replay validation:

- all three preserved pre-maint fixtures replay exactly to their preserved
  post-maint fixtures under the oracle harness:
  - `fleet-order`
  - `planet-build`
  - `guard-starbase`
- the only extra generated output is `RANKINGS.TXT`, which is not part of the
  preserved post fixtures
- practical implication:
  - the oracle harness is validated
  - the remaining replay gaps are in the Rust-generated pre states, not in the
    replay method

Current replayable-init milestone:

- the shared gap in the Rust-generated pre states is now isolated to the same
  preserved pre-maint replay context files across all three known scenarios:
  - `CONQUEST.DAT`
  - `DATABASE.DAT`
- those bytes are identical across:
  - `ecmaint-fleet-pre`
  - `ecmaint-build-pre`
  - `ecmaint-starbase-pre`
- use:
  - `ec-cli scenario-init-replayable [source_dir] <target_dir> <scenario>`
  when you want an exact preserved pre-maint directory for a known scenario,
  not just an accepted gameplay-table shape
- this now closes the gap completely for the known scenarios:
  - `fleet-order`
  - `planet-build`
  - `guard-starbase`
- practical implication:
  - the earlier `replay-known` residuals were caused by missing shared
    pre-maint replay context, not by unresolved per-scenario post-maint rules
  - for the known scenario families, the next rule-discovery queue is no
    longer in `PLANETS.DAT` gameplay bytes; it is in broader mechanics that do
    not yet have preserved replayable pre-maint constructors

Escalate to deep RE only when:

- the path is blocking broader compliant gamestate generation
- black-box testing has plateaued
- the expected rule is reusable

The current Guard Starbase / `unknown starbase` thread meets that bar. Do not
use its depth as the default workflow for unrelated mechanics.

## Recently Resolved

### Autopilot flag — PLAYER.DAT offset 0x6d

Controlled black-box experiment on `original/v1.5`:

- clearing `PLAYER.DAT[0x6d] = 0` (player 1) eliminated all army and battery
  growth on Dust Bowl across an ECMAINT run
- with `PLAYER.DAT[0x6d] = 1` (original state), ECMAINT builds planetary
  defenses on autopilot (armies +19, batteries +1 in that run)
- confirmed: **`PLAYER.DAT` offset `0x6d` is the autopilot flag**
  (1 = on, 0 = off); matches player docs: "mostly building your planetary
  defenses"
- `PLAYER.DAT` offset `0x00` is the player active/present flag
  (1 = joined player, 0 = unjoined slot)

### `raw[0x0E]` isolated behavior

Without autopilot, `raw[0x0E]` on an owned planet decrements by 1 per tick.
With autopilot on, it reflects autopilot production spending. Not yet fully
decoded, but it is not the empire-wide tax rate (that is PLAYER.DAT[0x51]).

### Factory growth behavior

With a positive player tax rate, current_production (the factories Real at
`raw[0x04..0x0A]`) doubles approximately every 2–3 ticks, with `raw[0x0E]`
acting as an accumulator that resets near 3–4 after each doubling. The exact
accumulator rule is not yet decoded. Current_production can exceed `potential`
during growth.

### Economy tick: unjoined homeworld seeds are stable

Canonical baseline (tax=0, unjoined players) → zero PLANETS.DAT changes under
ECMAINT, regardless of army/battery/factories values. Tax=0 means no
production points, so no factory growth and no autopilot spending.

---

## Current State

What is strong:

- `ec-data::CoreGameData` is now the shared model for current-known multi-file
  state, validation, mutation, and repair
- the current-known post-maint core baseline is byte-complete for:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `IPBM.DAT`
  - `SETUP.DAT`
  - `CONQUEST.DAT`
- Rust can now:
  - materialize current-known baseline directories
  - materialize exact canonical post-maint core-baseline directories
  - validate current-known structural rules
  - validate exact canonical post-maint core-byte matches
  - transform the preserved initialized fixture
    [ecutil-init/v1.5](/home/mag/dev/esterian_conquest/fixtures/ecutil-init/v1.5)
    all the way to the exact canonical post-maint core baseline
  - emit accepted scenario directories for:
    - `fleet-order`
    - `planet-build`
    - `guard-starbase`
    - `ipbm`
    - `move`
    - `bombard`
    - `fleet-battle`
    - `invade`

What is still incomplete:

- arbitrary `ECMAINT`-compliant gamestate generation
- remaining `ECMAINT` cross-file linkage rules beyond the now-complete
  Guard Starbase blocker pass
- deeper `IPBM` gameplay semantics beyond the currently mapped structure
- reliable local `ECGAME` startup / ANSI preservation, which is useful but not
  the main blocker

## Biggest Remaining Gains

Priority order:

1. Treat the Guard Starbase / `unknown starbase` blocker pass as complete
   - accepted one-base case uses direct decoded-key match on base-side
     `[3558] = [355A] = 1`
   - failing `fleet[0x23] = 0` case proves the discriminator is later than:
     - base-side kind-2 decode
     - fleet-side kind-1 summary emission
   - decisive late accept/reject structure is now recovered:
     - `0000:3fcf..41a0`
       - success requires located summary active, current summary `+0x01`,
         `+0x02`, and `+0x05` matching the located entry, and `350c > 0`
     - `0000:42d8..456e`
       - deeper structural match requires `3504`, `350d`, `350e`, and
         `f(351b..351f)` plus decoded local `+0x23 == 3525` and decoded local
         flag `+0x0a == 0`
   - late report-only findings are also recovered:
     - `3521` is a late report-layout / variant mode byte
     - mode map:
       - `6 -> [10, 20, 30, 40]`
       - `7 -> [20, 25, 25, 30]`
       - `8 -> [0, 0, 0, 100]`
     - `b9a7 != 0` -> merge/commit path
     - `b9a7 == 0` -> already-guarding / ship-limit abort-report path
   - runtime-only late path is now mapped back into the static image:
     - live `2895:27ac` -> static `2000:2fbc`
     - live `2895:7e4b` -> static `2000:865b`
   - stop condition:
     - remaining unresolved `3521` mode-text semantics are on the UI/report
       side, not the compliance side
     - do not spend more deep RE time here unless the task is explicit
       UI/report preservation

2. Recover initialized-to-post-maint deterministic rules
   - the clean preserved initialized fixture is now fully covered
   - after current-known normalization, the noisier shipped sample in
     `original/v1.5` now only differs from the canonical post-maint core
     baseline in:
     - `PLAYER.DAT`
     - `PLANETS.DAT`
     - `FLEETS.DAT`
   - important interpretation:
     - `original/v1.5` is not just a noisy initialized baseline
     - but coordinate differences alone are not evidence of a special campaign
       state, because the starmap and empire homeworlds are randomized per game
     - treat remaining coordinate/topology drift as setup variance until a
       non-coordinate rule is proven
   - use canonical post-maint diff output from normalized `original/v1.5`
   - promote only clearly reusable clusters from it into shared Rust rules
   - do not assume its remaining planet/fleet drift represents a deterministic
     initialized-to-post-maint transition

3. Expand `IPBM` from structural to semantic
   - the file is structurally mapped enough for Rust tooling
   - but not semantically complete enough for general engine replacement

4. Defer `ECGAME` ANSI/startup work unless needed for a specific preservation
   task
   - useful, but not the main blocker for compliant gamestate generation

## Concrete Next Task

All known scenarios now have Rust generators and passing tests:

| Scenario | generator | tests | pre-fixture exact match |
|----------|-----------|-------|------------------------|
| fleet-order | ✅ | ✅ | ✅ |
| planet-build | ✅ | ✅ | ✅ |
| guard-starbase | ✅ | ✅ | ✅ |
| move | ✅ | ✅ | ✅ |
| ipbm | ✅ | ✅ | ✅ |
| bombard | ✅ | ✅ | ✅ |
| fleet-battle | ✅ | ✅ | ✅ |
| invade | ✅ | ✅ | ✅ |
| econ | ✅ | ✅ | ✅ |

**Milestone 1 (Known accepted scenarios) is complete.**

**Milestone 2 parameterized init is complete.**

**Milestone 3 in progress:** Phase 1 — DATABASE.DAT modeling and generation.

**Current implementation task:** 

1. Add `DatabaseDat` and `DatabaseRecord` types to `ec-data`
2. Implement `parse()`/`to_bytes()` for round-trip
3. Build generator that creates DATABASE.DAT from PLANETS.DAT + CONQUEST.DAT
4. Wire into workspace init to replace raw file copy
5. Validate with oracle: target zero DATABASE.DAT drift

See Milestone 3 section at top of this file for full plan.

## Canonical Baseline Tools

Use these when comparing Rust output to the preserved post-maint oracle:

- `cargo run -q -p ec-cli -- core-validate-current-known-baseline <dir>`
- `cargo run -q -p ec-cli -- core-diff-canonical-current-known-baseline <dir>`
- `cargo run -q -p ec-cli -- core-diff-canonical-current-known-baseline-offsets <dir>`
- `cargo run -q -p ec-cli -- core-init-canonical-current-known-baseline [source_dir] <target_dir>`
- `cargo run -q -p ec-cli -- core-sync-canonical-current-known-baseline <dir>`

Current important distinction:

- `core-sync-current-known-baseline` applies the bounded shared-model
  normalizer
- `core-sync-canonical-current-known-baseline` overlays the exact preserved
  post-maint core `.DAT` oracle

## RE Focus Files

Read these for the current phase:

- [RE_NOTES.md](/home/mag/dev/esterian_conquest/RE_NOTES.md)
  Focus on DATABASE.DAT sections and the `5EE4` integrity validator.
- [tools/dump_db.py](/home/mag/dev/esterian_conquest/tools/dump_db.py)
  DATABASE.DAT inspection tool.
- [approach.md](/home/mag/dev/esterian_conquest/docs/approach.md)
- [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/rust-architecture.md)

Historical handoff detail:

- [next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md)

## Preservation TODO

Still explicitly wanted, but not the immediate blocker:

- preserve original `ECGAME` ANSI opening/menu/report screens for the Rust
  client
- resume this once the local `ECGAME` harness is reliable enough or when UI
  preservation becomes the active milestone
