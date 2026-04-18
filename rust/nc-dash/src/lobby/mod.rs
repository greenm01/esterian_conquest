mod clipboard;
pub mod hosted;
pub mod models;
pub mod onboarding;
pub mod state;
pub mod storage;
pub mod transport;
pub mod update;

use crate::buffer::PlayfieldBuffer;
use crate::geometry::ScreenGeometry;
use crate::input::{KeyCode, KeyEvent};
use crate::input::{MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};
use tracing::info;

use crate::modal::{
    ModalTheme, Rect as ModalRect, measure_modal_text_lines, modal_box_rect_for_lines,
    modal_close_button_contains, render_modal_box,
};
use crate::native::NativeApp;
use crate::startup::LobbyStartupOptions;
use crate::theme;
use crate::ui::UiScene;
use crate::ui::screens::lobby as ui;

use self::state::{HostedSyncPhase, LobbyMouseGesture, LobbyRoute, LobbyTab, ThreadPaneFocus};

pub use self::state::LobbyApp;

const MATRIX_FRAME_STEP: Duration = Duration::from_millis(80);
const COMMS_CURSOR_BLINK_STEP: Duration = Duration::from_millis(500);
const GATE_ERROR_RESET_STEP: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct LobbyMouseRenderState {
    route: LobbyRoute,
    active_tab: LobbyTab,
    relay_label: Option<String>,
    player_handle: Option<String>,
    joined_games: Vec<self::models::JoinedGameRow>,
    open_games: Vec<self::models::OpenGameRow>,
    game_inbox: Vec<self::models::GameInboxRow>,
    notices: Vec<self::models::LobbyNotice>,
    direct_contacts: Vec<self::models::DirectContactRow>,
    thread_messages: Vec<self::models::ThreadMessage>,
    game_inbox_messages: Vec<self::models::GameInboxMessage>,
    joined_selected: usize,
    open_selected: usize,
    comms_selected: usize,
    comms_new_selected: usize,
    active_comms: Option<self::models::CommsConversationKey>,
    thread_pane_focus: ThreadPaneFocus,
    popup_position: Option<crate::overlays::frame::RelativePopupOrigin>,
    mouse_gesture: LobbyMouseGesture,
    status_message: Option<String>,
    status_tone: state::LobbyStatusTone,
    show_manual: bool,
    show_help: bool,
    show_resume_sync_overlay: bool,
    hosted_sync_phase: HostedSyncPhase,
    sandbox_join_notice: Option<String>,
    network_status: state::LobbyNetworkStatus,
    should_quit: bool,
}

impl LobbyApp {
    fn bypass_home_ratatui_scene(&self) -> bool {
        self.diagnostic_mode
            && self.state.route == LobbyRoute::Home
            && std::env::var_os("NC_DASH_BYPASS_RATATUI_HOME").is_some()
    }

    pub fn new(options: LobbyStartupOptions) -> Self {
        let route = onboarding::initial_route(nc_client::keychain::keychain_path().exists());
        let settings_path = storage::settings::settings_path();
        let settings = storage::settings::load_settings_from(&settings_path).unwrap_or_default();
        if theme::apply_theme_key(&settings.theme_key).is_err() {
            theme::apply_default_theme();
        }
        let now = Instant::now();
        Self {
            geometry: ScreenGeometry::new(120, 40),
            should_quit: false,
            state: state::LobbyState::new(options.clone(), route, settings),
            transport: transport::LobbyTransport::new(options.relay_override, options.native),
            settings_path,
            clipboard: clipboard::Clipboard::new(),
            popup_position: None,
            mouse_gesture: LobbyMouseGesture::None,
            last_activity_at: now,
            comms_cursor_visible: true,
            next_cursor_blink_at: now + COMMS_CURSOR_BLINK_STEP,
            gate_reset_deadline: None,
            gate_reset_action: None,
            matrix_rain: onboarding::MatrixRain::new(120, 40),
            next_matrix_frame_at: now + MATRIX_FRAME_STEP,
            next_cache_save_at: now + Duration::from_secs(10),
            diagnostic_mode: options.native.diagnostic_mode,
            freeze_live_updates: options.native.freeze_live_updates,
        }
    }

    pub fn new_for_tests(route: LobbyRoute, geometry: ScreenGeometry) -> Self {
        theme::apply_default_theme();
        let settings = storage::settings::LobbySettingsRecord::default();
        let now = Instant::now();
        let matrix_width = geometry.width();
        let matrix_height = geometry.height();
        let mut app = Self {
            geometry,
            should_quit: false,
            state: state::LobbyState::new(LobbyStartupOptions::default(), route, settings),
            transport: transport::LobbyTransport::new(None, LobbyStartupOptions::default().native),
            settings_path: storage::settings::settings_path(),
            clipboard: clipboard::Clipboard::new(),
            popup_position: None,
            mouse_gesture: LobbyMouseGesture::None,
            last_activity_at: now,
            comms_cursor_visible: true,
            next_cursor_blink_at: now + COMMS_CURSOR_BLINK_STEP,
            gate_reset_deadline: None,
            gate_reset_action: None,
            matrix_rain: onboarding::MatrixRain::new(matrix_width, matrix_height),
            next_matrix_frame_at: now + MATRIX_FRAME_STEP,
            next_cache_save_at: now + Duration::from_secs(10),
            diagnostic_mode: false,
            freeze_live_updates: false,
        };
        app.state.show_manual = false;
        app.state.manual_seen_this_session = false;
        app
    }

