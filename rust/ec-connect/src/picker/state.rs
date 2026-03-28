use crate::cache::{GameCache, load_cache};
use crate::connect::handshake::GameEntry;
use crate::wallet::Wallet;
use nostr_sdk::Keys;

use super::help::HelpTopic;
use super::overlay::{NoticeLevel, PickerOverlay};

pub const BODY_PAGE: isize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    GameList,
    JoinPrompt,
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

pub struct PickerState {
    pub cache: GameCache,
    pub selected: usize,
    pub wallet_selected: usize,
    pub screen: Screen,
    pub overlay: Option<PickerOverlay>,
    pub join_input: String,
    pub alias_input: String,
    pub wallet_input: String,
    pub relay_input: String,
    pub quit: bool,
    pub matrix: MatrixState,
}

impl PickerState {
    pub fn new(cache: GameCache) -> Self {
        Self {
            cache,
            selected: 0,
            wallet_selected: 0,
            screen: Screen::GameList,
            overlay: None,
            join_input: String::new(),
            alias_input: String::new(),
            wallet_input: String::new(),
            relay_input: String::new(),
            quit: false,
            matrix: MatrixState::new(),
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
}
