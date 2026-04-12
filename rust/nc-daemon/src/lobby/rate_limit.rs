#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteRateLimitKey {
    pub game_id: String,
    pub player_npub: String,
}
