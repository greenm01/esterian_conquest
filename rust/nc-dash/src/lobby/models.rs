#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedGameRow {
    pub game_id: String,
    pub status: String,
    pub game: String,
    pub host: String,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub seat: Option<u8>,
    pub turn_summary: String,
    pub invite_address: Option<String>,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
}

impl JoinedGameRow {
    pub fn new(
        game_id: &str,
        status: &str,
        game: &str,
        host: &str,
        relay_url: &str,
        daemon_pubkey: &str,
        seat: Option<u8>,
        turn_summary: &str,
    ) -> Self {
        Self {
            game_id: game_id.to_string(),
            status: status.to_string(),
            game: game.to_string(),
            host: host.to_string(),
            relay_url: relay_url.to_string(),
            daemon_pubkey: daemon_pubkey.to_string(),
            seat,
            turn_summary: turn_summary.to_string(),
            invite_address: None,
            last_turn: None,
            last_hash: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGameRow {
    pub game_id: String,
    pub game: String,
    pub host: String,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub recruiting: String,
    pub open_seats: u8,
    pub turn_summary: String,
    pub summary: String,
}

impl OpenGameRow {
    pub fn new(
        game_id: &str,
        game: &str,
        host: &str,
        relay_url: &str,
        daemon_pubkey: &str,
        recruiting: &str,
        open_seats: u8,
        turn_summary: &str,
        summary: &str,
    ) -> Self {
        Self {
            game_id: game_id.to_string(),
            game: game.to_string(),
            host: host.to_string(),
            relay_url: relay_url.to_string(),
            daemon_pubkey: daemon_pubkey.to_string(),
            recruiting: recruiting.to_string(),
            open_seats,
            turn_summary: turn_summary.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxItem {
    pub kind: String,
    pub request_id: Option<String>,
    pub game_id: String,
    pub game: String,
    pub status: String,
    pub message: String,
    pub invite_address: Option<String>,
}

impl InboxItem {
    pub fn new(
        kind: &str,
        request_id: Option<String>,
        game_id: &str,
        game: &str,
        status: &str,
        message: &str,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            request_id,
            game_id: game_id.to_string(),
            game: game.to_string(),
            status: status.to_string(),
            message: message.to_string(),
            invite_address: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyNotice {
    pub notice_id: String,
    pub sender: String,
    pub body: String,
    pub created_at: String,
}

impl LobbyNotice {
    pub fn new(sender: &str, body: &str) -> Self {
        Self {
            notice_id: format!("notice-{sender}"),
            sender: sender.to_string(),
            body: body.to_string(),
            created_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadMessage {
    pub message_id: String,
    pub game_id: String,
    pub sender: String,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}

impl ThreadMessage {
    pub fn incoming(game_id: &str, sender: &str, body: &str) -> Self {
        Self {
            message_id: format!("thread-in-{game_id}"),
            game_id: game_id.to_string(),
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: false,
            created_at: String::new(),
        }
    }

    pub fn outgoing(game_id: &str, sender: &str, body: &str) -> Self {
        Self {
            message_id: format!("thread-out-{game_id}"),
            game_id: game_id.to_string(),
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: true,
            created_at: String::new(),
        }
    }
}
