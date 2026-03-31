use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_ui::buffer::PlayfieldBuffer;
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;
use winit::keyboard::ModifiersState;

use crate::cache::{GameCache, load_cache};
use crate::config::{ConnectConfig, load_config};
use crate::launcher::render as launcher_render;
use crate::launcher::{GateSubmit, PasswordGateState};
use crate::map_store::resolve_maps_root;
use crate::password::wallet_exists;
use crate::picker::connecting::{
    ConnectTaskResult, PendingConnectRequest, apply_connect_outcome, poll_active_connect_result,
    start_pending_connect,
};
use crate::picker::flows::join_with_code;
use crate::picker::input::{
    handle_game_list_key, handle_game_select_key, handle_identity_overlay_key, handle_relay_key,
    handle_wallet_key,
};
use crate::picker::overlay::{PickerOverlay, handle_overlay_key};
use crate::picker::refresh::execute_pending_refresh;
use crate::picker::render as picker_render;
use crate::picker::session::load_picker_session;
use crate::picker::state::{PickerSession, PickerState, Screen};
use crate::wallet::io::{now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{Wallet, push_new_identity};

use super::clipboard::Clipboard;
use super::input::{is_key_press, is_paste_shortcut, pasteable_text, picker_key};
use super::terminal::TerminalView;
use super::{TERM_COLS, TERM_ROWS};

const LOCKED_FRAME_STEP: Duration = Duration::from_millis(80);

pub enum LaunchIntent {
    Normal,
    Join(String),
}

pub struct App {
    view: AppView,
    clipboard: Clipboard,
    mouse_pos: PhysicalPosition<f64>,
    pub needs_redraw: bool,
    pub exit_requested: bool,
}

enum AppView {
    Empty,
    Password(PasswordView),
    Picker(PickerView),
    Live(LiveView),
}

struct PasswordView {
    state: PasswordGateState,
    resume_picker: Option<PickerView>,
    launch_join: Option<String>,
}

struct PickerView {
    session: Option<PickerSession>,
    state: PickerState,
    rt: tokio::runtime::Runtime,
    gate_npub: String,
    lock_timeout_minutes: u16,
    last_activity: Instant,
    next_locked_frame: Instant,
}

struct LiveView {
    picker: PickerView,
    request: PendingConnectRequest,
    terminal: TerminalView,
}

enum PasswordAction {
    None,
    Cancel,
    Submit(String),
}

impl App {
    pub fn new(intent: LaunchIntent) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            view: AppView::Password(PasswordView {
                state: PasswordGateState::new(wallet_exists(&wallet_path()), None),
                resume_picker: None,
                launch_join: match intent {
                    LaunchIntent::Normal => None,
                    LaunchIntent::Join(invite) => Some(invite),
                },
            }),
            clipboard: Clipboard::new(),
            mouse_pos: PhysicalPosition::new(0.0, 0.0),
            needs_redraw: true,
            exit_requested: false,
        })
    }

    pub fn request_close(&mut self) {
        if let AppView::Live(live) = &mut self.view {
            live.terminal.close();
        }
        self.exit_requested = true;
    }

    pub fn handle_mouse_move(
        &mut self,
        position: PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.mouse_pos = position;
        if let AppView::Live(live) = &mut self.view {
            if live.terminal.handle_mouse_move(position)? {
                self.needs_redraw = true;
            }
        }
        Ok(())
    }

    pub fn handle_mouse_button(
        &mut self,
        button: MouseButton,
        pressed: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match button {
            MouseButton::Left => {
                if let AppView::Live(live) = &mut self.view {
                    if live.terminal.handle_mouse_button(pressed, self.mouse_pos)? {
                        self.needs_redraw = true;
                    }
                }
            }
            MouseButton::Right if pressed => {
                self.handle_paste()?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(
        &mut self,
        event: &winit::event::KeyEvent,
        modifiers: ModifiersState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if is_paste_shortcut(event, modifiers) {
            self.handle_paste()?;
            return Ok(());
        }

        if let AppView::Live(live) = &mut self.view {
            if live
                .terminal
                .handle_key(event, modifiers, &mut self.clipboard)?
            {
                self.needs_redraw = true;
                return Ok(());
            }
        }

        if matches!(&self.view, AppView::Picker(picker) if matches!(picker.state.screen, Screen::Locked))
            && is_key_press(event)
        {
            let picker = self.take_picker();
            self.view = AppView::Password(PasswordView {
                state: PasswordGateState::new(true, None),
                resume_picker: Some(picker),
                launch_join: None,
            });
            self.needs_redraw = true;
            return Ok(());
        }

        if self.handle_wallet_detail_copy_shortcut(event, modifiers)? {
            return Ok(());
        }

        match &mut self.view {
            AppView::Password(password) => match handle_password_key(&mut password.state, event) {
                PasswordAction::None => {}
                PasswordAction::Cancel => {
                    if let Some(picker) = password.resume_picker.take() {
                        self.view = AppView::Picker(picker);
                    } else {
                        self.exit_requested = true;
                    }
                }
                PasswordAction::Submit(password_text) => {
                    let launch_join = password.launch_join.take();
                    if let Some(mut picker) = password.resume_picker.take() {
                        match load_picker_session(password_text) {
                            Ok(session) => {
                                picker.session = Some(session);
                                picker.state.screen = Screen::GameList;
                                picker.last_activity = Instant::now();
                                self.view = AppView::Picker(picker);
                            }
                            Err(err) => {
                                password.state.error_msg = Some(format!("Error: {err}"));
                            }
                        }
                    } else {
                        if !wallet_exists(&wallet_path()) {
                            let mut wallet = Wallet::empty();
                            push_new_identity(&mut wallet, now_iso8601())?;
                            save_wallet_to(&wallet, &password_text, &wallet_path())?;
                        }
                        match PickerView::load(password_text, launch_join) {
                            Ok(picker) => self.view = AppView::Picker(picker),
                            Err(err) => {
                                password.state.error_msg = Some(format!("Error: {err}"));
                            }
                        }
                    }
                }
            },
            AppView::Picker(picker) => {
                if let Some(key) = picker_key(event, modifiers) {
                    picker.handle_key(key)?;
                }
            }
            AppView::Live(_) | AppView::Empty => {}
        }

        self.resolve_transitions()?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.view {
            AppView::Picker(picker) => {
                self.needs_redraw |= picker.tick()?;
            }
            AppView::Live(live) => {
                live.terminal.tick(&mut self.clipboard)?;
                self.needs_redraw = true;
            }
            AppView::Password(_) | AppView::Empty => {}
        }
        self.resolve_transitions()?;
        Ok(())
    }

    pub fn control_flow(&self) -> winit::event_loop::ControlFlow {
        match &self.view {
            AppView::Picker(picker) => picker.control_flow(),
            AppView::Live(_) => winit::event_loop::ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ),
            AppView::Password(_) | AppView::Empty => winit::event_loop::ControlFlow::Wait,
        }
    }

    pub fn current_buffer(&self) -> PlayfieldBuffer {
        match &self.view {
            AppView::Password(password) => launcher_render::render_inner_buffer(&password.state),
            AppView::Picker(picker) => {
                picker_render::render_inner_buffer(&picker.state, picker.session.as_ref())
            }
            AppView::Live(live) => live.terminal.render_buffer(),
            AppView::Empty => PlayfieldBuffer::new(
                TERM_COLS as usize,
                TERM_ROWS as usize,
                ec_ui::theme::classic::body_style(),
            ),
        }
    }

    fn handle_paste(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(text) = self.clipboard.get_text()? else {
            return Ok(());
        };
        self.apply_pasted_text(&text)
    }

    fn apply_pasted_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.view {
            AppView::Password(password) => {
                for ch in pasteable_text(text) {
                    password.state.push_char(ch);
                }
            }
            AppView::Picker(picker) => {
                for ch in pasteable_text(text) {
                    picker.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))?;
                }
            }
            AppView::Live(live) => {
                live.terminal.paste_text(text);
            }
            AppView::Empty => {}
        }
        self.resolve_transitions()?;
        self.needs_redraw = true;
        Ok(())
    }

    fn handle_wallet_detail_copy_shortcut(
        &mut self,
        event: &winit::event::KeyEvent,
        modifiers: ModifiersState,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !is_key_press(event) || !modifiers.control_key() {
            return Ok(false);
        }

        let AppView::Picker(picker) = &mut self.view else {
            return Ok(false);
        };
        let Some(PickerOverlay::WalletDetail { index }) = picker.state.overlay.as_ref() else {
            return Ok(false);
        };
        let Some(session) = picker.session.as_ref() else {
            return Ok(false);
        };
        let Some(identity) = session.selected_identity(*index) else {
            return Ok(false);
        };

        let Some((label, value)) = wallet_detail_copy_value(event, identity) else {
            return Ok(false);
        };

        self.clipboard.set_text(value)?;
        picker
            .state
            .show_notice(format!("{label} copied to the clipboard."));
        self.needs_redraw = true;
        Ok(true)
    }

    fn resolve_transitions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if matches!(&self.view, AppView::Live(live) if live.terminal.finished()) {
            let live = self.take_live();
            let picker = live.finish()?;
            self.view = AppView::Picker(picker);
            self.needs_redraw = true;
            return Ok(());
        }

        if let AppView::Picker(picker) = &self.view {
            if picker.state.quit {
                self.exit_requested = true;
                return Ok(());
            }
        }

        let prepared = match &mut self.view {
            AppView::Picker(picker) => picker.take_prepared_connect()?,
            _ => None,
        };
        if let Some((request, prepared, finalizer, username)) = prepared {
            let picker = self.take_picker();
            self.view = AppView::Live(LiveView::new(
                picker, request, prepared, finalizer, username,
            ));
            self.needs_redraw = true;
        }

        Ok(())
    }

    fn take_picker(&mut self) -> PickerView {
        match std::mem::replace(&mut self.view, AppView::Empty) {
            AppView::Picker(picker) => picker,
            other => {
                self.view = other;
                panic!("attempted to take picker view while not in picker");
            }
        }
    }

    fn take_live(&mut self) -> LiveView {
        match std::mem::replace(&mut self.view, AppView::Empty) {
            AppView::Live(live) => live,
            other => {
                self.view = other;
                panic!("attempted to take live view while not in live session");
            }
        }
    }
}

