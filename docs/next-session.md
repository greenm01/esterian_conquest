## Accomplished
- Completed the reverse engineering of the token gate mechanism and the `16A4` integrity bypass flag.
- Exhaustively proved that `DS:16A4` is never set to 1 due to a likely developer typo (command line `/B` sets `16A2`, but the integrity check tests `16A4`).
- Discovered the true reason `.TOK` files "bypass" the crash: the presence of `Move.Tok` triggers an automatic restore of `.SAV` backups over the `.DAT` files prior to the integrity check, causing the repaired files to pass naturally.
- Documented findings in `token-investigation.md`.

## Next Steps
The token-gate investigation is complete. The next work should return to the
main maintenance engine and focus on the remaining port-critical unknowns.

Primary project goal:

- reverse engineer enough of the original formats and engine rules to generate
  100% compliant gamestate files from Rust
- treat the original game and `ECMAINT` as the acceptance oracle for that
  milestone
- use that milestone as the first concrete step toward a full Rust port

1. `2000:5EE4` / integrity validator completion
   - Fully map the accepted and rejected structure rules in the early
     cross-file validator, especially the `PLAYER -> BASES -> FLEETS` path.
   - Treat the token question as closed; the remaining task here is the actual
     validation logic, not more `.TOK` experiments.
   - Save additional function names and branch notes in the live Ghidra project
     as the validator becomes clearer.
   - Current correction:
     - artifact: `artifacts/ghidra/ecmaint-live/5ee4-fleet-branch.txt`
     - script: `tools/ghidra_scripts_tmp/Report5EE4FleetBranch.java`
     - confirmed:
       - `2000:6040..6368` is the `FLEETS.DAT` validator branch, not the
         direct `BASES.DAT` loader
       - it opens stream `0x3178` with record size `0x36` and copies the
         active fleet record into local scratch at `[BP+0xFF3E]`
       - it emits kind-`1` summary entries through `0x2F72` / `0x2F76`
       - the early synthetic two-base integrity abort is therefore distinct
         from the later `Fleet assigned to an unknown starbase` behavior
   - Immediate next target:
     - reverse the downstream kind-`1` summary path rooted at `0000:02ED`
       and helper `2000:C067`
     - goal: map scratch block `3502` back to fleet record offsets and locate
       the exact later starbase-resolution rule that produces the
       `unknown starbase` error
   - Current kind-`1` follow-up:
     - artifact: `artifacts/ghidra/ecmaint-live/kind1-scratch-function.txt`
     - script: `tools/ghidra_scripts_tmp/ReportKind1ScratchFunction.java`
     - confirmed:
       - `0000:02ED..03D5` is the kind-`1` mirror of the generic summary
         loader, using scratch block `0x3502`
       - it consumes:
         - `350D`, `350F..3513`
         - `350E`, `3515..3519`
         - `3522`, `3523`
         - `351B..351F`
         - capped byte `3524`
         - selector/count byte `350C`
       - in the initial kind-`1` load path, the only explicit summary input
         passed to the `3502` loader is `ES:[DI+0x06]`
       - summary bytes `+0x01` / `+0x02` emitted by the fleet branch are not
         read there; they are overwritten later by the shared canonicalization
         stage
       - `5EE4` fleet-scratch offsets now correlate cleanly to the known fleet
         layout:
         - `[BP+0xFF40]` -> `record[0x02]`
         - `[BP+0xFF41]` -> `record[0x03..0x04]`
         - `[BP+0xFF43]` -> `record[0x05..0x06]`
         - `[BP+0xFF49]` -> `record[0x0B]`
         - `[BP+0xFF4A]` -> `record[0x0C]`
       - the second sub-branch is therefore following the per-empire
         `next fleet` link, not selecting another starbase-side record
       - working hypothesis:
         - summary `+0x06` is carrying a fleet-chain identifier
         - `player[0x40]` in the first sub-branch is likely the empire's
           head-of-chain fleet ID, not a count
     - correction:
       - raw-import entry `2000:C067` is not yet a trustworthy semantic
         function start; it decodes as a fragment inside a larger helper region
       - treat `3502` field correlation, not `C067` naming, as the next
         productive task
     - milestone:
       - the kind-`2` path at `0000:03DF..06AE` actively scans the summary
         table for a matching active kind-`1` entry before it finalizes the
         current base-side summary
       - candidate summary requirements:
         - same summary `+0x00`
         - kind `+0x04 == 1`
         - active/status `+0x03 != 0`
         - and then either:
           - direct word match `candidate +0x0A == [0x3558]`
           - or same `+0x01`, `+0x02`, `+0x05` plus helper-decoded `+0x06`
             matching `[0x355A]` with decoded kind `4` and flag `0`
       - practical consequence:
         - the later `unknown starbase` behavior is now best modeled as a
           failed kind-`1` / kind-`2` summary pairing problem
         - Rust-side compliant gamestate generation will need fleet/base
           linkage values that survive this summary pairing, not just
           individually plausible records

