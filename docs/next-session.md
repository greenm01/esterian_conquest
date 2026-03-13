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

Escalate to deep RE only when:

- the path is blocking broader compliant gamestate generation
- black-box testing has plateaued
- the expected rule is reusable

The current Guard Starbase / `unknown starbase` thread meets that bar. Do not
use its depth as the default workflow for unrelated mechanics.

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

Start with initialized-to-post-maint rule discovery, not more starbase deep RE.

Best immediate task:

Use the black-box oracle loop for new mechanics first, not more deep RE.

Best immediate task:

- initialize a clean directory
- submit one controlled order family
- run `ECMAINT`
- diff `.DAT` and report outputs
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
