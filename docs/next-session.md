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

2. `IPBM.DAT` resolution
   - Reverse the `IPBM.DAT` expectations inside the same early validation
     chain.
   - All preserved fixtures currently leave `IPBM.DAT` empty, so this is one
     of the main remaining blind spots in `2000:5EE4`.
   - Goal: identify record size, count rules, ownership/linkage, and the exact
     conditions that trigger integrity failure.
   - Current static foothold:
     - artifact: `artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt`
     - script: `tools/ghidra_scripts_tmp/Report5EE4IPBM.java`
     - confirmed:
       - DS:`31F8` is the `IPBM.DAT` stream
       - record size is `0x20`
       - first branch is `2000:675A..68E8`
       - follow-on branch is `2000:68E9..69B8`
       - fetched records are normalized into scratch buffer DS:`3538`
       - successful reads append `0x0C`-byte summary entries via `0x2F72` /
         `0x2F76`
   - Immediate next question:
     - identify the meanings of scratch fields `3538..3553` and determine what
       gate `DS:353B` controls in the second `IPBM` branch
   - New static lead:
     - artifact: `artifacts/ghidra/ecmaint-live/ipbm-scratch-uses.txt`
     - script: `tools/ghidra_scripts_tmp/ReportIPBMScratchScalarUses.java`
     - confirmed:
       - `3538..3553` are shared scratch/state fields, not just one local copy
         inside `2000:5EE4`
       - a separate function currently carved as `0000:02c0` writes and later
         reads:
         - `3541`, `3543..3547`
         - `3542`, `3549..354d`
         - `354f..3553`
       - `2000:5EE4` consumes those normalized fields when building the `IPBM`
         summary entries
   - Best immediate next RE target:
     - reverse the producer/consumer function at `0000:02c0` to name the
       scratch-field layout, then feed those names back into the `2000:5EE4`
       `IPBM` branches
   - Current function-level result:
     - artifact: `artifacts/ghidra/ecmaint-live/ipbm-scratch-function.txt`
     - confirmed:
       - `0000:02c0` takes a summary-entry index and dispatches on summary kind
         byte `ES:[DI+4]`
       - kind `1` uses scratch block `3502`
       - kind `2` uses scratch block `3558`
       - kind `3` uses the `IPBM` scratch block `3538`
       - the kind-`3` path consumes normalized fields
         `3541`, `3543..3547`, `3542`, `3549..354d`, `354f..3553`,
         `3555..3557`
     - practical meaning:
       - `0000:02c0` is a generic summary-entry dispatcher/normalizer, not an
         `IPBM`-only parser
       - it is also not just a one-way consumer: its later shared tail writes
         normalized values back into the active summary entry and the kind
         scratch blocks
   - Helper-region correction:
     - artifacts:
       - `artifacts/ghidra/ecmaint-live/ipbm-normalizer.txt`
       - `artifacts/ghidra/ecmaint-live/summary-helper-region.txt`
     - scripts:
       - `tools/ghidra_scripts_tmp/ReportIPBMNormalizer.java`
       - `tools/ghidra_scripts_tmp/ReportSummaryHelperRegion.java`
     - confirmed:
       - the direct call target at `2000:c0cd` decodes only as a tiny
         counted-string copy tail
       - the nearby clean helper start is `2000:c0dc`, a bounded copy helper
       - so `2000:c0cd` should not be treated as the semantic kind-`3`
         normalizer
   - Refined next question:
     - determine the semantic meaning of the kind-`3` scratch fields inside the
       generic summary machinery, especially `3541/3542`, `354f..3553`, and
       `353B/353D`
     - start from the shared post-kind pipeline at `0000:07da..0ea6`, not the
       misleading `2000:c0cd` helper tail
   - Focused post-kind pipeline:
     - artifact: `artifacts/ghidra/ecmaint-live/ipbm-postkind-pipeline.txt`
     - script: `tools/ghidra_scripts_tmp/ReportIPBMPostKindPipeline.java`
     - confirmed:
       - `0000:07da..0ea6` is shared canonicalization / merge logic, not a
         kind-`3`-only parser
       - it operates over three local normalized tuples:
         - A at `[BP-0x06..-0x02]`
         - B at `[BP-0x12..-0x0E]`
         - C at `[BP-0x24..-0x20]`
       - it seeds an auxiliary value from the kind-count byte and literal
         `0x86`
       - after the compare/combine tree, it writes canonicalized results back
         to summary offsets `+0x01`, `+0x02`, `+0x05` and then to the kind
         scratch block
     - next use:
       - map tuple A / B / C onto specific kind-`3` scratch fields and compare
         against the live baseline dump from `3538`
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
