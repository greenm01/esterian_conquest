#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSubmissionEnvelope {
    pub submit_id: String,
    pub game_id: String,
    pub turn: u32,
}
