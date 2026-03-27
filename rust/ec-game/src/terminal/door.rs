use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::screen::layout::ScreenGeometry;
use crate::screen::{GameColor, PlayfieldBuffer};
use crate::terminal::cp437;
use crate::terminal::{ColorMode, OutputEncoding, Terminal};
use crate::theme::classic;

use super::stdout::resolve_color;

const DOOR_ESCAPE_TIMEOUT_MS: i32 = 1500;
const MAX_ESCAPE_SEQUENCE_BYTES: usize = 16;

pub struct DoorTerminal {
    encoding: OutputEncoding,
    color_mode: ColorMode,
    geometry: ScreenGeometry,
    trace_dir: Option<PathBuf>,
    frame_seq: u64,
    decoder: DoorInputDecoder,
}

impl DoorTerminal {
    pub fn with_encoding_and_color_mode(
        encoding: OutputEncoding,
        color_mode: ColorMode,
        geometry: ScreenGeometry,
    ) -> Self {
        let trace_dir = std::env::var_os("EC_GAME_DOOR_TRACE_DIR").map(PathBuf::from);
        if let Some(path) = trace_dir.as_deref() {
            let _ = std::fs::create_dir_all(path);
        }
        Self {
            encoding,
            color_mode,
            geometry,
            trace_dir,
            frame_seq: 0,
            decoder: DoorInputDecoder::new(),
        }
    }
}

impl Terminal for DoorTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let frame =
            serialize_playfield_frame(playfield, self.geometry, self.encoding, self.color_mode);
        if let Some(trace_dir) = self.trace_dir.as_deref() {
            trace_output_frame(trace_dir, self.frame_seq, &frame)?;
            self.frame_seq = self.frame_seq.saturating_add(1);
        }
        stdout.write_all(&frame)?;
        stdout.flush()?;
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let mut lock = stdin.lock();
        self.decoder.next_key(&mut lock, self.trace_dir.as_deref())
    }

    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[0m\x1b[?25h\x1b[2J\x1b[H")?;
        stdout.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            stdout.write_all(b"\r\n")?;
        }
        stdout.flush()?;
        Ok(())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&style_sgr(
            classic::terminal_foreground(),
            classic::app_background(),
            false,
            self.color_mode,
        ));
        bytes.extend_from_slice(b"\x1b[?25h\x1b[2J\x1b[H");
        stdout.write_all(&bytes)?;
        stdout.flush()?;
        Ok(())
    }
}

pub fn serialize_playfield_frame(
    playfield: &PlayfieldBuffer,
    geometry: ScreenGeometry,
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Vec<u8> {
    let color_mode = match color_mode {
        ColorMode::Ansi16 => ColorMode::Ansi16,
        ColorMode::Color256 | ColorMode::TrueColor => ColorMode::Ansi16,
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    ));
    bytes.extend_from_slice(b"\x1b[?25h\x1b[2J\x1b[H");

    let visible_height = playfield.height().min(geometry.height());
    for row_idx in 0..visible_height {
        let row = playfield.row(row_idx);
        let Some(last_visible_idx) = row.iter().rposition(|cell| cell.ch != ' ') else {
            continue;
        };

        bytes.extend_from_slice(cursor_to(row_idx as u16, 0).as_bytes());
        let mut current_style = None;
        let mut run = String::new();
        for cell in &row[..=last_visible_idx] {
            if current_style != Some(cell.style) {
                if !run.is_empty() {
                    push_text(&mut bytes, &run, encoding);
                    run.clear();
                }
                bytes.extend_from_slice(&style_sgr(
                    cell.style.fg,
                    cell.style.bg,
                    cell.style.bold,
                    color_mode,
                ));
                current_style = Some(cell.style);
            }
            run.push(cell.ch);
        }
        if !run.is_empty() {
            push_text(&mut bytes, &run, encoding);
        }
    }

    let default_cursor_row = visible_height.saturating_sub(1) as u16;
    let (cursor_col, cursor_row) = playfield.cursor().unwrap_or((0, default_cursor_row));
    let clamped_cursor_row = cursor_row.min(default_cursor_row);
    bytes.extend_from_slice(&style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    ));
    bytes.extend_from_slice(cursor_to(clamped_cursor_row, cursor_col).as_bytes());
    bytes
}

