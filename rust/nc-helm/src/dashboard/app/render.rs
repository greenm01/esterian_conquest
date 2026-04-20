//! Top-level render dispatch: assembles the three-column dashboard frame.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::widgets::WidgetRect;

use crate::dashboard::app::panel_cache::CachedPanel;
use crate::dashboard::app::state::{ActiveOverlay, ActivePopup, DashApp, MapViewMode};
use crate::dashboard::layout::{
    self, dashboard_fits_canvas, dashboard_layout, draw_footer, draw_frame, draw_header,
    layout_canvas_requirement, new_dashboard_buffer, required_dashboard_frame,
};
use crate::dashboard::modal::{
    ModalPlacement, ModalTheme, draw_modal_frame_in_parent_with_placement,
};
use crate::dashboard::overlays;
use crate::dashboard::panels::{
    comms, diplomacy, economy, fleets, known_galaxy, planets, sector_detail, starmap, war_record,
};
use crate::dashboard::popups;
use crate::dashboard::theme;

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
        let blocker_required = crate::dashboard::geometry::ScreenGeometry::new(
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

    let rev = app.game_data_revision;
    let player = app.player_record_index_1_based;
    let mut cache = app.panel_cache.borrow_mut();

    // Left column panels.
    draw_cached(
        &mut buf,
        &mut cache.economy,
        panel_hash(rev, player, widgets.left_economy.outer),
        widgets.left_economy.outer,
        |buf| economy::draw(buf, app, widgets.left_economy),
    );
    draw_cached(
        &mut buf,
        &mut cache.planets,
        panel_hash(rev, player, widgets.left_planets.outer),
        widgets.left_planets.outer,
        |buf| planets::draw(buf, app, widgets.left_planets),
    );
    draw_cached(
        &mut buf,
        &mut cache.fleets,
        panel_hash(rev, player, widgets.left_fleets.outer),
        widgets.left_fleets.outer,
        |buf| fleets::draw(buf, app, widgets.left_fleets),
    );
    draw_cached(
        &mut buf,
        &mut cache.war_record,
        panel_hash(rev, player, widgets.left_war_record.outer),
        widgets.left_war_record.outer,
        |buf| war_record::draw(buf, app, widgets.left_war_record),
    );

    // Center: starmap (also depends on crosshair, map mode, zoom, settings).
    draw_cached(
        &mut buf,
        &mut cache.starmap,
        starmap_hash(
            rev,
            player,
            widgets.center_map.outer,
            app.crosshair_x,
            app.crosshair_y,
            app.map_view_mode,
            app.map_zoom_level,
            app.client_settings.dense_empty_sector_dots,
        ),
        widgets.center_map.outer,
        |buf| starmap::draw(buf, app, widgets.center_map),
    );

    // Right column panels.
    draw_cached(
        &mut buf,
        &mut cache.comms,
        panel_hash(rev, player, widgets.right_comms.outer),
        widgets.right_comms.outer,
        |buf| comms::draw(buf, app, widgets.right_comms),
    );
    draw_cached(
        &mut buf,
        &mut cache.known_galaxy,
        panel_hash(rev, player, widgets.right_galaxy.outer),
        widgets.right_galaxy.outer,
        |buf| known_galaxy::draw(buf, app, widgets.right_galaxy),
    );
    draw_cached(
        &mut buf,
        &mut cache.diplomacy,
        diplomacy_hash(
            rev,
            player,
            widgets.right_diplomacy.outer,
            app.diplomacy_scroll,
        ),
        widgets.right_diplomacy.outer,
        |buf| diplomacy::draw(buf, app, widgets.right_diplomacy),
    );
    draw_cached(
        &mut buf,
        &mut cache.sector_detail,
        sector_detail_hash(
            rev,
            player,
            widgets.right_sector_detail.outer,
            app.crosshair_x,
            app.crosshair_y,
        ),
        widgets.right_sector_detail.outer,
        |buf| sector_detail::draw(buf, app, widgets.right_sector_detail),
    );

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
    canvas: crate::dashboard::geometry::ScreenGeometry,
    required: crate::dashboard::geometry::ScreenGeometry,
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
    map_frame: crate::dashboard::layout::MapWidgetFrame,
) {
    let parent = crate::dashboard::overlays::frame::overlay_parent_rect(map_frame);
    let placement = app
        .popup_position
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    let message = super::quit_confirm_message();
    let popup_width = super::quit_confirm_popup_width();
    let popup = draw_modal_frame_in_parent_with_placement(
        buf,
        super::QUIT_CONFIRM_TITLE,
        popup_width,
        super::QUIT_CONFIRM_HEIGHT as u16,
        parent,
        placement,
        ModalTheme {
            body_style: theme::prompt_style(),
            pad_style: theme::prompt_style(),
            chrome_style: theme::title_style(),
            title_style: theme::title_style(),
        },
    );
    let content = crate::dashboard::modal::modal_content_rect(popup);
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
    map_frame: crate::dashboard::layout::MapWidgetFrame,
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

fn draw_cached<F>(
    buf: &mut PlayfieldBuffer,
    entry: &mut Option<CachedPanel>,
    inputs_hash: u64,
    outer: WidgetRect,
    draw_fn: F,
) where
    F: Fn(&mut PlayfieldBuffer),
{
    if entry.as_ref().is_some_and(|e| e.inputs_hash == inputs_hash) {
        let cached = entry.as_ref().unwrap();
        buf.blit_region(outer.row, outer.col, outer.width, outer.height, &cached.cells);
        for glyph in &cached.overlay_glyphs {
            buf.push_overlay_glyph_at(glyph.ch, glyph.style, glyph.center_col, glyph.center_row);
        }
    } else {
        let overlay_len_before = buf.overlay_glyphs().len();
        draw_fn(buf);
        let cells = buf.copy_region(outer.row, outer.col, outer.width, outer.height);
        let overlay_glyphs = buf.overlay_glyphs()[overlay_len_before..].to_vec();
        *entry = Some(CachedPanel {
            inputs_hash,
            cells,
            overlay_glyphs,
        });
    }
}

fn panel_hash(revision: u64, player: usize, outer: WidgetRect) -> u64 {
    let mut h = DefaultHasher::new();
    revision.hash(&mut h);
    player.hash(&mut h);
    outer.row.hash(&mut h);
    outer.col.hash(&mut h);
    outer.width.hash(&mut h);
    outer.height.hash(&mut h);
    h.finish()
}

fn starmap_hash(
    revision: u64,
    player: usize,
    outer: WidgetRect,
    cx: u8,
    cy: u8,
    mode: MapViewMode,
    zoom: u8,
    dense_dots: bool,
) -> u64 {
    let mut h = DefaultHasher::new();
    revision.hash(&mut h);
    player.hash(&mut h);
    outer.row.hash(&mut h);
    outer.col.hash(&mut h);
    outer.width.hash(&mut h);
    outer.height.hash(&mut h);
    cx.hash(&mut h);
    cy.hash(&mut h);
    mode.hash(&mut h);
    zoom.hash(&mut h);
    dense_dots.hash(&mut h);
    h.finish()
}

fn diplomacy_hash(revision: u64, player: usize, outer: WidgetRect, scroll: usize) -> u64 {
    let mut h = DefaultHasher::new();
    revision.hash(&mut h);
    player.hash(&mut h);
    outer.row.hash(&mut h);
    outer.col.hash(&mut h);
    outer.width.hash(&mut h);
    outer.height.hash(&mut h);
    scroll.hash(&mut h);
    h.finish()
}

fn sector_detail_hash(revision: u64, player: usize, outer: WidgetRect, cx: u8, cy: u8) -> u64 {
    let mut h = DefaultHasher::new();
    revision.hash(&mut h);
    player.hash(&mut h);
    outer.row.hash(&mut h);
    outer.col.hash(&mut h);
    outer.width.hash(&mut h);
    outer.height.hash(&mut h);
    cx.hash(&mut h);
    cy.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::help_underlay_overlay;
    use crate::dashboard::app::state::ActiveOverlay;

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
