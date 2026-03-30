use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use alacritty_terminal::term::TermMode;

pub fn is_key_press(event: &WinitKeyEvent) -> bool {
    event.state == ElementState::Pressed
}

pub fn is_copy_shortcut(event: &WinitKeyEvent, modifiers: ModifiersState) -> bool {
    is_key_press(event)
        && modifiers.control_key()
        && modifiers.shift_key()
        && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("c"))
}

pub fn is_paste_shortcut(event: &WinitKeyEvent, modifiers: ModifiersState) -> bool {
    if !is_key_press(event) {
        return false;
    }
    if modifiers.shift_key() && matches!(&event.logical_key, Key::Named(NamedKey::Insert)) {
        return true;
    }
    modifiers.control_key()
        && modifiers.shift_key()
        && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("v"))
}

pub fn picker_key(event: &WinitKeyEvent, modifiers: ModifiersState) -> Option<KeyEvent> {
    if !is_key_press(event) {
        return None;
    }
    let key_modifiers = modifiers_to_crossterm(modifiers);
    let code = match &event.logical_key {
        Key::Named(NamedKey::ArrowUp) => KeyCode::Up,
        Key::Named(NamedKey::ArrowDown) => KeyCode::Down,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::Left,
        Key::Named(NamedKey::ArrowRight) => KeyCode::Right,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Escape) => KeyCode::Esc,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Named(NamedKey::Insert) => KeyCode::Insert,
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => KeyCode::BackTab,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::F1) => KeyCode::F(1),
        Key::Named(NamedKey::F2) => KeyCode::F(2),
        Key::Named(NamedKey::F3) => KeyCode::F(3),
        Key::Named(NamedKey::F4) => KeyCode::F(4),
        Key::Named(NamedKey::F5) => KeyCode::F(5),
        Key::Named(NamedKey::F6) => KeyCode::F(6),
        Key::Named(NamedKey::F7) => KeyCode::F(7),
        Key::Named(NamedKey::F8) => KeyCode::F(8),
        Key::Named(NamedKey::F9) => KeyCode::F(9),
        Key::Named(NamedKey::F10) => KeyCode::F(10),
        Key::Named(NamedKey::F11) => KeyCode::F(11),
        Key::Named(NamedKey::F12) => KeyCode::F(12),
        _ => {
            let ch = event
                .text
                .as_ref()
                .and_then(|text| text.chars().next())
                .filter(|ch| !ch.is_control())
                .or_else(|| match &event.logical_key {
                    Key::Character(text) => text.chars().next(),
                    _ => None,
                })?;
            let ch = if key_modifiers.contains(KeyModifiers::CONTROL) {
                ch.to_ascii_lowercase()
            } else {
                ch
            };
            KeyCode::Char(ch)
        }
    };
    Some(KeyEvent::new(code, key_modifiers))
}

