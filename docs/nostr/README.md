# Nostr-Authenticated Multiplayer

This directory specifies how Esterian Conquest uses the Nostr protocol for
player identity and session authentication, enabling hosted multiplayer
games where players connect directly from their terminal without needing
Unix accounts, BBS middleware, or manual SSH key management.

## Motivation

Esterian Conquest already supports two hosting models: localhost
single-player and BBS door integration via dropfiles. This spec adds a
third model designed for small-group multiplayer on a VPS, where an admin
organizes a game among friends and players connect with a single command.

Nostr solves the identity problem cleanly. Players authenticate with a
secp256k1 keypair, the same cryptographic identity used across the Nostr
ecosystem. For players who already use Nostr, they bring their existing
keys. For everyone else, `ec-connect` generates a keypair automatically
and encrypts it with a password. From the player's perspective the
experience is the same either way: run one command, enter a password or
have a signer handle it, and land directly in the game.

Using Nostr rather than a bespoke auth system has two additional benefits.
It may draw interest from the Nostr community, and it keeps the identity
layer portable across servers and compatible with the broader Nostr
tooling ecosystem.

## Relationship to the Grand Vision

[grand-vision.md](../grand-vision.md) describes Phase 2, Nostrian Conquest,
as a full decoupling of client and server where turn orders and game state
flow entirely through Nostr relays. This spec is an intermediate step.
Nostr handles identity and session establishment only. The actual game
session runs over SSH, because SSH is the proven, low-latency, feature-
complete protocol for interactive terminal sessions. The game binary
`ec-game` does not change at all.

This intermediate layer becomes the foundation that Nostrian Conquest
builds on. The identity model, invite code system, wallet format, local
game cache, and player roster carry forward directly. What changes later
is the transport: SSH gives way to relay-mediated turn submission and
state sync, and the TUI evolves from a remote PTY session to a local
client with a richer interface.

## Components

| Component | Role | Location |
|-----------|------|----------|
| `ec-connect` | Player-side client. Ratatui game picker, identity management, Nostr auth handshake, and SSH terminal bridging. Cross-platform (Linux, macOS, Windows). | Player's machine |
| `ec-sysop nostr` | Public sysop/operator surface for Nostr hosting. Internally backed by the `ec-gate` crate. Validates identity, manages invites and seats, provisions SSH sessions, and handles multi-game routing. | VPS |
| `ec-game` | The game TUI. Unchanged. Runs in a PTY on the server. | VPS |

## Auth Model Overview

Three hosting paths, one game binary:

| Path | Identity | Transport | Use case |
|------|----------|-----------|----------|
| Localhost | None needed | Direct PTY | Single-player, development |
| BBS door | Dropfile (caller alias) | Telnet/SSH via BBS | Traditional BBS hosting |
| Nostr | secp256k1 keypair | SSH (via ec-connect) | VPS multiplayer |

The Nostr path is the focus of this spec. Players join a game by redeeming
an invite code, which binds their Nostr public key to a player seat. On
subsequent connections, the server recognizes their key and routes them to
the correct game and seat automatically. Players can be in multiple games
on the same server; `ec-connect` caches joined games locally and includes
a game ID in reconnection requests for disambiguation.

## Player-Side Files

`ec-connect` follows XDG conventions for file placement:

| File | Path | Purpose |
|------|------|---------|
| Config | `~/.config/ec/config.kdl` | Server bookmarks, default Nostr relay |
| Wallet | `~/.local/share/ec/wallet.kdl` | Encrypted identity store |
| Cache | `~/.local/share/ec/cache.kdl` | Joined games and connection history |
| Maps | `~/.local/share/ec/maps/` | Downloaded static starmap bundles |

On Windows and macOS, platform-appropriate equivalents are used via the
`dirs` crate. Players can override the maps root with `maps-dir` in the
config file or `--maps-dir` on the command line.

## Reading Order

1. [architecture.md](architecture.md) -- system design, connection flow,
   and transport rationale
2. [ec-connect.md](ec-connect.md) -- player-side client: identity
   management, game picker, invite redemption, session lifecycle
3. [ec-gate.md](ec-gate.md) -- server-side daemon backend and public
   `ec-sysop nostr` workflow: invite codes, seat management, multi-game
   routing, SSH provisioning, deployment
4. [protocol.md](protocol.md) -- Nostr event kinds, session handshake,
   encryption, and security model
