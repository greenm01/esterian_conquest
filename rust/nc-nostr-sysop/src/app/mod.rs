use std::collections::VecDeque;
use chrono::{DateTime, Utc};

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

pub struct App {
    pub channels: Vec<SysopChannel>,
    pub active_channel_index: usize,
    pub messages: VecDeque<SysopMessage>,
    pub input: String,
    pub should_quit: bool,
    pub status_line: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            channels: vec![SysopChannel::Global],
            active_channel_index: 0,
            messages: VecDeque::with_capacity(1000),
            input: String::new(),
            should_quit: false,
            status_line: "Initializing...".to_string(),
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
