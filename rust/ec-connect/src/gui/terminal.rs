use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, RenderableContent, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Processor as AnsiProcessor, Rgb};
use ec_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use crate::connect::bridge::BridgeError;
use crate::connect::live::{LiveEvent, LiveSession, TerminalSpec};
use crate::connect::session::{PreparedLiveSession, PreparedSessionFinalizer};
use crate::shell::wrap_inner_buffer_in_terminal;

use super::clipboard::Clipboard;
use super::input::{encode_paste, terminal_key_bytes};
use super::{CELL_HEIGHT, CELL_WIDTH, OUTER_COLS, OUTER_ROWS, TERM_COLS, TERM_ROWS};

#[derive(Clone, Default)]
struct EventQueue(Arc<Mutex<Vec<TermEvent>>>);

impl EventListener for EventQueue {
    fn send_event(&self, event: TermEvent) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push(event);
        }
    }
}

pub struct TerminalView {
    live: LiveSession,
    finalizer: PreparedSessionFinalizer,
    term: Term<EventQueue>,
    parser: AnsiProcessor,
    events: EventQueue,
    title: Option<String>,
    selection_drag: bool,
    finished: Option<Result<u32, BridgeError>>,
}

impl TerminalView {
    pub fn new(
        prepared: PreparedLiveSession,
        finalizer: PreparedSessionFinalizer,
        _username: String,
    ) -> Self {
        let events = EventQueue::default();
        let mut term = Term::new(
            Config::default(),
            &TermSize::new(TERM_COLS as usize, TERM_ROWS as usize),
            events.clone(),
        );
        term.resize(TermSize::new(TERM_COLS as usize, TERM_ROWS as usize));
        let live = LiveSession::start(
            prepared.payload,
            prepared.keypair,
            _username.clone(),
            TerminalSpec {
                term: "xterm-256color".to_string(),
                cols: TERM_COLS,
                rows: TERM_ROWS,
            },
        );
        Self {
            live,
            finalizer,
            term,
            parser: AnsiProcessor::new(),
            events,
            title: None,
            selection_drag: false,
            finished: None,
        }
    }

    pub fn finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn take_finished(self) -> (PreparedSessionFinalizer, Result<u32, BridgeError>) {
        (
            self.finalizer,
            self.finished
                .expect("take_finished called before live session completed"),
        )
    }

    pub fn close(&self) {
        self.live.close();
    }

    pub fn paste_text(&mut self, text: &str) {
        let bytes = encode_paste(text, self.term.mode().contains(TermMode::BRACKETED_PASTE));
        self.live.send_input(bytes);
    }

    pub fn render_buffer(&self, identity_label: &str) -> PlayfieldBuffer {
        let mut inner = PlayfieldBuffer::new(
            TERM_COLS as usize,
            TERM_ROWS as usize,
            CellStyle::new(GameColor::White, GameColor::Black, false),
        );
        let content = self.term.renderable_content();
        let cursor = content.cursor;
        populate_terminal_buffer(&mut inner, content);
        if cursor.shape != CursorShape::Hidden {
            inner.set_cursor(cursor.point.column.0 as u16, cursor.point.line.0 as u16);
        }
        wrap_inner_buffer_in_terminal(
            &inner,
            Some(identity_label),
            OUTER_COLS as usize,
            OUTER_ROWS as usize,
            None,
        )
    }

    pub fn tick(
        &mut self,
        clipboard: &mut Clipboard,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok(event) = self.live.try_recv() {
            match event {
                LiveEvent::Output(data) => {
                    self.term.selection = None;
                    self.parser.advance(&mut self.term, &data);
                }
                LiveEvent::Exit(code) => {
                    self.finished = Some(Ok(code));
                    break;
                }
                LiveEvent::Error(err) => {
                    self.finished = Some(Err(err.into()));
                    break;
                }
            }
        }
        self.handle_term_events(clipboard)?;
        Ok(())
    }