fn wallet_detail_copy_value(
    event: &winit::event::KeyEvent,
    identity: &crate::wallet::Identity,
) -> Option<(&'static str, String)> {
    match &event.logical_key {
        winit::keyboard::Key::Character(text) if text.eq_ignore_ascii_case("p") => Some((
            "Public identity",
            crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string()),
        )),
        winit::keyboard::Key::Character(text) if text.eq_ignore_ascii_case("s") => {
            Some(("Secret key", identity.nsec.clone()))
        }
        _ => None,
    }
}

impl PickerView {
    fn load(
        password: String,
        launch_join: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
        let maps_root = resolve_maps_root(config.maps_dir.as_deref(), None);
        let mut picker = Self {
            session: Some(load_picker_session(password)?),
            state: PickerState::new(
                load_cache().unwrap_or_else(|_| GameCache::empty()),
                maps_root,
            ),
            rt: tokio::runtime::Runtime::new()?,
            gate_npub: String::new(),
            lock_timeout_minutes: config.effective_lock_timeout_minutes(),
            last_activity: Instant::now(),
            next_locked_frame: Instant::now() + LOCKED_FRAME_STEP,
        };
        if let Some(invite) = launch_join {
            join_with_code(&mut picker.state, &invite, &picker.gate_npub)?;
        }
        Ok(picker)
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        if is_manual_lock_key(key, self.text_entry_active()) {
            self.lock();
            return Ok(());
        }
        self.last_activity = Instant::now();

        if self.state.overlay.is_some() {
            handle_overlay_key(
                key,
                &mut self.state,
                self.session.as_mut(),
                &self.gate_npub,
                Some(&self.rt),
            )?;
            return Ok(());
        }

        match self.state.screen.clone() {
            Screen::GameList => {
                let session = self
                    .session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_game_list_key(key, &mut self.state, session, &self.gate_npub, &self.rt)?;
            }
            Screen::RelayList | Screen::RelayGames { .. } => {
                handle_relay_key(key, &mut self.state)?;
            }
            Screen::IdentityOverlay => handle_identity_overlay_key(key, &mut self.state),
            Screen::WalletList | Screen::WalletAddPrompt => {
                let session = self
                    .session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_wallet_key(key, &mut self.state, session)?;
            }
            Screen::GameSelect { .. } => {
                handle_game_select_key(key, &mut self.state)?;
            }
            Screen::Locked => {}
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut redraw = false;
        if let Some(session) = self.session.as_ref() {
            if let Some(request) = self.state.pending_refresh.as_ref() {
                if request.is_ready() {
                    execute_pending_refresh(&mut self.state, session, &self.rt)?;
                    redraw = true;
                }
            }
        }
        if let Some(session) = self.session.as_mut() {
            if self.state.pending_connect.is_some() {
                start_pending_connect(&mut self.state, session)?;
            }
        }
        if matches!(self.state.screen, Screen::Locked) {
            let now = Instant::now();
            if now >= self.next_locked_frame {
                self.state.matrix.advance();
                self.next_locked_frame = now + LOCKED_FRAME_STEP;
                redraw = true;
            }
            return Ok(redraw);
        }
        if should_lock_for_idle(self.lock_timeout_minutes, self.last_activity) {
            self.lock();
            return Ok(true);
        }
        Ok(redraw)
    }

    fn control_flow(&self) -> winit::event_loop::ControlFlow {
        if let Some(request) = self.state.pending_refresh.as_ref() {
            return winit::event_loop::ControlFlow::WaitUntil(
                Instant::now() + request.remaining_until_execute(),
            );
        }
        if self.state.active_connect.is_some() {
            return winit::event_loop::ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(50),
            );
        }
        if matches!(self.state.screen, Screen::Locked) {
            return winit::event_loop::ControlFlow::WaitUntil(self.next_locked_frame);
        }
        if self.lock_timeout_minutes == 0 {
            return winit::event_loop::ControlFlow::Wait;
        }
        let timeout = Duration::from_secs(u64::from(self.lock_timeout_minutes) * 60);
        let deadline = self.last_activity + timeout;
        winit::event_loop::ControlFlow::WaitUntil(
            deadline.min(Instant::now() + Duration::from_millis(250)),
        )
    }

