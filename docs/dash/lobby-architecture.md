# nc-lobby Architecture

## Overview

`nc-lobby` is the hosted-first entry flow inside the `nc-dash` binary.

It is not a separate binary and it is not a localhost/BBS surface. `nc-dash`
launches into the lobby by default, then transitions into the hosted dashboard
for a selected joined game. Direct game-dir startup remains available only as a
hidden developer path.

This document defines the client-side hosted architecture. The daemon/storage
topology and wire contract remain in:

- [../nostr/architecture-v2.md](../nostr/architecture-v2.md)
- [../nostr/protocol.md](../nostr/protocol.md)

## Core Direction

- keep `nc-dash` on the existing fullscreen native `nc-ui` window/cell-grid shell
- use `ratatui` inside `nc-dash` for lobby-owned settings/theme flows
- keep `nc-game` and `nc-door` on the existing `nc-ui` theme/runtime path
- keep local client state in KDL files under platform-specific user paths
- encrypt the local keychain with the user's password
- treat the player's pubkey as authoritative and the handle as editable display
  metadata
- keep the lobby as the primary player entry surface for hosted play

## Local Client State

`nc-lobby` keeps only client-owned state locally. Hosted game authority remains
on `nc-host`.

Required local files:

- `keychain.kdl`: encrypted single-identity keychain
- `cache.kdl`: encrypted joined-games cache, approved invite cache, inbox
  summaries, and per-game thread pointers
- `config.kdl`: optional relay and client preferences
- `settings.kdl`: local UI toggles plus the selected `nc-dash` theme key

Use platform-specific user paths:

- Linux/macOS XDG data dir for `keychain.kdl` and `cache.kdl`
  (for example `~/.local/share/nc/`)
- Linux/macOS XDG config dir for `config.kdl` and `settings.kdl`
  (for example `~/.config/nc/`)
- Windows AppData equivalents for the same files

The keychain and cache are encrypted with the user's password. The password is
local-only and is never transmitted to the daemon or relay.

The cache should stay KDL and remain convenience state only. It may be deleted
without losing hosted seat ownership.

## First Launch And Identity

First launch flow:

1. prompt for player handle
2. prompt for keychain password and confirmation
3. generate one local Nostr identity
4. save the encrypted keychain and initial cache/settings files
5. enter the lobby home view

Defaults:

- one active identity only
- no multi-identity picker in the main `nc-lobby` flow
- handle is identity-wide, editable later, and case-preserving
- import/export and advanced identity tooling stay secondary power-user paths

The player handle is client-owned display metadata. The daemon may cache the
latest handle seen for a pubkey for lobby and thread display, but the pubkey
remains authoritative.

For non-public hosted requests, the handle is sent only inside the encrypted
payload body. It is not exposed in public relay tags.

## Information Architecture

The lobby uses a three-pane layout inside the existing fullscreen `nc-dash`
shell.

- left pane: `Joined Games` table and `Inbox`
- center pane: `Open Games` recruiting table
- right pane: communication pane with `Notices` and the selected private
  `Thread`

Use overlays for:

- first-run onboarding
- keychain unlock
- keychain password change
- handle edit
- invite request compose
- invite approval / claim confirmation
- relay or sync error detail

The tables should follow the existing repo table standards and stay data-dense,
not widget-heavy.

Theme ownership for hosted play now lives in lobby settings rather than the
hosted dashboard overlay. The selected theme applies to the whole `nc-dash`
process, including the hosted dashboard, but that theme runtime is local to
`nc-dash` and does not replace `nc-game` or `nc-door` theme handling.

## Joined Games Table

This table shows only games relevant to the active local identity.

Rows may represent:

- claimed/joined hosted games
- approved-but-unclaimed invites

Pending invite requests do not live in the joined-games table. They appear in
the separate inbox pane.

Suggested columns:

- status
- game
- host
- seat
- year/turn
- last activity

Primary actions:

- open joined game
- open private sysop thread
- claim approved invite
- refresh state

## Open Games Table