    pub fn handle_key(
        &mut self,
        event: &winit::event::KeyEvent,
        modifiers: winit::keyboard::ModifiersState,
        clipboard: &mut Clipboard,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if super::input::is_copy_shortcut(event, modifiers) {
            if let Some(selection) = self.term.selection_to_string() {
                clipboard.set_text(selection)?;
            }
            return Ok(true);
        }
        if super::input::is_paste_shortcut(event, modifiers) {
            if let Some(text) = clipboard.get_text()? {
                let bytes = encode_paste(&text, self.term.mode().contains(TermMode::BRACKETED_PASTE));
                self.live.send_input(bytes);
            }
            return Ok(true);
        }
        if let Some(bytes) = terminal_key_bytes(event, modifiers, *self.term.mode()) {
            self.live.send_input(bytes);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn handle_mouse_move(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.selection_drag {
            return Ok(false);
        }
        let Some(point) = pixel_to_terminal_point(position) else {
            return Ok(false);
        };
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, Side::Right);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn handle_mouse_button(
        &mut self,
        pressed: bool,
        position: winit::dpi::PhysicalPosition<f64>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(point) = pixel_to_terminal_point(position) else {
            self.selection_drag = false;
            return Ok(false);
        };
        if pressed {
            self.term.selection = Some(Selection::new(SelectionType::Simple, point, Side::Left));
            self.selection_drag = true;
            return Ok(true);
        }
        self.selection_drag = false;
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, Side::Right);
            return Ok(true);
        }
        Ok(false)
    }

    fn handle_term_events(
        &mut self,
        clipboard: &mut Clipboard,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let queued = self
            .events
            .0
            .lock()
            .map_err(|_| "terminal event queue poisoned")?
            .drain(..)
            .collect::<Vec<_>>();
        for event in queued {
            match event {
                TermEvent::Title(title) => self.title = Some(title),
                TermEvent::ResetTitle => self.title = None,
                TermEvent::PtyWrite(text) => self.live.send_input(text.into_bytes()),
                TermEvent::ClipboardStore(_, text) => {
                    let _ = clipboard.set_text(text);
                }
                TermEvent::ClipboardLoad(_, formatter) => {
                    if let Some(text) = clipboard.get_text()? {
                        self.live.send_input(formatter(&text).into_bytes());
                    }
                }
                TermEvent::TextAreaSizeRequest(formatter) => {
                    let response = formatter(WindowSize {
                        num_lines: TERM_ROWS,
                        num_cols: TERM_COLS,
                        cell_width: CELL_WIDTH as u16,
                        cell_height: CELL_HEIGHT as u16,
                    });
                    self.live.send_input(response.into_bytes());
                }
                TermEvent::Exit => {
                    self.live.close();
                }
                TermEvent::ChildExit(code) => {
                    self.finished = Some(Ok(code as u32));
                }
                TermEvent::Wakeup
                | TermEvent::Bell
                | TermEvent::MouseCursorDirty
                | TermEvent::CursorBlinkingChange
                | TermEvent::ColorRequest(_, _) => {}
            }
        }
        Ok(())
    }
}

fn populate_terminal_buffer(buffer: &mut PlayfieldBuffer, content: RenderableContent<'_>) {
    for indexed in content.display_iter {
        let mut fg = resolve_color(indexed.cell.fg, content.colors);
        let mut bg = resolve_color(indexed.cell.bg, content.colors);
        let selected = content
            .selection
            .as_ref()
            .map(|selection| selection.contains_cell(&indexed, content.cursor.point, content.cursor.shape))
            .unwrap_or(false);
        if selected {
            std::mem::swap(&mut fg, &mut bg);
        }
        let ch = if indexed.cell.flags.contains(Flags::HIDDEN)
            || indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER)
            || indexed.cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
        {
            ' '
        } else {
            indexed.cell.c
        };
        buffer.set_cell(
            indexed.point.line.0 as usize,
            indexed.point.column.0,
            ch,
            CellStyle::new(fg, bg, indexed.cell.flags.contains(Flags::BOLD)),
        );
    }
}

fn pixel_to_terminal_point(position: winit::dpi::PhysicalPosition<f64>) -> Option<Point> {
    if position.x < 0.0 || position.y < 0.0 {
        return None;
    }
    let col = (position.x as usize) / CELL_WIDTH;
    let row = (position.y as usize) / CELL_HEIGHT;
    if !(1..=TERM_COLS as usize).contains(&col) || !(1..=TERM_ROWS as usize).contains(&row) {
        return None;
    }
    Some(Point::new(
        Line((row - 1) as i32),
        Column(col - 1),
    ))
}

