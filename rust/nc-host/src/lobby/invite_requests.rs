#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteRequestRecord {
    pub request_id: String,
    pub game_id: String,
    pub player_npub: String,
    pub message: String,
}
