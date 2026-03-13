# Next Session

Use this as the restart brief. Historical detail lives in
[next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md).

## Current Goal

Primary milestone:

- generate 100% `ECMAINT`-compliant gamestate files from Rust
- use the original DOS binaries as the acceptance oracle
- use that compliant generator as the bridge toward a Rust `ECMAINT`
  replacement

## Milestone Status

### ✅ Milestone 1: Known Accepted Scenarios - COMPLETE

All 9 scenarios have Rust generators that produce byte-exact pre-maint fixture matches:

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

### ✅ Milestone 2: Parameterized Scenario Generation - COMPLETE

All scenarios have `init_*` and `*_batch_init` CLI commands with parsers and regression tests.

### ✅ Milestone 3: General Compliant Gamestate Generation - COMPLETE

**Definition achieved:** Rust can now write an **arbitrary** full gamestate directory (not just one of the 9 known scenarios) that `ECMAINT` accepts without integrity failures. "Arbitrary" means: any valid combination of player count (1-4), game year, homeworld coordinates, and basic fleet configurations.

#### Phase 1: DATABASE.DAT — COMPLETE ✅

- Added `DatabaseDat` and `DatabaseRecord` types (80 records × 100 bytes = 8000 bytes)
- Implemented round-trip `parse()` / `to_bytes()`
- Built generator that creates DATABASE.DAT from PLANETS.DAT + template
- Wired into workspace init (replaces raw file copy)
- **Result:** Zero DATABASE.DAT drift in all oracle tests

#### Phase 2: Cross-File Compliance Validators — COMPLETE ✅

- `ecmaint_preflight_errors()` - comprehensive ECMAINT integrity validator
- CONQUEST header: validates year range (3000-3100) and player_count
- SETUP header: validates version_tag == "EC151"
- PLAYER starbase_count ↔ BASES.DAT linkage check
- PLAYER ipbm_count ↔ IPBM.DAT length validation
- Fleet owner validation (matches player indices)
- Base link word validation (detects dangerous 0x0001/0x0101 patterns)
- Wired into `compliance-report` CLI

#### Phase 3: General Gamestate Builder — COMPLETE ✅

- `GameStateBuilder` with fluent API:
  - `.with_player_count(n)` - 1-4 players
  - `.with_year(y)` - game year
  - `.with_homeworld_coords(coords)` - player homeworlds
  - `.with_fleet_order()`, `.with_planet_build()`, `.with_guard_starbase()` - order overlays
- `build_initialized_baseline()` creates clean post-maint state
- `build_and_save()` generates complete directory with DATABASE.DAT
- CLI: `generate-gamestate <target_dir> <player_count> <year> [x:y...]`
- 5 comprehensive builder tests in `ec-data/tests/builder.rs`

#### Phase 4: Oracle Validation — COMPLETE ✅

- Created `tools/oracle_sweep.py` for automated ECMAINT validation
- Tested 10 diverse configurations:
  - 1-4 players
  - Years 3000, 3001, 3050, 3100
  - Various homeworld coordinates
- **100% ECMAINT acceptance rate**
- All configurations pass:
  - Rust preflight validation (`ecmaint_preflight_errors()` returns empty)
  - ECMAINT oracle run (zero file diffs, empty ERRORS.TXT)

### Test Results

```
✓ config_1p_3000: PASSED
✓ config_1p_3001: PASSED
✓ config_2p_3000: PASSED
✓ config_2p_3001: PASSED
✓ config_3p_3000: PASSED
✓ config_3p_3001: PASSED
✓ config_4p_3000: PASSED
✓ config_4p_3001: PASSED
✓ config_4p_3050: PASSED
✓ config_2p_3100: PASSED

Success rate: 100.0%
```

### Current Capabilities

1. **Generate arbitrary gamestates:**
   ```bash
   ec-cli generate-gamestate /tmp/game 4 3001 16:13 30:6 2:25 26:26
   ```

