use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::connect::handshake::SessionUiMode;
use crate::connect::bridge::BridgeError;
use crate::connect::live::{LiveEvent, LiveSession, TerminalSpec};
use crate::connect::map_push::{MapPushMonitor, MapPushMonitorResult};
use crate::connect::session::{PreparedLiveSession, PreparedSessionFinalizer};
use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, RenderableContent, Term, TermMode};
use alacritty_terminal::vte::ansi::{
    Color, CursorShape, NamedColor, Processor as AnsiProcessor, Rgb,
};
use nc_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};

use super::clipboard::Clipboard;
use super::input::{encode_paste, terminal_key_bytes};
use super::{CELL_HEIGHT, CELL_WIDTH, TERM_COLS, TERM_ROWS};

trait LiveIo {
    fn send_input(&self, data: Vec<u8>);
    fn resize(&self, cols: u16, rows: u16);
    fn close(&self);
    fn try_recv(&mut self) -> Result<Option<LiveEvent>, String>;
}

struct SessionLiveIo {
    session: LiveSession,
}

impl LiveIo for SessionLiveIo {
    fn send_input(&self, data: Vec<u8>) {
        self.session.send_input(data);
    }

    fn resize(&self, cols: u16, rows: u16) {
        self.session.resize(cols, rows);
    }

    fn close(&self) {
        self.session.close();
    }

    fn try_recv(&mut self) -> Result<Option<LiveEvent>, String> {
        match self.session.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

enum SessionFinalizer {
    Real(PreparedSessionFinalizer),
    #[cfg(test)]
    Test,
}

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
    session_ui: SessionUiMode,
    live: Box<dyn LiveIo>,
    finalizer: SessionFinalizer,
    term: Term<EventQueue>,
    parser: AnsiProcessor,
    events: EventQueue,
    title: Option<String>,
    selection_drag: bool,
    finished: Option<Result<u32, BridgeError>>,
    map_push_monitor: Option<MapPushMonitor>,
    has_received_output: bool,
    created_at: Instant,
    last_output_at: Option<Instant>,
    viewport_cols: u16,
    viewport_rows: u16,
    viewport_pixel_width: u32,
    viewport_pixel_height: u32,
    terminal_cols: u16,
    terminal_rows: u16,
}

impl TerminalView {
    pub fn new(
        prepared: PreparedLiveSession,
        finalizer: PreparedSessionFinalizer,
        username: String,
        viewport_cols: u16,
        viewport_rows: u16,
        viewport_pixel_width: u32,
        viewport_pixel_height: u32,
    ) -> Self {
        let map_push_monitor = prepared.map_push_config.clone().map(MapPushMonitor::start);
        let events = EventQueue::default();
        let session_ui = prepared.payload.session_ui;
        let (terminal_cols, terminal_rows) = match session_ui {
            SessionUiMode::ClassicNcGame => (TERM_COLS, TERM_ROWS),
            SessionUiMode::FullscreenNcDash => (viewport_cols.max(1), viewport_rows.max(1)),
        };
        let mut term = Term::new(
            Config::default(),
            &TermSize::new(terminal_cols as usize, terminal_rows as usize),
            events.clone(),
        );
        term.resize(TermSize::new(terminal_cols as usize, terminal_rows as usize));
        let live = LiveSession::start(
            prepared.payload,
            prepared.keypair,
            username,
            TerminalSpec {
                term: "xterm-256color".to_string(),
                cols: terminal_cols,
                rows: terminal_rows,
            },
        );
        Self {
            session_ui,
            live: Box::new(SessionLiveIo { session: live }),
            finalizer: SessionFinalizer::Real(finalizer),
            term,
            parser: AnsiProcessor::new(),
            events,
            title: None,
            selection_drag: false,
            finished: None,
            map_push_monitor,
            has_received_output: false,
            created_at: Instant::now(),
            last_output_at: None,
            viewport_cols: viewport_cols.max(1),
            viewport_rows: viewport_rows.max(1),
            viewport_pixel_width: viewport_pixel_width.max(1),
            viewport_pixel_height: viewport_pixel_height.max(1),
            terminal_cols,
            terminal_rows,
        }
    }

