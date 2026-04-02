# Synchronet BBS Setup

This page covers Synchronet as a BBS host for `nc-door`.

Validated path:

- use `nc-door.exe` for the live Synchronet door entry
- use `DOOR32` so Synchronet passes the caller metadata and socket descriptor
- use the minimal command line shown below
- keep `Intercept I/O Interrupts` off for this native socket door
- this path is live-verified on a normal Windows `C:\SBBS` install with
  SyncTERM
- Linux Synchronet with `nc-door` is also validated through SyncTERM using a
  native `DOOR32` entry and a tiny wrapper that receives `%f` and execs
  `nc-door --dir ... --dropfile "$1"`

Use:

- `nc-sysop` to create and maintain the campaign
- `nc-game.exe` for local/direct play on the Windows host
- `nc-door.exe` for the Synchronet external program entry
- `nc-game` for local/direct play on Linux
- `nc-door` for the Synchronet external program entry on Linux

## 1. Platform Status

This repo now carries two tested Synchronet paths for `nc-door`:

- Windows:
  - live-verified on a normal `C:\SBBS` install with SyncTERM
  - use `DOOR32`
  - pass `--socket-descriptor %H`
- Linux:
  - validated on a localhost Synchronet install with SyncTERM
  - use `DOOR32`
  - keep the external program native, not DOS
  - use the wrapper shape shown below if Synchronet does not preserve the full
    `nc-door --dir ... --dropfile %f` command line cleanly

On both platforms, keep the command surface minimal:

- stage `nc-door`
- pass `--dir`
- pass a real dropfile
- do not add `--encoding` or `--color-mode` unless you are debugging a
  host-specific issue

## 2. Build the Rust binaries

From the repo root:

```text
cd rust
cargo build -q --release -p nc-game -p nc-sysop
```

The `nc-game` package builds the door entrypoint on both platforms:

- Windows:
  - `target\release\nc-game.exe`
  - `target\release\nc-door.exe`
  - `target\release\nc-sysop.exe`
- Linux:
  - `target/release/nc-game`
  - `target/release/nc-door`
  - `target/release/nc-sysop`

For a normal sysop layout, stage them somewhere stable such as:

```text
C:\SBBS\xtrn\nc-game\bin\nc-game.exe
C:\SBBS\xtrn\nc-game\bin\nc-door.exe
C:\SBBS\xtrn\nc-game\bin\nc-sysop.exe
```

or on Linux:

```text
/srv/sbbs/xtrn/nc-game/bin/nc-game
/srv/sbbs/xtrn/nc-game/bin/nc-door
/srv/sbbs/xtrn/nc-game/bin/nc-sysop
```

## 3. Create a campaign

Example `config.kdl`:

```kdl
players 4
reservations {
    seat player=1 alias="SYSOP"
}
```

Initialize the BBS campaign:

```text
cd rust
cargo run -q -p nc-sysop -- new-game --bbs C:\SBBS\xtrn\nc-game\campaign
```

If you want a reproducible map for testing, keep the seed on the creation
command line instead of in `config.kdl`:

```text
cd rust
cargo run -q -p nc-sysop -- new-game --bbs C:\SBBS\xtrn\nc-game\campaign --seed 1515
```

Run yearly maintenance with Task Scheduler, a nightly event, or your normal
host tooling:

```text
cd rust
cargo run -q -p nc-sysop -- maint C:\SBBS\xtrn\nc-game\campaign 1
```

## 4. Add the native program entry

In `SCFG`, add `nc-door.exe` on Windows or `nc-door` on Linux to the Native
Program List.

Use the staged binary, not a batch wrapper, and keep the external-program
command itself as the bare executable name with `startup_dir` pointed at the
staged binary directory unless you need the Linux wrapper below.

Suggested startup directories:

```text
C:\SBBS\xtrn\nc-game\bin
/srv/sbbs/xtrn/nc-game/bin
```

Tested Windows command line:

```text
nc-door.exe --dir C:\SBBS\xtrn\nc-game\campaign --dropfile %f --socket-descriptor %H
```

Do not add `--encoding` or `--color-mode` here. In door mode, `nc-door.exe`
already defaults to the expected CP437/ANSI behavior from the dropfile path,
and the minimal command line was the live-tested path on Windows Synchronet.

Validated Linux command line:

```text
bash /srv/sbbs/xtrn/nc-game/bin/sbbs-nc-door.sh %f
```

with wrapper contents:

