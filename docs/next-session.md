# Next Session

Use this as the restart brief. Historical detail lives in
[next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md).

## Current Goal

Primary milestone:

- generate 100% `ECMAINT`-compliant gamestate files from Rust
- use the original DOS binaries as the acceptance oracle
- use that compliant generator as the bridge toward a Rust `ECMAINT`
  replacement

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

Scenarios `fleet-order`, `planet-build`, `guard-starbase`, `move`, `ipbm`,
`bombard`, `fleet-battle`, and `invade` now have Rust generators and passing
tests. `bombard`, `fleet-battle`, and `invade` exact-match their preserved
pre-maint fixtures for `FLEETS.DAT` and `PLANETS.DAT`.

The remaining non-deterministic scenarios still need pre-maint generators:

1. **`econ`** — economy tick; requires understanding the production/tax rule
   (raw[0x0e] and factories_word at raw[0x08..0x09])

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

Read these for the next phase:

- [RE_NOTES.md](/home/mag/dev/esterian_conquest/RE_NOTES.md)
  Focus on the `5EE4`, Guard Starbase, and `IPBM` sections.
- [ghidra-workflow.md](/home/mag/dev/esterian_conquest/docs/ghidra-workflow.md)
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
