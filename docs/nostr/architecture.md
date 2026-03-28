# Architecture

This document describes the system design for Nostr-authenticated
multiplayer hosting of Esterian Conquest.

## Design Principles

The game binary `ec-game` does not change. It runs in a PTY on the
server exactly as it does today. The Nostr layer sits entirely outside the
game, handling identity and session establishment. SSH handles the
terminal transport. This separation means the three existing hosting paths
(localhost, BBS door, Nostr) share one game binary with no conditional
logic.

## Components

```
ec-connect (player terminal)          Nostr relay      ec-sysop nostr / ec-gate (VPS)
────────────────────────────          ───────────      ───────────────────────────────
                                                           ec-game (PTY)
                                                           sshd
```

There are three components. Two are new (`ec-connect` and the Nostr
hosting subsystem surfaced as `ec-sysop nostr`, internally backed by
`ec-gate`) and one is unchanged (`ec-game`).

**ec-connect** runs on the player's machine. It is a hybrid CLI and
ratatui TUI application. When run with no arguments, it shows a game
picker screen listing the player's joined games with options to connect,
join new games, or manage identity. When run with a server bookmark or
hostname, it connects directly without the picker. In both modes, it
manages the player's Nostr identity (keypair wallet), handles the
authentication handshake over a Nostr relay, and bridges the player's
terminal to an SSH session on the server. The player never interacts with
SSH directly. It works cross-platform on Linux, macOS, and Windows.

**ec-sysop nostr** is the public sysop command surface for the VPS daemon.
Internally, the current implementation lives in the `ec-gate` crate. It
listens on one or more Nostr relays for session requests, validates player
identity and invite codes, manages hosted seat claims, and provisions
ephemeral SSH keys that can only run `ec-game` for the correct player
seat. It also handles invite code generation and lifecycle management.

**ec-game** is the existing game TUI. It runs inside a PTY spawned by
sshd when a player connects with a provisioned ephemeral key. It receives
its player index and game directory as command-line arguments. It does not
know or care that Nostr was involved in getting the player there.

## Connection Flow

### First-Time Player (Invite Redemption)

```
Player                          Relay                 ec-gate              sshd / ec-game
  │                               │                      │                      │
  │  1. Generate ephemeral        │                      │                      │
  │     SSH keypair               │                      │                      │
  │                               │                      │                      │
  │  2. Publish 30501             │                      │                      │
  │     SessionRequest            │                      │                      │
  │     (signed, includes         │                      │                      │
  │      invite code +            │   ──────────────►    │                      │
  │      ephemeral SSH pubkey)    │                      │                      │
  │                               │                      │  3. Validate invite  │
  │                               │                      │     code             │
  │                               │                      │  4. Bind npub to     │
  │                               │                      │     seat             │
  │                               │                      │  5. Provision        │
  │                               │                      │     ephemeral key    │
  │                               │                      │     with command=    │
  │                               │                      │     restriction      │
  │                               │                      │                      │
  │  6. Receive 30502             │   ◄──────────────    │                      │
  │     SessionReady              │                      │                      │
  │     (NIP-44 encrypted,        │                      │                      │
  │      game_id + SSH details)   │                      │                      │
  │                               │                      │                      │
  │  7. Cache game locally        │                      │                      │
  │                               │                      │                      │
  │  8. SSH connect with ─────────────────────────────────────────────────►     │
  │     ephemeral key             │                      │                      │
  │                               │                      │        9. PTY + ec-game
  │  10. Raw terminal ◄────────────────────────────── PTY ──────── --player N  │
  │      (game session)           │                      │                      │
  │                               │                      │                      │
  │  11. Disconnect               │                      │     12. PTY cleaned  │
  │      Key invalidated          │                      │         up           │
```

### Returning Player (Reconnection)

The flow is identical except step 2 omits the invite code and may include
a `game-id` tag from the local cache. `ec-gate` recognizes the player's
npub from the hosted-seat data and provisions a session for their existing seat. No
invite code is needed after the first join.

