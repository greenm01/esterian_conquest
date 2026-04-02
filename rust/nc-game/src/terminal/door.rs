use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::screen::layout::ScreenGeometry;
use crate::screen::{GameColor, PlayfieldBuffer};
use crate::terminal::cp437;
use crate::terminal::{ColorMode, OutputEncoding, Terminal};
use crate::theme::classic;

use super::stdout::resolve_color;

#[cfg(unix)]
struct RawStdin;

#[cfg(unix)]
impl Read for RawStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::os::fd::AsRawFd;

        unsafe extern "C" {
            fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        }

        let fd = io::stdin().as_raw_fd();
        let ret = unsafe { read(fd, buf.as_mut_ptr(), buf.len()) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}

#[cfg(windows)]
struct RawStdin;

#[cfg(windows)]
impl Read for RawStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut stdin = io::stdin();
        if !stdin.is_terminal() {
            return stdin.read(buf);
        }

        use std::os::windows::io::AsRawHandle;

        unsafe extern "system" {
            fn ReadFile(
                handle: *mut std::ffi::c_void,
                buffer: *mut u8,
                count: u32,
                bytes_read: *mut u32,
                overlapped: *mut std::ffi::c_void,
            ) -> i32;
        }

        let handle = io::stdin().as_raw_handle();
        let mut bytes_read: u32 = 0;
        let len = buf.len().min(u32::MAX as usize) as u32;
        let rc = unsafe {
            ReadFile(
                handle as *mut _,
                buf.as_mut_ptr(),
                len,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };
        if rc == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(bytes_read as usize)
        }
    }
}

const DOOR_ESCAPE_TIMEOUT_MS: i32 = 100;
const DOOR_DOS_PREFIX_TIMEOUT_MS: i32 = 2000;
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
        let trace_dir = std::env::var_os("NC_GAME_DOOR_TRACE_DIR").map(PathBuf::from);
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
        let mut raw = RawStdin;
        self.decoder.next_key(&mut raw, self.trace_dir.as_deref())
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

    bytes.extend_from_slice(&style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    ));
    match playfield.cursor() {
        Some((cursor_col, cursor_row)) => {
            let default_cursor_row = visible_height.saturating_sub(1) as u16;
            let clamped_cursor_row = cursor_row.min(default_cursor_row);
            bytes.extend_from_slice(b"\x1b[?25h");
            bytes.extend_from_slice(cursor_to(clamped_cursor_row, cursor_col).as_bytes());
        }
        None => bytes.extend_from_slice(b"\x1b[?25l"),
    }
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

fn style_sgr(fg: GameColor, bg: GameColor, bold: bool, color_mode: ColorMode) -> Vec<u8> {
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
        _ => 37, // fallback to grey
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
        _ => 40, // fallback to black
    }
}

struct DoorInputDecoder {
    pending: VecDeque<u8>,
    sequence_deadline: Option<Instant>,
    orphaned_escape_suffix_deadline: Option<Instant>,
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
            sequence_deadline: None,
            orphaned_escape_suffix_deadline: None,
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
                    self.sequence_deadline = None;
                    self.orphaned_escape_suffix_deadline = None;
                    return Ok(event);
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut self.pending, consumed);
                    self.sequence_deadline = None;
                    self.orphaned_escape_suffix_deadline = None;
                }
                ParseResult::NeedMore => {
                    if self.pending.is_empty() {
                        self.sequence_deadline = None;
                        self.read_burst(input, trace_dir, None)?;
                        continue;
                    }
                    let timeout_ms = self.remaining_sequence_timeout_ms();
                    if timeout_ms > 0 && self.read_burst(input, trace_dir, Some(timeout_ms))? {
                        continue;
                    }
                    match finalize_pending_after_timeout(&self.pending) {
                        ParseResult::Event(event, consumed) => {
                            self.orphaned_escape_suffix_deadline =
                                orphaned_suffix_deadline(event, &self.pending, consumed);
                            drain_pending(&mut self.pending, consumed);
                            self.sequence_deadline = None;
                            return Ok(event);
                        }
                        ParseResult::Drop(consumed) => {
                            drain_pending(&mut self.pending, consumed);
                            self.sequence_deadline = None;
                            self.orphaned_escape_suffix_deadline = None;
                        }
                        ParseResult::NeedMore => {}
                    }
                }
            }
        }
    }

    fn read_burst(
        &mut self,
        input: &mut impl Read,
        trace_dir: Option<&Path>,
        timeout_ms: Option<i32>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut burst = read_burst(input, trace_dir, timeout_ms)?;
        drop_orphaned_escape_suffix(
            &mut burst,
            self.orphaned_escape_suffix_deadline
                .filter(|deadline| *deadline > Instant::now()),
        );
        if burst.is_empty() {
            self.orphaned_escape_suffix_deadline = None;
            return Ok(false);
        }
        self.orphaned_escape_suffix_deadline = None;
        self.pending.extend(burst);
        Ok(true)
    }

    fn remaining_sequence_timeout_ms(&mut self) -> i32 {
        let timeout = match self.pending.front().copied() {
            Some(0x00 | 0xe0) => DOOR_DOS_PREFIX_TIMEOUT_MS,
            _ => DOOR_ESCAPE_TIMEOUT_MS,
        };
        let deadline = self
            .sequence_deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_millis(timeout as u64));
        let remaining = deadline.saturating_duration_since(Instant::now());
        let ms = remaining.as_millis().min(i32::MAX as u128) as i32;
        if ms == 0 && !remaining.is_zero() {
            1
        } else {
            ms
        }
    }
}