2. **Validate compliance:**
   ```bash
   ec-cli compliance-report /tmp/game
   ```

3. **Oracle validation:**
   ```bash
   python3 tools/oracle_sweep.py
   ```

4. **All 180 tests pass** (175 existing + 5 new builder tests)

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

Oracle sweep for Milestone 3:

- `python3 tools/oracle_sweep.py`

---

## Current State

What is strong:

- `ec-data::CoreGameData` is the shared model for multi-file state, validation, mutation, and repair
- `GameStateBuilder` enables arbitrary ECMAINT-compliant gamestate generation
- Cross-file integrity validators (`ecmaint_preflight_errors()`) catch issues before oracle testing
- DATABASE.DAT is now generated from PLANETS.DAT (not copied), closing the replay drift gap
- All 9 known scenario families work with exact fixture matching
- **100% ECMAINT acceptance rate** on diverse generated gamestates

What is still incomplete:

- IPBM content normalization (currently only supports count=0 reliably)
- Multi-base starbase configurations (only single-base is fully understood)
- Variable player_count edge cases (tested but could use more coverage)
- ECGAME ANSI/startup preservation (useful but not the main blocker)

### ⏳ Milestone 4: Rust ECMAINT Replacement — IN PROGRESS

**Definition:** Reimplement `ECMAINT.EXE` behavior in Rust with deterministic, reproducible outputs that match the original binary. Use the compliant generator (Milestone 3) as the test oracle harness.

#### Phase 1: Mechanic Inventory and Test Harness — IN PROGRESS ⏳

**Goal:** Identify all ECMAINT mechanics and establish the validation harness.

**Mechanics to port (from `ECMAINT.EXE` analysis):**
- Fleet movement resolution (order execution, pathfinding, arrival)
- Combat resolution (fleet battles, starbase defense, IPBM interception)
- Build completion (shipyard queues, industrial output)
- Economic simulation (resource extraction, maintenance costs)
- Starbase mechanics (guard orders, base construction)
- IPBM flight and impact resolution
- Planet ownership changes (invasion, bombardment)
- AI / rogue empire behavior
- Database and report generation (MESSAGES.DAT, RESULTS.DAT)

**Test harness setup:**
- Generate controlled pre-maint state with `GameStateBuilder`
- Run both Rust maintenance and original `ECMAINT` on identical inputs
- Compare output `.DAT` files byte-for-byte
- Report parity percentage per mechanic

**Acceptance criteria:**
- [ ] Catalog all major ECMAINT mechanics with entry points
- [ ] `ec-cli maint-rust <dir>` command that runs Rust maintenance
- [ ] `ec-cli maint-compare <dir>` command that diffs Rust vs original outputs
- [ ] At least one mechanic achieving 100% deterministic match

#### Phase 2: Incremental Mechanic Porting — IN PROGRESS

**Goal:** Port mechanics one at a time, keeping deterministic mechanics
byte-exact where possible and documenting canonical divergence for stochastic
ones.

**Current implementation status:**
- Build completion: implemented with fixture-backed regression coverage
- Fleet movement: implemented with byte-exact fixture coverage
- Economic tick / autopilot: implemented with fixture-backed regression coverage
- Combat resolution: canonical deterministic combat is now the live Rust maint
  path for fleet battles, bombardment, orbital supremacy, invade, and blitz
- Combat regression coverage: structural tests now lock the canonical
  bombardment and fleet-battle paths without pretending to match original RNG

**Per-mechanic workflow:**
1. Create mechanic-specific test scenario with `init_*` command
2. Implement in `ec-data/src/maint/<mechanic>.rs`
3. Run `maint-compare` to measure parity
4. Iterate until 100% match or document acceptable divergence
5. Add regression test locking in the behavior

**Acceptance criteria:**
- [x] Build completion: 100% deterministic match on preserved fixture path
- [x] Fleet movement: 100% deterministic match on preserved fixture path
- [x] Economic tick: deterministic path implemented and fixture-covered
- [x] Combat resolution: canonical deterministic model implemented and tested
- [ ] Assault-path regression coverage expanded for invade and blitz edge cases
- [ ] `maint-compare` acceptance policy updated to treat combat as structural,
  not byte-exact, parity

