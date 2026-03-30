use std::env;
use std::num::NonZeroU32;
use std::path::Path;

use font8x8::UnicodeFonts;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

use crate::cache::load_cache;
use crate::companion::{
    command_for_cached_game, command_for_invite, companion_binary_path, write_password_handoff_file,
};
use crate::config::{ConnectConfig, load_config};
use crate::launcher::render as gate_render;
use crate::launcher::{GateSubmit, PasswordGateState};
use crate::password::wallet_exists;
use crate::picker::flows::{move_selection, queue_selected_game_refresh};
use crate::picker::overlay::{NoticeLevel, PickerOverlay};
use crate::picker::refresh::execute_pending_refresh;
use crate::picker::render as picker_render;
use crate::picker::session::load_picker_session;
use crate::picker::state::{PickerSession, PickerState, Screen};
use crate::wallet::io::{now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{Wallet, push_new_identity};

const COLS: u16 = 82;
const ROWS: u16 = 27;
const CELL_WIDTH: usize = 8;
const CELL_HEIGHT: usize = 16;
const WINDOW_TITLE: &str = "Esterian Conquest";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().len() > 1 {
        return hand_off_cli_args();
    }

    let event_loop = EventLoop::new()?;
    let logical_width = f64::from(COLS) * CELL_WIDTH as f64;
    let logical_height = f64::from(ROWS) * CELL_HEIGHT as f64;
    let window = Box::new(
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(logical_width, logical_height))
            .with_resizable(false)
            .build(&event_loop)?,
    );
    let window: &'static winit::window::Window = Box::leak(window);

    let context = softbuffer::Context::new(window)?;
    let mut surface = softbuffer::Surface::new(&context, window)?;
    let mut app = WindowsApp::new()?;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    if let (Some(width), Some(height)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        let _ = surface.resize(width, height);
                    }
                    app.needs_redraw = true;
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }
                    if let Some(key) = map_key(&event.logical_key) {
                        if let Err(err) = app.handle_key(key) {
                            app.show_error(err.to_string());
                        }
                        if app.exit_requested {
                            elwt.exit();
                        } else {
                            app.needs_redraw = true;
                            window.request_redraw();
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Err(err) = render_window(&mut surface, &app) {
                        show_fatal_error(&format!("unable to render ec-connect window: {err}"));
                        elwt.exit();
                    } else {
                        app.needs_redraw = false;
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                if app.needs_redraw {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

pub fn show_fatal_error(message: &str) {
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    use winapi::um::winuser::{MB_ICONERROR, MB_OK, MessageBoxW};

    let title: Vec<u16> = OsStr::new("ec-connect")
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let body: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

struct WindowsApp {
    view: AppView,
    needs_redraw: bool,
    exit_requested: bool,
}

enum AppView {
    Password(PasswordGateState),
    Picker(PickerApp),
}

struct PickerApp {
    session: PickerSession,
    state: PickerState,
    rt: tokio::runtime::Runtime,
    gate_npub: String,
}

#[derive(Clone, Copy)]
enum UiKey {
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Esc,
    Backspace,
    Space,
    Char(char),
}

impl WindowsApp {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            view: AppView::Password(PasswordGateState::new(wallet_exists(&wallet_path()), None)),
            needs_redraw: true,
            exit_requested: false,
        })
    }

    fn show_error(&mut self, message: String) {
        match &mut self.view {
            AppView::Password(state) => state.error_msg = Some(format!("Error: {message}")),
            AppView::Picker(picker) => picker.state.show_error(message),
        }
    }

    fn handle_key(&mut self, key: UiKey) -> Result<(), Box<dyn std::error::Error>> {
        let mut exit_requested = false;
        let mut next_password = None;
        match &mut self.view {
            AppView::Password(state) => match key {
                UiKey::Esc | UiKey::Char('q' | 'Q') if state.input.is_empty() => {
                    exit_requested = true;
                }
                UiKey::Backspace => state.backspace(),
                UiKey::Enter => {
                    let existing_wallet = wallet_exists(&wallet_path());
                    if let GateSubmit::Accepted(password) = state.submit() {
                        if !existing_wallet {
                            let mut wallet = Wallet::empty();
                            push_new_identity(&mut wallet, now_iso8601())?;
                            save_wallet_to(&wallet, &password, &wallet_path())?;
                        }
                        next_password = Some(password);
                    }
                }
                UiKey::Char(ch) => state.push_char(ch),
                _ => {}
            },
            AppView::Picker(picker) => {
                if Self::handle_picker_key(picker, key)? {
                    exit_requested = true;
                }
            }
        }
        if let Some(password) = next_password {
            self.view = AppView::Picker(PickerApp::load(password)?);
        }
        if exit_requested {
            self.exit_requested = true;
        }
        Ok(())
    }

    fn handle_picker_key(
        picker: &mut PickerApp,
        key: UiKey,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(PickerOverlay::JoinCodePopup { error }) = picker.state.overlay.as_mut() {
            match key {
                UiKey::Esc => {
                    picker.state.overlay = None;
                    picker.state.join_input.clear();
                }
                UiKey::Backspace => {
                    picker.state.join_input.pop();
                    *error = None;
                }
                UiKey::Enter => {
                    let code = picker.state.join_input.trim().to_string();
                    if code.is_empty() {
                        *error = Some("invite code must not be empty".to_string());
                    } else {
                        picker.launch_invite(&code)?;
                        return Ok(true);
                    }
                }
                UiKey::Char(ch) => {
                    picker.state.join_input.push(ch);
                    *error = None;
                }
                _ => {}
            }
            return Ok(false);
        }

        if picker.dismiss_simple_overlay(key) {
            if picker.state.quit {
                return Ok(true);
            }
            return Ok(false);
        }

        match picker.state.screen {
            Screen::IdentityOverlay => {
                picker.state.screen = Screen::GameList;
            }
            Screen::GameList => match key {
                UiKey::Esc | UiKey::Char('q' | 'Q') => picker.state.request_quit(),
                UiKey::Char('i' | 'I') => picker.state.screen = Screen::IdentityOverlay,
                UiKey::Char('n' | 'N') => {
                    picker.state.join_input.clear();
                    picker.state.overlay = Some(PickerOverlay::JoinCodePopup { error: None });
                }
                UiKey::Space => picker.refresh_selected_game()?,
                UiKey::Up | UiKey::Char('k') => move_selection(
                    &mut picker.state.selected,
                    -1,
                    picker.state.cache.sorted().len(),
                ),
                UiKey::Down | UiKey::Char('j') => move_selection(
                    &mut picker.state.selected,
                    1,
                    picker.state.cache.sorted().len(),
                ),
                UiKey::PageUp => move_selection(
                    &mut picker.state.selected,
                    -10,
                    picker.state.cache.sorted().len(),
                ),
                UiKey::PageDown => move_selection(
                    &mut picker.state.selected,
                    10,
                    picker.state.cache.sorted().len(),
                ),
                UiKey::Char('h' | 'H' | '?') => picker.state.open_help(),
                UiKey::Enter => {
                    picker.launch_selected_game()?;
                    return Ok(true);
                }
                _ => {}
            },
            _ => {
                picker.state.show_notice(
                    "This Windows GUI beta only supports game list, join, refresh, and connect. Use ec-connect-cli.exe for full terminal management.",
                );
                picker.state.screen = Screen::GameList;
            }
        }

        Ok(false)
    }

    fn current_buffer(&self) -> ec_ui::buffer::PlayfieldBuffer {
        match &self.view {
            AppView::Password(state) => gate_render::render_buffer(state, COLS, ROWS),
            AppView::Picker(picker) => {
                picker_render::render_buffer(&picker.state, Some(&picker.session), COLS, ROWS)
            }
        }
    }
}

impl PickerApp {
    fn load(password: String) -> Result<Self, Box<dyn std::error::Error>> {
        let session = load_picker_session(password)?;
        let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
        let maps_root = crate::map_store::resolve_maps_root(config.maps_dir.as_deref(), None);
        Ok(Self {
            session,
            state: PickerState::new(
                load_cache().unwrap_or_else(|_| crate::cache::GameCache::empty()),
                maps_root,
            ),
            rt: tokio::runtime::Runtime::new()?,
            gate_npub: String::new(),
        })
    }

    fn dismiss_simple_overlay(&mut self, key: UiKey) -> bool {
        match self.state.overlay.clone() {
            Some(PickerOverlay::Notice { .. })
            | Some(PickerOverlay::Help(_))
            | Some(PickerOverlay::MapsDownloaded { .. }) => {
                self.state.overlay = None;
                true
            }
            Some(PickerOverlay::QuitConfirm) => {
                match key {
                    UiKey::Enter | UiKey::Char('y' | 'Y') => {
                        self.state.overlay = None;
                        self.state.quit = true;
                    }
                    UiKey::Esc | UiKey::Char('n' | 'N' | 'q' | 'Q') => {
                        self.state.overlay = None;
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    fn refresh_selected_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        queue_selected_game_refresh(&mut self.state, &self.gate_npub)?;
        execute_pending_refresh(&mut self.state, &self.session, &self.rt)?;
        if !matches!(
            self.state.overlay,
            Some(PickerOverlay::Notice {
                level: NoticeLevel::Error,
                ..
            })
        ) {
            self.state.show_notice("Game info refreshed.");
        }
        Ok(())
    }

    fn launch_selected_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let sorted = self.state.cache.sorted();
        let Some(game) = sorted.get(self.state.selected).copied() else {
            self.state.show_error("No joined games yet.");
            return Ok(());
        };
        let password_file = write_password_handoff_file(&self.session.password)?;
        let current_exe = env::current_exe()?;
        let mut command = command_for_cached_game(&current_exe, game, &password_file);
        spawn_companion(&mut command, &current_exe)
    }

    fn launch_invite(&mut self, invite_code: &str) -> Result<(), Box<dyn std::error::Error>> {
        let password_file = write_password_handoff_file(&self.session.password)?;
        let current_exe = env::current_exe()?;
        let mut command = command_for_invite(&current_exe, invite_code, &password_file);
        spawn_companion(&mut command, &current_exe)
    }
}

fn hand_off_cli_args() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let companion = companion_binary_path(&current_exe);
    if !companion.exists() {
        return Err(format!("missing console companion: {}", companion.display()).into());
    }
    let mut command = std::process::Command::new(companion);
    command.args(env::args().skip(1));
    command.spawn()?;
    Ok(())
}

fn spawn_companion(
    command: &mut std::process::Command,
    current_exe: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let companion = companion_binary_path(current_exe);
    if !companion.exists() {
        return Err(format!("missing console companion: {}", companion.display()).into());
    }
    command.spawn()?;
    Ok(())
}

fn render_window(
    surface: &mut softbuffer::Surface<&winit::window::Window, &winit::window::Window>,
    app: &WindowsApp,
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = app.current_buffer();
    let pixel_width = buffer.width() * CELL_WIDTH;
    let pixel_height = buffer.height() * CELL_HEIGHT;
    surface.resize(
        NonZeroU32::new(pixel_width as u32).ok_or("pixel width must be non-zero")?,
        NonZeroU32::new(pixel_height as u32).ok_or("pixel height must be non-zero")?,
    )?;
    let mut frame = surface.buffer_mut()?;
    draw_buffer(&buffer, &mut frame, pixel_width);
    frame.present()?;
    Ok(())
}

fn draw_buffer(buffer: &ec_ui::buffer::PlayfieldBuffer, frame: &mut [u32], stride: usize) {
    for row in 0..buffer.height() {
        for col in 0..buffer.width() {
            let cell = buffer.row(row)[col];
            let bg = pack_color(cell.style.bg);
            let fg = pack_color(cell.style.fg);
            let glyph = glyph_bitmap(cell.ch);
            let x0 = col * CELL_WIDTH;
            let y0 = row * CELL_HEIGHT;
            for y in 0..CELL_HEIGHT {
                let glyph_row = glyph[(y / 2).min(7)];
                let dest = (y0 + y) * stride + x0;
                for x in 0..CELL_WIDTH {
                    let mask = 1u8 << x;
                    frame[dest + x] = if glyph_row & mask != 0 { fg } else { bg };
                }
            }
        }
    }
}

fn glyph_bitmap(ch: char) -> [u8; 8] {
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::BOX_FONTS.get(ch))
        .or_else(|| font8x8::BLOCK_FONTS.get(ch))
        .or_else(|| font8x8::GREEK_FONTS.get(ch))
        .unwrap_or_else(|| font8x8::BASIC_FONTS.get('?').expect("question-mark glyph"))
}

fn pack_color(color: ec_ui::buffer::GameColor) -> u32 {
    let (r, g, b) = match color {
        ec_ui::buffer::GameColor::Black => (0x00, 0x00, 0x00),
        ec_ui::buffer::GameColor::Red => (0x80, 0x00, 0x00),
        ec_ui::buffer::GameColor::Green => (0x00, 0x80, 0x00),
        ec_ui::buffer::GameColor::Yellow => (0x80, 0x80, 0x00),
        ec_ui::buffer::GameColor::Blue => (0x00, 0x00, 0x80),
        ec_ui::buffer::GameColor::Magenta => (0x80, 0x00, 0x80),
        ec_ui::buffer::GameColor::Cyan => (0x00, 0x80, 0x80),
        ec_ui::buffer::GameColor::White => (0xc0, 0xc0, 0xc0),
        ec_ui::buffer::GameColor::BrightBlack => (0x80, 0x80, 0x80),
        ec_ui::buffer::GameColor::BrightRed => (0xff, 0x00, 0x00),
        ec_ui::buffer::GameColor::BrightGreen => (0x00, 0xff, 0x00),
        ec_ui::buffer::GameColor::BrightYellow => (0xff, 0xff, 0x00),
        ec_ui::buffer::GameColor::BrightBlue => (0x00, 0x00, 0xff),
        ec_ui::buffer::GameColor::BrightMagenta => (0xff, 0x00, 0xff),
        ec_ui::buffer::GameColor::BrightCyan => (0x00, 0xff, 0xff),
        ec_ui::buffer::GameColor::BrightWhite => (0xff, 0xff, 0xff),
        ec_ui::buffer::GameColor::Indexed(index) => indexed_color(index),
        ec_ui::buffer::GameColor::Rgb(r, g, b) => (r, g, b),
    };
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn indexed_color(index: u8) -> (u8, u8, u8) {
    match index {
        0 => (0x00, 0x00, 0x00),
        1 => (0x80, 0x00, 0x00),
        2 => (0x00, 0x80, 0x00),
        3 => (0x80, 0x80, 0x00),
        4 => (0x00, 0x00, 0x80),
        5 => (0x80, 0x00, 0x80),
        6 => (0x00, 0x80, 0x80),
        7 => (0xc0, 0xc0, 0xc0),
        _ => (index, index, index),
    }
}

fn map_key(key: &Key) -> Option<UiKey> {
    match key.as_ref() {
        Key::Named(NamedKey::ArrowUp) => Some(UiKey::Up),
        Key::Named(NamedKey::ArrowDown) => Some(UiKey::Down),
        Key::Named(NamedKey::PageUp) => Some(UiKey::PageUp),
        Key::Named(NamedKey::PageDown) => Some(UiKey::PageDown),
        Key::Named(NamedKey::Enter) => Some(UiKey::Enter),
        Key::Named(NamedKey::Escape) => Some(UiKey::Esc),
        Key::Named(NamedKey::Backspace) => Some(UiKey::Backspace),
        Key::Character(" ") => Some(UiKey::Space),
        Key::Character(text) => text.chars().next().map(UiKey::Char),
        _ => None,
    }
}
