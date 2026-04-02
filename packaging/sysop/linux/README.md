# Nostrian Conquest Linux Sysop Package

This package is for Linux x64 localhost and BBS hosting.

It includes:

- `bin/nc-game`
- `bin/nc-sysop`
- `docs/nc_player_manual.pdf`
- `docs/nc_sysop_manual.pdf`
- `examples/config.kdl`
- `tools/bbs/run_nc_rust.sh`
- `BUILD-INFO.txt`

It does not include the VPS/Nostr daemon stack. For VPS hosting, build from
tagged source and use `scripts/install_vps.sh`.

## Localhost Quick Start

Create a fresh local game:

```bash
./bin/nc-sysop new-game /srv/nc/games/friday-night --name "Friday Night NC" --players 4
```

Launch a seat directly:

```bash
./bin/nc-game --dir /srv/nc/games/friday-night --player 1
```

Run yearly maintenance:

```bash
./bin/nc-sysop maint /srv/nc/games/friday-night 1
```

## BBS Quick Start

Create a game directory and write `config.kdl` first. Start from
`examples/config.kdl` and adjust the seat count and any fixed reservations.

Initialize the BBS campaign:

```bash
./bin/nc-sysop new-game --bbs /srv/nc/games/night-shift
```

For a one-off reproducible test map, keep the seed on the command line:

```bash
./bin/nc-sysop new-game --bbs /srv/nc/games/night-shift --seed 1515
```

Point your BBS door at:

```text
./tools/bbs/run_nc_rust.sh /srv/nc/games/night-shift <dropfile>
```

The wrapper passes:

- `--dir`
- `--dropfile`
- `--encoding cp437`
- `--color-mode ansi16`

For validated launcher examples, see:

- `docs/mystic-rust-setup.md`
- `docs/enigma-rust-setup.md`

## Notes

- Door-mode control contract: `HJKL` movement, `Ctrl-U` / `Ctrl-D` paging,
  `Q` or `Esc` for back/quit.
- New unreserved BBS callers land on the BBS first-time menu.
- Reserved callers and returning callers skip the generic BBS first-time menu.
- The signed `SHA256SUMS.txt` manifest on the GitHub release page covers this
  package.
