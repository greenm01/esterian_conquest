#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedGameRow {
    pub status: String,
    pub game: String,
    pub host: String,
    pub seat: Option<u8>,
    pub turn_summary: String,
}

impl JoinedGameRow {
    pub fn new(
        status: &str,
        game: &str,
        host: &str,
        seat: Option<u8>,
        turn_summary: &str,
    ) -> Self {
        Self {
            status: status.to_string(),
            game: game.to_string(),
            host: host.to_string(),
            seat,
            turn_summary: turn_summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGameRow {
    pub game: String,
    pub host: String,
    pub recruiting: String,
    pub open_seats: u8,
    pub turn_summary: String,
    pub summary: String,
}

impl OpenGameRow {
    pub fn new(
        game: &str,
        host: &str,
        recruiting: &str,
        open_seats: u8,
        turn_summary: &str,
        summary: &str,
    ) -> Self {
        Self {
            game: game.to_string(),
            host: host.to_string(),
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
    pub game: String,
    pub status: String,
}

impl InboxItem {
    pub fn new(kind: &str, game: &str, status: &str) -> Self {
        Self {
            kind: kind.to_string(),
            game: game.to_string(),
            status: status.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequestRow {
    pub game: String,
    pub status: String,
}

impl PendingRequestRow {
    pub fn new(game: &str, status: &str) -> Self {
        Self {
            game: game.to_string(),
            status: status.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyNotice {
    pub sender: String,
    pub body: String,
}

impl LobbyNotice {
    pub fn new(sender: &str, body: &str) -> Self {
        Self {
            sender: sender.to_string(),
            body: body.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadMessage {
    pub sender: String,
    pub body: String,
    pub outgoing: bool,
}

impl ThreadMessage {
    pub fn incoming(sender: &str, body: &str) -> Self {
        Self {
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: false,
        }
    }

    pub fn outgoing(sender: &str, body: &str) -> Self {
        Self {
            sender: sender.to_string(),
            body: body.to_string(),
            outgoing: true,
        }
    }
}