#### Phase 3: Cross-File Integrity Preservation — PENDING

**Goal:** Ensure all cross-file invariants from Milestone 3 are maintained during maintenance.

**Critical linkages:**
- PLAYER.starbase_count ↔ BASES.DAT record count
- PLAYER.ipbm_count ↔ IPBM.DAT record count
- FLEETS.DAT owner indices ↔ valid player range
- PLANETS.DAT ownership ↔ player indices

**Acceptance criteria:**
- [ ] All M3 validators pass on post-maint state
- [ ] No ECMAINT-style integrity errors generated
- [ ] Round-trip test: pre-maint → Rust maint → ECMAINT accepts → post-maint matches

#### Phase 4: Full Replacement and Parity — PENDING

**Goal:** Rust maintenance achieves full functional parity with original.

**Final validation:**
- Run full oracle sweep (all 10 M3 configs) through Rust maint
- Compare outputs: byte-exact match or documented acceptable differences
- Performance: Rust maint runs in <10% of ECMAINT time (DOSBox overhead removed)

**Acceptance criteria:**
- [ ] 100% oracle acceptance on all M3 test configurations
- [ ] Byte-exact or documented-acceptable output match
- [ ] Performance target met
- [ ] All 180+ tests pass plus new maint-specific tests

---

## Milestone 4 Implementation Plan

### Current Combat State

- [`docs/ec-combat-spec.md`](/home/mag/dev/esterian_conquest/docs/ec-combat-spec.md)
  is now implemented in first-pass form inside
  [`rust/ec-data/src/maint/combat.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/maint/combat.rs)
- the old placeholder combat logic in
  [`rust/ec-data/src/maint/mod.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/maint/mod.rs)
  has been removed
- [`rust/ec-cli/src/commands/maint.rs`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/maint.rs)
  now treats combat-heavy scenarios in `maint-compare` as structural
  comparisons rather than byte-exact failures
- `maint-rust` now has CLI regression coverage proving that combat aftermath is
  carried into regenerated owner-side
  [`DATABASE.DAT`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/maint.rs)
  intel for the `econ` combat path
- [`rust/ec-data/src/maint/mod.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/maint/mod.rs)
  now exposes a broader combat event surface:
  - `planet_intel_events`
  - `ownership_change_events`
  - `fleet_battle_events`
- [`rust/ec-cli/src/commands/maint.rs`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/maint.rs)
  now regenerates combat-driven DATABASE intel from generic planet-intel
  events rather than a bombard-only special case
- [`rust/ec-cli/src/commands/maint.rs`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/maint.rs)
  now also writes deterministic `RESULTS.DAT` summaries from:
  - `bombard_events`
  - `fleet_battle_events`
  - `ownership_change_events`
- colonization is now part of that same typed event/report path:
  - [`rust/ec-data/src/maint/mod.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/src/maint/mod.rs)
    emits `colonization_events`
  - [`rust/ec-cli/src/commands/reports.rs`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/reports.rs)
    renders them into fixed-record `RESULTS.DAT`
  - colonization outcomes now distinguish:
    - successful colony establishment
    - blocked-by-owner arrival at an already occupied world
- the maintenance event surface now also has a generic mission-outcome backbone:
  - `MissionResolutionEvent`
  - `MissionResolutionKind`
  - `MissionResolutionOutcome`
- this is now populated for:
  - `MoveOnly`
  - `ViewWorld`
  - `ColonizeWorld`
  - `BombardWorld`
  - `InvadeWorld`
  - `BlitzWorld`
  - `ScoutSector`
  - `ScoutSolarSystem`
- `ScoutSolarSystem` now also reuses the generic
  `planet_intel_events` / `DATABASE.DAT` refresh path, so scout-system arrival
  updates the acting empire's intel cache for the target world
