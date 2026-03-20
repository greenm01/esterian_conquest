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
  - shipped docs plus the clean bombard-only probe now confirm the negative
    case: bombardment damages a world but does not capture it and should not be
    treated as a new foreign-intel source for Total Planet Database visibility
  - live `ECGAME` review of the `TargetPrime` invade probe exposed a separate
    classic compat issue: Rust-maint report routing into `MESSAGES.DAT` was
    using the wrong on-disk format and produced garbled classic inbox output
  - compat-safe policy is now: keep maintenance reports in `RESULTS.DAT`,
    preserve existing classic `MESSAGES.DAT` unchanged, and keep Rust queued
    mail in SQLite/runtime state until the classic mail format is recovered
  - after removing the bad `MESSAGES.DAT` routing, original `ECGAME` accepts
    the successful-invade `TargetPrime` row on the Total Planet Database list:
    owner `#1`, max produx `100`, year seen `3003`, `ARs=2`, `GBs=0`,
    current produx `100`, stored points `65`, year scout `3003`
  - original `ECGAME` also accepts a distinct failed-invade `TargetPrime` row
    family: owner `#2`, max produx `100`, year seen `3004`, current produx
    `UNKNOWN`, stored points `65535`, and no scout payload (`ARs/GBs UNKNOWN`);
    the old Rust export that exposed defender armies/batteries was rewritten by
    `ECGAME` on login, so failed assault must stay view-only in compat export
  - failed blitz matches the same accepted enemy-view row family as failed
    invade on both list and detail screens, so the shared assault-failure
    compat export is now confirmed by oracle for both mission kinds
  - successful blitz also matches the expected captured-world list row family:
    owner `#1`, max produx `100`, year seen `3003`, `ARs=8`, `GBs=0`,
    current produx `100`, stored points `65`, year scout `3003`
  - selecting that successful-blitz `TargetPrime` entry again routes into the
    normal owned-world report path, not the foreign-intel detail layout
  - owned-world `Docked:` on that report path is driven by planet state, not
    `DATABASE.DAT`: after adding docked ships to `TargetPrime` without changing
    the compat database row, original `ECGAME` showed `Docked: 2 destroyers`
    and `1 ETAC`
  - selecting that owned `TargetPrime` entry appears to route into the normal
    owned-planet report path rather than the foreign-intel detail layout, so
    list acceptance is the stronger compat contract for newly captured worlds
  - corrected scan of `fixtures/ecutil-init/v1.5/DATABASE.DAT` confirmed that
    classic orbit rows are `100`-byte records at viewer/planet pairs
    `(1,15)`, `(2,13)`, `(3,5)`, and `(4,6)`; each row has zero years,
    current production `100`, unresolved display word `35`, and armies/GBs
    `10/4`
  - a clean orbit probe at `/tmp/ecgame-orbit-probe.ygqfkV` now has slot `1`
    prepared as `matched-preloaded-first-login` with alias `SYSOP`, while the
    preserved init-fixture `DATABASE.DAT` orbit rows remain byte-identical
  - current compat export now keeps `ViewWorld` distinct from
    `ScoutSolarSystem`: view rows expose name/owner/potential without scout
    payload, while scout reports keep the stardock summary
  - `DATABASE.DAT[0x1e..0x1f]` must still be treated as an unresolved compat
    display word: the accepted `Helios Prime` row shows `35` there while the
    same probe directory's `PLANETS.DAT stored_goods_raw()` is `342`
  - matched-preloaded first login on the homeworld-seed orbit probe confirmed
    that naming the homeworld does not load `Total Planet Database` yet:
    original `ECGAME` refused the database screen until after maintenance, but
    still allowed the direct owned-world report path
  - original `ECMAINT` then loaded that seed-family row into the database by
    stamping years `3000` onto the same `100 / 35 / 10 / 4` homeworld-seed
    payload; the maintained list row for `(16,13)` showed owner `#1`,
    current/max production `100`, `ARs=10`, `GBs=4`, stored points `35`,
    and year seen/scout `3000`
  - Rust compat export now preserves `DATABASE.DAT[0x1e..0x1f] = 0x23` for
    the named owned homeworld-seed row family; that fixes the only visible
    oracle mismatch found in the Rust-vs-oracle post-maint comparison
  - final live oracle validation on the repaired Rust-generated directory
    confirmed the same visible behavior on both list and detail/report screens;
    the remaining raw diff is only one non-displayed trailing name-area byte
  - patched `Helios Prime` probe proved that `DATABASE.DAT[0x1e..0x1f]` is the
    directly displayed `Stored Points` word on accepted foreign scout rows:
    changing only that word from `35` to `77` made original `ECGAME` show `77`
    on both the list and detail screens without rewriting the row
  - patched `Helios Prime` probe also proved that `DATABASE.DAT[0x1d]` is the
    directly displayed `Current Production` byte on accepted foreign scout
    rows: changing only that byte from `136` to `99` made original `ECGAME`
    show `99` on both the list and detail screens without rewriting the row
  - patched `Helios Prime` probe also proved that `DATABASE.DAT[0x23..0x24]`
    is the directly displayed `Armies` word on accepted foreign scout rows:
    changing only that word from `10` to `17` made original `ECGAME` show `17`
    on both the list and detail screens without rewriting the row
  - patched `Helios Prime` probe also proved that `DATABASE.DAT[0x25..0x26]`
    is the directly displayed `Ground Batteries` word on accepted foreign
    scout rows: changing only that word from `6` to `9` made original
    `ECGAME` show `9` on both the list and detail screens without rewriting
    the row
  - follow-up original-`ECMAINT` probes against a valid zero-turn
    `Helios Prime` campaign (`/tmp/ecgame-regular-preprobe`) still do not
    reproduce a clean foreign regular-world scout refresh:
    - a lone player-1 scout aimed at `(9,2)` repeatedly aborts with
      `Since we have lost all of our scouts`
    - that abort persists after stripping the fleet to one scout, moving the
      other player-1 fleets out of the sector, freezing hostile fleets in
      place, starting the scout one sector away, and matching the accepted
      `max_speed=6/current_speed=3` speed shape
    - fresh paired controls at
      `/tmp/ecgame-fail-regular-speed3-fresh` and
      `/tmp/ecgame-fail-regular-visible-helios-fresh` proved that preloading
      player 1's `DATABASE.DAT` row for `Helios Prime` with the accepted full
      foreign-scout row family does **not** change the abort at all:
      `RESULTS.DAT` is byte-identical in both runs, and the visible row
      survives unchanged through maintenance
    - a follow-up owner-slot probe at
      `/tmp/ecgame-regular-owner2-speed3-fresh` changed only
      `PLANETS.DAT[planet 5].owner_empire_slot` from `#4` to `#2`; original
      `ECMAINT` still emitted the same scout-abort report byte-for-byte
    - a follow-up stardock probe at
      `/tmp/ecgame-regular-nostardock-speed3-fresh` zeroed only the target
      world's stardock counts/kinds; original `ECMAINT` still emitted the same
      scout-abort report byte-for-byte
    - the same runs still advance year/economy and rewrite owned-world
      `DATABASE.DAT` rows, so the directory is valid; the unresolved issue is
      specifically the regular foreign scout path, not general file integrity
  - a separate raw transplant probe from the accepted `TargetPrime` scout
    baseline to a regular-world-shaped target caused original `ECMAINT` to
    perform zero writes at all, so raw planet-record substitution is too
    integrity-sensitive to use as the next `0x1e..0x1f` oracle path
  - a fresh headless Ghidra import of the on-disk packed binary into local
    project `ec-v15-local` confirmed that the scout-abort text fragments are
    not present in `original/v1.5/ECMAINT.EXE`, so the packed stub is not a
    useful string anchor for this path
  - a prompt-less DOSBox-X guest-RAM capture now works for the failing scout
    case: running the same directory with
    `-set 'memory file=/tmp/ecmaint-scout-abort.mem'` and carving
    `guest_ram[0x8140 : 0x8140 + 0x97eb0]` produced the usable live dump
    `/tmp/ecmaint-scout-abort-psp.MEMDUMP.BIN`
  - that carved live dump contains the scout-abort string anchors at raw
    offsets `0x5adc` / `0x6c06` (`Since we have lost all of our scouts`) and
    `0x248ff` (`Scouting mission report`); artifacts for this pass live under
    `artifacts/ghidra/ecmaint-scout-live/`
  - raw near-call scan plus ndisasm on the live dump recovered a shared
    mission-kind dispatcher at `0000:8a11`:
    `[0x3521] = 0x0b -> 5c18 -> 6817`,
    `[0x3521] = 0x0a -> 6c9d -> 6dda`,
    `[0x3521] = 0x0e -> 841a -> 8584`
  - raw write-site scan tightened more of the shared setup state:
    `0x350d/0x350e` are written at `0000:0c7a/0ca4` inside the same block that
    materializes `0x351b..0x351f`, while `0x3534` is zeroed at `0000:f941`
    and incremented at `0000:f99b` inside `0000:f914`, which also zeroes the
    sibling counters `0x3528/0x352a/0x352c/0x352e/0x3530/0x3532` and resets
    `0x3521` to `0` at `0000:f9cf`
  - live helper recovery also shows that both scout helpers gate on
    `word [0x3534]` and share the `0x350d/0x350e` vs `0x3522/0x3523`
    predicate, but only `0000:5c18` does the extra target-owner lookup through
    `ES:[DI + 0x5d]`
  - even after rebuilding and `make install`-ing the debug DOSBox-X binary,
    the pexpect-driven debugger prompt still does not surface `CS=`/register
    output in this environment; use the `memory file` fallback instead of the
    interactive prompt when you need a new local runtime dump
  - do not use the `foot` terminal emulator for DOSBox-X debugger work here:
    it can hide or fail to flush the live debugger stop/register prompt output
    even when DOSBox-X is waiting for input, which makes both manual and
    scripted dump capture paths appear hung
  - the current scout-abort RE blocker is no longer dump capture itself; it is
    tracing who primes `0x3521`, `0x3534`, `0x350c`, and the
    `0x350d..0x3524` target tuple before `0000:8a11` dispatches into the
    mission-specific helper
  - current classic probe crash triage on the separate planet-command-menu
    detail path found that `setup_classic_probe_game.py` intentionally seeds
    `Aurora Prime` as player 1-owned (`PLANETS.DAT` record `16`, owner slot
    `1`), so the lingering confusion there is not a foreign-row leak
  - that same probe harness now rewrites the extra player-1 colonies at
    records `16`, `17`, and `19` to a stable established-world
    `PLANETS.DAT` shape before `db-import` instead of relying only on sparse
    `planet-owner` / visible-field mutators; raw regeneration confirmed the
    target-world template bytes landed and later stardock writes still apply
  - that rewrite path now does `db-export <dir> <dir>` first so the direct
    `PLANETS.DAT` patch starts from the live joined-player snapshot instead of
    the stale first-login files left on disk; stage tracing confirmed this
    stops `db-import` from clobbering `PLAYER.DAT` back to civil-disorder
  - the same harness no longer parks player-1 fleet `3` on `Aurora Prime`;
    it now moves to empty sector `(5,4)` so the crash probe does not pile an
    orbiting fleet onto the same owned world already carrying docked ships
    while the owned-world space-force formatter is still under triage
  - live-dump disassembly now narrows the crash site itself: the
    `1958:76DE` runtime error aliases to the earlier `Space Forces:` formatter
    loop, not the later `Building:` / `Docked:` emitters
  - corrected-probe raw inspection on `/tmp/ec-classic-probe-verify` shows all
    four player-1 owned worlds have `raw[0x03]=0x87`, no fleets on their
    coordinates, and no starbases at all, so the old simple
    `Aurora Prime` overlap theory is no longer sufficient if the crash still
    reproduces
  - deeper field-offset inspection of the same `Space Forces:` routine found a
    stronger harness bug: the rewritten extra player-1 worlds had inherited
    `PLANETS.DAT[0x1d..0x23] = 05 1d 0b 11 25 1c 05`, which left
    `word [0x22..0x23] = 0x051c` while every ordinary owned world in the same
    probe had `0x0000` there
  - `setup_classic_probe_game.py` now zeros `0x1d..0x23` for records `16`,
    `17`, and `19`; fresh regen on `/tmp/ec-classic-probe-word22` confirmed
    those worlds now match the ordinary owned-world shape at `0x22..0x23`
    while classic login state still stays correct
  - the same harness now exposes an explicit Aurora Prime stardock bisect:
    `--aurora-stardock {busy,empty,single-dd}`
  - the same harness now also supports
    `--probe-dock-host {aurora,foundation}` so the exact same owned-world dock
    payload can be moved onto `Foundation` without changing the rest of the
    probe baseline
  - verified variants:
    - `/tmp/ec-classic-probe-word22` = busy reference (scout + two destroyers)
    - `/tmp/ec-classic-probe-aurora-empty` = empty Aurora dock
    - `/tmp/ec-classic-probe-aurora-single` = one destroyer in slot `0`
    - `/tmp/ec-classic-probe-foundation-busy` = same busy payload moved to
      `Foundation` while `Aurora Prime` stays empty
  - all four variants keep slot 1 as a valid `returning-player`, so the next
    manual `P -> D` check can isolate whether the crash follows Aurora itself
    or simply follows the busy owned-world dock payload
  - the separate planet-command-menu detail path still hits the known
    `Runtime error 201 at 1958:76DE` crash, but the next manual recheck should
    compare `/tmp/ec-classic-probe-aurora-empty`,
    `/tmp/ec-classic-probe-aurora-single`,
    `/tmp/ec-classic-probe-word22`, and
    `/tmp/ec-classic-probe-foundation-busy` because the old `0x051c`
    gate-word bug is fixed and the remaining question is whether the crash
    follows the owned-world dock payload or remains Aurora-specific

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
  especially when `ECMAINT` refreshes or preserves cached values such as
  `DATABASE.DAT[0x1d]` and `DATABASE.DAT[0x1e..0x1f]`
