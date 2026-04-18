#![allow(dead_code)]

use nc_client::keychain::{Keychain, active_identity_npub, now_iso8601, push_new_identity};
use nc_helm::{
    App, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind, Point,
    StoredSession,
};

pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub fn left_click(column: usize, row: usize) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        position: Point::from_usize(column, row),
        modifiers: KeyModifiers::NONE,
    }
}

pub fn view_cursor(app: &App) -> Point {
    app.view().cursor().expect("cursor should be set")
}

pub fn dummy_session(handle: &str) -> StoredSession {
    let mut keychain = Keychain::empty();
    push_new_identity(&mut keychain, now_iso8601(), Some(handle.to_string()))
        .expect("new identity");
    let active_npub = active_identity_npub(&keychain).expect("npub");
    let active = keychain.active_identity().expect("active identity").clone();
    StoredSession {
        keychain,
        active_npub,
        active_nsec: active.nsec.clone(),
        active_handle: active.handle.clone(),
    }
}
