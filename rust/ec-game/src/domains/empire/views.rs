use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::Enemies => app.enemies.render(
            &frame,
            &app.empire.enemies_input,
            app.empire.enemies_status.as_deref(),
            app.empire.enemies_scroll_offset,
            app.empire.enemies_cursor,
        ),
        ScreenId::EmpireStatus => app
            .empire_status
            .render_with_menu(&frame, app.command_return_menu),
        ScreenId::EmpireProfile => app
            .empire_profile
            .render_with_menu(&frame, app.command_return_menu),
        ScreenId::Rankings(sort) => {
            app.rankings
                .render_table(&frame, sort, app.command_return_menu)
        }
        _ => unreachable!("empire views called for non-empire screen"),
    }
}
