#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedGameRow {
    pub game_id: String,
    pub status: String,
    pub game: String,
    pub host: String,
    pub host_contact_npub: Option<String>,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub seat: Option<u8>,
    pub turn_summary: String,
    pub invite_address: Option<String>,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
}

impl JoinedGameRow {
    #[allow(clippy::too_many_arguments)]
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
            host_contact_npub: None,
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
    pub status: String,
    pub game: String,
    pub host: String,
    pub host_contact_npub: Option<String>,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub recruiting: String,
    pub open_seats: u8,
    pub total_seats: u8,
    pub created_date: String,
    pub turn_summary: String,
    pub summary: String,
}

impl OpenGameRow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        game_id: &str,
        status: &str,
        game: &str,
        host: &str,
        relay_url: &str,
        daemon_pubkey: &str,
        recruiting: &str,
        open_seats: u8,
        total_seats: u8,
        created_date: &str,
        turn_summary: &str,
        summary: &str,
    ) -> Self {
        Self {
            game_id: game_id.to_string(),
            status: status.to_string(),
            game: game.to_string(),
            host: host.to_string(),
            host_contact_npub: None,
            relay_url: relay_url.to_string(),
            daemon_pubkey: daemon_pubkey.to_string(),
            recruiting: recruiting.to_string(),
            open_seats,
            total_seats,
            created_date: created_date.to_string(),
            turn_summary: turn_summary.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInboxRow {
    pub game_id: String,
    pub game: String,
    pub other_empire_id: u8,
    pub other_empire_name: String,
    pub preview: String,
    pub updated_at: String,
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
pub struct DirectContactRow {
    pub npub: String,
    pub label: String,
    pub nip05: Option<String>,
    pub source: String,
    pub blocked: bool,
    pub hidden: bool,
    pub unread_count: u32,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadMessage {
    pub message_id: String,
    pub contact_npub: String,
    pub sender: String,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}

impl ThreadMessage {
    pub fn incoming(contact_npub: &str, sender: &str, body: &str) -> Self {
        Self {
            message_id: format!("thread-in-{contact_npub}"),
            contact_npub: contact_npub.to_string(),
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: false,
            created_at: String::new(),
        }
    }

    pub fn outgoing(contact_npub: &str, sender: &str, body: &str) -> Self {
        Self {
            message_id: format!("thread-out-{contact_npub}"),
            contact_npub: contact_npub.to_string(),
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: true,
            created_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInboxMessage {
    pub message_id: String,
    pub game_id: String,
    pub game: String,
    pub other_empire_id: u8,
    pub other_empire_name: String,
    pub sender: String,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}
