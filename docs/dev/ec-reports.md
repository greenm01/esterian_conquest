# EC Report Spec

This doc is the canonical player-facing report spec for classic-style
`RESULTS.DAT` output in the Rust maint/export path.

Use it to keep Rust report text faithful to original EC wording and structure.
Use placeholders like `<fleet_ordinal>` and `<empire>` for dynamic fields.

Read this together with:

- [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
  for `Stardate` placement and weekly assignment
- [ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-combat-spec.md)
  for combat and hostile-world mechanics
- [rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md)
  for where reports are emitted in the yearly loop

## Source Priority

Use this source order when implementing or correcting report text:

1. recovered binary string/hexdump artifacts from `ECMAINT`
2. shipped player-report corpus in `original/v1.5/ec-logs-2012/`
3. Rust-side composition rules that fill dynamic values into those recovered
   phrases

Important boundary:

- the binary artifacts preserve many canonical fixed phrases and report-family
  labels
- they do **not** preserve every final fully-rendered report as one static blob
- many reports are assembled from multiple fragments plus runtime values
- this doc therefore uses literal classic phrases plus placeholders

Implementation rule:

- when this doc and current Rust wording disagree, update Rust to match this doc
  unless shipped corpus or binary evidence proves the doc wrong
- when the evidence is incomplete, keep the classic sentence shape, suspense,
  and cadence instead of switching to modern summary prose

## Classic Layout Contract

These are part of the classic viewer contract, not optional style notes.

- one logical report is one source/header line, followed by zero or more
  left-justified body lines, followed by exactly one
  `<end of transmission>` line
- `Stardate: <week>/<year>` appears on the first line only and is right-justified
- body text must wrap on word boundaries to the classic 72-character record
  width
- body text must not overflow into the next logical report
- long reports may span multiple visible screens; pagination is a viewer concern,
  not a renderer excuse to truncate or restart the report
- fleet-origin reports must name a fleet; do not emit `From your fleet...`
- classic-facing reports use full ship names, never abbreviations such as `BB`,
  `CA`, or `DD`

## Placeholders

Use these placeholder names consistently:

- `<fleet_ordinal>`
- `<empire>`
- `<empire_no>`
- `<planet>`
- `<starbase_no>`
- `<system_x>`
- `<system_y>`
- `<sector_x>`
- `<sector_y>`
- `<week>`
- `<year>`
- `<ship_summary>`
- `<enemy_ship_summary>`
- `<army_count>`
- `<ground_battery_count>`
- `<goods_points>`
- `<production_points>`
- `<retreat_location>`

## Universal Header Forms

These are the standard first-line source headers seen in classic reports.

Fleet in system:

```text
From your <fleet_ordinal> Fleet, located in System(<system_x>,<system_y>):           Stardate: <week>/<year>
```

Fleet in deep space:

```text
From your <fleet_ordinal> Fleet, located in Sector(<sector_x>,<sector_y>):           Stardate: <week>/<year>
```

Planet source:

```text
From planet "<planet>" in System(<system_x>,<system_y>):                    Stardate: <week>/<year>
```

Starbase source:

```text
From Starbase <starbase_no>, located in System(<system_x>,<system_y>)               Stardate: <week>/<year>
```

Fleet Command Center source:

```text
From your Fleet Command Center:                        Stardate: <week>/<year>
```

Footer terminator:

```text
<end of transmission>
```

## Binary-Backed Fixed Phrases

These phrases are recovered directly from the preserved `ECMAINT` memory/string
artifacts and should be treated as canonical wording anchors.

Recovered string probes:

- `Sensor contact shows an alien fleet in `
  - [string-probe-sensor-contact-shows-an-alien-fleet.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-sensor-contact-shows-an-alien-fleet.txt)
- `We have located and identified the alien fleet in `
  - [string-probe-we-have-located-and-identified-the-alien-fleet.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-located-and-identified-the-alien-fleet.txt)
- `We are in extended orbit around `
  - [string-probe-we-are-in-extended-orbit-around.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-are-in-extended-orbit-around.txt)
- `We have just concluded a bombing run against planet "`
  - [string-probe-we-have-just-concluded-a-bombing-run.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-just-concluded-a-bombing-run.txt)
- `We were attacked by the `
  - [string-probe-we-were-attacked-by.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-were-attacked-by.txt)
- `We have entered System(`
  - [string-probe-we-have-entered-system.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-entered-system.txt)

Recovered late-report string anchors:

- `Invasion mission report: `
- `Scouting mission report: `
- `Guard/Blockade World mission report: `
- `Starbase mission report: `
- `We have arrived at Starbase `
- `and are now merged with the `
- `Accelerating to intercept alien fleet...`
- `However, we are unable to intercept alien fleet.`
- `In accordance to our ROE, we are avoiding this alien fleet...`

Sources:

- [turn-cycle-function-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt)
- [string-xrefs.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-xrefs.txt)
- [interesting-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/interesting-strings.txt)
- [unknown-starbase-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt)

## Template Reference

The templates below use the recovered binary wording where available and fill
the missing dynamic or trailing text from the shipped report corpus.

Status tags used in the section titles and prose below:

- `binary-backed`: directly anchored by preserved `ECMAINT` strings
- `corpus-completed`: finished from shipped player logs where the binary probe
  only preserved partial text
- `rust-only`: not a classic family; keep the tone administrative and restrained

### Classic-Backed Families

### Move / Navigation

Move arrival:

```text
Move mission report: We have arrived at our destination and are awaiting new orders.
```

Move attacked:

```text
Move mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in <location>. Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Patrol arrival:

```text
Patrol mission report: We have arrived at our destination and are beginning our patrolling assignment.
```

Seek-Home arrival:

```text
Seek-Home mission report: We have arrived at our destination and are awaiting new orders.
```

Rendezvous waiting:

```text
Rendezvous mission report: We have arrived at the our rendezvous point and are waiting for more fleets to arrive.
```

Join merge:

```text
Join mission report: We have joined the <fleet_ordinal> Fleet and are now merging with them.
```

Join host destroyed:

```text
Join mission report: In light of the destruction of the <fleet_ordinal> Fleet, we are holding our current position in Sector(<sector_x>,<sector_y>) and are awaiting new orders.
```

### Guard / Starbase / Blockade

Guard Starbase arrival:

```text
Guard Starbase mission report: We have arrived at Starbase <starbase_no> and are beginning our guard/escort mission.
```

Guard Starbase intercept:

```text
Guard Starbase mission report: We successfully intercepted the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>). We had <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Guard/Blockade arrival:

