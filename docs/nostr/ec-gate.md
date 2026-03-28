# ec-gate

`ec-gate` is the server-side daemon that authenticates players over Nostr
and provisions SSH sessions for `ec-game`. It runs on the VPS alongside
`sshd` and manages the bridge between Nostr identity and SSH access.

For sysops, the public command surface is `ec-sysop nostr ...`. The
`ec-gate` name remains the current internal crate/backend name.

In normal hosted play, the sysop must also tell players which relay URL to
use. The gate npub alone is not enough for `ec-connect` to find the right
Nostr relay.

## Responsibilities

- Listen on Nostr relays for session request events from players
- Validate invite codes and bind player npubs to game seats
- Manage hosted seat state in each game's `ecgame.db`
- Provision ephemeral SSH authorized keys with `command=` restrictions
- Publish session-ready or session-error responses to players
- Publish compressed static map bundles to authorized players
- Publish game definition events for discovery
- Expire stale ephemeral keys
- Disambiguate multi-game players

## Daemon Identity

`ec-gate` has its own Nostr keypair, generated once on first
initialization:

```
$ ec-sysop nostr init
Daemon identity created at: /etc/ec-gate/identity.kdl
Public key (npub): npub1abc...xyz
```

The identity file:

```
/etc/ec-gate/identity.kdl
```

Or for user-level installs:

```
~/.local/share/ec-gate/identity.kdl
```

Format:

```kdl
daemon nsec="nsec1..." created="2026-03-26T12:00:00Z"
```

This keypair signs all events published by `ec-gate` (game definitions,
session responses). Players and `ec-connect` use the daemon's npub to
verify that session-ready events are authentic.

Running `init` again is safe and will not overwrite an existing identity.

## Hosted Seats

Each hosted game stores its seat claims in the campaign database:

```
/srv/ec/friday-night/ecgame.db
```

The hosted-seat rows track the player seat, current invite code, claim
status, and claimed player `npub`. This state is authoritative for
`ec-gate`. Legacy `roster.kdl` files are migration input only.

Fields:

| Field | Description |
|-------|-------------|
| `id` | Game identifier slug, derived from the directory name. Unique across all games on the server. Used in `game-id` tags for disambiguation. |
| `name` | Human-readable game name, loaded from `config.kdl` `game_name`. |
| `player` | Player seat index (1-based, matches `--player-record-index-1-based`) |
| `code` | Current invite code for this seat |
| `status` | `pending` (unclaimed) or `claimed` (bound to npub) |
| `npub` | Player's Nostr public key (present only when claimed) |

### Game ID Slugs

The game ID is a short slug derived from the game directory name. For
example, `/srv/ec/friday-night` produces `friday-night`. Slugs are
lowercase, alphanumeric with hyphens, and unique across all games managed
by the same `ec-gate` instance.

`ec-sysop` generates the slug when creating the game. `ec-gate` uses it
as the `d` tag in 30500 GameDefinition events and includes it in 30502
SessionReady payloads so that `ec-connect` can cache it for future
connections.

When a player redeems an invite code, `ec-gate` updates the hosted-seat
row in `ecgame.db`: it sets the seat to `claimed` and records the
player's `npub`.

## Invite Code Management

### Generation

Invite codes are generated for hosted seats when `ec-sysop new-game`
creates the campaign, or later when the admin reissues one seat. Codes
are two words from the Monero mnemonic wordlist (1626 words), hyphenated
and lowercase.

```
$ ec-sysop new-game /srv/ec/friday-night --players 4 --seed 1515
$ ec-sysop nostr seats --dir /srv/ec/friday-night
Game: Friday Night EC
Dir: /srv/ec/friday-night
Seat  1: pending velvet-mountain
Seat  2: pending copper-sunrise
Seat  3: pending amber-cascade
Seat  4: pending silver-meadow
```

The sysop shares the invite code, daemon `npub`, and relay URL with each
player. The relay is not baked into the invite code.

### Validation

When `ec-gate` receives a 30501 SessionRequest with an invite code:

1. Normalize the code (lowercase, trim whitespace, strip `@relay` suffix)
2. Search all configured hosted seats for a seat with a matching code
3. Verify the seat status is `pending`
4. Bind the player's npub to the seat (set status to `claimed`)
5. Save the updated hosted-seat row in `ecgame.db`
6. Proceed to provision the SSH session

