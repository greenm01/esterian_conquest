# ENiGMA½ Rust Door Setup

Status note:

- this path is not the current validated baseline
- live testing still shows broken full-screen rendering through ENiGMA's
  `abracadabra` local-door path
- prefer [mystic-rust-setup.md](mystic-rust-setup.md) or SSH/local hosting for
  real Rust play right now

This document remains useful for ENiGMA-specific experimentation while the
separate bridge-service path is being built.

Use the native Rust stack:

- `ec-sysop` to create and maintain the campaign
- `ec-game` as the player door
- `abracadabra` in `stdio` mode for the ENiGMA launcher

Do not use the legacy DOS wrapper unless you specifically want to host the
original `ECGAME.EXE`.

## 1. Build the Rust client

From the repo root:

```bash
cd rust
cargo build -q -p ec-game -p ec-sysop
```

For a lower-latency door, prefer release builds:

```bash
cd rust
cargo build -q --release -p ec-game -p ec-sysop
```

The helper script [`tools/bbs/run_ec_rust.sh`](../../tools/bbs/run_ec_rust.sh)
will use `target/release/ec-game` first, then `target/debug/ec-game`, then
fall back to `cargo run`.

## 2. Create a campaign

Example:

```bash
cd rust
cargo run -q -p ec-sysop -- new-game /path/to/ec-campaign --players 4 --seed 1515
```

Run yearly maintenance from cron, `systemd`, or a BBS event hook:

```bash
cd rust
cargo run -q -p ec-sysop -- maint /path/to/ec-campaign 1
```

## 3. Reserve caller aliases in `config.kdl`

For BBS hosting, reserve each caller alias to a fixed empire slot so the door
can resolve the seat from the dropfile:

```kdl
reservations {
    seat player=1 alias="niltempus"
    seat player=2 alias="NightShade"
}
```

If a caller alias is not reserved, `ec-game` still requires `--player`, which
is awkward for a normal BBS door flow.

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
        cmd: /home/niltempus/dev/esterian_conquest/tools/bbs/run_ec_rust.sh
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
- native `ec-game` reads and writes directly on stdin/stdout
- `DOOR32` is still useful for caller alias and timeout metadata

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
        cmd: /home/niltempus/dev/esterian_conquest/tools/bbs/run_ec_rust.sh
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
cmd: /home/niltempus/dev/esterian_conquest/tools/bbs/run_ec_rust.sh
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
