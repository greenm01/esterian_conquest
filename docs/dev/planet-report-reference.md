# Planet Report Reference

This document records coordinate-linked scouting and bombardment reference
worlds from the preserved text captures.

Purpose:

- provide repeatable target-world profiles for future `ECMAINT` black-box
  scenarios
- keep known planet statistics close to the reverse-engineering notes
- avoid inventing synthetic target values when historical report data already
  exists

These references are not yet tied to matching `.DAT` snapshots in the repo.
They are report-side evidence only.

## Reference Worlds

### Fran

- system: `(25,22)`
- owner: `Melody Lake` (`Empire #2`)

Observed reports:

- `ec18.txt`
  - potential `100`
  - present `100`
  - stored `51`
  - armies `15`
  - ground batteries `8`
  - docked: `1 destroyer and 1 battleship`
- `ec19.txt`
  - potential `100`
  - present `100`
  - stored `51`
  - armies `15`
  - ground batteries `8`
- `ec20.txt`
  - potential `100`
  - present `100`
  - stored `51`
  - armies `15`
  - ground batteries `9`
- `ec31.txt`
  - same world still exists as a named planet report source in later years

Why it matters:

- stable mature enemy colony
- repeated reports show durable high production and army counts
- ground batteries and stardock contents vary over time

### 90

- system: `(25,12)`
- owner: `Melody Lake` (`Empire #2`)

Observed reports:

- `ec13.txt`
  - potential `90`
  - present `35`
  - stored `27`
  - armies `4`
  - ground batteries `0`
- `ec14.txt`
  - potential `90`
  - present `38`
  - stored `26`
  - armies `4`
  - ground batteries `1`
- `ec20.txt`
  - appears as a planet report source in the same system

Why it matters:

- moderate-development enemy colony
- small army count
- visible battery increase over time

### 126

- system: `(27,12)`
- owner: `Melody Lake` (`Empire #2`)

Observed reports:

- `ec21.txt`
  - potential `126`
  - present `75`
  - stored `40`
  - armies `15`
  - ground batteries `9`
  - docked: `1 cruiser`
- `ec25.txt`
  - potential `126`
  - present `27`
  - stored `13`
  - armies `1`
  - ground batteries `2`
- `ec27.txt`
  - potential `126`
  - present `10`
  - stored `5`
  - armies `1`
  - ground batteries `0`

Why it matters:

- same world observed at materially different strength levels
- good candidate for tracking how bombardment or attrition might map into
  underlying planet fields

### Micro

- system: `(24,16)`
- owner: `Melody Lake` (`Empire #2`)

Observed report:

- `ec10.txt`
  - potential `21`
  - present `10`
  - stored `5`
  - armies `8`
  - ground batteries `0`
  - docked: empty

Why it matters:

- low-production enemy colony
- useful small target profile for future synthetic scenarios

## Use In ECMAINT Work

Recommended usage:

1. choose one reference world profile
2. encode only the fields we can currently justify in `PLANETS.DAT`
3. run `ECMAINT`
4. compare the output against the expected report-side shape
5. promote only repeated or controlled-diff-backed field mappings into code

Do not treat these report values as exact byte mappings by themselves. They are
target behavior references.
