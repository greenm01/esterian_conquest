# ec-rust Invite Code System Design

## Overview

The invite code system uses NIP-05 identity resolution to bootstrap a secure
connection between a player and a game server, without requiring the player to
handle raw npubs or bech32-encoded strings.

## Invite Code Format

```
{token}@{domain}
```

Example: `foo-bar@esterianconquest.com`

- `token` — a short server-generated secret (e.g. two hyphenated words)
- `domain` — the sysop's domain, used to resolve server identity and relay

## Sysop Requirements

### 1. Nostr Keypair

The game server maintains its own Nostr keypair, separate from the sysop's
personal Nostr identity. This keypair signs all game events and serves as the
trust anchor for connecting players.

### 2. Nostr Relay

The sysop runs a Nostr relay (e.g. `nostr-rs-relay`) accessible at a known
WebSocket endpoint:

```
wss://relay.esterianconquest.com
```

Running a relay is a hard requirement for hosting an ec-rust game server. It
is the communication backbone for all client-server interaction and is
equivalent in spirit to the BBS node of the original Esterian Conquest era.

### 3. NIP-05 Identity File

Serve a static JSON file over HTTPS at:

```
https://{domain}/.well-known/nostr.json
```

Example:

```json
{
  "names": {
    "ec_sysop":    "npub1<sysop_pubkey>",
    "ec-rust":     "npub1<server_pubkey>"
  },
  "relays": {
    "npub1<server_pubkey>": ["wss://relay.esterianconquest.com"]
  }
}
```

- `ec_sysop` — the sysop's personal Nostr identity (optional but conventional)
- `ec-rust` — the game server's identity; this is what the client resolves
- `relays` — maps the server npub to its recommended relay URL (NIP-05 extension)

This file is served as static content by nginx. No dynamic backend required.

### nginx snippet

```nginx
location /.well-known/nostr.json {
    add_header Access-Control-Allow-Origin *;
    alias /var/www/nostr.json;
}
```

The CORS header is required by the NIP-05 spec.

## Client Connection Flow

Given invite code `foo-bar@esterianconquest.com`:

1. Parse domain: `esterianconquest.com`, token: `foo-bar`
2. Fetch `https://esterianconquest.com/.well-known/nostr.json`
3. Extract server npub from `names["ec-rust"]`
4. Extract relay URL from `relays[npub]`
5. Open WebSocket to `wss://relay.esterianconquest.com`
6. Subscribe to events signed by the server npub
7. Publish invite redemption event signed by player keypair, containing token `foo-bar`
8. Server validates token, issues game session

## Server-Side Invite Issuance

The server generates invite tokens and stores them in SQLite:

```sql
CREATE TABLE invites (
    token      TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    redeemed   INTEGER NOT NULL DEFAULT 0,
    player_pk  TEXT
);
```

Token format is at the sysop's discretion. Two hyphenated lowercase words
from a fixed word list is recommended for human friendliness.

## Player Relay Configuration

The relay embedded in nostr.json is the bootstrap relay only. Once connected,
the player's ec-rust client can be pointed at any relay the server publishes
to. The server may advertise additional or preferred relays via a
`kind:10002` relay list event, which the client picks up automatically.

The player can also override the relay manually in the ec-rust client config
at any time. The invite relay is not a permanent constraint.

## Security Properties

- The domain is the trust anchor. Players trust the sysop's domain via HTTPS,
  which resolves to the server npub, which signs all game events.
- The invite token is a short-lived server-side secret. It cannot be forged
  without access to the server's database.
- The server npub is the long-term game identity. Even if the sysop rotates
  their relay URL, existing invite codes remain valid as long as nostr.json
  is updated.
- No bech32 encoding appears in the user-facing invite code.
