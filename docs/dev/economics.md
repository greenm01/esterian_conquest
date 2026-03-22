# Esterian Conquest Canonical Economics Spec

This document defines the current canonical Rust economy model for Esterian
Conquest.

It is not a claim that the original `ECMAINT.EXE` used these exact internal
formulas. It is the project’s explicit, auditable economy rule where the
manuals are clear and the original replay/oracle path is still awkward to
probe directly.

For recovered phase placement and mission interaction, read this together with:

- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [ec-combat-spec.md](ec-combat-spec.md)

## Status

This is the current source of truth for:

- player-facing production terminology
- opening homeworld economy setup
- empire-wide tax behavior
- yearly revenue
- yearly current-production growth toward potential
- starbase effects on build capacity and growth

The starbase `5x` question is now closed at the semantic level:

- the manuals explicitly tie `5x` to build capacity
- a focused Ghidra follow-up against the recovered unwrapped
  `ECMAINTU.EXE` project did **not** surface evidence for a starbase `5x`
  growth multiplier
- the follow-up black-box sweep in
  [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
  reconfirmed the starbase/build-capacity side but still did **not** produce a
  trustworthy exact classic growth/tax formula from generated probe worlds
- the exact classic growth bonus remains unrecovered, so the Rust
  `+50%` growth bonus below remains the documented canonical Rust policy,
  not a claim of byte-exact classic recovery

It does not replace the original manuals as historical sources. It translates
them into an explicit Rust rule set suitable for reproducible maintenance.

It does **not** define the whole maintenance turn order by itself. This
document defines the Rust economy/build policy that runs inside the recovered
post-loop world/player update region.

## Turn-Order Placement

The recovered yearly turn order now constrains where economy/build behavior
lives:

- build completion and economy run in the **post-loop world/player update
  region**
- that region happens **after** the weekly `52`-pass fleet-combat loop
- it happens **before** the later ready hostile world-resolution region
  (`BombardWorld` / `InvadeWorld` / `BlitzWorld`)

Practical consequences:

- ship and starbase builds can enter stardock before a ready hostile
  world-resolution path hits the same planet
- ready bombardment/invasion therefore sees post-build planet state
- armies and ground batteries that complete in this region go straight onto the
  planet rather than into stardock
- later same-turn hostile world-resolution reads those units as planet state,
  not as stardock inventory

## Source Basis

Primary manual references:

- [original/v1.5/ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC)

Important manual-facing claims:

- players begin with one homeworld at current production `100`
- the empire tax rate is empire-wide, not per planet
- taxes generate yearly production points for spending
- newly colonized planets begin below maximum production
- lower taxes improve planetary development
- taxes above roughly `65%` can directly harm current production
- starbases:
  - help planets endure tax burden better
  - let planets spend up to `5x` current production on builds
  - help underdeveloped planets grow current production faster

Current Ghidra-backed reading:

- the recovered unwrapped `ECMAINTU.EXE` project exposes real planet-side
  functions, but this pass did not recover a clean starbase `* 5` growth path
- taken together with the manuals, current evidence supports:
  - `5x` build capacity
  - some separate starbase growth acceleration
  - no verified claim that growth itself is multiplied by `5`

Current black-box follow-up:

- the controlled starbase/tax sweep in
  [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
  does preserve the commissioned starbase and the larger build-capacity effect
- but the resulting colony production values are still too pathological to
  promote into an exact classic growth/tax formula
- so the manuals still carry more semantic weight here than the current
  generated-oracle colony sweep

## Canonical Terms

Rust should use these player-facing terms:

- `Present Production`
  - the current productive capacity of a planet or empire
- `Potential Production`
  - the maximum productive capacity of a planet or empire
- `Total Available Points`
  - the current turn’s spendable tax revenue budget
- `Stored Production Points`
  - accumulated stored points on a planet

Rust should avoid exposing low-level storage nicknames like `factories` in
player/client surfaces.

## Opening Economy

Generated Rust starts now encode the intended opening economy directly:

- homeworld `Potential Production = 100`
- homeworld `Present Production = 100`
- default empire tax rate `= 50%`
- first-turn `Total Available Points = 50`
- homeworld defenses:
  - armies `= 10`
  - ground batteries `= 4`

This applies to:

- joinable `ECGAME` new-game baselines
- canonical initialized builder baselines

## Empire-Wide Tax

The tax rate is stored on the player/empire, not chosen separately per planet.

Canonical yearly revenue uses the empire tax rate across all owned planets:

`yearly_tax_revenue_for_planet = floor(present_production * empire_tax_rate / 100)`

Empire `Total Available Points` for the player-facing summary is:

`sum(floor(present_production * empire_tax_rate / 100))` across owned planets

This is the current turn’s spendable build budget, not a raw stored-goods sum.

## Present Production Growth And Tax Pressure

Each maintenance turn, every owned active planet grows toward its potential
production.

The canonical Rust rule is:

1. Compute remaining growth gap:

`gap = potential_production - present_production`

2. Compute tax headroom:

`tax_headroom = 100 - min(empire_tax_rate, 95)`

3. Base yearly growth:

`growth = ceil(gap * tax_headroom / 400)`

4. If the planet has a friendly starbase, apply a growth bonus:

`growth = growth + ceil(growth / 2)`

5. Clamp growth:

- minimum growth is `1` while the planet is still below potential
- growth may not exceed the remaining gap

6. New present production:

`present_production = min(present_production + growth, potential_production)`

7. Apply a high-tax penalty when empire tax exceeds the safe threshold:

- safe threshold without starbase: `65%`
- safe threshold with friendly starbase: `70%`

`overtax = empire_tax_rate - safe_threshold`

`penalty = ceil(present_production * overtax / 500)`

Clamp the penalty:

- minimum penalty is `1` when tax exceeds the threshold and production is nonzero
- penalty may not exceed current present production

Final yearly result:

`present_production = min(present_production + growth, potential_production) - penalty`

### Effects of This Rule

- lower tax produces faster long-term development
- higher tax produces more immediate revenue but slower growth
- taxes above `65%` can directly reduce present production
- starbase worlds tolerate up to `70%` before the direct penalty begins
- growth slows naturally as a planet approaches potential
- starbase worlds recover/develop faster than non-starbase worlds

This brings the canonical Rust rule into closer compliance with the manuals’
explicit warning that production may suffer above `65%`, while still keeping
the curve simple and auditable.

## Starbase Effects

Starbases affect economy in two separate ways.

### 1. Growth bonus

If a planet has a friendly active starbase in orbit at its coordinates, yearly
current-production growth is boosted by `+50%` over the base growth amount.

This refers to an active commissioned starbase, not an uncommissioned starbase
item still sitting in stardock.

This `+50%` rule remains the canonical Rust policy for now. The current manual
plus Ghidra evidence does **not** support rewriting this into a starbase
`5x` growth multiplier.

### 2. Build capacity multiplier

The manuals say a starbase lets a planet spend up to `5x` its current
production on building units.

Canonical Rust implementation:

- without starbase:
  - per-turn build capacity = `present_production`
- with friendly starbase:
  - per-turn build capacity = `present_production * 5`

This affects build-queue completion, not tax revenue directly.

## Stored Production Points

Yearly tax revenue is added to each planet’s stored production pool:

`stored_production_points += yearly_tax_revenue_for_planet`

This is separate from the empire summary’s `Total Available Points` line, which
is the turn-budget view used by the original player-facing screens.

## Special Cases

### Homeworld seeds

True homeworld seeds start at full production from the opening turn.

### Civil disorder and rogue paths

The normal Rust economy formulas in this document do not replace the recovered
classic rogue/autopilot gate.

Current practical split:

- the recovered classic autopilot/rogue pass is gated by `player[0] == 0xFF`
- civil-disorder `0x00` empires are economically frozen in the recovered
  classic path
- the broader Rust normal-planet economy policy is documented here as the
  explicit Rust rule set for maintainable implementation

Their specialized maintenance behavior is handled separately.

## Known Raw-Field Caveat

`PLANETS.DAT raw[0x0E]` is currently not treated as a settled semantic “planet
tax” field.

Mixed-tax Rust probes show that this byte is overwritten during the existing
autopilot/rogue-AI maintenance path. For that reason:

- Rust code should treat it as an overloaded economy marker byte
- player-facing tax comes from the empire/player record
- this raw byte should not be used as a stable player-facing tax source until
  the original semantics are fully decoded

## Unit Build Completion and Stardock Policy

When a build queue slot reaches zero points remaining during maintenance, the
completed units are dispatched based on their kind:

### Ships and starbases → stardock

Destroyers, cruisers, battleships, scouts, troop transports, ETACs, and
starbases (kinds 1–6, 9) are staged in the planet's stardock slots awaiting
commission.

- They must be commissioned by the player before they can be used.
- Uncommissioned ships in stardock can be destroyed by a bombardment mission.
- Because build completion happens before ready hostile world resolution,
  newly completed stardock contents are already exposed to same-turn ready
  bombardment / invasion losses.
- This matches the manual: "Bombard a world: destroy its production and anything
  orbiting the world, including recently built ships stored in stardock."

### When stardock is full

If a ship or starbase build reaches completion and the planet's stardock has no
open slot, EC does **not** consume or refund the order.

Instead:

- the build queue slot stays in place unchanged
- the points remaining stay unchanged
- the player may still abort that queued order manually from the build menu
- once stardock space is freed, later maintenance can complete the build

This is an intentional Rust safety policy.

Original `ECMAINT` does **not** handle this edge case safely. A focused probe
with a full stardock and a completing ship build showed that classic
maintenance:

- cleared the build slot
- emitted no `ERRORS.TXT`
- corrupted the target planet's stardock bytes

So EC treats "completion into a full stardock" as an invalid classic state
and holds the build in queue rather than reproducing that corruption bug.

### Armies and ground batteries → direct to planet

Armies (kind 8) and ground batteries (kind 7) are surface and ground defensive
units. On build completion they are added directly to the planet's army count
and ground battery count respectively. They do **not** enter stardock.

Rationale:
- The manual never mentions armies or batteries being stored in stardock or
  requiring commission. Stardock is explicitly a ship staging area.
- Commission is a fleet-building concept; you commission ships into fleets.
  Armies are loaded onto troop transports separately. Batteries are fixed.
- Armies and batteries deployed to the planet surface are **not** handled as
  stardock losses. They are already on the ground and belong to the planet's
  defensive state.
- They may still be lost later through normal planet bombardment / assault
  damage, but that is hostile world-resolution damage, not stardock handling.
- Treating them as stardocked units would mean a player could lose an entire
  army build to a bombardment before ever using them, which contradicts the
  manual's framing of armies as planet defenders.
- A full stardock does not block them. They complete normally because they do
  not use stardock at all.

### Planet army / battery byte caps

Original `ECMAINT` treats both of these planet-side fields as hard byte-sized
caps.

Focused oracle probes showed:

- if a planet already has `255` armies, a completing army build is still
  consumed, but the planet remains at `255`
- if a planet already has `255` ground batteries, a completing battery build is
  still consumed, but the planet remains at `255`

So for EC:

- planet armies should currently be treated as capped at `255`
- planet batteries should currently be treated as capped at `255`
- client and engine guards should prevent silent overflow where practical
- if a queued army or battery build would complete past that cap, Rust keeps the
  build queued unchanged instead of silently consuming it
- if a player tries to unload troop-transport armies onto a full planet, Rust
  blocks the unload and reports the cap clearly

This is distinct from loaded fleet armies, which are stored more widely in
`FLEETS.DAT`.

## Validation Status

The current canonical model is backed by:

- Rust regression tests for:
  - opening homeworld production
  - first-turn available points
  - tax-sensitive growth
  - starbase growth bonus
  - build-capacity behavior
- mixed-tax probe runs using:
  - [tools/economy_tax_probe.py](../../tools/economy_tax_probe.py)

Current limitation:

- the mutated-directory original `ECMAINT` replay path still does not provide a
  reliable byte-diff oracle for these tax-growth experiments
- when stronger original-binary evidence becomes available, this canonical rule
  should be refined rather than treated as frozen forever
