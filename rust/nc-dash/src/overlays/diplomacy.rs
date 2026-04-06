//! D overlay: centered diplomacy and leaderboard table.

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_hline, draw_overlay_frame_for_body, write_clipped};
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D D S I <Q> ->";
const HEADER: &str = "Rnk Empire             Planets Prod State      Relations";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let viewer_slot = app.player_record_index_1_based as u8;
    let viewer = app.game_data.player.records.get(player_idx);
    let mut rows = app
        .game_data
        .player
        .records
        .iter()
        .enumerate()
        .filter(|(_, player)| player.player_mode_raw() != 0x00)
        .map(|(idx, player)| {
            let slot = (idx + 1) as u8;
            let name = String::from_utf8_lossy(player.empire_name_bytes())
                .trim_end_matches('\0')
                .chars()
                .take(17)
                .collect::<String>();
            let state = if player.is_civil_disorder_player() {
                "Civil Dis"
            } else if slot == viewer_slot {
                "(you)"
            } else {
                "Stable"
            };
            let relation = if slot == viewer_slot {
                "—"
            } else if viewer
                .and_then(|viewer| viewer.diplomatic_relation_toward(slot))
                == Some(DiplomaticRelation::Enemy)
            {
                "Enemy"
            } else {
                "Neutral"
            };
            (
                slot,
                player.production_score_raw(),
                format!(
                    "{:<3} {:<17} {:>7} {:>4} {:<10} {}",
                    0,
                    name,
                    player.planet_count_raw(),
                    player.production_score_raw(),
                    state,
                    relation,
                ),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    let desired_visible_rows = rows.len().clamp(1, buf.height().saturating_sub(8));
    let body_width = rows
        .iter()
        .map(|(_, _, line)| line.chars().count())
        .max()
        .unwrap_or(0)
        .max(HEADER.chars().count());
    let frame = draw_overlay_frame_for_body(
        buf,
        "DIPLOMACY",
        body_width,
        desired_visible_rows + 2,
        FOOTER,
    );

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        HEADER,
        theme::section_title_style(),
    );
    draw_hline(buf, frame.body_row + 1, frame.body_col, frame.body_width, theme::border_style());

    let list_start = frame.body_row + 2;
    let max_rows = frame.body_height.saturating_sub(2);
    let selected = app
        .diplomacy_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.diplomacy_overlay.scroll, selected, max_rows, rows.len());

    for (visible_idx, (slot, _, line)) in rows.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let style = if absolute_idx == selected {
            theme::alert_style()
        } else if *slot == viewer_slot {
            theme::friendly_style()
        } else {
            theme::value_style()
        };
        if absolute_idx == selected {
            buf.fill_rect(row, frame.body_col, frame.body_width, 1, style);
        }
        let line = format!("{:<3} {}", absolute_idx + 1, &line[4..]);
        write_clipped(buf, row, frame.body_col, frame.body_width, &line, style);
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
