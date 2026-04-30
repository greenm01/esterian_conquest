//! Right panel: compact diplomacy rows.

use crate::dashboard::app::state::DashApp;
use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer, StyledSpan};
use crate::dashboard::diplomacy_view::{display_name, relation_label_and_style};
use crate::dashboard::layout::widgets::WidgetRect;
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::theme;
use nc_data::EmpireProductionRankingSort;

pub(crate) const TITLE: &str = "DIPLOMACY";
pub(crate) const MIN_BODY_ROWS: usize = 4;
const PREFERRED_NAME_WIDTH: usize = 10;
const MIN_NAME_WIDTH: usize = 4;
const CP_WIDTH: usize = 5;

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
    let visible_row_count = frame.body.height;
    if visible_row_count == 0 {
        return;
    }

    let start = visible_row_start(&rows, app.diplomacy_scroll, visible_row_count);
    let visible_rows = rows.iter().skip(start).take(visible_row_count);
    let metrics = column_metrics(&rows, frame.body.width);

    let mut body_row = 0usize;
    for row_data in visible_rows {
        write_diplomacy_row(buf, frame.body, body_row, row_data, &metrics);
        body_row += 1;
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
    row_line_width(
        metrics.slot_width,
        PREFERRED_NAME_WIDTH,
        metrics.relation_width,
    )
}

#[cfg(test)]
pub(crate) fn minimum_body_width(app: &DashApp) -> usize {
    let rows = body_rows(app);
    let metrics = column_metrics(&rows, usize::MAX / 2);
    row_line_width(metrics.slot_width, MIN_NAME_WIDTH, metrics.relation_width)
}

#[derive(Clone, Copy, Debug)]
struct ColumnMetrics {
    slot_width: usize,
    name_width: usize,
    relation_width: usize,
}

fn column_metrics(rows: &[DiplomacyPanelRow], available_width: usize) -> ColumnMetrics {
    let slot_width = rows
        .iter()
        .map(|row| empire_slot_label(row.empire_slot).chars().count())
        .max()
        .unwrap_or(2)
        .max(2);
    let relation_width = rows
        .iter()
        .map(|row| row.relation.chars().count())
        .max()
        .unwrap_or("Neutral".chars().count());
    let available_name =
        available_width.saturating_sub(row_line_width(slot_width, 0, relation_width));
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
        relation_width,
    }
}

fn empire_slot_label(empire_slot: u8) -> String {
    format!("#{empire_slot}")
}

fn cp_label(production: u16) -> String {
    format!("{production:0CP_WIDTH$} CP")
}

fn row_line_width(slot_width: usize, name_width: usize, relation_width: usize) -> usize {
    slot_width + 1 + name_width + 1 + 2 + relation_width + 3 + (CP_WIDTH + 3) + 2
}

fn format_row_line(row: &DiplomacyPanelRow, metrics: &ColumnMetrics, body_width: usize) -> String {
    let slot = empire_slot_label(row.empire_slot);
    let available_name = body_width.saturating_sub(row_line_width(
        metrics.slot_width,
        0,
        metrics.relation_width,
    ));
    let name_width = metrics.name_width.min(available_name.max(MIN_NAME_WIDTH));
    let name = ellipsize(&row.name, name_width);
    format!(
        "{slot:<slot_width$} {name:<name_width$} [ {relation:<relation_width$} | {cp} ]",
        relation = row.relation,
        cp = cp_label(row.production),
        slot_width = metrics.slot_width,
        name_width = name_width,
        relation_width = metrics.relation_width,
    )
}

fn write_diplomacy_row(
    buf: &mut PlayfieldBuffer,
    body: WidgetRect,
    body_row_offset: usize,
    row: &DiplomacyPanelRow,
    metrics: &ColumnMetrics,
) {
    if body_row_offset >= body.height {
        return;
    }

    let text = format_row_line(row, metrics, body.width)
        .chars()
        .take(body.width)
        .collect::<String>();
    let empire_style = theme::empire_slot_style(row.empire_slot);
    let split = slot_name_end(&text);
    let spans = [
        StyledSpan::new(&text[..split], empire_style),
        StyledSpan::new(&text[split..], row.relation_style),
    ];
    buf.write_spans_clipped(body.row + body_row_offset, body.col, &spans);
}

fn slot_name_end(text: &str) -> usize {
    text.find(" [ ").unwrap_or(text.len())
}

fn visible_row_start(
    rows: &[DiplomacyPanelRow],
    requested_scroll: usize,
    visible_row_count: usize,
) -> usize {
    if visible_row_count == 0 {
        return 0;
    }
    requested_scroll.min(rows.len().saturating_sub(visible_row_count))
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
        DiplomacyPanelRow, MIN_BODY_ROWS, body_rows, column_metrics, cp_label, ellipsize,
        format_row_line, minimum_body_width, preferred_body_width, visible_row_start,
    };
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::layout::PanelWidgetFrame;
    use crate::dashboard::theme;
    use nc_data::GameStateBuilder;
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
    fn compact_row_uses_relation_and_zero_padded_cp() {
        let app = dash_app();
        let rows = body_rows(&app);
        let metrics = column_metrics(&rows, 37);
        let row = format_row_line(&rows[0], &metrics, 37);

        assert!(row.contains("[ Neutral | 00100 CP ]"), "{row}");
        assert!(row.starts_with("#"), "{row}");
    }

    #[test]
    fn compact_row_fits_enemy_relation_in_full_width_panel() {
        let rows = vec![
            DiplomacyPanelRow {
                empire_slot: 25,
                name: "The Glorious Empire".to_string(),
                production: 0,
                relation: "Enemy",
                relation_style: theme::enemy_style(),
            },
            DiplomacyPanelRow {
                empire_slot: 2,
                name: "Neutral Empire".to_string(),
                production: 100,
                relation: "Neutral",
                relation_style: theme::dim_style(),
            },
        ];
        let metrics = column_metrics(&rows, 37);
        let row = format_row_line(&rows[0], &metrics, 37);

        assert_eq!(row, "#25 The Glo... [ Enemy   | 00000 CP ]");
        assert_eq!(row.chars().count(), 37);
    }

    #[test]
    fn cp_label_uses_fixed_five_digit_field() {
        assert_eq!(cp_label(0), "00000 CP");
        assert_eq!(cp_label(100), "00100 CP");
    }

    #[test]
    fn visible_scroll_is_clamped_to_last_complete_row() {
        let app = dash_app();
        let rows = body_rows(&app);

        assert_eq!(visible_row_start(&rows, usize::MAX, 2), 1);
    }

    #[test]
    fn narrow_panel_render_does_not_panic() {
        let app = dash_app();
        let frame = PanelWidgetFrame {
            outer: crate::dashboard::layout::widgets::WidgetRect {
                col: 1,
                row: 1,
                width: 18,
                height: 6,
            },
            title_row: 1,
            body: crate::dashboard::layout::widgets::WidgetRect {
                col: 2,
                row: 2,
                width: 17,
                height: 5,
            },
        };
        let mut buffer = PlayfieldBuffer::new(24, 10, theme::body_style());

        super::draw(&mut buffer, &app, frame);

        assert!(buffer.plain_line(frame.body.row).contains("#"));
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
