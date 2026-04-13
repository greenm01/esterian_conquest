mod clipboard;
pub mod hosted;
pub mod models;
pub mod onboarding;
pub mod panels;
mod ratatui;
pub mod state;
pub mod storage;
pub mod threads;
pub mod transport;
pub mod update;

use crossterm::event::KeyEvent;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use nc_ui::modal::{ModalTheme, Rect, centered_rect, draw_box, render_modal_box};
use nc_ui::{PlayfieldBuffer, ScreenGeometry};
use std::time::{Duration, Instant};

use crate::native::NativeApp;
use crate::startup::LobbyStartupOptions;
use crate::theme;

use self::state::{LobbyFocus, LobbyMouseGesture, LobbyRoute, ThreadPaneFocus};

pub use self::state::LobbyApp;

const MATRIX_FRAME_STEP: Duration = Duration::from_millis(80);

impl LobbyApp {
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
            transport: transport::LobbyTransport::new(options.relay_override),
            settings_path,
            clipboard: clipboard::Clipboard::new(),
            popup_position: None,
            mouse_gesture: LobbyMouseGesture::None,
            last_activity_at: now,
            matrix_rain: onboarding::MatrixRain::new(120, 40),
            next_matrix_frame_at: now + MATRIX_FRAME_STEP,
        }
    }

    pub fn new_for_tests(route: LobbyRoute, geometry: ScreenGeometry) -> Self {
        theme::apply_default_theme();
        let settings = storage::settings::LobbySettingsRecord::default();
        let now = Instant::now();
        let matrix_width = geometry.width();
        let matrix_height = geometry.height();
        Self {
            geometry,
            should_quit: false,
            state: state::LobbyState::new(LobbyStartupOptions::default(), route, settings),
            transport: transport::LobbyTransport::new(None),
            settings_path: storage::settings::settings_path(),
            clipboard: clipboard::Clipboard::new(),
            popup_position: None,
            mouse_gesture: LobbyMouseGesture::None,
            last_activity_at: now,
            matrix_rain: onboarding::MatrixRain::new(matrix_width, matrix_height),
            next_matrix_frame_at: now + MATRIX_FRAME_STEP,
        }
    }

    pub fn set_clipboard_text(&mut self, text: impl Into<String>) {
        self.clipboard.replace_fallback(text.into());
    }

    pub fn render_for_test(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        <Self as NativeApp>::render_playfield(self)
    }

    pub fn dispatch_mouse_event_for_test(&mut self, mouse: MouseEvent) {
        <Self as NativeApp>::dispatch_mouse_event(self, mouse);
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
        self.state.show_resume_sync_overlay = false;
        self.state.route = LobbyRoute::MatrixLocked;
        self.mouse_gesture = LobbyMouseGesture::None;
        self.matrix_rain.reset();
        self.next_matrix_frame_at = Instant::now() + MATRIX_FRAME_STEP;
    }

    pub fn begin_unlock_prompt(&mut self) {
        self.state.unlock_password_input.clear();
        self.state.status_message = None;
        self.state.route = LobbyRoute::Locked;
    }

    fn dismiss_resume_sync_overlay(&mut self) {
        self.state.show_resume_sync_overlay = false;
    }

    fn render_resume_sync_overlay(&self, buffer: &mut PlayfieldBuffer) {
        let lines = vec![format!(
            "Network : {}",
            network_dialog_label(self.state.network_status)
        )];
        let _ = render_modal_box(buffer, "NETWORK", &lines, modal_theme());
    }

    fn scheduled_wakeup(&self) -> Option<Instant> {
        if self.state.route == LobbyRoute::MatrixLocked {
            return Some(self.next_matrix_frame_at);
        }
        if !self.transport.is_unlocked() {
            return None;
        }
        let minutes = self.state.settings.lock_timeout_minutes;
        if minutes == 0 {
            return None;
        }
        Some(self.last_activity_at + Duration::from_secs(u64::from(minutes) * 60))
    }

    fn record_activity(&mut self, now: Instant) {
        self.last_activity_at = now;
    }

    fn render_submit_turn(&self, buffer: &mut PlayfieldBuffer) {
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
        let _ = render_modal_box(buffer, "SUBMIT TURN", &lines, modal_theme());
    }

    fn handle_lobby_mouse_down(&mut self, mouse: MouseEvent) {
        if ratatui::popup_title_bar_contains(self, mouse.column, mouse.row) {
            if let Some(popup) = ratatui::active_popup_rect(self) {
                self.mouse_gesture = LobbyMouseGesture::DraggingPopup {
                    grab_col_offset: mouse.column.saturating_sub(popup.x) as usize,
                    grab_row_offset: mouse.row.saturating_sub(popup.y) as usize,
                };
            }
            return;
        }

        self.mouse_gesture = LobbyMouseGesture::None;
        if self.state.route == LobbyRoute::MatrixLocked {
            self.begin_unlock_prompt();
            return;
        }
        if self.state.route != LobbyRoute::Home {
            return;
        }

        let Some(hit) = ratatui::hit_test_home(&self.state, self.geometry, mouse.column, mouse.row)
        else {
            return;
        };
        let previous_context = self.state.preferred_game_context_id().map(str::to_string);
        self.state.focus = hit.focus;
        match hit.focus {
            LobbyFocus::JoinedGames => {
                if let Some(selected) = hit.selected_row {
                    self.state.joined_selected = selected;
                }
            }
            LobbyFocus::Inbox => {
                if let Some(selected) = hit.selected_row {
                    self.state.inbox_selected = selected;
                }
            }
            LobbyFocus::OpenGames => {
                if let Some(selected) = hit.selected_row {
                    self.state.open_selected = selected;
                }
            }
            LobbyFocus::Notices => {
                if let Some(selected) = hit.selected_row {
                    self.state.notices_selected = selected;
                }
            }
            LobbyFocus::Thread => {
                if let Some(selected) = hit.selected_row {
                    let visible = self.state.visible_direct_contacts();
                    if let Some((absolute_index, _)) = visible.get(selected) {
                        self.state.contact_selected = *absolute_index;
                    }
                }
                self.state.thread_pane_focus =
                    hit.thread_pane_focus.unwrap_or(ThreadPaneFocus::Transcript);
                self.state.thread_scroll = 0;
                self.state.thread_composing = false;
            }
        }
        update::reset_context_dependent_views(self, previous_context);
    }

    fn handle_lobby_mouse_drag(&mut self, mouse: MouseEvent) {
        let LobbyMouseGesture::DraggingPopup {
            grab_col_offset,
            grab_row_offset,
        } = self.mouse_gesture
        else {
            return;
        };
        let Some(layout) = ratatui::home_layout(::ratatui::layout::Rect::new(
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

    fn dispatch_key_event(&mut self, key: KeyEvent) {
        if self.state.show_resume_sync_overlay {
            match key.code {
                crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Enter => {
                    self.dismiss_resume_sync_overlay();
                }
                _ => {}
            }
            return;
        }
        update::apply_key(self, key);
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) {
        if self.state.show_resume_sync_overlay {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.dismiss_resume_sync_overlay();
            }
            return;
        }
        if self.state.route == LobbyRoute::HostedGame {
            if let Some(hosted) = self.state.hosted_game.as_mut() {
                hosted.dashboard.dispatch_mouse_event(mouse);
                if hosted.dashboard.should_quit {
                    self.should_quit = true;
                }
            }
            return;
        }

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
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        self.matrix_rain
            .reset_for_size(cols as usize, rows as usize);
        if let Some(hosted) = self.state.hosted_game.as_mut() {
            hosted.dashboard.resize_canvas(cols, rows);
        }
    }

    fn render_playfield(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
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
                LobbyRoute::MatrixLocked => onboarding::render_matrix_locked(&mut buffer, &self.matrix_rain),
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
        ratatui::render_scene(&mut buffer, self);
        if self.state.show_resume_sync_overlay {
            self.render_resume_sync_overlay(&mut buffer);
        }
        Ok(buffer)
    }

    fn on_idle(&mut self) -> bool {
        let now = Instant::now();
        if self.state.route == LobbyRoute::MatrixLocked && now >= self.next_matrix_frame_at {
            self.matrix_rain.advance();
            self.next_matrix_frame_at = now + MATRIX_FRAME_STEP;
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
        match self.transport.poll_updates() {
            Ok(Some(loaded)) => {
                self.state.apply_loaded(loaded);
                if self.state.focus == LobbyFocus::Thread
                    && self.state.thread_pane_focus == ThreadPaneFocus::Transcript
                    && self.state.thread_scroll == 0
                    && self
                        .state
                        .selected_direct_contact()
                        .is_some_and(|contact| contact.unread_count > 0)
                {
                    if let Some(contact_npub) = self
                        .state
                        .selected_direct_contact()
                        .map(|contact| contact.npub.clone())
                    {
                        if let Ok(loaded) = self.transport.mark_direct_contact_read(&contact_npub) {
                            self.state.apply_loaded(loaded);
                        }
                    }
                }
                if self.state.show_resume_sync_overlay
                    && self.state.network_status == state::LobbyNetworkStatus::Synced
                {
                    self.dismiss_resume_sync_overlay();
                }
                true
            }
            Ok(None) => false,
            Err(err) => {
                let changed = self.state.status_message.as_deref() != Some(err.as_str());
                update::set_network_error(self, err);
                changed
            }
        }
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

fn draw_panel_frame(buffer: &mut PlayfieldBuffer, rect: Rect, title: &str, focused: bool) {
    let title_style = if focused {
        theme::classic::selected_row_style()
    } else {
        theme::table_header_style()
    };
    draw_box(
        buffer,
        rect,
        title,
        theme::table_chrome_style(),
        title_style,
    );
    buffer.fill_rect(
        rect.y as usize + 1,
        rect.x as usize + 1,
        rect.width.saturating_sub(2) as usize,
        rect.height.saturating_sub(2) as usize,
        theme::table_body_style(),
    );
}

pub(crate) fn panel_content_rect(rect: Rect) -> Rect {
    centered_rect(
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
        Rect::new(
            rect.x.saturating_add(1),
            rect.y.saturating_add(1),
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        ),
    )
}

pub(crate) fn write_panel_rows(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    rows: &[String],
    selected: Option<usize>,
) {
    let content = panel_content_rect(rect);
    for (idx, row) in rows.iter().enumerate() {
        if idx >= content.height as usize {
            break;
        }
        let style = if selected == Some(idx) {
            theme::classic::selected_row_style()
        } else {
            theme::table_body_style()
        };
        buffer.write_text_clipped(content.y as usize + idx, content.x as usize, row, style);
    }
}

pub(crate) fn focus_selected(
    focus: LobbyFocus,
    target: LobbyFocus,
    selected: usize,
) -> Option<usize> {
    (focus == target).then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
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
}
