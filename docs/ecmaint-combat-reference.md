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

Synthetic-fixture correction from the current repo work:

- the first synthetic bombardment fixture used a target cloned from an
  initialized seeded colony shell
- that was enough to trigger a two-pass attack lifecycle and attacker losses
- it was not enough to produce planet-side damage or player-facing reports
- comparison against the shipped mature world `Dust Bowl` at `(16,13)` shows
  that a developed colony has a different tail/state block than the seeded
  shell used in the synthetic target

Practical consequence:

- the next bombardment fixture should be built around a mature colony-style
  target record, not another initialized homeworld seed clone
- any follow-up fixture should compare at least:
  - the mature planet record in `PLANETS.DAT`
  - the corresponding `DATABASE.DAT` state
  - any empire/ownership linkage needed to make the target world fully valid

Additional correction from the throwaway `Dust Bowl` clone test:

- cloning a mature colony record by itself is not sufficient if the target is
  effectively friendly to the attacker
- when the shipped mature colony `Dust Bowl` was cloned onto the synthetic
  target coordinates, `ECMAINT` converted the attacker's standing order from
  `bombard` to `guard/blockade` on arrival and never produced world-side damage
- the next combat fixture therefore needs a mature enemy colony, not merely a
  mature colony

Further correction from the hybrid mature-enemy throwaway test:

- replacing the likely empire-linked bytes with those from an enemy seed shell
  was enough to preserve the `bombard` order through arrival and then resolve
  an attack on the second pass
- however, even that hybrid target still produced no `PLANETS.DAT` damage and
  no `MESSAGES.DAT` / `RESULTS.DAT` output

Current interpretation:

- ownership hostility is necessary, but not sufficient
- the remaining missing state is most likely inside other developed-world
  fields in `PLANETS.DAT` rather than in the derived `DATABASE.DAT` intel cache

Field-isolation follow-up:

- a later synthetic variant set the target world's candidate army byte
  (`PLANETS.DAT[0x5A]`) to `0` while keeping the hostile mature target shape
- that variant still consumed the bombard order, but it no longer inflicted any
  attacker ship losses
- instead, the target planet record itself changed in several bytes, including
  a decrement at `0x58`

Why it matters:

- this is the strongest current evidence that `PLANETS.DAT[0x5A]` is tied to
  planetary defensive strength, likely armies
- it also confirms that bombardment world-side damage lives in `PLANETS.DAT`,
  not only in fleet-side attrition or derived database output

Second field-isolation follow-up:

- keeping the zero-army target but also forcing `PLANETS.DAT[0x58] = 0`
  preserved the same no-loss fleet outcome
- however, it changed the planet-side damage pattern again

Why it matters:

- `0x58` now looks like an active bombardment damage/development field rather
  than a passive maturity byte
- current best black-box model is:
  - `0x5A` strongly affects defender resistance and attacker losses
  - `0x58` strongly affects the world-side damage pattern once resistance is
    low enough that the attacker survives intact

Third field-isolation follow-up:

- setting the same target's candidate army byte to `1` instead of `0`
  produced an intermediate bombardment outcome
- the bombard order was still consumed on the second pass
- the fleet reached the target, but now took partial losses:
  - `CA 3 -> 2`
  - `DD 5 -> 2`
- the target planet also changed in a richer way than either zero-army case:
  - `0x04..0x07`: `00 00 00 00 -> 3d 3d cc 03`
  - `0x08..0x09`: `48 87 -> 3d 85`
  - `0x0A..0x0D`: `00 00 00 00 -> 44 3e bc ac`
  - `0x0E`: `04 -> 46`
  - `0x58`: `8e -> 8d`
  - `0x5A`: `01 -> 00`

Why it matters:

- this gives a three-point progression for the same hostile mature target:
  - `0x5A = 0`: no attacker losses
  - `0x5A = 1`: partial attacker losses
  - stronger baseline target: heavier attacker losses
- that is the clearest current evidence that `PLANETS.DAT[0x5A]` acts like a
  graded defense or army-strength field, not just a binary presence flag

Fourth field-isolation follow-up:

- keeping the same `army1` target but also forcing `PLANETS.DAT[0x58] = 0`
  produced yet another distinct bombardment outcome
- the order was still consumed and the fleet still ended at the target
- attacker losses became lighter than the plain `army1` case:
  - `CA 3 -> 2`
  - `DD 5 -> 4`
- the target world's changing bytes also shifted again:
  - `0x04..0x07`: `00 00 00 00 -> 4f 4c 55 ba`
  - `0x08..0x09`: `48 87 -> 3a 86`
  - `0x0A..0x0D`: `00 00 00 00 -> 06 ea 29 25`
  - `0x0E`: `04 -> 35`
  - `0x58`: stayed `0`
  - `0x5A`: `01 -> 0`

Why it matters:

- `0x58` is now implicated in more than just post-hit world-state encoding
- on the `army1` target, zeroing `0x58` also reduced the destroyer losses
  from `5 -> 2` down to `5 -> 4`
- current best black-box model is:
  - `0x5A` scales defender resistance
  - `0x58` modulates both world-side damage and at least part of the
    attacker-loss calculation

Fifth field-isolation follow-up:

- keeping `0x58 = 0` and `0x5A = 1`, but changing the target's byte `0x0E`
  from `0x04` to `0x0c`, produced another distinct bombardment result
- the order was still consumed and the fleet still ended at the target
- attacker losses became much heavier:
  - `CA 3 -> 3`
  - `DD 5 -> 1`
- the world-damage window changed again:
  - `0x04..0x07`: `00 00 00 00 -> 8b 15 60 b5`
  - `0x08..0x09`: `48 87 -> 3e 86`
  - `0x0A..0x0D`: `00 00 00 00 -> d8 c6 49 e3`
  - `0x0E`: `0x0c -> 0x54`
  - `0x58`: stayed `0`
  - `0x5A`: `0x01 -> 0`

Why it matters:

- with `0x58` and `0x5A` held constant, `0x0E` alone now clearly changes the
  attacker-loss profile
- that makes `PLANETS.DAT[0x0E]` a strong candidate for another defense-side
  field, possibly something like ground batteries or a related installation

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
