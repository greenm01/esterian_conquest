# Esterian Conquest Canonical Combat Spec

This document defines the canonical Rust combat model for Esterian Conquest.
While the 1992 `ECMAINT.EXE` internal formulas remain unrecovered, this spec
translates the documented manuals into an auditable, seeded, and reproducible
rule set.

The intent is to preserve the spirit of the original player manuals while
adopting a clean simultaneous-resolution structure inspired by *Empire of the
Sun*. Forces meet, strike at the same instant, and survivors move forward.

Read this together with:
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [ec-timing-spec.md](ec-timing-spec.md)

## Status

This is the source of truth for:
- Fleet-vs-fleet combat
- Bombardment, Invasion, and Blitz
- Retreat and ROE interaction

It does not replace the 1992 manuals as historical sources. It defines the Rust
rule set used for maintenance runs. The turn-cycle specs decide where these
mechanics sit in the yearly loop.

## Source Basis

Primary 1992 manual references:
- [original/v1.5/ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC)
- [docs/ecmaint-combat-reference.md](ecmaint-combat-reference.md)

Manual-facing requirements:
- Combat fleets are governed by ROE thresholds (**0–10**).
- Bombardment damages production, stardock assets, and defenses.
- Invasion follows a three-stage attack:
  1. Destroy ground batteries.
  2. Soften resistance with orbital fire.
  3. Land armies to eliminate survivors.
- Blitz is a risky landing favoring overwhelming force.
- Starbases have more firepower than a battleship and absorb more hits.

## Design Goals

The canonical EC combat model:
- Preserves manual concepts: ROE, Bombard, Invade, Blitz, and Starbase defense.
- Uses a seeded campaign RNG for reproducibility.
- Produces plausible mutual attrition, not just one-sided wipeouts.
- Handles multi-empire battles without arbitrary pairwise ordering.
- Resolves simultaneous arrivals deterministically.

## Core Principles

### 1. Simultaneous Resolution
Fleet combat, orbital fire, and ground battles resolve at the same instant.

### 2. ROE Thresholds
Rules of Engagement gate his willingness to engage, his decision to break off,
and his guard posture. ROE does not inject randomness.

### 3. Aggregate Combat
EC uses ship counts, not per-hull damage state. The Rust engine uses a
**Nominal -> Crippled -> Destroyed** step-loss model during a battle. Only
destroyed hulls are written back to the save file.

### 4. Defender Wins Ties
If a tie occurs and no side has a clear edge, the defender wins.

### 5. Combined Arms
The model rewards mixed fleets (**DD/CA/BB**) with a small effectiveness bonus
over mono-type swarms.

## Combat Actors

### Fleet and Orbital Units

| Unit | AS (Attack) | DS (Defense) | Notes |
| ---- | -- | -- | ----- |
| Destroyer | 1 | 1 | Fast screen |
| Cruiser | 3 | 3 | Balanced fighter |
| Battleship | 9 | 10 | Battle line anchor |
| Scout | 0 | 1 | Non-combatant |
| Troop Transport | 0 | 1 | Vulnerable landing craft |
| ETAC | 0 | 2 | Colony ship |
| Starbase | 10 | 12 | Orbital fortress |

### Ground and Planetary Defenses

| Unit | AS | DS | Notes |
| ---- | -- | -- | ----- |
| Ground Battery | 9 | 2 | Anti-orbital cannon |
| Army | 1 | 1 | Surface combatant |

## Contact and Hostility

The model treats contact as a single contested event from a shared board state.

### Enemy vs. Hostile
- **Enemy**: A stored diplomatic stance set in the client.
- **Hostile**: A tactical state triggered by ROE, intrusions, or attacks.

Fleets attack declared enemies. They also attack when a player enters one of
his solar systems or tries to enter/leave a world he is blockading. Initiating
an attack escalates diplomacy to **Enemy** automatically.

### Interception Matrix
- **Deep Space**: Neutral fleets report contact but do not fight. Enemies are
  intercepted if ROE allows.
- **Defended Systems**: Neutral assault fleets (**Bombard/Invade/Blitz**) are
  treated as hostile intrusions. Orbit is contested immediately.
- **Blockades**: Any fleet attempting to pass a blockade boundary triggers an
  orbital contest.

## Seeded CRT Resolution

Combat derives its results from the `campaign_seed` plus battle context. Each
exchange rolls a `d10` against the Combat Results Table (CRT).

### The CRT
The model selects a column based on the force ratio and applies modifiers:
- **Disadvantaged**: Ratio < 0.5
- **Pressed**: Ratio 0.5 – 1.0
- **Even**: Ratio 1.0 – 1.5
- **Advantaged**: Ratio 1.5 – 3.0
- **Overwhelming**: Ratio >= 3.0

Modifiers shift the column (e.g., **+1** for mixed fleets or starbases). An
unmodified **9** is a critical hit, forcing at least one real loss.

## Hit Allocation

### Screened Fleet Allocation
1. Nominal ships are reduced to crippled before any crippled ships are
   destroyed.
2. Fire targets the combat line (**DD, CA, BB, SB**) first.
3. Auxiliaries (**SC, TT, ET**) are screened until the combat line collapses.
4. Target selection favors the lowest **DS** first.

### Bombardment Priority
1. Stardock contents (docked ships).
2. Ground batteries.
3. Armies.
4. Stored goods and factories.

### Invasion Priority
1. Orbital suppression (Batteries).
2. Softening fire (Armies, goods, factories).
3. Landing battle (Armies).

## Fleet Combat Sequence

1. **Identify Participants**: Combine all participating fleets into task forces.
2. **Pre-Round ROE**: Fleets failing ROE attempt to withdraw.
3. **Simultaneous Fire**: Both sides generate and apply hits.
4. **Post-Round ROE**: Survivors check if they can still meet their thresholds.
5. **End State**: Combat ends when one side is destroyed or retreats.

## Simultaneous Arrival at a Planet

No empire executes an assault (**Bombard, Invade, Blitz**) until it achieves
orbital supremacy. If multiple hostile empires arrive at once, they must
contest the orbit before attacking the world. A planet can change ownership at
most once per turn.

## Summary

This model provides a clear, auditable alternative to the 1992 engine's opaque
RNG. It preserves the classic mechanics while ensuring outcomes are
reproducible and fair.
