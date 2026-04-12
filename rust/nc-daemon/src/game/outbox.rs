#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxJob {
    pub game_id: String,
    pub kind: String,
}
