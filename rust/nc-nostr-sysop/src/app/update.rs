use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
