# Hosted Nostr Protocol

> Status: future `nc-host` / `nc-dash` draft.
>
> This document supersedes the older SSH session handshake as the target hosted
> direction. The retired `nc-connect` / `nc-gate` protocol remains historical
> reference only.

## 1. Scope

This protocol covers the future relay-native hosted path:

- public recruiting-game discovery
- daemon-mediated join requests
- encrypted direct contact chat
- encrypted anonymous per-game diplomacy
- private join approval/rejection delivery
- hosted state sync
- hosted turn submission

It does not cover localhost/BBS play.

## 2. Design Rules

- one daemon has one dedicated relay/node and one daemon identity
- all hosted games under that daemon publish through that relay
- public events expose only recruiting metadata
- all non-public hosted events use NIP-44 encryption
- raw invite codes are never published in public events
- the normal controlled-lobby first join is approval-based and code-free for the player

Private hosted payloads use one encrypted inner envelope:

- versioned JSON envelope inside NIP-44
- `compression = none` for small payloads
- `compression = zstd` for payloads at least `1024` bytes when compression is
  smaller than the original plaintext
- public relay tags remain routing-only; compression metadata stays inside the
  encrypted envelope

All request/response kinds use parameterized replaceable events with `d` as the
deduplication key unless a later implementation constraint proves otherwise.

## 3. Event Kinds

| Kind | Name | Publisher | Encryption | Purpose |
|------|------|-----------|------------|---------|
| `30500` | `GameDefinition` | `nc-host` | None | Public recruiting-game catalog row |
| `30507` | `StateRequest` | `nc-dash` | NIP-44 | Request a fresh snapshot or delta decision |
| `30510` | `SeatClaimRequest` | Reserve/manual client | NIP-44 | Redeem a reserved invite code |
| `30511` | `SeatClaimResult` | `nc-host` | NIP-44 | Reserve/manual first-join success or failure |
| `30513` | `InviteRequest` | `nc-dash` | NIP-44 | Ask to join a recruiting game |
| `30514` | `InviteRequestReceipt` | `nc-host` | NIP-44 | Request accepted or immediately rejected |
| `30515` | `InviteDecision` | `nc-host` | NIP-44 | Final sysop approval or rejection |
| `30516` | `LobbyNotice` | `nc-host` | None | Public host-wide notice board item |
| `30517` | `SysopThreadMessage` | `nc-host` / `nc-dash` | NIP-44 | Encrypted legacy/operator thread surface |
| `30518` | `ContactMessage` | `nc-dash` | NIP-44 | Encrypted direct contact chat by known `npub` |
| `30520` | `GameState` | `nc-host` | NIP-44 | Full fog-of-war-filtered state snapshot |
| `30521` | `StateDelta` | `nc-host` | NIP-44 | Incremental state update |
| `30522` | `TurnCommands` | `nc-dash` | NIP-44 | Submitted player turn orders |
| `30523` | `PlayerMessage` | `nc-host` / `nc-dash` | NIP-44 | Encrypted anonymous player-to-player game mail |
| `30524` | `TurnReceipt` | `nc-host` | NIP-44 | Turn submission accepted or rejected |

Legacy SSH-oriented kinds `30501`/`30502`/`30503` and related map/session
flows are intentionally outside this hosted v2 spec.

If configured, `nc-host` may also send a summary-only NIP-17/NIP-59 DM to the
sysop contact for Primal/mobile notification. That DM mirror is not part of
the authoritative hosted messaging protocol and does not replace `30518` or
`30523`.

## 4. 30500 `GameDefinition`

`GameDefinition` is the public lobby listing event. Only recruiting games with
`lobby_visibility=public` should publish it.

Example:

```json
{
  "kind": 30500,
  "pubkey": "<daemon-npub>",
  "created_at": 1770000000,
  "tags": [
    ["d", "friday-night"],
    ["name", "Friday Night NC"],
    ["status", "active"],
    ["players", "4"],
    ["recruiting", "replacement_players"],
    ["open-seats", "1"],
    ["year", "3012"],
    ["turn", "12"],
    ["summary", "Veteran game looking for one replacement admiral."],
    ["host-alias", "Green Host"],
    ["slot", "4", "<invite-code-hash>", "", "pending"]
  ],
  "content": "",
  "sig": "..."
}
```

Required tags:

- `d`: game id slug
- `name`: human-readable game name
- `status`: `setup`, `active`, or `finished`
- `players`: total seat count
- `recruiting`: `none`, `new_players`, or `replacement_players`
- `open-seats`: current open seat count
- `year`: current game year
- `turn`: current turn number

Optional tags:

- `summary`: short lobby-facing description
- `host-alias`: display name for the host/sysop
- `host-contact-npub`: direct contact `npub` for the listed host contact
- `host-contact-label`: compact label shown in the lobby contact list and host column
- `host-contact-nip05`: optional full NIP-05 stored privately by the client
- `slot`: hashed seat metadata for invite matching and diagnostics

`slot` shape:

```text
["slot", "<seat>", "<invite-code-hash>", "<player-pubkey-or-empty>", "<status>"]
```

