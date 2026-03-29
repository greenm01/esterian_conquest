# Nostr Protocol

This document specifies the Nostr event kinds and message flows used by
`ec-connect` and `ec-gate` for public seat claiming, session authentication,
post-session seat metadata refresh, and static player starmap delivery.

## Design Principles

The protocol is designed to be as small as possible. Nostr is used only
for the authentication handshake, not for ongoing game communication.
Once a session is established, all game traffic flows over SSH and the
relay connection is closed.

The event kind range is 305xx, adjacent to but distinct from ec4x's
304xx range, so that both games can coexist on the same relay without
kind collisions.

## Event Kinds

| Kind | Name | Publisher | Encryption | Purpose |
|------|------|-----------|------------|---------|
| 30500 | GameDefinition | ec-gate | None | Public game metadata and seat status |
| 30501 | SessionRequest | ec-connect | None | Player requests a session launch |
| 30502 | SessionReady | ec-gate | NIP-44 | Session provisioned, SSH details enclosed |
| 30503 | SessionError | ec-gate | NIP-44 | Session request failed |
| 30504 | MapRequest | ec-connect | None | Player requests the static map bundle for a joined game |
| 30505 | MapBundle | ec-gate | NIP-44 | Compressed static map bundle |
| 30506 | MapError | ec-gate | NIP-44 | Map request failed |
| 30507 | SessionStateRequest | ec-connect | None | Player requests refreshed seat metadata after a hosted session |
| 30508 | SessionStateReady | ec-gate | NIP-44 | Current game name, seat, and empire name |
| 30509 | SessionStateError | ec-gate | NIP-44 | Session-state refresh failed |
| 30510 | SeatClaimRequest | ec-connect | None | Deprecated pre-claim flow from older clients |
| 30511 | SeatClaimError | ec-gate | NIP-44 | Deprecated claim flow rejected or invite claim failed |

All request/response kinds are parameterized replaceable events (NIP-33). The `d` tag
serves as the deduplication key.

## Required and Optional Tags by Kind

| Kind | Required Tags | Optional Tags |
|------|---------------|---------------|
| 30500 | `d` (game-id), `name`, `status`, `ssh-host`, `ssh-port` | |
| 30501 | `d` (session-nonce), `p` (gate npub), `ssh-pubkey`, `game-id` | |
| 30502 | `d` (session-nonce), `p` (player npub) | |
| 30503 | `d` (session-nonce), `p` (player npub) | |
| 30504 | `d` (map-request-nonce), `p` (gate npub), `game-id` | |
| 30505 | `d` (map-request-nonce), `p` (player npub) | |
| 30506 | `d` (map-request-nonce), `p` (player npub) | |
| 30507 | `d` (state-request-nonce), `p` (gate npub), `game-id` | |
| 30508 | `d` (state-request-nonce), `p` (player npub) | |
| 30509 | `d` (state-request-nonce), `p` (player npub) | |
| 30510 | `d` (claim-nonce), `p` (gate npub) | `game-id` |
| 30511 | `d` (claim-nonce), `p` (player npub) | |

## Event Specifications

### 30500: GameDefinition

Published by `ec-gate` for each game it serves. This is a public,
unencrypted event that lets players and clients discover games on a relay.

```json
{
  "kind": 30500,
  "pubkey": "<gate-npub>",
  "created_at": 1711468800,
  "tags": [
    ["d", "<game-id>"],
    ["name", "Friday Night EC"],
    ["status", "active"],
    ["ssh-host", "play.example.com"],
    ["ssh-port", "2222"],
    ["players", "4"],
    ["slot", "1", "<invite-code-hash>", "<player-npub>", "claimed"],
    ["slot", "2", "<invite-code-hash>", "", "pending"],
    ["slot", "3", "<invite-code-hash>", "<player-npub>", "claimed"],
    ["slot", "4", "<invite-code-hash>", "", "pending"]
  ],
  "content": "",
  "sig": "..."
}
```

**Tag definitions:**

- `d`: Game identifier slug, derived from the game directory name (e.g.,
  `friday-night`). Unique per `ec-gate` instance.
- `name`: Human-readable game name
- `status`: `setup` (waiting for players), `active` (game in progress),
  or `finished`
- `ssh-host`: SSH hostname players use for the hosted session
- `ssh-port`: SSH port players use for the hosted session
- `players`: Total number of seats
- `slot`: `[index, invite-code-hash, player-pubkey-or-empty, status]`

