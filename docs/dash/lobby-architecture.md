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
- `cache.kdl`: encrypted personal-games cache, direct
  contacts, direct chat history, anonymous game-mail history, and joined-game
  diplomacy rosters
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

- left pane: `My Games`
- center pane: `Open Games` recruiting table
- right pane: community `COMMS` hotlist

Use overlays for:

- first-run onboarding
- keychain unlock
- keychain password change
- handle edit
- join request compose
- relay or sync error detail

The tables should follow the existing repo table standards and stay data-dense,
not widget-heavy.

Theme ownership for hosted play now lives in lobby settings rather than the
hosted dashboard overlay. The selected theme applies to the whole `nc-dash`
process, including the hosted dashboard, but that theme runtime is local to
`nc-dash` and does not replace `nc-game` or `nc-door` theme handling.

## My Games Table

This table shows only games relevant to the active local identity.

Rows may represent:

- pending hosted join requests
- rejected hosted join requests
- joined hosted games

Suggested columns:

- status
- game
- host
- seat
- year/turn
- last activity

Primary actions:

- open joined game
- review pending/rejected request status
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

- select the game's published host contact as the default `THREADS` target
- compose/send a join request
- refresh the public catalog

## Communication Model

`nc-lobby` keeps community communication separate from anonymous in-game mail.

### Public Notices

The public area is one host-wide notice board.

- sysop-authored only
- readable by all lobby users on that daemon
- used for announcements, recruiting updates, outages, and general host notices

This is not a player chat room in v1.

### COMMS

`COMMS` is the lobby's community communication surface.

- encrypted by default
- contains one read-only `BROADCAST` thread for sysop/community notices
- contains direct-contact threads keyed by known contact `npub`
- seeded automatically with the selected game's published host contact
- manual contacts may be added by `npub` or NIP-05

The default direct thread target for a selected game should be that game's
published host contact. In production this may be the relay or game server
owner account such as `nc_sysop@nostrian-conquest.com`, but the lobby should
normally display the compact label, not the full NIP-05, in dense widgets.

Anonymous player-to-player diplomacy for joined games stays in-game rather than
inside lobby `COMMS`.

## Hosted Event Mapping

`nc-lobby` consumes and emits these hosted events:

- `30500 GameDefinition`: public recruiting table
- `30513 InviteRequest`: structured join request
- `30514 InviteRequestReceipt`: immediate daemon intake result
- `30515 InviteDecision`: approval or rejection, including assigned seat on approval
- `30516 LobbyNotice`: public host-wide notice post
- `30517 SysopThreadMessage`: optional host/operator private thread surface
- `30518 ContactMessage`: encrypted direct contact chat
- `30510 SeatClaimRequest`: reserve/manual claim path only
- `30511 SeatClaimResult`: reserve/manual claim result
- `30507 StateRequest`: refresh hosted state
- `30520 GameState`: full hosted snapshot
- `30521 StateDelta`: incremental hosted update
- `30522 TurnCommands`: hosted order submission
- `30523 PlayerMessage`: encrypted anonymous player-to-player diplomacy
- `30524 TurnReceipt`: hosted order intake result

The normal `nc-dash` lobby flow does not cache or expose invite strings. Invite
codes remain a hidden operator-only reserve path for manual seat issuance or
future paid-seat flows.

Only `30500` and `30516` are public/plain. All other hosted lobby/game events
use NIP-44, and large private payloads may be compressed inside the encrypted
envelope before relay publish.

## State And Sync Model

The lobby keeps a small client-owned model:

- active identity summary
- player handle
- relay target and connection health
- personal-games cache
- joined-game diplomacy rosters
- direct contact list
- direct `COMMS` history
- public/community notice feed window
- selected direct or broadcast transcript window

The daemon remains authoritative for:

- recruiting catalog
- request/decision state
- seat ownership
- encrypted direct-contact and anonymous game-mail delivery
- hosted game state

The client may cache recent notices, thread snippets, and joined-game metadata
for fast startup, but must tolerate cache deletion and daemon resync.

## Module Boundaries

Keep the hosted lobby split into focused modules under `rust/nc-dash/src/`.

- `lobby/state.rs`: lobby-only app state
- `lobby/models.rs`: table rows, contact rows, thread items, and local cache records
- `lobby/update.rs`: message/update logic
- `lobby/transport.rs`: relay subscriptions and hosted request/response wiring
- `lobby/onboarding.rs`: first-run handle/keychain flow
- `lobby/threads.rs`: `COMMS` transcript formatting and community-thread rendering
- `lobby/panels/`: joined-games, open-games, and community communication panes

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
- daemon rejects request
- cached community threads/notices unavailable until reconnect

All of these should be explicit UI states, not silent failures.

## Acceptance

The architecture is complete when:

- `nc-dash` has a lobby-first startup story
- local hosted client state is defined as KDL files under platform-specific
  user paths
- keychain encryption and password handling are clearly local-only
- encrypted cache/state files are clearly scoped to platform-specific user
  paths
- joined-games, open-games, and lobby `COMMS` are separate and well-scoped
- the doc leaves no ambiguity about how `nc-lobby` maps onto hosted daemon
  events