Rules:

- raw invite codes are normalized, hashed, and never published directly
- non-recruiting games should not appear in the public lobby
- private games may omit `30500` entirely

## 5. 30513 `InviteRequest`

Players use this to ask to join a recruiting game from the lobby.

Example:

```json
{
  "kind": 30513,
  "pubkey": "<player-npub>",
  "created_at": 1770000100,
  "tags": [
    ["d", "<request-id>"],
    ["p", "<daemon-npub>"],
    ["game-id", "friday-night"]
  ],
  "content": "Interested in the replacement seat. Evening US availability.",
  "sig": "..."
}
```

Required tags:

- `d`: request id / nonce
- `p`: daemon pubkey
- `game-id`

Encrypted payload shape:

```json
{
  "message": "Interested in the replacement seat. Evening US availability.",
  "handle": "StarRider"
}
```

Rules:

- the event is signed by the player identity
- the request message and optional handle live only inside the encrypted
  payload, not in public tags
- the daemon persists the request in the target game store before any outbound
  notification side effects

## 6. 30514 `InviteRequestReceipt`

The daemon sends this immediately after it accepts or immediately rejects a
join request for processing.

Encrypted payload example:

```json
{
  "request_id": "<request-id>",
  "game_id": "friday-night",
  "status": "received",
  "message": "Your request has been queued for the sysop."
}
```

Possible `status` values:

- `received`
- `not_recruiting`
- `game_closed`
- `rate_limited`
- `unknown_game`

This is not the final approval decision. It only confirms daemon-side intake.

## 7. 30515 `InviteDecision`

The daemon sends this after the sysop approves or rejects the request.

Approved example:

```json
{
  "request_id": "<request-id>",
  "game_id": "friday-night",
  "decision": "approved",
  "message": "Seat 4 is yours.",
  "seat": 4
}
```

Rejected example:

```json
{
  "request_id": "<request-id>",
  "game_id": "friday-night",
  "decision": "rejected",
  "message": "The open seat has been filled."
}
```

Rules:

- `seat` is present only on approval
- in the normal `nc-dash` lobby flow, approval immediately binds that seat to
  the requesting player identity as part of the same game-level transaction
  that records the decision
- rejection never leaks seat or roster internals

## 7A. 30516 `LobbyNotice`

`LobbyNotice` is the public host-wide notice board event shown in
`nc-lobby`.

Required tags:

- `d`: notice id

Rules:

- published by the daemon/sysop only
- readable by all lobby users on that daemon
- used for announcements, recruiting notices, and outages
- not a public player chat surface in v1

## 7B. 30517 `SysopThreadMessage`

`SysopThreadMessage` remains available for host/operator-specific private
threads, but it is no longer the canonical player-to-player diplomacy channel.

Required tags:

- `d`: thread message id
- `p`: recipient pubkey
- `game-id`

Recommended payload fields:

```json
{
  "message_id": "<thread-message-id>",
  "game_id": "friday-night",
  "sender_role": "player",
  "sender_npub": "<player-npub>",
  "sender_handle": "StarRider",
  "body": "I can take the replacement seat and usually play evenings."
}
```

Rules:

- one persistent thread per player/game pair when the host chooses to expose it
- available before approval and after join
- sender handle is display metadata only and should be snapshotted at send time
- pubkey plus `game-id` remain authoritative
- `30517` is appropriate for sysop/operator contact and legacy hosted flows

## 7C. 30518 `ContactMessage`

`ContactMessage` is the encrypted direct-contact event used by the `THREADS`
surface in `nc-dash`.

Required tags:

- `d`: message id
- `p`: recipient pubkey

Payload fields:

```json
{
  "message_id": "<message-id>",
  "sender_npub": "<player-npub>",
  "sender_label": "nc_sysop",
  "body": "Relay maintenance window starts tonight.",
  "created_at": 1770000200
}
```

Rules:

- direct contacts are keyed by known `npub`
- the lobby may seed host contacts from `30500 host-contact-*` metadata
- manual contacts may be added by `npub` or resolved NIP-05
- clients should persist contact metadata and direct chat history locally in the
  encrypted cache

## 7D. 30523 `PlayerMessage`

`PlayerMessage` is the canonical encrypted player-to-player diplomacy channel
for joined hosted games.

Required tags:

- `d`: message id
- `p`: recipient pubkey
- `game-id`

Sender-to-host request payload:

```json
{
  "message_id": "<message-id>",
  "game_id": "friday-night",
  "sender_pubkey": "<player-pubkey>",
  "recipient_empire_id": 2,
  "body": "Shall we arrange a cease-fire?",
  "created_at": 1770000300
}
```

Recipient-facing payload:

```json
{
  "message_id": "<message-id>",
  "game_id": "friday-night",
  "sender_empire_id": 1,
  "sender_empire_name": "Terran Union",
  "recipient_empire_id": 2,
  "recipient_empire_name": "Rigel Empire",
  "body": "Shall we arrange a cease-fire?",
  "created_at": 1770000300
}
```