Invite codes are normalized (lowercase, trimmed) before hashing with
SHA-256. The hash is published in the event so that `ec-connect` can
verify it holds a valid code for a pending slot without revealing the
code to relay observers. `ec-connect` also uses `ssh-host` and `ssh-port`
to match the right public game definition before first join.

**Publishing:** `ec-gate` publishes an updated 30500 whenever a seat
status changes (player joins, admin reissues). The parameterized
replaceable semantics (NIP-33) ensure the relay retains only the latest
version.

**Optional:** Game definition publishing can be disabled if the admin
wants fully private games. Without it, players must know the server
hostname, relay URL, and gate npub out of band.

### 30510: SeatClaimRequest (Deprecated)

Older clients published this for a normal public first join after discovery.
Current clients no longer use it.

```json
{
  "kind": 30510,
  "pubkey": "<player-npub>",
  "created_at": 1711468898,
  "tags": [
    ["d", "<claim-nonce>"],
    ["p", "<gate-npub>"],
    ["game-id", "<game-id-slug>"]
  ],
  "content": "<invite-code>",
  "sig": "..."
}
```

Current servers reject this deprecated flow. Hosted seats are now claimed only
after the player saves the in-game empire name during the first session.

### 30511: SeatClaimError

Published by `ec-gate` when a deprecated 30510 invite claim is received or
cannot be fulfilled.

The encrypted payload shape matches other simple error responses:

```json
{"error":"invalid_code","message":"The invite code is not valid."}
```

### 30501: SessionRequest

Published by `ec-connect` when a player wants to start a session.

```json
{
  "kind": 30501,
  "pubkey": "<player-npub>",
  "created_at": 1711468900,
  "tags": [
    ["d", "<session-nonce>"],
    ["p", "<gate-npub>"],
    ["ssh-pubkey", "<ephemeral-ed25519-pubkey>"],
    ["game-id", "<game-id-slug>"]
  ],
  "content": "<invite-code or empty string>",
  "sig": "..."
}
```

**Tag definitions:**

- `d`: A unique session nonce (random hex string, 32 bytes). Used to
  correlate the request with the response and prevent replay.
- `p`: The `ec-gate` daemon's npub. Tells the relay to route this event
  to the gate's subscription.
- `ssh-pubkey`: The ephemeral ed25519 SSH public key generated by
  `ec-connect` for this session. Encoded in OpenSSH `ssh-ed25519` format.
- `game-id`: The game identifier slug from discovery or local cache.
  Public first joins resolve this directly from 30500 before the first session.
  Reconnects use the cached game ID when available. If it is absent and
  the player is in multiple games on the same server, `ec-gate` returns
  `multiple_games`.

**Content field:** on first join, the invite code; on returning sessions, the
empty string.

**Signing:** The event is signed with the player's Nostr private key.
This is the authentication step: the signature proves the player controls
the npub.

The first public session still carries the invite because the hosted seat is
not claimed until the in-game join is actually completed.

### 30502: SessionReady

Published by `ec-gate` when a session has been provisioned.

```json
{
  "kind": 30502,
  "pubkey": "<gate-npub>",
  "created_at": 1711468905,
  "tags": [
    ["d", "<session-nonce>"],
    ["p", "<player-npub>"]
  ],
  "content": "<NIP-44-encrypted-payload>",
  "sig": "..."
}
```

**Tag definitions:**

- `d`: Echoes the session nonce from the request. `ec-connect` uses this
  to match the response to its pending request.
- `p`: The player's npub. Tells the relay to route this event to the
  player's subscription.

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "game_id": "friday-night",
  "ssh_host": "play.example.com",
  "ssh_port": 22,
  "host_fingerprint": "SHA256:...",
  "game_name": "Friday Night EC",
  "seat": 2,
  "player_name": "Empire of Sol"
}
```

Fields:

| Field | Description |
|-------|-------------|
| `game_id` | Game identifier slug. Used by `ec-connect` to populate the local game cache and disambiguate future connections. |
| `ssh_host` | Hostname or IP to SSH into |
| `ssh_port` | SSH port number |
| `host_fingerprint` | SSH server host key fingerprint for verification |
| `game_name` | Human-readable game name |
| `seat` | Player seat number (1-based) |
| `player_name` | Empire name for this seat |

### 30503: SessionError

Published by `ec-gate` when a session request cannot be fulfilled.

```json
{
  "kind": 30503,
  "pubkey": "<gate-npub>",
  "created_at": 1711468905,
  "tags": [
    ["d", "<session-nonce>"],
    ["p", "<player-npub>"]
  ],
  "content": "<NIP-44-encrypted-payload>",
  "sig": "..."
}
```

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "error": "invalid_code",
  "message": "The invite code 'velvet-mountain' is not valid."
}
```

