//! F overlay: dashboard-sized fleet and starbase command table.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_hline, draw_overlay_frame, write_clipped};
use crate::panels::fleets::order_abbrev;
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D O C M T I <Q> ->";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let preferred_width = buf.width().saturating_sub(12).clamp(96, 138);
    let preferred_height = buf.height().saturating_sub(6).clamp(18, 28);
    let frame = draw_overlay_frame(buf, "FLEET LIST", preferred_width, preferred_height, FOOTER);

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        "Fleet Coords  Ord Spd ROE AR ETA  Ships / Forces",
        theme::section_title_style(),
    );
    draw_hline(buf, frame.body_row + 1, frame.body_col, frame.body_width, theme::border_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let mut rows = Vec::new();

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        rows.push((
            false,
            format!(
                "#{:<3} ({:02},{:02}) {:>2} {:>3} {:>3} {:>2} {:>3}  {}",
                fleet.local_slot_word_raw(),
                fleet.current_location_coords_raw()[0],
                fleet.current_location_coords_raw()[1],
                order_abbrev(fleet.standing_order_kind()),
                fleet.current_speed(),
                fleet.rules_of_engagement(),
                fleet.army_count(),
                "—",
                truncate(&fleet.ship_composition_summary(), frame.body_width.saturating_sub(35)),
            ),
        ));
    }

    for base in &app.game_data.bases.records {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        rows.push((
            true,
            format!(
                "SB{:<2} ({:02},{:02}) Gs   0   0  0  —    Starbase",
                base.base_id_raw(),
                base.coords_raw()[0],
                base.coords_raw()[1],
            ),
        ));
    }

    let list_start = frame.body_row + 2;
    let max_rows = frame.body_height.saturating_sub(2);
    let selected = app
        .fleet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.fleet_overlay.scroll, selected, max_rows, rows.len());

    for (visible_idx, (is_base, line)) in rows.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let style = if absolute_idx == selected {
            theme::alert_style()
        } else if *is_base {
            theme::friendly_style()
        } else {
            theme::value_style()
        };
        if absolute_idx == selected {
            buf.fill_rect(row, frame.body_col, frame.body_width, 1, style);
        }
        write_clipped(buf, row, frame.body_col, frame.body_width, line, style);
    }

    if rows.is_empty() {
        write_clipped(
            buf,
            list_start,
            frame.body_col,
            frame.body_width,
            "You have no active fleets or starbases.",
            theme::dim_style(),
        );
    }
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
