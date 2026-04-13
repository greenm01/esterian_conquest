# Nostrian Conquest Hosted Architecture v2

> Status: future design draft, not the current shipped stack.
>
> Today the supported public gameplay surfaces are `nc-game` (localhost),
> `nc-door` (BBS), and `nc-sysop` (local/BBS administration). This document
> defines the separate future hosted stack centered on `nc-host` and
> `nc-dash`.

## 1. Core Direction

The hosted Nostr path is a clean split from localhost/BBS play.

- `nc-sysop` remains localhost/BBS-only.
- `nc-host` owns relay-native hosted play.
- `nc-dash` grows a hosted lobby plus hosted dashboard mode.
- `nc-dash` keeps its own local keychain/cache/settings in platform-specific
  user paths using KDL files.
- Hosted storage does not reuse localhost/BBS `ncgame.db`.

The daemon model is:

- one daemon process hosts many simultaneous games
- one dedicated relay/node belongs to that daemon
- one daemon identity keypair belongs to that daemon
- each hosted game keeps its own self-contained directory and DB

## 2. Hosted Topology

```text
+-----------+      +-----------+      +---------------------------+
|  nc-dash  | <--> |   relay   | <--> |        nc-host          |
|  lobby +  |      | (daemon-  |      | supervisor + game workers |
| dashboard |      | dedicated)|      +---------------------------+
+-----------+      +-----------+                    |
                                                    v
                                         <games-root>/<slug>/hosted.db
```

Localhost/BBS stays separate:

```text
+----------+     +----------+     +-----------+
| nc-game  | --> | nc-door  | --> | ncgame.db |
| nc-sysop |     |          |     |           |
+----------+     +----------+     +-----------+
```

## 3. Storage Boundary

Each hosted game is fully self-contained under its own directory.

Example:

```text
/srv/nc-host/games/friday-night/
  hosted.db
```

Daemon-global files live outside the games root:

```text
/etc/nc-host/host.kdl
/etc/nc-host/host.nsec
```

`hosted.db` is the authoritative per-game store for:

- game metadata and lobby settings
- seat roster and invite-code state
- maintenance schedule state
- pending turn submissions
- outbound publish/outbox jobs
- stored invite requests and audit trail

The hosted stack does not revive retired hosted tables inside localhost/BBS
`ncgame.db`.

## 4. Runtime Model

The daemon uses a hybrid TEA-style architecture.

- the process-level supervisor owns relay connectivity, game catalog loading,
  scheduling, and worker lifecycle
- each game runs as its own worker with a typed loop:
  `GameMsg -> update(GameRuntime, GameMsg) -> GameEffects`
- all DB mutation for one game is serialized through that game worker
- Nostr publishing is staged through a per-game outbox for retry-safe delivery

This keeps the code lean:

- no giant `serve.rs`
- no catch-all runtime object with every feature jammed into one module
- no shared multi-tenant mutation path across games

## 5. Lobby Model

`nc-dash --lobby` is the public hosted discovery surface.

The public lobby shows only games that are both:

- `lobby_visibility=public`
- actively recruiting

Recruiting values:

- `none`
- `new_players`
- `replacement_players`

Players see public recruiting metadata only:

- game name
- current year/turn
- recruiting mode
- open seat count
- short lobby summary
- host alias

Players do not see:

- raw invite codes
- hidden seat roster details
- private per-player state

The player client keeps minimal local state only:

- encrypted keychain
- encrypted joined-games and invite cache
- relay/config/settings files

These live in platform-specific user paths as KDL files and are not the
authoritative hosted store.

First launch flow:

1. set player handle
2. set keychain password
3. generate one local identity
4. write encrypted keychain/cache plus local config/settings files
5. enter the lobby

The handle is client-owned display metadata. It may be changed later. The
daemon may cache the latest handle seen for a pubkey for lobby and thread
display, but the pubkey remains authoritative.

All non-public hosted kinds are private-by-default:

- only `30500 GameDefinition` and `30516 LobbyNotice` remain public/plain
- `30507`, `30510`, `30513`, `30517`, `30518`, `30522`, and `30523` are
  player-authored private events
- `30511`, `30514`, `30515`, `30517`, `30520`, `30521`, `30523`, and `30524`
  are host-authored private events
- private event content uses NIP-44 plus an inner versioned envelope that may
  apply zstd compression for larger payloads

`nc-lobby` has two communication surfaces:

- one host-wide public notice board, sysop-authored only
- one encrypted direct-contact `THREADS` surface keyed by known `npub`
- one encrypted anonymous per-game `GAME INBOX` diplomacy surface keyed by
  game and empire

## 6. Invite and Join Flow

Hosted first joins still use old-style human-readable invite codes:

```text
{token}@{relay-host[:port]}
```

But the public lobby never exposes those codes. The server flow is:

1. `nc-host` publishes a public `30500 GameDefinition` for recruiting games.
2. `nc-dash --lobby` lists those games.
3. A player sends an invite request over Nostr to the daemon.
4. The daemon stores the request in the target game's request queue and
   notifies the sysop contact identity.
5. The sysop approves or rejects the request through `nc-host`.
6. If approved, the daemon privately sends the invite string to the player.
7. The player redeems that invite with a hosted claim event.
8. The daemon validates the invite token, binds the claimed seat to the
   player's pubkey, and returns a private claim result.
9. Later rejoin is by pubkey plus `game-id`, not by reusing the invite as the
   primary identity.

Seat lifecycle is intentionally small:

- `pending`
- `claimed`

Admin actions:

- reissue invite
- reset seat binding
- open seat
- close seat

## 7. Hosted Turn Policy

Hosted games default to wall-clock/manual resolution, not early auto-resolve.

Per-game settings include:

- `maintenance_enabled`
- `maintenance_interval_minutes`
- `maintenance_next_due_unix_seconds`

The supervisor schedules due games and sends maintenance work to the owning
game worker. The first spec does not auto-run turns merely because all players
submitted.

## 8. Nostr Event Surface

Hosted `nc-host` owns these kinds:

| Kind | Name | Purpose |
|------|------|---------|
| `30500` | `GameDefinition` | Public recruiting-game catalog row |
| `30507` | `StateRequest` | Client requests a refresh |
| `30510` | `SeatClaimRequest` | Player redeems an approved invite |
| `30511` | `SeatClaimResult` | Daemon confirms or rejects first join |
| `30513` | `InviteRequest` | Player asks the sysop for an invite |
| `30514` | `InviteRequestReceipt` | Daemon acknowledges receipt/rejection |
| `30515` | `InviteDecision` | Sysop approval or rejection result |
| `30516` | `LobbyNotice` | Public host-wide notice board post |
| `30517` | `SysopThreadMessage` | Legacy/operator private thread surface |
| `30518` | `ContactMessage` | Encrypted direct contact chat |
| `30520` | `GameState` | Full fog-of-war-filtered snapshot |
| `30521` | `StateDelta` | Incremental hosted update |
| `30522` | `TurnCommands` | Submitted player orders |
| `30523` | `PlayerMessage` | Encrypted anonymous player-to-player diplomacy |
| `30524` | `TurnReceipt` | Turn accepted/rejected with details |

Retired SSH bridge kinds `30501`/`30502`/`30503` remain legacy reference only
for the old `nc-connect` / `nc-gate` path.

For private request kinds, public tags stay routing-only:

- `d`
- `p`
- `game-id`
- `turn` for `30522`

Player handle, invite strings, state request bodies, and raw turn text live
inside the encrypted payload, not in public tags.

## 9. `GameDefinition` Requirements

`30500 GameDefinition` is the public lobby catalog event.

Required tags:

- `d` game id
- `name`
- `status`
- `players`
- `recruiting`
- `open-seats`
- `year`
- `turn`

