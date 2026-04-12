use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn normalize_game_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

static OUTBOX_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_outbox_id(prefix: &str, game_id: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = OUTBOX_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{game_id}-{nanos:x}-{counter:x}")
}
