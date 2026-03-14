# Esterian Conquest Canonical Combat Spec

This document defines the **canonical Rust combat model** for Esterian
Conquest. It is not a claim that the original `ECMAINT.EXE` used these exact
internal formulas. It is the project’s auditable, deterministic combat design
for the mechanics that the original game resolved stochastically.

The intent is to preserve the **spirit** of the original player manuals while
adopting a cleaner simultaneous-resolution structure inspired by
*Empire of the Sun*.

That debt is deliberate. Where the original game often hid combat in opaque
RNG and processing order, this spec assumes a more legible universe: forces
meet, both strike from the same instant, and history is written from the
survivors.

## Status

This is a design spec for implementation. It is the source of truth for:

- fleet-vs-fleet combat
- bombardment
- invasion
- blitz
- retreat / ROE interaction

It does **not** replace the original manuals as historical sources. It
translates them into a deterministic Rust rule set suitable for reproducible
maintenance runs.

## Source Basis

Primary Esterian Conquest guidance:

- [original/v1.5/ECPLAYER.DOC](/home/mag/dev/esterian_conquest/original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](/home/mag/dev/esterian_conquest/original/v1.5/ECQSTART.DOC)
- [original/v1.5/WHATSNEW.DOC](/home/mag/dev/esterian_conquest/original/v1.5/WHATSNEW.DOC)
- [docs/ecmaint-combat-reference.md](/home/mag/dev/esterian_conquest/docs/ecmaint-combat-reference.md)

High-value original-behavior cues from those sources:

- combat fleets are governed by ROE, with the exact 0-10 engagement thresholds
- bombardment is meant to damage production, orbiting assets, and defenses
- invasion is explicitly a three-stage attack:
  1. destroy ground batteries
  2. soften resistance with bombardment
  3. land armies to fight surviving armies
- blitz is a riskier, faster landing that favors overwhelming transport force
- starbases have slightly more firepower than a battleship and can take more hits
- v1.50 specifically improved combat so large fleets no longer erase small fleets
  without taking some damage of their own

External structural inspiration:

- *Empire of the Sun* simultaneous combat flow:
  - both sides generate hits from attack strength
  - hits are applied simultaneously in the normal case
  - defender/reaction side wins ties
  - ground combat is a separate simultaneous step

## Design Goals

The canonical EC combat model shall:

- feel like classic EC, not like a generic 4X skirmish engine
- preserve manual-facing concepts: ROE, bombard, invade, blitz, starbase defense
- be deterministic and reproducible from save-state bytes alone
- produce plausible mutual attrition rather than one-sided wipeouts
- keep combat math explicit and inspectable
- avoid requiring hidden RNG state or per-ship tactical simulation
- handle multi-empire battles in one system without requiring pairwise ad hoc
  ordering
- define simultaneous-arrival resolution at planets so identical orders from
  multiple empires resolve deterministically

## Core Principles

### 1. Simultaneous resolution is the default

Fleet combat, orbital fire, bombardment return fire, and ground combat all
resolve simultaneously unless a future mechanic explicitly says otherwise.

### 2. ROE determines commitment, not die rolls

ROE is preserved from the original manuals as the fleet’s willingness to stay
in the fight. It does not inject randomness. It gates:

- whether a fleet chooses to engage
- whether it breaks off after a round
- whether guarding fleets hold or peel off to pursue

### 3. EC uses aggregate combat, not persistent ship damage states

The original save files store ship **counts**, not per-hull damage state.
Therefore this spec uses **virtual step damage** inside a battle only:

- each unit contributes one or more fresh steps derived from `DS`
- hits must first exhaust the fresh step across eligible targets
- only then do destroyed hulls appear
- only destroyed hull counts are written back to the save

This preserves the EOTS-style “cripple before destroy” feel without inventing a
new on-disk crippled-state system.

`DS` is therefore not cosmetic. It defines how much temporary in-battle
durability a unit contributes before actual on-disk losses occur.

### 4. Defenders win ties

Where a winner must be chosen and no side has a clear post-resolution edge, the
defender/reaction side wins the tie.

### 5. Mixed fleets should matter

The manuals repeatedly imply that mixed fleets use tactics unavailable to
single-type fleets. The canonical model rewards combined DD/CA/BB fleets with a
small effectiveness edge over mono-type swarms.

