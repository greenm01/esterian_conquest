use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::lobby::LobbyApp;
use crate::lobby::models::{CommsConversationKind, CommsConversationRow, DirectContactRow};
use crate::lobby::ui::{
    panel_block, scroll_offset, truncate_title, with_panel_bg, write_text,
};
use crate::lobby::state::{LobbyFocus, LobbyState, ThreadPaneFocus};
use crate::theme;

use super::format::{
    ThreadRenderLine, direct_thread_render_lines, notice_render_lines,
};
use super::layout::workspace_layout;

pub fn render_comms_hotlist_panel(
    buffer: &mut Buffer,
    area: Rect,
    focused: bool,
    state: &LobbyState,
) {
    const TYPE_WIDTH: u16 = 8;
    let styles = theme::tui_theme();
    let block = panel_block(" COMMS ", focused);
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header_area, body_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    buffer.set_stringn(
        header_area.x,
        header_area.y,
        &" ".repeat(header_area.width as usize),
        header_area.width as usize,
        with_panel_bg(styles.label),
    );
    let [type_area, title_area, preview_area] = Layout::horizontal([
        Constraint::Length(TYPE_WIDTH),
        Constraint::Fill(2),
        Constraint::Fill(3),
    ])
    .spacing(1)
    .areas(header_area);
    buffer.set_stringn(
        type_area.x,
        type_area.y,
        "Type",
        type_area.width as usize,
        with_panel_bg(styles.label),
    );
    buffer.set_stringn(
        title_area.x,
        title_area.y,
        "Thread",
        title_area.width as usize,
        with_panel_bg(styles.label),
    );
    buffer.set_stringn(
        preview_area.x,
        preview_area.y,
        "Preview",
        preview_area.width as usize,
        with_panel_bg(styles.label),
    );

    let rows = state.comms_hotlist_rows();
    if rows.is_empty() {
        buffer.set_stringn(
            body_area.x,
            body_area.y,
            "<no messages yet>",
            body_area.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let visible_rows = body_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let scroll = scroll_offset(rows.len(), visible_rows, state.comms_selected);
    for (offset, row) in rows.iter().skip(scroll).take(visible_rows).enumerate() {
        let line_area = Rect::new(body_area.x, body_area.y + offset as u16, body_area.width, 1);
        let row_style = if focused && state.comms_selected == scroll + offset {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        buffer.set_stringn(
            line_area.x,
            line_area.y,
            &" ".repeat(line_area.width as usize),
            line_area.width as usize,
            row_style,
        );
        let [type_area, title_area, preview_area] = Layout::horizontal([
            Constraint::Length(TYPE_WIDTH),
            Constraint::Fill(2),
            Constraint::Fill(3),
        ])
        .spacing(1)
        .areas(line_area);
        let unread = if row.unread_count == 0 {
            String::new()
        } else {
            format!(" {}", row.unread_count)
        };
        let title = format!(
            "{}{}",
            truncate_title(&row.title, title_area.width.saturating_sub(unread.len() as u16) as usize),
            unread
        );
        buffer.set_stringn(
            type_area.x,
            type_area.y,
            conversation_kind_label(row.kind),
            type_area.width as usize,
            row_style,
        );
        buffer.set_stringn(
            title_area.x,
            title_area.y,
            &title,
            title_area.width as usize,
            row_style,
        );
        buffer.set_stringn(
            preview_area.x,
            preview_area.y,
            &row.preview.replace('\n', " "),
            preview_area.width as usize,
            row_style,
        );
    }
}

pub fn render_comms_scene(buffer: &mut Buffer, area: Rect, app: &LobbyApp) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let split = workspace_layout(area);
    render_comms_chat_bar(buffer, split.chat, app);
    render_comms_transcript(buffer, split.transcript, &app.state);
    render_comms_unread(buffer, split.unread, &app.state);
    render_comms_sidebar(buffer, split.contacts, &app.state);
}

pub fn render_thread_line(
    buffer: &mut Buffer,
    row: u16,
    start_col: u16,
    width: usize,
    line: &ThreadRenderLine,
) {
    let styles = theme::tui_theme();
    let mut col = start_col;
    let end = start_col.saturating_add(width as u16);
    if let Some(timestamp) = line.timestamp.as_deref() {
        col = write_text(buffer, row, col, end, timestamp, with_panel_bg(styles.dim));
    }
    if let Some(nick) = line.nick.as_deref() {
        col = write_text(buffer, row, col, end, nick, nick_style_for(line));
    } else {
        col = col.saturating_add(line.indent as u16);
    }
    if col < end {
        let remaining = end.saturating_sub(col) as usize;
        buffer.set_stringn(
            col,
            row,
            &line.body,
            remaining,
            with_panel_bg(styles.value),
        );
    }
}

fn render_comms_transcript(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let styles = theme::tui_theme();
    let active_label = state
        .active_comms_row()
        .map(|row| row.title)
        .unwrap_or_else(|| "No Conversation".to_string());
    let title = format!(" THREAD: {} ", truncate_title(&active_label, 28));
    let block = panel_block(
        &title,
        state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Chat,
    );
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let lines = active_render_lines(state, inner.width as usize);
    if lines.is_empty() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no conversation selected>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let visible_rows = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(visible_rows);
    let scroll = state.comms_scroll.min(max_scroll);
    let start = scroll.min(lines.len());
    let end = (start + visible_rows).min(lines.len());
    let visible = &lines[start..end];
    let first_row = inner.y;
    for (idx, line) in visible.iter().enumerate() {
        render_thread_line(
            buffer,
            first_row + idx as u16,
            inner.x,
            inner.width as usize,
            line,
        );
    }
    if scroll > 0 && inner.y < inner.bottom() {
        let marker = format!("*** scrollback: {scroll}");
        buffer.set_stringn(
            inner.x,
            inner.y,
            &marker,
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
    }
}

fn render_comms_chat_bar(buffer: &mut Buffer, area: Rect, app: &LobbyApp) {
    let styles = theme::tui_theme();
    let state = &app.state;
    let border = if state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Chat
    {
        styles.accent
    } else {
        styles.border
    };
    let title = if state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Chat
    {
        styles.selected
    } else {
        styles.title
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [?] [ESC] ")
        .style(styles.body)
        .border_style(with_panel_bg(border))
        .title_style(with_panel_bg(title));
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some(active) = state.active_comms_row() else {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no active thread>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    };

    if active.read_only {
        buffer.set_stringn(
            inner.x.saturating_add(1),
            inner.y,
            "Broadcast is read-only.",
            inner.width.saturating_sub(1) as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let draft_x = inner.x.saturating_add(1);
    let draft_width = inner.width.saturating_sub(1) as usize;
    let draft = trailing_chars(&state.compose_message_input, draft_width.saturating_sub(1));
    let style = if state.focus == LobbyFocus::Thread
        && state.thread_pane_focus == ThreadPaneFocus::Chat
    {
        with_panel_bg(styles.value)
    } else {
        with_panel_bg(styles.dim)
    };
    if draft_width == 0 {
        return;
    }
    buffer.set_stringn(draft_x, inner.y, &draft, draft_width, style);
    if state.focus == LobbyFocus::Thread
        && state.thread_pane_focus == ThreadPaneFocus::Chat
        && app.comms_cursor_visible
    {
        let cursor_col = draft_x
            .saturating_add(draft.chars().count().min(draft_width.saturating_sub(1)) as u16);
        if cursor_col < inner.right() {
            buffer.set_stringn(
                cursor_col,
                inner.y,
                "█",
                1,
                with_panel_bg(styles.cursor),
            );
        }
    }
}

fn render_comms_unread(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let styles = theme::tui_theme();
    let unread_total = state.thread_unread_total();
    let title = if unread_total == 0 {
        " NEW ".to_string()
    } else {
        format!(" NEW ({unread_total}) ")
    };
    let block = panel_block(
        &title,
        state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::New,
    );
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let rows = state.comms_unread_rows();
    if rows.is_empty() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no unread threads>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let selected = state.comms_new_selected.min(rows.len().saturating_sub(1));
    let scroll = scroll_offset(rows.len(), inner.height as usize, selected);
    for (offset, row) in rows.iter().skip(scroll).take(inner.height as usize).enumerate() {
        let line_row = inner.y + offset as u16;
        let is_selected = selected == scroll + offset;
        let style = if is_selected
            && state.focus == LobbyFocus::Thread
            && state.thread_pane_focus == ThreadPaneFocus::New
        {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        let prefix = conversation_kind_group(row.kind);
        let suffix = format!(" {}", row.unread_count);
        let title_width = inner.width as usize - prefix.chars().count() - suffix.chars().count() - 2;
        let title = truncate_title(&row.title, title_width.max(1));
        let line = format!("{prefix} {title:<title_width$}{suffix}");
        buffer.set_stringn(inner.x, line_row, &line, inner.width as usize, style);
    }
}

fn render_comms_sidebar(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let styles = theme::tui_theme();
    let block = panel_block(
        " THREADS ",
        state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Threads,
    );
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let grouped = grouped_sidebar_rows(state);
    let line_count = grouped.len();
    if line_count == 0 {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no conversations>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let selected = sidebar_selected_index(state).unwrap_or(0);
    let scroll = scroll_offset(line_count, inner.height as usize, selected);
    for (offset, line) in grouped.iter().skip(scroll).take(inner.height as usize).enumerate() {
        let row = inner.y + offset as u16;
        match line {
            SidebarLine::Header(text) => {
                buffer.set_stringn(
                    inner.x,
                    row,
                    text,
                    inner.width as usize,
                    with_panel_bg(styles.label),
                );
            }
            SidebarLine::Empty(text) => {
                buffer.set_stringn(
                    inner.x,
                    row,
                    text,
                    inner.width as usize,
                    with_panel_bg(styles.dim),
                );
            }
            SidebarLine::Conversation(item) => {
                let is_selected = selected == scroll + offset;
                let style = if is_selected
                    && state.focus == LobbyFocus::Thread
                    && state.thread_pane_focus == ThreadPaneFocus::Threads
                {
                    styles.selected
                } else if state
                    .active_comms_row()
                    .is_some_and(|active| active.key == item.key)
                {
                    with_panel_bg(styles.accent)
                } else {
                    with_panel_bg(styles.value)
                };
                let suffix = sidebar_suffix(item);
                let max_title = inner.width as usize - suffix.chars().count();
                let title = truncate_title(&item.title, max_title.max(1));
                buffer.set_stringn(
                    inner.x,
                    row,
                    &format!("{title:<max_title$}{suffix}"),
                    inner.width as usize,
                    style,
                );
            }
        }
    }
}

fn active_render_lines(state: &LobbyState, width: usize) -> Vec<ThreadRenderLine> {
    match state.active_comms_row().map(|row| row.kind) {
        Some(CommsConversationKind::Announcement) => notice_render_lines(state, width),
        Some(CommsConversationKind::Direct) => direct_thread_render_lines(state, width),
        Some(CommsConversationKind::GameMail) => Vec::new(),
        None => Vec::new(),
    }
}

fn conversation_kind_label(kind: CommsConversationKind) -> &'static str {
    match kind {
        CommsConversationKind::Announcement => "BCAST",
        CommsConversationKind::GameMail => "GAME",
        CommsConversationKind::Direct => "DIRECT",
    }
}

fn conversation_kind_group(kind: CommsConversationKind) -> &'static str {
    match kind {
        CommsConversationKind::Announcement => "CAST",
        CommsConversationKind::GameMail => "GAME",
        CommsConversationKind::Direct => "DM",
    }
}

fn sidebar_suffix(row: &CommsConversationRow) -> String {
    if row.blocked {
        " !".to_string()
    } else if row.hidden {
        " h".to_string()
    } else if row.unread_count > 0 {
        format!(" {}", row.unread_count)
    } else {
        String::new()
    }
}

enum SidebarLine {
    Header(&'static str),
    Empty(&'static str),
    Conversation(CommsConversationRow),
}

fn grouped_sidebar_rows(state: &LobbyState) -> Vec<SidebarLine> {
    let rows = state.comms_sidebar_rows();
    let mut grouped = Vec::new();

    let announcements = rows
        .iter()
        .filter(|row| row.kind == CommsConversationKind::Announcement)
        .cloned()
        .collect::<Vec<_>>();
    grouped.push(SidebarLine::Header("BROADCAST"));
    if announcements.is_empty() {
        grouped.push(SidebarLine::Empty("<no broadcast threads>"));
    } else {
        grouped.extend(announcements.into_iter().map(SidebarLine::Conversation));
    }

    let direct = rows
        .iter()
        .filter(|row| row.kind == CommsConversationKind::Direct)
        .cloned()
        .collect::<Vec<_>>();
    grouped.push(SidebarLine::Header("DIRECT"));
    if direct.is_empty() {
        grouped.push(SidebarLine::Empty("<no direct threads>"));
    } else {
        grouped.extend(direct.into_iter().map(SidebarLine::Conversation));
    }

    grouped
}

fn sidebar_selected_index(state: &LobbyState) -> Option<usize> {
    let active = state.active_comms_row()?;
    let grouped = grouped_sidebar_rows(state);
    grouped.iter().position(|line| match line {
        SidebarLine::Header(_) => false,
        SidebarLine::Empty(_) => false,
        SidebarLine::Conversation(row) => row.key == active.key,
    })
}

fn trailing_chars(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        return text.to_string();
    }
    text.chars().skip(count - limit).collect()
}

fn nick_style_for(line: &ThreadRenderLine) -> Style {
    if line.outgoing {
        return with_panel_bg(theme::tui_theme().success);
    }
    let palette = [
        theme::tui_theme().accent,
        theme::tui_theme().warning,
        theme::tui_theme().error,
        theme::tui_theme().menu_hotkey,
    ];
    let hash = line
        .nick_key
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_mul(33).wrapping_add(byte as usize));
    with_panel_bg(palette[hash % palette.len()].add_modifier(Modifier::BOLD))
}

#[allow(dead_code)]
fn _contact_label(contact: &DirectContactRow) -> &str {
    &contact.label
}
