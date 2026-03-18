> **Archival working document.** This file is the original chronological
> reverse-engineering notebook. Most findings have been formalized into
> dedicated spec docs in this directory. Consult those for canonical
> information; this file preserves the exploratory record.
>
> | Section | Status | Canonical doc |
> |---|---|---|
> | Session 2026-03-14 - growth rule | Superseded | `economics.md` |
> | RE Policy: Stochastic Mechanics | Canonical | `approach.md` §9 |
> | Current Status | Partially stale | Language confirmed as Borland Pascal |
> | Known Working Runtime / Launch Recipe | Stale | `dosbox-workflow.md` |
> | Door File Findings | Mixed | `dosbox-workflow.md` |
> | Reverse Engineering Notes | Stale | Compiler confirmed as Borland Pascal |
> | Data Files To Decode Next | Stale | All decoded in `ec-data` |
> | Draft File Layouts | Superseded | Rust record definitions in `ec-data` |
> | ECMAINT File-I/O Trace | Superseded | `ec-turn-cycle-spec.md` |
> | ECMAINT Live Memory Dump Anchors | Reference | Ghidra catalog, still unique |
> | ECUTIL Surface | Mixed | Unique setup utility notes |
> | Modern TUI Direction | Stale | `refactor-tui.md` |
> | ECMAINT Build/Fleet Scenarios | Superseded | Fixtures + spec docs |
> | Historical Combat References | Reference | Unique doc text captures |
> | Synthetic Bombardment Sequence | Superseded | `ec-combat-spec.md` |
> | Fleet Command Surface | Mixed | Some unique fleet order detail |
> | Porting Strategy / Preservation | Stale | `approach.md`, `rust-architecture.md` |
> | Session 2026-03-10: Invasions | Superseded | `ec-combat-spec.md` |
> | Session 2026-03-10: Starbases | Mixed | Some unique starbase detail |
> | Environment Setup Notes | Stale | `dosbox-workflow.md` |
> | Guard Starbase / Build Queue | Superseded | `ec-turn-cycle-spec.md` |
> | Oracle-First Method Shift | Stale | `approach.md` |
> | Economy sessions | Superseded | `economics.md` |
> | Fleet oracle verification | Superseded | `ec-turn-cycle-spec.md` |
> | ECMAINT timing/stardate | Superseded | `ec-timing-spec.md` |

# Esterian Conquest v1.5 RE Notes

## Session 2026-03-14 - Canonical planet growth rule adopted

- Rechecked the shipped docs:
  - empire tax rate is player/empire-wide, not per-planet
  - yearly tax revenue comes from current production
  - lower taxes help colonies develop current production faster
  - starbases improve growth and build capacity
- Built a dedicated mixed-production probe surface:
  - `ec-cli economy-tax-probe-init`
  - `ec-cli economy-report`
  - `tools/economy_tax_probe.py`
- The existing `ECMAINT /R` replay harness is still awkward for mutated economy
  experiments:
  - preserved replay fixtures advance correctly
  - once a directory is materially mutated for tax/growth probing, the current
    replay path can collapse to a no-op even when the directory still loads
    and passes basic oracle acceptance
- Adopted a documented canonical Rust rule in
  `rust/ec-data/src/maint/mod.rs`:
  - apply empire-wide tax to every owned active planet
  - add yearly tax revenue to `stored_goods_raw`
  - grow current production toward potential each year
  - lower tax => faster growth
  - friendly starbase => growth bonus and higher build capacity
  - civil-disorder fixture baselines are excluded so preserved maint fixtures
    remain stable
- Added focused regression coverage in `rust/ec-data/tests/production.rs`:
  - Real48 encode/decode round-trips common production values
  - lower tax produces faster colony growth than high tax
  - starbases accelerate colony growth

## RE Policy: Stochastic Mechanics

`ECMAINT` uses an internal RNG for combat (fleet battles, bombardment losses)
and rogue/autopilot AI. We do **not** attempt byte-exact reproduction of those
results. Instead:

- use oracle diffs to understand **which fields change** and **in what range**
- define our own **deterministic canonical rules** for the magnitude of random
  effects (documented below each mechanic as it is ported)
- byte-exact fixture match is the target only for fully deterministic mechanics
  (movement, year, build queues, economy totals, cross-file linking)
- see `docs/approach.md` §9 for the full rationale

## Current Status

- `ECGAME.EXE`, `ECUTIL.EXE`, and `ECMAINT.EXE` are 16-bit DOS `MZ` executables.
- `ECGAME.EXE` and `ECUTIL.EXE` both carry an `LZ91`/LZEXE-style wrapper.
- The game is runnable under a clean `DOSBox-X` setup.
- `dosemu` was not reliable for this target and produced misleading crashes.
- Stock `dosbox 0.74` was also not a reliable baseline for startup analysis.

Compiler/runtime evidence from `ECUTIL`:

- a DOSBox-X debugger memory dump of the live `ECUTIL` image exposed unpacked Borland runtime strings
- observed in `/tmp/ecinit/MEMDUMP.BIN`:
  - `Runtime error `
  - ` at `
  - `Portions Copyright (c) 1983,90 Borland`

Current best inference:

- `ECUTIL` was built with a Borland toolchain
- the runtime string style is a strong match for `Turbo Pascal` / `Borland Pascal`
- this is not a formal proof of source language yet, but it is materially stronger evidence than the earlier generic Borland guess

Matching evidence from `ECGAME`:

- a DOSBox-X debugger memory dump of the live `ECGAME` image exposed the same unpacked Borland runtime strings
- observed in `/tmp/ecgboot_chain/MEMDUMP.BIN`:
  - `Runtime error `
  - ` at `
  - `Portions Copyright (c) 1983,90 Borland`

Updated best inference:

- both `ECUTIL` and `ECGAME` were built with a Borland toolchain
- the shared runtime strings make `Turbo Pascal` / `Borland Pascal` the leading hypothesis for the codebase, not just a vague possibility
- it is still possible that some low-level routines were written in assembly, but the main application/runtime now looks Borland-derived

Unpacked `ECGAME` code-shape observations:

- the live memory image contains many `55 8B EC` stack-frame prologues
- many procedures return with `CB` / `CA nn 00` (`retf` / `retf n`), which is consistent with 16-bit large-model or Pascal-style far procedures
- repeated parameter handling patterns look like:
  - `LES DI, [BP+..]`
  - pointer/result writes via `ES:DI`
  - calls into shared helper routines followed by `retf`
- one cluster around offsets `0x315B..0x321F` in `/tmp/ecgboot_chain/MEMDUMP.BIN` looks like text/file helper code:
  - byte-by-byte reads
  - CR/LF handling
  - counted output writes
  - object/record callback-style calls through far pointers

Practical inference:

- the unpacked program layout looks much more like a Borland Pascal application linked against Borland RTL routines than a purely hand-written assembly program
- future static RE should treat repeated helper clusters as probable RTL/file I/O support and focus on higher-level callers, menu dispatch, and data-record updates

First useful unpacked anchors in `/tmp/ecgboot_chain/MEMDUMP.BIN`:

- filename table around `0x12D4..0x1339`:
  - stored as Pascal-style short strings with length bytes, e.g. `0B 'planets.dat'`, `09 'bases.dat'`, `0C 'messages.dat'`
  - entries include:
    - `planets.dat`
    - `bases.dat`
    - `messages.dat`
    - `results.dat`
    - `fleets.dat`
    - `ipbm.dat`
    - `player.dat`
    - `conquest.dat`
    - `Setup.dat`
    - `DataBase.dat`
- probable Borland-style file helper cluster around `0x4612..0x47D0`:
  - `int 21h / ah=3D` open
  - `int 21h / ah=3C` create
  - `int 21h / ah=3E` close
  - `int 21h / ah=3F` read
  - `int 21h / ah=40` write
  - `int 21h / ax=4200` seek
- this cluster also uses a small file-record/object structure with magic-like words `0xD7B0` and `0xD7B3`, plus a global error cell at `0x33FC`

Practical inference:

- the code near `0x4612..0x47D0` is almost certainly shared RTL or a thin Borland-style wrapper layer around DOS file I/O
- callers above that layer are the right place to hunt for game-specific logic such as loading `PLAYER.DAT`, `PLANETS.DAT`, and `FLEETS.DAT`
- the length-prefixed filename table is another concrete sign that the program was built around Pascal data conventions

First likely application-owned parser routine:

- procedure around `0x0B39..0x0D43` in `/tmp/ecgboot_chain/MEMDUMP.BIN`
- behavior from disassembly:
  - reads and normalizes a caller-supplied string buffer
  - trims leading/trailing spaces
  - classifies characters through helper calls
  - accumulates up to four extracted values into local slots
  - returns those extracted values through four output pointers

Why it matters:

- this looks like game/UI parsing logic, not generic Borland RTL
- it is a good first candidate for naming and reuse during future decompilation or porting

## Known Working Runtime

The most reliable environment found so far is:

- `DOSBox-X`
- `-defaultconf`
- `-nopromptfolder`
- `dosv=off`
- `machine=vgaonly`
- `core=normal`
- `cputype=386_prefetch`
- `cycles=fixed 3000`
- `xms=false`
- `ems=false`
- `umb=false`
- `output=surface`

Built binary used during testing:

- [`/tmp/dosbox-x/src/dosbox-x`](/tmp/dosbox-x/src/dosbox-x)

## Working Launch Recipe

First initialize game data with `ECUTIL.EXE` in a DOS game directory, then run `ECGAME`.

Example launch for `ECGAME`:

```bash
/tmp/dosbox-x/src/dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -defaultdir /tmp/ecgboot_chain \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c /tmp/ecgboot_chain" \
  -c "c:" \
  -c "mode co80" \
  -c "ECGAME"
```

Important detail:

- Running `ECGAME` directly from the game directory worked more reliably than `ECGAME C:\` in the synthetic local test setup.

## Door File Findings

- `ECGAME` does parse `CHAIN.TXT` successfully when given a sufficiently complete WWIV-style file.
- Minimal synthetic `CHAIN.TXT` files were rejected as invalid.
- A complete 32-line `CHAIN.TXT` format was required to clear the initial parser gate.
- Repo helper scripts now normalize this through `tools/ecgame_dropfiles.py`, which writes the known local `CHAIN.TXT` shape with explicit DOS CRLF line endings.
- Final local-play correction:
  - the successful local `CHAIN.TXT` shape also needed true local-console
    values, not remote/modem defaults
  - specifically:
    - line 15 `remote = 0`
    - line 20 `user baud = 0`
    - line 21 `COM port = 0`
    - line 31 `COM baud = 0`
  - before that correction, `ECGAME` wrote:
    - `ECGAME: could not find a Door File in path: \N`
  - after that correction:
    - DOS `type chain.txt` confirmed the new file was visible in the mounted
      directory
    - `ERRORS.TXT` stopped being regenerated on launch
    - this confirms the old parser failure was the dropfile shape, not missing
      game data
- `ECGAME` diplomacy byte mapping is now partially confirmed from a live menu
  run:
  - declaring empire `2` an enemy as player `1` changed exactly one relevant
    byte when compared against the same post-session neutral state:
    - player record `1`
    - offset `0x55`
    - `0x00 -> 0x01`
  - surrounding bytes in player 1's record:
    - neutral: `0x54..0x57 = 00 00 00 00`
    - enemy #2: `0x54..0x57 = 00 01 00 00`
  - current inferred mapping:
    - `PLAYER.DAT[player].raw[0x54 + (target_empire_raw - 1)]`
    - `0x00 = neutral`
    - `0x01 = enemy`
- A second repo-level harness bug was also fixed: multiple old `ECGAME` pexpect scripts were building argv correctly, then breaking it with `pexpect.spawn(" ".join(cmd), ...)`.
- Practical effect: `-c "DEBUGBOX ECGAME.EXE /L"` lost its quoting boundary and `/L` was being parsed by DOSBox-X itself instead of reaching `ECGAME`.
- Fresh evidence from the corrected boot-dump run:
  - the old bogus warning `Unknown option l` disappeared once argv was passed as a real argument vector
  - `tools/dump_ecgame_memory.py` resumed producing `/tmp/ecgame-dump/MEMDUMP.BIN`
- More importantly, once the corrected harness actually reached `ECGAME`, the program itself treated `/L` as a door-file path, not a local-mode switch:
  - `ERRORS.TXT` recorded `ECGAME: could not find a Door File in path: C:\/L\`
  - this explains the older contradictory `/L` behavior in stale scripts
- Current best local-launch rule:
  - use plain `ECGAME` / `ECGAME.EXE` with normalized `CHAIN.TXT` present in the game directory
  - do **not** rely on `/L` for local play on this build
  - for local console play, `CHAIN.TXT` must use local `0` baud/COM values,
    not remote modem values
- Corrected no-`/L` `DEBUGBOX` probing now proves the plain startup path really does enter the game process:
  - with `BPINT 21 3D` armed, `DEBUGBOX ECGAME.EXE` hits the first DOS open breakpoint
  - `DOS MCBS` at that stop shows:
    - `0813        622256     0814          ECGAME`
  - so the live `ECGAME` PSP is again confirmed as `0814` on the current local setup
- Important contrast:
  - the equivalent non-debug launches currently return immediately with no visible `ERRORS.TXT`
  - corrected headless runs of both:
    - `ECGAME.EXE`
    - `ECGAME.EXE C:\CHAIN.TXT`
    produced no observable DOSBox file-I/O log entries before returning to DOS
  - so the current reproducible startup hook is the debugger-assisted path, not plain non-debug execution
- First-open breakpoint state captured so far:
  - `AX=3D02`
  - `DS=44A1`
  - `EV AX BX CX DX SI DI BP SP DS ES SS` gives:
    - `AX=3D02`
    - `DX=A506`
    - `SI=FABE`
    - `DS=44A1`
  - dumping `DS:ESI` was all zeroes
  - dumping `DS:DX` (`44A1:A506`) yields the first startup-open filename:
    - `Setup.dat`
- Early startup-open finding:
  - on the corrected no-`/L` path, `ECGAME` first opens `Setup.dat`
  - this is the first concrete startup file-order fact recovered from the live debugger path
- Startup file-op sequence is now stable under the corrected debugger-assisted harness:
  - artifact:
    - `artifacts/ecgame-startup/startup-fileops.txt`
  - script:
    - `tools/capture_ecgame_startup_fileops.py`
  - confirmed sequence:
    1. open `Setup.dat` with mode `0x02`
    2. read `0x20A` bytes from handle `5` into `DS:44A1:0xA556`
    3. close handle `5`
    4. open `C:\CHAIN.TXT` with mode `0x00`
    5. read `0x80` bytes from handle `5` into `DS:44A1:0x40BC`
    6. close handle `5`
    7. terminate via `INT 21h / AH=4C` with exit code `0x1C`
  - consistency checks:
    - preserved `fixtures/ecutil-init/v1.5/SETUP.DAT` is `522` bytes
      (`0x20A`), exactly matching the first read count
    - generated local `CHAIN.TXT` is `107` bytes, so the `0x80` read is a
      partial-prefix read, not a full-file read
  - practical implication:
    - the post-`Setup.dat` non-open stop at `CS=4294 EIP=00000637` is not the
      next startup mystery anymore; it is the second file-open path and leads
      directly into a `CHAIN.TXT` prefix read before early process exit
- Current startup blocker is narrower now:
  - `ECGAME` is definitely reading both `SETUP.DAT` and the first `0x80` bytes
    of `CHAIN.TXT`
  - the next remaining question is why that `CHAIN.TXT` path terminates with
    exit code `0x1C` on the current local harness instead of proceeding into
    the richer door flow seen in older notes
- `CHAIN.TXT` buffer capture is now pinned down too:
  - artifact:
    - `artifacts/ecgame-startup/chain-buffer-summary.txt`
    - `artifacts/ecgame-startup/chain-buffer-prefix.bin`
  - script:
    - `tools/capture_ecgame_chain_buffer.py`
  - confirmed:
    - at the post-read close stop (`AX=3E01`), dumping the `0x80`-byte read
      buffer at `DS:40BC` shows the first `107` bytes exactly match the
      generated local `CHAIN.TXT`
    - bytes beyond EOF are stale scratch bytes, not parser output
  - practical implication:
    - the current local startup failure is not caused by `ECGAME` misreading
      the normalized dropfile prefix
    - the remaining failure is in the semantic validation/decision path after
      that successful prefix read
- Small `CHAIN.TXT` variant matrix now rules out several obvious causes:
  - artifact:
    - `artifacts/ecgame-startup/chain-variant-matrix.json`
  - script:
    - `tools/test_ecgame_chain_variants.py`
  - tested variants:
    - default normalized `CHAIN.TXT`
    - `first_name = HANNIBAL`
    - `remote = Y`
    - padded-to-128-byte default file
    - padded-to-128-byte `HANNIBAL` variant
  - all five variants still produce the same early DOS file-op sequence:
    - `3F00`
    - `3E00`
    - `3D00`
    - `3F01`
    - `3E01`
    - `4C00`
  - all five still terminate with exit code `0x1C`
  - practical implication:
    - the current local failure is not explained by:
      - short `CHAIN.TXT` length vs `0x80` read size
      - simple first-name content
      - the `remote` Y/N flag
    - next work should trace the semantic decision path after the successful
      `CHAIN.TXT` prefix read, not iterate more obvious dropfile-shape variants
- Dropfile auto-detection order is now confirmed:
  - artifact:
    - `artifacts/ecgame-startup/dropfile-probe.json`
  - script:
    - `tools/test_ecgame_dropfile_probe.py`
  - confirmed selection rules:
    - `chain_only`:
      - second open is `C:\CHAIN.TXT`
    - `door_only`:
      - second open is `C:\DOOR.SYS`
    - `both`:
      - second open is still `C:\CHAIN.TXT`
  - practical implication:
    - local plain `ECGAME` prefers `CHAIN.TXT` when both dropfile families are
      present
    - removing `CHAIN.TXT` is enough to force the `DOOR.SYS` parser path
- `DOOR.SYS` fallback path is materially different from the `CHAIN.TXT` path:
  - `chain_only` / `both` sequence:
    - `3F00`
    - `3E00`
    - `3D00`
    - `3F01`
    - `3E01`
    - `4C00`
  - `door_only` sequence:
    - `3F00`
    - `3E00`
    - `3D00`
    - `3FFF`
    - `3F30`
    - `3E01`
    - `4C00`
  - all three still end with exit code `0x1C`
  - practical implication:
    - `ECGAME` is not treating `DOOR.SYS` as just another alias for the
      `CHAIN.TXT` parser; it follows a distinct read path before failing
    - this is currently the best lead for recovering the local startup gate:
      trace why the `DOOR.SYS` fallback also exits with `0x1C`
- `DOOR.SYS` buffer capture is now fully characterized at the read level:
  - artifact:
    - `artifacts/ecgame-startup/door-buffer-summary.txt`
    - `artifacts/ecgame-startup/door-buffer-first.bin`
    - `artifacts/ecgame-startup/door-buffer-second.bin`
  - script:
    - `tools/capture_ecgame_door_buffers.py`
  - confirmed:
    - `DOOR.SYS` length is `250` bytes
    - first completed read:
      - reads the first `128` bytes exactly
      - buffer matches `DOOR.SYS[0:128]` byte-for-byte
    - second completed read:
      - fills the same `0x40BC` buffer with the remaining `122` bytes
      - buffer prefix matches `DOOR.SYS[128:250]`
      - bytes beyond the `122`-byte tail are stale scratch bytes
  - practical implication:
    - the fallback path is not failing on low-level `DOOR.SYS` I/O either
    - the remaining startup blocker is semantic validation/decision logic after
      a successful two-chunk `DOOR.SYS` read
- Legacy `DOOR.SYS` shape from the older fossil harness is now a concrete lead:
  - the legacy format from `tools/test_fossil_commission.py` differs materially
    from the current shared `write_door_sys()` output
  - quick format comparison:
    - current shared writer:
      - `250` bytes
      - modernized field count / line set
    - legacy fossil harness shape:
      - `124` bytes
      - different early line structure including `19200`, `8`, `1`, `19200`
        and a much shorter tail
  - dynamic result:
    - current shared `DOOR.SYS` still follows:
      - `3F00`
      - `3E00`
      - `3D00`
      - `3FFF`
      - `3F30`
      - `3E01`
      - `4C00`
      - exit `0x1C`
    - legacy fossil-style `DOOR.SYS` instead continues much deeper:
      - `3F00`
      - `3E00`
      - `3D00`
      - `3FFF`
      - `3F05`
      - `3F06`
      - `3F07`
      - `3F08`
      - `3F09`
      - `3F0A`
      - ...
      - later:
        - `3F10`
        - `3FFF`
        - `3F1A`
        - `3E01`
        - `4C00`
      - still eventually exits `0x1C`
  - practical implication:
    - the shared `write_door_sys()` layout is likely not faithful enough for
      the deeper local `ECGAME` path
    - the legacy fossil-style `DOOR.SYS` gets materially farther into the
      parser and is now the best lead for recovering a usable local harness
- First structural read-trace on the legacy `DOOR.SYS` path:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-reads.json`
  - script:
    - `tools/capture_ecgame_legacy_door_reads.py`
  - important correction:
    - the earlier naive model of "each `3Fnn` stop corresponds to a completed
      DOS read whose returned bytes should advance through the file" does not
      hold cleanly here
    - the dumped `0x40BC` buffer remains anchored to the same leading
      `DOOR.SYS` text prefix across many stops
  - useful stable facts:
    - the legacy path repeatedly hits `INT 21h / AH=3F` with:
      - handle `BX=5`
      - count `CX=0x80`
      - buffer `DX=0x40BC`
    - after the initial `3FFF`, the low byte in `AX` walks upward across many
      consecutive `3F` stops:
      - `3F05`
      - `3F06`
      - `3F07`
      - ...
      - `3F10`
      - later `3FFF`
      - then `3F1A`
    - that strongly suggests the legacy fallback is inside a deeper
      iterative parser/validator loop, not a single read-and-exit path
  - practical implication:
    - the next pass should not treat these stops as simple sequential file
      reads
    - the right next target is the code path around the recurring legacy
      `3F` loop, or a buffer/counter snapshot paired with that loop
- First semantic local-state result from that legacy loop:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-locals.json`
  - script:
    - `tools/capture_ecgame_legacy_door_locals.py`
  - confirmed stable frame facts:
    - through the `3F05..3F10` run:
      - `BP = F6A4`
      - `SP = F688`
      - `DX = 40BC`
      - `DI = 403C`
    - a local word at `SS:[BP+0x0C]` increments exactly with the observed
      `AH=3F` low-byte progression:
      - `3F05` -> `SS:[BP+0x0C] = 0x0006`
      - `3F06` -> `0x0007`
      - `3F07` -> `0x0008`
      - ...
      - `3F10` -> `0x0011`
  - practical interpretation:
    - this strongly looks like a parser-progress counter or current-field index
      inside the legacy `DOOR.SYS` loop
    - it is the first concrete semantic state variable recovered from the
      local `ECGAME` startup gate
  - additional clue:
    - the loop frame also contains an inline `COM` prefix near the same local
      region, consistent with token/field parsing from the `DOOR.SYS` text
- Tail-count matrix now shows the loop limit is fixed, not file-length-driven:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-tail-matrix.json`
  - script:
    - `tools/test_ecgame_legacy_door_tail_matrix.py`
  - tested variants:
    - legacy base shape plus `4`, `6`, `8`, `10`, and `12` trailing `90` lines
  - confirmed:
    - the stable loop-local limit remains `17` (`0x0011`) in every case
    - the loop-local current index still climbs toward that same fixed limit
    - extra trailing `90` lines only change the earlier low-byte pattern /
      starting index, not the fixed stable-loop limit
  - practical implication:
    - the deeper local parser is not validating an arbitrary-length dropfile
    - it appears to care about a fixed field window that tops out at
      field/index `17`
    - this sharply narrows the next RE target: focus on which early `DOOR.SYS`
      fields drive the fixed-limit loop and the later `3FFF` / `3F1A`
      transition, rather than on the long `90` tail
- Representative early-field subset mutations did not perturb that fixed window:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-field-subset.json`
  - script:
    - `tools/test_ecgame_legacy_door_field_subset.py`
  - tested representative mutations:
    - line `1`: `COM1:` -> `COM2:`
    - line `2`: `19200` -> `9600`
    - line `6`: `Y` -> `N`
    - line `10`: `Sysop First` -> `Alice`
    - line `13`: `1` -> `2`
    - line `16`: `9000` -> `100`
    - line `18`: `2` -> `1`
  - all tested variants still produced the same:
    - `3FFF`
    - `3F05..3F10`
    - `3FFF`
    - `3F1A`
    - `3E01`
    - `4C00`
    - stable loop-local progression `6 -> 17`
    - exit code `0x1C`
  - practical implication:
    - those representative transport/flag/name/numeric fields are not the
      primary discriminator for the current local startup failure
    - the next highest-value mutations should target the still-untested early
      fields in the fixed window, especially the dense `Y/Y/Y` flag run and
      the lines between the initial transport fields and the final numeric IDs
- Focused remaining-flag cluster mutations also came back negative:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-flag-cluster.json`
  - script:
    - `tools/test_ecgame_legacy_door_flag_cluster.py`
  - tested mutations:
    - line `7`: `Y` -> `N`
    - line `8`: `Y` -> `N`
    - line `9`: `Y` -> `N`
    - line `17`: `1` -> `2`
  - all still preserved:
    - `3FFF -> 3F05..3F10 -> 3FFF -> 3F1A -> 3E01 -> 4C00`
    - stable loop-local `6 -> 17`
    - exit code `0x1C`
  - practical implication:
    - the dense `Y/Y/Y` flag run also does not appear to be the primary
      discriminator for the current local startup gate
    - remaining likely causes are now:
      - still-untested early lines `3`, `4`, `5`, `11`, `12`, `14`, `15`
      - or a later semantic/code-side comparison that is insensitive to these
        obvious line-value tweaks
- The legacy parser phase boundary is now explicit:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-transition.txt`
  - script:
    - `tools/summarize_ecgame_legacy_door_transition.py`
  - confirmed:
    - stable loop phase ends at `3F10` with:
      - `[BP+0x0A] = 0x0011`
      - `[BP+0x0C] = 0x0011`
    - the next `3FFF` stop switches to a different frame shape:
      - `BP=F6A8`
      - `SP=F68A`
      - `SI=F8B8`
    - the later `3F1A` stop uses a third frame shape:
      - `BP=F6AE`
      - `SP=F692`
    - the old `0x0011` loop-limit pair is gone by that point
  - practical interpretation:
    - `3F10` completes the fixed early-field parser loop
    - control then transfers into a follow-on phase that repacks parser state
      before the later `0x1C` exit path
    - this is now a better next target than more broad value-mutation sweeps
- Handoff pointer capture now identifies the live dropfile stream object:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-handoff.json`
  - script:
    - `tools/capture_ecgame_legacy_door_handoff_buffers.py`
  - confirmed:
    - `DI=403C` is stable across `3F10`, `3FFF`, and `3F1A`
    - dumping `DS:403C` yields a consistent structure head:
      - `05 00`
      - `B1 D7`
      - `80 00`
      - `...`
      - `BC 40 A1 44`
      - `0C 06 94 42`
      - `E8 06 94 42`
    - practical interpretation of that object head:
      - handle-like word `0x0005`
      - Borland/RTL-style file-object magic near `0xD7B1`
      - buffer size `0x0080`
      - buffer pointer `44A1:40BC`
      - code/data pointers back into the live `4294:` image near `060C` and
        `06E8`
  - why it matters:
    - this strongly suggests `DI=403C` is the live dropfile stream/file object
      used by the parser loop
    - the recurring `AH=3F` loop is likely calling through that object or its
      adjacent helper layer rather than hand-rolling DOS calls directly
    - the `4294:060C` / `4294:06E8` pointers are now concrete code-adjacent
      anchors for the post-`3F10` handoff path
- Post-handoff code-hit capture now pins the local failure path much more
  tightly:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-code-hits.json`
  - script:
    - `tools/capture_ecgame_legacy_code_hits.py`
  - important dynamic rule:
    - the displayed `4294:` code addresses became reliable breakpoint targets
      only after arming them from the first live `BPINT 21 3D` startup stop
  - confirmed hit sequence on the legacy `DOOR.SYS` path:
    - `4294:06FC`
      - first post-loop handoff hit
      - state still matches the handoff phase:
        - `AX=3FFF`
        - `BX=0005`
        - `CX=0080`
        - `DX=40BC`
        - `SI=F8B8`
        - `DI=403C`
        - `BP=F6A8`
        - `SP=F68A`
      - implication:
        - `06FC` is on the real post-`3F10` handoff path
    - `4294:076D`
      - later close/error path hit multiple times with `AX=3E01`
      - one hit carries inline frame text:
        - `ECGAME: found invalid data in file: C:\DOOR.SYS`
      - implication:
        - `076D` is on or immediately adjacent to the invalid-dropfile
          reporter path
    - `4294:01A3`
      - final hit before process termination with `AX=4C67`
      - active stack text is:
        - `ECGAME: found an unexpected End Of File in File: C:\DOOR.SYS`
      - implication:
        - `01A3` sits on the EOF-report / final termination path
        - low exit byte `0x67` is the internal error selector before the
          later DOS termination code `0x1C`
  - practical conclusion:
    - the remaining local-startup blocker is now a narrow semantic parser rule
      in the legacy `DOOR.SYS` validator
    - it is no longer a low-level file I/O problem, a CRLF problem, or one of
      the already-tested obvious transport/flag field tweaks
- Once valid, `ECGAME` stopped writing `ERRORS.TXT` and proceeded into the door flow.

Current caveat:

- fixing dropfile generation and argv passing repaired the stale harness layer, but the remaining interactive/local-flow problem is now narrower:
  - several old `DEBUGBOX` scripts also forgot to issue `RUN`, so their fake "game input" was going to the debugger prompt rather than `ECGAME`
  - the first reliable pause point for the corrected no-`/L` path is file-open (`INT 21h / AH=3D`), not keyboard wait
  - plain non-debug startup still exits too early to give useful file-open traces, so immediate next RE should stay on the debugger-assisted first-open path
  - some old scripts still have brittle debugger prompt handling
  - the currently regenerated `MEMDUMP.BIN` images look like earlier-boot snapshots and still do not expose the later door/parser strings cited from the older `/tmp/ecgboot_chain` work

Useful test files created during analysis:

- `/tmp/canon_remote.txt`
- `/tmp/canon_local0.txt`
- `/tmp/canon_local1.txt`

## Initialization Findings

`ECUTIL.EXE` is required to initialize game state.

Observed effects after initializing a new game:

- [`BASES.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/BASES.DAT) was zeroed
- `IPBM.DAT` was created
- `MESSAGES.DAT` was created
- `RESULTS.DAT` was created
- other game data files remained in place but now behaved correctly under `ECGAME`

Without this initialization step, `ECGAME` would accept the door file but fail to reach the real game flow.

## Confirmed Working Game Flow

Observed live screens:

- initial door information screen
- ANSI prompt
- splash / registration screens
- first-time menu
- join flow
- main menu

Confirmed menu families:

- first-time flow
- main command loop
- report/database commands

Observed state gate:

- `Total Planet Database` reports that the planetary database is not yet loaded until maintenance has run

This suggests `ECMAINT.EXE` is important for later-state or year/turn progression.

## Reverse Engineering Notes

### What runtime work proved

- the game is not blocked on a live BBS connection
- the door/drop-file path is real and understood enough to emulate
- the core game logic can be executed locally
- the next RE work should focus on data formats and command handlers, not basic emulator compatibility

### Current language/toolchain assessment

Not confirmed yet.

Best current guess:

- Borland Pascal or Borland C/C++
- possibly mixed with handwritten x86 assembly in the startup/loader path

Reason this is not confirmed:

- the main binaries are still wrapped in an LZEXE-style packed/self-modifying loader
- a clean recovered compiler/runtime signature has not yet been extracted

## Data Files To Decode Next

Highest-value targets for a port:

- [`SETUP.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/SETUP.DAT)
- [`PLAYER.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/PLAYER.DAT)
- [`PLANETS.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/PLANETS.DAT)
- [`FLEETS.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/FLEETS.DAT)
- [`CONQUEST.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/CONQUEST.DAT)
- [`DATABASE.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/DATABASE.DAT)

Current observations:

- `SETUP.DAT` begins with `EC151`
- `PLAYER.DAT` appears to be fixed-record structured data with player/empire strings
- `PLANETS.DAT` appears to be fixed-record structured data with planet names and ownership/status strings

## Draft File Layouts

These are first-pass RE notes. Items marked `confirmed` are based on exact size/boundary checks. Items marked `inferred` still need action/diff validation.

### `PLAYER.DAT`

Status:

- confirmed size: `440` bytes
- older size split hypothesis (`5 x 88`) is superseded by `ECMAINT` runtime trace
- `ECMAINT` reads the file in `110`-byte strides at offsets `0`, `110`, `220`, `330`
- practical runtime model: `4` records of `110` bytes each

Why this is likely:

- DOSBox-X `INT 21h` / file-I/O tracing during `ECMAINT /R` showed:
  - seek `0`, read `110`
  - seek `110`, read `110`
  - seek `220`, read `110`
  - seek `330`, read `110`
  - seek `440`, EOF
- this is stronger evidence than the older visual split guess

Draft record layout for record 0:

- `0x00`: `u8` active/occupied flag or empire id (`01` in initialized joined state)
- `0x01..0x1A`: 26-byte padded uppercase handle / door username
- `0x1B`: `u8` length or string metadata (`0x09` for `niltempus`)
- `0x1C..0x2E`: 19-byte padded empire name
- `0x2F`: terminator/attribute byte (`0xEF` in joined state)
- `0x30..0x3F`: mostly zero in joined state
- `0x40..0x57`: small numeric fields, likely empire stats/options/status

Confirmed fields inside that tail block:

- `0x44..0x45`: empire starbase count (`u16`, 0 = no starbases)
  - shipped sample: `1`
  - after ECUTIL init: `0`
  - this field is checked by ECMAINT when resolving Guard Starbase orders
- `0x4E..0x4F`: last run year (`u16`)
  - shipped sample: `3022`
  - after ECUTIL init: `0`
- `0x51`: empire tax rate percentage
  - shipped sample: `65`
  - after initial join: `50`
  - after in-game tax change (`Tax rate: Empire` screen): `60`
- `0x52..0x55`: treasury (`u32` LongInt)

Observed joined-state strings:

- username: `MRBILL`
- empire name: `niltempusDisorder's`

Important note:

- records `1..4` still contain what looks like uninitialized/stale text fragments after join
- that suggests either:
  - only record 0 is currently meaningful in this test game, or
  - the remaining records are alternate empire slots with untouched garbage/shareware defaults

Observed original -> initialized changes:

- record `0` is the only heavily rewritten record
- `0x01..0x08`: handle changed from `HANNIBAL` to `MRBILL`
- `0x1C..0x2E`: empire name changed from `Empire Of Dustder's` to `niltempusDisorder's`
- `0x42..0x47` and `0x4C..0x51`: several small status/stat bytes changed during join
- records `1..4` only changed at a handful of numeric offsets:
  - record `1`: `0x15`, `0x56`
  - record `2`: `0x00`
  - record `3`: `0x14`, `0x16`, `0x20`, `0x21`
  - record `4`: `0x2A`, `0x2C`, `0x36`, `0x37`

Practical inference:

- `PLAYER.DAT` mixes one live empire slot with four additional slot records
- record `0` almost certainly contains the current caller's door identity plus empire-visible metadata
- later records already look like fixed-width structures, but not yet fully initialized for active play
- bytes `0x50..0x52` are likely a compact option/status cluster for the active empire
  - current observed values after tax edit: `01 3c 64`
  - the middle byte is confirmed tax rate
  - the role of `0x50` and `0x52` is still unknown

Runtime trace note:

- `ECMAINT`'s actual file walker is currently the best evidence for on-disk
  record geometry. Any older notes based on visual boundaries should be treated
  as provisional unless they match the runtime access pattern.

## ECMAINT File-I/O Trace Findings

Using DOSBox-X with `-log-int21` and `-log-fileio` on `ECMAINT /R` produced
high-confidence file access geometry for the maintenance engine.

Confirmed initial load order:

1. `CONQUEST.DAT`
2. `SETUP.DAT`
3. `PLAYER.DAT`
4. `PLANETS.DAT`
5. `FLEETS.DAT`
6. `BASES.DAT`
7. `IPBM.DAT`

Confirmed runtime record sizes from the successful trace:

- `PLAYER.DAT`: `4 x 110` bytes
- `PLANETS.DAT`: `20 x 97` bytes
- `FLEETS.DAT`: `16 x 54` bytes
- `BASES.DAT`: `35`-byte records
- `CONQUEST.DAT`: single `2085`-byte block
- `SETUP.DAT`: single `522`-byte block
- `DATABASE.DAT`: processed in `2000`-byte pages and later patched in `100`-byte slots

Practical inference:

- these runtime access widths are stronger evidence than earlier visual record
  guesses
- any Rust-side binary accessor geometry should be aligned to these trace-backed
  widths unless contradicted by stronger evidence

## Starbase 2 Integrity Gate Trace

A DOSBox-X file-I/O trace of the failing `test_starbase2_list.py` scenario shows
that the multi-starbase experiment aborts very early.

Observed behavior:

- `ECMAINT` completes the initial read sweep across:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
- then writes only to `ERRORS.TXT`
- no normal maintenance writeback occurs to:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `CONQUEST.DAT`
- no database/report generation path is reached

Observed `ERRORS.TXT` contents:

- `Game file(s) missing or failed integrity check!`
- `Attempting to restore game from last saved point...`
- `Backup game file(s) missing or failed integrity check`
- `Maintenance aborting...`
- `Unable to restore previous game - maintenance aborting`

Practical inference:

- the Starbase 2 test case is failing a front-loaded cross-file integrity gate,
  not a later starbase-order resolution branch
- the decisive consistency check happens after the first full read pass and
  before the normal maintenance mutation/report pipeline
- this narrows the static RE target: we should hunt for the integrity validator
  that consumes the initial `PLAYER` / `PLANETS` / `FLEETS` / `BASES` snapshot,
  not for a late combat/order handler

Trace comparison against the known-good Guard Starbase fixture:

- new reproducer/report generator:
  - `tools/compare_ecmaint_validation_trace.py`
  - outputs under `artifacts/ecmaint-validation-trace/`
- the passing Guard Starbase fixture and the failing raw Starbase 2 case share
  the same initial `CONQUEST` / `SETUP` / `PLAYER` / `PLANETS` / `FLEETS`
  read sweep
- first divergence is the initial `BASES.DAT` selector:
  - good case: `seek BASES.DAT offset=0`
  - failing case: `seek BASES.DAT offset=35`
  - both then read `35` bytes
- immediately after that:
  - good case continues into `IPBM.DAT` and `DATABASE.DAT`
  - failing case opens and writes `ERRORS.TXT`

Practical inference:

- the failing synthetic Starbase 2 path is already selecting the second base
  record before the abort
- that strengthens the `PLAYER[0x44] -> BASES` linkage hypothesis recovered
  from `0x25EE4`
- the remaining question is not “does ECMAINT notice base 2 at all?” but
  “what additional condition allows the selected second base record to survive
  the integrity validator without falling into the error/token path?”

Token gate investigation (resolved):

- The token-gate question is now closed. The detailed exploratory work is
  preserved in `docs/dev/archive/token-investigation.md`; the summary below is the final
  model that supersedes the earlier speculative token notes.
- Key static anchors:
  - `2000:9430`: global option/flag initializer
  - `2000:5EE4`: main cross-file integrity validator
  - `2000:6D9B`: top-level integrity/restore routine
  - `2000:7323`: `Setup.Tok` probe and early file setup
  - `2000:96C4`: named token existence checker
  - `2000:997C`: master token wait loop
  - `2000:9D48`: `Move.Tok` recovery path
  - `0x29120`: command-line parser

Final conclusions:

1. `DS:16A4` is a dead bypass flag.
   - The integrity validator at `2000:5EE4` checks `CMP byte ptr [0x16A4],0`.
   - Exhaustive static reference scanning shows `16A4` is only initialized to
     zero at `2000:9430` and is never set non-zero anywhere in the executable.
   - The command-line parser at `0x29120` sets `DS:16A2` when `/B` is passed,
     not `DS:16A4`.
   - Practical conclusion: this is a developer typo. The intended direct
     bypass mechanism is broken and unreachable.

2. `.TOK` files do not directly bypass the integrity validator.
   - The master loop at `2000:997C` is a BBS/node lock-file wait gate over the
     game token set.
   - `2000:96C4` is the generic token checker used by that loop and by other
     named-file probes.
   - Recognized tokens change runtime control flow, but they do not flip a
     hidden "skip integrity" global.

3. The real Starbase 2 "token bypass" is restore-from-backup.
   - After the token wait logic, ECMAINT checks for `Move.Tok`.
   - `Move.Tok` indicates that a previous maintenance run halted during the
     movement phase.
   - The recovery path restores `.SAV` backups over the working `.DAT` files
     before re-entering the main integrity path rooted at `2000:6D9B`.
   - Once the broken `.DAT` files have been replaced by valid backups, the
     normal integrity validator at `2000:5EE4` passes naturally.
   - Practical conclusion: `.TOK` files only appear to "bypass" the synthetic
     Starbase 2 failure because `Move.Tok` triggers automatic rollback to clean
     game state before validation.

Recovered command-line flag map:

- `/F` -> `DS:16A0 = 1`
- `/R` -> `DS:16A1 = 1`
- `/B` -> `DS:16A2 = 1` (intended bypass flag, but ineffective because
  `2000:5EE4` checks `16A4`)
- `/C` -> `DS:16A3 = 1`
- `/I` -> `DS:169C = 1`
- `/S=nnn` -> timeout value at `DS:16A6/16A8`
- `/Y=nnn` -> year override at `DS:169E`

Practical status:

- The token gate investigation is complete.
- Any future work on the maintenance engine should treat the token path as:
  - a lock/wait system for BBS node coordination, plus
  - a crash-recovery/restore entry point through `Move.Tok`,
  - not as an active integrity-bypass flag path.

## ECMAINT Live Memory Dump Anchors

A DOSBox-X debugger-assisted live dump of `ECMAINT` exposed the unpacked
maintenance image directly.

Working approach:

- launch DOSBox-X with `DEBUGBOX ECMAINT /R`
- set `BPINT 21 3D` to break on the first DOS file-open after the LZEXE stub
  has unpacked the real program
- at that breakpoint, `DOS MCBS` showed the `ECMAINT` block as:
  - PSP `0814`
  - allocation size `622256` bytes (`0x97EB0`)
- dump the full live block with:
  - `MEMDUMPBIN 0814:0000 97EB0`

Confirmed from `/tmp/ecmaint-debug/MEMDUMP.BIN`:

- Borland runtime strings are present:
  - `Runtime error `
  - `Portions Copyright (c) 1983,90 Borland`
- the startup/integrity failure strings from `ERRORS.TXT` are present in the
  live image, confirming the failure path is application code in the unpacked
  program

Useful dump offsets:

- `0x143B`: early filename/error text cluster beginning with `Planets.Dat`
- `0x26B86..0x26D97`: backup and primary filename tables followed by integrity
  and restore strings
- `0x26D98`: likely procedure start immediately after the integrity-string table
- `0x2841B..0x284E5`: `main.tok` startup guard strings including:
  - `Performing integrity check of game files...`
  - `Unable to restore previous game - maintenance aborting`

Practical inference:

- the integrity/restore logic is now anchored in the unpacked live image
- the code immediately following `0x26D98` is a strong first candidate for the
  front-loaded cross-file validator that rejects the synthetic Starbase 2 state
- future static RE should target the live dump rather than the packed
  `ECMAINT.EXE` stub

Ghidra follow-up on the live dump:

- importing `/tmp/ecmaint-debug/MEMDUMP.BIN` as a raw binary with processor
  `x86:LE:16:Real Mode` succeeded
- Ghidra recovered `280` functions from the dump
- the integrity-string anchor maps to Ghidra address `2000:6d98`
- the `main.tok` startup-guard cluster maps to `2000:841b`
- Ghidra did not auto-create a function at `2000:6d98`, so that region likely
  needs manual code/data carving even though surrounding areas disassemble well

First control-flow recovery from the integrity region:

- the code beginning at linear `0x26D9B` is a top-level integrity/restore
  routine even though Ghidra did not auto-create a function there because it
  immediately follows string data
- it takes one byte-like stack argument at `[bp+4]`
- observed behavior:
  - argument `0`: validate the primary game state
  - on failure, emit restore/abort messaging and recursively call itself with
    argument `1`
  - argument `1`: run the backup/restore-side validation path
- the recursive call is explicit:
  - `0x26F7C..0x26F81`: pushes `1` and calls back to `0x26D9B`

Most important helper under it:

- `0x25EE4` is the first substantial validator helper called by the top-level
  integrity routine
- early confirmed checks inside `0x25EE4`:
  - opens/reads DS:`3278` with size `0x006E` (`110`) -> strong match for
    `PLAYER.DAT`
  - opens/reads DS:`2F78` with size `0x0061` (`97`) -> strong match for
    `PLANETS.DAT`
  - opens/reads DS:`3178` with size `0x0036` (`54`) -> strong match for
    `FLEETS.DAT`
  - opens/reads DS:`2FF8` with size `0x0023` (`35`) -> strong match for
    `BASES.DAT`

First concrete `BASES.DAT` integrity logic recovered:

- after the initial `PLAYER` / `PLANETS` / `FLEETS` phases, helper `0x25EE4`
  enters the `BASES.DAT` pass at linear `0x263D3`
- for each player-derived entry, it reads one `BASES.DAT` record by index:
  - record index source: `PLAYER` entry field at offset `0x44`
  - code: `0x2643C` loads `es:[di+0x44]`, decrements it, and uses that as the
    record selector for the `BASES.DAT` reader
- after loading the base record into a stack-local buffer, it checks:
  - local byte `-0x88` against the current player index (`0x26488..0x264A0`)
- practical meaning:
  - `PLAYER.DAT[0x44]` is definitely part of the startup integrity relation
    between player records and base records
  - the validator does not trust `BASES.DAT[0x04] = 0x02` by itself; it first
    resolves the player-owned base index through `PLAYER.DAT[0x44]`

Follow-on linkage inside the same `BASES` validator:

- after the direct `PLAYER[0x44] -> BASES` record check, helper `0x25EE4`
  continues into a second `BASES` branch at `0x26582`
- this branch uses the loaded base-buffer word at offsets `0x05..0x06` as
  another base-record selector:
  - `0x26582..0x265A3` loads local word `-0x87(%bp)` and reads that base record
- after loading that secondary base record, it again checks buffer offset
  `0x04` against the current player index:
  - `0x265E2..0x265FA`
- the emitted summary entry for this branch also copies:
  - base `0x02..0x03`
  - base `0x0B..0x0C`
  - base `0x07..0x08`
  - base `0x19..0x1D`

Practical inference:

- the validator is not modeling bases as isolated records
- there is at least one additional base-to-base linkage field at
  `BASES[0x05..0x06]`
- this strengthens the current hypothesis that a true second-base state needs a
  consistent internal linkage structure, not just `PLAYER[0x44] = 2` plus a
  second record with `BASES[0x04] = 2`

Direct fixture probing of `BASES[0x05..0x06]`:

- new reproducer: `tools/test_starbase_link_gate.py`
- using the otherwise accepted duplicate-base case
  (`BASES[0x02] = 0x01`, `BASES[0x04] = 0x01` on the second record):
  - `BASES[0x05..0x06] = 00 00` => accepted
  - `BASES[0x05..0x06] = 01 00` => accepted
  - `BASES[0x05..0x06] = 00 01` => early integrity abort
  - `BASES[0x05..0x06] = 01 01` => early integrity abort
  - `BASES[0x05..0x06] = 02 00` => `Unable to allocate memory.`

Practical inference:

- `BASES[0x05..0x06]` behaves like a little-endian selector / linkage word
- `0x0001` is a plausible valid reference in the current one-base-compatible
  state
- `0x0100` is not a byte-swap-tolerant encoding; it is invalid and hits the
  early integrity gate
- `0x0002` appears to drive the program into a bad self-/second-record path
  severe enough to trigger an allocation failure rather than a clean integrity
  error

First stable accepted multi-record `BASES` state:

- a clean reproducer exists inside `tools/test_starbase_link_gate.py`:
  - base 1: `0x08 = 0x00`
  - base 2:
    - `0x00 = 0x02`
    - `0x02 = 0x01`
    - `0x04 = 0x01`
    - `0x05..0x06 = 0x0001`
    - `0x07 = 0x01`
    - `0x08 = 0x00`
- observed result after `ECMAINT`:
  - no `ERRORS.TXT`
  - `PLAYER.DAT[0x44..0x47]` remains `02 00 02 00`
  - `BASES.DAT` remains `70` bytes (two records)
  - both records are canonicalized to the same 9-byte header:
    - `02 00 01 00 01 01 00 01 00`
- this state survives a second maintenance pass unchanged

Practical inference:

- the validator can accept a multi-record `BASES.DAT` state without collapsing
  it back to one record if the linkage fields are internally consistent
- however, the accepted state still duplicates starbase identity `0x04 = 0x01`,
  so it is best described as a stable duplicated-base structure, not yet proof
  of a valid true “Starbase 2” identity

Promotion attempts from the stable duplicated-base state:

- starting from the accepted duplicated-base layout, the following all still
  fail the early integrity gate when base 2 is promoted to `BASES[0x04] = 0x02`:
  - `BASES[0x02] = 0x01`, `BASES[0x05..0x06] = 0x0001`
  - `BASES[0x02] = 0x01`, `BASES[0x05..0x06] = 0x0000`
  - `BASES[0x02] = 0x01`, `BASES[0x05..0x06] = 0x0002`
  - `BASES[0x02] = 0x02`, `BASES[0x05..0x06] = 0x0001`
  - `BASES[0x02] = 0x02`, `BASES[0x05..0x06] = 0x0000`

Practical inference:

- the missing second-base precondition is not solved by local tweaks to
  `BASES[0x02]` or `BASES[0x05..0x06]` around the accepted duplicated-base
  layout
- the next unexplained input is likely outside this immediate base-header
  neighborhood, or requires a coordinated update to another file/structure that
  the early validator consumes

Update: recognized `.TOK` marker files are the missing gate

- new reproducer: `tools/test_starbase2_tok_gate.py`
- the raw two-base construction
  (`PLAYER[0x44..0x47] = 02 00 02 00`, second `BASES` record with
  `BASES[0x04] = 0x02`) still fails with no token files present
- adding a single zero-length recognized token file makes the same state pass:
  - `MAIN.TOK` => pass
  - `PLAYER.TOK` => pass
  - earlier spot checks also showed `PLANETS.TOK`, `FLEETS.TOK`,
    `DATABASE.TOK`, and `CONQUEST.TOK` each work alone
- an arbitrary marker name is not enough:
  - `FOO.TOK` => still fails the integrity check
- the accepted `MAIN.TOK` / `PLAYER.TOK` cases survive a second `ECMAINT` pass
  unchanged, so this is not just a one-pass normalization artifact

Practical inference:

- the remaining Starbase 2 blocker was not another hidden byte inside the
  `BASES` record
- `ECMAINT` has a separate mode/path keyed by specific `*.TOK` marker names
- once a recognized token marker is present, the raw Starbase 2 construction is
  accepted without needing the earlier canonicalized `BASES` / `FLEETS` state

Live-dump anchors for the token path:

- `main.tok` string cluster is at linear `0x2841B`
  - adjacent messages:
    - `Error - Previous maintenance halted prematurely.`
    - `Performing integrity check of game files...`
    - `Unable to restore previous game - maintenance aborting`
- `conquest.tok` string cluster is at linear `0x26FC6`
  - adjacent messages:
    - `Timeout occured for deletion of token file "conquest.tok"`
    - `Ignoring and continuing...`
    - `Will manually remove token file "Conquest.Tok"...`
    - `Unable to open file "Conquest.Dat"`
- another token-management string cluster begins around `0x29680`:
  - `Waiting for token file`
  - `Disk I/O error - Unable to delete token file`

Practical inference:

- `main.tok` is tied to the startup / previous-maintenance guard path
- `conquest.tok` is tied to token deletion / cleanup during the run
- the token gate is not just a passive file-exists check; there is explicit
  management code for named token files in the live image

Additional player-side linkage:

- after the `BASES` branches, the validator enters another phase at `0x2675A`
  driven by `PLAYER` offset `0x48`
- it reads DS:`31F8` records of size `0x20` using `PLAYER[0x48]` as an index
- baseline shipped state has `PLAYER[0x48..0x49] = 0x0000`, so this path is
  dormant in the one-base scenario

Direct fixture confirmation:

- updated reproducer: `tools/test_player48_gate.py`
- on the original one-base shipped baseline:
  - `PLAYER[0x48] = 0` with empty `IPBM.DAT` => `ECMAINT` succeeds
  - `PLAYER[0x48] = 1` with empty `IPBM.DAT` => immediate integrity abort
  - `PLAYER[0x48] = 1` with `IPBM.DAT` length `0x20` => success
  - `PLAYER[0x48] = 2` with `IPBM.DAT` length `0x40` => success
  - `PLAYER[0x48] = 3` with `IPBM.DAT` length `0x60` => success
  - mismatched counts (`2` with `0x20`, `3` with `0x40`) fail
- practical conclusion:
  - `PLAYER[0x48]` is the count of `0x20`-byte `IPBM.DAT` records
  - this validator path is about planetary missile data, not hidden starbase
    metadata

Practical inference:

- `PLAYER[0x48]` is no longer a starbase candidate
- DS:`31F8` corresponds to the `IPBM.DAT` record stream
- the remaining two-base blocker is back to the `BASES`-side integrity logic,
  especially the direct `PLAYER[0x44] -> BASES` path and the secondary
  base-to-base linkage through loaded base offset `0x05..0x06`

Static `IPBM.DAT` branch report:

- new headless Ghidra script:
  - `tools/ghidra_scripts_tmp/Report5EE4IPBM.java`
- artifact:
  - `artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt`
- concrete control-flow anchors inside `2000:5EE4`:
  - `2000:675A..68E8` = player-indexed `IPBM.DAT` branch
  - `2000:68E9..69B8` = follow-on summary branch after the first `IPBM` pass
- confirmed static behavior:
  - the branch starts by treating DS:`31F8` as a `0x20`-byte record stream:
    - `MOV DI,0x31f8`
    - `MOV AX,0x20`
    - `CALLF 0x3000:4f7a`
  - player iteration count is driven by global `0x16AE`, which already tracks
    the validated `PLAYER.DAT` entries from the earlier pass
  - for each player entry:
    - load player pointer via table at `0x16AC`
    - test `ES:[DI+0x48]`
    - if `PLAYER[0x48] == 0`, skip the `IPBM` read path
    - otherwise read `IPBM` record index `PLAYER[0x48] - 1`
  - the indexed read uses:
    - DS:`31F8` as the source stream
    - scratch buffer DS:`3538`
    - helper `0x3000:50CD` for indexed record fetch
    - helper `0x3000:502F` to copy/normalize the fetched record into DS:`3538`
  - on successful validation, the branch appends a `0x0C`-byte summary entry
    through the pointer table at `0x2F72` while incrementing `0x2F76`
- concrete summary-field writes observed in the first branch:
  - summary `+0x00` = player index from local loop counter, unless the dead
    bypass path at `0x16A4` overrides it with `DS:353A`
  - summary `+0x01` = `DS:3541`
  - summary `+0x02` = `DS:3542`
  - summary `+0x04` = constant `0x03`
  - summary `+0x05` = success/failure bit from helper `0x3000:488D`
  - summary `+0x06` = original `PLAYER[0x48]`
  - summary `+0x0A` = word from DS:`3538`
- follow-on branch at `2000:6906`:
  - gated by word `DS:353B`
  - reuses the same DS:`31F8` stream and DS:`3538` scratch buffer
  - appends more `0x0C`-byte entries via `0x2F72` / `0x2F76`
- practical consequence:
  - `IPBM.DAT` is not just length-validated; `2000:5EE4` also builds a
    structured in-memory summary from the fetched `0x20`-byte records
  - the next `IPBM` RE task is now narrower: name the DS:`3538..3553` scratch
    fields and determine what `DS:353B` represents in the second branch

Scalar sweep of the `IPBM` scratch fields:

- new headless Ghidra script:
  - `tools/ghidra_scripts_tmp/ReportIPBMScratchScalarUses.java`
- artifact:
  - `artifacts/ghidra/ecmaint-live/ipbm-scratch-uses.txt`
- important result:
  - the useful accesses to `DS:3538..3553` are not limited to `2000:5EE4`
  - a separate function currently carved as `0000:02c0` both writes and reads
    a larger field family around:
    - `3541`, `3543..3547`
    - `3542`, `3549..354d`
    - `354f..3553`
- concrete write cluster in `0000:02c0`:
  - `0e4c` writes `3541`
  - `0e5d..0e64` writes `3543..3547`
  - `0e76` writes `3542`
  - `0e87..0e8e` writes `3549..354d`
  - `0e9b..0ea2` writes `354f..3553`
- concrete read cluster in the same function:
  - `06d7` reads `3541`
  - `06e3..06eb` reads `3543..3547`
  - `06fd` reads `3542`
  - `0709..0711` reads `3549..354d`
  - `0765..076c` reads `354f..3553`
- practical inference:
  - DS:`3538..3553` is a structured scratch/state block, not just a single
    copied `IPBM` record image
  - the layout strongly suggests:
    - two tagged values or tagged tuples rooted at `3541` and `3542`
    - multiple associated word triples (`3543..3547`, `3549..354d`,
      `354f..3553`)
  - `2000:5EE4` is consuming already-normalized fields from that shared block
    rather than interpreting raw `IPBM` bytes directly
- second-branch refinement:
  - `2000:6906` gates on word `353B`
  - `2000:6A4B` later copies word `353D` into summary offset `+0x06`
  - practical meaning:
  - `353B` / `353D` form another paired result from the shared `IPBM`
      normalization block and should be reversed together

`0000:02C0` summary-dispatch function:

- new headless Ghidra script:
  - `tools/ghidra_scripts_tmp/ReportIPBMScratchFunction.java`
- artifact:
  - `artifacts/ghidra/ecmaint-live/ipbm-scratch-function.txt`
- concrete entry behavior:
  - takes a summary-entry index in `[BP+4]`
  - indexes through the summary pointer table at `0x2F72`
  - dispatches on summary kind byte `ES:[DI+4]`
  - confirmed branches:
    - kind `1` -> uses scratch block rooted at `0x3502`
    - kind `2` -> uses scratch block rooted at `0x3558`
    - kind `3` -> uses the `IPBM` scratch block rooted at `0x3538`
- kind `3` branch specifics:
  - copies / loads from `0x3538`
  - consumes normalized fields at:
    - `3541`, `3543..3547`
    - `3542`, `3549..354d`
    - `354f..3553`
    - `3555..3557`
  - then continues into generic comparison/normalization logic shared with the
    other kinds
- practical inference:
  - `0000:02C0` is not an `IPBM`-specific parser
  - it is a generic summary-entry dispatcher / normalizer that reuses the same
    downstream logic for multiple summary kinds, one of which is the `IPBM`
    kind emitted by `2000:5EE4`
  - the `IPBM` task is therefore split:
    - `2000:5EE4` emits kind-`3` summary entries from `PLAYER[0x48]`
    - `0000:02C0` later consumes those kind-`3` entries through the shared
      summary-processing machinery
  - `0000:02C0` also eventually writes normalized data back out:
    - summary entry fields `+0x01`, `+0x02`, `+0x05`
    - kind-specific scratch blocks rooted at `3502`, `3538`, and `3558`
  - so the current best model is "generic round-trip summary normalizer",
    not a one-way consumer of pre-normalized `IPBM` state

Mis-carved low-level helper caveat:

- new report:
  - `artifacts/ghidra/ecmaint-live/summary-kind-helpers.txt`
- the apparent helper entries `2000:C067`, `2000:C09A`, and `2000:C0CD` are
  not yet trustworthy semantic function starts in the raw import
- at least `2000:C0CD` clearly behaves like a tiny counted-string/byte-copy
  helper rather than a high-level kind parser
- follow-up region dump:
  - `artifacts/ghidra/ecmaint-live/summary-helper-region.txt`
  - `2000:C0DC..C0FD` is a clean bounded counted-string copy helper
  - `2000:C0CD` still looks like a tail-entry or raw-import misalignment,
    not the real semantic start of an `IPBM` routine
- practical consequence:
  - the real semantic target remains `0000:02C0` and the scratch layouts it
    uses, not the current misleading helper names around `C067..C0CD`

Follow-up correction on the kind-`3` helper model:

- new headless Ghidra scripts:
  - `tools/ghidra_scripts_tmp/ReportIPBMNormalizer.java`
  - `tools/ghidra_scripts_tmp/ReportSummaryHelperRegion.java`
- new artifacts:
  - `artifacts/ghidra/ecmaint-live/ipbm-normalizer.txt`
  - `artifacts/ghidra/ecmaint-live/summary-helper-region.txt`
- corrected result:
  - the direct call target from `0000:02C0` is still `2000:C0CD`
  - but the bytes at `2000:C0CD` decode only as a tiny copy tail
  - the nearby clean helper start is `2000:C0DC`, which takes bounded
    counted-string copy arguments from the stack
  - so `C0CD` should not be treated as the semantic kind-`3` normalizer
- practical implication:
  - the real `IPBM` meaning is still concentrated in `0000:02C0`
  - the next static task is to understand the common post-kind pipeline from
    `0000:07DA` onward, where `0000:02C0` compares, combines, and writes the
    normalized values back into the summary entry and scratch blocks

Focused post-kind pipeline result:

- new headless Ghidra script:
  - `tools/ghidra_scripts_tmp/ReportIPBMPostKindPipeline.java`
- new artifact:
  - `artifacts/ghidra/ecmaint-live/ipbm-postkind-pipeline.txt`
- concrete structure inside `0000:07DA..0EA6`:
  - the pipeline starts by converting local kind-count byte `[BP-0x19]` into a
    3-word value and scaling it by literal `0x86`
  - it then works over three local normalized tuples:
    - tuple A at `[BP-0x06 .. -0x02]`
    - tuple B at `[BP-0x12 .. -0x0E]`
    - tuple C at `[BP-0x24 .. -0x20]`
  - first branch:
    - if tuple A equals the first auxiliary tuple and tuple B equals the second
      auxiliary tuple, and tuple C passes the `0x3000:488D` comparison, it
      skips the combine path
  - otherwise it builds/updates auxiliary tuples at `[BP-0x30..-0x2C]` and
    `[BP-0x3C..-0x38]` using helper family `0x3000:488D`, `0x3000:4871`,
    `0x3000:4883`, `0x3000:487D`, and `0x2000:4E2D`
  - practical reading:
    - this is common canonicalization / merge logic over three normalized
      values, not simple field copying
- writeback stage at `0000:0BE8..`:
  - writes the finalized tuple C-derived boolean back to summary offset `+0x05`
  - writes canonicalized tuple A / tuple B tags back to summary offsets
    `+0x01` and `+0x02`
  - then dispatches on summary kind and writes the canonicalized tuples back to
    the corresponding scratch block (`3502`, `3558`, or later `3538`)
- practical consequence:
  - `0000:02C0` is acting as a summary-entry normalizer/coalescer
  - for kind `3`, the `IPBM` scratch block is not just "decoded state"; it is
    also the destination of a later canonicalized rewrite
  - the next semantic RE target is now sharper:
    - identify what the three tuple families A/B/C represent for kind `3`
    - then correlate those tuple roles with the live `3538` baseline capture

Tail transition / kind split clarification:

- new headless Ghidra script:
  - `tools/ghidra_scripts_tmp/ReportIPBMTailTransition.java`
- new artifact:
  - `artifacts/ghidra/ecmaint-live/ipbm-tail-transition.txt`
- confirmed control flow at `0000:0DE9..0EC8`:
  - common writeback always updates summary offsets:
    - `+0x05` from the finalized tuple-C-derived boolean
    - `+0x01` from tuple A via `0x3000:4895`
    - `+0x02` from tuple B via `0x3000:4895`
  - kind `2` then takes an additional side path through stack buffer
    `BP+0xF7B6` and helper `0x2000:C100`, after which it skips directly to the
    shared tail at `0x0EA6`
  - kind `3` skips that kind-2-only path and instead writes the finalized
    tuples back into the `IPBM` scratch block:
    - tuple A -> `3541`, `3543..3547`
    - tuple B -> `3542`, `3549..354d`
    - tuple C -> `354f..3553`
- practical implication:
  - tuple A / tuple B are definitely the two single-byte-plus-word-triple
    families in the kind-`3` block
  - tuple C is definitely the trailing word triple `354f..3553`
  - `3555..3557` are therefore not part of the main A/B/C writeback and should
    be treated as a separate trailing kind-`3` control group

Refined kind-`3` scratch layout boundary:

- follow-up artifact review across:
  - `artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt`
  - `artifacts/ghidra/ecmaint-live/ipbm-scratch-uses.txt`
  - `artifacts/ghidra/ecmaint-live/ipbm-scratch-function.txt`
- current confinement:
  - `353D` is only consumed by the second `IPBM` branch in `2000:5EE4`
    (`2000:6A4B -> summary +0x06`)
  - `3555..3557` are only visible in the kind-`3` path inside `0000:02C0`
- practical inference:
  - kind `3` appears to use at least two related normalized field groups:
    - primary group: `3541`, `3543..3547`, `3542`, `3549..354d`,
      `354f..3553`
    - trailing group: `3555..3557`
  - the second `5EE4` branch likely consumes a separate follow-on result pair
    `353B` / `353D` produced by the same overall normalization flow, but not
    by the generic dispatcher's trailing `3555..3557` block
  - the next semantic RE question is no longer “where are these fields used?”
    but “what real game concepts do the primary group, trailing group, and
    `353B/353D` pair encode?”

First live kind-`3` scratch snapshot:

- dynamic case:
  - `/tmp/ecmaint-debug-ipbm`
  - `PLAYER[0x48] = 1`
  - `IPBM.DAT` length `0x20`
  - record contents all zero
- live breakpoint:
  - `BP 2814:6870`
  - DOSBox-X stop reported as `2895:6060`, which is the first summary write
    from DS:`3538` inside `2000:5EE4`
- register snapshot:
  - `CS=2895 EIP=6060 DS=3529 ES=59F9 SS=39AB`
  - `SP=F9CA BP=FB22 AX=0048 BX=59F9 CX=0000 DX=59F9 SI=59F9 DI=0000`
- preserved artifacts:
  - `artifacts/ecmaint-ipbm-debug/registers-6870.txt`
  - `artifacts/ecmaint-ipbm-debug/scratch-3538-6870.txt`
- dumped bytes from `DS:3538` (`32` bytes):
  - `00 00 00 00 00 01 00 00 00 00 00 80 00 00 00 00`
  - `00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00`
- field-level interpretation from that zero-record baseline:
  - `3538 = 0x0000`
  - `353A = 0x00`
  - `353B = 0x0000`
  - `353D = 0x0001`
  - `3541 = 0x00`
  - `3542 = 0x00`
  - `3543 = 0x0080`
  - `3545 = 0x0000`
  - `3547 = 0x0000`
  - `3549 = 0x0080`
  - `354B = 0x0000`
  - `354D = 0x0000`
  - `354F = 0x0000`
  - `3551 = 0x0000`
  - `3553 = 0x0000`
  - `3555 = 0x00`
  - `3556 = 0x00`
  - `3557 = 0x00`
- practical consequence:
  - the zeroed valid record establishes a baseline normalization shape
  - `353D = 1` is now the strongest current candidate for the second-branch
    follow-on count / resolved record selector copied at `2000:6A4B`
  - `3543` and `3549` defaulting to `0x0080` suggests they are normalized
    constants or default magnitudes rather than copied raw bytes

First mutated `IPBM` correlation point:

- dynamic case:
  - `/tmp/ecmaint-debug-ipbm`
  - `PLAYER[0x48] = 1`
  - `IPBM.DAT[0x00] = 0x01`
  - all other `IPBM` bytes zero
- live breakpoint:
  - same first summary-write stop at live `2814:6870`
  - DOSBox-X again stopped at `2895:6060`
- preserved artifacts:
  - `artifacts/ecmaint-ipbm-debug/off_00_val_01-registers.txt`
  - `artifacts/ecmaint-ipbm-debug/off_00_val_01-scratch.txt`
- dumped bytes from `DS:3538` (`32` bytes):
  - `01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00`
  - `00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00`
- delta vs baseline:
  - `3538` changed from `0x0000` to `0x0001`
  - baseline `353D = 0x0001` was cleared to `0x0000`
  - baseline `3543 = 0x0080` was cleared to `0x0000`
  - baseline `3549 = 0x0080` was cleared to `0x0000`
- practical implication:
  - raw `IPBM` offset `0x00` definitely feeds the main tuple-C / summary-`+0x0A`
    word path rooted at `3538`
  - it also suppresses the zero-record default normalization that previously
    produced `353D = 1` and the paired `0x0080` defaults in tuple A / tuple B
  - for Rust-side compliance work, `IPBM[0x00]` is now the first confirmed
    byte with strong downstream effects across both the early `5EE4` summary
    emission and the later kind-`3` normalized state

Second mutated `IPBM` correlation point:

- dynamic case:
  - `/tmp/ecmaint-debug-ipbm`
  - `PLAYER[0x48] = 1`
  - `IPBM.DAT[0x01] = 0x01`
  - all other `IPBM` bytes zero
- preserved artifacts:
  - `artifacts/ecmaint-ipbm-debug/off_01_val_01-registers.txt`
  - `artifacts/ecmaint-ipbm-debug/off_01_val_01-scratch.txt`
- dumped bytes from `DS:3538` (`32` bytes):
  - `00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00`
  - `00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00`
- delta vs baseline:
  - `3538` changed from `0x0000` to `0x0100`
  - baseline `353D = 0x0001` was cleared to `0x0000`
  - baseline `3543 = 0x0080` was cleared to `0x0000`
  - baseline `3549 = 0x0080` was cleared to `0x0000`
- practical implication:
  - raw `IPBM` offsets `0x00..0x01` map directly into `3538` as a little-endian
    word
  - so tuple C / early summary `+0x0A` is now confirmed to derive from the
    first `u16` in the raw `IPBM` record
  - the same non-zero-first-word condition also suppresses the zero-record
    default normalization that produced `353D = 1` and the paired `0x0080`
    defaults in tuple A / tuple B

Expanded raw-to-scratch mapping for the `IPBM` record prefix:

- additional dynamic cases:
  - `/tmp/ecmaint-debug-ipbm`
  - `PLAYER[0x48] = 1`
  - one non-zero byte in `IPBM.DAT`; all other bytes zero
  - same first summary-write stop at live `2814:6870`
- preserved scratch artifacts:
  - `artifacts/ecmaint-ipbm-debug/off_02_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_03_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_04_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_05_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_06_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_07_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_09_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_0a_val_01-scratch.txt`
- observed one-byte deltas:
  - `IPBM[0x02] = 0x01` -> scratch byte `353A = 0x01`
  - `IPBM[0x03] = 0x01` -> scratch byte `353B = 0x01`
  - `IPBM[0x04] = 0x01` -> scratch byte `353C = 0x01`
  - `IPBM[0x05] = 0x01` -> scratch byte `353D = 0x01`
  - `IPBM[0x06] = 0x01` -> scratch byte `353E = 0x01`
  - `IPBM[0x07] = 0x01` -> scratch byte `353F = 0x01`
  - `IPBM[0x09] = 0x01` -> scratch byte `3541 = 0x01`
  - `IPBM[0x0A] = 0x01` -> scratch byte `3542 = 0x01`
- combined with the earlier `0x00` / `0x01` probes and `2000:5EE4` field uses:
  - the front of the raw `0x20`-byte record is now confirmed to copy
    contiguously into the scratch block rooted at `3538`
  - the first interpreted fields are:
    - raw `0x00..0x01` -> scratch `3538..3539` -> `u16` copied to summary
      `+0x0A`
    - raw `0x02` -> scratch `353A` -> player / empire byte copied to summary
      `+0x00` in the non-bypass path
    - raw `0x03..0x04` -> scratch `353B..353C` -> non-aligned `u16` that
      gates the second `IPBM` branch (`CMP word ptr [0x353b],0`)
    - raw `0x05..0x06` -> scratch `353D..353E` -> non-aligned `u16` used by
      the second branch when it writes summary `+0x06`
    - raw `0x09` -> scratch `3541` -> kind-`3` summary tag byte written to
      summary `+0x01`
    - raw `0x0A` -> scratch `3542` -> kind-`3` summary tag byte written to
      summary `+0x02`
- practical correction:
  - the apparent `353B` and `353D` "words" are not aligned raw fields at
    offsets `0x02` and `0x04`; they are overlapping interpretations over the
    contiguous copied byte stream
  - baseline all-zero normalization still adds derived defaults like
    `353D = 1` and `3543 = 3549 = 0x0080`, but the underlying raw-copy layout
    for the prefix bytes is now straightforward
- current best raw record prefix model:
  - `0x00..0x01` = primary selector / target `u16`
  - `0x02` = owning / current empire byte
  - `0x03..0x04` = follow-on selector or linked-record count/index `u16`
  - `0x05..0x06` = secondary selector / payload `u16`
  - `0x09` = tuple-A tag
  - `0x0A` = tuple-B tag

Kind-`3` group-start confirmation:

- additional dynamic cases:
  - one-byte mutations at raw offsets `0x0B`, `0x11`, `0x17`, `0x1D`
  - same first summary-write stop at live `2814:6870`
- preserved scratch artifacts:
  - `artifacts/ecmaint-ipbm-debug/off_0b_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_11_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_17_val_01-scratch.txt`
  - `artifacts/ecmaint-ipbm-debug/off_1d_val_01-scratch.txt`
- observed one-byte deltas:
  - `IPBM[0x0B] = 0x01` -> scratch byte `3543 = 0x01`
  - `IPBM[0x11] = 0x01` -> scratch byte `3549 = 0x01`
  - `IPBM[0x17] = 0x01` -> scratch byte `354F = 0x01`
  - `IPBM[0x1D] = 0x01` -> scratch byte `3555 = 0x01`
  - `IPBM[0x1E] = 0x01` -> scratch byte `3556 = 0x01`
  - `IPBM[0x1F] = 0x02` -> scratch byte `3557 = 0x02` at the first
    `2000:5EE4` summary-write stop
- combined with the existing shared-kind writeback:
  - raw `0x0B..0x0F` is the on-disk source for tuple-A payload block
    `3543..3547`
  - raw `0x11..0x15` is the on-disk source for tuple-B payload block
    `3549..354D`
  - raw `0x17..0x1B` is the on-disk source for tuple-C payload block
    `354F..3553`
  - raw `0x1D..0x1F` is the on-disk source for the trailing control group
    `3555..3557`
- trailing-control semantics from `0000:0723..0797`:
  - `3555` and `3556` are treated as scalar bytes, widened through helper
    `0x3000:4891`, then expanded with `0x3000:486B` against literal `0x80`
  - `3557` is clamped so values above `1` normalize back to `1`
  - dynamic clarification:
    - the initial `5EE4` scratch capture still shows the raw on-disk `3557`
      byte before that clamp runs
    - so `2000:5EE4` / `0x3000:502F` is a raw copier into `3538`, while the
      cap-to-`1` behavior belongs to the later shared summary path in
      `0000:02C0`
  - practical reading:
    - raw offsets `0x1D` and `0x1E` are small scalar control bytes
    - raw offset `0x1F` behaves like a boolean / capped mode byte
- current best practical raw-record layout:
  - `0x00..0x01` = primary selector / target `u16`
  - `0x02` = owning / current empire byte
  - `0x03..0x04` = follow-on selector / count / linked-record `u16`
  - `0x05..0x06` = secondary selector / payload `u16`
  - `0x07..0x08` = still structurally copied, semantics not yet named
  - `0x09` = tuple-A tag
  - `0x0A` = tuple-B tag
  - `0x0B..0x0F` = tuple-A payload
  - `0x10` = copied gap / currently-unused byte
  - `0x11..0x15` = tuple-B payload
  - `0x16` = copied gap / currently-unused byte
  - `0x17..0x1B` = tuple-C payload
  - `0x1C` = copied gap / currently-unused byte
  - `0x1D..0x1E` = trailing scalar controls
  - `0x1F` = trailing boolean / capped mode byte

Targeted fixture confirmation:

- new reproducer: `tools/test_starbase2_baseid_gate.py`
- with `PLAYER.DAT[0x44..0x47] = 02 00 02 00` and two base records:
  - if base 2 keeps `BASES[0x04] = 0x02`, `ECMAINT` aborts with the
    front-loaded integrity error
  - if base 2 is changed back to `BASES[0x04] = 0x01`, `ECMAINT` accepts the
    run and canonicalizes the state back to one base
- observed accepted post-state:
  - `PLAYER.DAT[0x44..0x47]` normalized from `02 00 02 00` to `01 00 01 00`
  - `BASES.DAT` collapsed from two 35-byte records to one 35-byte record
  - surviving base record kept slot byte `0x00 = 0x02` but `0x04 = 0x01`
- additional matrix check:
  - varying the duplicate record's slot byte `BASES[0x00]` between `0x01` and
    `0x02` does not change the outcome
  - varying `BASES[0x04]` between `0x01` and `0x02` does change the outcome
  - therefore the front-loaded integrity gate is sensitive to `0x04`, not
    `0x00`
  - varying `BASES[0x02]` from `0x01` to `0x02` with `BASES[0x04] = 0x01` does
    **not** trigger the front-loaded integrity abort; instead it reaches the
    later `unknown starbase` path

Refined interpretation from the matrix:

- `BASES[0x04] = 0x02` is what trips the early cross-file integrity validator
- `BASES[0x02]` also matters, but in a different way:
  - `BASES[0x02] = 0x02` with `BASES[0x04] = 0x01` yields
    `Fleet assigned to an unknown starbase.`
  - so `0x02` participates in Guard Starbase ownership/lookup semantics, while
    `0x04` is the byte that currently pushes the synthetic two-base attempt into
    the earlier integrity-failure branch

Practical conclusion:

- `BASES.DAT[0x04]` is the decisive identity value in the early integrity path
- duplicate records that still claim starbase identity `1` are mergeable /
  canonicalizable
- a true second starbase must satisfy more than just record presence plus
  `PLAYER.DAT[0x44] = 2`; the validator rejects the first attempt as soon as
  `BASES[0x04]` advances to `2`

Practical inference:

- the synthetic Starbase 2 failure is now narrowed to the startup validator
  rooted at `0x26D9B`, with `0x25EE4` as the first concrete helper to reverse
- that matches the DOSBox-X trace evidence: the abort happens after the initial
  file sweep and before the normal maintenance pipeline

Validator branch correction:

- static report: `artifacts/ghidra/ecmaint-live/5ee4-fleet-branch.txt`
- script: `tools/ghidra_scripts_tmp/Report5EE4FleetBranch.java`
- `2000:6040..6368` is the `FLEETS.DAT` validator branch inside `2000:5EE4`,
  not the direct `BASES.DAT` loader
- evidence:
  - opens stream `0x3178`
  - uses record size `0x36`
  - copies the active fleet record into local scratch at `[BP+0xFF3E]`
- structure:
  - loops over per-player pointers from `0x16AC`
  - validates fleet-owner byte `[BP+0xFF40]` against the current loop index
    when the dead `16A4` bypass flag is off
  - emits kind-`1` summary entries through `0x2F72` / `0x2F76`
  - first sub-branch writes summary `+0x06` from `player[0x40]`
  - second sub-branch is gated by local word `[BP+0xFF41]` and writes summary
    `+0x06` from `[BP+0xFF43]`
- practical consequence:
  - the front-loaded synthetic two-base integrity abort is distinct from the
    later `Fleet assigned to an unknown starbase` behavior
  - that later error is now most likely produced by downstream kind-`1`
    summary resolution over scratch block `0x3502`, not by this loader loop

Kind-`1` scratch dispatch follow-up:

- new artifact: `artifacts/ghidra/ecmaint-live/kind1-scratch-function.txt`
- new script: `tools/ghidra_scripts_tmp/ReportKind1ScratchFunction.java`
- `0000:02ED..03D5` is the kind-`1` mirror of the already-mapped kind-`3`
  summary loader:
  - pushes summary field `ES:[DI+0x06]`
  - passes scratch base `0x3502`
  - then reads the normalized field family:
    - `350D`, `350F..3513`
    - `350E`, `3515..3519`
    - `3522`
    - `3523`
    - `351B..351F`
    - capped byte `3524`
    - selector/count byte `350C`
- practical interpretation:
  - kind `1` uses the same generic summary-dispatch architecture as kind `3`
  - the later Guard Starbase failure path should be recoverable from scratch
    block `0x3502` plus the common post-kind canonicalization logic
- stronger input-path result:
  - in `0000:02ED..03D5`, the only explicit summary field passed into the
    kind-`1` scratch loader is `ES:[DI+0x06]`
  - the fleet-emitted summary bytes `+0x01` / `+0x02` are not read in the
    initial kind-`1` load path; they are overwritten later by the shared
    canonicalization/writeback stage at `0000:0BE8..0CD4`
  - practical consequence: the later starbase-resolution logic is more likely
    keyed by the secondary word in summary `+0x06` than by the provisional
    bytes written directly from the fleet record in `2000:6040..6368`
- fleet-scratch offset correlation:
  - using the confirmed `54`-byte fleet layout plus the initialized
    `fixtures/ecutil-init/v1.5/FLEETS.DAT` bytes:
    - `[BP+0xFF40]` = fleet `record[0x02]` (owner / empire byte)
    - `[BP+0xFF41]` = fleet `record[0x03..0x04]`
    - `[BP+0xFF43]` = fleet `record[0x05..0x06]`
    - `[BP+0xFF49]` = fleet `record[0x0B]` (current X)
    - `[BP+0xFF4A]` = fleet `record[0x0C]` (current Y)
    - `[BP+0xFF57..0xFF5B]` = fleet `record[0x19..0x1D]` (internal flag /
      state cluster)
  - the initialized fixture makes the second sub-branch interpretation much
    stronger:
    - fleet `record[0x03]` is the local `next fleet ID`
    - therefore the gate `CMP word ptr [BP+0xFF41],0` is effectively testing
      whether the current fleet links to another fleet in the owning empire's
      chain
    - the follow-on load using `[BP+0xFF41] - 1` is therefore a chained-fleet
      lookup, not a starbase-record selector
- practical inference:
  - summary `+0x06` in the second kind-`1` emission is likely carrying the
      chained fleet identifier from `record[0x05..0x06]`
  - summary `+0x06` in the first kind-`1` emission, taken from `player[0x40]`,
      is now more likely the head-of-chain fleet identifier for that empire
      than a count field
- important correction:
  - the raw-import entry at `2000:C067` is not yet a trustworthy semantic
    helper start; like the earlier `C0CD` false lead, it decodes as a fragment
    inside a larger arithmetic/helper region
  - so the next useful step is not to assign semantics to `C067` itself, but
    to correlate the `3502` fields back to the `FLEETS.DAT` offsets already
    observed in `2000:6040..6368`

Kind-`2` matching milestone:

- the kind-`2` path in `0000:03DF..06AE` is not an isolated base normalizer; it
  actively scans the summary table for a matching live kind-`1` entry before
  finalizing the current entry
- concrete scan predicate from `0000:0524..06AB`:
  - candidate summary must have:
    - same summary byte `+0x00`
    - kind byte `+0x04 == 1`
    - active/status byte `+0x03 != 0`
  - then it accepts either:
    - direct ID match: candidate summary word `+0x0A == [0x3558]`
    - or a stronger structural match:
      - same summary bytes `+0x01`, `+0x02`, and `+0x05`
      - helper result from candidate summary `+0x06` with:
        - decoded kind byte `== 4`
        - decoded word `== [0x355A]`
        - decoded flag byte `== 0`
- practical interpretation:
  - the later starbase-resolution layer is matching base-side kind-`2`
    summaries against fleet-side kind-`1` summaries
  - the `unknown starbase` failure is therefore more likely "no acceptable
    kind-`1` / kind-`2` pair survived summary resolution" than a direct raw
    `FLEETS.DAT[0x22]` parse failure
  - for Rust-generated compliant gamestates, it will not be enough to emit
    locally plausible `FLEETS.DAT` and `BASES.DAT`; the fleet/base chain IDs,
    coordinates, and the summary `+0x06` linkage values must also normalize
    into at least one accepted pair

Rust Guard Starbase encoder milestone:

- the accepted one-base Guard Starbase scenario is no longer emitted from a
  raw 35-byte `BASES.DAT` constant in `ec-cli`
- `ec-data::BaseRecord` now exposes named setters for the currently mapped
  integrity-critical base fields:
  - local slot (`0x00`)
  - active flag (`0x02`)
  - base ID (`0x04`)
  - link word (`0x05..0x06`)
  - chain word (`0x07..0x08`)
  - coords (`0x0B..0x0C`)
  - tuple A payload (`0x0D..0x11`)
  - tuple B payload (`0x13..0x17`)
  - tuple C payload (`0x19..0x1D`)
  - trailing coords (`0x20..0x21`)
  - owner empire (`0x22`)
- `ec-cli scenario <dir> guard-starbase` now builds the accepted base record
  through those setters and still reproduces
  `fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT` exactly
- `ec-cli validate <dir> guard-starbase` now checks the currently-known
  accepted one-base scenario invariants directly:
  - `PLAYER[1].starbase_count_raw == 1`
  - `FLEET[1].order == 0x04`
  - `FLEET[1].aux == [0x01, 0x01]`
  - `BASES.DAT` contains exactly one record matching the structured accepted
    one-base encoding
- `ec-cli scenario-init [source_dir] <target_dir> guard-starbase` now copies a
  compliant baseline and applies the accepted one-base scenario in one step,
  producing a runnable directory directly from Rust
- practical meaning:
  - the Rust layer has moved one more step away from fixture-byte templating
    and toward explicit compliant gamestate encoding
  - the next Rust-side step is not another raw blob transplant; it is to name
    the remaining linkage semantics well enough to validate or emit additional
    base/fleet pairings deliberately

Rust fleet/build scenario CLI milestone:

- the previously low-level exact-fixture rewrites are now exposed as named
  accepted scenarios in `ec-cli`:
  - `ec-cli scenario <dir> fleet-order`
  - `ec-cli scenario <dir> planet-build`
- both scenarios also have validation entry points:
  - `ec-cli validate <dir> fleet-order`
  - `ec-cli validate <dir> planet-build`
  - `ec-cli validate <dir> all` now runs the current scenario validators as a
    directory-classification pass and reports which known accepted scenarios
    match
- the Rust-side known accepted scenarios are now centralized behind one
  catalog:
  - `ec-cli scenario <dir> list`
  - `ec-cli scenario <dir> show <scenario>`
  - `ec-cli scenario-init-all [source_dir] <target_root>`
- scenario validation now has two layers:
  - rule-shaped acceptance checks:
    - `ec-cli validate <dir> <scenario>`
    - `ec-cli validate <dir> all`
  - preserved exact-match checks:
    - `ec-cli validate-preserved <dir> <scenario>`
    - `ec-cli validate-preserved <dir> all`
- preserved scenario drift can now be inspected directly:
  - `ec-cli compare-preserved <dir> <scenario>`
  - `ec-cli compare-preserved <dir> all`
- both scenarios can now be materialized into runnable directories from a
  compliant baseline in one command:
  - `ec-cli scenario-init [source_dir] <target_dir> fleet-order`
  - `ec-cli scenario-init [source_dir] <target_dir> planet-build`
- parser/usability correction:
  - the documented optional-source CLI forms now work as intended
  - `ec-cli init <target_dir>` defaults to `original/v1.5`
  - `ec-cli scenario-init <target_dir> <scenario>` defaults to the compliant
    `fixtures/ecmaint-post/v1.5` baseline instead of incorrectly treating the
    target directory as the source argument
- current accepted scenario checks are intentionally narrow and tied to the
  preserved fixture evidence:
  - fleet-order:
    - `FLEET[1].current_speed == 3`
    - `FLEET[1].order == 0x0c`
    - `FLEET[1].target == (15, 13)`
  - planet-build:
    - `PLANET[15].build_slot == 0x03`
    - `PLANET[15].build_kind == 0x01`
- practical meaning:
  - Rust can now generate and sanity-check three preserved accepted scenarios
    through one consistent scenario-oriented interface
  - Rust can now materialize the full current known-scenario set in one batch,
    which lowers the cost of running multiple original-engine experiments from
    the same baseline
  - that lowers the cost of spinning up new original-engine runs while the
    remaining integrity-linkage semantics are still being decoded

Project milestone framing:

- the explicit milestone ladder is now:
  - known accepted scenarios
  - parameterized scenario generation
  - general compliant gamestate generation
  - full Rust `ECMAINT` replacement
- current state:
  - milestone 1 is active and useful
  - milestone 2 has started
  - milestone 3 is still blocked on remaining `5EE4` linkage semantics

Base-side summary emitter mapping:

- new artifact: `artifacts/ghidra/ecmaint-live/5ee4-base-branch.txt`
- new script: `tools/ghidra_scripts_tmp/Report5EE4BaseBranch.java`
- `2000:63D3..6759` is the full `BASES.DAT` validator / kind-`2` summary
  emitter between the fleet and `IPBM` passes
- primary branch (`2000:63D3..657F`):
  - opens stream `0x2FF8` with record size `0x23`
  - uses `player[0x44] - 1` as the first base-record selector
  - loads the base record into local scratch at `[BP+0xFF74]`
  - validates `[BP+0xFF78]` against the current player index
  - emits a kind-`2` summary entry with:
    - summary `+0x0A` <- base `[0x02..0x03]` (`[BP+0xFF76]`)
    - summary `+0x00` <- current player index or bypass-side owner byte
    - summary `+0x04` <- `2`
    - summary `+0x01` <- base `[0x0B]`
    - summary `+0x02` <- base `[0x0C]`
    - summary `+0x05` <- derived from base `[0x19..0x1D]`
    - summary `+0x03` <- `1`
    - summary `+0x06` <- `player[0x44]`
- follow-on branch (`2000:6582..66D0`):
  - gated by base linkage word `[BP+0xFF79]`
  - re-reads another base record using that word minus one
  - emits another kind-`2` summary entry with the same `+0x0A`, `+0x01`,
    `+0x02`, `+0x05` pattern
  - but summary `+0x06` now comes from base `[0x07..0x08]`
- local offset map for the loaded base scratch:
  - `[BP+0xFF76]` -> base `0x02..0x03`
  - `[BP+0xFF78]` -> base `0x04`
  - `[BP+0xFF79]` -> base `0x05..0x06`
  - `[BP+0xFF7B]` -> base `0x07..0x08`
  - `[BP+0xFF7F]` -> base `0x0B`
  - `[BP+0xFF80]` -> base `0x0C`
  - `[BP+0xFF8D..0xFF91]` -> base `0x19..0x1D`
- practical consequence for the pairing rule:
  - kind-`2` summary `+0x01` / `+0x02` are definitely base coordinates
  - kind-`2` summary `+0x0A` is definitely rooted in base bytes `0x02..0x03`
  - the still-unknown helper-decoded keys around `3558/355A` must derive from
    either `player[0x44]` or base `0x07..0x08`, depending on which base-side
    sub-branch produced the active summary entry

Kind-`2` matcher decode milestone:

- new artifact: `artifacts/ghidra/ecmaint-live/kind2-matcher.txt`
- new script: `tools/ghidra_scripts_tmp/ReportKind2Matcher.java`
- `0000:03DF..06AE` is now preserved as a concrete kind-`2` pairing loop
  instead of a vague "remaining `3558/355A` logic" placeholder
- the important clarification is that `3558` / `355A` are not compared as raw
  base fields:
  - the current kind-`2` summary entry first pushes summary word `+0x06`
  - then calls far helper `0x2000:c09a` with destination `0x3558`
  - that helper populates a decode scratch family rooted at `3558`, including:
    - `3558` and `355A` as comparison keys
    - supporting decode fields at `3563..357a`
- after that decode, the matcher builds three normalized comparison tuples from
  the `3558` scratch family:
  - `3563 + 3565/3567/3569`
  - `3564 + 356b/356d/356f`
  - `3578`, `3579`, `3571/3573/3575`, capped `357a`
- those tuples are compared against local reference tuples through the common
  far helpers at `0x3000:4891`, `0x3000:486b`, and `0x3000:488d`
- only if that decode/normalization succeeds does the scan advance to candidate
  kind-`1` summaries:
  - direct accept path:
    - candidate summary word `+0x0A == [0x3558]`
  - structural accept path:
    - same summary bytes `+0x01`, `+0x02`, and `+0x05`
    - candidate summary word `+0x06` decoded through `0x2000:c067`
    - decoded kind byte `== 4`
    - decoded word `== [0x355A]`
    - decoded flag byte `== 0`
- practical interpretation:
  - `3558` / `355A` are now better modeled as helper-decoded linkage keys
    derived from base-side summary `+0x06`, not as raw persistent fields
    directly stored in `BASES.DAT`
  - the next high-value RE target is therefore the decode-helper pair
    `0x2000:c09a` and `0x2000:c067`, because they bridge raw summary `+0x06`
    values into the actual pairing keys

Helper-region correction after focused dump:

- new artifact: `artifacts/ghidra/ecmaint-live/kind2-decode-helpers.txt`
- new script: `tools/ghidra_scripts_tmp/ReportKind2DecodeHelpers.java`
- the focused dump around `0x2000:c067` and `0x2000:c09a` is a useful
  correction, not yet a semantic decode:
  - in the raw live import, both addresses still sit inside a dense helper
    island that includes arithmetic and counted-string helpers
  - `0x2000:c067` currently lands in the middle of an arithmetic / shift /
    divide-style region, not at a trustworthy standalone function prologue
  - `0x2000:c09a` also does not yet decode as a clean semantic entry point in
    this raw view
- practical consequence:
  - the matcher's call targets remain operationally important, but the raw
    import still does not support naming `c067` / `c09a` themselves as
    recovered helpers with confidence
  - the next productive step is finer carving around the real call boundaries
    and/or dynamic capture of the call arguments/results, not more naive
    function naming at the current addresses

Caller-pattern milestone for the helper island:

- new artifact: `artifacts/ghidra/ecmaint-live/kind2-helper-callers.txt`
- new script: `tools/ghidra_scripts_tmp/ReportKind2HelperCallers.java`
- despite the raw helper region still being poorly carved, the callers now
  establish a stable contract for both targets:
  - `0x2000:c067`
  - `0x2000:c09a`
- shared observed calling convention:
  - caller pushes source summary word `ES:[DI + 0x06]`
  - caller then pushes a destination far pointer
    - either `DS:offset`
    - or `SS:local`
  - then performs the far call
- confirmed high-value callers:
  - `0000:0307`:
    - kind-`1` loader pushes summary `+0x06`
    - destination `DS:3502`
    - then immediately consumes `350d..` as decoded scratch
  - `0000:03fe`:
    - kind-`2` matcher pushes summary `+0x06`
    - destination `DS:3558`
    - then immediately consumes `3563..357a` as decoded scratch
  - `0000:0681`:
    - kind-`2` structural accept path pushes candidate kind-`1` summary `+0x06`
    - destination `SS:[BP+f7b6]`
    - then checks decoded output at offsets:
      - `+0x1f` -> kind byte `== 4`
      - `+0x23` -> word compared against `[0x355A]`
      - `+0x0a` -> flag byte `== 0`
- practical interpretation:
  - `c067` is now strongly supported as a generic summary-`+0x06` decoder used
    by the kind-`1` loader and by the matcher's structural comparison path
  - `c09a` is likewise supported as a sibling decoder used to populate the
    `3558` scratch family from base-side summary `+0x06`
  - even without a clean raw function carve, the decoded-output offsets are now
    concrete enough to guide both future dynamic capture and Rust-side naming

Decoded-output field-layout milestone:

- new artifact: `artifacts/ghidra/ecmaint-live/kind2-decoded-field-uses.txt`
- new script: `tools/ghidra_scripts_tmp/ReportKind2DecodedFieldUses.java`
- the immediate post-call reads now show that `3502` and `3558` are sibling
  decoded summary-`+0x06` structures with the same tuple layout:
  - first tuple:
    - tag byte at `+0x0b`
    - payload words at `+0x0d`, `+0x0f`, `+0x11`
  - second tuple:
    - tag byte at `+0x0c`
    - payload words at `+0x13`, `+0x15`, `+0x17`
  - shared scalar/control group:
    - byte at `+0x20`
    - words at `+0x22`, `+0x24`, `+0x26`
    - capped byte at `+0x28`
- concrete mapped instances:
  - `DS:3502`:
    - `350d`, `350f/3511/3513`
    - `350e`, `3515/3517/3519`
    - `3522`, `3524`, and the already-known surrounding group
  - `DS:3558`:
    - `3563`, `3565/3567/3569`
    - `3564`, `356b/356d/356f`
    - `3578`, `3571/3573/3575`, capped `357a`
- local structural-match decode from `0000:0681` confirms the key offsets in a
  stack-local instance of the same family:
  - local `+0x1f` = decoded kind byte
  - local `+0x23` = decoded word compared against `[0x355A]`
  - local `+0x0a` = decoded flag byte expected to be `0`
- practical interpretation:
  - the important remaining unknown is now the meaning of these decoded tuple
    groups, not their existence or rough field shape
  - this is enough structure to start naming a generic "decoded summary `+0x06`
    buffer" in Rust-facing notes instead of treating `3502` and `3558` as
    unrelated scratch islands

Relevant documentation cross-check:

- `ECPLAYER.DOC` confirms `X` toggles a player-level `expert mode` setting and `T` changes the empire-wide tax rate
- that makes `PLAYER.DAT` the right place to continue hunting for saved UI/options flags after the tax byte

Expert mode persistence test:

- toggled `X` expert mode in-game
- quit cleanly
- compared `/tmp/PLAYER.before_expert_toggle.DAT` with the resulting `PLAYER.DAT`
- result: no byte differences at all

Practical inference:

- expert mode is not persisted in `PLAYER.DAT`
- it is likely a session-only runtime flag, or stored in a transient/message/drop-file path rather than campaign state

### `PLANETS.DAT`

Status:

- confirmed size: `1940` bytes
- confirmed structure: `20` records of `97` bytes

Why this is likely:

- `1940 / 97 = 20`
- every 97-byte chunk contains the same string slot and similar numeric layout

Draft record layout:

- `0x00..0x02`: small numeric header, likely coordinates / planet class / index
- `0x03..0x0E`: numeric state fields (mostly zero on many records)
- `0x0F`: string length byte
- `0x10..0x1C`: 13-byte name/status string slot
- `0x1D..0x24`: numeric fields, possibly production/resources/position
- `0x25..0x4F`: mostly zero in the current test state
- `0x50..0x5F`: tail numeric fields; nonzero on some records only
- `0x60`: final terminator/status byte

Observed names/statuses:

- many records: `Unowned`
- several records: `Not Named Yet`

Inferred meaning:

- `Unowned` is almost certainly the current owner/status string for neutral planets
- `Not Named Yet` likely marks special colonies/homeworld slots that are still unnamed in the fresh game state

Observed original -> initialized changes:

- every record changed in the first three bytes, which strongly suggests a compact per-planet header rather than random garbage
- records `5`, `7`, `14`, `15`, and `19` changed most heavily
- record `5` changed from:
  - header `0f 04 1e 00`
  - string `Unowned`
  to:
  - header `0d 05 64 87`
  - string `Not Named Yet`
- record `15` changed from:
  - string `Dust Bowl Yet`
  to:
  - string `Unowned`
- record `19` kept `Unowned` but had trailing garbage zeroed after the string slot

Practical inference:

- the `0x10` string field is real game state, not display-only padding
- initialization normalizes several records from demo/sample data into fresh-campaign placeholders
- the first 2-4 bytes likely encode a planet id plus a compact location/class tuple
- records with `Not Named Yet` are probably special player-start or colony targets that later receive player-defined names
- later per-planet economic/build choices appear to live deeper in the same record rather than in a separate queue file

Database mirror observation:

- `PLANETS.DAT` record `14` string bytes changed from `Not N...` to `prime`
- the same `prime` string change appears in `DATABASE.DAT` at offsets `0x578..0x57D`

Practical inference:

- `DATABASE.DAT` appears to cache or index planet-display strings from `PLANETS.DAT`
- planet naming/visibility work should be modeled as updates to both the core planet record and a derived database/report structure

Observed build-order changes:

- after issuing two build orders on the active planet (`ETAC` ship and `Destroyer`), no new differences appeared in `FLEETS.DAT`
- the new changes appeared in `PLANETS.DAT` record `14` only:
  - `0x24`: `0 -> 3`
  - `0x2E`: `0 -> 1`
- those bytes were still zero in the earlier post-mission state, so they are attributable to the build-order step rather than join or fleet orders

Practical inference:

- at least part of the planet production queue is encoded directly in each 97-byte planet record
- `0x24` and `0x2E` are strong candidates for build-item type/count slots, queue depth, or production-allocation flags
- ship construction orders do not appear to allocate or rewrite fleet records immediately; fleets are likely materialized later during maintenance

### `FLEETS.DAT`

Status:

- original sample size: `702` bytes
- initialized sample size: `864` bytes
- initialized structure strongly suggests `16` records of `54` bytes each

Why `54` bytes is the current best fit:

- `864 / 54 = 16`
- after initialization the file splits into `16` repeating records with the same internal layout
- the first bytes of those records form a clear grid:
  - record `0`: `01 00 01 02 00 01 ...`
  - record `1`: `02 00 01 03 00 02 ...`
  - record `2`: `03 00 01 04 00 03 ...`
  - record `3`: `04 00 01 00 00 04 ...`
- records `4..7`, `8..11`, and `12..15` repeat the same pattern with the second little-endian word incrementing from `2` to `4`

Draft record layout:

- `0x00..0x01`: small id/counter field
- `0x02..0x03`: row/group index
- `0x04..0x05`: linked id or previous slot
- `0x06..0x07`: linked id or next slot
- `0x08..0x09`: small flag/counter
- `0x0A..0x21`: mostly constant initialized template values
- `0x22..0x35`: trailing status/capacity flags, with one bit pattern distinguishing the first two records from the latter two inside each 4-record block

Practical inference:

- initialization expands fleet storage into a fully templated fixed-record table
- the `16` records look more like fleet slots or route templates than ad hoc save data
- this is now structured enough to port as a fixed record array even before every field is named

Observed mission-order changes:

- after issuing missions to fleets `1`, `2`, `3`, and `4`, only records `0..3` changed relative to the fresh `ECUTIL` baseline
- changed offsets were confined to the same small region in each 54-byte record:
  - record `0`: `0x0A`, `0x1F`, `0x20`
  - record `1`: `0x0A`, `0x1F`, `0x20`, `0x21`
  - record `2`: `0x0A`, `0x1F`, `0x20`
  - record `3`: `0x0A`, `0x1F`, `0x20`
- exact value changes:
  - record `0`: `0x0A 0->3`, `0x1F 5->12`, `0x20 16->15`
  - record `1`: `0x0A 0->3`, `0x1F 5->12`, `0x20 16->18`, `0x21 13->15`
  - record `2`: `0x0A 0->6`, `0x1F 5->9`,  `0x20 16->15`
  - record `3`: `0x0A 0->6`, `0x1F 5->6`,  `0x20 16->15`

Practical inference:

- `0x0A` is likely the chosen current speed for the order
- `0x1F..0x21` likely encode mission parameters such as destination coordinates, target slot, or route endpoint
- `0x22`: mission-specific parameter — for Guard Starbase orders, this is the **starbase number** to guard
- `0x23`: mission-specific parameter — for Guard Starbase orders, must be **exactly `0x01`** for the order to resolve
- most of each fleet record remains unchanged by orders, which supports the idea that fleet identity/capacity lives in fixed header fields and only a compact mission block mutates during command entry

### `SETUP.DAT`

Status:

- confirmed size: `522` bytes
- header: `EC151`

Observed header bytes:

- `45 43 31 35 31` = `EC151`
- next bytes: `04 03 04 03 01 01 01 01`

Inferred:

- version marker plus compact global settings
- likely includes player-count / schedule / option toggles

Observed original -> initialized changes:

- no byte differences between the shipped sample and the initialized local test file

Practical inference:

- `ECUTIL` initialization does not rewrite `SETUP.DAT`
- `SETUP.DAT` is likely installation/global configuration, while the mutable campaign state lives primarily in `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and the auxiliary files created by `ECUTIL`

### `CONQUEST.DAT`

Status:

- confirmed size: `2085` bytes
- no useful printable strings in either the fresh `ECUTIL` state or the post-maintenance state
- post-maintenance differences are concentrated entirely in the first `0x55` bytes

Observed post-maintenance changes versus fresh `ECUTIL` baseline:

- total changed bytes: `51`
- changed offsets:
  - `0x00`
  - sparse changes from `0x12..0x3B`
  - dense changes from `0x40..0x54`
- examples:
  - `0x00`: `0xB8 -> 0xB9`
  - `0x12..0x13`: `0x64 0x00 -> 0xFF 0xFF`
  - `0x1A..0x1B`: `0x64 0x00 -> 0x74 0x33`
  - `0x20..0x23`: `0x64 0x00 0x64 0x00 -> 0x75 0x03 0x65 0x20`
  - `0x40..0x49`: `0x01`-filled bytes replaced by `ff 00 00 00 c2 00 00 08 6f 00`

Practical inference:

- `CONQUEST.DAT` begins with a packed global header or control block
- maintenance updates year/turn/summary counters here, not in `PLAYER.DAT` or `PLANETS.DAT`
- the repeated `0x0064` (`100`) values in the fresh baseline suggest default percentages, capacities, or turn constants
- the dense post-maintenance writes at `0x40..0x54` look like derived summary totals or timing/state fields produced during turn processing
- this file is a prime candidate for the core campaign clock and global statistics model in a port

Confirmed field:

- `CONQUEST.DAT[0x00..0x01]` (`u16`, little-endian): game year
  - shipped sample: `3022`
  - fresh `ECUTIL` init fixture: `3000`
  - post-maintenance fixture: `3001`

Confirmed field:

- `CONQUEST.DAT[0x02]` (`u8`): player count
  - shipped sample: `4`
  - fresh `ECUTIL` init fixture: `4`
  - post-maintenance fixture: `4`

Why this is high confidence:

- `ECPLAYER.DOC` states “The year is 3000.”
- `ECPLAYER.DOC` and `ECQSTART.DOC` both state that each round equals one year of game time
- the initialized-to-post-maint transition increments this field by exactly `1`

Why this is high confidence:

- `ECREADME.DOC` states that `ECUTIL` sets the maximum number of players.
- `ECPLAYER.DOC` states that the number of solar systems is `5` times the number of players.
- the preserved initialized fixture has `20` planet records and `20 / 5 = 4`.
- the low byte of the `0x0104` control word is `4` in all preserved states.

Practical caution:

- `CONQUEST.DAT[0x02..0x03]` is still exposed in the Rust code as a combined `player_config_word`.
- only the low byte is currently named with confidence.

Confirmed field block:

- `CONQUEST.DAT[0x03..0x09]` (`7 x u8`): maintenance schedule, ordered:
  - `[0x03]` Sunday
  - `[0x04]` Monday
  - `[0x05]` Tuesday
  - `[0x06]` Wednesday
  - `[0x07]` Thursday
  - `[0x08]` Friday
  - `[0x09]` Saturday

Confirmed encoding:

- `0x00` means the day is disabled for maintenance
- enabled days store a nonzero day-specific code, not a plain boolean

Observed values from controlled `ECUTIL` `F2 Change Maintenance Days` edits:

- Sunday `Yes` = `0x01`
- Monday `Yes` = `0x01`
- Tuesday `Yes` = `0xCA`
- Wednesday `Yes` = `0x01`
- Thursday `Yes` = `0x0A`
- Friday `Yes` = `0x01`
- Saturday `Yes` = `0x26`

High-confidence baseline:

- the preserved post-maintenance fixture stores `[01, 01, 01, 01, 01, 01, 01]`
- the live `ECUTIL` experiments proved that zeroing a day changes the corresponding byte to `0x00`

Practical implication for the Rust port:

- preserve the schedule as raw bytes first
- interpret `0x00` as disabled
- do not collapse the nonzero values to booleans until the original encoding scheme is better understood

Useful structural clue from initialized fixtures:

- in the preserved `4`-player initialized state, `FLEETS.DAT` contains `16` populated `54`-byte records
- `ECPLAYER.DOC` states that each empire starts with `4` fleets
- `4 players x 4 starting fleets = 16`, which matches the initialized fixture exactly

Practical implication:

- the preserved initialized `FLEETS.DAT` layout is consistent with a fixed fleet-record table sized to the current player count times the starting fleet allotment
- this is useful for port design, but not enough yet to name individual fleet fields

Preserved initialized fleet baseline:

From `original/v1.5/ec-logs-2012/ec.txt`, the first empire's four starting fleets in the
post-maintenance `3001 A.D.` state are:

- Fleet `1`: Speed `3`, ETA `1`, Ships `2`, ROE `6`, `Sector(14,14)`,
  `Colonize world in System (13,15)`
- Fleet `2`: Speed `3`, ETA `2`, Ships `2`, ROE `6`, `Sector(17,12)`,
  `Colonize world in System (20,11)`
- Fleet `3`: Speed `6`, ETA `2`, Ships `1`, ROE `6`, `Sector(19,9)`,
  `View world in System (23,5)`
- Fleet `4`: Speed `0`, ETA `0`, Ships `1`, ROE `6`, `Planet(15,13)`,
  `Guard/blockade world in System (15,13)`

The same log gives the detailed ship contents:

- Fleet `1`: `CA=1 ET=1`
- Fleet `2`: `CA=1 ET=1`
- Fleet `3`: `DD=1`
- Fleet `4`: `DD=1`

Practical implication:

- these preserved runtime values are the best current ground truth for naming the early fields in
  the initialized `FLEETS.DAT` records
- they are also a useful conformance target for a future Rust `inspect` view that decodes fleet
  location, mission, ROE, speed, ETA, and ship composition

Confirmed `FLEETS.DAT` fields from the initialized `16 x 54` layout:

- `record[0x05]` (`u8`): global fleet ID
  - records `1..16` store IDs `1..16`
- `record[0x00]` (`u8`): local fleet slot within the owning empire's four-fleet starting block
  - cycles `1,2,3,4` across the initialized table
  - preserved `ECGAME` text logs now also strongly suggest this is the
    player-facing fleet number:
    - `ec.txt` commissions the first player fleet after the starting four as
      the `5th Fleet`
    - later campaign logs refer to both your own `11th Fleet` and an alien
      `11th Fleet of "Melody Lake"` in the same era, which is only coherent if
      displayed fleet numbers are per-empire rather than globally unique
  - the shipped active `original/v1.5/FLEETS.DAT` now reinforces the split:
    - owner 2 uses local slots `1..4` while the paired `record[0x05]` values
      are `2..5`
    - owner 3 uses local slots `1..4` while the paired `record[0x05]` values
      are `6..9`
    - owner 4 uses local slots `1..4` while the paired `record[0x05]` values
      are `10..13`
    - so local slot and structural fleet ID are not the same on disk once the
      campaign is no longer the simple player-1-only initialized view
- `record[0x03]` (`u8`): next fleet ID in the local linked order
  - fleet `1 -> 2`, `2 -> 3`, `3 -> 4`, `4 -> 0`
- `record[0x07]` (`u8`): previous fleet ID in the local linked order
  - fleet `1 <- 0`, `2 <- 1`, `3 <- 2`, `4 <- 3`
- `record[0x09]` (`u8`): maximum speed
  - matches the preserved starting fleet listing: `3, 3, 6, 6`
- `record[0x0A]` (`u8`): current speed
  - matches preserved live order-entry behavior:
    - fleets `1` and `2` were ordered with current speed `3`, and `0x0A` became `0x03`
    - fleets `3` and `4` were ordered with current speed `6`, and `0x0A` became `0x06`
  - later combat-era logs also show the fleet brief list carrying this chosen travel speed separately
    from maximum speed
- `record[0x25]` (`u8`): rules of engagement
  - matches the preserved starting fleet listing: all `6`
- `record[0x28]` (`u8`): cruiser count
  - starting fleets `1` and `2` have `CA=1`
- `record[0x2A]` (`u8`): destroyer count
  - starting fleets `3` and `4` have `DD=1`
- `record[0x30]` (`u8`): ETAC count
  - starting fleets `1` and `2` have `ET=1`

Useful but still conservatively named:

- `record[0x0B..0x0C]`: current-location coordinate pair
  - in the initialized fixture this looked like a shared home-system pair because every starting fleet
    begins at home
  - empire-group values in the initialized fixture are:
    - fleets `1..4`: `[16, 13]`
    - fleets `5..8`: `[4, 13]`
    - fleets `9..12`: `[6, 5]`
    - fleets `13..16`: `[13, 5]`
- `record[0x1F..0x21]`: mission parameter bytes
  - best current interpretation from preserved fleet-order screenshots:
    - `record[0x1F]`: standing-order mission code
    - `record[0x20]`: target X coordinate
    - `record[0x21]`: target Y coordinate
  - preserved `v1.11` screenshot menu codes show:
    - `0` none / hold position
    - `1` move fleet only
    - `2` seek home
    - `3` patrol a sector
    - `5` guard/blockade a world
    - `6` bombard a world
    - `9` view a world
    - `12` colonize a world
    - `13` join another fleet
  - in the initialized fixture, all four-fleet empire blocks store `[5, X, Y]` where `X,Y`
    match the block's initial current-location pair, which strongly suggests the initial standing orders
    are `Guard/Blockade` at the empire's home system

Practical implication for the Rust port:

- `ec-data` can now expose a small but real typed fleet model for initialized states
- the next useful fleet target is to decode current location, destination, ETA, and mission type

Confirmed initialized fleet-table structure:

- the initialized `16 x 54` table is the full 4-player starting roster, not just the current
  player's fleets
- records are grouped as four 4-fleet empire blocks:
  - group 1: fleet IDs `1..4`
  - group 2: fleet IDs `5..8`
  - group 3: fleet IDs `9..12`
  - group 4: fleet IDs `13..16`
- within each 4-fleet block:
  - `local_slot` cycles `1,2,3,4`
  - `previous_fleet_id` and `next_fleet_id` form a local chain ending in `0`
  - ship loadout is always:
    - slots `1` and `2`: `CA=1 ET=1`
    - slots `3` and `4`: `DD=1`
  - `max_speed` is always:
    - slots `1` and `2`: `3`
    - slots `3` and `4`: `6`
  - `current_location_coords_raw` is constant within the block
  - `mission_param_bytes` is also constant within the block

Observed initialized block current-location pairs:

- IDs `1..4`: `[16, 13]`
- IDs `5..8`: `[4, 13]`
- IDs `9..12`: `[6, 5]`
- IDs `13..16`: `[13, 5]`

Observed initialized block mission-param triples:

- IDs `1..4`: `[5, 16, 13]`
- IDs `5..8`: `[5, 4, 13]`
- IDs `9..12`: `[5, 6, 5]`
- IDs `13..16`: `[5, 13, 5]`

Practical implication:

- bytes `0x0B..0x0C` and `0x1F..0x21` look identical across an initialized empire block because all
  starting fleets begin at their home location with the same guard/blockade standing order
- the next likely per-fleet order-state bytes are around the still-unnamed early header values such
  as speed/ETA/current-location fields

Negative result from the initialized first-four-fleet scan:

- across fleets `1..4`, the only byte positions that vary are:
  - `0x00` local slot
  - `0x03` next fleet ID
  - `0x05` fleet ID
  - `0x07` previous fleet ID
  - `0x09` max speed
  - `0x28` cruiser count
  - `0x2A` destroyer count
  - `0x30` ETAC count
- no other single byte in the initialized records matches the preserved brief-list `ETA`,
  current location, or displayed ship-total columns directly

Practical implication:

- the displayed `ETA` and current location for initialized fleets are probably derived from a
  combination of:
  - standing order code / target
  - current-location raw pair
  - local slot / fleet composition
  - game-wide movement rules
- or they are encoded in multi-byte/stateful forms that do not appear as simple scalar per-fleet
  fields in the initialized snapshot

## ECUTIL Surface

Preserved DOSBox-X screenshot:

- `/home/niltempus/Pictures/ecv1.5/ecutil_000.raw1.png`

Versioned screenshot/archive policy:

- `original/v1.5/EC-Screenshots-v1.11/` is a bundled historical reference set from `v1.11`
- `captures/v1.5-dosboxx/` is the preserved local runtime evidence set for this project's
  `v1.5` reverse engineering work
- when `v1.11` and `v1.5` screenshots differ, prefer the `v1.5` capture set for preservation
  notes and Rust compatibility work

Current preserved `v1.5` capture set:

- `captures/v1.5-dosboxx/ecgame_000.png` through `captures/v1.5-dosboxx/ecgame_030.png`
- `captures/v1.5-dosboxx/ecutil_000.png`
- `captures/v1.5-dosboxx/ecutil_000.raw1.png`
- `captures/v1.5-dosboxx/ecutil_001.png`
- `captures/v1.5-dosboxx/ecutil_002.png`

Confirmed `ECUTIL` main menu text:

- `Esterian Conquest Sysop's Utility`
- `MAIN MENU`
- `F1  Initialize a New Game`
- `F2  Change Maintenance Days`
- `F3  Change Empire Ownership`
- `F4  Modify Program Options`
- `F5  Change Modem/Com Port Configuration`
- `F10 Exit to DOS`

Footer text from the preserved screenshot:

- `Esterian Conquest Sysop's Utility - Test Drive Version 1.51`
- `Copyright (C) 1990-1992 by Bentley C. Griffith.`
- `All rights reserved worldwide.`

Practical implication for the Rust port:

- the preserved `ec-cli init` command corresponds directly to `F1`
- the preserved `ec-cli maintenance-days` command corresponds directly to `F2`
- the preserved `ec-cli setup-programs` command now mirrors the decoded `F4` screen wording
- the screenshot gives exact wording for a future faithful text-mode compatibility frontend

Confirmed `ECUTIL` F4 Setup The Programs menu from `captures/v1.5-dosboxx/ecutil_002.png`:

- `A` Purge messages & reports after
- `B` Autopilot any empires inactive for
- `C` Snoop Enabled
- `D` Enable timeout for local users
- `E` Enable timeout for remote users
- `F` Maximum time between key strokes
- `G` Minimum time granted
- `X` Exit Setup

Current Rust CLI coverage for the decoded `F4` fields:

- `ec-cli setup-programs [dir]`
- `ec-cli snoop [dir] <on|off>`
- `ec-cli local-timeout [dir] <on|off>`
- `ec-cli remote-timeout [dir] <on|off>`
- `ec-cli max-key-gap [dir] <minutes>`
- `ec-cli minimum-time [dir] <minutes>`
- `ec-cli purge-after [dir] <turns>`
- `ec-cli autopilot-after [dir] <turns>`

This means the decoded `F4 Modify Program Options` surface is now fully represented in the std-only Rust CLI, even though the command names are intentionally more Unix-like than the original single-letter menu.

Confirmed `ECUTIL` `F3 Change Empire Ownership` flow from:

- `captures/v1.5-dosboxx/ecutil_004.png`
- `captures/v1.5-dosboxx/ecutil_005.png`
- `captures/v1.5-dosboxx/ecutil_006.png`
- `captures/v1.5-dosboxx/ecutil_007.png`
- `captures/v1.5-dosboxx/ecutil_008.png`

Preserved option surface:

- `P` Assign empire to a new `PLAYER`
- `R` Make empire a `ROGUE` empire
- `U` Make empire `UNOWNED` (`Civil Disorder`)
- `N` No change

Conservative `PLAYER.DAT` ownership findings from the preserved `F3` fixture `fixtures/ecutil-f3-owner/v1.5/PLAYER.DAT`:

- `F3` touched `PLAYER.DAT` only in the observed test; `PLANETS.DAT` did not change.
- Record 0, byte `0x00`, changed `0x00 -> 0xff` when empire `#1` was made rogue.
- Record 0, bytes `0x1B..`, form a Pascal-style status/label field:
  - max length byte at `0x1A` remained `0x18`
  - current length at `0x1B` changed `0x11 -> 0x06`
  - text at `0x1C..` became `Rogues`
- Record 1, byte `0x16`, changed `0x00 -> 0x01` when empire `#2` was assigned to a player.
- Record 1, bytes `0x17..0x2F`, now contain the uppercased player handle `FOO` in a fixed-width field.
- Record 1, bytes `0x31..`, form a second Pascal-style name field:
  - current length at `0x31` became `0x03`
  - text at `0x32..` became `foo`

Rust preservation impact:

- `ec-data` now exposes conservative player ownership summaries:
  - `owner_mode_raw()`
  - `assigned_player_flag_raw()`
  - `legacy_status_name_summary()`
  - `assigned_player_handle_summary()`
  - `controlled_empire_name_summary()`
  - `ownership_summary()`
- This is intentionally narrower than a full player-record decode; it only covers the ownership fields that `ECUTIL F3` demonstrably touched.

Confirmed `ECUTIL` `F5 Modem / Com Port Setup` flow from:

- `captures/v1.5-dosboxx/ecutil_009.png`
- `captures/v1.5-dosboxx/ecutil_010.png`
- `captures/v1.5-dosboxx/ecutil_011.png`

Preserved `F5` surface:

- `A` `COM 1 Interrupt Request Number`
- `B` `COM 2 Interrupt Request Number`
- `C` `COM 3 Interrupt Request Number`
- `D` `COM 4 Interrupt Request Number`
- `E` `Restore Default IRQ Numbers for COM1 to COM4`
- `F` `COM 1 Hardware Flow Control`
- `G` `COM 2 Hardware Flow Control`
- `H` `COM 3 Hardware Flow Control`
- `I` `COM 4 Hardware Flow Control`
- `X` Exit Setup

Confirmed from the preserved `v1.5` screenshots and live diff:

- the IRQ editor prompt accepts direct numeric input in the range `0..7`
- `SETUP.DAT[5..8]` store the raw COM IRQ values for `COM1..COM4`
- the shipped fixture values are `[4, 3, 4, 3]`, matching the preserved `F5` screen
- `SETUP.DAT[9..12]` store `COM1..COM4` hardware flow control flags
- disabling all four flow-control options in `ECUTIL F5` changed those bytes from `[1, 1, 1, 1]` to `[0, 0, 0, 0]`
- `CONQUEST.DAT` did not change during the observed `F5` test

Rust preservation impact:

- `ec-data` now exposes:
  - `com_irq_raw()`
  - `set_com_irq_raw()`
  - `com_hardware_flow_control_enabled()`
  - `set_com_hardware_flow_control_enabled()`
- `ec-cli` now exposes:
  - `port-setup [dir]`
  - `com-irq <dir> <com1|com2|com3|com4> [0..7]`
  - `flow-control <dir> <com1|com2|com3|com4> [on|off]`

The CLI now covers the verified `F5` flow-control toggles directly and exposes the raw IRQ editor bytes with the same observed `0..7` value range as the original utility.

## Modern TUI Direction

Current preservation split:

- `ec-data` stays focused on binary formats and decoded fields
- `ec-cli` stays std-only and scriptable for RE work
- `ec-tui` is the new interactive terminal frontend

Current `ec-tui` shape:

- one shared TUI crate, not separate apps for utility and player modes
- `ec-tui` defaults to player mode in the current working directory
- `ec-tui util` opens the utility/admin mode in the current working directory
- optional directory override is still supported as the first positional path
- when the current directory is not a valid game directory, `ec-tui` now falls back to the preserved `fixtures/ecmaint-post/v1.5` snapshot instead of the noisier shipped sample
- utility mode now uses a modern EC-classic presentation instead of trying to mimic the original DOS utility screen-for-screen
- no function-key dependency in the new shell; section switching is handled with `1/2/3`, `Tab`, and `q`

The first `ec-tui` scaffold is intentionally a shell, not a faithful DOS clone:

- player mode is the default user-facing entry point
- utility mode surfaces the already-decoded setup, ownership, and port data in cleaner sectioned panels:
  - `Dashboard`
  - `Empire Control`
  - `Program & Port Setup`
- the historical `v1.5` UI is preserved via screenshots and notes rather than by reproducing every original panel verbatim

## First ECMAINT Phase-1 Build Scenario

Preserved fixtures:

- `fixtures/ecmaint-build-pre/v1.5/`
- `fixtures/ecmaint-build-post/v1.5/`

This first maintenance scenario used a direct file edit, not a clean one-click player action.

Why:

- we had prior observed evidence that a build-order-like state landed in `PLANETS.DAT` record `14` (zero-based)
- the exact single-order encoding was still unclear
- so the first black-box maintenance cycle was driven by the smallest previously observed planet-side build queue bytes

Pre-maint setup:

- baseline: `fixtures/ecutil-init/v1.5/`
- modified file: `PLANETS.DAT`
- modified record: record `14` (zero-based), the `(16,13)` homeworld-style record
- modified bytes:
  - `0x24`: `0x00 -> 0x03`
  - `0x2E`: `0x00 -> 0x01`

Post-maint result:

- `SETUP.DAT` unchanged
- `CONQUEST.DAT` matched the clean `ecmaint-post` fixture exactly
- `FLEETS.DAT` matched the clean `ecmaint-post` fixture exactly
- `PLANETS.DAT` differed from clean `ecmaint-post` only in record `14`
- `DATABASE.DAT` differed from clean `ecmaint-post` by `1` byte

Observed planet transition in record `14`:

- queued build bytes were cleared:
  - `0x24`: `0x03 -> 0x00`
  - `0x2E`: `0x01 -> 0x00`
- new post-maint state appeared at:
  - `0x38`: `0x00 -> 0x03`
  - `0x4C`: `0x00 -> 0x01`

Interpretation:

- `ECMAINT` consumed the synthetic build-queue-like bytes instead of leaving them in place
- it did not materialize a new fleet in `FLEETS.DAT` in this first scenario
- it did create a persistent planet-state transition and a tiny derived `DATABASE.DAT` change

Rust preservation impact:

- `ec-data` now has a fixture-backed test that locks in this first maintenance transform
- this is enough to prove the phase-1 workflow works, even though the exact semantics of the new `0x38` and `0x4C` planet bytes are not named yet

## Second ECMAINT Scenario: Single Fleet Order

Preserved fixtures:

- `fixtures/ecmaint-fleet-pre/v1.5/`
- `fixtures/ecmaint-fleet-post/v1.5/`

This second maintenance scenario used the smallest previously observed fleet-order mutation from the live game notes.

Pre-maint setup:

- baseline: `fixtures/ecutil-init/v1.5/`
- modified file: `FLEETS.DAT`
- modified record: record `0` (zero-based), fleet `1`
- modified bytes:
  - `0x0A`: `0x00 -> 0x03`
  - `0x1F`: `0x05 -> 0x0C`
  - `0x20`: `0x10 -> 0x0F`

Post-maint result relative to clean `fixtures/ecmaint-post/v1.5/`:

- `SETUP.DAT` unchanged
- `CONQUEST.DAT` unchanged
- `MESSAGES.DAT` unchanged
- `RESULTS.DAT` unchanged
- `DATABASE.DAT` differed by `29` bytes
- `FLEETS.DAT` differed by `9` bytes in fleet record `1`
- `PLANETS.DAT` differed by `18` bytes in planet record `14` (one-based display)

Observed fleet transition in record `0`:

- the queued order bytes were consumed:
  - `0x1F`: `0x0C -> 0x00`
- the fleet was rewritten into a held-at-target style state:
  - `0x0B`: `0x10 -> 0x0F`
  - `0x19`: `0x81 -> 0x80`
  - `0x1A`: `0x00 -> 0xB9`
  - `0x1B`: `0x00 -> 0xFF`
  - `0x1C`: `0x00 -> 0xFF`
  - `0x1D`: `0x00 -> 0xFF`
  - `0x1E`: `0x00 -> 0x7F`
  - `0x20`: `0x10 -> 0x0F`

Derived interpretation:

- fleet `1` moved from a guard/blockade home-world standing order into a hold-style post-maint state
- its current-location pair at `0x0B..0x0C` moved from `(16,13)` to `(15,13)`
- the fleet's target pair at `0x20..0x21` also ended at `(15,13)`
- this is the first controlled scenario showing `ECMAINT` consume a fleet order and rewrite persistent fleet state directly

Observed planet transition in record `13` (zero-based, `(15,13)`):

- `0x58`: `0x00 -> 0x01`
- `0x5C`: `0x00 -> 0x02`
- `0x5D`: `0x00 -> 0x01`

Interpretation:

- `ECMAINT` touched the target world as part of resolving the fleet order
- no new `FLEETS.DAT` records were created
- no global year/schedule change occurred
- the follow-on work should determine whether the `(15,13)` planet-side bytes represent colonization progress, occupation state, or another local status transition

Rust preservation impact:

- `ec-data` now has a second fixture-backed `ECMAINT` transform test covering fleet-side order consumption
- the preservation workflow now covers both a planet build queue case and a fleet order resolution case

## Historical Combat References From Later Text Captures

External reference set:

- `/home/niltempus/Documents/esterian-conquest/ec-logs-2022/`

These are not yet copied into the repo snapshot, but they are useful as
reference evidence for `ECMAINT` combat behavior because they preserve
player-issued orders and the next-year maintenance reports.

### Bombardment sequence: `ec9.txt -> ec10.txt`

In `ec9.txt`, fleet `13` is given a bombard mission:

- current location: `Sector(23,14)`
- mission chosen: `6` bombard a world
- target: `System(24,14)`
- travel time shown by the game: `1 year`
- resulting fleet list entry:
  - `13   4   1    0    4   6  Sector(23,14) Bombard world in System (24,14)`

In `ec10.txt`, the next-year report shows the resolved bombardment:

- report source: `13th Fleet`, located in `System(24,14)`
- planet owner: `Melody Lake` (`Empire #2`)
- defenses reported in the bombardment report:
  - `6 armies`
- bombardment results reported:
  - destroyed `5 armies`
  - destroyed `92%` of factories
  - destroyed `100%` of stored goods
  - destroyed all ships in stardock, including `1 troop transport`
- attacker losses:
  - none
- post-report fleet status:
  - “We are holding our position and are awaiting new orders.”
- matching fleet-list state in `ec10.txt`:
  - `13   0   0    0    4   6  Planet(24,14) No standing orders`

Interpretation:

- a successful bombardment consumes the standing order
- the fleet remains at the target world
- the fleet transitions to a no-standing-orders/hold state after the attack
- `ECMAINT` can directly alter:
  - planet armies
  - factories
  - stored production
  - stardock contents

### Follow-on invade sequence: `ec10.txt -> ec11.txt -> ec12.txt`

In `ec10.txt`, fleet `7` is reordered to invade the same world:

- new orders:
  - `Invade world in System (24,14)`
- fleet-list state after order entry:
  - `7    5   3   10   16   0  Planet(15,13) Invade world in System (24,14)`

In `ec11.txt`, the fleet is still traveling:

- fleet-list state:
  - `7    5   2   10   16   0  Sector(19,13) Invade world in System (24,14)`

In `ec12.txt`, the fleet is one year away:

- fleet-list state:
  - `7    5   1   10   16   0  Sector(24,14) Invade world in System (24,14)`

Interpretation:

- the `brief fleet list` preserves a useful observable movement model:
  - location
  - speed
  - ETA
  - army count
  - ship count
  - ROE
  - standing order text
- this is likely enough to build a future multi-turn invasion fixture once we
  have a compatible pre-maint state generator for mature games

### Fleet-vs-fleet combat reference: `ec11.txt`

Also in `ec11.txt`, fleet `1` reports a move-mission interception:

- `We were attacked by the 3rd Fleet of "In Civil Disorder", (Empire #8)`
- friendly force:
  - `1 cruiser`
  - `1 ETAC ship`
- alien force:
  - `1 destroyer`
- result:
  - enemy fled
  - no enemy ships destroyed
  - no friendly losses

Interpretation:

- `ECMAINT` emits explicit fleet-vs-fleet combat reports even on movement missions
- ROE and fleet composition probably govern:
  - whether interception happens
  - whether one side flees
  - whether losses are exchanged

Practical value for preservation:

- these text captures give us a real expected-output model for bombardment,
  invasion travel, and fleet-vs-fleet interception
- the next combat-oriented black-box fixture should be designed to reproduce a
  simplified bombardment outcome first, because that sequence is the clearest
  and the easiest to validate against observed report language

## Variable-Length Mature Fleet Tables

The repo's `original/v1.5/FLEETS.DAT` is not an invalid file. It is a valid
fleet table with a different record count:

- file size: `702` bytes
- record size: `54` bytes
- inferred record count: `13`

Preservation impact:

- `FLEETS.DAT` is not fixed to the initialized `16 x 54` roster
- the Rust parser now accepts any file length that is an exact multiple of
  `54` bytes
- this allows the mature `original/v1.5` snapshot to be inspected without
  special-case tooling

Observed mature-snapshot fleet shape:

- fleets `2..13` still decode coherently with the current field model
- the first record appears to be a much larger combined combat fleet:
  - `CA=9`
  - `DD=9`
  - `ET=2`
  - standing-order byte `0x1F = 0x04` (still unnamed)
- records `2..5`, `6..9`, and `10..13` still look like linked four-fleet home
  blocks with IDs ending in `0` at the tail, but the first empire block has
  been materially transformed by gameplay

Interpretation:

- the initialized 16-record layout is a starting-state template, not a universal
  fleet-table shape
- real games can collapse, merge, or otherwise restructure those starting
  blocks over time
- this makes the mature snapshot a better future source for combat-oriented
  `ECMAINT` work than the initialized fixtures, once enough planet/player-side
  context is decoded

## Mature `.SAV` Sidecars In `original/v1.5`

The mature `original/v1.5` snapshot includes:

- `BASES.SAV`
- `DATABASE.SAV`
- `FLEETS.SAV`
- `PLANETS.SAV`
- `PLAYER.SAV`

Observed differences relative to the matching `.DAT` files:

- `BASES.SAV` identical to `BASES.DAT`
- `FLEETS.SAV` identical to `FLEETS.DAT`
- `PLAYER.SAV` differs by `1` byte
- `PLANETS.SAV` differs by `3` bytes
- `DATABASE.SAV` differs by `15` bytes

Important detail:

- the changed words repeatedly include:
  - `0x0BCE -> 0x0BCB` in `PLAYER`
  - `0x0BCD -> 0x0BCC` in several `DATABASE` record regions

Best current interpretation:

- these `.SAV` sidecars are not a full clean pre/post-maint snapshot pair
- they look more like partial side backups or stale mirrored views than an
  immediately reusable engine-transition fixture
- they are still worth preserving as evidence, but they should not be treated
  as a ready-made combat scenario source

## Synthetic ECMAINT Bombardment Sequence

Preserved fixtures:

- `fixtures/ecmaint-bombard-pre/v1.5/`
- `fixtures/ecmaint-bombard-arrive/v1.5/`
- `fixtures/ecmaint-bombard-post/v1.5/`

Scenario design:

- baseline: `fixtures/ecutil-init/v1.5/`
- target planet: record `13` (zero-based), coordinates `(15,13)`
- target world was rewritten from `Unowned` into a cloned seeded-colony-style record using the
  bytes from record `12` `(4,13)`, while preserving target coordinates `(15,13)`
- attacking fleet: record `2` (zero-based), fleet `3`
- attacking order:
  - `current_speed = 3`
  - `standing_order = 6` (`Bombard world`)
  - `target = (15,13)`
- attacking ship loadout was increased to force a meaningful combat-style test:
  - `CA=3`
  - `DD=5`
  - `ET=0`

### First maintenance pass: arrival only

Relative to the synthetic pre-maint state:

- `FLEETS.DAT` changed only in fleet `3`
- fleet changes:
  - `current_location.x`: `16 -> 15`
  - standing order stayed `6` (`bombard`)
  - target stayed `(15,13)`
  - current speed stayed `3`
- `PLANETS.DAT` did not change at all
- `MESSAGES.DAT` and `RESULTS.DAT` remained empty
- `DATABASE.DAT` changed
- `CONQUEST.DAT` advanced through normal maintenance/year movement

Interpretation:

- in this synthetic case, arrival at the target and the bombardment attack itself are not resolved
  in the same maintenance pass
- `ECMAINT` moved the fleet onto the target world and preserved the bombard standing order

### Second maintenance pass: combat-style resolution

Relative to the arrival state:

- `FLEETS.DAT` changed only in fleet `3`
- order was consumed:
  - `current_speed`: `3 -> 0`
  - `standing_order`: `6 -> 0`
- location remained `(15,13)`
- attacker losses:
  - `CA`: `3 -> 2`
  - `DD`: `5 -> 1`
- internal fleet-state bytes at `0x19..0x1E` were also reset/rewritten
- `PLANETS.DAT` still did not change at all
- `MESSAGES.DAT` and `RESULTS.DAT` remained empty
- `DATABASE.DAT` changed by `27` bytes
- `CONQUEST.DAT` changed by `3` bytes:
  - year increment
  - one additional small header counter/field

Interpretation:

- this synthetic target encoding is sufficient to trigger fleet-side combat losses
- it is not sufficient to produce visible planet-side damage or player-facing message/report output
- best current inference:
  - either the cloned target world is still missing ownership/defense state from some other file or
    header field
  - or bombardment against this synthetic target resolves as hostile defensive attrition without
    entering the full report-producing planet-damage path

Follow-up comparison against the shipped mature snapshot makes the likely gap
clearer:

- the synthetic target at `(15,13)` was cloned from an initialized seeded world
  shell, not from a mature defended colony
- the initialized seeded shell and the synthetic target both share the same
  compact tail block at `0x58..0x60`:
  - `0a 00 04 00 02 02 00 00 00`
- a mature colony in the shipped snapshot, `Dust Bowl` at `(16,13)`, has a
  materially different tail block:
  - `8e 00 0f 00 02 01 00 00 00`
- that mature-world delta is currently the strongest explanation for why the
  synthetic bombardment produced attacker losses but no planet damage or
  player-facing bombardment report

Best current combat-target inference:

- our synthetic target was only a hostile seeded-colony shell
- a fully valid defended enemy world likely requires additional developed-world
  state beyond the visible coordinates/order bytes we copied
- likely candidates include:
  - matured planetary defense/resource fields inside `PLANETS.DAT`
  - ownership/state consistency with another file such as `DATABASE.DAT` and/or
    empire-linked state outside the single planet record

Next bombardment experiment should therefore clone a mature colony-style target,
not another initialized seed shell.

Additional mature-target throwaway test:

- a second synthetic bombardment scenario cloned the shipped mature colony
  `Dust Bowl` onto `(15,13)` instead of cloning an initialized seed shell
- that target used the mature tail/state block:
  - `8e 00 0f 00 02 01 00 00 00`
- first maintenance pass on that mature target produced:
  - fleet arrival at `(15,13)`
  - standing order rewritten from `6` (`bombard`) to `5` (`guard/blockade`)
  - no `PLANETS.DAT` change
  - no `MESSAGES.DAT` or `RESULTS.DAT` output
- second maintenance pass only zeroed the fleet's current speed while leaving
  the `guard/blockade` standing order in place

Interpretation:

- `Dust Bowl` behaves like a valid mature colony, but not like a hostile target
  for the attacking fleet
- best current inference:
  - the cloned mature planet was treated as friendly or same-empire state
  - so the next bombardment fixture needs a mature enemy colony, not just any
    mature colony record

Hybrid mature-enemy throwaway test:

- a follow-up synthetic target used the mature `Dust Bowl` colony as the base,
  but replaced the likely empire-linked bytes with those from the initialized
  empire-2 seed shell
- resulting target block highlights:
  - `0x20..0x22`: `11 25 1c`
  - `0x58..0x60`: `8e 00 0f 00 02 02 00 00 00`
- first maintenance pass:
  - fleet arrived at `(15,13)`
  - bombard order stayed active (`6`)
  - no `PLANETS.DAT` change
- second maintenance pass:
  - bombard order was consumed (`6 -> 0`)
  - attacker losses:
    - `CA 3 -> 1`
    - `DD 5 -> 1`
  - no `PLANETS.DAT` change
  - no `MESSAGES.DAT` or `RESULTS.DAT` output

Interpretation:

- hostile ownership markers are sufficient to keep the bombard mission active
  through arrival and to trigger attack resolution
- even with mature-world tail bytes, that is still not enough to produce
  visible planet damage or generated combat reports
- the remaining missing state is therefore likely in other `PLANETS.DAT`
  fields that encode a developed enemy colony's defenses/resources, not merely
  in `DATABASE.DAT`

`DATABASE.DAT` structure note:

- file size is `8000` bytes
- it divides cleanly into `80` subrecords of `100` bytes each
- repeated `UNKNOWN` blocks appear every `100` bytes in sparse/empty cases
- `ECPLAYER.DOC` describes this file as the player's planet information
  database, which matches the observed repeated intel-style entries
- best current inference:
  - `DATABASE.DAT` is a derived intel cache, not the authoritative source of
    planet combat state

Conservative `PLANETS.DAT` tail-field candidates:

- `0x5D` is very likely the owning empire slot
  - initialized seed worlds use `1..4` in exactly the expected four-empires
    pattern
  - the colonized world from the fleet-order fixture ends with owner slot `1`
- `0x5C` is likely an ownership/state marker
  - observed as `0x02` on owned colony-style records
- `0x5A` is a strong candidate for army count
  - initialized seed worlds: `4`
  - colonized world from the fleet-order fixture: `0`
  - mature `Dust Bowl` world: `15`
- `0x58` is a developed-world quantity that matters, but it is still unnamed
  - initialized seed worlds: `10`
  - colonized world from the fleet-order fixture: `1`
  - mature `Dust Bowl` world: `142`

These candidates are now exposed conservatively in the Rust parser and CLI as:

- `owner_empire_slot_raw()`
- `ownership_status_raw()`
- `likely_army_count_raw()`
- `developed_value_raw()`

Historical scouting-report reference set:

- the 2012 log bundle contains many repeated scouting reports for the same
  worlds, which gives us planet-side reference values even when we do not have
  matching `.DAT` snapshots for those exact turns
- useful repeated examples include:
  - `Fran` (`Melody Lake`, Empire #2)
    - `ec11.txt`: potential `100`, present `100`, stored `51`, armies `15`,
      ground batteries `5`, stardock `2 cruisers`
    - `ec12.txt`: same except ground batteries `6`
    - `ec13.txt`: same except stored `36`, ground batteries `7`
    - `ec15.txt`: same except ground batteries `8`
  - `33` (`Melody Lake`, Empire #2)
    - `ec17.txt`: potential `33`, present `11`, stored `6`, armies `0`,
      ground batteries `0`, stardock `2 destroyers`
    - `ec18.txt`: present `13`, stored `7`, stardock `1 destroyer`
    - `ec19.txt`: present `14`, stored `9`, stardock `1 destroyer`
  - `90` (`Melody Lake`, Empire #2)
    - `ec13.txt`: potential `90`, present `35`, stored `27`, armies `4`,
      ground batteries `0`
    - `ec14.txt`: present `38`, stored `26`, armies `4`,
      ground batteries `1`

Why this matters:

- these repeated profiles give us expected movement over time for:
  - current production
  - stored goods
  - armies
  - ground batteries
  - docked ships
- they are the best available reference set for naming additional `PLANETS.DAT`
  fields before we have exact matching historical snapshots in repo

Stable companion reference:

- see `docs/planet-report-reference.md` for the coordinate-linked condensed
  version of these report-side target profiles

Controlled bombardment field-isolation result:

- starting from the mature hostile hybrid target, setting `PLANETS.DAT[0x5A]`
  to `0` on the target world changed the two-pass bombardment outcome
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army0-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army0-post/v1.5/`

Observed outcome relative to the army-zero pre-state:

- fleet arrived and completed the bombard sequence over two maintenance passes
- bombard order was consumed
- attacking fleet took no ship losses
  - `CA` stayed `3`
  - `DD` stayed `5`
- target planet changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> 36 33 33 33`
  - bytes `0x08..0x09`: `48 87 -> 3b 85`
  - byte `0x0E`: `04 -> 08`
  - byte `0x58`: `0x8e -> 0x8a`
- `MESSAGES.DAT` and `RESULTS.DAT` still remained empty

Interpretation:

- `0x5A` is now a much stronger candidate for army count, not just a loose
  guess
- changing that single byte was enough to eliminate the attacker losses that
  appeared in the otherwise-similar hostile mature target
- `PLANETS.DAT` now clearly participates in the bombardment damage path through
  bytes outside the already-known tail owner fields

Second controlled bombardment field-isolation result:

- preserving the zero-army target but also setting `PLANETS.DAT[0x58] = 0`
  changed the planet-side damage pattern again
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army0-dev0-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army0-dev0-post/v1.5/`

Observed outcome relative to the `army0+dev0` pre-state:

- fleet outcome stayed the same as the `army0` case:
  - bombard order consumed
  - fleet ended at target
  - no attacker ship losses
- planet damage pattern changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> 03 00 00 00`
  - bytes `0x08..0x09`: `48 87 -> 3a 84`
  - byte `0x0E`: `04 -> 60`
  - byte `0x58`: stayed `0`
- compared directly to the `army0` post-state:
  - `0x04..0x07`: `36 33 33 33 -> 03 00 00 00`
  - `0x08..0x09`: `3b 85 -> 3a 84`
  - `0x0E`: `08 -> 60`
  - `0x58`: `8a -> 00`

Interpretation:

- `0x58` is not just a passive maturity marker
- it participates in the shape or magnitude of world-side bombardment damage
- current best model:
  - `0x5A` strongly affects whether the attacker takes fleet losses
  - `0x58` strongly affects how the target planet record is degraded when the
    attacker does *not* take those losses

Third controlled bombardment field-isolation result:

- starting from the same hostile mature target, setting
  `PLANETS.DAT[0x5A] = 1` produced a clean intermediate outcome between the
  zero-army case and the heavier-loss hostile mature target
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army1-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army1-post/v1.5/`

Observed outcome relative to the `army1` pre-state:

- after two maintenance passes:
  - bombard order was consumed
  - fleet ended at target with partial losses
  - `CA`: `3 -> 2`
  - `DD`: `5 -> 2`
- target planet changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> 3d 3d cc 03`
  - bytes `0x08..0x09`: `48 87 -> 3d 85`
  - bytes `0x0A..0x0D`: `00 00 00 00 -> 44 3e bc ac`
  - byte `0x0E`: `04 -> 46`
  - byte `0x58`: `0x8e -> 0x8d`
  - byte `0x5A`: `0x01 -> 0x00`
- `MESSAGES.DAT` and `RESULTS.DAT` still remained empty

Interpretation:

- `0x5A` now behaves like a graded defense-strength field rather than a simple
  on/off marker
- current bombardment progression from the controlled fixtures is:
  - `0x5A = 0`: no attacker losses
  - `0x5A = 1`: partial attacker losses
  - stronger hostile mature target: heavier attacker losses
- this is the strongest current black-box evidence that `0x5A` is tied to
  army/defender strength and that bombardment resolution scales with it

Fourth controlled bombardment field-isolation result:

- preserving the `army1` target but also forcing `PLANETS.DAT[0x58] = 0`
  changed the fleet-loss profile and the world-damage window again
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army1-dev0-post/v1.5/`

Observed outcome relative to the `army1+dev0` pre-state:

- after two maintenance passes:
  - bombard order was consumed
  - fleet ended at target
  - attacker losses were lighter than the plain `army1` case:
    - `CA`: `3 -> 2`
    - `DD`: `5 -> 4`
- target planet changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> 4f 4c 55 ba`
  - bytes `0x08..0x09`: `48 87 -> 3a 86`
  - bytes `0x0A..0x0D`: `00 00 00 00 -> 06 ea 29 25`
  - byte `0x0E`: `04 -> 35`
  - byte `0x58`: stayed `0`
  - byte `0x5A`: `0x01 -> 0x00`
- `MESSAGES.DAT` and `RESULTS.DAT` still remained empty

Interpretation:

- `0x58` does more than shape planet-side damage once the attacker survives
- at `0x5A = 1`, forcing `0x58 = 0` also changes the fleet-loss profile:
  - plain `army1`: `DD 5 -> 2`
  - `army1+dev0`: `DD 5 -> 4`
- current best bombardment model is therefore:
  - `0x5A` scales defender strength
  - `0x58` modulates both the world-damage pattern and at least part of the
    attacker-loss calculation

Fifth controlled bombardment field-isolation result:

- preserving the `army1+dev0` target but changing byte `0x0E` from `0x04` to
  `0x0c` produced another distinct outcome
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army1-dev0-e0c-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army1-dev0-e0c-post/v1.5/`

Observed outcome relative to the `army1+dev0+0x0E=0x0c` pre-state:

- after two maintenance passes:
  - bombard order was consumed
  - fleet ended at target
  - attacker losses became much heavier than the plain `army1+dev0` case:
    - `CA`: `3 -> 3`
    - `DD`: `5 -> 1`
- target planet changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> 8b 15 60 b5`
  - bytes `0x08..0x09`: `48 87 -> 3e 86`
  - bytes `0x0A..0x0D`: `00 00 00 00 -> d8 c6 49 e3`
  - byte `0x0E`: `0x0c -> 0x54`
  - byte `0x58`: stayed `0`
  - byte `0x5A`: `0x01 -> 0x00`
- `MESSAGES.DAT` and `RESULTS.DAT` still remained empty

Interpretation:

- byte `0x0E` is now a strong candidate for another planet-side defense field
- with `0x58 = 0` and `0x5A = 1` held constant, changing only `0x0E`
  substantially increased attacker losses:
  - plain `army1+dev0`: `DD 5 -> 4`
  - `army1+dev0+0x0E=0x0c`: `DD 5 -> 1`
- current best model is:
  - `0x5A` scales a primary defender-strength component
  - `0x58` modulates both world damage and some attacker-loss behavior
  - `0x0E` likely contributes an additional world-defense factor

Sixth controlled bombardment field-isolation result:

- preserving the `army1+dev0` target but changing byte `0x08` from `0x48` to
  `0x00` produced another distinct combat result
- preserved fixture pair:
  - `fixtures/ecmaint-bombard-army1-dev0-b08-pre/v1.5/`
  - `fixtures/ecmaint-bombard-army1-dev0-b08-post/v1.5/`

Observed outcome relative to the `army1+dev0+0x08=0x00` pre-state:

- after two maintenance passes:
  - bombard order was consumed
  - fleet ended at target
  - attacker losses became heavier than the plain `army1+dev0` case:
    - `CA`: `3 -> 1`
    - `DD`: `5 -> 3`
- target planet changed:
  - bytes `0x04..0x07`: `00 00 00 00 -> c3 34 8c c2`
  - bytes `0x08..0x09`: `00 87 -> 1f 86`
  - bytes `0x0A..0x0D`: `00 00 00 00 -> 06 ea 29 25`
  - byte `0x0E`: `04 -> 7f`
  - byte `0x58`: stayed `0`
  - byte `0x5A`: `0x01 -> 0x00`
- `MESSAGES.DAT` and `RESULTS.DAT` still remained empty

Interpretation:

- byte `0x08` is now another strong candidate for a defense/resource field
- with `0x58 = 0`, `0x5A = 1`, and `0x0E = 0x04` held constant, changing only
  `0x08` substantially increased attacker losses:
  - plain `army1+dev0`: `CA 3 -> 2`, `DD 5 -> 4`
  - `army1+dev0+0x08=0x00`: `CA 3 -> 1`, `DD 5 -> 3`
- current best model for the dense `0x04..0x0E` world block is now:
- `0x04..0x09` is a single 48-bit Borland Pascal `Real` representing `factories` (present capacity).
  - Modifying `0x08` or `0x09` individually in previous experiments altered the MSB and exponent of this `Real`, drastically changing the planet's effective development and thus the bombardment loss calculation.
  - Setting invalid floating-point bytes in `0x04` causes `ECMAINT` to crash with a runtime error during the movement/combat phase.
  - `0x0A..0x0D` is likely `stored goods (production points)`.
  - `0x0E` contributes to defender strength (possibly a forcefield or defense multiplier, as it is not explicitly named in the basic report).
  - `0x58` is **armies**. The generated combat report explicitly matches this byte to the number of armies.
  - `0x5A` is **ground batteries**. The generated combat report explicitly matches this byte to the number of ground batteries.

Eighth controlled field-isolation result (Byte `0x04`):

- attempted to isolate `PLANETS.DAT[0x04]` by changing it to `0x01` in the `army1-dev0-pre` baseline.
- `ECMAINT` crashed on the first run during the movement phase, leaving behind `.TOK` and `.SAV` files.
- on the second run, it detected the crash, output errors to `ERRORS.TXT`, and restored the game from the `.SAV` backups.
- this confirms that `0x04..0x09` acts as a monolithic structured field (a `Real`), and isolated byte edits cause floating-point exceptions (e.g., Runtime error 207) within the original Pascal FPU logic.
- decoded baseline value `00 00 00 00 48 87` = `100.0`, matching the scouting reports for `present` capacity perfectly.

Preservation value:

- this is the first fixture-backed sequence showing a two-step attack lifecycle:
  - year 1: move into bombard position
  - year 2: consume bombard order and inflict/receive ship losses
- even though planet damage was not achieved, the sequence still exposes useful `ECMAINT`
  behavior for later faithful combat modeling

Confirmed `SETUP.DAT` offsets from the live `F4` diffs:

- `SETUP[512]` `snoop_enabled`
- `SETUP[513]` `max_time_between_keys_minutes_raw`
- `SETUP[515]` `remote_timeout_enabled`
- `SETUP[516]` `local_timeout_enabled`
- `SETUP[517]` `minimum_time_granted_minutes_raw`
- `SETUP[518]` `purge_after_turns_raw`
- `SETUP[520]` `autopilot_inactive_turns_raw`

## Fleet Command Surface

Preserved screenshot references:

- `original/v1.5/EC-Screenshots-v1.11/fleet-command-menu.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-h.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o-5.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o-12.png`

Confirmed Fleet Command Center options:

- `H` Help with commands
- `Q` Quit to main menu
- `X` Xpert mode ON/OFF
- `S` STARBASE MENU...
- `V` View partial Starmap
- `B` Brief List of Fleets
- `F` Fleets/Detailed List
- `R` Review a Fleet
- `O` Order fleet on mission
- `C` Change a fleet's ROE
- `A` Alter a fleet's ID
- `E` ETA calculation
- `D` Detach a Fleet
- `M` Merge a Fleet
- `T` Transfer (reassign) ships
- `L` Load Armies to Transports
- `U` Unload Armies from Transport

Confirmed mission code menu under `O` Order fleet on mission:

- `0` None (hold position)
- `1` Move Fleet (only)
- `2` Seek Home
- `3` Patrol a Sector
- `5` Guard/Blockade a World
- `6` Bombard a World
- `9` View a World
- `12` Colonize a World
- `13` Join another fleet

Confirmed order-entry prompt shape:

- the game asks for X/Y destination coordinates for at least:
  - `5` Guard/Blockade a World
  - `12` Colonize a World
- it then prints travel time and resulting ETA year
- it prompts for current speed up to the fleet's maximum speed
- all missions may implicitly include movement if required

Practical implication for the Rust port:

- the preserved screenshots are now enough to build a first faithful text-mode Fleet Command menu
- known raw order codes in `FLEETS.DAT` can be displayed as named mission kinds instead of plain
  numbers

## Most Useful Next Diffs

To label fields efficiently, the best actions are:

1. Order a single fleet to a known coordinate and capture the exact command parameters, then diff `FLEETS.DAT`
2. Issue one isolated planet build order from a fresh snapshot and diff `PLANETS.DAT`
3. Run maintenance and diff `PLANETS.DAT`, `FLEETS.DAT`, `DATABASE.DAT`, and `RESULTS.DAT`
4. Change another empire-level economic setting and diff `PLAYER.DAT`

## Porting Strategy

Recommended approach:

1. Decode file formats.
2. Use before/after diffs from a few in-game actions to label fields.
3. Map command families from the working menus.
4. Reimplement behavior in a modern language from the observed state machine and file model.

Avoid trying to recover original source verbatim. A compatible reimplementation is more realistic.

## Preservation Target

Current recommendation for a preservation-oriented reimplementation:

- target language: `Rust`

Reasoning:

- the main archival goal is long-term maintainability and behavioral correctness, not fastest prototype speed
- `Rust` is a strong fit for:
  - exact binary record parsers/serializers for original `.DAT` files
  - strongly typed game-state models for players, planets, fleets, and maintenance phases
  - conformance tests against the original DOS behavior
  - a clean separation between:
    - core engine
    - file compatibility layer
    - standalone terminal UI
    - optional BBS/door compatibility adapter

Suggested crate layout:

- `ec-core`: rules, turn processing, economy, combat, maintenance
- `ec-data`: original file codecs and compatibility structures
- `ec-cli`: standalone terminal/text interface
- `ec-door`: optional BBS door adapter for legacy use
- `ec-import`: import or convert original EC 1.5 game state

Practical note:

- `Nim` would still be the faster experimentation language
- `Rust` is the better fit if the explicit goal is to preserve the game for posterity

Current scaffold status:

- a Rust workspace now exists under [`rust`](/home/niltempus/dev/esterian_conquest/rust)
- first crate: [`ec-data`](/home/niltempus/dev/esterian_conquest/rust/ec-data)
- first executable tool: [`ec-cli`](/home/niltempus/dev/esterian_conquest/rust/ec-cli)
- preserved fixture sets now include:
  - [`original/v1.5`](/home/niltempus/dev/esterian_conquest/original/v1.5)
  - [`fixtures/ecutil-init/v1.5`](/home/niltempus/dev/esterian_conquest/fixtures/ecutil-init/v1.5)
  - [`fixtures/ecmaint-post/v1.5`](/home/niltempus/dev/esterian_conquest/fixtures/ecmaint-post/v1.5)
- current code covers only confirmed fixed-size boundaries:
  - `PLAYER.DAT`: `5 x 88`
  - `PLANETS.DAT`: `20 x 97`
  - initialized `FLEETS.DAT`: `16 x 54`
  - `SETUP.DAT`: `522`
  - `CONQUEST.DAT`: `2085`
- unknown regions are intentionally preserved as raw byte arrays
- current test status: `cargo test` passes in the original archive workspace and now also in the GitHub-tracked preservation repo
- `ec-cli` now provides a first inspection command against `original/v1.5`
- `ec-cli init` now reproduces the known `ECUTIL` new-game initialization result by overlaying the preserved initialized fixture set onto a target directory
- the post-maint fixture set captures another confirmed RE result:
  - `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `SETUP.DAT` match the initialized baseline after maintenance
  - `CONQUEST.DAT` and `DATABASE.DAT` preserve the global maintenance/output differences
- `ec-cli` now also provides:
  - `headers` to dump the currently known `SETUP.DAT` option prefix and `CONQUEST.DAT` header words
  - `match` to identify whether a directory matches the preserved shipped, initialized, or post-maint fixture states
  - `compare` with integration coverage for the key fixture-state transitions

## Screenshot Archive

Captured gameplay screenshots were copied to:

- [ecv1.5](/home/niltempus/Pictures/ecv1.5)

## 2026-03-10: Fleet Ship Capacities and Planetary Invasions

A successful planetary invasion scenario completely decoded the `FLEETS.DAT` combat ship and troop counts block.

Previous assumptions were that `0x28` (Cruisers) and `0x2A` (Destroyers) were simple 8-bit counts. However, it was discovered that all main ship and troop values are actually stored as **16-bit (little-endian) integers**.

The exact byte mappings in `FLEETS.DAT` (starting at offset `0x24`) are:
- `0x24`: **Scouts** (`u8`)
- `0x26..0x27`: **Battleships** (`u16`)
- `0x28..0x29`: **Cruisers** (`u16`)
- `0x2A..0x2B`: **Destroyers** (`u16`)
- `0x2C..0x2D`: **Troop Transports** (`u16`)
- `0x2E..0x2F`: **Armies** loaded onto transports (`u16`)
- `0x30..0x31`: **ETACs** (Colonization ships) (`u16`)

Fleet Orders were also confirmed based on manual references and game engine reactions. Important order codes:
- `4`: **Guard a Starbase** (persistent — not consumed after maintenance)
- `5`: **Guard/Blockade / Sentry** (default standing order)
- `6`: **Bombard a World**
- `7`: **Invade a World**
- `8`: **Blitz a World**

### The Invasion Experiment
By creating a "heavy attacker" AI fleet with Battleships, Cruisers, Destroyers, Troop Transports, and Armies, then ordering them to "Invade" (`7`) the mature colony at `(15,13)`, `ECMAINT` generated a robust casualty report (`RESULTS.DAT`) and changed the planet's ownership. The surviving troop transport armies successfully populated the captured planet's `0x58` field (Armies).

## Original oracle probe: byte-sized unit limits

Probe script:

- `python3 tools/probe_u8_unit_limits.py`

Artifacts are written under `/tmp/ecgame-u8-unit-limits`.

Confirmed original-oracle outcomes:

1. `PLANETS.DAT[0x58]` planet armies behave as a hard `255` cap under
   `ECMAINT`.
   - Probe: set a player-1 homeworld to `255` armies, queue one army build
     (kind `8`), then run original `ECMAINT`.
   - Result:
     - planet armies remained `255`
     - the build slot cleared (`points -> 0`, `kind -> 0`)
   - Interpretation:
     - a completed army build at cap is silently consumed rather than held,
       wrapped, or extended elsewhere.
   - Rust policy:
     - do not reproduce that silent-loss behavior
     - hold the queued army build in place until the planet has room

2. `PLANETS.DAT[0x5A]` ground batteries behave as a hard `255` cap under
   `ECMAINT`.
   - Probe: set the same homeworld to `255` batteries, queue one battery build
     (kind `7`), then run original `ECMAINT`.
   - Result:
     - ground batteries remained `255`
     - the build slot cleared
   - Interpretation:
     - completed battery builds at cap are also silently consumed.
   - Rust policy:
     - do not reproduce that silent-loss behavior
     - hold the queued battery build in place until the planet has room

3. The fleet scout field `FLEETS.DAT[0x24]` is still byte-sized, but a simple
   merge probe does **not** behave like a clean overflow test.
   - Probe: set two player-1 fleets at the same sector to `200` scouts each,
     zero their other ship counts, flag the player for merge processing, set
     rendezvous-style orders, then run original `ECMAINT`.
   - Result:
     - one fleet survived
     - the surviving fleet still had `200` scouts, not `400`
     - a control run with `100 + 100` scouts also left the survivor at `100`
   - Interpretation:
     - classic fleet merging appears to drop merged-away scout counts entirely
       on this path
     - this is a separate classic scout-merge bug/quirk, not yet a clean
       answer to the scout overflow question

4. A headless `ECGAME` unload probe with a planet at `250` armies and a local
   fleet carrying `20` loaded armies produced no state change in this harness.
   - Do **not** treat that as gameplay evidence yet.
   - The scripted `ECGAME` transport path remains inconclusive until driven with
     a stronger screen-aware automation path.

### Fleet-vs-Fleet Interception

Through targeted scenarios in `ECMAINT`, the mechanics of fleet-vs-fleet combat were decoded:
- Fleets set to `Guard/Blockade` (order `5`) in a system will actively intercept any hostile fleets moving into that system.
- When a fleet enters a guarded sector, combat resolves automatically. 
- During a battle, exact ship counts are logged in the `RESULTS.DAT` combat report (e.g., "Our force contained 100 battleships. Alien force contained 1 cruiser and 1 ETAC ship.").
- If a fleet spots another but *does not engage* (due to ROE settings, lack of hostile orders, or fleeing before combat), it reports the fleet composition using only generic size categories ("large", "medium", "small") and lists them as "of unknown type" rather than identifying exact ship classes.
- The `ROE` (Rules of Engagement) byte at `0x25` governs the willingness to fight vs flee.
- A heavily outgunned fleet (e.g., 1 DD vs 100 BB) will attempt to flee before being destroyed, which is explicitly reported ("The aliens fled before us").
- AI Empires (e.g., "In Civil Disorder") actively defend their planets and intercept approaching player fleets.

This perfectly corroborates the 16-bit ship capacity offsets discovered during the planetary bombardment testing, as the fleet-vs-fleet combat reports correctly enumerated the exact same 16-bit fields for Battleships, Cruisers, Destroyers, and ETACs.

### Planetary Economics and Production

Through black-box simulation of `ECMAINT`, the planetary economic block was decoded. It relies heavily on Borland Pascal 48-bit `Real` values for large numbers (population, factories) and 32-bit `LongInt` for production points.

Confirmed `PLANETS.DAT` fields:
- `0x00`: **X coordinate** (u8).
- `0x01`: **Y coordinate** (u8).
- `0x02..0x03`: **Potential Production / Resource Rating** (2-byte Real prefix: [Mantissa high] [Exponent]).
- `0x04..0x09`: **Factories** (6-byte Borland Pascal Real).
- `0x0A..0x0D`: **Stored Goods (Production Points)** (4-byte LongInt).
- `0x0E`: **Planet Tax Rate** (8-bit, appears to be synced from Player settings during maintenance).
- `0x52..0x57`: **Population** (6-byte Borland Pascal Real).
- `0x58`: **Armies** (8-bit).
- `0x5A`: **Ground Batteries** (8-bit).
- `0x5D`: **Owner Empire** (u8, 1-indexed; 0 = unowned).

Economic Mechanics:
- **Income Generation:** Treasury increases based on planetary Population. 
- **Production:** Factories generate `Stored Goods` (Production Points) which are consumed by the Build Queue (`0x24..0x2E`).
- **Build Completion:** When a build order finishes (e.g., ship or factory), the points are deducted from `Stored Goods` and the build kind byte (`0x2E`) is cleared.

Confirmed `PLAYER.DAT` fields:
- `0x4E..0x4F`: **Last Run Year** (16-bit little-endian year offset/word).
- `0x52..0x55`: **Treasury** (32-bit LongInt).

## 2026-03-10: Starbases and BASES.DAT

### BASES.DAT Record Format

`BASES.DAT` stores starbase records. The original shipped state contains one
35-byte record (1 starbase). The file is 0 bytes when no starbases exist.

Record size: **35 bytes** (mirrors the first 35 bytes of the 54-byte `FLEETS.DAT`
record layout, with fleet-specific fields zeroed or adapted).

Confirmed field map:

| Offset | Size | Value (shipped) | Field |
|--------|------|-----------------|-------|
| `0x00` | u8 | `0x01` | Base local slot |
| `0x02` | u8 | `0x01` | Base active flag |
| `0x04` | u8 | `0x01` | Base ID / count |
| `0x07` | u8 | `0x01` | Unknown (always 1) |
| `0x09` | u8 | `0x00` | Max speed equivalent (0 for bases) |
| `0x0B` | u8 | `0x10` (16) | X coordinate |
| `0x0C` | u8 | `0x0D` (13) | Y coordinate |
| `0x0D` | u8 | `0x80` | Internal flag (same as fleets) |
| `0x13` | u8 | `0x80` | Internal flag (same as fleets) |
| `0x19` | u8 | `0x81` | Internal flag (same as fleets) |
| `0x1F` | u8 | `0x00` | Standing order equivalent (none) |
| `0x20` | u8 | `0x10` (16) | Target/home X (same as 0x0B) |
| `0x21` | u8 | `0x0D` (13) | Target/home Y (same as 0x0C) |
| `0x22` | u8 | `0x01` | Owner empire number |

Full hex of the shipped starbase record:

    0100 0100 0100 0001 0000 0010 0d80 0000
    0000 0080 0000 0000 0081 0000 0000 0000
    100d 01

Evidence:

- The original game at year 3022 has 1 starbase at (16,13) owned by empire 1
  (planet "Dust Bowl").
- Game log `ec10.txt` confirms: "There is a starbase orbiting planet 'zzzzrrr'"
  and fleet 4 with "Guard Starbase 1 now in System (15,13)".
- The `0x0B..0x0C` coordinates and `0x22` owner field were confirmed by matching
  the starbase to the planet and fleet owner.

### Guard Starbase Order (0x04)

`FLEETS.DAT[0x1F] = 0x04` is the "Guard a Starbase" standing order.

For this order to resolve successfully during ECMAINT, two additional fields
must be set correctly:

1. **`FLEETS.DAT[0x22]`**: the **starbase number** to guard (e.g., `0x01` =
   "Starbase 1"). This was previously documented as the mission parameter byte.

2. **`FLEETS.DAT[0x23]`**: must be set to **exactly `0x01`** for Guard Starbase
   to resolve. Values `0x00` and `0x02+` all cause ECMAINT to report "Fleet
   assigned to an unknown starbase" and zero out BASES.DAT. The exact semantics
   of this byte are not yet clear — it may be a guard-mode flag, a secondary
   starbase parameter, or an empire cross-reference.

3. **`PLAYER.DAT[0x44]`**: the **starbase count** for the owning empire. Must
   be `>= 1` for ECMAINT to find the starbase. When set to `0x00`, the lookup
   fails regardless of BASES.DAT contents. Values `>= 2` trigger an integrity
   error ("Game file(s) missing or failed integrity check") but the starbase
   itself still resolves — confirming this is a count, not a boolean.

Guard Starbase is a **persistent standing order**: after a successful maintenance
pass, `FLEETS.DAT[0x1F]` remains `0x04` and BASES.DAT is unchanged (unlike
bombard/invade orders which are consumed on resolution).

When the starbase lookup fails, ECMAINT:

- Writes "Fleet assigned to an unknown starbase" to `ERRORS.TXT`
- Zeros out BASES.DAT (truncates to 0 bytes)
- Clears the fleet's standing order (`FLEETS.DAT[0x1F]` → `0x00`)

### PLAYER.DAT[0x44]: Starbase Count

This field was identified through systematic bisection. Starting from a working
original game state, replacing only `PLAYER.DAT` with the ECUTIL-initialized
version caused the starbase lookup to fail. Bisecting Record 0 (88 bytes) by
removing one group of differing bytes at a time from a full patch identified
`0x44` as the sole essential byte.

Sweep results for `PLAYER.DAT[0x44]`:

| Value | Result |
|-------|--------|
| `0x00` | FAIL — "unknown starbase" error |
| `0x01` | OK — no errors (correct for 1 starbase) |
| `0x02+` | OK starbase-wise, but triggers integrity check error |

This confirms `0x44` is a **starbase count** for the empire, not a flag.

Updated `PLAYER.DAT` Record 0 layout (bytes `0x40..0x57`):

| Offset | Size | Shipped | Init | Field |
|--------|------|---------|------|-------|
| `0x40..0x41` | u16 | `0x0001` | `0x0001` | Unknown (always 1) |
| `0x42..0x43` | u16 | `0x0001` | `0x0004` | Unknown count (fleet groups?) |
| `0x44..0x45` | u16 | `0x0001` | `0x0000` | **Starbase count** |
| `0x46..0x47` | u16 | `0x0001` | `0x0000` | Unknown count |
| `0x48..0x4B` | 4B | `0x00000000` | `0x00000000` | Unknown |
| `0x4C` | u8 | `0x10` (16) | `0x0F` (15) | Homeworld X coordinate |
| `0x4D` | u8 | `0x10` (16) | `0x0F` (15) | Homeworld Y coordinate |
| `0x4E..0x4F` | u16 | `0x0BCE` (3022) | `0x0000` | Last run year |
| `0x50` | u8 | `0x01` | `0x01` | Unknown (always 1) |
| `0x51` | u8 | `0x41` (65) | `0x00` | Tax rate |
| `0x52..0x55` | u32 | varies | varies | Treasury |

### Starbases: Guard Order and Auto-Merge

The Guard Starbase order (`0x04`) has unique behavior regarding fleet management.

**Observed Behavior:**
- When multiple fleets are assigned to the same starbase (using the same mission
  parameter at `0x22`), `ECMAINT` automatically merges them into the lowest-ID
  fleet assigned to that base.
- This merge occurs even if the fleets are in different sectors (e.g., Fleet 1 at
  (16,13) and Fleet 2 at (17,13)).
- The resulting merged fleet inherits the ships from all component fleets and
  continues guarding the starbase.
- The standing order of the "consumed" fleets is reset to `0x05` (Sentry) and
  their ship counts are zeroed out (effectively deleting them as active units).

**Requirements for Order Resolution:**
- `FLEETS.DAT[0x23]` MUST be exactly `0x01`. Any other value causes a lookup
  failure.
- `FLEETS.DAT[0x22]` is an **empire-relative starbase index** (1-indexed).
- The lookup is **empire-specific**: a fleet can only guard a starbase owned
  by its own empire. Assigning a fleet to a starbase index owned by a different
  empire results in an "unknown starbase" error.
- `PLAYER.DAT[0x44]` must reflect the correct total count of starbases owned by
  the empire for the lookup logic to function.
- `BASES.DAT` record must exist at the coordinates where the fleet is ordered
   to guard.

**Persistence:**
- The Guard Starbase order is persistent across maintenance passes as long as
  the starbase exists.

### Rogue/AI Empire Behavior

Rogue empires (`PLAYER.DAT[0x00] = 0xFF`) are processed by `ECMAINT` during the
maintenance pass.

**Observed Behavior:**
- Rogue empires exhibit **automatic defensive clustering**.
- All fleets belonging to a rogue empire are automatically merged into a single
  large fleet at the empire's homeworld (or primary location).
- The standing order of the merged rogue fleet is set to `0x05` (Guard/Blockade).
- The ROE (Rules of Engagement) for rogue fleets is typically set to `10`
  (highly hostile/defensive).
- This auto-merge occurs regardless of the fleets' initial standing orders or
  coordinates.

**Conclusion:**
Rogue empires in Esterian Conquest act as "stationary" or "defensive" AI blocks
that consolidate their forces at their primary system when maintenance runs.

Evidence: the four homeworld planets (those with `0x03 = 0x87`) have `0x5D`
values of 1, 2, 3, 4 corresponding to the four empires. Non-homeworld planets
have `0x5D = 0x00`.

### Bisection Methodology

The starbase findings were produced using a systematic binary search approach:

1. Confirm a known-good baseline (original shipped state runs ECMAINT
   successfully with Guard Starbase).
2. Confirm the failure case (init state with BASES.DAT and fleet order patch
   produces "unknown starbase" error).
3. File-level bisection: replace one file at a time from original→init to find
   which file causes the failure. Result: PLAYER.DAT alone breaks it; all other
   files are safe to swap individually.
4. Record-level bisection: patch Record 0 (bytes 0x00-0x57) from original into
   init PLAYER.DAT. Result: Record 0 alone is sufficient.
5. Byte-group bisection: apply all Record 0 patches, then remove one logical
   group at a time. Result: only `0x44` is essential.
6. Cross-file interaction: when both PLAYER.DAT and FLEETS.DAT are from init,
   the `0x44` patch alone is insufficient. Bisecting fleet record bytes found
   `0x23` as the second essential byte.

### Minimum Working Init-Based Starbase Fixture

To create a working Guard Starbase scenario from the ECUTIL-initialized state,
three patches are required:

1. `FLEETS.DAT[0x1F] = 0x04` — set fleet 0 to Guard Starbase order
2. `FLEETS.DAT[0x23] = 0x01` — set the starbase resolver byte
3. `PLAYER.DAT[0x44] = 0x01` — set empire 1 starbase count to 1
4. Add `BASES.DAT` with a valid 35-byte starbase record at (16,13) for empire 1

### End-to-End Verification (Confirmed)

The full init-based fixture with all three patches was verified end-to-end:

- **Pass 1**: No errors. BASES.DAT unchanged (35 bytes). FLEETS.DAT unchanged
  (order 0x04 persists). Guard Starbase is confirmed persistent.
- **Pass 2**: No errors. BASES.DAT, FLEETS.DAT, PLAYER.DAT, PLANETS.DAT all
  identical to pass 1. Only CONQUEST.DAT byte 0 changed (year 3001 → 3002).
- Pre/post fixtures preserved in `fixtures/ecmaint-starbase-pre/v1.5/` and
  `fixtures/ecmaint-starbase-post/v1.5/`.

### PLAYER.DAT[0x46..0x47]: Starbase-Path Normalization

Follow-up sweep results for `PLAYER.DAT[0x46..0x47]` in the working Guard
Starbase fixture:

- baseline fixture: `fixtures/ecmaint-starbase-pre/v1.5/`
- tested pre-maint values: `0x0000`, `0x0001`, `0x0002`
- all three values resolved successfully through `ECMAINT` with:
  - no `ERRORS.TXT`
  - unchanged `BASES.DAT` (35 bytes)
  - persistent Guard Starbase order in fleet 0

Observed post-maint result:

- `PLAYER.DAT[0x44..0x47]` always ended as `01 00 01 00`
- the only `PLAYER.DAT` byte change between
  `fixtures/ecmaint-starbase-pre/v1.5/` and
  `fixtures/ecmaint-starbase-post/v1.5/` is at byte `0x46`: `0x00 -> 0x01`

Interpretation:

- `PLAYER.DAT[0x46..0x47]` is **not a required input gate** for Guard Starbase
  lookup or persistence
- instead, it appears to be a maintained or derived empire-level count/status
  that `ECMAINT` normalizes to `0x0001` on this successful starbase path

Further follow-up probes tightened this model:

- starting from `fixtures/ecmaint-starbase-pre/v1.5/`, changing fleet 0 from
  Guard Starbase (`0x04`) to Guard/Blockade (`0x05`) still produced post-maint
  `PLAYER.DAT[0x44..0x47] = 01 00 01 00`
- starting from `original/v1.5/`, zeroing `PLAYER.DAT[0x46..0x47]` before a
  maintenance pass also returned it to `0x0001`, both with the original Guard
  Starbase order intact and with fleet 0 changed to Guard/Blockade
- when the starbase was removed (`BASES.DAT` zeroed) or the owning empire's
  starbase count at `PLAYER.DAT[0x44..0x45]` was forced to `0x0000`,
  `PLAYER.DAT[0x46..0x47]` did **not** normalize to `0x0001`
- preserved non-starbase fixture families (`ecmaint-post`, build, fleet,
  economics, bombardment, invasion, fleet-battle, movement) do not change
  `PLAYER.DAT[0x46..0x47]`

Refined interpretation:

- `PLAYER.DAT[0x46..0x47]` is still not a Guard Starbase input gate
- it is also not specific to standing order `0x04`
- the strongest current model is that it is another maintained starbase-related
  count or flag, set to `0x0001` when `ECMAINT` recognizes a valid starbase for
  the empire through the combination of `BASES.DAT` and
  `PLAYER.DAT[0x44..0x45]`
- the next decisive experiment is a two-starbase scenario to determine whether
  it scales to `0x0002` or behaves like a boolean-style presence flag

### BASES.DAT[0x04]: Starbase Identity / Number Candidate

Follow-up multi-starbase probing on `fixtures/ecmaint-fleet-post/v1.5/` produced
the strongest evidence so far that `BASES.DAT[0x04]` is the actual starbase
identity/number field, while `BASES.DAT[0x00]` is not sufficient to define a
distinct second base.

Why this fixture was used:

- it already has two empire-1 planets at `(15,13)` and `(16,13)`
- a synthetic one-base state can be added there cleanly and accepted by
  `ECMAINT`

Observed results:

- adding one starbase record at `(16,13)` with `PLAYER.DAT[0x44..0x45] = 0x0001`
  succeeds cleanly and normalizes `PLAYER.DAT[0x46..0x47]` to `0x0001`
- adding a second record with:
  - `BASES.DAT[0x00] = 0x02`
  - `BASES.DAT[0x04] = 0x02`
  - coordinates `(15,13)`
  - `PLAYER.DAT[0x44..0x45] = 0x0002`
  causes the standard cross-file integrity failure
- changing only the second record's local slot-like byte (`0x00 = 0x02`) while
  leaving `BASES.DAT[0x04] = 0x01` does **not** produce a valid second base;
  instead `ECMAINT` accepts the run and canonicalizes `BASES.DAT` back down to a
  single 35-byte record
- the same collapse-to-one-base behavior occurs even if the duplicate record is
  placed first or second in the file

Additional negative probes:

- changing record-local flags at `BASES.DAT[0x02]`, `0x07`, or `0x19` did not
  make `BASES.DAT[0x04] = 0x02` pass integrity
- pre-linking a fleet to Guard Starbase 2 (`FLEETS.DAT[0x1F] = 0x04`,
  `FLEETS.DAT[0x22] = 0x02`, `FLEETS.DAT[0x23] = 0x01`) also did not clear the
  integrity gate

Interpretation:

- `BASES.DAT[0x04]` is the strongest current candidate for the actual
  empire-relative starbase number (`1`, `2`, `3`, ...)
- `BASES.DAT[0x00]` is not the decisive identity field; it behaves more like a
  local slot/order byte
- a real Starbase 2 requires at least one additional companion structure beyond:
  - a second empire-owned planet
  - a second 35-byte base record
  - `PLAYER.DAT[0x44..0x45] = 0x0002`
  - optional Guard Starbase fleet linkage

This aligns with the historical logs, which clearly show real `Starbase 2` and
`Starbase 3` states in later campaigns (`hector` and `helix` respectively), so
the current blocker is missing cross-file bookkeeping rather than a design limit
of one starbase per empire.

### One-Base Synthetic Runs: Side Effects Are Not Base-Coordinate Indexed

To look for the hidden companion structure required by `Starbase 2`, two
accepted single-base runs were compared on the same baseline fixture
(`fixtures/ecmaint-fleet-post/v1.5/`) with only the starbase coordinates changed:

- run A: one base at `(15,13)`
- run B: one base at `(16,13)`

Both runs succeeded cleanly and produced the same empire/global side effects:

- `PLAYER.DAT[0x44..0x47]`: `00 00 00 00 -> 01 00 01 00`
- `CONQUEST.DAT[0x00]`: year byte incremented by 1
- `CONQUEST.DAT[0x0A]`: `100 -> 101`
- `CONQUEST.DAT[0x3C]`: `1 -> 2`

The only coordinate-sensitive persistent change was in the target planet record
itself. In both accepted runs, exactly one 6-byte cluster changed in planet
record 13 (`(15,13)`):

- bytes `0x03..0x08` were rewritten to a new 6-byte value
- the value differed between the `(15,13)` and `(16,13)` starbase placements

However, the derived/indexed files did **not** vary with starbase location:

- `DATABASE.DAT` changed at the same offsets in both runs and with the same
  replacement values
- `CONQUEST.DAT` changed at the same offsets in both runs and with the same
  replacement values

Interpretation:

- the accepted synthetic one-base path does not reveal any obvious starbase-ID-
  or starbase-coordinate-indexed companion structure in `DATABASE.DAT` or the
  already-observed `CONQUEST.DAT` header bytes
- the hidden `Starbase 2` companion bookkeeping is therefore more likely to live
  in a different region/file or to require a multi-base-specific relationship
  that a single-base run cannot expose

### Planet Record `0x03..0x08`: Starbase-Related Rewrite, But Not Sufficient For Starbase 2

The accepted one-base runs on `fixtures/ecmaint-fleet-post/v1.5/` consistently
rewrite a 6-byte cluster in the target planet record:

- target record: planet 13 at `(15,13)`
- changed bytes: `PLANETS.DAT[record13 + 0x03..0x08]`

Observed values:

- baseline (`fixtures/ecmaint-fleet-post/v1.5/`): `81 00 00 00 00 00`
- accepted one-base run with base at `(15,13)`: `84 1f 85 eb 51 64`
- accepted one-base run with base at `(16,13)`: `84 dd 24 06 81 61`

Important behavior:

- the rewrite happens only on the `(15,13)` record, even when the accepted base
  is placed at `(16,13)`
- the replacement bytes differ based on which single-base placement was used
- no other persistent planet bytes changed in those accepted one-base runs

Follow-up test:

- pre-seeding this 6-byte cluster on the candidate second planet before a
  two-base run does **not** make `Starbase 2` pass integrity
- tested with both observed accepted one-base cluster values, and with both
  planets seeded

Interpretation:

- `PLANETS.DAT[0x03..0x08]` is starbase-related or starbase-side-effect-related
  state
- however, it is **not sufficient** as the missing companion structure for a
  valid `Starbase 2` setup
- the remaining multi-base gate therefore still points to some other linked
  bookkeeping outside this planet-side cluster

### Build Queue Follow-Up: No Delayed Fleet Materialization After Pass 2

The minimal preserved build-queue fixture was re-run for two consecutive
maintenance passes starting from `fixtures/ecmaint-build-pre/v1.5/`.

Pass 1 reproduced the original known transition exactly:

- `PLANETS.DAT` queue bytes cleared:
  - record 14 byte `0x24`: `0x86 -> 0x00`
  - record 14 byte `0x2E`: `0x0C -> 0x00`
- replacement planet-state bytes appeared:
  - record 14 byte `0x38`: `0x00 -> 0x03`
  - record 14 byte `0x4C`: `0x00 -> 0x01`
- `FLEETS.DAT` remained unchanged

Pass 2 result:

- `PLANETS.DAT` unchanged from pass 1
- `FLEETS.DAT` unchanged from pass 1
- only small derived churn remained:
  - `DATABASE.DAT`: 12 bytes changed
  - `CONQUEST.DAT`: byte `0x00` incremented by 1

Interpretation:

- the current minimal queue fixture is a real planet-state transition, not a
  delayed ship/fleet materialization that completes on the next maintenance pass
- follow-up build experiments should therefore vary the build encoding or the
  supporting economic/planet state rather than simply running additional passes

### Fleet Movement: Speed and Distance

The movement formula was recovered by observing Fleet 1 moving horizontally from
(16,13) with varying speeds across multiple maintenance passes.

**Movement Model:**
- Distance moved per pass is approximately `speed / 1.5`.
- Specifically, the following patterns were observed over 3 passes:

| Speed | Pass 1 | Pass 2 | Pass 3 | Total | Avg/Pass |
|-------|--------|--------|--------|-------|----------|
| 1     | 1      | 0      | 1      | 2     | 0.67     |
| 2     | 1      | 2      | 2      | 5     | 1.67     |
| 3     | 2      | 3      | 3      | 8     | 2.67     |

**Observations:**
- At Speed 1: Moves 1 unit on turn 1 and turn 3.
- At Speed 2: Moves 1 unit on turn 1, then 2 units subsequently.
- At Speed 3: Moves 2 units on turn 1, then 3 units subsequently.
- The first turn often shows a "startup penalty" of -1 unit compared to later
  turns for Speed 2 and 3, or it's a simple rounding effect.
- The long-term average strictly follows `distance = speed / 1.5`.

**Coordinate Update:**
- `FLEETS.DAT[0x0B..0x0C]` stores the current X, Y coordinates.
- These are updated during the maintenance pass based on the standing order.
- Move Only order (`0x01`) consumes the `current_speed` (`0x0A`) to reach the
  target coordinates (`0x20..0x21`).

**Fixture Details:**
- `fixtures/ecmaint-move-pre/v1.5/`: Fleet 1 at (16,13), Speed 3, Move to (26,13).
- `fixtures/ecmaint-move-post/v1.5/`: After 3 passes, Fleet 1 at (24,13).

## Environment Setup Notes

**Linux Headless Environments:**
- Using `dosbox-x` with `xvfb-run` is necessary for headless CI/CD or background tasks because `ECMAINT` requires a display context.
- **Critical:** The default SDL1 build of `dosbox-x` on some modern distributions (e.g. CachyOS / Arch) consistently segmentation faults in this setup (`Can't init SDL Couldn't open X11 display` followed by a segfault when switching to dummy/ttf drivers).
- **Resolution:** Use the SDL2 build (`dosbox-x-sdl2` from AUR). It correctly negotiates headless virtual X11 sessions and avoids the crash.
- **Pro Tip:** When using the SDL2 build, you can bypass `xvfb-run` entirely by setting `export SDL_VIDEODRIVER=dummy`. This runs `dosbox-x` headlessly with zero display overhead.

## Planet Stardock / Build Queue Notes

The mysterious bytes `0x38` and `0x4C` that appeared in `PLANETS.DAT` after clearing the build queue are highly likely to be the **Stardock**.

- `0x24..0x2D` acts as the active "Build Queue" (array of 10 `u8` quantities).
- `0x2E..0x37` acts as the active "Build Queue Types" (array of 10 `u8` types).
- When `ECMAINT` processes these, it consumes `Stored Goods`.
- Upon completion, the ships are moved to the planet's Stardock.
- `0x38..0x4B` corresponds to the count of built ships (array of 10 `u16` counts).
- `0x4C..0x55` corresponds to the ship type currently occupying that slot in the Stardock (array of 10 `u8` types).
- Ships in the Stardock do not automatically launch or appear in `FLEETS.DAT`. They remain docked on the planet until explicitly "Commissioned" (as observed in `WHATSNEW.DOC`: "AUTO-COMMISSION: Commission Fleets and starbases in all stardocks").

This perfectly explains why `FLEETS.DAT` didn't change on a second `ECMAINT` pass; the ships simply sat in the Stardock waiting for a player command to commission them into an active fleet.

### Full Stardock Completion Probe

Focused original-`ECMAINT` probe:

- baseline: `fixtures/ecmaint-build-pre/v1.5`
- target planet: record `15`
- mutation:
  - fill all `10` stardock slots with destroyers
  - leave the existing pending destroyer build in queue slot `0`
- harness:
  - `python3 tools/probe_full_stardock_build.py /tmp/ecmaint-full-stardock-build`

Observed original behavior:

- `ECMAINT` completed the pending build and cleared the build slot
- it did **not** emit `ERRORS.TXT`
- it did **not** preserve the full stardock cleanly
- instead, the target planet's stardock region in `PLANETS.DAT` became corrupted:
  - counts like `65535, 0, 65535, 0, ...`
  - kinds like `255, 255, 0, 0, 255, 255, ...`

Conclusion:

- full-stardock completion is not a safe or well-behaved classic state
- the Rust client-side guard that prevents queueing additional ship/starbase
  builds when no stardock slot remains should stay in place
- Rust maintenance should also guard it:
  - ship/starbase builds that would complete into a full stardock should remain
    queued unchanged until a slot opens
  - armies and ground batteries should still complete normally because they do
    not use stardock
- treating this as a precondition failure is more faithful than allowing the
  state and hoping maintenance handles it cleanly

## Guard Starbase Linkage Keys

Artifact:
- `artifacts/ghidra/ecmaint-live/summary-key-sources.txt`

Script:
- `tools/ghidra_scripts_tmp/ReportSummaryKeySources.java`

The most actionable matcher inputs are now pinned down to raw file words instead
of vague scratch names.

Kind-`1` summary sources (`2000:6040..6368`):
- summary `+0x0A` always comes from fleet raw `0x00..0x01`
  - `2000:6158 -> 6160`
  - `2000:62BA -> 62C2`
- primary-branch summary `+0x06` comes from player raw `0x40..0x41`
  - `2000:61E7 -> 61EF`
- follow-on summary `+0x06` comes from fleet raw `0x05..0x06`
  - `2000:62E5 -> 62ED`

Kind-`2` summary sources (`2000:63D3..6759`):
- summary `+0x0A` comes from base raw `0x02..0x03`
  - `2000:64EB -> 64F3`
  - `2000:6645 -> 664D`
- primary-branch summary `+0x06` comes from player raw `0x44..0x45`
  - `2000:6576 -> 657E`
- follow-on summary `+0x06` comes from base raw `0x07..0x08`
  - `2000:66C4 -> 66CC`

Matcher consequence (`0000:03DF..06AE`):
- direct accept path compares candidate kind-`1` summary `+0x0A` against
  decoded `[0x3558]`
- structural accept path decodes candidate kind-`1` summary `+0x06`, then
  requires:
  - decoded kind byte `== 4`
  - decoded word `== [0x355A]`
  - decoded flag byte `== 0`

Practical one-base inference:
- the preserved accepted one-base Guard Starbase case aligns all obvious raw key
  words to `1`
  - player `0x44..0x45 = 0x0001`
  - fleet `0x00..0x01 = 0x0001`
  - fleet `0x05..0x06 = 0x0001`
  - base `0x07..0x08 = 0x0001`
- for Rust-side compliant generation, the next useful abstraction is:
  - fleet direct-match key = raw `0x00..0x01`
  - fleet structural key = raw `0x05..0x06`
  - base/player direct decode source = player `0x44..0x45`
  - base structural decode source = base `0x07..0x08`

Accepted one-base matcher decode capture:

Artifacts:
- `artifacts/ecmaint-kind2-debug/base_decode_registers.txt`
- `artifacts/ecmaint-kind2-debug/base_decode_3558.txt`
- `artifacts/ecmaint-kind2-debug/summary.txt`

Tool:
- `tools/capture_ecmaint_kind2_decode.py`

New live translation anchor:
- raw matcher post-decode stop `0000:0403`
- live stop surfaced as `CS=0824 EIP=0303`

Accepted one-base decoded base-side matcher values:
- `[3558] = 0x0001`
- `[355A] = 0x0001`
- `[3563] = 0x10`
- `[3564] = 0x0D`
- `[3578] = 0x10`
- `[3579] = 0x0D`
- `[357A] = 0x01`

Practical interpretation:
- the accepted one-base case gives the matcher decoded key words equal to `1`,
  matching the preserved direct-linkage raw words
- the decoded tuple/control bytes also line up with the accepted guard target
  coordinates `(16,13)`
- this strengthens the hypothesis that the accepted one-base case succeeds on
  the direct decoded-key path (`candidate +0x0A == [3558]`) and may not need
  the later structural candidate-decode path at all

Failing `unknown starbase` matcher comparison (`fleet[0x23] = 0`):

Artifacts:
- `artifacts/ecmaint-kind2-debug/badenable/base_decode_registers.txt`
- `artifacts/ecmaint-kind2-debug/badenable/base_decode_3558.txt`
- `artifacts/ecmaint-kind2-debug/badenable/sanity_errors.txt`

Capture notes:
- source directory was the accepted one-base fixture with only `fleet[0x23]`
  cleared to `0`, which reliably produces:
  - `Fleet assigned to an unknown starbase.`
- the generalized capture helper was updated to:
  - accept `label` and source-directory arguments
  - preserve expected failing `ERRORS.TXT` output instead of aborting the run
  - write partial base-side artifacts as soon as the base decode is captured

Recovered failing-case base decode:
- live stop is still `CS=0824 EIP=0303`
- registers match the accepted one-base capture exactly:
  - `824 303 3529 3529 39ab f50c fd96 0 b 23 64 3538 2ff8`
- `DS:3558` dump is byte-for-byte identical to the accepted one-base case:
  - `[3558] = 0x0001`
  - `[355A] = 0x0001`
  - `[3563] = 0x10`
  - `[3564] = 0x0D`
  - `[3578] = 0x10`
  - `[3579] = 0x0D`
  - `[357A] = 0x01`

Practical consequence:
- the later `unknown starbase` failure is not caused by a difference in the
  base-side decoded matcher object
- the remaining discriminator must be later in the kind-`1` candidate-side
  match path or in a follow-on guard/flag check after the identical base decode

Static consequence from the fleet-side emitter:
- the `2000:6040..6368` kind-`1` summary emitter reads:
  - fleet `0x00..0x01` into summary `+0x0A`
  - fleet `0x05..0x06` into the follow-on summary `+0x06`
  - fleet `0x0B` / `0x0C` into summary `+0x01` / `+0x02`
  - fleet `0x19..0x1D` into the derived summary `+0x05`
- it does **not** read fleet `0x22` or `0x23` while building the kind-`1`
  summary entries
- practical implication:
  - the failing `fleet[0x23] = 0` case cannot be explained by a different
    emitted kind-`1` summary shape from `2000:6040..6368`
  - the remaining `unknown starbase` discriminator is later than:
    - the base-side kind-`2` decode
    - the fleet-side kind-`1` summary emission
  - the next highest-value target is therefore the post-match guard/flag
    resolution path rather than more matcher-key captures

Immediate post-match handoff dump (`0000:06AE..0800`):

Artifacts:
- `artifacts/ghidra/ecmaint-live/kind2-post-match.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportKind2PostMatch.java`

Recovered shape:
- `0000:06AE` immediately jumps to `0000:079E`
- the `0000:06B1..079A` block is a kind-`3` / `IPBM`-specific setup path:
  - `CMP AL,0x3`
  - decodes from `0x3538`
  - seeds tuple locals used by the later shared pipeline
- `0000:079E..` is the already-known common post-kind canonicalization path
  beginning at the same `0000:07DA` cluster previously tied to summary
  canonicalization

Practical consequence:
- the kind-`2` starbase path does not have a rich starbase-specific handoff
  immediately after `0000:06AE`; it falls directly into the generic
  canonicalization stage
- combined with the identical base-side decode and the fact that fleet
  `0x23` is not part of kind-`1` summary emission, the remaining
  `unknown starbase` discriminator is now best modeled as later than:
  - the base-side matcher decode
  - the fleet-side summary emission
  - the immediate post-match handoff into common canonicalization
- next best target: the later consumer of canonicalized kind-`1` / kind-`2`
  summaries that still has access to guard/order semantics

Later post-canonical consumer (`0000:108c..1400`):

Artifacts:
- `artifacts/ghidra/ecmaint-live/summary-post-canonical.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportSummaryPostCanonical.java`

Recovered structure:
- `0000:1104..123E` is a generic summary post-processing pass:
  - seeds summary word `+0x08` for every entry via helper `0x3000:4d2a`
  - then sorts / swaps the 12-byte summary records by that derived `+0x08`
    key using repeated `0x3000:4136` copies
- `0000:123E..12FD` appears to emit generic report/header material through
  repeated `0x3000:3f88`, `0x3000:4057`, and `0x3000:40ed` calls
- the first later active-summary consumer is `0000:1302..1361`:
  - loops summary index `1..[0x2F76]`
  - filters on summary active/status byte `+0x03 != 0`
  - for each active summary:
    - `CALL 0000:02C0` (shared summary loader/normalizer)
    - `CALLF 0x1000:a26e`
    - `CALLF 0x2000:c057`
    - `CALLF 0x1000:0b51`
    - `CALLF 0x2000:c057`

Practical consequence:
- this `0000:1302..1361` loop is the first concrete later consumer after the
  matcher/canonicalizer that still processes each active summary entry
- it is now the best current target for the unresolved `unknown starbase`
  behavior, because it sits later than:
  - base-side decode
  - fleet-side kind-`1` summary emission
  - immediate post-match handoff
  - generic sort/report staging
- next highest-value step: identify whether the segment-`1000` callees
  `a26e` and `0b51` are the starbase-resolution / error-emission path for
  active kind-`1` / kind-`2` summaries

Late active-summary callee dump:

Artifacts:
- `artifacts/ghidra/ecmaint-live/late-summary-callees.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportLateSummaryCallees.java`

First-pass read:
- `1000:a26e` does not look starbase-specific
  - it dispatches on a local byte value `1/2/3`
  - increments a local 32-bit pair by small constants (`+2`, `+7`, `+0x15`)
  - clamps local bytes to maxima like `0x0a`, `0x0f`, `0x06`, `0x05`
  - this looks more like generic layout/ranking/scoring state than guard
    resolution
- `1000:0b51` also looks generic/report-oriented
  - writes a fallback triple `0x81,0,0` into an object at `ES:[DI+3..7]`
  - formats through `0x3000:487d` / `0x3000:486b`
  - reads player-table state through `0x16ac`
  - but does not obviously branch on Guard Starbase-specific fields

Practical consequence:
- the first two later segment-`1000` callees reached from the active-summary
  loop do not yet look like the real `unknown starbase` discriminator
- they are more plausibly generic post-processing/report helpers
- next best target is still within the `0000:1302..1361` call chain, but not
  these two callees in isolation

Raw `unknown starbase` string anchor:

Artifacts:
- `artifacts/ghidra/ecmaint-live/unknown-starbase-string-refs.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseStringRefs.java`

Result:
- raw string was found in `/tmp/ecmaint-debug/MEMDUMP.BIN` at `0000:3f89`
- the raw-import xref pass still found no direct references to that address

Practical consequence:
- the message is likely reached indirectly via a table/helper path, not via a
  direct code reference the raw import can see cleanly

Starbase-specific later consumer after the raw `unknown starbase` string:

Artifacts:
- `artifacts/ghidra/ecmaint-live/unknown-starbase-region.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseRegion.java`

Recovered structure:
- the raw string `Fleet assigned to an unknown starbase.` is immediately
  followed by a real code region at `0000:3fcf..41a0`
- that region operates directly on the kind-`1` scratch block rooted at
  `0x3502`
- entry behavior:
  - clears local success flag `[BP-1] = 0`
  - calls into `0x1000:d183` with source `0x3502`
  - receives an index/result in `[BP-0x28]`
- main comparison path (`0000:3ff4..40a4`):
  - compares the current summary entry against the located entry on:
    - summary byte `+0x01`
    - summary byte `+0x02`
    - summary byte `+0x05`
  - then checks `byte ptr [0x350c] > 0`
  - on success sets local success flag `[BP-1] = 1`
- failure/report path (`0000:40a7..4184`):
  - works from scratch block `0x3502`
  - calls `0x2000:cb32`
  - formats several CS-local string fragments via `0x3000:4202`,
    `0x3000:428f`, and `0x2000:bec4`
  - reads:
    - `0x3525`
    - tuple words `0x351b..0x351f`
    - tag bytes `0x350d` and `0x350e`
    - byte `0x3504`
  - then clears:
    - `0x350c`
    - `0x3521`
- no-match early exit (`0000:4186..4195`):
  - calls `0x3000:159b` using CS:`0x0b17`
  - then also clears `0x350c` and `0x3521`

Strong practical consequence:
- this `0000:3fcf..41a0` region is the first later consumer that looks
  genuinely starbase-specific rather than generic
- it is now the best current candidate for the later Guard Starbase
  resolution/error-emission path behind the `unknown starbase` behavior
- the late predicate is now narrowed further:
  - caller-side current summary index is `[BP+0x04]`
  - located candidate summary slot is local `[BP-0x28]`
  - success requires:
    - located summary active (`+0x03 != 0`)
    - current `+0x01 == located +0x01`
    - current `+0x02 == located +0x02`
    - current `+0x05 == located +0x05`
    - `byte ptr [0x350c] > 0`
  - on success the routine only sets local success flag `[BP-1] = 1`
- the failure/report path is also now tightened:
  - it formats output from kind-`1` scratch fields:
    - `3525`
    - `351b..351f`
    - `350d`
    - `350e`
    - `3504`
  - branch `40f7..410c` selects between two nearby CS-local string variants
    depending on whether `351b..351f` is zero
  - both failure/report exits clear `350c` and `3521`
- early no-match path `4186..4195` emits a separate CS-local message through
  `0x3000:159b`, then also clears `350c` and `3521`
- a focused scalar-xref pass on `3504`, `350c`, `350d`, `350e`, `351b`,
  `351d`, `351f`, `3521`, and `3525` found no clean raw-import references
  outside this later region
  - practical consequence:
    - these should currently be treated as late-path scratch/state fields, not
      reliable named globals in the raw import
- the late report payload is now split more concretely:
  - `350d` / `350e` are the first two decoded tag bytes from the shared
    kind-`1` summary `+0x06` decoder
  - `351b..351f` is the later 3-word payload group consumed by the same
    kind-`1` dispatch path
  - the common post-kind pipeline writes those fields directly at:
    - `0c7a` -> `350d`
    - `0ca4` -> `350e`
    - `0cc9..0cd0` -> `351b..351f`
  - `350c` is the decoded selector/control byte copied out by the kind-`1`
    loader and later checked by the late starbase predicate
  - practical consequence:
    - the late starbase report block is consuming the shared kind-`1`
      canonicalized summary payload, not ad hoc local data
    - the real remaining late-path unknowns are now narrower:
      - `3521`
      - exact caller-side `AX` / located-summary-slot relationship at `3fe8`
- a second late starbase-specific block is now pinned down at
  `0000:42d8..456e`
  - it works from the same `3502` scratch block
  - scans active summaries in a later loop
  - requires candidate summary identity on:
    - entry byte `+0x00 == [0x3504]`
    - entry bytes `+0x01/+0x02 == [0x350d]/[0x350e]`
    - entry byte `+0x05 == f(351b..351f)` via `0x3000:489d`
  - rejects direct `+0x0A == [0x3502]` matches before taking the deeper path
  - then decodes candidate summary `+0x06` through `0x2000:c067`
  - requires:
    - decoded kind byte `== 4`
    - decoded local word at offset `+0x23` (`[BP-0x17]`) == `[0x3525]`
    - decoded local flag byte at offset `+0x0a` (`[BP-0x30]`) == `0`
  - practical consequence:
    - `3525` is no longer an opaque late scratch word
    - it behaves as the kind-`1` side comparison word paired against decoded
      local `+0x23`, analogous to how earlier matcher logic compares candidate
      decoded `+0x23` against base-side `[355A]`
  - the later half of that block is now clarified as a real resolution/report
    loop, not just another selector helper:
    - after a structural hit it calls `0x2000:b9a7`
    - that splits into two nearby CS-local report families around `0xd30` and
      `0xd53`
    - the `b9a7 != 0` branch uses the smaller two-fragment family and then
      calls `0x2000:d3bb`
      - practical consequence:
        - this is now best modeled as the merge/commit path after the deeper
          structural match
    - the `b9a7 == 0` branch uses the larger multi-fragment family, includes
      a formatted literal `0x0bb8` (`3000`), and does not call `0x2000:d3bb`
      before resetting `3521` / `350c`
      - practical consequence:
        - this is now best modeled as the already-guarding / ship-limit
          abort-report path rather than a successful merge path
    - the fallback path at `451b..456e` emits an additional message, re-runs
      `0x1000:d183`, copies the selected entry back through `0x2000:c151`,
      rewrites `351b..351f`, then finalizes through `0x2000:c100`,
      `0x2000:c02a`, and `0x2000:c2f0`
    - `3521` is explicitly cleared at `44f5`, alongside `350c` at `44fa`
  - practical consequence:
    - `42d8..456e` is the second late starbase resolution/report loop
    - it consumes the same kind-`1` canonicalized payload as
      `3fcf..41a0`, but drives a later report family after a deeper
      structural candidate match
- `3521` is now narrowed, though not fully named:
  - read by `0000:cce7..cd39` to select one of several small constant tables
    written to `0x630..0x633`
  - read again at `0000:f812` and passed to `0x3000:44b7` as a late
    report/control selector
  - practical consequence:
    - `3521` behaves like a late starbase report-mode / variant byte, not a
      generic summary payload field
    - the concrete mode map is now narrowed to three later values:
      - `6` -> writes spacing tuple `[10, 20, 30, 40]` to `0x630..0x633`
      - `7` -> writes spacing tuple `[20, 25, 25, 30]`
      - `8` -> writes spacing tuple `[0, 0, 0, 100]`
    - those tuples are later consumed via the downstream `f812` / `f8f2`
      gate, which:
      - passes `3521` and CS:`0x6766` to `0x3000:44b7`
      - only continues the later follow-on path if that helper returns
        nonzero
    - best current interpretation:
      - `3521` is a late report-layout / variant mode byte used to select
        presentation/threshold tables for the follow-on starbase report flow
      - the exact human-facing labels for modes `6`, `7`, and `8` still need
        recovery
- nearby raw strings after `0000:41a1` now show this region owns a wider
  starbase report family, not only the raw `unknown starbase` message:
  - `We have arrived at Starbase ... and are merging with the ... Fleet.`
  - `... and have found the ... Fleet already guarding it.`
  - `However, since a fleet cannot have more than ... ships, we cannot merge`
  - practical consequence:
    - `0000:3fcf..41a0` is the late starbase resolution/report block for
      guarding/merge outcomes more broadly
    - the remaining open part is the exact caller-side meaning of the scratch
      fields feeding those report variants, not whether this is the right late
      starbase path
- a direct raw-import attempt to decode the later CS-local constants
  `0x0d30`, `0x0d53`, etc. as counted strings does not yield printable text
  - practical consequence:
    - those constants should currently be treated as runtime-context-dependent
      report references, not plain raw-import string blobs
    - the useful static milestone is the branch-role split above, not the exact
      text bodies yet
- a focused raw-dump probe on the downstream `3521` consumer now confirms the
  same limitation for `0x3000:44b7` and `CS:6766`
  - in the current `MEMDUMP.BIN` raw import, both ranges are all zero bytes
  - practical consequence:
    - the remaining `3521` semantics cannot be recovered from the current raw
      import alone
    - the next method must be runtime-aware capture around the live consumer,
      not more blind carving of the zeroed `3000:` range
- a runtime-aware write-stop dump on the failing `unknown starbase` case
  tightens that further
  - using `BPINT 21 40`, the debugger now stops at the `ERRORS.TXT` write with:
    - `CS=3374 EIP=1953`
    - `AX=40d0`
    - `BX=0006`
  - `ERRORS.TXT` at that stop contains:
    - `Fleet assigned to an unknown starbase.`
  - but even in that late failing snapshot, the nominal raw-dump offsets for:
    - `3000:44b7`
    - `3000:6766`
    - `0x3521`
    still read as zero under the current linear mapping
  - practical consequence:
    - the remaining `3521` / `44b7` semantics are not just "later in time";
      they are outside what this PSP-owned dump exposes under the current
      raw-import address model
    - the next useful method is a runtime-segment-aware capture around the
      live consumer path, not more raw-dump carving at the old `3000:` offsets
  - probing the write-stop `DS`-relative state also clarifies why:
    - at the same stop, `DS=39ab` and `SS=39ab`
    - `DS:3502`, `DS:3521`, `DS:3525`, `DS:0630`, and `DS:6766` all read back
      as zero
  - practical consequence:
    - the `INT 21h/AH=40` stop is too DOS-owned to expose the program-side late
      scratch directly
    - the next useful breakpoint must be before the DOS write, on the
      program-side report path rather than inside DOS
  - the stack at that write stop also preserves a useful caller-side clue:
    - it contains a plausible far return pair `2895:27ac`
    - that segment matches the earlier live kind-`2` decode captures
      (`2895:6060`, `2895:62e3`)
    - the earlier live/static offset delta remains consistent:
      - `0000:0403 -> 2895:6060`
      - `0000:0686 -> 2895:62e3`
      - both give delta `+0x5c5d`
  - practical consequence:
    - future live breakpoint work on the late starbase path should treat
      segment `2895` and the `+0x5c5d` offset delta as the best current clue,
      but not yet a proven breakpoint recipe
    - a direct probe at the derived late return-site address still did not hit,
      so this remains suggestive rather than confirmed
  - that clue is now confirmed by a direct runtime capture at `2895:27ac`
    - live stop:
      - `CS=2895 EIP=27ac`
      - `DS=3529`
      - `SS=39ab`
      - `AX=0001`
      - `BX=0006`
      - `CX=0065`
    - practical consequence:
      - segment `2895` is now a confirmed program-side late-path context, not
        just a stack hint
      - the write-stop caller clue was real
  - however, that specific return site is already past the late scratch lifetime
    for the `3502` block:
    - `DS:3502` is zero at `2895:27ac`
    - so this is a post-write / post-clear return point, not the pre-write
      report-assembly point we still need
  - `DS:0630` at the same stop is nonzero:
    - `0d 05 05 12 01 00 00 05 01 00 16 07 05 0a 05 10`
    - practical consequence:
      - the later program-side path does preserve nontrivial report/control
        state after the DOS write
      - future runtime work should now search backwards from confirmed
        `2895:27ac`, not from DOS `3374:*`
  - the `2895:*` late-path code is now mapped back into the static live image
    by matching the confirmed runtime bytes into `/tmp/ecmaint-debug/MEMDUMP.BIN`
    - live `2895:2760` -> static `2000:2f70`
    - live `2895:27ac` -> static `2000:2fbc`
    - live `2895:7e20` -> static `2000:8630`
    - live `2895:7e4b` -> static `2000:865b`
  - practical consequence:
    - the remaining runtime-only late path is no longer floating outside the
      static model
    - `2895:27ac` sits inside generic late helper `2000:2db3`
    - caller `2895:7e4b` sits in the later top-level region around
      `2000:8630..`
  - that mapped static caller context is enough to set a stop condition on this
    rabbit hole:
    - `2000:8652` calls `2000:1da6`, `2000:0c06`, `2000:2db3`, and `2000:56be`
      before iterating later summaries
    - `2000:2db3` is generic late report/output plumbing, not the decisive
      compliance predicate
    - the remaining unresolved `3521` / report-variant semantics are therefore
      on the UI/report side of the starbase path, not the accept/reject side
  - strong methodological consequence:
    - the deep RE blocker for Rust compliance is now satisfied
    - preserve the recovered accept/reject structure from
      `3fcf..41a0` and `42d8..456e`
    - do **not** keep drilling on `3521` mode-text semantics unless the task is
      explicit UI/report preservation

Artifacts:
- `artifacts/ghidra/ecmaint-live/unknown-starbase-predicate.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-scratch-refs.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-payload-producers.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-scalar-scan.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-late-ranges.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-resolution-loop.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-variant-strings.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-mode-selector.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-variant-helper.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-mapped-late-regions.txt`
- `artifacts/ecmaint-kind2-debug/unknown-starbase-write/summary.txt`
- `artifacts/ecmaint-kind2-debug/unknown-starbase-write/MEMDUMP.BIN`
- `artifacts/ecmaint-kind2-debug/unknown-starbase-write/stack_at_write.txt`
- `artifacts/ecmaint-kind2-debug/unknown-starbase-return-site/summary.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportUnknownStarbasePredicate.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseScratchRefs.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseStrings.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseResolutionLoop.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseVariantStrings.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseModeSelector.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseVariantHelper.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseMappedLateRegions.java`
- `tools/capture_ecmaint_unknown_starbase_write_dump.py`
- `tools/capture_ecmaint_unknown_starbase_return_site.py`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbasePayloadProducers.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseScalarScan.java`
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseLateRanges.java`

`0x1000:d183` helper contract (first pass):

Artifacts:
- `artifacts/ghidra/ecmaint-live/unknown-starbase-helper.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseHelper.java`

Recovered structure:
- `0x1000:d183` sits inside a larger helper at `1000:d166..d4fe`
- entry behavior:
  - copies a `0x36`-byte source object from the caller into local `BP-0x38`
  - initializes:
    - candidate count at `[BP+0xFBD6] = 0`
    - candidate index list rooted at `[BP+0xFECC]`
- primary scan (`1000:d1a0..d21f`):
  - iterates entries through pointer table `0x1712`
  - filters on entry owner byte `ES:[DI+0x5d] == [BP-0x36]`
  - in mode `1` (`[BP+0x0e] == 1`), also requires:
    - entry byte `0x00 == [BP-0x2d]`
    - entry byte `0x01 == [BP-0x2c]`
  - matching entry indexes are appended into the local list at `FECC`
  - candidate count increments in `[BP+0xFBD6]`
- if more than one candidate matched:
  - the helper computes 3-word ranking tuples for each candidate into the local
    array rooted at `FBD8`
  - then sorts both the ranking tuples and the parallel candidate index list
    (`FECC`) via the `0x3000:488d` comparison helper
- return behavior (`1000:d4bc..d4fe`):
  - if no candidates matched:
    - returns `AL = 0`
  - otherwise:
    - returns `AL = 1`
    - copies two bytes from the selected entry in the `0x1712` table through
      the caller-supplied output pointers at `[BP+0x0a]` and `[BP+0x06]`
      (entry bytes `0x00` and `0x01`)

Current practical interpretation:
- `0x1000:d183` is a candidate locator/selector, not a direct error emitter
- for the later starbase-specific region at `0000:3fcf..41a0`, it returns:
  - success/failure in `AL`
  - two selected bytes from the winning entry
- this makes the later region's compare on summary bytes `+0x01` / `+0x02` /
  `+0x05` more coherent: it is checking the current summary against a
  best-matching located candidate, not scanning blindly
- return/list layout is now narrowed further:
  - candidate count lives at `[BP+0xFBD6]`
  - candidate indexes are appended into a 1-based local list rooted at
    `[BP+0xFECC]`
  - because the helper increments count before storing, the first real
    candidate lands at `[BP+0xFECE]`
  - the sort/swap block normalizes the winning candidate back into that first
    slot
  - the return block then reads the selected entry bytes `0x00` and `0x01`
    from `[BP+0xFECE]`
  - practical consequence:
    - the stable semantic side effect of `0x1000:d183` is the winning
      selected-entry pair
    - the direct register return is only a boolean success gate in `AL`

Artifacts:
- `artifacts/ghidra/ecmaint-live/unknown-starbase-selector-return.txt`

Tool:
- `tools/ghidra_scripts_tmp/ReportUnknownStarbaseSelectorReturn.java`

## Post-Starbase Transition Queue

Current state after closing the Guard Starbase / `unknown starbase` blocker:

- the preserved initialized fixture is now fully covered by the shared
  current-known baseline sync
- for the noisier shipped sample (`original/v1.5`), canonical drift after
  current-known normalization now remains only in:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
- `CONQUEST.DAT`, `SETUP.DAT`, `BASES.DAT`, and `IPBM.DAT` are already at the
  canonical post-maint baseline after normalization

Transition-queue implication:

- the remaining `FLEETS.DAT` drift is not an independent queue
  - after normalization it collapses to offsets:
    - `0x0b/0x0c` current location
    - `0x20/0x21` standing-order target
  - practical consequence:
    - fleet drift is downstream of the sample's planet/homeworld state
    - the next initialized-to-post-maint rule discovery pass should start with
      `PLANETS.DAT`, not another fleet-only dive
- the new semantic transition-details report makes that dependency concrete:
  - current normalized shipped-sample homeworld seed / owned-world state still
    differs from the canonical post-maint topology
  - important caution:
    - coordinate differences alone are not strong evidence of a special
      campaign progression state, because the starmap and empire homeworlds are
      randomized per game
    - treat the remaining coordinate drift as setup variance until a
      non-coordinate payload/ownership rule is proven
  - examples:
    - current record 13 homeworld seed at `(6,12)` vs canonical `(4,13)`
    - current record 16 owned `Dust Bowl` world at `(16,13)` vs canonical
      unowned record 16, while canonical player-1 homeworld seed lives at
      record 15 `(16,13)`
  - the remaining fleet-block drift follows those planet/homeworld coords
    directly:
    - block 2 current `(6,12)` vs canonical `(4,13)`
    - block 3 current `(16,5)` vs canonical `(6,5)`
    - block 4 current `(7,4)` vs canonical `(13,5)`
## Oracle-First Method Shift

The project default is now:

- initialize or materialize a controlled directory
- submit one narrow order family or field mutation
- run `ECMAINT` as the oracle
- diff `.DAT`, `MESSAGES.DAT`, `RESULTS.DAT`, and `ERRORS.TXT`
- promote only repeated deterministic effects into `CoreGameData`

Repo harness:

- `python3 tools/ecmaint_oracle.py prepare <target_dir> [source_dir]`
- `python3 tools/ecmaint_oracle.py run <target_dir>`
- `python3 tools/ecmaint_oracle.py replay-known <fleet-order|planet-build|guard-starbase> <target_dir>`

First replay result:

- `python3 tools/ecmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
  runs cleanly through `ECMAINT` and produces no `ERRORS.TXT`
- but it does **not** land exactly on `fixtures/ecmaint-fleet-post/v1.5`
- residual byte drift against the preserved post fixture:
  - `PLAYER.DAT`: `2`
  - `PLANETS.DAT`: `18`
  - `FLEETS.DAT`: `9`
  - `DATABASE.DAT`: `29`
- practical implication:
  - the current accepted `fleet-order` pre-maint generator is sufficient for
    the known scenario validator, but it is not yet an exact replay of the
    preserved campaign-state transition
  - use these residual diffs as the next black-box rule queue instead of
    assuming the preserved `fleet-order` pre state is fully exact

Further replay results:

- `python3 tools/ecmaint_oracle.py replay-known planet-build /tmp/ecmaint-build-oracle`
  - clean run, no `ERRORS.TXT`
  - residual drift:
    - `PLANETS.DAT`: `6` bytes, all in record `15`
    - `DATABASE.DAT`: `1` byte
  - exact `PLANETS.DAT` record-15 deltas:
    - `+0x09`: `134 -> 0`
    - `+0x0E`: tax `12 -> 0`
    - `+0x24`: build-count slot `3 -> 0`
    - `+0x2E`: build-kind slot `1 -> 0`
    - `+0x38`: developed-value byte `0 -> 3`
    - `+0x4C`: stardock-kind slot `0 -> 1`
  - practical interpretation:
    - this is the cleanest current black-box queue for build completion:
      queue entry consumed, planet economics normalized, completed output
      emitted into stardock

- `python3 tools/ecmaint_oracle.py replay-known guard-starbase /tmp/ecmaint-starbase-oracle`
  - clean run, no `ERRORS.TXT`
  - residual drift:
    - `PLAYER.DAT`: `1` byte at absolute offset `70`
  - practical implication:
    - Guard Starbase is now effectively out of the critical path for
      compliance work

Resolved replay-gap cause:

- comparing the Rust-generated known pre states directly against the preserved
  pre fixtures showed that the gameplay tables already matched:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT` where applicable
- the only shared pre-maint gap across all three known scenarios was:
  - `CONQUEST.DAT`
  - `DATABASE.DAT`
- that shared gap is identical across:
  - `ecmaint-fleet-pre`
  - `ecmaint-build-pre`
  - `ecmaint-starbase-pre`
- `ec-cli scenario-init-replayable [source_dir] <target_dir> <scenario>` now
  overlays that shared pre-maint replay context and produces exact preserved
  pre-maint directories for the known scenarios
- practical implication:
  - the earlier `replay-known` residuals were a scenario-construction issue,
    not missing per-scenario post-maint rules

---

## Economy tick / PLAYER.DAT autopilot investigation

### Session: 2026-03-12

Goal: understand what drives army and battery growth observed on Dust Bowl
in the `original/v1.5` ECMAINT run (`armies: 142→161`, `batteries: 15→16`,
`raw[0x0E]: 4→18`).

#### Key finding: autopilot flag at PLAYER.DAT offset 0x6d

`PLAYER.DAT` player-record offset `0x6d` (byte 109, the last byte of the
110-byte record) is `0x01` for player 1 in `original/v1.5` and `0x00` in
the canonical baseline.

Controlled experiment:

- took `original/v1.5`, cleared `PLAYER.DAT[0x6d] = 0` for player 1
- ran ECMAINT
- result: **no army or battery growth** on Dust Bowl
  - `armies: 142 → 142` (unchanged)
  - `batteries: 15 → 15` (unchanged)
  - `raw[0x0E]: 4 → 3` (decremented by 1, no autopilot spending)
  - factories: `100.0 → 200.0` (factory growth still happens)

With original `PLAYER.DAT[0x6d] = 1` (autopilot on):
- `armies: 142 → 161` (+19)
- `batteries: 15 → 16` (+1)
- `raw[0x0E]: 4 → 18` (+14, autopilot spent production points)

Conclusion: **PLAYER.DAT offset 0x6d is the autopilot flag** (1 = on, 0 = off).
When autopilot is on, ECMAINT acts as the AI player and spends production points
to build planetary defenses (armies and ground batteries). This matches the
player docs: "autopilot mode causes the computer to play your empire for you
(mostly building your planetary defenses)."

#### `raw[0x0E]` behavior isolated

Without autopilot, `raw[0x0E]` simply decrements by 1 per tick (4→3 in this
run). The autopilot path writes a different value because it spends production
points through some accounting that touches this field. The field's true
semantics remain unresolved, but:

- it is NOT the empire-wide tax rate (that lives in PLAYER.DAT[0x51])
- without autopilot it decrements by 1 per tick
- with autopilot it gets updated by however much production was spent
- it does not appear to be stored_goods (raw[0x0A..0x0E] is the 4-byte u32
  stored_goods field; raw[0x0E] is a separate single byte)

#### Factory doubling behavior

In the `ec-econ-tax` experiment (canonical homeworld seed, `factories=100.0`,
`potential=100`, player 1 `tax=65`, no autopilot):

- tick 1: factories `100.0 → 200.0`, `raw[0x0E]: 12 → 7`
- tick 2: factories `200.0 → 200.0`, `raw[0x0E]: 7 → 72`
- tick 3: factories `200.0 → 400.0`, `raw[0x0E]: 72 → 4`
- tick 4: factories `400.0 → 400.0`, `raw[0x0E]: 4 → 37`
- tick 5: factories `400.0 → 400.0`, `raw[0x0E]: 37 → 69`
- tick 6: factories `400.0 → 400.0`, `raw[0x0E]: 69 → 102`
- tick 7: factories `400.0 → 800.0`, `raw[0x0E]: 102 → 3`

Pattern: factories double on roughly every 2nd or 3rd tick, triggered when
`raw[0x0E]` crosses some threshold. The `raw[0x0E]` value resets near 3–4
after each factory doubling. The exact accumulator rule is not yet decoded.
Note: factories grow well past `potential` (100) in this experiment —
`potential` does not cap `current_production`; the game docs say production
grows toward the maximum, implying the Real field tracks something that can
exceed potential during growth phases.

#### Economy tick: no growth on unjoined homeworld seeds

Canonical baseline homeworld seeds (all 4 players, `factories=50.0`,
`potential=100`, `armies=10`, `batteries=4`, player tax=0) show **zero
PLANETS.DAT changes** across any number of ECMAINT ticks. This confirms:

- tax=0 means no production points are generated, so no factory growth
- homeworld seeds without an active human player (player active byte `0x00`)
  are stable under ECMAINT

#### Canonical baseline: player active byte

`PLAYER.DAT` offset `0x00` of each player record:
- `0x01` in original/v1.5 player 1 = active player with a name
- `0x00` in canonical baseline = no player joined (slot inactive)

This matches: the canonical baseline has 4 unjoined homeworld seeds and no
active players, so ECMAINT produces no planet state changes beyond the year
counter.

#### 2026-03-14: restore default joinable `sysop new-game` baseline

The Rust setup path had drifted into using an already-materialized post-join
campaign baseline for default `sysop new-game`, which made live `ECGAME`
onboarding incomplete:

- player slots looked active enough to trigger `game is full` / reserved-slot
  behavior without manual patching
- once join became possible, `ECGAME` still skipped the homeworld naming flow
  because the Rust baseline had already named/owned those worlds

Code change:

- `build_seeded_new_game(...)` now builds a **joinable pre-player baseline**
  again:
  - `PLAYER.DAT[0x00] = 0x00` for all slots
  - homeworld planets are `Not Named Yet` seeds with `tax=12`, `armies=10`,
    `batteries=4`, `ownership_status=2`, and owner-slot tags retained
  - player records now carry fixture-shaped pre-join fleet-range/homeworld-index
    fields instead of active-empire state
  - the `ECUTIL` init-style `CONQUEST.DAT` control header is now seeded instead
    of a mostly-zero synthetic one
- the previous post-join active-campaign baseline was preserved as a separate
  Rust path via `build_seeded_initialized_game(...)` and
  `setup_mode="builder-compatible"`

Coverage added:

- `ec-data` tests now lock in the joinable baseline semantics and keep the
  old initialized baseline available explicitly
- `ec-cli` `sysop` tests now assert that default `new-game` writes inactive
  player slots with `Not Named Yet` homeworlds
- maint/oracle automation was switched to the explicit builder-compatible setup
  config so engine sweeps still start from an active campaign baseline

Status:

- `cargo test -q` is green after the split
- still need one live `ECGAME` confirmation that the default generated
  directory now triggers the full first-join naming flow without manual patching

#### CONFIRMED: PLANETS.DAT planet record tail layout (97-byte records)

Cross-confirmed from multiple fixtures (`ecmaint-post`, `ecmaint-fleet-post`,
`ecmaint-bombard-heavy-pre`, bombard field-isolation fixtures, and the
`RESULTS.DAT` text report "15 ground batteries and 142 armies"):

- **`raw[0x58]` = army count** (e.g. 10 for homeworld seed, 142 for Dust Bowl,
  1 after fleet lands on unowned world, 0 for unowned/empty)
- **`raw[0x5A]` = ground batteries** (e.g. 4 for homeworld seed, 15 for Dust
  Bowl, 0 for unowned/empty)
- `raw[0x5B]` = 0 in all known fixtures (unnamed, leave raw)

Previously `raw[0x58]` was speculatively labeled `developed_value_raw()` and
`raw[0x5A]` was labeled `likely_army_count_raw()`. Both labels were wrong.
The accessor rename (`army_count_raw` → `raw[0x58]`, `ground_batteries_raw` →
`raw[0x5A]`, `developed_value_raw` removed) was completed and all tests are
green.

Note: the bombard scenario fixture names `army0`/`army1` are misleading — they
vary the **batteries** field (`0x5A`), not the army field (`0x58`).

---

## Rogue AI / autopilot planet economics — Session 2026-03-13

### How rogue empires are created

Two confirmed paths (from ECUTIL F3 and WHATSNEW docs):

1. **Sysop manually** via `ECUTIL F3 Change Empire Ownership → R (Make empire ROGUE)`.
   Sets `PLAYER.DAT[0x00] = 0xff` and writes `"Rogues"` into the empire label field.
2. **Bulk close** via `ECMAINT /C` command-line flag, which forces all currently
   unclaimed (`mode=0x00`) empires to become rogues. Used by sysops to "close"
   enrollment at the start of a game so latecomers become AI opponents.

`mode=0x00` (civil disorder / unclaimed slot) does **not** automatically become
rogue each turn. It only becomes rogue when the sysop explicitly acts.

### AI trigger rule — confirmed by black-box oracle

AI planet economics run for a player's planets when either:

- `PLAYER.DAT[0x00] == 0xff` (rogue mode) — regardless of autopilot flag, OR
- `PLAYER.DAT[0x00] == 0x01` (active human) AND `PLAYER.DAT[0x6D] == 0x01`
  (autopilot flag on)

`mode=0x00` (civil disorder) planets are **never touched** by the AI, even when the
player owns homeworld-type planets. Confirmed by oracle: econ fixture players 2 and
3 (mode=0x00) own planets 4 and 5 — both are unchanged across all oracle runs.

### Planet-side trigger — flags field must be exactly 0x87

The AI only modifies a planet if `PLANET.raw[0x03] == 0x87` (homeworld flag byte).
Other flag values (0x81 new-colony, 0x00 unowned, etc.) do not trigger the AI or
produce garbage field writes.

Sweep results for `PLANET.raw[0x03]` with a rogue owner:

| flags | result |
|-------|--------|
| `0x87` | clean AI: factories doubled, armies +17, `raw[0x0E]` updated |
| `0x81` | no change |
| `0x00` | no change |
| `0x84`, `0x86`, etc. | partial/garbage writes — AI stumbles into wrong offsets |

`0x87` is the exact homeworld-type byte assigned during game init for all player
homeworlds. Only one per empire in the base game setup.

### Factories field: Borland Pascal Real48, exponent +1 = doubling

`PLANET.raw[0x04..0x09]` is a 6-byte Borland Pascal Real48:
- byte `[5]` = exponent (stored at raw[0x09] in the planet record)
- byte `[4]` = high mantissa byte (stored at raw[0x08])
- bytes `[0..3]` = low mantissa bytes (stored at raw[0x04..0x07], all zero for
  homeworld seeds)

For the canonical homeworld seed:
- initial: `raw[0x08..0x09] = [0x48, 0x86]` → exponent=0x86=134 → `2^(134-129) × 1.5625 = 50.0`
- after one AI tick: `raw[0x08..0x09] = [0x48, 0x87]` → exponent=0x87=135 → `2^(135-129) × 1.5625 = 100.0`

The AI increments the exponent byte by 1, which doubles the Real48 value. Starting
from `factories=50.0`, one AI tick brings it to `factories=100.0` (= pot_prod).

This is **deterministic**: confirmed across all oracle runs. The increment is always
exactly +1 to raw[0x09] (the exponent byte of the Real48).

**Important:** the increment only happens when `raw[0x09] == 0x86` on entry. Once at
0x87 (factories=100.0 for pot_prod=100 planets), subsequent AI ticks do not further
increment the exponent — the factories field stops changing. Tested: pre=0x87 →
post=0x87 (no change). Pre=0x88 → post=0x87 (clamped back). The AI is converging
factories toward some target, not blindly incrementing.

### Army growth: +`pot_prod / 6` (rounded)

When the AI runs on a pot_prod=100 homeworld: armies increase by 17 per tick
(`100 / 6 = 16.67`, rounded to 17). Confirmed by black-box oracle.

This matches the `original/v1.5` run on Dust Bowl where armies grew +19 with
pot_prod=100 and the planet already had accumulated armies=142 at that point
(different factories/raw[0x0E] state → slightly different spending mix).

The formula `armies += round(pot_prod / 6)` is the current best model.

### `raw[0x0E]`: production accumulator, not tax rate

`PLANET.raw[0x0E]` is a per-planet production accumulator used by the AI spending
logic. It is **not** the empire tax rate (that is `PLAYER.DAT[0x51]`).

Observed behavior:
- Fresh homeworld seed: `raw[0x0E] = 12`
- Without autopilot/rogue AI: decrements by 1 per tick (12→11→...→4→3→...)
- After one AI tick from the fresh-seed state (raw[0x0E]=12): becomes 4
- After AI runs on an already-AI-run planet (raw[0x0E]=4): varies (2–4 range,
  partly non-deterministic due to DOSBox clock)

The exact accumulator rule is not yet decoded. It appears to track remaining
production points available for spending this tick, and resets to a small value
after the AI has spent them. The non-determinism in army count (occasionally armies
don't grow) is believed to come from this field's interaction with a timer or RNG
in ECMAINT, not from a fundamentally stochastic rule.

### DATABASE.DAT record 14 raw[0x1e] and raw[0x23]

When the AI runs on planet 14, two DATABASE.DAT bytes also change:
- `rec[14].raw[0x1e]`: `0x23 → 0x40 + owner_slot` (= 0x41 for slot=1)
  This is the fog-of-war ownership marker for the player's own planet.
- `rec[14].raw[0x23]`: mirrors the post-maint army count on planet 14.

Both are a consequence of the DATABASE.DAT fog-of-war update rule already
implemented in Rust — the planet's intel record gets refreshed after the AI
changes the army count. These are not new rules; they follow from the existing
`regenerate_database_dat()` logic once the army count is updated correctly.

### What is NOT yet decoded

- The exact formula for `raw[0x0E]` update (production accumulator arithmetic)
- Battery growth rule under autopilot (in `original/v1.5` batteries grew +1 per
  tick; not tested in the econ scenario where batteries stayed at 4)
- Whether the army formula varies with batteries count or other planet fields
- Multi-planet AI behavior when a rogue/autopilot player owns more than one
  homeworld-type planet (tested briefly: the same per-planet rule applies
  independently to each `flags==0x87` planet)

### Fleet order table: manuals over preserved invade pre-fixture

`ECPLAYER.DOC` and `ECQSTART.DOC` both give the fleet order table explicitly:

- `0x06 = BombardWorld`
- `0x07 = InvadeWorld`
- `0x08 = BlitzWorld`
- `0x09 = ViewWorld`
- `0x0a = ScoutSector`
- `0x0b = ScoutSolarSystem`
- `0x0c = ColonizeWorld`
- `0x0d = JoinAnotherFleet`
- `0x0e = RendezvousSector`
- `0x0f = Salvage`

The preserved `fixtures/ecmaint-invade-pre/v1.5/FLEETS.DAT` still carries
`raw[0x1f] = 0x0a` on the invade fleet. Rust now treats that byte as a
historical fixture quirk rather than semantic truth:

- the `Order` enum follows the manuals (`InvadeWorld = 0x07`)
- invade scenario generators now emit `0x07`
- invade scenario tests no longer require exact `FLEETS.DAT` parity for that
  conflicting byte

This is the current repository example of the documented policy:

- manuals are authoritative for gameplay semantics
- preserved fixtures/oracle remain authoritative for compatibility and file
  structure, unless a fixture byte directly conflicts with explicit manual
  semantics and does not buy compatibility

## `ECGAME` surrender UI check — Session 2026-03-13

- Goal: determine whether `ECGAME` exposes an explicit surrender/resign action
  or whether surrender exists only as a campaign/rules concept.
- Sources checked:
  - `original/v1.5/ECPLAYER.DOC`
  - `original/v1.5/ECQSTART.DOC`
  - live `ECGAME` General Command Center capture
- Manual findings:
  - both manuals describe winning by having the other players surrender and
    acknowledge you as Emperor
  - `ECPLAYER.DOC` documents the General Command menu options as:
    - autopilot
    - summaries/reports
    - declare neutral/enemy
    - message command center
  - no surrender/resign command is documented there
- Live UI finding:
  - the General Command Center screenshot confirms no visible surrender or
    resign option
- Conclusion:
  - surrender exists as a campaign-state concept in the rules, not as a
    currently observed `ECGAME` command
  - Rust should not invent a surrender menu flow without stronger evidence
  - the real missing implementation area is campaign-end state:
    - annihilation
    - fleet defection after loss of all planets
    - emperor recognition / submission of remaining empires

## `ECGAME` player mail / `MESSAGES.DAT` probe — Session 2026-03-14

- Goal: determine whether player-to-player communication uses `MESSAGES.DAT`,
  `RESULTS.DAT`, or another file, and whether mail is visible immediately or
  only after maintenance.
- Controlled findings from live `ECGAME` probing:
  - self-sent and cross-player mail both persist in `MESSAGES.DAT`
  - `RESULTS.DAT` remained empty during these communication-only probes
  - a single short two-player message produced an `85`-byte `MESSAGES.DAT`
    sample containing the literal body text
  - a longer self-sent message sample produced a `680`-byte `MESSAGES.DAT`
    file, divisible into `17` apparent `40`-byte records
  - practical consequence:
    - classic player mail format is not the same as the current Rust
      maintenance `84`-byte report chunks
    - the larger sample strongly suggests a compact mail-specific header/body
      structure, with repeated short body records
- Delivery timing finding:
  - the recipient could not read the newly sent player-to-player message until
    the next maintenance turn
  - practical consequence:
    - classic mail is maintenance-gated rather than immediate inbox delivery
- Rust maintenance interaction finding:
  - before the fix, `rust-maint` rewrote `MESSAGES.DAT` to empty on a directory
    containing pending classic player mail if Rust had no maintenance messages
    to emit
  - after the fix, `rust-maint` preserves a non-empty existing
    `MESSAGES.DAT` unchanged when the Rust-generated routed message stream is
    empty
  - practical consequence:
    - current Rust can coexist with queued classic player mail without
      destroying it
    - exact classic mail decode / merge semantics remain open work
- Review / delete state finding:
  - after recipient login, `ECGAME` presents new mail during login-time status
    flow before the General Command Center review path
  - the later `R>eview messages/results` path is therefore a review/archive
    surface for undeleted items, not the primary first-delivery moment
  - controlled unread/read/delete probes showed:
    - `MESSAGES.DAT` stayed byte-for-byte unchanged across unread, read-only,
      and read+delete states
    - `RESULTS.DAT` stayed empty
    - recipient `PLAYER.DAT` byte/word changes were:
      - unread baseline:
        - `0x30 = 0x01`
        - `0x34 = 0x01`
        - `0x4e..0x4f = 0x0bb8`
      - read but not deleted:
        - `0x30 = 0x01`
        - `0x34 = 0x01`
        - `0x4e..0x4f = 0x0bba`
      - read and deleted:
        - `0x30 = 0x00`
        - `0x34 = 0x00`
        - `0x4e..0x4f = 0x0bba`
  - practical consequence:
    - mail payload and mailbox state are separate concerns
    - `PLAYER.DAT[0x4e..0x4f]` is a strong candidate for new/unseen message
      presentation state
    - `PLAYER.DAT[0x30]` and `PLAYER.DAT[0x34]` are strong candidates for
      undeleted/reviewable-mail flags

## Conservative campaign-end rules promoted into Rust — Session 2026-03-13

- Manuals used:
  - `ECPLAYER.DOC` losing rule: fleets start to defect after loss of all
    planets unless another planet can be recaptured
  - `ECPLAYER.DOC` / `ECQSTART.DOC` victory rule: one empire is recognized as
    emperor after all other serious contenders are gone
- Existing RE constraint preserved:
  - `mode=0x00` civil-disorder slots do **not** automatically become rogue
  - therefore Rust should not model defections as a hidden auto-rogue
    conversion
- Promoted conservative Rust rules:
  1. active empires with no planets and no recovery path transition to
     `In Civil Disorder`
  2. once already in civil disorder and still planetless, an empire loses one
     surviving fleet to defection per maintenance turn
  3. if exactly one serious contender remains and that empire is still
     `Stable`, Rust recognizes it as emperor
- Notes:
  - this is intentionally deterministic and compatibility-safe
  - it is not claimed as a byte-exact reproduction of original hidden
    campaign-end internals
  - if stronger classic evidence appears later, fleet-defection cadence and
    emperor-recognition detail can be refined without changing the overall
    architecture
## Session 2026-03-14 - setup economy seeds and mixed-tax probe

- Rewrote builder-generated starting economy payloads to encode the intended
  semantics directly instead of relying on player-facing correction:
  - generated homeworlds now seed with present production `100`
  - generated starting empire tax is `50%`
  - initialized builder baselines now also start with `10` armies and `4`
    ground batteries on homeworlds
- Kept the older `current_known_*` homeworld seed validators intact because
  they are still anchored to preserved historical fixture payloads
  (`12%` / `50-production`) and should not be silently reinterpreted as the
  new canonical builder target.
- Ran `tools/economy_tax_probe.py` against mixed-production directories at
  `25 / 50 / 65 / 80%` empire-wide tax:
  - the original `ECMAINT` oracle still produced zero diffs on those mutated
    directories, so it remains unusable as a practical growth oracle for this
    path
  - the canonical Rust rule behaved as intended:
    - lower tax -> faster production growth
    - higher tax -> more immediate stored production points
- Important raw-field finding from the Rust probe:
  - `PLANETS.DAT raw[0x0E]` is not a stable "planet tax rate" field after
    maintenance
  - on the homeworld it was overwritten to `4` by the existing
    autopilot/rogue-AI maintenance path
  - treat `planet_tax_rate_raw()` as a legacy/raw label for now, not a settled
    player-facing semantic field
  - Rust code has now been renamed toward `economy_marker_raw()` to make that
    uncertainty explicit
- 2026-03-14: updated the canonical Rust economy rule to match the manuals'
  stronger tax warning more closely. Taxes above `65%` now apply a direct
  present-production penalty during maintenance; friendly starbase worlds
  tolerate up to `70%` before that penalty begins. This keeps the already
  implemented growth-slowdown rule, but adds the manual-backed "production may
  suffer" effect that was previously missing.

## Session 2026-03-16 - fleet oracle verification against classic ECMAINT

Goal: verify the remaining manual-adjacent fleet assumptions against the
original `ECMAINT.EXE`, and record reproducible classic defects as known
`v1.51` bugs instead of silently treating them as intended rules.

### Confirmed: `Seek Home` dynamically retargets

Controlled probe:

- source baseline: `fixtures/ecmaint-post/v1.5`
- player 1 was given two owned worlds:
  - kept a friendly world at `(4,13)`
  - lost control of the nearer target world at `(16,13)` before maintenance
- fleet 1 was moved to `(12,13)` with:
  - order `2` (`Seek Home`)
  - speed `3`
  - stale target `(16,13)`

Observed original result after one maint turn:

- fleet 1 moved to `(10,13)`
- order remained `Seek Home`
- target was rewritten from `(16,13)` to `(4,13)`

Practical conclusion:

- classic `ECMAINT` re-evaluates `Seek Home` semantically rather than blindly
  chasing a stale coordinate snapshot

### Confirmed: `Guard a Starbase` follows a moved base

Controlled probe:

- source baseline: `fixtures/ecmaint-starbase-pre/v1.5`
- moved the single base from `(16,13)` to `(12,13)` in `BASES.DAT`
- left fleet 1 on `Guard Starbase` with the original base linkage

Observed original result after one maint turn:

- fleet 1 current position became `(12,13)`
- fleet 1 target became `(12,13)`
- guard order remained active

Practical conclusion:

- classic `ECMAINT` refreshes guard-starbase pursuit from live base state

### Confirmed: invalid guard-starbase linkage aborts with classic error text

Controlled probe:

- source baseline: `fixtures/ecmaint-starbase-pre/v1.5`
- kept `BASES.DAT` present but zeroed the only base's active/summary fields and
  owner byte

Observed original result after one maint turn:

- fleet 1 was reset to order `0` (`Hold Position`)
- `ERRORS.TXT` was emitted with:
  - `Fleet assigned to an unknown starbase.`

Practical conclusion:

- classic has a specific integrity/error path for unresolved guard-starbase
  linkage

### Confirmed: patrol/contact reports include actionable hostile composition

Controlled probe:

- source baseline: direct replay of `fixtures/ecmaint-fleet-battle-pre/v1.5`

Observed original `RESULTS.DAT` text:

- patrol fleet first reports sensor contact
- then reports identified hostile fleet ownership/numbering
- then reports ship composition in coarse buckets:
  - `large`
  - `medium`
  - `small`
- when no engagement occurs, classic explicitly says:
  - `Ignoring alien fleet...`

Representative recovered text fragments:

- `Patrol mission report: Sensor contact shows an alien fleet ...`
- `We have located and identified the alien fleet ...`
- `Their fleet contains 51 large, 50 medium, and 50 small vessel(s) ...`
- `Ignoring alien fleet...`

Practical conclusion:

- patrol is definitely an ongoing contact/report posture
- classic reports actionable hostile composition, even when declining battle

### Confirmed: battle-loss reports include observed enemy composition and losses inflicted

Controlled probe:

- source baseline: `fixtures/ecmaint-fleet-battle-pre/v1.5`
- moved the player-owned fleet into a hostile empire-3 world sector
- weakened the player fleet and strengthened the empire-3 defender to force a
  losing contact

Observed original `RESULTS.DAT` text:

- classic emitted a Fleet Command Center loss-of-contact report
- the report included:
  - our fleet composition
  - initial observed enemy composition
  - enemy losses inflicted before destruction

Representative recovered text fragments:

- `We lost all contact with the 1st Fleet shortly after it was attacked ...`
- `Records show the 1st Fleet was composed of 20 battleships.`
- `the alien force initially contained 100 battleships.`
- `The flight recorder recorded alien ship casualties of 12 battleships.`

Practical conclusion:

- classic encounter reporting includes actionable hostile force intel
- classic also reports enemy losses inflicted, not just friendly losses

### Confirmed: salvage failure at non-owned targets aborts and seeks home

Controlled probe:

- source baseline: `fixtures/ecmaint-fleet-battle-pre/v1.5`
- rewrote player 2 fleet 5 to order `15` (`Salvage`) against:
  - owned world `(4,13)`
  - foreign world `(16,13)`
  - empty sector `(10,10)`

Observed original behavior:

- foreign-world probe produced a `RESULTS.DAT` salvage report saying:
  - `Since we no longer own the world in System(16,13), we are aborting our salvage mission.`
  - `we are going to seek to safety of a friendly controlled world.`
- empty-sector probe produced the same style of report, but with
  `System(10,10)`

Practical conclusion:

- classic salvage does not proceed at foreign targets
- the failure path explicitly aborts the mission and seeks a friendly refuge
- this strongly supports the owned-planet-only salvage interpretation

### Known v1.51 bug: empty-sector salvage uses wrong failure text

Repro:

- source baseline: `fixtures/ecmaint-fleet-battle-pre/v1.5`
- set player 2 fleet 5 to `Salvage` target `(10,10)`, where no planet exists

Observed original `RESULTS.DAT` text:

- `Salvage mission report: Since we no longer own the world in System(10,10), we are aborting our salvage mission.`

Why this looks buggy:

- there is no world at `(10,10)`
- classic reuses the "no longer own the world" failure text instead of a
  "no world found" or equivalent empty-target message

Practical consequence:

- treat this as a known `v1.51` reporting bug, not as intended salvage
  semantics

### `Join another fleet` hot pursuit is now confirmed from a player-authored classic probe

The earlier raw-crafted join probes were too brittle to settle the rule, so the
next escalation used live `ECGAME` order entry through the local BBS door and
then original `ECMAINT` on the same campaign state.

Working classic setup:

- live mutable directory: `original/v1.5`
- active joined empire:
  - player slot 2
  - handle summary: `SYSOP`
  - empire name: `foo`
- player-authored orders in classic `ECGAME`:
  - visible fleet `1`: `Move Fleet` to `(11,12)` at speed `3`
  - visible fleet `2`: `Join Another Fleet`, host fleet `1`, speed `3`

Classic pre-maint encoding after order entry:

- moving host fleet record:
  - structural fleet id `2`
  - local slot `1`
  - current location `(8,12)`
  - order `1` (`Move Fleet`)
  - target `(11,12)`
- joining fleet record:
  - structural fleet id `3`
  - local slot `2`
  - current location `(6,12)`
  - order `13` (`Join another fleet`)
  - target `(8,12)` = host fleet's current live location at order-entry time
  - mission aux bytes `00 01`
    - `aux1 = 1` is the player-facing host fleet number entered in `ECGAME`

Classic maintenance result after one `ECMAINT` turn:

- host fleet reached `(11,12)` and changed to `Hold position`
- joining fleet moved from `(6,12)` to `(8,12)`
- joining fleet target was rewritten from `(8,12)` to `(11,12)`

Classic maintenance result after the following `ECMAINT` turn:

- the join completed at `(11,12)`
- the host fleet absorbed the joining fleet
- host composition increased from:
  - `SC=70 CA=1 ET=1`
  - to `SC=70 CA=2 ET=2`
- the separate joining fleet record disappeared from the active fleet chain

Conclusion:

- classic `Join another fleet` is definitively a live semantic-target mission
- `ECGAME` stores:
  - host fleet number in mission aux
  - the host's current coordinates as the initial target snapshot
- `ECMAINT` then refreshes the joiner's target to the host's new live location
  on later turns
- the joiner continues pursuit until merge or later mission failure

Reporting note:

- this successful pursuit/merge path did **not** emit new `MESSAGES.DAT`
  payload in the sampled live oracle run

### Follow-up: direct raw join probes stayed too brittle to settle the rule

Additional controlled probes after the first ambiguous join test:

- tried empire-2 join setups on both:
  - `fixtures/ecmaint-post/v1.5`
  - `fixtures/ecmaint-fleet-battle-pre/v1.5`
- varied `FLEETS.DAT[0x22]` between:
  - local-slot-style host value
  - structural-fleet-id-style host value

Observed behavior:

- on `ecmaint-post`, the joiner often never moved at all, so the probe stayed
  underconstrained
- on `ecmaint-fleet-battle-pre`, classic normalized the involved fleets back to
  ordinary homeworld `Guard/Blockade` posture on the first turn before the join
  pursuit question became meaningful

Practical conclusion:

- direct `.DAT` surgery is still not giving a trustworthy join oracle
- the next serious join probe should be player-authored through live
  `ECGAME`/door flow instead of more raw file mutation

### Follow-up: no clean classic surviving-ROE-withdrawal report found yet

Additional fleet-battle sweeps tried to force a surviving withdrawal outcome by
varying fleet strengths and compositions around same-sector contact.

Observed classic outcomes so far:

- no-engagement/contact-only reports:
  - `Ignoring alien fleet...`
- destruction reports:
  - Fleet Command Center `lost all contact ...`
- mission-abort reports after fleets lose all warships:
  - `Since we have lost all of our warships in combat, we must abort our mission ...`

What did **not** appear yet in classic oracle outputs:

- a surviving encounter report explicitly saying our fleet withdrew under ROE

Practical conclusion:

- the user-memory behavior is still plausible, but this oracle pass has not yet
  reproduced it from preserved or direct `ECMAINT` battle setups
- keep the question open and avoid promoting classic wording/timing claims until
  a live or preserved classic case is captured

### Surviving ROE-style fleet withdrawal is now confirmed from a live classic probe

The earlier sweeps failed to surface a surviving withdrawal report, but a
player-authored live `ECGAME` + `ECMAINT` probe in the mutable
`original/v1.5` campaign did.

Working setup:

- active joined empire:
  - player slot 2
  - handle summary: `SYSOP`
  - empire name: `foo`
- player-authored changes in classic `ECGAME`:
  - declared `Empire Of Dust` (Empire #1) an `ENEMY`
  - set local fleet `1` ROE to `4`
  - moved that fleet into `(16,13)` and later ordered it to
    `Bombard a World` at `(16,13)`

Classic maintenance result:

- defender side:
  - Starbase 1 at `(16,13)` reported sighting the alien fleet and alerting
    nearby forces
  - the defending guard-starbase fleet intercepted and fought
- attacker side:
  - the bombardment fleet survived but scattered
  - mission changed from `Bombard` to `Seek Home`
  - destination changed to friendly planet `fooville` in system `(6,12)`
  - surviving fleet state after maint:
    - still alive
    - location `(14,13)`
    - composition reduced from `2 cruisers + 2 ETAC` to `2 ETAC`

Classic player-facing bombardment retreat report text:

- `Bombardment mission report: We were attacked by the 1st Fleet of`
- `"Empire Of Dust", (Empire #1) in System(16,13).`
- `Our force contained 2 cruisers and 2 ETAC ships.`
- `Alien force contained 9 battleships, 12 cruisers, 12 destroyers and 2 ETAC ships.`
- `To avoid the total destruction of our fleet, we were forced to scatter.`
- `However, we were able to destroy 1 destroyer.`
- `We lost 2 cruisers.`
- `After reassembling our fleet, we've decided to abort our mission of bombarding the world in System(16,13) and to seek the safety of a friendly controlled solar system at maximum speed.`
- `Until ordered otherwise, we are going to avoid contact with alien crafts.`
- `Our new destination is planet "fooville", located in System(6,12).`

Practical conclusions:

- surviving retreat/withdrawal after combat is definitively real in classic
- the player receives a detailed report when that happens
- the report includes:
  - enemy composition seen
  - own composition seen
  - enemy losses inflicted
  - own losses
  - explicit mission abort
  - retreat destination
  - avoidance/withdrawal wording
- the post-combat retreat path semantically becomes `Seek Home`

### Follow-up: `ecmaint-fleet-battle-pre` is not a valid returning-player `ECGAME` fixture

Manual live probe through the local BBS / `ECGAME` path:

- launched `ECGAME` against the preserved maint/combat fixture
  `fixtures/ecmaint-fleet-battle-pre/v1.5`
- caller identity was the local BBS test user
- classic did **not** treat player slot 2 (`FOO` / empire `foo`) as an existing
  joined player
- instead it entered the `FIRST TIME COMMAND` flow

Observed first-time `L` listing:

- only one currently player-owned empire was shown:
  - `Empire Of Dust`
  - `ID 1`
  - `ALIVE`
- footer text said:
  - space reserved for `4` empires
  - currently `1 of the 4` empires is owned by a player
  - `You can join this game if you wish by picking the "J" option.`

Observed `J` behavior:

- classic immediately prompted for a new empire name
- it did **not** offer to attach to preserved slot 2 / empire `foo`

Practical conclusion:

- the raw preserved state in `PLAYER.DAT` is not sufficient for classic
  `ECGAME` to recognize a slot as an existing returning-player login
- there is at least one additional join-state/client gate beyond:
  - `assigned_player_flag`
  - assigned player handle
  - empire name text
- therefore `fixtures/ecmaint-fleet-battle-pre/v1.5` should currently be
  treated as:
  - a valid maint/combat oracle fixture
  - **not** a valid returning-player `ECGAME` fixture

Implication for Rust:

- do not let the Rust client decide "joined returning player" from raw
  `PLAYER.DAT` assigned-player fields alone
- preserve the distinction between:
  - joinable/classic-first-time `ECGAME` starts
  - maint-valid but not login-valid preserved battle fixtures

Next oracle step for `Join another fleet`:

- stop trying to drive that question from `ecmaint-fleet-battle-pre` through
  the login UI
- instead use a truly joinable `ECGAME` baseline, perform a real player join,
  then issue the join-fleet order through classic menus before running
  `ECMAINT`

### `ECGAME` first-time `List Empires` screen is a styled report, not a plain dump

Live `ECGAME` first-time-menu probe from the local BBS door captured the
current `L` / "List Empires" presentation.

Useful player-facing details to preserve for the Rust client later:

- inline command prompt format:
  - `FIRST TIME COMMAND <-H Q L J A V-> l`
- large 4-line ASCII-art report title above the empire table
  - current captured glyph block:
    - `█▀▀ ▀▀█▀▀ █▀▀█ █▀▀█ █▀▀▄ █▀▀█ ▀▀█▀▀ █▀▀       ▀▀█ █▀█ ▀▀█ ▀▀█`
    - `█▄▄   █   █  █ █  █ █  █ █  █   █   █▄  ▄     ▄▄█ █ █ ▄▄█ ▄▄█`
    - `  █   █   █▀▀█ █▀█▀ █  █ █▀▀█   █   █           █ █ █ █   █`
    - `▄▄█   █   █  █ █ █▄ █▄▄█ █  █   █   █▄▄ ▀     ▄▄█ █▄█ █▄▄ █▄▄`
  - this is not just a heading string; it is a stylized title treatment that
    should be preserved or intentionally reinterpreted in the Rust client
- note line immediately under the title:
  - `NOTE: Previous Year's Information is Parenthesized.`
- framed table presentation with columns:
  - `Empire Name`
  - `ID`
  - `Planets Owned`
  - `Current Production`
  - `Status`
- footer summary text under the table:
  - total reserved empires in the game
  - current count of player-owned empires
  - explicit instruction that `J` can be used to join
- closing prompt:
  - `(Press Return)`

Practical implication:

- for the Rust client, the first-time empire list should be treated as a
  report-style screen with classic framing/copy and title art, not as a bare
  data table
- this is a UI-reference finding, not a new gameplay-rule claim

### Local BBS door wrapper writes live player joins into `original/v1.5`

Follow-up inspection after a successful manual join through the local BBS door:

- the join did **not** persist into the temporary local test directory
  `/tmp/ecgame-oracle-joinable`
- the active BBS wrapper was still mounting the repo's shipped sample
  directory `original/v1.5`
- inspecting `original/v1.5` after the join showed the new player state there

Observed persisted state in `original/v1.5`:

- player slot 2 is now active and stable:
  - handle summary: `SYSOP`
  - empire name: `foo`
  - tax: `50`
  - stored production: `3022`
- `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `DATABASE.DAT` all changed on
  disk during the live door session

Important wrapper/client implication:

- the local Enigma BBS door harness is not currently a neutral oracle against a
  caller-selected temp game directory
- it is mutating the mounted directory configured by `tools/bbs/run_ec_dos.sh`,
  which currently points at `original/v1.5`

Important identity implication:

- the persisted joined handle became `SYSOP`, not the external BBS username
  `mag`
- this matches the current wrapper/dropfile path more than the actual Enigma
  caller identity and means the local BBS oracle is currently using a
  simplified/fixed caller identity for classic joins

Practical consequence:

- for future live BBS oracle probes, either:
  - repoint the wrapper at a disposable copied game directory first, or
  - treat `original/v1.5` as mutable scratch state and snapshot it before/after
- do not assume the BBS oracle is exercising per-caller identity faithfully
  until the wrapper/dropfile handoff is tightened

### Returning-player `ECGAME` recognition is gated by caller-handle match, then a pre-loaded first-login flow

Follow-up live/local probe after the failing `FOO` fixture:

- copied `fixtures/ecmaint-fleet-battle-pre/v1.5` to a temp directory
- changed only player slot 2's persisted handle from `FOO` to `SYSOP`
- left the rest of the active campaign state intact
- launched local `ECGAME` with the normal generated `CHAIN.TXT`
  - current helper default alias is `Sysop`

Observed result:

- classic no longer entered the `FIRST TIME MENU`
- instead it treated the matched slot as a returning player and ran the
  post-intro onboarding flow

This is materially stronger evidence that the classic client's first-time vs
returning-player branch depends on caller identity matching the persisted
player handle, not just on active `PLAYER.DAT` ownership bytes.

Observed flow for a matched pre-loaded player:

1. intro pages run first
2. then classic prints:
   - `, you are a pre-loaded player and this is your first time on.`
   - `Your empire has been pre-named as "foo".`
   - `Would you like to rename your empire? (This is your only chance.) Y/[N] ->`
3. after that prompt, classic shows the usual identity/year/minutes/autopilot
   status screen:
   - `, you are "foo", (Empire #2)`
   - `The year is: 3010 A.D.`
   - `Last year on: NEVER`
   - `You have 57 minutes left to play.`
   - `Autopilot is off.`
4. then classic checks for pending reports and messages
   - in this probe:
     - `, you have no reports pending.`
     - `, you have no messages pending.`
5. then classic still asks the player to name the preloaded homeworld:
   - `You have a world in the solar system at X=4, Y=13. Its current production`
   - `level is 100 out of a possible 100 points, (100% efficiency).`
   - `Name this world (20 characters or less) ->`
6. after homeworld naming confirmation, classic enters `MAIN MENU`

Practical conclusions:

- `ECGAME` login recognition has at least two relevant client-side gates:
  - caller/dropfile identity must match the persisted player handle strongly
    enough to escape the first-time menu
  - after a successful match, classic can still route into a distinct
    `pre-loaded player and this is your first time on` onboarding path before
    normal menu play
- raw active player state in `PLAYER.DAT` is not enough by itself to decide
  whether Rust should show:
  - first-time menu
  - pre-loaded first-login onboarding
  - normal joined-player login flow

Implication for Rust client work:

- treat caller identity matching as part of login-state resolution
- model at least three startup branches:
  - no matching joined player -> first-time menu
  - matching pre-loaded player first login -> intro -> one-time empire rename
    prompt -> reports/messages -> homeworld naming -> main menu
  - established joined player -> reports/messages review and normal main menu

### Owned-world fleet salvage succeeds in classic and is reported as an estimated production yield

Follow-up live probe in the mutable `original/v1.5` campaign:

- starting state:
  - active player slot 2 / empire `foo`
  - surviving post-bombard fleet was already on a classic `Seek Home` path
    toward `fooville` in system `(6,12)`
- mutated that live fleet to a `Salvage` mission targeting owned world
  `(6,12)`
- advanced classic `ECMAINT` turn-by-turn until arrival

Observed classic behavior:

- classic accepted the salvage order on an owned world
- maintenance moved the fleet toward `(6,12)` over multiple turns exactly like
  another movement-based mission
- on arrival, the fleet record disappeared from the active fleet chain
- `RESULTS.DAT` gained a new player-visible report:
  - `Salvage mission report: We have arrived at planet "fooville" in`
  - `System(6,12) and have begun salvaging our fleet.`
  - `We estimate that our fleet will yield 20 production point(s).`

Practical conclusions:

- owned-world salvage is now directly confirmed from classic behavior, not just
  inferred from common sense
- foreign-world salvage failure and empty-target failure were already
  confirmed earlier
- together, the classic rule is now strongly:
  - owned world -> valid salvage path
  - non-owned world -> fail and seek home
  - empty target -> fail with the known wrong v1.51 text and seek home

One accounting detail remains open:

- the report clearly exposes an estimated production yield (`20` points in this
  case)
- the exact classic destination/storage field for those recovered points is not
  yet cleanly decoded from this probe alone
- do not promote a byte-level accounting claim from this one run yet; keep the
  semantic result settled and the final point-accounting field open

Follow-up accounting isolate from `/tmp/salvage-account-pre` ->
`/tmp/salvage-account-post`:

- replayed the same active-campaign salvage order in an isolated before/after
  pair until classic removed the fleet and emitted the success report
- semantic outcome stayed consistent:
  - year advanced from `3030` -> `3034`
  - salvage fleet disappeared
  - owned world `fooville` remained the target world
  - classic report still estimated `20 production point(s)`

Observed byte-level clues:

- `PLAYER.DAT` player-2 `stored_prod_pts_raw` stayed unchanged at `3029`
  throughout this replay
  - so the recovered salvage points are **not** obviously being deposited into
    the player's `stored_production_pts_raw` field
- `PLANETS.DAT` record 13 (`fooville` at `(6,12)`) changed in the putative
  stored-goods region:
  - pre bytes `0x0A..0x0F`: `00 00 00 80 59 08`
  - post bytes `0x0A..0x0F`: `00 00 00 C0 23 08`
  - under the current Rust raw accessor this looks like:
    - pre `stored_goods_raw = 0x80000000`
    - post `stored_goods_raw = 0xC0000000`
- `DATABASE.DAT` also changed in the same replay around the owning-planet row:
  - representative region near offset `1520`:
    - pre:  `00 8a 46 01 d5 0b d5 0b 00 00 64 64 41 00 ff 00 00 e1 00 17 00 d5 0b 00`
    - post: `00 8a 46 01 d9 0b d9 0b 00 00 64 64 41 00 ff 00 00 04 01 1a 00 d9 0b 00`

Current best interpretation:

- the accounting destination is more likely planet-scoped than player-scoped
- however, the changed bytes are **not** yet a clean plain-integer `+20`
  counter under current field assumptions
- this may mean either:
  - the relevant storage field is not yet decoded correctly, or
  - the visible reported salvage estimate is not written as a simple integer in
    the same turn

Keep the semantic rule settled and the accounting encoding unresolved.

---

## ECMAINT timing / stardate anchors

### Session: 2026-03-17

Goal: understand whether the `Stardate: D/YYYY` values in classic player
reports imply a real internal day system in `ECMAINT`, and locate the first
static anchors for that timing path.

#### Historical log sweep: strong evidence for a 52-step yearly scale

A quick sweep over `original/v1.5/ec-logs-2012/*.txt` found:

- `735` `Stardate:` lines total
- minimum observed day = `1`
- maximum observed day = `52`
- repeated year rollover pairs where one year reaches `52` and a later log for
  the next year starts at `1`

Representative rollover examples:

- `ec8.txt`: reaches `Stardate: 52/3009`
- `ec9.txt`: begins with `Stardate: 1/3010`
- `ec47.txt`: reaches `Stardate: 52/3049`
- `ec48.txt`: begins with `Stardate: 1/3052`

Practical conclusion:

- classic `Stardate` values clearly expose an in-year component in the closed
  range `1..52`
- this is much stronger than a cosmetic ranking-year label; the logs support a
  real internal yearly tick scale
- the leading semantic interpretation is now "week-of-year", since `52` fits
  weeks much better than literal days or months

This does **not** yet prove where the day lives in memory or on disk, only
that the player-facing reports behave as if `ECMAINT` is assigning event ticks
within a 52-step year.

#### Behavioral proof that the 52-step scale is mechanical, not just narrative

The strongest black-box evidence so far comes from the shipped `ec.txt` ->
`ec2.txt` campaign progression.

In `ec.txt` (`3001 A.D.`), the fleet review shows:

- `1st Fleet`: colonize `(13,15)`, `Travel Time: 1 year`
- `2nd Fleet`: colonize `(20,11)`, `Travel Time: 2 years`
- `3rd Fleet`: view `(23,5)`, `Travel Time: 2 years`
- `4th Fleet`: guard/blockade homeworld, `Travel Time: None`

Then `ec2.txt` shows the resulting unread reports with exact in-year
stardates:

- `1st Fleet` colonization completes at `Stardate: 32/3001`
- `4th Fleet` guard-starbase arrival reports at `Stardate: 1/3002`
- `3rd Fleet` viewing mission reports entry into `System(23,5)` at
  `Stardate: 12/3002`
- the **same 3rd Fleet** later reports move completion at
  `Stardate: 21/3002`
- `2nd Fleet` colonization completes at `Stardate: 25/3002`

Practical conclusion:

- the `1..52` scale is not just decorative report prose
- `ECMAINT` is sequencing mission progress and report emission on a real
  sub-year timeline
- the presence of two ordered `3002` reports for the same fleet is especially
  strong evidence for a mechanical intra-year schedule
- the main remaining question is the exact implementation of that scheduler,
  not whether it exists

#### New reusable static extractor

Added a focused headless Ghidra script:

- `tools/ghidra_scripts/ECMaintTimingFlow.java`

Current runner behavior on this machine required one small workflow fix:

- `tools/run_ghidra_script.sh` and `tools/run_ghidra_script_args.sh` now stage
  checked-in scripts from `tools/ghidra_scripts/` into
  `tools/ghidra_scripts_tmp/` before headless execution
- this keeps the checked-in script as the source of truth while still using
  the local script path that current headless Ghidra accepts here

The current report is emitted to:

- `artifacts/ghidra/ecmaint-live/timing-flow.txt`

Command:

- `tools/run_ghidra_script.sh ecmaint-live ECMaintTimingFlow.java`

#### First static findings

Static anchors frozen by the new report:

- `2000:6fc6`
  - string cluster includes:
    `Today is ' - maintenance is not scheduled to run.`
  - this remains the clearest current anchor for the maintenance date/schedule
    gate
- `3000:189c`
  - string cluster includes:
    `Enabling player-ranking text file generation...`
  - this is now the best current rankings-output anchor, but still lacks a
    decoded code path to the report stardate formatter
- `3000:39dc`
  - current time-query helper candidate from earlier token-anchor work
  - still not semantically decoded from the live dump

Most importantly, the earlier timestamp-helper assumption changed:

- `2000:945b` currently has only four direct call sites in the analyzed live
  dump
- all four are in the token/schedule region around `0x97xx..0x9exx`
- none are currently tied to player report generation

Practical interpretation:

- `2000:945b` is currently better treated as a current-date/status formatter
  used by maintenance scheduling/token handling
- do **not** treat `2000:945b` as the recovered `Stardate: D/YYYY` player
  report emitter yet

#### Current best model

- each round is still one year, matching the manuals
- within that yearly maintenance pass, classic behavior now strongly suggests a
  52-step internal timeline
- the leading semantic interpretation of that timeline is week-of-year
- player-visible reports appear to be stamped with event ticks inside that
  yearly timeline, not just with "the date maintenance ran"
- this timeline is mechanically relevant to mission/report sequencing, not just
  narrative decoration

#### Still open

- the actual report/rankings `Stardate` formatter path
- whether the tick value is persisted in classic files or exists only in
  scratch/runtime state
- which exact `CONQUEST.DAT` fields feed the maintenance schedule gate
- how specific movement/combat/report subphases map onto individual tick numbers

#### Follow-up static ref sweep

Added one narrower report helper:

- `tools/ghidra_scripts/ECMaintTimingRefs.java`

Command:

- `tools/run_ghidra_script.sh ecmaint-live ECMaintTimingRefs.java`

Current output:

- `artifacts/ghidra/ecmaint-live/timing-refs.txt`

Result:

- `2000:945b` still shows only the same four direct call sites in the
  token/schedule region
- `3000:39dc` still shows only the two token-timeout/schedule callers already
  suspected
- `2000:6fc6` and `3000:189c` show **no direct xrefs**
- there are also **no simple 16-bit scalar hits** for `0x6fc6`, `0x189c`,
  `0x39dc`, or `0x945b` in the current live dump

Practical interpretation:

- the ranking/schedule strings are probably not reached through trivial direct
  immediates in the analyzed memory image
- the next static step should look for string-table walkers, pointer tables, or
  data-segment base calculations rather than only direct references

#### Full shipped-log aggregate timing sweep

Added a reusable log-analysis helper:

- `tools/analyze_ec_report_logs.py`

Command:

- `python3 tools/analyze_ec_report_logs.py`

Current output:

- `artifacts/ec-report-log-analysis.txt`

Aggregate findings from the shipped `ec*.txt` logs:

- `47` files contain `735` timestamped report events
- every file is strictly nondecreasing by `(year, week)`
- `5` files span multiple report years, so unread reports can persist into
  later login sessions
- same-source same-week bundles are common:
  - especially `sensor contact -> identification -> interception`
- same-source multi-week sequences are also common:
  - `extended orbit`
  - later contact/identification
  - later world updates / aborts / interceptions
- Fleet Command Center reports are part of the same weekly ordering rather than
  a separate annual appendix
  - they usually read as administrative loss summaries after combat or
    interception outcomes

Practical interpretation:

- the report corpus has clear intra-year ordering structure
- the stardates are doing real sequencing work
- the current evidence now supports a mechanical weekly scheduler with both:
  - same-week event bundles
  - cross-week mission progression

#### Canonical turn-cycle spec promoted

Added a stable phase-order note:

- `docs/dev/ec-turn-cycle-spec.md`

Current promoted result:

- settled front half:
  - schedule/token gate
  - `Move.Tok` restore-from-backup
  - cross-file integrity validation / load
- settled late half:
  - summary canonicalization / sort
  - explicit late `1..52` weekly loop over active summary entries
- still-open middle:
  - exact placement of economy / production / movement / combat / assault
    resolution inside the yearly simulation core

Most important practical conclusion:

- `ECMAINT` is now clearly more structured than a flat "one yearly step"
  engine
- the oracle-backed model is:
  - gate / recover / validate
  - yearly simulation core
  - late weekly summary/report loop

#### New post-validate/report tail anchor

Added two focused helper scripts:

- `tools/ghidra_scripts/ECMaintTurnCycleAnchors.java`
- `tools/ghidra_scripts/ECMaintFunctionStrings.java`

Current artifacts:

- `artifacts/ghidra/ecmaint-live/turn-cycle-anchors.txt`
- `artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt`

Most useful new static result:

- the best current late-phase driver anchor is now the non-function region
  around `2000:861d`
- after successful `2000:6d9b -> 2000:5ee4` restore/validate flow, it runs a
  fixed call chain:
  - `2000:1da6`
  - `2000:0c06`
  - `2000:2db3`
  - `2000:56be`
  - conditional `2000:7659` if `0x169a != 0`

Interpretation so far:

- this is a late output/report tail, not the full gameplay core
- `2000:56be` is strongly report-oriented:
  - its string family includes mission-report labels for invasion,
    colonization, scouting, seek-home, and starbase/guard-blockade paths
- `2000:0c06` also looks player-report oriented and includes starbase-loss text
- `2000:2db3` is now the strongest `DATABASE.DAT` rebuild candidate:
  - it scales work by `planet_count * 100`
  - this matches the earlier recovered `DATABASE.DAT` `100`-byte slot model

Practical consequence:

- the turn-cycle model is tighter now:
  - restore / validate
  - late report/output tail
  - explicit `1..52` weekly summary loop
- the still-missing gameplay core is now more likely earlier than this
  `861d` tail, or hidden behind helpers feeding it

#### Startup/status indirection and tighter weekly-order corpus evidence

Added two focused helper scripts:

- `tools/ghidra_scripts/ECMaintOuterDriver.java`
- `tools/ghidra_scripts/ECMaintStringXrefs.java`

Current artifacts:

- `artifacts/ghidra/ecmaint-live/outer-driver.txt`
- `artifacts/ghidra/ecmaint-live/string-xrefs.txt`

New static findings:

- `0000:841a` is **not** the startup driver
  - it is a late kind-`1`/starbase-related helper using scratch fields
    `350d/350e/351b..351f/3522..3524`
- the startup/status string cluster at `2000:841b..855a` still has **no direct
  scalar xrefs** in the current live dump:
  - `main.tok`
  - `Performing integrity check of game files...`
  - `Creating main work file...`
  - `Merging joint fleets and setting required speeds...`
- practical interpretation:
  - the outer startup/status path is probably reached through an indirect
    string/pointer mechanism rather than inline `MOV DI, imm16` references
- by contrast, the later status strings at `2000:7c44/7ca1/7cd8/7cf3/7cf8` do
  have direct scalar hits in the `861d -> 8b3d` late tail

Log-analysis tightening:

- reran `python3 tools/analyze_ec_report_logs.py` after adding ordered-pattern
  aggregation
- new corpus-level timing/order results:
  - top same-week ordered bundle:
    - `38x sensor-contact -> identified`
  - same-week longer chain also recurs:
    - `3x sensor-contact -> identified -> interception`
  - adjacent report week-gap distribution is heavily concentrated at:
    - `gap 0: 350`
    - `gap 1: 67`

Practical consequence:

- weekly report order is structured, not just weekly timestamps glued onto
  free-form report text
- the corpus now supports a stronger timing claim:
  - same-week micro-order exists inside the weekly scheduler
  - immediate next-week transitions are also common, which fits a real ordered
    event stream

#### Corpus-side combat/admin follow-on ordering tightened further

Extended `tools/analyze_ec_report_logs.py` with targeted transition counts for
combat/admin follow-on patterns.

Current new aggregate results:

- `identified -> fleet-lost` same week: `4x`
- `attacked -> fleet-lost` next week: `2x`
- `fleet-lost -> join-retarget` same week: `2x`
- `fleet-lost -> planet-bombarded` same week: `4x`
- `intercepted -> planet-bombarded` next week: `3x`

Concrete examples:

- `ec20.txt` `40/3021`:
  - Fleet Command Center reports loss of the `7th Fleet`
  - same week, another fleet emits `Join mission report: In light of the
    destruction of the 7th Fleet...`
- `ec41.txt` `33/3042`:
  - Fleet Command Center reports loss of `Starbase 2`
  - same week, planet `"hector"` emits bombardment damage

Practical consequence:

- Fleet Command Center loss summaries are not a detached annual appendix
- at least some retargeting and bombardment consequences are emitted on the
  same weekly stream immediately after combat/admin loss events
- this further supports a staged but shared event pipeline:
  - simulation consequences land on ordered weekly ticks
  - admin/follow-on reports are injected into that same sequence

#### Post-validate phase-call roles tightened

Added a focused helper script:

- `tools/ghidra_scripts/ECMaintPhaseCalls.java`

Current artifact:

- `artifacts/ghidra/ecmaint-live/phase-calls.txt`

Most useful new static result:

- `2000:56be` is now even more clearly a mission-report emitter rather than a
  gameplay-core routine
  - it repeatedly walks summary-like pools rooted at:
    - `0x2f78`
    - `0x2ff8`
    - `0x31f8`
    - `0x3178`
  - it emits mission-family prefixes including:
    - invasion
    - colonization
    - scouting / viewing / move-like families
    - seek-home
    - starbase / guard-blockade world
- `2000:1da6` and `2000:0c06` also show the same broad output-heavy shape:
  - repeated `0x3000:4057 / 3f88 / 3fac / 3be9 / 159b` status/message writes
  - repeated file-ish helper usage around `4f4c / 4f7a / 4f83 / 4ffb / 502f /
    5036 / 50cd / 50fd / 5114 / 5189 / 51a0`
- `2000:2db3` still remains the strongest derived-output rebuild candidate
  - it shows the same file-ish helper family plus unique internal calls to
    `2000:33f7`
  - this still fits the earlier `DATABASE.DAT` rebuild hypothesis better than
    a direct simulation-core role

Practical consequence:

- the full `8652` post-validate chain now looks even more like a
  report/regeneration/output tail:
  - `1da6`
  - `0c06`
  - `2db3`
  - `56be`
- the unresolved gameplay core ordering is therefore increasingly likely to sit
  before this chain, or behind the earlier setup/helpers feeding it, rather
  than inside these functions themselves

#### `2000:7659` and `2000:8b4a` tightened as end-of-tail rankings/output cleanup

Reviewed:

- `artifacts/ghidra/ecmaint-live/probe-2000_7659.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_8b4a.txt`
- `artifacts/ghidra/ecmaint-live/turn-cycle-anchors.txt`

Current static result:

- `2000:7659` is only reached from the already-late `861d` tail when
  `0x169a != 0`
- inside `7659`:
  - it loops over staged `0x16ae` records, not live fleet/base summary pools
  - it allocates fixed `0x49`-byte blocks
  - it uses the same `3f17 / 3d84 / 3eea / 41b0` helper cluster already seen
    from other output-heavy regions
  - the surrounding timing/string work still fits the previously recovered
    player-ranking output anchor better than gameplay simulation
- `2000:8b4a` immediately after the late summary/coalescing region performs
  only flag reset/arming work:
  - `word [0x638] = 1`
  - `byte [0x636] = 0`
  - `byte [0x169a] = 0`
  - `byte [0x634] = 1`
  - `byte [0x635] = 0`
- side signal:
  - `0x169a` is set earlier in the restore/startup path (`6e91` / `6fb1`)
  - then gates `0c06`, `2db3`, and the optional `7659` call
  - this makes the `169a` family look like late output/ranking/report-control
    state, not middle simulation state

Practical consequence:

- `7659` is now best treated as optional rankings/output generation appended to
  the already-late report tail, not part of yearly gameplay mutation
- `8b4a` is the corresponding end-of-tail cleanup/reset point
- in the earlier late-tail functions too, `0x169a` now looks like setup-mode
  gating rather than pass-presence gating:
  - `1da6`, `0c06`, and `2db3` all test `0x169a` near entry
  - in each case the branch only selects an initial open/header/status setup
    path for the output workspace (`0x30f8`, `0x3078`, or local scratch around
    `0xff78`)
  - after that preamble, the main player/planet/report scan still proceeds
- `0x169a` now also has a tighter origin:
  - in `6d9b(arg=0)`, a direct `5ee4` success returns immediately without
    setting `0x169a`
  - `0x169a` is only set at `6fb1` after:
    - direct `5ee4` failure on the plain path
    - recursive `6d9b(arg=1)` registered-stream recovery
    - successful return from that recovery path
  - current best reading:
    `0x169a` means "late tail is running after restore/recovery succeeded",
    not "rankings are generally enabled" and not any gameplay-core phase bit
- the pre-`6d9b` feeder handshake is now tighter even though segment-`3000`
  decode remains missing:
  - at the outer driver:
    - `8612  CALLF 3000:1abc`
    - `8617  CALLF 3000:1e88`
    - `861c  PUSH AX`
    - `861d  CALL 2000:6d9b`
  - practical implication:
    - `3000:1abc` is side-effect only at this callsite
    - `3000:1e88` is the direct producer of the `6d9b` mode argument
    - so the feeder pair should be modeled as:
      - pre-validation setup helper
      - mode-selector/helper whose return becomes `6d9b(arg)`
- negative result:
  - probing `3000:1abc` and `3000:1e88` against both:
    - the live dump project
    - the original-binary `ec-v15` Ghidra project
    still yields `<no instruction>` / `<no function>`
  - current best reading is not "those helpers do nothing", but "this region is
    still unmapped/undecoded in the available projects", so feeder-side RE must
    continue indirectly from callers and effects
- the `731f` live-dump block on the recovery side of `6d9b` is now better
  bounded as restore/workspace preparation, not turn simulation:
  - entry sequence:
    - checks a `3000:1804` result from CS:`6a22`
    - on failure it emits status text through the same `0x46cc` +
      `3000:4057/3f88/3be9` message path already seen in token-timeout helpers
  - then:
    - allocates `0x20a` bytes with destination `0x33f8`
    - copies from `0x2d68` into that workspace
    - reads a bounded record into stack scratch
    - parses bytes out into `0x2d6a..0x2d70`
    - bulk-copies the seeded block back into `0x33f8`
  - current best reading:
    this block is reconstructing or refreshing token/restore working state
    after validation trouble, not running economy/movement/combat phases
  - practical implication:
    move remaining turn-order RE earlier than this feeder/recovery cluster
- the sibling durable-summary drivers `1000:00e8` and `1000:024d` are now
  better separated:
  - both still share the same broad shape:
    - reset local counters / flags
    - call `1000:f71d` (which reaches kind-1 writer `1000:dddb`)
    - call `1000:d5d2`
    - call `1000:b6d8`
    - eventually append kind-2 through `1000:e31b`
  - only `1000:024d` inserts an extra gate before the kind-2 append:
    - if player `+0x5a > 0`, call `1000:db04(arg=0x0a, player)`
    - then test `0x5dc/0x5de`
    - only when that pair stays zero does it call the unmapped helper window
      around `1000:f2c7` and then `1000:e31b`
  - `1000:db04` itself is not a top-level yearly phase:
    - it mutates counters inside the current player-side record at
      `+0x30`, `+0x34`, and `+0x36`
    - these same fields are read later by `2000:0c06`
    - current best reading is per-player timing / reviewable-state adjustment,
      not a separate movement/combat/economy driver
  - `1000:d5d2` is likewise better bounded as player-side prep/marking:
    - it sanitizes/initializes bytes in the active `0x16ac` record
    - then scans existing kind-1 entries in `0x2f72/0x2f76` for the same
      owner before returning
  - `1000:f2c7` is still not mapped as a named function boundary, but the
    recovered local window is enough for one practical conclusion:
    - it decodes a summary payload through `2000:c09a`
    - runs a local matcher via `1000:d166`
    - conditionally calls `2000:c151`
    - so it also looks like local summary/timing gating, not a gameplay-core
      subphase
- practical implication:
  - the `00e8/024d` split is currently best treated as conditional
    per-player summary/timing preparation inside the durable event-generation
    phase
  - do not promote that branch point into Rust as evidence for the canonical
    order of economy, movement, combat, or assaults
- the player-side timing/report counters at `+0x30/+0x32/+0x34/+0x36` are now
  tighter as late-output state, not gameplay-core scheduling:
  - `2000:0c06` begins by scanning all active `0x16ac` player records and
    sets its local "anything to report?" gate if either family is nonzero:
    - `(+0x32,+0x30)` nonzero
    - or `(+0x36,+0x34)` nonzero
  - only after that gate does `0c06` open and populate the `0x3078`
    output workspace
  - `2000:5404` is a straight merge helper over the same late fields:
    - adds source into destination for `+0x34`
    - also accumulates `+0x26/+0x28/+0x2a/+0x32/+0x30/+0x2e/+0x2c`
    - clamps byte `+0x09` / `+0x0a`
    - then calls local output/summary helpers `3fc0`, `451c`, and `3f27`
  - current best reading:
    these words are part of late player-output aggregation / reviewable state,
    not evidence for where economy, movement, or combat occur in the year
- `1000:cba4`, a direct writer reached from `1000:e79a -> 1000:e928`, makes
  the `+0x34` story tighter:
  - it computes a weighted scalar from scratch fields:
    - `0x15 * [+0x26]`
    - `0x07 * [+0x28]`
    - `0x02 * [+0x2a]`
    - plus `+0x2c/+0x2e`
    - while `+0x30/+0x32` currently contribute zero in this helper
  - it writes that derived value straight into `record[+0x34]`
  - then, if mission/order byte `record[+0x1f] == 4` and byte `record[+0x0a]`
    is zero, it adds `0x1e`
  - caller context:
    `1000:e79a` invokes `cc5b`, then `cba4`, then kind-1 writer `dddb`
- practical implication:
  - at least one important `+0x34` writer is a derived late-summary scorer or
    classifier built from already-staged scratch data, not a core simulation
    clock or weekly scheduler
  - keep looking earlier than this summary-prep family for canonical
    economy/movement/combat order
- the previously unmapped `1000:0612..0794` timing window is now better bounded
  too:
  - it iterates a per-slot table rooted at `[BP+0xff7c]`
  - for each slot, it adds `entry[+0x22]` into that slot's running
    `player[+0x34/+0x36]` pair
  - if the addition overflows the accepted range, it saturates the pair to
    `0xffff:0000`
  - it also folds the resulting values into `player[+0x58]` and `player[+0x5a]`
    with the same saturating style
  - then it zeroes `player[+0x34]` again before continuing
- practical implication:
  - this window also behaves like accumulator / carry-forward bookkeeping over
    late player-side counters, not like the hidden yearly gameplay-core order
  - combined with `0c06`, `5404`, and `cba4`, the `+0x34/+0x36` family is now
    strongly bounded as derived late-state plumbing rather than movement,
    combat, or economy sequencing evidence
- the `1000:f319` / `1000:f34a` sibling family is now tighter too:
  - shared helper list:
    - `d8a5`
    - `e79a`
    - `d92a`
    - `ea5f`
    - `eee7`
  - only `f319` adds the final `f1ee` postpass
  - `f1ee` is now firmly kind-2-only:
    - scans the durable pool `0x2f72/0x2f76`
    - filters on current owner and `entry[+0x04] == 2`
    - decodes via `2000:c09a`
    - runs `1000:d166`
    - conditionally calls `2000:c151` / `2000:c100`
  - `eee7` is the corresponding kind-1-side late classifier/postpass:
    - scans the same durable pool for current-owner entries with
      `entry[+0x04] == 1`
    - decodes via `2000:c067`
    - uses `d166`, `ba44`, `cc5b`, and `44b7`
    - branches across small local mode bytes (`2`, `5`, `0xa`, etc.)
  - `d8a5` is also bounded as local status derivation rather than a phase
    driver:
    - computes a value through `2000:88c8`, `2000:8830`, and numeric compare
      helpers
    - stores the resulting byte into player `+0x51`
- practical implication:
  - the whole `f319/f34a` family is now best treated as durable summary
    production plus kind-specific follow-up passes
  - it is no longer a promising place to search for the hidden middle
    turn-order sequencing
- first upstream anchor above that family:
  - `2000:0788` is a planet-side loop over `0x1712/0x1714`
  - it filters by owner byte `+0x5d`
  - sums planet byte `+0x02`
  - marks player byte `+0x6d = 1`
  - emits a one-time status string through `0x46cc` / `3000:4057/3f88/3be9`
  - then directly calls `2000:f34a` for the selected player
- practical implication:
  - `f34a` is at least sometimes entered from planet-side late aggregation,
    not only from an opaque deeper dispatcher
  - this still points to late summary/report generation rather than hidden
    economy/movement/combat ordering, but it gives a concrete upstream seam for
    future tracing
- stronger upstream selector above the sibling split:
  - `2000:05df..06e5` is now the first confirmed direct branch that chooses
    between the sibling summary families
  - inside its per-player loop:
    - if player byte `+0x00 == 0xff`, it emits a one-time status string through
      `0x46cc` / `3000:4057/3f88/3be9`
    - then it checks player byte `+0x50`
    - if `+0x50 > 0`, it directly calls `2000:f319`
    - otherwise it falls into local helper `2000:d6ac`
  - on the non-`0xff` branch, it resets `0x34fe/0x3500`, calls `2000:f713`,
    compares `0x190c - player[+0x4e]` against `0x2f70`, may mark
    `player[+0x6d] = 1`, then emits the later one-time string and calls
    `2000:f34a`
- practical implication:
  - the old "no visible `f319` caller" result is now closed; `f319` is a real
    sibling selected from the same late player loop as `f34a`
  - this loop still looks like late player/report selection, not the hidden
    gameplay-core yearly order
  - the useful next questions are now the raw meanings of player `+0x50`,
    `+0x4e`, and `+0x6d`, rather than whether `f319` is reachable at all
- `player[+0x50/+0x52]` is now bounded more tightly too:
  - in `2000:05df`, `player[+0x50] > 0` is the direct gate that selects the
    `f319` sibling instead of the local `d6ac` fallback on the `player[0] == 0xff`
    side of the late player loop
  - in `1000:e79a`, `player[+0x50/+0x52]` is consumed as a decrementing
    32-bit counter:
    - it tests `+0x52`, then `+0x50`
    - calls local helper `1000:e2da`
    - subtracts `1` from `+0x50` with borrow into `+0x52`
  - in `1000:ea5f`, the same family gates whether the shared kind-1-side scan
    even runs (`player[+0x40] > 0` and `player[+0x50] > 0`)
  - in the late summary-post-canonical pass, the same player family is
    refreshed:
    - clears `player[+0x50]`
    - calls `2000:8830` then `3000:4895`
    - stores the result into `player[+0x52]`
- practical implication:
  - `+0x50/+0x52` is best treated as a late per-player quota / work counter
    used by summary/report generation, not as evidence for gameplay-core
    sequencing
  - exact gameplay meaning is still open; current evidence is strong on
    lifetime and control use, but not yet enough to rename it semantically
- `player[+0x6d]` is now bounded negatively:
  - current live-dump text coverage only shows one writer and one reader, both
    inside `2000:05df..06e5`
  - writer:
    - after `2000:f713`, if `0x190c - player[+0x4e] >= 0x2f70`
    - and `0x2f70 > 0`
    - and player byte `+0x00 == 1`
    - then it sets `player[+0x6d] = 1`
  - reader:
    - the same loop immediately tests `player[+0x6d]`
    - only then emits the one-time `0x3c80` status string and calls `f34a`
- practical implication:
  - `+0x6d` is currently best treated as a local late-loop eligibility scratch
    mark, not durable player state and not evidence for gameplay-core ordering
  - the remaining useful unknown in this branch is more likely the predicate
    behind `player[+0x4e]` / `0x2f70` than the `+0x6d` byte itself
- practical Rust implication:
  - do not use the `169a` / `634` / `635` / `636` / `638` flag family as
    evidence for economy, movement, combat, or other gameplay-core ordering
  - keep `rust-maint` split between:
    - state mutation
    - durable report/event creation
    - optional report/ranking/output regeneration plus final housekeeping

#### Mission-family timing after combat is not one universal lag

Extended the report-corpus analysis with mission-family-specific lag checks for
combat-facing transitions.

Current results:

- bombardment:
  - `attacked -> bombing-run` appears at gaps `0`, `5`, `6`, and `7`
- invasion:
  - `attacked -> invaded` observed at gap `7`
- colonization:
  - `attacked -> arrived-target` observed at gap `2`
- guard/blockade:
  - `arrived-world -> intercepted` observed at gaps `1`, `6`, `27`, and `43`

Practical consequence:

- there is no single global "combat consequence delay" rule that Rust can copy
  across all mission families
- the oracle is placing outcomes onto the weekly schedule in a
  mission-context-dependent way
- this reinforces the intended Rust architecture:
  - deterministic combat resolution stays Rust-native
  - but the resulting aftermath must be placed into mission-specific weekly
    scheduling and report sequencing that mirrors the oracle

Additional same-week aftermath evidence:

- `bombing-run -> invaded` same week appears `2x`
  - this strongly suggests different fleets' mission outcomes are interleaved
    into one shared weekly event stream rather than emitted in isolated
    per-feature batches

#### `2000:33f7` tightened as intelligence/database helper, not gameplay core

Added a reusable single-function probe helper:

- `tools/ghidra_scripts/ECMaintFunctionProbe.java`

Current artifact:

- `artifacts/ghidra/ecmaint-live/probe-2000_33f7.txt`

Recovered structure for `2000:33f7`:

- incoming refs:
  - `2000:32a2`
  - `2000:33c2`
- the first useful embedded string at `2000:2bd2` reads into:
  - `Backing up intelligence database...`
  - followed by backup-failure text for the intelligence database
- the function:
  - allocates a large stack frame
  - copies `0x64 * [0x1714]` bytes into a scratch region
  - uses helper family:
    - `0x3000:4136`
    - `0x3000:4f83`
    - `0x3000:506c`
    - `0x3000:4ffb`
  - sets `byte ptr [0x34f8] = 1` on completion

Practical interpretation:

- `2000:33f7` is not combat/movement/economy logic
- it is a focused intelligence-database backup/regeneration helper inside the
  broader `2db3` derived-output path
- this further strengthens the reading that:
  - `2db3` belongs to the late derived-file/output side of `ECMAINT`
  - the unresolved gameplay simulation core still lies earlier than the
    `8652` call chain, or in the setup path feeding it

#### Late kind-`2` weekly loop tightened as summary coalescer, not gameplay core

Reviewed the larger `2000:87f4 -> 2000:8b15` region from
`artifacts/ghidra/ecmaint-live/turn-cycle-anchors.txt`.

Current static result:

- this region sits after the already-identified late post-validate call chain:
  - `2000:1da6`
  - `2000:0c06`
  - `2000:2db3`
  - `2000:56be`
- it iterates over the summary pointer table at:
  - `0x2f72`
  - count `0x2f76`
- outer selection is gated to summary kind `2`
- for each kind-`2` entry, it scans other summary entries looking for kind `1`
  candidates that match:
  - owner byte `+0x00`
  - X byte `+0x01`
  - Y byte `+0x02`
  - mode/flag byte `+0x05`
- once those structural keys match, it decodes summary payload word `+0x06`
  through helper pair:
  - `2000:3f5a`
  - `2000:3f27`
- the matched payloads then feed more late text/output helpers:
  - `2000:49f2`
  - `0x3000:4202`
  - `2000:47fe`
  - `0x3000:428f`

Practical consequence:

- `2000:87f4 -> 2000:8b15` is another late summary merge/coalescing stage
  inside the report/weekly side of the pipeline
- it is not the missing yearly gameplay simulation core
- that further narrows the real unresolved turn-order target:
  - not `2000:6d9b`
  - not `2000:5ee4`
  - not the fixed `8652` post-validate call chain
  - not the later kind-`2` / kind-`1` summary pairing loop
- the missing middle simulation ordering is therefore increasingly likely to be
  earlier than the `861d` tail or hidden behind helpers feeding that tail

#### `2000:9e1e` tightened as summary-pool initializer

Added direct probes:

- `artifacts/ghidra/ecmaint-live/probe-2000_9e1e.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_9b13.txt`

Current static result:

- `2000:9e1e` is not gameplay logic
- it performs startup-side state initialization:
  - calls `0x3000:39dc`
  - stores the returned time tuple at `0x34fa/0x34fc`
  - zeroes summary count `0x2f76`
  - requests size `0xfa00` through `2000:9b13`
  - stores the returned far pointer at `0x2f72/0x2f74`
- `2000:9b13` is likewise startup/plumbing:
  - time-delta / timeout flavored helper calls
  - allocation through `0x3000:397f`
  - timeout/failure text emission if the request cannot be satisfied

Practical consequence:

- the summary table consumed later by:
  - post-canonical sorting
  - the `1..52` weekly loop
  - the late kind-`2` / kind-`1` coalescing pass
  is explicitly initialized up front in the startup/token-side path
- this strengthens the current turn-cycle boundary:
  - startup path seeds the summary workspace early
  - later gameplay/report helpers populate and consume it
  - `9e1e` itself is not the missing yearly simulation core

#### `2000:5ee4` front half tightened as player/planet staging, with no new tail-side summary producer

Added/used probes:

- `artifacts/ghidra/ecmaint-live/probe-2000_5ee4.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_63ca.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_6ab4.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_0796.txt`
- `artifacts/ghidra/ecmaint-live/probe-2000_0c06.txt`
- `artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt`

Current static result:

- `2000:5ee4` starts by zeroing:
  - `0x16ae`
  - `0x1714`
  - `0x190a`
- it then stages two input collections before the already-known summary
  branches:
  - `0x3278` with record size `0x6e`
    - loaded into far-pointer table `0x16ac`
    - count tracked in `0x16ae`
  - `0x2f78` with record size `0x61`
    - loaded into far-pointer table `0x1712`
    - count tracked in `0x1714`
- the recovered direct summary-emission branches inside `5ee4` remain:
  - `0x3178` fleet -> kind `1`
  - `0x2ff8` base -> kind `2`
  - `0x31f8` IPBM -> kind `3`
- `2000:63ca` is the join from the front-half staging and fleet branch into the
  base-side branch; it does not introduce a separate player-only producer
- `2000:6ab4` is the tail gate:
  - if validation failed, return failure immediately
  - otherwise zero summary count `0x2f76`
  - free every staged `0x3278` buffer via size `0x6e`
  - free every staged `0x2f78` buffer via size `0x61`
  - zero `0x16ae` / `0x1714` and return
- supporting side evidence:
  - `2000:0c06` walks `0x16ae` / `0x16ac`, which fits player-side late
    processing rather than hidden simulation
  - `2000:0788` / `0796` walks `0x1714` / `0x1712`, filters on owner byte
    `+0x5d`, and sums byte `+0x02`, which fits planet-side aggregation

Practical consequence:

- current best model:
  - `9e1e` seeds the shared summary workspace
  - `5ee4` stages player-side and planet-side collections
  - `5ee4` emits the currently known fleet/base/IPBM summary entries
  - later canonicalization and weekly/report passes consume those summaries
- the remaining unresolved gameplay-core ordering is therefore less likely to
  be hiding in the already-recovered `5ee4` exits; the next productive RE
  target is the earlier helper chain that populates or consumes these staged
  collections before the late report side

Additional practical tightening:

- `5ee4`'s `0x2f72 / 0x2f76` writes are now better read as transient validation
  scratch, not automatically as the durable late-report summary/event pool
  because:
  - the fleet/base/IPBM branches do append `0x0c` entries and increment
    `0x2f76`
  - but tail `2000:6ac3` immediately executes:
    - `XOR AX,AX`
    - `MOV [0x2f76],AX`
  - before `5ee4` returns
- practical Rust implication:
  do not merge "validation findings" and "later report events" into one engine
  structure by default just because they share the same backing workspace in
  DOS memory; lifetimes appear to differ even if storage is reused

Additional durable-writer recovery:

- the first currently confirmed non-`5ee4` writers to the durable
  `0x2f72 / 0x2f76` pool are in segment `1000`, not `2000`
  - `1000:dddb` / probe point `1000:e09d`
  - `1000:e31b` / probe point `1000:e569`
- both functions:
  - `INC [0x2f76]`
  - allocate `0x0c` bytes via `0x3000:1c53`
  - store the new pointer in the shared pool table at `0x2f72`
  - then fill the entry in place
- the recovered layouts match the later kind-1 / kind-2 pipeline:
  - `1000:dddb` writes `entry[+0x04] = 1`
  - `1000:e31b` writes `entry[+0x04] = 2`
  - both also write:
    - `entry[+0x03] = 1`
    - owner / actor byte at `+0x00`
    - coords at `+0x01/+0x02`
    - payload word at `+0x06`
    - key-ish word at `+0x0a`
- practical Rust implication:
  the durable late report-event pool is now better modeled as a dedicated
  post-validation summary-generation phase driven by separate producer helpers,
  not as a continuation of `5ee4` scratch emission

Further ordering gain:

- the durable kind-1 / kind-2 writers are now placed inside sibling `1000`
  driver routines rather than treated as unrelated helpers:
  - `1000:00e8` calls `1000:f71d`, then later calls `1000:e31b`
  - `1000:024d` does the same
- `1000:f71d` reaches the kind-1 writer through `1000:f8a9 -> 1000:dddb`
- so, for this recovered durable-event family, kind-`1` emission precedes the
  direct kind-`2` emission in the same higher-level yearly flow
- practical Rust implication:
  do not collapse kind-`1` and kind-`2` durable event creation into one
  unordered "derive all summaries" sweep; preserve producer-pass ordering and
  append order as part of the event-generation phase

#### `2000:6d9b` tightened as restore/validation wrapper with recursive retry path

Added probe:

- `artifacts/ghidra/ecmaint-live/probe-2000_6f20.txt`

Current static result:

- `2000:6d9b` has two entry modes:
  - if `[BP+0x04] != 0`, stay on the existing `6daa` path
  - if `[BP+0x04] == 0`, jump to `2000:6f20`
- `2000:6f20` does:
  - call `5ee4`
  - if `5ee4` succeeds, return success immediately
  - if `5ee4` fails, emit recovery/error text through `46cc` / `159b`
  - then call back into `2000:6d9b` with pushed argument `1`
- the `arg = 1` path at `6daa`:
  - registers two `0x3000:4f4c` callback waves around `5ee4`
  - the registered stream anchors are:
    - `0x2f78`
    - `0x2ff8`
    - `0x3078`
    - `0x30f8`
    - `0x3178`
    - `0x31f8`
    - `0x3278`
    - `0x32f8`
    - `0x3478`
  - still funnels through `5ee4` and then returns a simple success/failure
    byte

Practical consequence:

- `6d9b` is increasingly well-bounded as restore/integrity scaffolding around
  `5ee4`, with an alternate registered-stream retry mode
- this is another negative result for the middle-turn-order hunt:
  - `6d9b` still does not look like the missing yearly gameplay-core phase
  - the unresolved ordering remains more likely in earlier helpers or outside
    the already-fixed `6d9b -> 5ee4 -> 8652` framing path

#### Step-4 workflow and `1000:024d` interior tightened

Added probes:

- `artifacts/ghidra/ecmaint-live/probe-1000_03ff.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_07ff.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_0b67.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_0d53.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_0dda.txt`

Current structural result:

- the partially recovered `1000:03ff..0d53` owned-planet body is **not** a
  standalone hidden driver
- it sits inside sibling durable-summary driver `1000:024d`
- `1000:0138` already showed the `1000:00e8` sibling stop before this region,
  while `1000:024d` continues through the deeper planet-side loop

What `1000:024d` now concretely does before returning:

- starts with the already known durable-event front half:
  - `cce7`
  - `f71d`
  - `d5d2`
  - `b6d8`
  - optional `db04(arg=0x0a)` when current planet `+0x5a > 0`
  - `f2c7`
  - `e31b`
  - `e1c0`
  - `f9ff`
  - `f914`
  - `c025`
  - `c9a0`
  - `fe73`
- then enters a deeper per-owned-planet loop at `03ff`:
  - iterates current planet records from staged table `0x1712`
  - skips entries with owner byte `planet[+0x5d] == 0`
  - advances byte `planet[+0x5c]` through a small state ladder
  - seeds local numeric triples from that ladder
  - gates on owner player `player[+0x44] > 0`
  - scans durable kind-`2` entries for current coords/owner with active flag
  - decodes them through `2000:c09a`
  - folds per-slot `entry[+0x22]` into running `+0x34/+0x36`
  - writes results back into planet words `+0x58/+0x5a`
  - applies additional real-valued transforms to planet `+0x09/+0x0b/+0x0d`
  - clears the temporary slot accumulators before looping

Late-half tightening from `07ff` and `0b67`:

- this deeper `024d` interior still mutates live planet fields directly
- it is not just report formatting:
  - clamps / updates planet reals at `+0x03/+0x05/+0x07`
  - clamps / updates another real triple at `+0x09/+0x0b/+0x0d`
  - uses owner player byte `player[+0x51]`
  - branches on planet byte `+0x60`
- practical reading:
  `024d` is a mixed step-4 pass that combines planet-side state mutation with
  later kind-`2` summary/event staging

Practical consequence for Rust:

- do not demote `1000:024d` to "late output helper"
- also do not treat `03ff..0d53` as a separate earlier global scheduler
- the safer current model is:
  - `00e8` / `024d` are sibling yearly producer passes
  - `024d` owns a materially richer planet-side mutation interior than `00e8`
  - this family is now a real bridge between step-4 state mutation and durable
    event creation
- best next RE target inside step `4` is now:
  - compare `00e8` vs `024d` semantics more tightly
  - determine what planet bytes `+0x5c` and `+0x60` mean operationally
  - identify where `024d` is entered from in the broader yearly flow

#### Direct oracle matrix on `PLANETS.DAT[+0x5a/+0x5c/+0x60]` against `ecmaint-econ-pre`

Controlled probe:

- baseline: `fixtures/ecmaint-econ-pre/v1.5`
- target world: record `15` at `(16,13)` owned by empire `1`
- direct matrix over one world only:
  - control
  - `+0x5a = 0`
  - `+0x5c = 0`
  - `+0x5c = 1`
  - `+0x60 = 1`
  - `+0x5a = 0` plus `+0x60 = 1`
- each case ran through classic `ECMAINT` for `2` maint ticks

Observed result:

- control case:
  - only record `14` changed
  - same broad planet-side diff family already expected from the econ fixture:
    - `+0x03..+0x09`
    - `+0x0e`
    - `+0x58`
    - `+0x5a`
- forcing target-world `+0x5a = 0`:
  - produced the same changed-record shape as control
  - no direct changes landed in record `15`
- forcing target-world `+0x5c = 0` or `+0x5c = 1`:
  - record `15` changed only at `+0x5c`
  - classic normalized that byte back to `2`
  - record `14` still took the usual deeper planet-side mutation, but with
    altered numeric outcomes in the real/value block and final `+0x58`
- forcing target-world `+0x60 = 1`:
  - record `15` now changed strongly at `+0x03..+0x0e`
  - no `ERRORS.TXT`
  - no `RESULTS.DAT` payload
  - record `14` still changed in its usual econ-family block
- forcing `+0x5a = 0` plus `+0x60 = 1`:
  - record `15` still changed strongly at `+0x03..+0x0e`
  - target `+0x5a` then also cleared from `4 -> 0`

Current practical reading:

- `planet[+0x60]` is the strongest currently observed selector for the deeper
  `1000:024d` interior:
  - setting it to `1` activates direct mutation of that same world's
    `+0x03..+0x0e` block
  - this happened without any report output side effects, which reinforces
    that the path is genuine planet-state mutation rather than text staging
- `planet[+0x5c]` currently looks more like a normalized status/input byte than
  the main deep-branch selector:
  - values `0` and `1` both normalized back to `2`
  - they influenced the neighboring econ-world outcome but did not activate
    the large direct `+0x03..+0x0e` rewrite on the edited world
- in this probe family, `planet[+0x5a]` did **not** decide whether the deeper
  `+0x03..+0x0e` rewrite occurred:
  - zeroing `+0x5a` alone did not trigger it
  - `+0x60 = 1` still triggered it even when `+0x5a` was zero

Practical consequence for Rust step-4 modeling:

- do not treat `+0x5a > 0` as the main selector for the deep `024d` planet
  path just because the front half gates `db04` on it
- treat `+0x60` as the highest-priority raw candidate for a planet-local
  branch inside the richer `024d` producer pass
- treat `+0x5c` as a likely normalized ownership/development-status input that
  affects outcomes but is not itself the same deep-branch bit

Follow-up confirmation on a maintenance-stable baseline:

- repeated the narrower probe against `fixtures/ecmaint-post/v1.5`
- target world again: record `15` at `(16,13)`
- results after `2` maint ticks:
  - control: no record-`15` changes
  - `+0x5c = 0`: only `+0x5c` normalized back to `2`
  - `+0x60 = 1`: direct rewrite again landed at `+0x03..+0x0e`

This strengthens the reading that:

- `+0x60` is a local branch/control byte for the deeper `024d` planet-mutator
  path, not just an econ-fixture-specific coincidence
- `+0x5c` is still behaving like a normalizable status field rather than the
  primary deep selector

Fixture-corpus sweep:

- checked the preserved `PLANETS.DAT` files in:
  - `ecmaint-post`
  - `ecmaint-econ-pre`
  - `ecmaint-econ-post`
  - `ecmaint-build-pre`
  - `ecmaint-invade-pre`
  - `ecmaint-fleet-pre`
  - `ecmaint-move-pre`
- every sampled planet record had `+0x60 = 0`

Practical implication:

- `+0x60` is currently a latent branch/control byte in the preserved corpus
- that explains why the deeper `024d` same-world rewrite path stayed hidden in
  fixture diffs until we forced the byte nonzero directly

Direct cofactor check on neighboring world fields:

- repeated a narrower matrix on the maintenance-stable `ecmaint-post` baseline
  for target world record `15`
- tested nearby plausible cofactors without forcing `+0x60`:
  - `+0x0e = 4`
  - `+0x58 = 35`
  - `+0x5a = 0`
  - combinations of those fields
- result:
  - none of those edits produced any record-`15` mutation after two maint
    ticks
  - the same deep `+0x03..+0x0e` rewrite only reappeared when `+0x60 = 1` was
    included

Practical consequence:

- current evidence favors `+0x60` as a true gate/branch byte for this
  planet-local `024d` path, not just as a proxy for the neighboring economy /
  defense fields already visible in the record

Owned/unowned and status follow-up on the same `+0x60` branch:

- repeated the direct `+0x60 = 1` probe on several `ecmaint-post` worlds:
  - owned record `15` (empire `1`)
  - owned record `13` (empire `2`)
  - owned record `5` (empire `3`)
  - unowned record `14` with owner `0`
- results after `2` maint ticks:
  - all three owned worlds took the same broad direct rewrite at `+0x03..+0x0e`
  - the exact numeric payload differed by world, but the structural rewrite
    shape matched across owners
  - the unowned world with owner `0` did **not** take that rewrite at all
- status normalization still coexists with the branch:
  - on owned record `15`, forcing `+0x5c = 0` or `1` together with `+0x60 = 1`
    still triggered the deep rewrite
  - classic then normalized `+0x5c` back to `2`

Report/output side evidence for these cases:

- `RESULTS.DAT`: empty
- `MESSAGES.DAT`: empty
- `ERRORS.TXT`: empty
- `DATABASE.DAT` / `RANKINGS.TXT`: regenerated as usual

Current practical reading:

- the deep `024d` same-world branch currently looks gated by:
  - `planet[+0x60] != 0`
  - plus the world being owned (`owner byte +0x5d > 0`)
- `planet[+0x5c]` still looks like a normalized status byte, not a blocker for
  the branch as long as ownership remains present

Unowned-world promotion follow-up:

- used the originally unowned world at record `14` in `ecmaint-post`
- tested four shapes with `+0x60 = 1`:
  - owner only: `+0x5d = 1`
  - owner plus `+0x5c = 2`
  - owner plus minimal visible owned-world fields:
    `+0x0e = 12`, `+0x58 = 10`, `+0x5a = 4`, `+0x5c = 2`, `+0x5d = 1`
  - near-full clone of record `15` with record `14` coords/name preserved

Observed result after `2` maint ticks:

- owner-only / owner+status / owner+minimal-owned-shape cases did **not**
  trigger the full deep rewrite
  - they only changed:
    - `+0x03: 0 -> 0x81`
    - and, when needed, normalized `+0x5c` to `2`
- the near-full clone **did** trigger the familiar deep rewrite shape:
  - `+0x03..+0x0e` broad rewrite
  - same silent-output profile:
    no `RESULTS.DAT`, `MESSAGES.DAT`, or `ERRORS.TXT`

Practical consequence:

- refine the current gate model:
  - `+0x60 != 0` is necessary for the deep same-world path in current probes
  - ownership is also necessary
  - but ownership alone is **not** sufficient
- the branch appears to depend on some richer established-world payload beyond
  the obvious visible bytes `+0x0e`, `+0x58`, `+0x5a`, `+0x5c`, and `+0x5d`
- that makes the still-opaque block around `+0x03..+0x0d` an even stronger
  candidate for the true semantic prerequisite set

Payload bisect over the established-world block `+0x03..+0x0d`:

- starting point:
  the near-full clone of record `15` into record `14` with `+0x60 = 1`
  reproduces the deep same-world rewrite
- bisected the `+0x03..+0x0d` block into lower and upper halves while keeping
  the rest of the cloned owned-world shape intact

Observed result after `2` maint ticks:

- resetting all of `+0x03..+0x0d` back to the unowned record shape collapses
  the effect to a minimal `+0x03: 0 -> 0x81`
- copying only the lower half:
  - `+0x03..+0x08`
  - produces a lower-half-only rewrite on output:
    offsets `+0x03..+0x08` move, but `+0x09..+0x0e` stay inert
- copying only the upper half:
  - `+0x09..+0x0d`
  - produces an upper-half-only rewrite on output:
    offsets `+0x09..+0x0e` move, while the lower half still collapses to the
    minimal `+0x03 -> 0x81` behavior
- copying just byte `+0x09` is already enough to activate the upper-half
  rewrite shape at `+0x09..+0x0e`
- copying `+0x03..+0x09` together reproduces the full broad same-world rewrite

Current practical reading:

- the prerequisite payload is no longer one undifferentiated opaque block
- it has at least two semantically separable sub-blocks:
  - lower sub-block around `+0x03..+0x08`
  - upper sub-block keyed by `+0x09` and the surrounding `+0x09..+0x0d`
- the deep `024d` planet-local path therefore appears to consume two coupled
  world-state numeric groups rather than one single flag family

Practical consequence for Rust step-4 modeling:

- do not treat `+0x03..+0x0d` as one all-or-nothing prerequisite anymore
- current best reduction is:
  - `+0x60` gate
  - owned-world requirement
  - lower numeric prerequisite block `+0x03..+0x08`
  - upper numeric prerequisite block keyed by `+0x09..+0x0d`
- that is enough to justify modeling the deeper `024d` branch as a distinct
  planet-state subphase that consumes two world numeric groups, even though the
  exact semantics of those groups are still unnamed

Mixed order probe: target-world `+0x60` against invasion / bombardment / fleet battle fixtures

Controlled probe:

- took the target world in three preserved pre-maint fixtures and forced
  `planet[+0x60] = 1`
  - `ecmaint-invade-pre`, record `14`
  - `ecmaint-bombard-pre`, record `14`
  - `ecmaint-fleet-battle-pre`, record `14`
- then inspected the target world after each maint tick together with:
  - `RESULTS.DAT`
  - `MESSAGES.DAT`
  - `ERRORS.TXT`

Observed result:

- `invade-pre` target world:
  - tick `1`: target world already took a substantial direct rewrite in
    `+0x03..+0x0e`
  - tick `1`: `RESULTS.DAT` still empty
  - tick `2`: target world rewrote again; `RESULTS.DAT` still empty in this
    forced-byte probe
- `bombard-pre` target world:
  - tick `1`: target world already changed in the same deep block
  - tick `1`: `RESULTS.DAT` still empty
  - tick `2`: deeper rewrite continued and ground batteries changed
  - tick `2`: `RESULTS.DAT` still empty in this forced-byte probe
- `fleet-battle-pre` target world:
  - tick `1`: target world changed in the deep block
  - tick `1`: `RESULTS.DAT` was non-empty the same tick

Current practical ordering implication:

- the `024d`-side planet-local mutator can definitely run in a yearly tick
  **before** the delayed visible mission consequences seen in invasion and
  bombardment fixtures
- so, at minimum, this producer-side planet mutation is not merely a late
  consequence formatter that waits for the same output timing as
  bombard/invade reports
- this is the strongest current evidence that step `4` contains multiple
  subphases inside the same yearly tick, with some silent planet-state work
  landing earlier than at least some later visible combat/mission outcomes

What this still does **not** prove:

- it does not yet place the `024d` family cleanly before all combat logic
- it does not yet prove whether the forced `+0x60` branch is naturally reached
  in those preserved combat fixtures without the direct byte override
- it does not yet distinguish "before combat resolution" from "before visible
  combat/report emission"

#### Reusable step-4 direct-probe harness added

New tool:

- `tools/step4_oracle_probe.py`

Purpose:

- clone a preserved or disposable baseline into a throwaway working directory
- apply direct `PLANETS.DAT` byte edits to chosen records
- snapshot the directory before and after each maint tick
- summarize per-tick file drift across:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `CONQUEST.DAT`
  - `DATABASE.DAT`
  - `RESULTS.DAT`
  - `MESSAGES.DAT`
  - `RANKINGS.TXT`
  - `ERRORS.TXT`
- also summarize watched-planet per-record offset changes after each tick

Practical consequence:

- the direct step-4 workflow is now reproducible and report-aware by default
- this should replace ad hoc one-off probe loops when checking whether a
  candidate state mutation is:
  - silent planet-state work
  - visible report-side work
  - or a mixed case

#### Per-tick shape tightening on the forced `+0x60` branch

Using the new direct-probe harness on:

- `fixtures/ecmaint-invade-pre/v1.5`
- `fixtures/ecmaint-bombard-pre/v1.5`
- `fixtures/ecmaint-fleet-battle-pre/v1.5`

all with forced target-world `record 14, +0x60 = 1` and watching planet record
`14`:

Observed result:

- `invade-pre`, tick `1`:
  - watched world already rewrites broadly at:
    - `+0x03..+0x04`
    - `+0x08..+0x0e`
    - `+0x58`
  - `RESULTS.DAT` still empty
- `bombard-pre`, tick `1`:
  - watched world rewrites more narrowly at:
    - `+0x03..+0x04`
    - `+0x08..+0x09`
    - `+0x0e`
  - `RESULTS.DAT` still empty
- `fleet-battle-pre`, tick `1`:
  - watched world changes again on the same tick that `RESULTS.DAT` becomes
    non-empty
  - but the shape is different from invade/bombard:
    - `+0x03`
    - `+0x08..+0x09`
    - `+0x0e`
    - plus side bytes `+0x38` and `+0x3c`

Current practical reading:

- the earlier ordering result still holds:
  some `024d`-side planet mutation can land before later visible delayed
  mission consequences
- the new detail is that the tick-`1` world-mutation *shape* is not uniform
  across delayed mission and combat-heavy fixtures
- this increases confidence that step `4` is not just one producer pass plus
  one universal report delay:
  mission family and/or neighboring subphase context appears to affect which
  part of the planet-local rewrite has landed by a given yearly tick

#### Control comparison against the same fixtures without forced `+0x60`

Ran the same harness on the preserved control fixtures without any direct
planet-byte edits, still watching target world record `14`:

- `ecmaint-invade-pre`
- `ecmaint-bombard-pre`
- `ecmaint-fleet-battle-pre`

Observed control result:

- `invade-pre`, tick `1`:
  - target world changes only at:
    - `+0x09`
    - `+0x0e`
    - `+0x38`
    - `+0x3c`
  - `RESULTS.DAT` still empty
- `bombard-pre`, ticks `1` and `2`:
  - watched target world stays unchanged
  - `RESULTS.DAT` stays empty
- `fleet-battle-pre`, tick `1`:
  - target world changes at the same four offsets as invade:
    - `+0x09`
    - `+0x0e`
    - `+0x38`
    - `+0x3c`
  - `RESULTS.DAT` is already non-empty

Current practical reading:

- the natural target-world bytes shared by invade and fleet-battle on tick `1`
  are now better bounded as mission/combat-side consequences, not the extra
  forced `+0x60` producer path by itself
- bombard is importantly different:
  the watched target world stays untouched in the preserved control case even
  though other files move and `DATABASE.DAT` / `RANKINGS.TXT` rebuild
- comparing forced vs control now gives a cleaner partition:
  - likely mission/combat-side target-world bytes on tick `1`:
    `+0x09`, `+0x0e`, `+0x38`, `+0x3c`
  - additional producer-side bytes exposed by forced `+0x60` vary by fixture
    but include earlier direct writes into the lower world block
    (`+0x03..+0x08`) and, in stronger cases, more of the upper world block
    (`+0x0a..+0x0d`) plus `+0x58`

Practical consequence for step-4 recovery:

- this is the clearest current separation between:
  - natural mission/combat consequences on the watched target world
  - and extra producer/mutator work that can be induced independently
- that makes the next question narrower:
  determine whether the producer-side lower/upper world-block writes are
  normally scheduled before, after, or only conditionally alongside the
  mission/combat-side `+0x09/+0x0e/+0x38/+0x3c` family

Direct forced-vs-control overlay on the same tick:

- compared the watched target-world record after tick `1` between:
  - forced `+0x60 = 1`
  - preserved control run
- cases checked:
  - `invade-pre`
  - `bombard-pre`
  - `fleet-battle-pre`

Observed overlay result:

- `invade-pre`:
  forced state differs from control at:
  `+0x03`, `+0x04`, `+0x08..+0x0e`, `+0x38`, `+0x3c`, `+0x58`
- `bombard-pre`:
  forced state differs from control at:
  `+0x03`, `+0x04`, `+0x08`, `+0x09`, `+0x0e`
- `fleet-battle-pre`:
  forced state differs from control at:
  `+0x03`, `+0x08`, `+0x09`, `+0x0e`

Current practical reading:

- the forced `+0x60` path is not merely additive on top of the natural
  mission/combat-side target-world pattern
- it can overwrite that natural pattern on the same yearly tick
  - most clearly at `+0x09` and `+0x0e`
  - and, in the invade case, it also suppresses the control-side `+0x38/+0x3c`
    marks while adding broader lower/upper world-block writes plus `+0x58`
- practical implication:
  step `4` should currently be modeled as overlapping neighboring subphases
  that can write some of the same world-state fields, not as isolated
  non-interacting passes

#### Target-world aftermath shape depends on starting world payload

Important fixture fact:

- `invade-pre` and `fleet-battle-pre` start with **identical** target-world
  record `14`
- `bombard-pre` uses a different weaker target-world seed at the same coords

Direct transplant probe:

- transplanted the weaker `bombard-pre` target-world record `14` into:
  - `fleet-battle-pre`
  - `invade-pre`
- then reran the natural control maint ticks with no forced `+0x60`

Observed result:

- original `invade-pre` / `fleet-battle-pre` natural tick-`1` target-world
  shape:
  - `+0x09`
  - `+0x0e`
  - `+0x38`
  - `+0x3c`
- after transplanting the weaker bombard-style target-world seed into those
  same scenarios, the natural tick-`1` shape collapses to:
  - `+0x09`
  - `+0x0e`
  - `+0x58`
- in both transplanted cases:
  - `+0x38/+0x3c` no longer appear
  - `invade` still keeps `RESULTS.DAT` empty on tick `1`
  - `fleet-battle` still keeps `RESULTS.DAT` non-empty on tick `1`

Current practical reading:

- the natural target-world aftermath shape is not determined by mission family
  alone
- it depends strongly on the starting world payload/class
- the shared `invade`/`fleet-battle` `+0x38/+0x3c` marks are therefore better
  read as target-world-state-dependent aftermath fields, not generic "hostile
  event happened" markers
- `+0x09` and `+0x0e` look more stable across target-world seed classes
- `+0x58` now looks like the fallback/alternate world-side consequence when
  the weaker bombard-style seed is used

Practical consequence for Rust step-4 modeling:

- target-world aftermath should not be keyed only by mission family
- it likely depends on both:
  - the surrounding hostile-resolution context
  - and the target world's current payload/class

Inverse transplant check on `bombard-pre`:

- transplanted the stronger `invade` / `fleet-battle` target-world seed into
  `bombard-pre`
- reran the natural `bombard` control probe for `2` ticks with no forced
  `+0x60`

Observed result:

- the watched target world still stayed unchanged on both ticks
- `RESULTS.DAT` stayed empty, just like preserved `bombard-pre`

Current practical reading:

- stronger target-world payload alone is **not** enough to produce the natural
  `invade` / `fleet-battle` target-world aftermath shape inside the bombard
  context
- combining this with the earlier transplant result gives the cleanest current
  rule:
  natural target-world aftermath depends on **both**
  - hostile-resolution context
  - and target-world payload/class
- neither side alone is sufficient

#### Timing-flow follow-up: explicit late weekly loop plus per-entry offset helper

Rechecked:

- `artifacts/ghidra/ecmaint-live/timing-flow.txt`
- `artifacts/ghidra/ecmaint-live/late-report-pipeline.txt`
- direct probe `artifacts/ghidra/ecmaint-live/probe-1000_a26e.txt`

Current strongest static timing gain:

- `timing-flow.txt` itself remains mostly negative evidence:
  - `2000:945b` is still best treated as shared current-date/status text
    formatting in the token/schedule path
  - it is **not** the recovered player-report `Stardate` formatter
- but the already-recovered late weekly/report loop at
  `0000:12ef..1369` now has a more useful timing-side helper:
  `0000:1339 -> 1000:a26e`

What `1000:a26e` concretely does:

- iterates a local table of `0x0a`-byte entries via count at stack slot
  `+0xfe58` and pointer at stack slot `+0xfe64`
- reads a small code byte from each entry at offset `-0x0a`
- updates a local 32-bit accumulator at stack slots `-0x18/-0x16`
- applies fixed code-dependent offsets:
  - code `1` -> `+2`
  - code `2` -> `+7`
  - code `3` -> `+0x15`
  - code `8` -> `+0x1e`
  - codes `4`, `5`, `6`, `7` -> `+0`
- also clamps two local byte fields as it goes:
  - one at `+0xfe1c` toward values like `0x0a`, `0x0f`, `0x14`, `0x19`
  - one at `-0x10` toward values like `6`, `5`, `4`, `3`, `1`

Current practical reading:

- this is the first strong static candidate for real week-bucket assignment or
  report-timing window shaping inside the late weekly loop
- it is more promising than `2000:945b` because it is:
  - called from the explicit summary-walk loop
  - driven by decoded per-entry codes
  - and mutates a local accumulator through fixed timing-like offsets
- cautious interpretation:
  do **not** call the exact semantics settled yet, but treat `1000:a26e` as a
  leading candidate for the mapping from summary entry class to week-offset /
  timing bucket constraints

Best next timing-side target:

- recover where the `0x0a`-byte local table consumed by `1000:a26e` is built
- identify what the code byte at entry offset `-0x0a` actually classifies
- relate those codes back to observed fleet/report transitions in preserved
  logs and fixture probes

#### Timing follow-up: late weekly path is now a three-stage selector

Additional direct probes:

- `artifacts/ghidra/ecmaint-live/probe-1000_9fa1.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_c103.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_9c0e.txt`
- `artifacts/ghidra/ecmaint-live/probe-1000_c102.txt`

New structural gain:

- `1000:a26e` is not a standalone helper after all:
  - it is an interior address inside `1000:9fa1`
  - `0000:1339` calls that mid-function entry directly from the explicit late
    weekly loop
- the same timing worker family is also consumed by `1000:c102`:
  - `1000:c103` calls `1000:9fa1` at the normal function entry
  - `1000:c102` then immediately calls `1000:9c0e` twice with selector args
    `2` then `1`

What the late weekly side now looks like:

- `0000:02c0` decodes a kind-`1` summary entry:
  - it runs only when `entry[+0x04] == 1`
  - decodes through `2000:c067`
  - seeds large stack-resident local state
- `1000:9fa1 / 1000:a26e` then computes timing windows from a local `0x0a`
  code table:
  - one accumulator family at `-0x14/-0x12` with byte bound `-0xf`
  - another at `-0x18/-0x16` with byte bound `-0x10`
  - code classes still map to fixed offsets:
    - `1 -> +2`
    - `2 -> +7`
    - `3 -> +0x15`
    - `8 -> +0x1e`
    - `4..7 -> +0`
- `1000:c102` is now the strongest candidate for the actual week-placement
  decision layer:
  - it consumes the timing-window results from `9fa1`
  - runs `9c0e(arg=2)` then `9c0e(arg=1)`
  - conditionally adds or subtracts the byte bounds into a running local
    score at `-0x6/-0x4`
  - compares that score against the current timing-window floors/ceilings at
    `0xfe1c`, `0xfe1d`, `-0xf`, and `-0x10`
  - raises flag `0xfe34 = 1` when the current week candidate violates those
    constraints

What `1000:9c0e` adds:

- it is not simple formatting or decoding
- it selects between two timing-window families depending on arg `1` vs `2`
- for nontrivial mode bytes it performs numeric compare/helper calls through
  `3000:4891`, `3000:4883`, and `3000:488d`
- current best practical reading:
  `9c0e` is a timing-window comparator/classifier used by `c102` to decide
  whether the current weekly candidate is too early, too late, or acceptable

Current practical timing model:

- late weekly scheduling is now best treated as a layered selector, not one
  flat "offset lookup":
  1. decode summary entry into local timing state (`0000:02c0`)
  2. derive timing-window candidates from per-entry timing codes
     (`1000:9fa1 / 1000:a26e`)
  3. score/test the current weekly slot against those windows
     (`1000:c102 / 1000:9c0e`)
- this is stronger evidence for a real weekly placement mechanism than the
  earlier `a26e`-only read
- the remaining missing piece is still the code-table builder and semantic
  meaning of those code bytes, not whether the scheduler has explicit
  acceptance / rejection logic

#### Timing follow-up: local code-1 writer recovered, code-2 demoted

Additional direct probes:

- `artifacts/ghidra/ecmaint-live/probe-0000_f1ba.txt`
- `artifacts/ghidra/ecmaint-live/probe-0000_f914.txt`

Current timing-side refinement:

- `0000:f1ba` is now a recovered scratch-local timing-entry initializer:
  - it writes only `0` or `1` into the local timing-entry code byte
  - it also seeds companion local flags at offsets `-0x09`, `-0x08`, and
    `-0x07`
- `0000:f914` is now a recovered late tally pass over the live timing-entry
  table at `0x5c8`:
  - it counts codes `1..7` into scratch counters rooted at
    `352c/352a/3528/3534/352e/3530/3532`
  - it then hands scratch block `3502` to `2000:ba44`
- whole-image timing-entry write scans still show no preserved ES-side writer
  for code `2`
  - consumer-side helpers still recognize code `2`
  - but the only explicit local code-byte writer recovered so far is
    `0000:f1ba`, and it writes only `0/1`

Practical consequence:

- the old timing open item "what exact semantic families feed local codes `1`
  and `2`" is no longer the right Rust-facing question
- code `1` is now a real scratch-local timing class in the preserved image,
  even if its historical label is still fuzzy
- code `2` is now better treated as an unfed/reserved consumer-side slot until
  contrary evidence appears
- the remaining implementation-relevant timing question is narrower:
  exact mapping from concrete movement/combat/admin report families onto weeks
  within the internal `1..52` scheduler

Focused shipped-log follow-up:

- preserved focused extract:
  - `artifacts/ec-report-transition-focus.txt`
  - `artifacts/ec-report-transition-splits.txt`
  - `artifacts/ec-report-cadence-focus.txt`
- newly strong concrete placement constraints:
  - `sensor-contact -> identified` is same-week in all focused corpus cases
    (`48x`)
  - `identified -> intercepted` is same-week where directly chained (`3x`)
  - direct same-source variable-gap families are now better bounded:
    - `entered-system -> attacked`: gaps `0/1`
    - `identified -> orbit-world`: gaps `0/1/4`
      - the preserved `0`-gap cases are all week `1`
    - `orbit-world -> sensor-contact`: wide periodic gaps
      `1/2/3/5/8/10/12/14/16/26/28/36`
    - `attacked -> bombing-run`: gaps `0/5/6/7`
      - the preserved `0`-gap case is week `1`
  - several loss/admin chains are now better classified as cross-source weekly
    interleaving rather than same-source mission progression:
    - `identified -> fleet-lost`: same-week cross-source adjacency in `4x`,
      with one later outlier at gap `4`
    - `attacked -> fleet-lost`: next-week cross-source adjacency in `2x`
    - `fleet-lost -> join-retarget`: same-week cross-source adjacency in `2x`
    - `fleet-lost -> planet-bombarded`: same-week cross-source adjacency in
      `4x`, with delayed variants at gaps `3` and `16`
- practical consequence:
  - the remaining Rust-facing timing question is narrower again:
    exact week placement of the remaining direct same-source variable-gap /
    periodic families, not whether the yearly scheduler contains strong
    same-week bundles at all and not whether the cross-source loss/admin pairs
    hide one missing fixed delay rule

Final timing-family cadence follow-up:

- `artifacts/ec-report-cadence-focus.txt` now captures the strongest compact
  evidence for the remaining direct same-source families
- multi-year extended-orbit carry is now clear in the shipped log corpus:
  fleets that remain in `extended orbit` commonly emit `orbit-world` again at
  week `1` of later years
  - this closes the best Rust-facing interpretation of the preserved
    zero-gap `identified -> orbit-world` cases:
    they are round-start carry inside the standing extended-orbit family, not
    evidence of an arbitrary universal same-week follow-on delay
- the wide `orbit-world -> sensor-contact` gaps are likewise now better read as
  independent hostile traffic/detection while the standing orbit status
  persists, not as one hidden orbit countdown
- bombardment continuation is also better bounded now:
  - `attacked -> bombing-run` appears at gaps `0/5/6/7`
  - a preserved `intercepted -> bombing-run` case appears at gap `6`
  - practical reading:
    hostile encounter during standing bombardment can be followed by later
    `bombing-run` continuation without one single global offset and without
    tying the family only to the literal `attacked` wording
- practical consequence:
  - no remaining timing questions in this thread block Rust implementation
  - the Rust-facing closure is now:
    - `entered-system -> attacked` belongs to shared-stream arrival/contact
      behavior, not one fixed delay
    - `identified -> orbit-world` and `orbit-world -> sensor-contact` belong
      to the standing extended-orbit family
    - hostile encounter during bombardment can be followed by continued
      bombardment without one single fixed delay rule
  - what remains open on the timing side is now low-value historical/static
    trivia such as exact formatter helpers or the historical label behind
    scratch-local code `1`

#### Top-down step-4 follow-up: earlier-driver target tightened, file-I/O trace added

Continued the step-4 recovery thread from the new top-down premise.

Current corrective conclusion:

- the direction shift is right, but the old "trace between `6d9b` and `861d`"
  framing is not:
  - `6d9b` remains restore/validation scaffolding
  - `861d` remains the already-bounded late output tail
  - the missing middle ordering is still more likely earlier than `861d` or in
    helpers that feed that tail

New coarse dynamic signal from DOSBox-X file-I/O logging on classic
`fixtures/ecmaint-bombard-pre/v1.5`:

- first-write order:
  - `FLEETS.DAT`
  - `DATABASE.DAT`
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `CONQUEST.DAT`
  - `RANKINGS.TXT`
- the write clustering is especially one-sided:
  - `FLEETS.DAT` writes start at event `511`
  - the next first write, `DATABASE.DAT`, does not occur until event `3910`
  - practical implication:
    the run clearly has a long earlier state-mutation block dominated by fleet
    writes before the later rebuild/flush family begins

Important limitation:

- this file-I/O trace is **not** proof of exact movement vs economy vs combat
  ordering inside step `4`
- it is only broad phase-boundary evidence:
  - heavy fleet-state mutation first
  - derived-file / report-output rebuild later

Debugger-capture follow-up:

- updated `tools/capture_ecmaint_sim_driver_trace.py` to:
  - compare hits by linear address instead of raw segment:offset
  - retarget breakpoints toward earlier startup/token seams:
    - `9e1e`
    - `9cb0`
    - `731f`
    - `6d9b`
    - `5ee4`
    - `00e8`
    - `024d`
    - plus `861d` only as an upper fence
- staged bridge result is now tighter:
  - `INT 21h / AH=3D` still works as the first reliable "loaded image is now
    present" stop
  - after that stop, arming `2814:96c4` again works as a clean post-load bridge
    into the unpacked ECMAINT image
  - the live stop surfaced at normalized `CS:EIP = 3159:0274`, which matches
    the same linear address as `2814:96c4`
- current DOSBox debugger limitation is now narrower and more concrete:
  - arming the full earlier-driver breakpoint set only after the first
    file-open stop is still too late for those seams
  - arming that same full set immediately at debugger startup still misses the
    loaded image and falls straight through to exit
  - but the two-stage bridge `first file-open -> 96c4` is now confirmed
  - the remaining problem is specifically the larger post-bridge breakpoint set,
    which still destabilizes or stalls the run before a clean next stop is
    captured

Practical next step:

- keep the top-down plan, but use the confirmed `first file-open -> 96c4`
  bridge as the staging seam for the next dynamic pass
- next dynamic work should bisect which post-bridge startup / driver
  breakpoints are individually safe to catch before trying to trace the full
  earlier-driver set in one run