Error codes:

| Code | Description |
|------|-------------|
| `invalid_code` | The invite code does not match any pending seat |
| `code_claimed` | The invite code has already been claimed by another player |
| `unknown_player` | The player's npub is not in any game roster (and no invite code was provided) |
| `multiple_games` | The player's npub is in multiple games and no `game-id` tag was provided (see below) |
| `game_full` | All seats are claimed |
| `game_not_active` | The game is not currently accepting connections |
| `rate_limited` | Too many session requests from this npub |

### 30504: MapRequest

Published by `ec-connect` after a successful first-time invite-code join,
or later when the player manually re-downloads maps from the picker.

```json
{
  "kind": 30504,
  "pubkey": "<player-npub>",
  "created_at": 1711468910,
  "tags": [
    ["d", "<map-request-nonce>"],
    ["p", "<gate-npub>"],
    ["game-id", "friday-night"]
  ],
  "content": "",
  "sig": "..."
}
```

**Tag definitions:**

- `d`: A unique request nonce, used to match the response.
- `p`: The gate daemon npub.
- `game-id`: The joined game's slug.

The event is signed by the player's Nostr identity. Authorization is by
signed player npub plus `game-id`; invite codes are not part of the map
request flow.

### 30505: MapBundle

Published by `ec-gate` when the player is authorized to receive the
static starmap bundle for a joined game.

```json
{
  "kind": 30505,
  "pubkey": "<gate-npub>",
  "created_at": 1711468911,
  "tags": [
    ["d", "<map-request-nonce>"],
    ["p", "<player-npub>"]
  ],
  "content": "<NIP-44-encrypted-payload>",
  "sig": "..."
}
```

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "game_id": "friday-night",
  "game_name": "Friday Night EC",
  "seat": 2,
  "files": [
    {
      "name": "starmap.txt",
      "codec": "zstd+base64",
      "sha256": "<sha256-of-original-bytes>",
      "content": "<base64-zstd-bytes>"
    },
    {
      "name": "starmap.csv",
      "codec": "zstd+base64",
      "sha256": "<sha256-of-original-bytes>",
      "content": "<base64-zstd-bytes>"
    },
    {
      "name": "starmap-DETAILS.csv",
      "codec": "zstd+base64",
      "sha256": "<sha256-of-original-bytes>",
      "content": "<base64-zstd-bytes>"
    }
  ]
}
```

The bundle always contains the same three player-safe files. Each file is
compressed individually with `zstd` and then base64-encoded for JSON
transport. If the final encoded payload would exceed 64 KiB, `ec-gate`
does not chunk it in this protocol version; it returns a 30506
`payload_too_large` error instead.

### 30506: MapError

Published by `ec-gate` when a map request cannot be fulfilled.

```json
{
  "kind": 30506,
  "pubkey": "<gate-npub>",
  "created_at": 1711468911,
  "tags": [
    ["d", "<map-request-nonce>"],
    ["p", "<player-npub>"]
  ],
  "content": "<NIP-44-encrypted-payload>",
  "sig": "..."
}
```

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "error": "map_unavailable",
  "message": "Unable to build map bundle right now."
}
```

Map error codes:

| Code | Description |
|------|-------------|
| `game_not_found` | The requested `game-id` is not served by this gate |
| `unknown_player` | The requesting npub is not bound to a seat in that game |
| `map_unavailable` | The game exists, but the map bundle could not be built |
| `payload_too_large` | The encoded map bundle exceeded the protocol size limit |

### 30507: SessionStateRequest

Published by `ec-connect` after a hosted SSH session ends successfully, so it
can refresh the local cache with the current empire name and seat metadata.

The event is signed by the player's identity and carries the same `game-id`
authorization shape as `MapRequest`.

### 30508: SessionStateReady

