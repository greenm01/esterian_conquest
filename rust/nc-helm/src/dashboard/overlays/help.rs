use crate::dashboard::app::state::{ActiveOverlay, DashApp, HelpContext};
use crate::dashboard::modal;

pub fn draw(
    buf: &mut crate::dashboard::buffer::PlayfieldBuffer,
    app: &DashApp,
    context: HelpContext,
) {
    let rows = help_lines(context);
    let title = match context {
        HelpContext::Global => "DASHBOARD HELP",
        HelpContext::OwnedPlanetPopup => "OWNED PLANET COMMANDS",
        HelpContext::PlanetList => "PLANET LIST COMMANDS",
        HelpContext::PlanetListSort => "PLANET SORTING",
        HelpContext::PlanetListFilter => "PLANET FILTERING",
        HelpContext::PlanetBuildSpecify => "CONSTRUCTION OPTIONS",
        HelpContext::PlanetBuildQuantity => "QUANTITY INPUT",
        HelpContext::PromptInput => "VALUE INPUT",
        HelpContext::FleetList => "FLEET LIST COMMANDS",
        HelpContext::FleetListSort => "FLEET SORTING",
        HelpContext::FleetListFilter => "FLEET FILTERING",
        HelpContext::FleetMissionPicker => "FLEET MISSIONS",
        HelpContext::FleetOrderInput => "ORDER INPUT",
        HelpContext::StarbaseMove => "STARBASE RELOCATION",
        HelpContext::IntelDatabase => "INTEL DATABASE COMMANDS",
        HelpContext::IntelDatabaseSort => "INTEL SORTING",
        HelpContext::IntelDatabaseFilter => "INTEL FILTERING",
        HelpContext::Inbox => "INBOX COMMANDS",
        HelpContext::InboxCompose => "MESSAGE EDITOR COMMANDS",
        HelpContext::Diplomacy => "DIPLOMACY COMMANDS",
    };

    let parent = crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(
        crate::dashboard::layout::dashboard::dashboard_layout(app).widgets,
    );
    let max_body_width = modal::max_content_width(parent);

    let metrics = modal::measure_modal_text_lines(&rows, max_body_width);
    let body_width = metrics.content_width;
    let body_height = metrics.lines.len();
    let frame = crate::dashboard::overlays::frame::draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        title,
        body_width,
        body_height,
        crate::dashboard::overlays::frame::OverlaySizePolicy::default(),
        crate::dashboard::table::TableFooter::None,
        app.overlay_position_for(ActiveOverlay::Help),
    );

    for (idx, row) in metrics.lines.iter().enumerate() {
        buf.write_text(
            frame.body_row + idx,
            frame.body_col,
            row,
            crate::dashboard::theme::body_style(),
        );
    }
}

pub fn popup_rect(
    app: &DashApp,
    _map_frame: crate::dashboard::layout::MapWidgetFrame,
) -> crate::dashboard::modal::Rect {
    let rows = help_lines(app.help_context);
    let title = "HELP";
    let parent = crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(
        crate::dashboard::layout::dashboard::dashboard_layout(app).widgets,
    );
    let max_body_width = modal::max_content_width(parent);
    let metrics = modal::measure_modal_text_lines(&rows, max_body_width);
    let body_width = metrics.content_width;
    let body_height = metrics.lines.len();
    crate::dashboard::overlays::frame::overlay_popup_rect_for_body_in_parent(
        parent,
        title,
        body_width,
        body_height,
        crate::dashboard::overlays::frame::OverlaySizePolicy::default(),
        crate::dashboard::table::TableFooter::None,
        app.overlay_position_for(ActiveOverlay::Help),
    )
}

