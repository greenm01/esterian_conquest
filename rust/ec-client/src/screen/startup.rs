use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::reports::ReportsPreview;
use crate::screen::layout::{
    draw_centered_text, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;

pub struct StartupScreen {
    summary: StartupSummary,
    reports: ReportsPreview,
    bbs_splash_pages: Vec<Vec<String>>,
}

impl StartupScreen {
    pub fn new(
        summary: StartupSummary,
        reports: ReportsPreview,
        bbs_splash_pages: Vec<Vec<String>>,
    ) -> Self {
        Self {
            summary,
            reports,
            bbs_splash_pages,
        }
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
        splash_page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match splash_page {
            0 => self.render_bbs_splash_page(),
            1 => render_startup_art_page(
                &EC_ART_PAGE_1,
                "Press any key to continue.",
                classic::bright_style(),
            ),
            2 => render_startup_art_page(
                &EC_ART_PAGE_2,
                "Press any key to continue.",
                classic::bright_style(),
            ),
            _ => self.render_intro_prompt_page(),
        }
    }

    fn render_intro(&self, intro_page: usize) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_centered_text(
            &mut buffer,
            0,
            "Esterian Conquest Ver 1.60",
            classic::bright_style(),
        );
        let lines = INTRO_PAGES
            .get(intro_page)
            .copied()
            .unwrap_or(INTRO_PAGES.last().copied().unwrap_or(&[]));
        for (row, line) in lines.iter().enumerate() {
            buffer.write_text(row + 2, 0, line, classic::body_style());
        }
        let prompt = if intro_page + 1 < INTRO_PAGES.len() {
            "Press any key for the next section."
        } else {
            "Press any key to continue."
        };
        draw_plain_prompt(&mut buffer, 19, prompt);
        Ok(buffer)
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
            "Press any key to continue to the login-time review flow.",
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
        draw_plain_prompt(&mut buffer, row, "Press any key to continue.");
        Ok(buffer)
    }
}

pub fn load_bbs_splash_pages(path: Option<&Path>) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let pages = if let Some(path) = path {
        let text = fs::read_to_string(path)?;
        let parsed = text
            .split('\u{000c}')
            .map(|page| {
                page.lines()
                    .map(|line| line.chars().take(80).collect::<String>())
                    .collect::<Vec<_>>()
            })
            .filter(|page| !page.is_empty())
            .collect::<Vec<_>>();
        if parsed.is_empty() {
            vec![DEFAULT_BBS_SPLASH_PAGE.iter().map(|line| (*line).to_string()).collect()]
        } else {
            parsed
        }
    } else {
        vec![DEFAULT_BBS_SPLASH_PAGE.iter().map(|line| (*line).to_string()).collect()]
    };
    Ok(pages)
}

impl StartupScreen {
    fn render_bbs_splash_page(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let page = self
            .bbs_splash_pages
            .first()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for (idx, line) in page.iter().take(18).enumerate() {
            draw_centered_text(&mut buffer, idx, line, classic::menu_hotkey_style());
        }
        draw_plain_prompt(&mut buffer, 19, "Press any key to continue.");
        Ok(buffer)
    }

    fn render_intro_prompt_page(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        for (idx, line) in EC_ART_PAGE_3.iter().enumerate() {
            buffer.write_text(idx, 0, line, classic::bright_style());
        }
        draw_plain_prompt(&mut buffer, 19, "View Introduction? Y/[N] ->");
        Ok(buffer)
    }
}

pub const STARTUP_INTRO_PAGE_COUNT: usize = INTRO_PAGES.len();
pub const STARTUP_SPLASH_PAGE_COUNT: usize = 4;

const DEFAULT_BBS_SPLASH_PAGE: [&str; 10] = [
    "",
    "THE BATTLE FIELD BBS",
    "",
    "presents",
    "",
    "ESTERIAN CONQUEST",
    "",
    "A classic galactic war door, rebuilt for the modern terminal.",
    "",
    "Default sysop splash. Override with EC_CLIENT_BBS_SPLASH if desired.",
];

