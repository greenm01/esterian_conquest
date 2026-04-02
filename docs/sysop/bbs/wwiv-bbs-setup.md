# WWIV BBS Setup

WWIV is a validated Linux BBS host for `nc-door`.

Status note:

- Linux WWIV with `CHAIN.TXT` is validated through SyncTERM
- the tested path stages `nc-door` and `nc-sysop` in a normal sysop-owned
  tree and launches `nc-door` as a WWIV chain
- Windows WWIV is still pending, so this guide currently documents the
  validated Linux path only

## 1. Stage the Rust binaries

For Linux BBS hosting, use the public `nc-sysop` package or build from source.
Localhost play remains a source-build `nc-game` path.

From the repo root:

```bash
cd rust
cargo build -q --release -p nc-game -p nc-sysop
```

For a normal sysop-owned layout, keep the WWIV root and the staged NC files in
one stable tree. Example:

```text
/path/to/wwiv/
  bbs
  wwivd
  wwivconfig
  wwivutil
  data/
  doors/
    nc-game/
      bin/
        nc-door
        nc-sysop
      campaign/
```

Use the staged `nc-door` binary as the live chain target. Do not point WWIV at
a source-tree helper script as the permanent sysop path.

## 2. Create the campaign

Example `config.kdl`:

```kdl
players 4
reservations {
    seat player=1 alias="SYSOP"
}
```

Initialize the BBS campaign:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/wwiv/doors/nc-game/campaign
```

If you want a reproducible test map, keep the seed on the creation command
line:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /path/to/wwiv/doors/nc-game/campaign --seed 1515
```

Run yearly maintenance with your normal host tooling:

```bash
cd rust
cargo run -q -p nc-sysop -- maint /path/to/wwiv/doors/nc-game/campaign 1
```

## 3. Initialize and configure WWIV

Initialize the BBS root once:

```bash
./wwivconfig --initialize --bbsdir /path/to/wwiv
```

Then update `data/wwivd.json` for a normal non-root local test host:

- bind telnet to a non-privileged port such as `2324`
- set `ssh_port` to `-1` unless you are intentionally running WWIV with a real
  SSH listener
- keep the single-node `bbses` entry aligned to node `1`
- if this is only a local validation host, keep the listener on localhost

The important point is simple: WWIV must be able to start as your normal user
without trying to bind privileged ports such as `22`.

## 4. Add the NC chain

WWIV writes `CHAIN.TXT`, and `%C` expands to the full path to that dropfile.
Point the chain at the staged `nc-door` binary and pass the campaign dir.

Example `data/chains.json` entry:

```json
{
  "version": 1,
  "chains": [
    {
      "filename": "/path/to/wwiv/doors/nc-game/bin/nc-door --dir /path/to/wwiv/doors/nc-game/campaign --dropfile %C --encoding cp437 --color-mode ansi16",
      "description": "Nostrian Conquest",
      "exec_mode": "STDIO",
      "dir": "bbs",
      "ansi": true,
      "local_only": false,
      "multi_user": false,
      "acs": "user.sl >= 10",
      "regby": [],
      "usage": 0,
      "local_console_cp437": true,
      "pause": false
    }
  ]
}
```

Practical notes:

- `--dropfile %C` is the important WWIV handoff
- keep `--encoding cp437 --color-mode ansi16` so the caller view stays aligned
  with classic ANSI BBS behavior
- the default WWIV `newusersl` of `10` already matches the example chain ACS

## 5. Start WWIV

From the WWIV root:

```bash
./wwivd --bbsdir /path/to/wwiv
```

For a local validation pass, connect with SyncTERM to the configured telnet port,
such as `127.0.0.1:2324`.

## 6. Validate

The expected first-pass validation pass is:

1. connect with SyncTERM or another telnet-capable BBS client
2. create or log into a WWIV user
3. open the Doors section
4. launch the Nostrian Conquest chain
5. confirm the game renders in the caller session with ANSI color
6. verify input, paging, and quit-back-to-BBS behavior

## 7. Troubleshooting

- if `wwivd` fails at startup with a socket-bind error on port `22`, disable
  SSH in `data/wwivd.json` or move it to a non-privileged port
- if the chain returns immediately, check the `filename` path in
  `data/chains.json` first and confirm the staged `nc-door` binary is
  executable
- if the caller reaches WWIV but not the door, confirm the user SL satisfies
  the chain ACS and that the chain description matches the menu entry you are
  launching
- if you are using the public `nc-sysop` package, the host-specific WWIV,
  Mystic, Synchronet, and ENiGMA½ guides are bundled under `docs/sysop/bbs/`
