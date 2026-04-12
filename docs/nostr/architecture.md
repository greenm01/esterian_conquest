# Architecture

This document describes the system design for Nostr-authenticated
multiplayer hosting of Esterian Conquest.

## Design Principles

The game binary `nc-game` does not change. It runs in a PTY on the
server exactly as it does today. The Nostr layer sits entirely outside the
game, handling identity and session establishment. SSH handles the
terminal transport. This separation means the three existing hosting paths
(localhost, BBS door, Nostr) share one game binary with no conditional
logic.

## Components

```
nc-connect (player terminal)          Nostr relay      nc-sysop nostr / nc-gate (VPS)
────────────────────────────          ───────────      ───────────────────────────────
                                                           nc-game (PTY)
                                                           sshd
```

There are three components. Two are new (`nc-connect` and the Nostr
hosting subsystem surfaced as `nc-sysop nostr`, internally backed by
`nc-gate`) and one is unchanged (`nc-game`).

**nc-connect** runs on the player's machine. It is a hybrid CLI and
ratatui TUI application. When run with no arguments, it shows a game
picker screen listing the player's joined games with options to connect,
join new games, or manage identity. When run with a server bookmark or
hostname, it connects directly without the picker. In both modes, it
manages the player's Nostr identity (keypair keychain), handles the
authentication handshake over a Nostr relay, and bridges the player's
terminal to an SSH session on the server. The player never interacts with
SSH directly. It works cross-platform on Linux, macOS, and Windows.

**nc-sysop nostr** is the public sysop command surface for the VPS daemon.
Internally, the current implementation lives in the `nc-gate` crate. It
listens on one or more Nostr relays for session requests, validates player
identity and invite codes, manages hosted seat claims, and provisions
ephemeral SSH keys that can only run `nc-game` for the correct player
seat. It also handles invite code generation and lifecycle management.

**nc-game** is the existing game TUI. It runs inside a PTY spawned by
sshd when a player connects with a provisioned ephemeral key. It receives
its player index and game directory as command-line arguments. It does not
know or care that Nostr was involved in getting the player there.

## Connection Flow

### First-Time Player (Invite Redemption)

```
Player                          Relay                 nc-gate              sshd / nc-game
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
  │                               │                      │        9. PTY + nc-game
  │  10. Raw terminal ◄────────────────────────────── PTY ──────── --player N  │
  │      (game session)           │                      │                      │
  │                               │                      │                      │
  │  11. Disconnect               │                      │     12. PTY cleaned  │
  │      Key invalidated          │                      │         up           │
```

### Returning Player (Reconnection)

The flow is identical except step 2 omits the invite code and may include
a `game-id` tag from the local cache. `nc-gate` recognizes the player's
npub from the hosted-seat data and provisions a session for their existing seat. No
invite code is needed after the first join.

### Failed Session

If the invite code is invalid, the seat is already taken, or the npub is
not recognized, `nc-gate` publishes a 30503 SessionError event (NIP-44
encrypted to the player). `nc-connect` displays the error and returns to
the picker (or exits in direct mode).

## Why SSH for Transport

Nostr is a message-passing protocol over WebSocket relays. It is excellent
for asynchronous communication but poorly suited as a terminal transport.
An interactive TUI like `nc-game` requires a bidirectional byte stream
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

On first launch, `nc-connect` generates a secp256k1 keypair and encrypts
it with a password chosen by the player. The keypair is stored in a local
keychain file. The player never sees the words "Nostr," "npub," or "nsec"
unless they go looking. They have a password-protected game identity and
that is all they need to know.

### Imported Identity (Nostr User Path)

A player who already has a Nostr keypair can import their nsec (bech32 or
hex) into the keychain. They can also use a NIP-46 remote signer if they
prefer not to paste their private key. This lets them use the same
identity across EC servers, other Nostr applications, and any future
relay-transport client.

### Keychain Storage

Both paths store the identity in the same keychain format at
`~/.local/share/nc/keychain.kdl`. The keychain is encrypted with
ChaCha20-Poly1305 using a key derived from the player's password via
PBKDF2-HMAC-SHA256. This follows the same keychain model used by ec4x.

## Invite Codes and Seat Assignment

Games are invitation-only. The admin creates a game and receives one
invite code per player seat. Codes are shared privately (Discord,
Telegram, email, in person) and redeemed by players when they first
connect.

### Code Format

Invite codes use two words from the Monero mnemonic wordlist, hyphenated
and lowercase, suffixed with the relay host[:port]:

```
velvet-mountain@relay.example.com
velvet-mountain@relay.example.com:7447
```

The wordlist contains 1626 words, giving approximately 2.6 million
combinations. This is sufficient entropy for private invite codes that
are not brute-forced.

### Code Lifecycle

```
PENDING ──── invite used, session starts ────► PENDING
   │                                              │
   │ admin deletes slot                           │ in-game empire save claims seat
   ▼                                              ▼
DELETED                                       CLAIMED
                                                  │
                                                  │ admin reissues
                                                  ▼
                                               PENDING (new code)
```

Invite codes are bearer tokens. Whoever uses one can start the first join
flow, but the seat is not officially claimed until the in-game empire-naming
save binds that seat to the player's `npub`. If the player leaves before that,
the code remains pending and reusable. Once claim completes, the code is
permanently bound to the player's `npub`. If a player loses their identity,
the admin reissues a new code for that seat.

