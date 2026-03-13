# Next Session

Use this as the restart brief. Historical detail lives in
[next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md).

## Current Goal

Primary milestone:

- generate 100% `ECMAINT`-compliant gamestate files from Rust
- use the original DOS binaries as the acceptance oracle
- use that compliant generator as the bridge toward a Rust `ECMAINT`
  replacement

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
  - emit accepted scenario directories for:
    - `fleet-order`
    - `planet-build`
    - `guard-starbase`
    - `ipbm`

What is still incomplete:

- arbitrary `ECMAINT`-compliant gamestate generation
- remaining `ECMAINT` cross-file linkage rules, especially the unresolved
  `5EE4` fleet/base matcher semantics
- deeper `IPBM` gameplay semantics beyond the currently mapped structure
- reliable local `ECGAME` startup / ANSI preservation, which is useful but not
  the main blocker

## Biggest Remaining Gains

Priority order:

1. Finish `ECMAINT` `5EE4` fleet/base linkage semantics
   - highest-value blocker for milestone 3
   - remaining gap is around the kind-`1` / kind-`2` matcher and decoded
     `3558/355A` keys
   - this is the main blocker between accepted one-base scenarios and more
     general compliant gamestate generation
   - latest concrete anchor from the accepted one-base fixture:
     - live matcher post-decode stop `CS=0824 EIP=0303`
     - decoded base-side keys `[3558] = 1`, `[355A] = 1`
     - decoded tuple/control bytes also line up with guard target
       coordinates `(16,13)`
     - practical implication:
       - the accepted one-base case likely succeeds on the direct decoded-key
         path (`candidate +0x0A == [3558]`)
       - the structural candidate-decode path may not be needed in that case
   - latest concrete narrowing from the failing `fleet[0x23] = 0` case:
     - it still reaches the same live base-decode stop `CS=0824 EIP=0303`
     - its base-side decode block is byte-for-byte identical to the accepted
       one-base case
     - practical implication:
       - the `unknown starbase` discriminator is not in the base-side decoded
         `3558/355A` object
       - static fleet-branch mapping also shows `fleet[0x23]` is not read by
         the kind-`1` summary emitter at `2000:6040..6368`
       - the remaining rule must therefore be later than both:
         - the base-side kind-`2` decode
         - the fleet-side kind-`1` summary emission
       - the new `0000:06AE..0800` dump shows kind `2` falls straight into the
         generic post-kind canonicalization path; the special work there is for
         kind `3`
       - the first concrete later consumer is now `0000:1302..1361`, which:
         - loops active summaries (`+0x03 != 0`)
         - calls the shared loader `0000:02C0`
         - then dispatches each active entry through two far calls in segment
           `1000`
       - first-pass dumps of `1000:a26e` and `1000:0b51` look generic /
         report-oriented rather than starbase-specific
       - the first genuinely starbase-specific later region is now
         `0000:3fcf..41a0`, immediately after the raw
         `Fleet assigned to an unknown starbase.` string
       - that region is now tightened to a concrete late predicate:
         - current summary index comes from caller arg `[BP+0x04]`
         - located candidate summary slot comes from local `[BP-0x28]`
         - success requires:
           - located summary active (`+0x03 != 0`)
           - current `+0x01 == located +0x01`
           - current `+0x02 == located +0x02`
           - current `+0x05 == located +0x05`
           - `byte ptr [0x350c] > 0`
         - on success it only sets local success flag `[BP-1] = 1`
         - on failure/report it formats output from `3502` scratch fields
           `3525`, `351b..351f`, `350d`, `350e`, and `3504`
         - branch `40f7..410c` selects between two nearby CS-local string
           variants depending on whether `351b..351f` is zero
         - both failure/report exits clear `350c` / `3521`
         - producer-side split is now tighter:
           - `350d` / `350e` are the first two decoded tag bytes from the
             shared kind-`1` summary `+0x06` decoder
           - `351b..351f` is the later 3-word payload group from the same
             common post-kind canonicalization pipeline
           - `350c` is the decoded selector/control byte copied out by the
             kind-`1` loader and checked by the late predicate
           - `3525` is now narrowed further by the later block at
             `42d8..456e`:
             - candidate summary must match `3504`, `350d`, `350e`, and
               `f(351b..351f)`
             - then decoded candidate local `+0x23` must equal `[3525]`
             - decoded candidate local flag `+0x0a` must be `0`
             - after that structural hit, the same block now looks like a real
               second late resolution/report loop:
               - it calls `0x2000:b9a7`
               - splits into two CS-local report families
               - `b9a7 != 0` takes the smaller family and then calls
                 `0x2000:d3bb`
                 - best current label: merge/commit path
               - `b9a7 == 0` takes the larger family, formats literal `3000`,
                 and exits after clearing `3521` / `350c`
                 - best current label: already-guarding / ship-limit
                   abort-report path
               - the fallback path re-runs `0x1000:d183`, copies the selected
                 entry back through `0x2000:c151`, rewrites `351b..351f`, and
                 finalizes through `0x2000:c100`, `0x2000:c02a`, and
                 `0x2000:c2f0`
               - it explicitly clears `3521` and `350c` before exit
           - `3521` behaves like a late report/control selector byte and is
             reset when the later report flow completes
             - concrete later mode map now recovered:
               - `6` -> writes `[10, 20, 30, 40]` to `0x630..0x633`
               - `7` -> writes `[20, 25, 25, 30]`
               - `8` -> writes `[0, 0, 0, 100]`
             - those values later flow through `f812` / `f8f2`, which pass
               `3521` and CS:`0x6766` to `0x3000:44b7` and only continue the
               follow-on path on nonzero return
             - best current label: late report-layout / variant mode byte
         - nearby raw strings after `41a1` show this region also owns the
           wider starbase merge/guard report family:
           - arrival at starbase
           - merging with fleet
           - found fleet already guarding it
           - cannot merge because fleet would exceed ship limit
       - `0x1000:d183` is now narrowed to a candidate locator/selector:
         - scans the `0x1712` table
         - filters matching entries
         - sorts multiple candidates
         - returns success in `AL` and two selected bytes via output pointers
         - the candidate index list is 1-based at local `FECC`
         - the first real candidate slot is `FECE`
         - the sort/swap block normalizes the winning candidate back into that
           first slot
         - the return block reads selected entry bytes `0x00` and `0x01` from
           `FECE`
         - practical implication:
           - the stable side effect is the selected-entry pair
           - the direct register return is only a boolean success gate
       - next target should now stay in `0000:3fcf..41a0`, especially:
         - the exact semantic labels for `3521`
         - the exact CS-local report variants chosen across both late blocks
           (`3fcf..41a0` and `42d8..456e`)
         - the human-facing meaning of `3521` modes `6`, `7`, and `8`
         - which scratch fields and helper returns choose each variant
         - exact runtime text bodies for the late CS-local report references
           around `0x0d30` / `0x0d53`, which did not decode as plain raw-import
           strings
         - the downstream `3521` consumer at `0x3000:44b7` also appears as
           zero bytes in the current raw `MEMDUMP.BIN`, and so does `CS:6766`
           - practical implication:
             - remaining `3521` semantics now require runtime-aware capture
               around the live consumer
             - do not spend more time blindly carving the zeroed `3000:` range
         - a runtime write-stop dump on the failing `unknown starbase` case now
           confirms the same limit:
           - at the `ERRORS.TXT` write stop (`AX=40d0`, `BX=0006`,
             `CS=3374:EIP=1953`), `ERRORS.TXT` already contains
             `Fleet assigned to an unknown starbase.`
           - but the nominal raw-dump ranges for `3000:44b7`, `3000:6766`,
             and even `0x3521` are still zero under the current linear model
           - practical implication:
             - the remaining selector semantics are outside what the current
               PSP-owned dump exposes under the old `3000:` assumptions
             - next method should be runtime-segment-aware capture around the
               live consumer path, not more raw-dump carving