    pub fn finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn take_finished(
        mut self,
    ) -> (
        PreparedSessionFinalizer,
        Result<u32, BridgeError>,
        MapPushMonitorResult,
    ) {
        (
            match self.finalizer {
                SessionFinalizer::Real(finalizer) => finalizer,
                #[cfg(test)]
                SessionFinalizer::Test => {
                    panic!("take_finished called on test terminal view")
                }
            },
            self.finished
                .take()
                .expect("take_finished called before live session completed"),
            self.map_push_monitor
                .take()
                .map(MapPushMonitor::finish)
                .unwrap_or_default(),
        )
    }

    pub fn close(&self) {
        tracing::debug!("nc-connect live terminal close requested");
        self.live.close();
    }

    pub fn paste_text(&mut self, text: &str) {
        let bytes = encode_paste(text, self.term.mode().contains(TermMode::BRACKETED_PASTE));
        tracing::debug!(
            bytes_len = bytes.len(),
            idle_ms = self.idle_for().as_millis() as u64,
            "nc-connect live paste forwarded"
        );
        self.live.send_input(bytes);
    }

    pub fn render_buffer(&self) -> PlayfieldBuffer {
        let mut buffer = self.render_terminal_buffer();
        if self.session_ui == SessionUiMode::ClassicNcGame {
            buffer = center_terminal_buffer(
                &buffer,
                usize::from(self.viewport_cols),
                usize::from(self.viewport_rows),
            );
        }
        buffer
    }

    fn render_terminal_buffer(&self) -> PlayfieldBuffer {
        let mut buffer = PlayfieldBuffer::new(
            usize::from(self.terminal_cols),
            usize::from(self.terminal_rows),
            CellStyle::new(GameColor::White, GameColor::Black, false),
        );
        let content = self.term.renderable_content();
        let cursor = content.cursor;
        populate_terminal_buffer(&mut buffer, content);
        if should_render_cursor(cursor.shape, self.has_received_output) {
            buffer.set_cursor(cursor.point.column.0 as u16, cursor.point.line.0 as u16);
        }
        buffer
    }

    pub fn cursor_blink_enabled(&self) -> bool {
        self.term.cursor_style().blinking
    }

    pub fn tick(&mut self, clipboard: &mut Clipboard) -> Result<bool, Box<dyn std::error::Error>> {
        let mut redraw = false;
        while let Some(event) = self
            .live
            .try_recv()
            .map_err(|err| format!("live session poll failed: {err}"))?
        {
            match event {
                LiveEvent::Output(data) => {
                    self.has_received_output = true;
                    self.last_output_at = Some(Instant::now());
                    self.term.selection = None;
                    self.parser.advance(&mut self.term, &data);
                    tracing::debug!(
                        bytes_len = data.len(),
                        "nc-connect live remote output received"
                    );
                    redraw = true;
                }
                LiveEvent::Exit(code) => {
                    self.finished = Some(Ok(code));
                    tracing::debug!(exit_code = code, "nc-connect live session exited");
                    redraw = true;
                    break;
                }
                LiveEvent::Error(err) => {
                    tracing::debug!(error = %err, "nc-connect live session failed");
                    self.finished = Some(Err(err.into()));
                    redraw = true;
                    break;
                }
            }
        }
        redraw |= self.handle_term_events(clipboard)?;
        Ok(redraw)
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
            tracing::debug!("nc-connect live copy shortcut handled");
            return Ok(true);
        }
        if let Some(bytes) = terminal_key_bytes(event, modifiers, *self.term.mode()) {
            tracing::debug!(
                logical_key = ?event.logical_key,
                text = event.text.as_deref().unwrap_or(""),
                bytes_len = bytes.len(),
                idle_ms = self.idle_for().as_millis() as u64,
                selection_drag = self.selection_drag,
                "nc-connect live key forwarded"
            );
            self.live.send_input(bytes);
            return Ok(true);
        }
        tracing::debug!(
            logical_key = ?event.logical_key,
            selection_drag = self.selection_drag,
            "nc-connect live key ignored"
        );
        Ok(false)
    }

