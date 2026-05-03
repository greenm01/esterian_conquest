//! Top-level render dispatch: assembles the three-column dashboard frame.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::widgets::WidgetRect;

use crate::dashboard::app::panel_cache::CachedPanel;
use crate::dashboard::app::state::{
    ActiveOverlay, ActivePopup, DashApp, FleetOverlayPromptMode, InboxPromptMode,
    IntelOverlayPromptMode, MapViewMode, PlanetOverlayPromptMode,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirtyRect {
    pub row: usize,
    pub col: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegionHashes {
    pub frame: u64,
    pub header: u64,
    pub footer: u64,
    pub economy: u64,
    pub planets: u64,
    pub fleets: u64,
    pub war_record: u64,
    pub starmap: u64,
    pub comms: u64,
    pub known_galaxy: u64,
    pub diplomacy: u64,
    pub sector_detail: u64,
    pub dynamic_layer_active: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct IncrementalRenderOutcome {
    pub hashes: RegionHashes,
    pub dirty_rects: Vec<DirtyRect>,
    pub full_rebuild: bool,
}

#[allow(dead_code)]
pub fn render(app: &DashApp) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buf = new_dashboard_buffer(app.geometry);
    render_into(app, &mut buf)?;
    Ok(buf)
}

pub fn render_into(
    app: &DashApp,
    buf: &mut PlayfieldBuffer,
) -> Result<(), Box<dyn std::error::Error>> {
    buf.reset(
        app.geometry.width(),
        app.geometry.height(),
        theme::body_style(),
    );

    let required = required_dashboard_frame(app);
    if app.geometry.width() < required.width() || app.geometry.height() < required.height() {
        render_too_small_blocker(buf, app.geometry, required);
        return Ok(());
    }

    let dashboard = dashboard_layout(app);
    if !dashboard_fits_canvas(app.geometry, &dashboard) {
        let layout_required = layout_canvas_requirement(&dashboard);
        let blocker_required = crate::dashboard::geometry::ScreenGeometry::new(
            required.width().max(layout_required.width()),
            required.height().max(layout_required.height()),
        );
        render_too_small_blocker(buf, app.geometry, blocker_required);
        return Ok(());
    }
    let widgets = dashboard.widgets;
    draw_frame(buf, dashboard.frame, &widgets);
    draw_header(buf, app, &dashboard);
    draw_footer(buf, app, &dashboard);
    let hashes = region_hashes(app, &dashboard);
    let mut cache = app.panel_cache.borrow_mut();
    draw_all_panels(buf, app, widgets, &hashes, &mut cache);
    draw_dynamic_layer(buf, app, widgets);

    Ok(())
}

pub(crate) fn render_incremental_into(
    app: &DashApp,
    buf: &mut PlayfieldBuffer,
    previous_hashes: Option<&RegionHashes>,
) -> Result<IncrementalRenderOutcome, Box<dyn std::error::Error>> {
    let required = required_dashboard_frame(app);
    let geometry_changed =
        buf.width() != app.geometry.width() || buf.height() != app.geometry.height();
    if app.geometry.width() < required.width() || app.geometry.height() < required.height() {
        render_into(app, buf)?;
        return Ok(IncrementalRenderOutcome {
            hashes: too_small_hashes(app),
            dirty_rects: vec![DirtyRect {
                row: 0,
                col: 0,
                width: app.geometry.width(),
                height: app.geometry.height(),
            }],
            full_rebuild: true,
        });
    }

    let dashboard = dashboard_layout(app);
    if !dashboard_fits_canvas(app.geometry, &dashboard) {
        render_into(app, buf)?;
        return Ok(IncrementalRenderOutcome {
            hashes: too_small_hashes(app),
            dirty_rects: vec![DirtyRect {
                row: 0,
                col: 0,
                width: app.geometry.width(),
                height: app.geometry.height(),
            }],
            full_rebuild: true,
        });
    }

    let widgets = dashboard.widgets;
    let hashes = region_hashes(app, &dashboard);
    let frame_changed = previous_hashes
        .map(|prev| prev.frame != hashes.frame)
        .unwrap_or(true);
    if geometry_changed || frame_changed {
        render_into(app, buf)?;
        return Ok(IncrementalRenderOutcome {
            hashes,
            dirty_rects: vec![DirtyRect {
                row: 0,
                col: 0,
                width: app.geometry.width(),
                height: app.geometry.height(),
            }],
            full_rebuild: true,
        });
    }

    let previous = previous_hashes.expect("previous hashes required after frame check");
    let mut dirty_rects = Vec::new();

    if previous.header != hashes.header {
        clear_horizontal_bar(buf, widgets.header_bar_row, &dashboard, app.geometry);
        draw_header(buf, app, &dashboard);
        dirty_rects.push(frame_bar_rect(
            widgets.header_bar_row,
            &dashboard,
            app.geometry,
        ));
    }

    if previous.footer != hashes.footer {
        clear_horizontal_bar(buf, widgets.footer_bar_row, &dashboard, app.geometry);
        draw_footer(buf, app, &dashboard);
        dirty_rects.push(frame_bar_rect(
            widgets.footer_bar_row,
            &dashboard,
            app.geometry,
        ));
    }

    let mut cache = app.panel_cache.borrow_mut();
    if hashes.dynamic_layer_active || previous.dynamic_layer_active {
        buf.clear_cursor();
        let interior = dashboard_interior_rect(widgets);
        buf.fill_rect(
            interior.row,
            interior.col,
            interior.width,
            interior.height,
            theme::body_style(),
        );
        draw_frame(buf, dashboard.frame, &widgets);
        draw_footer(buf, app, &dashboard);
        draw_all_panels(buf, app, widgets, &hashes, &mut cache);
        draw_dynamic_layer(buf, app, widgets);
        dirty_rects.push(dashboard_interior_rect(widgets));
    } else {
        redraw_panel_if_needed(
            buf,
            previous.economy != hashes.economy,
            widgets.left_economy.outer,
            &mut cache.economy,
            hashes.economy,
            |buf| economy::draw(buf, app, widgets.left_economy),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.planets != hashes.planets,
            widgets.left_planets.outer,
            &mut cache.planets,
            hashes.planets,
            |buf| planets::draw(buf, app, widgets.left_planets),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.fleets != hashes.fleets,
            widgets.left_fleets.outer,
            &mut cache.fleets,
            hashes.fleets,
            |buf| fleets::draw(buf, app, widgets.left_fleets),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.war_record != hashes.war_record,
            widgets.left_war_record.outer,
            &mut cache.war_record,
            hashes.war_record,
            |buf| war_record::draw(buf, app, widgets.left_war_record),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.starmap != hashes.starmap,
            widgets.center_map.outer,
            &mut cache.starmap,
            hashes.starmap,
            |buf| starmap::draw(buf, app, widgets.center_map),
            &mut dirty_rects,
        );
        starmap::apply_selection_overlay(buf, app, widgets.center_map);
        redraw_panel_if_needed(
            buf,
            previous.comms != hashes.comms,
            widgets.right_comms.outer,
            &mut cache.comms,
            hashes.comms,
            |buf| comms::draw(buf, app, widgets.right_comms),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.known_galaxy != hashes.known_galaxy,
            widgets.right_galaxy.outer,
            &mut cache.known_galaxy,
            hashes.known_galaxy,
            |buf| known_galaxy::draw(buf, app, widgets.right_galaxy),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.diplomacy != hashes.diplomacy,
            widgets.right_diplomacy.outer,
            &mut cache.diplomacy,
            hashes.diplomacy,
            |buf| diplomacy::draw(buf, app, widgets.right_diplomacy),
            &mut dirty_rects,
        );
        redraw_panel_if_needed(
            buf,
            previous.sector_detail != hashes.sector_detail,
            widgets.right_sector_detail.outer,
            &mut cache.sector_detail,
            hashes.sector_detail,
            |buf| sector_detail::draw(buf, app, widgets.right_sector_detail),
            &mut dirty_rects,
        );
    }

    Ok(IncrementalRenderOutcome {
        hashes,
        dirty_rects,
        full_rebuild: false,
    })
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
    render_centered_confirm(
        buf,
        app,
        map_frame,
        crate::dashboard::QUIT_CONFIRM_TITLE,
        super::quit_confirm_message(),
    );
}

fn render_inbox_message_confirm(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: crate::dashboard::layout::MapWidgetFrame,
    action: crate::dashboard::app::state::InboxMessageConfirmAction,
) {
    render_centered_confirm(
        buf,
        app,
        map_frame,
        super::inbox_message_confirm_title(),
        super::inbox_message_confirm_message(action),
    );
}

fn render_centered_confirm(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: crate::dashboard::layout::MapWidgetFrame,
    title: &str,
    message: &str,
) {
    let parent = crate::dashboard::overlays::frame::overlay_parent_rect(map_frame);
    let placement = app
        .popup_position
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    let popup_width = super::confirm_popup_width(title, message);
    let popup = draw_modal_frame_in_parent_with_placement(
        buf,
        title,
        popup_width,
        crate::dashboard::QUIT_CONFIRM_HEIGHT as u16,
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

fn draw_dynamic_layer(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    widgets: crate::dashboard::layout::DashboardWidgetFrames,
) {
    buf.clear_cursor();
    let underlay = help_underlay_overlay(app.overlay, app.help_return_overlay);
    if underlay != ActiveOverlay::None && !caller_overlay_is_hidden_by_popup(app) {
        draw_non_help_overlay(buf, app, widgets.center_map, underlay);
    }
    if app.overlay == ActiveOverlay::Help {
        overlays::help::draw(buf, app, app.help_context);
    }
    if let Some((title, message)) = confirm_popup_text(app.popup) {
        render_centered_confirm(buf, app, widgets.center_map, title, message);
        return;
    }
    if app.overlay == ActiveOverlay::None || caller_overlay_popup_is_visible(app) {
        draw_popup_layer(buf, app, widgets.center_map);
    }
}

fn caller_overlay_is_hidden_by_popup(app: &DashApp) -> bool {
    if matches!(
        app.popup,
        ActivePopup::QuitConfirm | ActivePopup::InboxMessageConfirm { .. }
    ) {
        return app.overlay != ActiveOverlay::None;
    }
    caller_overlay_popup_is_visible(app)
}

fn caller_overlay_popup_is_visible(app: &DashApp) -> bool {
    let popup_over_caller = matches!(
        app.popup,
        ActivePopup::OwnedPlanet { .. }
            | ActivePopup::PlanetDetail { .. }
            | ActivePopup::FleetDetail { .. }
            | ActivePopup::StartupReview
    );
    if !popup_over_caller {
        return false;
    }
    match app.overlay {
        ActiveOverlay::PlanetList => {
            app.planet_overlay.prompt_mode == PlanetOverlayPromptMode::None
        }
        ActiveOverlay::IntelDatabase => {
            app.intel_overlay.prompt_mode == IntelOverlayPromptMode::None
        }
        ActiveOverlay::FleetList => app.fleet_overlay.prompt_mode == FleetOverlayPromptMode::None,
        ActiveOverlay::Inbox => app.inbox_overlay.prompt_mode == InboxPromptMode::None,
        _ => false,
    }
}

fn draw_popup_layer(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: crate::dashboard::layout::MapWidgetFrame,
) {
    match app.popup {
        ActivePopup::QuitConfirm => render_quit_confirm(buf, app, map_frame),
        ActivePopup::InboxMessageConfirm { action } => {
            render_inbox_message_confirm(buf, app, map_frame, action);
        }
        ActivePopup::TaxPrompt => popups::tax_prompt::draw(buf, app, map_frame),
        ActivePopup::PlanetDetail {
            planet_record_index_1_based,
        } => {
            popups::planet_detail::draw(buf, app, map_frame, planet_record_index_1_based);
        }
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        } => {
            popups::owned_planet::draw(buf, app, map_frame, planet_record_index_1_based);
        }
        ActivePopup::FleetDetail {
            fleet_record_index_1_based,
        } => {
            popups::fleet_detail::draw(buf, app, map_frame, fleet_record_index_1_based);
        }
        ActivePopup::StartupReview => {
            popups::startup_review::draw(buf, app, map_frame);
        }
        ActivePopup::None => {}
    }
}

fn confirm_popup_text(popup: ActivePopup) -> Option<(&'static str, &'static str)> {
    match popup {
        ActivePopup::QuitConfirm => Some((
            crate::dashboard::QUIT_CONFIRM_TITLE,
            super::quit_confirm_message(),
        )),
        ActivePopup::InboxMessageConfirm { action } => Some((
            super::inbox_message_confirm_title(),
            super::inbox_message_confirm_message(action),
        )),
        _ => None,
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
        buf.blit_region(
            outer.row,
            outer.col,
            outer.width,
            outer.height,
            &cached.cells,
        );
    } else {
        draw_fn(buf);
        let cells = buf.copy_region(outer.row, outer.col, outer.width, outer.height);
        *entry = Some(CachedPanel { inputs_hash, cells });
    }
}

fn draw_all_panels(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    widgets: crate::dashboard::layout::DashboardWidgetFrames,
    hashes: &RegionHashes,
    cache: &mut crate::dashboard::app::panel_cache::PanelCache,
) {
    draw_cached(
        buf,
        &mut cache.economy,
        hashes.economy,
        widgets.left_economy.outer,
        |buf| economy::draw(buf, app, widgets.left_economy),
    );
    draw_cached(
        buf,
        &mut cache.planets,
        hashes.planets,
        widgets.left_planets.outer,
        |buf| planets::draw(buf, app, widgets.left_planets),
    );
    draw_cached(
        buf,
        &mut cache.fleets,
        hashes.fleets,
        widgets.left_fleets.outer,
        |buf| fleets::draw(buf, app, widgets.left_fleets),
    );
    draw_cached(
        buf,
        &mut cache.war_record,
        hashes.war_record,
        widgets.left_war_record.outer,
        |buf| war_record::draw(buf, app, widgets.left_war_record),
    );
    draw_cached(
        buf,
        &mut cache.starmap,
        hashes.starmap,
        widgets.center_map.outer,
        |buf| starmap::draw(buf, app, widgets.center_map),
    );
    starmap::apply_selection_overlay(buf, app, widgets.center_map);
    draw_cached(
        buf,
        &mut cache.comms,
        hashes.comms,
        widgets.right_comms.outer,
        |buf| comms::draw(buf, app, widgets.right_comms),
    );
    draw_cached(
        buf,
        &mut cache.known_galaxy,
        hashes.known_galaxy,
        widgets.right_galaxy.outer,
        |buf| known_galaxy::draw(buf, app, widgets.right_galaxy),
    );
    draw_cached(
        buf,
        &mut cache.diplomacy,
        hashes.diplomacy,
        widgets.right_diplomacy.outer,
        |buf| diplomacy::draw(buf, app, widgets.right_diplomacy),
    );
    draw_cached(
        buf,
        &mut cache.sector_detail,
        hashes.sector_detail,
        widgets.right_sector_detail.outer,
        |buf| sector_detail::draw(buf, app, widgets.right_sector_detail),
    );
}

fn redraw_panel_if_needed<F>(
    buf: &mut PlayfieldBuffer,
    dirty: bool,
    outer: WidgetRect,
    entry: &mut Option<CachedPanel>,
    hash: u64,
    draw_fn: F,
    dirty_rects: &mut Vec<DirtyRect>,
) where
    F: Fn(&mut PlayfieldBuffer),
{
    if !dirty {
        return;
    }
    buf.fill_rect(
        outer.row,
        outer.col,
        outer.width,
        outer.height,
        theme::body_style(),
    );
    draw_cached(buf, entry, hash, outer, draw_fn);
    dirty_rects.push(DirtyRect::from_widget(outer));
}

fn region_hashes(app: &DashApp, dashboard: &layout::DashboardLayout) -> RegionHashes {
    let widgets = dashboard.widgets;
    let rev = app.game_data_revision;
    let player = app.player_record_index_1_based;
    RegionHashes {
        frame: frame_hash(app, dashboard),
        header: header_hash(app),
        footer: footer_hash(app),
        economy: panel_hash(rev, player, widgets.left_economy.outer),
        planets: panel_hash(rev, player, widgets.left_planets.outer),
        fleets: panel_hash(rev, player, widgets.left_fleets.outer),
        war_record: panel_hash(rev, player, widgets.left_war_record.outer),
        starmap: starmap_hash(
            rev,
            player,
            widgets.center_map.outer,
            app.crosshair_x,
            app.crosshair_y,
            app.map_view_mode,
        ),
        comms: panel_hash(rev, player, widgets.right_comms.outer),
        known_galaxy: panel_hash(rev, player, widgets.right_galaxy.outer),
        diplomacy: diplomacy_hash(
            rev,
            player,
            widgets.right_diplomacy.outer,
            app.diplomacy_scroll,
        ),
        sector_detail: sector_detail_hash(
            rev,
            player,
            widgets.right_sector_detail.outer,
            app.crosshair_x,
            app.crosshair_y,
        ),
        dynamic_layer_active: dynamic_layer_active(app),
    }
}

fn too_small_hashes(app: &DashApp) -> RegionHashes {
    let mut hasher = DefaultHasher::new();
    app.geometry.width().hash(&mut hasher);
    app.geometry.height().hash(&mut hasher);
    app.is_terminal_too_small.hash(&mut hasher);
    let frame = hasher.finish();
    RegionHashes {
        frame,
        header: 0,
        footer: 0,
        economy: 0,
        planets: 0,
        fleets: 0,
        war_record: 0,
        starmap: 0,
        comms: 0,
        known_galaxy: 0,
        diplomacy: 0,
        sector_detail: 0,
        dynamic_layer_active: false,
    }
}

fn frame_hash(app: &DashApp, dashboard: &layout::DashboardLayout) -> u64 {
    let mut h = DefaultHasher::new();
    let widgets = dashboard.widgets;
    app.geometry.width().hash(&mut h);
    app.geometry.height().hash(&mut h);
    dashboard.frame.width().hash(&mut h);
    dashboard.frame.height().hash(&mut h);
    app.map_view_mode.hash(&mut h);
    widgets.outer_top.hash(&mut h);
    widgets.outer_bottom.hash(&mut h);
    widgets.header_bar_row.hash(&mut h);
    widgets.header_divider_row.hash(&mut h);
    widgets.footer_divider_row.hash(&mut h);
    widgets.footer_bar_row.hash(&mut h);
    widgets.left_divider_col.hash(&mut h);
    widgets.right_divider_col.hash(&mut h);
    h.finish()
}

fn header_hash(app: &DashApp) -> u64 {
    let mut h = DefaultHasher::new();
    app.game_data_revision.hash(&mut h);
    app.player_record_index_1_based.hash(&mut h);
    h.finish()
}

fn footer_hash(app: &DashApp) -> u64 {
    let mut h = DefaultHasher::new();
    app.crosshair_x.hash(&mut h);
    app.crosshair_y.hash(&mut h);
    app.map_coord_input.hash(&mut h);
    h.finish()
}

fn dynamic_layer_active(app: &DashApp) -> bool {
    app.overlay != ActiveOverlay::None
        || app.popup != ActivePopup::None
        || app.overlay_position.is_some()
        || app.popup_position.is_some()
        || app.help_return_overlay_position.is_some()
        || app.mouse_gesture != crate::dashboard::app::state::ActiveMouseGesture::None
}

fn clear_horizontal_bar(
    buf: &mut PlayfieldBuffer,
    row: usize,
    dashboard: &layout::DashboardLayout,
    canvas: crate::dashboard::geometry::ScreenGeometry,
) {
    let (ox, _) = layout::frame_offset_for(canvas, dashboard.frame);
    let start_col = ox.saturating_add(1);
    let width = dashboard.frame.width().saturating_sub(2);
    buf.fill_rect(row, start_col, width, 1, theme::body_style());
}

fn frame_bar_rect(
    row: usize,
    dashboard: &layout::DashboardLayout,
    canvas: crate::dashboard::geometry::ScreenGeometry,
) -> DirtyRect {
    let (ox, _) = layout::frame_offset_for(canvas, dashboard.frame);
    DirtyRect {
        row,
        col: ox.saturating_add(1),
        width: dashboard.frame.width().saturating_sub(2),
        height: 1,
    }
}

fn dashboard_interior_rect(widgets: crate::dashboard::layout::DashboardWidgetFrames) -> DirtyRect {
    let rect = crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(widgets);
    DirtyRect {
        row: rect.y as usize,
        col: rect.x as usize,
        width: rect.width as usize,
        height: rect.height as usize,
    }
}

impl DirtyRect {
    fn from_widget(rect: WidgetRect) -> Self {
        Self {
            row: rect.row,
            col: rect.col,
            width: rect.width,
            height: rect.height,
        }
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
