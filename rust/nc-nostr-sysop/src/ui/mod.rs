use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    // Reset hit detection rects
    app.channel_rects.clear();
    app.input_rect = Rect::default();

    // 1. Root Vertical Layout: [Title (3), Body (Min 0), Input (3)]
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title Bar
            Constraint::Min(0),    // Main Body
            Constraint::Length(3), // Input Area
        ])
        .split(size);
    
    app.input_rect = chunks[2];

    // --- DRAW TITLE BAR ---
    let current_channel = app.active_channel().label();
    
    // Left part of title: App name and handle
    let title_left = format!(" NC-NOSTR-SYSOP | {} ", app.sysop_handle);
    // Center part: Active channel
    let title_center = format!(" Channel: {} ", current_channel);
    // Right part: Connection status
    let title_right = format!(" Status: {} | Relays: {} ", app.connection_status, app.relay_count);

    let title_block = Block::default().borders(Borders::ALL);
    let title_inner = title_block.inner(chunks[0]);
    f.render_widget(title_block, chunks[0]);

    // Render components of the title bar
    f.render_widget(
        Paragraph::new(Span::styled(title_left, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        title_inner,
    );
    f.render_widget(
        Paragraph::new(title_center).alignment(Alignment::Center),
        title_inner,
    );
    f.render_widget(
        Paragraph::new(title_right).alignment(Alignment::Right),
        title_inner,
    );

    // 2. Body Horizontal Layout: [Chat (75%), Info Panel (25%)]
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    // --- DRAW CHAT AREA ---
    let active_channel = app.active_channel();
    let filtered_messages: Vec<&crate::app::SysopMessage> = app.messages.iter()
        .filter(|m| &m.channel == active_channel)
        .collect();

    let total_messages = filtered_messages.len();
    let scroll_indicator = if app.scroll_offset > 0 {
        format!(" [SCROLL: {}]", app.scroll_offset)
    } else {
        "".to_string()
    };

    let messages: Vec<ListItem> = filtered_messages.iter()
        .map(|m| {
            let time = m.timestamp.format("[%H:%M] ").to_string();
            let sender = format!("<{}> ", m.sender);
            
            let spans = vec![
                Span::styled(time, Style::default().fg(Color::DarkGray)),
                Span::styled(sender, Style::default().fg(if m.is_own { Color::Green } else { Color::Magenta })),
                Span::raw(&m.content),
            ];

            ListItem::new(Line::from(spans))
        }).collect();

    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} {} ", current_channel, scroll_indicator))
        .style(Style::default().fg(Color::Cyan));
    
    // Manage scroll state to keep bottom by default but allow j/k
    let list_height = body_chunks[0].height.saturating_sub(2) as usize;
    if total_messages > list_height {
        let top_index = (total_messages - list_height).saturating_sub(app.scroll_offset);
        app.chat_list_state.select(Some(top_index));
    } else {
        app.chat_list_state.select(Some(0));
    }

    let chat = List::new(messages).block(chat_block);
    f.render_stateful_widget(chat, body_chunks[0], &mut app.chat_list_state);

    // --- DRAW INFO PANEL ---
    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Identity
            Constraint::Length(4), // Connection
            Constraint::Min(0),    // Channels
        ])
        .split(body_chunks[1]);

    // Identity
    let id_text = vec![
        Line::from(vec![Span::raw("Nick: "), Span::styled(&app.sysop_handle, Style::default().fg(Color::Green))]),
        Line::from(vec![Span::raw("Type: "), Span::styled("sysop", Style::default().fg(Color::Yellow))]),
        Line::from(vec![Span::raw("Npub: "), Span::styled(if app.sysop_npub.len() > 12 { &app.sysop_npub[..12] } else { "" }, Style::default().fg(Color::Gray)), Span::raw("...")]),
    ];
    f.render_widget(Paragraph::new(id_text).block(Block::default().borders(Borders::ALL).title(" Identity ")), info_chunks[0]);

    // Connection
    let conn_text = vec![
        Line::from(vec![Span::raw("Status: "), Span::styled(&app.connection_status, Style::default().fg(Color::Green))]),
        Line::from(vec![Span::raw("Relays: "), Span::styled(app.relay_count.to_string(), Style::default().fg(Color::Cyan))]),
    ];
    f.render_widget(Paragraph::new(conn_text).block(Block::default().borders(Borders::ALL).title(" Connection ")), info_chunks[1]);

    // Channels List
    let sidebar_rect = info_chunks[2];
    let channel_items: Vec<ListItem> = app.channels.iter().enumerate().map(|(i, c)| {
        let style = if i == app.active_channel_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        // Track the Rect for each channel for mouse selection
        let channel_rect = Rect::new(sidebar_rect.x + 1, sidebar_rect.y + 1 + i as u16, sidebar_rect.width - 2, 1);
        app.channel_rects.push((i, channel_rect));

        ListItem::new(c.label()).style(style)
    }).collect();
    f.render_widget(List::new(channel_items).block(Block::default().borders(Borders::ALL).title(" Channels ")), sidebar_rect);

    // --- DRAW INPUT AREA ---
    let (input_text, mode_text, input_style) = match app.input_mode {
        crate::app::InputMode::Normal => (
            "".to_string(),
            "[NORMAL] Press 'i' to chat, 'q' to quit",
            Style::default().fg(Color::White),
        ),
        crate::app::InputMode::Editing => (
            format!("> {}", app.input),
            "[INSERT] ESC=normal, ENTER=send",
            Style::default().fg(Color::Green),
        ),
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(mode_text)
        .style(input_style);

    let input_para = Paragraph::new(input_text).block(input_block);
    f.render_widget(input_para, chunks[2]);

    // Set cursor in editing mode
    if app.input_mode == crate::app::InputMode::Editing {
        f.set_cursor_position((
            chunks[2].x + (app.input.len() as u16 + 3).min(chunks[2].width - 2),
            chunks[2].y + 1,
        ));
    }
}
