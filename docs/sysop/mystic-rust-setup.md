# Mystic Rust Door Setup

This is the current baseline local-door BBS host for the Rust-native
`ec-game` client.

Status note:

- this path is validated with the current Rust door client
- callers should use `HJKL` for movement and `^U` / `^D` for paging in door
  mode
- `Esc` and `Q` remain the supported back/quit keys

Use:

- `ec-sysop` to create and maintain the campaign
- `ec-game` as the player door
- Mystic's `DC` door command so Mystic writes `CHAIN.TXT` into `%P`
- [`tools/bbs/run_ec_rust.sh`](../../tools/bbs/run_ec_rust.sh) as the door launcher

`ec-game` already accepts `CHAIN.TXT`, so Mystic does not need a format
translation layer.

## 1. Build the Rust binaries

From the repo root:

```bash
cd rust
cargo build -q --release -p ec-game -p ec-sysop
```

## 2. Create a campaign

Example:

```bash
cd rust
cargo run -q -p ec-sysop -- new-game /path/to/ec-campaign --players 4 --seed 1515
```

Reserve each caller alias in `config.kdl` so the door can resolve the seat
from the dropfile without `--player`:

```kdl
reservations {
    seat player=1 alias="mag"
    seat player=2 alias="NightShade"
}
```

Run yearly maintenance with your normal host tooling:

```bash
cd rust
cargo run -q -p ec-sysop -- maint /path/to/ec-campaign 1
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
/home/niltempus/dev/esterian_conquest/tools/bbs/run_ec_rust.sh /path/to/ec-campaign %PCHAIN.TXT
```

Why `DC`:

- Mystic generates `CHAIN.TXT` automatically
- `ec-game` parses `CHAIN.TXT` directly
- no DOS compatibility wrapper is required

## 5. Start Mystic

From the Mystic root:

```bash
./mis root /path/to/mystic server
```

For local testing, connect with SyncTERM or telnet to the configured port.

## 6. Validate

The expected first-pass smoke test is:

1. create or log into a Mystic user whose alias is reserved in `config.kdl`
2. open the Doors menu
3. launch the EC entry
4. confirm the EC first-time menu renders in color and sits on the normal
   `80x25` playfield
5. verify that `HJKL` navigation and `^U` / `^D` paging work on list screens
6. choose `J` and verify the join flow reaches empire naming
