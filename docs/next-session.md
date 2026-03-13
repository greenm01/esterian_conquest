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

## Next Milestone: Rust ECMAINT Replacement

With Milestone 3 complete, the foundation is set for the ultimate goal: a full Rust reimplementation of ECMAINT. The path forward:

1. Use the compliant generator as the test oracle harness
2. Port ECMAINT mechanics incrementally, validating against the original binary
3. Eventually achieve 100% deterministic output match

---

## Canonical Baseline Tools

- `cargo run -q -p ec-cli -- generate-gamestate <dir> <players> <year> [coords...]`
- `cargo run -q -p ec-cli -- compliance-report <dir>`
- `cargo run -q -p ec-cli -- core-validate-current-known-baseline <dir>`
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
