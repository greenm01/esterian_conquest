use nc_ui::modal::Rect;
use nc_ui::theme::classic;
use nc_ui::PlayfieldBuffer;

use crate::lobby::state::HostedGameView;

pub fn render(buffer: &mut PlayfieldBuffer, rect: Rect, hosted: &HostedGameView) {
    let state = &hosted.snapshot.state;
    let player = state.get("player").cloned().unwrap_or_default();
    let worlds = state
        .get("starmap")
        .and_then(|starmap| starmap.get("worlds"))
        .and_then(|worlds| worlds.as_array())
        .cloned()
        .unwrap_or_default();
    let planets = state
        .get("owned_planets")
        .and_then(|planets| planets.as_array())
        .cloned()
        .unwrap_or_default();
    let fleets = state
        .get("owned_fleets")
        .and_then(|fleets| fleets.as_array())
        .cloned()
        .unwrap_or_default();

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
            player
                .get("empire_name")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
        ),
        format!(
            "Handle         : {}",
            player
                .get("handle")
                .and_then(|value| value.as_str())
                .unwrap_or("-")
        ),
        format!(
            "Tax Rate       : {}%",
            player
                .get("tax_rate")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        ),
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
        let coords = planet
            .get("coords")
            .and_then(|coords| coords.as_array())
            .map(|coords| {
                format!(
                    "{:02},{:02}",
                    coords.first().and_then(|value| value.as_u64()).unwrap_or(0),
                    coords.get(1).and_then(|value| value.as_u64()).unwrap_or(0)
                )
            })
            .unwrap_or_else(|| "--,--".to_string());
        let row = format!(
            "{} | {} | prod {} / {} | store {}",
            planet.get("name").and_then(|value| value.as_str()).unwrap_or("world"),
            coords,
            planet
                .get("current_production")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            planet
                .get("potential_production")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            planet
                .get("stored_points")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        );
        buffer.write_text_clipped(line, rect.x as usize + 2, &row, classic::table_body_style());
        line += 1;
    }

    line += 1;
    buffer.write_text_clipped(line, rect.x as usize + 2, "FLEETS", classic::table_header_style());
    line += 1;
    for fleet in fleets.iter().take(6) {
        let coords = fleet
            .get("coords")
            .and_then(|coords| coords.as_array())
            .map(|coords| {
                format!(
                    "{:02},{:02}",
                    coords.first().and_then(|value| value.as_u64()).unwrap_or(0),
                    coords.get(1).and_then(|value| value.as_u64()).unwrap_or(0)
                )
            })
            .unwrap_or_else(|| "--,--".to_string());
        let row = format!(
            "#{} | {} | {} | {}",
            fleet.get("fleet_id").and_then(|value| value.as_u64()).unwrap_or(0),
            coords,
            fleet
                .get("order_summary")
                .and_then(|value| value.as_str())
                .unwrap_or("holding"),
            fleet
                .get("ships")
                .and_then(|ships| ships.get("summary"))
                .and_then(|value| value.as_str())
                .unwrap_or("-")
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
