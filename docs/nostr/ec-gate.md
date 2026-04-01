# nc-gate

`nc-gate` is the server-side daemon that authenticates players over Nostr
and provisions SSH sessions for `nc-game`. It runs on the VPS alongside
`sshd` and manages the bridge between Nostr identity and SSH access.

For sysops, the public command surface is `nc-sysop nostr ...`. The
`nc-gate` name remains the current internal crate/backend name.

In normal hosted play, the relay host is part of the invite itself, derived
from the configured `relay` URL. Players no longer need separate `--relay`
or `--gate` flags for the normal public join path.

## Responsibilities

- Listen on Nostr relays for claim and session request events from players
- Validate invite codes and bind player npubs to game seats
- Manage hosted seat state in each game's `ncgame.db`
- Provision ephemeral SSH authorized keys with `command=` restrictions
- Publish session-ready or session-error responses to players
- Publish compressed static map bundles to authorized players
- Publish game definition events for discovery
- Expire stale ephemeral keys
- Disambiguate multi-game players

## Daemon Identity

`nc-gate` has its own Nostr keypair, generated once on first
initialization:

```
$ nc-sysop nostr init
Daemon identity created at: /etc/nc-gate/identity.kdl
Public key (npub): npub1abc...xyz
```

The identity file:

```
/etc/nc-gate/identity.kdl
```

Or for user-level installs:

```
~/.local/share/nc-gate/identity.kdl
```

Format:

```kdl
daemon nsec="nsec1..." created="2026-03-26T12:00:00Z"
```

This keypair signs all events published by `nc-gate` (game definitions,
session responses). `nc-connect` can discover the daemon npub from a public
30500 game definition, and still accepts it explicitly as a fallback.

Running `init` again is safe and will not overwrite an existing identity.

## Hosted Seats

Each hosted game stores its seat claims in the campaign database:

```
/srv/ec/games/friday-night/ncgame.db
```

The hosted-seat rows track the player seat, current invite code, claim
status, and claimed player `npub`. This state is authoritative for
`nc-gate`. Legacy `roster.kdl` files are migration input only.

Fields:

| Field | Description |
|-------|-------------|
| `id` | Game identifier slug, derived from the directory name. Unique across all games on the server. Used in `game-id` tags for disambiguation. |
| `name` | Human-readable game name, loaded from the campaign settings rows in `ncgame.db`. |
| `player` | Player seat index (1-based, matches `nc-game --player <N>`) |
| `code` | Current invite code for this seat |
| `status` | `pending` (unclaimed) or `claimed` (bound to npub) |
| `npub` | Player's Nostr public key (present only when claimed) |

### Game ID Slugs

The game ID is a short slug derived from the game directory name. For
example, `/srv/ec/games/friday-night` produces `friday-night`. Slugs are
lowercase, alphanumeric with hyphens, and unique across all games managed
by the same `nc-gate` instance.

`nc-sysop` generates the slug when creating the game. `nc-gate` uses it
as the `d` tag in 30500 GameDefinition events and includes it in 30502
SessionReady payloads so that `nc-connect` can cache it for future
connections.

When a player completes the in-game first-time join flow and saves the
empire name, `nc-game` updates the hosted-seat row in `ncgame.db`: it sets
the seat to `claimed` and records the player's `npub`.

## Invite Code Management

### Generation

Invite codes are generated for hosted seats when `nc-sysop new-game`
creates the campaign, or later when the admin reissues one seat. Codes
are two words from the Monero mnemonic wordlist (1626 words), hyphenated
and lowercase.

```
$ nc-sysop new-game /srv/ec/games/friday-night --name "Friday Night EC" --players 4 --seed 1515
$ sudo nc-sysop host games add --config /etc/nc-gate/config.kdl --dir /srv/ec/games/friday-night
$ sudo systemctl restart nc-nostr.service
$ nc-sysop nostr seats --dir /srv/ec/games/friday-night
Game: Friday Night EC
Dir:  /srv/ec/games/friday-night

Seat 1  [pending]
  nc-connect --join velvet-mountain@relay.example.com
```