fn push_text(bytes: &mut Vec<u8>, text: &str, encoding: OutputEncoding) {
    match encoding {
        OutputEncoding::Utf8 => bytes.extend_from_slice(text.as_bytes()),
        OutputEncoding::Cp437 => bytes.extend_from_slice(&cp437::str_to_cp437(text)),
    }
}

fn cursor_to(row_zero_based: u16, col_zero_based: u16) -> String {
    format!("\x1b[{};{}H", row_zero_based + 1, col_zero_based + 1)
}

fn style_sgr(
    fg: GameColor,
    bg: GameColor,
    bold: bool,
    color_mode: ColorMode,
) -> Vec<u8> {
    let fg_code = ansi_fg_code(resolve_color(fg, color_mode));
    let bg_code = ansi_bg_code(resolve_color(bg, color_mode));
    let mut seq = format!("\x1b[0;{fg_code};{bg_code}");
    if bold {
        seq.push_str(";1");
    }
    seq.push('m');
    seq.into_bytes()
}

fn ansi_fg_code(color: crossterm::style::Color) -> u8 {
    match color {
        crossterm::style::Color::Black => 30,
        crossterm::style::Color::DarkRed => 31,
        crossterm::style::Color::DarkGreen => 32,
        crossterm::style::Color::DarkYellow => 33,
        crossterm::style::Color::DarkBlue => 34,
        crossterm::style::Color::DarkMagenta => 35,
        crossterm::style::Color::DarkCyan => 36,
        crossterm::style::Color::Grey => 37,
        crossterm::style::Color::DarkGrey => 90,
        crossterm::style::Color::Red => 91,
        crossterm::style::Color::Green => 92,
        crossterm::style::Color::Yellow => 93,
        crossterm::style::Color::Blue => 94,
        crossterm::style::Color::Magenta => 95,
        crossterm::style::Color::Cyan => 96,
        crossterm::style::Color::White => 97,
        other => panic!("door terminal only supports ANSI16 colors, got {other:?}"),
    }
}

fn ansi_bg_code(color: crossterm::style::Color) -> u8 {
    match color {
        crossterm::style::Color::Black => 40,
        crossterm::style::Color::DarkRed | crossterm::style::Color::Red => 41,
        crossterm::style::Color::DarkGreen | crossterm::style::Color::Green => 42,
        crossterm::style::Color::DarkYellow | crossterm::style::Color::Yellow => 43,
        crossterm::style::Color::DarkBlue | crossterm::style::Color::Blue => 44,
        crossterm::style::Color::DarkMagenta | crossterm::style::Color::Magenta => 45,
        crossterm::style::Color::DarkCyan | crossterm::style::Color::Cyan => 46,
        crossterm::style::Color::Grey
        | crossterm::style::Color::DarkGrey
        | crossterm::style::Color::White => 47,
        other => panic!("door terminal only supports ANSI16 colors, got {other:?}"),
    }
}

struct DoorInputDecoder {
    pending: VecDeque<u8>,
}

enum ParseResult {
    Event(KeyEvent, usize),
    NeedMore,
    Drop(usize),
}

impl DoorInputDecoder {
    fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    fn next_key(
        &mut self,
        input: &mut impl Read,
        trace_dir: Option<&Path>,
    ) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        loop {
            match try_decode_complete(&self.pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut self.pending, consumed);
                    return Ok(event);
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut self.pending, consumed);
                }
                ParseResult::NeedMore => {
                    if self.pending.is_empty() {
                        read_burst(input, &mut self.pending, trace_dir, None)?;
                        continue;
                    }
                    if read_burst(
                        input,
                        &mut self.pending,
                        trace_dir,
                        Some(DOOR_ESCAPE_TIMEOUT_MS),
                    )? {
                        continue;
                    }
                    match finalize_pending_after_timeout(&self.pending) {
                        ParseResult::Event(event, consumed) => {
                            drain_pending(&mut self.pending, consumed);
                            return Ok(event);
                        }
                        ParseResult::Drop(consumed) => {
                            drain_pending(&mut self.pending, consumed);
                        }
                        ParseResult::NeedMore => {}
                    }
                }
            }
        }
    }
}

