//! Right panel: compact 2-line diplomacy blocks.

use crate::app::state::DashApp;
use crate::diplomacy_view::{display_name, relation_label_and_style};
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;
use nc_data::EmpireProductionRankingSort;
use nc_ui::{CellStyle, PlayfieldBuffer};

pub(crate) const TITLE: &str = "DIPLOMACY";
pub(crate) const MIN_BODY_ROWS: usize = 4;
const PREFERRED_NAME_WIDTH: usize = 10;
const MIN_NAME_WIDTH: usize = 4;

#[derive(Debug, Clone)]
pub(crate) struct DiplomacyPanelRow {
    pub empire_slot: u8,
    pub name: String,
    pub production: u16,
    pub relation: &'static str,
    pub relation_style: CellStyle,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    let rows = body_rows(app);
    let visible_block_count = frame.body.height / 2;
    if visible_block_count == 0 {
        return;
    }

    let start = visible_block_start(&rows, app.diplomacy_scroll, visible_block_count);
    let visible_rows = rows.iter().skip(start).take(visible_block_count);
    let metrics = column_metrics(&rows, frame.body.width);

    let mut body_row = 0usize;
    for row_data in visible_rows {
        let top_line = format_top_line(row_data, &metrics, frame.body.width);
        let bottom_line = format_bottom_line(row_data, &metrics);
        let top_style = theme::empire_slot_style(row_data.empire_slot);

        layout::write_panel_body_line(buf, frame, body_row, &top_line, top_style);
        layout::write_panel_body_line(
            buf,
            frame,
            body_row + 1,
            &bottom_line,
            row_data.relation_style,
        );
        body_row += 2;
    }

