//! Top-level render dispatch: assembles the three-column dashboard frame.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout;
use crate::panels::{diplomacy, economy, fleets, known_galaxy, planets, reports, starmap};

pub fn render(app: &DashApp) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buf = layout::new_dashboard_buffer(app.geometry);

    // Draw structural borders.
    layout::draw_frame(&mut buf, app);

    // Header bar.
    layout::draw_header(&mut buf, app);

    // Footer bar.
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

    Ok(buf)
}
