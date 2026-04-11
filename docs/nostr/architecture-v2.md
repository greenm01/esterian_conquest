# Nostrian Conquest Architecture v2

> Status: future design draft, not the current shipped stack.
>
> Today’s supported gameplay surfaces are `nc-game` (localhost), `nc-door`
> (BBS), and `nc-sysop` (local/BBS administration). This document describes a
> future relay-native hosted architecture centered on `nc-daemon` and
> `nc-dash`, not an implemented release.

## 1. Overview

Nostrian Conquest (NC) may evolve to support fully decentralized Nostr-powered
hosted games while preserving traditional localhost and BBS play modes.

**Core vision:**
- **Nostr-hosted:** Players discover and play games via Nostr protocol — no persistent VPS session required
- **Local/BBS:** Traditional play via nc-game (localhost) or nc-door (BBS) with direct campaign store access
- **Independent stacks:** nc-sysop (localhost/BBS) and nc-daemon (Nostr-hosted) are completely separate systems

---

## 2. Component Reference

| Component | Description | Status |
|-----------|-------------|--------|
| `nc-sysop` | Game creation/management for localhost/BBS only (no Nostr) | Active |
| `nc-game` | Localhost play TUI (direct ncgame.db access) | Active |
| `nc-door` | nc-game packaged for BBS sysops | Active |
| `nc-dash` | Dashboard UI (existing) + lobby mode (new) | Active |
| `nc-lobby` | Game picker + keychain (integrated into nc-dash) | New |
| `nc-daemon` | Nostr game server — game creation, hosting, state sync, turn submission | New |
| `nc-connect` | SSH bridge to nc-game | Deprecated |
| `nc-gate` | Nostr session provisioning daemon | Deprecated |

---

## 3. Play Modes

### 3.1 Localhost / BBS Play

```
+--------+     +----------+     +-----------+
| Player | --> | nc-game  | --> | ncgame.db |
| Local  |     | nc-door  |     | (local)   |
+--------+     +----------+     +-----------+
```

**Workflow:**
1. Sysop runs `nc-sysop new-game <dir> --players 4 --name "Game Name"`
2. Player runs `nc-game` or connects via BBS door (nc-door)
3. Player selects game directory, plays directly against local campaign store

**Commands:**
```bash
nc-sysop new-game <dir> [--players N] [--name "Name"] [--seed N]
nc-sysop new-game --bbs <dir>           # BBS mode with config.kdl
nc-sysop maint <dir> [turns]
nc-sysop settings show <dir>
nc-sysop settings set <dir> [--game-name "Name"] [--maintenance-enabled on|off]
nc-sysop settings reserve <dir> --player N --alias "Name"]
```

### 3.2 Nostr-Hosted Play

```
+--------+     +---------+      +------------+     +-----------+
| Player | --> | nc-dash | <--> |  Nostr    | --> | nc-daemon |
| Local  |     | (lobby) |      |   Relay   |     |  (VPS)    |
+--------+     +---------+      +------------+     +-----------+
                                                      |
                                                      v
                                                   ncgame.db
```

**Workflow:**
1. Sysop runs `nc-daemon new-game <dir> --players 4 --name "Game Name"`
2. nc-daemon creates game and publishes GameDefinition (30500) to relay
3. Player opens nc-dash, enters lobby mode
4. nc-dash scans relay for available games, displays game picker
5. Player selects game, nc-dash connects via Nostr
6. nc-daemon sends full state (30520), then deltas (30521)
7. Player submits turn via 30522 TurnCommands

**Commands:**
```bash
# nc-daemon (server side)
nc-daemon nostr init                      # Create daemon identity
nc-daemon new-game <dir> [--players N] [--name "Name"] [--seed N]  # Create hosted game
nc-daemon serve [--config <path>]         # Start Nostr event loop
nc-daemon host status                     # Show all game status
nc-daemon maint <dir> [turns]             # Run turn resolution
nc-daemon settings show <dir>
nc-daemon settings set <dir> [...]

# nc-dash (client side)
nc-dash --lobby                           # Enter lobby mode
nc-dash --lobby --relay <url>             # Connect to specific relay
nc-dash <game-dir>                        # Local dashboard (existing)
```

