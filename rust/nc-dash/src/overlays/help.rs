//! ? overlay: keyboard reference, centered on screen.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::{Rect, format_help_rows, wrap_formatted_help_lines};
use nc_ui::table::TableFooter;

use crate::app::state::{ActiveOverlay, DashApp, HelpContext};
use crate::layout::MapWidgetFrame;
use crate::layout::dashboard;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let lines = help_lines(app.help_context);
    let wrapped = wrap_formatted_help_lines(&lines, max_overlay_body_width(map_frame));
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "HELP",
        wrapped.content_width,
        wrapped.lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Help),
    );
    assert_overlay_body_write_fits(frame, "HELP", wrapped.content_width, wrapped.lines.len());

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
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "HELP",
        wrapped.content_width,
        wrapped.lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Help),
    )
}

fn help_lines(context: HelpContext) -> Vec<String> {
    format_help_rows(match context {
        HelpContext::Global => vec![
            ("P", "Open Planet List"),
            ("F", "Open Fleet List"),
            ("I", "Open Total Planet Database"),
            ("R", "Open Inbox"),
            ("D", "Open Diplomacy"),
            ("S", "Open Settings"),
            ("?", "Open this helper"),
            ("Esc", "Close overlay or quit dashboard"),
            ("Q", "Close overlay or quit dashboard"),
            ("Tab", "Cycle dashboard focus"),
            ("Shift+Tab", "Cycle dashboard focus backward"),
            ("Enter", "Open planet detail for the selected world"),
            ("XX,YY", "Jump crosshair to real map coordinates"),
            ("[", "Jump to the previous planet on the map"),
            ("]", "Jump to the next planet on the map"),
            ("+", "Zoom the map in"),
            ("-", "Zoom the map out"),
            ("Z", "Reset the map zoom for the current view mode"),
            ("V", "Toggle readable and fill map view"),
            (
                "Viewport",
                "Small terminals auto-clip the map around the crosshair",
            ),
            ("Mouse", "Hover-follow can be toggled in Settings"),
            ("Left Click", "Open player fleets at that sector, if any"),
            ("Right Click", "Open planet list/info for the clicked world"),
            (
                "Map Exit",
                "Leaving the map widget resets home when hover-follow is on",
            ),
            ("E:Pot|Curr|Pts", "Potential, current, and stored points"),
            ("D:AR|GB|SB", "Armies, ground batteries, and starbases"),
        ],
        HelpContext::PlanetList => vec![
            ("F", "Open the planet-list filter prompt"),
            ("S", "Open the planet-list sort menu"),
            ("B", "Specify new build orders for the selected planet"),
            ("D", "Display queued build orders for the selected planet"),
            ("A", "Abort queued build orders for the selected planet"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla max cur trs bdg rev gro bui sta sbs ars gbs"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla max cur trs bdg rev gro bui sta sbs ars gbs"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Coords", "coo accepts xx,yy or xx,yy/r"),
            ("Value", "Text contains; numbers accept > >= < <= = !="),
            ("all", "Clear the current filter"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::PromptInput => vec![
            ("Type", "Enter the value shown on the command line"),
            ("Enter", "Accept the current prompt"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetBuildSpecify => vec![
            ("Type", "Enter a build-unit number"),
            ("0", "Return to the planet list"),
            ("Enter", "Accept the selected unit"),
            ("Q", "Return to the planet list"),
            ("Esc", "Return to the planet list"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetBuildQuantity => vec![
            ("Type", "Enter the quantity to queue"),
            ("Enter", "Queue that many units"),
            ("Q", "Return to unit selection"),
            ("Esc", "Return to unit selection"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetList => vec![
            ("F", "Open the fleet-list filter prompt"),
            ("S", "Open the fleet-list sort menu"),
            ("SPACE", "Toggle the checked state of the current fleet row"),
            ("O", "Order checked fleets, or the selected fleet/starbase"),
            ("C", "Change checked fleets, or the selected fleet"),
            ("M", "Merge checked fleets, or the selected fleet"),
            ("T", "Transfer ships between checked fleets, or the selected fleet"),
            (
                "Fleet / SB ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "id sel loc ord tar spd eta roe ars shi"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "id sel loc ord tar spd eta roe ars shi"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Order", "ord also accepts holding, moving, and combat"),
            ("Selected", "sel accepts yes/no, selected, unselected, or x"),
            ("all", "Clear the current filter"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetMissionPicker => vec![
            ("Type", "Enter a mission number from 0 to 15"),
            ("Enter", "Choose the current mission"),
            ("Up/Down", "Move between enabled missions"),
            ("PgUp/PgDn", "Page through the mission list"),
            (
                "Filter",
                "Only missions valid for all selected fleets stay enabled",
            ),
            ("Q", "Return to the fleet list"),
            ("Esc", "Return to the fleet list"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetOrderInput => vec![
            ("Type", "Enter the requested target or confirm input"),
            ("Enter", "Accept the current step"),
            ("Q", "Return to the previous order step"),
            ("Esc", "Return to the previous order step"),
            ("?", "Open this helper"),
        ],
        HelpContext::StarbaseMove => vec![
            ("M", "Move the selected starbase"),
            ("H", "Halt the selected starbase"),
            ("Enter", "Accept the current move step"),
            ("Q", "Return to the fleet list"),
            ("Esc", "Return to the fleet list"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabase => vec![
            ("F", "Open the database filter prompt"),
            ("S", "Open the database sort menu"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseSort => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
            ("rng", "Sort by range from a sector"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Repeat", "Same sort flips ASC/DESC"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseFilter => vec![
            ("Type", "Enter a column code or unique prefix, then Enter"),
            ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
            ("Prefix", "Ambiguous prefixes stay open and show matching codes"),
            ("Coords", "coo accepts xx,yy or xx,yy/r"),
            ("Unknown", "Use ? for unknown database values"),
            ("Value", "Text contains; numbers accept > >= < <= = !="),
            ("all", "Clear the current filter"),
            ("Q", "Return to the table"),
            ("Esc", "Return to the table"),
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
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::Diplomacy => vec![
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::Settings => vec![
            ("M", "Toggle hover-follow crosshair on the map"),
            ("G", "Toggle full dense map-grid dots"),
            ("Clicks", "Always move the crosshair and run sector actions"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::help_lines;
    use crate::app::state::HelpContext;

    #[test]
    fn fleet_help_mentions_typed_jump_and_real_actions() {
        let lines = help_lines(HelpContext::FleetList);

        assert!(lines.iter().any(|line| line.contains("Typed jump")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("O") && line.contains("Order checked fleets"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("SPACE") && line.contains("checked state"))
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
        assert!(!planet.iter().any(|line| line.contains("Enter")));

        let intel = help_lines(HelpContext::IntelDatabase);
        assert!(intel.iter().any(|line| line.contains("Coords")));
        assert!(!intel.iter().any(|line| line.contains("TODO")));
        assert!(!intel.iter().any(|line| line.contains("Enter")));

        let inbox = help_lines(HelpContext::Inbox);
        assert!(inbox.iter().any(|line| line.contains("Tab")));
        assert!(!inbox.iter().any(|line| line.contains("TODO")));
        assert!(!inbox.iter().any(|line| line.contains("Enter")));

        let diplomacy = help_lines(HelpContext::Diplomacy);
        assert_eq!(diplomacy.len(), 3);
        assert!(!diplomacy.iter().any(|line| line.contains("TODO")));

        let settings = help_lines(HelpContext::Settings);
        assert!(
            settings
                .iter()
                .any(|line| line.contains("hover-follow crosshair"))
        );
        assert!(!settings.iter().any(|line| line.contains("TODO")));
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
        assert!(
            lines
                .iter()
                .any(|line| line.contains("V") && line.contains("fill map view"))
        );
        assert!(lines.iter().any(|line| line.contains("Left Click")));
        assert!(lines.iter().any(|line| line.contains("Right Click")));
        assert!(lines.iter().any(|line| line.contains("Map Exit")));
        assert!(lines.iter().any(|line| line.contains("Potential, current")));
        assert!(!lines.iter().any(|line| line.contains("P / F / I / R")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
    }
}
