use crate::model::ClassicLoginState;
use crate::reports::{ReportsPreview, ReviewBlock, wrap_review_text_preserving_spacing};
use crate::screen::layout::{
    PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, draw_plain_prompt, draw_status_line, draw_title_bar,
    new_playfield,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupReviewMode {
    ViewPrompt,
    ItemBody,
    DeletePrompt,
    ContinuePrompt,
    EndStatus,
}

pub struct StartupScreen {
    summary: StartupSummary,
    result_blocks: Vec<ReviewBlock>,
    message_blocks: Vec<ReviewBlock>,
}

pub(crate) const STARTUP_REVIEW_VISIBLE_LINES: usize = PLAYFIELD_HEIGHT - 4;
const ITEM_HEADER_PREFIX: &str = " -> ";
const ITEM_BODY_PREFIX: &str = "< ";

impl StartupScreen {
    pub fn new(summary: StartupSummary, reports: ReportsPreview) -> Self {
        Self {
            summary,
            result_blocks: reports.result_blocks,
            message_blocks: reports.message_blocks,
        }
    }

    pub fn replace(&mut self, summary: StartupSummary, reports: ReportsPreview) {
        self.summary = summary;
        self.result_blocks = reports.result_blocks;
        self.message_blocks = reports.message_blocks;
    }

    pub fn result_block_count(&self) -> usize {
        self.result_blocks.len()
    }

    pub fn message_block_count(&self) -> usize {
        self.message_blocks.len()
    }

    pub fn result_blocks(&self) -> &[ReviewBlock] {
        &self.result_blocks
    }

    pub fn message_blocks(&self) -> &[ReviewBlock] {
        &self.message_blocks
    }

    pub fn results_block_row_count(&self, block: usize) -> usize {
        block_review_rows(
            block_lines(&self.result_blocks, block),
            "Reports are marked pending, but no review text is available yet.",
        )
        .len()
    }

    pub fn messages_block_row_count(&self, block: usize) -> usize {
        block_review_rows(
            block_lines(&self.message_blocks, block),
            "Messages are marked pending, but no review text is available yet.",
        )
        .len()
    }

    pub fn render_phase(
        &self,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
        splash_page: usize,
        intro_page: usize,
        results_block: usize,
        results_scroll_offset: usize,
        results_mode: StartupReviewMode,
        results_nonstop: bool,
        messages_block: usize,
        messages_scroll_offset: usize,
        messages_mode: StartupReviewMode,
        messages_nonstop: bool,
        results_deleted_any: bool,
        messages_deleted_any: bool,
        game_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => render_splash(splash_page),
            StartupPhase::Intro => render_game_intro_page(intro_page, "Slap a key."),
            StartupPhase::LoginSummary => self.render_login_summary(frame),
            StartupPhase::Results => self.render_review(
                frame,
                "RESULTS REVIEW",
                "report",
                "reports",
                "Reports",
                &self.result_blocks,
                self.summary.pending_results,
                "Reports are marked pending, but no review text is available yet.",
                results_block,
                results_scroll_offset,
                results_mode,
                results_nonstop,
                results_deleted_any,
                game_year,
            ),
            StartupPhase::Messages => self.render_review(
                frame,
                "MESSAGES REVIEW",
                "message",
                "messages",
                "Messages",
                &self.message_blocks,
                self.summary.pending_messages,
                "Messages are marked pending, but no review text is available yet.",
                messages_block,
                messages_scroll_offset,
                messages_mode,
                messages_nonstop,
                messages_deleted_any,
                game_year,
            ),
            StartupPhase::Complete => Ok(new_playfield()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_review(
        &self,
        frame: &ScreenFrame<'_>,
        title: &str,
        singular: &str,
        plural: &str,
        section_label: &str,
        blocks: &[ReviewBlock],
        pending: bool,
        empty_notice: &str,
        block: usize,
        scroll_offset: usize,
        mode: StartupReviewMode,
        nonstop: bool,
        deleted_any: bool,
        game_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let delete_prompt = format!("Delete this {singular} Y/[N] ->");
        let continue_prompt =
            format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->");

        match mode {
            StartupReviewMode::ViewPrompt => {
                draw_title_bar(&mut buffer, 0, &format!("{title}: "));
                draw_status_line(
                    &mut buffer,
                    2,
                    "Empire: ",
                    &format!(
                        "{}  Player {}",
                        display_or_unknown(&frame.player.empire_name),
                        frame.player.record_index_1_based
                    ),
                );
                if pending && blocks.is_empty() {
                    buffer.write_text(4, 0, empty_notice, classic::body_style());
                    draw_plain_prompt(&mut buffer, 6, "(Slap a key)");
                } else {
                    draw_plain_prompt(
                        &mut buffer,
                        4,
                        &format!(
                            "You have undeleted {plural}. View them? [Y]es, <N>o, <NS> (non-stop) ->"
                        ),
                    );
                }
            }
            StartupReviewMode::ItemBody | StartupReviewMode::DeletePrompt => {
                let header = format!("{section_label}: Current game year is {game_year} A.D.");
                buffer.write_text(0, 0, &header, classic::body_style());

                let mut transcript_rows = Vec::new();
                for previous_block in 0..block {
                    let previous_rows =
                        block_review_rows(block_lines(blocks, previous_block), empty_notice);
                    push_completed_block_transcript(
                        &mut transcript_rows,
                        previous_rows,
                        &delete_prompt,
                        &continue_prompt,
                        true,
                    );
                }

                let rows = block_review_rows(block_lines(blocks, block), empty_notice);
                let revealed_end =
                    usize::min(scroll_offset + STARTUP_REVIEW_VISIBLE_LINES, rows.len());
                transcript_rows.extend(rows[..revealed_end].iter().cloned());

                render_review_transcript(&mut buffer, &transcript_rows);

                let prompt_row = PLAYFIELD_HEIGHT - 1;
                if revealed_end < rows.len() {
                    draw_plain_prompt(&mut buffer, prompt_row, "(Slap a key for more)");
                } else if !nonstop {
                    draw_plain_prompt(&mut buffer, prompt_row, &delete_prompt);
                } else {
                    draw_plain_prompt(&mut buffer, prompt_row, "(Slap a key)");
                }
            }
            StartupReviewMode::ContinuePrompt => {
                let header = format!("{section_label}: Current game year is {game_year} A.D.");
                buffer.write_text(0, 0, &header, classic::body_style());
                let mut transcript_rows = Vec::new();
                for previous_block in 0..block {
                    let previous_rows =
                        block_review_rows(block_lines(blocks, previous_block), empty_notice);
                    let include_continue_prompt = previous_block + 1 < block;
                    push_completed_block_transcript(
                        &mut transcript_rows,
                        previous_rows,
                        &delete_prompt,
                        &continue_prompt,
                        include_continue_prompt,
                    );
                }
                render_review_transcript(&mut buffer, &transcript_rows);
                draw_plain_prompt(&mut buffer, PLAYFIELD_HEIGHT - 1, &continue_prompt);
            }
            StartupReviewMode::EndStatus => {
                draw_title_bar(&mut buffer, 0, &format!("{title}: "));
                let status = if deleted_any {
                    format!("{} deleted.", capitalize(plural))
                } else {
                    format!("All {plural} seen.")
                };
                buffer.write_text(4, 0, &status, classic::body_style());
                draw_plain_prompt(&mut buffer, 6, "(Slap a key)");
            }
        }

        Ok(buffer)
    }

    fn render_login_summary(
        &self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "REVIEW STATUS: ");
        draw_status_line(
            &mut buffer,
            2,
            "Empire: ",
            &format!(
                "{}  Player {}",
                display_or_unknown(&frame.player.empire_name),
                frame.player.record_index_1_based
            ),
        );
        let login_status = match self.summary.login_state {
            ClassicLoginState::MatchedPreloadedFirstLogin => {
                "Matched pre-loaded commander. First-login review is required."
            }
            ClassicLoginState::ReturningPlayer => {
                "Returning commander recognized. Resuming login-time review."
            }
            ClassicLoginState::FirstTimeMenu => "First-time commander path.",
        };
        buffer.write_text(3, 0, login_status, classic::body_style());

        if self.summary.pending_results {
            let report_status = if self.result_blocks.is_empty() {
                "Reports are marked pending, but no review text is available yet.".to_string()
            } else {
                "Reports are waiting for your review.".to_string()
            };
            buffer.write_text(4, 0, &report_status, classic::body_style());
        } else {
            buffer.write_text(4, 0, "You have no reports pending.", classic::body_style());
        }

        if self.summary.pending_messages {
            let message_status = if self.message_blocks.is_empty() {
                "Messages are marked pending, but no review text is available yet.".to_string()
            } else {
                "Messages are waiting for your review.".to_string()
            };
            buffer.write_text(5, 0, &message_status, classic::body_style());
        } else {
            buffer.write_text(
                5,
                0,
                "You have no undeleted messages.",
                classic::body_style(),
            );
        }

        draw_plain_prompt(&mut buffer, 7, "(Slap a key to continue)");
        Ok(buffer)
    }
}

pub const STARTUP_INTRO_PAGE_COUNT: usize = INTRO_PAGES.len();
pub const STARTUP_SPLASH_PAGE_COUNT: usize = 1;
pub const GAME_VERSION: &str = "1.60";

pub fn version_title() -> String {
    format!("Esterian Conquest Ver {GAME_VERSION}")
}

pub fn render_game_intro_page(
    intro_page: usize,
    final_prompt: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    let text_start_row = 2;
    let lines = INTRO_PAGES
        .get(intro_page)
        .copied()
        .unwrap_or(INTRO_PAGES.last().copied().unwrap_or(&[]));
    for (row, line) in lines.iter().enumerate() {
        buffer.write_text(row + text_start_row, 1, line, classic::body_style());
    }
    if intro_page + 1 == INTRO_PAGES.len() {
        let version_row = text_start_row + lines.len() + 3;
        if version_row < 19 {
            buffer.write_text(version_row, 1, &version_title(), classic::bright_style());
        }
    }
    let prompt = if intro_page + 1 < INTRO_PAGES.len() {
        "Slap a key for the next section."
    } else {
        final_prompt
    };
    draw_plain_prompt(&mut buffer, 19, prompt);
    Ok(buffer)
}

fn render_splash(_splash_page: usize) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    let version = version_title();
    let logo_width = INTRO_LOGO.iter().map(|line| line.len()).max().unwrap_or(0);
    let logo_left = 80usize.saturating_sub(logo_width) / 2;
    let block_height = INTRO_LOGO.len() + 3 + 1;
    let start_row = (19usize.saturating_sub(block_height)) / 2;
    for (row, line) in INTRO_LOGO.iter().enumerate() {
        buffer.write_text(row + start_row, logo_left, line, classic::logo_style());
    }
    buffer.write_text(
        start_row + INTRO_LOGO.len() + 3,
        logo_left,
        &version,
        classic::bright_style(),
    );
    draw_plain_prompt(&mut buffer, 19, "View the game introduction? Y/[N] -> ");
    Ok(buffer)
}

fn block_lines<'a>(blocks: &'a [ReviewBlock], block: usize) -> &'a [String] {
    blocks.get(block).map(|b| b.lines.as_slice()).unwrap_or(&[])
}

