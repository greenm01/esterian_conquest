use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::model::ReviewSummary;
use crate::reports::{ReportsPreview, wrap_review_text_preserving_spacing};
use crate::screen::layout::{
    PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, draw_command_prompt, draw_status_line, draw_title_bar,
    new_playfield,
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
        let report_rows = section_rows(
            "results",
            self.summary.reviewable_results,
            &self.preview.results_lines,
        );
        let message_rows = section_rows(
            "messages",
            self.summary.reviewable_messages,
            &self.preview.message_lines,
        );
        let content_budget = PLAYFIELD_HEIGHT
            .saturating_sub(1)
            .saturating_sub(row)
            .saturating_sub(4);
        let (visible_report_rows, visible_message_rows) =
            split_section_budget(content_budget, report_rows.len(), message_rows.len());

        buffer.write_text(row, 0, "REPORTS", classic::status_value_style());
        row += 1;
        buffer.write_text(row, 0, "-------", classic::status_label_style());
        row += 1;
        row += write_section(&mut buffer, row, &report_rows, visible_report_rows)?;
        buffer.write_text(row, 0, "MESSAGES", classic::status_value_style());
        row += 1;
        buffer.write_text(row, 0, "--------", classic::status_label_style());
        row += 1;
        row += write_section(&mut buffer, row, &message_rows, visible_message_rows)?;
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

/// Report header lines in the summary view start with 2-space indent + "From your".
fn is_report_header(line: &str) -> bool {
    line.starts_with("  From your")
}

fn write_section(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    rows: &[String],
    max_rows: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut written = 0;
    for line in rows.iter().take(max_rows) {
        let style = if is_report_header(line) {
            classic::report_header_style()
        } else {
            classic::body_style()
        };
        buffer.write_text(start_row + written, 0, line, style);
        written += 1;
    }
    if rows.len() > max_rows {
        buffer.write_text(
            start_row + written,
            0,
            &format!(
                "  <... {} more line(s); use startup review for full suspense>",
                rows.len() - max_rows
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

fn section_rows(section_name: &str, reviewable: bool, lines: &[String]) -> Vec<String> {
    if !reviewable {
        return vec!["  <none currently reviewable>".to_string()];
    }

    if lines.is_empty() {
        let empty_notice = match section_name {
            "results" => "  <reports are marked pending, but no review text is available yet>",
            "messages" => "  <messages are marked pending, but no review text is available yet>",
            _ => "  <review items are marked pending, but no review text is available yet>",
        };
        return vec![empty_notice.to_string()];
    }

    let mut rows = Vec::new();
    for line in lines {
        if line.is_empty() {
            rows.push("  ".to_string());
            continue;
        }
        rows.extend(
            wrap_review_text_preserving_spacing(line, PLAYFIELD_WIDTH.saturating_sub(2))
                .into_iter()
                .map(|wrapped| format!("  {wrapped}")),
        );
    }
    rows
}

fn split_section_budget(total: usize, first_len: usize, second_len: usize) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }

    let mut first = usize::from(first_len > 0);
    let mut second = usize::from(second_len > 0);
    let mut remaining = total.saturating_sub(first + second);

    while remaining > 0 && (first < first_len || second < second_len) {
        if first < first_len && (first <= second || second >= second_len) {
            first += 1;
        } else if second < second_len {
            second += 1;
        } else if first < first_len {
            first += 1;
        }
        remaining -= 1;
    }

    (first.min(first_len), second.min(second_len))
}
