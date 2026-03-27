use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::screen::layout::ScreenGeometry;
use crate::screen::{GameColor, PlayfieldBuffer};
use crate::terminal::cp437;
use crate::terminal::{ColorMode, OutputEncoding, Terminal};
use crate::theme::classic;

use super::stdout::resolve_color;

const ESC_INITIAL_TIMEOUT_MS: i32 = 500;
const ESC_SEQUENCE_TIMEOUT_MS: i32 = 1200;
const MAX_ESCAPE_SEQUENCE_BYTES: usize = 16;

pub struct DoorTerminal {
    encoding: OutputEncoding,
    color_mode: ColorMode,
    geometry: ScreenGeometry,
    trace_dir: Option<PathBuf>,
    frame_seq: u64,
    pending_input: VecDeque<u8>,
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
            pending_input: VecDeque::new(),
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
        read_key_from_stdin(&mut self.pending_input, self.trace_dir.as_deref())
    }

    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[0m\x1b[2J\x1b[H")?;
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
        bytes.extend_from_slice(b"\x1b[2J\x1b[H");
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
    bytes.extend_from_slice(b"\x1b[2J\x1b[H");

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

fn read_key_from_stdin(
    pending: &mut VecDeque<u8>,
    trace_dir: Option<&Path>,
) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    loop {
        if pending.is_empty() {
            read_and_enqueue(&mut lock, pending, trace_dir)?;
        }
        match decode_pending_input(pending) {
            PendingDecode::Key { event, consumed } => {
                drain_pending(pending, consumed);
                return Ok(event);
            }
            PendingDecode::NeedMore(timeout_ms) => {
                if stdin_ready(timeout_ms)? {
                    read_and_enqueue(&mut lock, pending, trace_dir)?;
                    continue;
                }
                let (event, consumed) = finalize_pending_after_timeout(pending);
                drain_pending(pending, consumed);
                return Ok(event);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingDecode {
    Key { event: KeyEvent, consumed: usize },
    NeedMore(i32),
}

fn decode_pending_input(pending: &VecDeque<u8>) -> PendingDecode {
    let Some(&first) = pending.front() else {
        return PendingDecode::Key {
            event: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            consumed: 0,
        };
    };

    match first {
        0x03 => key_decode(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), 1),
        0x05 => key_decode(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL), 1),
        0x18 => key_decode(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL), 1),
        b'\r' | b'\n' => key_decode(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1),
        b'\t' => key_decode(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), 1),
        0x08 | 0x7f => key_decode(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), 1),
        0x20..=0x7e => key_decode(KeyEvent::new(KeyCode::Char(first as char), KeyModifiers::NONE), 1),
        0x00 | 0xe0 => decode_dos_pending(pending),
        0x1b => decode_escape_pending(pending),
        _ => key_decode(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE), 1),
    }
}

fn key_decode(event: KeyEvent, consumed: usize) -> PendingDecode {
    PendingDecode::Key { event, consumed }
}

fn decode_dos_pending(pending: &VecDeque<u8>) -> PendingDecode {
    let Some(byte) = pending.get(1).copied() else {
        return PendingDecode::NeedMore(ESC_SEQUENCE_TIMEOUT_MS);
    };
    key_decode(map_dos_extended(byte), 2)
}

