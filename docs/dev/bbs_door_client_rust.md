# Rust Player Client Path

This document captures the current path forward for the Rust player-side client
after the engine/oracle milestone.

It replaces older notes that assumed a future client crate and a deferred
runtime/storage split. The repository today is centered on:

- `ec-data`: canonical game state, classic `.DAT` parsing/writing, shared
  setup config/builder helpers, validation
- `ec-engine`: gameplay rules, maintenance, movement/pathfinding, and
  setup/mapgen execution
- `ec-cli`: sysop/admin/oracle/inspection workflows on top of `ec-data`
- `ec-client`: the SQLite-native player application layer

The next client work should be built on top of that reality, not beside it.

## Current Position

By project criteria, the engine side is now in the right place to support a
real player client:

- default `sysop new-game` creates a joinable `ECGAME`-compatible start
- `maint-rust` runs repeated campaigns and stays green against the current
  oracle suite
- `ec-client` already runs against `ecgame.db`
- `CoreGameData` is the canonical in-memory snapshot boundary
- classic `.DAT` directories remain the compatibility boundary, but only
  through explicit CLI import/export/materialization flows

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

Current versioning direction:

- preserve late-classic `v1.5` behavior and flow as the source baseline
- present the Rust continuation as Esterian Conquest (EC)
- avoid framing the Rust client as `EC 2.0` or `v1.6`

## Architecture Boundary

Keep the current crate responsibilities:

- `ec-engine`
  - owns gameplay rule execution, yearly maintenance, movement/pathfinding,
    and setup/map generation
  - consumes `ec-data` state/model types rather than duplicating them
- `ec-data`
  - owns runtime/store/model state, shared plain payload types, and setup
    config/builder helpers
- `ec-cli`
  - remains the sysop/admin/oracle/testing tool
  - owns the explicit classic import/export/materialization bridge
- `ec-client`
  - owns rendering, input, screen flow, and transport concerns
  - runs from SQLite-backed runtime state
  - does not reimplement game rules or emit classic `.DAT` files directly

Current workspace shape:

```text
rust/
├── ec-data     # runtime/store/model + shared payload/data helpers
├── ec-engine   # gameplay rule execution + maintenance/navigation/setup
├── ec-cli      # sysop/admin/oracle/inspection workflows + compat bridge
└── ec-client   # player-facing client (local first, door later)
```

If BBS door concerns grow large enough, that can later split into:

```text
rust/
├── ec-data
├── ec-engine
├── ec-cli
├── ec-client   # shared player UI/application layer
└── ec-door     # optional thin door launcher / dropfile adapter
```

That split should happen only if it buys clarity. It is not needed on day one.

## Storage Boundary

The storage direction is now:

- `ecgame.db` is the first-class persisted campaign store
- `CoreGameData` is the canonical in-memory snapshot model
- classic `.DAT` directories remain the required compatibility projection

SQLite is the runtime source of truth. Classic directories remain required for
oracle validation and DOS interoperability, but they now sit behind explicit
CLI import/export/materialization workflows instead of inside the player client.

Current storage rules:

- default campaign DB filename: `ecgame.db`
- SQLite must be bundled/self-hosted in the compiled Rust build
- no external SQLite runtime dependency should be required
- `ec-client` should load/save runtime state from `ecgame.db` only
- Rust maintenance should also run against `ecgame.db` and save the next
  snapshot there
- classic `.DAT` directories should enter or leave the Rust runtime only
  through explicit CLI import/export workflows
- `ec-client` should not create or refresh classic `.DAT` files as a side
  effect of normal play
- some classic-shaped outputs may still live in compatibility-oriented tables
  while the normalized Rust-native model matures
- total planet database / Main / General intel views should be free to consume
  SQLite-backed metadata such as `Last Intel` year

## UI Direction

The client should preserve EC's structure, not just its data.

That means:

- classic command-center organization
- reports/results review flow
- starmap and database viewing
- diplomacy/menu navigation
- order entry and order review
- map export/download from the galaxy map flow

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

ANSI policy:

- local `ec-client` should assume ANSI/CP437 output and render in color by default
- do not keep a plain-text local mode as a first-class UI target
- if future door compatibility needs the historical `ANSI color ON/OFF` prompt,
  keep that as an optional door-mode shim rather than the default EC flow

The client should treat the original UI as a fixed DOS playfield, not as a
modern fluid terminal layout:

- render into a fixed `80x25` playfield first
- center that playfield inside larger terminals
- keep menu bars, prompts, and reports positioned within that playfield
- do not globally center-justify ordinary text blocks
- use a real command-line cursor when the client is waiting for input
- do not add spinner-style idle animation; static prompt state is closer to
  the original feel
- when reusing preserved full-screen ANSI assets, it is acceptable to parse
  them on a virtual DOS-sized canvas first and then project/crop the result
  back into the real `80x25` player window
- keep that projection logic in the client renderer; do not dump raw ANSI
  directly to the user's terminal during normal `ec-client` startup

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

