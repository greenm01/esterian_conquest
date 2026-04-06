//! Top-level render dispatch: assembles the three-column dashboard frame.

use nc_ui::PlayfieldBuffer;

use crate::app::state::{ActiveOverlay, ActivePopup, DashApp};
use crate::layout;
use crate::overlays::{
    self,
    frame::{draw_full_backdrop, overlay_backdrop, OverlayBackdrop},
};
use crate::panels::{diplomacy, economy, fleets, known_galaxy, planets, sector_detail, starmap};
use crate::popups;

pub fn render(app: &DashApp) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buf = layout::new_dashboard_buffer(app.geometry);
    let dashboard = layout::dashboard_layout(app);
    let widgets = dashboard.widgets;

    // Draw structural borders and header/footer.
    layout::draw_frame(&mut buf, dashboard.frame, &widgets);
    layout::draw_header(&mut buf, app, &dashboard);
    layout::draw_footer(&mut buf, app, &dashboard);

    // Left column panels.
    economy::draw(&mut buf, app, widgets.left_economy);
    planets::draw(&mut buf, app, widgets.left_planets);
    fleets::draw(&mut buf, app, widgets.left_fleets);

    // Center: starmap.
    starmap::draw(&mut buf, app, widgets.center_map);

    // Right column panels.
    known_galaxy::draw(&mut buf, app, widgets.right_galaxy);
    diplomacy::draw(&mut buf, app, widgets.right_diplomacy);
    sector_detail::draw(&mut buf, app, widgets.right_sector_detail);

    let underlay = help_underlay_overlay(app.overlay, app.help_return_overlay);

    // Overlay (drawn over everything if active). Help keeps the calling screen visible
    // underneath it instead of dropping back to the raw dashboard.
    if matches!(overlay_backdrop(underlay), OverlayBackdrop::FullBackdrop) {
        draw_full_backdrop(&mut buf);
    }

    if underlay != ActiveOverlay::None {
        draw_non_help_overlay(&mut buf, app, underlay);
    }

    if app.overlay == ActiveOverlay::Help {
        overlays::help::draw(&mut buf, app);
    }

    if app.overlay == ActiveOverlay::None {
        if let ActivePopup::PlanetDetail {
            planet_record_index_1_based,
        } = app.popup
        {
            popups::planet_detail::draw(
                &mut buf,
                app,
                widgets.center_map,
                planet_record_index_1_based,
            );
        }
    }

    Ok(buf)
}

fn help_underlay_overlay(
    active: ActiveOverlay,
    help_return_overlay: ActiveOverlay,
) -> ActiveOverlay {
    if active == ActiveOverlay::Help {
        help_return_overlay
    } else {
        active
    }
}

fn draw_non_help_overlay(buf: &mut PlayfieldBuffer, app: &DashApp, overlay: ActiveOverlay) {
    match overlay {
        ActiveOverlay::None | ActiveOverlay::Help => {}
        ActiveOverlay::PlanetList => overlays::planet_list::draw(buf, app),
        ActiveOverlay::FleetList => overlays::fleet_list::draw(buf, app),
        ActiveOverlay::IntelDatabase => overlays::intel_database::draw(buf, app),
        ActiveOverlay::Inbox => overlays::inbox::draw(buf, app),
        ActiveOverlay::Diplomacy => overlays::diplomacy::draw(buf, app),
        ActiveOverlay::Settings => overlays::settings::draw(buf, app),
    }
}

#[cfg(test)]
mod tests {
    use super::help_underlay_overlay;
    use crate::app::state::ActiveOverlay;

    #[test]
    fn help_over_dense_overlay_keeps_dense_underlay() {
        assert_eq!(
            help_underlay_overlay(ActiveOverlay::Help, ActiveOverlay::PlanetList),
            ActiveOverlay::PlanetList
        );
    }

    #[test]
    fn global_help_has_no_underlay_overlay() {
        assert_eq!(
            help_underlay_overlay(ActiveOverlay::Help, ActiveOverlay::None),
            ActiveOverlay::None
        );
    }
}