fn block_review_rows(lines: &[String], empty_notice: &str) -> Vec<String> {
    if lines.is_empty() {
        if empty_notice.is_empty() {
            return Vec::new();
        }
        return wrap_review_text_preserving_spacing(
            empty_notice,
            PLAYFIELD_WIDTH.saturating_sub(ITEM_BODY_PREFIX.len()),
        )
        .into_iter()
        .map(|line| format!("{ITEM_BODY_PREFIX}{line}"))
        .collect();
    }

    let mut rows = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            rows.push("<".to_string());
            continue;
        }
        let prefix = if line_idx == 0 {
            ITEM_HEADER_PREFIX
        } else {
            ITEM_BODY_PREFIX
        };
        let max_width = PLAYFIELD_WIDTH.saturating_sub(prefix.len());
        let wrapped = wrap_review_text_preserving_spacing(line, max_width);
        for (wrap_idx, segment) in wrapped.iter().enumerate() {
            let seg_prefix = if line_idx == 0 && wrap_idx == 0 {
                ITEM_HEADER_PREFIX
            } else {
                ITEM_BODY_PREFIX
            };
            rows.push(format!("{seg_prefix}{segment}"));
        }
    }
    rows
}

fn push_completed_block_transcript(
    transcript_rows: &mut Vec<String>,
    block_rows: Vec<String>,
    delete_prompt: &str,
    continue_prompt: &str,
    include_continue_prompt: bool,
) {
    transcript_rows.extend(block_rows);
    transcript_rows.push(String::new());
    transcript_rows.push(delete_prompt.to_string());
    transcript_rows.push(String::new());
    if include_continue_prompt {
        transcript_rows.push(continue_prompt.to_string());
    }
}