fn decode_escape_pending(pending: &VecDeque<u8>) -> PendingDecode {
    let Some(second) = pending.get(1).copied() else {
        return PendingDecode::NeedMore(ESC_INITIAL_TIMEOUT_MS);
    };
    match second {
        b'[' => decode_csi_pending(pending),
        b'O' => decode_ss3_pending(pending),
        b'A' => key_decode(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 2),
        b'B' => key_decode(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 2),
        b'C' => key_decode(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), 2),
        b'D' => key_decode(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), 2),
        b'H' => key_decode(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), 2),
        b'F' => key_decode(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), 2),
        _ => key_decode(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn decode_csi_pending(pending: &VecDeque<u8>) -> PendingDecode {
    let bytes = pending.iter().copied().collect::<Vec<_>>();
    for (idx, byte) in bytes.iter().copied().enumerate().skip(2) {
        match byte {
            b'A' => return key_decode(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), idx + 1),
            b'B' => return key_decode(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), idx + 1),
            b'C' => return key_decode(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), idx + 1),
            b'D' => return key_decode(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), idx + 1),
            b'H' => return key_decode(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), idx + 1),
            b'F' => return key_decode(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), idx + 1),
            b'~' => {
                let event = match &bytes[2..=idx] {
                    b"1~" | b"7~" => KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
                    b"4~" | b"8~" => KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
                    b"3~" => KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
                    b"5~" => KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
                    b"6~" => KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
                    _ => KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
                };
                return key_decode(event, idx + 1);
            }
            _ => {}
        }
    }
    if pending.len() >= MAX_ESCAPE_SEQUENCE_BYTES {
        key_decode(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE), pending.len())
    } else {
        PendingDecode::NeedMore(ESC_SEQUENCE_TIMEOUT_MS)
    }
}

fn decode_ss3_pending(pending: &VecDeque<u8>) -> PendingDecode {
    let Some(byte) = pending.get(2).copied() else {
        return PendingDecode::NeedMore(ESC_SEQUENCE_TIMEOUT_MS);
    };
    let event = match byte {
        b'A' => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        b'B' => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        b'C' => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        b'D' => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        b'H' => KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
        b'F' => KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
        _ => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    };
    key_decode(event, 3)
}

fn finalize_pending_after_timeout(pending: &VecDeque<u8>) -> (KeyEvent, usize) {
    match pending.front().copied() {
        Some(0x1b) => finalize_escape_after_timeout(pending),
        Some(0x00 | 0xe0) => (
            KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            pending.len().min(2),
        ),
        Some(_) => (KeyEvent::new(KeyCode::Null, KeyModifiers::NONE), 1),
        None => (KeyEvent::new(KeyCode::Null, KeyModifiers::NONE), 0),
    }
}

fn finalize_escape_after_timeout(pending: &VecDeque<u8>) -> (KeyEvent, usize) {
    match pending.get(1).copied() {
        None => (KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
        Some(b'[' | b'O') => (
            KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            pending.len(),
        ),
        Some(_) => (KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn map_dos_extended(byte: u8) -> KeyEvent {
    match byte {
        b'H' => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        b'P' => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        b'M' => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        b'K' => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        b'G' => KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
        b'O' => KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
        b'S' => KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
        b'I' => KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
        b'Q' => KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
        _ => KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
    }
}

fn read_and_enqueue(
    input: &mut impl Read,
    pending: &mut VecDeque<u8>,
    trace_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let byte = read_one(input)?;
    pending.push_back(byte);
    if let Some(trace_dir) = trace_dir {
        append_input_trace(trace_dir, &[byte])?;
    }
    Ok(())
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
    let path = trace_dir.join("input.bin");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

#[doc(hidden)]
pub fn decode_input_bytes_for_test(bytes: &[u8]) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let pending = bytes.iter().copied().collect::<VecDeque<_>>();
    if pending.is_empty() {
        return Ok(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
    }
    Ok(match decode_pending_input(&pending) {
        PendingDecode::Key { event, .. } => event,
        PendingDecode::NeedMore(_) => finalize_pending_after_timeout(&pending).0,
    })
}

#[doc(hidden)]
pub fn decode_fragmented_input_for_test(
    initial: &[u8],
    continuation: &[u8],
) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let mut pending = initial.iter().copied().collect::<VecDeque<_>>();
    if matches!(decode_pending_input(&pending), PendingDecode::NeedMore(_)) {
        pending.extend(continuation.iter().copied());
    }
    Ok(match decode_pending_input(&pending) {
        PendingDecode::Key { event, .. } => event,
        PendingDecode::NeedMore(_) => finalize_pending_after_timeout(&pending).0,
    })
}
