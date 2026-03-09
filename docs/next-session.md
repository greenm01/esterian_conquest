# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`, treated as a deterministic
black box.

**Highest-confidence planet model:**

- `PLANETS.DAT[0x04..0x09]` is a 48-bit Borland Pascal `Real` (6 bytes)
  representing `present` capacity (baseline `100.0`).
- `PLANETS.DAT[0x0A..0x0D]` is a 4-byte field, possibly a `LongInt` or part of a
  larger structure (baseline `0`).
- `PLANETS.DAT[0x0E]` is **confirmed** as **ground batteries**.
  - Baseline `4` for mature worlds.
  - `0xff` caused total fleet destruction.
  - Higher values scale attacker losses significantly.
- `PLANETS.DAT[0x5A]` is the strongest current army/defender-count candidate.
- `PLANETS.DAT[0x58]` modulates both world damage and part of attacker losses.
- the dense world-defense block is now clearly `PLANETS.DAT[0x04..0x0E]`.

**Recent findings:**

- Bombardment with 50 Cruisers + 50 Destroyers successfully annihilated a
  target world's capacity (Real at `0x04..0x09` went to `0.0`).
- Even with a player-assigned target empire (using `ECUTIL F3`), `MESSAGES.DAT`
  and `RESULTS.DAT` remain empty at Year 3001.

## Latest Commits

- `edd013e` `Identify 0x04-0x09 as Real, add 0x09 bombardment fixture and test`
- `73aefb7` `Update handoff for next bombardment scaling experiment`

## Next Experiment

Goal: Trigger generated text reports in `MESSAGES.DAT` or `RESULTS.DAT`.

Hypothesis:
1. Reports might only be generated after Year 3001 (Year 3000 is the "start"
   year).
2. Reports might only be generated for "named" colonies (not `Unowned` or
   `Not Named Yet`).

Suggested procedure:

1. Use the "Heavy Attacker" setup from the previous attempt (50 CA, 50 DD).
2. Clone the `Dust Bowl` (record 15) record onto the target coordinates (15,13)
   to ensure it is a "named" mature colony.
3. Advance `CONQUEST.DAT` to a much later year (e.g., 3010) before running
   maintenance.
4. run `ECMAINT /R` twice.
5. inspect `MESSAGES.DAT` and `RESULTS.DAT`.
