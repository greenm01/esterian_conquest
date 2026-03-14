# Rust Player Client Path

This document captures the current path forward for the Rust player-side client
after the engine/oracle milestone.

It replaces older notes that assumed a different crate layout and an immediate
SQLite pivot. The repository today is centered on:

- `ec-data`: canonical game state, classic `.DAT` parsing/writing, maintenance,
  setup, validation
- `ec-cli`: sysop/admin/oracle/inspection workflows on top of `ec-data`

The next client work should be built on top of that reality, not beside it.

## Current Position

By project criteria, the engine side is now in the right place to support a
real player client:

- default `sysop new-game` creates a joinable `ECGAME`-compatible start
- `maint-rust` runs repeated campaigns and stays green against the current
  oracle suite
- `CoreGameData` is the canonical in-memory state boundary
- classic `.DAT` directories remain the compatibility boundary

That means the next phase is not "finish the engine first, then think about the
client." The next phase is building the player client on top of the engine we
now have.

## Goal

Build a Rust player client that preserves Esterian Conquest's command flow,
reports, and ANSI feel while replacing the original DOS/BBS constraints.

Initial target:

- local terminal client first
- BBS door support second
- optional direct telnet mode later if it still seems useful

The local client is the fastest path to replacing `ECGAME` behavior without
adding door-file and telnet complexity too early.

## Architecture Boundary

Keep the current crate responsibilities:

- `ec-data`
  - owns state, rules, maintenance, pathfinding, setup, report generation
  - continues to be the only place that knows classic file semantics
- `ec-cli`
  - remains the sysop/admin/oracle/testing tool
  - can keep growing inspection and setup helpers
- future player client crate
  - owns rendering, input, screen flow, and transport concerns
  - does not reimplement game rules

Recommended next workspace shape:

```text
rust/
├── ec-data     # state, rules, maintenance, setup, classic .DAT I/O
├── ec-cli      # sysop/admin/oracle/inspection workflows
└── ec-client   # future player-facing client (local first, door later)
```

If BBS door concerns grow large enough, that can later split into:

```text
rust/
├── ec-data
├── ec-cli
├── ec-client   # shared player UI/application layer
└── ec-door     # optional thin door launcher / dropfile adapter
```

That split should happen only if it buys clarity. It is not needed on day one.

## Storage Boundary

Do not move the player client to SQLite first.

The current project standard is:

- `CoreGameData` is the canonical state model
- classic `.DAT` directories are the required compatibility projection

So the first Rust player client should load and save the same game directories
the current engine already understands.

SQLite may still be useful later for:

- campaign history
- analytics
- sysop convenience
- richer modern hosting

But it should remain additive. It is not the right first dependency for
cloning `ECGAME`.

## UI Direction

The client should preserve EC's structure, not just its data.

That means:

- classic command-center organization
- reports/results review flow
- starmap and database viewing
- diplomacy/menu navigation
- order entry and order review

But it does not mean preserving every legacy interaction cost. The client can
improve:

- navigation speed
- clarity of status panels
- consistency of key handling
- command review/editing

The best near-term target is:

- faithful menu and report structure
- cleaner local-terminal ergonomics
- ANSI/CP437 presentation where it matters

## Rendering Stack

Current likely stack:

- terminal control: `crossterm`
- structured app state: hand-rolled screen/app model
- optional async/networking later: `tokio`
- logging/debugging: `tracing`

Avoid `ratatui` for the first pass.

Reasons:

- EC screens are closer to fixed-layout ANSI screens than widget dashboards
- CP437 art and exact menu placement matter
- the current job is preserving a specific command/report flow, not building a
  generic terminal app shell

## Transport Modes

Recommended order:

1. Local terminal mode
2. Door/dropfile mode
3. Direct telnet mode only if still justified

Local mode should be the main development loop because it removes:

- dropfile parsing
- BBS-specific launch friction
- telnet negotiation noise

Door mode can then wrap the same application layer with:

- `DOOR32.SYS` / `DOOR.SYS` parsing
- stdin/stdout transport
- CP437-safe output policy
- session time-limit awareness if needed

## Application Model

Keep the player client as a thin layer over `ec-data`.

Recommended core objects:

- `App`
  - current loaded `CoreGameData`
  - current player identity / empire
  - current screen stack
  - pending unsaved order mutations
- `Screen`
  - draw
  - handle key
  - return transition/action
- `Action`
  - pure UI intent that the app resolves against `CoreGameData`

The important point is separation:

- screens decide interaction flow
- `ec-data` decides state semantics
- rendering stays isolated from rules

## Immediate Feature Slice

The first usable Rust client does not need full parity on day one.

Recommended first slice:

1. Login/load existing player context
2. Auto-show pending reports/messages on entry
3. General command menu
4. Review messages/results
5. Empire/fleet/planet status views
6. Database and starmap viewing
7. Order review

After that:

8. Diplomacy commands
9. Planet commands
10. Fleet commands

This matches the repo's current strengths: the engine/report side is already
farther along than full player-order UI replication.

## Message/Report Handling

Current reality:

- classic player mail lives in `MESSAGES.DAT`
- `RESULTS.DAT` is the maintenance/report stream
- `ECGAME` auto-shows new items on login and later lets players review
  undeleted items

The Rust client should preserve that user-facing model even if the internal
report formatting stays Rust-native.

Near-term client rule:

- preserve classic queued mail if present
- display Rust maintenance reports through the same player-facing workflow
- keep review/delete semantics explicit in client state handling

## Suggested Near-Term Milestones

### Milestone 1: Client Skeleton

- add `ec-client` crate to the Rust workspace
- load a game directory and player identity
- render a static main menu / status shell in a local terminal

### Milestone 2: Read-Only Client

- show reports/messages
- show empire/planet/fleet summaries
- show database/starmap views
- no order editing yet

### Milestone 3: Order Workflow

- edit and review fleet/planet/diplomacy orders
- save back through existing `ec-data` mutation paths

### Milestone 4: Door Wrapper

- add door launch mode
- parse dropfiles
- keep local mode and door mode on the same application layer

## Non-Goals For The First Client Pass

- replacing `.DAT` with SQLite
- direct multiplayer networking
- inventing a new game UX unrelated to EC
- reproducing every DOS rendering quirk exactly
- solving the future Nostr/client-server track now

## Practical Recommendation

Start with `ec-client` as a local terminal app that replays the current
`ECGAME` command flow over `CoreGameData`.

That path:

- uses the engine we already trust
- keeps the compatibility boundary intact
- avoids premature storage churn
- gives the fastest route to a usable Rust replacement for `ECGAME`

Door support should follow once that local player workflow is solid.
