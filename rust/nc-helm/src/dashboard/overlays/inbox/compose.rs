use crate::dashboard::app::state::{ActiveOverlay, DashApp, InboxPromptMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, overlay_chrome_height,
    overlay_popup_rect_for_body_in_parent, standard_table_body_height, write_clipped,
};
use crate::dashboard::table::{
    TableColumn, TableFooter, TableWidthMode, draw_scrollbar_at, resolve_table_columns_for_widget,
    table_render_width, write_table_window_with_cursor_at,
};
use crate::dashboard::theme;

const RECIPIENT_COLUMNS: [TableColumn<'static>; 2] =
    [TableColumn::right("ID", 3), TableColumn::left("Empire", 28)];

const TITLE: &str = "COMMUNICATE (SEND MESSAGE)";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let mode = app.inbox_overlay.prompt_mode;
    if mode == InboxPromptMode::ComposeRecipient {
        draw_recipient_picker(buf, app);
    } else {
        draw_prompt_overlay(buf, app);
    }
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let mode = app.inbox_overlay.prompt_mode;
    if mode == InboxPromptMode::ComposeRecipient {
        recipient_picker_rect(app)
    } else {
        prompt_overlay_rect(app)
    }
}

fn draw_recipient_picker(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let rows = recipient_rows(app);
    let footer = TableFooter::None;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let visible_rows = recipient_visible_rows(parent, rows.len(), footer);
    let scrollable = rows.len() > visible_rows;

    let columns = resolve_table_columns_for_widget(
        &RECIPIENT_COLUMNS,
        &rows,
        80, // Target width
        scrollable,
        TableWidthMode::Compact,
        Some(TITLE),
        Some(footer),
    );
    let table_width = table_render_width(&columns);
    let body_width = table_width + usize::from(scrollable);
    let body_height = standard_table_body_height(visible_rows);

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    assert_overlay_body_write_fits(frame, TITLE, body_width, body_height);

    let _metrics = write_table_window_with_cursor_at(
        buf,
        frame.body_row,
        frame.body_col,
        &columns,
        &rows,
        app.inbox_overlay.compose_recipient_scroll,
        visible_rows,
        theme::value_style(),
        theme::label_style(),
        Some(app.inbox_overlay.compose_recipient_selected),
        0,
    );

    draw_scrollbar_at(
        buf,
        frame.body_row,
        frame.body_col + table_width,
        visible_rows,
        rows.len(),
        app.inbox_overlay.compose_recipient_scroll,
        theme::table_theme(),
    );
}

fn draw_prompt_overlay(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let footer = prompt_footer(app);
    let body_width = 60;
    let body_height = 2;

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );

    let recipient_name = app
        .inbox_overlay
        .compose_recipient_empire
        .and_then(|id| {
            app.game_data
                .player
                .records
                .get(id as usize - 1)
                .map(|p| p.controlled_empire_name_summary())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        &format!("To: {recipient_name}"),
        theme::label_style(),
    );

    if app.inbox_overlay.prompt_mode == InboxPromptMode::ComposeSubject {
        write_clipped(
            buf,
            frame.body_row + 1,
            frame.body_col,
            frame.body_width,
            &format!("Subject: {}", app.inbox_overlay.prompt_input),
            theme::value_style(),
        );
    } else if app.inbox_overlay.prompt_mode == InboxPromptMode::ComposeConfirm {
        write_clipped(
            buf,
            frame.body_row + 1,
            frame.body_col,
            frame.body_width,
            &format!("Subject: {}", app.inbox_overlay.compose_subject),
            theme::label_style(),
        );
    }
}

pub fn recipient_rows(app: &DashApp) -> Vec<Vec<String>> {
    app.game_data
        .player
        .records
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx + 1 != app.player_record_index_1_based)
        .map(|(idx, player)| {
            let empire_id = idx + 1;
            let name = player.controlled_empire_name_summary();
            let fallback = player.legacy_status_name_summary();
            let display = if !name.is_empty() { name } else { fallback };
            vec![format!("{:>2}", empire_id), display]
        })
        .collect()
}

fn prompt_footer<'a>(app: &'a DashApp) -> TableFooter<'a> {
    match app.inbox_overlay.prompt_mode {
        InboxPromptMode::ComposeSubject => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Subject -> ",
            input: &app.inbox_overlay.prompt_input,
        },
        InboxPromptMode::ComposeConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Send message? Y/[N] ->",
        },
        _ => TableFooter::None,
    }
}

fn recipient_picker_rect(app: &DashApp) -> Rect {
    let rows = recipient_rows(app);
    let footer = TableFooter::None;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let visible_rows = recipient_visible_rows(parent, rows.len(), footer);
    let scrollable = rows.len() > visible_rows;
    let columns = resolve_table_columns_for_widget(
        &RECIPIENT_COLUMNS,
        &rows,
        80,
        scrollable,
        TableWidthMode::Compact,
        Some(TITLE),
        Some(footer),
    );
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        table_render_width(&columns) + usize::from(scrollable),
        standard_table_body_height(visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

fn prompt_overlay_rect(app: &DashApp) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        60,
        2,
        OverlaySizePolicy::default(),
        prompt_footer(app),
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

fn recipient_visible_rows(parent: Rect, row_count: usize, footer: TableFooter<'_>) -> usize {
    let max_popup_height = (parent.height as usize).saturating_sub(2).max(1);
    let max_body_height = max_popup_height.saturating_sub(overlay_chrome_height(footer));
    let max_table_rows = max_body_height.saturating_sub(4);
    row_count.min(max_table_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::state::{DashApp, InboxPromptMode};
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn recipient_picker_popup_reserves_full_table_body_height() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeRecipient;

        let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(&app).widgets);
        let popup = recipient_picker_rect(&app);
        let visible_rows =
            recipient_visible_rows(parent, recipient_rows(&app).len(), TableFooter::None);
        let body_height = popup.height as usize - overlay_chrome_height(TableFooter::None);

        assert!(body_height >= standard_table_body_height(visible_rows));
    }
}