fn render_review_transcript(buffer: &mut PlayfieldBuffer, transcript_rows: &[String]) {
    let visible_start = transcript_rows
        .len()
        .saturating_sub(STARTUP_REVIEW_VISIBLE_LINES);
    let visible_rows = &transcript_rows[visible_start..];
    let first_row = 18usize.saturating_sub(visible_rows.len());
    for (i, line) in visible_rows.iter().enumerate() {
        buffer.write_text(first_row + i, 0, line, classic::body_style());
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}

const INTRO_LOGO: [&str; 11] = [
    "  o     #######   ###### ########  #######  ######    ##    #####    ###  ##",
    "    .  ##       ##         ##     ##       ##   ##   ##   ##   ##   #### ##",
    "      ####      #####     ##     ####     ######    ##   #######   ## ####   .",
    "     ##            ##    ##     ##       ## ##     ##   ##   ##   ##  ###",
    " .  #######  ######     ##     #######  ##   ##   ##   ##   ##   ##   ##",
    "",
    "   *   ######   #####    ###  ##   #####   ##   ##  #######   ###### ########",
    "     ##       ##   ##   #### ##  ##   ##  ##   ##  ##       ##         ##  .",
    "  . ##       ##   ##   ## ####  ##   ##  ##   ##  ####      #####     ##",
    "   ##       ##   ##   ##  ###  ## # ##  ##   ##  ##            ##    ##      .",
    "   ######   #####    ##   ##   ######   #####   #######  ######     ##",
];

const INTRO_PAGE_1: [&str; 13] = [
    "Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy of",
    "contested solar systems. The old masters are gone. Their stations are silent,",
    "their patrols vanished, and their subjects left with fleets, factories, and",
    "enough knowledge to build empires.",
    "",
    "You rise as one of the new Star Masters. From a single world and a few small",
    "fleets, you must tax, build, scout, bargain, threaten, and strike before rival",
    "powers can do the same. Some systems will join your banner willingly. Others",
    "will require persuasion from orbit.",
    "",
    "Each maintenance marks the passage of a year. In that span, fleets cross the",
    "dark between stars, colonies grow or starve, alliances turn cold, and wars are",
    "decided by distance, industry, mathematics, and will.",
];

const INTRO_PAGE_2: [&str; 7] = [
    "",
    "In profound respect and admiration to Bentley C. Griffith and his fellow",
    "pioneers, who between 1990 and 1992 forged the enduring legend of Esterian",
    "Conquest, and to the ancient dreamers, strategists, and storytellers whose",
    "timeless visions of galactic dominion still light the way among these stars.",
    "",
    "",
];

const INTRO_PAGES: [&[&str]; 2] = [&INTRO_PAGE_1, &INTRO_PAGE_2];
