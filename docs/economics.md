# Esterian Conquest Canonical Economics Spec

This document defines the current canonical Rust economy model for Esterian
Conquest.

It is not a claim that the original `ECMAINT.EXE` used these exact internal
formulas. It is the project’s explicit, auditable economy rule where the
manuals are clear and the original replay/oracle path is still awkward to
probe directly.

## Status

This is the current source of truth for:

- player-facing production terminology
- opening homeworld economy setup
- empire-wide tax behavior
- yearly revenue
- yearly current-production growth toward potential
- starbase effects on build capacity and growth

It does not replace the original manuals as historical sources. It translates
them into an explicit Rust rule set suitable for reproducible maintenance.

## Source Basis

Primary manual references:

- [original/v1.5/ECPLAYER.DOC](/home/mag/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](/home/mag/dev/esterian_conquest/original/v1.5/ECQSTART.DOC)

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

The general canonical economy pass currently skips those empires so preserved
maintenance fixtures remain stable.

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
- This matches the manual: "Bombard a world: destroy its production and anything
  orbiting the world, including recently built ships stored in stardock."

### Armies and ground batteries → direct to planet

Armies (kind 8) and ground batteries (kind 7) are surface and ground defensive
units. On build completion they are added directly to the planet's army count
and ground battery count respectively. They do **not** enter stardock.

Rationale:
- The manual never mentions armies or batteries being stored in stardock or
  requiring commission. Stardock is explicitly a ship staging area.
- Commission is a fleet-building concept; you commission ships into fleets.
  Armies are loaded onto troop transports separately. Batteries are fixed.
- Armies and batteries deployed to the planet surface cannot be wiped out by
  a bombardment targeting stardocked ships. They are already on the ground,
  defending the planet.
- Treating them as stardocked units would mean a player could lose an entire
  army build to a bombardment before ever using them, which contradicts the
  manual's framing of armies as planet defenders.

## Validation Status

The current canonical model is backed by:

- Rust regression tests for:
  - opening homeworld production
  - first-turn available points
  - tax-sensitive growth
  - starbase growth bonus
  - build-capacity behavior
- mixed-tax probe runs using:
  - [tools/economy_tax_probe.py](/home/mag/dev/esterian_conquest/tools/economy_tax_probe.py)

Current limitation:

- the mutated-directory original `ECMAINT` replay path still does not provide a
  reliable byte-diff oracle for these tax-growth experiments
- when stronger original-binary evidence becomes available, this canonical rule
  should be refined rather than treated as frozen forever