    pub fn set_clipboard_text(&mut self, text: impl Into<String>) {
        self.clipboard.replace_fallback(text.into());
    }

    pub fn render_for_test(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_lobby_playfield()
    }

    pub fn dispatch_mouse_event_for_test(&mut self, mouse: MouseEvent) -> bool {
        <Self as NativeApp>::dispatch_mouse_event(self, mouse)
    }

    pub fn dispatch_key_event_for_test(&mut self, key: KeyEvent) {
        <Self as NativeApp>::dispatch_key_event(self, key);
    }

    pub fn enter_session_lock(&mut self) {
        if !self.transport.is_unlocked() || self.state.route == LobbyRoute::FirstRun {
            return;
        }
        self.transport.lock();
        self.state.gate_mode = state::KeychainGateMode::ResumeSession;
        self.state.unlock_return_route = self.state.route;
        self.state.unlock_password_input.clear();
        self.state.status_message = None;
        self.gate_reset_deadline = None;
        self.gate_reset_action = None;
        self.set_hosted_sync_phase(HostedSyncPhase::Idle);
        self.state.route = LobbyRoute::MatrixLocked;
        self.mouse_gesture = LobbyMouseGesture::None;
        self.matrix_rain.reset();
        self.next_matrix_frame_at = Instant::now() + MATRIX_FRAME_STEP;
    }

    pub fn begin_unlock_prompt(&mut self) {
        self.state.unlock_password_input.clear();
        self.state.status_message = None;
        self.gate_reset_deadline = None;
        self.gate_reset_action = None;
        self.state.route = LobbyRoute::Locked;
    }

    pub(crate) fn schedule_gate_reset(
        &mut self,
        action: state::GateResetAction,
        now: Instant,
        message: String,
    ) {
        self.state.status_message = Some(message);
        self.state.status_tone = state::LobbyStatusTone::Error;
        self.gate_reset_deadline = Some(now + GATE_ERROR_RESET_STEP);
        self.gate_reset_action = Some(action);
    }

    pub(crate) fn clear_gate_reset(&mut self) {
        self.gate_reset_deadline = None;
        self.gate_reset_action = None;
    }

    fn complete_gate_reset(&mut self) {
        let Some(action) = self.gate_reset_action.take() else {
            self.gate_reset_deadline = None;
            return;
        };
        self.gate_reset_deadline = None;
        self.state.status_message = None;
        self.state.status_tone = state::LobbyStatusTone::Info;
        match action {
            state::GateResetAction::UnlockRetry => {
                self.state.unlock_password_input.clear();
                self.state.route = LobbyRoute::Locked;
            }
            state::GateResetAction::FirstRunRetry => {
                self.state.first_run_password_input.clear();
                self.state.first_run_confirm_input.clear();
                self.state.first_run_field = state::FirstRunField::Password;
                self.state.route = LobbyRoute::FirstRun;
            }
        }
    }

    #[doc(hidden)]
    pub fn process_idle_for_test(&mut self) -> bool {
        self.on_idle()
    }

    #[doc(hidden)]
    pub fn process_idle_for_test_at(&mut self, now: Instant) -> bool {
        self.on_idle_at(now)
    }

    #[doc(hidden)]
    pub fn next_wakeup_for_test(&self) -> Option<Instant> {
        self.scheduled_wakeup()
    }

    fn dismiss_resume_sync_overlay(&mut self) {
        if matches!(
            self.state.hosted_sync_phase,
            HostedSyncPhase::ResumingHostedGame { .. }
        ) {
            self.state.hosted_sync_phase = HostedSyncPhase::Idle;
        }
        self.refresh_resume_sync_overlay();
        update::maybe_open_home_manual(self);
    }

    fn refresh_resume_sync_overlay(&mut self) {
        let hosted_route = matches!(
            self.state.route,
            LobbyRoute::HostedGame | LobbyRoute::SubmitTurn
        ) && self.state.hosted_game.is_some();
        self.state.show_resume_sync_overlay =
            hosted_route && self.state.hosted_sync_phase.is_resuming_hosted_game();
    }

    fn set_hosted_sync_phase(&mut self, phase: HostedSyncPhase) {
        self.state.hosted_sync_phase = phase;
        self.refresh_resume_sync_overlay();
    }

    fn begin_hosted_resume_sync(&mut self) {
        let phase = self
            .state
            .hosted_game
            .as_ref()
            .map(|hosted| HostedSyncPhase::ResumingHostedGame {
                game_id: hosted.row.game_id.clone(),
            })
            .unwrap_or(HostedSyncPhase::Idle);
        self.set_hosted_sync_phase(phase);
    }

    fn complete_hosted_sync_from_cached_state(&mut self) -> bool {
        if matches!(
            self.state.hosted_sync_phase,
            HostedSyncPhase::Idle | HostedSyncPhase::AwaitingTurnReceipt { .. }
        ) {
            return false;
        }
        if let Some(hosted) = self.state.hosted_game.as_mut() {
            if !matches!(
                self.state.hosted_sync_phase,
                HostedSyncPhase::ResumingHostedGame { .. }
            ) && hosted.submit_status.is_none()
            {
                hosted.submit_status =
                    Some("Hosted dashboard synchronized from nc-host.".to_string());
            }
        }
        self.set_hosted_sync_phase(HostedSyncPhase::Idle);
        true
    }

