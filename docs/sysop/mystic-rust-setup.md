# Mystic Rust Door Setup

This is the current baseline local-door BBS host for the Rust-native
`nc-game` client.

Status note:

- this path is validated with the current Rust door client
- callers should use `HJKL` for movement and `^U` / `^D` for paging in door
  mode
- `Esc` and `Q` remain the supported back/quit keys

Use:

- `nc-sysop` to create and maintain the campaign
- `nc-game` as the player door
- Mystic's `DC` door command so Mystic writes `CHAIN.TXT` into `%P`
- [`tools/bbs/run_ec_rust.sh`](../../tools/bbs/run_ec_rust.sh) as the door launcher

`nc-game` already accepts `CHAIN.TXT`, so Mystic does not need a format
translation layer.

## 1. Build the Rust binaries

During the current beta, build these from source or use a direct/private beta
build. A public Linux x64 BBS door package is planned later.

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
seed 1515
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

Seat reservations are optional. If you want to add or change them later, use:

```bash
cd rust
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 1 --alias mag
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 2 --alias NightShade
```

If a caller alias is not reserved, Mystic can still launch `nc-game` with
`CHAIN.TXT` alone:

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

Use Mystic's `DC` menu command. That command writes `CHAIN.TXT` into the
node temp directory, and `%P` expands to that directory with the trailing
separator already included.

Door command:

```text
DC
```

Door data:

```text
/path/to/esterian_conquest/tools/bbs/run_ec_rust.sh /path/to/ec-campaign %PCHAIN.TXT
```

Why `DC`:

- Mystic generates `CHAIN.TXT` automatically
- `nc-game` parses `CHAIN.TXT` directly
- no DOS compatibility wrapper is required

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
3. launch the EC entry
4. confirm a new unreserved caller lands on the EC first-time menu in color on
   the normal `80x25` playfield
5. verify that `HJKL` navigation and `^U` / `^D` paging work on list screens
6. choose `J` and verify the join flow reaches empire naming when an open
   unreserved empire exists
