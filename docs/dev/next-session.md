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
  - patched follow-up partial-intel probe on `/tmp/ecgame-partial-probe`
    confirmed that `Helios Prime` still appears on the list/detail screens when
    the scout payload is cleared:
    list shows `ARs=65535`, `GBs=65535`, current produx `136`, stored points
    `35`, year scout `3003`; detail shows `Armies: UNKNOWN` and
    `Ground Batteries: UNKNOWN`
  - follow-up `ViewWorld` probe on `/tmp/ecgame-view-probe` confirmed that
    original `ECGAME` also accepts the view-only foreign row on both list and
    detail screens:
    list shows `Curr Prod=255`, `Stored Points=65535`, `ARs=65535`,
    `GBs=65535`, `Year Scout=3003`; detail shows `Current Production: UNKNOWN`,
    `Production Points Stored: 65535`, `Armies: UNKNOWN`, and
    `Ground Batteries: UNKNOWN`
  - clearing `raw[0x27..0x28]` to zero did not stop `ECGAME` from showing
    posted/scout year `3003`, so that display year is not fully decoded yet
  - year-split follow-up on `/tmp/ecgame-yearsplit-probe` set
    seen year `3003` but scout year word `2991`; original `ECGAME` still showed
    `3003` on both the list and detail screens
  - year-source follow-up on `/tmp/ecgame-yearsource-probe` set
    seen year `2992` and scout year word `2991` while `CONQUEST.DAT` stayed at
    `3004`; original `ECGAME` showed `2992` on both the list and detail
    screens, so the visible year source is the seen-year words, not the scout
    year word
  - shipped docs plus maint-report regression coverage now confirm that
    `Scout Solar System` reports also include stardock contents; this is proven
    for the mission-report path, not yet for any Total Planet Database
    `Docked:` display field
  - current compat export now keeps `ViewWorld` distinct from
    `ScoutSolarSystem`: view rows expose name/owner/potential without scout
    payload, while scout reports keep the stardock summary
  - `DATABASE.DAT[0x1e..0x1f]` must still be treated as an unresolved compat
    display word: the accepted `Helios Prime` row shows `35` there while the
    same probe directory's `PLANETS.DAT stored_goods_raw()` is `342`
  - the separate planet-command-menu detail path still hits the known
    `Runtime error 201 at 1958:76DE` crash

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
- determining the real semantics of the Total Planet Database display bytes,
  especially `DATABASE.DAT[0x1d]` and `DATABASE.DAT[0x1e..0x1f]`
- determining whether any classic `Docked:` / stardock display path depends on
  `DATABASE.DAT`, `PLANETS.DAT`, or some other compat state
- keeping runtime/client code free of accidental classic-file side effects
- implementation drift between canonical SQLite intel facts and classic export

## Immediate Next Steps

1. Keep oracle probing focused on classic export behavior, especially
   newly granted foreign intel rather than already-known homeworld/detail
   screens.
2. Keep the accepted `Helios Prime` scout row and the accepted `Helios Prime`
   `ViewWorld` row as locked positive compat cases. New probes should target
   other row families, especially combat-granted rows and orbit-record
   preservation.
3. Treat the seen-year words (`raw[0x16..0x19]`) as the visible year source
   for current Total Planet Database list/detail displays. The scout-year word
   (`raw[0x27..0x28]`) remains unresolved but is not driving those screens.
4. Treat `DATABASE.DAT[0x1e..0x1f]` as an unresolved compat display word, not
   as proven `PLANETS.DAT stored_goods_raw()`. Preserve accepted/template word
   families unless a new oracle probe proves a real semantic mapping.
5. If probing stardock beyond the mission-report path, focus specifically on
   whether any Total Planet Database / planet-detail `Docked:` display is
   driven by `DATABASE.DAT`, `PLANETS.DAT`, or some other classic state.
6. Keep `ec-client` and normal Rust mutation paths SQLite-native; do not add
   direct `.DAT` ownership back into the client/runtime.
7. When classic tooling changes a directory, fold those edits back through
   `db-import` before the next Rust maint/client step.
8. After meaningful Rust changes, rerun:
   - `python3 tools/oracle_sweep.py --mode seeded`
   - `python3 tools/rust_maint_sweep.py --turns 3`
   - `cargo test -q`
9. Keep `next-session.md` short and current; archive bulky probe history
   instead of rebuilding a running notebook here.
