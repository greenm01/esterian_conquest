use crate::app::state::App;
use crate::domains::starmap::StarmapAction;

pub fn update(app: &mut App, action: StarmapAction) {
    match action {
        StarmapAction::OpenFull => app.open_starmap(),
        StarmapAction::OpenPartialView(menu) => app.open_partial_starmap_view(menu),
        StarmapAction::BeginDump => app.begin_starmap_dump(),
        StarmapAction::AdvancePage => app.advance_starmap_page(),
        StarmapAction::Export => {
            if let Err(err) = app.export_starmap() {
                app.log_action_error("export_starmap", err.as_ref());
                let status =
                    Some("Unable to export the star map right now. Please try again.".to_string());
                if app.current_screen == crate::screen::ScreenId::PartialStarmapView {
                    app.starmap_state.partial_status = status;
                } else {
                    app.starmap_state.status = status;
                }
            }
        }
        StarmapAction::MovePartial(dx, dy) => app.move_partial_starmap(dx, dy),
        StarmapAction::OpenPlanetInfoAtCenter => app.open_partial_starmap_planet_info(),
    }
}
