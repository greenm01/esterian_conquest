#![allow(dead_code)]

use std::time::Instant;

use crate::dashboard::client_settings::PersistedWindowState;
use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::input::MouseEvent;
use crate::dashboard::ui::UiScene;

pub(crate) trait NativeApp {
    fn window_title(&self) -> &'static str;
    fn geometry(&self) -> ScreenGeometry;
    fn wants_window_focus(&self) -> bool {
        false
    }
    fn wants_text_input(&self) -> bool {
        false
    }
    fn saved_window_state(&self) -> Option<PersistedWindowState> {
        None
    }
    fn persist_window_state(&mut self, _state: PersistedWindowState) -> Result<(), String> {
        Ok(())
    }
    fn dispatch_key_event(&mut self, key: crate::dashboard::input::KeyEvent);
    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool;
    fn resize_canvas(&mut self, cols: u16, rows: u16);
    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>>;
    fn debug_render_signature(&self) -> Option<String> {
        None
    }
    fn on_idle(&mut self) -> bool {
        false
    }
    fn is_dragging_surface(&self) -> bool {
        false
    }
    fn note_user_activity(&mut self, _now: Instant) {}
    fn next_wakeup(&self) -> Option<Instant> {
        None
    }
    fn should_quit(&self) -> bool;
    fn set_should_quit(&mut self, should_quit: bool);
}
