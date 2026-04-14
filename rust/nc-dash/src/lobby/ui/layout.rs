use ratatui::layout::{Constraint, Layout, Rect};

use crate::lobby::state::{LobbyApp, LobbyFocus, LobbyRoute, LobbyState};
use crate::overlays::frame::RelativePopupOrigin;

const HOME_MIN_WIDTH: u16 = 72;
const HOME_MIN_HEIGHT: u16 = 26;
const HEADER_HEIGHT: u16 = 5;
const FOOTER_HEIGHT: u16 = 5;

#[derive(Debug, Clone, Copy)]
pub struct HomeLayout {
    pub header: Rect,
    pub joined: Rect,
    pub open: Rect,
    pub comms: Rect,
    pub footer: Rect,
    pub body: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneHit {
    pub focus: LobbyFocus,
    pub selected_row: Option<usize>,
}

pub fn home_layout(area: Rect) -> Option<HomeLayout> {
    if area.width < HOME_MIN_WIDTH || area.height < HOME_MIN_HEIGHT {
        return None;
    }
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .areas(area);
    let [joined, open, comms] = Layout::horizontal([
        Constraint::Fill(28),
        Constraint::Fill(38),
        Constraint::Fill(34),
    ])
    .spacing(1)
    .areas(body);
    Some(HomeLayout {
        header,
        joined,
        open,
        comms,
        footer,
        body,
    })
}

pub fn hit_test_home(
    state: &LobbyState,
    geometry: nc_ui::ScreenGeometry,
    column: u16,
    row: u16,
) -> Option<PaneHit> {
    let layout = home_layout(Rect::new(0, 0, geometry.width() as u16, geometry.height() as u16))?;
    pane_hit(
        state,
        layout.joined,
        LobbyFocus::JoinedGames,
        column,
        row,
        state.joined_selected,
    )
    .or_else(|| {
        pane_hit(
            state,
            layout.open,
            LobbyFocus::OpenGames,
            column,
            row,
            state.open_selected,
        )
    })
    .or_else(|| {
        pane_hit(
            state,
            layout.comms,
            LobbyFocus::Thread,
            column,
            row,
            state.comms_selected,
        )
    })
}

pub fn popup_title_bar_contains(app: &LobbyApp, column: u16, row: u16) -> bool {
    active_popup_rect(app)
        .map(|popup| row == popup.y && column >= popup.x && column < popup.right())
        .unwrap_or(false)
}

pub fn active_popup_rect(app: &LobbyApp) -> Option<Rect> {
    let area = Rect::new(0, 0, app.geometry.width() as u16, app.geometry.height() as u16);
    let layout = home_layout(area)?;
    let size = match app.state.route {
        LobbyRoute::Settings => Some((60, 17)),
        LobbyRoute::ThemePicker => Some((82, 20)),
        LobbyRoute::ComposeInvite => Some((64, 11)),
        LobbyRoute::EditHandle => Some((58, 11)),
        LobbyRoute::ContactPicker => Some((64, 16)),
        LobbyRoute::AddContact => Some((60, 10)),
        _ if app.state.show_help => Some(help_popup_size(layout.body)),
        _ => None,
    }?;
    Some(popup_rect(layout.body, size, app.popup_position))
}

pub(crate) fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

pub(crate) fn padded_inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(2),
        area.y.saturating_add(2),
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    )
}

pub(crate) fn scroll_offset(total_rows: usize, visible_rows: usize, selected: usize) -> usize {
    if total_rows == 0 || visible_rows == 0 {
        return 0;
    }
    selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(total_rows.saturating_sub(visible_rows))
}

pub(super) fn popup_rect(
    parent: Rect,
    preferred: (u16, u16),
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    let (preferred_width, preferred_height) = preferred;
    let width = preferred_width.min(parent.width.saturating_sub(2)).max(10);
    let height = preferred_height.min(parent.height.saturating_sub(2)).max(5);
    if let Some(origin) = origin {
        let max_x = parent.right().saturating_sub(width);
        let max_y = parent.bottom().saturating_sub(height);
        let x = (parent.x + origin.col_offset as u16).min(max_x);
        let y = (parent.y + origin.row_offset as u16).min(max_y);
        return Rect::new(x, y, width, height);
    }
    let x = parent.x + parent.width.saturating_sub(width) / 2;
    let y = parent.y + parent.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

pub(super) fn help_popup_size(parent: Rect) -> (u16, u16) {
    (parent.width.saturating_sub(8).min(72), 17)
}

fn pane_hit(
    state: &LobbyState,
    area: Rect,
    focus: LobbyFocus,
    column: u16,
    row: u16,
    selected: usize,
) -> Option<PaneHit> {
    if !contains(area, column, row) {
        return None;
    }
    let content = padded_inner(area);
    let selected_row = if contains(content, column, row) {
        clicked_row(
            focus_row_count(state, focus),
            content,
            selected,
            row,
            table_header_rows(focus),
        )
    } else {
        None
    };
    Some(PaneHit {
        focus,
        selected_row,
    })
}

fn focus_row_count(state: &LobbyState, focus: LobbyFocus) -> usize {
    match focus {
        LobbyFocus::JoinedGames => state.joined_games.len(),
        LobbyFocus::OpenGames => state.open_games.len(),
        LobbyFocus::Thread => state.comms_hotlist_rows().len(),
    }
}

fn clicked_row(
    total_rows: usize,
    content: Rect,
    selected: usize,
    row: u16,
    header_rows: usize,
) -> Option<usize> {
    if total_rows == 0 || !contains(content, content.x, row) {
        return None;
    }
    let relative_row = row.saturating_sub(content.y) as usize;
    if relative_row < header_rows {
        return None;
    }
    let visible_rows = (content.height as usize).saturating_sub(header_rows);
    if visible_rows == 0 {
        return None;
    }
    let scroll = scroll_offset(total_rows, visible_rows, selected);
    let absolute_row = scroll + relative_row.saturating_sub(header_rows);
    (absolute_row < total_rows).then_some(absolute_row)
}

fn table_header_rows(focus: LobbyFocus) -> usize {
    match focus {
        LobbyFocus::JoinedGames => 1,
        LobbyFocus::OpenGames => 2,
        LobbyFocus::Thread => 1,
    }
}
