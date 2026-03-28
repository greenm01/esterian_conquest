# esterian_conquest

![Esterian Conquest banner](docs/assets/ec_v1.5_banner.png)

_Esterian Conquest (c) 1992 Bentley C. Griffith. A fan-built resurrection._

**Status:** v1.0.0-beta.1 — Playable beta. Seeking playtesters and sysops.

## Premise

Beyond the mapped frontiers of the old Esterian dominion lies a galaxy of contested solar systems. The old masters are gone. Their stations are silent. You are left with a fleet, a factory, and the knowledge to build an empire.

EC is a faithful reconstruction. We kept the campaign feel, the menus, the reports, and the old-school tension — now running on a modern Rust engine.

## Screenshots

![Splash screen](docs/assets/screenshots/ec1.png)  
![Main menu](docs/assets/screenshots/ec2.png)  
![Interactive starmap](docs/assets/screenshots/ec3.png)  
![Planet database](docs/assets/screenshots/ec4.png)

## Play

Esterian Conquest is best played today over **Nostr**.

Nostr is the protocol that powers multiplayer in EC. It delivers a clean, secure, and decentralized experience — no traditional BBS middleware, no manual Unix accounts, and far less middleman friction than the old days.

Joining is straightforward:

- A sysop gives you an invite code. You join the campaign with a single command.
- The `ec-connect` tool creates and manages your encrypted Nostr identity, then opens a secure SSH-backed session.
- On your first connection, the client automatically downloads the campaign starmap and CSV sheets to your local machine. From then on, your assets stay on your own system.

This keeps the classic EC rhythm — connect, read reports, issue orders, log out — while cutting away most of the old friction. Just you, your empire, and the stars.

### Local and Hotseat
Play entirely in your terminal. Launch `ec-game` against a local campaign directory to learn the interface, test scenarios, or run a private campaign on one machine.

### BBS Hosting
We still support legacy BBS doors. The Rust client works natively with `DOOR32.SYS`, `DOOR.SYS`, and `CHAIN.TXT`. It is the perfect drop-in replacement for sysops running classic environments on modern hardware.

## Learn How To Play

The manuals cover everything from quick-start basics to deep strategy:

- **[EC Player Manual (PDF)](docs/manuals/ec_player_manual.pdf)**
- **[EC Sysop Manual (PDF)](docs/manuals/ec_sysop_manual.pdf)**

Historical `.DOC` files are preserved in [original/v1.5](original/v1.5).

## Background

Esterian Conquest was a 1992 BBS door game with yearly turns and printed starmaps. This project is a full reimplementation in Rust — not a wrapper, but a ground-up reconstruction of the original rules and feel.

If you want to know how the recovery work was done, see [How EC was recovered](docs/dev/approach.md#how-ec-was-recovered).

The engine is explicit. Seeded RNG ensures reproducible results, and the logs explain exactly why a combat resolved the way it did.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

## Quick Start

### For Sysops: Create a Campaign
\`\`\`bash
cd rust
cargo run -q -p ec-sysop -- new-game /tmp/ec-game --players 4
\`\`\`

### For Sysops: Host via Nostr
Initialize and run the Nostr-facing daemon:
\`\`\`bash
cd rust
cargo run -q -p ec-sysop -- nostr init
cargo run -q -p ec-sysop -- nostr serve
\`\`\`

Generate player invite codes:
\`\`\`bash
cargo run -q -p ec-sysop -- nostr seats --dir /path/to/mygame
\`\`\`

### For Players: Join a Game
1. Run `ec-connect`.
2. Press `N`.
3. Paste the invite code and press Enter.

Power users can join directly:
\`\`\`bash
ec-connect --join ecinv1...
\`\`\`

### Run Maintenance
Advance the game year:
\`\`\`bash
cargo run -q -p ec-sysop -- maint /tmp/ec-game 3
\`\`\`
Schedule this via `cron` or `systemd`. EC does not manage its own scheduler.

## Useful Commands

**Inspect a game directory:**
\`\`\`bash
cargo run -q -p ec-cli -- core-report /tmp/ec-game
\`\`\`

**Export a player starmap:**
\`\`\`bash
cargo run -q -p ec-cli -- map-export /tmp/ec-game 1 /tmp/ECMAP-P1.TXT
\`\`\`

**Inspect player mail:**
\`\`\`bash
cargo run -q -p ec-cli -- inspect-messages /tmp/ec-game
\`\`\`

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

Esterian Conquest (c) Bentley C. Griffith. These materials are included for preservation and research only.