If validation fails, `ec-gate` publishes a 30503 SessionError with the
reason.

### Reissue

If a player loses their identity and needs a new invite code for their
seat:

```
$ ec-sysop nostr reissue --dir /srv/ec/friday-night --player 2
Reissued invite for seat 2: jade-river
```

This generates a new code, clears the old npub binding, and sets the seat
back to `pending`. The admin shares the new code with the player, who
redeems it with their new identity.

### Uniqueness

Invite codes must be unique across all games managed by the same
`ec-gate` instance. Code generation checks existing hosted seats and
regenerates on collision. With approximately 2.6 million possible
two-word combinations and typical game counts in the single digits, this
is not a practical concern.

## Session Routing

When `ec-gate` receives a 30501 SessionRequest, it determines which game
and seat to provision based on the request contents:

### With Invite Code

The invite code uniquely identifies the game and seat (codes are unique
across all games on the server). `ec-gate` validates the code, binds the
npub, and provisions the session. The `game-id` tag, if present, is
ignored when an invite code is provided.

### Without Invite Code, With game-id Tag

`ec-gate` looks up the player's npub in the hosted-seat table for the
specified game. If found, it provisions the session. If the npub is not
in that game, it returns an `unknown_player` error.

### Without Invite Code, Without game-id Tag

`ec-gate` searches all configured games for the player's npub:

- **One match:** Provision the session for that game and seat.
- **Multiple matches:** Return a 30503 SessionError with error code
  `multiple_games` and a game list in the encrypted payload. The payload
  includes the game ID, name, and seat number for each match.
  `ec-connect` presents the list to the player and retries with the
  selected `game-id` tag.
- **No matches:** Return an `unknown_player` error.

## Static Map Delivery

`ec-gate` also answers player map requests after a seat has been claimed.
This is part of the daemon behind `ec-sysop nostr serve`; there is no
separate public map-serving command.

When `ec-gate` receives a 30504 `MapRequest`:

1. Read the signed player npub from the event
2. Read the requested `game-id` tag
3. Authorize the request by checking that the npub is bound to a seat in
   that game's hosted-seat table
4. Build the player-safe fog-of-war starmap bundle for that seat
5. Compress each file individually with `zstd`, base64-encode it for
   JSON transport, and publish 30505 `MapBundle`

The bundle always contains:

- `starmap.txt`
- `starmap.csv`
- `starmap-DETAILS.csv`

If the request cannot be fulfilled, `ec-gate` returns a 30506
`MapError`. Failures here do not invalidate the player's seat or session;
they only prevent that particular map transfer.

## SSH Key Provisioning

When `ec-gate` validates a player's session request, it provisions a
short-lived SSH authorized key entry that allows the player's ephemeral
key to connect and run `ec-game` for the correct seat.

### Ephemeral Key Flow

1. The player's 30501 SessionRequest includes an ephemeral ed25519 SSH
   public key generated by `ec-connect` for this session only.
2. `ec-gate` writes an authorized key entry restricted with `command=`:

   ```
   command="/usr/local/bin/ec-game --game-dir /srv/ec/friday-night --player-record-index-1-based 2",no-port-forwarding,no-X11-forwarding,no-agent-forwarding <ephemeral-pubkey>
   ```

3. The entry is written to the authorized keys store for the `ecgame`
   service user.
4. `ec-gate` publishes 30502 SessionReady to the player, including the
   game ID, SSH details, seat number, and the current empire name for
   that seat.
5. `ec-connect` uses the ephemeral private key to SSH in.
6. sshd matches the key, enforces the `command=` restriction, and spawns
   `ec-game` in a PTY.

### Key Expiration

Ephemeral keys have a short time-to-live (default: 60 seconds). If the
player does not connect within that window, `ec-gate` removes the key
entry. This prevents accumulation of stale keys.

When the SSH session ends (player disconnects), `ec-gate` also removes
the key entry. A background reaper task periodically scans for and
removes expired entries as a safety net.

### Service User

SSH sessions run under a dedicated system user (e.g., `ecgame`) with:

- No login shell (`/usr/sbin/nologin` or `/bin/false`)
- No home directory contents beyond the authorized keys mechanism
- No sudo or privilege escalation
- `command=` restriction on every authorized key entry

The player has zero access to the underlying system. The only process
that can ever run is `ec-game` with server-determined arguments.

