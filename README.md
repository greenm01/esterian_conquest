# esterian_conquest

![Esterian Conquest banner](docs/assets/ec_v1.5_banner.png)

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
A fan-built resurrection -- not affiliated with the original._

**Status:** v1.0.0-beta.1 — beta-quality and playable, seeking player and sysop playtesters

## Premise

Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy
of contested solar systems. The old masters are gone. Their stations are
silent, their patrols vanished, and their subjects left with fleets,
factories, and enough knowledge to build empires.

EC takes that premise seriously. The goal is not to sand away the identity of
the original game, but to keep its campaign feel, menus, reports, and
old-school tension — now running on a modern Rust engine.

## Screenshots

![Splash screen](docs/assets/screenshots/ec1.png)
![Main menu](docs/assets/screenshots/ec2.png)
![Interactive starmap](docs/assets/screenshots/ec3.png)
![Planet database](docs/assets/screenshots/ec4.png)

## Play

If you are starting Esterian Conquest today, the recommended way to play is
through a hosted game over Nostr with `ec-connect`. A sysop runs
`ec-sysop nostr serve`, gives you an invite code and the host's Nostr public
key, and you join from your own machine with one command. `ec-connect` manages
the encrypted player identity, opens the SSH-backed `ec-game` session, and on
the first successful join downloads the campaign's static player-safe starmap
bundle so you have the printable text map and CSV sheets locally.

That hosted flow is now the primary multiplayer story for the Rust edition.
It preserves the old EC rhythm — connect, read reports, issue orders, log
out, wait for maintenance — without requiring BBS middleware or manual Unix
account setup. `ec-connect` now uses a fixed `80x25` local shell with a game
table, wallet manager, and lock screen, while still supporting direct
invite-code joins from the command line.

Local and hotseat play remain first-class. If you want to learn the
interface, test a scenario, or run a private campaign on one machine, you can
still launch `ec-game` directly against a local campaign directory and play
entirely in your own terminal.

BBS door hosting also remains supported, but it is now a legacy-compatibility
path rather than the recommended modern way to host EC. The Rust door client
still works for sysops who want the classic environment, and the original
dropfile world remains part of the game's history, but new public-hosted games
should start with `ec-sysop nostr` and `ec-connect`.

The Rust client stack is cross-platform. `ec-connect` and `ec-game` run on
Linux, macOS, and Windows, and Linux and macOS playtest bundles are supported
as standalone archives. `ec-cli` remains developer/oracle tooling and is not
part of the shipped player/sysop bundle.

In local terminal sessions, players can switch between the campaign's
available ANSI themes from the client itself. `ec-game` ships with `classic`,
a larger bundle of alternate palettes, and a `Mono` option, and each empire's
last local theme choice is remembered in the campaign database. In BBS door
mode, the client instead uses the classic ANSI color on/off toggle and starts
from the campaign default theme for that session.

EC does not show you everything on one screen. The game exports your starmap
as a printable text file and a companion CSV. Many players print the map or
pull the CSV into a spreadsheet to track fleet positions, mark explored
systems, and plan routes by hand. The Rust client has an interactive map
viewer in the TUI, but pencil on a printed starmap is the old-school way and
still a good one.

## Learn How To Play

The player manual covers everything from quick-start basics through economy,
combat, fleet missions, and strategy:

- **[EC Player Manual (PDF)](docs/manuals/ec_player_manual.pdf)**

Sysops setting up and administering a campaign — whether through the
recommended `ec-sysop nostr` hosted flow, on localhost, or through a legacy
BBS door — should also read:

- **[EC Sysop Manual (PDF)](docs/manuals/ec_sysop_manual.pdf)**

The original `.DOC` files are preserved in [original/v1.5](original/v1.5) for
historical reference.

## Background

Esterian Conquest was a BBS door game released in 1992. It had a yearly turn
structure, empire reports, a starmap you could actually print, and an
asymmetrical mix of scouting, production, and fleet combat that rewarded
patient players. It ran on DOS and died with the BBS era. This project brings
it back.

The Rust engine is not a wrapper around the original binary. It is a full
reimplementation: the rules, the turn cycle, the maintenance pass, the
reports, the player client. The original game's feel and structure are the
design target; the original binaries and manuals are the reference. Where the
original was opaque or stochastic, the Rust version is explicit — a seeded
campaign RNG means results are reproducible, and the engine will tell you why
a combat resolved the way it did.

