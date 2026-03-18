# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](next-session-archive.md),
not here.

## Current State

`rust-maint` is now end-to-end capable in a conservative, manual-faithful way.

The maintenance engine is also now on much firmer authority footing:

- major player-authored inputs are validated in shared `ec-data`, not trusted
  from the client
- malformed player state is sanitized and reported during `maint-rust` instead
  of being silently executed
- `core-validate` now audits gameplay-invalid player input, not just structural
  linkage
- deterministic malformed-directory stress coverage now exists at both
  `ec-data` and `ec-cli` layers

Current grade:

- maintenance engine authority / invalid-input resistance: `A+`
- maintenance engine behavior against `ECPLAYER.DOC`: `A+`
- overall `rust-maint` status: `A+`

Local development baseline:

- Rust builds already use Cargo's normal multi-core job scheduling by default
- `sccache` is now the recommended local compile-speed dependency
- do not treat `mold` as a required repo dependency; keep it optional/local

Latest oracle signal against the remaining manual-adjacent fleet assumptions:

- confirmed in classic `ECMAINT`:
  - `Seek Home` dynamically retargets when the nearer refuge is lost
  - `Guard a Starbase` follows a moved base
  - invalid guard-starbase linkage aborts with `ERRORS.TXT`
  - patrol/contact reports include actionable hostile composition
  - battle-loss reports include observed enemy composition and enemy losses inflicted
  - owned-world `Salvage` succeeds from a live classic probe:
    - the fleet moves to the owned world
    - the fleet is removed on arrival
    - classic reports an estimated recovered production yield
  - salvage failure at non-owned targets aborts and seeks home
  - `Join another fleet` hot pursuit is now confirmed from a player-authored
    classic `ECGAME` + `ECMAINT` probe:
    - `ECGAME` stores the host fleet number in mission aux and snapshots the
      host's current coordinates
    - later `ECMAINT` turns refresh the joiner's target to the host's new live
      location
    - on arrival, the host absorbs the joining fleet
  - surviving retreat after fleet combat is now confirmed from a player-authored
    classic bombardment probe:
    - the surviving fleet aborted its mission
    - switched to a seek-home retreat
    - reported enemy composition, enemy losses inflicted, own losses, and the
      named retreat destination
- confirmed known classic defect:
  - empty-sector salvage reuses the wrong failure text
    (`Since we no longer own the world...`) even when no world exists there
- confirmed in live `ECGAME` login probing:
  - `fixtures/ecmaint-fleet-battle-pre/v1.5` is maint-valid but not a valid
    returning-player client fixture when the persisted handle does not match the
    caller/dropfile identity:
    - classic enters the first-time menu
  - changing only the persisted slot-2 handle from `FOO` to `SYSOP`
    (matching the generated `CHAIN.TXT` alias) is enough to flip classic into
    a matched pre-loaded-player path
  - that matched path is distinct from both the first-time menu and the normal
    established-player login:
    - intro pages
    - one-time empire rename prompt
    - status screen
    - report/message review
    - homeworld naming
    - then `MAIN MENU`
  - `ec-cli inspect-classic-login <dir> <caller_alias>` now reports the
    compatibility-layer classification Rust expects for each slot:
    `first-time-menu`, `matched-preloaded-first-login`, or
    `returning-player`
  - `ec-cli classic-login-prepare <dir> <player_record> <caller_alias>
    [empire_name]` now provides a narrow local-probe helper that aligns the
    persisted player handle with the caller alias without changing broader
    gameplay state

It can currently:

- create new classic-compatible games across the documented `4 / 9 / 16 / 25`
  player tiers
- run repeated Rust maintenance turns over full campaign state
- handle movement, economy, scouting, contact reporting, diplomacy,
  deterministic combat, conquest, civil disorder, fleet defection, and
  conservative emperor recognition
- regenerate classic `DATABASE.DAT` and `RESULTS.DAT`
- write a first-pass routed `MESSAGES.DAT` stream from recipient-scoped maint
  events
- preserve existing classic player-mail `MESSAGES.DAT` payloads during
  `rust-maint` when no routed maint messages are emitted
- keep producing directories the original `ECMAINT` accepts
- generate default `sysop new-game` directories as joinable `ECGAME` starts
  again:
  - inactive player slots
  - `Not Named Yet` homeworld seeds
  - pre-join fleet blocks at seeded homeworld coords
- keep the older post-join active campaign baseline available through
  `setup_mode="builder-compatible"` for maint/oracle sweeps and test fixtures
- support a documented local hybrid campaign loop:
  - Rust creates the campaign
  - classic `ECGAME` launches against the same working directory
  - `classic-login-prepare` can align a local caller alias with a persisted
    player handle for matched probes
  - `maint-rust` advances the same directory and reprojects classic files back
    into place

Recent validation:

- `python3 tools/oracle_sweep.py --mode seeded`
  - current result: `12/12` zero-diff `ECMAINT` oracle passes across
    `4/9/16/25` players and seeds `1515/2025/4242`
- `python3 tools/rust_maint_sweep.py --turns 3`
  - current result: `8/8` passes across `4/9/16/25` players and seeds
    `1515/2025`
- `cargo test -q`
  - current workspace status: green

## Current Goal

Primary goal:

- keep `rust-maint` honest as a full-game engine by continuing repeated oracle
  validation against classic `.DAT` output
- refine only where stronger manual evidence or original-binary evidence shows
  the Rust rule should move
- shift the next major implementation phase toward cloning `ECGAME` in Rust on
  top of the new Rust-native SQLite campaign store
- for the active `ECMAINT` turn-cycle RE thread, the completion target is
  explicit:
  fully recover the complete week-assignment and cross-turn fleet-behavior
  process well enough to call that oracle behavior fully recovered, not merely
  approximated

Highest-value remaining `ECMAINT` oracle RE targets:

