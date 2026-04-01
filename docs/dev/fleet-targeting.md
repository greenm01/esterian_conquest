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

## Mission Table

| Order | Input type | Smart target candidate set | Extra filtering / fallback | Default rule |
| --- | --- | --- | --- | --- |
| `0` Hold Position | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `1` Move Fleet (only) | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `2` Seek Home | coordinates | owned planets | nearest first; target must be an owned planet/system | `XX/YY` default to nearest owned world |
| `3` Patrol a Sector | coordinates | selected fleet's current sector | none | `XX/YY` default to anchor sector |
| `4` Guard a Starbase | starbase id | closest owned starbase | sorted by nearest | default starbase number |
| `5` Guard/Blockade a World | coordinates | owned planets | nearest first | `XX/YY` default to nearest owned world |
| `6` Bombard a World | coordinates | known enemy planets | if no known enemy intel exists, falls back to actual enemy-owned planets from game data | `XX/YY` default to nearest eligible enemy world |
| `7` Invade a World | coordinates | known enemy planets | same as bombard | same |
| `8` Blitz a World | coordinates | known enemy planets | same as bombard | same |
| `9` View a World | coordinates | under-scouted known worlds | if none are under-scouted, falls back to all known worlds | `XX/YY` default to nearest eligible known world |
| `10` Scout a Sector | coordinates | known unowned worlds | excludes owned systems and scout targets already claimed by other friendly scout fleets; if empty and anchor is not owned, falls back to anchor sector | `XX/YY` default to nearest eligible scout sector |
| `11` Scout a Solar System | coordinates | known non-self-owned worlds | excludes scout targets already claimed by other friendly scout fleets | `XX/YY` default to nearest eligible system |
| `12` Colonize a World | coordinates | known unowned worlds | excludes colonize targets already claimed by other friendly ETAC fleets | `XX/YY` default to nearest eligible colonize world |
| `13` Join Another Fleet | fleet id | closest owned fleet not in selection | excludes self for single-fleet order and excludes the selected group for group order | default fleet number |
| `14` Rendezvous at Sector | coordinates | friendly fleets already on `RendezvousSector` orders | excludes selected fleets; if none exist, falls back to anchor sector | `XX/YY` default to nearest rendezvous target |
| `15` Salvage | coordinates | owned planets | nearest first; target must be an owned planet/system | `XX/YY` default to nearest owned world |

## Shared Rules

| Rule | Behavior |
| --- | --- |
| Anchor point | Single-fleet order uses the selected fleet's coordinates. Group order uses the first selected fleet row's coordinates. |
| Sorting | Coordinate candidates are sorted by distance from the anchor, nearest first. |
| Dedup | Duplicate candidate coordinates are removed after sorting. |
| Known-world gate | `View`, `Scout`, `Colonize`, and the primary hostile-world targeting path use the player starmap/intel projection. A world counts as known if any key intel field is known. |
| Enemy-world fallback | `Bombard`, `Invade`, and `Blitz` first prefer known enemy worlds. If none exist in intel, they fall back to actual enemy-owned planets from runtime game data. |
| Owned-world targeting | `Seek Home`, `Guard/Blockade`, and `Salvage` use owned planets directly from runtime game data, not the player-intel projection. |
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
