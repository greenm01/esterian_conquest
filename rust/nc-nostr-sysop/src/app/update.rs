use crossterm::event::{KeyCode, KeyEvent};
use super::{App, SysopMessage};
use chrono::Utc;

pub enum UpdateResult {
    None,
    MessageSent(String),
    Command(String),
    Quit,
    Redraw,
}

pub fn handle_input(app: &mut App, key: KeyEvent) -> UpdateResult {
    match app.input_mode {
        super::InputMode::Normal => match key.code {
            KeyCode::Char('i') => {
                app.input_mode = super::InputMode::Editing;
                UpdateResult::None
            }
            KeyCode::Char('q') => {
                app.should_quit = true;
                UpdateResult::Quit
            }
            KeyCode::Char('k') => {
                app.scroll_offset = app.scroll_offset.saturating_add(1);
                UpdateResult::None
            }
            KeyCode::Char('j') => {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
                UpdateResult::None
            }
            KeyCode::Tab => {
                app.active_channel_index = (app.active_channel_index + 1) % app.channels.len();
                app.scroll_offset = 0;
                UpdateResult::None
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap() as usize;
                if idx > 0 && idx <= app.channels.len() {
                    app.active_channel_index = idx - 1;
                    app.scroll_offset = 0;
                }
                UpdateResult::None
            }
            _ => UpdateResult::None,
        },
        super::InputMode::Editing => match key.code {
            KeyCode::Esc => {
                app.input_mode = super::InputMode::Normal;
                UpdateResult::None
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
                    app.scroll_offset = 0;
                    UpdateResult::MessageSent(input)
                }
            }
            _ => UpdateResult::None,
        },
    }
}

pub fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) -> UpdateResult {
    if mouse.kind != crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) {
        return UpdateResult::None;
    }

    let x = mouse.column;
    let y = mouse.row;

    // Check channel sidebar hits
    for (idx, rect) in &app.channel_rects {
        if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
            app.active_channel_index = *idx;
            app.scroll_offset = 0;
            return UpdateResult::Redraw;
        }
    }

    // Check input box hit
    let rect = app.input_rect;
    if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
        app.input_mode = super::InputMode::Editing;
        return UpdateResult::Redraw;
    }

    UpdateResult::None
}

fn handle_command(app: &mut App, input: &str) -> UpdateResult {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return UpdateResult::None;
    }
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
        "/join" => {
            if parts.len() < 2 {
                app.push_message(SysopMessage {
                    timestamp: Utc::now(),
                    channel: super::SysopChannel::Global,
                    sender: "SYSTEM".to_string(),
                    content: "Usage: /join <game_id>".to_string(),
                    is_own: false,
                });
                return UpdateResult::None;
            }
            let game_id = parts[1].to_string();
            let channel = super::SysopChannel::Game(game_id.clone());
            if !app.channels.contains(&channel) {
                app.channels.push(channel);
                app.active_channel_index = app.channels.len() - 1;
                app.scroll_offset = 0;
            }
            UpdateResult::None
        }
        "/msg" => {
            if parts.len() < 2 {
                app.push_message(SysopMessage {
                    timestamp: Utc::now(),
                    channel: super::SysopChannel::Global,
                    sender: "SYSTEM".to_string(),
                    content: "Usage: /msg <npub> [message]".to_string(),
                    is_own: false,
                });
                return UpdateResult::None;
            }
            let npub = parts[1].to_string();
            let channel = super::SysopChannel::Direct(npub.clone());
            
            if !app.channels.contains(&channel) {
                app.channels.push(channel);
            }
            
            // Switch to that channel
            if let Some(idx) = app.channels.iter().position(|c| c == &super::SysopChannel::Direct(npub.clone())) {
                app.active_channel_index = idx;
                app.scroll_offset = 0;
            }

            if parts.len() > 2 {
                let message = parts[2..].join(" ");
                app.push_message(SysopMessage {
                    timestamp: Utc::now(),
                    channel: super::SysopChannel::Direct(npub),
                    sender: "You".to_string(),
                    content: message.clone(),
                    is_own: true,
                });
                UpdateResult::MessageSent(message)
            } else {
                UpdateResult::None
            }
        }
        "/help" => {
            let help_text = "Available commands: /join <id>, /msg <npub>, /clear, /quit";
            app.push_message(SysopMessage {
                timestamp: Utc::now(),
                channel: app.active_channel().clone(),
                sender: "SYSTEM".to_string(),
                content: help_text.to_string(),
                is_own: false,
            });
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
