use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::reports::ReportsPreview;
use crate::screen::layout::{draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;

pub struct StartupScreen {
    summary: StartupSummary,
    reports: ReportsPreview,
}

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
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => self.render_splash(frame, splash_page),
            StartupPhase::Intro => self.render_intro(intro_page),
            StartupPhase::LoginSummary => self.render_login_summary(frame),
            StartupPhase::Results => {
                self.render_report_lines(frame, "PENDING RESULTS", &self.reports.results_lines)
            }
            StartupPhase::Messages => {
                self.render_report_lines(frame, "PENDING MESSAGES", &self.reports.message_lines)
            }
            StartupPhase::Complete => Ok(new_playfield()),
        }
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
        draw_title_bar(&mut buffer, 0, "LOGIN STATUS: ");
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

        if self.summary.pending_results {
            buffer.write_text(
                4,
                0,
                &format!(
                    "You have {} report line(s) pending.",
                    self.summary.results_line_count
                ),
                classic::body_style(),
            );
        } else {
            buffer.write_text(4, 0, "You have no reports pending.", classic::body_style());
        }

        if self.summary.pending_messages {
            buffer.write_text(
                5,
                0,
                &format!(
                    "You have undeleted messages: {} line(s) currently reviewable.",
                    self.summary.message_line_count
                ),
                classic::body_style(),
            );
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
            "Slap a key to continue to the login-time review flow.",
        );
        Ok(buffer)
    }

    fn render_report_lines(
        &self,
        frame: &ScreenFrame<'_>,
        title: &str,
        lines: &[String],
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

        for line in lines.iter().take(12) {
            buffer.write_text(row, 0, line, classic::body_style());
            row += 1;
        }
        if lines.len() > 12 {
            row += 1;
            buffer.write_text(
                row,
                0,
                &format!("... {} more line(s)", lines.len() - 12),
                classic::body_style(),
            );
            row += 1;
        }

        row += 1;
        draw_plain_prompt(&mut buffer, row, "Slap a key.");
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
