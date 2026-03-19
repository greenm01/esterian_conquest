use crate::model::ClassicLoginState;
use crate::reports::{ReportsPreview, ReviewBlock};
use crate::screen::layout::{
    PLAYFIELD_WIDTH, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
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

const STARTUP_REVIEW_VISIBLE_LINES: usize = 12;
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

    pub fn results_block_page_count(&self, block: usize) -> usize {
        let rows = block_review_rows(
            block_lines(&self.result_blocks, block),
            "Reports are marked pending, but no review text is available yet.",
        );
        page_count(&rows)
    }

    pub fn messages_block_page_count(&self, block: usize) -> usize {
        let rows = block_review_rows(
            block_lines(&self.message_blocks, block),
            "Messages are marked pending, but no review text is available yet.",
        );
        page_count(&rows)
    }

    pub fn render_phase(
        &self,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
        splash_page: usize,
        intro_page: usize,
        results_block: usize,
        results_page: usize,
        results_mode: StartupReviewMode,
        messages_block: usize,
        messages_page: usize,
        messages_mode: StartupReviewMode,
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
                results_page,
                results_mode,
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
                messages_page,
                messages_mode,
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
        page: usize,
        mode: StartupReviewMode,
        deleted_any: bool,
        game_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();

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

                let rows = block_review_rows(block_lines(blocks, block), empty_notice);
                let start = page * STARTUP_REVIEW_VISIBLE_LINES;
                let end = usize::min(start + STARTUP_REVIEW_VISIBLE_LINES, rows.len());

                for (i, line) in rows[start..end].iter().enumerate() {
                    buffer.write_text(2 + i, 0, line, classic::body_style());
                }

                if end < rows.len() {
                    draw_plain_prompt(&mut buffer, 19, "(Slap a key for more)");
                } else if mode == StartupReviewMode::DeletePrompt {
                    draw_plain_prompt(&mut buffer, 19, &format!("Delete this {singular} Y/[N] ->"));
                } else {
                    draw_plain_prompt(&mut buffer, 19, "(Slap a key)");
                }
            }
            StartupReviewMode::ContinuePrompt => {
                draw_title_bar(&mut buffer, 0, &format!("{title}: "));
                draw_plain_prompt(
                    &mut buffer,
                    4,
                    &format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->"),
                );
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
        return wrap_review_text(
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
        let wrapped = wrap_review_text(line, max_width);
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

fn page_count(rows: &[String]) -> usize {
    usize::max(1, rows.len().div_ceil(STARTUP_REVIEW_VISIBLE_LINES))
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