fn resolve_color(color: Color, colors: &alacritty_terminal::term::color::Colors) -> GameColor {
    match color {
        Color::Spec(Rgb { r, g, b }) => GameColor::Rgb(r, g, b),
        Color::Indexed(index) => GameColor::Rgb(
            indexed_color(index).0,
            indexed_color(index).1,
            indexed_color(index).2,
        ),
        Color::Named(named) => colors[named]
            .map(|Rgb { r, g, b }| GameColor::Rgb(r, g, b))
            .unwrap_or_else(|| named_color(named)),
    }
}

fn named_color(named: NamedColor) -> GameColor {
    match named {
        NamedColor::Black => GameColor::Black,
        NamedColor::Red => GameColor::Red,
        NamedColor::Green => GameColor::Green,
        NamedColor::Yellow => GameColor::Yellow,
        NamedColor::Blue => GameColor::Blue,
        NamedColor::Magenta => GameColor::Magenta,
        NamedColor::Cyan => GameColor::Cyan,
        NamedColor::White => GameColor::White,
        NamedColor::BrightBlack => GameColor::BrightBlack,
        NamedColor::BrightRed => GameColor::BrightRed,
        NamedColor::BrightGreen => GameColor::BrightGreen,
        NamedColor::BrightYellow => GameColor::BrightYellow,
        NamedColor::BrightBlue => GameColor::BrightBlue,
        NamedColor::BrightMagenta => GameColor::BrightMagenta,
        NamedColor::BrightCyan => GameColor::BrightCyan,
        NamedColor::BrightWhite | NamedColor::BrightForeground | NamedColor::Foreground => {
            GameColor::BrightWhite
        }
        NamedColor::Background => GameColor::Black,
        NamedColor::Cursor => GameColor::White,
        NamedColor::DimBlack => GameColor::Indexed(8),
        NamedColor::DimRed => GameColor::Rgb(0x66, 0x00, 0x00),
        NamedColor::DimGreen => GameColor::Rgb(0x00, 0x66, 0x00),
        NamedColor::DimYellow => GameColor::Rgb(0x66, 0x66, 0x00),
        NamedColor::DimBlue => GameColor::Rgb(0x00, 0x00, 0x66),
        NamedColor::DimMagenta => GameColor::Rgb(0x66, 0x00, 0x66),
        NamedColor::DimCyan => GameColor::Rgb(0x00, 0x66, 0x66),
        NamedColor::DimWhite | NamedColor::DimForeground => GameColor::White,
    }
}

fn indexed_color(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => match named_color(match index {
            0 => NamedColor::Black,
            1 => NamedColor::Red,
            2 => NamedColor::Green,
            3 => NamedColor::Yellow,
            4 => NamedColor::Blue,
            5 => NamedColor::Magenta,
            6 => NamedColor::Cyan,
            7 => NamedColor::White,
            8 => NamedColor::BrightBlack,
            9 => NamedColor::BrightRed,
            10 => NamedColor::BrightGreen,
            11 => NamedColor::BrightYellow,
            12 => NamedColor::BrightBlue,
            13 => NamedColor::BrightMagenta,
            14 => NamedColor::BrightCyan,
            _ => NamedColor::BrightWhite,
        }) {
            GameColor::Black => (0, 0, 0),
            GameColor::Red => (0x80, 0, 0),
            GameColor::Green => (0, 0x80, 0),
            GameColor::Yellow => (0x80, 0x80, 0),
            GameColor::Blue => (0, 0, 0x80),
            GameColor::Magenta => (0x80, 0, 0x80),
            GameColor::Cyan => (0, 0x80, 0x80),
            GameColor::White => (0xc0, 0xc0, 0xc0),
            GameColor::BrightBlack => (0x80, 0x80, 0x80),
            GameColor::BrightRed => (0xff, 0, 0),
            GameColor::BrightGreen => (0, 0xff, 0),
            GameColor::BrightYellow => (0xff, 0xff, 0),
            GameColor::BrightBlue => (0, 0, 0xff),
            GameColor::BrightMagenta => (0xff, 0, 0xff),
            GameColor::BrightCyan => (0, 0xff, 0xff),
            GameColor::BrightWhite => (0xff, 0xff, 0xff),
            GameColor::Indexed(_) | GameColor::Rgb(_, _, _) => unreachable!(),
        },
        16..=231 => {
            let idx = index - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;
            let expand = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            (expand(r), expand(g), expand(b))
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
    }
}