## Combat Actors

### Fleet and orbital units

The canonical combat tables use two explicit values:

- `AS`: attack strength
- `DS`: defensive strength

These are abstract combat values, not build costs.

| Unit | AS | DS | Notes |
| ---- | -- | -- | ----- |
| Destroyer | 1 | 1 | Fast escort / screen |
| Cruiser | 3 | 3 | Roughly 3x destroyer power from manual text |
| Battleship | 9 | 10 | Roughly 3x cruiser power; primary battle line |
| Scout | 0 | 1 | Non-combat, but can be lost if screen collapses |
| Troop Transport | 0 | 1 | Non-combat, vulnerable during landings |
| ETAC | 0 | 2 | Large and expensive, but not a combat ship |
| Starbase | 10 | 12 | Slightly more firepower and durability than battleship |

### Ground and planetary defenses

| Unit | AS | DS | Notes |
| ---- | -- | -- | ----- |
| Ground Battery | 9 | 2 | Land cannon with roughly battleship-scale firepower |
| Army | 1 | 1 | Surface defense and invasion combatant |

### Planetary economic targets

These are not combatants in the fleet sense, but bombardment may damage them.

- stored goods
- factories / development
- stardock contents

Their exact byte-level damage formulas remain implementation details, but the
priority order is defined below.

## Simultaneous Contact Doctrine

Esterian Conquest is a simultaneous-orders game. The canonical combat model
therefore treats contact in one location as a **single contested event** from a
shared board state, not as an arbitrary sequence of pairwise skirmishes based
on file order.

This doctrine applies to:

- deep-space fleet combat
- orbital combat at a planet
- simultaneous arrivals at a defended or blockaded world
- multi-empire encounters in open space or orbit

### Enemy vs hostile

The original manuals distinguish between a player you have declared an
`enemy` and a fleet that is currently `hostile` for combat purposes.

From the manuals:

- players may be declared `neutral` or `enemy` in `ECGAME`
- fleets automatically attack fleets belonging to declared enemies when they
  are encountered
- fleets always fight back if attacked
- fleets also attack when another player's fleets enter one of their solar
  systems, or when another player's fleets try to enter or leave a world they
  are blockading

Canonical interpretation:

- `enemy` is a stored diplomatic stance
- `hostile` is the broader tactical category used by ROE and contact resolution
- declared enemies are hostile on encounter
- some contacts become hostile even without prior enemy declaration:
  - entering an empire's solar system
  - entering or leaving a world under blockade
  - attacking first
- attacking another player's fleet or planet should also escalate diplomacy:
  once one empire initiates offensive action against another, both empires are
  treated as enemies for later encounters unless some future diplomacy system
  explicitly allows de-escalation

This spec therefore uses `hostile` for combat eligibility and `enemy` only
for the narrower diplomatic declaration concept.

Current implementation note:

- Rust now has a typed stored-diplomacy seam for this distinction
- the actual persisted `PLAYER.DAT` bytes for enemy/neutral status are still
  unresolved
- until those bytes are mapped, the live engine still preserves a canonical
  foreign co-location fallback after applying the manual defensive triggers

### Shared contact rules

When two or more hostile empires are present in the same location in the same
maintenance step:

- all hostile empires are evaluated from the same start-of-step board state
- each empire forms one combined **task force** from its participating fleets
  and local fixed defenses
- diplomatic hostility still matters: non-hostile empires are present but do
  not exchange fire
- combat rounds are resolved simultaneously between task forces, not in hidden
  initiative order
- file order, fleet ID, and mission code never determine who fires first

Any fleet encounter, whether hostile or not, should still generate a contact
or intelligence report for the empires that observed it. Contact reporting is
broader than combat reporting.

### Shared tie-break rules

If a battle needs a reaction-side concept for tie handling:

1. incumbent local defenders win ties
2. if there is no incumbent defender, guarding or blockading forces win ties
3. if there is still no defender posture, the surviving task force with the
   highest combat AS wins
4. if still tied, the lowest empire number wins

### Shared post-contact rules

After hostile combat resolves:

1. remove destroyed fleets and structures
2. apply retreats and disengagement outcomes
3. only then apply friendly merges, rendezvous resolution, or mission
   completion side effects

This ensures that fleets do not benefit from a friendly merge or landing that
should not occur before enemy contact is resolved.