fn try_decode_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(&first) = pending.front() else {
        return ParseResult::NeedMore;
    };

    match first {
        0x03 => ParseResult::Event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), 1),
        0x05 => ParseResult::Event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL), 1),
        0x18 => ParseResult::Event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL), 1),
        b'\r' | b'\n' => ParseResult::Event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1),
        b'\t' => ParseResult::Event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), 1),
        0x08 | 0x7f => {
            ParseResult::Event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), 1)
        }
        0x20..=0x7e => {
            ParseResult::Event(KeyEvent::new(KeyCode::Char(first as char), KeyModifiers::NONE), 1)
        }
        0x00 | 0xe0 => try_decode_dos_complete(pending),
        0x1b => try_decode_escape_complete(pending),
        _ => ParseResult::Drop(1),
    }
}

fn try_decode_dos_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(byte) = pending.get(1).copied() else {
        return ParseResult::NeedMore;
    };
    match map_dos_extended(byte) {
        Some(event) => ParseResult::Event(event, 2),
        None => ParseResult::Drop(2),
    }
}

fn try_decode_escape_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(second) = pending.get(1).copied() else {
        return ParseResult::NeedMore;
    };
    match second {
        b'[' => try_decode_csi_complete(pending),
        b'O' => try_decode_ss3_complete(pending),
        b'A' => ParseResult::Event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 2),
        b'B' => ParseResult::Event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 2),
        b'C' => ParseResult::Event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 2),
        b'D' => ParseResult::Event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), 2),
        b'H' => ParseResult::Event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), 2),
        b'F' => ParseResult::Event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), 2),
        _ => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn try_decode_csi_complete(pending: &VecDeque<u8>) -> ParseResult {
    let bytes = pending.iter().copied().collect::<Vec<_>>();
    for (idx, byte) in bytes.iter().copied().enumerate().skip(2) {
        match byte {
            b'A' => {
                return ParseResult::Event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), idx + 1);
            }
            b'B' => {
                return ParseResult::Event(
                    KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
                    idx + 1,
                );
            }
            b'C' => {
                return ParseResult::Event(
                    KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                    idx + 1,
                );
            }
            b'D' => {
                return ParseResult::Event(
                    KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
                    idx + 1,
                );
            }
            b'H' => {
                return ParseResult::Event(
                    KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
                    idx + 1,
                );
            }
            b'F' => {
                return ParseResult::Event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), idx + 1);
            }
            b'~' => {
                let event = match &bytes[2..=idx] {
                    b"1~" | b"7~" => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
                    b"4~" | b"8~" => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
                    b"3~" => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
                    b"5~" => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
                    b"6~" => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
                    _ => None,
                };
                return match event {
                    Some(event) => ParseResult::Event(event, idx + 1),
                    None => ParseResult::Drop(idx + 1),
                };
            }
            0x40..=0x7e => return ParseResult::Drop(idx + 1),
            _ => {}
        }
    }
    if pending.len() >= MAX_ESCAPE_SEQUENCE_BYTES {
        ParseResult::Drop(pending.len())
    } else {
        ParseResult::NeedMore
    }
}

fn try_decode_ss3_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(byte) = pending.get(2).copied() else {
        return ParseResult::NeedMore;
    };
    let event = match byte {
        b'A' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'B' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'C' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'D' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'F' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        _ => None,
    };
    match event {
        Some(event) => ParseResult::Event(event, 3),
        None => ParseResult::Drop(3),
    }
}

fn finalize_pending_after_timeout(pending: &VecDeque<u8>) -> ParseResult {
    match pending.front().copied() {
        Some(0x1b) => finalize_escape_after_timeout(pending),
        Some(0x00 | 0xe0) => ParseResult::Drop(pending.len().min(2)),
        Some(_) => ParseResult::Drop(1),
        None => ParseResult::NeedMore,
    }
}

