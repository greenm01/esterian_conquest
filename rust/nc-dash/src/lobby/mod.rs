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

use crossterm::event::{KeyEvent, MouseEvent};
use nc_ui::modal::{ModalTheme, Rect, centered_rect, draw_box, render_modal_box};
use nc_ui::{PlayfieldBuffer, ScreenGeometry};

use crate::native::NativeApp;
use crate::startup::LobbyStartupOptions;
use crate::theme;

use self::state::{LobbyFocus, LobbyRoute};

pub use self::state::LobbyApp;

impl LobbyApp {
    pub fn new(options: LobbyStartupOptions) -> Self {
        let route = onboarding::initial_route(nc_client::keychain::keychain_path().exists());
        let settings_path = storage::settings::settings_path();
        let settings = storage::settings::load_settings_from(&settings_path).unwrap_or_default();
        if theme::apply_theme_key(&settings.theme_key).is_err() {
            theme::apply_default_theme();
        }
        Self {
            geometry: ScreenGeometry::new(120, 40),
            should_quit: false,
            state: state::LobbyState::new(options.clone(), route, settings),
            transport: transport::LobbyTransport::new(options.relay_override),
            settings_path,
            clipboard: clipboard::Clipboard::new(),
        }
    }

    pub fn new_for_tests(route: LobbyRoute, geometry: ScreenGeometry) -> Self {
        theme::apply_default_theme();
        let settings = storage::settings::LobbySettingsRecord::default();
        Self {
            geometry,
            should_quit: false,
            state: state::LobbyState::new(LobbyStartupOptions::default(), route, settings),
            transport: transport::LobbyTransport::new(None),
            settings_path: storage::settings::settings_path(),
            clipboard: clipboard::Clipboard::new(),
        }
    }

    pub fn set_clipboard_text(&mut self, text: impl Into<String>) {
        self.clipboard.replace_fallback(text.into());
    }

    pub fn render_for_test(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        <Self as NativeApp>::render_playfield(self)
    }

    fn render_header(&self, buffer: &mut PlayfieldBuffer) {
        let title = "NOSTRIAN CONQUEST LOBBY";
        buffer.write_text_clipped(0, 2, title, theme::shell_title_style());
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
        buffer.write_text_clipped(0, start, &right, theme::shell_label_style());
    }

    fn render_footer(&self, buffer: &mut PlayfieldBuffer) {
        let footer = match self.state.route {
            LobbyRoute::Home => "COMMANDS <- Tab Shift-Tab J K Enter N M H S R Q ->",
            LobbyRoute::FirstRun => {
                "FIRST RUN <- type handle/password Enter next-create Up Down Q quit ->"
            }
            LobbyRoute::Locked => "LOCKED <- type password Enter unlock Q quit ->",
            LobbyRoute::ComposeInvite => "REQUEST INVITE <- type message Enter send Esc close ->",
            LobbyRoute::ComposeThread => "THREAD MESSAGE <- type message Enter send Esc close ->",
            LobbyRoute::EditHandle => "EDIT HANDLE <- type handle Enter save Esc close ->",
            LobbyRoute::Settings => "SETTINGS <- J K move Enter toggle/open S save Esc cancel ->",
            LobbyRoute::ThemePicker => "THEMES <- J K preview Enter accept Esc cancel ->",
            LobbyRoute::HostedGame => "HOSTED GAME <- R refresh T submit-turn Esc lobby ->",
            LobbyRoute::SubmitTurn => "SUBMIT TURN <- type commands Enter send Esc cancel ->",
        };
        let row = buffer.height().saturating_sub(1);
        buffer.write_text_clipped(row, 1, footer, theme::prompt_style());
    }

    fn render_modal_route(&self, buffer: &mut PlayfieldBuffer) {
        match self.state.route {
            LobbyRoute::Home => ratatui::render_home(buffer, &self.state),
            LobbyRoute::HostedGame => {}
            LobbyRoute::Settings => {
                ratatui::render_settings(buffer, &self.state);
            }
            LobbyRoute::ThemePicker => {
                ratatui::render_theme_picker(buffer, &self.state);
            }
            LobbyRoute::FirstRun => {
                onboarding::render_first_run(buffer, &self.state);
            }
            LobbyRoute::Locked => {
                onboarding::render_locked(buffer, &self.state);
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
                            self.state.player_handle.as_deref().unwrap_or("<unset>")
                        ),
                        format!("New handle   : {}", self.state.edit_handle_input),
                        "Enter saves the local keychain handle.".to_string(),
                    ],
                    modal_theme(),
                );
            }
            LobbyRoute::SubmitTurn => {
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
                    lines.push(hosted.submit_status.clone().unwrap_or_else(|| {
                        "Enter sends the staged hosted turn.kdl as 30522.".to_string()
                    }));
                }
                let _ = render_modal_box(buffer, "SUBMIT TURN", &lines, modal_theme());
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
            theme::body_style(),
        );
        if matches!(self.state.route, LobbyRoute::FirstRun | LobbyRoute::Locked) {
            self.render_modal_route(&mut buffer);
            return Ok(buffer);
        }
        if self.state.route == LobbyRoute::Home {
            self.render_modal_route(&mut buffer);
            return Ok(buffer);
        }
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
                update::set_network_error(self, err);
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
        body_style: theme::table_body_style(),
        pad_style: theme::body_style(),
        chrome_style: theme::table_chrome_style(),
        title_style: theme::table_header_style(),
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
