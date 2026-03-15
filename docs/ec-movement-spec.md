# EC Movement And Pathfinding Spec

This document separates two concerns:

- classic EC movement semantics from the manuals and observed maintenance rules
- canonical Rust pathfinding policy for choosing safer routes

The first is a fidelity target. The second is an explicit extension.

## Source Authority

Primary sources:

- [ECPLAYER.DOC](original/v1.5/ECPLAYER.DOC)
- current movement RE and fixtures in
  [RE_NOTES.md](RE_NOTES.md)

## Classic Movement Semantics

Current-known movement rules in Rust already model:

- simultaneous yearly movement
- sector-grid travel toward a target location
- order-driven arrival and mission resolution
- contact and combat based on post-movement position

The live movement implementation is in:

- [mod.rs](rust/ec-data/src/maint/mod.rs)

The current implementation matches the known `ECMAINT` movement formula for the
preserved movement scenarios. That deterministic movement path should remain the
compatibility baseline.

## Contact And Hostility

The manuals support this distinction:

- `enemy` is a stored diplomatic relation
- `hostile` is the broader encounter state used for actual combat

Canonical Rust direction:

- fleets that encounter one another should always generate a contact/intel
  report
- fleets should automatically engage only when the encounter is hostile
- declared enemies meeting in one final location should auto-engage
- non-enemy contacts should not fight unless:
  - one side attacks
  - a defensive rule makes the contact hostile
  - blockade / system-entry rules apply

## Current Limitation

The current Rust maint path resolves contact from final post-movement
co-location. It does not yet model true mid-course interception geometry for
"crossed paths" between sectors.

That is a known simplification and should remain explicit in docs and tests.

## Canonical Rust Pathfinding Extension

Threat-aware pathfinding is a desirable Rust extension, but it should be
documented as an intentional improvement rather than implied to be a recovered
original mechanic.

### Why It Belongs

If a player sends a fleet to a destination, the route should not be stupid.
When known hostile systems, blockades, or starbases exist, a modern faithful
port should be able to avoid obvious destruction when the mission does not
require entering hostile space.

### Pathfinding Goals

Canonical Rust pathfinding should:

- preserve the same movement execution rules once a route is chosen
- choose safer routes when multiple legal paths exist
- avoid accidental entry into hostile systems unless the mission explicitly
  targets that system
- prefer reduced detection/contact risk for scouting and transit missions

### Suggested Cost Model

Use shortest-path routing with additive penalties for known danger:

- enemy-owned solar systems
- known starbases
- active blockades
- known hostile homeworlds

Do not treat transient deep-space fleet sightings as durable route hazards by
default. A fleet sighting in open space is perishable intel; by the time the
moving fleet reaches that sector, the observed force may already be gone.
Pathfinding should therefore avoid building stable route penalties from open
space contact reports alone.

Suggested policy:

- hard-avoid hostile systems for civilian or non-combat transit where possible
- soft-penalize risky sectors for scouts and movement missions
- ignore transient deep-space sightings unless a later, explicitly documented
  policy introduces a very short-lived penalty
- allow direct hostile routing when the target mission is itself hostile:
  - bombard
  - invade
  - blitz
  - guard/blockade hostile target

### Algorithm Direction

The intended long-term routing engine is:

- grid-based A* over the sector map
- deterministic tie-breaking
- route costs derived from visible game state and diplomatic hostility

This should be implemented as a route-planning layer above the current movement
formula, not as a replacement for the movement execution rules themselves.

## Fog Of War Rule

Threat-aware routing must preserve fog of war.

- route costs must be derived from player-visible intel, not hidden global
  truth from the whole gamestate
- the planner may use:
  - visible world ownership
  - visible starbases
  - known blockades
  - stored diplomatic hostility
- the planner should not treat deep-space fleet sightings as stable hazards by
  default
- the planner must not "cheat" by consulting unseen foreign planets, fleets,
  or defenses

Current Rust implementation status:

- the `ec-data` pathfinder now accepts explicit visible hazard intel as input
- the live maint path now accepts owner-scoped visible hazards and the CLI
  derives first-pass foreign-world hazard intel from each empire's
  `DATABASE.DAT` view
- the live visible-hazard path now also derives hostile blockade hazards for an
  empire's own worlds from current gamestate, because that information is
  player-visible without consulting hidden foreign-space intel
- only visible intel should feed the planner; unknown foreign worlds remain
  invisible to routing until discovered
- combat/contact handling now also has an explicit hostility decision seam
  rather than burying that choice inside the battle loop
- `ec-data` now exposes a typed stored-diplomacy seam, and the classic
  `PLAYER.DAT` enemy/neutral bytes are now mapped as a contiguous table:
  - `PLAYER.DAT[player].raw[0x54 + (target_empire_raw - 1)]`
  - `0x00 = neutral`
  - `0x01 = enemy`
- the contiguous `0x54..=0x6C` span provides 25 diplomacy slots
- the live Rust path now also accepts a `diplomacy.kdl` sidecar in the game
  directory as a migration/fallback source; persistable relations are absorbed
  into `PLAYER.DAT`
- the current hostility predicate therefore uses:
  - declared-enemy status from stored bytes
  - declared-enemy status from `diplomacy.kdl`, if present
  - defensive/manual hostility triggers
- foreign co-location by itself should generate contact intel but not force
  combat
- fleet encounters now generate contact/intel reports even when the encounter
  source is not a scout mission

## Compatibility Rule

Threat-aware routing may change which sectors a Rust-controlled fleet chooses to
travel through. That is acceptable if:

- the resulting orders and yearly movement remain classic-compatible on disk
- the pathfinding policy is documented and deterministic
- the policy remains faithful to player intent and manual combat/diplomacy
  rules

It is not acceptable to silently claim that such routing was proven to be the
original DOS behavior when it has not been recovered.
