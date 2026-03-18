# Esterian Conquest README

_Readable Markdown transcription of [`original/v1.5/ECREADME.DOC`](/home/mag/dev/esterian_conquest/original/v1.5/ECREADME.DOC)._

The original `.DOC` file remains the preserved source artifact. This
transcription is provided for easier reading, quoting, and linking.

This document is the original quick installation readme for sysops.

## Features That Sysops Like About Esterian Conquest

- Low-maintenance. The original readme describes EC as easy to install and
  largely self-reliant after initialization.
- Self-correcting. The game automatically backs up the previous state before
  processing moves and can restore that backup if integrity checks fail.
- Bulletproof. The original readme emphasizes resilience against line noise,
  disconnects, and missed keystrokes.
- Multi-node. The game supports simultaneous multi-node play.
- Customized operation. Sysops can control player count, reserve empires,
  specify maintenance days, and configure other options through `ECUTIL`.

## Features That Users Will Love

- Realistic action. The original readme calls out battle, planetary growth,
  espionage, seven ship types, fifteen fleet missions, and detailed planetary
  control.
- Advanced, yet easy to use. Menus, optional ANSI graphics, multi-command
  entry, expert mode, and in-game help are all highlighted.
- Extras that go the full nine yards. Built-in messaging, autopilot, and
  word-wrapped mission reports are part of the original pitch.

## System Requirements

The original readme lists these requirements:

- MS-DOS `2.11` or later
- `512K` of memory, with `640K` recommended for larger `25`-player games
- a hard disk with at least `1MB` free
- BBS software that can generate one of these door-file formats:

```text
PCBOARD.SYS   (PCBoard v14+)
DOOR.SYS      (GAP standard)
DORINFOx.DEF  (QBBS, RBBS, RA, FoReM, etc.)
CALLINFO.BBS  (Wildcat!)
SFDOORS.DAT   (Spitfire)
CHAIN.TXT     (WWIV)
INFO.BBS      (Phoenix)
```

## Files on the Program Diskette

- `ECUTIL.EXE` — Esterian Conquest Sysop Utility
- `ECGAME.EXE` — the user interface
- `ECMAINT.EXE` — the maintenance program
- `WHATSNEW.DOC` — revision and update notes
- `ECREADME.DOC` — this installation readme
- `ECSYSOP.DOC` — sysop guide
- `ECQSTART.DOC` — quick-start guide for players
- `ECPLAYER.DOC` — full player guide

## Installing Esterian Conquest

The original readme describes four high-level steps:

1. Copy the program files into a directory.
2. Create a door-game batch file to run `ECGAME`.
3. Add commands to run `ECMAINT` to the BBS daily event batch file.
4. Make `ECPLAYER.DOC` and `ECQSTART.DOC` available for players to download.

### Copy the Program Files

The original recommendation is:

- create a directory such as `\EC` for the program files
- copy `ECUTIL.EXE`, `ECGAME.EXE`, and `ECMAINT.EXE` there
- create a separate game directory such as `\GAME01`
- use additional game directories like `\GAME02` if you want multiple games

### Door-Game Batch File

The readme says to switch into the game directory before running `ECGAME`, and
to pass the path that contains the door file.

Example batch commands from the original readme:

```text
C:
CD\GAME01
C:\EC\ECGAME C:\DOOR
```

The last line means:

- run `ECGAME` from `C:\EC`
- point it at the directory containing the active door file such as
  `PCBOARD.SYS` or `DOOR.SYS`

### Add the Maintenance Program to the Daily Event Batch File

The readme says to switch into the game directory before running `ECMAINT`.

It also includes an important warning:

- adding `ECMAINT` to the event batch file effectively starts the game
- sysops should advertise the game first and give players time to sign up
  before enabling maintenance

Example daily-event commands from the original readme:

```text
C:
CD\GAME01
C:\EC\ECMAINT /R
COPY RANKINGS.TXT \BULLETINS
```

The `/R` example generates a rankings file, which the readme then copies into
the BBS bulletins directory.

### Player Documentation Files

The original readme tells sysops to make `ECPLAYER.DOC` and `ECQSTART.DOC`
available to users, and suggests compressing them with tools such as `PKZIP`.

After that, the readme says the game is ready to be initialized through
`ECUTIL.EXE`.
