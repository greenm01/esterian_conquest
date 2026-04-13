use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::lobby::ratatui::{
    panel_block, scroll_offset, truncate_title, with_panel_bg, write_text,
};
use crate::lobby::state::{LobbyFocus, LobbyState, ThreadPaneFocus};
use crate::theme;

use super::format::{ThreadRenderLine, direct_thread_render_lines, thread_prompt_label};
use super::layout::workspace_layout;

pub fn render_direct_thread_surface(
    buffer: &mut Buffer,
    area: Rect,
    state: &LobbyState,
    modal: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let split = workspace_layout(area);
    render_thread_history(buffer, split.transcript, state, modal);
    render_thread_contacts(buffer, split.contacts, state);
    render_thread_footer(buffer, split.footer, state, modal);
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

fn render_thread_history(buffer: &mut Buffer, area: Rect, state: &LobbyState, modal: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let title = format!(
        " THREAD: {} ",
        truncate_title(&state.direct_thread_context_display(), 24)
    );
    let block = panel_block(
        &title,
        state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Transcript,
    );
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if state.selected_direct_contact().is_none() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<choose a direct contact>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }
    let lines = direct_thread_render_lines(state, inner.width as usize);
    if lines.is_empty() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no encrypted direct messages>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }
    let visible_rows = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(visible_rows);
    let scroll = state.thread_scroll.min(max_scroll);
    let end = lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows);
    let visible = &lines[start..end];
    let first_row = inner.bottom().saturating_sub(visible.len() as u16);
    for (idx, line) in visible.iter().enumerate() {
        let row = first_row + idx as u16;
        render_thread_line(buffer, row, inner.x, inner.width as usize, line);
    }
    if modal && scroll > 0 && inner.y < inner.bottom() {
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

fn render_thread_contacts(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let title = if state.thread_unread_total() == 0 {
        " CONTACTS ".to_string()
    } else {
        format!(" CONTACTS ({}) ", state.thread_unread_total())
    };
    let block = panel_block(
        &title,
        state.focus == LobbyFocus::Thread && state.thread_pane_focus == ThreadPaneFocus::Contacts,
    );
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let header_row = inner.y;
    buffer.set_stringn(
        inner.x,
        header_row,
        "Buffers",
        inner.width as usize,
        with_panel_bg(styles.label),
    );
    let visible = state.visible_direct_contacts();
    if visible.is_empty() {
        if inner.height > 1 {
            buffer.set_stringn(
                inner.x,
                header_row + 1,
                "<no direct contacts>",
                inner.width as usize,
                with_panel_bg(styles.dim),
            );
        }
        return;
    }

    let list_height = inner.height.saturating_sub(1) as usize;
    if list_height == 0 {
        return;
    }
    let selected = state.selected_visible_contact_index().unwrap_or(0);
    let scroll = scroll_offset(visible.len(), list_height, selected);
    for (offset, (absolute_index, contact)) in
        visible.iter().skip(scroll).take(list_height).enumerate()
    {
        let row = header_row + 1 + offset as u16;
        let style = if state.thread_pane_focus == ThreadPaneFocus::Contacts
            && state.focus == LobbyFocus::Thread
            && selected == scroll + offset
        {
            styles.selected
        } else if state.contact_selected == *absolute_index {
            with_panel_bg(styles.accent)
        } else {
            with_panel_bg(styles.value)
        };
        let unread = if contact.unread_count == 0 {
            String::new()
        } else {
            format!(" {}", contact.unread_count)
        };
        let label = format_contact_list_label(contact, inner.width as usize, &unread);
        buffer.set_stringn(inner.x, row, &label, inner.width as usize, style);
    }
}

fn render_thread_footer(buffer: &mut Buffer, area: Rect, state: &LobbyState, modal: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let title = match state.selected_direct_contact() {
        Some(contact) => format!(" CHAT TO: {} ", truncate_title(&contact.label, 18)),
        None => " CHAT ".to_string(),
    };
    let block = panel_block(&title, state.focus == LobbyFocus::Thread);
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if state.thread_composing {
        render_compose_footer(buffer, inner, state);
    } else {
        render_idle_footer(buffer, inner, state.focus == LobbyFocus::Thread, modal);
    }
}

fn render_compose_footer(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let styles = theme::tui_theme();
    let nick = thread_prompt_label(state);
    let prompt = format!("<{nick}>: ");
    let prompt_style = with_panel_bg(theme::tui_theme().success);
    let text_style = with_panel_bg(styles.value);
    let mut col = area.x;
    let end = area.right();
    col = write_text(buffer, area.y, col, end, &prompt, prompt_style);
    if col >= end {
        return;
    }
    let visible_width = end.saturating_sub(col) as usize;
    let draft = trailing_chars(&state.compose_message_input, visible_width);
    buffer.set_stringn(col, area.y, &draft, visible_width, text_style);
}

fn render_idle_footer(buffer: &mut Buffer, area: Rect, focused: bool, modal: bool) {
    let styles = theme::tui_theme();
    let style = if focused {
        with_panel_bg(styles.menu)
    } else {
        with_panel_bg(styles.dim)
    };
    let text = if modal {
        "M>essage  Type to message  A>ddrBook  [ / ] Pane  J/K Move  Esc Close"
    } else {
        "M>essage  Type to message  A>ddrBook  [ / ] Pane  J/K Move  D>elete  Enter Popout"
    };
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.x + area.width.saturating_sub(width) / 2;
    buffer.set_stringn(start, area.y, text, area.width as usize, style);
}

fn format_contact_list_label(
    contact: &super::super::models::DirectContactRow,
    width: usize,
    unread_suffix: &str,
) -> String {
    let max_label = width.saturating_sub(unread_suffix.len()).max(1);
    let label = contact.label.chars().take(max_label).collect::<String>();
    format!("{label:<max_label$}{unread_suffix}")
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