---

## 4. Nostr Protocol (305xx Range)

| Kind | Name | Direction | Purpose |
|------|------|-----------|---------|
| 30500 | GameDefinition | Server → Client | Published game metadata (name, players, status) |
| 30501 | SessionRequest | Client → Server | Request session (deprecated, for SSH bridge) |
| 30502 | SessionReady | Server → Client | Session approved (deprecated) |
| 30503 | SessionError | Server → Client | Session rejected (deprecated) |
| 30504 | MapRequest | Client → Server | Request player starmap |
| 30505 | MapBundle | Server → Client | Starmap data (encrypted) |
| 30506 | MapError | Server → Client | Map request failed |
| 30507 | StateRequest | Client → Server | Request game state refresh |
| 30508 | SessionStateReady | Server → Client | State data (metadata only) |
| 30509 | SessionStateError | Server → Client | State request failed |
| **30520** | **GameState** | **Server → Client** | **Full player-filtered state snapshot** |
| **30521** | **StateDelta** | **Server → Client** | **Incremental state changes** |
| **30522** | **TurnCommands** | **Client → Server** | **Player's turn orders** |
| 30523 | PlayerMessage | Bidirectional | NIP-44 encrypted P2P diplomacy (future) |

### 4.1 State Sync Flow (30520/30521)

```
Client (nc-dash)                    Server (nc-daemon)
     |                                     |
     |------ 30520 GameState (first) ----->|  # Full sync on connect
     |<---- 30520 GameState (reply) -------|  # Fog-of-war filtered
     |                                     |
     |   [player's turn]                  |
     |------ 30522 TurnCommands ---------->|  # Submit orders
     |                                     |  # nc-engine resolves
     |<---- 30521 StateDelta --------------|  # Incremental update
     |                                     |
     |   [repeat for each turn]           |
```

### 4.2 Event Payloads (Draft)

**30520 GameState (server → client):**
```json
{
  "game_id": "abc123",
  "turn": 42,
  "year": 3042,
  "player_seat": 2,
  "player_name": "Player 2 Empire",
  "state": { /* full CoreGameData, fog-of-war filtered */ },
  "queued_mail": [ /* pending reports */ ],
  "report_blocks": [ /* turn report rows */ ]
}
```

**30521 StateDelta (server → client):**
```json
{
  "game_id": "abc123",
  "turn": 43,
  "deltas": {
    "planets": [...],
    "fleets": [...],
    "events": [...]
  }
}
```

**30522 TurnCommands (client → server):**
```json
{
  "game_id": "abc123",
  "turn": 43,
  "orders": "fleet-move 1 5 10\nplanet-build 3 starbase\n..."
}
```

---

## 5. Local Cache Design

### 5.1 Overview

nc-dash maintains a local SQLite cache per game for offline play and fast dashboard rendering. The cache is fog-of-war filtered — it contains only data the player is allowed to see.

**Location:** `~/.local/share/nc/cache/<game-id>/cache.db`

### 5.2 Schema (Simplified for Client)