2. Recover initialized-to-post-maint deterministic rules
   - use canonical post-maint diff output from normalized `original/v1.5`
   - promote remaining deterministic byte clusters into shared Rust rules

3. Expand `IPBM` from structural to semantic
   - the file is structurally mapped enough for Rust tooling
   - but not semantically complete enough for general engine replacement

4. Defer `ECGAME` ANSI/startup work unless needed for a specific preservation
   task
   - useful, but not the main blocker for compliant gamestate generation

## Concrete Next Task

Start with `5EE4` rule discovery, not more Rust refactoring.

Best immediate task:

- finish the unresolved base/fleet linkage semantics in the `ECMAINT` matcher
- target the remaining kind-`1` / kind-`2` path around the decoded `+0x06`
  helper keys and the `3558/355A` comparisons
- specifically compare:
  - the accepted one-base direct-key path
  - a failing `unknown starbase` case
  - the new narrowing is that the failing `fleet[0x23] = 0` case already has
    the same base-side decoded keys as the accepted case
  - static fleet-branch mapping also rules out `fleet[0x23]` as an input to
    the emitted kind-`1` summary itself
  - the immediate `0000:06AE..0800` handoff is mostly generic
    canonicalization, not a starbase-specific decision block
  - the next concrete consumer after generic sort/report staging is now
    `0000:1302..1361`
  - the first two segment-`1000` callees from that loop (`a26e`, `0b51`) now
    look generic/report-oriented
  - the raw `unknown starbase` string exists at `0000:3f89`, but the raw-import
    xref pass found no direct references
  - the new concrete later target is `0000:3fcf..41a0`, immediately after that
    raw string
  - so the next capture/search should focus on:
    - the exact semantic label for `3521`
    - the exact late starbase report variants around the raw strings after
      `41a1`
    - any dynamic confirmation of the caller-side `AX` / located-summary slot
      relationship at `3fe8`
- once those rules are recovered, promote them into `CoreGameData`

Why this first:

- Rust tooling is no longer the main bottleneck
- this is the narrowest remaining RE gap with the biggest payoff
- it should unlock broader compliant starbase/base/fleet generation

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