The sysop shares that single join line with each player. The relay host is
baked into the invite, and `nc-connect` discovers the rest from the relay's
published game definitions.

### Validation

For normal public first joins, `nc-gate` receives a 30501 session request with
the invite code still present:

1. Normalize the code (lowercase, trim whitespace, strip `@relay` suffix)
2. Search all configured hosted seats for a seat with a matching code
3. Verify the seat status is `pending`
4. Provision the SSH-backed `nc-game` session without claiming the seat yet
5. Let `nc-game` claim the seat only after the player saves the empire name
6. Publish an updated 30500 GameDefinition after that claim lands

If the player disconnects before saving the empire name, the invite remains
pending and reusable.

### Reissue

If a player loses their identity and needs a new invite code for their
seat:

```
$ nc-sysop nostr reissue --dir /srv/ec/games/friday-night --player 2
Reissued invite for seat 2: jade-river
```

This generates a new code, clears the old npub binding, and sets the seat
back to `pending`. The admin shares the new code with the player, who
redeems it with their new identity.

### Uniqueness

Invite codes must be unique across all games managed by the same
`nc-gate` instance. Code generation checks existing hosted seats and
regenerates on collision. With approximately 2.6 million possible
two-word combinations and typical game counts in the single digits, this
is not a practical concern.

## Session Routing

When `nc-gate` receives a 30501 SessionRequest, it determines which game
and seat to provision based on claimed identity and game selection:

### With game-id Tag

`nc-gate` looks up the player's npub in the hosted-seat table for the
specified game. If found, it provisions the session. If the npub is not
in that game, it returns an `unknown_player` error.

### Without game-id Tag

`nc-gate` searches all configured games for the player's npub:

- **One match:** Provision the session for that game and seat.
- **Multiple matches:** Return a 30503 SessionError with error code
  `multiple_games` and a game list in the encrypted payload. The payload
  includes the game ID, name, and seat number for each match.
  `nc-connect` presents the list to the player and retries with the
  selected `game-id` tag.
- **No matches:** Return an `unknown_player` error.

## Static Map Delivery

`nc-gate` also answers player map requests after a seat has been claimed.
This is part of the daemon behind `nc-sysop nostr serve`; there is no
separate public map-serving command.

When `nc-gate` receives a 30504 `MapRequest`:

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

If the request cannot be fulfilled, `nc-gate` returns a 30506
`MapError`. Failures here do not invalidate the player's seat or session;
they only prevent that particular map transfer.

## SSH Key Provisioning

When `nc-gate` validates a player's session request, it provisions a
short-lived SSH authorized key entry that allows the player's ephemeral
key to connect and run `nc-game` for the correct seat.

### Ephemeral Key Flow

1. The player's 30501 SessionRequest includes an ephemeral ed25519 SSH
   public key generated by `nc-connect` for this session only, plus the
   resolved `game-id`.
2. `nc-gate` writes an authorized key entry restricted with `command=`:

   ```
   command="exec /usr/local/bin/nc-game --dir /srv/ec/games/friday-night --player 2 --session-token 4d9f...",no-port-forwarding,no-X11-forwarding,no-agent-forwarding <ephemeral-pubkey>
   ```

3. The entry is written to the authorized keys store for the `ecgame`
   service user.
4. `nc-gate` publishes 30502 SessionReady to the player, including the
   game ID, SSH details, seat number, and the current empire name for
   that seat.
5. `nc-connect` uses the ephemeral private key to SSH in.
6. sshd matches the key, enforces the `command=` restriction, and the shell
   `exec`s `nc-game` in a PTY.
7. After the hosted session ends, `nc-connect` may issue a lightweight
   30507 state refresh so its local cache can pick up the player's final
   empire name and seat metadata.

### Key Expiration

Ephemeral keys have a short time-to-live (default: 60 seconds). If the
player does not connect within that window, `nc-gate` removes the key
entry. This prevents accumulation of stale keys.

When the SSH session ends (player disconnects), `nc-gate` also removes
the key entry. A background reaper task periodically scans for and
removes expired entries as a safety net.

### Service User

SSH sessions run under a dedicated system user (e.g., `ecgame`) with:

- A real shell (`/bin/bash` or `/bin/sh`) so sshd can execute the forced
  `nc-game` command after key authentication
- No home directory contents beyond the authorized keys mechanism
- No sudo or privilege escalation
- `command=` restriction on every authorized key entry

The player has zero access to the underlying system. The only process
that can ever run is `nc-game` with server-determined arguments.

## sshd Integration

### Option A: AuthorizedKeysCommand

sshd can be configured to call an external script to fetch authorized
keys dynamically:

```
# /etc/ssh/sshd_config (or a Match block)
Match User ecgame
    AuthorizedKeysCommand /usr/local/bin/nc-gate-keys %u
    AuthorizedKeysCommandUser ecgame
    ForceCommand /bin/false
```

`nc-gate-keys` is a helper that reads the current set of provisioned
ephemeral keys from a shared store (a file or a Unix socket to
`nc-gate`). This avoids writing to `authorized_keys` directly and is
the preferred approach.

### Option B: Dynamic authorized_keys File

`nc-gate` writes and manages entries in
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

`nc-gate` reads its configuration from:

```
/etc/nc-gate/config.kdl
```

Or for user-level installs:

```
~/.config/nc-gate/config.kdl
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
auth-keys-path "/var/lib/nc-gate/keys"

// Ephemeral key TTL in seconds
key-ttl 60

// Game directories to serve. Zero or more entries are valid.
game "/srv/ec/games/friday-night"
game "/srv/ec/games/saturday-showdown"
```

## Multi-Game Support

A single `nc-gate` instance can serve multiple games. Each game directory
has its own hosted-seat rows in `ncgame.db` with its own invite codes and
game ID slug.

On startup, `nc-gate` loads the hosted-seat snapshot for each configured
game from `ncgame.db` plus the human-readable `game_name` from the campaign
settings rows in that same database.

When invite codes are claimed or seats are reissued, `nc-gate` and
`nc-sysop` update the SQLite-backed hosted-seat rows directly.

A player's npub can appear in multiple hosted games on the same server.
The disambiguation protocol described in the Session Routing section
handles this case.

## Game Definition Publishing

`nc-gate` publishes a 30500 GameDefinition event for each game it serves.
This is a public event on the relay that includes the game name, number
of seats, and which seats are claimed (with npubs, but invite code hashes
rather than plaintext codes). This enables game discovery: a player with
`nc-connect` could list available games on a relay before joining.

Publishing game definitions is optional and can be disabled in the config
if the admin prefers fully private games.

## Deployment

### Prerequisites

- A VPS with a public IP
- `sshd` running and configured
- A Nostr relay (can be self-hosted with `nostr-rs-relay` or a public
  relay)
- If the relay is self-hosted on this VPS, a public HTTPS websocket front end
  for the relay host (for example Caddy proxying `relay.example.com` to a
  local `nostr-rs-relay` on `127.0.0.1:8080`)
- The `nc-game` and `nc-sysop` binaries installed
- One or more game directories created with `nc-sysop`

### Setup Steps

1. Install the `nc-sysop` binary
2. Run `nc-sysop nostr init` to generate the daemon identity
3. Create the `ecgame` system user
4. Configure sshd for the `ecgame` user (see sshd integration above)
5. Create games with `nc-sysop new-game`
6. Register the game directories in `/etc/nc-gate/config.kdl` as root
7. Restart the daemon after game-registry changes so it reloads the config
8. Start the daemon with `nc-sysop nostr serve` (systemd unit recommended)
9. Share invite codes with players

For a fresh VPS host, `scripts/install_vps.sh` can bootstrap the standard
filesystem layout, install the binaries under `/usr/local/bin`, write the
systemd units, and initialize `/etc/nc-gate/identity.kdl`. If the relay also
lives on that VPS, make sure its public reverse proxy is enabled too; the EC
daemon cannot reach `wss://relay-host` until something is actually listening
on `443`.

### systemd Unit

```ini
[Unit]
Description=EC Gate - Nostr Auth Daemon for Esterian Conquest
After=network-online.target sshd.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/nc-sysop nostr serve
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