```sql
-- Game metadata
CREATE TABLE game_meta (
    game_id      TEXT PRIMARY KEY,
    game_name   TEXT NOT NULL,
    turn        INTEGER NOT NULL,
    year        INTEGER NOT NULL,
    player_seat INTEGER NOT NULL,
    player_name TEXT,
    state_hash  TEXT NOT NULL,        -- SHA256(state)
    updated_at  INTEGER NOT NULL       -- Unix timestamp
);

-- Visible planets (fog-of-war filtered)
CREATE TABLE planets (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    x            INTEGER NOT NULL,
    y            INTEGER NOT NULL,
    owner_seat   INTEGER,              -- NULL = unclaimed
    starbase     INTEGER DEFAULT 0,
    population   INTEGER DEFAULT 0,
    industry     INTEGER DEFAULT 0,
    updated_at   INTEGER NOT NULL
);

-- Visible fleets
CREATE TABLE fleets (
    id           INTEGER PRIMARY KEY,
    owner_seat   INTEGER NOT NULL,
    x            INTEGER NOT NULL,
    y            INTEGER NOT NULL,
    mission      TEXT,                 -- "move", "attack", "patrol", etc.
    warp         INTEGER DEFAULT 0,
    updated_at   INTEGER NOT NULL
);

-- Player's unsubmitted orders (local draft)
CREATE TABLE pending_orders (
    id           INTEGER PRIMARY KEY,
    order_type   TEXT NOT NULL,        -- "fleet-move", "planet-build", etc.
    target_id    INTEGER,
    value        TEXT,
    created_at   INTEGER NOT NULL
);

-- Cached turn reports/mail
CREATE TABLE reports (
    id           INTEGER PRIMARY KEY,
    turn         INTEGER NOT NULL,
    report_type  TEXT NOT NULL,        -- "battle", "production", "diplomacy", etc.
    subject      TEXT,
    body         TEXT,
    read         INTEGER DEFAULT 0,
    created_at   INTEGER NOT NULL
);

-- Sync metadata
CREATE TABLE sync_meta (
    key         TEXT PRIMARY KEY,
    value       TEXT
);
```

### 5.3 State Hash

**Purpose:** Validate cache freshness without transferring full state

**Calculation:**
```
state_hash = SHA256(
    game_id ||
    turn ||
    player_seat ||
    planet_state_checksum ||
    fleet_state_checksum
)
```

Where `*_state_checksum` is a deterministic hash of the fog-of-war filtered entities.

### 5.4 Sync Flow

```
nc-dash (local cache)              nc-daemon (server)
      |                                  |
      |  CONNECT                         |
      |  last_turn=N                    |
      |  last_hash=XYZ                  |
      |-------------------------------> |
      |                                  |
      |  If turn matches AND hash valid:|
      |<----- 30521 StateDelta ----------|  # Incremental
      |                                  |
      |  If turn gap > 5 OR hash mismatch:
      |<----- 30520 GameState ------------|  # Full sync
      |                                  |
      |  [merge into local cache]       |
      |  [update state_hash]            |
```

### 5.5 Offline Mode

**Fully offline operation:**
1. On launch → load from local cache
2. Player can view map, reports, status
3. Player can draft orders (stored in `pending_orders`)
4. When online → validate hash, sync deltas
5. When reconnecting → submit `pending_orders` as 30522

**No network detection:**
- If relay unreachable → continue in offline mode
- Show "OFFLINE" indicator in UI
- Queue orders locally

### 5.6 Cache Invalidation Triggers

| Trigger | Action |
|---------|--------|
| First launch (no cache) | Request 30520 |
| Turn gap > 5 | Request 30520 |
| Hash mismatch | Request 30520 |
| Hash matches, turn advances | Request 30521 |
| Explicit "Refresh" | Request 30520 |

---

## 6. Identity & Keychain

All Nostr clients share the same identity store:

**Location:** `~/.local/share/nc/wallet.kdl`

**Format:**
```kdl
keychain {
    active 0
    identity nsec1..." {
        nsec "nsec1..."
        type "local"
        created "2025-01-15T10:30:00Z"
        alias "My Identity"
    }
}
```

**Protected by:** ChaCha20-Poly1305 + PBKDF2-HMAC-SHA256

---

## 7. Data Flow Diagrams

### 7.1 Full Architecture (Target State)