```text
#!/usr/bin/env bash
set -euo pipefail
exec /srv/sbbs/xtrn/nc-game/bin/nc-door --dir /srv/sbbs/xtrn/nc-game/campaign --dropfile "$1"
```

That wrapper shape is the tested Linux path because it avoids Synchronet
argument-mangling cases where a long native `cmd=` entry gets split badly and
`nc-door` sees truncated flags such as `--d`.

## 5. Add the external program

In the online external programs section, configure the entry as a native
Synchronet door:

- section: `Games` or equivalent
- drop file type: `DOOR32`
- startup directory: your staged `bin` directory
- command line: the tested platform-specific command above
- `Native (32-bit) Executable`: `Yes` on Windows
- `Native` executable bit: `Yes` on Linux too
- `Intercept I/O Interrupts`: `No`
- `Multiuser`: `Yes`
- `ANSI`: `Yes`
- do not use `cmd.exe /c`, `run_nc_rust.cmd`, `nc-game.exe`, or a DOS wrapper

Why `DOOR32`:

- Synchronet writes the caller alias and time-left metadata `nc-game` already uses
- on Windows, `nc-door.exe` also needs the duplicated socket descriptor passed
  explicitly with `%H`, so the session stays inside the caller's terminal
  instead of opening a second console window
- on Linux, `DOOR32` still provides the caller metadata `nc-door` expects, but
  the tested path does not use `--socket-descriptor`

If you edit `xtrn.ini` directly instead of using `SCFG`, the tested working
Windows entry was:

```text
[prog:GAMES:NCGAME]
name=Nostrian Conquest
type=12
settings=16387
cmd=nc-door.exe --dir C:\SBBS\xtrn\nc-game\campaign --dropfile %f --socket-descriptor %H
startup_dir=C:\SBBS\xtrn\nc-game\bin\
```

`settings=16387` is the working combination for:

- `Multiuser`
- `ANSI`
- `Native (32-bit) Executable`

Do not use `16391` for this setup. That extra `Intercept I/O Interrupts` bit
caused the door to launch but hang on input during live Windows testing.

The validated Linux `xtrn.ini` shape was:

```text
[prog:GAMES:NCGAME]
name=Nostrian Conquest
type=12
settings=0x14005
cmd=bash /srv/sbbs/xtrn/nc-game/bin/sbbs-nc-door.sh %f
startup_dir=/srv/sbbs/xtrn/nc-game/bin
```

The important Linux bits are:

- `type=12` for `DOOR32`
- keep the entry native, not DOS
- keep `Intercept I/O Interrupts` off
- use a wrapper if direct long arguments do not survive cleanly through
  Synchronet

## 6. Validate

The expected first-pass smoke test is:

1. connect with SyncTERM or another telnet-capable BBS client
2. log in and open the external programs menu
3. launch the NC entry
4. confirm the game renders in the caller session with no extra console window
5. verify normal navigation and paging behavior
6. quit and confirm control returns cleanly to Synchronet

On Linux, a good terminal baseline is SyncTERM in classic 80x24 BBS view.

## 7. Troubleshooting

- if the door opens a second Windows console window, you are launching
  `nc-game.exe` or a stdio wrapper instead of `nc-door.exe --socket-descriptor %H`
- if the door launches but keypresses hang, verify `Intercept I/O Interrupts`
  is off and the entry resolves to `settings=16387`, not `16391`
- if Synchronet reports argument errors such as unknown encoding, color mode,
  or path fragments, strip the command back to the minimal tested form above
- if Linux Synchronet reports truncated arguments such as `unknown argument:
  --d`, move the full `nc-door --dir ... --dropfile ...` command into a tiny
  wrapper script and keep `cmd=` itself to `bash /path/to/sbbs-nc-door.sh %f`
- if Synchronet says DOS programs are not supported on this node, the external
  program is missing the native executable bit
- if the door returns immediately or the screen stays blank, check the
  Synchronet node log first
- if Windows Security or third-party AV interferes, verify that it is not
  blocking inherited socket-handle use in the `sbbs.exe` child process
- if a local test port gets wedged, move `TelnetPort` in `ctrl\sbbs.ini` to a
  fresh localhost port such as `2324`, restart Synchronet, and reconnect your
  test client to that new port
- if you need a source-tree helper for ad hoc testing, keep
  [`tools/bbs/run_nc_rust.cmd`](../../tools/bbs/run_nc_rust.cmd) as a dev-only
  fallback, not the permanent sysop path
