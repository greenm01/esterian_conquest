//! R overlay: centered split-pane inbox for reports and messages.

use nc_ui::PlayfieldBuffer;
use nc_ui::table::{TableFooter, draw_scrollbar_at};

use crate::app::state::{DashApp, InboxFocus};
use crate::inbox::{DashInboxItem, project_inbox_items};
use crate::overlays::frame::{draw_hline, draw_overlay_frame, draw_vline, write_clipped};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? Tab J K ^U ^D M R A Y D C <Q>";
const LIST_WIDTH: usize = 28;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let selected_default = selected_default(app);
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.inbox_overlay.jump_input,
    };
    let preferred_width = buf.width().saturating_sub(10).clamp(104, 142);
    let preferred_height = buf.height().saturating_sub(6).clamp(20, 30);
    let frame = draw_overlay_frame(buf, "INBOX", preferred_width, preferred_height, footer);
    let divider_col = frame.body_col + LIST_WIDTH;
    let preview_col = divider_col + 2;
    let preview_width = frame.body_width.saturating_sub(LIST_WIDTH + 2);
    let items = inbox_items(app);

    let filter_line = format!(
        "Filter:{}  Year:{}  Focus:{}{}",
        app.inbox_overlay.filter.label(),
        if app.inbox_overlay.current_year_only {
            "Current"
        } else {
            "All"
        },
        match app.inbox_overlay.focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        },
        if app.inbox_overlay.delete_confirm {
            "  Delete this item? Y/[N]"
        } else {
            ""
        }
    );
    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        &filter_line,
        theme::section_title_style(),
    );
    draw_hline(
        buf,
        frame.body_row + 1,
        frame.body_col,
        frame.body_width,
        theme::border_style(),
    );
    draw_vline(
        buf,
        frame.body_row + 2,
        divider_col,
        frame.body_height.saturating_sub(2),
        theme::border_style(),
    );
    buf.set_cell(frame.body_row + 1, divider_col, '┬', theme::border_style());
    buf.set_cell(
        frame.footer_row - 1,
        divider_col,
        '┴',
        theme::border_style(),
    );

    write_clipped(
        buf,
        frame.body_row + 2,
        frame.body_col,
        LIST_WIDTH.saturating_sub(1),
        "ID  Type Year Subject",
        theme::section_title_style(),
    );
    write_clipped(
        buf,
        frame.body_row + 2,
        preview_col,
        preview_width.saturating_sub(1),
        "Preview",
        theme::section_title_style(),
    );
    draw_hline(
        buf,
        frame.body_row + 3,
        frame.body_col,
        frame.body_width,
        theme::border_style(),
    );
    buf.set_cell(frame.body_row + 3, divider_col, '┼', theme::border_style());

    let list_start = frame.body_row + 4;
    let max_rows = frame.body_height.saturating_sub(4);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let scroll = clamp_scroll(app.inbox_overlay.scroll, selected, max_rows, items.len());

    for (visible_idx, item) in items.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let is_selected = absolute_idx == selected;
        let list_style = if is_selected && matches!(app.inbox_overlay.focus, InboxFocus::List) {
            theme::alert_style()
        } else if is_selected {
            theme::value_style()
        } else {
            theme::dim_style()
        };
        if is_selected {
            buf.fill_rect(
                row,
                frame.body_col,
                LIST_WIDTH.saturating_sub(1),
                1,
                list_style,
            );
        }
        let line = format!(
            "{:>2}  {}   {:>4} {}",
            absolute_idx + 1,
            item.item_type.code(),
            item.year.to_string(),
            truncate(&item.subject, LIST_WIDTH.saturating_sub(14)),
        );
        write_clipped(
            buf,
            row,
            frame.body_col,
            LIST_WIDTH.saturating_sub(1),
            &line,
            list_style,
        );
    }

    if items.is_empty() {
        write_clipped(
            buf,
            list_start,
            frame.body_col,
            LIST_WIDTH.saturating_sub(1),
            "(empty)",
            theme::dim_style(),
        );
    }

    draw_scrollbar_at(
        buf,
        list_start,
        frame.body_col + LIST_WIDTH.saturating_sub(1),
        max_rows,
        items.len(),
        scroll,
        theme::table_theme(),
    );

    if let Some(item) = items.get(selected) {
        let preview_style = if matches!(app.inbox_overlay.focus, InboxFocus::Preview) {
            theme::value_style()
        } else {
            theme::label_style()
        };
        let preview_scroll = app
            .inbox_overlay
            .preview_scroll
            .min(item.body_lines.len().saturating_sub(max_rows.max(1)));
        for (visible_idx, line) in item
            .body_lines
            .iter()
            .skip(preview_scroll)
            .take(max_rows)
            .enumerate()
        {
            write_clipped(
                buf,
                list_start + visible_idx,
                preview_col,
                preview_width.saturating_sub(1),
                line,
                preview_style,
            );
        }
    }
}

pub(crate) fn inbox_items(app: &DashApp) -> Vec<DashInboxItem> {
    let viewer = app.player_record_index_1_based as u8;
    let current_year = app.game_data.conquest.game_year();
    project_inbox_items(
        &app.game_data,
        viewer,
        &app.report_block_rows,
        &app.queued_mail,
    )
    .into_iter()
    .filter(|item| {
        item.matches_filter(
            app.inbox_overlay.filter,
            app.inbox_overlay.current_year_only,
            current_year,
        )
    })
    .collect()
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    inbox_items(app)
        .into_iter()
        .enumerate()
        .map(|(idx, _)| vec![format!("{:02}", idx + 1)])
        .collect()
}

fn selected_default(app: &DashApp) -> Option<String> {
    let items = inbox_items(app);
    let selected = app.inbox_overlay.selected.min(items.len().saturating_sub(1));
    items.get(selected).map(|_| format!("{:02}", selected + 1))
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}