## ROE

The canonical ROE thresholds remain exactly aligned with the player manual:

| ROE | Engage if friendly combat AS is at least... |
| --- | ------------------------------------------- |
| 0 | never voluntarily engage |
| 1 | enemy is defenseless |
| 2 | 4:1 |
| 3 | 3:1 |
| 4 | 2:1 |
| 5 | 3:2 |
| 6 | 1:1 |
| 7 | 2:3 |
| 8 | 1:2 |
| 9 | 1:3 |
| 10 | always engage |

Interpretation rules:

- non-combat fleets have effective ROE `0`
- `combat AS` for ROE checks is the sum of `DD`, `CA`, `BB`, and defending
  `Starbase` strength only
- fleets that are directly attacked always return fire in that round
- guarding / blockading fleets count as defenders and do not pre-emptively flee
  before the first exchange
- after each round, surviving fleets may disengage if their post-loss ratio no
  longer meets their ROE threshold

## Combat Effectiveness Ratings

To keep the EOTS flavor, both sides convert their raw attack strength into hits
through a deterministic combat effectiveness rating (`CER`).

### Space / orbital CER

Base `CER` is determined from the side’s current combat posture:

| Condition | CER |
| --------- | --- |
| badly overmatched (`AS ratio < 1:2`) | 0.50 |
| under pressure (`1:2` to `< 1:1`) | 0.75 |
| even fight (`1:1` to `< 3:2`) | 1.00 |
| local advantage (`3:2` to `< 3:1`) | 1.25 |
| overwhelming advantage (`>= 3:1`) | 1.50 |

Then apply these deterministic modifiers:

- `+0.25` if the side fields at least two combat ship classes among `DD`, `CA`,
  `BB`
- `+0.25` for an undamaged defending starbase in orbital combat
- `-0.25` if the side has no combat ships and is firing only with a starbase or
  batteries

Clamp final space/orbital `CER` to `0.25 .. 1.50`.

For this purpose, an `undamaged` starbase is one that has lost no fresh steps
in the current battle.

### Ground CER

Ground combat is bloodier and simpler:

| Condition | CER |
| --------- | --- |
| badly overmatched (`AS ratio < 1:2`) | 0.50 |
| under pressure (`1:2` to `< 1:1`) | 1.00 |
| local advantage (`1:1` to `< 2:1`) | 1.50 |
| overwhelming advantage (`>= 2:1`) | 2.00 |

Ground modifiers:

- `+0.50` defender bonus for a blitz defense
- `+0.25` attacker bonus for invade after all batteries have been destroyed
- `-0.25` attacker penalty if transports land while any batteries still survive
  under a blitz

Clamp final ground `CER` to `0.50 .. 2.00`.

These modifiers stack with the ratio-based ground `CER` and are then clamped.

## Hit Generation

In every simultaneous step:

`hits = ceil(total_AS * CER)`

Where `total_AS` is the sum of all participating units in that step.

Each side computes hits independently from the same pre-hit board state for that
step. Losses are then applied simultaneously.

## Hit Allocation

### Virtual two-step hull rule

Each eligible ship, starbase, battery, or army contributes:

- a number of fresh steps derived from `DS`
- one destroyed step

Fresh-step count is:

`fresh_steps = max(1, ceil(DS / 6))`

With the current canonical table, this yields:

- `DS 1-6` -> 1 fresh step
- `DS 7-12` -> 2 fresh steps

That means destroyers, cruisers, scouts, transports, ETACs, batteries, and
armies have 1 fresh step, while battleships and starbases have 2 fresh steps.
This is the intended way the model expresses the manual claim that starbases
and battleships withstand more punishment than lighter units.

Hits are allocated in two passes:

1. hits first remove fresh steps from eligible targets according to priority
2. once a target has no fresh steps left, later hits destroy units and reduce
   on-disk counts

Any partially used fresh-step damage disappears when the battle ends. Only
destroyed units persist.

This is the deliberate abstraction that gives EC the “large fleets still take
some damage” feel without introducing persistent crippled hull state.

### Target priority: fleet combat

Eligible targets are allocated in this order:

1. destroyers
2. cruisers
3. battleships
4. starbases
5. scouts
6. troop transports
7. ETACs

Rationale:

