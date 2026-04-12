#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameMsg {
    Tick,
    PublishLobbyCatalog,
    ProcessInviteRequest { request_id: String },
    ProcessTurnSubmission { submit_id: String },
}
