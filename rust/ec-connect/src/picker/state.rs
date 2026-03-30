use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::cache::{GameCache, load_cache};
use crate::connect::handshake::GameEntry;
use crate::connect::resolve::ResolvedTarget;
use crate::wallet::Wallet;
use nostr_sdk::Keys;

use super::connecting::{ActiveConnect, PendingConnectRequest};
use super::help::HelpTopic;
use super::overlay::{NoticeLevel, PickerOverlay};
use super::refresh::PendingRefreshRequest;

pub const BODY_PAGE: isize = 10;
const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    GameList,
    RelayList,
    RelayGames {
        relay_url: String,
    },
    IdentityOverlay,
    WalletList,
    WalletAddPrompt,
    GameSelect {
        games: Vec<GameEntry>,
        selected: usize,
        server_host: String,
        server_port: u16,
        relay_url: String,
        gate_npub: String,
    },
    Locked,
}

pub struct MatrixState {
    pub frame: u64,
}

impl MatrixState {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn reset(&mut self) {
        self.frame = 0;
    }

    pub fn advance(&mut self) {
        self.frame = self.frame.saturating_add(1);
    }
}

pub struct PickerSession {
    pub password: String,
    pub wallet: Wallet,
    pub keys: Keys,
    pub npub: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectOrigin {
    GameList,
    JoinPrompt,
    GameSelect,
    GameRelayPrompt { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectDisplay {
    pub lines: Vec<String>,
}

impl ConnectDisplay {
    fn server_line(target: &ResolvedTarget) -> String {
        if target.server_host.trim().is_empty() {
            "Server: discover via relay".to_string()
        } else {
            format!("Server: {}:{}", target.server_host, target.server_port)
        }
    }

    pub fn from_game(name: &str, target: &ResolvedTarget) -> Self {
        Self {
            lines: vec![
                format!("Game: {}", name),
                Self::server_line(target),
                format!("Relay: {}", target.relay_url),
                "Attempting to connect...".to_string(),
            ],
        }
    }

    pub fn from_invite(invite_code: &str, target: &ResolvedTarget) -> Self {
        Self {
            lines: vec![
                format!("Invite: {}", invite_code),
                Self::server_line(target),
                format!("Relay: {}", target.relay_url),
                "Attempting to connect...".to_string(),
            ],
        }
    }

    pub fn from_invite_claim(invite_code: &str, target: &ResolvedTarget) -> Self {
        Self {
            lines: vec![
                format!("Invite: {}", invite_code),
                Self::server_line(target),
                format!("Relay: {}", target.relay_url),
                "Claiming invite...".to_string(),
            ],
        }
    }
}

pub struct PickerState {
    pub cache: GameCache,
    pub maps_root: PathBuf,
    pub selected: usize,
    pub relay_selected: usize,
    pub relay_game_selected: usize,
    pub wallet_selected: usize,
    pub screen: Screen,
    pub overlay: Option<PickerOverlay>,
    pub pending_connect: Option<PendingConnectRequest>,
    pub active_connect: Option<ActiveConnect>,
    pub pending_refresh: Option<PendingRefreshRequest>,
    pub join_input: String,
    pub maps_input: String,
    pub alias_input: String,
    pub wallet_input: String,
    pub relay_input: String,
    pub quit: bool,
    pub matrix: MatrixState,
    manual_refresh_ready_at: Option<Instant>,
}

impl PickerState {
    pub fn new(cache: GameCache, maps_root: PathBuf) -> Self {
        Self {
            cache,
            maps_root,
            selected: 0,
            relay_selected: 0,
            relay_game_selected: 0,
            wallet_selected: 0,
            screen: Screen::GameList,
            overlay: None,
            pending_connect: None,
            active_connect: None,
            pending_refresh: None,
            join_input: String::new(),
            maps_input: String::new(),
            alias_input: String::new(),
            wallet_input: String::new(),
            relay_input: String::new(),
            quit: false,
            matrix: MatrixState::new(),
            manual_refresh_ready_at: None,
        }
    }

    pub fn open_help(&mut self) {
        if let Some(topic) = HelpTopic::for_screen(&self.screen) {
            self.overlay = Some(PickerOverlay::Help(topic));
        }
    }

    pub fn show_notice(&mut self, message: impl Into<String>) {
        self.overlay = Some(PickerOverlay::Notice {
            level: NoticeLevel::Notice,
            message: message.into(),
        });
    }

    pub fn show_error(&mut self, message: impl Into<String>) {
        self.overlay = Some(PickerOverlay::Notice {
            level: NoticeLevel::Error,
            message: message.into(),
        });
    }

    pub fn request_quit(&mut self) {
        self.overlay = Some(PickerOverlay::QuitConfirm);
    }

    pub fn refresh_cache(&mut self) {
        if let Ok(cache) = load_cache() {
            self.cache = cache;
        }
        let len = self.cache.sorted().len();
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
    }

    pub fn can_manual_refresh(&self) -> bool {
        self.manual_refresh_ready_at
            .map(|ready_at| Instant::now() >= ready_at)
            .unwrap_or(true)
    }

    pub fn mark_manual_refresh(&mut self) {
        self.manual_refresh_ready_at = Some(Instant::now() + MANUAL_REFRESH_COOLDOWN);
    }
}
