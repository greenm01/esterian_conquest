use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_quit_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('q' | 'Q'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
    )
}

pub fn is_escape_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Esc,
            ..
        }
    )
}

pub fn is_back_key(key: KeyEvent) -> bool {
    is_quit_key(key) || is_escape_key(key)
}

pub fn is_help_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('?'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
    )
}

pub fn is_manual_refresh_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(' '))
        && !key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
}

pub fn is_yes_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('y' | 'Y'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
    )
}

pub fn is_cancel_confirm_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Enter,
            ..
        }
    ) || is_back_key(key)
        || matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('n' | 'N'),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            }
        )
}
