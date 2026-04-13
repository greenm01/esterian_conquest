enum ClipboardState {
    Uninitialized,
    Available(arboard::Clipboard),
    Unavailable,
}

pub struct Clipboard {
    state: ClipboardState,
    override_text: Option<String>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            state: ClipboardState::Uninitialized,
            override_text: None,
        }
    }

    pub fn get_text(&mut self) -> Option<String> {
        if let Some(text) = self.override_text.clone() {
            return Some(text);
        }
        self.ensure_available();
        let ClipboardState::Available(inner) = &mut self.state else {
            return None;
        };
        match inner.get_text() {
            Ok(text) => Some(text),
            Err(err) => {
                tracing::warn!("Disabling nc-dash clipboard after backend read failure: {err}");
                self.state = ClipboardState::Unavailable;
                None
            }
        }
    }

    pub fn replace_fallback(&mut self, text: String) {
        self.override_text = Some(text);
    }

    fn ensure_available(&mut self) {
        if !matches!(self.state, ClipboardState::Uninitialized) {
            return;
        }
        match arboard::Clipboard::new() {
            Ok(clipboard) => {
                self.state = ClipboardState::Available(clipboard);
            }
            Err(err) => {
                tracing::warn!("Disabling nc-dash clipboard after backend init failure: {err}");
                self.state = ClipboardState::Unavailable;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn disable_for_tests(&mut self) {
        self.state = ClipboardState::Unavailable;
    }
}

#[cfg(test)]
mod tests {
    use super::{Clipboard, ClipboardState};

    #[test]
    fn clipboard_starts_lazy_and_uses_override_text_without_backend_init() {
        let mut clipboard = Clipboard::new();
        clipboard.replace_fallback("relay-note".to_string());

        assert!(matches!(clipboard.state, ClipboardState::Uninitialized));
        assert_eq!(clipboard.get_text().as_deref(), Some("relay-note"));
        assert!(matches!(clipboard.state, ClipboardState::Uninitialized));
    }

    #[test]
    fn unavailable_clipboard_stays_disabled() {
        let mut clipboard = Clipboard::new();
        clipboard.disable_for_tests();

        assert_eq!(clipboard.get_text(), None);
        assert!(matches!(clipboard.state, ClipboardState::Unavailable));
        assert_eq!(clipboard.get_text(), None);
        assert!(matches!(clipboard.state, ClipboardState::Unavailable));
    }
}