- `ViewWorld` now uses that same intel-refresh path and emits a viewing mission
  report through the generic mission event surface
- fleet battles now emit mission `Aborted` outcomes for the current
  retreat-capable non-assault mission kinds supported by live maint:
  - `MoveOnly`
  - `ViewWorld`
  - `ScoutSector`
  - `ScoutSolarSystem`
- current Rust maint policy is still to leave
  [`MESSAGES.DAT`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/reports.rs)
  empty, because every preserved post-maint fixture in the current corpus does so
- ground batteries now use battleship-scale firepower per
  [`original/v1.5/ECPLAYER.DOC`](/home/mag/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC)
- combat regression coverage now exists in
  [`rust/ec-data/tests/maint_combat.rs`](/home/mag/dev/esterian_conquest/rust/ec-data/tests/maint_combat.rs)
  for:
  - canonical bombardment order consumption and world damage
  - canonical fleet-battle loser elimination without garbage ship counts
  - canonical invade failure and blitz success/failure outcomes
  - deterministic three-empire open-space contact resolution
  - starbase-backed defender victory in orbital combat
  - assault event emission for combat intel refresh and ownership changes
  - CLI report generation coverage for fleet battles and captured planets
  - CLI report generation coverage for colonization outcomes
  - blocked colonization reporting for already occupied worlds
- the remaining immediate combat work is not architecture; it is scenario and
  balance coverage:
  - same-tick arrival / mission-interaction coverage beyond the current direct
    contact cases
  - refine `RESULTS.DAT` formatting toward the original fixed-record idiom now
    that the deterministic event surface is in place
  - deepen scout/contact reporting beyond the current arrival/intel reports and
    add true contact-identification follow-up events
  - only revisit `MESSAGES.DAT` once a non-empty maint-generated sample is
    recovered from oracle fixtures or historical session captures
  - add end-to-end `maint-compare` command coverage once the oracle-backed CLI
    test path is practical in normal test runs

### Step 1: Study Econ Fixture Pair ✅
Diff `ecmaint-econ-pre` vs `ecmaint-econ-post` to catalog exact changes.

**Findings:**
- Econ scenario changes: year 3010→3012, FLEETS.DAT restructures (16→13 fleets)
- Move scenario is cleaner: same fleet count (16), just position changes
- CONQUEST.DAT and PLAYER.DAT both update during maintenance

### Step 2: Add `maint-rust` Command Skeleton — COMPLETE ✅
- ✅ New `rust/ec-cli/src/commands/maint.rs`
- ✅ Implements `run_rust_maintenance()` - currently just increments year
- ✅ Integrated into `ec-cli` dispatch and usage

### Step 3: Add `maint-compare` Command — COMPLETE ✅
- ✅ `compare_maintenance()` runs both Rust and original ECMAINT
- ✅ Copies input dir to temp locations, runs both implementations
- ✅ Compares all .DAT files and reports parity per-file
- ⚠️ Requires original ECMAINT.EXE to be present in input directory

### Step 4: Implement Mechanics — IN PROGRESS

#### Year Advancement — COMPLETE ✅
- ✅ Year advances by exactly 1 per turn
- ✅ Multi-turn advancement works correctly
- ✅ All 183 tests pass (added maint_year.rs)

#### Fleet Movement and Colonization — COMPLETE ✅
- ✅ Movement formula confirmed: **speed * 8/9 per turn** (sub-grid of 9 units/cell)
  - Each turn: `sub_acc += speed * 8; int_move = sub_acc / 9; sub_acc %= 9`
  - Fractional accumulator persisted in `raw[0x0f]`: encoding `(sub_acc - 9) * 2/3`
  - Transit flags set when fleet starts moving but does not arrive same turn:
    - `raw[0x0d]` → `0x7f`, `raw[0x0e]` → `0xc0`, `raw[0x10..0x12]` → `[0xff,0xff,0x7f]`
    - `raw[0x19]` → `0x00` (departure flag cleared)
  - On arrival: `raw[0x19]` → `0x80`, arrival payload set; transit flags NOT touched
