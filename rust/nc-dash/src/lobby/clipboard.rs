#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use std::ffi::c_void;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

enum ClipboardState {
    Uninitialized,
    CrossPlatformAvailable(arboard::Clipboard),
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    WaylandAvailable(smithay_clipboard::Clipboard),
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

    pub fn attach_window(&mut self, window: &winit::window::Window) {
        if !matches!(self.state, ClipboardState::Uninitialized) {
            return;
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            let display = match window.display_handle() {
                Ok(handle) => match wayland_display_ptr(handle.as_raw()) {
                    Some(display) => display,
                    None => return,
                },
                Err(err) => {
                    tracing::warn!(
                        "Disabling nc-dash clipboard after display handle lookup failure: {err}"
                    );
                    self.state = ClipboardState::Unavailable;
                    return;
                }
            };
            self.state =
                ClipboardState::WaylandAvailable(unsafe { smithay_clipboard::Clipboard::new(display) });
        }
    }

    pub fn get_text(&mut self) -> Option<String> {
        if let Some(text) = self.override_text.clone() {
            return Some(text);
        }
        self.ensure_available();
        match &mut self.state {
            ClipboardState::CrossPlatformAvailable(inner) => match inner.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    tracing::warn!(
                        "Disabling nc-dash clipboard after backend read failure: {err}"
                    );
                    self.state = ClipboardState::Unavailable;
                    None
                }
            },
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            ClipboardState::WaylandAvailable(inner) => match inner.load() {
                Ok(text) => Some(text),
                Err(err) => {
                    tracing::warn!(
                        "Disabling nc-dash clipboard after Wayland clipboard read failure: {err}"
                    );
                    self.state = ClipboardState::Unavailable;
                    None
                }
            },
            ClipboardState::Uninitialized | ClipboardState::Unavailable => None,
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
                self.state = ClipboardState::CrossPlatformAvailable(clipboard);
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

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn wayland_display_ptr(raw_display_handle: RawDisplayHandle) -> Option<*mut c_void> {
    match raw_display_handle {
        RawDisplayHandle::Wayland(handle) => Some(handle.display.as_ptr()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Clipboard, ClipboardState};

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    use std::ptr::NonNull;

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    use winit::raw_window_handle::{RawDisplayHandle, WaylandDisplayHandle, XlibDisplayHandle};

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

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[test]
    fn wayland_display_handle_is_detected() {
        let mut display_value = 0_u8;
        let display = NonNull::from(&mut display_value).cast();
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display));

        assert_eq!(super::wayland_display_ptr(raw), Some(display.as_ptr()));
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[test]
    fn non_wayland_display_handle_is_ignored() {
        let raw = RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, 0));

        assert_eq!(super::wayland_display_ptr(raw), None);
    }
}