fn help_lines(context: HelpContext) -> Vec<String> {
    format_help_rows(match context {
        HelpContext::Global => vec![
            ("P", "Open Planet List"),
            ("F", "Open Fleet List"),
            ("T", "Open Total Planet Database"),
            ("I", "Open Inbox"),
            ("D", "Open Diplomacy"),
            ("Alt-Q", "Return to lobby"),
            ("?", "Open this helper"),
            ("Tab", "Cycle dashboard focus"),
            ("Shift+Tab", "Cycle dashboard focus backward"),
            ("Left Click", "Select map sector or dashboard widget"),
            (
                "Right Click",
                "Open owned-planet commands or planet info for the clicked world",
            ),
            (
                "Map Exit",
                "Leaving the map widget resets the crosshair home",
            ),
        ],
        HelpContext::OwnedPlanetPopup => vec![
            ("B", "Specify build orders"),
            ("C", "Commission a completed stardock slot"),
            ("M", "Mass construction..."),
            ("T", "Transport units..."),
            ("L", "Land docked fleets"),
            ("U", "Unload all cargo"),
            ("X", "Scorch planet"),
            ("Esc", "Close popup"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetList => vec![
            ("S", "Sort planets"),
            ("B", "Filter planets"),
            ("C", "Filter to current-year construction"),
            ("A", "Clear all filters"),
            ("L", "Jump crosshair to planet"),
            ("U", "Unload cargo (at planet)"),
            ("X", "Scorch planet"),
            ("Esc", "Close overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListSort => vec![
            ("I", "Sort by ID"),
            ("N", "Sort by Name"),
            ("E", "Sort by Empire"),
            ("C", "Sort by Class"),
            ("P", "Sort by Population"),
            ("F", "Sort by Factories"),
            ("R", "Sort by Resources"),
            ("D", "Sort by Defense"),
            ("T", "Sort by Treasury"),
            ("Esc", "Cancel sorting"),
            ("?", "Open this helper"),
        ],
        HelpContext::PlanetListFilter => vec![
            ("N", "Filter by Name"),
            ("E", "Filter by Empire"),
            ("C", "Filter by Class"),
            ("P", "Filter by Population"),
            ("F", "Filter by Factories"),
            ("R", "Filter by Resources"),
            ("D", "Filter by Defense"),
            ("T", "Filter by Treasury"),
            ("Esc", "Cancel filtering"),
            ("?", "Open this helper"),
        ],
        HelpContext::PromptInput => {
            vec![("Esc", "Cancel input"), ("Enter", "Accept value / default")]
        }
        HelpContext::PlanetBuildSpecify => {
            vec![("Esc", "Cancel build specify"), ("?", "Open this helper")]
        }
        HelpContext::PlanetBuildQuantity => vec![
            ("Esc", "Cancel build quantity"),
            ("Enter", "Accept quantity / default"),
        ],
        HelpContext::FleetList => vec![
            ("S", "Sort fleets"),
            ("B", "Filter fleets"),
            ("C", "Change rules of engagement"),
            ("E", "Expand/Collapse all fleet components"),
            ("D", "Dissolve/Merge selected fleets"),
            ("M", "Move selected fleets"),
            ("T", "Transfer ships/cargo"),
            ("L", "Load all units"),
            ("U", "Unload all units"),
            ("Esc", "Close overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListSort => vec![
            ("I", "Sort by ID"),
            ("E", "Sort by Empire"),
            ("S", "Sort by Ships"),
            ("O", "Sort by Orders"),
            ("R", "Sort by ROE"),
            ("M", "Sort by Speed"),
            ("L", "Sort by Location"),
            ("Esc", "Cancel sorting"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetListFilter => vec![
            ("I", "Filter by ID"),
            ("E", "Filter by Empire"),
            ("S", "Filter by Ships"),
            ("R", "Filter by ROE"),
            ("M", "Filter by Speed"),
            ("L", "Filter by Location"),
            ("Esc", "Cancel filtering"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetMissionPicker => vec![
            ("Esc", "Cancel mission selection"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetOrderInput => vec![
            ("Esc", "Cancel order input"),
            ("Enter", "Accept coordinate / default"),
        ],
        HelpContext::StarbaseMove => vec![
            ("Esc", "Cancel starbase relocation"),
            ("Enter", "Accept destination / default"),
        ],
        HelpContext::IntelDatabase => vec![
            ("S", "Sort intel"),
            ("B", "Filter intel"),
            ("A", "Clear all filters"),
            ("L", "Jump crosshair to planet"),
            ("Esc", "Close overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseSort => vec![
            ("I", "Sort by ID"),
            ("N", "Sort by Name"),
            ("E", "Sort by Empire"),
            ("C", "Sort by Class"),
            ("P", "Sort by Population"),
            ("F", "Sort by Factories"),
            ("R", "Sort by Resources"),
            ("D", "Sort by Defense"),
            ("K", "Sort by Range"),
            ("Esc", "Cancel sorting"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabaseFilter => vec![
            ("N", "Filter by Name"),
            ("E", "Filter by Empire"),
            ("C", "Filter by Class"),
            ("P", "Filter by Population"),
            ("F", "Filter by Factories"),
            ("R", "Filter by Resources"),
            ("D", "Filter by Defense"),
            ("K", "Filter by Range"),
            ("T", "Filter by Intel Tier"),
            ("Y", "Filter by Intel Year"),
            ("Unknown", "Use ? for unknown database values"),
            ("Value", "Text contains; numbers accept > >= < <= = !="),
            ("all", "Clear the current filter"),
            ("?", "Open this helper"),
        ],
        HelpContext::Inbox => vec![
            ("C", "Compose message"),
            ("O", "Open Outbox"),
            ("M", "Filter to messages"),
            ("R", "Filter to reports"),
            ("A", "Filter to all items"),
            ("Y", "Toggle the current-year filter"),
            ("D", "Delete the selected item"),
            ("Tab", "Switch list and preview focus"),
            ("Ctrl-E", "Send composed message"),
            ("Ctrl-X", "Discard composed message"),
            (
                "Visible ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("?", "Open this helper"),
        ],
        HelpContext::InboxCompose => vec![
            ("Ctrl-E", "Finish and send message"),
            ("Ctrl-X", "Discard message"),
            ("Esc", "Close prompt"),
            ("?", "Open this helper"),
        ],
        HelpContext::Diplomacy => vec![
            ("E", "Mark the selected empire as Enemy"),
            ("N", "Mark the selected empire as Neutral"),
            ("?", "Open this helper"),
        ],
    })
}

fn format_help_rows(rows: Vec<(&str, &str)>) -> Vec<String> {
    let key_width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    rows.into_iter()
        .map(|(k, v)| format!("{:<width$} : {}", k, v, width = key_width))
        .collect()
}
