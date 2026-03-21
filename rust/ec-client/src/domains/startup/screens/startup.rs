use crate::reports::{wrap_review_text_preserving_spacing, ReportsPreview, ReviewBlock};
use crate::screen::layout::{draw_plain_prompt, new_playfield, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
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
                "report",
                "reports",
                "Reports",
                &self.result_blocks,
                self.summary.pending_results,
                "Reports are marked pending, but no review text is available yet.",
                &[],
                results_block,
                results_scroll_offset,
                results_mode,
                results_nonstop,
                results_deleted_any,
                game_year,
            ),
            StartupPhase::Messages => {
                let prior_results_rows = if self.summary.pending_results {
                    completed_review_history_rows(
                        &self.result_blocks,
                        "Reports are marked pending, but no review text is available yet.",
                        "report",
                        "reports",
                        results_deleted_any,
                        game_year,
                        "Reports",
                    )
                } else {
                    Vec::new()
                };
                self.render_review(
                    frame,
                    "message",
                    "messages",
                    "Messages",
                    &self.message_blocks,
                    self.summary.pending_messages,
                    "Messages are marked pending, but no review text is available yet.",
                    &prior_results_rows,
                    messages_block,
                    messages_scroll_offset,
                    messages_mode,
                    messages_nonstop,
                    messages_deleted_any,
                    game_year,
                )
            }
            StartupPhase::Complete => Ok(new_playfield()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_review(
        &self,
        frame: &ScreenFrame<'_>,
        singular: &str,
        plural: &str,
        section_label: &str,
        blocks: &[ReviewBlock],
        pending: bool,
        empty_notice: &str,
        prior_transcript_rows: &[String],
        block: usize,
        scroll_offset: usize,
        mode: StartupReviewMode,
        nonstop: bool,
        deleted_any: bool,
        game_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let delete_prompt = format!("Delete this {singular} Y/[N] ->");
        let view_prompt =
            format!("You have undeleted {plural}. View them? [Y]es, <N>o, <NS> (non-stop) ->");
        let continue_prompt =
            format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->");

        match mode {
            StartupReviewMode::ViewPrompt => {
                let mut transcript_rows = startup_login_summary_rows(frame, game_year);
                if !prior_transcript_rows.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend_from_slice(prior_transcript_rows);
                }
                if pending && blocks.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend(
                        wrap_review_text_preserving_spacing(empty_notice, PLAYFIELD_WIDTH)
                            .into_iter(),
                    );
                    render_review_transcript(&mut buffer, &transcript_rows);
                    draw_plain_prompt(&mut buffer, PLAYFIELD_HEIGHT - 1, "(Slap a key)");
                } else {
                    render_review_transcript(&mut buffer, &transcript_rows);
                    draw_plain_prompt(&mut buffer, PLAYFIELD_HEIGHT - 1, &view_prompt);
                }
            }
            StartupReviewMode::ItemBody | StartupReviewMode::DeletePrompt => {
                let mut transcript_rows = startup_login_summary_rows(frame, game_year);
                if !prior_transcript_rows.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend_from_slice(prior_transcript_rows);
                }
                transcript_rows.push(String::new());
                transcript_rows.push(view_prompt.clone());
                transcript_rows.push(String::new());
                transcript_rows.push(format!(
                    "{section_label}: Current game year is {game_year} A.D."
                ));
                transcript_rows.push(String::new());
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
                let mut transcript_rows = startup_login_summary_rows(frame, game_year);
                if !prior_transcript_rows.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend_from_slice(prior_transcript_rows);
                }
                transcript_rows.push(String::new());
                transcript_rows.push(view_prompt.clone());
                transcript_rows.push(String::new());
                transcript_rows.push(format!(
                    "{section_label}: Current game year is {game_year} A.D."
                ));
                transcript_rows.push(String::new());
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
                let mut transcript_rows = startup_login_summary_rows(frame, game_year);
                if !prior_transcript_rows.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend_from_slice(prior_transcript_rows);
                }
                transcript_rows.push(String::new());
                transcript_rows.push(view_prompt);
                transcript_rows.push(String::new());
                transcript_rows.extend(completed_review_history_rows(
                    blocks,
                    empty_notice,
                    singular,
                    plural,
                    deleted_any,
                    game_year,
                    section_label,
                ));
                render_review_transcript(&mut buffer, &transcript_rows);
                draw_plain_prompt(
                    &mut buffer,
                    PLAYFIELD_HEIGHT - 1,
                    &format!("All {plural} seen. (Slap a key)"),
                );
            }
        }

        Ok(buffer)
    }

    fn render_login_summary(
        &self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let rows = startup_login_summary_rows(frame, self.summary.game_year);
        render_review_transcript(&mut buffer, &rows);
        draw_plain_prompt(&mut buffer, PLAYFIELD_HEIGHT - 1, "(Slap a key)");
        Ok(buffer)
    }
}

