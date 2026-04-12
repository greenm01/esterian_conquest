use nc_nostr::state_sync::GameState;
use nc_ui::ScreenGeometry;

use crate::startup::LobbyStartupOptions;

use super::models::{InboxItem, JoinedGameRow, LobbyNotice, OpenGameRow, ThreadMessage};
use super::transport::LobbyTransport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyRoute {
    FirstRun,
    Locked,
    Home,
    ComposeInvite,
    ComposeThread,
    EditHandle,
    HostedGame,
    SubmitTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunField {
    Handle,
    Password,
    Confirm,
}

impl FirstRunField {
    pub fn next(self) -> Self {
        match self {
            Self::Handle => Self::Password,
            Self::Password => Self::Confirm,
            Self::Confirm => Self::Handle,
        }
    }
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
pub struct HostedGameView {
    pub row: JoinedGameRow,
    pub snapshot: GameState,
    pub submit_input: String,
    pub submit_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LobbyState {
    pub route: LobbyRoute,
    pub focus: LobbyFocus,
    pub relay_override: Option<String>,
    pub relay_label: Option<String>,
    pub player_handle: Option<String>,
    pub joined_games: Vec<JoinedGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub inbox: Vec<InboxItem>,
    pub notices: Vec<LobbyNotice>,
    pub thread_messages: Vec<ThreadMessage>,
    pub joined_selected: usize,
    pub open_selected: usize,
    pub inbox_selected: usize,
    pub notices_selected: usize,
    pub thread_selected: usize,
    pub status_message: Option<String>,
    pub first_run_field: FirstRunField,
    pub first_run_handle_input: String,
    pub first_run_password_input: String,
    pub first_run_confirm_input: String,
    pub unlock_password_input: String,
    pub compose_message_input: String,
    pub edit_handle_input: String,
    pub hosted_game: Option<HostedGameView>,
}

impl LobbyState {
    pub fn new(options: LobbyStartupOptions, route: LobbyRoute) -> Self {
        Self {
            route,
            focus: LobbyFocus::OpenGames,
            relay_override: options.relay_override.clone(),
            relay_label: options.relay_override.map(|relay| format!("relay: {relay}")),
            player_handle: None,
            joined_games: Vec::new(),
            open_games: Vec::new(),
            inbox: Vec::new(),
            notices: vec![LobbyNotice::new(
                "nc-host",
                "Public notices and private sysop threads land in phase 2.",
            )],
            thread_messages: vec![ThreadMessage::incoming(
                "lobby",
                "nc-host",
                "Thread messaging is not wired yet. Use invite requests from the lobby first.",
            )],
            joined_selected: 0,
            open_selected: 0,
            inbox_selected: 0,
            notices_selected: 0,
            thread_selected: 0,
            status_message: None,
            first_run_field: FirstRunField::Handle,
            first_run_handle_input: String::new(),
            first_run_password_input: String::new(),
            first_run_confirm_input: String::new(),
            unlock_password_input: String::new(),
            compose_message_input: String::new(),
            edit_handle_input: String::new(),
            hosted_game: None,
        }
    }

    pub fn apply_loaded(&mut self, loaded: super::transport::LobbyLoadedState) {
        self.relay_label = loaded.relay_label;
        self.player_handle = loaded.player_handle;
        self.joined_games = loaded.joined_games;
        self.open_games = loaded.open_games;
        self.inbox = loaded.inbox;
        self.notices = loaded.notices;
        self.thread_messages = loaded.thread_messages;
        self.status_message = loaded.status_message;
        self.joined_selected = self
            .joined_selected
            .min(self.joined_games.len().saturating_sub(1));
        self.open_selected = self.open_selected.min(self.open_games.len().saturating_sub(1));
        self.inbox_selected = self.inbox_selected.min(self.inbox.len().saturating_sub(1));
        self.notices_selected = self.notices_selected.min(self.notices.len().saturating_sub(1));
        self.thread_selected = self
            .thread_selected
            .min(self.visible_thread_messages().len().saturating_sub(1));
        self.edit_handle_input = self.player_handle.clone().unwrap_or_default();
    }

    pub fn relay_label(&self) -> Option<String> {
        self.relay_label.clone()
    }

    pub fn player_handle_label(&self) -> Option<String> {
        self.player_handle
            .as_ref()
            .map(|handle| format!("handle: {handle}"))
    }

    pub fn selected_open_game(&self) -> Option<&OpenGameRow> {
        self.open_games.get(self.open_selected)
    }

    pub fn selected_joined_game(&self) -> Option<&JoinedGameRow> {
        self.joined_games.get(self.joined_selected)
    }

    pub fn thread_context_game_id(&self) -> Option<&str> {
        match self.focus {
            LobbyFocus::JoinedGames => self.selected_joined_game().map(|row| row.game_id.as_str()),
            LobbyFocus::Inbox => self.inbox.get(self.inbox_selected).map(|row| row.game_id.as_str()),
            LobbyFocus::OpenGames => self.selected_open_game().map(|row| row.game_id.as_str()),
            _ => self
                .selected_joined_game()
                .map(|row| row.game_id.as_str())
                .or_else(|| self.selected_open_game().map(|row| row.game_id.as_str())),
        }
    }

    pub fn thread_context_display(&self) -> String {
        self.thread_context_game_id()
            .map(str::to_string)
            .unwrap_or_else(|| "no game selected".to_string())
    }

    pub fn visible_thread_messages(&self) -> Vec<&ThreadMessage> {
        let Some(game_id) = self.thread_context_game_id() else {
            return Vec::new();
        };
        self.thread_messages
            .iter()
            .filter(|message| message.game_id == game_id)
            .collect()
    }
}

pub struct LobbyApp {
    pub geometry: ScreenGeometry,
    pub state: LobbyState,
    pub transport: LobbyTransport,
    pub should_quit: bool,
}