### Admin Workflow

```
$ nc-sysop new-game /srv/ec/friday-night --players 4 --seed 1515 --nostr
Game created: /srv/ec/friday-night
Invite codes:
  Seat 1: velvet-mountain@relay.example.com
  Seat 2: copper-sunrise@relay.example.com
  Seat 3: amber-cascade@relay.example.com
  Seat 4: silver-meadow@relay.example.com
```

The admin shares these codes with their friends. Each player redeems
their code on first connect.

## Multi-Game Support

A single `nc-gate` instance can serve multiple games. Each game directory
has its own hosted-seat rows in `ncgame.db` with its own invite codes and a unique game ID
slug derived from the directory name.

### One Identity, Multiple Games

A player's npub can be in multiple games on the same server. This is the
common case for a group of friends running several concurrent campaigns.
Each game has its own hosted-seat binding for the npub.

### Game Selection

When a returning player connects:

- If `nc-connect` has a cached game ID for this server (from a previous
  session), it includes a `game-id` tag in the SessionRequest. `nc-gate`
  uses this to route directly to the correct game.
- If the player is in only one game on the server, no disambiguation is
  needed. `nc-gate` finds the single hosted-game match and provisions the
  session.
- If the player is in multiple games and no `game-id` was provided,
  `nc-gate` returns a `multiple_games` error with a game list.
  `nc-connect` presents the list to the player (in the picker or as a
  numbered prompt in direct mode), and retries with the selected game ID.

The local game cache at `~/.local/share/nc/cache.kdl` stores joined
games so that disambiguation only happens on cache miss (new machine,
cleared cache, etc.).

### First Join

On first join, the invite code uniquely identifies the game (codes are
unique across all games on the server). No game ID is needed. The first
session is provisional until the player finishes the in-game empire-naming
save that claims the hosted seat for that `npub`. Only after that confirmed
claim does `nc-connect` cache the game ID, empire name, gate key, and relay
URL so future picker reconnects do not need to rediscover them. If the player
backs out before claim, the invite stays reusable and no durable picker row is
written. After a completed first join, `nc-connect` also performs a second,
short Nostr recovery fetch only if needed. The primary path is now proactive:
the hosted seat claim writes a durable publish job, `nc-gate` publishes 30512
`MapPush`, and `nc-connect` saves the player-safe starmap bundle during the
same live session so it is already present for turn 1. Manual picker map
downloads still use the explicit 30504/30505 pull path.

## Player-Side File Layout

`nc-connect` stores downloaded maps under the user's Documents folder and
keeps its other local files in the platform-appropriate config/data
locations:

| File | Path | Purpose |
|------|------|---------|
| Config | `~/.config/nc/config.kdl` | Server bookmarks, default relay |
| Keychain | `~/.local/share/nc/keychain.kdl` | Encrypted identity store |
| Cache | `~/.local/share/nc/cache.kdl` | Joined games and connection history |
| Maps | `~/Documents/nc/maps/` | Downloaded static starmap bundles |

Config is user-edited (server bookmarks, relay preference). Keychain and
cache are managed by `nc-connect` and should not be hand-edited. They are
local client state, not authoritative hosted game state.

## Coexistence with Other Auth Paths

The three auth paths are independent and do not interfere with each
other. A single VPS could run the `nc-sysop nostr serve` daemon for
Nostr-authenticated games alongside an Enigma BBS serving the same or
different game directories via dropfiles. `nc-game` does not care which
path spawned its PTY.

The only constraint is that a given game directory should be served by
one auth path at a time to avoid concurrent access conflicts. The admin
is responsible for not pointing both the Nostr daemon and a BBS door at
the same game directory simultaneously with overlapping player sessions.

## Future: Relay-Mediated Transport

The next evolution described in `grand-vision.md` replaces SSH with
relay-mediated turn submission and state sync. In that model:

- `nc-connect` evolves into a local TUI client that renders game state
  natively rather than bridging a remote PTY
- Turn orders are submitted as encrypted Nostr events (like ec4x's 30402)
- Game state and turn results flow back as encrypted per-player events
- `nc-gate` evolves into (or is replaced by) a headless game daemon that
  runs `ec-maint` on a schedule and publishes results to relays
- The keychain, invite code system, hosted seat data, and local game cache
  carry forward directly from this spec

This spec is designed so that the identity infrastructure does not need
to be rebuilt when the transport changes.

## Crate Placement

`nc-connect` is a separate public binary. The public VPS/operator surface
lives under `nc-sysop nostr`, backed internally by the `nc-gate` crate,
with no dependency on `nc-game` internals. Proposed workspace layout:

```
rust/
├── nc-connect/     # player-side client (new crate)
├── nc-gate/        # internal server-side Nostr daemon crate
├── nc-game/        # unchanged
├── nc-sysop/       # gains invite code generation commands
└── ...
```

`nc-connect` and `nc-gate` share a common dependency on `nostr-sdk` for
Nostr protocol handling. They do not depend on `nc-data`, `nc-engine`,
or any other game crate. The public integration point is `nc-sysop`,
which owns the `nostr` command surface and related hosted-seat management.