1. canonical middle turn order
2. weekly event assignment rules inside the `1..52` yearly timeline
3. summary/event record format plus late weekly emission pipeline
4. report routing / recipient policy across `RESULTS.DAT`, `MESSAGES.DAT`,
   and rankings output
5. economy / production application timing

Current best workflow for target `1`:

- start with controlled one-mechanic oracle scenarios, not broad static
  disassembly
- diff persistent `.DAT` state and durable summary/event outputs after classic
  `ECMAINT`
- always inspect report/output files too:
  `RESULTS.DAT`, `MESSAGES.DAT`, `ERRORS.TXT`, `DATABASE.DAT`,
  `RANKINGS.TXT`
- use those repeated mutations to choose the next static seam
- treat the partially recovered `1000:03ff..0d53` owned-planet body as the
  current strongest step-4 candidate until a better earlier driver appears
- when the one-mechanic probes plateau, switch to top-down driver recovery:
  - target the earlier startup/token/driver path, not the already-bounded
    `861d` late tail
  - treat `861d` only as an upper boundary fence for the missing middle
  - use dynamic tracing only when it is aimed at those earlier seams
- use DOSBox file-I/O traces only as coarse support, not proof of exact
  movement/economy/combat ordering
- avoid spending more time in already-bounded late summary/report families
  unless a scenario diff points back there

Current Rust-facing implication:

- keep treating `rust-maint` as three distinct boundaries:
  - early structural validation / restore framing
  - yearly state mutation plus summary/event creation
  - late summary canonicalization, report emission, and derived-file rebuild
- do not currently collapse validation-time `5ee4` summary-like entries into
  the same Rust structure as the later durable report-event pool:
  - `5ee4` increments `0x2f76`
  - tail `0x6ac3` then clears `0x2f76` before returning
  - current best reading is "shared workspace, different lifetime"
- do now treat durable report-event creation as a separate later phase:
  - first confirmed non-`5ee4` writers are `1000:dddb` and `1000:e31b`
  - they append fresh `0x0c` entries after the `5ee4` scratch clear
  - they write the later-consumed kind bytes directly:
    - `1000:dddb` -> kind `1`
    - `1000:e31b` -> kind `2`
- do not keep tuning Rust gameplay order against the already-recovered late
  `5ee4` / `6d9b` / `8652` machinery; the remaining ordering risk is now more
  likely in earlier simulation helpers
- but do now treat `1000:024d` as a mixed yearly producer pass rather than a
  pure report helper:
  - its front half still matches the known durable kind-`2` producer family
  - its deeper interior `1000:03ff..0d53` is inside that same function, not a
    separate hidden driver
  - that interior iterates owned planets from `0x1712`, mutates live planet
    fields including `+0x58/+0x5a` and real triples at `+0x03..+0x0d`, and
    consults staged player state plus durable kind-`2` entries before looping
  - practical implication:
    `00e8/024d` now looks like a real bridge between step-4 planet mutation
    and durable event creation
