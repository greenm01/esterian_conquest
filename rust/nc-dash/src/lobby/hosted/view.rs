use nc_ui::modal::Rect;
use nc_ui::theme::classic;
use nc_ui::PlayfieldBuffer;

use crate::lobby::state::HostedGameView;

pub fn render(buffer: &mut PlayfieldBuffer, rect: Rect, hosted: &HostedGameView) {
    let state = &hosted.snapshot.state;
    let player = &state.player;
    let worlds = &state.starmap.worlds;
    let planets = &state.owned_planets;
    let fleets = &state.owned_fleets;

    buffer.write_text_clipped(
        rect.y as usize,
        rect.x as usize + 2,
        &format!(
            "HOSTED GAME | {} | Y{} T{} | {}",
            hosted.row.game, hosted.snapshot.year, hosted.snapshot.turn, hosted.snapshot.player_name
        ),
        classic::table_header_style(),
    );

    let mut line = rect.y as usize + 2;
    let rows = [
        format!(
            "Empire         : {}",
            player.empire_name
        ),
        format!(
            "Handle         : {}",
            player.handle.as_deref().unwrap_or("-")
        ),
        format!("Tax Rate       : {}%", player.tax_rate),
        format!("Visible Worlds : {}", worlds.len()),
        format!("Owned Planets  : {}", planets.len()),
        format!("Owned Fleets   : {}", fleets.len()),
        format!("Queued Mail    : {}", hosted.snapshot.queued_mail.len()),
        format!("Report Blocks  : {}", hosted.snapshot.report_blocks.len()),
    ];
    for row in rows {
        buffer.write_text_clipped(line, rect.x as usize + 2, &row, classic::table_body_style());
        line += 1;
    }

    line += 1;
    buffer.write_text_clipped(line, rect.x as usize + 2, "PLANETS", classic::table_header_style());
    line += 1;
    for planet in planets.iter().take(6) {
        let coords = format!("{:02},{:02}", planet.coords[0], planet.coords[1]);
        let row = format!(
            "{} | {} | prod {} / {} | store {}",
            planet.name,
            coords,
            planet.current_production,
            planet.potential_production,
            planet.stored_points
        );
        buffer.write_text_clipped(line, rect.x as usize + 2, &row, classic::table_body_style());
        line += 1;
    }

    line += 1;
    buffer.write_text_clipped(line, rect.x as usize + 2, "FLEETS", classic::table_header_style());
    line += 1;
    for fleet in fleets.iter().take(6) {
        let coords = format!("{:02},{:02}", fleet.coords[0], fleet.coords[1]);
        let row = format!(
            "#{} | {} | {} | {}",
            fleet.fleet_id,
            coords,
            fleet.order_summary,
            fleet.ships.summary
        );
        buffer.write_text_clipped(line, rect.x as usize + 2, &row, classic::table_body_style());
        line += 1;
    }

    if let Some(status) = hosted.submit_status.as_deref() {
        let footer_row = rect.y as usize + rect.height.saturating_sub(2) as usize;
        buffer.write_text_clipped(
            footer_row,
            rect.x as usize + 2,
            status,
            classic::notice_style(),
        );
    }
}
