use crate::ratatui::layout::{Constraint, Layout, Rect};

use super::super::state::{LobbyState, ThreadPaneFocus};
use super::super::ui::{contains, padded_inner, scroll_offset};

#[derive(Debug, Clone, Copy)]
pub struct ThreadWorkspaceLayout {
    pub chat: Rect,
    pub transcript: Rect,
    pub unread: Rect,
    pub contacts: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadWorkspaceHit {
    pub selected_row: Option<usize>,
    pub pane_focus: ThreadPaneFocus,
}

pub fn workspace_layout(area: Rect) -> ThreadWorkspaceLayout {
    let [left, right] = Layout::horizontal([Constraint::Fill(4), Constraint::Length(30)])
        .spacing(1)
        .areas(area);
    let [transcript, chat] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(left);
    let [unread, contacts] = Layout::vertical([Constraint::Length(9), Constraint::Min(0)])
        .spacing(1)
        .areas(right);
    ThreadWorkspaceLayout {
        chat,
        transcript,
        unread,
        contacts,
    }
}

pub fn hit_test_workspace(
    state: &LobbyState,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<ThreadWorkspaceHit> {
    if !contains(area, column, row) {
        return None;
    }
    let split = workspace_layout(area);
    if contains(split.unread, column, row) {
        let content = padded_inner(split.unread);
        return Some(ThreadWorkspaceHit {
            selected_row: clicked_unread_row(state, content, row),
            pane_focus: ThreadPaneFocus::New,
        });
    }
    if contains(split.contacts, column, row) {
        let content = padded_inner(split.contacts);
        return Some(ThreadWorkspaceHit {
            selected_row: clicked_sidebar_row(state, content, row),
            pane_focus: ThreadPaneFocus::Threads,
        });
    }
    Some(ThreadWorkspaceHit {
        selected_row: None,
        pane_focus: ThreadPaneFocus::Chat,
    })
}

fn clicked_unread_row(state: &LobbyState, content: Rect, row: u16) -> Option<usize> {
    if row < content.y || row >= content.bottom() || content.width == 0 {
        return None;
    }
    let list_rows = state.comms_unread_rows();
    if list_rows.is_empty() {
        return None;
    }
    let visible_rows = content.height as usize;
    if visible_rows == 0 {
        return None;
    }
    let selected = state
        .comms_new_selected
        .min(list_rows.len().saturating_sub(1));
    let scroll = scroll_offset(list_rows.len(), visible_rows, selected);
    let clicked = scroll + usize::from(row.saturating_sub(content.y));
    (clicked < list_rows.len()).then_some(clicked)
}

fn clicked_sidebar_row(state: &LobbyState, content: Rect, row: u16) -> Option<usize> {
    if row < content.y || row >= content.bottom() || content.width == 0 {
        return None;
    }
    let list_rows = state.comms_sidebar_rows();
    let total_rows = list_rows.len();
    if total_rows == 0 {
        return None;
    }
    let visible_rows = content.height as usize;
    if visible_rows == 0 {
        return None;
    }
    let selected = state
        .active_comms_row()
        .and_then(|active| list_rows.iter().position(|row| row.key == active.key))
        .unwrap_or(0);
    let scroll = scroll_offset(total_rows, visible_rows, selected);
    let clicked = scroll + usize::from(row.saturating_sub(content.y));
    (clicked < total_rows).then_some(clicked)
}