    fn take_prepared_connect(
        &mut self,
    ) -> Result<
        Option<(
            PendingConnectRequest,
            crate::connect::session::PreparedLiveSession,
            crate::connect::session::PreparedSessionFinalizer,
            String,
        )>,
        Box<dyn std::error::Error>,
    > {
        let Some(result) = poll_active_connect_result(&mut self.state)? else {
            return Ok(None);
        };
        match result {
            ConnectTaskResult::Prepared { request, prepared } => {
                let username = self
                    .session
                    .as_ref()
                    .ok_or("picker session missing while entering live session")?
                    .npub
                    .clone();
                let (prepared, finalizer) = prepared.split();
                Ok(Some((request, prepared, finalizer, username)))
            }
            ConnectTaskResult::Outcome { request, outcome } => {
                apply_connect_outcome(&mut self.state, request, outcome)?;
                Ok(None)
            }
        }
    }

    fn lock(&mut self) {
        self.session = None;
        self.state.overlay = None;
        self.state.screen = Screen::Locked;
        self.state.join_input.clear();
        self.state.maps_input.clear();
        self.state.maps_input_prefilled = false;
        self.state.wallet_input.clear();
        self.state.relay_input.clear();
        self.state.pending_connect = None;
        self.state.active_connect = None;
        self.state.pending_refresh = None;
        self.state.matrix.reset();
        self.next_locked_frame = Instant::now() + LOCKED_FRAME_STEP;
    }