fn finalize_escape_after_timeout(pending: &VecDeque<u8>) -> ParseResult {
    match pending.get(1).copied() {
        None => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
        Some(b'[' | b'O') => ParseResult::Drop(pending.len()),
        Some(_) => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn map_dos_extended(byte: u8) -> Option<KeyEvent> {
    match byte {
        b'H' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'P' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'M' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'K' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'G' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'O' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        b'S' => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
        b'I' => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        b'Q' => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        _ => None,
    }
}

fn read_burst(
    input: &mut impl Read,
    pending: &mut VecDeque<u8>,
    trace_dir: Option<&Path>,
    timeout_ms: Option<i32>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(timeout_ms) = timeout_ms {
        if !stdin_ready(timeout_ms)? {
            return Ok(false);
        }
    }

    let mut burst = vec![read_one(input)?];
    while burst.len() < MAX_ESCAPE_SEQUENCE_BYTES && stdin_ready(0)? {
        burst.push(read_one(input)?);
    }
    pending.extend(burst.iter().copied());
    if let Some(trace_dir) = trace_dir {
        append_input_trace(trace_dir, &burst)?;
    }
    Ok(true)
}

fn drain_pending(pending: &mut VecDeque<u8>, consumed: usize) {
    for _ in 0..consumed.min(pending.len()) {
        pending.pop_front();
    }
}

fn read_one(input: &mut impl Read) -> Result<u8, Box<dyn std::error::Error>> {
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte)?;
    Ok(byte[0])
}

#[cfg(unix)]
fn stdin_ready(timeout_ms: i32) -> Result<bool, Box<dyn std::error::Error>> {
    use std::os::fd::AsRawFd;

    const POLLIN: i16 = 0x0001;

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    unsafe extern "C" {
        fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    }

    let stdin = io::stdin();
    let mut fds = [PollFd {
        fd: stdin.as_raw_fd(),
        events: POLLIN,
        revents: 0,
    }];

    loop {
        let rc = unsafe { poll(fds.as_mut_ptr(), fds.len(), timeout_ms) };
        if rc >= 0 {
            return Ok(rc > 0 && (fds[0].revents & POLLIN) != 0);
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err.into());
    }
}

#[cfg(not(unix))]
fn stdin_ready(_timeout_ms: i32) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(false)
}

fn trace_output_frame(
    trace_dir: &Path,
    frame_seq: u64,
    frame: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(trace_dir)?;
    let path = trace_dir.join(format!("frame-{frame_seq:06}.bin"));
    std::fs::write(path, frame)?;
    Ok(())
}

fn append_input_trace(trace_dir: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(trace_dir)?;
    let raw_path = trace_dir.join("input.bin");
    let mut raw = OpenOptions::new()
        .create(true)
        .append(true)
        .open(raw_path)?;
    raw.write_all(bytes)?;

    let log_path = trace_dir.join("input.log");
    let mut log = OpenOptions::new().create(true).append(true).open(log_path)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    write!(log, "{ts}")?;
    for byte in bytes {
        write!(log, " {:02x}", byte)?;
    }
    writeln!(log)?;
    Ok(())
}

#[doc(hidden)]
pub fn decode_input_bytes_for_test(bytes: &[u8]) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let mut pending = bytes.iter().copied().collect::<VecDeque<_>>();
    if pending.is_empty() {
        return Ok(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
    }
    loop {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, _) => return Ok(event),
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
                if pending.is_empty() {
                    return Ok(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
                }
            }
            ParseResult::NeedMore => {
                return Ok(match finalize_pending_after_timeout(&pending) {
                    ParseResult::Event(event, _) => event,
                    ParseResult::Drop(_) | ParseResult::NeedMore => {
                        KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)
                    }
                });
            }
        }
    }
}

#[doc(hidden)]
pub fn decode_fragmented_input_for_test(
    initial: &[u8],
    continuation: &[u8],
) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let mut pending = initial.iter().copied().collect::<VecDeque<_>>();
    if matches!(try_decode_complete(&pending), ParseResult::NeedMore) {
        pending.extend(continuation.iter().copied());
    }
    decode_input_bytes_for_test(&pending.iter().copied().collect::<Vec<_>>())
}

#[doc(hidden)]
pub fn decode_input_stream_for_test(
    bytes: &[u8],
) -> Result<Vec<KeyEvent>, Box<dyn std::error::Error>> {
    let mut pending = bytes.iter().copied().collect::<VecDeque<_>>();
    let mut events = Vec::new();
    while !pending.is_empty() {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, consumed) => {
                drain_pending(&mut pending, consumed);
                events.push(event);
            }
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
            }
            ParseResult::NeedMore => match finalize_pending_after_timeout(&pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut pending, consumed);
                    events.push(event);
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut pending, consumed);
                }
                ParseResult::NeedMore => break,
            },
        }
    }
    Ok(events)
}