```text
Guard/Blockade World mission report: We have arrived at planet "<planet>" in Sector(<sector_x>,<sector_y>) and are beginning our guarding/blockading assignment.
```

Guard/Blockade sensor contact:

```text
Guard/Blockade World mission report: Sensor contact shows an alien fleet in System(<system_x>,<system_y>) traveling at a translight speed of <speed>. Closing to check it out...
```

Guard/Blockade identified hostile:

```text
Guard/Blockade World mission report: We have located and identified the alien fleet in System(<system_x>,<system_y>). It is the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>). Their fleet contains <enemy_ship_summary> of unknown type. <follow_on_clause>
```

Common `follow_on_clause` variants:

- `Accelerating to engage the enemy...`
- `The alien fleet seems to be preparing to bombard our planet "<planet>". Accelerating to intercept alien fleet...`
- `The alien fleet seems to be preparing to invade our planet "<planet>". Accelerating to intercept alien fleet...`
- `The alien fleet seems to be preparing to blitz our planet "<planet>". Accelerating to intercept alien fleet...`
- `In accordance to our ROE, we are avoiding this alien fleet...`
- `However, we are unable to intercept alien fleet.`

Guard/Blockade intercepted:

```text
Guard/Blockade World mission report: We successfully intercepted the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>). We had <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Guard/Blockade attacked:

```text
Guard/Blockade World mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in System(<system_x>,<system_y>). Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

### Scouting / Viewing

Scout sector arrival:

```text
Scouting mission report: We have arrived at our destination and are beginning to scout this sector.
```

