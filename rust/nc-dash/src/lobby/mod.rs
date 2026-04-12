pub mod hosted;
pub mod models;
pub mod onboarding;
pub mod panels;
pub mod state;
pub mod storage;
pub mod threads;
pub mod transport;
pub mod update;

use crossterm::event::{KeyEvent, MouseEvent};
use nc_ui::modal::{ModalTheme, Rect, centered_rect, draw_box, render_modal_box};
use nc_ui::theme::classic;
use nc_ui::{PlayfieldBuffer, ScreenGeometry};

use crate::native::NativeApp;
use crate::startup::LobbyStartupOptions;

use self::state::{LobbyFocus, LobbyRoute};

pub use self::state::LobbyApp;

impl LobbyApp {
    pub fn new(options: LobbyStartupOptions) -> Self {
        let route = onboarding::initial_route(nc_client::keychain::keychain_path().exists());
        Self {
            geometry: ScreenGeometry::new(120, 40),
            should_quit: false,
            state: state::LobbyState::new(options.clone(), route),
            transport: transport::LobbyTransport::new(options.relay_override),
        }
    }

    pub fn new_for_tests(route: LobbyRoute, geometry: ScreenGeometry) -> Self {
        Self {
            geometry,
            should_quit: false,
            state: state::LobbyState::new(LobbyStartupOptions::default(), route),
            transport: transport::LobbyTransport::new(None),
        }
    }

    pub fn render_for_test(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        <Self as NativeApp>::render_playfield(self)
    }

    fn render_header(&self, buffer: &mut PlayfieldBuffer) {
        let title = "NOSTRIAN CONQUEST LOBBY";
        buffer.write_text_clipped(0, 2, title, classic::shell_title_style());
        let relay = self
            .state
            .relay_label()
            .unwrap_or_else(|| "relay: not set".to_string());
        let handle = self
            .state
            .player_handle_label()
            .unwrap_or_else(|| "handle: unset".to_string());
        let right = format!("{handle}   {relay}");
        let start = buffer.width().saturating_sub(right.chars().count() + 2);
        buffer.write_text_clipped(0, start, &right, classic::shell_label_style());
    }

    fn render_footer(&self, buffer: &mut PlayfieldBuffer) {
        let footer = match self.state.route {
            LobbyRoute::Home => "COMMANDS <- Tab Shift-Tab J K Enter N M H R Q ->",
            LobbyRoute::FirstRun => "FIRST RUN <- type handle/password Enter create Q quit ->",
            LobbyRoute::Locked => "LOCKED <- type password Enter unlock Q quit ->",
            LobbyRoute::ComposeInvite => "REQUEST INVITE <- type message Enter send Esc close ->",
            LobbyRoute::ComposeThread => "THREAD MESSAGE <- type message Enter send Esc close ->",
            LobbyRoute::EditHandle => "EDIT HANDLE <- type handle Enter save Esc close ->",
            LobbyRoute::HostedGame => "HOSTED GAME <- R refresh T submit-turn Esc lobby ->",
            LobbyRoute::SubmitTurn => "SUBMIT TURN <- type commands Enter send Esc cancel ->",
        };
        let row = buffer.height().saturating_sub(1);
        buffer.write_text_clipped(row, 1, footer, classic::prompt_style());
    }

    fn render_home(&self, buffer: &mut PlayfieldBuffer) {
        let width = buffer.width() as u16;
        let height = buffer.height() as u16;
        if width < 40 || height < 16 {
            let lines = vec![
                "nc-lobby needs a larger window.".to_string(),
                "Resize and reopen the lobby.".to_string(),
            ];
            let _ = render_modal_box(buffer, "WINDOW TOO SMALL", &lines, modal_theme());
            return;
        }

        let body = Rect::new(0, 1, width, height.saturating_sub(2));
        let gap = 1u16;
        let left_w = width.saturating_mul(28) / 100;
        let center_w = width.saturating_mul(32) / 100;
        let right_w = width
            .saturating_sub(left_w)
            .saturating_sub(center_w)
            .saturating_sub(gap * 4)
            .max(24);

        let left = Rect::new(1, body.y + 1, left_w, body.height.saturating_sub(2));
        let center = Rect::new(
            left.x + left.width + gap,
            body.y + 1,
            center_w,
            body.height.saturating_sub(2),
        );
        let right = Rect::new(
            center.x + center.width + gap,
            body.y + 1,
            right_w,
            body.height.saturating_sub(2),
        );

        let left_top_h = left.height.saturating_mul(3) / 5;
        let joined = Rect::new(left.x, left.y, left.width, left_top_h.max(8));
        let inbox = Rect::new(
            left.x,
            joined.y + joined.height + gap,
            left.width,
            left.height.saturating_sub(joined.height).saturating_sub(gap),
        );
        let notices_h = right.height.saturating_mul(2) / 5;
        let notices = Rect::new(right.x, right.y, right.width, notices_h.max(7));
        let thread = Rect::new(
            right.x,
            notices.y + notices.height + gap,
            right.width,
            right.height.saturating_sub(notices.height).saturating_sub(gap),
        );

        panels::joined_games::render(buffer, joined, &self.state, self.state.focus);
        panels::inbox::render(buffer, inbox, &self.state, self.state.focus);
        panels::open_games::render(buffer, center, &self.state, self.state.focus);
        panels::notices::render(buffer, notices, &self.state, self.state.focus);
        panels::thread::render(buffer, thread, &self.state, self.state.focus);

        if let Some(status) = self.state.status_message.as_deref() {
            let row = buffer.height().saturating_sub(2);
            buffer.write_text_clipped(row, 2, status, classic::notice_style());
        }
    }