The renderer should follow a small cell-buffer model closer to `tcell` than to
widget-layout TUI frameworks:

- keep one shared `80x25` playfield buffer of styled cells
- let each screen write exact rows/columns into that buffer
- centralize terminal painting, palette handling, and cursor placement
- keep screen geometry screen-specific when the original layout is exact
- keep one shared internal table widget for all tabular screens, with the
  standard presentation defined in
  [ec-client-table-standard.md](ec-client-table-standard.md)
- keep the visual palette and semantic color tokens defined in
  [tui_style_guide.md](tui_style_guide.md)
- keep table browse/prompt rows on the shared `COMMANDS` grammar rather than
  per-screen command labels
- use one restrained Tokyo Night-inspired dark theme across the whole client
  instead of mixing bright inverse DOS bars with dark-body screens
- use split tables only when a screen genuinely needs two synchronized halves;
  grouped or stacked headers are not the normal list standard
- bootstrap the default theme KDL into the platform-standard user config
  directory on first run and load that file on startup
- keep interactive styling on `crossterm` rather than hand-built ANSI escape
  strings so Linux, macOS, and Windows terminal behavior stays aligned

In other words: DRY the rendering pipeline, not the classic screen geometry.

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

Keep the code layout aligned to that model:

- `src/app/` should stay the shell:
  - `App`
  - the root `Action`
  - the single update/reducer path
- `src/domains/<domain>/` should own that domain's:
  - state
  - screens/views
  - update dispatch
  - domain-specific `impl App` methods

Do not leave a second copy of domain controller logic under `src/app/` once the
domain slice exists.

### Client Loop Style

Use a reducer-style action/update/render loop inspired by SAM, but do not turn
that into a framework.

Recommended shape:

- screen renders current state
- key input maps to an `Action`
- one update path applies that action to app state
- the app renders again from the new state

That gives the useful part of SAM:

- explicit actions
- one-way state transitions
- render-from-state discipline

Without adding unnecessary ceremony like a formal acceptor/next-action
framework for what is currently a local terminal state machine.

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
- present maintenance/results reports before player mail, both on login and in
  later review flows
- keep player mail after reports so social commentary does not spoil
  maintenance outcomes before the official report stream is seen
- keep review/delete semantics explicit in client state handling

## Preserved Client Assets

The Rust client now has a stronger reference set for the original intro and
first-time flow under [artifacts/ecgame-client](../../artifacts/ecgame-client):

- [intro-sequence.txt](../../artifacts/ecgame-client/intro-sequence.txt)
  - text-flow reference assembled from earlier preserved captures
- [archive-2022/2022-07-23-NEW-GAME.txt](../../artifacts/ecgame-client/archive-2022/2022-07-23-NEW-GAME.txt)
  - richer escape-sequence transcript of the intro/new-game flow
- [archive-2022/ansi/first-time-menu.ans](../../artifacts/ecgame-client/archive-2022/ansi/first-time-menu.ans)
- [archive-2022/ansi/ftj-join.ans](../../artifacts/ecgame-client/archive-2022/ansi/ftj-join.ans)
- [archive-2022/ansi/post-join-first-menu.ans](../../artifacts/ecgame-client/archive-2022/ansi/post-join-first-menu.ans)

These should be treated as the current best reference set for:

- splash/logo timing and composition
- first-time join flow
- pre-main-menu report presentation
- first-menu layout after join

Current startup-art policy:

- startup now uses one built-in ASCII EC splash screen plus in-client intro
  pages
- keep startup inside the fixed `80x20` client model
- do not route startup through external ANSI assets or startup-specific KDL
  config unless a later art system is deliberately reintroduced

## Suggested Near-Term Milestones

### Milestone 1: Close Remaining Command Gaps

- finish the remaining command/menu surfaces that are still missing or rough
- keep screen flow and review/edit behavior aligned with classic `ECGAME`

### Milestone 2: Tighten Runtime Fidelity

- keep client views driven by SQLite-backed runtime/intel state
- remove any lingering assumptions that classic `.DAT` files are present or
  current during normal client play
- keep order-save paths on shared `ec-data` mutation helpers

### Milestone 3: Door Wrapper

- add door launch mode
- parse dropfiles
- keep local mode and door mode on the same application layer

## Non-Goals For The First Client Pass

- teaching `ec-client` to own classic `.DAT` import/export directly
- direct multiplayer networking
- inventing a new game UX unrelated to EC
- reproducing every DOS rendering quirk exactly
- solving the future Nostr/client-server track now

## Practical Recommendation

Continue building `ec-client` as a local terminal app that replays the current
`ECGAME` command flow over SQLite-backed runtime state and shared `ec-data`
helpers.

That path:

- uses the engine we already trust
- keeps the compatibility boundary explicit in `ec-cli`
- avoids reintroducing classic-file churn into normal client play
- gives the fastest route to a usable Rust replacement for `ECGAME`

Door support should follow once that local player workflow is solid.
