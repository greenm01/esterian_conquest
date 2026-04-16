# nc-dash Wayland Disconnect Investigation

Date: 2026-04-16

## Problem

`nc-dash` crashes on Wayland during normal pointer movement in the hosted-game
starmap flow. The native fatal error is:

```text
nc-dash native event loop failed ... native event loop disconnected before a
clean exit; likely Wayland compositor/protocol disconnect inside winit while
flushing or dispatching events
```

Observed low-level errors:

- `Io error: Broken pipe (os error 32)`
- `Io error: Connection reset by peer (os error 104)`

## What We Observed

### App-side diagnostic logs

The app-side `nc_dash::native` logs consistently narrowed the failure window to:

1. many `queue_pointer` dispatches
2. no app-state mutation in the final dispatches
3. `native idle no-op`
4. `winit` Wayland event-loop dispatch error
5. `wgpu` teardown

Representative sequence:

```text
native idle no-op
calloop WaylandSource processing
Error dispatching event loop: other error during loop operation
wgpu resource drops...
```

### Wayland protocol trace

`WAYLAND_DEBUG=1` did not show a clean protocol rejection such as
`wl_display.error(...)`. The final visible traffic is plain inbound pointer
motion:

- `wl_pointer.frame()`
- `wl_pointer.motion(...)`

and then the socket dies abruptly.

That means the client does not see a normal compositor protocol error before
disconnect.

### Journal checks

Both:

- `journalctl --user --since ... --until ...`
- `journalctl --since ... --until ...`

showed no compositor crash, protocol, or Wayland diagnostics around the failure
window.

## What We Ruled Out

### 1. Passive hover state mutation in `DashApp`

We added diagnostics and then suppressed Wayland passive hover dispatch in the
native shell. That changed the symptom shape, but did not eliminate the crash.

Result:

- not explained by the hosted hover/crosshair mutation path alone

### 2. `RenderedUi` / generic first-frame rendering

Existing native repro examples showed that simpler `winit + ratatui-wgpu`
window paths can render and redraw without crashing.

Result:

- not a generic first-frame or generic renderer initialization failure

### 3. Wayland relative-pointer binding

We vendored `winit` and disabled Wayland relative-pointer binding at
initialization:

- [rust/Cargo.toml](../../../rust/Cargo.toml)
- [rust/vendor/winit-0.29.15/src/platform_impl/linux/wayland/state.rs](../../../rust/vendor/winit-0.29.15/src/platform_impl/linux/wayland/state.rs)

After that change, `WAYLAND_DEBUG=1` no longer showed
`zwp_relative_pointer_v1` traffic, but the crash still occurred.

Result:

- not caused solely by `winit`'s relative-pointer path

### 4. Wayland client-side decorations / frame handling

We forced `nc-dash` windows to be undecorated on Wayland in:

- [rust/nc-dash/src/native.rs](../../../rust/nc-dash/src/native.rs)

The crash still occurred.

Result:

- not caused solely by `winit` Wayland CSD/decorations

### 5. Plain `CursorMoved + redraw` at the minimal window level

We added a minimal example:

- [rust/nc-dash/examples/cursor_motion_native_repro.rs](../../../rust/nc-dash/examples/cursor_motion_native_repro.rs)

This example does only:

- `WindowEvent::CursorMoved`
- `window.request_redraw()`
- `ratatui-wgpu` draw

and does **not** crash under Wayland.

Result:

- the remaining bug is above the bare `CursorMoved + redraw` path
- likely in `nc-dash`'s native loop/app-specific cadence or state path rather
  than raw pointer motion alone

## Current Best Read

The remaining crash is most likely in one of these buckets:

1. `nc-dash` native-loop behavior that still differs from the minimal repro
2. a `winit` Wayland backend edge case only triggered by the fuller `nc-dash`
   event/render cadence
3. a compositor-side abort path that does not surface a normal
   `wl_display.error(...)` to the client

What is no longer the primary suspect:

- `DashApp` passive hover mutation by itself
- raw relative-pointer traffic
- Wayland decorations/CSD
- plain `CursorMoved + redraw` in a minimal `winit + ratatui-wgpu` window

## Files Added or Changed During Investigation

### Native `nc-dash` changes

- [rust/nc-dash/src/native.rs](../../../rust/nc-dash/src/native.rs)
  - added pointer/idle/render diagnostics
  - suppressed passive Wayland hover dispatch
  - forced undecorated Wayland windows for experiment

### Tests

- `rust/nc-dash/src/native.rs` test coverage was extended around:
  - passive hover suppression
  - pointer preservation for subsequent clicks
  - no queued flush on suppressed hover paths

### Repro examples

- [rust/nc-dash/examples/cursor_motion_native_repro.rs](../../../rust/nc-dash/examples/cursor_motion_native_repro.rs)

### Vendored dependency experiment

- [rust/Cargo.toml](../../../rust/Cargo.toml)
- [rust/vendor/winit-0.29.15/src/platform_impl/linux/wayland/state.rs](../../../rust/vendor/winit-0.29.15/src/platform_impl/linux/wayland/state.rs)

## Repro Commands

### Full `nc-dash` Wayland repro

```bash
cargo run -p nc-dash -- --diagnostic --backend wayland --relay ws://127.0.0.1:8080
```

### Full Wayland trace

```bash
env WAYLAND_DEBUG=1 RUST_LOG=nc_dash::native=info,winit=trace,calloop=trace cargo run -p nc-dash -- --diagnostic --backend wayland --relay ws://127.0.0.1:8080 2> /tmp/nc-dash-wayland.log
```

### Minimal motion repro

```bash
cargo run -p nc-dash --example cursor_motion_native_repro -- --backend wayland
```

## Recommended Next Step

Compare the native-loop behavior in:

- [rust/nc-dash/src/native.rs](../../../rust/nc-dash/src/native.rs)
- [rust/nc-dash/examples/cursor_motion_native_repro.rs](../../../rust/nc-dash/examples/cursor_motion_native_repro.rs)

and reduce `nc-dash` further toward that minimal path until the crash flips from
"present" to "absent." The most useful next discriminator is likely a new repro
that adds back only one `nc-dash`-specific ingredient at a time:

1. `DashApp` rendering without hosted snapshot logic
2. `QueuePointer`/`FlushPointer` behavior
3. hosted snapshot rendering with real window motion
4. any remaining window-state or redraw scheduling differences
