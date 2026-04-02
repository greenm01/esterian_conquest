# Nostrian Conquest Windows Sysop Package

This package is for Windows x64 localhost and BBS hosting.

It includes:

- `nc-game.exe`
- `nc-sysop.exe`
- `nc_player_manual.pdf`
- `nc_sysop_manual.pdf`
- `config.kdl`
- `BUILD-INFO.txt`

It does not include the VPS/Nostr daemon stack. For VPS hosting, build from
tagged source on Linux and use `scripts/install_vps.sh`.

## Localhost Quick Start

Create a fresh local game:

```text
nc-sysop.exe new-game C:\nc\games\friday-night --name "Friday Night NC" --players 4
```

Launch a seat directly:

```text
nc-game.exe --dir C:\nc\games\friday-night --player 1
```

Run yearly maintenance:

```text
nc-sysop.exe maint C:\nc\games\friday-night 1
```

## BBS Quick Start

Create a game directory and copy `config.kdl` into it. Adjust `players` and
any `reservations`, then initialize the BBS campaign:

```text
nc-sysop.exe new-game --bbs C:\nc\games\night-shift
```

For a one-off reproducible test map, keep the seed on the command line:

```text
nc-sysop.exe new-game --bbs C:\nc\games\night-shift --seed 1515
```

Point the BBS door directly at `nc-game.exe` with the dropfile path:

```text
nc-game.exe --dir C:\nc\games\night-shift --dropfile <dropfile> --encoding cp437 --color-mode ansi16
```

## Notes

- Door-mode control contract: `HJKL` movement, `Ctrl-U` / `Ctrl-D` paging,
  `Q` or `Esc` for back/quit.
- New unreserved BBS callers land on the BBS first-time menu.
- Reserved callers and returning callers skip the generic BBS first-time menu.
- The signed `SHA256SUMS.txt` manifest on the GitHub release page covers this
  package.