Optional tags:

- `summary`
- `host-alias`
- `host-contact-npub`
- `host-contact-label`
- `host-contact-nip05`
- `slot`

`slot` remains hashed-only:

```text
["slot", "<seat>", "<invite-code-hash>", "<player-pubkey-or-empty>", "<status>"]
```

Public events must never reveal raw invite codes.

## 10. Module Layout

The hosted implementation should be split like this.

### `nc-host`

- `src/main.rs`
- `src/dispatch.rs`
- `src/commands/`
  - `new_game.rs`
  - `serve.rs`
  - `maint.rs`
  - `settings.rs`
  - `games.rs`
  - `seats.rs`
  - `requests.rs`
  - `nostr.rs`
- `src/config/`
  - `host_config.rs`
  - `identity.rs`
  - `relay.rs`
  - `sysop_contact.rs`
- `src/supervisor/`
  - `catalog.rs`
  - `runtime.rs`
  - `scheduler.rs`
  - `routing.rs`
- `src/lobby/`
  - `catalog_publish.rs`
  - `catalog_view.rs`
  - `invite_requests.rs`
  - `notify_sysop.rs`
  - `rate_limit.rs`
- `src/game/`
  - `msg.rs`
  - `state.rs`
  - `update.rs`
  - `effects.rs`
  - `maint.rs`
  - `seats.rs`
  - `turns.rs`
  - `outbox.rs`
- `src/support/`

### `nc-data`

- `src/hosted/`
  - `store.rs`
  - `schema.rs`
  - `settings.rs`
  - `seats.rs`
  - `invite_requests.rs`
  - `turn_queue.rs`
  - `outbox.rs`
  - `snapshots.rs`

### `nc-nostr`

- `game_definition.rs`
- `seat_claim.rs`
- `invite_request.rs`
- `state_sync.rs`
- `turn_commands.rs`
- `turn_receipt.rs`
- `tags.rs`
- `json.rs`

The ownership rule is the same as the rest of the repo:

- command modules orchestrate
- `nc-data` stores
- `nc-nostr` defines wire shapes
- `nc-engine` owns rules

## 11. Command Surface

### `nc-host`

```text
nc-host nostr init
nc-host new-game <dir> [--players N] [--name "Name"] [--seed N]
nc-host serve --root <games-root> [--config <path>]
nc-host status [--config <path>] [--root <path>] [--json]
nc-host games list
nc-host games status [--dir <path>]
nc-host maint <dir> [turns]
nc-host settings show --dir <path>
nc-host settings set --dir <path> ...
nc-host seats list --dir <path>
nc-host seats reissue --dir <path> --player N
nc-host seats reset --dir <path> --player N
nc-host seats open --dir <path> --player N
nc-host seats close --dir <path> --player N
nc-host requests list [--dir <path>]
nc-host requests show --dir <path> --request <id>
nc-host requests approve --dir <path> --request <id> --player N
nc-host requests reject --dir <path> --request <id> --message "..."
nc-host notices post --message "..." [--handle <name>]
nc-host threads list --dir <path>
nc-host threads show --dir <path> --player <npub>
nc-host threads send --dir <path> --player <npub> --message "..." [--handle <name>]
```

### `nc-dash`

```text
nc-dash
nc-dash --relay <url>
```

`nc-dash` is lobby-first for the hosted path. It keeps local keychain/cache
state in platform-specific KDL files and does not launch an SSH/PTy bridge.

See [../dash/lobby-architecture.md](../dash/lobby-architecture.md) for the
client-side hosted UI/state architecture.

## 12. Explicit Non-Goals

The first hosted spec does not require:

- relay federation
- per-game relay overrides
- a shared multi-tenant control DB
- direct player-to-player diplomacy events
- player-authored public lobby chat
- auto-issuing invites for public seats
- early auto-resolve when all turns are submitted

These can be added later without collapsing the per-game storage or per-game
worker boundaries.
