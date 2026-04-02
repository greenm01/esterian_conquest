# Mystic Rust Door Setup

Mystic is the current baseline local-door BBS host for the Rust-native
`nc-door` path.

Status note:

- this path is validated with the current Rust door client

Use:

- `nc-sysop` to create and maintain the campaign
- `nc-door` as the staged player door on Unix-like hosts
- `nc-door.exe` as the staged player door on native Windows hosts
- Mystic's native door commands, not DOS wrappers
- [`tools/bbs/run_nc_rust.sh`](../../tools/bbs/run_nc_rust.sh) only as a
  source-tree/dev helper on Unix-like hosts

Use the dropfile/transport that matches the host:

- Unix-like Mystic hosts can use `DC` with `CHAIN.TXT`
- native Windows Mystic hosts should use `D3` with `DOOR32.SYS`

## 1. Build the Rust binaries

For Linux BBS hosting, use the public `nc-sysop` package or build from source.
Localhost play remains a source-build `nc-game` path.

From the repo root:

```bash
cd rust
cargo build -q --release -p nc-game -p nc-sysop
```

## 2. Create a campaign

Example:

Create `/path/to/ec-campaign/config.kdl`:

```kdl
players 4
reservations {
    seat player=1 alias="mag"
    seat player=2 alias="NightShade"
}
```

Then initialize the campaign:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/ec-campaign
```

If you need a reproducible map for a one-off test, keep the seed on the
creation command line instead of in `config.kdl`:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/ec-campaign --seed 1515
```

Seat reservations are optional. If you want to add or change them later, use:

```bash
cd rust
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 1 --alias mag
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 2 --alias NightShade
```

If a caller alias is not reserved, Mystic can still launch `nc-door` from
dropfile data alone:

- returning callers resume automatically by stored caller handle
- new callers land on the BBS first-time menu
- `J` claims the lowest-numbered open unreserved empire only when the join is
  confirmed
- if the game is full, the caller still reaches the first-time menu, but `J`
  is refused

Run yearly maintenance with your normal host tooling:

```bash
cd rust
cargo run -q -p nc-sysop -- maint /path/to/ec-campaign 1
```

## 3. Install Mystic

Install Mystic from the upstream build for your platform. On Linux, the
documented non-interactive path is:

```bash
./install auto /path/to/mystic
```

For a local-only test harness, bind Mystic to a non-privileged localhost port
such as `127.0.0.1:2323`.

## 4. Add the EC door

### Unix-like Mystic hosts

Use Mystic's `DC` menu command. That command writes `CHAIN.TXT` into the
node temp directory, and `%P` expands to that directory with the trailing
separator already included.

Door command:

```text
DC
```

Door data:

```text
/path/to/nc-door --dir /path/to/ec-campaign --dropfile %PCHAIN.TXT --encoding cp437 --color-mode ansi16
```

If you are wiring Mystic from a live source tree instead of a staged binary,
`tools/bbs/run_nc_rust.sh` remains a convenient Unix-like helper.

### Native Windows Mystic hosts

Use Mystic's `D3` menu command. That command writes `DOOR32.SYS` into the
node temp directory and passes the native socket handoff that `nc-door.exe`
expects on Windows.

Door command:

```text
D3
```

Door data:

```text
C:\path\to\nc-door.exe --dir C:\path\to\ec-campaign --dropfile %Pdoor32.sys
```

For a permanent Windows install, point Mystic directly at the staged
`nc-door.exe`. Keep `run_nc_rust.cmd` only as a source-tree/dev helper.

## 5. Start Mystic

From the Mystic root:

```bash
./mis root /path/to/mystic server
```

For local testing, connect with SyncTERM or telnet to the configured port.

## 6. Validate

The expected first-pass smoke test is:

1. create or log into a Mystic user, either reserved or brand-new
2. open the Doors menu
3. launch the NC entry
4. confirm a new unreserved caller lands on the EC first-time menu in color on
   the normal `80x25` playfield
5. verify that normal navigation and paging work on list screens
6. choose `J` and verify the join flow reaches empire naming when an open
   unreserved empire exists

### Native Windows note

The verified Windows-native Mystic setup is:

```text
D3
C:\Mystic\doors\nc-game\bin\nc-door.exe --dir C:\Mystic\doors\nc-game\campaign --dropfile %Pdoor32.sys
```

That path was smoke-tested on Windows with SyncTERM against a normal
`C:\Mystic` install. `DC` with `CHAIN.TXT` is still correct for Unix-like
Mystic hosts, but it is not the preferred native Windows path.