- direct black-box tightening on the same pass now points at planet `+0x60` as
  the strongest current selector candidate inside that deeper `024d` interior:
  - on `fixtures/ecmaint-econ-pre/v1.5`, forcing target-world `+0x60 = 1`
    caused direct mutation of that same world's `+0x03..+0x0e` block after two
    classic maint ticks
  - forcing `+0x5c = 0` or `1` only normalized it back to `2` and perturbed
    neighboring econ-world outcomes
  - forcing `+0x5a = 0` alone did not activate the same deep rewrite
  - practical implication:
    do not currently treat `+0x5a` as the main selector for the deep `024d`
    planet path; `+0x60` is the stronger raw candidate
  - follow-up on the maintenance-stable `ecmaint-post` baseline confirms the
    same thing:
    `+0x60 = 1` alone still triggers the direct `+0x03..+0x0e` rewrite on that
    world, while `+0x5c = 0` only normalizes back to `2`
  - preserved-fixture sweep so far shows `+0x60 = 0` everywhere in the sampled
    corpus, so this looks like a latent branch byte rather than a commonly
    exercised visible world-state flag
  - nearby cofactor check on the same stable baseline did not reproduce the
    branch through plausible neighboring fields alone:
    `+0x0e`, `+0x58`, `+0x5a`, and simple combinations stayed inert unless
    `+0x60` was also forced
  - ownership/status follow-up narrows the preconditions further:
    - forcing `+0x60 = 1` on multiple owned worlds across different empires
      consistently triggered the same broad `+0x03..+0x0e` rewrite
    - forcing it on an unowned world did not
    - forcing `+0x5c = 0` or `1` together with `+0x60 = 1` still triggered the
      rewrite, then normalized `+0x5c` back to `2`
    - practical implication:
      current best raw gate is "owned world plus `+0x60 != 0`"
  - but direct promotion of an originally unowned world tightens that again:
    - setting only owner/status and the visible owned-world bytes
      (`+0x0e`, `+0x58`, `+0x5a`, `+0x5c`, `+0x5d`) was still not enough to
      reproduce the full deep rewrite
    - a near-full clone of an established owned-world record was enough
    - practical implication:
      `+0x60` is a real gate, but the branch also depends on a richer
      established-world payload that is still partly hidden in `+0x03..+0x0d`
  - direct payload bisect now reduces that hidden prerequisite further:
    - lower block `+0x03..+0x08` and upper block `+0x09..+0x0d` can drive
      matching lower/upper halves of the same-world rewrite independently
    - byte `+0x09` is already enough to activate the upper-half rewrite shape
      at `+0x09..+0x0e`
    - copying `+0x03..+0x09` together reproduces the full broad rewrite
    - practical implication:
      the deeper `024d` path appears to consume two coupled world numeric
      groups, not one undifferentiated opaque payload
  - mixed combat-order probe now gives the first direct timing hint:
    - forcing target-world `+0x60 = 1` in preserved invasion and bombardment
      fixtures causes the deep `+0x03..+0x0e` rewrite to begin on tick `1`
      while `RESULTS.DAT` is still empty
    - practical implication:
      at least some `024d`-side planet mutation can land earlier in step `4`
      than later visible combat/mission consequences
  - new reusable direct-probe harness now exists at:
    `tools/step4_oracle_probe.py`
    - clones a disposable working directory
    - applies direct planet-byte edits
    - snapshots each maint tick
    - summarizes watched-world deltas plus
      `RESULTS.DAT` / `MESSAGES.DAT` / `ERRORS.TXT` / `DATABASE.DAT` /
      `RANKINGS.TXT`
    - practical implication:
      keep using this harness for step-`4` timing work rather than ad hoc
      manual diff loops
  - first harness-driven per-tick shape comparison now tightens the earlier
    timing result:
    - `invade-pre` with forced target `+0x60 = 1` rewrites the watched world
      broadly on tick `1` while `RESULTS.DAT` is still empty
    - `bombard-pre` also rewrites the watched world on tick `1` with
      `RESULTS.DAT` still empty, but in a narrower early shape
    - `fleet-battle-pre` changes the watched world on tick `1` when
      `RESULTS.DAT` is already non-empty, and the world-delta shape differs
      again
    - practical implication:
      the remaining step-`4` puzzle is likely not just "one producer pass plus
      one generic report delay"; mission-family context appears to change which
      portion of the planet-local rewrite has landed by a given yearly tick
  - preserved-control comparison without forced `+0x60` narrows that further:
    - `invade-pre` and `fleet-battle-pre` naturally change the same target
      world bytes on tick `1`:
      `+0x09`, `+0x0e`, `+0x38`, `+0x3c`
    - `bombard-pre` leaves the watched target world unchanged across its
      preserved ticks
    - practical implication:
      those shared invade/fleet-battle bytes are now the best current
      candidate for natural mission/combat-side target-world consequences,
      while the extra lower/upper world-block writes exposed by forced `+0x60`
      look more like a separate producer/mutator family
  - direct forced-vs-control overlay on the same tick tightens that again:
    - the forced `+0x60` branch does not merely add extra writes beside the
      natural target-world pattern
    - it can overwrite the natural tick-`1` bytes at `+0x09` and `+0x0e`
    - in the invade case it also suppresses the control-side `+0x38/+0x3c`
      marks while adding broader lower/upper world-block writes and `+0x58`
    - practical implication:
      step `4` currently looks like overlapping neighboring subphases that can
      write some of the same world-state fields, not clean isolated passes
  - target-world seed transplant now narrows the natural aftermath side too:
    - `invade-pre` and `fleet-battle-pre` share the same starting target-world
      record, while `bombard-pre` uses a weaker different seed
    - transplanting the weaker bombard-style target-world record into either
      `invade` or `fleet-battle` collapses the natural tick-`1` world delta
      from `+0x09/+0x0e/+0x38/+0x3c` down to `+0x09/+0x0e/+0x58`
    - practical implication:
      the natural target-world aftermath shape depends strongly on target-world
      payload/class, not only on mission family or whether report output is
      already active
  - inverse transplant into `bombard-pre` closes the other half:
    - transplanting the stronger `invade` / `fleet-battle` target-world seed
      into `bombard-pre` still leaves the watched target world unchanged across
      both preserved ticks
    - practical implication:
      target-world payload/class alone is not enough; the natural aftermath
      shape depends on both hostile context and target-world payload/class

Latest static tightening on the turn-cycle side:

- `2000:87f4 -> 2000:8b15` is now better classified as a late summary
  coalescing pass:
  - it walks the summary table at `0x2f72` / `0x2f76`
  - pairs kind-`2` entries against kind-`1` entries on owner/coords/flag keys
  - then feeds late text/output helpers
- practical implication:
  do not keep treating that region as a candidate gameplay-core phase; the
  missing yearly simulation order is increasingly likely to sit earlier than
  the `861d` late tail or behind helpers that populate the summary pool
- `2000:9e1e` is now better classified as the summary-pool initializer:
  - records startup time at `0x34fa/0x34fc`
  - zeroes summary count `0x2f76`
  - allocates `0xfa00` bytes via `2000:9b13`
  - stores the pool pointer at `0x2f72/0x2f74`
- practical implication:
  the late weekly/report machinery is consuming a workspace seeded early in
  startup, not inventing it only at the end
- `2000:5ee4` is now better bounded internally:
  - front half stages `0x3278` (`0x6e` records) into `0x16ac` / `0x16ae`
  - then stages `0x2f78` (`0x61` records) into `0x1712` / `0x1714`
  - the direct summary emitters still visible inside the function remain:
    - `0x3178` fleet
    - `0x2ff8` base
  - `0x31f8` IPBM
  - tail `0x6ac3..0x6b74` zeros `0x2f76`, frees the staged player/planet
    buffers, and returns
- practical implication:
  player-side and planet-side collections are currently best modeled as staged
  validation / lookup inputs for the known fleet/base/IPBM summary producers,
  not as additional direct summary kinds hidden in the `5ee4` tail; and the
  `5ee4`-time `0x2f76` entries themselves are increasingly likely to be
  transient validation scratch rather than the final late-report event pool
- later durable summary production is now anchored more concretely:
  - `1000:dddb` / `1000:e09d` emits kind-`1` pool entries
  - `1000:e31b` / `1000:e569` emits kind-`2` pool entries
  - both allocate `0x0c` records and fill owner / coords / payload fields in
    the shared pool after `5ee4` has already cleared the validation scratch
- the first recovered ordering between those durable producers is now concrete:
  - sibling drivers `1000:00e8` and `1000:024d` both call `1000:f71d` first
  - `1000:f71d` reaches the kind-`1` writer through `1000:f8a9 -> 1000:dddb`
  - only after that do the same drivers call `1000:e31b` for kind-`2` entries
  - practical implication:
    preserve durable producer-pass ordering in Rust instead of collapsing kind
    creation into one unordered post-pass