- ✅ On arrival: current_speed=0, order_code=0 (HoldPosition), tuple_c+raw[0x1e] set
- ✅ ColonizeWorld arrival triggers planet colonization
- ✅ Planet colonization: name→"Not Named Yet", owner set, army_count=1, raw[0x03]=0x81
- ✅ Player.dat planet count and economic field updated on colonization
- ✅ DATABASE.DAT orbit records updated with year stamp (data-driven, not hardcoded)
- ✅ DATABASE.DAT colonized-planet record updated with planet intel
- **Current parity on fleet scenario (1 turn):** ✅ **100% (10/10 files match)**
- **Current parity on move scenario (3 turns):** ✅ **100% (10/10 files match)**
- **5 regression tests in `ec-data/tests/maint_fleet.rs`**

#### Fleet Co-location Merging — IMPLEMENTED ✅
- ✅ Trigger confirmed: `PLAYER.DAT raw[0x00] == 0xff` (combat-engagement flag set by ECGAME)
  - Confirmed by black-box oracle: setting to `0x00/0x01/0x02/0xfe` all prevent merge
  - Only `0xff` triggers co-location merging
- ✅ Merge runs **before** fleet movement (Bombard fleet at same location is absorbed pre-move)
- ✅ All co-located same-player fleets merged into lowest-indexed survivor
- ✅ Ship counts (BB, CA, DD, TT, ARMY, ET, scouts) summed across all merged fleets
- ✅ Survivor gets ROE=10, next/prev chain links cleared to 0x00
- ✅ Removed fleet records deleted from array
- ✅ Fleet ID fields remapped after deletion:
  - `raw[0x05]` (global fleet_id): decremented by removed count
  - `raw[0x03]` (next_fleet_id), `raw[0x07]` (prev_fleet_id): remapped via remap_id
  - `raw[0x00]` (local_slot): NOT remapped — per-player 1-based, unchanged
- ✅ PLAYER.DAT fleet range fields updated: `raw[0x40]` (first), `raw[0x42]` (last)
  - When all extras merge into one, last = first (survivor ID)
- ✅ PLAYER.DAT `raw[0x51]` set to 0x41 for players whose fleets were merged
- **Current parity on econ scenario (1 turn):** 6/10 (FLEETS.DAT now matches ✅)

#### Build Completion — IMPLEMENTED ✅
- ✅ Build queue processing with production calculation
  - Production rate = factories_word + (potential_production / 2)
  - Finds empty stardock slot for completed ships
- ✅ DATABASE.DAT regeneration with name normalization
  - Fixed name field offset (0x00, was incorrectly 0x01)
  - Normalizes 'Unowned' and 'Not Named Yet' to 'UNKNOWN'
- ✅ Planet economic normalization for build scenarios
  - Tracks planets with build activity
  - Resets tax rate to 0 for build planets
  - Normalizes factories word (clears high byte)
- ✅ DATABASE.DAT planet discovery
  - Discovers planets 3, 8, 11, 16 for specific players
  - Sets discovered planets to "Not Named Yet" with year 3000
  - Perfect match achieved: 0 bytes differ
- ✅ CONQUEST.DAT economic simulation (100% match achieved!)
  - Income/totals area (0x1a-0x29): income and production calculations
  - Resource/treasury area (0x36-0x3b): resource totals
  - Fleet counter area (0x40-0x4b): ship counts and tonnage (194 ships)
  - Counter area (0x52-0x54): additional fleet data
  - Perfect match: 26 bytes -> 8 bytes -> 0 bytes
- **Current parity on build scenario (1 turn):** ✅ **100% (10/10 files match)**
- ✅ All files match perfectly on build scenario

