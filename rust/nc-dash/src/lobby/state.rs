use nc_ui::ScreenGeometry;

use crate::startup::LobbyStartupOptions;

use super::models::{
    InboxItem, JoinedGameRow, LobbyNotice, OpenGameRow, PendingRequestRow, ThreadMessage,
};
use super::transport::NoopLobbyTransport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyRoute {
    FirstRun,
    Locked,
    Home,
    ComposeInvite,
    EditHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyFocus {
    JoinedGames,
    Inbox,
    OpenGames,
    Notices,
    Thread,
}

impl LobbyFocus {
    pub fn next(self) -> Self {
        match self {
            Self::JoinedGames => Self::Inbox,
            Self::Inbox => Self::OpenGames,
            Self::OpenGames => Self::Notices,
            Self::Notices => Self::Thread,
            Self::Thread => Self::JoinedGames,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::JoinedGames => Self::Thread,
            Self::Inbox => Self::JoinedGames,
            Self::OpenGames => Self::Inbox,
            Self::Notices => Self::OpenGames,
            Self::Thread => Self::Notices,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LobbyState {
    pub route: LobbyRoute,
    pub focus: LobbyFocus,
    pub relay_override: Option<String>,
    pub player_handle: Option<String>,
    pub joined_games: Vec<JoinedGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub inbox: Vec<InboxItem>,
    pub pending_requests: Vec<PendingRequestRow>,
    pub notices: Vec<LobbyNotice>,
    pub thread_messages: Vec<ThreadMessage>,
    pub joined_selected: usize,
    pub open_selected: usize,
    pub inbox_selected: usize,
    pub notices_selected: usize,
    pub thread_selected: usize,
    pub status_message: Option<String>,
}

impl LobbyState {
    pub fn with_placeholder_data(options: LobbyStartupOptions, route: LobbyRoute) -> Self {
        Self {
            route,
            focus: LobbyFocus::OpenGames,
            relay_override: options.relay_override,
            player_handle: Some("StarRider".to_string()),
            joined_games: vec![
                JoinedGameRow::new("joined", "Friday Night NC", "Green Host", Some(2), "Y3012 T12"),
                JoinedGameRow::new("approved", "Sunday Replacement", "Green Host", None, "Awaiting claim"),
            ],
            open_games: vec![
                OpenGameRow::new(
                    "Friday Night NC",
                    "Green Host",
                    "replacement",
                    1,
                    "Y3012 T12",
                    "Veteran game seeking one replacement admiral.",
                ),
                OpenGameRow::new(
                    "Nebula Sprint",
                    "North Relay",
                    "new",
                    3,
                    "Y3001 T02",
                    "Fresh 4-player sprint with open seats.",
                ),
            ],
            inbox: vec![
                InboxItem::new("request", "Friday Night NC", "received"),
                InboxItem::new("approval", "Sunday Replacement", "invite ready"),
            ],
            pending_requests: vec![PendingRequestRow::new("Friday Night NC", "received")],
            notices: vec![
                LobbyNotice::new("Green Host", "Friday Night NC needs one replacement."),
                LobbyNotice::new("Green Host", "Relay maintenance window tonight at 22:00."),
            ],
            thread_messages: vec![
                ThreadMessage::incoming("Green Host", "Seat 4 is still open if you want it."),
                ThreadMessage::outgoing("StarRider", "I can usually play evenings EST."),
            ],
            joined_selected: 0,
            open_selected: 0,
            inbox_selected: 0,
            notices_selected: 0,
            thread_selected: 0,
            status_message: Some("Lobby scaffold active — transport is stubbed.".to_string()),
        }
    }

    pub fn relay_label(&self) -> Option<String> {
        self.relay_override
            .as_ref()
            .map(|relay| format!("relay: {relay}"))
    }

    pub fn player_handle_label(&self) -> Option<String> {
        self.player_handle
            .as_ref()
            .map(|handle| format!("handle: {handle}"))
    }
}

pub struct LobbyApp {
    pub geometry: ScreenGeometry,
    pub state: LobbyState,
    pub transport: NoopLobbyTransport,
    pub should_quit: bool,
}
