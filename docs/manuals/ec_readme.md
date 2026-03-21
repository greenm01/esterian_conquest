# Esterian Conquest Legacy Readme

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
This is an independent reimplementation and is not affiliated with the original._

_Reference edition based on
[`original/v1.5/ECREADME.DOC`](../../original/v1.5/ECREADME.DOC)._

This file exists to preserve the release and deployment context of the original
DOS package without presenting that old operational material as the current
the EC Rust setup path.

## How To Use This In EC Rust

Read this when you want historical context on how Esterian Conquest was shipped
and hosted in the DOS door era. For current deployment guidance, use the Rust
documentation in [docs/sysop](../sysop/).
If you want the untouched original text, read
[ECREADME.DOC](../../original/v1.5/ECREADME.DOC).

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

## What Still Matters From The Original Readme

Three points still carry forward:

- Esterian Conquest was designed to be low-touch once initialized.
- Maintenance integrity and backup behavior are central to safe campaign
  operation.
- Good player documentation was considered part of deployment from the start.

Those ideas remain valid in EC Rust, even though the runtime stack has changed.
