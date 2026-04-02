# Synchronet Rust Door Setup

This is the native Windows host path for the Rust door.

Status note:

- use `nc-door.exe` for the live Synchronet door entry
- use `DOOR32` so Synchronet passes the caller metadata and socket descriptor
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

In `SCFG`, add `nc-door.exe` to the Native Program List. Use the staged binary,
not a batch wrapper.

Suggested startup directory:

```text
C:\SBBS\xtrn\nc-game\bin
```

Suggested command line:

```text
C:\SBBS\xtrn\nc-game\bin\nc-door.exe --dir C:\SBBS\xtrn\nc-game\campaign --dropfile %f --encoding cp437 --color-mode ansi16
```

## 4. Add the external program

In the online external programs section:

- put the entry in your `Games` section or equivalent
- set the drop file type to `DOOR32`
- point the program at the native command above
- do not use `cmd.exe /c`, `run_nc_rust.cmd`, or a DOS wrapper

Why `DOOR32`:

- Synchronet writes the caller alias and time-left metadata `nc-game` already uses
- on Windows, `nc-door.exe` can also adopt the inherited `DOOR32` socket
  descriptor, so the session stays inside the caller's terminal instead of
  opening a second console window

## 5. Validate

The expected first-pass smoke test is:

1. connect with SyncTERM or another telnet-capable BBS client
2. log in and open the external programs menu
3. launch the NC entry
4. confirm the game renders in the caller session with no extra console window
5. verify `HJKL` movement and `^U` / `^D` paging
6. quit and confirm control returns cleanly to Synchronet

## 6. Troubleshooting

- if the door returns immediately or the screen stays blank, check the
  Synchronet node log first
- if Windows Security or third-party AV interferes, verify that it is not
  blocking inherited socket-handle use in the `sbbs.exe` child process
- if you need a source-tree helper for ad hoc testing, keep
  [`tools/bbs/run_nc_rust.cmd`](../../tools/bbs/run_nc_rust.cmd) as a dev-only
  fallback, not the permanent sysop path
