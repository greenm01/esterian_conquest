use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1 << 0);
    pub const CONTROL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Esc,
    F(u8),
    Char(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

pub fn key_modifiers_from_winit(modifiers: ModifiersState) -> KeyModifiers {
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

pub fn key_event_from_winit(event: &WinitKeyEvent, modifiers: ModifiersState) -> Option<KeyEvent> {
    key_event_from_parts(
        &event.logical_key,
        event.text.as_deref(),
        event.state,
        modifiers,
    )
}

fn key_event_from_parts(
    logical_key: &Key,
    text: Option<&str>,
    state: ElementState,
    modifiers: ModifiersState,
) -> Option<KeyEvent> {
    if state != ElementState::Pressed {
        return None;
    }
    let key_modifiers = key_modifiers_from_winit(modifiers);
    let code = match logical_key {
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
            let ch = text
                .and_then(|text| text.chars().next())
                .filter(|ch| !ch.is_control())
                .or_else(|| match logical_key {
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

#[cfg(test)]
mod tests {
    use super::{KeyCode, KeyEvent, KeyModifiers, key_event_from_parts, key_modifiers_from_winit};
    use winit::event::ElementState;
    use winit::keyboard::{Key, ModifiersState, NamedKey};

    #[test]
    fn named_keys_map_to_local_codes() {
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::ArrowUp),
                None,
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        );
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::F12),
                None,
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            Some(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE))
        );
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::Tab),
                None,
                ElementState::Pressed,
                ModifiersState::SHIFT,
            ),
            Some(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn text_preferred_for_printable_characters() {
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::Enter),
                Some("\r"),
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        );
        assert_eq!(
            key_event_from_parts(
                &Key::Character("x".into()),
                Some("x"),
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            Some(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        );
    }

    #[test]
    fn logical_character_fallback_handles_missing_text() {
        assert_eq!(
            key_event_from_parts(
                &Key::Character("Q".into()),
                None,
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            Some(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE))
        );
    }

    #[test]
    fn control_characters_are_lowercased() {
        assert_eq!(
            key_event_from_parts(
                &Key::Character("V".into()),
                Some("V"),
                ElementState::Pressed,
                ModifiersState::CONTROL,
            ),
            Some(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL))
        );
    }

    #[test]
    fn unsupported_or_released_keys_return_none() {
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::Enter),
                None,
                ElementState::Released,
                ModifiersState::empty(),
            ),
            None
        );
        assert_eq!(
            key_event_from_parts(
                &Key::Named(NamedKey::Shift),
                None,
                ElementState::Pressed,
                ModifiersState::empty(),
            ),
            None
        );
    }

    #[test]
    fn winit_modifiers_map_to_local_modifiers() {
        let mut expected = KeyModifiers::SHIFT;
        expected.insert(KeyModifiers::CONTROL);
        assert_eq!(
            key_modifiers_from_winit(ModifiersState::SHIFT | ModifiersState::CONTROL),
            expected
        );
        assert_eq!(
            key_modifiers_from_winit(ModifiersState::ALT),
            KeyModifiers::ALT
        );
    }
}
