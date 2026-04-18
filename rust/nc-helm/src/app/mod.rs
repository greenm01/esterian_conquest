mod chrome;
mod update;
mod view;

use crate::input::{KeyCode, KeyEvent, MouseEvent};
use crate::storage::{BootSnapshot, StoredSession};
use crate::transport::LobbySnapshot;
use crate::{GameColor, PlayfieldBuffer, Point, ScreenGeometry};

pub const DEFAULT_RELAY_URL: &str = "ws://127.0.0.1:8080";
pub const DEFAULT_GEOMETRY: ScreenGeometry = ScreenGeometry::new(100, 36);

#[derive(Debug, Clone)]
pub struct App {
    model: Model,
}

impl App {
    pub fn new(relay_override: Option<String>) -> (Self, Vec<Effect>) {
        let relay_overridden = relay_override.is_some();
        let model = Model {
            geometry: DEFAULT_GEOMETRY,
            relay_url: relay_override.unwrap_or_else(|| DEFAULT_RELAY_URL.to_string()),
            relay_overridden,
            route: Route::Boot(BootModel {
                status: "Loading local client state...".to_string(),
            }),
            session: None,
            network: NetworkState::Idle,
            games: Vec::new(),
            notices: Vec::new(),
            window_focused: false,
            should_quit: false,
        };
        (Self { model }, vec![Effect::LoadBoot])
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn dispatch(&mut self, msg: Msg) -> Vec<Effect> {
        update::update(&mut self.model, msg)
    }

    pub fn view(&self) -> PlayfieldBuffer {
        view::render(&self.model)
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub geometry: ScreenGeometry,
    pub relay_url: String,
    pub relay_overridden: bool,
    pub route: Route,
    pub session: Option<SessionState>,
    pub network: NetworkState,
    pub games: Vec<GameRow>,
    pub notices: Vec<String>,
    pub window_focused: bool,
    pub should_quit: bool,
}

impl Model {
    pub fn wants_text_input(&self) -> bool {
        matches!(self.route, Route::FirstRun(_) | Route::Locked(_))
    }

    pub fn wants_window_focus(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub enum Route {
    Boot(BootModel),
    FirstRun(FirstRunModel),
    Locked(LockedModel),
    Lobby(LobbyModel),
    FatalError(String),
}

#[derive(Debug, Clone)]
pub struct BootModel {
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunField {
    Handle,
    Password,
    Confirm,
    Relay,
}

impl FirstRunField {
    fn next(self) -> Self {
        match self {
            Self::Handle => Self::Password,
            Self::Password => Self::Confirm,
            Self::Confirm => Self::Relay,
            Self::Relay => Self::Handle,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Handle => Self::Relay,
            Self::Password => Self::Handle,
            Self::Confirm => Self::Password,
            Self::Relay => Self::Confirm,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirstRunModel {
    pub active_field: FirstRunField,
    pub handle_input: String,
    pub password_input: String,
    pub confirm_input: String,
    pub relay_input: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LockedModel {
    pub password_input: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyTab {
    Home,
    OpenGames,
    Comms,
    Settings,
}

impl LobbyTab {
    fn next(self) -> Self {
        match self {
            Self::Home => Self::OpenGames,
            Self::OpenGames => Self::Comms,
            Self::Comms => Self::Settings,
            Self::Settings => Self::Home,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LobbyModel {
    pub active_tab: LobbyTab,
    pub help_open: bool,
    pub selected_game: usize,
    pub editing_relay: bool,
    pub relay_draft: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub password: String,
    pub active_npub: String,
    pub active_nsec: String,
    pub active_handle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    Idle,
    Connecting,
    Synced,
    Error,
}

#[derive(Debug, Clone)]
pub struct GameRow {
    pub game_id: String,
    pub name: String,
    pub host: String,
    pub tier: String,
    pub seats: String,
    pub when: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Resize(ScreenGeometry),
    FocusChanged(bool),
    Key(KeyEvent),
    TextInput(String),
    Mouse(MouseEvent),
    BootLoaded(Result<BootSnapshot, String>),
    IdentityCreated(Result<StoredSession, String>),
    Unlocked(Result<StoredSession, String>),
    LobbyUpdated(Result<LobbySnapshot, String>),
    RelaySaved(Result<String, String>),
}

#[derive(Debug, Clone)]
pub enum Effect {
    LoadBoot,
    CreateIdentity {
        handle: String,
        password: String,
        relay_url: String,
    },
    Unlock {
        password: String,
    },
    ConnectTransport {
        relay_url: String,
        nsec: String,
    },
    SaveRelayUrl {
        relay_url: String,
    },
    DisconnectTransport,
    Quit,
}

fn bootstrap_route(snapshot: &BootSnapshot, relay_url: String) -> Route {
    if snapshot.has_keychain {
        Route::Locked(LockedModel {
            password_input: String::new(),
            status: None,
        })
    } else {
        Route::FirstRun(FirstRunModel {
            active_field: FirstRunField::Handle,
            handle_input: String::new(),
            password_input: String::new(),
            confirm_input: String::new(),
            relay_input: relay_url,
            status: None,
        })
    }
}

fn lobby_route(status: Option<String>, relay_url: String) -> Route {
    Route::Lobby(LobbyModel {
        active_tab: LobbyTab::Home,
        help_open: true,
        selected_game: 0,
        editing_relay: false,
        relay_draft: relay_url,
        status,
    })
}

fn active_session_from_stored(stored: StoredSession, password: String) -> SessionState {
    SessionState {
        password,
        active_npub: stored.active_npub,
        active_nsec: stored.active_nsec,
        active_handle: stored.active_handle,
    }
}

fn handle_help_click(model: &mut Model, position: Point) -> bool {
    let Route::Lobby(lobby) = &mut model.route else {
        return false;
    };
    if !lobby.help_open {
        return false;
    }
    let width = model.geometry.width();
    let popup_width = 60usize.min(width.saturating_sub(4));
    let left = (width.saturating_sub(popup_width)) / 2;
    let top = 8usize;
    let close_col = left + popup_width.saturating_sub(4);
    let row = position.row.as_usize();
    let column = position.column.as_usize();
    if row >= top && row <= top + 10 && column >= left && column <= left + popup_width {
        if row == top && column >= close_col && column <= close_col + 2 {
            lobby.help_open = false;
        } else {
            lobby.help_open = false;
        }
        true
    } else {
        false
    }
}

fn is_printable_key(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch) => Some(ch),
        _ => None,
    }
}

fn append_text(target: &mut String, text: &str) {
    target.extend(text.chars().filter(|ch| !ch.is_control()));
}

fn field_string_mut(model: &mut FirstRunModel) -> &mut String {
    match model.active_field {
        FirstRunField::Handle => &mut model.handle_input,
        FirstRunField::Password => &mut model.password_input,
        FirstRunField::Confirm => &mut model.confirm_input,
        FirstRunField::Relay => &mut model.relay_input,
    }
}

fn mask(value: &str) -> String {
    "*".repeat(value.chars().count())
}

fn status_color(status: NetworkState) -> GameColor {
    match status {
        NetworkState::Idle => GameColor::BrightBlack,
        NetworkState::Connecting => GameColor::Yellow,
        NetworkState::Synced => GameColor::BrightGreen,
        NetworkState::Error => GameColor::BrightRed,
    }
}