This table is populated from public recruiting `30500 GameDefinition` events.

It shows recruiting metadata only:

- game name
- host alias
- recruiting mode
- open seats
- year/turn
- short summary

It never shows raw invite codes, private roster details, or per-player state.

Primary actions:

- open the game's private sysop thread
- compose/send an invite request
- refresh the public catalog

## Communication Model

`nc-lobby` has two distinct communication surfaces.

### Public Notices

The public area is one host-wide notice board.

- sysop-authored only
- readable by all lobby users on that daemon
- used for announcements, recruiting updates, outages, and general host notices

This is not a player chat room in v1.

### Private Per-Game Threads

Each game has one persistent encrypted private thread between the player and
the sysop.

- available before invite approval
- remains available after approval and after seat claim
- scoped to one game
- snapshots sender handle at send time for display history

Use the private thread for:

- invite request context and follow-up
- sysop questions or clarifications
- post-join hosted admin contact

The structured invite workflow remains separate from freeform thread messages.

## Hosted Event Mapping

`nc-lobby` consumes and emits these hosted events:

- `30500 GameDefinition`: public recruiting table
- `30513 InviteRequest`: structured invite request
- `30514 InviteRequestReceipt`: immediate daemon intake result
- `30515 InviteDecision`: approval or rejection, including full invite string
- `30516 LobbyNotice`: public host-wide notice post
- `30517 SysopThreadMessage`: encrypted per-game private thread message
- `30510 SeatClaimRequest`: claim approved invite
- `30511 SeatClaimResult`: claim success/failure
- `30507 StateRequest`: refresh hosted state
- `30520 GameState`: full hosted snapshot
- `30521 StateDelta`: incremental hosted update
- `30522 TurnCommands`: hosted order submission
- `30524 TurnReceipt`: hosted order intake result

Approved invite strings should be cached locally in `cache.kdl` for the active
identity until they are claimed or invalidated.

Only `30500` and `30516` are public/plain. All other hosted lobby/game events
use NIP-44, and large private payloads may be compressed inside the encrypted
envelope before relay publish.

## State And Sync Model

The lobby keeps a small client-owned model:

- active identity summary
- player handle
- relay target and connection health
- joined-games cache
- pending requests inbox
- approved-but-unclaimed invites
- public notice feed window
- selected private thread transcript window

The daemon remains authoritative for:

- recruiting catalog
- request/decision state
- seat ownership
- private thread persistence
- hosted game state

The client may cache recent notices, thread snippets, and joined-game metadata
for fast startup, but must tolerate cache deletion and daemon resync.

## Module Boundaries

Keep the hosted lobby split into focused modules under `rust/nc-dash/src/`.

- `lobby/state.rs`: lobby-only app state
- `lobby/models.rs`: table rows, thread items, and local cache records
- `lobby/update.rs`: message/update logic
- `lobby/transport.rs`: relay subscriptions and hosted request/response wiring
- `lobby/onboarding.rs`: first-run handle/keychain flow
- `lobby/threads.rs`: notices and private thread orchestration
- `lobby/panels/`: joined-games, open-games, inbox, and communication panes

Keep `main.rs`, the native shell, and the root app dispatcher thin. Do not let
hosted lobby logic accumulate in one giant app module.

## Failure And Empty States

Required states:

- no keychain yet
- locked keychain
- no relay configured
- relay unreachable
- no public recruiting games
- no joined games yet
- pending request with no decision yet
- approved invite waiting for claim
- daemon rejects request or claim
- cached thread/notices unavailable until reconnect

All of these should be explicit UI states, not silent failures.

## Acceptance

The architecture is complete when:

- `nc-dash` has a lobby-first startup story
- local hosted client state is defined as KDL files under platform-specific
  user paths
- keychain encryption and password handling are clearly local-only
- encrypted cache/state files are clearly scoped to platform-specific user
  paths
- joined-games, open-games, public notices, and private per-game threads are
  separate and well-scoped
- the doc leaves no ambiguity about how `nc-lobby` maps onto hosted daemon
  events
