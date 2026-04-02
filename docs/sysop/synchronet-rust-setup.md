# Synchronet Rust Door Setup

This is the native Windows host path for the Rust door.

Validated path:

- use `nc-door.exe` for the live Synchronet door entry
- use `DOOR32` so Synchronet passes the caller metadata and socket descriptor
- use the minimal command line shown below
- keep `Intercept I/O Interrupts` off for this native socket door
- treat `HJKL` as the primary movement keys and `^U` / `^D` as the paging keys

Use:

- `nc-sysop` to create and maintain the campaign
- `nc-game.exe` for local/direct play on the Windows host
- `nc-door.exe` for the Synchronet external program entry

## 1. Build the Rust binaries

From the repo root:

```text
cd rust
cargo build -q --release -p nc-game -p nc-sysop
```

The `nc-game` package now builds both Windows binaries:

- `target\release\nc-game.exe`
- `target\release\nc-door.exe`

For a normal sysop layout, stage them somewhere stable such as:

```text
C:\SBBS\xtrn\nc-game\bin\nc-game.exe
C:\SBBS\xtrn\nc-game\bin\nc-door.exe
C:\SBBS\xtrn\nc-game\bin\nc-sysop.exe
```

## 2. Create a campaign

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

## 3. Add the native program entry

In `SCFG`, add `nc-door.exe` to the Native Program List.

Use the staged binary, not a batch wrapper, and keep the external-program
command itself as the bare executable name with `startup_dir` pointed at the
staged binary directory.

Suggested startup directory:

```text
C:\SBBS\xtrn\nc-game\bin
```

Tested command line:

```text
nc-door.exe --dir C:\SBBS\xtrn\nc-game\campaign --dropfile %f --socket-descriptor %H
```

Do not add `--encoding` or `--color-mode` here. In door mode, `nc-door.exe`
already defaults to the expected CP437/ANSI behavior from the dropfile path,
and the minimal command line was the live-tested path on Windows Synchronet.

## 4. Add the external program

In the online external programs section, configure the entry as a native
Windows door:

- section: `Games` or equivalent
- drop file type: `DOOR32`
- startup directory: `C:\SBBS\xtrn\nc-game\bin`
- command line: the tested command above
- `Native (32-bit) Executable`: `Yes`
- `Intercept I/O Interrupts`: `No`
- `Multiuser`: `Yes`
- `ANSI`: `Yes`
- do not use `cmd.exe /c`, `run_nc_rust.cmd`, `nc-game.exe`, or a DOS wrapper

Why `DOOR32`:

- Synchronet writes the caller alias and time-left metadata `nc-game` already uses
- on Windows, `nc-door.exe` also needs the duplicated socket descriptor passed
  explicitly with `%H`, so the session stays inside the caller's terminal
  instead of opening a second console window

If you edit `xtrn.ini` directly instead of using `SCFG`, the tested working
entry was:

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

## 5. Validate

The expected first-pass smoke test is:

1. connect with SyncTERM or another telnet-capable BBS client
2. log in and open the external programs menu
3. launch the NC entry
4. confirm the game renders in the caller session with no extra console window
5. verify `HJKL` movement and `^U` / `^D` paging
6. quit and confirm control returns cleanly to Synchronet

## 6. Troubleshooting

- if the door opens a second Windows console window, you are launching
  `nc-game.exe` or a stdio wrapper instead of `nc-door.exe --socket-descriptor %H`
- if the door launches but keypresses hang, verify `Intercept I/O Interrupts`
  is off and the entry resolves to `settings=16387`, not `16391`
- if Synchronet reports argument errors such as unknown encoding, color mode,
  or path fragments, strip the command back to the minimal tested form above
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
