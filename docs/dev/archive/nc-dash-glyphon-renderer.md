# nc-dash Glyphon Renderer Architecture

## Purpose

This note defines the renderer direction for `nc-dash` before implementation
starts. It exists because the current `nc-dash` renderer path is not the one
described in older native-rendering notes, and because glyph correctness is now
the forcing problem.

This is an `nc-dash`-only architecture note. It does not change `nc-ui`,
`nc-game`, or `nc-door`.

## Current State

Today, `nc-dash` still presents through a vendored `ratatui-wgpu` backend:

- `rust/nc-dash/src/native.rs` owns the native `winit` shell
- `rust/nc-dash/src/native_grid/mod.rs` builds a `Terminal<WgpuBackend>`
- `rust/vendor/ratatui-wgpu-0.5.0/` owns the current GPU text pipeline

The app-side rendering contract is already narrower than that backend surface:

- dashboard rendering builds a final `PlayfieldBuffer`
- lobby rendering also ends as a final `PlayfieldBuffer`
- `RenderedUi` is a presentation adapter that converts that final playfield into
  a ratatui `Buffer` for the current backend path

So the real app model is already cell-grid first. `ratatui::Terminal` is not
the source of truth for layout or interaction semantics in `nc-dash`.

## Problem

The immediate problem is glyph rendering correctness in the current
`ratatui-wgpu` path.

The deeper architectural problem is ownership:

- `nc-dash` needs exact cell-grid rendering, not a generic terminal backend
- the vendored backend owns text shaping, font fallback, glyph atlas behavior,
  and cell presentation rules that `nc-dash` cannot treat as incidental
- forking `ratatui-wgpu` would preserve the wrong abstraction and the wrong
  maintenance burden

`nc-dash` only needs a small part of the current backend contract:

- create the GPU presentation path
- resize with the window
- render the final cell grid

It does not need a general-purpose ratatui GPU backend with a broad public API.

## Architectural Decision

`nc-dash` should replace the current vendored `ratatui-wgpu` presentation path
with a tightly integrated renderer that is private to `nc-dash`.

This renderer should:

- consume the final `PlayfieldBuffer` directly
- use `glyphon` + `wgpu` for text rendering
- keep cell-grid presentation rules owned by `nc-dash`
- replace the vendored `ratatui-wgpu` path rather than layering a second full
  presentation stack beside it

This renderer should not:

- live in `nc-ui`
- start as a standalone project
- start as a generic reusable crate
- start as a fork of `ratatui-wgpu`

## Why Not Fork ratatui-wgpu

Forking `ratatui-wgpu` would keep the project inside a generic-backend design
that `nc-dash` does not need.

That would preserve:

- backend-oriented API surface
- backend-owned text/fallback/cache policy
- upgrade burden from a vendored third-party architecture
- extra renderer behavior that is not part of `nc-dash`'s real contract

The likely result would be a fork that still behaves like a terminal backend
library while the game only needs a precise native cell-grid renderer.

## Why Direct Rendering Fits nc-dash

Direct rendering fits the current `nc-dash` code shape better because the game
already computes the final cell buffer before presentation.

That gives the renderer a clean input contract:

- cell contents
- cell foreground/background colors
- cursor/highlight state
- exact window pixel dimensions

It also keeps presentation ownership where the project actually needs it:

- exact cell metrics
- per-cell clipping
- centered grid placement inside the window
- cursor inversion/highlight behavior
- box-drawing, block, and map primitives
- redraw and diff policy

## Renderer Ownership Split

The new `nc-dash` renderer should own:

- window and surface setup for the `nc-dash` native path
- cell metrics and grid pixel sizing
- per-cell background fills
- per-cell clipping rules
- cursor/invert/highlight presentation
- hard-rendered line/block/shade/map primitives when font rendering is not the
  right source of truth
- frame diffing and redraw policy

`glyphon` should own:

- text shaping
- font fallback resolution
- glyph raster caching
- atlas management
- text draw submission inside the render pass

The app should continue to own:

- all gameplay and lobby layout
- all final cell-grid content
- the `PlayfieldBuffer` output contract

## Rendering Contract

The long-term presentation input for `nc-dash` should be:

- `PlayfieldBuffer`
- cursor metadata
- optional renderer-side presentation metadata if needed later

`RenderedUi` should be treated as transitional compatibility glue for the
current ratatui backend path, not as the future architecture boundary.

## Success Criteria

The replacement renderer is successful when it can render `nc-dash`'s current
UI correctly without the vendored `ratatui-wgpu` backend and without moving the
renderer into `nc-ui`.

The renderer must later prove:

- correct Greek and other non-ASCII glyph rendering used by `nc-dash`
- correct box-drawing and block/shade presentation
- no glyph bleed into adjacent cells
- stable fallback behavior across the chosen embedded fonts
- correct cursor/highlight presentation
- correct resize and centered pixel-to-cell hit testing
- no visible regression in lobby or dashboard layout fidelity

## Non-Goals

- no renderer extraction to a shared crate in v1
- no public generic ratatui backend crate in v1
- no `nc-ui` renderer rewrite
- no `nc-game` / `nc-door` renderer changes
- no visual redesign of `nc-dash`
