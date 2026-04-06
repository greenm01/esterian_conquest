//! ? overlay: keyboard reference, centered on screen.

use nc_ui::modal::{format_help_rows, wrap_formatted_help_lines};
use nc_ui::table::TableFooter;
use nc_ui::PlayfieldBuffer;

use crate::app::state::{DashApp, HelpContext};
use crate::overlays::frame::{draw_overlay_frame_for_body, write_clipped};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let lines = help_lines(app.help_context);
    let wrapped = wrap_formatted_help_lines(&lines, buf.width().saturating_sub(6));
    let frame = draw_overlay_frame_for_body(
        buf,
        "HELP",
        wrapped.content_width,
        wrapped.lines.len(),
        TableFooter::Dismiss,
    );

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
            ("[", "Jump to the previous planet on the map"),
            ("]", "Jump to the next planet on the map"),
            ("E:Pot|Curr|Pts", "Potential, current, and stored points"),
            ("D:AR|GB|SB", "Armies, ground batteries, and starbases"),
        ],
        HelpContext::PlanetList => vec![
            ("B", "TODO - build queue for the selected planet"),
            ("A", "TODO - auto-commission ships from stardock"),
            ("C", "TODO - commission ships into a fleet"),
            ("L", "TODO - load armies onto transports"),
            ("U", "TODO - unload armies from transports"),
            ("X", "TODO - scorch the selected planet"),
            ("S", "TODO - sort the planet list"),
            ("I", "TODO - review the highlighted planet"),
            ("Enter", "TODO - review the highlighted planet"),
            ("T", "TODO - transfer focus to a typed target"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::FleetList => vec![
            ("O", "TODO - assign orders to the selected fleet"),
            ("C", "TODO - change the selected fleet"),
            ("M", "TODO - merge the selected fleet"),
            ("T", "TODO - transfer ships between fleets"),
            ("I", "TODO - review the highlighted fleet"),
            ("Enter", "TODO - review the highlighted fleet"),
            (
                "Fleet / SB ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::IntelDatabase => vec![
            ("S", "TODO - sort the database"),
            ("I", "TODO - inspect the highlighted world"),
            ("Enter", "TODO - inspect the highlighted world"),
            ("Coords", "Typed jump; exact match clears the footer input"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::Inbox => vec![
            ("M", "Filter to messages"),
            ("R", "Filter to reports"),
            ("A", "Filter to all items"),
            ("Y", "Toggle the current-year filter"),
            ("D", "Delete the selected item"),
            ("C", "TODO - compose a message"),
            ("Tab", "Switch list and preview focus"),
            ("Enter", "Toggle preview focus when the jump field is empty"),
            (
                "Visible ID",
                "Typed jump; exact match clears the footer input",
            ),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::Diplomacy => vec![
            ("D", "TODO - declare enemy or neutral"),
            ("S", "TODO - sort the standings"),
            ("I", "TODO - inspect the highlighted empire"),
            ("Enter", "TODO - inspect the highlighted empire"),
            ("Q", "Close this overlay"),
            ("Esc", "Close this overlay"),
            ("?", "Open this helper"),
        ],
        HelpContext::Settings => vec![
            ("Theme", "TODO - choose a dashboard theme"),
            ("Mouse", "TODO - configure mouse actions"),
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
    fn fleet_help_mentions_typed_jump_and_todo_actions() {
        let lines = help_lines(HelpContext::FleetList);

        assert!(lines.iter().any(|line| line.contains("Typed jump")));
        assert!(lines
            .iter()
            .any(|line| line.contains("O") && line.contains("assign orders")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Enter") && line.contains("review the highlighted fleet")));
        assert!(!lines.iter().any(|line| line.contains("O / C / M / T")));
        assert!(!lines.iter().any(|line| line.contains("FLEET LIST")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
        assert!(!lines.iter().any(|line| line.contains("PgUp")));
    }

    #[test]
    fn global_help_keeps_dashboard_overview() {
        let lines = help_lines(HelpContext::Global);

        assert!(!lines.iter().any(|line| line.contains("GLOBAL HOTKEYS")));
        assert!(lines
            .iter()
            .any(|line| line.contains("P") && line.contains("Open Planet List")));
        assert!(lines
            .iter()
            .any(|line| line.contains("[") && line.contains("previous planet")));
        assert!(lines.iter().any(|line| line.contains("Potential, current")));
        assert!(!lines.iter().any(|line| line.contains("P / F / I / R")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
    }
}