## sshd Integration

### Option A: AuthorizedKeysCommand

sshd can be configured to call an external script to fetch authorized
keys dynamically:

```
# /etc/ssh/sshd_config (or a Match block)
Match User ecgame
    AuthorizedKeysCommand /usr/local/bin/ec-gate-keys %u
    AuthorizedKeysCommandUser ecgame
    ForceCommand /bin/false
```

`ec-gate-keys` is a helper that reads the current set of provisioned
ephemeral keys from a shared store (a file or a Unix socket to
`ec-gate`). This avoids writing to `authorized_keys` directly and is
the preferred approach.

### Option B: Dynamic authorized_keys File

`ec-gate` writes and manages entries in
`/home/ecgame/.ssh/authorized_keys` directly. Simpler to set up but
requires file-level coordination and atomic writes.

### Recommended sshd Hardening

```
Match User ecgame
    PasswordAuthentication no
    PubkeyAuthentication yes
    PermitTTY yes
    X11Forwarding no
    AllowTcpForwarding no
    AllowAgentForwarding no
    PermitOpen none
    MaxSessions 6
```

## Configuration

`ec-gate` reads its configuration from:

```
/etc/ec-gate/config.kdl
```

Or for user-level installs:

```
~/.config/ec-gate/config.kdl
```

Format:

```kdl
// Nostr relay to listen on and publish to
relay "wss://relay.example.com"

// SSH server details sent to players in SessionReady
ssh-host "play.example.com"
ssh-port 22

// Service user for SSH sessions
ssh-user "ecgame"

// Authorized keys mechanism
auth-keys-method "command"  // "command" or "file"
auth-keys-path "/var/lib/ec-gate/keys"

// Ephemeral key TTL in seconds
key-ttl 60

// Game directories to serve
game "/srv/ec/friday-night"
game "/srv/ec/saturday-showdown"
```

## Multi-Game Support

A single `ec-gate` instance can serve multiple games. Each game directory
has its own hosted-seat rows in `ecgame.db` with its own invite codes and
game ID slug.

On startup, `ec-gate` loads the hosted-seat snapshot for each configured
game from `ecgame.db` plus the human-readable `game_name` from
`config.kdl`.

When invite codes are claimed or seats are reissued, `ec-gate` and
`ec-sysop` update the SQLite-backed hosted-seat rows directly.

A player's npub can appear in multiple hosted games on the same server.
The disambiguation protocol described in the Session Routing section
handles this case.

## Game Definition Publishing

`ec-gate` publishes a 30500 GameDefinition event for each game it serves.
This is a public event on the relay that includes the game name, number
of seats, and which seats are claimed (with npubs, but invite code hashes
rather than plaintext codes). This enables game discovery: a player with
`ec-connect` could list available games on a relay before joining.

Publishing game definitions is optional and can be disabled in the config
if the admin prefers fully private games.

## Deployment

### Prerequisites

- A VPS with a public IP
- `sshd` running and configured
- A Nostr relay (can be self-hosted with `nostr-rs-relay` or a public
  relay)
- The `ec-game` and `ec-sysop` binaries installed
- One or more game directories created with `ec-sysop`

### Setup Steps

1. Install the `ec-sysop` binary
2. Run `ec-sysop nostr init` to generate the daemon identity
3. Create the `ecgame` system user
4. Configure sshd for the `ecgame` user (see sshd integration above)
5. Create games with `ec-sysop new-game`
6. Configure `ec-gate` with relay URL and game directories
7. Start the daemon with `ec-sysop nostr serve` (systemd unit recommended)
8. Share invite codes with players

### systemd Unit

```ini
[Unit]
Description=EC Gate - Nostr Auth Daemon for Esterian Conquest
After=network-online.target sshd.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/ec-sysop nostr serve
Restart=on-failure
RestartSec=5
User=ecgame
Group=ecgame

[Install]
WantedBy=multi-user.target
```

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `nostr-sdk` | Nostr protocol: event signing, NIP-44 encryption, relay WebSocket |
| `tokio` | Async runtime for relay listener and key reaper |
| `russh-keys` | ed25519 key generation and OpenSSH authorized_keys formatting |
| `kdl` | Config and legacy roster migration parsing |
| `clap` | CLI argument parsing if the daemon gains direct developer-only helper entrypoints again |