    fn apply_active_hosted_poll_update(
        &mut self,
        active_game: &transport::ActiveHostedGamePollUpdate,
    ) -> bool {
        let Some(current_game_id) = self
            .state
            .hosted_game
            .as_ref()
            .map(|hosted| hosted.row.game_id.as_str())
        else {
            return false;
        };
        if active_game.game_id.as_deref() != Some(current_game_id) {
            if matches!(
                self.state.hosted_sync_phase,
                HostedSyncPhase::ResumingHostedGame { .. }
            ) && self.state.network_status == state::LobbyNetworkStatus::Synced
            {
                return self.complete_hosted_sync_from_cached_state();
            }
            return false;
        }

        let mut changed = false;
        if let Some(receipt) = active_game.turn_receipt.as_ref() {
            changed |= update::apply_active_turn_receipt(self, receipt);
        }
        if let Some(promoted_hash) = active_game.promoted_state_hash.as_deref() {
            let current_hash = self
                .state
                .hosted_game
                .as_ref()
                .map(|hosted| hosted.snapshot.state_hash.as_str());
            if current_hash == Some(promoted_hash) {
                changed |= self.complete_hosted_sync_from_cached_state();
            } else if update::reload_hosted_dashboard_from_cached_snapshot(self) {
                changed = true;
                changed |= self.complete_hosted_sync_from_cached_state();
            }
        } else if matches!(
            self.state.hosted_sync_phase,
            HostedSyncPhase::ResumingHostedGame { .. }
        ) && self.state.network_status == state::LobbyNetworkStatus::Synced
        {
            changed |= self.complete_hosted_sync_from_cached_state();
        }

        changed
    }

    fn mouse_render_state(&self) -> LobbyMouseRenderState {
        LobbyMouseRenderState {
            route: self.state.route,
            active_tab: self.state.active_tab,
            relay_label: self.state.relay_label.clone(),
            player_handle: self.state.player_handle.clone(),
            joined_games: self.state.joined_games.clone(),
            open_games: self.state.open_games.clone(),
            game_inbox: self.state.game_inbox.clone(),
            notices: self.state.notices.clone(),
            direct_contacts: self.state.direct_contacts.clone(),
            thread_messages: self.state.thread_messages.clone(),
            game_inbox_messages: self.state.game_inbox_messages.clone(),
            joined_selected: self.state.joined_selected,
            open_selected: self.state.open_selected,
            comms_selected: self.state.comms_selected,
            comms_new_selected: self.state.comms_new_selected,
            active_comms: self.state.active_comms.clone(),
            thread_pane_focus: self.state.thread_pane_focus,
            popup_position: self.popup_position,
            mouse_gesture: self.mouse_gesture,
            status_message: self.state.status_message.clone(),
            status_tone: self.state.status_tone,
            show_manual: self.state.show_manual,
            show_help: self.state.show_help,
            show_resume_sync_overlay: self.state.show_resume_sync_overlay,
            hosted_sync_phase: self.state.hosted_sync_phase.clone(),
            sandbox_join_notice: self.state.sandbox_join_notice.clone(),
            network_status: self.state.network_status,
            should_quit: self.should_quit,
        }
    }

