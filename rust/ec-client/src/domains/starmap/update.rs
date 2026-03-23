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
                eprintln!("export starmap failed: {err}");
            }
        }
        StarmapAction::MovePartial(dx, dy) => app.move_partial_starmap(dx, dy),
        StarmapAction::OpenPlanetInfoAtCenter => app.open_partial_starmap_planet_info(),
    }
}
