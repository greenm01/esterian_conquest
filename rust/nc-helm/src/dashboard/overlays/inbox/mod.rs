use crate::dashboard::app::state::{DashApp, InboxPromptMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::modal::Rect;

pub mod compose;
pub mod editor;
pub mod list;

pub(crate) const HOTKEYS: &str = "? M I A Y D C O <ESC>";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    match app.inbox_overlay.prompt_mode {
        InboxPromptMode::ComposeRecipient
        | InboxPromptMode::ComposeSubject
        | InboxPromptMode::ComposeConfirm => {
            compose::draw(buf, app, map_frame);
        }
        InboxPromptMode::ComposeBody => {
            editor::draw(buf, app, map_frame);
        }
        _ => {
            list::draw(buf, app, map_frame);
        }
    }
}

pub fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    match app.inbox_overlay.prompt_mode {
        InboxPromptMode::ComposeRecipient
        | InboxPromptMode::ComposeSubject
        | InboxPromptMode::ComposeConfirm => compose::popup_rect(app, map_frame),
        InboxPromptMode::ComposeBody => editor::popup_rect(app, map_frame),
        _ => list::popup_rect(app, map_frame),
    }
}

pub use list::{hit_test_inbox_pane, inbox_items, selection_rows, staged_outbox_messages};
