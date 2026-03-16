# Sysop Map Export Setup

This document covers the first `v1.6` player map-delivery workflow.

The goal is to preserve the classic printable starmap while making it practical
for local and BBS-hosted play:

- players can still trigger a full text starmap dump from `M`
- the Rust client can also write downloadable map files
- sysops can stage those files into a BBS download/queue area

## What Gets Exported

The export produces two player-facing files from the same fog-of-war
projection:

- `ECMAP-P<player>-Y<year>.TXT`
  - printable ASCII starmap
  - fixed text grid
  - `*` marks world locations
  - sector legends on the axes
  - horizontally paged in 18-column printable panels with form-feeds for
    larger maps
- `ECMAP-P<player>-Y<year>.CSV`
  - same coordinate coverage
  - spreadsheet-friendly detail rows

Fog-of-war policy:

- all players can see the full map layout and all world coordinates
- world details only come from that player's `DATABASE.DAT`
- undiscovered worlds still appear on the map as locations
- undiscovered detail fields stay blank/unknown in the companion exports

## Local / Hotseat Usage

Export directly from the player client:

```bash
cd rust
cargo run -q -p ec-client -- \
  --dir /tmp/ec-game \
  --player 1 \
  --export-root /tmp/ec-exports
```

Inside `M`:

- press any key to begin the classic text dump
- press `E` to write the printable `.TXT` map and companion `.CSV`

You can also generate the files directly from the CLI:

```bash
cd rust
cargo run -q -p ec-cli -- map-export /tmp/ec-game 1 /tmp/ec-exports/ECMAP-P1-Y3000.TXT
```

## Door / BBS Staging

The first implementation supports queue-style delivery by staging files into a
configured export area and, optionally, copying them into a queue/download
directory.

`ec-client` recognizes:

- `EC_CLIENT_EXPORT_ROOT`
  - where generated map files are written
- `EC_CLIENT_QUEUE_DIR`
  - optional directory that receives a second copy suitable for BBS download
    queue pickup

Example:

```bash
export EC_CLIENT_EXPORT_ROOT=/bbs/doors/ecgame/exports
export EC_CLIENT_QUEUE_DIR=/bbs/files/player-queue
cd rust
cargo run -q -p ec-client -- --dir /bbs/games/ec --player 2
```

When the player presses `E` from `M`, the client writes both files under the
export root and copies them into the queue directory if one is configured.

## Mystic Example

Recommended approach:

- point `EC_CLIENT_EXPORT_ROOT` at a door-local temp/export directory
- point `EC_CLIENT_QUEUE_DIR` at the Mystic file-queue or staged-download area
- let Mystic handle the actual caller download after the door returns

Suggested shape:

```text
/mystic/doors/ec/exports
/mystic/files/queue/ec
```

Then launch the client/door wrapper with:

```bash
EC_CLIENT_EXPORT_ROOT=/mystic/doors/ec/exports \
EC_CLIENT_QUEUE_DIR=/mystic/files/queue/ec \
...
```

If your callers use SyncTERM or another BBS client with ZMODEM support, Mystic
can handle immediate transfer on the BBS side after the file is queued. The
current Rust client does not run ZMODEM itself.

## ENiGMA½ Example

Recommended approach:

- point `EC_CLIENT_EXPORT_ROOT` at an ENiGMA-owned temp/export directory
- point `EC_CLIENT_QUEUE_DIR` at a directory ENiGMA exposes through its normal
  download or temporary-file flow

Suggested shape:

```text
/enigma-bbs/misc/ec/exports
/enigma-bbs/file_base/temp/ec
```

Then the BBS can expose the queued file using its normal temporary-download or
web-backed delivery path after the door exits.

## Telnet Screen Capture

The `M` command still supports the classic map-capture workflow.

Current Rust behavior:

1. `M` warns the player to turn on screen capture in their telnet client
2. the player slaps a key to begin the text dump
3. the client writes the printable ASCII starmap directly to the terminal
4. the client returns to a completion screen telling the player to turn
   capture off

This preserves the old planning feel while still allowing direct downloadable
exports.