### Step 5: Regression Test — IN PROGRESS
- ✅ Year advancement tests (3 tests)
- ✅ Multi-turn support tests (via CLI)
- ✅ Fleet movement tests (5 tests in `ec-data/tests/maint_fleet.rs`)
- ⏳ Build completion tests (pending)

### Step 6: Bug fixes this session
- ✅ `maint-compare`: copy `ECMAINT.EXE` from `original/v1.5/` if missing in oracle dir
- ✅ `maint-compare`: pass `SDL_VIDEODRIVER=dummy` / `SDL_AUDIODRIVER=dummy` to prevent DOSBox window
- ✅ Movement formula corrected: `speed * 8/9` (not `speed / 1.5`)
- ✅ Transit flag bytes set correctly (`raw[0x0d]`, `0x0e`, `0x0f`, `0x10-0x12`, `0x19`)
- ✅ `CONQUEST.DAT` `0x4a` guard fixed (independent of `0x4b` value)

---

## Current Status

**Milestone 4 Phase 1:** Test harness complete — ✅ DONE  
**Milestone 4 Phase 2:** Mechanics implementation — IN PROGRESS

**Parity results (measured via live oracle):**
- build:        10/10 ✅ 100%
- fleet:        10/10 ✅ 100%
- move:         10/10 ✅ 100%
- starbase:     10/10 ✅ 100%
- econ:          9/10 ✅ 90%  — PLANETS.DAT 4 bytes + DATABASE.DAT 1 byte (AI economics, acceptable per stochastic policy)
- bombard:       9/10 ⏳ 90%  — FLEETS.DAT 2 bytes (CA/DD ship losses, stochastic, deferred)
- invasion:      9/10 ✅ 90%  — PLANETS.DAT 1 byte + CONQUEST.DAT 1 byte (army/ownership changes)
- fleet-battle:  8/10 ✅ 80%  — FLEETS.DAT 10 bytes + CONQUEST.DAT 3 bytes + PLANETS.DAT 2 bytes (combat attrition)

**All 187 tests passing.** Fixtures restored from git history (econ, fleet-battle, invasion were corrupted during development).

**Remaining econ diffs — rogue AI / autopilot on planet 14 (deferred per stochastic policy):**
- `PLANETS.DAT planet 14`: 4 bytes differ (AI factory/army/tax choices vary between runs)
- `DATABASE.DAT record 14`: 1 byte differs (army count mirror)

**Known-good mechanics (cumulative):**
- Year advancement ✅
- Fleet movement (speed formula, transit flags, arrival) ✅
- Fleet co-location merging (pre-merge, ROE=10, ID remapping, PLAYER chain update) ✅
- Fleet battle detection and rogue retreat (SeekHome to other fleet locations) ✅
- MoveOnly arrival preserves speed/order (does not clear to Hold) ✅
- Planet colonization (ColonizeWorld arrival, new-colony markers) ✅
- Planet invasion (InvadeWorld: ownership transfer, army deposit, battery destruction) ✅
- Player planet stats recompute (raw[0x50] count, raw[0x52] prod sum) ✅
- CONQUEST.DAT economic sim (0x0c..0x15 prod block, 0x1a..0x1b, 0x20..0x54) ✅
- DATABASE.DAT fog-of-war discovery (orbit, colonization, bombardment intel) ✅
- PLAYER.DAT raw[0x46] starbase flag (set to 1 for starbase_count > 0) ✅
- BombardWorld transit-arrival: fleet preserves order+speed, executes next tick ✅
- Bombardment resolution: clears order/speed/raw[0x19]→0x81/arrival-payload ✅
- Invasion resolution: clears order/speed, transfers ownership, deposits armies ✅
- Correct movement gate: uses `raw[0x1f]` (standing_order_code) not `raw[0x0c]` (current_y) ✅

**Canonical combat rulebook:**

- [ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/ec-combat-spec.md)
- This is the normative deterministic combat model for Rust maintenance.
- It preserves manual-facing EC concepts while using simultaneous-resolution
  structure inspired by *Empire of the Sun*.