    if body_row == 0 {
        layout::write_panel_body_line(buf, frame, 0, " (none)", theme::dim_style());
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<DiplomacyPanelRow> {
    let viewer_idx = app.player_record_index_1_based;
    let Some(viewer) = app
        .game_data
        .player
        .records
        .get(viewer_idx.saturating_sub(1))
    else {
        return Vec::new();
    };
    let viewer_slot = viewer_idx as u8;
    let rankings = app
        .game_data
        .empire_production_ranking_rows(EmpireProductionRankingSort::Production);

    rankings
        .into_iter()
        .filter_map(|ranking| {
            let empire_slot = ranking.empire_id;
            if empire_slot == viewer_slot {
                return None;
            }
            let player = app
                .game_data
                .player
                .records
                .get(empire_slot.saturating_sub(1) as usize)?;
            let (relation, relation_style) =
                relation_label_and_style(Some(viewer), viewer_slot, empire_slot);
            Some(DiplomacyPanelRow {
                empire_slot,
                name: display_name(player, empire_slot),
                production: ranking.current_production,
                relation,
                relation_style,
            })
        })
        .collect()
}

pub(crate) fn preferred_body_width(app: &DashApp) -> usize {
    let rows = body_rows(app);
    let metrics = column_metrics(&rows, usize::MAX / 2);
    top_line_width(metrics.slot_width, PREFERRED_NAME_WIDTH).max(bottom_line_width(
        metrics.slot_width,
        metrics.cp_width,
        metrics.relation_width,
    ))
}

pub(crate) fn minimum_body_width(app: &DashApp) -> usize {
    let rows = body_rows(app);
    let metrics = column_metrics(&rows, usize::MAX / 2);
    top_line_width(metrics.slot_width, MIN_NAME_WIDTH).max(bottom_line_width(
        metrics.slot_width,
        metrics.cp_width,
        metrics.relation_width,
    ))
}

#[derive(Clone, Copy, Debug)]
struct ColumnMetrics {
    slot_width: usize,
    name_width: usize,
    cp_width: usize,
    relation_width: usize,
}

fn column_metrics(rows: &[DiplomacyPanelRow], available_width: usize) -> ColumnMetrics {
    let slot_width = rows
        .iter()
        .map(|row| empire_slot_label(row.empire_slot).chars().count())
        .max()
        .unwrap_or(2)
        .max(2);
    let cp_width = rows
        .iter()
        .map(|row| cp_label(row.production).chars().count())
        .max()
        .unwrap_or("CP 0".chars().count());
    let relation_width = rows
        .iter()
        .map(|row| row.relation.chars().count())
        .max()
        .unwrap_or("Neutral".chars().count());
    let available_name = available_width.saturating_sub(top_line_width(slot_width, 0));
    let preferred_name = rows
        .iter()
        .map(|row| row.name.chars().count().min(PREFERRED_NAME_WIDTH))
        .max()
        .unwrap_or(MIN_NAME_WIDTH)
        .max(MIN_NAME_WIDTH);
    let name_width = preferred_name.min(available_name.max(MIN_NAME_WIDTH));

    ColumnMetrics {
        slot_width,
        name_width,
        cp_width,
        relation_width,
    }
}

fn empire_slot_label(empire_slot: u8) -> String {
    format!("#{empire_slot}")
}

fn cp_label(production: u16) -> String {
    format!("CP {production}")
}

fn top_line_width(slot_width: usize, name_width: usize) -> usize {
    slot_width + 3 + name_width
}

fn bottom_line_width(slot_width: usize, cp_width: usize, relation_width: usize) -> usize {
    slot_width + 3 + cp_width + 3 + relation_width
}

fn format_top_line(row: &DiplomacyPanelRow, metrics: &ColumnMetrics, body_width: usize) -> String {
    let slot = empire_slot_label(row.empire_slot);
    let available_name = body_width.saturating_sub(top_line_width(metrics.slot_width, 0));
    let name_width = metrics.name_width.min(available_name.max(MIN_NAME_WIDTH));
    let name = ellipsize(&row.name, name_width);
    format!("{slot:<width$} | {name}", width = metrics.slot_width)
}

fn format_bottom_line(row: &DiplomacyPanelRow, metrics: &ColumnMetrics) -> String {
    let slot = empire_slot_label(row.empire_slot);
    let cp = cp_label(row.production);
    format!(
        "{slot:<slot_width$} | {cp:<cp_width$} | {}",
        row.relation,
        slot_width = metrics.slot_width,
        cp_width = metrics.cp_width,
    )
}

fn visible_block_start(
    rows: &[DiplomacyPanelRow],
    requested_scroll: usize,
    visible_block_count: usize,
) -> usize {
    if visible_block_count == 0 {
        return 0;
    }
    requested_scroll.min(rows.len().saturating_sub(visible_block_count))
}

fn ellipsize(text: &str, width: usize) -> String {
    let text_width = text.chars().count();
    if text_width <= width {
        return text.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let keep = width.saturating_sub(3);
    let prefix: String = text.chars().take(keep).collect();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::{
        MIN_BODY_ROWS, body_rows, column_metrics, ellipsize, format_bottom_line, format_top_line,
        minimum_body_width, preferred_body_width, visible_block_start,
    };
    use crate::app::state::DashApp;
    use crate::layout::PanelWidgetFrame;
    use crate::theme;
    use nc_data::GameStateBuilder;
    use nc_ui::{PlayfieldBuffer, ScreenGeometry};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn diplomacy_rows_exclude_self_and_keep_production_sort_order() {
        let app = dash_app();
        let rows = body_rows(&app);

        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.empire_slot != 1));
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].production >= pair[1].production)
        );
    }

    #[test]
    fn long_names_ellipsize_with_three_dots() {
        assert_eq!(ellipsize("Foobarbaz", 6), "Foo...");
    }

    #[test]
    fn preferred_width_stays_above_minimum() {
        let app = dash_app();
        assert!(preferred_body_width(&app) >= minimum_body_width(&app));
        assert_eq!(MIN_BODY_ROWS, 4);
    }

    #[test]
    fn top_and_bottom_lines_keep_first_separator_aligned() {
        let app = dash_app();
        let rows = body_rows(&app);
        let metrics = column_metrics(&rows, 18);
        let top = format_top_line(&rows[0], &metrics, 18);
        let bottom = format_bottom_line(&rows[0], &metrics);

        assert_eq!(top.find('|'), bottom.find('|'));
        assert!(bottom.contains("| CP "));
    }

    #[test]
    fn visible_scroll_is_clamped_to_last_complete_block() {
        let app = dash_app();
        let rows = body_rows(&app);

        assert_eq!(visible_block_start(&rows, usize::MAX, 2), 1);
    }

    #[test]
    fn narrow_panel_render_does_not_panic() {
        let app = dash_app();
        let frame = PanelWidgetFrame {
            outer: crate::layout::widgets::WidgetRect {
                col: 1,
                row: 1,
                width: 18,
                height: 6,
            },
            title_row: 1,
            body: crate::layout::widgets::WidgetRect {
                col: 2,
                row: 2,
                width: 17,
                height: 5,
            },
        };
        let mut buffer = PlayfieldBuffer::new(24, 10, theme::body_style());

        super::draw(&mut buffer, &app, frame);

        assert!(buffer.plain_line(frame.body.row).contains("|"));
    }

    fn dash_app() -> DashApp {
        DashApp::new_for_tests(
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        )
    }
}
