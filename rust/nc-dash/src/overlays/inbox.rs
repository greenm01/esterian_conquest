//! R overlay: centered split-pane inbox for reports and messages.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::Rect;
use nc_ui::table::{TableFooter, draw_scrollbar_at};
use nc_ui::theme::classic;

use crate::app::state::{ActiveOverlay, DashApp, InboxFocus};
use crate::inbox::{DashInboxItem, matches_filter, project_inbox_items};
use crate::layout::MapWidgetFrame;
use crate::layout::dashboard;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect, draw_hline,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, draw_vline,
    max_overlay_body_width, overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? M R A Y D <Q>";
const LIST_WIDTH: usize = 28;
const SPLIT_GAP_WIDTH: usize = 2;
const TARGET_PREVIEW_WIDTH: usize = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InboxPaneLayout {
    list_width: usize,
    divider_offset: usize,
    preview_offset: usize,
    preview_width: usize,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let items = inbox_items(app);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let selected_default = items.get(selected).map(|_| format!("{:02}", selected + 1));
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.inbox_overlay.jump_input,
    };
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
    let target_body_width = filter_line
        .chars()
        .count()
        .max(LIST_WIDTH + SPLIT_GAP_WIDTH + TARGET_PREVIEW_WIDTH);
    let body_width = target_body_width.min(max_overlay_body_width(map_frame));
    let natural_content_rows = items.len().max(1).max(
        items
            .get(selected)
            .map(|item| item.body_lines.len().max(1))
            .unwrap_or(1),
    );
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "INBOX",
        body_width,
        4 + natural_content_rows,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    let max_rows = frame.body_height.saturating_sub(4);
    assert_overlay_body_write_fits(frame, "INBOX", frame.body_width, 4 + max_rows);
    let pane_layout = inbox_pane_layout(frame.body_width);
    let divider_col = frame.body_col + pane_layout.divider_offset;
    let preview_col = frame.body_col + pane_layout.preview_offset;
    let preview_width = pane_layout.preview_width;
    let table_theme = theme::table_theme();
    let list_focus = matches!(app.inbox_overlay.focus, InboxFocus::List);
    let divider_style = if list_focus {
        classic::notice_style()
    } else {
        theme::border_style()
    };
    let list_header_style = if list_focus {
        classic::notice_style()
    } else {
        table_theme.header_style
    };

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
        divider_style,
    );
    buf.set_cell(frame.body_row + 1, divider_col, '┬', divider_style);
    buf.set_cell(frame.footer_row - 1, divider_col, '┴', divider_style);

    write_clipped(
        buf,
        frame.body_row + 2,
        frame.body_col,
        pane_layout.list_width.saturating_sub(1),
        "ID  Type Year Subject",
        list_header_style,
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
    buf.set_cell(frame.body_row + 3, divider_col, '┼', divider_style);

    let list_start = frame.body_row + 4;
    let scroll = clamp_scroll(app.inbox_overlay.scroll, selected, max_rows, items.len());

    for (visible_idx, item) in items.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let is_selected = absolute_idx == selected;
        let list_style = table_theme.body_style;
        let line = format!(
            "{:>2}  {}   {:>4} {}",
            absolute_idx + 1,
            item.item_type.code(),
            item.year.to_string(),
            truncate(&item.subject, pane_layout.list_width.saturating_sub(14)),
        );
        write_clipped(
            buf,
            row,
            frame.body_col,
            pane_layout.list_width.saturating_sub(1),
            &line,
            list_style,
        );
        if is_selected {
            highlight_selected_id_cell(
                buf,
                row,
                frame.body_col,
                absolute_idx + 1,
                table_theme.selected_style,
            );
        }
    }

    if items.is_empty() {
        write_clipped(
            buf,
            list_start,
            frame.body_col,
            pane_layout.list_width.saturating_sub(1),
            "(empty)",
            theme::dim_style(),
        );
    }

    draw_scrollbar_at(
        buf,
        list_start,
        frame.body_col + pane_layout.list_width.saturating_sub(1),
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

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let items = inbox_items(app);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let selected_default = items.get(selected).map(|_| format!("{:02}", selected + 1));
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.inbox_overlay.jump_input,
    };
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
    let target_body_width = filter_line
        .chars()
        .count()
        .max(LIST_WIDTH + SPLIT_GAP_WIDTH + TARGET_PREVIEW_WIDTH);
    let body_width = target_body_width.min(max_overlay_body_width(map_frame));
    let natural_content_rows = items.len().max(1).max(
        items
            .get(selected)
            .map(|item| item.body_lines.len().max(1))
            .unwrap_or(1),
    );
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "INBOX",
        body_width,
        4 + natural_content_rows,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
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
        matches_filter(
            item,
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

fn inbox_pane_layout(body_width: usize) -> InboxPaneLayout {
    let list_width = LIST_WIDTH.min(body_width.saturating_sub(SPLIT_GAP_WIDTH + 1));
    let divider_offset = list_width;
    let preview_offset = divider_offset + SPLIT_GAP_WIDTH;
    let preview_width = body_width.saturating_sub(preview_offset);
    InboxPaneLayout {
        list_width,
        divider_offset,
        preview_offset,
        preview_width,
    }
}

#[cfg(test)]
fn target_inbox_body_width(filter_line_width: usize, max_body_width: usize) -> usize {
    filter_line_width
        .max(LIST_WIDTH + SPLIT_GAP_WIDTH + TARGET_PREVIEW_WIDTH)
        .min(max_body_width)
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn highlight_selected_id_cell(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    visible_id: usize,
    style: nc_ui::CellStyle,
) {
    buf.write_text(row, col, &format!("{visible_id:>2}"), style);
}

#[cfg(test)]
mod tests {
    use super::{HOTKEYS, inbox_pane_layout, target_inbox_body_width};
    use crate::app::render;
    use crate::app::state::{ActiveOverlay, DashApp};
    use nc_data::{GameStateBuilder, QueuedPlayerMail, ReportBlockRow};
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_inbox_commands() {
        assert_eq!(HOTKEYS, "? M R A Y D <Q>");
    }

    #[test]
    fn inbox_pane_layout_keeps_default_list_width_when_space_allows() {
        let layout = inbox_pane_layout(103);

        assert_eq!(layout.list_width, 28);
        assert_eq!(layout.divider_offset, 28);
        assert_eq!(layout.preview_offset, 30);
        assert_eq!(layout.preview_width, 73);
    }

    #[test]
    fn inbox_pane_layout_shrinks_list_to_preserve_preview_space() {
        let layout = inbox_pane_layout(20);

        assert_eq!(layout.list_width, 17);
        assert_eq!(layout.divider_offset, 17);
        assert_eq!(layout.preview_offset, 19);
        assert_eq!(layout.preview_width, 1);
    }

    #[test]
    fn inbox_target_body_width_prefers_fixed_preview_width() {
        assert_eq!(target_inbox_body_width(40, 103), 102);
        assert_eq!(target_inbox_body_width(120, 103), 103);
    }

    #[test]
    fn inbox_overlay_clamps_to_map_body_width_without_panicking() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(25)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            vec![ReportBlockRow {
                viewer_empire_id: 0,
                block_index: 0,
                decoded_text: "Stardate: 03/3012\nThis is an intentionally very long inbox preview line that should exceed the modal natural width on an X13-sized terminal but still render safely inside the centered split-pane popup.".to_string(),
                raw_bytes: None,
                recipient_deleted: false,
            }],
            vec![QueuedPlayerMail {
                sender_empire_id: 2,
                recipient_empire_id: 1,
                year: 3012,
                subject: "Long diplomatic subject that also stretches the inbox list width".to_string(),
                body: "Second very long preview line to keep the selected preview pane wide enough to trigger clamping instead of a panic.".to_string(),
                recipient_deleted: false,
            }],
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.overlay = ActiveOverlay::Inbox;

        let buffer = render::render(&app).expect("render inbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(lines.iter().any(|line| line.contains("INBOX")));
        assert!(lines.iter().any(|line| line.contains("Preview")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("ID  Type Year Subject"))
        );
    }

    #[test]
    fn inbox_modal_width_stays_stable_across_selected_messages() {
        let app = inbox_test_app();

        let mut short_selected = app;
        short_selected.overlay = ActiveOverlay::Inbox;
        short_selected.inbox_overlay.selected = 0;

        let mut long_selected = inbox_test_app();
        long_selected.overlay = ActiveOverlay::Inbox;
        long_selected.inbox_overlay.selected = 1;

        let short_buffer = render::render(&short_selected).expect("render short inbox");
        let long_buffer = render::render(&long_selected).expect("render long inbox");

        let short_title_row = find_line(&short_buffer, "INBOX");
        let long_title_row = find_line(&long_buffer, "INBOX");
        assert_eq!(
            short_buffer.plain_line(short_title_row),
            long_buffer.plain_line(long_title_row)
        );
    }

    fn inbox_test_app() -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(25)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            vec![
                ReportBlockRow {
                    viewer_empire_id: 0,
                    block_index: 0,
                    decoded_text: "Stardate: 03/3012\nShort line.".to_string(),
                    raw_bytes: None,
                    recipient_deleted: false,
                },
                ReportBlockRow {
                    viewer_empire_id: 0,
                    block_index: 1,
                    decoded_text: "Stardate: 04/3012\nThis is an intentionally very long inbox preview line that should exceed the fixed preview width target but must not change the centered modal width when selected.".to_string(),
                    raw_bytes: None,
                    recipient_deleted: false,
                },
            ],
            vec![QueuedPlayerMail {
                sender_empire_id: 2,
                recipient_empire_id: 1,
                year: 3012,
                subject: "Long diplomatic subject that also stretches the inbox list width".to_string(),
                body: "Second very long preview line to keep the selected preview pane wide enough to trigger clipping without resizing.".to_string(),
                recipient_deleted: false,
            }],
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn find_line(buffer: &nc_ui::PlayfieldBuffer, needle: &str) -> usize {
        (0..buffer.height())
            .find(|row| buffer.plain_line(*row).contains(needle))
            .expect("line containing needle")
    }
}