pub const STARTUP_INTRO_PAGE_COUNT: usize = INTRO_PAGES.len();
pub const STARTUP_SPLASH_PAGE_COUNT: usize = 1 + INTRO_PAGES.len();
pub fn version_title() -> String {
    format!("EC v{}", env!("CARGO_PKG_VERSION"))
}

const ATTRIBUTION: &str = "Original game (c) 1992 Bentley C. Griffith";

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
    let prompt = if intro_page + 1 < INTRO_PAGES.len() {
        "Slap a key."
    } else {
        final_prompt
    };
    draw_plain_prompt(&mut buffer, 19, prompt);
    Ok(buffer)
}

fn render_splash(splash_page: usize) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();

    if splash_page == 0 {
        // First page: centered logo with attribution and version.
        let logo_width = INTRO_LOGO.iter().map(|line| line.len()).max().unwrap_or(0);
        let logo_left = 80usize.saturating_sub(logo_width) / 2;
        let block_height = INTRO_LOGO.len() + 4 + 1;
        let start_row = (19usize.saturating_sub(block_height)) / 2;
        for (row, line) in INTRO_LOGO.iter().enumerate() {
            buffer.write_text(row + start_row, logo_left, line, classic::logo_style());
        }
        buffer.write_text(
            start_row + INTRO_LOGO.len() + 2,
            logo_left,
            ATTRIBUTION,
            classic::bright_style(),
        );
        buffer.write_text(
            start_row + INTRO_LOGO.len() + 4,
            logo_left,
            &version_title(),
            classic::bright_style(),
        );
        draw_plain_prompt(&mut buffer, 19, "View the game introduction? Y/[N] -> ");
    } else {
        // Subsequent pages: transcript-style scrolling intro text.
        let mut transcript: Vec<String> = Vec::new();
        // Seed with the logo block as plain text lines.
        let logo_width = INTRO_LOGO.iter().map(|line| line.len()).max().unwrap_or(0);
        let logo_left = 80usize.saturating_sub(logo_width) / 2;
        let logo_pad: String = " ".repeat(logo_left);
        for line in &INTRO_LOGO {
            transcript.push(format!("{logo_pad}{line}"));
        }
        transcript.push(String::new());
        transcript.push(format!("{}{ATTRIBUTION}", " ".repeat(logo_left)));
        transcript.push(String::new());
        transcript.push(format!("{}{}", " ".repeat(logo_left), version_title()));
        transcript.push(String::new());
        // Append intro pages up to the current splash page.
        let intro_index = splash_page - 1;
        for page_idx in 0..=intro_index.min(INTRO_PAGES.len().saturating_sub(1)) {
            transcript.push(String::new());
            for line in INTRO_PAGES[page_idx] {
                transcript.push(format!(" {line}"));
            }
        }
        render_review_transcript(&mut buffer, &transcript);
        let prompt = if intro_index + 1 < INTRO_PAGES.len() {
            "Slap a key."
        } else {
            "Slap a key."
        };
        draw_plain_prompt(&mut buffer, 19, prompt);
    }

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
        transcript_rows.push(String::new());
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

fn startup_login_summary_rows(frame: &ScreenFrame<'_>, game_year: u16) -> Vec<String> {
    let identity = if !frame.player.handle.is_empty() {
        frame.player.handle.as_str()
    } else {
        display_or_unknown(&frame.player.empire_name)
    };
    vec![
        format!(
            "You are \"{identity}\", (Empire #{})",
            frame.player.record_index_1_based
        ),
        String::new(),
        format!("The year is: {game_year} A.D."),
        String::new(),
        format!("Last year on: {} A.D.", game_year.saturating_sub(1)),
    ]
}

fn completed_review_history_rows(
    blocks: &[ReviewBlock],
    empty_notice: &str,
    singular: &str,
    plural: &str,
    deleted_any: bool,
    game_year: u16,
    section_label: &str,
) -> Vec<String> {
    let delete_prompt = format!("Delete this {singular} Y/[N] ->");
    let continue_prompt =
        format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->");
    let mut transcript_rows = vec![format!(
        "{section_label}: Current game year is {game_year} A.D."
    )];
    transcript_rows.push(String::new());
    for previous_block in 0..blocks.len() {
        let previous_rows = block_review_rows(block_lines(blocks, previous_block), empty_notice);
        let include_continue_prompt = previous_block + 1 < blocks.len();
        push_completed_block_transcript(
            &mut transcript_rows,
            previous_rows,
            &delete_prompt,
            &continue_prompt,
            include_continue_prompt,
        );
    }
    if deleted_any {
        transcript_rows.push(String::new());
        transcript_rows.push(format!("{} deleted.", capitalize(plural)));
    }
    transcript_rows.push(String::new());
    transcript_rows.push(format!("All {plural} seen."));
    transcript_rows
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
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
