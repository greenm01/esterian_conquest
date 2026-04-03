# Fleet Order Selection Targeting Spec

This document defines the Rust client policy for fleet-order fleet selection,
target defaults, coordinate entry, and validation.

The manuals define the mission target classes and gameplay semantics. This spec
defines how the Rust TUI should help the player choose a target without turning
UI defaults into hard blockers.

## Source Authority

Primary sources:

- [nc_player_manual.typ](../manuals/nc_player_manual.typ)
- [ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)

Current implementation lives in:

- [controller.rs](../../rust/nc-game/src/domains/fleet/controller.rs)
- [orders.rs](../../rust/nc-game/src/domains/fleet/orders.rs)
- [fleet.rs](../../rust/nc-game/src/domains/fleet/screens/fleet.rs)

## Scope

This spec covers:

- single-fleet Order flow
- post-selection Group Fleet Order flow
- Fleet Menu `Order Fleet # [default]` fleet selection
- target defaults shown in `[XX]` and `[YY]`
- target validation and screen routing after entry

This spec does not change yearly maintenance semantics or the underlying mission
rules beyond the player-facing validation the client should enforce.

## Manual Semantics vs Rust UI Policy

Manual-backed semantics:

- which missions target sectors vs worlds/systems
- which missions are hostile
- which missions require owned, non-owned, or unowned worlds
- the player-facing meaning of Guard, Scout, Colonize, Bombard, and related
  orders

Rust-client policy:

- which fleet number should be suggested by default for `Order Fleet #`
- which valid target should be suggested by default
- when `[YY]` should be blank
- when duplicate scout/colonize defaults should be suppressed
- when invalid coordinate entry returns the player to `XX`

## Mission Target Classes

- `Move`, `Patrol`, `Scout Sector`, and `Rendezvous` target sectors.
- `Guard/Blockade`, `Bombard`, `Invade`, `Blitz`, `View`, `Scout System`,
  `Colonize`, and `Salvage` target worlds/systems.
- `Seek Home` is resolved automatically to the nearest owned world and does not
  need manual coordinate entry.
- `Guard Starbase` and `Join Fleet` use non-coordinate target prompts.

## Default Fleet Number Policy

- This policy applies only to the Fleet Menu `Order Fleet # [default]` prompt.
- Ordering from Fleet List or Fleet Review remains contextual and uses the
  selected fleet directly.
- Review, Change, ETA, merge, transfer, detach, transport, and other
  fleet-number prompts keep the existing strongest-owned-fleet default.
- The order prompt default is chosen from player-owned fleets with this
  precedence:
  1. fleets not currently on `Patrol` or `Guard/Blockade`
  2. if none exist, fleets on `Patrol` or `Guard/Blockade`
  3. within that pool, fleets that look ready for a fresh assignment:
     `Hold`, target `[0,0]`, or target equal to current location
  4. within the ready bucket, fleets with `ETAC > 0`
  5. final tie-break: the existing strongest-fleet ranking
- If no ready fleets exist in the chosen pool, the client falls back to the
  strongest fleet in that pool.
- Patrol/blockade avoidance is advisory only for the default prompt. The player
  can still type any owned fleet number manually.

## Default Target Policy

- `Move`: current sector
- `Patrol`: current sector
- `Guard/Blockade`: nearest owned world
  - *Rationale*: Guard is the more common defensive use case. Blockade users targeting enemy worlds are expected to enter coordinates manually.
- `Bombard`, `Invade`, `Blitz`: nearest known enemy-owned world (owner > 0 and owner != self)
  - Unowned worlds (owner 0) are excluded since they are never valid hostile targets.
- `View`: closest unknown world from the player's total planet database,
  preferring worlds not already targeted by another friendly fleet on any
  mission
  - If no unknown worlds remain, there is no smart default and the player must
    enter coordinates manually.
  - *Rationale*: View already collects partial intel, so partial worlds are not
    useful smart defaults.
