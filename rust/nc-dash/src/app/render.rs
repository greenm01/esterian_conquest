//! Top-level render dispatch: assembles the three-column dashboard frame.

use nc_ui::PlayfieldBuffer;

use crate::app::state::{ActiveOverlay, DashApp};
use crate::layout;
use crate::overlays;
use crate::panels::{diplomacy, economy, fleets, known_galaxy, planets, reports, starmap};

pub fn render(app: &DashApp) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buf = layout::new_dashboard_buffer(app.geometry);

    // Draw structural borders and header/footer.
    layout::draw_frame(&mut buf, app);
    layout::draw_header(&mut buf, app);
    layout::draw_footer(&mut buf, app);

    // Left column panels.
    economy::draw(&mut buf, app);
    planets::draw(&mut buf, app);
    fleets::draw(&mut buf, app);

    // Center: starmap.
    starmap::draw(&mut buf, app);

    // Right column panels.
    known_galaxy::draw(&mut buf, app);
    diplomacy::draw(&mut buf, app);
    reports::draw(&mut buf, app);

    // Overlay (drawn over everything if active).
    match app.overlay {
        ActiveOverlay::None => {}
        ActiveOverlay::PlanetList => overlays::planet_list::draw(&mut buf, app),
        ActiveOverlay::FleetList => overlays::fleet_list::draw(&mut buf, app),
        ActiveOverlay::IntelDatabase => overlays::intel_database::draw(&mut buf, app),
        ActiveOverlay::Inbox => overlays::inbox::draw(&mut buf, app),
        ActiveOverlay::Diplomacy => overlays::diplomacy::draw(&mut buf, app),
        ActiveOverlay::Settings => overlays::settings::draw(&mut buf, app),
        ActiveOverlay::Help => overlays::help::draw(&mut buf, app),
    }

    Ok(buf)
}
