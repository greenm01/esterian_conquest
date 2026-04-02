# ENiGMA½ BBS Setup

ENiGMA½ is a verified BBS host for `nc-door`.

The important split is simple. On both Linux and Windows, ENiGMA½ should stage
`nc-door` as the live door binary and `nc-sysop` as the campaign tool. On
native Windows, ENiGMA½ should use `abracadabra` in `socket` mode so
`nc-door.exe` connects back to ENiGMA's temporary localhost socket. Do not use
the legacy DOS wrapper unless you specifically want to host the original
`ECGAME.EXE`.

## 1. Build or stage the Rust binaries

For Linux BBS hosting, use the public `nc-sysop` package or build from source.
Localhost play remains a source-build `nc-game` path.

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

For a live BBS host, stage `target/release/nc-door` or `target/release/nc-door.exe`
and point ENiGMA directly at that binary. Keep
[`tools/bbs/run_nc_rust.sh`](../../tools/bbs/run_nc_rust.sh) only as a
source-tree/dev helper on Unix-like hosts.

For a normal native Windows install, the simplest layout is to stage the
public Windows `nc-sysop` package under the ENiGMA root itself. The verified
layout on Windows was:

```text
C:\enigma-bbs\doors\nc-game\nc-door.exe
C:\enigma-bbs\doors\nc-game\nc-sysop.exe
C:\enigma-bbs\doors\nc-game\campaign\
```

That keeps the door binary, sysop tool, and campaign under the same permanent
sysop-owned tree instead of relying on temp paths or source-tree wrappers.

## 2. Create a campaign

Example:

Create `/path/to/ec-campaign/config.kdl`:

```kdl
players 4
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

If you need a reproducible map for a one-off test, keep the seed on the
creation command line instead of in `config.kdl`:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/ec-campaign --seed 1515
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

If a caller alias is not reserved, `nc-door` still works cleanly from the
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
- `io: socket`
- `encoding: cp437`

Example menu entry:

```hjson
doorEsterianConquestRust: {
    desc: Esterian Conquest
    module: abracadabra
    config: {
        name: Esterian Conquest
        dropFileType: DOOR32
        cmd: /path/to/nc-door
        args: [
            "--dir"
            "/path/to/ec-campaign"
            "--dropfile"
            "{dropFilePath}"
            "--socket-port"
            "{srvPort}"
            "--encoding"
            "cp437"
            "--color-mode"
            "ansi16"
        ]
        io: socket
        encoding: cp437
    }
}
```

Windows-native command swap:

```hjson
cmd: C:\\path\\to\\nc-door.exe
args: [
    "--dir"
    "C:\\path\\to\\ec-campaign"
    "--dropfile"
    "{dropFilePath}"
    "--socket-port"
    "{srvPort}"
    "--encoding"
    "cp437"
    "--color-mode"
    "ansi16"
]
```

Why `socket`:

- ENiGMA½ does not directly share a `DOOR32.SYS` socket descriptor with a
  child process
- ENiGMA's `abracadabra` `stdio` path runs through `node-pty`, which is not a
  good match for the native Windows `nc-door.exe` GUI entrypoint
- `nc-door` can instead connect back to ENiGMA's temporary localhost socket by
  using `--socket-port {srvPort}`
- `DOOR32` still carries caller alias and timeout metadata through
  `{dropFilePath}`

### Native Windows ENiGMA setup

On native Windows, treat ENiGMA as a permanent local install. Put the BBS in a
stable root such as `C:\enigma-bbs`. Stage the Windows `nc-sysop` package
under `C:\enigma-bbs\doors\nc-game`, create the campaign there, and point the
door entry straight at `nc-door.exe`.

The live-tested Windows menu block was:

```hjson
doorNostrianConquest: {
    desc: Nostrian Conquest
    module: abracadabra
    config: {
        name: Nostrian Conquest
        dropFileType: DOOR32
        cmd: C:\\enigma-bbs\\doors\\nc-game\\nc-door.exe
        args: [
            "--dir"
            "C:\\enigma-bbs\\doors\\nc-game\\campaign"
            "--dropfile"
            "{dropFilePath}"
            "--socket-port"
            "{srvPort}"
            "--encoding"
            "cp437"
            "--color-mode"
            "ansi16"
        ]
        io: socket
        encoding: cp437
    }
}
```

This is the correct Windows ENiGMA path. Do not try to treat ENiGMA like
native Windows Synchronet or Mystic. ENiGMA does not hand the child process a
usable `DOOR32` socket descriptor on Windows. It starts the door through
`abracadabra`, and `nc-door.exe` must connect back to `{srvPort}` instead.

If you are editing the menu from an existing ENiGMA setup, the practical rule
is blunt: `dropFileType: DOOR32`, `io: socket`, `--dropfile {dropFilePath}`,
and `--socket-port {srvPort}` all belong together.

## 5. Optional map-export staging

If you are still launching from a live source tree, the helper wrapper sets:

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
        cmd: /path/to/esterian_conquest/tools/bbs/run_nc_rust.sh
        args: [
            "/path/to/ec-campaign"
            "{dropFilePath}"
            "--socket-port"
            "{srvPort}"
        ]
        io: socket
        encoding: cp437
        env: {
            EC_CLIENT_QUEUE_DIR: /enigma-bbs/file_base/temp/ec
        }
    }
}
```

