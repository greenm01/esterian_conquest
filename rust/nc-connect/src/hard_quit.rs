use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_hard_quit_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('c' | 'C'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL)
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::is_hard_quit_key;

    #[test]
    fn ctrl_c_is_hard_quit() {
        assert!(is_hard_quit_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
    }

    #[test]
    fn plain_c_is_not_hard_quit() {
        assert!(!is_hard_quit_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
        )));
    }
}
