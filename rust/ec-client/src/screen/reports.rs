use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::model::ReviewSummary;
use crate::reports::ReportsPreview;
use crate::screen::layout::{
    PLAYFIELD_WIDTH, draw_command_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame, command_menu_label};
use crate::theme::classic;

pub struct ReportsScreen {
    preview: ReportsPreview,
    summary: ReviewSummary,
}

impl ReportsScreen {
    pub fn new(preview: ReportsPreview, summary: ReviewSummary) -> Self {
        Self { preview, summary }
    }

    pub fn replace(&mut self, preview: ReportsPreview, summary: ReviewSummary) {
        self.preview = preview;
        self.summary = summary;
    }

    pub fn render_with_menu(
        &mut self,
        frame: &ScreenFrame<'_>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let mut row = 0;
        draw_title_bar(&mut buffer, row, "MESSAGES / RESULTS REVIEW: ");
        row += 2;
        draw_status_line(
            &mut buffer,
            row,
            "Player: ",
            &format!(
                "{}  Empire: {}",
                frame.player.record_index_1_based,
                display_or_unknown(&frame.player.empire_name)
            ),
        );
        row += 2;
        buffer.write_text(row, 0, "REPORTS", classic::status_value_style());
        row += 1;
        buffer.write_text(row, 0, "-------", classic::status_label_style());
        row += 1;
        row += write_section(
            &mut buffer,
            row,
            "results",
            self.summary.reviewable_results,
            &self.preview.results_lines,
        )?;
        row += 1;
        buffer.write_text(row, 0, "MESSAGES", classic::status_value_style());
        row += 1;
        buffer.write_text(row, 0, "--------", classic::status_label_style());
        row += 1;
        row += write_section(
            &mut buffer,
            row,
            "messages",
            self.summary.reviewable_messages,
            &self.preview.message_lines,
        )?;
        row += 1;
        draw_command_prompt(&mut buffer, row, command_menu_label(menu), "SLAP A KEY");
        Ok(buffer)
    }
}

impl Screen for ReportsScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_menu(frame, CommandMenu::General)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn write_section(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    section_name: &str,
    reviewable: bool,
    lines: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    if !reviewable {
        buffer.write_text(
            start_row,
            0,
            "  <none currently reviewable>",
            classic::body_style(),
        );
        return Ok(1);
    }

    if lines.is_empty() {
        let empty_notice = match section_name {
            "results" => "  <reports are marked pending, but no review text is available yet>",
            "messages" => "  <messages are marked pending, but no review text is available yet>",
            _ => "  <review items are marked pending, but no review text is available yet>",
        };
        buffer.write_text(
            start_row,
            0,
            empty_notice,
            classic::body_style(),
        );
        return Ok(1);
    }

    let mut written = 0;
    let wrapped_rows = review_rows(lines);
    for line in wrapped_rows.iter().take(10) {
        buffer.write_text(start_row + written, 0, line, classic::body_style());
        written += 1;
    }
    if wrapped_rows.len() > 10 {
        buffer.write_text(
            start_row + written,
            0,
            &format!(
                "  <... {} more line(s); use startup review for full suspense>",
                wrapped_rows.len() - 10
            ),
            classic::body_style(),
        );
        written += 1;
    }
    Ok(written)
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}

fn review_rows(lines: &[String]) -> Vec<String> {
    let mut rows = Vec::new();
    for line in lines {
        if line.is_empty() {
            rows.push("  ".to_string());
            continue;
        }
        rows.extend(
            wrap_review_text(line, PLAYFIELD_WIDTH.saturating_sub(2))
                .into_iter()
                .map(|wrapped| format!("  {wrapped}")),
        );
    }
    rows
}

fn wrap_review_text(text: &str, width: usize) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>();
    if normalized.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    let mut current = String::new();
    for word in normalized {
        let separator = if current.is_empty() { 0 } else { 1 };
        if current.len() + separator + word.len() > width && !current.is_empty() {
            rows.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }

        if word.len() > width && current.is_empty() {
            let mut remaining = word;
            while remaining.len() > width {
                rows.push(remaining[..width].to_string());
                remaining = &remaining[width..];
            }
            current.push_str(remaining);
        } else {
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    rows
}
