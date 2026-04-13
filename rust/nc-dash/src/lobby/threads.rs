use chrono::{DateTime, Local};

use super::models::{LobbyNotice, ThreadMessage};
use super::state::LobbyState;

pub fn notice_rows(state: &LobbyState) -> Vec<String> {
    state.notices.iter().map(format_notice).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRenderLine {
    pub timestamp: Option<String>,
    pub nick: Option<String>,
    pub body: String,
    pub indent: usize,
    pub nick_key: String,
    pub outgoing: bool,
}

pub fn thread_render_lines(state: &LobbyState, width: usize) -> Vec<ThreadRenderLine> {
    state
        .visible_thread_messages()
        .iter()
        .flat_map(|message| format_thread_message(state, message, width))
        .collect()
}

pub fn thread_rows(state: &LobbyState) -> Vec<String> {
    thread_render_lines(state, 72)
        .into_iter()
        .map(|line| {
            let mut row = String::new();
            if let Some(timestamp) = line.timestamp {
                row.push_str(&timestamp);
            } else if line.indent > 0 {
                row.push_str(&" ".repeat(line.indent));
            }
            if let Some(nick) = line.nick {
                row.push_str(&nick);
            }
            row.push_str(&line.body);
            row
        })
        .collect()
}

pub fn thread_prompt_label(state: &LobbyState) -> String {
    state
        .player_handle
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("you")
        .to_string()
}

fn format_notice(notice: &LobbyNotice) -> String {
    format!("{}: {}", notice.sender, notice.body)
}

fn format_thread_message(
    state: &LobbyState,
    message: &&ThreadMessage,
    width: usize,
) -> Vec<ThreadRenderLine> {
    let timestamp = short_local_time(&message.created_at);
    let nick = if message.outgoing {
        thread_prompt_label(state)
    } else if message.sender.trim().is_empty() {
        "daemon".to_string()
    } else {
        message.sender.clone()
    };
    let timestamp_prefix = format!("[{timestamp}] ");
    let nick_prefix = format!("<{nick}>: ");
    let indent = timestamp_prefix.chars().count() + nick_prefix.chars().count();
    let available_width = width.saturating_sub(indent).max(1);
    let wrapped = wrap_chat_body(&message.body, available_width);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(idx, segment)| ThreadRenderLine {
            timestamp: (idx == 0).then_some(timestamp_prefix.clone()),
            nick: (idx == 0).then_some(nick_prefix.clone()),
            body: segment,
            indent: if idx == 0 { 0 } else { indent },
            nick_key: nick.clone(),
            outgoing: message.outgoing,
        })
        .collect()
}

fn short_local_time(raw: &str) -> String {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Local).format("%H:%M").to_string())
        .unwrap_or_else(|_| "--:--".to_string())
}

fn wrap_chat_body(body: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let normalized = body.replace(['\r', '\n'], " ");
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in normalized.split_whitespace() {
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            push_long_word_lines(&mut lines, word, width);
            continue;
        }
        let extra = if current.is_empty() { 0 } else { 1 };
        if current.chars().count() + extra + word.chars().count() > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_long_word_lines(lines: &mut Vec<String>, word: &str, width: usize) {
    let mut current = String::new();
    for ch in word.chars() {
        current.push(ch);
        if current.chars().count() == width {
            lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
}