Scout system arrival:

```text
Scouting mission report: We have arrived at our destination and are beginning to scout this solar system.
```

Extended orbit detailed report:

```text
Scouting mission report: We are in extended orbit around planet "<planet>" and have compiled the following data:
  Owned by: "<empire>", (Empire #<empire_no>)
  Potential production: <production_points> points
  Estimated present production: <production_points> points
  Estimated amount of stored goods: <goods_points> points
  Number of armies: <army_count>
  Number of ground batteries: <ground_battery_count>
```

Unowned/civil-disorder orbit reports may replace the owner line with the
classic visible equivalent for that world state.

Scout sensor contact:

```text
Scouting mission report: Sensor contact shows an alien fleet in System(<system_x>,<system_y>) traveling at a translight speed of <speed>. Closing to check it out...
```

Scout identified hostile:

```text
Scouting mission report: We have located and identified the alien fleet in System(<system_x>,<system_y>). It is the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>). Their fleet contains <enemy_ship_summary> of unknown type. Ignoring alien fleet...
```

Scout attacked:

```text
Scouting mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in System(<system_x>,<system_y>). Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Scout control/update:

```text
Scouting mission report: Since we now control System(<system_x>,<system_y>), we are <follow_on_status>.
```

Viewing success:

```text
Viewing mission report: We have entered System(<system_x>,<system_y>) and have completed a long range viewing analysis of the world found within. The world is <owner_clause> and has a potential of <production_points> points. Until ordered otherwise, we will be moving out of the solar system.
```

Viewing attacked:

```text
Viewing mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in System(<system_x>,<system_y>). Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

### Colonization / Invasion / Blitz / Bombardment / Salvage

Colonization entered occupied system:

```text
Colonization mission report: We have entered System(<system_x>,<system_y>) and have determined that aliens are already living on the world found within! We have gone ahead and performed a long range viewing analysis and have determined that the world is owned by "<empire>", (Empire #<empire_no>) and has a potential of <production_points> points. We are aborting our mission and are leaving the alien solar system.
```

Colonization arrival at target world:

```text
Colonization mission report: We have arrived at our target world, <planetary_clause>.
```

Colonization attacked:

```text
Colonization mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in System(<system_x>,<system_y>). Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Bombardment arrival:

```text
Bombardment mission report: We have arrived at our target world and are preparing for bombardment.
```

Bombardment attacked:

```text
Bombardment mission report: We were attacked by the <fleet_ordinal> Fleet of "<empire>", (Empire #<empire_no>) in System(<system_x>,<system_y>). Our force contained <ship_summary>. Alien force contained <enemy_ship_summary>. <battle_outcome>. <friendly_losses>.
```

Bombardment result:

```text
Bombardment mission report: We have just concluded a bombing run against planet "<planet>", currently owned by "<empire>", (Empire #<empire_no>). The target world was defended by <army_count> armies. We managed to destroy <destroyed_armies> armies, <factory_damage>% of the factories, <goods_damage>% of the stored goods (production points), and all of the ship(s) in stardock including <stardock_losses>. Our force initially contained <ship_summary>. <friendly_losses>. We are holding our position and are awaiting new orders.
```

Invasion arrival:

```text
Invasion mission report: We have arrived at our target world and are preparing to begin the invasion.
```

Invasion success:

```text
Invasion mission report: We have successfully invaded and taken planet "<planet>" from "<empire>", (Empire #<empire_no>). The target world was defended by <defense_summary>. In the attack, we managed to capture <goods_points> production point(s) worth of goods. Our force initially contained <ship_summary> carrying <army_count> armies. In the raid, we lost <friendly_loss_summary>. After the smoke cleared, we had <remaining_armies> armies enforcing control on the planet. We are holding our position and are awaiting new orders.
```

Invasion defeated:

```text
Invasion mission report: Our invasion attempt was defeated. We failed to capture planet "<planet>" from "<empire>", (Empire #<empire_no>). The target world was defended by <defense_summary>. <loss_summary>.
```

Blitz success:

```text
Blitz mission report: We have successfully blitzed and taken planet "<planet>" from "<empire>", (Empire #<empire_no>). The target world was defended by <defense_summary>. In the attack, we managed to capture <capture_summary>. Our force initially contained <ship_summary> carrying <army_count> armies. In the raid, we lost <friendly_loss_summary>. After the smoke cleared, we had <remaining_armies> armies enforcing control on the planet. We are holding our position and are awaiting new orders.
```

Salvage start:

```text
Salvage mission report: We have arrived at planet "<planet>" in System(<system_x>,<system_y>) and have begun salvaging our fleet. We estimate that our fleet will yield <production_points> production point(s).
```

### Planet / Fleet Command Center Reports

Planet reports use the planet-source header and then a body specific to growth,
attack aftermath, or local damage. Keep their wording tied to the preserved
corpus instead of inventing new planet-report prose.

Fleet Command Center loss summary:

```text
We lost all contact with the <fleet_ordinal> Fleet shortly after it <hostile_clause>. Records show the <fleet_ordinal> Fleet was composed of <ship_summary> and carried <army_count> armies. According to a burnt flight recorder we recovered, the alien force initially contained <enemy_ship_summary>. The flight recorder recorded alien ship casualties of <enemy_losses>.
```

Starbase mutiny loss summary:

```text
<prefix> the starving crew of Starbase '<starbase_name>' mutinied and scuttled their starbase. They were last seen flying escape pods <location_clause>.
```

This wording is anchored by the late-report string region at
[turn-cycle-function-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt).

### Rust-Only Administrative Notices

These notices exist for Rust-maint sanitization and validation paths. They are
player-visible, but they are not classic report families and should not pretend
to be.

Rules:

- keep the tone terse and administrative
- prefer source clauses like `From your central administration:` or
  `From your foreign ministry:`
- do not invent mission-family labels such as `Fleet readiness report` or
  `Order validation report`
- do not use these notices when a classic-backed mission or contact family can
  express the same event

Typical Rust-only families:

- invalid fleet order/input correction notices
- invalid planet-input correction notices
- diplomacy-input sanitization notices
- tax-rate correction notices
- other maint-side normalization notices that have no preserved classic family

## Style Rules

- use full ship names in classic-facing reports
- do not switch to abbreviations like `BB`, `CA`, or `DD`
- keep enemy fleet numbers only when the empire and specific fleet are known
- if the report says `alien fleet`, keep it generic and omit the fleet number
- always terminate each logical report with `<end of transmission>`
- keep source header and `Stardate` on the first line only
- prefer classic reveal order:
  contact -> identify -> intercept/attack/result
- do not collapse classic suspense into one modern summary paragraph when the
  corpus clearly stages the report across multiple logical reports

## Evidence Pointers

Primary binary-backed evidence:

- [string-probe-sensor-contact-shows-an-alien-fleet.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-sensor-contact-shows-an-alien-fleet.txt)
- [string-probe-we-have-located-and-identified-the-alien-fleet.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-located-and-identified-the-alien-fleet.txt)
- [string-probe-we-are-in-extended-orbit-around.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-are-in-extended-orbit-around.txt)
- [string-probe-we-have-just-concluded-a-bombing-run.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-just-concluded-a-bombing-run.txt)
- [string-probe-we-were-attacked-by.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-were-attacked-by.txt)
- [string-probe-we-have-entered-system.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-probe-we-have-entered-system.txt)
- [turn-cycle-function-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt)
- [string-xrefs.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/string-xrefs.txt)
- [interesting-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/interesting-strings.txt)
- [unknown-starbase-strings.txt](/home/mag/dev/esterian_conquest/artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt)

Secondary corpus used to fill binary gaps:

- [ec5.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec5.txt)
- [ec8.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec8.txt)
- [ec10.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec10.txt)
- [ec12.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec12.txt)
- [ec16.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec16.txt)
- [ec20.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec20.txt)
- [ec42.txt](/home/mag/dev/esterian_conquest/original/v1.5/ec-logs-2012/ec42.txt)