    pub fn handle_mouse_move(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.selection_drag {
            return Ok(false);
        }
        let Some(point) = self.pixel_to_terminal_point(position) else {
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
        let Some(point) = self.pixel_to_terminal_point(position) else {
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

    pub fn resize_viewport(
        &mut self,
        viewport_cols: u16,
        viewport_rows: u16,
        viewport_pixel_width: u32,
        viewport_pixel_height: u32,
    ) -> bool {
        let viewport_cols = viewport_cols.max(1);
        let viewport_rows = viewport_rows.max(1);
        let viewport_pixel_width = viewport_pixel_width.max(1);
        let viewport_pixel_height = viewport_pixel_height.max(1);
        let changed = self.viewport_cols != viewport_cols
            || self.viewport_rows != viewport_rows
            || self.viewport_pixel_width != viewport_pixel_width
            || self.viewport_pixel_height != viewport_pixel_height;
        self.viewport_cols = viewport_cols;
        self.viewport_rows = viewport_rows;
        self.viewport_pixel_width = viewport_pixel_width;
        self.viewport_pixel_height = viewport_pixel_height;

        if self.session_ui == SessionUiMode::FullscreenNcDash
            && (self.terminal_cols != viewport_cols || self.terminal_rows != viewport_rows)
        {
            self.terminal_cols = viewport_cols;
            self.terminal_rows = viewport_rows;
            self.term
                .resize(TermSize::new(viewport_cols as usize, viewport_rows as usize));
            self.live.resize(viewport_cols, viewport_rows);
            return true;
        }

        changed
    }

    fn handle_term_events(
        &mut self,
        clipboard: &mut Clipboard,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut redraw = false;
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
                TermEvent::PtyWrite(text) => {
                    tracing::debug!(
                        bytes_len = text.len(),
                        idle_ms = self.idle_for().as_millis() as u64,
                        "nc-connect live terminal emitted pty write"
                    );
                    self.live.send_input(text.into_bytes());
                }
                TermEvent::ClipboardStore(_, text) => {
                    let _ = clipboard.set_text(text);
                }
                TermEvent::ClipboardLoad(_, formatter) => {
                    if let Some(text) = clipboard.get_text()? {
                        tracing::debug!(
                            chars = text.chars().count(),
                            "nc-connect live clipboard load forwarded"
                        );
                        self.live.send_input(formatter(&text).into_bytes());
                    }
                }
                TermEvent::TextAreaSizeRequest(formatter) => {
                    let response = formatter(WindowSize {
                        num_lines: self.terminal_rows,
                        num_cols: self.terminal_cols,
                        cell_width: CELL_WIDTH as u16,
                        cell_height: CELL_HEIGHT as u16,
                    });
                    tracing::debug!("nc-connect live terminal answered text-area size request");
                    self.live.send_input(response.into_bytes());
                }
                TermEvent::Exit => {
                    tracing::debug!("nc-connect live terminal exit requested by terminal");
                    self.live.close();
                }
                TermEvent::ChildExit(code) => {
                    self.finished = Some(Ok(code as u32));
                    tracing::debug!(exit_code = code, "nc-connect live child exit observed");
                    redraw = true;
                }
                TermEvent::Wakeup
                | TermEvent::Bell
                | TermEvent::MouseCursorDirty
                | TermEvent::ColorRequest(_, _) => {}
                TermEvent::CursorBlinkingChange => {
                    redraw = true;
                }
            }
        }
        Ok(redraw)
    }

    pub fn idle_for(&self) -> Duration {
        self.last_output_at
            .map(|instant| instant.elapsed())
            .unwrap_or_else(|| self.created_at.elapsed())
    }

    pub fn selection_drag_active(&self) -> bool {
        self.selection_drag
    }

    fn pixel_to_terminal_point(&self, position: winit::dpi::PhysicalPosition<f64>) -> Option<Point> {
        pixel_to_terminal_point(
            position,
            self.viewport_cols,
            self.viewport_rows,
            self.viewport_pixel_width,
            self.viewport_pixel_height,
            self.terminal_cols,
            self.terminal_rows,
            self.session_ui,
        )
    }

    #[cfg(test)]
    fn forward_test_input(&mut self, bytes: Vec<u8>) -> bool {
        self.live.send_input(bytes);
        true
    }
}

fn should_render_cursor(shape: CursorShape, has_received_output: bool) -> bool {
    has_received_output && shape != CursorShape::Hidden
}

fn populate_terminal_buffer(buffer: &mut PlayfieldBuffer, content: RenderableContent<'_>) {
    for indexed in content.display_iter {
        let mut fg = resolve_color(indexed.cell.fg, content.colors);
        let mut bg = resolve_color(indexed.cell.bg, content.colors);
        let selected = content
            .selection
            .as_ref()
            .map(|selection| {
                selection.contains_cell(&indexed, content.cursor.point, content.cursor.shape)
            })
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

fn pixel_to_terminal_point(
    position: winit::dpi::PhysicalPosition<f64>,
    viewport_cols: u16,
    viewport_rows: u16,
    viewport_pixel_width: u32,
    viewport_pixel_height: u32,
    terminal_cols: u16,
    terminal_rows: u16,
    session_ui: SessionUiMode,
) -> Option<Point> {
    if position.x < 0.0 || position.y < 0.0 {
        return None;
    }

    let grid_pixel_width = usize::from(viewport_cols) * CELL_WIDTH;
    let grid_pixel_height = usize::from(viewport_rows) * CELL_HEIGHT;
    let grid_origin_x = ((viewport_pixel_width as usize).saturating_sub(grid_pixel_width)) / 2;
    let grid_origin_y = ((viewport_pixel_height as usize).saturating_sub(grid_pixel_height)) / 2;
    let x = position.x as usize;
    let y = position.y as usize;
    if x < grid_origin_x
        || y < grid_origin_y
        || x >= grid_origin_x + grid_pixel_width
        || y >= grid_origin_y + grid_pixel_height
    {
        return None;
    }

    let viewport_col = (x - grid_origin_x) / CELL_WIDTH;
    let viewport_row = (y - grid_origin_y) / CELL_HEIGHT;
    let content_col_offset = match session_ui {
        SessionUiMode::ClassicNcGame => {
            usize::from(viewport_cols.saturating_sub(terminal_cols)) / 2
        }
        SessionUiMode::FullscreenNcDash => 0,
    };
    let content_row_offset = match session_ui {
        SessionUiMode::ClassicNcGame => {
            usize::from(viewport_rows.saturating_sub(terminal_rows)) / 2
        }
        SessionUiMode::FullscreenNcDash => 0,
    };

    if viewport_col < content_col_offset || viewport_row < content_row_offset {
        return None;
    }
    let col = viewport_col - content_col_offset;
    let row = viewport_row - content_row_offset;
    if col >= terminal_cols as usize || row >= terminal_rows as usize {
        return None;
    }
    Some(Point::new(Line(row as i32), Column(col)))
}

fn center_terminal_buffer(
    inner: &PlayfieldBuffer,
    viewport_width: usize,
    viewport_height: usize,
) -> PlayfieldBuffer {
    let width = viewport_width.max(inner.width());
    let height = viewport_height.max(inner.height());
    let mut outer = PlayfieldBuffer::new(
        width,
        height,
        CellStyle::new(GameColor::White, GameColor::Black, false),
    );
    let origin_col = width.saturating_sub(inner.width()) / 2;
    let origin_row = height.saturating_sub(inner.height()) / 2;
    for row in 0..inner.height() {
        for (col, cell) in inner.row(row).iter().enumerate() {
            outer.set_cell(origin_row + row, origin_col + col, cell.ch, cell.style);
        }
    }
    if let Some((col, row)) = inner.cursor() {
        outer.set_cursor(
            col + origin_col as u16,
            row + origin_row as u16,
        );
    }
    outer
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

#[cfg(test)]
mod tests {
    use super::{
        EventQueue, LiveIo, SessionFinalizer, TERM_COLS, TERM_ROWS, TerminalView,
        center_terminal_buffer,
        should_render_cursor,
    };
    use crate::connect::handshake::SessionUiMode;
    use crate::connect::live::LiveEvent;
    use crate::gui::clipboard::Clipboard;
    use crate::gui::terminal::pixel_to_terminal_point;
    use crate::gui::{CELL_HEIGHT, CELL_WIDTH};
    use nc_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
    use alacritty_terminal::term::test::TermSize;
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::CursorShape;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalPosition;
    use winit::keyboard::ModifiersState;

    #[derive(Clone, Default)]
    struct TestLiveHandle {
        sent_inputs: Arc<Mutex<Vec<Vec<u8>>>>,
        sent_resizes: Arc<Mutex<Vec<(u16, u16)>>>,
        queued_events: Arc<Mutex<VecDeque<LiveEvent>>>,
    }

    impl TestLiveHandle {
        fn send_events(&self, events: impl IntoIterator<Item = LiveEvent>) {
            self.queued_events
                .lock()
                .expect("queued events")
                .extend(events);
        }

        fn sent_inputs(&self) -> Vec<Vec<u8>> {
            self.sent_inputs.lock().expect("sent inputs").clone()
        }

        fn sent_resizes(&self) -> Vec<(u16, u16)> {
            self.sent_resizes.lock().expect("sent resizes").clone()
        }
    }

    struct TestLiveIo {
        handle: TestLiveHandle,
    }

    impl LiveIo for TestLiveIo {
        fn send_input(&self, data: Vec<u8>) {
            self.handle
                .sent_inputs
                .lock()
                .expect("sent inputs")
                .push(data);
        }

        fn resize(&self, cols: u16, rows: u16) {
            self.handle
                .sent_resizes
                .lock()
                .expect("sent resizes")
                .push((cols, rows));
        }

        fn close(&self) {}

        fn try_recv(&mut self) -> Result<Option<LiveEvent>, String> {
            Ok(self
                .handle
                .queued_events
                .lock()
                .expect("queued events")
                .pop_front())
        }
    }

    fn test_terminal_view() -> (TerminalView, TestLiveHandle) {
        test_terminal_view_with_mode(SessionUiMode::ClassicNcGame)
    }

    fn test_terminal_view_with_mode(session_ui: SessionUiMode) -> (TerminalView, TestLiveHandle) {
        let handle = TestLiveHandle::default();
        let events = EventQueue::default();
        let (terminal_cols, terminal_rows) = match session_ui {
            SessionUiMode::ClassicNcGame => (TERM_COLS, TERM_ROWS),
            SessionUiMode::FullscreenNcDash => (120, 40),
        };
        let mut term = Term::new(
            Config::default(),
            &TermSize::new(terminal_cols as usize, terminal_rows as usize),
            events.clone(),
        );
        term.resize(TermSize::new(terminal_cols as usize, terminal_rows as usize));
        (
            TerminalView {
                session_ui,
                live: Box::new(TestLiveIo {
                    handle: handle.clone(),
                }),
                finalizer: SessionFinalizer::Test,
                term,
                parser: alacritty_terminal::vte::ansi::Processor::new(),
                events,
                title: None,
                selection_drag: false,
                finished: None,
                map_push_monitor: None,
                has_received_output: false,
                created_at: Instant::now() - Duration::from_secs(5),
                last_output_at: None,
                viewport_cols: 120,
                viewport_rows: 40,
                viewport_pixel_width: 1200,
                viewport_pixel_height: 720,
                terminal_cols,
                terminal_rows,
            },
            handle,
        )
    }

    #[test]
    fn live_cursor_is_suppressed_before_first_remote_output() {
        assert!(!should_render_cursor(CursorShape::Block, false));
    }

    #[test]
    fn live_cursor_renders_after_first_remote_output_when_visible() {
        assert!(should_render_cursor(CursorShape::Block, true));
    }

    #[test]
    fn hidden_cursor_stays_hidden_after_remote_output() {
        assert!(!should_render_cursor(CursorShape::Hidden, true));
    }

    #[test]
    fn live_key_forwards_immediately_after_idle() {
        let (mut terminal, handle) = test_terminal_view();

        assert!(terminal.forward_test_input(b"g".to_vec()));
        assert_eq!(handle.sent_inputs(), vec![b"g".to_vec()]);
    }

    #[test]
    fn live_output_triggers_redraw() {
        let (mut terminal, handle) = test_terminal_view();
        handle.send_events([LiveEvent::Output(b"menu".to_vec())]);

        assert!(terminal.tick(&mut Clipboard::new()).expect("tick"));
        assert!(terminal.has_received_output);
    }

    #[test]
    fn selection_drag_does_not_block_keyboard_forwarding() {
        let (mut terminal, handle) = test_terminal_view();
        let start_x =
            (((terminal.viewport_cols - TERM_COLS) / 2) as usize * CELL_WIDTH + (CELL_WIDTH / 2))
                as f64;
        let start_y =
            (((terminal.viewport_rows - TERM_ROWS) / 2) as usize * CELL_HEIGHT
                + (CELL_HEIGHT / 2)) as f64;
        terminal
            .handle_mouse_button(true, PhysicalPosition::new(start_x, start_y))
            .expect("mouse down");
        assert!(terminal.selection_drag_active());

        assert!(terminal.forward_test_input(b"p".to_vec()));
        assert_eq!(handle.sent_inputs(), vec![b"p".to_vec()]);
    }

    #[test]
    fn helper_points_stay_available_for_live_tests() {
        assert!(
            pixel_to_terminal_point(
                PhysicalPosition::new(0.0, 0.0),
                TERM_COLS,
                TERM_ROWS,
                (TERM_COLS as u32) * CELL_WIDTH as u32,
                (TERM_ROWS as u32) * CELL_HEIGHT as u32,
                TERM_COLS,
                TERM_ROWS,
                SessionUiMode::ClassicNcGame,
            )
            .is_some()
        );
        assert_eq!(ModifiersState::empty(), ModifiersState::default());
    }

    #[test]
    fn classic_live_resize_keeps_fixed_pty_size() {
        let (mut terminal, handle) = test_terminal_view_with_mode(SessionUiMode::ClassicNcGame);

        assert!(terminal.resize_viewport(140, 45, 1400, 810));
        assert!(handle.sent_resizes().is_empty());
        assert_eq!(terminal.terminal_cols, TERM_COLS);
        assert_eq!(terminal.terminal_rows, TERM_ROWS);
        assert_eq!(terminal.render_buffer().width(), 140);
        assert_eq!(terminal.render_buffer().height(), 45);
    }

    #[test]
    fn dash_live_resize_forwards_pty_resize() {
        let (mut terminal, handle) = test_terminal_view_with_mode(SessionUiMode::FullscreenNcDash);

        assert!(terminal.resize_viewport(132, 44, 1320, 792));
        assert_eq!(handle.sent_resizes(), vec![(132, 44)]);
        assert_eq!(terminal.terminal_cols, 132);
        assert_eq!(terminal.terminal_rows, 44);
    }

    #[test]
    fn classic_live_buffer_is_centered_in_viewport() {
        let mut inner = PlayfieldBuffer::new(
            2,
            1,
            CellStyle::new(GameColor::White, GameColor::Black, false),
        );
        inner.write_text(0, 0, "OK", CellStyle::new(GameColor::White, GameColor::Black, false));
        let centered = center_terminal_buffer(&inner, 6, 3);
        assert_eq!(centered.width(), 6);
        assert_eq!(centered.height(), 3);
        assert_eq!(centered.plain_line(1), "  OK");
    }
}
