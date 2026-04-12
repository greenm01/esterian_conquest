#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedLobbyGame {
    pub game_id: String,
    pub recruiting: String,
    pub open_seats: u8,
}
