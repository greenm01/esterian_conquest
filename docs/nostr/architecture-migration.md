# Nostrian Conquest Architecture & Nostr Migration Path

This document outlines the current state of the Nostrian Conquest (NC) architecture, compares it to the `ec4x` transport model, and defines the roadmap for migrating **nc-dash** to a fully decentralized Nostr transport while retaining **nc-connect** and **nc-game** on the classic SSH/PTY stack.

## 1. Current State: Nostr-Authenticated SSH

Currently, NC uses Nostr for **identity and session establishment**, but relies on **SSH for terminal transport** for all game modes.

### Block Diagram (Current)

```text
+--------------------------------------------------------------+
|                          VPS Host                            |
|--------------------------------------------------------------|
|  +-----------+      +------------+     +------------------+  |
|  |  nc-gate  |      | sshd (PTY) | --> | nc-game (TUI)    |  |
|  | (daemon)  |      +------------+     | (1 per player)   |  |
|  +-----------+            ^            +------------------+  |
|        |                  |                     |            |
|        v                  |                     v            |
|   (provisions SSH key)    |                ncgame.db         |
+--------^------------------|----------------------------------+
         | Nostr (Auth)     | SSH (Transport)
         v                  v
    +---------+       +------------+
    |  Relay  | <---- | nc-connect | (Player Local)
    +---------+       +------------+
```

---

## 2. The Target State: Hybrid Transport (Local vs Remote)

In the target architecture, **nc-dash** is a native local client using full
Nostr transport (state sync + async orders). It preserves the existing
terminal-style dashboard look, but it no longer runs inside the SSH/PTY event
loop. Instead it uses a native on-demand cell-grid renderer and a TEA-style
window event loop. **nc-connect** remains the dedicated utility for launching
**nc-game** via the classic SSH/PTY stack.

### Block Diagram (Target Hybrid)

```text
+--------------------------------------------------------------+
|                          VPS Host                            |
|--------------------------------------------------------------|
|  +-----------+      +------------+     +------------------+  |
|  |  nc-gate  |      | sshd (PTY) | --> | nc-game (Classic)|  |
|  | (daemon)  |      +------------+     | (VPS Intensive)  |  |
|  +-----------+            ^            +---------+--------+  |
|        |                  |                      |           |
|        v                  +----------------------+           |
|   (SSH Provisioning       | SSH (Classic Path)   |           |
|    for nc-game only)      |                      v           |
|                           |                  ncgame.db       |
|                           |                      ^           |
|                           |      (Resolved by    |           |
|                           |       nc-engine)     |           |
+--------^------------------|----------------------+-----------+
         |                  |                      |
         | Nostr (Auth)     |                      | Nostr (State/Orders)
         v                  v                      v
    +---------+       +------------+         +------------+
    |  Relay  | <---- | nc-connect |         |  nc-dash   |
    +---------+       |  (Classic  |         |  (Modern   |
                      |   Bridge)  |         |   Path)    |
                      +------------+         +------------+
                             (Player 1 Local)
```

### Path A: nc-dash (Modern / Low Resource)
1. **Local Execution:** The client runs entirely on the player's machine in a
   native window, reading from a local SQLite cache.
2. **Command Submission:** Players submit turns via Nostr event `30522 TurnCommands`.
3. **Turn Resolution:** The VPS resolver (`nc-engine`) stages orders and computes deltas.
4. **State Sync:** VPS publishes deltas (`30521 StateDelta`) to update the local client.
5. **VPS Impact:** Zero persistent footprint. Minimal CPU usage during atomic resolution.

### Path B: nc-connect -> nc-game (Classic / High Resource)
1. **Remote Execution:** The client remains as-is, running on the VPS in a PTY.
2. **Transport:** Uses the existing SSH bridge provided by `nc-connect`.
3. **State:** Reads and writes directly to the VPS-side `ncgame.db`.
4. **VPS Impact:** Persistent CPU/Memory for every active session. Retained for fidelity and players preferring the classic SSH experience.

---

## 3. Client Roles & Consolidation

While `nc-connect` persists as the primary bridge for the SSH stack, its core logic will be "folded into" the broader `nc-dash` codebase to ensure a single source of truth for identity and discovery.

- **nc-dash:** The primary modern client. Manages the Nostr wallet, provides
  the game picker, and renders the dashboard locally in a native window using
  Nostr state sync.
- **nc-connect:** Maintained as the specialized bridge utility for the classic `nc-game` stack. Shares the same underlying identity (wallet) and discovery logic as `nc-dash`.

---

## 4. Migration Roadmap

### Phase 1: Expand Local State Sync
*Status: Partially implemented (Static Maps).*
Expand `nc-gate` to push the full, fog-of-war filtered `PlayerState` via Nostr.
- `nc-dash` receives these events and builds a robust local SQLite cache.

### Phase 2: Fold nc-connect Logic into nc-dash
- Reorganize the workspace so `nc-dash` and `nc-connect` share the same library for wallet management, Nostr handshake, and SSH provisioning.
- Both clients use `~/.local/share/nc/wallet.kdl` as the shared identity source.
- Share the native cell-grid renderer/input layer out of `nc-ui` so both
  clients use one `PlayfieldBuffer`-driven window stack.

### Native Client Event Model

The `nc-dash` local client should follow this shape:

```text
WindowEvent -> DashMsg -> update(Model, Msg) -> Effects -> redraw if dirty
```

This is intentionally TEA-like rather than SAM-like. It keeps Rust-side input
handling explicit, makes hover coalescing straightforward, and fits the future
path where async Nostr/database completions re-enter the client as typed
messages.

### Phase 3: Nostr Command Submission (nc-dash only)
- `nc-dash` gains the ability to build and submit `turn.kdl` locally.
- Implement event `30522 TurnCommands`.
- `nc-gate` subscribes to `30522` and stages orders in the server's `ncgame.db`.

### Phase 4: Server-Side Resolution Orchestration
- Upgrade `nc-gate` to orchestrate turns using `nc-engine`.
- `nc-gate` computes deltas and publishes `30521 StateDelta`.
- Ensure `nc-game` (launched via `nc-connect`) can still operate on the same `ncgame.db` without conflicting with the new async pipeline.

---

## 5. Required Protocol Additions (305xx Range)

| Proposed Kind | Name | Purpose |
|---------------|------|---------|
| `30520` | `GameState` | Full player-filtered state snapshot (Initial sync). |
| `30521` | `StateDelta` | Turn-by-turn state changes (Bandwidth efficiency). |
| `30522` | `TurnCommands` | Player's submitted `turn.kdl` orders. |
| `30523` | `PlayerMessage` | NIP-44 encrypted player-to-player in-game diplomacy. |
