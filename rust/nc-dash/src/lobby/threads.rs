use super::models::{LobbyNotice, ThreadMessage};
use super::state::LobbyState;

pub fn notice_rows(state: &LobbyState) -> Vec<String> {
    state
        .notices
        .iter()
        .map(format_notice)
        .collect()
}

pub fn thread_rows(state: &LobbyState) -> Vec<String> {
    state
        .thread_messages
        .iter()
        .map(format_thread_message)
        .collect()
}

fn format_notice(notice: &LobbyNotice) -> String {
    format!("{}: {}", notice.sender, notice.body)
}

fn format_thread_message(message: &ThreadMessage) -> String {
    let prefix = if message.outgoing { "you" } else { &message.sender };
    format!("{prefix}: {}", message.body)
}
