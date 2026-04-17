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
  `PlayfieldBuffer`, and the native presentation layer paints it to the
  window.

This is intentionally a compatibility step, not a full reducer rewrite. The
old crossterm-shaped key/mouse handlers still exist inside `DashApp`, but they
now sit behind a native shell boundary instead of owning the process loop.

## Renderer Status

The earlier native-renderer migration note is now partially stale.

`nc-dash` does **not** currently present through the `nc-ui` native renderer.
Its active path still goes through the vendored `ratatui-wgpu` backend in:

- `rust/nc-dash/src/native_grid/mod.rs`
- `rust/vendor/ratatui-wgpu-0.5.0/`

That mismatch matters because the app-side render contract is already
cell-buffer oriented while the current presentation path is still backend
oriented.

The renderer replacement direction is defined separately in
[nc-dash-glyphon-renderer.md](nc-dash-glyphon-renderer.md).

## Redraw Policy

`nc-dash` is redraw-on-demand.

- resize requests redraw
- key actions request redraw
- mouse button actions request redraw
- passive pointer motion is coalesced and only redraws when the effective
  hovered cell changes
- active modal dragging uses a latest-pointer-wins path: many raw drag events
  collapse to the newest pending cell and the app consumes that position once
  on the next redraw cycle

There is no fixed-FPS render loop. The window loop uses `ControlFlow::Wait`.

## Pointer Coalescing

High-rate pointer motion is queued as pending cell movement.

- hover motion flushes once on `AboutToWait`
- drag motion schedules redraw and flushes once on `RedrawRequested`
- many raw `CursorMoved` events collapse to the latest pending cell
- repeated movement within the same cell does not dispatch again
- leaving the rendered grid becomes a synthetic outside-pointer event

This avoids replaying stale intermediate drag positions and keeps the client
aligned to the latest pointer location instead of building a backlog.

## Native Rendering

The long-term renderer remains a native cell-grid presentation path, not a PTY
or terminal-emulation model.

The active implementation details are still in transition. This document keeps
the native event-loop model authoritative, while
[nc-dash-glyphon-renderer.md](nc-dash-glyphon-renderer.md) defines the intended
renderer replacement.

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
