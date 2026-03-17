use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        database: &app.database,
        player: &app.player,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::Starmap if app.starmap_state.capture_complete => {
            app.starmap.render_complete()
        }
        ScreenId::Starmap if app.starmap_state.dump_active => app
            .starmap
            .render_dump_page(&app.starmap_state.dump_lines, app.starmap_state.dump_offset),
        ScreenId::Starmap => app.starmap.render_prompt(app.starmap_state.status.as_deref()),
        ScreenId::PartialStarmapPrompt => app.partial_starmap.render_prompt(
            app.starmap_state.partial_center,
            &app.starmap_state.partial_input,
            app.starmap_state.partial_error.as_deref(),
            app.command_return_menu,
        ),
        ScreenId::PartialStarmapView => app.partial_starmap.render_view(
            &frame,
            &app.database,
            app.starmap_state.partial_center,
        ),
        _ => unreachable!("starmap views called for non-starmap screen"),
    }
}
