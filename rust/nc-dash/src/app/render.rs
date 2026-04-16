//! Top-level render dispatch: assembles the three-column dashboard frame.

use crate::buffer::PlayfieldBuffer;

use crate::app::state::{ActiveOverlay, ActivePopup, DashApp};
use crate::layout::{
    self, dashboard_fits_canvas, dashboard_layout, draw_footer, draw_frame, draw_header,
    layout_canvas_requirement, new_dashboard_buffer, required_dashboard_frame,
};
use crate::modal::{
    ModalPlacement, ModalTheme, draw_modal_frame_in_parent_with_placement_without_close_button,
    modal_min_width_for_title,
};
use crate::overlays;
use crate::panels::{
    comms, diplomacy, economy, fleets, known_galaxy, planets, sector_detail, starmap, war_record,
};
use crate::popups;
use crate::theme;

pub fn render(app: &DashApp) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buf = new_dashboard_buffer(app.geometry);

    let required = required_dashboard_frame(app);
    if app.geometry.width() < required.width() || app.geometry.height() < required.height() {
        render_too_small_blocker(&mut buf, app.geometry, required);
        return Ok(buf);
    }

    let dashboard = dashboard_layout(app);
    if !dashboard_fits_canvas(app.geometry, &dashboard) {
        let layout_required = layout_canvas_requirement(&dashboard);
        let blocker_required = crate::geometry::ScreenGeometry::new(
            required.width().max(layout_required.width()),
            required.height().max(layout_required.height()),
        );
        render_too_small_blocker(&mut buf, app.geometry, blocker_required);
        return Ok(buf);
    }
    let widgets = dashboard.widgets;

    // Draw structural borders and header/footer.
    draw_frame(&mut buf, dashboard.frame, &widgets);
    draw_header(&mut buf, app, &dashboard);
    draw_footer(&mut buf, app, &dashboard);

    // Left column panels.
    economy::draw(&mut buf, app, widgets.left_economy);
    planets::draw(&mut buf, app, widgets.left_planets);
    fleets::draw(&mut buf, app, widgets.left_fleets);
    war_record::draw(&mut buf, app, widgets.left_war_record);

    // Center: starmap.
    starmap::draw(&mut buf, app, widgets.center_map);

    // Right column panels.
    comms::draw(&mut buf, app, widgets.right_comms);
    known_galaxy::draw(&mut buf, app, widgets.right_galaxy);
    diplomacy::draw(&mut buf, app, widgets.right_diplomacy);
    sector_detail::draw(&mut buf, app, widgets.right_sector_detail);

    let underlay = help_underlay_overlay(app.overlay, app.help_return_overlay);

    if underlay != ActiveOverlay::None {
        draw_non_help_overlay(&mut buf, app, widgets.center_map, underlay);
    }

    if app.overlay == ActiveOverlay::Help {
        overlays::help::draw(&mut buf, app, widgets.center_map);
    }

    if app.overlay == ActiveOverlay::None {
        match app.popup {
            ActivePopup::QuitConfirm => render_quit_confirm(&mut buf, app, widgets.center_map),
            ActivePopup::PlanetDetail {
                planet_record_index_1_based,
            } => {
                popups::planet_detail::draw(
                    &mut buf,
                    app,
                    widgets.center_map,
                    planet_record_index_1_based,
                );
            }
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => {
                popups::owned_planet::draw(
                    &mut buf,
                    app,
                    widgets.center_map,
                    planet_record_index_1_based,
                );
            }
            ActivePopup::None => {}
        }
    }

    Ok(buf)
}

fn render_too_small_blocker(
    buf: &mut PlayfieldBuffer,
    canvas: crate::geometry::ScreenGeometry,
    required: crate::geometry::ScreenGeometry,
) {
    let msg1 = "TERMINAL WINDOW TOO SMALL";
    let msg2 = format!(
        "Requires {}x{} (Current: {}x{})",
        required.width(),
        required.height(),
        canvas.width(),
        canvas.height()
    );
    let msg3 = "Resize window or press Alt-Q to quit.";

    let row_mid = canvas.height() / 2;
    let col_mid = canvas.width() / 2;

    let start1 = col_mid.saturating_sub(msg1.chars().count() / 2);
    layout::write_clipped(
        buf,
        row_mid.saturating_sub(1),
        start1,
        canvas.width().saturating_sub(start1),
        msg1,
        theme::error_style(),
    );
    let start2 = col_mid.saturating_sub(msg2.chars().count() / 2);
    layout::write_clipped(
        buf,
        row_mid,
        start2,
        canvas.width().saturating_sub(start2),
        &msg2,
        theme::body_style(),
    );
    let start3 = col_mid.saturating_sub(msg3.chars().count() / 2);
    layout::write_clipped(
        buf,
        row_mid.saturating_add(1),
        start3,
        canvas.width().saturating_sub(start3),
        msg3,
        theme::dim_style(),
    );
}

fn render_quit_confirm(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: crate::layout::MapWidgetFrame,
) {
    let parent = crate::overlays::frame::overlay_parent_rect(map_frame);
    let placement = app
        .popup_position
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    let message = "Quit Game? Y/[N]";
    let popup_width = (message.chars().count() + 4).max(modal_min_width_for_title("QUIT"));
    let popup = draw_modal_frame_in_parent_with_placement_without_close_button(
        buf,
        "QUIT",
        popup_width,
        5,
        parent,
        placement,
        ModalTheme {
            body_style: theme::prompt_style(),
            pad_style: theme::prompt_style(),
            chrome_style: theme::title_style(),
            title_style: theme::title_style(),
        },
    );
    let content = crate::modal::modal_content_rect(popup);
    let message_width = message.chars().count();
    let content_width = content.width as usize;
    let start_col = content.x as usize + content_width.saturating_sub(message_width) / 2;
    let row = content.y as usize + content.height.saturating_sub(1) as usize / 2;
    layout::write_clipped(
        buf,
        row,
        start_col,
        content_width.saturating_sub(start_col.saturating_sub(content.x as usize)),
        message,
        theme::prompt_style(),
    );
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

fn draw_non_help_overlay(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: crate::layout::MapWidgetFrame,
    overlay: ActiveOverlay,
) {
    match overlay {
        ActiveOverlay::None | ActiveOverlay::Help => {}
        ActiveOverlay::PlanetList => overlays::planet_list::draw(buf, app, map_frame),
        ActiveOverlay::FleetList => overlays::fleet_list::draw(buf, app, map_frame),
        ActiveOverlay::IntelDatabase => overlays::intel_database::draw(buf, app, map_frame),
        ActiveOverlay::Inbox => overlays::inbox::draw(buf, app, map_frame),
        ActiveOverlay::Diplomacy => overlays::diplomacy::draw(buf, app, map_frame),
        ActiveOverlay::Settings => overlays::settings::draw(buf, app, map_frame),
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
