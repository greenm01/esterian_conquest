use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Chat & Sidebar
            Constraint::Length(1), // Status
            Constraint::Length(1), // Input
        ])
        .split(f.area());

    // 1. Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" NC-NOSTR-SYSOP ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" - Remote Game Management"),
    ]));
    f.render_widget(header, chunks[0]);

    // 2. Chat & Sidebar split
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Sidebar
            Constraint::Min(0),     // Chat
        ])
        .split(chunks[1]);

    // Sidebar (Channels/Games)
    let channels: Vec<ListItem> = app.channels.iter().enumerate().map(|(i, c)| {
        let style = if i == app.active_channel_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(c.label()).style(style)
    }).collect();

    let sidebar = List::new(channels)
        .block(Block::default().borders(Borders::RIGHT).title("Games"));
    f.render_widget(sidebar, body_chunks[0]);

    // Main Chat Transcript
    let active_channel = app.active_channel();
    let messages: Vec<ListItem> = app.messages.iter()
        .filter(|m| &m.channel == active_channel)
        .map(|m| {
        let time = m.timestamp.format("%H:%M:%S ").to_string();
        let sender = format!("{}: ", m.sender);
        
        let spans = vec![
            Span::styled(time, Style::default().fg(Color::DarkGray)),
            Span::styled(sender, Style::default().fg(if m.is_own { Color::Cyan } else { Color::Green })),
            Span::raw(&m.content),
        ];

        ListItem::new(Line::from(spans))
    }).collect();

    let chat = List::new(messages)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(chat, body_chunks[1]);

    // 3. Status Line
    let status = Paragraph::new(app.status_line.as_str())
        .style(Style::default().fg(Color::Black).bg(Color::White));
    f.render_widget(status, chunks[2]);

    // 4. Input Bar
    let input = Paragraph::new(format!("> {}", app.input))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input, chunks[3]);
}
