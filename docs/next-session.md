# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`. 

**Headless Ghidra (Ready):**
- `tools/ghidra_ecmaint.sh` imports and analyzes `original/v1.5/ECMAINT.EXE` headlessly.
- Repo-local Ghidra state lives under `.ghidra/`; logs live under `artifacts/ghidra/ecmaint/`.
- Current confirmed baseline:
  - Loader: old-style DOS `MZ`
  - Language: `x86:LE:16:Real Mode:default`
  - MD5: `21489ef9798df77b20b7a02eb9347071`
- Important limitation:
  - `ECMAINT.EXE` is still LZEXE-packed, so current Ghidra output only sees the loader stub.
  - The first post-script pass produced only one recovered function (`entry`) and no useful strings.

**ECMAINT File-I/O Trace (New):**
- Initial runtime load order:
  - `CONQUEST.DAT`
  - `SETUP.DAT`
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `IPBM.DAT`
- Trace-backed runtime record sizes:
  - `PLAYER.DAT` = `4 x 110`
  - `PLANETS.DAT` = `20 x 97`
  - `FLEETS.DAT` = `16 x 54`
  - `BASES.DAT` = `35`-byte records
- This runtime evidence supersedes the older `PLAYER.DAT = 5 x 88` guess.

**Starbase 2 Integrity Gate (New):**
- The failing multi-starbase test case aborts immediately after the initial
  read sweep of `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `BASES.DAT`.
- It writes only to `ERRORS.TXT`; it does **not** reach the normal maintenance
  writeback/report pipeline.
- `ERRORS.TXT` reports:
  - `Game file(s) missing or failed integrity check!`
  - `Attempting to restore game from last saved point...`
  - `Backup game file(s) missing or failed integrity check`
  - `Maintenance aborting...`
  - `Unable to restore previous game - maintenance aborting`
- Practical conclusion:
  - the Starbase 2 blocker is a **front-loaded cross-file integrity validator**,
    not a late Guard Starbase resolution branch.
- Trace comparison artifact now exists:
  - script: `tools/compare_ecmaint_validation_trace.py`
  - outputs: `artifacts/ecmaint-validation-trace/`
  - key divergence against the known-good Guard Starbase fixture:
    - good case seeks `BASES.DAT` to offset `0`
    - failing raw Starbase 2 case seeks `BASES.DAT` to offset `35`
    - both then read `35` bytes, after which:
      - good case proceeds into `IPBM.DAT` / `DATABASE.DAT`
      - bad case opens and writes `ERRORS.TXT`
  - practical meaning:
    - the bad path is not diverging after a generic one-base `BASES` read
    - it is already selecting the **second** 35-byte base record before the
      integrity abort path
  - token-trace caveat from the new three-case runner:
    - `artifacts/ecmaint-validation-trace/bad-player-tok.log` shows that a
      recognized token case does **not** fall directly into the same core trace
      shape under full debug logging
    - instead it creates/deletes synthetic `*.tok` files for multiple game
      files, then enters a poll loop on the recognized token name
      (`Player.Tok` in the current run) until the time limit kills DOSBox-X
    - practical meaning:
      - token-gated runs are not comparable to the no-token validator trace
        unless we also model or break past that wait loop
      - this directly supports the live-dump/string evidence that token
        handling is active control flow, not a passive file-exists check
  - controlled host-side token release probe now exists:
    - script: `tools/test_ecmaint_token_release.py`
    - logs: `artifacts/ecmaint-token-release/`
    - observed behavior:
      - if `PLAYER.TOK` is deleted **before** ECMAINT reaches the check, it
        simply recreates `Player.Tok` and continues
      - if `PLAYER.TOK` is deleted **during** the repeated `Player.Tok` search
        loop, ECMAINT eventually creates `Player.Tok`, then resumes normal file
        access
      - after release, it still follows the same failing validator path:
        `PLAYER.DAT` read sweep -> `BASES.DAT` seek `35` -> `ERRORS.TXT`
  - practical meaning:
    - the token wait loop is a real synchronization gate
    - passing Starbase 2 behavior requires more than merely releasing the wait
      loop; there is at least one additional token-path side effect or mode
      switch beyond that rendezvous
  - debug-trace timing caveat is now confirmed:
    - a long `PLAYER.TOK` run with full `-debug -log-int21 -log-fileio`
      (`artifacts/ecmaint-token-release/player-tok-debug-long.log`) does **not**
      reproduce the earlier successful black-box `PLAYER.TOK` result
    - instead ECMAINT emits `ERRORS.TXT` beginning with:
      `03-11-2026 6:43pm -> Timeout error for tokenfile Player.Tok`
    - practical meaning:
      - the heavy DOSBox-X debug/file-I/O logging perturbs the token path enough
        to trigger ECMAINT's own token timeout behavior
      - token-path RE should avoid assuming that full debug logging preserves
        original timing semantics
  - low-overhead token matrix now exists:
    - script: `tools/test_ecmaint_token_matrix.py`
    - artifact: `artifacts/ecmaint-token-matrix.txt`
    - accepted names do **not** all normalize to one identical token residue:
      - `MAIN.TOK` => `OK`, leaves only `MAIN.TOK`
      - `PLAYER.TOK` => `OK`, leaves `FLEETS.TOK`, `MAIN.TOK`,
        `PLANETS.TOK`, `PLAYER.TOK`
      - `PLANETS.TOK` => `OK`, leaves `MAIN.TOK`, `PLANETS.TOK`
      - `FLEETS.TOK` => `OK`, leaves `FLEETS.TOK`, `MAIN.TOK`,
        `PLANETS.TOK`
      - `CONQUEST.TOK` => `OK`, leaves only `CONQUEST.TOK`
      - `DATABASE.TOK` => `OK`, leaves the full expanded token set
      - unrecognized `FOO.TOK` and no token both fail and also leave the full
        expanded token set
  - practical meaning:
    - recognized token names are not interchangeable aliases for one boolean
      “token mode”
    - they appear to enter different submodes or cleanup horizons inside the
      same token-management machinery
  - static token-anchor follow-up now exists:
    - script: `tools/ghidra_scripts/ECMaintTokenAnchors.java`
    - report: `artifacts/ghidra/ecmaint-live/token-anchors.txt`
    - saved project label/function:
      - `2000:9b13` => `ecmaint_token_wait_timeout_helper`
    - first concrete static inference:
      - the generic wait/delete string cluster at `2000:9680` sits immediately
        before `ecmaint_token_wait_timeout_helper`
      - that helper begins by calling `0x3000:39dc`, capturing a timestamp, and
        subtracting it from globals at `0x34fa:0x34fc`, which is consistent
        with timeout-loop bookkeeping
    - deeper flow report now exists:
      - script: `tools/ghidra_scripts/ECMaintTokenFlowReport.java`
      - report: `artifacts/ghidra/ecmaint-live/token-flow.txt`
      - additional saved names:
        - `2000:9e1e` => `ecmaint_wait_for_named_token_candidate`
        - `3000:39dc` => `ecmaint_time_query_helper_candidate`
      - first control-flow recovery:
        - `ecmaint_wait_for_named_token_candidate` calls the time helper,
          stores the result to globals `0x34fa:0x34fc`, clears `0x2f76`,
          pushes literal `0xFA00`, and calls
          `ecmaint_token_wait_timeout_helper`
        - on return it stores `AX:DX` into globals `0x2f72:0x2f74`
        - inside `ecmaint_token_wait_timeout_helper`, the timeout path begins
          around `0x9b82`, loading DS:`46cc` and CS:`0653` before a cluster of
          far calls consistent with formatting/emitting the timeout message
    - reachability/carve follow-up now exists:
      - script: `tools/ghidra_scripts/ECMaintTokenReachabilityReport.java`
      - report: `artifacts/ghidra/ecmaint-live/token-reachability.txt`
      - new saved label:
        - `2000:9d48` => `ecmaint_move_tok_recovery_candidate`
      - recovered adjacent token-management block:
        - `2000:9d48..0x9e1d` is a real `Move.Tok` recovery/delete path sitting
          immediately before `ecmaint_wait_for_named_token_candidate`
        - it reuses the same DS:`46cc` message-buffer pattern and emits
          message strings via CS offsets `0x877`, `0x8b1`, and `0x8db`
        - those `CALL 0x945b` sites are now explained:
          `2000:945b` is a reusable timestamp-message formatter that writes a
          date/time string into DS:`2b26` and emits it through the standard
          message/output helpers
        - the tiny prelude before that block is no longer opaque:
          - `2000:9c91` => `ecmaint_move_tok_check_wrapper_candidate`
          - `2000:9cb0` => `ecmaint_move_tok_delete_wrapper_candidate`
          - `2000:9c91` passes counted-string `Move.Tok` to
            `2000:96c4 ecmaint_check_named_token_candidate` and returns `AL`
          - `2000:9cb0` passes counted-string `Move.Tok` to
            labeled anchor `2000:9887 ecmaint_delete_named_token_candidate`
        - first generic-helper interpretation:
          - `ecmaint_check_named_token_candidate` is a real saved function
          - it canonicalizes the supplied token name into local buffers, uses
            the same `0x3000:4f4c` helper seen elsewhere in token handling, and
            returns a byte status in `AL`
          - `ecmaint_delete_named_token_candidate` is still only a label because
            the raw import does not cleanly carve that start, but the `Move.Tok`
            delete wrapper calls it directly at `2000:9cb9`
        - Ghidra still refuses to promote `0x9d48` to a clean function because
          this area is interleaved with nearby counted token strings/data, so
          only the label is currently saved there
      - important caveat:
        - the raw-binary import cannot treat DS:`46cc` / CS:`0653` as reliable
          linear string addresses; those are runtime segment-relative operands,
          and the imported bytes at those linear offsets decode as code, not the
          final timeout text
    - reproducible dynamic-debug prep now exists:
      - script: `tools/prepare_ecmaint_token_debug_case.py`
      - target: `/tmp/ecmaint-debug-token`
      - contents: raw two-base Starbase 2 repro plus zero-length `PLAYER.TOK`
      - use this as the default debugger scenario when breaking on
        `2000:96c4`, `2000:9cb9`, and `2000:9e1e`
    - first live-debugger mapping result:
      - the DOSBox-X debugger can stop reliably at the first real
        `INT 21h / AH=3Dh` file-open with:
        - `BPINT 21 3D`
        - `RUN`
      - at that first stop, live registers were:
        - `CS=3374 EIP=1880 DS=39AB ES=39AB SS=39AB`
        - `SP=F7F4 BP=F7F6 AX=3D02 BX=F7F6 CX=0000 DX=F832 SI=000D DI=F802`
      - `DOS MCBS` at the same stop still shows the unpacked `ECMAINT` block at
        PSP `0814`, size `622256` bytes
      - practical debugger consequence:
        - raw-import code addresses at segment `2000:` are **not** the live
          debugger code segment
        - the working translation for this dump is PSP-relative:
          `2000:xxxx -> 2814:xxxx` using PSP `0814`
        - use live breakpoints like `2814:96c4`, `2814:9cb9`, and `2814:9e1e`,
          not `2000:...`
        - DOSBox-X may report the same stop under a different normalized
          segment:offset pair; for example `BP 2814:96c4` stopped at
          `3159:0274`
      - remaining caveat:
        - under the token-wait path, later code-break stops still do not surface
          cleanly in the headless TTY debugger transcript, so the next pass
          should assume the address translation is solved but the stop/capture
          method still needs refinement
      - latest dynamic repro:
        - `BP 2814:96c4` hits cleanly first and DOSBox-X reports the stop as
          `3159:0274`
        - after deleting the `96c4` breakpoint and continuing with only
          `2814:9cb9` and `2814:9e1e` armed, the headless debugger falls into
          repeated `Illegal Unhandled Interrupt Called 6` logging before either
          later breakpoint surfaces
        - disk side effects at that point are minimal: no `ERRORS.TXT`, no
          extra `*.TOK`, and the original `PLAYER.TOK` still exists
        - practical conclusion: the current blocker is now a debugger/runtime
          interaction after the generic helper, not bad breakpoint translation
      - new caller recovery from the stable `96c4` stop:
        - By parsing the live memory dump offline we identified the exact stack return addresses when breaking inside the token validation helper (`2814:96c4` / `3159:0274`).
        - The validation for `setup.tok` originates from a `CALL FAR` at `2000:7333` (return IP `0x6b28`).
        - The caller passes the string pointer via `CS:DI` (e.g. `2000:6a22` points to `\x09setup.tok`).
        - The master loop that iterates and checks the remaining game `.tok` files (`Player.Tok`, `Fleets.Tok`, `Database.Tok`, etc.) is located at `2000:997C`.
        - This master loop cleverly fakes a `FAR` call from a `NEAR` call within the same segment to masquerade its parameter passing and cross-file integrity invocation: `MOV DI, 0x4e5; PUSH CS; PUSH DI; PUSH CS; CALL NEAR ...`

**ECMAINT Live Dump (New):**
- The productive path is now a DOSBox-X debugger memory dump, not more blind
  packer guessing.
- Working breakpoint recipe:
  - launch with `DEBUGBOX ECMAINT /R`
  - set `BPINT 21 3D`
  - when it breaks on the first file open, run `DOS MCBS`
  - dump the live block with `MEMDUMPBIN 0814:0000 97EB0`
- Confirmed dump file:
  - `/tmp/ecmaint-debug/MEMDUMP.BIN`
- Best current anchors inside the live image:
  - `0x26B86..0x26D97`: backup/primary filename tables and integrity strings
  - `0x26D98`: likely integrity/restore procedure start
  - `0x2841B..0x284E5`: `main.tok` startup guard strings including
    `Performing integrity check of game files...`
- Raw-binary Ghidra import of the dump also works:
  - project: `.ghidra/projects/ecmaint-live`
  - recovered functions: `280`
  - Ghidra anchor addresses:
    - `2000:6d98` for the integrity cluster
    - `2000:841b` for the `main.tok` startup-guard cluster
  - headless label/carve script now exists:
    - `tools/ghidra_scripts/ECMaintNameIntegrityAnchors.java`
    - output report: `artifacts/ghidra/ecmaint-live/integrity-anchors.txt`
- First manual disassembly result:
  - linear `0x26D9B` is now saved in the project as
    `ecmaint_integrity_restore_entry`
  - `[bp+4] = 0` validates the primary state
  - on failure it recursively calls itself with argument `1` for the
    backup/restore-side path
  - helper `0x25EE4` is now saved as `ecmaint_validate_primary_state` and
    immediately checks structures matching `PLAYER.DAT` (`110` bytes),
    `PLANETS.DAT` (`97` bytes), and `FLEETS.DAT` (`54` bytes)
  - the next phase inside `0x25EE4` reads `BASES.DAT` (`35` bytes) using
    `PLAYER.DAT[0x44]` as the base-record selector
  - after loading that base record, it compares one base byte against the
    current player index before accepting the relation
  - targeted repro in `tools/test_starbase2_baseid_gate.py` confirms the key
    byte is `BASES[0x04]`:
    - base 2 with `0x04 = 0x02` => integrity abort
    - base 2 with `0x04 = 0x01` => accepted and normalized back to one base
    - changing duplicate-record slot byte `BASES[0x00]` does not affect this
      result
    - changing `BASES[0x02]` to `0x02` with `BASES[0x04] = 0x01` does not hit
      the same integrity gate; it instead produces `Fleet assigned to an
      unknown starbase`
  - helper `0x25EE4` then follows an additional base-to-base selector through
    loaded base-buffer word `0x05..0x06`
  - targeted repro in `tools/test_starbase_link_gate.py` shows
    `BASES[0x05..0x06]` behaves like a little-endian selector:
    - `0000` and `0001` are tolerated in the duplicate-base normalization case
    - `0100` and `0101` hit the early integrity abort
    - `0002` is bad enough to produce `Unable to allocate memory.`
  - there is now a stable accepted two-record `BASES` state:
    - base 1 `0x08 = 0`
    - base 2 `0x00 = 2`, `0x02 = 1`, `0x04 = 1`, `0x05..0x06 = 0001`,
      `0x07 = 1`, `0x08 = 0`
    - post-maint `PLAYER[0x44..0x47]` stays `02000200`
    - `BASES.DAT` stays at two records across a second pass
  - caveat: this is a stable duplicated-base state, not yet a true accepted
    `BASES[0x04] = 0x02` second-base identity
  - promotion attempts from that stable state still fail when base 2 advances
    to `BASES[0x04] = 0x02`, even after varying nearby fields `0x02` and
    `0x05..0x06`
  - corrected finding: the real missing precondition is not another local base
    field; it is the presence of a recognized zero-length `*.TOK` marker file
  - `tools/test_starbase2_tok_gate.py` shows:
    - no token file => raw `Starbase 2` attempt still fails integrity
    - `MAIN.TOK` alone => same raw `Starbase 2` attempt succeeds
    - `PLAYER.TOK` alone => also succeeds
    - arbitrary `FOO.TOK` => still fails, so this is not just any `.TOK` name
  - the accepted `MAIN.TOK` / `PLAYER.TOK` cases survive a second maintenance
    pass, keeping both base records and `BASES[0x04] = 0x02` on the second one
  - live-dump token anchors:
    - `main.tok` + startup guard strings at linear `0x2841B`
    - `conquest.tok` + token-deletion strings at linear `0x26FC6`
    - generic token wait/delete strings around `0x29680`
  - after that, it enters a separate secondary phase driven by
    `PLAYER.DAT[0x48]` reading DS:`31F8` records of size `0x20`
  - direct repro in `tools/test_player48_gate.py` shows this is the `IPBM.DAT`
    count gate, not a starbase companion structure:
    - `PLAYER[0x48] = n` requires `IPBM.DAT` length `n * 0x20`
    - mismatched counts trigger the same early integrity error

**Movement math (Recovered):**
- Distance moved per pass = `speed / 1.5` (approximate, with turn-based rounding).
- Observed pattern for Speed 3: Turn 1 (+2), Turn 2 (+3), Turn 3 (+3).
- Observed pattern for Speed 1: Turn 1 (+1), Turn 2 (+0), Turn 3 (+1).

**Starbase Guard Order (Definitive):**
- `FLEETS.DAT[0x22]` = empire-relative starbase index.
- `FLEETS.DAT[0x23]` = must be `0x01` for resolution.
- **Auto-merge**: multiple fleets guarding the same base merge automatically.
- `PLAYER.DAT[0x46..0x47]` is **not required as a precondition** for Guard Starbase resolution and is **not specific to order `0x04`**; it normalizes to `0x0001` when ECMAINT sees a valid starbase state for the empire.
- `BASES.DAT[0x04]` behaves like the real starbase identity/number; promoting it to `0x02` is what triggers the multi-starbase integrity gate, while changing only `BASES.DAT[0x00]` is not enough.

**Rogue Empires (Confirmed):**
- `PLAYER.DAT[0x00] = 0xFF`.
- **Auto-merge**: all rogue fleets consolidate at the homeworld into one fleet.
- Order forced to `0x05` (Guard/Blockade), ROE forced to `10`.

**Planet Owner Field (Confirmed):**
- `PLANETS.DAT[0x5D]`: owner empire number (1-indexed).

## Next Steps

1. **Compare early validation traces**: run a known-good Guard Starbase baseline and diff its initial read/validation phase against the failing Starbase 2 scenario.
2. **Reverse the token gate side-effects (Partially Solved)**: Statically analyzed `2000:997C` and `2000:96C4`. Neither directly sets a global token bypass flag. However, analyzing the Starbase 2 integrity check at `2000:5EE4` revealed that it relies on a bypass flag at `DS:16A4`. A binary-wide scan shows `16A4` is never explicitly set to `1` by direct instructions, meaning it is set via indirect memory writes, string operations, or during command line initialization. The next step is to dynamically track memory writes to `DS:16A4` using a DOSBox-X debug script to capture the instruction that puts ECMAINT into "bypass mode".
3. **IPBM resolution**: investigate planetary bombardment missiles — still untouched in preserved fixtures, and `IPBM.DAT` is currently 0 bytes in all repo fixture families.
4. **Build queue mechanics (Partially Solved)**: When a build order finishes, the newly constructed ships are moved into the planet's **Stardock** (`PLANETS.DAT[0x38]` and `0x4C`). They do not immediately form a fleet in `FLEETS.DAT` until they are manually "Commissioned" by the player. We need to map out exactly how `0x38` and `0x4C` encode multiple ships/types.

## Standard Runtime Command

See `docs/dosbox-workflow.md`.