### Failed Session

If the invite code is invalid, the seat is already taken, or the npub is
not recognized, `ec-gate` publishes a 30503 SessionError event (NIP-44
encrypted to the player). `ec-connect` displays the error and returns to
the picker (or exits in direct mode).

## Why SSH for Transport

Nostr is a message-passing protocol over WebSocket relays. It is excellent
for asynchronous communication but poorly suited as a terminal transport.
An interactive TUI like `ec-game` requires a bidirectional byte stream
with low latency, proper PTY negotiation, window resize handling, and raw
mode support. SSH provides all of this out of the box.

Streaming terminal I/O through Nostr relays would add 50-200ms of
latency per hop, create a dependency on relay uptime for the duration of
the session, and require building a terminal protocol from scratch on
top of Nostr events. For a turn-based game this latency might be
tolerable, but the complexity and fragility are not justified when SSH
exists and works perfectly for this purpose.

The design decision is: use Nostr where it excels (identity, key
management, relay-mediated handshake) and SSH where it excels (terminal
sessions). The relay is only involved during the brief authentication
handshake, not during gameplay.

## Dual Identity Onramp

The identity model is designed so that players who have never heard of
Nostr have a seamless experience, while players who are active Nostr
users can bring their existing identity.

### Auto-Generated Identity (Normie Path)

On first launch, `ec-connect` generates a secp256k1 keypair and encrypts
it with a password chosen by the player. The keypair is stored in a local
wallet file. The player never sees the words "Nostr," "npub," or "nsec"
unless they go looking. They have a password-protected game identity and
that is all they need to know.

### Imported Identity (Nostr User Path)

A player who already has a Nostr keypair can import their nsec (bech32 or
hex) into the wallet. They can also use a NIP-46 remote signer if they
prefer not to paste their private key. This lets them use the same
identity across EC servers, other Nostr applications, and any future
relay-transport client.

### Wallet Storage

Both paths store the identity in the same wallet format at
`~/.local/share/ec/wallet.kdl`. The wallet is encrypted with
ChaCha20-Poly1305 using a key derived from the player's password via
PBKDF2-HMAC-SHA256. This follows the same wallet model used by ec4x.

## Invite Codes and Seat Assignment

Games are invitation-only. The admin creates a game and receives one
invite code per player seat. Codes are shared privately (Discord,
Telegram, email, in person) and redeemed by players when they first
connect.

### Code Format

Invite codes use two words from the Monero mnemonic wordlist, hyphenated
and lowercase, optionally suffixed with a relay URL:

```
velvet-mountain
velvet-mountain@play.example.com
velvet-mountain@play.example.com:2222
```

The wordlist contains 1626 words, giving approximately 2.6 million
combinations. This is sufficient entropy for private invite codes that
are not brute-forced.

### Code Lifecycle

```
PENDING ──── player claims with npub ────► CLAIMED
   │                                          │
   │ admin deletes slot                       │ admin reissues
   ▼                                          ▼
DELETED                                    PENDING (new code)
```

Invite codes are bearer tokens. Whoever presents one first claims the
seat. Once claimed, the code is permanently bound to the player's npub.
If a player loses their identity, the admin reissues a new code for that
seat.

### Admin Workflow

```
$ ec-sysop new-game /srv/ec/friday-night --players 4 --seed 1515 --nostr
Game created: /srv/ec/friday-night
Invite codes:
  Seat 1: velvet-mountain@play.example.com
  Seat 2: copper-sunrise@play.example.com
  Seat 3: amber-cascade@play.example.com
  Seat 4: silver-meadow@play.example.com
```

The admin shares these codes with their friends. Each player redeems
their code on first connect.

## Multi-Game Support

A single `ec-gate` instance can serve multiple games. Each game directory
has its own hosted-seat rows in `ecgame.db` with its own invite codes and a unique game ID
slug derived from the directory name.

### One Identity, Multiple Games

