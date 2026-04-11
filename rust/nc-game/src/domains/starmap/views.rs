use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        player_activity_states: &app.player_activity_states,
        player_lifecycle_states: &app.player_lifecycle_states,
        winner_state: app.winner_state,
        planet_intel_snapshots: &app.planet_intel_snapshots,
        owned_planet_years: &app.owned_planet_years,
        geometry: app.screen_geometry,
    };
    match app.current_screen {
        ScreenId::Starmap if app.starmap_state.capture_complete => {
            app.starmap.render_complete(frame.geometry)
        }
        ScreenId::Starmap if app.starmap_state.dump_active => app.starmap.render_dump_page(
            frame.geometry,
            &app.starmap_state.dump_lines,
            app.starmap_state.dump_offset,
        ),
        ScreenId::Starmap => app
            .starmap
            .render_prompt(frame.geometry, app.starmap_state.status.as_deref()),
        ScreenId::PartialStarmapView => app.partial_starmap.render_view(
            &frame,
            app.starmap_state.partial_center,
            app.starmap_state.partial_status.as_deref(),
        ),
        _ => unreachable!("starmap views called for non-starmap screen"),
    }
}
