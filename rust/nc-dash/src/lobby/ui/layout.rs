use ratatui::layout::{Constraint, Layout, Rect};

use crate::lobby::state::{LobbyApp, LobbyRoute, LobbyState, LobbyTab};
use crate::overlays::frame::RelativePopupOrigin;

const HOME_MIN_WIDTH: u16 = 72;
const HOME_MIN_HEIGHT: u16 = 20;
const MAX_HOME_WIDTH: u16 = 140;
const TABLE_TAB_MAX_WIDTH: u16 = 120;
const HEADER_HEIGHT: u16 = 2;
const FOOTER_HEIGHT: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub struct HomeLayout {
    pub shell: Rect,
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneHit {
    pub tab: LobbyTab,
    pub selected_row: Option<usize>,
}

pub fn home_layout(area: Rect) -> Option<HomeLayout> {
    if area.width < HOME_MIN_WIDTH || area.height < HOME_MIN_HEIGHT {
        return None;
    }
    let [_, shell, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(MAX_HOME_WIDTH),
        Constraint::Fill(1),
    ])
    .areas(area);
    let inner = super::chrome::shell_inner(shell);

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .areas(inner);
    Some(HomeLayout {
        shell,
        header,
        body,
        footer,
    })
}

pub fn hit_test_home(
    state: &LobbyState,
    geometry: crate::geometry::ScreenGeometry,
    column: u16,
    row: u16,
) -> Option<PaneHit> {
    let layout = home_layout(Rect::new(
        0,
        0,
        geometry.width() as u16,
        geometry.height() as u16,
    ))?;
    if contains(layout.header, column, row) {
        if let Some(tab) = super::home::hit_test_tabs(state, layout.header, column, row) {
            return Some(PaneHit {
                tab,
                selected_row: None,
            });
        }
    }
    if !contains(layout.body, column, row) {
        return None;
    }
    let content = padded_inner(home_tab_content_area(layout.body, state.active_tab));
    let selected_row = if contains(content, column, row) {
        clicked_row(
            tab_row_count(state),
            content,
            tab_selected_index(state),
            row,
            tab_header_rows(state.active_tab),
        )
    } else {
        None
    };
    Some(PaneHit {
        tab: state.active_tab,
        selected_row,
    })
}

pub fn home_tab_content_area(body: Rect, tab: LobbyTab) -> Rect {
    match tab {
        LobbyTab::Comms => body,
        LobbyTab::MyGames | LobbyTab::OpenGames => centered_width(body, TABLE_TAB_MAX_WIDTH),
    }
}

pub fn popup_title_bar_contains(app: &LobbyApp, column: u16, row: u16) -> bool {
    active_popup_rect(app)
        .map(|popup| row == popup.y && column >= popup.x && column < popup.right())
        .unwrap_or(false)
}

pub fn active_popup_rect(app: &LobbyApp) -> Option<Rect> {
    let area = Rect::new(
        0,
        0,
        app.geometry.width() as u16,
        app.geometry.height() as u16,
    );
    let layout = home_layout(area)?;
    let size = match app.state.route {
        LobbyRoute::Settings => Some(super::popups::settings_popup_size(app, layout.body)),
        LobbyRoute::ThemePicker => Some(super::popups::theme_picker_popup_size(app, layout.body)),
        LobbyRoute::FirstJoinSetup => {
            Some(super::popups::first_join_setup_popup_size(app, layout.body))
        }
        LobbyRoute::QuitConfirm => Some(super::popups::quit_confirm_popup_size(app, layout.body)),
        LobbyRoute::ComposeInvite => {
            Some(super::popups::compose_invite_popup_size(app, layout.body))
        }
        LobbyRoute::SandboxJoinConfirm => Some(super::popups::sandbox_join_confirm_popup_size(
            app,
            layout.body,
        )),
        LobbyRoute::SandboxJoinUnavailable => Some(
            super::popups::sandbox_join_unavailable_popup_size(app, layout.body),
        ),
        LobbyRoute::EditHandle => Some(super::popups::edit_handle_popup_size(app, layout.body)),
        LobbyRoute::ContactPicker => {
            Some(super::popups::contact_picker_popup_size(app, layout.body))
        }
        LobbyRoute::AddContact => Some(super::popups::add_contact_popup_size(app, layout.body)),
        _ if app.state.show_manual => Some(super::popups::manual_popup_size(app, layout.body)),
        _ if app.state.show_help => Some(super::popups::help_popup_size(app, layout.body)),
        _ => None,
    }?;
    Some(popup_rect(layout.body, size, app.popup_position))
}

pub fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

pub fn padded_inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(2),
        area.y.saturating_add(2),
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    )
}

fn centered_width(area: Rect, width: u16) -> Rect {
    let width = area.width.min(width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    Rect::new(x, area.y, width, area.height)
}

pub fn scroll_offset(total_rows: usize, visible_rows: usize, selected: usize) -> usize {
    if total_rows == 0 || visible_rows == 0 {
        return 0;
    }
    selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(total_rows.saturating_sub(visible_rows))
}

pub fn popup_rect(
    parent: Rect,
    preferred: (u16, u16),
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    crate::modal_ratatui::placed_popup_rect(parent, preferred, origin)
}

fn tab_row_count(state: &LobbyState) -> usize {
    match state.active_tab {
        LobbyTab::MyGames => state.joined_games.len(),
        LobbyTab::OpenGames => state.open_games.len(),
        LobbyTab::Comms => state.comms_hotlist_rows().len(),
    }
}

fn tab_selected_index(state: &LobbyState) -> usize {
    match state.active_tab {
        LobbyTab::MyGames => state.joined_selected,
        LobbyTab::OpenGames => state.open_selected,
        LobbyTab::Comms => state.comms_selected,
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

fn tab_header_rows(tab: LobbyTab) -> usize {
    match tab {
        LobbyTab::MyGames => 1,
        LobbyTab::OpenGames => 2,
        LobbyTab::Comms => 1,
    }
}
