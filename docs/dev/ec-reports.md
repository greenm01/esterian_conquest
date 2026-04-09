# EC Report Spec

This doc is the canonical player-facing report spec for classic-style
`RESULTS.DAT` output in the Rust maint/export path.

Use it to keep Rust report text faithful to original EC wording and structure.
Use placeholders like `<fleet_ordinal>` and `<empire>` for dynamic fields.

Read this together with:

- [ec-timing-spec.md](ec-timing-spec.md)
  for `Stardate` placement and weekly assignment
- [ec-combat-spec.md](ec-combat-spec.md)
  for combat and hostile-world mechanics
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  for where reports are emitted in the yearly loop

## Source Priority

Use this source order when implementing or correcting report text:

1. recovered binary string/hexdump artifacts from `ECMAINT`
2. shipped player-report corpus in `original/v1.5/nc-logs-2012/`
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

## RESULTS.DAT Binary Format

Each record in RESULTS.DAT is an 84-byte packed Borland Pascal record:

```
Offset  Size  Field
0       1     Kind byte (report type AND record count — see below)
1       73    Text: String[72] (1 byte length prefix + 72 chars)
74      10    Tail: chain pointers + year
```

Tail layout (10 bytes):

```
Offset  Size  Field
74-75   2     ChainId (u16 LE) — previous report's header index + 1; 0 for first
76-77   2     Reserved (zero)
78-79   2     NextChainId (u16 LE) — next report's header index + 1; 0 for last
80-81   2     Reserved (zero)
82-83   2     Year (u16 LE)
```

### Kind byte = record count (critical discovery)

**The kind byte tells ECGAME how many 84-byte records to read for this
report.** ECGAME reads exactly `kind` records per logical report, then
shows a `Delete this report Y/[N]` prompt.

This was verified against every available oracle/post-maintenance fixture —
in every case, the kind byte equals the number of records in that report:

| Kind | Hex  | Report families (examples from oracle data) |
|------|------|---------------------------------------------|
| 4    | 0x04 | (observed in fleet-battle-post) |
| 5    | 0x05 | sensor contact, fleet movement |
| 6    | 0x06 | fleet battle, identify |
| 8    | 0x08 | bombardment result |
| 11   | 0x0B | scout system extended orbit (verified from shipped corpus) |
| 12   | 0x0C | invasion/blitz result |

**The kind is NOT a fixed type identifier** — it is the record count.
Different instances of the same report family may produce different kind
values depending on how many lines the text wraps to.

**Implementation rule:** compute the kind byte dynamically as
`text_lines + 1` (text records + EOT record). This ensures every report
is exactly the right size with no blank padding and no truncation.
The computed kind is written to byte 0 of every record in the report.

### Chain pointer semantics

Chain pointers use 1-based record indexing:

- `ChainId` on the header record = `previous_header_0based_index + 1`
  (0 for the first report)
- `NextChainId` on the header record = `next_header_0based_index + 1`
  (0 for the last report)
- **Continuation and EOT records** must have `NextChainId = 0`
  (only the header carries the forward pointer)
- All records within a report share the same `ChainId` as their header

This was verified by comparing oracle RESULTS.DAT files through an FPC
(Free Pascal) reader that uses the native `file of TResultsRecord` with
`String[72]` layout.

### String[72] and buffer semantics

The Text field is a Borland Pascal `String[72]`: byte 0 is the length (0–72),
bytes 1–72 are character data. Standard BP string comparison uses only the
length-prefixed portion; trailing bytes after the declared length are ignored.

Original ECMAINT exhibits buffer reuse: when writing a new string to the
record variable, only the first N chars are overwritten, leaving previous
record data in the trailing positions. Rust-generated records zero-fill the
trailing bytes. This difference does not affect ECGAME display or EOT
detection — BP string comparison is length-aware.

### Empire clause format

When naming an empire in report text, use the format:

```
"<empire>", (Empire #<empire_no>)
```

Example: `"Red Horizon Pact", (Empire #2)`

When the empire name is unknown, fall back to `Empire #<empire_no>`.

## Classic Layout Contract

These are part of the classic viewer contract, not optional style notes.

- one logical report is one source/header line, followed by zero or more
  left-justified body lines, followed by exactly one
  `<end of transmission>` line
