# Esterian Conquest v1.5 RE Notes

## Current Status

- `ECGAME.EXE`, `ECUTIL.EXE`, and `ECMAINT.EXE` are 16-bit DOS `MZ` executables.
- `ECGAME.EXE` and `ECUTIL.EXE` both carry an `LZ91`/LZEXE-style wrapper.
- The game is runnable under a clean `DOSBox-X` setup.
- `dosemu` was not reliable for this target and produced misleading crashes.
- Stock `dosbox 0.74` was also not a reliable baseline for startup analysis.

Compiler/runtime evidence from `ECUTIL`:

- a DOSBox-X debugger memory dump of the live `ECUTIL` image exposed unpacked Borland runtime strings
- observed in `/tmp/ecinit/MEMDUMP.BIN`:
  - `Runtime error `
  - ` at `
  - `Portions Copyright (c) 1983,90 Borland`

Current best inference:

- `ECUTIL` was built with a Borland toolchain
- the runtime string style is a strong match for `Turbo Pascal` / `Borland Pascal`
- this is not a formal proof of source language yet, but it is materially stronger evidence than the earlier generic Borland guess

Matching evidence from `ECGAME`:

- a DOSBox-X debugger memory dump of the live `ECGAME` image exposed the same unpacked Borland runtime strings
- observed in `/tmp/ecgboot_chain/MEMDUMP.BIN`:
  - `Runtime error `
  - ` at `
  - `Portions Copyright (c) 1983,90 Borland`

Updated best inference:

- both `ECUTIL` and `ECGAME` were built with a Borland toolchain
- the shared runtime strings make `Turbo Pascal` / `Borland Pascal` the leading hypothesis for the codebase, not just a vague possibility
- it is still possible that some low-level routines were written in assembly, but the main application/runtime now looks Borland-derived

Unpacked `ECGAME` code-shape observations:

- the live memory image contains many `55 8B EC` stack-frame prologues
- many procedures return with `CB` / `CA nn 00` (`retf` / `retf n`), which is consistent with 16-bit large-model or Pascal-style far procedures
- repeated parameter handling patterns look like:
  - `LES DI, [BP+..]`
  - pointer/result writes via `ES:DI`
  - calls into shared helper routines followed by `retf`
- one cluster around offsets `0x315B..0x321F` in `/tmp/ecgboot_chain/MEMDUMP.BIN` looks like text/file helper code:
  - byte-by-byte reads
  - CR/LF handling
  - counted output writes
  - object/record callback-style calls through far pointers

Practical inference:

- the unpacked program layout looks much more like a Borland Pascal application linked against Borland RTL routines than a purely hand-written assembly program
- future static RE should treat repeated helper clusters as probable RTL/file I/O support and focus on higher-level callers, menu dispatch, and data-record updates

First useful unpacked anchors in `/tmp/ecgboot_chain/MEMDUMP.BIN`:

- filename table around `0x12D4..0x1339`:
  - stored as Pascal-style short strings with length bytes, e.g. `0B 'planets.dat'`, `09 'bases.dat'`, `0C 'messages.dat'`
  - entries include:
    - `planets.dat`
    - `bases.dat`
    - `messages.dat`
    - `results.dat`
    - `fleets.dat`
    - `ipbm.dat`
    - `player.dat`
    - `conquest.dat`
    - `Setup.dat`
    - `DataBase.dat`
- probable Borland-style file helper cluster around `0x4612..0x47D0`:
  - `int 21h / ah=3D` open
  - `int 21h / ah=3C` create
  - `int 21h / ah=3E` close
  - `int 21h / ah=3F` read
  - `int 21h / ah=40` write
  - `int 21h / ax=4200` seek
- this cluster also uses a small file-record/object structure with magic-like words `0xD7B0` and `0xD7B3`, plus a global error cell at `0x33FC`

Practical inference:

- the code near `0x4612..0x47D0` is almost certainly shared RTL or a thin Borland-style wrapper layer around DOS file I/O
- callers above that layer are the right place to hunt for game-specific logic such as loading `PLAYER.DAT`, `PLANETS.DAT`, and `FLEETS.DAT`
- the length-prefixed filename table is another concrete sign that the program was built around Pascal data conventions

First likely application-owned parser routine:

- procedure around `0x0B39..0x0D43` in `/tmp/ecgboot_chain/MEMDUMP.BIN`
- behavior from disassembly:
  - reads and normalizes a caller-supplied string buffer
  - trims leading/trailing spaces
  - classifies characters through helper calls
  - accumulates up to four extracted values into local slots
  - returns those extracted values through four output pointers

Why it matters:

- this looks like game/UI parsing logic, not generic Borland RTL
- it is a good first candidate for naming and reuse during future decompilation or porting

## Known Working Runtime

The most reliable environment found so far is:

- `DOSBox-X`
- `-defaultconf`
- `-nopromptfolder`
- `dosv=off`
- `machine=vgaonly`
- `core=normal`
- `cputype=386_prefetch`
- `cycles=fixed 3000`
- `xms=false`
- `ems=false`
- `umb=false`
- `output=surface`

Built binary used during testing:

- [`/tmp/dosbox-x/src/dosbox-x`](/tmp/dosbox-x/src/dosbox-x)

## Working Launch Recipe

First initialize game data with `ECUTIL.EXE` in a DOS game directory, then run `ECGAME`.

Example launch for `ECGAME`:

```bash
/tmp/dosbox-x/src/dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -defaultdir /tmp/ecgboot_chain \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c /tmp/ecgboot_chain" \
  -c "c:" \
  -c "mode co80" \
  -c "ECGAME"
```

Important detail:

- Running `ECGAME` directly from the game directory worked more reliably than `ECGAME C:\` in the synthetic local test setup.

## Door File Findings

- `ECGAME` does parse `CHAIN.TXT` successfully when given a sufficiently complete WWIV-style file.
- Minimal synthetic `CHAIN.TXT` files were rejected as invalid.
- A complete 32-line `CHAIN.TXT` format was required to clear the initial parser gate.
- Once valid, `ECGAME` stopped writing `ERRORS.TXT` and proceeded into the door flow.

Useful test files created during analysis:

- `/tmp/canon_remote.txt`
- `/tmp/canon_local0.txt`
- `/tmp/canon_local1.txt`

## Initialization Findings

`ECUTIL.EXE` is required to initialize game state.

Observed effects after initializing a new game:

- [`BASES.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/BASES.DAT) was zeroed
- `IPBM.DAT` was created
- `MESSAGES.DAT` was created
- `RESULTS.DAT` was created
- other game data files remained in place but now behaved correctly under `ECGAME`

Without this initialization step, `ECGAME` would accept the door file but fail to reach the real game flow.

## Confirmed Working Game Flow

Observed live screens:

- initial door information screen
- ANSI prompt
- splash / registration screens
- first-time menu
- join flow
- main menu

Confirmed menu families:

- first-time flow
- main command loop
- report/database commands

Observed state gate:

- `Total Planet Database` reports that the planetary database is not yet loaded until maintenance has run

This suggests `ECMAINT.EXE` is important for later-state or year/turn progression.

## Reverse Engineering Notes

### What runtime work proved

- the game is not blocked on a live BBS connection
- the door/drop-file path is real and understood enough to emulate
- the core game logic can be executed locally
- the next RE work should focus on data formats and command handlers, not basic emulator compatibility

### Current language/toolchain assessment

Not confirmed yet.

Best current guess:

- Borland Pascal or Borland C/C++
- possibly mixed with handwritten x86 assembly in the startup/loader path

Reason this is not confirmed:

- the main binaries are still wrapped in an LZEXE-style packed/self-modifying loader
- a clean recovered compiler/runtime signature has not yet been extracted

## Data Files To Decode Next

Highest-value targets for a port:

- [`SETUP.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/SETUP.DAT)
- [`PLAYER.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/PLAYER.DAT)
- [`PLANETS.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/PLANETS.DAT)
- [`FLEETS.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/FLEETS.DAT)
- [`CONQUEST.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/CONQUEST.DAT)
- [`DATABASE.DAT`](/home/niltempus/Documents/esterian-conquest/v1.5/DATABASE.DAT)

Current observations:

- `SETUP.DAT` begins with `EC151`
- `PLAYER.DAT` appears to be fixed-record structured data with player/empire strings
- `PLANETS.DAT` appears to be fixed-record structured data with planet names and ownership/status strings

## Draft File Layouts

These are first-pass RE notes. Items marked `confirmed` are based on exact size/boundary checks. Items marked `inferred` still need action/diff validation.

### `PLAYER.DAT`

Status:

- confirmed size: `440` bytes
- confirmed structure: `5` records of `88` bytes each

Why this is likely:

- `440 / 88 = 5`
- the file visually splits cleanly on 88-byte boundaries
- only the first record changed materially when the test user joined the game

Draft record layout for record 0:

- `0x00`: `u8` active/occupied flag or empire id (`01` in initialized joined state)
- `0x01..0x1A`: 26-byte padded uppercase handle / door username
- `0x1B`: `u8` length or string metadata (`0x09` for `niltempus`)
- `0x1C..0x2E`: 19-byte padded empire name
- `0x2F`: terminator/attribute byte (`0xEF` in joined state)
- `0x30..0x3F`: mostly zero in joined state
- `0x40..0x57`: small numeric fields, likely empire stats/options/status

Confirmed field inside that tail block:

- `0x51`: empire tax rate percentage
  - shipped sample: `65`
  - after initial join: `50`
  - after in-game tax change (`Tax rate: Empire` screen): `60`

Observed joined-state strings:

- username: `MRBILL`
- empire name: `niltempusDisorder's`

Important note:

- records `1..4` still contain what looks like uninitialized/stale text fragments after join
- that suggests either:
  - only record 0 is currently meaningful in this test game, or
  - the remaining records are alternate empire slots with untouched garbage/shareware defaults

Observed original -> initialized changes:

- record `0` is the only heavily rewritten record
- `0x01..0x08`: handle changed from `HANNIBAL` to `MRBILL`
- `0x1C..0x2E`: empire name changed from `Empire Of Dustder's` to `niltempusDisorder's`
- `0x42..0x47` and `0x4C..0x51`: several small status/stat bytes changed during join
- records `1..4` only changed at a handful of numeric offsets:
  - record `1`: `0x15`, `0x56`
  - record `2`: `0x00`
  - record `3`: `0x14`, `0x16`, `0x20`, `0x21`
  - record `4`: `0x2A`, `0x2C`, `0x36`, `0x37`

Practical inference:

- `PLAYER.DAT` mixes one live empire slot with four additional slot records
- record `0` almost certainly contains the current caller's door identity plus empire-visible metadata
- later records already look like fixed-width structures, but not yet fully initialized for active play
- bytes `0x50..0x52` are likely a compact option/status cluster for the active empire
  - current observed values after tax edit: `01 3c 64`
  - the middle byte is confirmed tax rate
  - the role of `0x50` and `0x52` is still unknown

Relevant documentation cross-check:

- `ECPLAYER.DOC` confirms `X` toggles a player-level `expert mode` setting and `T` changes the empire-wide tax rate
- that makes `PLAYER.DAT` the right place to continue hunting for saved UI/options flags after the tax byte

Expert mode persistence test:

- toggled `X` expert mode in-game
- quit cleanly
- compared `/tmp/PLAYER.before_expert_toggle.DAT` with the resulting `PLAYER.DAT`
- result: no byte differences at all

Practical inference:

- expert mode is not persisted in `PLAYER.DAT`
- it is likely a session-only runtime flag, or stored in a transient/message/drop-file path rather than campaign state

### `PLANETS.DAT`

Status:

- confirmed size: `1940` bytes
- confirmed structure: `20` records of `97` bytes

Why this is likely:

- `1940 / 97 = 20`
- every 97-byte chunk contains the same string slot and similar numeric layout

Draft record layout:

- `0x00..0x02`: small numeric header, likely coordinates / planet class / index
- `0x03..0x0E`: numeric state fields (mostly zero on many records)
- `0x0F`: string length byte
- `0x10..0x1C`: 13-byte name/status string slot
- `0x1D..0x24`: numeric fields, possibly production/resources/position
- `0x25..0x4F`: mostly zero in the current test state
- `0x50..0x5F`: tail numeric fields; nonzero on some records only
- `0x60`: final terminator/status byte

Observed names/statuses:

- many records: `Unowned`
- several records: `Not Named Yet`

Inferred meaning:

- `Unowned` is almost certainly the current owner/status string for neutral planets
- `Not Named Yet` likely marks special colonies/homeworld slots that are still unnamed in the fresh game state

Observed original -> initialized changes:

- every record changed in the first three bytes, which strongly suggests a compact per-planet header rather than random garbage
- records `5`, `7`, `14`, `15`, and `19` changed most heavily
- record `5` changed from:
  - header `0f 04 1e 00`
  - string `Unowned`
  to:
  - header `0d 05 64 87`
  - string `Not Named Yet`
- record `15` changed from:
  - string `Dust Bowl Yet`
  to:
  - string `Unowned`
- record `19` kept `Unowned` but had trailing garbage zeroed after the string slot

Practical inference:

- the `0x10` string field is real game state, not display-only padding
- initialization normalizes several records from demo/sample data into fresh-campaign placeholders
- the first 2-4 bytes likely encode a planet id plus a compact location/class tuple
- records with `Not Named Yet` are probably special player-start or colony targets that later receive player-defined names
- later per-planet economic/build choices appear to live deeper in the same record rather than in a separate queue file

Database mirror observation:

- `PLANETS.DAT` record `14` string bytes changed from `Not N...` to `prime`
- the same `prime` string change appears in `DATABASE.DAT` at offsets `0x578..0x57D`

Practical inference:

- `DATABASE.DAT` appears to cache or index planet-display strings from `PLANETS.DAT`
- planet naming/visibility work should be modeled as updates to both the core planet record and a derived database/report structure

Observed build-order changes:

- after issuing two build orders on the active planet (`ETAC` ship and `Destroyer`), no new differences appeared in `FLEETS.DAT`
- the new changes appeared in `PLANETS.DAT` record `14` only:
  - `0x24`: `0 -> 3`
  - `0x2E`: `0 -> 1`
- those bytes were still zero in the earlier post-mission state, so they are attributable to the build-order step rather than join or fleet orders

Practical inference:

- at least part of the planet production queue is encoded directly in each 97-byte planet record
- `0x24` and `0x2E` are strong candidates for build-item type/count slots, queue depth, or production-allocation flags
- ship construction orders do not appear to allocate or rewrite fleet records immediately; fleets are likely materialized later during maintenance

### `FLEETS.DAT`

Status:

- original sample size: `702` bytes
- initialized sample size: `864` bytes
- initialized structure strongly suggests `16` records of `54` bytes each

Why `54` bytes is the current best fit:

- `864 / 54 = 16`
- after initialization the file splits into `16` repeating records with the same internal layout
- the first bytes of those records form a clear grid:
  - record `0`: `01 00 01 02 00 01 ...`
  - record `1`: `02 00 01 03 00 02 ...`
  - record `2`: `03 00 01 04 00 03 ...`
  - record `3`: `04 00 01 00 00 04 ...`
- records `4..7`, `8..11`, and `12..15` repeat the same pattern with the second little-endian word incrementing from `2` to `4`

Draft record layout:

- `0x00..0x01`: small id/counter field
- `0x02..0x03`: row/group index
- `0x04..0x05`: linked id or previous slot
- `0x06..0x07`: linked id or next slot
- `0x08..0x09`: small flag/counter
- `0x0A..0x21`: mostly constant initialized template values
- `0x22..0x35`: trailing status/capacity flags, with one bit pattern distinguishing the first two records from the latter two inside each 4-record block

Practical inference:

- initialization expands fleet storage into a fully templated fixed-record table
- the `16` records look more like fleet slots or route templates than ad hoc save data
- this is now structured enough to port as a fixed record array even before every field is named

Observed mission-order changes:

- after issuing missions to fleets `1`, `2`, `3`, and `4`, only records `0..3` changed relative to the fresh `ECUTIL` baseline
- changed offsets were confined to the same small region in each 54-byte record:
  - record `0`: `0x0A`, `0x1F`, `0x20`
  - record `1`: `0x0A`, `0x1F`, `0x20`, `0x21`
  - record `2`: `0x0A`, `0x1F`, `0x20`
  - record `3`: `0x0A`, `0x1F`, `0x20`
- exact value changes:
  - record `0`: `0x0A 0->3`, `0x1F 5->12`, `0x20 16->15`
  - record `1`: `0x0A 0->3`, `0x1F 5->12`, `0x20 16->18`, `0x21 13->15`
  - record `2`: `0x0A 0->6`, `0x1F 5->9`,  `0x20 16->15`
  - record `3`: `0x0A 0->6`, `0x1F 5->6`,  `0x20 16->15`

Practical inference:

- `0x0A` is likely the chosen current speed for the order
- `0x1F..0x21` likely encode mission parameters such as destination coordinates, target slot, or route endpoint
- most of each fleet record remains unchanged by orders, which supports the idea that fleet identity/capacity lives in fixed header fields and only a compact mission block mutates during command entry

### `SETUP.DAT`

Status:

- confirmed size: `522` bytes
- header: `EC151`

Observed header bytes:

- `45 43 31 35 31` = `EC151`
- next bytes: `04 03 04 03 01 01 01 01`

Inferred:

- version marker plus compact global settings
- likely includes player-count / schedule / option toggles

Observed original -> initialized changes:

- no byte differences between the shipped sample and the initialized local test file

Practical inference:

- `ECUTIL` initialization does not rewrite `SETUP.DAT`
- `SETUP.DAT` is likely installation/global configuration, while the mutable campaign state lives primarily in `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and the auxiliary files created by `ECUTIL`

### `CONQUEST.DAT`

Status:

- confirmed size: `2085` bytes
- no useful printable strings in either the fresh `ECUTIL` state or the post-maintenance state
- post-maintenance differences are concentrated entirely in the first `0x55` bytes

Observed post-maintenance changes versus fresh `ECUTIL` baseline:

- total changed bytes: `51`
- changed offsets:
  - `0x00`
  - sparse changes from `0x12..0x3B`
  - dense changes from `0x40..0x54`
- examples:
  - `0x00`: `0xB8 -> 0xB9`
  - `0x12..0x13`: `0x64 0x00 -> 0xFF 0xFF`
  - `0x1A..0x1B`: `0x64 0x00 -> 0x74 0x33`
  - `0x20..0x23`: `0x64 0x00 0x64 0x00 -> 0x75 0x03 0x65 0x20`
  - `0x40..0x49`: `0x01`-filled bytes replaced by `ff 00 00 00 c2 00 00 08 6f 00`

Practical inference:

- `CONQUEST.DAT` begins with a packed global header or control block
- maintenance updates year/turn/summary counters here, not in `PLAYER.DAT` or `PLANETS.DAT`
- the repeated `0x0064` (`100`) values in the fresh baseline suggest default percentages, capacities, or turn constants
- the dense post-maintenance writes at `0x40..0x54` look like derived summary totals or timing/state fields produced during turn processing
- this file is a prime candidate for the core campaign clock and global statistics model in a port

Confirmed field:

- `CONQUEST.DAT[0x00..0x01]` (`u16`, little-endian): game year
  - shipped sample: `3022`
  - fresh `ECUTIL` init fixture: `3000`
  - post-maintenance fixture: `3001`

Confirmed field:

- `CONQUEST.DAT[0x02]` (`u8`): player count
  - shipped sample: `4`
  - fresh `ECUTIL` init fixture: `4`
  - post-maintenance fixture: `4`

Why this is high confidence:

- `ECPLAYER.DOC` states “The year is 3000.”
- `ECPLAYER.DOC` and `ECQSTART.DOC` both state that each round equals one year of game time
- the initialized-to-post-maint transition increments this field by exactly `1`

Why this is high confidence:

- `ECREADME.DOC` states that `ECUTIL` sets the maximum number of players.
- `ECPLAYER.DOC` states that the number of solar systems is `5` times the number of players.
- the preserved initialized fixture has `20` planet records and `20 / 5 = 4`.
- the low byte of the `0x0104` control word is `4` in all preserved states.

Practical caution:

- `CONQUEST.DAT[0x02..0x03]` is still exposed in the Rust code as a combined `player_config_word`.
- only the low byte is currently named with confidence.

Confirmed field block:

- `CONQUEST.DAT[0x03..0x09]` (`7 x u8`): maintenance schedule, ordered:
  - `[0x03]` Sunday
  - `[0x04]` Monday
  - `[0x05]` Tuesday
  - `[0x06]` Wednesday
  - `[0x07]` Thursday
  - `[0x08]` Friday
  - `[0x09]` Saturday

Confirmed encoding:

- `0x00` means the day is disabled for maintenance
- enabled days store a nonzero day-specific code, not a plain boolean

Observed values from controlled `ECUTIL` `F2 Change Maintenance Days` edits:

- Sunday `Yes` = `0x01`
- Monday `Yes` = `0x01`
- Tuesday `Yes` = `0xCA`
- Wednesday `Yes` = `0x01`
- Thursday `Yes` = `0x0A`
- Friday `Yes` = `0x01`
- Saturday `Yes` = `0x26`

High-confidence baseline:

- the preserved post-maintenance fixture stores `[01, 01, 01, 01, 01, 01, 01]`
- the live `ECUTIL` experiments proved that zeroing a day changes the corresponding byte to `0x00`

Practical implication for the Rust port:

- preserve the schedule as raw bytes first
- interpret `0x00` as disabled
- do not collapse the nonzero values to booleans until the original encoding scheme is better understood

Useful structural clue from initialized fixtures:

- in the preserved `4`-player initialized state, `FLEETS.DAT` contains `16` populated `54`-byte records
- `ECPLAYER.DOC` states that each empire starts with `4` fleets
- `4 players x 4 starting fleets = 16`, which matches the initialized fixture exactly

Practical implication:

- the preserved initialized `FLEETS.DAT` layout is consistent with a fixed fleet-record table sized to the current player count times the starting fleet allotment
- this is useful for port design, but not enough yet to name individual fleet fields

Preserved initialized fleet baseline:

From `original/v1.5/ec-logs-2012/ec.txt`, the first empire's four starting fleets in the
post-maintenance `3001 A.D.` state are:

- Fleet `1`: Speed `3`, ETA `1`, Ships `2`, ROE `6`, `Sector(14,14)`,
  `Colonize world in System (13,15)`
- Fleet `2`: Speed `3`, ETA `2`, Ships `2`, ROE `6`, `Sector(17,12)`,
  `Colonize world in System (20,11)`
- Fleet `3`: Speed `6`, ETA `2`, Ships `1`, ROE `6`, `Sector(19,9)`,
  `View world in System (23,5)`
- Fleet `4`: Speed `0`, ETA `0`, Ships `1`, ROE `6`, `Planet(15,13)`,
  `Guard/blockade world in System (15,13)`

The same log gives the detailed ship contents:

- Fleet `1`: `CA=1 ET=1`
- Fleet `2`: `CA=1 ET=1`
- Fleet `3`: `DD=1`
- Fleet `4`: `DD=1`

Practical implication:

- these preserved runtime values are the best current ground truth for naming the early fields in
  the initialized `FLEETS.DAT` records
- they are also a useful conformance target for a future Rust `inspect` view that decodes fleet
  location, mission, ROE, speed, ETA, and ship composition

Confirmed `FLEETS.DAT` fields from the initialized `16 x 54` layout:

- `record[0x05]` (`u8`): global fleet ID
  - records `1..16` store IDs `1..16`
- `record[0x00]` (`u8`): local fleet slot within the owning empire's four-fleet starting block
  - cycles `1,2,3,4` across the initialized table
- `record[0x03]` (`u8`): next fleet ID in the local linked order
  - fleet `1 -> 2`, `2 -> 3`, `3 -> 4`, `4 -> 0`
- `record[0x07]` (`u8`): previous fleet ID in the local linked order
  - fleet `1 <- 0`, `2 <- 1`, `3 <- 2`, `4 <- 3`
- `record[0x09]` (`u8`): maximum speed
  - matches the preserved starting fleet listing: `3, 3, 6, 6`
- `record[0x0A]` (`u8`): current speed
  - matches preserved live order-entry behavior:
    - fleets `1` and `2` were ordered with current speed `3`, and `0x0A` became `0x03`
    - fleets `3` and `4` were ordered with current speed `6`, and `0x0A` became `0x06`
  - later combat-era logs also show the fleet brief list carrying this chosen travel speed separately
    from maximum speed
- `record[0x25]` (`u8`): rules of engagement
  - matches the preserved starting fleet listing: all `6`
- `record[0x28]` (`u8`): cruiser count
  - starting fleets `1` and `2` have `CA=1`
- `record[0x2A]` (`u8`): destroyer count
  - starting fleets `3` and `4` have `DD=1`
- `record[0x30]` (`u8`): ETAC count
  - starting fleets `1` and `2` have `ET=1`

Useful but still conservatively named:

- `record[0x0B..0x0C]`: current-location coordinate pair
  - in the initialized fixture this looked like a shared home-system pair because every starting fleet
    begins at home
  - empire-group values in the initialized fixture are:
    - fleets `1..4`: `[16, 13]`
    - fleets `5..8`: `[4, 13]`
    - fleets `9..12`: `[6, 5]`
    - fleets `13..16`: `[13, 5]`
- `record[0x1F..0x21]`: mission parameter bytes
  - best current interpretation from preserved fleet-order screenshots:
    - `record[0x1F]`: standing-order mission code
    - `record[0x20]`: target X coordinate
    - `record[0x21]`: target Y coordinate
  - preserved `v1.11` screenshot menu codes show:
    - `0` none / hold position
    - `1` move fleet only
    - `2` seek home
    - `3` patrol a sector
    - `5` guard/blockade a world
    - `6` bombard a world
    - `9` view a world
    - `12` colonize a world
    - `13` join another fleet
  - in the initialized fixture, all four-fleet empire blocks store `[5, X, Y]` where `X,Y`
    match the block's initial current-location pair, which strongly suggests the initial standing orders
    are `Guard/Blockade` at the empire's home system

Practical implication for the Rust port:

- `ec-data` can now expose a small but real typed fleet model for initialized states
- the next useful fleet target is to decode current location, destination, ETA, and mission type

Confirmed initialized fleet-table structure:

- the initialized `16 x 54` table is the full 4-player starting roster, not just the current
  player's fleets
- records are grouped as four 4-fleet empire blocks:
  - group 1: fleet IDs `1..4`
  - group 2: fleet IDs `5..8`
  - group 3: fleet IDs `9..12`
  - group 4: fleet IDs `13..16`
- within each 4-fleet block:
  - `local_slot` cycles `1,2,3,4`
  - `previous_fleet_id` and `next_fleet_id` form a local chain ending in `0`
  - ship loadout is always:
    - slots `1` and `2`: `CA=1 ET=1`
    - slots `3` and `4`: `DD=1`
  - `max_speed` is always:
    - slots `1` and `2`: `3`
    - slots `3` and `4`: `6`
  - `current_location_coords_raw` is constant within the block
  - `mission_param_bytes` is also constant within the block

Observed initialized block current-location pairs:

- IDs `1..4`: `[16, 13]`
- IDs `5..8`: `[4, 13]`
- IDs `9..12`: `[6, 5]`
- IDs `13..16`: `[13, 5]`

Observed initialized block mission-param triples:

- IDs `1..4`: `[5, 16, 13]`
- IDs `5..8`: `[5, 4, 13]`
- IDs `9..12`: `[5, 6, 5]`
- IDs `13..16`: `[5, 13, 5]`

Practical implication:

- bytes `0x0B..0x0C` and `0x1F..0x21` look identical across an initialized empire block because all
  starting fleets begin at their home location with the same guard/blockade standing order
- the next likely per-fleet order-state bytes are around the still-unnamed early header values such
  as speed/ETA/current-location fields

Negative result from the initialized first-four-fleet scan:

- across fleets `1..4`, the only byte positions that vary are:
  - `0x00` local slot
  - `0x03` next fleet ID
  - `0x05` fleet ID
  - `0x07` previous fleet ID
  - `0x09` max speed
  - `0x28` cruiser count
  - `0x2A` destroyer count
  - `0x30` ETAC count
- no other single byte in the initialized records matches the preserved brief-list `ETA`,
  current location, or displayed ship-total columns directly

Practical implication:

- the displayed `ETA` and current location for initialized fleets are probably derived from a
  combination of:
  - standing order code / target
  - current-location raw pair
  - local slot / fleet composition
  - game-wide movement rules
- or they are encoded in multi-byte/stateful forms that do not appear as simple scalar per-fleet
  fields in the initialized snapshot

## ECUTIL Surface

Preserved DOSBox-X screenshot:

- `/home/niltempus/Pictures/ecv1.5/ecutil_000.raw1.png`

Versioned screenshot/archive policy:

- `original/v1.5/EC-Screenshots-v1.11/` is a bundled historical reference set from `v1.11`
- `captures/v1.5-dosboxx/` is the preserved local runtime evidence set for this project's
  `v1.5` reverse engineering work
- when `v1.11` and `v1.5` screenshots differ, prefer the `v1.5` capture set for preservation
  notes and Rust compatibility work

Current preserved `v1.5` capture set:

- `captures/v1.5-dosboxx/ecgame_000.png` through `captures/v1.5-dosboxx/ecgame_030.png`
- `captures/v1.5-dosboxx/ecutil_000.png`
- `captures/v1.5-dosboxx/ecutil_000.raw1.png`
- `captures/v1.5-dosboxx/ecutil_001.png`
- `captures/v1.5-dosboxx/ecutil_002.png`

Confirmed `ECUTIL` main menu text:

- `Esterian Conquest Sysop's Utility`
- `MAIN MENU`
- `F1  Initialize a New Game`
- `F2  Change Maintenance Days`
- `F3  Change Empire Ownership`
- `F4  Modify Program Options`
- `F5  Change Modem/Com Port Configuration`
- `F10 Exit to DOS`

Footer text from the preserved screenshot:

- `Esterian Conquest Sysop's Utility - Test Drive Version 1.51`
- `Copyright (C) 1990-1992 by Bentley C. Griffith.`
- `All rights reserved worldwide.`

Practical implication for the Rust port:

- the preserved `ec-cli init` command corresponds directly to `F1`
- the preserved `ec-cli maintenance-days` command corresponds directly to `F2`
- the preserved `ec-cli setup-programs` command now mirrors the decoded `F4` screen wording
- the screenshot gives exact wording for a future faithful text-mode compatibility frontend

Confirmed `ECUTIL` F4 Setup The Programs menu from `captures/v1.5-dosboxx/ecutil_002.png`:

- `A` Purge messages & reports after
- `B` Autopilot any empires inactive for
- `C` Snoop Enabled
- `D` Enable timeout for local users
- `E` Enable timeout for remote users
- `F` Maximum time between key strokes
- `G` Minimum time granted
- `X` Exit Setup

Current Rust CLI coverage for the decoded `F4` fields:

- `ec-cli setup-programs [dir]`
- `ec-cli snoop [dir] <on|off>`
- `ec-cli local-timeout [dir] <on|off>`
- `ec-cli remote-timeout [dir] <on|off>`
- `ec-cli max-key-gap [dir] <minutes>`
- `ec-cli minimum-time [dir] <minutes>`
- `ec-cli purge-after [dir] <turns>`
- `ec-cli autopilot-after [dir] <turns>`

This means the decoded `F4 Modify Program Options` surface is now fully represented in the std-only Rust CLI, even though the command names are intentionally more Unix-like than the original single-letter menu.

Confirmed `ECUTIL` `F3 Change Empire Ownership` flow from:

- `captures/v1.5-dosboxx/ecutil_004.png`
- `captures/v1.5-dosboxx/ecutil_005.png`
- `captures/v1.5-dosboxx/ecutil_006.png`
- `captures/v1.5-dosboxx/ecutil_007.png`
- `captures/v1.5-dosboxx/ecutil_008.png`

Preserved option surface:

- `P` Assign empire to a new `PLAYER`
- `R` Make empire a `ROGUE` empire
- `U` Make empire `UNOWNED` (`Civil Disorder`)
- `N` No change

Conservative `PLAYER.DAT` ownership findings from the preserved `F3` fixture `fixtures/ecutil-f3-owner/v1.5/PLAYER.DAT`:

- `F3` touched `PLAYER.DAT` only in the observed test; `PLANETS.DAT` did not change.
- Record 0, byte `0x00`, changed `0x00 -> 0xff` when empire `#1` was made rogue.
- Record 0, bytes `0x1B..`, form a Pascal-style status/label field:
  - max length byte at `0x1A` remained `0x18`
  - current length at `0x1B` changed `0x11 -> 0x06`
  - text at `0x1C..` became `Rogues`
- Record 1, byte `0x16`, changed `0x00 -> 0x01` when empire `#2` was assigned to a player.
- Record 1, bytes `0x17..0x2F`, now contain the uppercased player handle `FOO` in a fixed-width field.
- Record 1, bytes `0x31..`, form a second Pascal-style name field:
  - current length at `0x31` became `0x03`
  - text at `0x32..` became `foo`

Rust preservation impact:

- `ec-data` now exposes conservative player ownership summaries:
  - `owner_mode_raw()`
  - `assigned_player_flag_raw()`
  - `legacy_status_name_summary()`
  - `assigned_player_handle_summary()`
  - `controlled_empire_name_summary()`
  - `ownership_summary()`
- This is intentionally narrower than a full player-record decode; it only covers the ownership fields that `ECUTIL F3` demonstrably touched.

Confirmed `ECUTIL` `F5 Modem / Com Port Setup` flow from:

- `captures/v1.5-dosboxx/ecutil_009.png`
- `captures/v1.5-dosboxx/ecutil_010.png`
- `captures/v1.5-dosboxx/ecutil_011.png`

Preserved `F5` surface:

- `A` `COM 1 Interrupt Request Number`
- `B` `COM 2 Interrupt Request Number`
- `C` `COM 3 Interrupt Request Number`
- `D` `COM 4 Interrupt Request Number`
- `E` `Restore Default IRQ Numbers for COM1 to COM4`
- `F` `COM 1 Hardware Flow Control`
- `G` `COM 2 Hardware Flow Control`
- `H` `COM 3 Hardware Flow Control`
- `I` `COM 4 Hardware Flow Control`
- `X` Exit Setup

Confirmed from the preserved `v1.5` screenshots and live diff:

- the IRQ editor prompt accepts direct numeric input in the range `0..7`
- `SETUP.DAT[5..8]` store the raw COM IRQ values for `COM1..COM4`
- the shipped fixture values are `[4, 3, 4, 3]`, matching the preserved `F5` screen
- `SETUP.DAT[9..12]` store `COM1..COM4` hardware flow control flags
- disabling all four flow-control options in `ECUTIL F5` changed those bytes from `[1, 1, 1, 1]` to `[0, 0, 0, 0]`
- `CONQUEST.DAT` did not change during the observed `F5` test

Rust preservation impact:

- `ec-data` now exposes:
  - `com_irq_raw()`
  - `set_com_irq_raw()`
  - `com_hardware_flow_control_enabled()`
  - `set_com_hardware_flow_control_enabled()`
- `ec-cli` now exposes:
  - `port-setup [dir]`
  - `com-irq <dir> <com1|com2|com3|com4> [0..7]`
  - `flow-control <dir> <com1|com2|com3|com4> [on|off]`

The CLI now covers the verified `F5` flow-control toggles directly and exposes the raw IRQ editor bytes with the same observed `0..7` value range as the original utility.

## Modern TUI Direction

Current preservation split:

- `ec-data` stays focused on binary formats and decoded fields
- `ec-cli` stays std-only and scriptable for RE work
- `ec-tui` is the new interactive terminal frontend

Current `ec-tui` shape:

- one shared TUI crate, not separate apps for utility and player modes
- `ec-tui` defaults to player mode in the current working directory
- `ec-tui util` opens the utility/admin mode in the current working directory
- optional directory override is still supported as the first positional path
- when the current directory is not a valid game directory, `ec-tui` now falls back to the preserved `fixtures/ecmaint-post/v1.5` snapshot instead of the noisier shipped sample
- utility mode now uses a modern EC-classic presentation instead of trying to mimic the original DOS utility screen-for-screen
- no function-key dependency in the new shell; section switching is handled with `1/2/3`, `Tab`, and `q`

The first `ec-tui` scaffold is intentionally a shell, not a faithful DOS clone:

- player mode is the default user-facing entry point
- utility mode surfaces the already-decoded setup, ownership, and port data in cleaner sectioned panels:
  - `Dashboard`
  - `Empire Control`
  - `Program & Port Setup`
- the historical `v1.5` UI is preserved via screenshots and notes rather than by reproducing every original panel verbatim

## First ECMAINT Phase-1 Build Scenario

Preserved fixtures:

- `fixtures/ecmaint-build-pre/v1.5/`
- `fixtures/ecmaint-build-post/v1.5/`

This first maintenance scenario used a direct file edit, not a clean one-click player action.

Why:

- we had prior observed evidence that a build-order-like state landed in `PLANETS.DAT` record `14` (zero-based)
- the exact single-order encoding was still unclear
- so the first black-box maintenance cycle was driven by the smallest previously observed planet-side build queue bytes

Pre-maint setup:

- baseline: `fixtures/ecutil-init/v1.5/`
- modified file: `PLANETS.DAT`
- modified record: record `14` (zero-based), the `(16,13)` homeworld-style record
- modified bytes:
  - `0x24`: `0x00 -> 0x03`
  - `0x2E`: `0x00 -> 0x01`

Post-maint result:

- `SETUP.DAT` unchanged
- `CONQUEST.DAT` matched the clean `ecmaint-post` fixture exactly
- `FLEETS.DAT` matched the clean `ecmaint-post` fixture exactly
- `PLANETS.DAT` differed from clean `ecmaint-post` only in record `14`
- `DATABASE.DAT` differed from clean `ecmaint-post` by `1` byte

Observed planet transition in record `14`:

- queued build bytes were cleared:
  - `0x24`: `0x03 -> 0x00`
  - `0x2E`: `0x01 -> 0x00`
- new post-maint state appeared at:
  - `0x38`: `0x00 -> 0x03`
  - `0x4C`: `0x00 -> 0x01`

Interpretation:

- `ECMAINT` consumed the synthetic build-queue-like bytes instead of leaving them in place
- it did not materialize a new fleet in `FLEETS.DAT` in this first scenario
- it did create a persistent planet-state transition and a tiny derived `DATABASE.DAT` change

Rust preservation impact:

- `ec-data` now has a fixture-backed test that locks in this first maintenance transform
- this is enough to prove the phase-1 workflow works, even though the exact semantics of the new `0x38` and `0x4C` planet bytes are not named yet

## Second ECMAINT Scenario: Single Fleet Order

Preserved fixtures:

- `fixtures/ecmaint-fleet-pre/v1.5/`
- `fixtures/ecmaint-fleet-post/v1.5/`

This second maintenance scenario used the smallest previously observed fleet-order mutation from the live game notes.

Pre-maint setup:

- baseline: `fixtures/ecutil-init/v1.5/`
- modified file: `FLEETS.DAT`
- modified record: record `0` (zero-based), fleet `1`
- modified bytes:
  - `0x0A`: `0x00 -> 0x03`
  - `0x1F`: `0x05 -> 0x0C`
  - `0x20`: `0x10 -> 0x0F`

Post-maint result relative to clean `fixtures/ecmaint-post/v1.5/`:

- `SETUP.DAT` unchanged
- `CONQUEST.DAT` unchanged
- `MESSAGES.DAT` unchanged
- `RESULTS.DAT` unchanged
- `DATABASE.DAT` differed by `29` bytes
- `FLEETS.DAT` differed by `9` bytes in fleet record `1`
- `PLANETS.DAT` differed by `18` bytes in planet record `14` (one-based display)

Observed fleet transition in record `0`:

- the queued order bytes were consumed:
  - `0x1F`: `0x0C -> 0x00`
- the fleet was rewritten into a held-at-target style state:
  - `0x0B`: `0x10 -> 0x0F`
  - `0x19`: `0x81 -> 0x80`
  - `0x1A`: `0x00 -> 0xB9`
  - `0x1B`: `0x00 -> 0xFF`
  - `0x1C`: `0x00 -> 0xFF`
  - `0x1D`: `0x00 -> 0xFF`
  - `0x1E`: `0x00 -> 0x7F`
  - `0x20`: `0x10 -> 0x0F`

Derived interpretation:

- fleet `1` moved from a guard/blockade home-world standing order into a hold-style post-maint state
- its current-location pair at `0x0B..0x0C` moved from `(16,13)` to `(15,13)`
- the fleet's target pair at `0x20..0x21` also ended at `(15,13)`
- this is the first controlled scenario showing `ECMAINT` consume a fleet order and rewrite persistent fleet state directly

Observed planet transition in record `13` (zero-based, `(15,13)`):

- `0x58`: `0x00 -> 0x01`
- `0x5C`: `0x00 -> 0x02`
- `0x5D`: `0x00 -> 0x01`

Interpretation:

- `ECMAINT` touched the target world as part of resolving the fleet order
- no new `FLEETS.DAT` records were created
- no global year/schedule change occurred
- the follow-on work should determine whether the `(15,13)` planet-side bytes represent colonization progress, occupation state, or another local status transition

Rust preservation impact:

- `ec-data` now has a second fixture-backed `ECMAINT` transform test covering fleet-side order consumption
- the preservation workflow now covers both a planet build queue case and a fleet order resolution case

## Historical Combat References From Later Text Captures

External reference set:

- `/home/niltempus/Documents/esterian-conquest/ec-logs-2022/`

These are not yet copied into the repo snapshot, but they are useful as
reference evidence for `ECMAINT` combat behavior because they preserve
player-issued orders and the next-year maintenance reports.

### Bombardment sequence: `ec9.txt -> ec10.txt`

In `ec9.txt`, fleet `13` is given a bombard mission:

- current location: `Sector(23,14)`
- mission chosen: `6` bombard a world
- target: `System(24,14)`
- travel time shown by the game: `1 year`
- resulting fleet list entry:
  - `13   4   1    0    4   6  Sector(23,14) Bombard world in System (24,14)`

In `ec10.txt`, the next-year report shows the resolved bombardment:

- report source: `13th Fleet`, located in `System(24,14)`
- planet owner: `Melody Lake` (`Empire #2`)
- defenses reported in the bombardment report:
  - `6 armies`
- bombardment results reported:
  - destroyed `5 armies`
  - destroyed `92%` of factories
  - destroyed `100%` of stored goods
  - destroyed all ships in stardock, including `1 troop transport`
- attacker losses:
  - none
- post-report fleet status:
  - “We are holding our position and are awaiting new orders.”
- matching fleet-list state in `ec10.txt`:
  - `13   0   0    0    4   6  Planet(24,14) No standing orders`

Interpretation:

- a successful bombardment consumes the standing order
- the fleet remains at the target world
- the fleet transitions to a no-standing-orders/hold state after the attack
- `ECMAINT` can directly alter:
  - planet armies
  - factories
  - stored production
  - stardock contents

### Follow-on invade sequence: `ec10.txt -> ec11.txt -> ec12.txt`

In `ec10.txt`, fleet `7` is reordered to invade the same world:

- new orders:
  - `Invade world in System (24,14)`
- fleet-list state after order entry:
  - `7    5   3   10   16   0  Planet(15,13) Invade world in System (24,14)`

In `ec11.txt`, the fleet is still traveling:

- fleet-list state:
  - `7    5   2   10   16   0  Sector(19,13) Invade world in System (24,14)`

In `ec12.txt`, the fleet is one year away:

- fleet-list state:
  - `7    5   1   10   16   0  Sector(24,14) Invade world in System (24,14)`

Interpretation:

- the `brief fleet list` preserves a useful observable movement model:
  - location
  - speed
  - ETA
  - army count
  - ship count
  - ROE
  - standing order text
- this is likely enough to build a future multi-turn invasion fixture once we
  have a compatible pre-maint state generator for mature games

### Fleet-vs-fleet combat reference: `ec11.txt`

Also in `ec11.txt`, fleet `1` reports a move-mission interception:

- `We were attacked by the 3rd Fleet of "In Civil Disorder", (Empire #8)`
- friendly force:
  - `1 cruiser`
  - `1 ETAC ship`
- alien force:
  - `1 destroyer`
- result:
  - enemy fled
  - no enemy ships destroyed
  - no friendly losses

Interpretation:

- `ECMAINT` emits explicit fleet-vs-fleet combat reports even on movement missions
- ROE and fleet composition probably govern:
  - whether interception happens
  - whether one side flees
  - whether losses are exchanged

Practical value for preservation:

- these text captures give us a real expected-output model for bombardment,
  invasion travel, and fleet-vs-fleet interception
- the next combat-oriented black-box fixture should be designed to reproduce a
  simplified bombardment outcome first, because that sequence is the clearest
  and the easiest to validate against observed report language

## Variable-Length Mature Fleet Tables

The repo's `original/v1.5/FLEETS.DAT` is not an invalid file. It is a valid
fleet table with a different record count:

- file size: `702` bytes
- record size: `54` bytes
- inferred record count: `13`

Preservation impact:

- `FLEETS.DAT` is not fixed to the initialized `16 x 54` roster
- the Rust parser now accepts any file length that is an exact multiple of
  `54` bytes
- this allows the mature `original/v1.5` snapshot to be inspected without
  special-case tooling

Observed mature-snapshot fleet shape:

- fleets `2..13` still decode coherently with the current field model
- the first record appears to be a much larger combined combat fleet:
  - `CA=9`
  - `DD=9`
  - `ET=2`
  - standing-order byte `0x1F = 0x04` (still unnamed)
- records `2..5`, `6..9`, and `10..13` still look like linked four-fleet home
  blocks with IDs ending in `0` at the tail, but the first empire block has
  been materially transformed by gameplay

Interpretation:

- the initialized 16-record layout is a starting-state template, not a universal
  fleet-table shape
- real games can collapse, merge, or otherwise restructure those starting
  blocks over time
- this makes the mature snapshot a better future source for combat-oriented
  `ECMAINT` work than the initialized fixtures, once enough planet/player-side
  context is decoded

## Mature `.SAV` Sidecars In `original/v1.5`

The mature `original/v1.5` snapshot includes:

- `BASES.SAV`
- `DATABASE.SAV`
- `FLEETS.SAV`
- `PLANETS.SAV`
- `PLAYER.SAV`

Observed differences relative to the matching `.DAT` files:

- `BASES.SAV` identical to `BASES.DAT`
- `FLEETS.SAV` identical to `FLEETS.DAT`
- `PLAYER.SAV` differs by `1` byte
- `PLANETS.SAV` differs by `3` bytes
- `DATABASE.SAV` differs by `15` bytes

Important detail:

- the changed words repeatedly include:
  - `0x0BCE -> 0x0BCB` in `PLAYER`
  - `0x0BCD -> 0x0BCC` in several `DATABASE` record regions

Best current interpretation:

- these `.SAV` sidecars are not a full clean pre/post-maint snapshot pair
- they look more like partial side backups or stale mirrored views than an
  immediately reusable engine-transition fixture
- they are still worth preserving as evidence, but they should not be treated
  as a ready-made combat scenario source

## Synthetic ECMAINT Bombardment Sequence

Preserved fixtures:

- `fixtures/ecmaint-bombard-pre/v1.5/`
- `fixtures/ecmaint-bombard-arrive/v1.5/`
- `fixtures/ecmaint-bombard-post/v1.5/`

Scenario design:

- baseline: `fixtures/ecutil-init/v1.5/`
- target planet: record `13` (zero-based), coordinates `(15,13)`
- target world was rewritten from `Unowned` into a cloned seeded-colony-style record using the
  bytes from record `12` `(4,13)`, while preserving target coordinates `(15,13)`
- attacking fleet: record `2` (zero-based), fleet `3`
- attacking order:
  - `current_speed = 3`
  - `standing_order = 6` (`Bombard world`)
  - `target = (15,13)`
- attacking ship loadout was increased to force a meaningful combat-style test:
  - `CA=3`
  - `DD=5`
  - `ET=0`

### First maintenance pass: arrival only

Relative to the synthetic pre-maint state:

- `FLEETS.DAT` changed only in fleet `3`
- fleet changes:
  - `current_location.x`: `16 -> 15`
  - standing order stayed `6` (`bombard`)
  - target stayed `(15,13)`
  - current speed stayed `3`
- `PLANETS.DAT` did not change at all
- `MESSAGES.DAT` and `RESULTS.DAT` remained empty
- `DATABASE.DAT` changed
- `CONQUEST.DAT` advanced through normal maintenance/year movement

Interpretation:

- in this synthetic case, arrival at the target and the bombardment attack itself are not resolved
  in the same maintenance pass
- `ECMAINT` moved the fleet onto the target world and preserved the bombard standing order

### Second maintenance pass: combat-style resolution

Relative to the arrival state:

- `FLEETS.DAT` changed only in fleet `3`
- order was consumed:
  - `current_speed`: `3 -> 0`
  - `standing_order`: `6 -> 0`
- location remained `(15,13)`
- attacker losses:
  - `CA`: `3 -> 2`
  - `DD`: `5 -> 1`
- internal fleet-state bytes at `0x19..0x1E` were also reset/rewritten
- `PLANETS.DAT` still did not change at all
- `MESSAGES.DAT` and `RESULTS.DAT` remained empty
- `DATABASE.DAT` changed by `27` bytes
- `CONQUEST.DAT` changed by `3` bytes:
  - year increment
  - one additional small header counter/field

Interpretation:

- this synthetic target encoding is sufficient to trigger fleet-side combat losses
- it is not sufficient to produce visible planet-side damage or player-facing message/report output
- best current inference:
  - either the cloned target world is still missing ownership/defense state from some other file or
    header field
  - or bombardment against this synthetic target resolves as hostile defensive attrition without
    entering the full report-producing planet-damage path

Follow-up comparison against the shipped mature snapshot makes the likely gap
clearer:

- the synthetic target at `(15,13)` was cloned from an initialized seeded world
  shell, not from a mature defended colony
- the initialized seeded shell and the synthetic target both share the same
  compact tail block at `0x58..0x60`:
  - `0a 00 04 00 02 02 00 00 00`
- a mature colony in the shipped snapshot, `Dust Bowl` at `(16,13)`, has a
  materially different tail block:
  - `8e 00 0f 00 02 01 00 00 00`
- that mature-world delta is currently the strongest explanation for why the
  synthetic bombardment produced attacker losses but no planet damage or
  player-facing bombardment report

Best current combat-target inference:

- our synthetic target was only a hostile seeded-colony shell
- a fully valid defended enemy world likely requires additional developed-world
  state beyond the visible coordinates/order bytes we copied
- likely candidates include:
  - matured planetary defense/resource fields inside `PLANETS.DAT`
  - ownership/state consistency with another file such as `DATABASE.DAT` and/or
    empire-linked state outside the single planet record

Next bombardment experiment should therefore clone a mature colony-style target,
not another initialized seed shell.

Additional mature-target throwaway test:

- a second synthetic bombardment scenario cloned the shipped mature colony
  `Dust Bowl` onto `(15,13)` instead of cloning an initialized seed shell
- that target used the mature tail/state block:
  - `8e 00 0f 00 02 01 00 00 00`
- first maintenance pass on that mature target produced:
  - fleet arrival at `(15,13)`
  - standing order rewritten from `6` (`bombard`) to `5` (`guard/blockade`)
  - no `PLANETS.DAT` change
  - no `MESSAGES.DAT` or `RESULTS.DAT` output
- second maintenance pass only zeroed the fleet's current speed while leaving
  the `guard/blockade` standing order in place

Interpretation:

- `Dust Bowl` behaves like a valid mature colony, but not like a hostile target
  for the attacking fleet
- best current inference:
  - the cloned mature planet was treated as friendly or same-empire state
  - so the next bombardment fixture needs a mature enemy colony, not just any
    mature colony record

Hybrid mature-enemy throwaway test:

- a follow-up synthetic target used the mature `Dust Bowl` colony as the base,
  but replaced the likely empire-linked bytes with those from the initialized
  empire-2 seed shell
- resulting target block highlights:
  - `0x20..0x22`: `11 25 1c`
  - `0x58..0x60`: `8e 00 0f 00 02 02 00 00 00`
- first maintenance pass:
  - fleet arrived at `(15,13)`
  - bombard order stayed active (`6`)
  - no `PLANETS.DAT` change
- second maintenance pass:
  - bombard order was consumed (`6 -> 0`)
  - attacker losses:
    - `CA 3 -> 1`
    - `DD 5 -> 1`
  - no `PLANETS.DAT` change
  - no `MESSAGES.DAT` or `RESULTS.DAT` output

Interpretation:

- hostile ownership markers are sufficient to keep the bombard mission active
  through arrival and to trigger attack resolution
- even with mature-world tail bytes, that is still not enough to produce
  visible planet damage or generated combat reports
- the remaining missing state is therefore likely in other `PLANETS.DAT`
  fields that encode a developed enemy colony's defenses/resources, not merely
  in `DATABASE.DAT`

`DATABASE.DAT` structure note:

- file size is `8000` bytes
- it divides cleanly into `80` subrecords of `100` bytes each
- repeated `UNKNOWN` blocks appear every `100` bytes in sparse/empty cases
- `ECPLAYER.DOC` describes this file as the player's planet information
  database, which matches the observed repeated intel-style entries
- best current inference:
  - `DATABASE.DAT` is a derived intel cache, not the authoritative source of
    planet combat state

Conservative `PLANETS.DAT` tail-field candidates:

- `0x5D` is very likely the owning empire slot
  - initialized seed worlds use `1..4` in exactly the expected four-empires
    pattern
  - the colonized world from the fleet-order fixture ends with owner slot `1`
- `0x5C` is likely an ownership/state marker
  - observed as `0x02` on owned colony-style records
- `0x5A` is a strong candidate for army count
  - initialized seed worlds: `4`
  - colonized world from the fleet-order fixture: `0`
  - mature `Dust Bowl` world: `15`
- `0x58` is a developed-world quantity that matters, but it is still unnamed
  - initialized seed worlds: `10`
  - colonized world from the fleet-order fixture: `1`
  - mature `Dust Bowl` world: `142`

These candidates are now exposed conservatively in the Rust parser and CLI as:

- `owner_empire_slot_raw()`
- `ownership_status_raw()`
- `likely_army_count_raw()`
- `developed_value_raw()`

Historical scouting-report reference set:

- the 2012 log bundle contains many repeated scouting reports for the same
  worlds, which gives us planet-side reference values even when we do not have
  matching `.DAT` snapshots for those exact turns
- useful repeated examples include:
  - `Fran` (`Melody Lake`, Empire #2)
    - `ec11.txt`: potential `100`, present `100`, stored `51`, armies `15`,
      ground batteries `5`, stardock `2 cruisers`
    - `ec12.txt`: same except ground batteries `6`
    - `ec13.txt`: same except stored `36`, ground batteries `7`
    - `ec15.txt`: same except ground batteries `8`
  - `33` (`Melody Lake`, Empire #2)
    - `ec17.txt`: potential `33`, present `11`, stored `6`, armies `0`,
      ground batteries `0`, stardock `2 destroyers`
    - `ec18.txt`: present `13`, stored `7`, stardock `1 destroyer`
    - `ec19.txt`: present `14`, stored `9`, stardock `1 destroyer`
  - `90` (`Melody Lake`, Empire #2)
    - `ec13.txt`: potential `90`, present `35`, stored `27`, armies `4`,
      ground batteries `0`
    - `ec14.txt`: present `38`, stored `26`, armies `4`,
      ground batteries `1`

Why this matters:

- these repeated profiles give us expected movement over time for:
  - current production
  - stored goods
  - armies
  - ground batteries
  - docked ships
- they are the best available reference set for naming additional `PLANETS.DAT`
  fields before we have exact matching historical snapshots in repo

Stable companion reference:

- see `docs/planet-report-reference.md` for the coordinate-linked condensed
  version of these report-side target profiles

Preservation value:

- this is the first fixture-backed sequence showing a two-step attack lifecycle:
  - year 1: move into bombard position
  - year 2: consume bombard order and inflict/receive ship losses
- even though planet damage was not achieved, the sequence still exposes useful `ECMAINT`
  behavior for later faithful combat modeling

Confirmed `SETUP.DAT` offsets from the live `F4` diffs:

- `SETUP[512]` `snoop_enabled`
- `SETUP[513]` `max_time_between_keys_minutes_raw`
- `SETUP[515]` `remote_timeout_enabled`
- `SETUP[516]` `local_timeout_enabled`
- `SETUP[517]` `minimum_time_granted_minutes_raw`
- `SETUP[518]` `purge_after_turns_raw`
- `SETUP[520]` `autopilot_inactive_turns_raw`

## Fleet Command Surface

Preserved screenshot references:

- `original/v1.5/EC-Screenshots-v1.11/fleet-command-menu.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-h.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o-5.png`
- `original/v1.5/EC-Screenshots-v1.11/fleet-command-o-12.png`

Confirmed Fleet Command Center options:

- `H` Help with commands
- `Q` Quit to main menu
- `X` Xpert mode ON/OFF
- `S` STARBASE MENU...
- `V` View partial Starmap
- `B` Brief List of Fleets
- `F` Fleets/Detailed List
- `R` Review a Fleet
- `O` Order fleet on mission
- `C` Change a fleet's ROE
- `A` Alter a fleet's ID
- `E` ETA calculation
- `D` Detach a Fleet
- `M` Merge a Fleet
- `T` Transfer (reassign) ships
- `L` Load Armies to Transports
- `U` Unload Armies from Transport

Confirmed mission code menu under `O` Order fleet on mission:

- `0` None (hold position)
- `1` Move Fleet (only)
- `2` Seek Home
- `3` Patrol a Sector
- `5` Guard/Blockade a World
- `6` Bombard a World
- `9` View a World
- `12` Colonize a World
- `13` Join another fleet

Confirmed order-entry prompt shape:

- the game asks for X/Y destination coordinates for at least:
  - `5` Guard/Blockade a World
  - `12` Colonize a World
- it then prints travel time and resulting ETA year
- it prompts for current speed up to the fleet's maximum speed
- all missions may implicitly include movement if required

Practical implication for the Rust port:

- the preserved screenshots are now enough to build a first faithful text-mode Fleet Command menu
- known raw order codes in `FLEETS.DAT` can be displayed as named mission kinds instead of plain
  numbers

## Most Useful Next Diffs

To label fields efficiently, the best actions are:

1. Order a single fleet to a known coordinate and capture the exact command parameters, then diff `FLEETS.DAT`
2. Issue one isolated planet build order from a fresh snapshot and diff `PLANETS.DAT`
3. Run maintenance and diff `PLANETS.DAT`, `FLEETS.DAT`, `DATABASE.DAT`, and `RESULTS.DAT`
4. Change another empire-level economic setting and diff `PLAYER.DAT`

## Porting Strategy

Recommended approach:

1. Decode file formats.
2. Use before/after diffs from a few in-game actions to label fields.
3. Map command families from the working menus.
4. Reimplement behavior in a modern language from the observed state machine and file model.

Avoid trying to recover original source verbatim. A compatible reimplementation is more realistic.

## Preservation Target

Current recommendation for a preservation-oriented reimplementation:

- target language: `Rust`

Reasoning:

- the main archival goal is long-term maintainability and behavioral correctness, not fastest prototype speed
- `Rust` is a strong fit for:
  - exact binary record parsers/serializers for original `.DAT` files
  - strongly typed game-state models for players, planets, fleets, and maintenance phases
  - conformance tests against the original DOS behavior
  - a clean separation between:
    - core engine
    - file compatibility layer
    - standalone terminal UI
    - optional BBS/door compatibility adapter

Suggested crate layout:

- `ec-core`: rules, turn processing, economy, combat, maintenance
- `ec-data`: original file codecs and compatibility structures
- `ec-cli`: standalone terminal/text interface
- `ec-door`: optional BBS door adapter for legacy use
- `ec-import`: import or convert original EC 1.5 game state

Practical note:

- `Nim` would still be the faster experimentation language
- `Rust` is the better fit if the explicit goal is to preserve the game for posterity

Current scaffold status:

- a Rust workspace now exists under [`rust`](/home/niltempus/dev/esterian_conquest/rust)
- first crate: [`ec-data`](/home/niltempus/dev/esterian_conquest/rust/ec-data)
- first executable tool: [`ec-cli`](/home/niltempus/dev/esterian_conquest/rust/ec-cli)
- preserved fixture sets now include:
  - [`original/v1.5`](/home/niltempus/dev/esterian_conquest/original/v1.5)
  - [`fixtures/ecutil-init/v1.5`](/home/niltempus/dev/esterian_conquest/fixtures/ecutil-init/v1.5)
  - [`fixtures/ecmaint-post/v1.5`](/home/niltempus/dev/esterian_conquest/fixtures/ecmaint-post/v1.5)
- current code covers only confirmed fixed-size boundaries:
  - `PLAYER.DAT`: `5 x 88`
  - `PLANETS.DAT`: `20 x 97`
  - initialized `FLEETS.DAT`: `16 x 54`
  - `SETUP.DAT`: `522`
  - `CONQUEST.DAT`: `2085`
- unknown regions are intentionally preserved as raw byte arrays
- current test status: `cargo test` passes in the original archive workspace and now also in the GitHub-tracked preservation repo
- `ec-cli` now provides a first inspection command against `original/v1.5`
- `ec-cli init` now reproduces the known `ECUTIL` new-game initialization result by overlaying the preserved initialized fixture set onto a target directory
- the post-maint fixture set captures another confirmed RE result:
  - `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `SETUP.DAT` match the initialized baseline after maintenance
  - `CONQUEST.DAT` and `DATABASE.DAT` preserve the global maintenance/output differences
- `ec-cli` now also provides:
  - `headers` to dump the currently known `SETUP.DAT` option prefix and `CONQUEST.DAT` header words
  - `match` to identify whether a directory matches the preserved shipped, initialized, or post-maint fixture states
  - `compare` with integration coverage for the key fixture-state transitions

## Screenshot Archive

Captured gameplay screenshots were copied to:

- [ecv1.5](/home/niltempus/Pictures/ecv1.5)