- determining what original `ECGAME` does with the preserved init-fixture
  orbit-row family (`raw[0x15] in 0x01..0x04`)
- keeping runtime/client code free of accidental classic-file side effects
- implementation drift between canonical SQLite intel facts and classic export

## Immediate Next Steps

1. Keep oracle probing focused on classic export behavior, especially
   newly granted foreign intel rather than already-known homeworld/detail
   screens.
2. Keep the accepted `Helios Prime` scout row, `ViewWorld` row, assault
   success/failure row families, and maintained homeworld-seed row family as
   locked compat cases.
3. Treat the seen-year words (`raw[0x16..0x19]`) as the visible year source
   for current Total Planet Database list/detail displays. The scout-year word
   (`raw[0x27..0x28]`) remains unresolved but is not driving those screens.
4. Treat accepted foreign scout rows as directly display-driven for at least:
   `raw[0x1d]` = current production, `raw[0x1e..0x1f]` = stored points,
   `raw[0x23..0x24]` = armies, and `raw[0x25..0x26]` = ground batteries.
   These are not yet proven to equal canonical `PLANETS.DAT` values in
   general, so preserve accepted/template values unless a row-family-specific
   oracle probe proves a semantic mapping.
5. Treat owned-world `Docked:` as closed: it comes from planet state, not
   `DATABASE.DAT`. The remaining orbit-row work is now mainly about whether
   any non-owned/foreign-intel display path reuses the same `0x23` family.