```
+----------------+      +------------------+      +----------------+
|   VPS Host     |      |   Nostr Relay    |      |  Player Local  |
+----------------+      +------------------+      +----------------+
|                |      |                  |      |                |
|  +-----------+ |      |                  |      |  +-----------+ |
|  | nc-daemon | |<---->| 30500-30522     |<---->|  | nc-dash   | |
|  |           | |      |                  |      |  | (lobby)   | |
|  +-----------+ |      |                  |      |  +-----------+ |
|        |       |      |                  |      |        |       |
|        v       |      |                  |      |        v       |
|   ncgame.db    |      |                  |      |   nc-dash     |
|                |      |                  |      |   (dashboard)  |
+----------------+      +------------------+      +----------------+


+----------------+      +------------------+
|  BBS/Local     |      |  Sysop Local     |
+----------------+      +------------------+
|                |      |                  |
|  +-----------+ |      |  +-----------+  |
|  | nc-door   | |      |  | nc-sysop  |  |
|  | nc-game   | |      |  |           |  |
|  +-----------+ |      |  +-----------+  |
|        |       |      |        |         |
|        v       |      |        v         |
|   ncgame.db    |      |   ncgame.db      |
|   (local)      |      |   (local)        |
+----------------+      +------------------+
```

### 7.2 Lobby Mode Flow

```
nc-dash --lobby
    |
    +---> Unlock keychain (prompt password)
    |
    +---> Connect to relay(s)
    |
    +---> Subscribe to 30500 (GameDefinition)
    |
    +---> Display game picker
    |         |
    |         +---> [Select Game] ---> Connect to game
    |         |                           |
    |         |                           +---> Subscribe 30520/30521
    |         |                           +---> Load local cache
    |         |                           +---> Enter dashboard mode
    |         |
    |         +---> [Refresh] ---> Re-fetch 30500 events
    |
    +---> [Quit]
```

---

## 8. Migration Path

### Phase 1: Create nc-daemon (Priority: High)
- Implement 30520/30521/30522 protocol
- Add `new-game` command (independent from nc-sysop)
- Add `nostr init` and `serve` commands
- Test state sync with isolated game

### Phase 2: Integrate Lobby into nc-dash
- Add keychain support to nc-dash
- Add game picker UI (re-use nc-connect picker)
- Add relay connection management
- Add state sync client (30520/30521)

### Phase 3: Turn Submission
- Add turn builder UI in nc-dash
- Implement 30522 client-side submission
- Test full turn loop (submit → resolve → delta)

### Phase 4: Deprecation
- Mark nc-connect deprecated
- Mark nc-gate deprecated
- Update docs to point to nc-daemon
- Remove from CI/release builds (after transition period)

### Phase 5: Cleanup
- Remove nc-gate code
- Remove nc-connect code
- Update architecture documentation

---

## 9. Deferred / Future

- **30523 PlayerMessage** — NIP-44 encrypted P2P diplomacy
- **Relay federation** — Multiple relays, relay switching

---

## 10. Command Reference

### nc-sysop (localhost/BBS only)
```bash
nc-sysop new-game <dir> [--players N] [--name "Name"] [--seed N]
nc-sysop new-game --bbs <dir>
nc-sysop maint <dir> [turns]
nc-sysop maint-all [--config <path>]
nc-sysop settings show <dir>
nc-sysop settings set <dir> [--game-name "Name"] [--maintenance-enabled on|off]
nc-sysop settings reserve <dir> --player N --alias "Name"
nc-sysop settings unreserve <dir> --player N
```

### nc-daemon (Nostr-hosted only)
```bash
nc-daemon nostr init [--identity <path>]
nc-daemon new-game <dir> [--players N] [--name "Name"] [--seed N]
nc-daemon serve [--config <path>] [--identity <path>]
nc-daemon host status
nc-daemon maint <dir> [turns]
nc-daemon settings show <dir>
nc-daemon settings set <dir> [...]
```

### nc-dash (modified)
```bash
nc-dash <game-dir>                    # Dashboard mode (existing)
nc-dash --lobby                       # Lobby mode (new)
nc-dash --lobby --relay <url>         # Lobby with specific relay
```

---

*Document Version: 2.0*  
*Replaces: `docs/nostr/architecture-migration.md` (kept for historical context)*  
*Status: Draft*
