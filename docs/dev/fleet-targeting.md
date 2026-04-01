# Fleet Targeting

This document records the current fleet target-entry and smart-target default
behavior in `nc-game`.

Source of truth:

- [rust/nc-game/src/domains/fleet/orders.rs](../../rust/nc-game/src/domains/fleet/orders.rs)
- [rust/nc-game/tests/update.rs](../../rust/nc-game/tests/update.rs)

## Summary

- Every fleet mission now requires a target.
- Most missions use split coordinate entry:
  - `Target XX [..] <Q> ->`
  - `Target YY [..] <Q> ->`
- `Guard Starbase` uses a starbase-number prompt.
- `Join Another Fleet` uses a fleet-number prompt.
- Smart defaults are mission-driven.
- Mission availability is still fleet-composition-driven.
- Smart world targeting is driven by the player's fog-of-war `TOTAL PLANET DATABASE:`.

## Mission Table

| Order | Input type | Smart target candidate set | Extra filtering / fallback | Default rule |
| --- | --- | --- | --- | --- |
| `0` Hold Position | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `1` Move Fleet (only) | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `2` Seek Home | coordinates | owned worlds from the player's total planet database | nearest first; target must be an owned planet/system | `XX/YY` default to nearest owned world |
| `3` Patrol a Sector | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `4` Guard a Starbase | starbase id | closest owned starbase | sorted by nearest | default starbase number |
| `5` Guard/Blockade a World | coordinates | owned worlds from the player's total planet database | nearest first | `XX/YY` default to nearest owned world |
| `6` Bombard a World | coordinates | hostile worlds known hostile in the player's total planet database | no raw runtime fallback | `XX/YY` default to nearest eligible hostile world |
| `7` Invade a World | coordinates | hostile worlds known hostile in the player's total planet database | same as bombard | same |
| `8` Blitz a World | coordinates | hostile worlds known hostile in the player's total planet database | same as bombard | same |
| `9` View a World | coordinates | under-scouted worlds from the player's total planet database | if none are `Partial`/`Unknown`, falls back to all database worlds | `XX/YY` default to nearest eligible world |
| `10` Scout a Sector | coordinates | non-self-owned or unknown-owner worlds from the player's total planet database | excludes scout targets already claimed by other friendly scout fleets; if empty and anchor is not owned, falls back to anchor sector | `XX/YY` default to nearest eligible scout sector |
| `11` Scout a Solar System | coordinates | non-self-owned or unknown-owner worlds from the player's total planet database | excludes scout targets already claimed by other friendly scout fleets | `XX/YY` default to nearest eligible system |
| `12` Colonize a World | coordinates | unknown-owner or known-unowned worlds from the player's total planet database | skips worlds known colonized by any empire; excludes colonize targets already claimed by other friendly ETAC fleets | `XX/YY` default to nearest eligible colonize world |
| `13` Join Another Fleet | fleet id | closest owned fleet not in selection | excludes self for single-fleet order and excludes the selected group for group order | default fleet number |
| `14` Rendezvous at Sector | coordinates | friendly fleets already on `RendezvousSector` orders | excludes selected fleets; if none exist, falls back to anchor sector | `XX/YY` default to nearest rendezvous target |
| `15` Salvage | coordinates | owned worlds from the player's total planet database | nearest first; target must be an owned planet/system | `XX/YY` default to nearest owned world |

## Planet Database Criteria

World-target smart defaults use the same world set that backs `TOTAL PLANET DATABASE:` in
`nc-game`. That is the player's fog-of-war world database, including rows whose owner or
other fields are still unknown.

| Order | Source set | Eligible when | Excluded when | Fallback |
| --- | --- | --- | --- | --- |
| `2` Seek Home | total planet database | `known_owner_empire_id == self` | not player-owned | none |
| `5` Guard/Blockade a World | total planet database | `known_owner_empire_id == self` | not player-owned | none |
| `6` Bombard | total planet database | `known_owner_empire_id` is a hostile empire (`> 0` and not self) | unknown-owner, unowned, or self-owned | none |
| `7` Invade | total planet database | same as `Bombard` | same as `Bombard` | none |
| `8` Blitz | total planet database | same as `Bombard` | same as `Bombard` | none |
| `9` View | total planet database | prefer rows with `IntelTier::Partial` or `IntelTier::Unknown` | none | all database rows if no under-scouted rows exist |
| `10` Scout a Sector | total planet database | `known_owner_empire_id != self` or owner unknown | already claimed by another friendly scout | anchor sector if no eligible database world exists and the anchor is not owned |
| `11` Scout a Solar System | total planet database | `known_owner_empire_id != self` or owner unknown | already claimed by another friendly scout | none |
| `12` Colonize | total planet database | `known_owner_empire_id` is `None` or `Some(0)` | known owned by any empire, or already claimed by another friendly ETAC | none |
| `15` Salvage | total planet database | `known_owner_empire_id == self` | not player-owned | none |

## Shared Rules

| Rule | Behavior |
| --- | --- |
| Anchor point | Single-fleet order uses the selected fleet's coordinates. Group order uses the first selected fleet row's coordinates. |
| Sorting | Coordinate candidates are sorted by distance from the anchor, nearest first. |
| Dedup | Duplicate candidate coordinates are removed after sorting. |
| Fog-of-war source | Smart world targeting uses the same player-visible fog-of-war world set as `TOTAL PLANET DATABASE:`. |
| Hostile-world privacy | `Bombard`, `Invade`, and `Blitz` do not fall back to hidden runtime ownership data. If the database does not know a hostile owner, no smart hostile default is shown. |
| Colonize intelligence | `Colonize` treats unknown-owner worlds as eligible smart defaults and skips only worlds the database knows are already colonized. |
| `XX` default | The first candidate's `X` value. |
| `YY` default | The first candidate's `Y` value whose `X` matches the entered or accepted `XX`. |
| `YY` filtering scope | `YY` smart filtering now applies to every coordinate-target mission, not just world-target missions. |
| Empty Enter | If the prompt shows `[XX]` or `[YY]`, pressing Enter accepts that default. |
| No smart candidate | No brackets are shown; the player must enter the value manually. |
| Invalid chosen `XX` | If the entered `XX` has no eligible candidate, no smart `YY` default is shown for that column. |

## Availability And Validation Notes

- Mission availability is checked from fleet composition before target entry.
  - Example: `Colonize` still requires an ETAC.
  - Example: `Scout` missions still require a scout.
- Smart targeting is mission-based after the mission is allowed.
- World-target smart defaults are filtered against player-visible database intel, not hidden runtime ownership.
- `Guard Starbase` and `Join Another Fleet` require a valid default target to
  exist up front; otherwise the mission picker rejects them.
- Coordinate missions can still open manual target entry even when no smart
  default exists.
- Planet-system validation is enforced for:
  - `Seek Home`
  - `Guard/Blockade`
  - `Bombard`
  - `Invade`
  - `Blitz`
  - `View`
  - `Scout Solar System`
  - `Colonize`
  - `Salvage`
- Owned-planet validation is enforced for:
  - `Seek Home`
  - `Salvage`
- Hostile-world validation rejects owned worlds for:
  - `Bombard`
  - `Invade`
  - `Blitz`
- Scout validation rejects owned worlds/systems for:
  - `Scout a Sector`
  - `Scout a Solar System`
