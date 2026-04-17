use chrono::{DateTime, Utc};
use std::collections::VecDeque;

pub mod update;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysopChannel {
    Global,
    Game(String),
    Direct(String), // npub
}

impl SysopChannel {
    pub fn label(&self) -> String {
        match self {
            Self::Global => "GLOBAL".to_string(),
            Self::Game(id) => format!("#{}", id),
            Self::Direct(npub) => format!("@{}", &npub[..12]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SysopMessage {
    pub timestamp: DateTime<Utc>,
    pub channel: SysopChannel,
    pub sender: String, // handle or npub
    pub content: String,
    pub is_own: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub channels: Vec<SysopChannel>,
    pub active_channel_index: usize,
    pub messages: VecDeque<SysopMessage>,
    pub scroll_offset: usize,
    pub chat_list_state: ratatui::widgets::ListState,
    pub input: String,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub status_line: String,
    pub sysop_handle: String,
    pub sysop_npub: String,
    pub relay_count: usize,
    pub connection_status: String,

    // UI Layout tracking for mouse interaction
    pub channel_rects: Vec<(usize, ratatui::layout::Rect)>,
    pub input_rect: ratatui::layout::Rect,
}

impl App {
    pub fn new() -> Self {
        Self {
            channels: vec![SysopChannel::Global],
            active_channel_index: 0,
            messages: VecDeque::with_capacity(1000),
            scroll_offset: 0,
            chat_list_state: ratatui::widgets::ListState::default(),
            input: String::new(),
            input_mode: InputMode::Normal,
            should_quit: false,
            status_line: "Initializing...".to_string(),
            sysop_handle: "sysop".to_string(),
            sysop_npub: String::new(),
            relay_count: 0,
            connection_status: "Connecting...".to_string(),
            channel_rects: Vec::new(),
            input_rect: ratatui::layout::Rect::default(),
        }
    }

    pub fn active_channel(&self) -> &SysopChannel {
        &self.channels[self.active_channel_index]
    }

    pub fn push_message(&mut self, msg: SysopMessage) {
        if self.messages.len() >= 1000 {
            self.messages.pop_front();
        }
        self.messages.push_back(msg);
    }
}