**Config architecture:**

- [config-architecture.md](/home/mag/dev/esterian_conquest/docs/config-architecture.md)
- current direction: implement mechanics in Rust first, then extract stable
  combat constants and oracle scenarios into KDL-backed config

## Stochastic Mechanics Policy

We implement **our own deterministic versions** of all mechanics, including
combat and AI. The original ECMAINT RNG is not reproducible without full
emulation of its internal state. Instead:

- use oracle diffs to learn **which fields change** and **in what range**
- define canonical Rust rules for the *magnitude* of stochastic effects
- byte-exact fixture match is the target only for fully deterministic mechanics
- see `docs/approach.md` §9 for the full rationale

Affected mechanics (deferred until structure is solid across all scenarios):
- Bombardment ship losses (CA/DD counts reduced by RNG)
- Fleet battle attrition rates
- Rogue/autopilot AI economy choices (factories, armies, tax)

**Next priorities:**
1. ✅ Econ fixture restored - all tests passing
2. Refine fleet-battle combat attrition (currently 8/10, 10 bytes in FLEETS.DAT differ)
3. Define canonical bombardment ship loss rules (currently 9/10, 2 bytes differ)
4. Build clean oracle fixtures for fleet-battle scenario validation
5. Address minor invasion differences (1 byte PLANETS, 1 byte CONQUEST)

---

## Canonical Baseline Tools

- `cargo run -q -p ec-cli -- generate-gamestate <dir> <players> <year> [coords...]`
- `cargo run -q -p ec-cli -- compliance-report <dir>`
- `cargo run -q -p ec-cli -- core-validate-current-known-baseline <dir>`
- `cargo run -q -p ec-cli -- maint-rust <dir> [turns]` — Run Rust maintenance
- `cargo run -q -p ec-cli -- maint-compare <dir> [turns]` — Compare Rust vs ECMAINT
- `python3 tools/oracle_sweep.py`

## RE Focus Files

- [RE_NOTES.md](/home/mag/dev/esterian_conquest/RE_NOTES.md)
- [approach.md](/home/mag/dev/esterian_conquest/docs/approach.md)
- [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/rust-architecture.md)

Historical handoff detail:

- [next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md)

## Preservation TODO

- preserve original `ECGAME` ANSI opening/menu/report screens for the Rust client
- resume this once the local `ECGAME` harness is reliable enough or when UI preservation becomes the active milestone

### ⏳ Milestone 5: Game Event System — IN PROGRESS

**Definition:** ECMAINT mechanics emit typed maintenance/report events instead
of writing report strings inline. A single report-generation pass at the end
converts those events into `DATABASE.DAT`, `RESULTS.DAT`, and later any
justified `MESSAGES.DAT` output.

**Design sketch:**
```rust
enum MaintEvent {
    FleetBattleResolved { coords, participants, winner },
    PlanetIntelRefreshed { planet_id, viewer },
    PlanetOwnershipChanged { planet_id, from, to },
    ColonizationSucceeded { fleet_id, planet_id, player },
    ColonizationAborted { fleet_id, planet_id, owner, player },
    ScoutReport { fleet_id, planet_id, intel },
    MissionResolved { fleet_id, mission, outcome },
}
```

**Benefits:**
- One place for word-wrap, stardate formatting, Pascal string encoding
- Event list is independently testable without touching binary format
- Useful for a future Rust ECGAME client
- Matches likely internal ECMAINT structure (templated report strings)

**Current state:**

- combat maintenance now already emits:
  - `bombard_events`
  - `planet_intel_events`
  - `ownership_change_events`
  - `fleet_battle_events`
- [`rust/ec-cli/src/commands/reports.rs`](/home/mag/dev/esterian_conquest/rust/ec-cli/src/commands/reports.rs)
  now consumes that event surface to regenerate:
  - `DATABASE.DAT`
  - deterministic `RESULTS.DAT`
- `MESSAGES.DAT` still remains empty in the canonical Rust path because the
  current preserved maint corpus provides no non-empty maint-generated samples

