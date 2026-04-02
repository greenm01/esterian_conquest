# ENiGMA½ Rust Door Setup

Status note:

- this path is now verified with the current Rust door client
- ENiGMA callers should use `HJKL` for movement and `^U` / `^D` for paging in
  door mode
- `Esc` and `Q` remain the supported back/quit keys

Use the native Rust stack:

- `nc-sysop` to create and maintain the campaign
- `nc-game` as the player door
- `abracadabra` in `stdio` mode for the ENiGMA launcher

Do not use the legacy DOS wrapper unless you specifically want to host the
original `ECGAME.EXE`.

## 1. Build the Rust client

During the current beta, build these from source or use a direct/private beta
build. A public Linux x64 BBS door package is planned later.

From the repo root:

```bash
cd rust
cargo build -q -p nc-game -p nc-sysop
```

For a lower-latency door, prefer release builds:

```bash
cd rust
cargo build -q --release -p nc-game -p nc-sysop
```

The helper script [`tools/bbs/run_ec_rust.sh`](../../tools/bbs/run_ec_rust.sh)
will use `target/release/nc-game` first, then `target/debug/nc-game`, then
fall back to `cargo run`.

## 2. Create a campaign

Example:

Create `/path/to/ec-campaign/config.kdl`:

```kdl
players 4
seed 1515
reservations {
    seat player=1 alias="niltempus"
    seat player=2 alias="NightShade"
}
```

Then initialize the campaign:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/ec-campaign
```

Run yearly maintenance from cron, `systemd`, or a BBS event hook:

```bash
cd rust
cargo run -q -p nc-sysop -- maint /path/to/ec-campaign 1
```

## 3. Reserve caller aliases

Reservations are optional. If you want to add or change them later, use:

```bash
cd rust
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 1 --alias niltempus
cargo run -q -p nc-sysop -- settings reserve --dir /path/to/ec-campaign --player 2 --alias NightShade
```

If a caller alias is not reserved, `nc-game` still works cleanly from the
dropfile alone:

- returning callers resume automatically by stored caller handle
- new callers land on the BBS first-time menu
- `J` claims the lowest-numbered open unreserved empire only when the join is
  confirmed
- if the game is full, the caller still reaches the first-time menu, but `J`
  is refused

## 4. Add the ENiGMA door entry

Use `abracadabra` with:

- `dropFileType: DOOR32`
- `io: stdio`
- `encoding: cp437`

Example menu entry:

```hjson
doorEsterianConquestRust: {
    desc: Esterian Conquest
    module: abracadabra
    config: {
        name: Esterian Conquest
        dropFileType: DOOR32
        cmd: /path/to/esterian_conquest/tools/bbs/run_ec_rust.sh
        args: [
            "/path/to/ec-campaign"
            "{dropFilePath}"
        ]
        io: stdio
        encoding: cp437
    }
}
```

Why `stdio`:

- ENiGMA `socket` mode is for doors or wrappers that actively connect back to
  `{srvPort}`
- native `nc-game` reads and writes directly on stdin/stdout
- `DOOR32` is still useful for caller alias and timeout metadata

Door-control note:

- treat `HJKL` as the primary movement keys in the Rust door
- use `^U` / `^D` for paging long tables and reports
- do not rely on arrows or `PgUp` / `PgDn` in door sessions

## 5. Optional map-export staging

The wrapper script sets:

- `EC_CLIENT_EXPORT_ROOT=$GAME_DIR/exports` by default

If you want ENiGMA to expose those exports through a download area, also set:

- `EC_CLIENT_QUEUE_DIR`

Example:

```hjson
doorEsterianConquestRust: {
    desc: Esterian Conquest
    module: abracadabra
    config: {
        name: Esterian Conquest
        dropFileType: DOOR32
        cmd: /path/to/esterian_conquest/tools/bbs/run_ec_rust.sh
        args: [
            "/path/to/ec-campaign"
            "{dropFilePath}"
        ]
        io: stdio
        encoding: cp437
        env: {
            EC_CLIENT_QUEUE_DIR: /enigma-bbs/file_base/temp/ec
        }
    }
}
```

See [sysop-map-exports.md](sysop-map-exports.md) for the export/queue flow.

## 6. Local testing with your existing launcher

Your local `~/launch_bbs.sh` can stay as-is if it already:

1. starts ENiGMA
2. opens SyncTERM against `localhost:8888`
3. shuts ENiGMA down when you exit

Once the menu entry above points at `run_ec_rust.sh`, launching the EC door
from ENiGMA should run the Rust client instead of the DOS wrapper.

## 7. Current local config change

If you already have a door block pointing at the DOS wrapper, the minimal swap
is:

```hjson
dropFileType: DOOR32
cmd: /path/to/esterian_conquest/tools/bbs/run_ec_rust.sh
args: [
    "/path/to/ec-campaign"
    "{dropFilePath}"
]
io: stdio
encoding: cp437
```

Replace the old DOS-only pieces:

- `dropFileType: DORINFO`
- `cmd: .../tools/bbs/run_ec_dos.sh`
- `args` containing `{node}` / `{srvPort}`
- `io: socket`
