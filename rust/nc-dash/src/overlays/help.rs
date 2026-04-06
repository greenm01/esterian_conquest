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
                ("Tab / Shift+Tab", "Cycle dashboard focus"),
                ("Esc / Q", "Close overlay or quit dashboard"),
            ])
            .join("\n"),
            String::new(),
            String::from("MAP AND LISTS"),
            String::new(),
            format_help_rows([
                ("J/K  Up/Down", "Move selection or crosshair"),
                ("^U /^D PgUp/PgDn", "Page through long lists"),
                ("Home / End", "Jump to start or end"),
                ("Tab", "Switch inbox list/preview focus"),
                ("M / R / A / Y", "Inbox filters and year scope"),
            ])
            .join("\n"),
        ],
        HelpContext::PlanetList => overlay_help_blocks(
            "PLANET LIST",
            "Type coords to jump; exact match clears the footer input.",
            &["B A C L U X S I T are TODO in nc-dash overlay."],
        ),
        HelpContext::FleetList => overlay_help_blocks(
            "FLEET LIST",
            "Type fleet or SB ID to jump; exact match clears the footer input.",
            &["O C M T I are TODO in nc-dash overlay."],
        ),
        HelpContext::IntelDatabase => overlay_help_blocks(
            "TOTAL PLANET DATABASE",
            "Type coords to jump; exact match clears the footer input.",
            &["S and I are TODO in nc-dash overlay."],
        ),
        HelpContext::Inbox => overlay_help_blocks(
            "INBOX",
            "Type visible ID to jump; exact match clears the footer input.",
            &["C is TODO in nc-dash overlay."],
        ),
        HelpContext::Diplomacy => overlay_help_blocks(
            "DIPLOMACY",
            "J/K and ^U/^D scroll the standings list.",
            &["D, S, and I are TODO in nc-dash overlay."],
        ),
        HelpContext::Settings => overlay_help_blocks(
            "SETTINGS",
            "This screen is informational for now.",
            &["Theme and mouse actions are TODO in nc-dash overlay."],
        ),
    };

    blocks
        .iter()
        .flat_map(|block| block.lines().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

fn overlay_help_blocks(title: &str, jump_line: &str, todo_lines: &[&str]) -> Vec<String> {
    let mut blocks = vec![
        title.to_string(),
        String::new(),
        format_help_rows([
            ("J/K  Up/Down", "Move selection"),
            ("^U /^D PgUp/PgDn", "Page through the list"),
            ("Home / End", "Jump to start or end"),
            ("Q / Esc", "Close this overlay"),
            ("?", "Show this helper"),
        ])
        .join("\n"),
        String::new(),
        String::from("TYPED JUMP"),
        String::new(),
        jump_line.to_string(),
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

        assert!(lines.iter().any(|line| line.contains("Type fleet or SB ID to jump")));
        assert!(lines.iter().any(|line| line.contains("TODO")));
        assert!(lines.iter().any(|line| line.contains("O C M T I are TODO")));
    }

    #[test]
    fn global_help_keeps_dashboard_overview() {
        let lines = help_lines(HelpContext::Global);

        assert!(lines.iter().any(|line| line.contains("GLOBAL HOTKEYS")));
        assert!(lines.iter().any(|line| line.contains("Planet, Fleet, Intel, Inbox overlays")));
    }
}