Published by `ec-gate` when the requesting player is enrolled in the requested
game and the current seat metadata can be loaded.

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "game_id": "friday-night",
  "game_name": "Friday Night EC",
  "seat": 2,
  "player_name": "Empire of Sol"
}
```

### 30509: SessionStateError

Published by `ec-gate` when a session-state refresh cannot be fulfilled.

**Encrypted payload** (NIP-44 encrypted to the player's npub):

```json
{
  "error": "unknown_player",
  "message": "Your identity is not enrolled in that game."
}
```

Session-state error codes:

| Code | Description |
|------|-------------|
| `game_not_found` | The requested `game-id` is not served by this gate |
| `unknown_player` | The requesting npub is not bound to a seat in that game |
| `internal_error` | The server could not load current game metadata |

### multiple_games Error Payload

When a returning player's npub matches seats in more than one game on the
server and the SessionRequest did not include a `game-id` tag, `ec-gate`
returns a `multiple_games` error with a game list so that `ec-connect`
can present a selection:

```json
{
  "error": "multiple_games",
  "message": "Your identity is in multiple games on this server.",
  "games": [
    {"game_id": "friday-night", "name": "Friday Night EC", "seat": 2},
    {"game_id": "saturday-showdown", "name": "Saturday Showdown", "seat": 5}
  ]
}
```

`ec-connect` displays the game list to the player, lets them select one,
and retries the SessionRequest with the chosen `game-id` tag. Both games
are added to the local cache so future connections can include the
`game-id` directly.

## Message Flows

### First-Time Join (Invite Redemption)

```
ec-connect                       Relay                     ec-gate
    │                              │                          │
    │  30501 SessionRequest        │                          │
    │  (game-id + ssh-pubkey +     │                          │
    │   invite code in content)    │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Validate invite     │
    │                              │      Provision SSH key   │
    │                              │                          │
    │  30502 SessionReady          │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  SSH connect ────────────────────────────────────────►  │
    │                              │                     ec-game
    │  PTY session ◄───────────────────────────────────────── │
    │                              │                          │
    │  Player saves empire name ───────────────────────────►  │
    │                              │                          │
    │  30507 SessionStateRequest   │                          │
    │  (after session exit)        │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │  30508 SessionStateReady     │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
```

### Static Map Download

After a completed first join, `ec-connect` performs a second short Nostr
round-trip so the player receives the campaign's static starmap bundle once.
Players can later trigger the same flow manually from the picker.

```
ec-connect                       Relay                     ec-gate
    │                              │                          │
    │  30504 MapRequest            │                          │
    │  (game-id tag)               │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Authorize by npub   │
    │                              │      + game-id           │
    │                              │      Build map bundle    │
    │                              │      Compress files      │
    │                              │                          │
    │  30505 MapBundle             │                          │
    │  (encrypted zstd+base64      │                          │
    │   file bundle)               │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  Write local map files       │                          │
```

### Returning Player (Single Game)

```
ec-connect                       Relay                     ec-gate
    │                              │                          │
    │  30501 SessionRequest        │                          │
    │  (no invite code,            │                          │
    │   game-id from cache)        │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Look up npub +      │
    │                              │      game-id in roster   │
    │                              │      Provision SSH key   │
    │                              │                          │
    │  30502 SessionReady          │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  SSH connect ────────────────────────────────────────►  │
    │  PTY session ◄───────────────────────────────────────── │
```

### Returning Player (Multiple Games, No Cached ID)

```
ec-connect                       Relay                     ec-gate
    │                              │                          │
    │  30501 SessionRequest        │                          │
    │  (no invite code,            │                          │
    │   no game-id)                │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Find multiple       │
    │                              │      roster matches      │
    │                              │                          │
    │  30503 SessionError          │                          │
    │  (multiple_games +           │                          │
    │   game list)                 │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  Player selects game         │                          │
    │                              │                          │
    │  30501 SessionRequest        │                          │
    │  (game-id from selection)    │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Look up npub +      │
    │                              │      game-id             │
    │                              │      Provision SSH key   │
    │                              │                          │
    │  30502 SessionReady          │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  SSH connect ────────────────────────────────────────►  │
    │  PTY session ◄───────────────────────────────────────── │
```

### Failed Join

```
ec-connect                       Relay                     ec-gate
    │                              │                          │
    │  30501 SessionRequest        │                          │
    │  (bad invite code)           │                          │
    ├─────────────────────────────►│─────────────────────────►│
    │                              │                          │
    │                              │      Validate fails      │
    │                              │                          │
    │  30503 SessionError          │                          │
    │  (encrypted error message)   │                          │
    │◄─────────────────────────────│◄─────────────────────────┤
    │                              │                          │
    │  Display error, exit.        │                          │