    fn debug_render_signature_text(&self) -> String {
        let hosted_signature = self
            .state
            .hosted_game
            .as_ref()
            .and_then(|hosted| hosted.dashboard.debug_render_signature())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "route={:?} tab={:?} net={:?} joined={} open={} notices={} contacts={} threads={} inbox={} help={} manual={} resume_sync={} sync_game={} hosted={} hosted_sig={} popup_pos={} gesture={:?} status={}",
            self.state.route,
            self.state.active_tab,
            self.state.network_status,
            self.state.joined_games.len(),
            self.state.open_games.len(),
            self.state.notices.len(),
            self.state.direct_contacts.len(),
            self.state.thread_messages.len(),
            self.state.game_inbox_messages.len(),
            self.state.show_help,
            self.state.show_manual,
            self.state.show_resume_sync_overlay,
            self.state
                .hosted_sync_phase
                .current_game_id()
                .unwrap_or("-"),
            self.state.hosted_game.is_some(),
            hosted_signature,
            self.popup_position.is_some(),
            self.mouse_gesture,
            self.state.status_message.as_deref().unwrap_or("-"),
        )
    }

    fn log_diagnostic_state_change(
        &self,
        source: &str,
        before: &LobbyMouseRenderState,
        after: &LobbyMouseRenderState,
    ) {
        if !self.diagnostic_mode || before == after {
            return;
        }
        info!(
            target: "nc_dash::lobby",
            source,
            before = ?before,
            after = ?after,
            "lobby visible state changed"
        );
    }

    fn render_resume_sync_overlay(&self, buffer: &mut PlayfieldBuffer) {
        let lines = self.resume_sync_overlay_lines();
        let _ = render_modal_box(buffer, "NETWORK", &lines, modal_theme());
    }

    fn render_home_bypass_playfield(&self, buffer: &mut PlayfieldBuffer) {
        let title_style = theme::title_style();
        let label_style = theme::table_header_style();
        let value_style = theme::body_style();
        let accent_style = theme::value_style();
        let dim_style = theme::dim_style();

        buffer.write_text(1, 2, "NOSTRIAN CONQUEST LOBBY", title_style);
        let network = format!("NETWORK: {}", self.state.network_status.label());
        let network_col = self
            .geometry
            .width()
            .saturating_sub(network.chars().count() + 2);
        buffer.write_text(1, network_col, &network, accent_style);
        buffer.write_text(
            3,
            2,
            "[ Home/OpenGames ratatui bypass enabled ]",
            accent_style,
        );
        buffer.write_text(
            4,
            2,
            "Set NC_DASH_BYPASS_RATATUI_HOME=1 only for crash isolation.",
            dim_style,
        );

        let status = self.state.status_message.as_deref().unwrap_or("-");
        let rows = [
            ("Route", "Home"),
            ("Tab", "OpenGames"),
            ("Joined", &self.state.joined_games.len().to_string()),
            ("Open", &self.state.open_games.len().to_string()),
            ("Contacts", &self.state.direct_contacts.len().to_string()),
            ("Notices", &self.state.notices.len().to_string()),
            ("Status", status),
        ];
        for (idx, (label, value)) in rows.into_iter().enumerate() {
            let row = 6 + idx;
            buffer.write_text(row, 2, label, label_style);
            buffer.write_text(row, 14, value, value_style);
        }

        buffer.write_text(15, 2, "Open Games", label_style);
        if self.state.open_games.is_empty() {
            buffer.write_text(16, 4, "<none>", dim_style);
        } else {
            for (idx, row) in self.state.open_games.iter().take(8).enumerate() {
                let marker = if idx == self.state.open_selected {
                    '>'
                } else {
                    ' '
                };
                let line = format!(
                    "{marker} {}  {}  {}/{}  {}",
                    row.game,
                    row.host,
                    row.total_seats.saturating_sub(row.open_seats),
                    row.total_seats,
                    row.turn_summary
                );
                buffer.write_text(16 + idx, 4, &line, value_style);
            }
        }

        buffer.write_text(26, 2, "My Games", label_style);
        if self.state.joined_games.is_empty() {
            buffer.write_text(27, 4, "<none>", dim_style);
        } else {
            for (idx, row) in self.state.joined_games.iter().take(6).enumerate() {
                let marker = if idx == self.state.joined_selected {
                    '>'
                } else {
                    ' '
                };
                let seat = row
                    .seat
                    .map(|seat| seat.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let line = format!("{marker} {}  seat:{}  {}", row.game, seat, row.turn_summary);
                buffer.write_text(27 + idx, 4, &line, value_style);
            }
        }
    }

    fn render_lobby_playfield(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if self.state.route == LobbyRoute::HostedGame {
            if let Some(hosted) = self.state.hosted_game.as_ref() {
                let mut buffer = hosted.dashboard.render_playfield()?;
                if self.state.show_resume_sync_overlay {
                    self.render_resume_sync_overlay(&mut buffer);
                }
                return Ok(buffer);
            }
        }
        let mut buffer = PlayfieldBuffer::new(
            self.geometry.width(),
            self.geometry.height(),
            theme::body_style(),
        );
        if matches!(
            self.state.route,
            LobbyRoute::FirstRun | LobbyRoute::MatrixLocked | LobbyRoute::Locked
        ) {
            match self.state.route {
                LobbyRoute::FirstRun => onboarding::render_first_run(&mut buffer, &self.state),
                LobbyRoute::MatrixLocked => {
                    onboarding::render_matrix_locked(&mut buffer, &self.matrix_rain)
                }
                LobbyRoute::Locked => onboarding::render_locked(&mut buffer, &self.state),
                _ => {}
            }
            return Ok(buffer);
        }
        if self.state.route == LobbyRoute::SubmitTurn {
            self.render_submit_turn(&mut buffer);
            if self.state.show_resume_sync_overlay {
                self.render_resume_sync_overlay(&mut buffer);
            }
            return Ok(buffer);
        }
        if self.bypass_home_ratatui_scene() {
            self.render_home_bypass_playfield(&mut buffer);
        } else {
            ui::render_scene(&mut buffer, self);
        }
        if self.state.show_resume_sync_overlay {
            self.render_resume_sync_overlay(&mut buffer);
        }
        Ok(buffer)
    }

    fn scheduled_wakeup(&self) -> Option<Instant> {
        let gate_reset_wakeup = self.gate_reset_deadline;
        let cursor_wakeup = if self.state.route == LobbyRoute::Home
            && self.state.active_tab == LobbyTab::Comms
            && self.state.thread_pane_focus == ThreadPaneFocus::Chat
            && self
                .state
                .active_comms_row()
                .is_some_and(|row| !row.read_only)
        {
            Some(self.next_cursor_blink_at)
        } else {
            None
        };
        if self.state.route == LobbyRoute::MatrixLocked {
            return gate_reset_wakeup
                .map(|gate| {
                    cursor_wakeup
                        .map(|cursor| gate.min(cursor.min(self.next_matrix_frame_at)))
                        .unwrap_or_else(|| gate.min(self.next_matrix_frame_at))
                })
                .or_else(|| {
                    cursor_wakeup
                        .map(|cursor| cursor.min(self.next_matrix_frame_at))
                        .or(Some(self.next_matrix_frame_at))
                });
        }

        let network_wakeup = if self.transport.is_unlocked() && !self.freeze_live_updates {
            self.transport.next_poll_deadline(Instant::now())
        } else {
            None
        };

        if !self.transport.is_unlocked() {
            return match (gate_reset_wakeup, cursor_wakeup) {
                (Some(gate), Some(cursor)) => Some(gate.min(cursor)),
                (Some(gate), None) => Some(gate),
                (None, Some(cursor)) => Some(cursor),
                (None, None) => None,
            };
        }

        let minutes = self.state.settings.lock_timeout_minutes;
        let idle = if minutes == 0 {
            None
        } else {
            Some(self.last_activity_at + Duration::from_secs(u64::from(minutes) * 60))
        };

        let mut next = network_wakeup;
        if let Some(gate) = gate_reset_wakeup {
            next = Some(next.map_or(gate, |n| n.min(gate)));
        }
        if let Some(cursor) = cursor_wakeup {
            next = Some(next.map_or(cursor, |n| n.min(cursor)));
        }
        if let Some(idle_time) = idle {
            next = Some(next.map_or(idle_time, |n| n.min(idle_time)));
        }
        next
    }

    fn on_idle_at(&mut self, now: Instant) -> bool {
        if let Some(deadline) = self.gate_reset_deadline {
            if now >= deadline {
                self.complete_gate_reset();
                return true;
            }
        }
        if self.state.route == LobbyRoute::MatrixLocked && now >= self.next_matrix_frame_at {
            self.matrix_rain.advance();
            self.next_matrix_frame_at = now + MATRIX_FRAME_STEP;
            return true;
        }
        if self.state.route == LobbyRoute::Home
            && self.state.active_tab == LobbyTab::Comms
            && self.state.thread_pane_focus == ThreadPaneFocus::Chat
            && self
                .state
                .active_comms_row()
                .is_some_and(|row| !row.read_only)
            && now >= self.next_cursor_blink_at
        {
            self.comms_cursor_visible = !self.comms_cursor_visible;
            self.next_cursor_blink_at = now + COMMS_CURSOR_BLINK_STEP;
            return true;
        }
        if self.transport.is_unlocked() {
            let minutes = self.state.settings.lock_timeout_minutes;
            if minutes != 0
                && now.duration_since(self.last_activity_at)
                    >= Duration::from_secs(u64::from(minutes) * 60)
            {
                self.enter_session_lock();
                return true;
            }
        }
        if self.transport.is_unlocked() && now >= self.next_cache_save_at {
            if let Err(err) = self.transport.flush_cache() {
                tracing::error!("failed to flush lobby cache: {err}");
            }
            self.next_cache_save_at = now + Duration::from_secs(10);
        }
        if self.freeze_live_updates {
            return false;
        }
        let active_game_id = self
            .state
            .hosted_game
            .as_ref()
            .map(|hosted| hosted.row.game_id.as_str());
        match self.transport.poll_updates(active_game_id) {
            Ok(Some(update)) => {
                let before = self.mouse_render_state();
                self.state.apply_loaded(update.loaded);
                let hosted_changed = self.apply_active_hosted_poll_update(&update.active_game);
                if self.state.route == LobbyRoute::Home
                    && self.state.active_tab == LobbyTab::Comms
                    && self.state.thread_pane_focus == ThreadPaneFocus::Chat
                    && self.state.comms_scroll == 0
                    && self
                        .state
                        .active_direct_contact()
                        .is_some_and(|contact| contact.unread_count > 0)
                {
                    if let Some(contact_npub) = self
                        .state
                        .active_direct_contact()
                        .map(|contact| contact.npub.clone())
                    {
                        if let Ok(loaded) = self.transport.mark_direct_contact_read(&contact_npub) {
                            self.state.apply_loaded(loaded);
                        }
                    }
                }
                if self.state.show_resume_sync_overlay
                    && self.state.network_status == state::LobbyNetworkStatus::Synced
                    && !matches!(
                        self.state.hosted_sync_phase,
                        HostedSyncPhase::ResumingHostedGame { .. }
                    )
                {
                    self.dismiss_resume_sync_overlay();
                }
                let after = self.mouse_render_state();
                self.log_diagnostic_state_change("poll_updates.apply_loaded", &before, &after);
                hosted_changed || before != after
            }
            Ok(None) => false,
            Err(err) => {
                let changed = self.state.status_message.as_deref() != Some(err.as_str());
                update::set_network_error(self, err);
                changed
            }
        }
    }

    fn record_activity(&mut self, now: Instant) {
        self.last_activity_at = now;
        self.comms_cursor_visible = true;
        self.next_cursor_blink_at = now + COMMS_CURSOR_BLINK_STEP;
    }

    fn render_submit_turn(&self, buffer: &mut PlayfieldBuffer) {
        let lines = self.submit_turn_lines();
        let _ = render_modal_box(buffer, "SUBMIT TURN", &lines, modal_theme());
    }

    fn resume_sync_overlay_lines(&self) -> Vec<String> {
        vec![format!(
            "Network : {}",
            network_dialog_label(self.state.network_status)
        )]
    }

    fn resume_sync_overlay_rect(&self) -> ModalRect {
        let lines = self.resume_sync_overlay_lines();
        let wrapped = measure_modal_text_lines(&lines, self.geometry.width().saturating_sub(12));
        modal_box_rect_for_lines(
            ModalRect::new(
                0,
                0,
                self.geometry.width() as u16,
                self.geometry.height() as u16,
            ),
            "NETWORK",
            &wrapped,
            self.geometry.width().saturating_sub(8),
        )
    }

    fn submit_turn_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "Game     : {}",
                self.state
                    .hosted_game
                    .as_ref()
                    .map(|hosted| hosted.row.game.as_str())
                    .unwrap_or("<none>")
            ),
            format!(
                "Turn     : {}",
                self.state
                    .hosted_game
                    .as_ref()
                    .map(|hosted| hosted.snapshot.turn.to_string())
                    .unwrap_or_else(|| "-".to_string())
            ),
            "Staged turn.kdl:".to_string(),
        ];
        if let Some(hosted) = self.state.hosted_game.as_ref() {
            if hosted.submit_input.is_empty() {
                lines.push("  <no staged orders>".to_string());
            } else {
                lines.extend(
                    hosted
                        .submit_input
                        .lines()
                        .map(|line| format!("  {line}"))
                        .collect::<Vec<_>>(),
                );
            }
            lines.push(
                hosted.submit_status.clone().unwrap_or_else(|| {
                    "Enter sends the staged hosted turn.kdl as 30522.".to_string()
                }),
            );
        }
        lines
    }

    fn submit_turn_popup_rect(&self) -> ModalRect {
        let lines = self.submit_turn_lines();
        let wrapped = measure_modal_text_lines(&lines, self.geometry.width().saturating_sub(12));
        modal_box_rect_for_lines(
            ModalRect::new(
                0,
                0,
                self.geometry.width() as u16,
                self.geometry.height() as u16,
            ),
            "SUBMIT TURN",
            &wrapped,
            self.geometry.width().saturating_sub(8),
        )
    }

    fn close_active_modal(&mut self) {
        if self.state.show_resume_sync_overlay {
            self.dismiss_resume_sync_overlay();
        } else {
            update::close_active_popup(self);
        }
        self.mouse_gesture = LobbyMouseGesture::None;
    }

    fn handle_lobby_mouse_down(&mut self, mouse: MouseEvent) {
        if self.state.show_manual {
            if let Some(popup) = ui::active_popup_rect(self) {
                let popup = ModalRect::new(popup.x, popup.y, popup.width, popup.height);
                if modal_close_button_contains(popup, mouse.column as usize, mouse.row as usize) {
                    self.close_active_modal();
                    return;
                }
            }
            self.state.show_manual = false;
            self.popup_position = None;
            self.mouse_gesture = LobbyMouseGesture::None;
            return;
        }
        if self.state.route == LobbyRoute::SubmitTurn
            && modal_close_button_contains(
                self.submit_turn_popup_rect(),
                mouse.column as usize,
                mouse.row as usize,
            )
        {
            self.close_active_modal();
            return;
        }
        if ui::popup_close_button_contains(self, mouse.column, mouse.row) {
            self.close_active_modal();
            return;
        }
        if ui::popup_title_bar_contains(self, mouse.column, mouse.row) {
            if let Some(popup) = ui::active_popup_rect(self) {
                self.mouse_gesture = LobbyMouseGesture::DraggingPopup {
                    grab_col_offset: mouse.column.saturating_sub(popup.x) as usize,
                    grab_row_offset: mouse.row.saturating_sub(popup.y) as usize,
                };
            }
            return;
        }

        if matches!(
            self.state.route,
            LobbyRoute::SandboxJoinConfirm | LobbyRoute::SandboxJoinUnavailable
        ) {
            self.state.route = LobbyRoute::Home;
            self.state.sandbox_join_target = None;
            self.state.sandbox_join_notice = None;
            self.popup_position = None;
            self.mouse_gesture = LobbyMouseGesture::None;
            return;
        }

        self.mouse_gesture = LobbyMouseGesture::None;
        if self.state.route == LobbyRoute::MatrixLocked {
            self.begin_unlock_prompt();
            return;
        }

        if self.state.route == LobbyRoute::Home {
            if let Some(layout) = ui::home_layout(crate::ui::cell::layout::Rect::new(
                0,
                0,
                self.geometry.width() as u16,
                self.geometry.height() as u16,
            )) {
                if let Some(tab) =
                    ui::hit_test_tabs(&self.state, layout.header, mouse.column, mouse.row)
                {
                    let previous_context =
                        self.state.preferred_game_context_id().map(str::to_string);
                    self.state.active_tab = tab;
                    self.state.sync_default_contact_selection();
                    update::reset_context_dependent_views(self, previous_context);
                    return;
                }
            }
        }

        if self.state.route == LobbyRoute::Home && self.state.active_tab == LobbyTab::Comms {
            let Some(layout) = ui::home_layout(crate::ui::cell::layout::Rect::new(
                0,
                0,
                self.geometry.width() as u16,
                self.geometry.height() as u16,
            )) else {
                return;
            };
            if let Some(hit) = ui::hit_test_workspace(
                &self.state,
                ui::home_tab_content_area(layout.body, LobbyTab::Comms),
                mouse.column,
                mouse.row,
            ) {
                self.state.thread_pane_focus = hit.pane_focus;
                match hit.pane_focus {
                    ThreadPaneFocus::Chat => {}
                    ThreadPaneFocus::New => {
                        if let Some(selected) = hit.selected_row {
                            self.state.comms_new_selected = selected;
                            if let Some(row) = self.state.comms_unread_rows().get(selected).cloned()
                            {
                                self.state.set_active_comms(row.key);
                                self.state.thread_pane_focus = ThreadPaneFocus::Chat;
                            }
                        }
                    }
                    ThreadPaneFocus::Threads => {
                        if let Some(selected) = hit.selected_row {
                            if let Some(row) =
                                self.state.comms_sidebar_rows().get(selected).cloned()
                            {
                                self.state.set_active_comms(row.key);
                                self.state.thread_pane_focus = ThreadPaneFocus::Chat;
                            }
                        }
                    }
                }
            }
            return;
        }

        if self.state.route == LobbyRoute::Settings {
            if let Some(selected) = ui::hit_test_settings(self, mouse.column, mouse.row) {
                self.state.settings_selected = selected;
                update::apply_key(
                    self,
                    KeyEvent::new(KeyCode::Enter, crate::input::KeyModifiers::NONE),
                );
                return;
            }
        }

        if self.state.route == LobbyRoute::ThemePicker {
            if let Some(selected) = ui::hit_test_theme_picker(self, mouse.column, mouse.row) {
                self.state.theme_selected = selected;
                update::apply_key(
                    self,
                    KeyEvent::new(KeyCode::Enter, crate::input::KeyModifiers::NONE),
                );
                return;
            }
        }

        if self.state.route != LobbyRoute::Home {
            return;
        }

        let Some(hit) = ui::hit_test_home(&self.state, self.geometry, mouse.column, mouse.row)
        else {
            return;
        };
        let previous_context = self.state.preferred_game_context_id().map(str::to_string);
        let activate_open_game = hit.tab == LobbyTab::OpenGames
            && self.state.active_tab == LobbyTab::OpenGames
            && hit.selected_row.is_some()
            && hit.selected_row == Some(self.state.open_selected);
        self.state.active_tab = hit.tab;
        match hit.tab {
            LobbyTab::MyGames => {
                if let Some(selected) = hit.selected_row {
                    self.state.joined_selected = selected;
                }
            }
            LobbyTab::OpenGames => {
                if let Some(selected) = hit.selected_row {
                    self.state.open_selected = selected;
                }
            }
            LobbyTab::Comms => {
                if let Some(selected) = hit.selected_row {
                    self.state.comms_selected = selected;
                    if let Some(row) = self.state.selected_comms_hotlist() {
                        self.state.set_active_comms(row.key);
                    }
                }
                self.state.thread_pane_focus = ThreadPaneFocus::Chat;
                self.state.comms_scroll = 0;
                self.state.thread_scroll = 0;
            }
        }
        update::reset_context_dependent_views(self, previous_context);
        if activate_open_game {
            update::activate_selected_open_game(self);
        }
    }

    fn handle_lobby_mouse_drag(&mut self, mouse: MouseEvent) {
        let LobbyMouseGesture::DraggingPopup {
            grab_col_offset,
            grab_row_offset,
        } = self.mouse_gesture
        else {
            return;
        };
        let Some(layout) = ui::home_layout(crate::ui::cell::layout::Rect::new(
            0,
            0,
            self.geometry.width() as u16,
            self.geometry.height() as u16,
        )) else {
            self.mouse_gesture = LobbyMouseGesture::None;
            return;
        };
        let target_x = mouse.column.saturating_sub(grab_col_offset as u16);
        let target_y = mouse.row.saturating_sub(grab_row_offset as u16);
        self.popup_position = Some(crate::overlays::frame::RelativePopupOrigin {
            col_offset: target_x.saturating_sub(layout.body.x) as usize,
            row_offset: target_y.saturating_sub(layout.body.y) as usize,
        });
    }
}