- `Scout Sector`: nearest known unowned world sector not already claimed by
  another active friendly scout order; if none, current sector only if that
  sector is not one of the player’s owned systems
- `Scout System`: nearest known non-owned world not already claimed by another
  active friendly scout order
- `Colonize`: nearest known unowned world not already claimed by another active
  friendly ETAC colonize order
- `Rendezvous`: target of the nearest other friendly fleet already on
  Rendezvous; if none, current sector
- `Salvage`: nearest owned world

Interpretation rules:

- "known world" means a world visible in the player starmap projection with at
  least partial useful intel
- "known unowned" means owner `0` in that projection, not "unknown owner"
- "known enemy-owned" means a world whose `known_owner_empire_id` is `Some(id)`
  where `id != 0` and `id != self` — explicitly excludes unowned (owner 0)
  worlds, which are never valid hostile-mission targets
- Worlds with no intel at all (never scouted, viewed, or encountered) never appear
  as default candidates for any mission. The player must type coordinates manually
  to target an unscouted world.
- defaults are advisory; the player may still type coordinates manually unless
  the final validation rejects them

## Duplicate-Target Suppression

- ETAC defaults should skip worlds already targeted by other friendly fleets
  that:
  - contain ETACs
  - already have active `Colonize` orders
  - target the same coordinates
- Scout defaults should skip worlds/sectors already targeted by other friendly
  fleets that:
  - contain scouts
  - already have active `Scout Sector` or `Scout System` orders
  - target the same coordinates
- View defaults should prefer unknown worlds not already targeted by another
  friendly fleet on any mission.
- The fleets currently being ordered are excluded from these duplicate checks.
- If every otherwise-valid candidate is already claimed, the default should be
  blank rather than falling back to a claimed target.

## Coordinate Entry Flow

- Defaults shown in `[XX]` and `[YY]` are advisory only.
- Entering `XX` never blocks progress on its own.
- After the player enters `XX`, the Rust client recomputes the suggested `YY`.
- For world/system-target missions, `[YY]` should only be prefilled when a
  valid target exists in that `XX` column.
- If no valid target exists in that `XX` column, `[YY]` is blank.
- Blank `[YY]` is not itself an error.

## Target Entry Layouts

- Coordinate-target entry uses the detailed target-entry layout for single-fleet
  order:
  - title
  - location / speed / ROE / current order
  - ships
  - `Enter target coordinates for new order: <mission>`
  - `Target XX` / `Target YY` prompt
- Named-target entry (`Guard Starbase`, `Join Fleet`) uses the same detailed
  single-fleet summary layout, but replaces the coordinate instruction with the
  explicit prompt text:
  - `Enter the starbase number for Guard a Starbase.`
  - `Enter the host fleet number for Join another fleet.`
- Single-fleet named-target entry does not render `New Orders:` above the
  prompt.
- Post-selection group order keeps the compact group summary:
  - `Selected fleets: N`
  - explicit instruction line for named-target entry
  - prompt line
- Group named-target entry does not use the generic
  `Enter target for new order: <mission>` line.

## Validation Flow

- Validation happens only after the player completes both coordinates.
- If the final `(XX,YY)` is valid, the flow proceeds to confirmation.
- If the final `(XX,YY)` is invalid, the client shows a notice/error and
  returns to `XX` entry while preserving the typed coordinate values.

Client-side validation rules:

- `Bombard`, `Invade`, and `Blitz` must reject owned-world targets.
- `Scout Sector` must reject sectors that match one of the player's owned
  planet coordinates.
  - Note: Scout Sector targets sectors, not worlds. Empty sectors are valid targets.
- `Scout System` must reject owned-world targets.
- `Salvage` must reject non-owned or empty-sector targets.
- `Guard/Blockade` may target either owned or non-owned worlds.

## Single vs Group Orders

- Single-fleet and post-selection group-fleet order flows use the same target
  recommendation and validation rules.
- Group defaults use the first selected fleet as the distance anchor.
- Group duplicate-target suppression excludes the fleets currently selected for
  the new group order.