- `2000:6d9b` is now better bounded as restore/validation scaffolding:
  - `arg=0` goes through `0x6f20`, calls `5ee4`, and on failure emits
    recovery/error text before recursively calling `6d9b(arg=1)`
  - `arg=1` brackets `5ee4` with two `0x3000:4f4c` registration waves over
    the stream anchors rooted at `0x2f78`, `0x2ff8`, `0x3078`, `0x30f8`,
    `0x3178`, `0x31f8`, `0x3278`, `0x32f8`, and `0x3478`
- practical implication:
  `6d9b` is looking more like integrity/restore wrapper logic around `5ee4`
  than a hidden gameplay-core phase; the missing middle turn order is still
  more likely earlier than the fixed late `8652` chain or inside helpers not
  yet split out
- timing-flow follow-up produced one stronger static lead:
  - `2000:945b` still looks like token/schedule date text formatting, not the
    player-report `Stardate` emitter
  - but the explicit late weekly loop at `0000:12ef..1369` calls
    `1000:a26e`, which now looks like a per-entry timing-bucket helper
  - `1000:a26e` walks a `0x0a`-byte local table, reads a small code byte from
    each entry, and applies fixed offsets to a local accumulator:
    `+2`, `+7`, `+0x15`, `+0x1e`
  - practical implication:
    `1000:a26e` is currently the strongest static candidate for mapping
    summary entry class to week-offset / timing-window constraints inside the
    late `1..52` scheduler
  - follow-up probes now tighten that into a three-stage late timing path:
    - `0000:02c0` decodes kind-`1` summary entries through `2000:c067` and
      seeds large stack-resident local timing state
    - `1000:9fa1 / 1000:a26e` derives two timing-window families from a local
      `0x0a`-byte code table using fixed offsets like `+2`, `+7`, `+0x15`,
      and `+0x1e`
    - `1000:c102 / 1000:9c0e` then scores/tests the current weekly candidate
      against those windows and raises a rejection flag when the slot falls
      outside the computed range
  - practical implication:
    the late scheduler now looks like explicit week-placement logic, not just
    decorative timestamp formatting or a flat offset lookup

Combat policy for the Rust clone remains:

- do not chase original combat RNG parity
- do keep deterministic Rust combat embedded in the oracle-backed:
  - turn order
  - weekly timing
  - follow-on consequence sequencing
  - late report/output pipeline

## What Is Settled

- manuals are the semantic authority
- original DOS binaries are the compatibility oracle
- `.DAT` remains the compliance boundary
- hidden or stochastic original behavior may be reimplemented canonically if
  the result remains faithful to the manuals and stays classic-compatible
- deterministic Rust combat is the chosen canonical replacement for opaque
  original combat RNG
- `ECGAME` local DOSBox launch is now documented and working with the corrected
  local-console `CHAIN.TXT` settings in
  [`docs/dosbox-workflow.md`](dosbox-workflow.md)
- planet economy now has an explicit canonical Rust rule where the original
  replay oracle is still awkward to probe directly:
  - empire-wide tax sets yearly revenue on every owned planet
  - lower tax accelerates current-production growth toward potential
  - taxes above `65%` can now directly reduce present production
  - starbases boost growth and build capacity
  - civil-disorder baselines are left alone so preserved maint fixtures stay
    stable
- the canonical economy rule is now documented in
  [economics.md](economics.md)
- builder-generated starts now encode the intended opening economy directly:
  - homeworld current production starts at `100`
  - default empire tax starts at `50%`
  - when a player joins a fresh slot, the claimed homeworld now starts with the
    opening spendable production implied by the manuals: `50` stored points at
    the default `50%` tax rate on `100` present production
  - canonical initialized homeworlds start with `10` armies and `4` batteries
- a focused original-`ECMAINT` probe now shows that letting a ship build complete
  into a full stardock is unsafe:
  - the build slot clears
  - no `ERRORS.TXT` is emitted
  - the target planet's stardock bytes are corrupted
  - Rust now keeps blocked ship/starbase builds queued unchanged until a
    stardock slot opens, while armies and batteries still complete normally
  - keep the Rust client-side stardock-capacity guard in place
- a focused original-byte-limit probe now shows:
  - planet armies at `255` stay at `255` and still consume a completing army build
  - planet batteries at `255` stay at `255` and still consume a completing battery build
  - a simple scout-fleet merge probe is not a clean overflow oracle because
    classic merge processing appears to drop merged-away scouts even below `255`
  - keep the Rust planet unload cap guard in place for now
  - the exact original `ECGAME` load/unload UI behavior above `255` is still
    worth a stronger screen-aware probe later
  - Rust now diverges intentionally on the planet-side byte caps:
    - army/battery builds that would overflow stay queued
    - unload to a full planet is rejected cleanly in the client and engine

## Biggest Remaining Engine Questions

- player-facing production semantics are not fully decoded yet:
  - original `ECGAME` exposes `Present Production`, `Potential Production`,
    `Total Available Points`, and empire/planet production rankings
  - Rust still has raw/RE-facing economic field names like `factories` for
    underlying Borland Pascal `Real` storage
  - next engine/UI alignment work should decode and expose the original
    production semantics instead of leaking raw field names into client screens
- `PLANETS.DAT raw[0x0E]` is not a settled planet-tax field:
  - mixed-tax Rust probes show it being overwritten during the existing
    autopilot/rogue AI path
  - do not treat `planet_tax_rate_raw()` as a stable player-facing semantic
    field after maintenance until that byte is fully decoded