impl NativeApp for LobbyApp {
    fn window_title(&self) -> &'static str {
        "Nostrian Conquest Lobby"
    }

    fn geometry(&self) -> ScreenGeometry {
        self.geometry
    }

    fn saved_window_state(&self) -> Option<self::storage::settings::PersistedWindowState> {
        self.state.settings.persisted_window_state()
    }

    fn native_window_ready(&mut self, window: &winit::window::Window) {
        self.clipboard.attach_window(window);
    }

    fn wants_window_focus(&self) -> bool {
        true
    }

    fn wants_text_input(&self) -> bool {
        matches!(
            self.state.route,
            LobbyRoute::FirstRun
                | LobbyRoute::Locked
                | LobbyRoute::EditHandle
                | LobbyRoute::AddContact
                | LobbyRoute::ComposeInvite
                | LobbyRoute::ComposeThread
                | LobbyRoute::GameInboxThread
                | LobbyRoute::SubmitTurn
        ) || (self.state.route == LobbyRoute::Home
            && self.state.active_tab == crate::lobby::state::LobbyTab::Comms
            && self.state.thread_pane_focus == crate::lobby::state::ThreadPaneFocus::Chat)
    }

    fn persist_window_state(
        &mut self,
        state: self::storage::settings::PersistedWindowState,
    ) -> Result<(), String> {
        self.state.settings.set_persisted_window_state(state);
        self.state.settings_draft.set_persisted_window_state(state);
        self::storage::settings::save_settings_to(&self.state.settings, &self.settings_path)
            .map_err(|err| err.to_string())
    }

    fn dispatch_key_event(&mut self, key: KeyEvent) {
        if self.state.show_resume_sync_overlay {
            match key.code {
                crate::input::KeyCode::Esc | crate::input::KeyCode::Enter => {
                    self.dismiss_resume_sync_overlay();
                }
                _ => {}
            }
            return;
        }
        update::apply_key(self, key);
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        if self.state.show_resume_sync_overlay {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                let before = self.mouse_render_state();
                if modal_close_button_contains(
                    self.resume_sync_overlay_rect(),
                    mouse.column as usize,
                    mouse.row as usize,
                ) {
                    self.close_active_modal();
                } else {
                    self.dismiss_resume_sync_overlay();
                }
                return before != self.mouse_render_state();
            }
            return false;
        }
        if self.state.route == LobbyRoute::HostedGame {
            let before = self.mouse_render_state();
            let mut changed = false;
            let draft_before = self
                .state
                .hosted_game
                .as_ref()
                .and_then(|hosted| hosted.dashboard.hosted_turn_draft.clone());
            if let Some(hosted) = self.state.hosted_game.as_mut() {
                changed = hosted.dashboard.dispatch_mouse_event(mouse);
                if hosted.dashboard.should_quit {
                    self.should_quit = true;
                }
            }
            let draft_after = self
                .state
                .hosted_game
                .as_ref()
                .and_then(|hosted| hosted.dashboard.hosted_turn_draft.clone());
            if draft_before != draft_after {
                update::sync_hosted_dashboard_draft(self);
            }
            return changed || before != self.mouse_render_state();
        }

        let before = self.mouse_render_state();
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_lobby_mouse_down(mouse),
            MouseEventKind::Drag(MouseButton::Left) => self.handle_lobby_mouse_drag(mouse),
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_gesture = LobbyMouseGesture::None;
            }
            MouseEventKind::Moved => {
                if !matches!(self.mouse_gesture, LobbyMouseGesture::DraggingPopup { .. }) {
                    self.mouse_gesture = LobbyMouseGesture::None;
                }
            }
            _ => {}
        }
        before != self.mouse_render_state()
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        self.matrix_rain
            .reset_for_size(cols as usize, rows as usize);
        if let Some(hosted) = self.state.hosted_game.as_mut() {
            hosted.dashboard.resize_canvas(cols, rows);
        }
    }

    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
        if matches!(self.state.route, LobbyRoute::FirstRun | LobbyRoute::Locked) {
            let scene = match self.state.route {
                LobbyRoute::FirstRun => {
                    onboarding::render_first_run_scene(self.geometry, &self.state)
                }
                LobbyRoute::Locked => onboarding::render_locked_scene(self.geometry, &self.state),
                _ => unreachable!(),
            };
            return Ok(scene);
        }
        Ok(UiScene::from(self.render_lobby_playfield()?))
    }

    fn debug_render_signature(&self) -> Option<String> {
        Some(self.debug_render_signature_text())
    }

    fn on_idle(&mut self) -> bool {
        self.on_idle_at(Instant::now())
    }

    fn is_dragging_surface(&self) -> bool {
        matches!(self.mouse_gesture, LobbyMouseGesture::DraggingPopup { .. })
    }

    fn note_user_activity(&mut self, now: Instant) {
        self.record_activity(now);
    }

    fn next_wakeup(&self) -> Option<Instant> {
        self.scheduled_wakeup()
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn set_should_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }
}

