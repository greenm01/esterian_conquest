use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::model::ClassicLoginState;
use crate::reports::ReportsPreview;
use crate::screen::layout::{
    PLAYFIELD_WIDTH, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;

pub struct StartupScreen {
    summary: StartupSummary,
    reports: ReportsPreview,
}

const STARTUP_REVIEW_VISIBLE_LINES: usize = 12;
const STARTUP_REVIEW_PREFIX: &str = "< ";

impl StartupScreen {
    pub fn new(summary: StartupSummary, reports: ReportsPreview) -> Self {
        Self { summary, reports }
    }

    pub fn render_phase(
        &mut self,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
        splash_page: usize,
        intro_page: usize,
        results_page: usize,
        messages_page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => self.render_splash(frame, splash_page),
            StartupPhase::Intro => self.render_intro(intro_page),
            StartupPhase::LoginSummary => self.render_login_summary(frame),
            StartupPhase::Results => self.render_report_lines(
                frame,
                "RESULTS REVIEW",
                &self.reports.results_lines,
                "Reports are marked pending, but no review text is available yet.",
                results_page,
            ),
            StartupPhase::Messages => self.render_report_lines(
                frame,
                "MESSAGES REVIEW",
                &self.reports.message_lines,
                "Messages are marked pending, but no review text is available yet.",
                messages_page,
            ),
            StartupPhase::Complete => Ok(new_playfield()),
        }
    }

    pub fn results_page_count(&self) -> usize {
        review_page_count(&review_rows(
            &self.reports.results_lines,
            "Reports are marked pending, but no review text is available yet.",
        ))
    }

    pub fn messages_page_count(&self) -> usize {
        review_page_count(&review_rows(
            &self.reports.message_lines,
            "Messages are marked pending, but no review text is available yet.",
        ))
    }

    pub fn handle_key(&self, phase: StartupPhase, key: KeyEvent) -> Action {
        match (phase, key.code) {
            (StartupPhase::Splash, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                Action::OpenStartupIntro
            }
            (_, KeyCode::Char('q') | KeyCode::Char('Q')) => Action::Quit,
            _ => Action::AdvanceStartup,
        }
    }

    fn render_splash(
        &self,
        _frame: &ScreenFrame<'_>,
        _splash_page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
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

    fn render_intro(
        &self,
        intro_page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        render_game_intro_page(intro_page, "Slap a key.")
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
            let report_status = if self.summary.results_line_count == 0 {
                "Reports are marked pending, but no review text is available yet.".to_string()
            } else {
                "Reports are waiting for your review.".to_string()
            };
            buffer.write_text(4, 0, &report_status, classic::body_style());
        } else {
            buffer.write_text(4, 0, "You have no reports pending.", classic::body_style());
        }

        if self.summary.pending_messages {
            let message_status = if self.summary.message_line_count == 0 {
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

        draw_plain_prompt(
            &mut buffer,
            7,
            "(Slap a key to continue to the login-time review flow)",
        );
        Ok(buffer)
    }

    fn render_report_lines(
        &self,
        frame: &ScreenFrame<'_>,
        title: &str,
        lines: &[String],
        empty_notice: &str,
        page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let mut row = 0;
        draw_title_bar(&mut buffer, row, &format!("{title}: "));
        row += 2;
        draw_status_line(
            &mut buffer,
            row,
            "Empire: ",
            &format!(
                "{}  Player {}",
                display_or_unknown(&frame.player.empire_name),
                frame.player.record_index_1_based
            ),
        );
        row += 2;

        let review_rows = review_rows(lines, empty_notice);
        let start = page.saturating_mul(STARTUP_REVIEW_VISIBLE_LINES);
        let end = usize::min(start + STARTUP_REVIEW_VISIBLE_LINES, review_rows.len());
        for line in &review_rows[start..end] {
            buffer.write_text(row, 0, line, classic::body_style());
            row += 1;
        }
        if end < review_rows.len() {
            row += 1;
            buffer.write_text(
                row,
                0,
                &format!("... {} more line(s)", review_rows.len() - end),
                classic::body_style(),
            );
            row += 1;
        }

        row += 1;
        let prompt = if (page + 1) >= review_page_count(&review_rows) {
            "(Slap a key)"
        } else {
            "(Slap a key for more)"
        };
        draw_plain_prompt(&mut buffer, row, prompt);
        Ok(buffer)
    }
}

pub const STARTUP_INTRO_PAGE_COUNT: usize = INTRO_PAGES.len();
pub const STARTUP_SPLASH_PAGE_COUNT: usize = 1;
pub const GAME_VERSION: &str = "1.60";

pub fn version_title() -> String {
    format!("Esterian Conquest Ver {GAME_VERSION}")
}

fn review_page_count(lines: &[String]) -> usize {
    usize::max(1, lines.len().div_ceil(STARTUP_REVIEW_VISIBLE_LINES))
}

fn review_rows(lines: &[String], empty_notice: &str) -> Vec<String> {
    if lines.is_empty() {
        return wrap_review_text(empty_notice, PLAYFIELD_WIDTH)
            .into_iter()
            .map(|line| format!("{STARTUP_REVIEW_PREFIX}{line}"))
            .collect();
    }

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            rows.push("<".to_string());
            continue;
        }
        rows.extend(
            wrap_review_text(
                line,
                PLAYFIELD_WIDTH.saturating_sub(STARTUP_REVIEW_PREFIX.len()),
            )
            .into_iter()
            .map(|wrapped| format!("{STARTUP_REVIEW_PREFIX}{wrapped}")),
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

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
