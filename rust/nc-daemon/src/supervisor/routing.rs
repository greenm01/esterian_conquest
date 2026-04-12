#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutedDaemonRequest {
    InviteRequest { game_id: String },
    StateRequest { game_id: String },
    TurnCommands { game_id: String },
}
