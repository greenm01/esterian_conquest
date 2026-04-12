use crate::game::effects::GameEffects;

#[derive(Debug, Clone)]
pub enum GameMsg {
    Tick,
    PublishLobbyCatalog,
    ProcessInviteRequest { request_id: String },
    ProcessTurnSubmission { submit_id: String },
    HandleEffect(GameEffects),
}

pub struct RoutedGame {
    pub game_id: String,
    pub db_path: std::path::PathBuf,
}
