# Hosted Nostr Protocol

> Status: future `nc-daemon` / `nc-dash` draft.
>
> This document supersedes the older SSH session handshake as the target hosted
> direction. The retired `nc-connect` / `nc-gate` protocol remains historical
> reference only.

## 1. Scope

This protocol covers the future relay-native hosted path:

- public recruiting-game discovery
- daemon-mediated invite requests
- private invite approval/rejection delivery
- hosted state sync
- hosted turn submission

It does not cover localhost/BBS play.

## 2. Design Rules

- one daemon has one dedicated relay/node and one daemon identity
- all hosted games under that daemon publish through that relay
- public events expose only recruiting metadata
- private per-player state uses NIP-44 encryption
- raw invite codes are never published in public events
- hosted first join still uses invite codes, but only after private approval

All request/response kinds use parameterized replaceable events with `d` as the
deduplication key unless a later implementation constraint proves otherwise.

## 3. Event Kinds

| Kind | Name | Publisher | Encryption | Purpose |
|------|------|-----------|------------|---------|
| `30500` | `GameDefinition` | `nc-daemon` | None | Public recruiting-game catalog row |
| `30507` | `StateRequest` | `nc-dash` | None | Request a fresh snapshot or delta decision |
| `30513` | `InviteRequest` | `nc-dash` | None | Ask the sysop for an invite |
| `30514` | `InviteRequestReceipt` | `nc-daemon` | NIP-44 | Request accepted or immediately rejected |
| `30515` | `InviteDecision` | `nc-daemon` | NIP-44 | Final sysop approval or rejection |
| `30520` | `GameState` | `nc-daemon` | NIP-44 | Full fog-of-war-filtered state snapshot |
| `30521` | `StateDelta` | `nc-daemon` | NIP-44 | Incremental state update |
| `30522` | `TurnCommands` | `nc-dash` | None | Submitted player turn orders |
| `30524` | `TurnReceipt` | `nc-daemon` | NIP-44 | Turn submission accepted or rejected |

Legacy SSH-oriented kinds `30501`/`30502`/`30503` and related map/session
flows are intentionally outside this hosted v2 spec.

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

Players use this to ask for an invite from the lobby.

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

Rules:

- the event is signed by the player identity
- the content is a short plain-text request message
- the daemon persists the request in the target game store before any outbound
  notification side effects

## 6. 30514 `InviteRequestReceipt`

The daemon sends this immediately after it accepts or immediately rejects an
invite request for processing.

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
  "invite": "amber-river@relay.example.com"
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

- `invite` is present only on approval
- approval should mint or reissue a seat code as part of the same game-level
  transaction that records the decision
- rejection never leaks seat or roster internals

## 8. 30507 `StateRequest`

The client requests a refresh after joining or reconnecting.

Example:

```json
{
  "kind": 30507,
  "pubkey": "<player-npub>",
  "created_at": 1770000200,
  "tags": [
    ["d", "<state-request-id>"],
    ["p", "<daemon-npub>"],
    ["game-id", "friday-night"]
  ],
  "content": "{\"last_turn\":12,\"last_hash\":\"abc123\"}",
  "sig": "..."
}
```

The daemon chooses whether to respond with `30520` or `30521`.

## 9. 30520 `GameState`

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

## 10. 30521 `StateDelta`

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

Example:

```json
{
  "kind": 30522,
  "pubkey": "<player-npub>",
  "created_at": 1770000300,
  "tags": [
    ["d", "<turn-submit-id>"],
    ["p", "<daemon-npub>"],
    ["game-id", "friday-night"],
    ["turn", "13"]
  ],
  "content": "fleet 1 { order speed=3 kind=\"move\" x=5 y=10 }",
  "sig": "..."
}
```

Rules:

- authorization is by claimed seat pubkey plus `game-id`
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

- persist invite requests before notifying the sysop
- persist turn submissions before publishing receipts
- route every inbound event to the correct game worker by `game-id`
- keep retries in a per-game outbox
- republish `30500` when recruiting state or open-seat state changes
- refuse to start hosted service without a configured dedicated relay/node

## 14. Client Expectations

`nc-dash --lobby` should:

- subscribe to `30500` for public recruiting games
- render recruiting metadata only
- submit `30513` for invite requests
- listen for `30514` and `30515`
- store approved invite strings in the player keychain/cache
- use `30507`, `30520`, `30521`, `30522`, and `30524` for live hosted play

## 15. Deferred

This draft intentionally defers:

- player-to-player diplomacy messaging
- relay federation
- per-game relay overrides
- automatic invite issuance for public seats