fn modal_theme() -> ModalTheme {
    ModalTheme {
        body_style: theme::table_body_style(),
        pad_style: theme::body_style(),
        chrome_style: theme::table_chrome_style(),
        title_style: theme::table_header_style(),
    }
}

fn network_dialog_label(status: state::LobbyNetworkStatus) -> &'static str {
    match status {
        state::LobbyNetworkStatus::NoRelay => "No Relay",
        state::LobbyNetworkStatus::Connecting => "Connecting",
        state::LobbyNetworkStatus::Connected => "Connected",
        state::LobbyNetworkStatus::Refreshing => "Refreshing",
        state::LobbyNetworkStatus::Synced => "Synced",
        state::LobbyNetworkStatus::Error => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeyModifiers;
    use crate::native::NativeApp;

    #[test]
    fn popup_drag_reports_dragging_surface_state() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Settings, ScreenGeometry::new(120, 40));
        let buffer = app.render_for_test().expect("render settings");
        let row = (0..buffer.height())
            .find(|&idx| buffer.plain_line(idx).contains(" LOBBY SETTINGS "))
            .expect("settings title row");
        let column = buffer
            .plain_line(row)
            .find("LOBBY SETTINGS")
            .expect("settings title") as u16;

        app.dispatch_mouse_event(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row: row as u16,
            modifiers: KeyModifiers::empty(),
        });
        assert!(<LobbyApp as NativeApp>::is_dragging_surface(&app));

        app.dispatch_mouse_event(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column,
            row: row as u16,
            modifiers: KeyModifiers::empty(),
        });
        assert!(!<LobbyApp as NativeApp>::is_dragging_surface(&app));
    }

    #[test]
    fn passive_home_mouse_move_without_drag_reports_no_redraw() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

        assert!(!app.dispatch_mouse_event_for_test(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::empty(),
        }));
    }

    #[test]
    fn popup_drag_reports_redraw_when_position_changes() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Settings, ScreenGeometry::new(120, 40));
        let buffer = app.render_for_test().expect("render settings");
        let row = (0..buffer.height())
            .find(|&idx| buffer.plain_line(idx).contains(" LOBBY SETTINGS "))
            .expect("settings title row");
        let column = buffer
            .plain_line(row)
            .find("LOBBY SETTINGS")
            .expect("settings title") as u16;

        assert!(app.dispatch_mouse_event_for_test(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row: row as u16,
            modifiers: KeyModifiers::empty(),
        }));
        assert!(app.dispatch_mouse_event_for_test(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: column + 10,
            row: row as u16 + 2,
            modifiers: KeyModifiers::empty(),
        }));
    }

    #[test]
    fn gate_routes_request_text_input() {
        let first_run = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));
        let locked = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));

        assert!(<LobbyApp as NativeApp>::wants_text_input(&first_run));
        assert!(<LobbyApp as NativeApp>::wants_text_input(&locked));
    }

    #[test]
    fn lobby_routes_request_window_focus() {
        let home = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
        let locked = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));

        assert!(<LobbyApp as NativeApp>::wants_window_focus(&home));
        assert!(<LobbyApp as NativeApp>::wants_window_focus(&locked));
    }

    #[test]
    fn home_route_does_not_request_text_input() {
        let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

        assert!(!<LobbyApp as NativeApp>::wants_text_input(&app));
    }
}
