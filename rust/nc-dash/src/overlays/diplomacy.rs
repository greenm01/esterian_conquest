//! D overlay: centered diplomacy and leaderboard table.

use nc_ui::PlayfieldBuffer;
use nc_ui::table::{TableFooter, draw_scrollbar_at};

use crate::app::state::DashApp;
use crate::diplomacy_view::{
    display_name, empire_name_style, relation_label_and_style, state_label_and_style,
};
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    draw_hline, draw_overlay_frame_for_body_in_map, max_overlay_body_height, write_clipped,
};
use crate::theme;

const HOTKEYS: &str = "? D S I <Q>";
const HEADER: &str = "Rnk Empire             Planets Prod State      Relations";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let viewer_slot = app.player_record_index_1_based as u8;
    let viewer = app.game_data.player.records.get(player_idx);
    let mut rows = app
        .game_data
        .player
        .records
        .iter()
        .enumerate()
        .map(|(idx, player)| {
            let slot = (idx + 1) as u8;
            let name = display_name(player, slot)
                .chars()
                .take(17)
                .collect::<String>();
            let (state, state_style) = state_label_and_style(player, viewer_slot, slot);
            let (relation, relation_style) = relation_label_and_style(viewer, viewer_slot, slot);
            DiplomacyRow {
                slot,
                name,
                planets: player.planet_count_raw(),
                production: player.production_score_raw(),
                state: state.to_string(),
                state_style,
                relation: relation.to_string(),
                relation_style,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.production.cmp(&a.production));
    let desired_visible_rows = rows.len().clamp(
        1,
        max_overlay_body_height(map_frame).saturating_sub(2).max(1),
    );
    let body_width = HEADER.chars().count() + 1;
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: None,
        input: "",
    };
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "DIPLOMACY",
        body_width,
        desired_visible_rows + 2,
        footer,
    );

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        HEADER,
        theme::section_title_style(),
    );
    draw_hline(
        buf,
        frame.body_row + 1,
        frame.body_col,
        frame.body_width,
        theme::border_style(),
    );

    let list_start = frame.body_row + 2;
    let max_rows = frame.body_height.saturating_sub(2);
    let selected = app
        .diplomacy_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.diplomacy_overlay.scroll, selected, max_rows, rows.len());

    for (visible_idx, row_data) in rows.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let row_style = if absolute_idx == selected {
            theme::alert_style()
        } else if row_data.slot == viewer_slot {
            theme::friendly_style()
        } else {
            theme::value_style()
        };
        if absolute_idx == selected {
            buf.fill_rect(row, frame.body_col, frame.body_width, 1, row_style);
        }
        write_diplomacy_row(
            buf,
            row,
            frame.body_col,
            frame.body_width.saturating_sub(1),
            absolute_idx + 1,
            row_data,
            row_style,
        );
    }

    draw_scrollbar_at(
        buf,
        list_start,
        frame.body_col + frame.body_width.saturating_sub(1),
        max_rows,
        rows.len(),
        scroll,
        theme::table_theme(),
    );
}

#[derive(Debug, Clone)]
struct DiplomacyRow {
    slot: u8,
    name: String,
    planets: u8,
    production: u16,
    state: String,
    state_style: nc_ui::CellStyle,
    relation: String,
    relation_style: nc_ui::CellStyle,
}

fn write_diplomacy_row(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    rank: usize,
    row_data: &DiplomacyRow,
    row_style: nc_ui::CellStyle,
) {
    write_clipped(buf, row, col, width, &format!("{rank:<3}"), row_style);
    write_clipped(
        buf,
        row,
        col + 4,
        17,
        &format!("{:<17}", row_data.name),
        empire_name_style(row_data.slot, row_style.bg, row_style.bold),
    );
    write_clipped(
        buf,
        row,
        col + 22,
        7,
        &format!("{:>7}", row_data.planets),
        row_style,
    );
    write_clipped(
        buf,
        row,
        col + 30,
        4,
        &format!("{:>4}", row_data.production),
        row_style,
    );
    write_clipped(
        buf,
        row,
        col + 35,
        10,
        &format!("{:<10}", row_data.state),
        nc_ui::CellStyle::new(row_data.state_style.fg, row_style.bg, row_style.bold),
    );
    write_clipped(
        buf,
        row,
        col + 46,
        width.saturating_sub(46),
        &row_data.relation,
        nc_ui::CellStyle::new(row_data.relation_style.fg, row_style.bg, row_style.bold),
    );
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