    fn render_modal_route(&self, buffer: &mut PlayfieldBuffer) {
        match self.state.route {
            LobbyRoute::Home => self.render_home(buffer),
            LobbyRoute::HostedGame => {}
            LobbyRoute::FirstRun => {
                let _ = render_modal_box(
                    buffer,
                    "FIRST RUN",
                    &onboarding::first_run_lines(&self.state),
                    modal_theme(),
                );
            }
            LobbyRoute::Locked => {
                let _ = render_modal_box(
                    buffer,
                    "UNLOCK KEYCHAIN",
                    &onboarding::locked_lines(&self.state),
                    modal_theme(),
                );
            }
            LobbyRoute::ComposeInvite => {
                let _ = render_modal_box(
                    buffer,
                    "REQUEST INVITE",
                    &vec![
                        format!(
                            "Game    : {}",
                            self.state
                                .selected_open_game()
                                .map(|row| row.game.as_str())
                                .unwrap_or("<none>")
                        ),
                        format!("Message : {}", self.state.compose_message_input),
                        "Enter sends a 30513 invite request.".to_string(),
                    ],
                    modal_theme(),
                );
            }
            LobbyRoute::ComposeThread => {
                let _ = render_modal_box(
                    buffer,
                    "PRIVATE THREAD",
                    &vec![
                        format!("Game    : {}", self.state.thread_context_display()),
                        format!("Message : {}", self.state.compose_message_input),
                        "Enter sends an encrypted 30517 sysop thread message.".to_string(),
                    ],
                    modal_theme(),
                );
            }
            LobbyRoute::EditHandle => {
                let _ = render_modal_box(
                    buffer,
                    "EDIT HANDLE",
                    &vec![
                        format!(
                            "Current handle: {}",
                            self.state
                                .player_handle
                                .as_deref()
                                .unwrap_or("<unset>")
                        ),
                        format!("New handle   : {}", self.state.edit_handle_input),
                        "Enter saves the local keychain handle.".to_string(),
                    ],
                    modal_theme(),
                );
            }
            LobbyRoute::SubmitTurn => {
                let _ = render_modal_box(
                    buffer,
                    "SUBMIT TURN",
                    &vec![
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
                        format!(
                            "Commands : {}",
                            self.state
                                .hosted_game
                                .as_ref()
                                .map(|hosted| hosted.submit_input.as_str())
                                .unwrap_or("")
                        ),
                        "Enter sends raw 30522 turn text.".to_string(),
                    ],
                    modal_theme(),
                );
            }
        }
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
        update::apply_key(self, key);
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) {
        if self.state.route == LobbyRoute::HostedGame {
            if let Some(hosted) = self.state.hosted_game.as_mut() {
                hosted.dashboard.dispatch_mouse_event(mouse);
                if hosted.dashboard.should_quit {
                    self.should_quit = true;
                }
            }
        }
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        if let Some(hosted) = self.state.hosted_game.as_mut() {
            hosted.dashboard.resize_canvas(cols, rows);
        }
    }

    fn render_playfield(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if self.state.route == LobbyRoute::HostedGame {
            if let Some(hosted) = self.state.hosted_game.as_ref() {
                return hosted.dashboard.render_playfield();
            }
        }
        let mut buffer = PlayfieldBuffer::new(
            self.geometry.width(),
            self.geometry.height(),
            classic::body_style(),
        );
        self.render_header(&mut buffer);
        self.render_modal_route(&mut buffer);
        self.render_footer(&mut buffer);
        Ok(buffer)
    }

    fn on_idle(&mut self) -> bool {
        match self.transport.poll_updates() {
            Ok(Some(loaded)) => {
                self.state.apply_loaded(loaded);
                true
            }
            Ok(None) => false,
            Err(err) => {
                let changed = self.state.status_message.as_deref() != Some(err.as_str());
                self.state.status_message = Some(err);
                changed
            }
        }
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
        body_style: classic::table_body_style(),
        pad_style: classic::body_style(),
        chrome_style: classic::table_chrome_style(),
        title_style: classic::table_header_style(),
    }
}

fn draw_panel_frame(buffer: &mut PlayfieldBuffer, rect: Rect, title: &str, focused: bool) {
    let title_style = if focused {
        classic::selected_row_style()
    } else {
        classic::table_header_style()
    };
    draw_box(
        buffer,
        rect,
        title,
        classic::table_chrome_style(),
        title_style,
    );
    buffer.fill_rect(
        rect.y as usize + 1,
        rect.x as usize + 1,
        rect.width.saturating_sub(2) as usize,
        rect.height.saturating_sub(2) as usize,
        classic::table_body_style(),
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
            classic::selected_row_style()
        } else {
            classic::table_body_style()
        };
        buffer.write_text_clipped(
            content.y as usize + idx,
            content.x as usize,
            row,
            style,
        );
    }
}

pub(crate) fn focus_selected(focus: LobbyFocus, target: LobbyFocus, selected: usize) -> Option<usize> {
    (focus == target).then_some(selected)
}
