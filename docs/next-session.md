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

Preserved replay harness:

- `python3 tools/ecmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved planet-build /tmp/ecmaint-build-pre-direct`
- `python3 tools/ecmaint_oracle.py replay-preserved guard-starbase /tmp/ecmaint-starbase-pre-direct`

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

- `planet-build` is now the cleanest next black-box queue
  - `python3 tools/ecmaint_oracle.py replay-known planet-build /tmp/ecmaint-build-oracle`
  - clean `ECMAINT` run, no `ERRORS.TXT`
  - residual drift is tightly isolated:
    - `PLANETS.DAT`: `6` bytes, all in record `15`
    - `DATABASE.DAT`: `1` byte
  - current -> canonical post deltas on `PLANETS.DAT` record `15`:
    - offset `0x09`: factory tail byte `134 -> 0`
    - offset `0x0E`: tax `12 -> 0`
    - offset `0x24`: build-count slot `3 -> 0`
    - offset `0x2E`: build-kind slot `1 -> 0`
    - offset `0x38`: developed-value byte `0 -> 3`
    - offset `0x4C`: stardock-kind slot `0 -> 1`
  - practical interpretation:
    - this looks like a clean build-completion / queue-consumption /
      stardock-emission transition
    - it is a better immediate rule-discovery target than the broader
      `fleet-order` replay gap

- `guard-starbase` replay is nearly exact
  - `python3 tools/ecmaint_oracle.py replay-known guard-starbase /tmp/ecmaint-starbase-oracle`
  - only residual drift:
    - `PLAYER.DAT`: `1` byte at offset `70`
  - this is currently lower priority than the isolated `planet-build` queue

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

Start with the verified black-box oracle loop, not more starbase deep RE.

Best immediate task:

- use `replay-preserved` to validate the oracle path for a mechanic family
- use `replay-known` to measure the remaining gap in the Rust-generated
  pre-maint state
- promote those residual diffs into `CoreGameData`
- only if that plateaus, return to static/dynamic RE

Useful prep/oracle commands:

- `python3 tools/ecmaint_oracle.py prepare /tmp/ecmaint-oracle`
- `python3 tools/ecmaint_oracle.py run /tmp/ecmaint-oracle`
- `cargo run -q -p ec-cli -- core-init-current-known-baseline original/v1.5 /tmp/ec-from-original`
- `cargo run -q -p ec-cli -- core-report-canonical-transition-clusters /tmp/ec-from-original`
- `cargo run -q -p ec-cli -- core-report-canonical-transition-details /tmp/ec-from-original`

Recommended order:

1. `PLANETS.DAT`
   - explain the remaining repeated economy/homeworld payload clusters
   - this is now the upstream target for the shipped sample
   - the new transition-details report shows the shipped sample still has a
     different homeworld/unowned topology from the canonical post-maint
     baseline, but the coordinates themselves may simply reflect randomized
     setup
   - likewise, planet-name drift is not deterministic maintenance state by
     itself because players are allowed to rename colonized planets
   - examples:
     - current record 13 homeworld seed at `(6,12)` vs canonical `(4,13)`
     - current record 16 `Dust Bowl` owned world at `(16,13)` vs canonical
       unowned record 16 and canonical player-1 homeworld seed at record 15
         `(16,13)`
2. `FLEETS.DAT`
   - after current-known normalization, remaining fleet drift collapses to
     offsets `11/12` and `32/33`
   - those are already-mapped location/target coordinate fields
   - practical implication:
     - remaining `FLEETS.DAT` drift is derived from the sample’s planet /
       homeworld state, not an independent queue
   - the new detail report confirms the dependency:
     - fleet block 2 follows current `(6,12)` vs canonical `(4,13)`
     - fleet block 3 follows current `(16,5)` vs canonical `(6,5)`
     - fleet block 4 follows current `(7,4)` vs canonical `(13,5)`
3. `PLAYER.DAT`
   - promote only count/summary words that are supported by evidence
4. `IPBM.DAT`
   - move from structural validity toward real gameplay semantics

Why this first:

- the Guard Starbase blocker is complete enough for compliance work
- Rust tooling is no longer the main bottleneck
- this loop scales better than another deep rabbit hole
- initialized-to-post-maint rule discovery is now best driven by controlled
  before/after oracle runs

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
