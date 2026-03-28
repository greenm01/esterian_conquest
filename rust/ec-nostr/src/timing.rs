use std::time::{SystemTime, UNIX_EPOCH};

use nostr_sdk::Event;

/// Maximum age of a Nostr request event before it is rejected (replay prevention).
pub const MAX_EVENT_AGE_SECS: u64 = 60;

/// Return true if the event's `created_at` is older than [`MAX_EVENT_AGE_SECS`].
pub fn is_event_stale(event: &Event) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(event.created_at.as_secs()) > MAX_EVENT_AGE_SECS
}