- escorts screen heavier and softer assets
- combat ships protect auxiliaries, matching the player manual
- starbases stand in the main line once orbital combat begins

Non-combat ships are still valid targets so long as they have `DS > 0`.

### Multi-empire targeting rule

When one side has multiple hostile opponents in the same round, it allocates
its generated hits against exactly one hostile task force using this priority:

1. hostile force currently blockading or guarding the contested world
2. hostile force threatening the side's own planet or starbase
3. hostile force with the highest combat AS
4. hostile force with the lowest empire number

This keeps allocation deterministic and avoids fractional hit-splitting across
multiple targets in one round.

### Target priority: bombardment

Bombardment hits are allocated in this order:

1. stardock ships in orbit
2. ground batteries
3. armies
4. stored goods
5. factories / development

Rationale:

- the manuals describe bombardment as damaging orbiting assets and production
- observed fixtures show bombardment also kills batteries and armies
- bombardment is meant to cripple a world even when not capturing it

The exact bombardment weights and planetary return-fire formulas below are
canonical Rust combat rules. They are intended to match the manuals' relative
roles and observed fixture ranges, but they are not claimed as recovered
original `ECMAINT` formulas.

### Target priority: invasion

Invasion resolves in three stages:

1. orbital suppression
2. softening fire
3. landing battle

Orbital suppression targets batteries first.

Softening fire targets:

1. armies
2. stored goods
3. factories / development

Landing battle targets armies only.

### Target priority: blitz

Blitz skips the full deliberate orbital-suppression program of an invasion, but
it still includes a brief cover-fire / distraction step before the transports
commit to the descent.

The canonical blitz sequence is:

1. a brief light cover-fire step from escorting combat ships against batteries
2. landing under fire from any batteries that survive that cover fire
3. immediate ground combat between landed attackers and defending armies

Defender fire under blitz targets:

1. troop transports
2. escort combat ships
3. landed armies

Attacker fire under blitz targets:

1. armies
2. batteries

This makes blitz faster, riskier, and more dependent on army superiority, which
matches the manuals: combat ships help the drop, but they do not reduce the
planet the way a full invade does.

## Fleet-Vs-Fleet Combat

Fleet-vs-fleet combat resolves in up to three rounds.

### Step 1: Identify participants

Participants include:

- fleets that voluntarily engage under ROE
- fleets that are directly attacked or intercepted
- guard/blockade fleets at the battle location
- starbases only if the battle is in orbital defense of their world

If multiple empires are present, each empire contributes one task force made
from all participating fleets at that location.

### Step 2: Pre-round disengagement

Fleets that do not meet ROE for voluntary engagement attempt to avoid battle.
If the opposing side contains an intercepting guard/blockade fleet, the
withdrawing fleet still suffers one **pursuit fire** exchange before escaping.
In that exchange, the pursuer's `CER` is forced to `0.50`; the withdrawing side
uses no special pursuit modifier.

### Step 3: Simultaneous fire

Both sides generate hits and apply them simultaneously.

For three-way or four-way fights:

- each task force computes hits once from its own current state
- each task force selects one hostile target by the multi-empire targeting rule
- all chosen attacks are then applied simultaneously
- a task force may be targeted by multiple hostile empires in the same round

### Step 4: Post-round morale / ROE check

After losses, each fleet checks ROE again against the surviving enemy combat AS.

- fleets still meeting ROE may remain engaged
- fleets failing ROE attempt to disengage
- defending guard/blockade fleets get one free hold attempt before breaking

### Step 5: End state

Combat ends when:

- one side has no combat-capable force remaining
- all remaining fleets on one side disengage
- three rounds have been resolved as a hard safety cap

Winner determination:

- the side with remaining combat AS in the contested location wins
- if both sides still remain after round three, the defender wins ties

For multi-empire battles:

- if exactly one hostile task force remains willing and able to fight, it wins
- if multiple hostile task forces remain after round three, the local defender
  or incumbent blockader wins the tie
- if there is no incumbent defender or blockader, the surviving task force with
  the highest combat AS holds the system
- if still tied, lowest empire number wins

## Simultaneous Fleet Arrival In Open Space

This section covers non-planet fleet contact when multiple empires enter the
same sector or system in the same maintenance tick without a planetary assault
being the immediate focus.

Examples:

- two empires `Move` into the same location
- one empire `Patrol`s while two hostile empires pass through
- several fleets `Rendezvous` at the same sector but belong to hostile empires
- a `Seek Home` fleet and a `Move` fleet reach the same system together

### Open-space contest rule

When hostile fleets from multiple empires occupy the same non-planet location at
the same step, resolve one shared fleet battle event from the simultaneous
start-of-step state.

There is no hidden initiative order based on:

- fleet ID
- empire number
- file order
- mission code

Those values may break ties only after combat, never before it.

### Attacker / defender posture in open space

Open-space battles still need a reaction-side concept for tie handling. Use:

1. fleets already present in the location at tick start are defenders
2. if no one was already present, fleets on `Patrol`, `Guard Starbase`, or
   `Guard/Blockade` count as defenders against pure movers
3. if all hostile fleets arrived simultaneously and no one has a guarding
   posture, there is no natural defender; ties go to the side with the highest
   surviving combat AS, then lowest empire number

### Same-mission simultaneous arrivals

If multiple hostile empires arrive with the same mission in open space, they do
not cooperate and do not pass through one another.

Examples:

- two `Move` fleets reach the same system: resolve open-space combat normally
- two `Rendezvous` groups from hostile empires reach the same sector: resolve
  combat before any same-empire merge occurs
- two `Seek Home` fleets from hostile empires reach the same refuge world’s
  orbit: resolve combat unless diplomacy says they are not hostile

### Merge and rendezvous timing

Friendly merge-style effects happen only after hostile combat is resolved.

Order of operations for same-location fleet processing:

1. collect all fleets present after movement
2. resolve hostile fleet combat
3. remove destroyed / retreated fleets
4. only then apply same-empire join / rendezvous / friendly merge logic

This prevents a fleet from gaining extra protection or firepower from a merge
that should not have happened before enemy contact.

### Transit and interception

If a moving fleet passes through or arrives in a location containing a hostile
patrol or blockade force, the patrol/blockade force is treated as the local
defender for tie purposes.

If multiple hostile moving fleets and a local patrol all coincide:

- all hostile task forces participate in the same open-space contest
- the patrol side retains defender priority on ties

### Outcome

After the open-space battle:

- surviving fleets that disengaged continue with their mission only if the
  mission is still valid and the fleet remains combat-capable enough to do so
- surviving fleets that lost and were forced to withdraw receive a retreat /
  seek-home style post-battle destination as defined by implementation
- if no battle occurs because all ROE checks refuse engagement, fleets coexist
  only if diplomacy allows it; otherwise patrol/blockade interception forces a
  one-round pursuit-fire exchange

### Crossing paths during movement

The original manuals describe fleets as being encountered when they meet, but
they do not spell out a precise sub-turn interception geometry for fleets whose
movement paths merely cross between starting and ending sectors.

Canonical Rust rule for now:

- movement resolves first
- hostile contact is then evaluated from the final post-movement board state
- fleets engage if they occupy the same final location after movement
- fleets do not currently engage solely because their movement paths crossed
  between two different final locations in the same tick
- if fleets occupy the same final location and are enemies, they should resolve
  hostile contact automatically
- if fleets occupy the same final location and are not enemies, they should
  still generate encounter intel but should not perform hostile operations
  unless one side attacks or a defensive hostility rule applies

So:

- same final sector or same final orbit this tick: contact may occur
- one fleet passes through while the other departs and they end in different
  places: no combat from path crossing alone

This is intentionally narrower than a full interception geometry model. If
later RE or oracle work proves true mid-path interception rules, this section
should be revised explicitly rather than inferred from ROE alone.

## Simultaneous Arrival At A Planet

This section defines what happens when more than one empire reaches the same
planet in the same maintenance step, whether by movement, bombardment, invade,
blitz, or blockade-style missions.

### Arrival classes

Arriving fleets are grouped into these mission classes:

1. `Guard/Blockade`
2. `Bombard`
3. `Invade`
4. `Blitz`
5. other fleet-presence missions

### Planet contest sequence

When multiple empires are present at the same planet in the same step, resolve:

1. incumbent defenders already at the world
2. newly arrived orbital contestants
3. bombard / invasion / blitz against the planet only after orbital supremacy
   is established

This preserves the manual spirit that you do not freely land troops or pound a
world while hostile fleets still contest orbit.

