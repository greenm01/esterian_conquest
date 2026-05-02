//! ? overlay: keyboard reference, centered on screen.

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::table::TableFooter;

use crate::dashboard::app::state::{ActiveOverlay, DashApp, HelpContext};
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{Rect, format_help_rows, wrap_formatted_help_lines};
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    max_overlay_body_height_in_parent, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::dashboard::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let lines = help_lines(app.help_context);
    let wrapped = wrap_formatted_help_lines(&lines, max_overlay_body_width(map_frame));
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_height = wrapped
        .lines
        .len()
        .min(max_overlay_body_height_in_parent(parent, TableFooter::None));
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        "HELP",
        wrapped.content_width,
        body_height,
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.overlay_position_for(ActiveOverlay::Help),
    );
    assert_overlay_body_write_fits(frame, "HELP", wrapped.content_width, body_height);

    for (idx, line) in wrapped.lines.iter().enumerate().take(frame.body_height) {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            theme::label_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let lines = help_lines(app.help_context);
    let wrapped = wrap_formatted_help_lines(&lines, max_overlay_body_width(map_frame));
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_height = wrapped
        .lines
        .len()
        .min(max_overlay_body_height_in_parent(parent, TableFooter::None));
    overlay_popup_rect_for_body_in_parent(
        parent,
        "HELP",
        wrapped.content_width,
        body_height,
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.overlay_position_for(ActiveOverlay::Help),
    )
}

