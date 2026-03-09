# ECMAINT Combat Reference

This document records observed combat-oriented maintenance outcomes from
preserved text captures. The goal is to ground future `ECMAINT` black-box
fixtures in real historical behavior rather than invented scenarios.

## Source Set

Primary external reference:

- `/home/niltempus/Documents/esterian-conquest/ec-logs-2022/`

These files are not yet part of the repo snapshot, but they provide
high-value evidence for:

- bombardment resolution
- invasion travel state
- fleet-vs-fleet interception reports

## Reference 1: Bombardment Resolution

Files:

- `ec9.txt`
- `ec10.txt`

Observed order entry in `ec9.txt`:

- fleet `13`
- location: `Sector(23,14)`
- mission selected: `6` `Bombard a World`
- target: `System(24,14)`
- travel time shown: `1 year`
- resulting fleet-list row:
  - `13   4   1    0    4   6  Sector(23,14) Bombard world in System (24,14)`

Observed maintenance result in `ec10.txt`:

- source fleet: `13th Fleet`
- post-maint location: `System(24,14)`
- target owner: `Melody Lake` (`Empire #2`)
- target defenses at report time:
  - `6 armies`
- result:
  - `5 armies` destroyed
  - `92%` of factories destroyed
  - `100%` of stored goods destroyed
  - all ships in stardock destroyed, including `1 troop transport`
- attacker losses:
  - none
- post-maint fleet-list row:
  - `13   0   0    0    4   6  Planet(24,14) No standing orders`

Implications:

- a completed bombardment consumes the order
- the fleet remains at the target world
- bombardment can alter at least:
  - armies
  - factories
  - stored production
  - stardock contents

## Reference 2: Follow-on Invasion Travel

Files:

- `ec10.txt`
- `ec11.txt`
- `ec12.txt`

Observed order entry in `ec10.txt`:

- fleet `7`
- new orders:
  - `Invade world in System (24,14)`
- post-order fleet-list row:
  - `7    5   3   10   16   0  Planet(15,13) Invade world in System (24,14)`

Observed movement in `ec11.txt`:

- fleet-list row:
  - `7    5   2   10   16   0  Sector(19,13) Invade world in System (24,14)`

Observed movement in `ec12.txt`:

- fleet-list row:
  - `7    5   1   10   16   0  Sector(24,14) Invade world in System (24,14)`

Implications:

- the fleet brief list exposes a useful movement model:
  - location
  - speed
  - ETA
  - army count
  - ship count
  - ROE
  - standing order text
- invasion can be studied as a multi-turn transform even before we decode the
  entire internal movement representation

## Reference 3: Fleet-vs-Fleet Interception

File:

- `ec11.txt`

Observed report:

- mission context:
  - move mission
- attacker:
  - `3rd Fleet` of `In Civil Disorder` (`Empire #8`)
- friendly force:
  - `1 cruiser`
  - `1 ETAC ship`
- alien force:
  - `1 destroyer`
- result:
  - enemy fled
  - no alien ships destroyed
  - no friendly losses

Implications:

- `ECMAINT` resolves fleet-vs-fleet encounters during movement
- combat reports expose:
  - which side initiated contact
  - participating ship composition
  - flee/no-flee outcomes
  - actual losses

## Recommended Next Fixture

The best next combat-oriented black-box scenario is a simplified bombardment
fixture, because the historical evidence already gives a clear expected shape:

- order present before maintenance
- fleet arrives at target world
- order consumed after maintenance
- fleet remains at target
- world-side state materially degraded

Once that is captured, the next likely scenario should be a travel-heavy invade
fixture, followed by a fleet-vs-fleet interception fixture.