The game is playable today and has reached a real beta stage. Fresh campaigns
run across all four documented player tiers (4, 9, 16, and 25 empires),
yearly maintenance processes real turns, and the Rust player, connection, and
sysop tools cover the core campaign workflow. The main work now is
playtesting, collecting feedback, and fixing the rough edges and bugs that
only show up in real games.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

## Quick Start

Create a new campaign:

```bash
cd rust
cargo run -q -p ec-sysop -- new-game /tmp/ec-game --players 4 --seed 1515
```

If you are hosting the recommended public multiplayer flow, initialize and run
the Nostr-facing daemon:

```bash
cd rust
cargo run -q -p ec-sysop -- nostr init
cargo run -q -p ec-sysop -- nostr serve
```

Share an invite code and the daemon's public key with each player. The
simplest player join path today is:

```bash
cd rust
cargo run -q -p ec-connect -- --join amber-river@play.example.com --gate npub1...
```

Add `--relay wss://relay.example.com` when you want to override relay
discovery, and `--maps-dir /path/to/maps` when you want starmap bundles stored
somewhere other than the platform default data directory. On the first
successful join, `ec-connect` downloads the game's static player-safe starmap
bundle before opening the terminal session.

Run maintenance to close a year:

```bash
cd rust
cargo run -q -p ec-sysop -- maint /tmp/ec-game 3
```

Schedule `ec-sysop maint` with your host tools — `systemd` timers, `cron`,
BBS event hooks, or manual invocation. EC does not manage its own scheduler.

If you want sysop-side diagnostics, `ec-sysop` also accepts opt-in file
logging flags before the subcommand:

```bash
cd rust
cargo run -q -p ec-sysop -- --log-file /tmp/ec-sysop.log --log-level info maint /tmp/ec-game 3
```

If you are playing locally or running a hotseat/private test campaign, launch
the player client directly:

```bash
cd rust
cargo run -q -p ec-game -- --dir /tmp/ec-game --player 1
```

To capture client diagnostics without polluting the terminal session, add
`--log-file` and optionally `--log-level`:

```bash
cd rust
cargo run -q -p ec-game -- --dir /tmp/ec-game --player 1 --log-file /tmp/ec-game-p1.log --log-level debug
```

For legacy BBS door deployments, pass the drop file directly. If the caller
alias is reserved in `config.kdl`, `ec-game` can infer the player seat
automatically:

```bash
ec-game --dir /path/to/campaign --dropfile /path/to/DOOR32.SYS
```

`ec-game` auto-detects `DOOR32.SYS`, `DOOR.SYS`, and `CHAIN.TXT` — no
wrapper scripts or format massaging required. The `--timeout <minutes>` flag
overrides the session timeout from the command line if needed. Unreserved
callers can still use `--player` explicitly.

The current Rust door client is verified on both Mystic and ENiGMA. In BBS
door mode, the stable control contract is `HJKL` for movement, `Ctrl-U` /
`Ctrl-D` for paging, and `Q` or `Esc` for back/quit. Do not rely on arrows or
`PgUp` / `PgDn` as primary controls through BBS hosts.

Submit a player turn from a KDL file:

Players normally use the interactive TUI, but `ec-game` also supports a
file-based turn submission path for localhost, shared-host, and custom-client
workflows. Validate first with `--check`, then apply the same file directly to
the campaign runtime state:

```bash
cd rust
cargo run -q -p ec-game -- submit-turn --check --dir /tmp/ec-game --player 1 --file /tmp/player1-turn.kdl
cargo run -q -p ec-game -- submit-turn --dir /tmp/ec-game --player 1 --file /tmp/player1-turn.kdl
```

The turn file format is documented in [docs/sysop/turn-kdl.md](docs/sysop/turn-kdl.md).

## Useful Commands

Inspect a game directory:

```bash
cd rust
cargo run -q -p ec-cli -- core-report /tmp/ec-game
```

Export a player-safe printable starmap and companion CSV:

```bash
cd rust
cargo run -q -p ec-cli -- map-export /tmp/ec-game 1 /tmp/ec-exports/ECMAP-P1-Y3000.TXT
```

Inspect player mail:

```bash
cd rust
cargo run -q -p ec-cli -- inspect-messages /tmp/ec-game
```

`ec-cli` also hosts the internal developer and compatibility workflows — oracle
sweeps, harness scenarios, combat sweeps, and classic `.DAT` bridge commands.
Those are documented in the contributor section below.

## Local Dependencies

For normal development the baseline is:

- Rust toolchain with `cargo`
- Python 3 for support scripts under `scripts/`

Recommended build-speed tooling:

- `sccache`

Enable it in your local Cargo config:

```toml
[build]
rustc-wrapper = "sccache"
```

For compatibility and provenance work (oracle testing against the original DOS
binaries, static analysis, DOSBox-X probes) you also need:

- DOSBox-X — the only currently verified local runner for the original EC v1.5 binaries
- `python-pexpect` for the DOSBox-X debugger helpers under `tools/`
- Ghidra plus JDK 21 for the headless static-analysis workflow

On Arch-based systems:

```bash
sudo pacman -S sccache python-pexpect dosbox-x ghidra jdk21-openjdk
```

If you use the packaged Arch/CachyOS Ghidra build:

```bash
export GHIDRA_HOME=/opt/ghidra
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
```

## For Contributors

Start here before editing Rust code:

- [docs/approach.md](docs/dev/approach.md)
- [docs/next-session.md](docs/dev/next-session.md)
- [docs/rust-architecture.md](docs/dev/rust-architecture.md)

Supporting docs:

- [docs/ec-setup-spec.md](docs/dev/ec-setup-spec.md)
- [docs/economics.md](docs/dev/economics.md)
- [docs/ec-combat-spec.md](docs/dev/ec-combat-spec.md)
- [docs/ec-movement-spec.md](docs/dev/ec-movement-spec.md)
- [docs/bbs_door_client_rust.md](docs/dev/bbs_door_client_rust.md)
- [docs/sysop-map-exports.md](docs/sysop/sysop-map-exports.md)
- [docs/dosbox-workflow.md](docs/dev/dosbox-workflow.md)
- [docs/reverse_engineering/README.md](docs/reverse_engineering/README.md) — how the original binary was recovered

The `ec-cli` harness commands cover scenario and combat sweep workflows:

```bash
cd rust
cargo run -q -p ec-cli -- harness run-scenario --file /tmp/scenario.kdl --dir /tmp/ec-scenario
cargo run -q -p ec-cli -- harness run-sweep --file /tmp/combat-sweep.kdl
cargo run -q -p ec-cli -- harness play-until --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --turn 5
```

Harness format and coordinator workflow: [docs/dev/harness/README.md](docs/dev/harness/README.md),
[docs/dev/harness/campaign-play.md](docs/dev/harness/campaign-play.md).

Classic DOS oracle scripts live under [scripts/](scripts/). The fastest
busy-campaign probe:

```bash
./scripts/run_classic_report_probe.sh
```

## Repository Layout

- `original/`: original EC 1.5 materials — binaries, manuals, data files; historical reference
- `EC_UNLOCKED/`: curated runnable plain-MZ copies of the original DOS executables
- `docs/`: engineering, design, and milestone docs
- `docs/reverse_engineering/`: oracle, provenance, and binary-recovery docs
- `docs/dev/archive/RE_NOTES.md`: chronological reverse-engineering notebook
- `rust/ec-data`: shared runtime/store/state-model crate
- `rust/ec-classic`: low-level classic record/codec support crate
- `rust/ec-engine`: public gameplay/maintenance/rules crate
- `rust/ec-compat`: classic `.DAT` import/export bridge
- `rust/ec-cli`: internal developer/oracle/compatibility CLI
- `rust/ec-sysop`: public sysop/admin CLI
- `rust/ec-game`: Rust player client, ships as the `ec-game` binary
- `tools/`: oracle runners, DOSBox helpers, and analysis scripts

## License

The new source code and tooling in this repository are licensed under the
O'Saasy License Agreement. See [LICENSE](LICENSE) or
[osaasy.dev](https://osaasy.dev/).

The original Esterian Conquest DOS binaries, data files, manuals, logs, and
other preserved game materials remain original works of Bentley C. Griffith and
their original rights holders. Their inclusion here is for preservation,
research, and compatibility work; they are not relicensed under the O'Saasy
License Agreement by this repository.