- fleet numbering now has an important split to preserve:
  - preserved `ECGAME` logs strongly suggest the displayed `Nth Fleet` number is
    per-empire
  - the shipped active `original/v1.5/FLEETS.DAT` also shows per-owner local
    slots alongside globally unique structural fleet IDs, so those two fields
    should stay distinct in the Rust model
  - the current recovered structural fleet-chain model still treats
    `FLEETS.DAT record[0x05]` as a separate global linkage key
  - keep player-facing fleet numbering and structural fleet linkage distinct
    until deeper oracle evidence proves they are the same field
- emperor-recognition details may still need refinement if stronger classic
  evidence appears
- fleet-defection cadence is currently conservative and deterministic, not
  proven byte-for-byte original behavior
- report wording and visibility can still be tightened when new `ECGAME` or
  manual evidence appears
- the newest oracle pass closed the remaining fleet/manual uncertainty:
  - `Seek Home`, `Guard Starbase`, `Join another fleet`, patrol contact intel,
    salvage success/failure semantics, and surviving retreat/abort reporting now
    all have direct classic evidence
- the combat spec now includes an explicit contact / hostility escalation
  matrix:
  - neutral deep-space transit is separate from neutral hostile local intrusion
  - `PatrolSector` and anchored guard / blockade / starbase defense are now
    documented as distinct layers
- the remaining salvage question is no longer gameplay legality; it is record
  decoding:
  - the recovered points do not obviously land in
    `PLAYER.DAT.player.stored_prod_pts_raw`
  - the owned planet record and matching `DATABASE.DAT` row do change
  - the changed bytes are not yet a clean plain-integer `+20` under current
    field assumptions
- exact classic `MESSAGES.DAT` mail/report format and routing semantics are
  still only partially recovered; current Rust behavior preserves classic mail
  but does not yet decode or reproduce it faithfully
- `ECMAINT` timing / `Stardate` recovery is now partially grounded:
  - shipped historical logs strongly support a real `1..52` in-year timeline
    with rollover from `52/YYYY` to `1/YYYY+1`
  - the leading semantic interpretation is now week-of-year rather than
    literal day-of-year
  - black-box behavior now also shows this timeline is mechanically relevant,
    not just report narration:
    - `ec.txt -> ec2.txt` shows fleets with `Travel Time: 1/2 years` resolving
      at specific in-year stardates
    - the same `3rd Fleet` produces ordered reports in `3002` at weeks `12`
      and `21`, showing intra-year mission sequencing
  - the new timing-focused Ghidra report lives at
    [ec-timing-spec.md](ec-timing-spec.md) and
    `artifacts/ghidra/ecmaint-live/timing-flow.txt`
  - the first static correction is important:
    - `2000:945b` currently looks like a maintenance schedule/status date
      formatter in the token path, not the player-report `Stardate` emitter
  - the corpus-side ordering evidence is now stronger:
    - same-week ordering repeatedly shows
      `sensor contact -> identification -> interception`
    - adjacent report transitions are dominated by gap `0` and gap `1` weeks,
      which fits a real ordered weekly event stream
    - Fleet Command Center loss summaries also participate in immediate
      follow-on sequencing:
      - `fleet-lost -> join-retarget` same week
      - `fleet-lost -> planet-bombarded` same week
  - the actual report/rankings timestamp writer and any persisted per-day field
    are still open
