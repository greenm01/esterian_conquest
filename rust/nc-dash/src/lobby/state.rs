use std::time::Instant;

use crate::geometry::ScreenGeometry;
use nc_nostr::state_sync::GameState;

use crate::app::state::DashApp;
use crate::overlays::frame::RelativePopupOrigin;
use crate::startup::LobbyStartupOptions;
use crate::theme::ThemeCatalogEntry;

use super::clipboard::Clipboard;
use super::models::{
    CommsConversationKey, CommsConversationKind, CommsConversationRow, DirectContactRow,
    GameInboxMessage, GameInboxRow, JoinedGameRow, LobbyNotice, OpenGameRow, ThreadMessage,
};
use super::onboarding::MatrixRain;
use super::storage::settings::LobbySettingsRecord;
use super::transport::LobbyTransport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyRoute {
    FirstRun,
    MatrixLocked,
    Locked,
    Home,
    FirstJoinSetup,
    QuitConfirm,
    ComposeInvite,
    SandboxJoinConfirm,
    SandboxJoinUnavailable,
    GameInboxThread,
    ComposeThread,
    ContactPicker,
    AddContact,
    EditHandle,
    Settings,
    ThemePicker,
    HostedGame,
    SubmitTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyNetworkStatus {
    NoRelay,
    Connecting,
    Connected,
    Refreshing,
    Synced,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeychainGateMode {
    Startup,
    ResumeSession,
}

impl LobbyNetworkStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoRelay => "NO RELAY",
            Self::Connecting => "CONNECTING",
            Self::Connected => "CONNECTED",
            Self::Refreshing => "REFRESHING",
            Self::Synced => "SYNCED",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyStatusTone {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunField {
    Handle,
    Password,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstJoinSetupField {
    Empire,
    Homeworld,
}

impl FirstJoinSetupField {
    pub fn next(self) -> Self {
        match self {
            Self::Empire => Self::Homeworld,
            Self::Homeworld => Self::Empire,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateResetAction {
    UnlockRetry,
    FirstRunRetry,
}

impl FirstRunField {
    pub fn next(self) -> Self {
        match self {
            Self::Handle => Self::Password,
            Self::Password => Self::Confirm,
            Self::Confirm => Self::Handle,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Handle => Self::Confirm,
            Self::Password => Self::Handle,
            Self::Confirm => Self::Password,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyTab {
    MyGames,
    OpenGames,
    Comms,
}

impl LobbyTab {
    pub fn next(self) -> Self {
        match self {
            Self::MyGames => Self::OpenGames,
            Self::OpenGames => Self::Comms,
            Self::Comms => Self::MyGames,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::MyGames => Self::Comms,
            Self::OpenGames => Self::MyGames,
            Self::Comms => Self::OpenGames,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPaneFocus {
    Chat,
    New,
    Threads,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedSyncPhase {
    Idle,
    ResumingHostedGame { game_id: String },
    AwaitingTurnReceipt { game_id: String, turn: u32 },
    AwaitingAuthoritativeState { game_id: String, turn: u32 },
}

impl HostedSyncPhase {
    pub fn current_game_id(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::ResumingHostedGame { game_id }
            | Self::AwaitingTurnReceipt { game_id, .. }
            | Self::AwaitingAuthoritativeState { game_id, .. } => Some(game_id.as_str()),
        }
    }

    pub fn is_resuming_hosted_game(&self) -> bool {
        matches!(self, Self::ResumingHostedGame { .. })
    }
}

pub struct HostedGameView {
    pub row: JoinedGameRow,
    pub snapshot: GameState,
    pub dashboard: DashApp,
    pub submit_input: String,
    pub submit_status: Option<String>,
}

pub struct FirstJoinSetupView {
    pub row: JoinedGameRow,
    pub empire_input: String,
    pub homeworld_input: String,
    pub active_field: FirstJoinSetupField,
    pub status: Option<String>,
    pub homeworld_coords: [u8; 2],
    pub present_production: u16,
    pub potential_production: u16,
}

pub struct LobbyState {
    pub route: LobbyRoute,
    pub quit_confirm_return_route: LobbyRoute,
    pub gate_mode: KeychainGateMode,
    pub unlock_return_route: LobbyRoute,
    pub active_tab: LobbyTab,
    pub relay_override: Option<String>,
    pub relay_label: Option<String>,
    pub player_handle: Option<String>,
    pub joined_games: Vec<JoinedGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub game_inbox: Vec<GameInboxRow>,
    pub notices: Vec<LobbyNotice>,
    pub direct_contacts: Vec<DirectContactRow>,
    pub thread_messages: Vec<ThreadMessage>,
    pub game_inbox_messages: Vec<GameInboxMessage>,
    pub joined_selected: usize,
    pub open_selected: usize,
    pub contact_selected: usize,
    pub contact_picker_selected: usize,
    pub game_inbox_filter_selected: usize,
    pub comms_selected: usize,
    pub comms_new_selected: usize,
    pub active_comms: Option<CommsConversationKey>,
    pub thread_pane_focus: ThreadPaneFocus,
    pub comms_scroll: usize,
    pub thread_scroll: usize,
    pub game_inbox_scroll: usize,
    pub game_inbox_composing: bool,
    pub network_status: LobbyNetworkStatus,
    pub status_message: Option<String>,
    pub status_tone: LobbyStatusTone,
    pub show_help: bool,
    pub show_manual: bool,
    pub show_resume_sync_overlay: bool,
    pub auto_open_manual_after_onboarding: bool,
    pub manual_seen_this_session: bool,
    pub sandbox_join_target: Option<OpenGameRow>,
    pub sandbox_join_notice: Option<String>,
    pub first_join_setup: Option<FirstJoinSetupView>,
    pub first_run_field: FirstRunField,
    pub first_run_handle_input: String,
    pub first_run_password_input: String,
    pub first_run_confirm_input: String,
    pub unlock_password_input: String,
    pub compose_message_input: String,
    pub game_inbox_message_input: String,
    pub add_contact_input: String,
    pub edit_handle_input: String,
    pub edit_handle_return_route: LobbyRoute,
    pub settings: LobbySettingsRecord,
    pub settings_draft: LobbySettingsRecord,
    pub settings_selected: usize,
    pub theme_selected: usize,
    pub theme_original_key: String,
    pub hosted_game: Option<HostedGameView>,
    pub hosted_sync_phase: HostedSyncPhase,
}

impl LobbyState {
    pub fn new(
        options: LobbyStartupOptions,
        route: LobbyRoute,
        settings: LobbySettingsRecord,
    ) -> Self {
        Self {
            route,
            quit_confirm_return_route: route,
            gate_mode: KeychainGateMode::Startup,
            unlock_return_route: LobbyRoute::Home,
            active_tab: LobbyTab::OpenGames,
            relay_override: options.relay_override.clone(),
            relay_label: options
                .relay_override
                .map(|relay| format!("relay: {relay}")),
            player_handle: None,
            joined_games: Vec::new(),
            open_games: Vec::new(),
            game_inbox: Vec::new(),
            notices: vec![LobbyNotice::new(
                "nc-host",
                "Waiting for live public notices from nc-host.",
            )],
            direct_contacts: Vec::new(),
            thread_messages: Vec::new(),
            game_inbox_messages: Vec::new(),
            joined_selected: 0,
            open_selected: 0,
            contact_selected: 0,
            contact_picker_selected: 0,
            game_inbox_filter_selected: 0,
            comms_selected: 0,
            comms_new_selected: 0,
            active_comms: None,
            thread_pane_focus: ThreadPaneFocus::Chat,
            comms_scroll: 0,
            thread_scroll: 0,
            game_inbox_scroll: 0,
            game_inbox_composing: false,
            network_status: LobbyNetworkStatus::NoRelay,
            status_message: None,
            status_tone: LobbyStatusTone::Info,
            show_help: false,
            show_manual: false,
            show_resume_sync_overlay: false,
            auto_open_manual_after_onboarding: false,
            manual_seen_this_session: false,
            sandbox_join_target: None,
            sandbox_join_notice: None,
            first_join_setup: None,
            first_run_field: FirstRunField::Handle,
            first_run_handle_input: String::new(),
            first_run_password_input: String::new(),
            first_run_confirm_input: String::new(),
            unlock_password_input: String::new(),
            compose_message_input: String::new(),
            game_inbox_message_input: String::new(),
            add_contact_input: String::new(),
            edit_handle_input: String::new(),
            edit_handle_return_route: LobbyRoute::Settings,
            settings_draft: settings.clone(),
            settings,
            settings_selected: 0,
            theme_selected: 0,
            theme_original_key: crate::theme::default_theme_key().to_string(),
            hosted_game: None,
            hosted_sync_phase: HostedSyncPhase::Idle,
        }
    }

    pub fn apply_loaded(&mut self, loaded: super::transport::LobbyLoadedState) {
        self.relay_label = loaded.relay_label;
        self.player_handle = loaded.player_handle;
        self.joined_games = loaded.joined_games;
        self.open_games = loaded.open_games;
        self.game_inbox = loaded.game_inbox;
        self.notices = loaded.notices;
        self.direct_contacts = loaded.direct_contacts;
        self.ensure_host_contacts_present();
        self.sync_host_labels_from_contacts();
        self.sort_direct_contacts();
        self.thread_messages = loaded.thread_messages;
        self.game_inbox_messages = loaded.game_inbox_messages;
        self.network_status = loaded.network_status;
        self.status_message = loaded.status_message;
        self.status_tone = loaded.status_tone;
        self.joined_selected = self
            .joined_selected
            .min(self.joined_games.len().saturating_sub(1));
        self.open_selected = self
            .open_selected
            .min(self.open_games.len().saturating_sub(1));
        self.contact_selected = self
            .contact_selected
            .min(self.direct_contacts.len().saturating_sub(1));
        self.contact_picker_selected = self
            .contact_picker_selected
            .min(self.direct_contacts.len().saturating_sub(1));
        self.sync_visible_contact_selection();
        self.comms_selected = self
            .comms_selected
            .min(self.comms_hotlist_rows().len().saturating_sub(1));
        self.comms_new_selected = self
            .comms_new_selected
            .min(self.comms_unread_rows().len().saturating_sub(1));
        self.comms_scroll = self.comms_scroll.min(self.active_comms_messages_len());
        self.thread_scroll = self.comms_scroll;
        self.game_inbox_scroll = self
            .game_inbox_scroll
            .min(self.visible_game_inbox_messages().len());
        self.edit_handle_input = self.player_handle.clone().unwrap_or_default();
        self.sync_default_contact_selection();
        self.sync_active_comms_selection();
    }

    fn ensure_host_contacts_present(&mut self) {
        let joined = self
            .joined_games
            .iter()
            .map(|row| (row.host_contact_npub.clone(), row.host.clone()))
            .collect::<Vec<_>>();
        for (npub, label) in joined {
            self.ensure_host_contact(npub.as_deref(), label.as_str());
        }
        let open = self
            .open_games
            .iter()
            .map(|row| (row.host_contact_npub.clone(), row.host.clone()))
            .collect::<Vec<_>>();
        for (npub, label) in open {
            self.ensure_host_contact(npub.as_deref(), label.as_str());
        }
        self.direct_contacts.sort_by(|left, right| {
            right
                .unread_count
                .cmp(&left.unread_count)
                .then_with(|| right.last_activity_at.cmp(&left.last_activity_at))
                .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
                .then_with(|| left.npub.cmp(&right.npub))
        });
    }

    fn sync_host_labels_from_contacts(&mut self) {
        let labels = self
            .direct_contacts
            .iter()
            .filter(|contact| contact.source == "host")
            .map(|contact| (contact.npub.clone(), contact.label.clone()))
            .collect::<Vec<_>>();
        for row in &mut self.joined_games {
            if let Some(label) = row
                .host_contact_npub
                .as_ref()
                .and_then(|npub| labels.iter().find(|(known, _)| known == npub))
                .map(|(_, label)| label.clone())
            {
                row.host = label;
            }
        }
        for row in &mut self.open_games {
            if let Some(label) = row
                .host_contact_npub
                .as_ref()
                .and_then(|npub| labels.iter().find(|(known, _)| known == npub))
                .map(|(_, label)| label.clone())
            {
                row.host = label;
            }
        }
    }

    fn ensure_host_contact(&mut self, npub: Option<&str>, label: &str) {
        let Some(npub) = npub.map(str::trim).filter(|value| !value.is_empty()) else {
            return;
        };
        let label = label.trim();
        if let Some(existing) = self
            .direct_contacts
            .iter_mut()
            .find(|contact| contact.npub == npub)
        {
            if existing.source == "host" && !label.is_empty() {
                existing.label = label.to_string();
            }
            return;
        }
        self.direct_contacts.push(DirectContactRow {
            npub: npub.to_string(),
            label: label.to_string(),
            nip05: None,
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        });
    }

    fn sort_direct_contacts(&mut self) {
        self.direct_contacts.sort_by(|left, right| {
            right
                .unread_count
                .cmp(&left.unread_count)
                .then_with(|| right.last_activity_at.cmp(&left.last_activity_at))
                .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
                .then_with(|| left.npub.cmp(&right.npub))
        });
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

    pub fn selected_game_inbox(&self) -> Option<&GameInboxRow> {
        self.filtered_game_inbox().first().copied()
    }

    pub fn selected_direct_contact(&self) -> Option<&DirectContactRow> {
        self.direct_contacts.get(self.contact_selected)
    }

    pub fn visible_direct_contacts(&self) -> Vec<(usize, &DirectContactRow)> {
        self.direct_contacts
            .iter()
            .enumerate()
            .filter(|(_, contact)| !contact.blocked && !contact.hidden)
            .collect()
    }

    pub fn selectable_direct_contacts(&self) -> Vec<(usize, &DirectContactRow)> {
        self.direct_contacts
            .iter()
            .enumerate()
            .filter(|(_, contact)| !contact.blocked)
            .collect()
    }

    pub fn selected_visible_contact_index(&self) -> Option<usize> {
        let selected_npub = self.selected_direct_contact()?.npub.as_str();
        self.visible_direct_contacts()
            .iter()
            .position(|(_, contact)| contact.npub == selected_npub)
    }

    pub fn game_filter_options(&self) -> Vec<(Option<String>, String)> {
        let mut options = vec![(None, "All".to_string())];
        options.extend(
            self.joined_games
                .iter()
                .map(|row| (Some(row.game_id.clone()), row.game.clone())),
        );
        options
    }

    pub fn selected_game_filter(&self) -> Option<String> {
        self.game_filter_options()
            .get(self.game_inbox_filter_selected)
            .and_then(|(game_id, _)| game_id.clone())
    }

    pub fn filtered_game_inbox(&self) -> Vec<&GameInboxRow> {
        match self.selected_game_filter() {
            Some(game_id) => self
                .game_inbox
                .iter()
                .filter(|row| row.game_id == game_id)
                .collect(),
            None => self.game_inbox.iter().collect(),
        }
    }

    pub fn preferred_game_context_id(&self) -> Option<&str> {
        match self.active_tab {
            LobbyTab::MyGames => self
                .selected_joined_game()
                .map(|row| row.game_id.as_str())
                .or_else(|| self.selected_open_game().map(|row| row.game_id.as_str())),
            LobbyTab::OpenGames => self
                .selected_open_game()
                .map(|row| row.game_id.as_str())
                .or_else(|| self.selected_joined_game().map(|row| row.game_id.as_str())),
            _ => self
                .selected_joined_game()
                .map(|row| row.game_id.as_str())
                .or_else(|| self.selected_open_game().map(|row| row.game_id.as_str())),
        }
    }

    pub fn preferred_host_contact_npub(&self) -> Option<&str> {
        self.selected_joined_game()
            .and_then(|row| row.host_contact_npub.as_deref())
            .or_else(|| {
                self.selected_open_game()
                    .and_then(|row| row.host_contact_npub.as_deref())
            })
    }

    pub fn direct_thread_context_display(&self) -> String {
        self.active_direct_contact()
            .map(|contact| {
                contact
                    .nip05
                    .clone()
                    .unwrap_or_else(|| contact.label.clone())
            })
            .unwrap_or_else(|| "no direct contact selected".to_string())
    }

    pub fn game_inbox_context_display(&self) -> String {
        self.active_game_inbox()
            .map(|row| format!("{} / {}", row.game, row.other_empire_name))
            .unwrap_or_else(|| "no game inbox thread selected".to_string())
    }

    pub fn visible_thread_messages(&self) -> Vec<&ThreadMessage> {
        let Some(contact_npub) = self
            .active_direct_contact()
            .map(|contact| contact.npub.as_str())
        else {
            return Vec::new();
        };
        self.thread_messages
            .iter()
            .filter(|message| message.contact_npub == contact_npub)
            .collect()
    }

    pub fn visible_game_inbox_messages(&self) -> Vec<&GameInboxMessage> {
        let Some(row) = self.active_game_inbox() else {
            return Vec::new();
        };
        self.game_inbox_messages
            .iter()
            .filter(|message| {
                message.game_id == row.game_id && message.other_empire_id == row.other_empire_id
            })
            .collect()
    }

    pub fn available_themes(&self) -> Vec<ThemeCatalogEntry> {
        crate::theme::catalog()
    }

    pub fn reset_thread_view(&mut self) {
        self.thread_pane_focus = ThreadPaneFocus::Chat;
        self.comms_scroll = 0;
        self.thread_scroll = 0;
        self.compose_message_input.clear();
    }

    pub fn reset_game_inbox_view(&mut self) {
        self.game_inbox_scroll = 0;
        self.game_inbox_composing = false;
        self.game_inbox_message_input.clear();
    }

    pub fn sync_default_contact_selection(&mut self) {
        let Some(host_npub) = self.preferred_host_contact_npub() else {
            self.sync_visible_contact_selection();
            return;
        };
        if let Some(index) = self
            .direct_contacts
            .iter()
            .position(|contact| contact.npub == host_npub)
        {
            self.contact_selected = index;
            self.contact_picker_selected = index;
        }
        self.sync_visible_contact_selection();
    }

    pub fn sync_visible_contact_selection(&mut self) {
        if self
            .selected_direct_contact()
            .is_some_and(|contact| !contact.blocked && !contact.hidden)
        {
            return;
        }
        if let Some((index, _)) = self.visible_direct_contacts().into_iter().next() {
            self.contact_selected = index;
            self.contact_picker_selected = index;
        }
    }

    pub fn thread_unread_total(&self) -> u32 {
        self.direct_contacts
            .iter()
            .filter(|contact| !contact.blocked && !contact.hidden)
            .map(|contact| contact.unread_count)
            .sum()
    }

    pub fn comms_hotlist_rows(&self) -> Vec<CommsConversationRow> {
        let mut rows = self.comms_conversation_rows(false);
        rows.sort_by(|left, right| {
            right
                .unread_count
                .cmp(&left.unread_count)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });
        rows
    }

    pub fn comms_unread_rows(&self) -> Vec<CommsConversationRow> {
        let mut rows = self
            .comms_hotlist_rows()
            .into_iter()
            .filter(|row| row.unread_count > 0)
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .unread_count
                .cmp(&left.unread_count)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });
        rows
    }

    pub fn comms_sidebar_rows(&self) -> Vec<CommsConversationRow> {
        let mut announcements = self
            .comms_conversation_rows(true)
            .into_iter()
            .filter(|row| row.kind == CommsConversationKind::Announcement)
            .collect::<Vec<_>>();
        let mut direct = self
            .comms_conversation_rows(true)
            .into_iter()
            .filter(|row| row.kind == CommsConversationKind::Direct)
            .collect::<Vec<_>>();
        direct.sort_by(|left, right| {
            right
                .unread_count
                .cmp(&left.unread_count)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });
        announcements.append(&mut direct);
        announcements
    }

    fn comms_conversation_rows(&self, include_hidden_direct: bool) -> Vec<CommsConversationRow> {
        let mut rows = Vec::new();
        if let Some(latest_notice) = self.notices.last() {
            rows.push(CommsConversationRow {
                key: CommsConversationKey::Announcements,
                kind: CommsConversationKind::Announcement,
                title: "Broadcast".to_string(),
                preview: latest_notice.body.clone(),
                updated_at: latest_notice.created_at.clone(),
                unread_count: 0,
                blocked: false,
                hidden: false,
                read_only: true,
            });
        }
        rows.extend(self.direct_contacts.iter().filter_map(|contact| {
            if !include_hidden_direct && (contact.hidden || contact.blocked) {
                return None;
            }
            Some(CommsConversationRow {
                key: CommsConversationKey::Direct {
                    contact_npub: contact.npub.clone(),
                },
                kind: CommsConversationKind::Direct,
                title: contact.label.clone(),
                preview: self
                    .thread_messages
                    .iter()
                    .rev()
                    .find(|message| message.contact_npub == contact.npub)
                    .map(|message| message.body.clone())
                    .unwrap_or_else(|| "<no direct messages>".to_string()),
                updated_at: contact.last_activity_at.clone().unwrap_or_default(),
                unread_count: contact.unread_count,
                blocked: contact.blocked,
                hidden: contact.hidden,
                read_only: false,
            })
        }));
        rows
    }

    pub fn selected_comms_hotlist(&self) -> Option<CommsConversationRow> {
        self.comms_hotlist_rows().get(self.comms_selected).cloned()
    }

    pub fn set_active_comms(&mut self, key: CommsConversationKey) {
        self.active_comms = Some(key.clone());
        self.sync_active_direct_contact_index();
        self.sync_active_unread_selection();
        self.comms_scroll = 0;
        self.thread_scroll = 0;
        self.compose_message_input.clear();
    }

    pub fn sync_active_comms_selection(&mut self) {
        let hotlist = self.comms_hotlist_rows();
        let rows = self.comms_sidebar_rows();
        if rows.is_empty() {
            self.active_comms = None;
            self.comms_selected = 0;
            return;
        }
        if let Some(active) = self.active_comms.as_ref() {
            if rows.iter().any(|row| &row.key == active) {
                self.comms_selected = hotlist
                    .iter()
                    .position(|row| Some(&row.key) == self.active_comms.as_ref())
                    .unwrap_or(0);
                self.sync_active_direct_contact_index();
                self.sync_active_unread_selection();
                return;
            }
        }
        if let Some(host_npub) = self.preferred_host_contact_npub() {
            let host_key = CommsConversationKey::Direct {
                contact_npub: host_npub.to_string(),
            };
            if rows.iter().any(|row| row.key == host_key) {
                self.active_comms = Some(CommsConversationKey::Direct {
                    contact_npub: host_npub.to_string(),
                });
                self.comms_selected = hotlist
                    .iter()
                    .position(|row| row.key == host_key)
                    .unwrap_or(0);
                self.sync_active_direct_contact_index();
                self.sync_active_unread_selection();
                return;
            }
        }
        self.active_comms = Some(rows[0].key.clone());
        self.comms_selected = hotlist
            .iter()
            .position(|row| row.key == rows[0].key)
            .unwrap_or(0);
        self.sync_active_direct_contact_index();
        self.sync_active_unread_selection();
    }

    pub fn active_direct_contact(&self) -> Option<&DirectContactRow> {
        let Some(CommsConversationKey::Direct { contact_npub }) = self.active_comms.as_ref() else {
            return None;
        };
        self.direct_contacts
            .iter()
            .find(|contact| &contact.npub == contact_npub)
    }

    pub fn active_game_inbox(&self) -> Option<&GameInboxRow> {
        let Some(CommsConversationKey::GameMail {
            game_id,
            other_empire_id,
        }) = self.active_comms.as_ref()
        else {
            return None;
        };
        self.game_inbox
            .iter()
            .find(|row| &row.game_id == game_id && row.other_empire_id == *other_empire_id)
    }

    pub fn active_notice(&self) -> Option<&LobbyNotice> {
        matches!(self.active_comms, Some(CommsConversationKey::Announcements))
            .then(|| self.notices.last())
            .flatten()
    }

    pub fn active_comms_row(&self) -> Option<CommsConversationRow> {
        let active = self.active_comms.as_ref()?;
        self.comms_sidebar_rows()
            .into_iter()
            .find(|row| &row.key == active)
    }

    pub fn active_comms_messages_len(&self) -> usize {
        match self.active_comms.as_ref() {
            Some(CommsConversationKey::Announcements) => self.notices.len().max(1),
            Some(CommsConversationKey::GameMail { .. }) => self.visible_game_inbox_messages().len(),
            Some(CommsConversationKey::Direct { .. }) => self.visible_thread_messages().len(),
            None => 0,
        }
    }

    fn sync_active_direct_contact_index(&mut self) {
        let Some(CommsConversationKey::Direct { contact_npub }) = self.active_comms.as_ref() else {
            return;
        };
        if let Some(index) = self
            .direct_contacts
            .iter()
            .position(|contact| &contact.npub == contact_npub)
        {
            self.contact_selected = index;
            self.contact_picker_selected = index;
        }
    }

    fn sync_active_unread_selection(&mut self) {
        let rows = self.comms_unread_rows();
        let Some(active) = self.active_comms.as_ref() else {
            self.comms_new_selected = 0;
            return;
        };
        self.comms_new_selected = rows.iter().position(|row| &row.key == active).unwrap_or(0);
    }
}

pub struct LobbyApp {
    pub geometry: ScreenGeometry,
    pub state: LobbyState,
    pub transport: LobbyTransport,
    pub should_quit: bool,
    pub settings_path: std::path::PathBuf,
    pub(crate) clipboard: Clipboard,
    pub popup_position: Option<RelativePopupOrigin>,
    pub mouse_gesture: LobbyMouseGesture,
    pub last_activity_at: Instant,
    pub comms_cursor_visible: bool,
    pub next_cursor_blink_at: Instant,
    pub gate_reset_deadline: Option<Instant>,
    pub gate_reset_action: Option<GateResetAction>,
    pub matrix_rain: MatrixRain,
    pub next_matrix_frame_at: Instant,
    pub next_cache_save_at: Instant,
    pub diagnostic_mode: bool,
    pub freeze_live_updates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyMouseGesture {
    None,
    DraggingPopup {
        grab_col_offset: usize,
        grab_row_offset: usize,
    },
}
