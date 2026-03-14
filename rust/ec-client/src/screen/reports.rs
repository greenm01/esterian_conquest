use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::model::ReviewSummary;
use crate::reports::ReportsPreview;
use crate::screen::layout::{draw_command_prompt, draw_status_line, draw_title_bar, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
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
}

impl Screen for ReportsScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
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
            self.summary.reviewable_messages,
            &self.preview.message_lines,
        )?;
        row += 1;
        draw_command_prompt(&mut buffer, row, "GENERAL COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}

fn write_section(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
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
        buffer.write_text(start_row, 0, "  <none>", classic::body_style());
        return Ok(1);
    }

    let mut written = 0;
    for line in lines.iter().take(10) {
        buffer.write_text(
            start_row + written,
            0,
            &format!("  {line}"),
            classic::body_style(),
        );
        written += 1;
    }
    if lines.len() > 10 {
        buffer.write_text(
            start_row + written,
            0,
            &format!("  ... {} more line(s)", lines.len() - 10),
            classic::body_style(),
        );
        written += 1;
    }
    Ok(written)
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
}