pub fn terminal_key_bytes(
    event: &WinitKeyEvent,
    modifiers: ModifiersState,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if !is_key_press(event) {
        return None;
    }
    let mut bytes = match &event.logical_key {
        Key::Named(NamedKey::Enter) => b"\r".to_vec(),
        Key::Named(NamedKey::Escape) => vec![0x1b],
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => b"\x1b[Z".to_vec(),
        Key::Named(NamedKey::Tab) => b"\t".to_vec(),
        Key::Named(NamedKey::Backspace) => vec![0x7f],
        Key::Named(NamedKey::ArrowUp) => cursor_sequence(mode, b'A'),
        Key::Named(NamedKey::ArrowDown) => cursor_sequence(mode, b'B'),
        Key::Named(NamedKey::ArrowRight) => cursor_sequence(mode, b'C'),
        Key::Named(NamedKey::ArrowLeft) => cursor_sequence(mode, b'D'),
        Key::Named(NamedKey::Home) => home_end_sequence(mode, true),
        Key::Named(NamedKey::End) => home_end_sequence(mode, false),
        Key::Named(NamedKey::Insert) => b"\x1b[2~".to_vec(),
        Key::Named(NamedKey::Delete) => b"\x1b[3~".to_vec(),
        Key::Named(NamedKey::PageUp) => b"\x1b[5~".to_vec(),
        Key::Named(NamedKey::PageDown) => b"\x1b[6~".to_vec(),
        Key::Named(NamedKey::F1) => b"\x1bOP".to_vec(),
        Key::Named(NamedKey::F2) => b"\x1bOQ".to_vec(),
        Key::Named(NamedKey::F3) => b"\x1bOR".to_vec(),
        Key::Named(NamedKey::F4) => b"\x1bOS".to_vec(),
        Key::Named(NamedKey::F5) => b"\x1b[15~".to_vec(),
        Key::Named(NamedKey::F6) => b"\x1b[17~".to_vec(),
        Key::Named(NamedKey::F7) => b"\x1b[18~".to_vec(),
        Key::Named(NamedKey::F8) => b"\x1b[19~".to_vec(),
        Key::Named(NamedKey::F9) => b"\x1b[20~".to_vec(),
        Key::Named(NamedKey::F10) => b"\x1b[21~".to_vec(),
        Key::Named(NamedKey::F11) => b"\x1b[23~".to_vec(),
        Key::Named(NamedKey::F12) => b"\x1b[24~".to_vec(),
        _ => {
            if modifiers.control_key() {
                let ch = match &event.logical_key {
                    Key::Character(text) => text.chars().next(),
                    _ => event.text.as_ref().and_then(|text| text.chars().next()),
                }?;
                vec![control_byte(ch)?]
            } else {
                event.text
                    .as_ref()
                    .map(|text| text.as_bytes().to_vec())
                    .or_else(|| match &event.logical_key {
                        Key::Character(text) => Some(text.as_bytes().to_vec()),
                        _ => None,
                    })?
            }
        }
    };
    if modifiers.alt_key() && !bytes.is_empty() && bytes[0] != 0x1b {
        let mut with_alt = vec![0x1b];
        with_alt.extend(bytes);
        bytes = with_alt;
    }
    Some(bytes)
}

pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let mut bytes = b"\x1b[200~".to_vec();
        bytes.extend(text.as_bytes());
        bytes.extend(b"\x1b[201~");
        bytes
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

pub fn pasteable_text(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().filter(|ch| !matches!(ch, '\r' | '\n' | '\u{7f}'))
}

fn modifiers_to_crossterm(modifiers: ModifiersState) -> KeyModifiers {
    let mut mapped = KeyModifiers::empty();
    if modifiers.shift_key() {
        mapped.insert(KeyModifiers::SHIFT);
    }
    if modifiers.control_key() {
        mapped.insert(KeyModifiers::CONTROL);
    }
    if modifiers.alt_key() {
        mapped.insert(KeyModifiers::ALT);
    }
    mapped
}

fn cursor_sequence(mode: TermMode, final_byte: u8) -> Vec<u8> {
    if mode.contains(TermMode::APP_CURSOR) {
        vec![0x1b, b'O', final_byte]
    } else {
        vec![0x1b, b'[', final_byte]
    }
}

fn home_end_sequence(mode: TermMode, home: bool) -> Vec<u8> {
    if mode.contains(TermMode::APP_CURSOR) {
        vec![0x1b, b'O', if home { b'H' } else { b'F' }]
    } else {
        vec![0x1b, b'[', if home { b'H' } else { b'F' }]
    }
}

fn control_byte(ch: char) -> Option<u8> {
    Some(match ch.to_ascii_lowercase() {
        '@' | ' ' | '2' => 0x00,
        'a'..='z' => (ch.to_ascii_lowercase() as u8) & 0x1f,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '7' | '/' => 0x1f,
        '8' | '?' => 0x7f,
        _ => return None,
    })
}
