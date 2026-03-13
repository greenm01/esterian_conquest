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
       - next target should therefore be those later active-summary callees,
         not the matcher/canonicalizer itself

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
  - so the next capture/search should target the segment-`1000` callees from
    that loop, not the base decode, raw kind-`1` summary emission, or the
    immediate post-match handoff
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