### Orbital supremacy gate

No empire may execute bombardment, invasion, or blitz against the planet unless
it is the sole remaining hostile force in orbit after the orbital-combat step.

If multiple hostile empires survive in orbit after the round limit:

- the incumbent defender or blockader is treated as retaining orbital control
- otherwise, no one achieves orbital supremacy this tick
- all assault missions remain pending if they still have valid surviving fleets

At most one empire may proceed to bombardment, invasion, or blitz in any one
maintenance tick.

### Same-mission simultaneous arrivals

If two or more empires arrive with the same assault mission against the same
planet in the same tick:

- they do **not** cooperate
- they first resolve orbital combat as hostile independent task forces
- only the empire that achieves orbital supremacy may continue to bombard,
  invade, or blitz that turn

Examples:

- two empires arrive to `Bombard`: they fight for orbit first; only the winner
  bombards
- two empires arrive to `Invade`: they fight for orbit first; only the winner
  may begin battery suppression and landing
- two empires arrive to `Blitz`: they fight for orbit first; only the winner
  attempts the fast landing

### Mixed-mission simultaneous arrivals

If different hostile empires arrive with different assault missions, orbit is
still resolved first. Mission type matters only after one empire controls orbit.

Priority after orbital supremacy:

1. `Blitz`
2. `Invade`
3. `Bombard`
4. `Guard/Blockade`

Interpretation:

- `Blitz` and `Invade` are active capture attempts and outrank pure
  bombardment if the same empire somehow has multiple eligible assault fleets
- `Bombard` outranks passive guard posture once orbit is secured
- if one empire has multiple surviving fleets with different assault missions,
  it may execute only the highest-priority assault class in that tick

### Planet ownership and contested assaults

A planet can change ownership at most once per maintenance tick.

If one empire captures a planet during the tick:

- surviving assault fleets from other empires do not immediately re-resolve a
  second capture in the same tick
- they remain in orbit and will contest the newly captured world on the next
  maintenance tick if still hostile and eligible

This avoids ping-pong ownership changes inside one simultaneous turn.

## Bombardment

Bombardment is a one-turn orbital attack by combat ships already at the world.
It is resolved in one simultaneous bombardment step per maintenance tick.

### Bombardment attacker AS

Only `DD`, `CA`, and `BB` contribute bombardment AS.

Apply these mission weights:

- destroyer bombardment AS uses `0.5x`
- cruiser bombardment AS uses `1.0x`
- battleship bombardment AS uses `1.5x`

Scouts, ETACs, and transports do not contribute bombardment fire.

### Bombardment defender AS

Planetary return fire AS is:

`battery_AS + ceil(army_AS / 2)`

This captures the manual idea that batteries are the primary anti-orbital weapon
while armies contribute some resistance.

This return-fire formula is also a canonical Rust combat rule rather than a
claimed original-engine formula.

### Bombardment outcome rules

- bombardment always consumes the bombard order once the fleet is already in orbit
- the fleet remains at the target world
- bombardment can destroy orbiting stardock ships
- bombardment can reduce batteries, armies, goods, and factories
- planets may survive bombardment intact enough to remain enemy-held

## Invasion

Invasion is explicitly a three-stage attack, matching the player manual.

### Stage 1: Orbital suppression

Attacker combat ships exchange simultaneous fire with ground batteries.

- if batteries survive, transports do not land this turn
- the mission still inflicts planetary damage through suppression fire

### Stage 2: Softening fire

If all batteries are destroyed, surviving combat ships inflict bombardment-style
softening damage on armies and industry.

This represents the manual’s “pound the population centers a little to soften
resistance”.

### Stage 3: Landing battle

Loaded troop transports deliver armies only after batteries are gone.

Ground combat then resolves simultaneously:

- attacker AS = landed armies
- defender AS = surviving planetary armies
- ties favor the defender

This landing battle uses the ground `CER` table, including any applicable
ground modifiers.

Ownership changes only if:

- attacker has surviving armies on the surface
- defender has no surviving armies

Post-capture:

- surviving attacking armies become the new garrison
- any remaining defender armies are removed
- batteries remain destroyed

Failed invasion:

- if the attacker does not capture the planet, no attacking armies remain on the
  surface in the final gamestate