```

## Security Model

### Authentication

Authentication is implicit in the Nostr event signature. When `ec-gate`
receives a 30501 session request, it verifies the event signature against
the sender's pubkey. A valid signature proves the sender controls the
corresponding private key. No passwords, tokens, or challenge-response
exchanges are needed beyond what Nostr already provides.

### Replay Prevention

Each session request includes a unique session nonce in the `d` tag and
a `created_at` timestamp. `ec-gate` rejects requests with:

- A `created_at` older than 60 seconds (prevents replaying captured
  events)
- A `d` nonce that has been seen before (prevents immediate replays)

The nonce also binds the response to the request: `ec-connect` only
accepts a 30502/30503 or 30505/30506 whose `d` tag matches the nonce it
generated.

### Invite Code Security

Invite codes are bearer tokens. They should be treated like passwords and
shared privately. Mitigations:

- Codes are hashed in public 30500 GameDefinition events, so relay
  observers cannot read unclaimed codes.
- Each code can only be claimed once. After the in-game join is completed,
  it is permanently bound to the claiming npub.
- `ec-gate` should rate-limit invalid first-join session requests per
  source npub to prevent brute-force code guessing. With approximately 2.6 million
  possible two-word codes, a rate limit of 10 attempts per minute makes
  brute force impractical.

### Ephemeral SSH Key Scope

The SSH key provisioned by `ec-gate` is restricted in multiple ways:

- `command=` forces it to run only `ec-game` with specific arguments
- `no-port-forwarding`, `no-X11-forwarding`, `no-agent-forwarding`
  prevent tunneling
- The key has a 60-second TTL and is removed after use or expiration
- The key is ephemeral: generated per session, never stored on disk by
  `ec-connect`

Even if an attacker captured the ephemeral public key from a relay event,
they could not use it because they do not have the corresponding private
key (which only exists in `ec-connect`'s memory).

### NIP-44 Encryption

SessionReady (30502), SessionError (30503), MapBundle (30505),
MapError (30506), and SeatClaimError (30511) payloads are NIP-44
encrypted to the player's npub. This prevents relay operators and
observers from reading SSH connection details, map contents, or error
messages. The encryption uses secp256k1 ECDH shared secret with
XChaCha20-Poly1305.

First-join SessionRequest (30501) content carries the invite code and is not
encrypted by default. The code is a one-time bearer token that becomes
useless after claim. If pre-claim code confidentiality is important, the
content field can optionally be NIP-44 encrypted to the gate's npub.

### Metadata Visibility

NIP-44 protects payload contents, but relay-visible metadata remains:

- Event kind (30501, 30502, 30503)
- Tags (`d`, `p`, `ssh-pubkey`, `game-id`)
- Sender pubkey
- Event timestamp
- Message size

An observer can see that a specific npub requested a session from a
specific gate, and whether the gate responded with success or error.
They cannot see SSH credentials, game names, seat assignments, or error
details.

The `game-id` tag is visible when present. This reveals which game slug
the player is connecting to, but not the game name, seat, or any game
state. If slug confidentiality is desired, the tag could be moved into
an encrypted content field, but this adds complexity for minimal benefit
since the slug is a short opaque string.

This matches ec4x's security posture: payload confidentiality, not
metadata confidentiality.

### Trust Model

**Players trust:**
- `ec-gate` to enforce invite codes honestly and provision sessions
  correctly
- The relay to deliver events without censorship or modification
- The SSH server to be the one described in the SessionReady payload

**ec-gate trusts:**
- Nostr signatures to authenticate player identity
- The relay to deliver session requests from legitimate players

**Mitigations:**
- `ec-gate` is open source and auditable
- Players verify the SSH host key fingerprint from the SessionReady
  payload
- Multiple relay fallback could be supported for censorship resistance
  (future work)

## Comparison with ec4x

ec4x uses 8 event kinds (30400-30407) because Nostr is the full game
transport: turn commands, game state, deltas, player messages, and state
sync all flow through relays. EC uses 4 event kinds (30500-30503) because
Nostr is only the authentication layer. The game session runs over SSH.

| Aspect | ec4x | EC |
|--------|------|----|
| Event kinds | 8 (30400-30407) | 4 (30500-30503) |
| Transport | Nostr relays (full game) | SSH (game), Nostr (auth only) |
| Wire format | msgpack + zstd + NIP-44 | JSON + NIP-44 |
| Relay usage | Continuous (entire session) | Brief (handshake only) |
| Invite codes | Same format and lifecycle | Same format and lifecycle |
| Wallet | Same format and location | Same format and location |
| Identity | Same model | Same model |

The identity model, invite code system, and wallet format are shared so
that a future relay-transport client can reuse them directly.