    fn text_entry_active(&self) -> bool {
        matches!(self.state.screen, Screen::WalletAddPrompt)
            || matches!(
                self.state.overlay,
                Some(crate::picker::overlay::PickerOverlay::RelayEditor { .. })
                    | Some(crate::picker::overlay::PickerOverlay::GameRelayPrompt { .. })
                    | Some(crate::picker::overlay::PickerOverlay::JoinCodePopup { .. })
                    | Some(crate::picker::overlay::PickerOverlay::MapsDownloadPrompt { .. })
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{App, AppView, LaunchIntent, PickerView};
    use crate::cache::GameCache;
    use crate::picker::overlay::PickerOverlay;
    use crate::picker::state::{PickerState, Screen};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn picker_for_join_popup() -> PickerView {
        let mut state = PickerState::new(GameCache::empty(), PathBuf::from("/tmp/ec/maps"));
        state.screen = Screen::GameList;
        state.overlay = Some(PickerOverlay::JoinCodePopup { error: None });
        PickerView {
            session: None,
            state,
            rt: tokio::runtime::Runtime::new().expect("runtime"),
            gate_npub: String::new(),
            lock_timeout_minutes: 5,
            last_activity: Instant::now(),
            next_locked_frame: Instant::now() + Duration::from_millis(80),
        }
    }

    #[test]
    fn apply_pasted_text_filters_line_breaks_for_password_prompt() {
        let mut app = App::new(LaunchIntent::Normal).expect("app");

        app.apply_pasted_text("ab\r\ncd\n").expect("paste");

        let AppView::Password(password) = &app.view else {
            panic!("expected password view");
        };
        assert_eq!(password.state.input, "abcd");
    }

    #[test]
    fn apply_pasted_text_feeds_join_popup_input() {
        let mut app = App::new(LaunchIntent::Normal).expect("app");
        app.view = AppView::Picker(picker_for_join_popup());

        app.apply_pasted_text("amber-river@relay.example.com\r\n")
            .expect("paste");

        let AppView::Picker(picker) = &app.view else {
            panic!("expected picker view");
        };
        assert_eq!(picker.state.join_input, "amber-river@relay.example.com");
    }

    #[test]
    fn right_click_pastes_into_password_prompt_when_clipboard_is_available() {
        let mut app = App::new(LaunchIntent::Normal).expect("app");
        app.clipboard
            .set_text("ab\r\ncd\n".to_string())
            .expect("clipboard write");
        if app.clipboard.get_text().expect("clipboard read").is_none() {
            return;
        }
        app.handle_mouse_button(winit::event::MouseButton::Right, true)
            .expect("right click paste");

        let AppView::Password(password) = &app.view else {
            panic!("expected password view");
        };
        assert_eq!(password.state.input, "abcd");
    }
}

impl LiveView {
    fn new(
        picker: PickerView,
        request: PendingConnectRequest,
        prepared: crate::connect::session::PreparedLiveSession,
        finalizer: crate::connect::session::PreparedSessionFinalizer,
        username: String,
    ) -> Self {
        Self {
            picker,
            request,
            terminal: TerminalView::new(prepared, finalizer, username),
        }
    }

    fn finish(mut self) -> Result<PickerView, Box<dyn std::error::Error>> {
        let (finalizer, bridge_result) = self.terminal.take_finished();
        let outcome = self.picker.rt.block_on(finalizer.finish(bridge_result));
        apply_connect_outcome(&mut self.picker.state, self.request, outcome)?;
        self.picker.last_activity = Instant::now();
        Ok(self.picker)
    }
}

fn handle_password_key(
    state: &mut PasswordGateState,
    event: &winit::event::KeyEvent,
) -> PasswordAction {
    if !is_key_press(event) {
        return PasswordAction::None;
    }
    match &event.logical_key {
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => PasswordAction::Cancel,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
            state.backspace();
            PasswordAction::None
        }
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => match state.submit() {
            GateSubmit::Pending => PasswordAction::None,
            GateSubmit::Accepted(password) => PasswordAction::Submit(password),
        },
        _ => {
            if let Some(text) = event.text.as_ref() {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    state.push_char(ch);
                }
            }
            PasswordAction::None
        }
    }
}

fn should_lock_for_idle(lock_timeout_minutes: u16, last_activity: Instant) -> bool {
    lock_timeout_minutes != 0
        && last_activity.elapsed() >= Duration::from_secs(u64::from(lock_timeout_minutes) * 60)
}

fn is_manual_lock_key(key: KeyEvent, text_entry: bool) -> bool {
    let alt_l = matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('l' | 'L'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::ALT)
    );
    let plain_l = matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('l' | 'L'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
    );
    alt_l || (plain_l && !text_entry)
}