- any surviving attacking landing force is treated as lost in failed withdrawal,
  surrender, or dispersal after the unsuccessful assault
- planet ownership remains unchanged
- defending batteries and surviving defender armies remain with the defender

## Blitz

Blitz is the short, violent alternative to invasion.

It deliberately sacrifices safety for speed.

### Blitz rules

- transports attempt immediate landing without waiting for all batteries to die
- a blitz begins with one brief low-intensity cover-fire round against batteries
- defender batteries that survive that cover fire fire during the landing
- troops killed in destroyed transports during descent are tracked separately
  from later surface-combat losses and should be reported as such
- landed armies then fight defender armies in simultaneous ground combat
- defender gets the blitz defense bonus in `CER`
- attacker is expected to bring overwhelming army numbers

The landing battle uses the ground `CER` table, including the blitz defense
modifier, after both the cover-fire step and the battery-fire-on-landing step
have resolved.

Ownership changes only if the attacker clears all defending armies.

Post-capture:

- surviving attacking armies become the new garrison
- any remaining defender armies are removed
- surviving ground batteries transfer intact with the captured planet

Failed blitz:

- if the attacker does not capture the planet, no attacking armies remain on the
  surface in the final gamestate
- any surviving attacking landing force is treated as lost in failed withdrawal,
  surrender, or dispersal after the unsuccessful assault
- planet ownership remains unchanged
- defending batteries and surviving defender armies remain with the defender

Blitz should generally cause less industrial damage than invade, but greater
transport losses and higher risk of outright failure.

## Starbases

Starbases are orbital defenders, not raiding attackers.

Canonical starbase rules:

- a starbase adds AS/DS only in orbital defense of its own world
- a guarding fleet and a starbase fight as one defensive force
- the starbase contributes to orbital tie-breaking as part of the defender
- a starbase may survive even if all guarding ships are destroyed

This follows the manuals: the base helps in a fight, has slightly more
firepower than a battleship, and can withstand more hits.

In a multi-empire system fight, the starbase always belongs to exactly one task
force: the current planetary owner’s defense.

If a planet changes ownership during a tick, the starbase does not switch sides
mid-resolution. It remains part of the pre-capture defender for that tick and
joins the new owner only on the next maintenance tick if still present.

## Results and Reports

The combat model shall support later generation of deterministic combat reports.
At minimum, combat events should be capable of expressing:

- participating fleets and defending world
- pre-battle and post-battle ship counts
- batteries destroyed
- armies lost on both sides
- industrial damage from bombardment / invasion
- whether a fleet broke off due to ROE
- whether a world changed ownership

## Explicit Non-Goals

This spec intentionally does not attempt to reproduce:

- the original Pascal RNG
- per-ship tactical movement
- hidden ambush / detection minigames
- persistent crippled hull state in save files

Those would produce a brittle clone rather than a clear, maintainable canonical
combat model.

## Worked Example

### Fleet battle: 4 BB vs 12 DD

Assume:

- attacker: `4 BB` -> combat `AS = 36`
- defender: `12 DD` -> combat `AS = 12`
- both sides have `ROE 6`
- no starbase
- no mixed-fleet bonus

Round 1 ratio and `CER`:

- attacker ratio = `36:12 = 3:1` -> `CER 1.50`
- defender ratio = `12:36 = 1:3` -> `CER 0.50`

Hits:

- attacker hits = `ceil(36 * 1.50) = 54`
- defender hits = `ceil(12 * 0.50) = 6`

Durability:

- each BB has `DS 10` -> `2 fresh steps`
- each DD has `DS 1` -> `1 fresh step`

Allocation sketch:

- attacker first strips destroyer fresh steps, then uses remaining hits to
  destroy destroyers
- defender strips fresh steps from the battleship line, but does not yet
  destroy all four battleships immediately

The important intended outcome is structural:

- the smaller destroyer force is badly mauled
- the battleship force still absorbs meaningful return punishment
- the larger fleet does not erase the smaller force without taking some damage

## Implementation Notes

When implementing this spec in Rust:

- keep class weights and `CER` tables explicit constants
- keep hit allocation pure and testable
- store intermediate “virtual step” damage only inside battle resolution
- update `RE_NOTES.md` only when fixture/oracle evidence forces a spec revision
- treat this document as the normative combat rulebook for Rust maintenance
