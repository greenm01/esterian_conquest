use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatusReport {
    pub generated_at: u64,
    pub config_path: Option<String>,
    pub games_root: String,
    pub relay: RelayStatusReport,
    pub totals: DaemonStatusTotals,
    pub games: Vec<GameStatusRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelayStatusReport {
    pub url: String,
    pub configured: bool,
    pub reachable: bool,
    pub status: String,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DaemonStatusTotals {
    pub discovered_games: u32,
    pub public_recruiting_games: u32,
    pub due_maintenance_games: u32,
    pub pending_requests: u32,
    pub pending_decisions: u32,
    pub pending_turns: u32,
    pub outbox_pending: u32,
    pub outbox_failed: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameStatusRow {
    pub game_id: String,
    pub dir: String,
    pub name: String,
    pub status: String,
    pub year: u32,
    pub turn: u32,
    pub players: u32,
    pub claimed_seats: u32,
    pub open_seats: u32,
    pub recruiting: String,
    pub lobby_visibility: String,
    pub catalog_state: String,
    pub maintenance_enabled: bool,
    pub maintenance_due_unix_seconds: Option<i64>,
    pub maintenance_due_now: bool,
    pub pending_requests: u32,
    pub pending_decisions: u32,
    pub pending_turns: u32,
    pub outbox_pending: u32,
    pub outbox_failed: u32,
}