See [../sysop-map-exports.md](../sysop-map-exports.md) for the export/queue flow.

## 6. Local testing with your existing launcher

Your local `~/launch_bbs.sh` can stay as-is if it already:

1. starts ENiGMA
2. opens SyncTERM against `localhost:8888`
3. shuts ENiGMA down when you exit

Once the menu entry above points at `run_nc_rust.sh`, launching the NC door
from ENiGMA should run the Rust client instead of the DOS wrapper.

## 7. Minimal config swap from an older door

If you already have a door block pointing at the DOS wrapper, the minimal swap
is:

```hjson
dropFileType: DOOR32
cmd: /path/to/nc-door
args: [
    "--dir"
    "/path/to/ec-campaign"
    "--dropfile"
    "{dropFilePath}"
    "--socket-port"
    "{srvPort}"
    "--encoding"
    "cp437"
    "--color-mode"
    "ansi16"
]
io: socket
encoding: cp437
```

On native Windows, the same swap becomes:

```hjson
dropFileType: DOOR32
cmd: C:\\path\\to\\nc-door.exe
args: [
    "--dir"
    "C:\\path\\to\\ec-campaign"
    "--dropfile"
    "{dropFilePath}"
    "--socket-port"
    "{srvPort}"
    "--encoding"
    "cp437"
    "--color-mode"
    "ansi16"
]
io: socket
encoding: cp437
```

Replace the old DOS-only pieces:

- `dropFileType: DORINFO`
- `cmd: .../tools/bbs/run_ec_dos.sh`
- `args` containing `{node}` / `{srvPort}`
- old wrapper-specific `io` handling

## 8. Validate

The expected first-pass smoke test is straightforward:

1. connect to ENiGMA with SyncTERM or another telnet-capable BBS client
2. log in or create a user
3. open the Doors menu
4. launch the NC entry
5. confirm the game renders in the caller session with no extra Windows
   console window
6. verify normal movement, paging, and menu navigation
7. quit and confirm control returns cleanly to ENiGMA

The native Windows path above was smoke-tested on a normal `C:\enigma-bbs`
install with SyncTERM and the staged Windows release package.

## 9. Legacy DOS Compatibility Path

Use this only when you explicitly want to host the original DOS
`ECGAME.EXE`. It is a compatibility bridge, not the main ENiGMA deployment
story.

Keep the main lessons straight:

- ENiGMA-generated `DOOR.SYS` and `DORINFO` files are not reliable for the
  original DOS binary
- the most reliable legacy path is a strict 32-line WWIV-style `CHAIN.TXT`
- `ECGAME.EXE` should be launched with zero arguments from the mounted game
  directory
- DOSBox-X needs a headless-safe launch shape on modern Linux hosts

The current compatibility wrapper is `tools/bbs/run_ec_dos.sh`. It ignores the
native ENiGMA dropfile, generates a strict `CHAIN.TXT`, and launches DOSBox-X
against the preserved game directory.

Use the compatibility menu block only for that DOS path:

```hjson
doorEsterianConquest: {
    desc: Esterian Conquest
    module: abracadabra
    config: {
        name: Esterian Conquest
        dropFileType: DORINFO
        cmd: /path/to/esterian_conquest/tools/bbs/run_ec_dos.sh
        args: [
            "{dropFile}"
            "{node}"
            "{srvPort}"
        ]
        io: socket
    }
}
```

Performance expectations are different here. The DOS path still paints over
emulated serial I/O and will feel much slower than the Rust-native door. Use
it for compatibility or migration work, not because it is the preferred sysop
path.