6. Treat successful `ScoutSolarSystem` refreshes as rewriting stale
   visible scout-row payload, but not yet as a fully decoded semantic rebuild.
   The clean oracle proof is `/tmp/ecgame-scout-refresh-row34.QYjsVJ`, where a
   stale visible row (`Curr Prod = 44`, years `2999`) was refreshed by original
   `ECMAINT` to `Curr Prod = 100`, seen/scout years `3010`.
   Follow-up `/tmp/ecgame-scout-refresh-arbg.SnZG9j` also showed stale
   `ARs/GBs` (`17/9`) refreshing to live `142/15`.
   Follow-up `/tmp/ecgame-scout-refresh-word1e.kawp4C` showed the same clean
   scout path rewriting a stale visible `0x1e..0x1f` word (`77`) back to the
   row-family value `66`.
7. Treat scout acceptance as sensitive to classic fleet pre-state. A clean
   original-ECMAINT scout run was only reproduced with an at-rest pure scout
   fleet (`tupleA/B = 0x80...`, `tupleC = 0x81...`, no attached DDs, no same-
   sector merge partner) in `/tmp/ecgame-classic-atrest-purescout.gMcRea`.
   The older synthetic `Helios Prime` probes were failing because they mixed
   transit-family tuple bytes and other scenario noise into the scout order.
