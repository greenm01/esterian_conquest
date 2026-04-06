//! ? overlay: keyboard reference, centered on screen.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::format_help_rows;
use nc_ui::table::TableFooter;

use crate::app::state::{DashApp, HelpContext};
use crate::overlays::frame::{draw_overlay_frame, write_clipped};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let lines = help_lines(app.help_context);
    let frame = draw_overlay_frame(
        buf,
        "HELP",
        76,
        lines.len() + 5,
        TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "? or <Q> to close ->",
        },
    );

    for (idx, line) in lines.iter().enumerate().take(frame.body_height) {
        let style = if line.chars().all(|ch| !ch.is_lowercase()) && !line.is_empty() {
            theme::section_title_style()
        } else {
            theme::label_style()
        };
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            style,
        );
    }
}

fn help_lines(context: HelpContext) -> Vec<String> {
    let blocks = match context {
        HelpContext::Global => vec![
            String::from("GLOBAL HOTKEYS"),
            String::new(),
            format_help_rows([
                ("P / F / I / R", "Planet, Fleet, Intel, Inbox overlays"),
                ("D / S / ?", "Diplomacy, Settings, Help"),
                ("Esc / Q", "Close overlay or quit dashboard"),
            ])
            .join("\n"),
            String::new(),
            String::from("DASHBOARD"),
            String::new(),
            format_help_rows([("Tab / Shift+Tab", "Cycle dashboard focus")]).join("\n"),
            String::new(),
            String::from("MAP"),
            String::new(),
            format_help_rows([
                ("Enter", "Open planet detail for the selected world"),
                ("[ / ]", "Jump to previous or next planet on the map"),
                ("E:Pot|Curr|Pts", "Potential, current, and stored points"),
                ("D:AR|GB|SB", "Armies, ground batteries, and starbases"),
            ])
            .join("\n"),
        ],
        HelpContext::PlanetList => overlay_help_blocks(
            "PLANET LIST",
            &[
                ("B / A / C", "Build, auto-commission, or commission"),
                ("L / U / X", "Load, unload, or scorch"),
                ("S", "Sort the planet list"),
                ("I / Enter", "Review highlighted planet"),
                ("Coords", "Typed jump; exact match clears the footer input"),
                ("Q / Esc", "Close this overlay"),
                ("?", "Show this helper"),
            ],
            &["B A C L U X S I T are TODO in nc-dash overlay."],
        ),
        HelpContext::FleetList => overlay_help_blocks(
            "FLEET LIST",
            &[
                ("O / C / M / T", "Order, change, merge, or transfer"),
                ("I / Enter", "Review highlighted fleet"),
                (
                    "Fleet / SB ID",
                    "Typed jump; exact match clears the footer input",
                ),
                ("Q / Esc", "Close this overlay"),
                ("?", "Show this helper"),
            ],
            &["O C M T I are TODO in nc-dash overlay."],
        ),
        HelpContext::IntelDatabase => overlay_help_blocks(
            "TOTAL PLANET DATABASE",
            &[
                ("S", "Sort the database"),
                ("I / Enter", "Inspect highlighted world"),
                ("Coords", "Typed jump; exact match clears the footer input"),
                ("Q / Esc", "Close this overlay"),
                ("?", "Show this helper"),
            ],
            &["S and I are TODO in nc-dash overlay."],
        ),
        HelpContext::Inbox => overlay_help_blocks(
            "INBOX",
            &[
                ("M / R / A", "Filter messages, reports, or all items"),
                ("Y", "Toggle current-year filter"),
                ("D", "Delete the selected item"),
                ("C", "Compose a message"),
                ("Tab", "Switch list and preview focus"),
                ("Enter", "Toggle preview focus when the jump field is empty"),
                (
                    "Visible ID",
                    "Typed jump; exact match clears the footer input",
                ),
                ("Q / Esc", "Close this overlay"),
                ("?", "Show this helper"),
            ],
            &["C is TODO in nc-dash overlay."],
        ),
        HelpContext::Diplomacy => overlay_help_blocks(
            "DIPLOMACY",
            &[
                ("D", "Declare enemy or neutral"),
                ("S", "Sort the standings"),
                ("I / Enter", "Inspect highlighted empire"),
                ("Q / Esc", "Close this overlay"),
                ("?", "Show this helper"),
            ],
            &["D, S, and I are TODO in nc-dash overlay."],
        ),
        HelpContext::Settings => overlay_help_blocks(
            "SETTINGS",
            &[("Q / Esc", "Close this overlay"), ("?", "Show this helper")],
            &["Theme and mouse actions are TODO in nc-dash overlay."],
        ),
    };

    blocks
        .iter()
        .flat_map(|block| block.lines().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

fn overlay_help_blocks(title: &str, rows: &[(&str, &str)], todo_lines: &[&str]) -> Vec<String> {
    let mut blocks = vec![
        title.to_string(),
        String::new(),
        format_help_rows(rows.iter().copied()).join("\n"),
    ];

    if !todo_lines.is_empty() {
        blocks.push(String::new());
        blocks.push(String::from("TODO"));
        blocks.push(String::new());
        blocks.extend(todo_lines.iter().map(|line| line.to_string()));
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::help_lines;
    use crate::app::state::HelpContext;

    #[test]
    fn fleet_help_mentions_typed_jump_and_todo_actions() {
        let lines = help_lines(HelpContext::FleetList);

        assert!(lines.iter().any(|line| line.contains("Typed jump")));
        assert!(lines.iter().any(|line| line.contains("TODO")));
        assert!(lines.iter().any(|line| line.contains("O C M T I are TODO")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
        assert!(!lines.iter().any(|line| line.contains("PgUp")));
    }

    #[test]
    fn global_help_keeps_dashboard_overview() {
        let lines = help_lines(HelpContext::Global);

        assert!(lines.iter().any(|line| line.contains("GLOBAL HOTKEYS")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Planet, Fleet, Intel, Inbox overlays"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Jump to previous or next planet"))
        );
        assert!(lines.iter().any(|line| line.contains("Potential, current")));
        assert!(!lines.iter().any(|line| line.contains("Up/Down")));
    }
}