const EC_ART_PAGE_1: [&str; 19] = [
    "        .                .                          .                  *        ",
    "                                                                                ",
    "                                                                                ",
    "                      E S T E R I A N                                           ",
    "                                                                                ",
    "                        C O N Q U E S T                                         ",
    "                                                                                ",
    "                                                                                ",
    "                 .                                .                             ",
    "       *                            .                                           ",
    "                                                  .                             ",
    "                              Version 1.60                                      ",
    "                                                                                ",
    "             A galaxy of old wreckage, fresh empires, and unfinished wars.     ",
    "                                                                                ",
    "                                                                                ",
    "      .                    *                               .                    ",
    "                                                                                ",
    "                                                                                ",
];

const EC_ART_PAGE_2: [&str; 19] = [
    "                                                                                ",
    "                             .                        .                         ",
    "                                                                                ",
    "                                                                                ",
    "                                   /\\                                           ",
    "                                  /  \\                                          ",
    "                     __==========/====\\==========__                             ",
    "                  .-'          /  /\\  \\          '-.                            ",
    "                .'            /__/  \\__\\            '.                          ",
    "               /________________________________________________\\               ",
    "                                                                                ",
    "                           .                       *                             ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
];

const EC_ART_PAGE_3: [&str; 19] = [
    "                                                                                ",
    "                           *                                .                   ",
    "                                                                                ",
    "                                    /\\                                          ",
    "                                   /  \\                                         ",
    "                     __===========/====\\===========__                           ",
    "                  .-'           /  /\\  \\           '-.                          ",
    "                .'             /__/  \\__\\             '.                        ",
    "               /________________________________________________\\               ",
    "********************************************************************************",
    "                                                                                ",
    "                           ESTERIAN CONQUEST  Ver 1.60                          ",
    "                                                                                ",
    "        Use Ctrl-S throughout the game to pause and resume output.              ",
    "        Use Ctrl-X or Ctrl-K to abort most listings.                            ",
    "        Copyright (C) 1990, 1991 by Bentley C. Griffith.                        ",
    "                                                                                ",
    "                                                                                ",
    "                                                                                ",
];

fn render_startup_art_page(
    lines: &[&str],
    prompt: &str,
    style: crate::screen::CellStyle,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    for (idx, line) in lines.iter().enumerate().take(19) {
        buffer.write_text(idx, 0, line, style);
    }
    draw_plain_prompt(&mut buffer, 19, prompt);
    Ok(buffer)
}

const INTRO_PAGE_1: [&str; 14] = [
    "Beyond the mapped frontiers of the old Esterian dominion lies a small",
    "galaxy of contested solar systems. The old masters are gone. Their",
    "stations are silent, their patrols vanished, and their subjects left",
    "with fleets, factories, and enough knowledge to build empires.",
    "",
    "You rise as one of the new Star Masters. From a single world and a few",
    "small fleets, you must tax, build, scout, bargain, threaten, and",
    "strike before rival powers can do the same. Some systems will join",
    "your banner willingly. Others will require persuasion from orbit.",
    "",
    "Each maintenance marks the passage of a year. In that span, fleets",
    "cross the dark between stars, colonies grow or starve, alliances turn",
    "cold, and wars are decided by distance, industry, mathematics, and",
    "will.",
];

const INTRO_PAGE_2: [&str; 8] = [
    "In profound respect and admiration to Bentley C. Griffith and his",
    "fellow pioneers, who between 1990 and 1992 forged the enduring legend",
    "of Esterian Conquest-a digital realm where star empires rose and fell",
    "across BBS screens-and to the ancient dreamers, strategists, and",
    "storytellers whose timeless visions of galactic dominion first lit the",
    "way for every commander who still dares to claim worlds among these",
    "endless stars.",
    "",
];

const INTRO_PAGES: [&[&str]; 2] = [&INTRO_PAGE_1, &INTRO_PAGE_2];

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
}
