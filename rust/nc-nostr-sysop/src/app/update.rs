use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::{App, SysopMessage};
use chrono::Utc;

pub enum UpdateResult {
    None,
    MessageSent(String),
    Command(String),
    Quit,
}

pub fn handle_input(app: &mut App, key: KeyEvent) -> UpdateResult {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            UpdateResult::Quit
        }
        KeyCode::Char(c) => {
            app.input.push(c);
            UpdateResult::None
        }
        KeyCode::Backspace => {
            app.input.pop();
            UpdateResult::None
        }
        KeyCode::Enter => {
            if app.input.is_empty() {
                return UpdateResult::None;
            }

            let input = std::mem::take(&mut app.input);
            if input.starts_with('/') {
                handle_command(app, &input)
            } else {
                let channel = app.active_channel().clone();
                app.push_message(SysopMessage {
                    timestamp: Utc::now(),
                    channel,
                    sender: "You".to_string(),
                    content: input.clone(),
                    is_own: true,
                });
                UpdateResult::MessageSent(input)
            }
        }
        KeyCode::Tab => {
            app.active_channel_index = (app.active_channel_index + 1) % app.channels.len();
            UpdateResult::None
        }
        _ => UpdateResult::None,
    }
}

fn handle_command(app: &mut App, input: &str) -> UpdateResult {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        "/quit" | "/exit" => {
            app.should_quit = true;
            UpdateResult::Quit
        }
        "/clear" => {
            app.messages.clear();
            UpdateResult::None
        }
        _ => {
            app.push_message(SysopMessage {
                timestamp: Utc::now(),
                channel: super::SysopChannel::Global,
                sender: "SYSTEM".to_string(),
                content: format!("Unknown command: {}", cmd),
                is_own: false,
            });
            UpdateResult::None
        }
    }
}