8. The `0x1e..0x1f` word for successful scout rows remains row-family-specific.
   Do not generalize it to live `PLANETS.DAT` stored goods yet:
   - regular `Helios Prime` scout rows still accept `35`
   - but that `35` family is still only `ECGAME`-accepted, not yet cleanly
     reproduced from original `ECMAINT` on a valid regular-world scout probe
   - the successful unknown->visible `TargetPrime` scout row in the classic
     homeworld-style fixture came out as `0x42` (`66` displayed)
   - the clean `word1e` refresh probe proves original `ECMAINT` can rewrite a
     stale visible `0x1e..0x1f`, but the rewritten value is still not proven to
     come from canonical live stored goods
   - current compat policy should therefore refresh stale scout `0x1d`, and
     keep treating `0x1e..0x1f` as a row-family-specific compat word until a
     tighter oracle rule exists
9. Treat the remaining regular-world scout-abort gate as **not** explained by:
   pre-existing `DATABASE.DAT` visibility state, simple target owner-slot
   identity (`#4` vs `#2`), or Helios Prime's populated stardock bytes. Those
   probes all preserved the exact same abort report in original `ECMAINT`.
10. The on-disk packed `ECMAINT.EXE` is not a useful string anchor for the
    scout-abort path. Headless Ghidra on local project `ec-v15-local` found no
    matches for `Scouting mission report`, `Since we have lost`, or
    `abort our mission`; use the live dump path instead of the packed EXE stub.