2. `IPBM.DAT` resolution
   - Practical status: structurally complete enough for Rust-side compliant
     file generation.
   - Confirmed:
     - record size `0x20`
     - `PLAYER[0x48]` is the record count gate
     - the raw record copies contiguously into `DS:3538`
     - the overlapping field map is documented in `RE_NOTES.md`
   - Remaining work is semantic naming of some tuple payloads, not file-layout
     discovery or integrity-critical structure.
     - script: `tools/ghidra_scripts_tmp/ReportIPBMTailTransition.java`
     - confirmed:
       - common writeback always updates summary offsets `+0x01`, `+0x02`,
         `+0x05`
       - kind `2` has an extra side path through helper `0x2000:c100`
       - kind `3` writes finalized tuples back into:
         - tuple A -> `3541`, `3543..3547`
         - tuple B -> `3542`, `3549..354d`
         - tuple C -> `354f..3553`
     - practical implication:
       - `3555..3557` are outside the main tuple A/B/C writeback and should be
         treated as a separate trailing control group
   - Current boundary result:
     - `353D` is only consumed by the second `5EE4` `IPBM` branch
     - `3555..3557` are only visible inside the kind-`3` path in `0000:02c0`
     - practical implication:
       - kind `3` has a primary normalized field group
         (`3541`, `3543..3547`, `3542`, `3549..354d`, `354f..3553`)
       - plus a trailing group (`3555..3557`)
       - while `353B/353D` likely belong to the second-branch follow-on result
         path rather than the generic trailing group
   - First live baseline capture:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/registers-6870.txt`
       - `artifacts/ecmaint-ipbm-debug/scratch-3538-6870.txt`
     - setup:
       - valid one-record baseline with `PLAYER[0x48] = 1`
       - zero-filled `IPBM.DAT` of length `0x20`
       - breakpoint at live `2814:6870`
     - key observed normalized values:
       - `353D = 0x0001`
       - `3543 = 0x0080`
       - `3549 = 0x0080`
       - observed `3541`, `3542`, and `354F..3557` bytes/words are zero
     - practical implication:
       - this is the baseline shape to compare against future mutated `IPBM`
         records
   - First mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x00] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0001`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM` offset `0x00` definitely feeds tuple C / the summary-`+0x0A`
         word path
       - it also suppresses the zero-record default normalization that
         previously produced `353D = 1` and the paired `0x0080` defaults
   - Second mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x01] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0100`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM[0x00..0x01]` maps directly into `3538` as a little-endian
         word
       - tuple C / early summary `+0x0A` is therefore confirmed to derive from
         the first `u16` in the raw record
   - Expanded prefix mapping:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_02_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_03_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_04_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_05_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_06_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_07_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_09_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_0a_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x02]` copies to scratch `353A`
       - `IPBM[0x03]` copies to scratch `353B`
       - `IPBM[0x04]` copies to scratch `353C`
       - `IPBM[0x05]` copies to scratch `353D`
       - `IPBM[0x06]` copies to scratch `353E`
       - `IPBM[0x07]` copies to scratch `353F`
       - `IPBM[0x09]` copies to scratch `3541`
       - `IPBM[0x0A]` copies to scratch `3542`
     - semantic read with `2000:5EE4`:
       - `353A` is the player / empire byte copied to summary `+0x00`
       - `353B..353C` is the non-aligned `u16` that gates the second `IPBM`
         branch
       - `353D..353E` is the non-aligned `u16` later written to summary `+0x06`
       - `3541` and `3542` are the kind-`3` tag bytes written to summary
         `+0x01` and `+0x02`
     - practical implication:
       - the front of the raw `IPBM` record copies contiguously into scratch,
         then `ECMAINT` interprets overlapping byte/word fields over that copy
       - baseline all-zero defaults like `353D = 1` and `3543 = 3549 = 0x0080`
         are derived normalization, not the raw on-disk values
   - Group-start confirmation:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_0b_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_11_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_17_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_1d_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x0B]` copies to scratch `3543` (tuple-A payload block start)
       - `IPBM[0x11]` copies to scratch `3549` (tuple-B payload block start)
       - `IPBM[0x17]` copies to scratch `354F` (tuple-C payload block start)
       - `IPBM[0x1D]` copies to scratch `3555` (trailing control group start)
     - static follow-up from `0000:0723..0797`:
       - `3555` and `3556` are read as scalar bytes and expanded through
         helper `0x3000:486B`
       - `3557` is clamped to at most `1`, so raw `0x1F` behaves like a
         boolean / capped mode byte
     - dynamic clarification:
       - `artifacts/ecmaint-ipbm-debug/off_1e_val_01-scratch.txt` confirms
         `IPBM[0x1E]` copies to `3556`
       - `artifacts/ecmaint-ipbm-debug/off_1f_val_02-scratch.txt` shows the
         first `5EE4` stop still sees raw `3557 = 0x02`
       - the clamp to `1` therefore happens later in shared summary logic,
         not during the initial record-to-scratch copy
     - practical implication:
       - the coarse full-record layout is now stable enough for Rust-side
         binary encoding even though several gameplay semantics remain unnamed

3. Build queue / stardock encoding
   - Continue the partially solved `PLANETS.DAT[0x38]` / `0x4C` work.
   - Determine how completed production is stored in the stardock, including
     multiple ship types/counts and the exact handoff to later commissioning.
   - Goal: enough format/behavior detail to reproduce build completion without
     depending on the original executable.

4. Maintenance phase ordering around `Move.Tok`
   - Now that `Move.Tok` is understood as the crash-recovery marker, map the
     broader maintenance phase boundaries around it.
   - Determine what runs before movement, what runs after movement, when
     backups are written, and when token files are created/deleted.
   - Goal: recover the high-level phase order for faithful reimplementation of
     the maintenance engine.

Suggested execution order:
- First finish `2000:5EE4` branch mapping with emphasis on `BASES.DAT` and
  `IPBM.DAT`.
- Then move to build queue / stardock mechanics.
- Finally map movement-phase ordering and backup/token lifecycle around
  `Move.Tok`.
