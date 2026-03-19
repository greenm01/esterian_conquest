# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](archive/next-session-archive.md),
not here.

## Current State

The SQLite-first runtime split is now in place:

- `ec-client` reads runtime state from `ecgame.db` and no longer needs live
  `.DAT` ownership
- `maint-rust` advances the SQLite-backed runtime snapshot instead of silently
  rewriting classic files in place
- classic `.DAT` files are now an explicit compatibility/export layer driven by
  `db-import`, `db-export`, and classic materialization/oracle workflows
- read-only CLI/report paths should not create `ecgame.db` just by inspecting a
  classic directory
- current oracle-facing `DATABASE.DAT` / planet-intel work belongs in the
  classic compatibility layer, not in `ec-client`

Recent validation baseline:

- `cargo test -q`
  - workspace green at the last recorded sweep
- latest live oracle probe on `/tmp/ecgame-planet-probe`:
  - successful manual run through main menu -> `Total Planet Database` filter
    -> list -> `Foundation` detail -> exit
  - `DATABASE.DAT` content stayed byte-identical throughout that path
  - `PLAYER.DAT[0x4E]` advanced from `3003` to `3004`
  - follow-up manual list screenshots confirmed that player 1's foreign
    `Helios Prime` row at `(9,2)` is accepted by original `ECGAME` on the
    Total Planet Database list with the exported values:
    owner `#4`, max produx `136`, year seen `3003`, ARs `10`, GBs `6`,
    current produx `136`, stored points `35`, year scout `3003`
  - follow-up detail screenshot confirmed that original `ECGAME` also accepts
    the same row on the planet detail screen, showing:
    owner `Helios Crown (#4)`, max/current produx `136`, stored points `35`,
    armies `10`, ground batteries `6`, and posted year `3003`

## Canonical Docs

Use these first when changing engine behavior:

- [ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md)
- [rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md)
- [ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-combat-spec.md)
- [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
- [economics.md](/home/mag/dev/esterian_conquest/docs/dev/economics.md)
- [ec-movement-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-movement-spec.md)
- [ec-setup-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-setup-spec.md)

For repo structure and workflow, also keep
[approach.md](/home/mag/dev/esterian_conquest/docs/dev/approach.md) and
[rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)
close at hand.

## Current Goal

Keep the SQLite-native runtime/client path stable while recovering the
remaining classic projection rules needed for oracle compatibility and hybrid
DOS playback.

Practical posture:

- treat the current spec docs as the authority for rules/ordering
- only reopen rule recovery when a classic oracle diff, manual reading, or
  reproducible probe shows the Rust rule should move
- keep classic export quirks isolated to the compatibility layer instead of
  reintroducing live `.DAT` dependencies into `ec-client`
- prefer implementation and regression coverage over more deep RE when the
  remaining questions are non-blocking
- do not assume that simply entering Total Planet Database or planet-detail
  screens rewrites `DATABASE.DAT`; current live evidence says otherwise

## Real Blockers

No known oracle/spec blockers remain in the core maint engine. The main
remaining risks are:

- classic `DATABASE.DAT` / intel projection rules for ECGAME-facing views
- determining which foreign-intel row shapes `ECGAME` actually accepts as
  visible list/detail entries
- keeping runtime/client code free of accidental classic-file side effects
- implementation drift between canonical SQLite intel facts and classic export

## Immediate Next Steps

1. Keep oracle probing focused on classic export behavior, especially
   newly granted foreign intel rather than already-known homeworld/detail
   screens.
2. Use the current `/tmp/ecgame-planet-probe` style scenario to probe player
   1's foreign row for planet record `5` (`Helios Prime`, marker `0x23`) and
   treat the list/detail acceptance as a locked positive compat case. Next
   probes should target other row families, not this one again.
3. Keep `ec-client` and normal Rust mutation paths SQLite-native; do not add
   direct `.DAT` ownership back into the client/runtime.
4. When classic tooling changes a directory, fold those edits back through
   `db-import` before the next Rust maint/client step.
5. After meaningful Rust changes, rerun:
   - `python3 tools/oracle_sweep.py --mode seeded`
   - `python3 tools/rust_maint_sweep.py --turns 3`
   - `cargo test -q`
6. Keep `next-session.md` short and current; archive bulky probe history
   instead of rebuilding a running notebook here.