A player's npub can be in multiple games on the same server. This is the
common case for a group of friends running several concurrent campaigns.
Each game has its own hosted-seat binding for the npub.

### Game Selection

When a returning player connects:

- If `ec-connect` has a cached game ID for this server (from a previous
  session), it includes a `game-id` tag in the SessionRequest. `ec-gate`
  uses this to route directly to the correct game.
- If the player is in only one game on the server, no disambiguation is
  needed. `ec-gate` finds the single hosted-game match and provisions the
  session.
- If the player is in multiple games and no `game-id` was provided,
  `ec-gate` returns a `multiple_games` error with a game list.
  `ec-connect` presents the list to the player (in the picker or as a
  numbered prompt in direct mode), and retries with the selected game ID.

The local game cache at `~/.local/share/ec/cache.kdl` stores joined
games so that disambiguation only happens on cache miss (new machine,
cleared cache, etc.).

### First Join

On first join, the invite code uniquely identifies the game (codes are
unique across all games on the server). No game ID is needed. After the
join succeeds, `ec-connect` caches the game ID, empire name, gate key,
and relay URL from the successful session so future picker reconnects do
not need to rediscover them. It also performs a second, short Nostr
request to fetch the game's static player-safe starmap bundle. That map
download is best-effort and happens only on first invite-code join, not
on every reconnect.

## Player-Side File Layout

`ec-connect` stores its files according to XDG conventions (with
platform-appropriate equivalents on Windows and macOS):

| File | Path | Purpose |
|------|------|---------|
| Config | `~/.config/ec/config.kdl` | Server bookmarks, default relay |
| Wallet | `~/.local/share/ec/wallet.kdl` | Encrypted identity store |
| Cache | `~/.local/share/ec/cache.kdl` | Joined games and connection history |
| Maps | `~/.local/share/ec/maps/` | Downloaded static starmap bundles |

Config is user-edited (server bookmarks, relay preference). Wallet and
cache are managed by `ec-connect` and should not be hand-edited. They are
local client state, not authoritative hosted game state.

## Coexistence with Other Auth Paths

The three auth paths are independent and do not interfere with each
other. A single VPS could run the `ec-sysop nostr serve` daemon for
Nostr-authenticated games alongside an Enigma BBS serving the same or
different game directories via dropfiles. `ec-game` does not care which
path spawned its PTY.

The only constraint is that a given game directory should be served by
one auth path at a time to avoid concurrent access conflicts. The admin
is responsible for not pointing both the Nostr daemon and a BBS door at
the same game directory simultaneously with overlapping player sessions.

## Future: Relay-Mediated Transport

The next evolution described in `grand-vision.md` replaces SSH with
relay-mediated turn submission and state sync. In that model:

- `ec-connect` evolves into a local TUI client that renders game state
  natively rather than bridging a remote PTY
- Turn orders are submitted as encrypted Nostr events (like ec4x's 30402)
- Game state and turn results flow back as encrypted per-player events
- `ec-gate` evolves into (or is replaced by) a headless game daemon that
  runs `ec-maint` on a schedule and publishes results to relays
- The wallet, invite code system, hosted seat data, and local game cache
  carry forward directly from this spec

This spec is designed so that the identity infrastructure does not need
to be rebuilt when the transport changes.

## Crate Placement

`ec-connect` is a separate public binary. The public VPS/operator surface
lives under `ec-sysop nostr`, backed internally by the `ec-gate` crate,
with no dependency on `ec-game` internals. Proposed workspace layout:

```
rust/
├── ec-connect/     # player-side client (new crate)
├── ec-gate/        # internal server-side Nostr daemon crate
├── ec-game/        # unchanged
├── ec-sysop/       # gains invite code generation commands
└── ...
```

`ec-connect` and `ec-gate` share a common dependency on `nostr-sdk` for
Nostr protocol handling. They do not depend on `ec-data`, `ec-engine`,
or any other game crate. The public integration point is `ec-sysop`,
which owns the `nostr` command surface and related hosted-seat management.