Rules:

- players never learn another player's `npub` from `30523`
- `nc-host` validates seat ownership and recipient empire before routing
- the client should label these conversations by game name plus empire name, not
  by pubkey
- this is the canonical `GAME INBOX` transport

## 8. 30510 `SeatClaimRequest`

`30510` remains the reserve/manual claim path for operator-issued invite codes.
It is no longer part of the normal `nc-dash` controlled-lobby join flow.

Required tags:

- `d`: nonce
- `p`: daemon pubkey
- `game-id`

Encrypted payload shape:

```json
{
  "invite": "amber-river@relay.example.com",
  "handle": "StarRider"
}
```

Rules:

- the encrypted payload carries the full invite string
- the daemon validates only the invite token portion against the stored seat
  hash
- first successful claim binds the seat to the player pubkey

## 9. 30511 `SeatClaimResult`

The daemon returns an encrypted claim result for the nonce from `30510`.

Example payload:

```json
{
  "nonce": "<claim-nonce>",
  "game_id": "friday-night",
  "status": "claimed",
  "message": "Seat 4 claimed.",
  "seat": 4
}
```

Possible `status` values:

- `claimed`
- `invalid_invite`
- `already_claimed`

## 10. 30507 `StateRequest`

The client requests a refresh after joining or reconnecting.

Encrypted payload shape:

```json
{
  "last_turn": 12,
  "last_hash": "abc123",
  "handle": "StarRider"
}
```

The daemon chooses whether to respond with `30520` or `30521`.

## 11. 30520 `GameState`

`GameState` sends a full fog-of-war-filtered snapshot.

Example encrypted payload:

```json
{
  "game_id": "friday-night",
  "turn": 12,
  "year": 3012,
  "player_seat": 4,
  "player_name": "Fourth Empire",
  "state_hash": "abc123",
  "state": {},
  "queued_mail": [],
  "report_blocks": []
}
```

Rules:

- one player's visible world only
- authoritative snapshot comes from the owning game worker
- payload includes a deterministic `state_hash` for cache validation

## 12. 30521 `StateDelta`

`StateDelta` sends an incremental hosted update when a full snapshot is not
needed.

Example encrypted payload:

```json
{
  "game_id": "friday-night",
  "turn": 13,
  "base_hash": "abc123",
  "state_hash": "def456",
  "deltas": {
    "planets": [],
    "fleets": [],
    "events": []
  }
}
```

Rules:

- deltas are per-player and fog-of-war filtered
- hash mismatch on the client should trigger a new `30507` requesting a full
  refresh

## 11. 30522 `TurnCommands`

Players submit turn orders with this event.

Encrypted payload shape:

```json
{
  "commands": "fleet 1 { order speed=3 kind=\"move\" x=5 y=10 }",
  "handle": "StarRider"
}
```

Rules:

- authorization is by claimed seat pubkey plus `game-id`
- public tags must not expose handle or raw turn text
- the daemon queues the submission in the per-game store before acknowledging it
- one game's turn queue must not block any other game

## 12. 30524 `TurnReceipt`

The daemon replies with the result of turn intake.

Accepted example:

```json
{
  "submit_id": "<turn-submit-id>",
  "game_id": "friday-night",
  "turn": 13,
  "status": "accepted",
  "message": "Orders staged for the next maintenance run."
}
```

Rejected example:

```json
{
  "submit_id": "<turn-submit-id>",
  "game_id": "friday-night",
  "turn": 13,
  "status": "rejected",
  "errors": [
    {
      "path": "fleet[1].order",
      "message": "target sector is outside the map"
    }
  ]
}
```

Status values:

- `accepted`
- `rejected`
- `superseded`
- `not_claimed`
- `wrong_turn`

## 13. Server Behavior Requirements

The daemon side must:

- persist join requests before notifying the sysop
- cache latest player display handle by pubkey from player-authored hosted
  events
- persist public notice posts and encrypted conversation events before
  publishing
- persist turn submissions before publishing receipts
- route every inbound event to the correct game worker by `game-id`
- keep retries in a per-game outbox
- republish `30500` when recruiting state or open-seat state changes
- refuse to start hosted service without a configured dedicated relay/node

## 14. Client Expectations

`nc-dash --lobby` should:

- subscribe to `30500` for public recruiting games
- render recruiting metadata only
- keep its local keychain, cache, config, and settings in KDL files under
  platform-specific user paths
- encrypt the local keychain and cache with the user's password
- prompt on first launch for player handle and keychain password
- submit `30513` for join requests
- listen for `30514` and `30515`
- subscribe to public `30516` notice posts
- seed host contacts from `30500 host-contact-*`
- send and receive encrypted `30518` direct contact messages
- send and receive encrypted `30523` anonymous game mail
- mark approved requests as joined locally using the assigned seat
- use `30507`, `30520`, `30521`, `30522`, and `30524` for live hosted play

## 15. Deferred

This draft intentionally defers:

- relay federation
- per-game relay overrides
- automatic invite issuance for public seats
- player-authored public lobby chat
