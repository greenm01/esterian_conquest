# Esterian Conquest Legacy Readme

_v1.6 reference edition based on
[`original/v1.5/ECREADME.DOC`](/home/niltempus/dev/esterian_conquest/original/v1.5/ECREADME.DOC)._

This file exists to preserve the release and deployment context of the original
DOS package without presenting that old operational material as the current
`v1.6` setup path.

## How To Use This In v1.6

Read this when you want historical context on how Esterian Conquest was shipped
and hosted in the DOS door era. For current deployment guidance, use the Rust
documentation in [docs/sysop](/home/niltempus/dev/esterian_conquest/docs/sysop/).
If you want the untouched original text, read
[ECREADME.DOC](/home/niltempus/dev/esterian_conquest/original/v1.5/ECREADME.DOC).

## What This Document Originally Covered

The original readme was a short operator note for setting up Esterian Conquest
as a DOS BBS door. It emphasized that the game was easy to install, could back
up and restore itself when maintenance failed integrity checks, and supported
multi-node play.

That material is still useful as project history. It is not the recommended
deployment model for `v1.6`.

## Historical DOS Package Expectations

The original package assumed a DOS host, a small hard drive, and a BBS setup
capable of launching door programs and supplying one of the supported dropfile
formats. It also assumed the bundled DOS binaries were the live game engine and
maintenance path.

Those assumptions are now legacy-only. In `v1.6`, the primary stack is:

- `ec-cli` for campaign setup and admin work
- `maint-rust` for turn processing
- `ec-client` as the long-term replacement for `ECGAME`

If you still want to host the original DOS client for compatibility reasons,
use the modern guidance in
[docs/sysop/enigma-bbs-setup.md](/home/niltempus/dev/esterian_conquest/docs/sysop/enigma-bbs-setup.md)
instead of following the original readme literally.

## Historical Package Contents

The original readme treated these files as the main shipped operator bundle:

| File | Original role |
| --- | --- |
| `ECUTIL.EXE` | Sysop utility |
| `ECGAME.EXE` | Player interface |
| `ECMAINT.EXE` | Maintenance program |
| `ECREADME.DOC` | Installation readme |
| `ECSYSOP.DOC` | Sysop guide |
| `ECQSTART.DOC` | Quick-start player guide |
| `ECPLAYER.DOC` | Full player guide |

That bundle matters today mainly as a compatibility and preservation reference.

## Historical Install Outline

The original setup flow was simple. A sysop copied the DOS binaries into a
program directory, created one or more game directories, arranged for the BBS
to launch `ECGAME.EXE` from the active game directory, and scheduled
`ECMAINT.EXE` to run from a daily event. The player guides were then distributed
as downloadable documentation.

That is still the basic shape of the old DOS deployment model, but the modern
project no longer expects you to build a live campaign that way unless you are
deliberately running a compatibility door.

## What Still Matters From The Original Readme

Three points still carry forward:

- Esterian Conquest was designed to be low-touch once initialized.
- Maintenance integrity and backup behavior are central to safe campaign
  operation.
- Good player documentation was considered part of deployment from the start.

Those ideas remain valid in `v1.6`, even though the runtime stack has changed.