- `ECMAINT` canonical phase-order recovery now has a dedicated stable note at
  [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
  - settled front/back boundaries:
    - schedule/token gate
    - `Move.Tok` crash restore before integrity validation
    - late summary canonicalization
    - late `1..52` weekly report loop
  - newly anchored late tail after restore/validate:
    - fixed call chain at `2000:861d`:
      `1da6 -> 0c06 -> 2db3 -> 56be -> [7659?]`
    - `56be` is mission-report oriented
    - `1da6` and `0c06` also now look heavily report/message oriented
    - `2db3` is the strongest current `DATABASE.DAT` / intelligence-output
      rebuild candidate
      - its internal helper `33f7` now ties directly to
        `Backing up intelligence database...`
    - `7659` is now better bounded as optional rankings/output generation:
      - only called when late flag `0x169a != 0`
      - loops staged player-side `0x16ae` records
      - allocates fixed `0x49`-byte blocks
      - looks like end-of-tail report/ranking work, not yearly simulation
    - `8b4a` is now a useful end-of-tail cleanup anchor:
      - resets `0x169a`
      - sets `0x634 = 1`
      - clears `0x635` / `0x636`
      - sets `0x638 = 1`
      - practical implication:
        keep the `169a/634/635/636/638` family out of Rust gameplay-order
        reasoning unless new evidence ties it back to simulation rather than
        output housekeeping
    - tighter `0x169a` read across the earlier late-tail passes:
      - `1da6`, `0c06`, and `2db3` all test `0x169a` near function entry
      - in each case it only chooses an initial output-workspace/header setup
        path
      - the main player/planet/report scan still runs afterward
      - practical implication:
        `0x169a` currently looks like late output-mode control, not a switch
        that turns yearly gameplay subphases on/off
      - tighter origin:
        `0x169a` is only set when plain `6d9b(arg=0)` validation fails and the
        recursive registered-stream `6d9b(arg=1)` recovery succeeds
      - practical implication:
        treat `0x169a` as "recovered restore mode reached the late tail", not
        as a generic rankings/gameplay phase flag
    - tighter `8612/8617/861d` feeder read:
      - `3000:1abc` is side-effect only at this callsite
      - `3000:1e88` is the value-producing feeder whose `AX` return is pushed
        directly into `6d9b(arg)`
      - current best model:
        pre-validation setup helper, then mode-selector helper, then
        `6d9b(mode)`
      - negative result:
        both `3000:1abc` and `3000:1e88` are still unmapped as code in both
        the live-dump project and the original-binary `ec-v15` project, so
        continue inferring them from callers/effects rather than waiting on
        direct disassembly first
    - tighter `731f` feeder-side classification from the live dump:
      - on the failure/recovery side of `6d9b`, `731f` first uses the same
        `0x46cc` timestamp/message plumbing as the token-timeout helpers
      - it allocates `0x20a` bytes at `0x33f8`, copies state from `0x2d68`,
        parses a short bounded record into `0x2d6a..0x2d70`, then bulk-copies
        that seeded state back into the `0x33f8` workspace
      - practical implication:
        treat `731f` as restore/workspace reconstruction or token-side
        housekeeping, not as part of the missing economy/movement/combat turn
        ordering
    - tighter `1000:00e8` vs `1000:024d` durable-summary split:
      - both still sit inside the same durable event-generation family:
        `f71d -> dddb`, then later `e31b`
      - only `024d` inserts an extra gate before the direct kind-2 append:
        player `+0x5a` check, then `db04(arg=0x0a)`, then `0x5dc/0x5de`,
        then the unmapped helper window around `f2c7`
      - `db04`, `d5d2`, and the recovered `f2c7` local window all currently
        look like per-player counter/timing / summary-prep helpers, not
        gameplay-core phase drivers
      - practical implication:
        keep this branch point inside "durable summary/event creation" for
        Rust-facing modeling; it is still not evidence for canonical
        movement/combat/economy ordering
    - tighter late meaning of player fields `+0x30/+0x32/+0x34/+0x36`:
      - `2000:0c06` scans those words up front only to decide whether late
        player output/report work exists before opening `0x3078`
      - `2000:5404` then acts as a merge helper over the same family,
        especially `+0x34`, before calling local output/summary helpers
      - `1000:cba4`, reached inside the kind-1 producer path, computes one
        `+0x34` value from weighted scratch fields (`+0x26/+0x28/+0x2a`
        plus `+0x2c/+0x2e`) and a special guard-starbase-style bonus gate
      - the previously unmapped `1000:0612..0794` window also only treats
        `+0x34/+0x36` as saturating accumulator/carry-forward state and folds
        them into `+0x58/+0x5a` before clearing `+0x34` again
      - practical implication:
        keep treating those fields as late player-output aggregation /
        reviewable-state counters, not evidence for gameplay-core yearly
        phase placement
    - tighter `f319/f34a` summary-family split:
      - `f1ee`, present only in `f319`, is now firmly a kind-2-only durable
        pool postpass
      - `eee7`, shared by both siblings, is the corresponding kind-1-side
        classifier/postpass
      - `d8a5` only derives a byte into player `+0x51`
      - upstream anchors:
        `2000:0788` can reach `f34a` directly from a planet-side owner-filtered
        aggregation loop after marking player `+0x6d`
      - stronger selector:
        `2000:05df..06e5` is the first confirmed direct branch that chooses
        between `f319` and `f34a` inside one late player loop:
        `player[+0x00] == 0xff` plus `player[+0x50] > 0` reaches `f319`;
        the non-`0xff` side runs `f713`, compares `0x190c - player[+0x4e]`
        against `0x2f70`, may mark `player[+0x6d] = 1`, then reaches `f34a`
      - tighter `+0x50/+0x52` bound:
        this pair is now better treated as a late per-player quota/work
        counter, not a phase flag:
        `e79a` decrements it as a 32-bit value around repeated `e2da` calls,
        `ea5f` gates on it, and summary-post-canonical refreshes it through
        `8830 -> 4895`
      - tighter `+0x6d` bound:
        current live-dump coverage only shows it as a local scratch eligibility
        mark inside `2000:05df..06e5`; it does not currently fan out into the
        wider late pipeline
      - practical implication:
        this whole sibling family now looks like late durable-summary
        production plus kind-specific follow-up, not hidden middle
        turn-order sequencing
    - the startup `main.tok` / `Creating main work file...` / `Merging joint
      fleets...` cluster still has no direct scalar xrefs in the live dump, so
      that outer startup/status path is likely indirect/table-driven
    - first coarse dynamic file-I/O trace on the classic `bombard` scenario now
      supports a broad split between early mutation and later rebuild/flush:
      - first write burst is overwhelmingly `FLEETS.DAT`
      - only later do `DATABASE.DAT`, `PLAYER.DAT`, `PLANETS.DAT`,
        `CONQUEST.DAT`, and `RANKINGS.TXT` get written
      - practical implication:
        use file-I/O tracing as evidence that heavy fleet-state mutation lands
        before the derived-output tail, but not as evidence for the exact
        order of economy, movement, combat, or producer passes inside step `4`
    - current DOSBox debugger capture caveat:
      - a narrower staged bridge now works:
        first file-open (`INT 21h / AH=3D`) then `2814:96c4`
      - that `96c4` stop surfaced live as normalized `3159:0274`, confirming
        the same linear-address bridge into the unpacked image
      - arming the full earlier-driver breakpoint set only after first
        file-open still misses those seams
      - arming that full set immediately at debugger start still misses the
        loaded image and falls straight through to exit
      - practical implication:
        keep the top-down dynamic-trace plan, but stage through the confirmed
        `first file-open -> 96c4` bridge and bisect which later startup/driver
        breakpoints are individually safe before trying the whole set again
  - still-open middle block:
    - exact ordering of economy / production / movement / combat / assaults
    - weekly aftermath timing is now clearly mission-family dependent rather
      than one universal delay
- for the Rust client, do not infer "returning joined player" from
  `PLAYER.DAT` assigned-player fields alone:
  - live classic probing now shows caller/dropfile identity matching the
    persisted player handle is part of login recognition
  - keep a distinction between:
    - maint-valid fixtures
    - login-valid matching-player fixtures
    - matched pre-loaded first-login fixtures
  - Rust client startup should branch at least three ways:
    - first-time menu
    - matched pre-loaded player first-login onboarding
    - established joined-player login flow
  - future BBS-door dropfile support should stay Rust-native and forward-looking:
    - parse classic `CHAIN.TXT` plus modern telnet/BBS dropfile shapes through a
      thin `ec-client` session adapter layer
    - normalize those inputs into one internal Rust session/startup context
    - keep door-file parsing out of `ec-data` and core gameplay state
    - if the integration surface grows, split it into a thin launcher/adapter
      crate rather than pushing BBS-specific logic down into the engine
- SQLite-backed campaign persistence is now started:
  - each campaign uses a bundled/self-hosted `ecgame.db`
  - `ec-client` now loads/saves runtime state from `ecgame.db`
  - `maint-rust` now also runs against `ecgame.db` and stores its next
    snapshot there
  - classic `.DAT` import/export is now an explicit `ec-cli` bridge rather
    than the live runtime path for the client or Rust maintenance
  - for hybrid classic-client campaigns, `maint-rust` now refreshes SQLite
    from the live working directory before processing if classic `.DAT` files
    have changed since the last stored snapshot
  - Rust-created new games now seed `ecgame.db` automatically
  - the store keeps normalized record-set snapshots plus compatibility/export
    payloads for unresolved classic outputs
  - the total planet database now has a path for SQLite-backed `Last Intel`
    year metadata
  - intel tiers are now explicit:
    - `owned`
    - `full`
    - `partial`
    - `unknown`
  - current intel-year stamping is still first-pass and should be refined as
    more gameplay/report paths sync into SQLite

These are refinement tasks, not blockers for calling `rust-maint` a usable
full-game engine.

## Next Phase: SQLite-backed Rust ECGAME

The next major phase should be cloning `ECGAME` in Rust while keeping the
existing `.DAT` compatibility boundary intact.

Initial scope:

- replicate the player-facing command flow and reports, not just the maint
  backend
- use the existing Rust maintenance/report pipeline instead of recreating game
  rules in a second place
- use the SQLite campaign store as the first-class persisted campaign state
  while keeping `.DAT` import/export as the oracle compatibility boundary
- preserve classic terminology, menu structure, and campaign feel where the
  manuals or live `ECGAME` behavior are clear
- do not invent a surrender UI action; the manuals describe surrender as a
  campaign outcome, and live `ECGAME` evidence shows no General Command
  surrender option
- for the Rust client, present official maintenance/results reports before
  player-to-player mail so reports reveal outcomes before social commentary can
  spoil them

First concrete work:

- document the `ECGAME` command/menu surface we want to clone first
- identify which current `ec-cli` report and inspection surfaces already cover
  those needs
- start a Rust `ECGAME` phase around:
  - status / reports / database viewing
  - diplomacy commands
  - order entry and review
  - classic player workflow around the existing Rust engine

Treat the login/startup side as one explicit pre-command-center pipeline:

- show the built-in EC ASCII splash first
- then show the in-client text intro pages
- after the intro, branch by player state before any command center opens
  - unjoined player:
    - first-time help/list/join flow
    - then First Time Menu
  - joined player:
    - reports/messages review
    - homeworld/new-colony naming prompts when applicable
    - then Main Menu
- keep this as one Rust client flow so onboarding, login-time review, naming,
  and menu entry are modeled together instead of as disconnected screens

## Step-4 RE Findings (2026-03-17)

Major structural recovery from enriched file-I/O trace analysis across 6
scenarios (bombard, econ, fleet-order, fleet-battle, invade, planet-build):

1. **The yearly simulation is a 52-pass weekly fleet-processing loop**:
   all non-destructive scenarios show exactly 832 fleet writes = 52 passes ×
   16 records. Each pass reads-then-writes every active fleet record once.

2. **Fleet visit order is data-dependent**: varies by scenario but is stable
   within a scenario (all 52 passes use the same order). Not sequential by
   slot index.

3. **Combat reports are emitted inline during the weekly loop**: in
   fleet-battle, RESULTS.DAT writes (11 × 84 bytes) are interleaved
   inside fleet write pass 7. Report generation is NOT deferred to a
   post-simulation phase.

4. **Fleet destruction is dynamic**: fleet-battle has 15 active records
   (one fleet absent), invade has non-integer passes/records (685/15 = 45.7)
   indicating mid-pass fleet destruction.

5. **Incremental fleet activation**: fleet-battle early passes show fleets
   entering the active write set gradually (1 record in pass 1, growing to
   14 by pass 6), suggesting movement arrival activates fleets dynamically.

Tooling delivered:
- `tools/analyze_fileio_trace.py` — mines existing traces for fleet pass
  timelines, cross-file interleaves, database slot correlations
- `tools/step4_ordering_probes.py` — 5 black-box ordering probes
  (econ-vs-movement, production-vs-combat, command-normalization,
  econ-weekly-timing, invade-ordering)
- `tools/capture_ecmaint_fileio_trace.py` — now accumulates per-scenario
  artifacts and runs auto-analysis
- `tools/capture_ecmaint_sim_driver_trace.py` — breakpoints now
  parameterizable via `-b label` for safe single-breakpoint probing

Cross-scenario comparison artifact:
`artifacts/ecmaint-fileio-trace/cross_scenario_comparison.txt`

Additional findings from deep analysis:

6. **Fleet visit order is PRNG-shuffled**: four scenarios with identical
   CONQUEST/SETUP/FLEETS but different PLANETS produce completely different
   visit orders. Small planet data changes cascade into different orderings.
   Likely seeded from Borland Pascal Random.

7. **Combat resolution is triggered by first co-located hostile fleet**: in
   fleet-battle pass 7, fleet 4 (empire 2) processing reads fleet 0
   (empire 1), resolves combat, emits 11 RESULTS records inline, then
   writes fleet 4. Fleet 0's writeback happens later in the same pass.

8. **Fleet slot reassignment (capture)**: fleet-battle shows fleets 2, 3
   changing owner empire 1→2 and fleet 8 changing 3→4. Reassigned fleet
   slots are excluded from the weekly visit set.

Spec updates: `ec-turn-cycle-spec.md` sections 4i-4o,
`rust-turn-cycle-implementation.md` updated block diagram, driver skeleton,
settled facts (6 new), and open questions (refined from 8 to 7).

### Remaining gaps for faithful Rust reproduction

Closed (high confidence, actionable):
- [x] Outer loop structure: 52-week fleet processing loop
- [x] Combat reports emitted inline during fleet processing
- [x] Combat resolution triggered by first co-located hostile fleet
- [x] Fleet visit order: sort-by-random-priority (mechanism fully recovered;
      exact replication infeasible but slot order produces oracle-identical results)
- [x] Fleet destruction/capture dynamics
- [x] File write/flush ordering
- [x] Movement is position-first, mission resolves next year
- [x] Colonization is atomic on arrival (ownership+armies+name+status)
- [x] Economy/autopilot processing gated by `player[0]` (rogue vs civil disorder)
- [x] Economy runs AFTER fleet loop (PLANETS.DAT never accessed during loop;
      economy outcomes depend on post-combat fleet state)

- [x] Fleet "incremental activation" is a pre-loop capture/setup phase
      (5 passes in fleet-battle, 0 in non-combat; passes 6-57 = exactly 52 weeks)

- [x] Mission-family timing constants recovered from `1000:a26e` switch:
      8 codes with offsets +2/+7/+21/+0/+0/+0/+0/+30, min weeks
      10/15/20/0/0/0/0/25, priorities 6/5/4/6/5/5/3/1. Contact→ID=5wk
      gap, ID→intercept=14wk gap, late resolution=+30wk
- [x] Code-to-fleet-composition mapping FULLY recovered: only codes 3-6
      are ever written (starbase→3, BS→4, CA/TT/army→5, scout/DD→6).
      Codes 1,2,7,8 are dead code — never assigned by any producer in the
      entire binary (confirmed by full memdump search)

- [x] Inner loop body resolved: 52-week loop is event scheduling, not
      physics simulation. Movement is annual (position + tuple_c scratch).
      Weekly passes handle encounter detection, combat, report emission.
      Stardates from timing codes, not physical arrival time.

Still open:
- [ ] Exact PRNG for visit order: LCG confirmed ($08088405), RandSeed at
      DS:0x03A6, DS=0x3529 at runtime, RandSeed=0x000E000E at bridge.
      Full 2^32 search ruled out Fisher-Yates and sort-by-key. Seed is
      accumulated from validation-phase calls. `capture_randseed.py` tool
      works through bridge but needs segment-renormalization fix to reach
      deeper breakpoints (861d/8652). Next: either fix the DOSBox BP
      staging or use Ghidra to find the shuffle call site directly

## Immediate Next Steps

1. Probe the remaining client/login edge cases in live `ECGAME`:
   - verify the restored default `sysop new-game` path still triggers the full
     first-join naming/onboarding flow
   - distinguish clearly between:
     - unmatched caller -> first-time menu
     - matched pre-loaded player first login
     - established joined-player login
   - capture any remaining differences in report/message ordering or prompt
     wording between those three startup branches
2. Keep running periodic seeded multi-turn `rust-maint` sweeps to guard against
   regressions while the UI/client work begins.
3. Treat maint hardening as settled unless new evidence contradicts it:
   - do not weaken the shared-engine validation/sanitization path just to match
     older client-local behavior
   - if a future manual-only `A+` pass is desired, prove the remaining
     interpretation-heavy edges against original binaries before changing the
     current canonical Rust rules
4. Finish the fleet oracle pass before changing any manual-adjacent mission logic:
   - keep recording reproducible classic defects as known `v1.51` bugs instead
     of copying them into Rust by default
5. Tighten the remaining CLI/storage boundary:
   - identify which `ec-cli` mutators still operate directly on classic `.DAT`
   - decide which should become SQLite-native next and which should remain
     explicit compatibility tooling
   - keep the rule that only explicit CLI import/export paths bridge classic
     directories into the runtime
   - current intentional exception:
     `core-init-current-known-baseline` still mutates the projected `.DAT`
     directory directly because the canonical transition reports depend on its
     exact file-shape drift against the preserved post-maint baseline
6. Write a focused Rust `ECGAME` phase plan:
   - command center
   - reports and intel views
   - diplomacy screens
   - order-entry workflow
   - fleet mission target defaults:
     - combat missions should default to the closest known enemy world, not the
       player's homeworld
     - if no known enemy world exists, show a brief notice instead of opening a
       misleading target prompt
     - ETAC colonize targeting should later prefer the closest uncolonized
       planet, skipping the player's own worlds, skipping known colonized
       worlds, and avoiding planets already targeted by other friendly ETAC
       colonize missions, sorted by distance
   - defer real `X`/expert-mode behavior until the remaining command/menu
     surfaces are finished; implement it as a final menu-verbosity pass rather
     than a premature partial toggle
7. Keep tightening original production semantics for player-facing screens:
   - empire profile / rankings / planet info should use classic terms like
     `Present Production`, `Potential Production`, and `Total Available Points`
   - do not expose raw internal names like `factories` in the client UI
   - if stronger oracle evidence appears, refine the canonical Rust growth
     formula rather than reintroducing placeholder arithmetic
   - decode or rename the overloaded per-planet `raw[0x0E]` byte before using
     it for more player-facing economy output
8. Use the now-working DOSBox `ECGAME` harness to capture only the player-side
   screens and behaviors needed for the first Rust clone pass.
9. Continue the SQLite transition:
   - keep `ecgame.db` bundled/self-hosted with no external SQLite dependency
   - expand client/state sync so gameplay mutations refresh the latest snapshot
   - move more report/intel/history surfaces onto SQLite-backed queries
   - preserve `.DAT` export compatibility with oracle sweeps
10. Keep the total planet database aligned with the intel model:
   - all planets listed, fog-filtered
   - `?` for unknown fields
   - `Last Intel` year shown as `Y####` or `?`
   - Main/General remain intel views; Planet menus remain owned-asset views
