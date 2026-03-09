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

## Latest Commits

- `edd013e` `Identify 0x04-0x09 as Real, add 0x09 bombardment fixture and test`

## Next Experiment

Goal: Trigger planet damage that results in a player-facing report in `RESULTS.DAT`.

Recommended path: Increase attacker fleet power significantly (e.g., 50 cruisers)
against the standard `army1+dev0` baseline, but also try increasing the
`developed` values on the planet.

Suggested procedure:

1. copy `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/` to a throwaway `/tmp`
   directory
2. Increase fleet ships in record `2` (offset 108): `CA=50`, `DD=50`
3. run `ECMAINT /R` twice
4. inspect:
   - `MESSAGES.DAT`
   - `RESULTS.DAT`
   - `PLANETS.DAT` record `13`