fn try_decode_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(&first) = pending.front() else {
        return ParseResult::NeedMore;
    };

    match first {
        0x03 => ParseResult::Event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), 1),
        0x04 => ParseResult::Event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), 1),
        0x05 => ParseResult::Event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL), 1),
        0x15 => ParseResult::Event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), 1),
        0x18 => ParseResult::Event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL), 1),
        b'\r' | b'\n' => ParseResult::Event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1),
        b'\t' => ParseResult::Event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), 1),
        0x08 | 0x7f => ParseResult::Event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), 1),
        0x20..=0x7e => ParseResult::Event(
            KeyEvent::new(KeyCode::Char(first as char), KeyModifiers::NONE),
            1,
        ),
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
        _ => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn try_decode_csi_complete(pending: &VecDeque<u8>) -> ParseResult {
    let bytes = pending.iter().copied().collect::<Vec<_>>();
    for (idx, byte) in bytes.iter().copied().enumerate().skip(2) {
        if let Some(event) = map_csi_event(byte, &bytes[2..=idx]) {
            return ParseResult::Event(event, idx + 1);
        }
        if is_csi_final(byte) {
            return ParseResult::Drop(idx + 1);
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
        b'G' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'I' => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        b'K' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'M' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'O' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        b'P' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'Q' => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        b'S' => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
        _ => None,
    }
}

fn map_csi_event(byte: u8, sequence: &[u8]) -> Option<KeyEvent> {
    match byte {
        b'A' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'B' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'C' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'D' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'F' | b'K' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        b'U' => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        b'V' => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        b'~' => match sequence {
            b"1~" | b"7~" => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
            b"4~" | b"8~" => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
            b"3~" => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
            b"5~" => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            b"6~" => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            _ => None,
        },
        _ => None,
    }
}

fn is_csi_final(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

fn orphaned_suffix_deadline(
    event: KeyEvent,
    pending: &VecDeque<u8>,
    consumed: usize,
) -> Option<Instant> {
    let is_bare_escape = event.code == KeyCode::Esc
        && matches!(pending.front().copied(), Some(0x1b))
        && consumed == 1
        && pending.len() == 1;
    is_bare_escape.then(|| Instant::now() + Duration::from_millis(DOOR_ESCAPE_TIMEOUT_MS as u64))
}

fn drop_orphaned_escape_suffix(burst: &mut Vec<u8>, orphan_deadline: Option<Instant>) {
    if orphan_deadline.is_none() || burst.is_empty() {
        return;
    }
    let consumed = match burst[0] {
        b'[' => orphan_csi_suffix_len(&burst[1..]).map(|len| len + 1),
        b'O' => orphan_ss3_suffix_len(&burst[1..]).map(|len| len + 1),
        _ => None,
    };
    if let Some(consumed) = consumed {
        burst.drain(..consumed.min(burst.len()));
    }
}

fn orphan_csi_suffix_len(bytes: &[u8]) -> Option<usize> {
    for (idx, byte) in bytes.iter().copied().enumerate() {
        if is_csi_final(byte) {
            return Some(idx + 1);
        }
    }
    (!bytes.is_empty()).then_some(bytes.len())
}

fn orphan_ss3_suffix_len(bytes: &[u8]) -> Option<usize> {
    match bytes.first().copied() {
        Some(b'A' | b'B' | b'C' | b'D' | b'H' | b'F') => Some(1),
        Some(_) => Some(1),
        None => None,
    }
}

fn read_burst(
    input: &mut impl Read,
    trace_dir: Option<&Path>,
    timeout_ms: Option<i32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(timeout_ms) = timeout_ms {
        if !stdin_ready(timeout_ms)? {
            return Ok(Vec::new());
        }
    }

    let mut burst = vec![read_one(input)?];
    while burst.len() < MAX_ESCAPE_SEQUENCE_BYTES && stdin_ready(0)? {
        burst.push(read_one(input)?);
    }
    if let Some(trace_dir) = trace_dir {
        append_input_trace(trace_dir, &burst)?;
    }
    Ok(burst)
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

#[cfg(windows)]
fn stdin_ready(timeout_ms: i32) -> Result<bool, Box<dyn std::error::Error>> {
    use std::os::windows::io::AsRawHandle;

    unsafe extern "system" {
        fn PeekNamedPipe(
            handle: *mut std::ffi::c_void,
            buffer: *mut u8,
            buffer_size: u32,
            bytes_read: *mut u32,
            total_bytes_available: *mut u32,
            bytes_left_this_message: *mut u32,
        ) -> i32;
        fn WaitForSingleObject(handle: *mut std::ffi::c_void, millis: u32) -> u32;
    }

    const WAIT_OBJECT_0: u32 = 0;

    let stdin = io::stdin();
    let handle = stdin.as_raw_handle();
    if !stdin.is_terminal() {
        let deadline = if timeout_ms < 0 {
            None
        } else {
            Some(Instant::now() + Duration::from_millis(timeout_ms as u64))
        };
        loop {
            let mut total_bytes_available = 0u32;
            let rc = unsafe {
                PeekNamedPipe(
                    handle as *mut _,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut total_bytes_available,
                    std::ptr::null_mut(),
                )
            };
            if rc != 0 {
                return Ok(total_bytes_available > 0);
            }

            let err = io::Error::last_os_error();
            if timeout_ms == 0 {
                return Ok(false);
            }
            if deadline.is_some_and(|value| Instant::now() >= value) {
                return Ok(false);
            }
            if err.kind() == io::ErrorKind::BrokenPipe {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    let millis = if timeout_ms < 0 {
        0xFFFFFFFF
    } else {
        timeout_ms as u32
    };
    let rc = unsafe { WaitForSingleObject(handle as *mut _, millis) };
    Ok(rc == WAIT_OBJECT_0)
}

#[cfg(not(any(unix, windows)))]
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
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let ts = trace_clock_start().elapsed().as_millis();
    write!(log, "{ts}")?;
    for byte in bytes {
        write!(log, " {:02x}", byte)?;
    }
    writeln!(log)?;
    Ok(())
}

fn trace_clock_start() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
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

#[doc(hidden)]
pub fn decode_timed_input_stream_for_test(
    chunks: &[(u64, &[u8])],
) -> Result<Vec<KeyEvent>, Box<dyn std::error::Error>> {
    let mut pending = VecDeque::new();
    let mut events = Vec::new();
    let mut deadline_ms = None;
    let mut orphan_deadline_ms = None;

    for &(chunk_time_ms, bytes) in chunks {
        if let Some(deadline) = deadline_ms {
            if chunk_time_ms > deadline {
                finalize_timed_pending(
                    &mut pending,
                    &mut events,
                    &mut orphan_deadline_ms,
                    chunk_time_ms,
                );
                deadline_ms = None;
            }
        }

        let mut burst = bytes.to_vec();
        if orphan_deadline_ms.is_some_and(|deadline| chunk_time_ms <= deadline) {
            drop_orphaned_escape_suffix(
                &mut burst,
                Some(Instant::now() + Duration::from_millis(DOOR_ESCAPE_TIMEOUT_MS as u64)),
            );
        }
        orphan_deadline_ms = None;
        pending.extend(burst.iter().copied());
        loop {
            match try_decode_complete(&pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut pending, consumed);
                    events.push(event);
                    deadline_ms = None;
                    orphan_deadline_ms = None;
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut pending, consumed);
                    deadline_ms = None;
                    orphan_deadline_ms = None;
                }
                ParseResult::NeedMore => {
                    if pending.is_empty() {
                        deadline_ms = None;
                    } else if deadline_ms.is_none() {
                        deadline_ms = Some(chunk_time_ms + DOOR_ESCAPE_TIMEOUT_MS as u64);
                    }
                    break;
                }
            }
        }
    }

    if deadline_ms.is_some() {
        finalize_timed_pending(&mut pending, &mut events, &mut orphan_deadline_ms, u64::MAX);
    }
    while !pending.is_empty() {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, consumed) => {
                drain_pending(&mut pending, consumed);
                events.push(event);
            }
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
            }
            ParseResult::NeedMore => {
                finalize_timed_pending(
                    &mut pending,
                    &mut events,
                    &mut orphan_deadline_ms,
                    u64::MAX,
                );
            }
        }
    }
    Ok(events)
}

fn finalize_timed_pending(
    pending: &mut VecDeque<u8>,
    events: &mut Vec<KeyEvent>,
    orphan_deadline_ms: &mut Option<u64>,
    now_ms: u64,
) {
    match finalize_pending_after_timeout(pending) {
        ParseResult::Event(event, consumed) => {
            if event.code == KeyCode::Esc
                && matches!(pending.front().copied(), Some(0x1b))
                && consumed == 1
                && pending.len() == 1
            {
                *orphan_deadline_ms = Some(now_ms.saturating_add(DOOR_ESCAPE_TIMEOUT_MS as u64));
            } else {
                *orphan_deadline_ms = None;
            }
            drain_pending(pending, consumed);
            events.push(event);
        }
        ParseResult::Drop(consumed) => {
            drain_pending(pending, consumed);
            *orphan_deadline_ms = None;
        }
        ParseResult::NeedMore => {}
    }
}
