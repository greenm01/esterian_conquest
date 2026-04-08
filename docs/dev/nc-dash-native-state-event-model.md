# nc-dash Native State/Event Model

## Purpose

`nc-dash` is now a native local client. The visible dashboard stays the same:
same `PlayfieldBuffer`, same fixed cell-grid presentation, same overlays, same
map/table workflows. The migration is about the plumbing under that UI:
responsive local input, on-demand redraw, and a state/event loop that fits the
future Nostr-synced client.

## Why Native Now

The old SSH/PTY loop rendered before every blocking input read. That was good
enough for keyboard-first terminal use, but it is the wrong shape for
high-frequency mouse hover and for a local state-synced client.

The native move is not a visual rewrite. It is the transport/runtime rewrite
that removes PTY latency and lets `nc-dash` behave like a local application
while preserving the deliberate ANSI/cell-grid look.

## Core Decision

`nc-dash` uses a TEA-style native shell:

```text
WindowEvent -> DashMsg -> update(DashApp, DashMsg) -> DashEffect -> redraw if dirty
```

This is preferred over SAM for the client because:

- typed message enums are easier to test, log, and reason about in Rust
- pointer motion can be coalesced before update
- redraw policy can stay explicit and frame-boundary based
- async Nostr/cache work can re-enter the app as typed messages later

SAM remains reasonable for orchestration/daemon flows. It is not the preferred
client event model here.

## Current Code Shape

The first migration keeps `DashApp` as the concrete model object. The native
shell lives around it in `rust/nc-dash/src/native.rs`.

- `DashApp` still owns dashboard state and existing interaction logic.
- The native shell translates `winit` input into typed `DashMsg` values.
- `update()` on the native shell mutates `DashApp` and returns `DashEffect`
  values such as `RequestRedraw` and `Exit`.
- Rendering stays separate: `DashApp::render_playfield()` builds a
  `PlayfieldBuffer`, and the shared native renderer paints it to the window.

This is intentionally a compatibility step, not a full reducer rewrite. The
old crossterm-shaped key/mouse handlers still exist inside `DashApp`, but they
now sit behind a native shell boundary instead of owning the process loop.

## Shared Native Renderer

The native cell-grid stack now lives in `nc-ui`, not `nc-connect`.

- `rust/nc-ui/src/native/mod.rs`
- `rust/nc-ui/src/native/font.rs`

That shared layer owns:

- the window cell metrics
- centered pixel-to-cell hit testing
- JetBrains Mono + fallback font rasterization
- `PlayfieldBuffer` to `softbuffer` presentation
- shared `winit` -> crossterm-style key translation

`nc-connect` now uses the same shared renderer/input pieces. This avoids
drifting native stacks across two clients.

## Redraw Policy

`nc-dash` is redraw-on-demand.

- resize requests redraw
- key actions request redraw
- mouse button actions request redraw
- pointer motion is coalesced and only redraws when the effective hovered cell
  changes

There is no fixed-FPS render loop. The window loop uses `ControlFlow::Wait`.

## Pointer Coalescing

High-rate pointer motion is queued as pending cell movement and flushed once on
`AboutToWait`.

- many raw `CursorMoved` events collapse to the latest pending cell
- repeated movement within the same cell does not dispatch again
- leaving the rendered grid becomes a synthetic outside-pointer event

This is the first concrete fix for the old “molasses” crosshair behavior.

## Preserved UI Contract

This migration does **not** redesign `nc-dash`.

- no widget GUI rewrite
- no new visual language
- no pixel-position gameplay semantics
- no change to the dashboard layout model

The long-term presentation model is still the terminal-style cell grid rendered
from `PlayfieldBuffer`.

## Follow-up Direction

The next architectural step is to move more `DashApp` transitions behind a
pure reducer boundary so async Nostr state sync and command submission can
re-enter the model as typed messages rather than direct imperative calls. That
is a refinement step, not a prerequisite for the native migration already in
place.
