#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledGame {
    pub game_id: String,
    pub due_unix_seconds: u64,
}
