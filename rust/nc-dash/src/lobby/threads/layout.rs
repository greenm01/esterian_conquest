use ratatui::layout::{Constraint, Layout, Rect};

use super::super::ratatui::{contains, padded_inner, scroll_offset};
use super::super::state::{LobbyState, ThreadPaneFocus};

#[derive(Debug, Clone, Copy)]
pub struct ThreadWorkspaceLayout {
    pub transcript: Rect,
    pub contacts: Rect,
    pub footer: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadWorkspaceHit {
    pub selected_row: Option<usize>,
    pub pane_focus: ThreadPaneFocus,
}

pub fn workspace_layout(area: Rect) -> ThreadWorkspaceLayout {
    let [top, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(5)])
        .spacing(1)
        .areas(area);
    let [transcript, contacts] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(24)])
            .spacing(1)
            .areas(top);
    ThreadWorkspaceLayout {
        transcript,
        contacts,
        footer,
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
    if contains(split.contacts, column, row) {
        let content = padded_inner(split.contacts);
        let visible = state.visible_direct_contacts();
        let selected_row = clicked_contact_row(
            visible.len(),
            content,
            state.selected_visible_contact_index().unwrap_or(0),
            row,
        );
        return Some(ThreadWorkspaceHit {
            selected_row,
            pane_focus: ThreadPaneFocus::Contacts,
        });
    }
    Some(ThreadWorkspaceHit {
        selected_row: None,
        pane_focus: ThreadPaneFocus::Transcript,
    })
}

fn clicked_contact_row(
    total_rows: usize,
    content: Rect,
    selected: usize,
    row: u16,
) -> Option<usize> {
    if total_rows == 0 || row < content.y || row >= content.bottom() || content.width == 0 {
        return None;
    }
    let [_header_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(content);
    if row < list_area.y || row >= list_area.bottom() {
        return None;
    }
    let visible_rows = list_area.height as usize;
    if visible_rows == 0 {
        return None;
    }
    let scroll = scroll_offset(total_rows, visible_rows, selected);
    let clicked = scroll + usize::from(row.saturating_sub(list_area.y));
    (clicked < total_rows).then_some(clicked)
}
