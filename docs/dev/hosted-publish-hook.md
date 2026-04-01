# Hosted Publish Hook

Hosted first joins now use a durable host-side publish hook instead of tying
outbound Nostr work to clean SSH session exit.

## Current Implementation

- `nc-game` still owns the gameplay-side first-time join flow.
- When the player saves the empire name and the hosted seat claim commits,
  `nc-game` writes a durable publish job into `ncgame.db` in the same
  transaction.
- `nc-gate serve` polls that queue and performs the actual Nostr publish.

Current implemented job kind:

- `MapPackOnFirstClaim`
  publishes 30512 `MapPush` with the same encrypted payload shape as 30505
  `MapBundle`

This split is intentional. `nc-game` does not currently own relay config,
gate keys, or a direct Nostr publishing client, so the durable DB queue is the
boundary between gameplay state mutation and host-side Nostr side effects.

## Why This Exists

- The hosted seat claim happens immediately when the empire name is saved.
- A player can be disconnected after claim but before cleanly leaving
  `nc-game`.
- Map delivery for turn 1 should not depend on returning neatly through the old
  post-session `nc-connect` finalizer path.

The durable queue gives us an at-least-once host-side publish seam that
survives early disconnects.

## Reserved Future Use

This hook is also the planned seam for parallel outbound Nostr work triggered
from gameplay events.

Documented but not implemented yet:

- fan-out of in-game messages as parallel Nostr messages

That future work should add a new explicit job kind to the same durable queue
rather than teaching `nc-game` to publish directly.
