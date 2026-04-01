# Nostrian Conquest

_Nostrian Conquest – A from-scratch Rust recreation inspired by the classic 1990s BBS door game Esterian Conquest. Built on the Nostr protocol for decentralized play. All code, UI, and assets are original. Not affiliated with any original release. Created for fun and retro preservation._

**Status:** v1.0.0-beta.1 — Playable beta. Seeking playtesters and sysops.

## Premise

Beyond the mapped frontiers of the old Nostrian dominion lies a galaxy of contested solar systems. The old masters are gone. Their stations are silent. You are left with a fleet, a factory, and the knowledge to build an empire.

NC is a faithful reconstruction. We kept the campaign feel, the menus, the reports, and the old-school tension — now running on a modern Rust engine.

## Play

NC is powered by the Nostr protocol for decentralized multiplayer via native Windows, macOS, and Linux clients. We don't do web apps.

If you are looking for a live game, start at
[nostrian-conquest.com](https://nostrian-conquest.com). That landing page points
to the current public meeting places for game announcements and player
recruitment, including the Discord invite:
[discord.gg/FMr8sfBa](https://discord.gg/FMr8sfBa).

Nostr is the protocol that powers multiplayer in NC. It delivers a clean, secure, and decentralized experience — no traditional BBS middleware, no manual Unix accounts, and far less middleman friction than the old days.

Joining is straightforward:

- A sysop gives you an invite code. You join the campaign with a single command.
- The `nc-connect` tool creates and manages your encrypted Nostr identity, then opens a secure SSH-backed session.
- One hosted identity can claim only one seat in a given game. If you already joined that game, reconnect with the same wallet identity instead of redeeming another invite from it.
- During the current beta, the public GitHub player download lives on the repo's GitHub Releases page. Public `nc-connect` player archives are available for Windows x64, Linux x64, and macOS Apple Silicon, bundled with the player manual PDF. The packaged desktop client supports Windows, Linux, and macOS, and the Linux build supports both X11 and Wayland from the same package. `nc-connect-cli` remains a Cargo-only power-user binary and is not part of the normal player handoff.
- On your first connection, the client automatically downloads the campaign starmap and CSV sheets to your local machine. From then on, your assets stay on your own system.

This keeps the classic NC rhythm — connect, read reports, issue orders, log out — while cutting away most of the old friction. Just you, your empire, and the stars.

### Local and Hotseat
Play entirely in your terminal. Launch `nc-game` against a local campaign directory to learn the interface, test scenarios, or run a private campaign on one machine.

### BBS Hosting
We still support legacy BBS doors. The Rust client works natively with `DOOR32.SYS`, `DOOR.SYS`, and `CHAIN.TXT`. It is the perfect drop-in replacement for sysops running classic environments on modern hardware.

## Learn How To Play

The manuals cover everything from quick-start basics to deep strategy:

- **[NC Player Manual (PDF)](docs/manuals/nc_player_manual.pdf)**
- **[NC Sysop Manual (PDF)](docs/manuals/nc_sysop_manual.pdf)**

Historical `.DOC` files are preserved in [original/v1.5](original/v1.5).

## Beta Release Policy

Public Rust downloads are intentionally limited during beta. The current
policy is:

| Audience | Current Path |
|---|---|
| Normal player | Download the public Windows x64, Linux x64, or macOS Apple Silicon `nc-connect` archive from GitHub Releases |
| Rust self-host sysop | Build from tagged source with Cargo |
| Rust VPS sysop | Build from tagged source with Cargo and use `scripts/install_vps.sh` |
| BBS sysop | Build from source, or use a direct/private beta build |

Public GitHub Releases now include Windows x64, Linux x64, and macOS Apple
Silicon `nc-connect` player archives alongside the DOS compatibility bundles.
See [Release Policy](docs/release-policy.md).

## Background

Nostrian Conquest is a from-scratch Rust recreation inspired by a 1990s BBS door game with yearly turns and printed starmaps. It is not a wrapper, but a ground-up reconstruction of the original rules and feel for modern decentralized play.

If you want to know how the recovery work was done, see [How NC was recovered](docs/dev/approach.md#how-ec-was-recovered).

The engine is explicit. Seeded RNG ensures reproducible results, and the logs explain exactly why a combat resolved the way it did.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

## Quick Start

### 1. Self-Host One Game
```bash
cd rust
cargo run -q -p nc-sysop -- new-game /srv/ec/games/friday-night --name "Friday Night NC" --players 4
```

Each hosted game directory contains one runtime file:

```text
/srv/ec/games/friday-night/
  ncgame.db
```

Run the client directly for local play or trusted SSH use:
```bash
cd rust
cargo run -q -p nc-game -- --dir /srv/ec/games/friday-night --player 1
```

Advance the game when needed:
```bash
cd rust
cargo run -q -p nc-sysop -- maint /srv/ec/games/friday-night 1
```

### 2. Host Many Games On One VPS
Bootstrap the standard host layout:
```bash
sudo ./scripts/install_vps.sh \
  --relay wss://relay.example.com \
  --ssh-host play.example.com
```

That installs:

```text
/usr/local/bin/nc-game
/usr/local/bin/nc-sysop
/usr/local/bin/nc-gate-keys
/etc/nc-gate/config.kdl
/etc/nc-gate/identity.kdl
/var/lib/nc-gate/keys/
/srv/ec/games/<slug>/ncgame.db
```

The host relay and game-server address live in `/etc/nc-gate/config.kdl`.
`install_vps.sh` writes them from `--relay`, `--ssh-host`, and `--ssh-port`.
If you change them later, edit that file and restart `nc-nostr.service`.
If you self-host the relay on the same VPS, the relay host also needs a
public HTTPS websocket front end. A common setup is `nostr-rs-relay` bound
to `127.0.0.1:8080` with Caddy or another reverse proxy serving
`relay.example.com` on `443`.

Create and register games:
```bash
sudo -u ecgame /usr/local/bin/nc-sysop new-game /srv/ec/games/friday-night --name "Friday Night NC" --players 4
sudo /usr/local/bin/nc-sysop host games add --config /etc/nc-gate/config.kdl --dir /srv/ec/games/friday-night
sudo systemctl restart nc-nostr.service
```

Create hosted games as the `ecgame` service user so `nc-nostr.service` can
write session leases into `ncgame.db`.

Run the daemon:
```bash
cd rust
cargo run -q -p nc-sysop -- nostr init
cargo run -q -p nc-sysop -- nostr serve
```

Schedule the fleet-wide sweep with `systemd` or `cron`:
```bash
cargo run -q -p nc-sysop -- maint-all --config /etc/nc-gate/config.kdl
```

Players join with `nc-connect`:
```bash
nc-connect --join amber-river@relay.example.com
```

If a hosted invite was reissued or a player reports that the relay cannot find
an invite that should be pending, verify and republish that game's public
metadata:
```bash
sudo /usr/local/bin/nc-sysop nostr verify --dir /srv/ec/games/friday-night
sudo /usr/local/bin/nc-sysop nostr publish --dir /srv/ec/games/friday-night
```

### 3. Run `nc-game` As A BBS Door
Create the game and reserve caller aliases:
```bash
cargo run -q -p nc-sysop -- new-game /srv/ec/games/night-shift --name "Night Shift NC" --players 4
cargo run -q -p nc-sysop -- settings reserve --dir /srv/ec/games/night-shift --player 1 --alias SYSOP
```

During the current beta, a BBS sysop should build from source or use a
direct/private test build. Then point the door entry at `nc-game` with a
dropfile. For working setups, see:

- [Mystic Rust Door Setup](docs/sysop/mystic-rust-setup.md)
- [ENiGMA½ Rust Door Setup](docs/sysop/enigma-rust-setup.md)

## Operator Docs

- [NC Sysop Manual (PDF)](docs/manuals/nc_sysop_manual.pdf)
- [Sysop Documentation Index](docs/sysop/README.md)
- [NC Player Manual (PDF)](docs/manuals/nc_player_manual.pdf)

## Developer Commands

Inspect a game directory:
```bash
cd rust
cargo run -q -p nc-cli -- core-report /tmp/nc-game
```

Inspect player mail:
```bash
cd rust
cargo run -q -p nc-cli -- inspect-messages /tmp/nc-game
```

## Local Dependencies

- Rust toolchain
- Python 3
- `sccache` (recommended)

For compatibility work (DOSBox-X, Ghidra), see the contributor docs.

## For Contributors

Read these before editing code:
- [docs/approach.md](docs/dev/approach.md)
- [docs/rust-architecture.md](docs/dev/rust-architecture.md)

## Repository Layout

- `original/`: Original binaries and manuals.
- `docs/`: Engineering and design documentation.
- `rust/`: The core engine, sysop tools, and player clients.
- `tools/`: Oracle runners and analysis scripts.

## License

Source code and tooling are licensed under the **O'Saasy License Agreement**. See [LICENSE](LICENSE).

Preserved original `Esterian Conquest` materials are included for preservation and research only.
