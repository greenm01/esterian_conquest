//! F overlay: dashboard-sized fleet and starbase command table.

use nc_ui::PlayfieldBuffer;
use nc_ui::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, write_table_window_with_theme_at,
};

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_overlay_frame_for_body, write_clipped};
use crate::panels::fleets::order_abbrev;
use crate::theme;

pub(crate) const HOTKEYS: &str = "? J K ^U ^D O C M T I <Q>";
const COLUMNS: [TableColumn<'static>; 9] = [
    TableColumn::right("ID", 4),
    TableColumn::left("Location", 8),
    TableColumn::left("Order", 5),
    TableColumn::left("Target", 8),
    TableColumn::right("Spd", 3),
    TableColumn::right("ETA", 4),
    TableColumn::right("ROE", 3),
    TableColumn::right("AR", 3),
    TableColumn::left_flex("Ships / Forces", 24, 1),
];

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let rows = table_rows(app);
    let selected = app.fleet_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .and_then(|row| row.first())
        .map(String::as_str);
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default,
        input: &app.fleet_overlay.jump_input,
    };

    let desired_visible_rows = rows.len().clamp(1, buf.height().saturating_sub(10));
    let columns = resolve_table_columns(
        &COLUMNS,
        &rows,
        buf.width().saturating_sub(12),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You have no active fleets or starbases.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body(
        buf,
        "FLEET LIST",
        body_width,
        desired_visible_rows + 4,
        footer,
    );
    let visible_rows = frame.body_height.saturating_sub(4);
    let scroll = clamp_scroll(app.fleet_overlay.scroll, selected, visible_rows, rows.len());
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &columns,
        &rows,
        scroll,
        visible_rows,
        theme::table_theme(),
        rows.get(selected).map(|_| selected),
        0,
        None,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            "You have no active fleets or starbases.",
            theme::dim_style(),
        );
    }
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .filter_map(|row| row.first().cloned().map(|cell| vec![cell]))
        .collect()
}

fn table_rows(app: &DashApp) -> Vec<Vec<String>> {
    let owner_slot = app.player_record_index_1_based as u8;
    let mut rows = Vec::new();

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        rows.push(vec![
            fleet.local_slot_word_raw().to_string(),
            format_coords(fleet.current_location_coords_raw()),
            order_abbrev(fleet.standing_order_kind()).to_string(),
            format_target(fleet.standing_order_target_coords_raw()),
            fleet.current_speed().to_string(),
            String::from("--"),
            fleet.rules_of_engagement().to_string(),
            fleet.army_count().to_string(),
            truncate(&fleet.ship_composition_summary(), COLUMNS[8].width),
        ]);
    }

    for base in &app.game_data.bases.records {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        rows.push(vec![
            format!("SB{}", base.base_id_raw()),
            format_coords(base.coords_raw()),
            String::from("Gs"),
            String::from("--"),
            String::from("0"),
            String::from("--"),
            String::from("0"),
            String::from("0"),
            String::from("Starbase"),
        ]);
    }

    rows
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

fn format_coords(coords: [u8; 2]) -> String {
    format!("({:02},{:02})", coords[0], coords[1])
}

fn format_target(coords: [u8; 2]) -> String {
    if coords[0] == 0 || coords[1] == 0 {
        String::from("--")
    } else {
        format_coords(coords)
    }
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}