**Acceptance criteria:**
- [x] typed maintenance/report events exist in `ec-data/src/maint/`
- [x] combat maintenance pushes events into a per-turn event buffer
- [x] report-generation pass consumes events → `DATABASE.DAT` / `RESULTS.DAT`
- [x] colonization outcomes emit first-class typed events
- [x] blocked colonization emits a first-class typed event and report
- [x] generic mission outcome events exist for current colonize / bombard / invade / blitz paths
- [x] scout mission arrivals emit first-class typed events
- [x] `ViewWorld` emits typed mission results and intel refresh
- [x] battle-driven mission aborts emit typed outcomes for move/view/scout
- [ ] scout intel/contact follow-up reports are modeled beyond arrival/intel
- [ ] no inline report string construction outside the report generation pass

---

### ⏳ Milestone 6: Reproduce ECMAINT Player Turn Reports — IN PROGRESS

**Definition:** Rust maintenance generates byte-exact MESSAGES.DAT and RESULTS.DAT content matching the original ECMAINT output for all scenario families.

**Context:** ECMAINT writes per-player turn reports into MESSAGES.DAT and RESULTS.DAT. ECGAME reads and displays these on player login. Built on top of the Milestone 5 game event system. The 2012 real-game player session logs in `original/v1.5/ec-logs-2012/` (ec2.txt–ec51.txt) are the primary human-readable source for what these reports look like and what triggers them. Reports cover:
- Fleet movement arrivals ("We have arrived at our destination...")
- Combat outcomes ("We were attacked by...", "We managed to destroy...")
- Colonization results ("We have successfully terraformed...")
- Guard/Blockade arrivals ("We are beginning our guarding/blockading assignment...")
- Scouting intel ("We are in extended orbit around planet...")
- Invasion outcomes ("We have been invaded and conquered...")
- Bombardment results ("We have just concluded a bombing run...")

**Known facts:**
- RESULTS.DAT is non-empty for fleet-battle and invade scenarios (combat reports)
- MESSAGES.DAT is empty in all known preserved maint post-states in the current
  fixture corpus
- Guard/blockade and econ-only turns produce empty RESULTS.DAT (no report generated)
- Report format: Pascal-style length-prefixed strings, word-wrapped at ~72 chars, with stardate header
- Reports are per-player: each player only sees reports about their own fleets/planets
- The ec-logs are the best oracle for report text, triggers, and formatting before doing binary RE

**Current state:**

- Rust now writes deterministic fixed-record `RESULTS.DAT` output from typed
  maintenance events
- the report writer now follows the observed 84-byte record family more closely:
  - family/type first byte
  - fixed trailing bytes by report family
  - multi-record chunking instead of single-record truncation
- this is structural and stylistic progress, not byte-exact parity yet
- the next additions should be event-driven reports for:
  - scout/recon outcomes
  - more mission-result categories beyond current combat summaries

**Acceptance criteria:**
- [ ] Byte-exact RESULTS.DAT match on fleet-battle scenario
- [ ] Byte-exact RESULTS.DAT match on invade scenario
- [ ] Byte-exact RESULTS.DAT match on bombard scenario (currently empty — verify)
- [ ] All scenario families produce correct MESSAGES.DAT / RESULTS.DAT
- [ ] Report format (word-wrap, stardate, sender/receiver addressing) matches original

**Immediate next steps:**

1. Add typed scout/contact follow-up events for reconnaissance intel and
   first-contact style reporting beyond the current arrival/intel reports.
2. Decide whether `JoinAnotherFleet` / `RendezvousSector` should become the
   next non-combat mission families promoted into the generic mission-outcome
  /report pipeline, or whether contact-identification should come first.
3. Keep refining `RESULTS.DAT` family formatting against preserved fixtures and
   historical session logs.
4. Do not add a canonical `MESSAGES.DAT` writer until a non-empty maint-driven
   sample is recovered.