- **each logical report occupies exactly `kind` records** in RESULTS.DAT;
  the kind byte is computed dynamically as `text_lines + 1` (EOT) so every
  report is exactly the right size with no padding or truncation
- `Stardate: <week>/<year>` appears on the first line only and is right-justified
- body text must wrap on word boundaries to the classic 72-character record
  width
- body text must not overflow into the next logical report — ECGAME reads a
  fixed number of records per report and overflow corrupts subsequent reports
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
  - `artifacts/ghidra/ecmaint-live/string-probe-sensor-contact-shows-an-alien-fleet.txt`
- `We have located and identified the alien fleet in `
  - `artifacts/ghidra/ecmaint-live/string-probe-we-have-located-and-identified-the-alien-fleet.txt`
- `We are in extended orbit around `
  - `artifacts/ghidra/ecmaint-live/string-probe-we-are-in-extended-orbit-around.txt`
- `We have just concluded a bombing run against planet "`
  - `artifacts/ghidra/ecmaint-live/string-probe-we-have-just-concluded-a-bombing-run.txt`
- `We were attacked by the `
  - `artifacts/ghidra/ecmaint-live/string-probe-we-were-attacked-by.txt`
- `We have entered System(`
  - `artifacts/ghidra/ecmaint-live/string-probe-we-have-entered-system.txt`

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

- `artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt`
- `artifacts/ghidra/ecmaint-live/string-xrefs.txt`
- `artifacts/ghidra/ecmaint-live/interesting-strings.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt`

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
  The planet's stardock appears to be empty.
```

When the stardock contains ships, the last line is replaced with:

```text
  Scanning the planet's stardock, we detected <ship_list>.
```

This report typically produces kind=0x0B (11 records). The kind is
computed dynamically from the text — it is not hardcoded.

Unowned/civil-disorder orbit reports may replace the owner clause with the
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
Bombardment mission report: We have just concluded a bombing run against planet "<planet>", currently owned by "<empire>", (Empire #<empire_no>). The target world was defended by <army_count> armies. We managed to destroy <destroyed_armies> armies, <factory_damage>% of the factories, <goods_damage>% of the stored goods (production points), and all of the ship(s) in stardock including <stardock_losses>. We attacked with <ship_summary>. <friendly_losses>. We are holding our position and are awaiting new orders.
```

Invasion arrival:

```text
Invasion mission report: We have arrived at our target world and are preparing to begin the invasion.
```

Invasion success:

```text
Invasion mission report: Our armies have captured planet "<planet>". We attacked with <ship_summary> carrying <army_count> armies. The defending world had <defense_summary>. Friendly losses: <friendly_loss_summary>. Enemy losses: <enemy_loss_summary>.
```

Invasion defeated:

```text
Invasion mission report: The landing was repulsed. We attacked with <ship_summary> carrying <army_count> armies. The defending world had <defense_summary>. Friendly losses: <friendly_loss_summary>. Enemy losses: <enemy_loss_summary>.
```

Blitz success:

```text
Blitz mission report: We have seized planet "<planet>" in a fast assault. We attacked with <ship_summary> carrying <army_count> armies. The defending world had <defense_summary>. Friendly losses: <friendly_loss_summary>. Enemy losses: <enemy_loss_summary>. <landing_note>
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

## Current Rust Combat Wording Anchors

These worked examples are the current style anchors for Rust-maintained
fleet-combat reports. Use them when tightening wording or resolving report
composition regressions.

Style rules reflected below:

- report the starting force and losses; do not append a survivor recap
- winner-side reports should say `The enemy fled the field.` when the opponent
  retreats
- winner-side reports should say `The aliens were completely destroyed.` when
  the opposing fleet is wiped out
- loser-side ROE language belongs only in the loser report, not the winner's

### Retreating Side

```text
-> From your 8th Fleet, located in System(9,5):           Stardate: 02/3025
   -> Patrol mission report: We engaged the 3rd Fleet of "Player2", (Empire #2).
   -> We had 1 battleship, 5 cruisers and 3 destroyers. The alien force contained
   -> 2 battleships, 4 cruisers and 6 destroyers. In accordance with our ROE, we
   -> withdrew toward System(7,5). We lost 2 cruisers and 1 destroyer. We inflicted
   -> losses of 1 cruiser and 2 destroyers.
   -> <end of transmission>
```

### Victorious Side After Enemy Retreat

```text
-> From your 3rd Fleet, located in System(9,5):           Stardate: 02/3025
   -> Guard/Blockade World mission report: We successfully intercepted the 8th Fleet
   -> of "Player1", (Empire #1). We had 2 battleships, 4 cruisers and 6 destroyers.
   -> The alien force contained 1 battleship, 5 cruisers and 3 destroyers. The enemy
   -> fled the field. We lost 1 cruiser and 2 destroyers. We inflicted losses of
   -> 2 cruisers and 1 destroyer.
   -> <end of transmission>
```

### Victorious Side After Total Enemy Destruction

```text
-> From your 6th Fleet, located in System(11,4):          Stardate: 03/3025
   -> Bombardment mission report: We successfully intercepted the 9th Fleet of
   -> "Player2", (Empire #2). We had 2 battleships, 6 cruisers and 4 destroyers.
   -> The alien force contained 1 battleship, 3 cruisers and 2 troop transport ships
   -> carrying 2 armies. The aliens were completely destroyed. We lost 1 cruiser.
   -> <end of transmission>
```

### Destroyed Side Telemetry Report

```text
-> From your Fleet Command Center:                        Stardate: 03/3025
   -> We lost all contact with the 9th Fleet shortly after it was attacked by the
   -> 6th Fleet of "Player1", (Empire #1) in System(11,4). It was composed of
   -> 1 battleship, 3 cruisers and 2 troop transport ships carrying 2 armies.
   -> Recovered telemetry indicates the alien force contained 2 battleships,
   -> 6 cruisers and 4 destroyers and suffered casualties of 1 cruiser.
   -> <end of transmission>
```

Starbase mutiny loss summary:

```text
<prefix> the starving crew of Starbase '<starbase_name>' mutinied and scuttled their starbase. They were last seen flying escape pods <location_clause>.
```

This wording is anchored by the late-report string region at
`artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt`.

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

### Record budget — no fixed limit

Because the kind byte is computed dynamically from the text, there is no
fixed record budget per report family. The text can be as long or short as
needed — the kind will match automatically.

However, extremely long reports consume more display space in ECGAME and
may scroll beyond what the player can comfortably read. As a practical
guideline, keep reports under ~12 records (the invasion result report is
the longest observed in the oracle corpus at 12 records).

## Evidence Pointers

Primary binary-backed evidence:

- `artifacts/ghidra/ecmaint-live/string-probe-sensor-contact-shows-an-alien-fleet.txt`
- `artifacts/ghidra/ecmaint-live/string-probe-we-have-located-and-identified-the-alien-fleet.txt`
- `artifacts/ghidra/ecmaint-live/string-probe-we-are-in-extended-orbit-around.txt`
- `artifacts/ghidra/ecmaint-live/string-probe-we-have-just-concluded-a-bombing-run.txt`
- `artifacts/ghidra/ecmaint-live/string-probe-we-were-attacked-by.txt`
- `artifacts/ghidra/ecmaint-live/string-probe-we-have-entered-system.txt`
- `artifacts/ghidra/ecmaint-live/turn-cycle-function-strings.txt`
- `artifacts/ghidra/ecmaint-live/string-xrefs.txt`
- `artifacts/ghidra/ecmaint-live/interesting-strings.txt`
- `artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt`

FPC verification tool:

- [tools/fpc_results_reader.pas](../../tools/fpc_results_reader.pas) —
  Free Pascal program that reads RESULTS.DAT using native BP `file of TResultsRecord`
  with `String[72]`. Compile with `fpc tools/fpc_results_reader.pas`. Dumps kind,
  chain pointers, text, hex, and boundary analysis for each record.

Secondary corpus used to fill binary gaps:

- [ec5.txt](../../original/v1.5/nc-logs-2012/ec5.txt)
- [ec8.txt](../../original/v1.5/nc-logs-2012/ec8.txt)
- [ec10.txt](../../original/v1.5/nc-logs-2012/ec10.txt)
- [ec12.txt](../../original/v1.5/nc-logs-2012/ec12.txt)
- [ec16.txt](../../original/v1.5/nc-logs-2012/ec16.txt)
- [ec20.txt](../../original/v1.5/nc-logs-2012/ec20.txt)
- [ec42.txt](../../original/v1.5/nc-logs-2012/ec42.txt)