11. Use DOSBox-X `memory file` rather than the interactive debugger prompt for
    local scout-abort dump capture here. The reliable carve is:
    `guest_ram[0x8140 : 0x8140 + 0x97eb0] ->
    /tmp/ecmaint-scout-abort-psp.MEMDUMP.BIN`.
12. Treat `0000:8a11` as the current upstream live anchor for this RE thread:
    `[0x3521] = 0x0b -> 5c18 -> 6817`,
    `[0x3521] = 0x0a -> 6c9d -> 6dda`,
    `[0x3521] = 0x0e -> 841a -> 8584`.
13. Next RE should trace the call path into:
    `0000:0c7a/0x0ca4` for the `0x350d..0x351f` target-state tuple,
    `0000:f914..0xf9cf` for the `0x3534` counter family and `0x3521` reset,
    and then the later write sites that raise `0x3521` from `0` to the
    mission-kind values consumed by `8a11`.
14. Keep `ec-client` and normal Rust mutation paths SQLite-native; do not add
   direct `.DAT` ownership back into the client/runtime.
15. Keep the distinction explicit in docs/tests:
   - `ECGAME`-accepted row shapes are not automatically original-`ECMAINT`
     emitted row shapes
   - the regular-world foreign scout family is still missing a clean oracle
     maint proof
16. When classic tooling changes a directory, fold those edits back through
    `db-import` before the next Rust maint/client step.

## Combat System Status

Recent combat implementation work (phases 1-3):

- **Screen-then-kill hit allocation** (lines 256-279 of `combat.rs`): Fresh steps
  removed from all classes before any hull destruction. Prevents the degenerate
  waterfall where DDs die first.

- **Pursuit fire** (lines 1305-1377 of `combat.rs`): Guard/blockade fleets can
  intercept fleeing fleets that declined ROE. Pursuer fires at flat CER 0.50,
  withdrawer fires at normal CER. Losses are now properly tracked using
  pre/post-combat state capture.

- **Guard free-hold** (lines 1207-1261 of `combat.rs`): Defending guard/blockade
  task forces get one free hold when ROE threshold fails in post-round check.
  They stay and fight one more round before considering retreat on subsequent
  failures.

- **ROE retreat scope** (narrowed): `apply_roe_retreat_to_task_force` now takes
  `fleet_indices: &[usize]` instead of empire+coords, ensuring only the actual
  retreating task force members retreat.

Validation:

- All 12 oracle sweeps pass
- All 8 maint sweeps pass
- All Rust tests pass

Note on pursuit fire design tension: Guards stay at station for innocent transit
fleets (MoveOnly, Patrol, etc.). The pursuit fire mechanic only triggers when an
attacker in assault posture (Invade/Bombard/Blitz) at a defended world declines
ROE. Deep space encounters with transit fleets result in clean coexistence, not
combat or pursuit fire.

17. After meaningful Rust changes, rerun:
   - `python3 tools/oracle_sweep.py --mode seeded`
   - `python3 tools/rust_maint_sweep.py --turns 3`
   - `cargo test -q`
18. Keep `next-session.md` short and current; archive bulky probe history
   instead of rebuilding a running notebook here.