fn help_lines(context: HelpContext) -> Vec<String> {
    format_help_rows(match context {
        HelpContext::Global => vec![
            ("P", "Open Planet List"),
            ("F", "Open Fleet List"),
            ("T", "Open Total Planet Database"),
            ("R", "Open Inbox"),
            ("D", "Open Diplomacy"),
            ("Alt-Q", "Return to lobby"),
            ("?", "Open this helper"),
            ("Tab", "Cycle dashboard focus"),
            ("Shift+Tab", "Cycle dashboard focus backward"),
            ("XX,YY", "Jump crosshair to real map coordinates"),
            ("[", "Jump to the previous planet on the map"),
            ("]", "Jump to the next planet on the map"),
            ("+", "Zoom the map in"),
            ("-", "Zoom the map out"),
            ("Z", "Reset the map zoom for the current view mode"),
            (
                "Viewport",
                "Small terminals auto-clip the map around the crosshair",
            ),
            ("Mouse", "Hovering over the map moves the crosshair"),
            ("Left Click", "Open player fleets at that sector, if any"),
            (
                "Right Click",
                "Open owned-planet commands or planet info for the clicked world",
            ),
            (
                "Map Exit",
                "Leaving the map widget resets the crosshair home",
            ),
            ("E:Pot|Curr|Pts", "Potential, current, and stored points"),
            ("D:AR|GB|SB", "Armies, ground batteries, and starbases"),
        ],
        HelpContext::OwnedPlanetPopup => vec![
            ("B", "Specify new build orders for this planet"),
            ("C", "Commission a completed stardock slot"),
            ("M", "Automatically commission all completed stardock slots"),
            ("L", "Load armies from this planet onto a fleet in orbit"),
            ("U", "Unload armies from a fleet in orbit onto this planet"),
            ("X", "Stage a scorch order for this planet"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetList => vec![
            ("Enter", "Open status for the selected planet"),
            ("F", "Open the planet-list filter prompt"),
            ("S", "Open the planet-list sort menu"),
            ("B", "Specify new build orders for the selected planet"),
            ("C", "Commission a completed stardock slot"),
            ("M", "Automatically commission all completed stardock slots"),
            ("L", "Load armies from this planet onto a fleet in orbit"),
            ("U", "Unload armies from a fleet in orbit onto this planet"),
            ("X", "Stage a scorch order for this planet"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            (
                "Codes",
                "coo pla max cur trs bdg rev gro bui sta sbs ars gbs",
            ),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            (
                "Codes",
                "coo pla max cur trs bdg rev gro bui sta sbs ars gbs",
            ),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Coords", "coo accepts xx,yy or xx,yy/r"),
            ("Value", "Text contains; numbers accept > >= < <= = !="),
            ("all", "Clear the current filter"),
            ("?", "Open this helper"),
        ],
        HelpContext::PromptInput => vec![
            ("Type", "Enter the value shown on the command line"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetBuildSpecify => vec![
            ("Type", "Enter a unit number to jump/highlight it"),
            ("?", "Open this helper"),
            ("+", "Queue one unit of the highlighted type"),
            ("-", "Remove one queued unit of the highlighted type"),
            ("D", "Clear queued builds for the highlighted unit type"),
        ],
        HelpContext::PlanetBuildQuantity => vec![
            ("Type", "Enter the quantity to queue"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetList => vec![
            ("Enter", "Open review for the selected fleet"),
            ("F", "Open the fleet-list filter prompt"),
            ("S", "Open the fleet-list sort menu"),
            ("SPACE", "Toggle the checked state of the current fleet row"),
            ("O", "Assign fleet/starbase orders"),
            ("C", "Change ROE, ID, or speed"),
            ("M", "Merge fleets"),
            ("T", "Transfer ships"),
            (
                "Fleet / SB ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "id sel loc ord tar spd eta roe ars shi"),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "id sel loc ord tar spd eta roe ars shi"),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Order", "ord also accepts holding, moving, and combat"),
            ("Selected", "sel accepts yes/no, selected, unselected, or x"),
            ("all", "Clear the current filter"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetMissionPicker => vec![
            ("Type", "Enter a mission number from 0 to 15"),
            ("Up/Down", "Move between enabled missions"),
            ("PgUp/PgDn", "Page through the mission list"),
            (
                "Filter",
                "Only missions valid for all selected fleets stay enabled",
            ),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetOrderInput => vec![
            ("Type", "Enter the requested target or confirm input"),
            ("?", "Open this helper"),
        ],
        HelpContext::StarbaseMove => vec![
            ("M", "Move the selected starbase"),
            ("H", "Halt the selected starbase"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabase => vec![
            ("F", "Open the database filter prompt"),
            ("S", "Open the database sort menu"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
            ("rng", "Sort by range from a sector"),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
            (
                "Prefix",
                "Ambiguous prefixes stay open and show matching codes",
            ),
            ("Coords", "coo accepts xx,yy or xx,yy/r"),
            ("Unknown", "Use ? for unknown database values"),
            ("Value", "Text contains; numbers accept > >= < <= = !="),
            ("all", "Clear the current filter"),
            ("?", "Open this helper"),
        ],
        HelpContext::Inbox => vec![
            ("M", "Filter to messages"),
            ("R", "Filter to reports"),
            ("A", "Filter to all items"),
            ("Y", "Toggle the current-year filter"),
            ("D", "Delete the selected item"),
            ("Tab", "Switch list and preview focus"),
            (
                "Visible ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("?", "Open this helper"),
        ],
        HelpContext::Diplomacy => vec![
            ("E", "Mark the selected empire as Enemy"),
            ("N", "Mark the selected empire as Neutral"),
            ("?", "Open this helper"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::{draw, help_lines, popup_rect};
    use crate::dashboard::app::state::{ActiveOverlay, DashApp, HelpContext};
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::layout::dashboard::dashboard_layout;

    #[test]
    fn fleet_help_mentions_typed_jump_and_real_actions() {
        let lines = help_lines(HelpContext::FleetList);

        assert!(lines.iter().any(|line| line.contains("Typed jump")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("O") && line.contains("Assign fleet/starbase orders"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("SPACE") && line.contains("checked state"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("C") && line.contains("Change ROE, ID, or speed"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("M") && line.contains("Merge fleets"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("T") && line.contains("Transfer ships"))
        );
        assert!(!lines.iter().any(|line| line.contains("checked fleets")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Enter") && line.contains("selected fleet"))
        );
        assert!(!lines.iter().any(|line| line.contains("O / C / M / T")));
        assert!(!lines.iter().any(|line| line.contains("TODO")));
        assert!(!lines.iter().any(|line| line.contains("FLEET LIST")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
        assert!(!lines.iter().any(|line| line.contains("PgUp")));
    }

    #[test]
    fn overlay_help_omits_stale_browse_commands() {
        let planet = help_lines(HelpContext::PlanetList);
        assert!(
            planet
                .iter()
                .any(|line| line.contains("B") && line.contains("build orders"))
        );
        assert!(!planet.iter().any(|line| line.contains("TODO")));
        assert!(
            planet
                .iter()
                .any(|line| line.contains("Enter") && line.contains("selected planet"))
        );

        let intel = help_lines(HelpContext::IntelDatabase);
        assert!(intel.iter().any(|line| line.contains("Coords")));
        assert!(!intel.iter().any(|line| line.contains("TODO")));
        assert!(!intel.iter().any(|line| line.contains("Enter")));

        let inbox = help_lines(HelpContext::Inbox);
        assert!(inbox.iter().any(|line| line.contains("Tab")));
        assert!(!inbox.iter().any(|line| line.contains("TODO")));
        assert!(!inbox.iter().any(|line| line.contains("Enter")));

        let diplomacy = help_lines(HelpContext::Diplomacy);
        assert!(!diplomacy.iter().any(|line| line.contains("Rows")));
        assert!(
            diplomacy
                .iter()
                .any(|line| line.contains("E") && line.contains("Enemy"))
        );
        assert!(
            diplomacy
                .iter()
                .any(|line| line.contains("N") && line.contains("Neutral"))
        );
        assert!(!diplomacy.iter().any(|line| line.contains("TODO")));
    }

    #[test]
    fn global_help_keeps_dashboard_overview() {
        let lines = help_lines(HelpContext::Global);

        assert!(!lines.iter().any(|line| line.contains("GLOBAL HOTKEYS")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("P") && line.contains("Open Planet List"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("[") && line.contains("previous planet"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("XX,YY") && line.contains("map coordinates"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("+") && line.contains("Zoom the map in"))
        );
        assert!(!lines.iter().any(|line| line.contains("Open Settings")));
        assert!(!lines.iter().any(|line| line.contains("fill map view")));
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("toggled in Settings"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Mouse") && line.contains("moves the crosshair"))
        );
        assert!(lines.iter().any(|line| line.contains("Left Click")));
        assert!(lines.iter().any(|line| line.contains("Right Click")));
        assert!(lines.iter().any(|line| line.contains("Map Exit")));
        assert!(lines.iter().any(|line| line.contains("Potential, current")));
        assert!(!lines.iter().any(|line| line.contains("P / F / I / R")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
    }

    #[test]
    fn help_overlay_omits_implied_navigation_rows() {
        for context in [
            HelpContext::Global,
            HelpContext::OwnedPlanetPopup,
            HelpContext::PlanetList,
            HelpContext::PlanetListSort,
            HelpContext::PlanetListFilter,
            HelpContext::PromptInput,
            HelpContext::PlanetBuildSpecify,
            HelpContext::PlanetBuildQuantity,
            HelpContext::FleetList,
            HelpContext::FleetListSort,
            HelpContext::FleetListFilter,
            HelpContext::FleetMissionPicker,
            HelpContext::FleetOrderInput,
            HelpContext::StarbaseMove,
            HelpContext::IntelDatabase,
            HelpContext::IntelDatabaseSort,
            HelpContext::IntelDatabaseFilter,
            HelpContext::Inbox,
            HelpContext::Diplomacy,
        ] {
            let lines = help_lines(context);
            assert!(!lines.iter().any(|line| line.starts_with("Arrows")));
            assert!(!lines.iter().any(|line| line.starts_with("H J K L")));
            if !matches!(context, HelpContext::PlanetList | HelpContext::FleetList) {
                assert!(!lines.iter().any(|line| line.starts_with("Enter")));
            }
            assert!(!lines.iter().any(|line| line.starts_with("Esc")));
        }
    }

    #[test]
    fn help_overlay_clamps_to_available_dashboard_height() {
        let mut app =
            DashApp::new_for_repro(ScreenGeometry::new(120, 40), ScreenGeometry::new(108, 26));
        app.overlay = ActiveOverlay::Help;
        app.help_context = HelpContext::Global;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let mut buffer = app.render_playfield().expect("playfield");

        draw(&mut buffer, &app, map_frame);

        let rect = popup_rect(&app, map_frame);
        assert!(rect.height > 0);
    }

    #[test]
    fn help_overlay_has_no_dismiss_footer() {
        let mut app =
            DashApp::new_for_repro(ScreenGeometry::new(120, 40), ScreenGeometry::new(108, 26));
        app.overlay = ActiveOverlay::Help;
        app.help_context = HelpContext::Global;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let mut buffer = app.render_playfield().expect("playfield");

        draw(&mut buffer, &app, map_frame);

        let rect = popup_rect(&app, map_frame);
        let left = rect.x as usize;
        let width = rect.width as usize;
        let bottom_inner_row = rect.y as usize + rect.height as usize - 2;
        let popup_text = |row: usize| {
            buffer.row(row)[left..left + width]
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>()
        };

        for row in rect.y as usize..rect.y as usize + rect.height as usize {
            assert!(!popup_text(row).contains("(slap a key)"));
        }

        let bottom_inner_line = popup_text(bottom_inner_row);
        assert!(!bottom_inner_line.contains("├"));
        assert!(!bottom_inner_line.contains("┤"));
    }
}
