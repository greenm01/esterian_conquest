//! F overlay: fullscreen fleet + starbase management table.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::panels::fleets::order_abbrev;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let w = buf.width();
    let h = buf.height();
    let col = 2;

    buf.fill_row(0, theme::header_style());
    buf.write_text(0, col, "FLEET LIST", theme::title_style());
    buf.fill_row(h.saturating_sub(1), theme::footer_style());
    buf.write_text(
        h.saturating_sub(1),
        col,
        "COMMAND <- ? J K ^U ^D O C M T <Q> ->",
        theme::footer_style(),
    );

    let header_row = 1;
    buf.fill_row(header_row, theme::section_title_style());
    let hdr = format!(
        " {:<5} {:>8} {:>4} {:>4} {:>4} {:>4} {:>4} {:>5}  {}",
        "Fleet", "Coords", "Ord", "Spd", "ROE", "AR", "ETA", "Ships", ""
    );
    buf.write_text(header_row, col - 1, &hdr, theme::section_title_style());

    let sep_row = 2;
    for c in 0..w { buf.set_cell(sep_row, c, '─', theme::border_style()); }

    let owner_slot = app.player_record_index_1_based as u8;
    let mut row = sep_row + 1;
    let max_rows = h.saturating_sub(sep_row + 3);
    let mut shown = 0;

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        if shown < app.fleets_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows + app.fleets_scroll {
            break;
        }

        let num = fleet.local_slot_word_raw();
        let coords = fleet.current_location_coords_raw();
        let order = fleet.standing_order_kind();
        let abbrev = order_abbrev(order);
        let speed = fleet.current_speed();
        let roe = fleet.rules_of_engagement();
        let armies = fleet.army_count();
        let ships = fleet.ship_composition_summary();

        let line = format!(
            " #{:<4} ({:02},{:02})  {:<2}  {:>3}  {:>3}  {:>3}  {:>3}  {}",
            num, coords[0], coords[1], abbrev, speed, roe, armies, "—",
            &ships[..ships.len().min(w.saturating_sub(50))]
        );
        buf.write_text(row, 0, &line, theme::value_style());
        row += 1;
        shown += 1;
    }

    // Starbases.
    for base in &app.game_data.bases.records {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        if shown < app.fleets_scroll { shown += 1; continue; }
        if shown >= max_rows + app.fleets_scroll { break; }

        let coords = base.coords_raw();
        let line = format!(
            " SB {:<3} ({:02},{:02})  Gs   0    0    0    —",
            base.base_id_raw(), coords[0], coords[1]
        );
        buf.write_text(row, 0, &line, theme::friendly_style());
        row += 1;
        shown += 1;
    }

    if shown == 0 {
        buf.write_text(sep_row + 1, col, "(no fleets)", theme::dim_style());
    }
}
