# Esterian Conquest Canonical Economics Spec

This document defines the canonical Rust economy model for Esterian Conquest.
It is the project’s auditable economy rule. While the 1992 `ECMAINT.EXE` formulas
remain opaque, this spec translates the documented manuals into an explicit
Rust rule set.

Read this together with:
- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [ec-combat-spec.md](ec-combat-spec.md)

## Status

This is the source of truth for:
- Production terminology
- Opening homeworld setup
- Empire-wide tax behavior
- Yearly revenue and growth
- Starbase effects on build capacity and growth

The starbase **5x** rule is settled:
- The manuals tie **5x** to build capacity.
- Ghidra analysis of the 1992 binaries found no evidence for a starbase growth
  multiplier.
- Black-box sweeps in [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
  confirm the commissioned starbase grants a **5x** build-capacity bonus.
- Starbases improve the yearly **grow** allowance under tax pressure.
- This document defines the canonical Rust policy where classic recovery is
  incomplete.

This spec does not replace the 1992 manuals as historical sources. It does
not define the full maintenance turn order. It defines the economy and build
policy that runs inside the recovered post-loop world update region.

## Turn-Order Placement

The recovered yearly turn order constrains economy and build behavior:
- Build completion and economy run in the **post-loop world update region**.
- This occurs **after** the weekly 52-pass fleet combat loop.
- This occurs **before** the hostile world-resolution region (Bombard, Invade,
  Blitz).

### Impact:
- Ships and starbases enter stardock before a hostile assault hits the planet.
- Bombardment and invasion see post-build planet state.
- Armies and batteries completed in this region go straight to the planet surface.
- Later hostile actions read these units as planet state, not stardock inventory.

## Source Basis

Primary 1992 manual references:
- [original/v1.5/ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC)

Manual-facing requirements:
- Each player begins with one homeworld at current production **100**.
- The tax rate is empire-wide.
- Taxes generate yearly production points.
- Newly colonized planets start below potential.
- Lower taxes improve planetary development.
- Taxes above **65%** can harm current production.
- Starbases:
  - Help planets endure tax burden.
  - Grant a **5x** build-capacity multiplier.
  - Help underdeveloped planets grow faster.

## Canonical Terms

Use these player-facing terms:
- **Present Production**: Current productive capacity.
- **Potential Production**: Maximum productive capacity.
- **Total Available Points**: Spendable tax revenue budget for the current turn.
- **Stored Production Points**: Accumulated points stored on a planet.

Avoid low-level storage nicknames like `factories` in player surfaces.

## Opening Economy

Rust starts encode the intended opening economy:
- Homeworld Potential Production: **100**.
- Homeworld Present Production: **100**.
- Default Tax Rate: **50%**.
- First-Turn Total Available Points: **50**.
- Homeworld Defenses: **10** armies, **4** ground batteries.

## Empire-Wide Tax

The tax rate is stored on the player record. He chooses one rate for all his
planets.

Yearly revenue per planet:
`revenue = floor(present_production * tax_rate / 100)`

Empire Total Available Points:
`total = sum(revenue)` across all owned planets.

## Present Production Growth and Tax Pressure

Every owned planet grows toward its potential production each maintenance turn.

### The Canonical Rust Rule:

1. **Growth Gap**: `gap = potential_production - present_production`
2. **Tax Headroom**: `headroom = 100 - min(tax_rate, 95)`
3. **Base Growth**: `base = ceil(gap * headroom / 400)`
4. **Starbase Bonus**:
   - Apply a bonus if a friendly starbase is in orbit.
   - Tax <= 50%: **+50%** bonus.
   - Tax 51%–64%: Linearly taper bonus to 0%.
   - Tax >= 65%: No bonus.
   `growth = base + ceil(base * bonus_percent / 100)`
5. **Clamp**: Minimum growth is **1**; maximum is the remaining gap.
6. **New Production**: `production = present_production + growth`
7. **High-Tax Penalty**:
   - If tax > 65%, calculate penalty: `penalty = ceil(production * (tax - 65) / 500)`
   - Minimum penalty is **1**.
   - Penalty cannot exceed current production.

Final result:
`present_production = (present_production + growth) - penalty`

### Impact:
- Lower taxes produce faster growth.
- Higher taxes yield immediate revenue but slow development.
- Taxes above **65%** directly reduce production.
- Starbases accelerate development at low and moderate tax.
- Starbases do not grant an exemption from high-tax penalties.

## Starbase Effects

### 1. Growth Bonus
An active, commissioned starbase in orbit provides a tax-sensitive growth
bonus. It applies **+50%** of base growth at tax <= 50%, tapering to 0% at tax
65%. This is the canonical Rust policy.

### 2. Build Capacity Multiplier
A starbase lets a planet spend up to **5x** its current production on units
in a single turn. Without a starbase, capacity is **1x**. This affects build
completion, not tax revenue.

## Stored Production Points

Yearly tax revenue is added to each planet’s stored production pool. This is
separate from the empire’s Total Available Points view.

When maintenance processes a build queue, the planet spends from that stored
pool by the number of build points actually applied that year. If a build cost
is larger than the planet’s current per-turn build capacity, only that yearly
processed amount is deducted and the remaining cost stays queued for later
turns. If a build is blocked and no work is applied, stored production is not
consumed.

## Special Cases

### Homeworlds
Homeworlds start at full production from turn one.

### Civil Disorder and Autopilot
The classic autopilot pass handles disorder. Empires in disorder (**0x00**) are
economically frozen. Normal-planet economy policy does not apply to them.

## Unit Build Completion

When a build queue finishes, units are dispatched:

- build spending is applied during maintenance from Stored Production Points
- partial progress carries the remaining build cost into later turns
- blocked builds remain queued and do not consume stored production that year

### Ships and Starbases (Stardock)
Destroyers, cruisers, battleships, scouts, transports, ETACs, and starbases
enter the planet's stardock.
- The player must commission them before use.
- Uncommissioned units can be destroyed by bombardment.
- If stardock is full, the build remains in queue. Rust avoids the classic
  stardock corruption bug.

### Armies and Batteries (Planet Surface)
Armies and ground batteries go directly to the planet surface. They do not
require commission.
- A full stardock does not block them.
- Planet armies and batteries are capped at **255**.
- If a build would exceed the cap, it stays in the queue.

## Validation

The canonical model is backed by regression tests and the economy tax probe
tool. Refine this rule only when stronger 1992 evidence becomes available.
