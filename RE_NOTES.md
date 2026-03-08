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

- `0x0A` is likely a mission type / order code
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

## Screenshot Archive

Captured gameplay screenshots were copied to:

- [ecv1.5](/home/niltempus/Pictures/ecv1.5)
