use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_line_default_input, draw_command_prompt, draw_help_panel, draw_plain_prompt,
    draw_status_line, draw_title_bar, new_playfield, MenuEntry,
};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

pub struct FirstTimeMenuScreen;
pub struct FirstTimeHelpScreen;
pub struct FirstTimeEmpiresScreen;
pub struct FirstTimeIntroScreen;

const FIRST_TIME_ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "H", "elp with commands"),
    MenuEntry::new(28, "L", "ist current empires"),
    MenuEntry::new(55, "A", "nsi color ON/OFF"),
];

const FIRST_TIME_ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "Q", "uit back to BBS"),
    MenuEntry::new(28, "J", "oin this game"),
    MenuEntry::new(55, "V", "iew Game Introduction"),
];

const HELP_LINES: [&str; 6] = [
    "<A> - ANSI color stays on in the Rust client; this century has standards.",
    "<H> - describe First Time Menu commands",
    "<J> - join the game and control an unowned empire",
    "<L> - list all empires in the order you specify",
    "<Q> - quit Esterian Conquest and return to the BBS",
    "<V> - view the introduction to this game",
];

impl FirstTimeMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "FIRST TIME MENU:");
        crate::screen::layout::draw_menu_row(&mut buffer, 1, &FIRST_TIME_ROW_1);
        crate::screen::layout::draw_menu_row(&mut buffer, 2, &FIRST_TIME_ROW_2);
        if let Some(status) = status {
            draw_status_line(&mut buffer, 4, "Notice: ", status);
        }
        draw_command_prompt(&mut buffer, 5, "FIRST TIME COMMAND", "H Q L J A V");
        Ok(buffer)
    }
}

impl Screen for FirstTimeMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render(None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenFirstTimeHelp,
            KeyCode::Char('l') | KeyCode::Char('L') => Action::OpenFirstTimeEmpires,
            KeyCode::Char('v') | KeyCode::Char('V') => Action::OpenFirstTimeIntro,
            KeyCode::Char('a') | KeyCode::Char('A') => Action::ShowAnsiAlwaysOnNotice,
            KeyCode::Char('j') | KeyCode::Char('J') => Action::OpenFirstTimeJoinConfirm,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::Quit,
            _ => Action::Noop,
        }
    }
}

impl FirstTimeHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for FirstTimeHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "FIRST TIME HELP:",
            "Help - First Time Menu command descriptions:",
            &HELP_LINES,
            "FIRST TIME COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenFirstTimeMenu
    }
}

pub fn render_first_time_join_confirm() -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "FIRST TIME JOIN:");
    buffer.write_text(
        2,
        0,
        "Would you like to join the current game?",
        classic::body_style(),
    );
    draw_plain_prompt(&mut buffer, 4, "Join the current game? Y/N -> ");
    Ok(buffer)
}

pub fn render_first_time_join_name(
    input: &str,
    status: Option<&str>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "FIRST TIME JOIN:");
    buffer.write_text(
        2,
        0,
        "Enter the name of your empire (up to 20 characters).",
        classic::body_style(),
    );
    if let Some(status) = status {
        draw_status_line(&mut buffer, 4, "Notice: ", status);
    }
    draw_command_line_default_input(
        &mut buffer,
        "EMPIRE NAME",
        "Enter the name of your empire (up to 20 characters) ",
        "",
        input,
    );
    Ok(buffer)
}

pub fn render_first_time_join_name_confirm(
    empire_name: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "FIRST TIME JOIN:");
    buffer.write_text(
        2,
        0,
        &format!("\"{empire_name}\" <- Is this correct? [Y]/N ->"),
        classic::body_style(),
    );
    Ok(buffer)
}

pub fn render_first_time_join_summary(
    empire_name: &str,
    empire_id: usize,
    year: u16,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "JOIN COMPLETE:");
    buffer.write_text(
        2,
        0,
        &format!("Commander, you are \"{empire_name}\", (Empire #{empire_id})"),
        classic::body_style(),
    );
    buffer.write_text(4, 0, &format!("The year is: {year} A.D."), classic::body_style());
    buffer.write_text(6, 0, "Last year on: NEVER", classic::body_style());
    buffer.write_text(
        8,
        0,
        "You have 60 minutes left to play.",
        classic::body_style(),
    );
    buffer.write_text(10, 0, "Autopilot is off.", classic::body_style());
    draw_plain_prompt(&mut buffer, 12, "(Press Return)");
    Ok(buffer)
}

pub fn render_first_time_join_no_pending() -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "JOIN COMPLETE:");
    buffer.write_text(2, 0, "You have no reports pending.", classic::body_style());
    buffer.write_text(4, 0, "You have no messages pending.", classic::body_style());
    draw_plain_prompt(&mut buffer, 6, "(Press Return)");
    Ok(buffer)
}

pub fn render_first_time_homeworld_name(
    coords: [u8; 2],
    present_production: u16,
    potential_production: u16,
    input: &str,
    status: Option<&str>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "HOMEWORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!(
            "You have a world in the solar system at X={}, Y={}. Its current production",
            coords[0], coords[1]
        ),
        classic::body_style(),
    );
    buffer.write_text(
        3,
        0,
        &format!(
            "level is {} out of a possible {} points, (100% efficiency).",
            present_production, potential_production
        ),
        classic::body_style(),
    );
    if let Some(status) = status {
        draw_status_line(&mut buffer, 5, "Notice: ", status);
    }
    draw_command_line_default_input(
        &mut buffer,
        "HOMEWORLD",
        "Name this world (20 characters or less) ",
        "",
        input,
    );
    Ok(buffer)
}

pub fn render_first_time_homeworld_confirm(
    homeworld_name: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "HOMEWORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!("\"{homeworld_name}\" <- Is this correct? Y/[N] ->"),
        classic::body_style(),
    );
    Ok(buffer)
}

impl FirstTimeEmpiresScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_rows(
        &mut self,
        rows: &[String],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "CURRENT EMPIRES:");
        for (idx, row) in rows.iter().take(16).enumerate() {
            buffer.write_text(idx + 2, 0, row, classic::body_style());
        }
        draw_command_prompt(&mut buffer, 19, "FIRST TIME COMMAND", "SLAP A KEY");
        Ok(buffer)
    }
}

impl Screen for FirstTimeEmpiresScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_rows(&[])
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenFirstTimeMenu
    }
}

impl FirstTimeIntroScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_page(
        &mut self,
        page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        crate::screen::layout::draw_centered_text(
            &mut buffer,
            0,
            "Esterian Conquest Ver 1.60",
            classic::bright_style(),
        );
        let lines = FIRST_TIME_INTRO_PAGES
            .get(page)
            .copied()
            .unwrap_or(FIRST_TIME_INTRO_PAGES.last().copied().unwrap_or(&[]));
        for (row, line) in lines.iter().enumerate() {
            buffer.write_text(row + 2, 0, line, classic::body_style());
        }
        let prompt = if page + 1 < FIRST_TIME_INTRO_PAGES.len() {
            "Press any key for the next section."
        } else {
            "Press any key to return to the First Time Menu."
        };
        draw_plain_prompt(&mut buffer, 19, prompt);
        Ok(buffer)
    }
}

impl Screen for FirstTimeIntroScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_page(0)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenFirstTimeMenu
    }
}

pub const FIRST_TIME_INTRO_PAGE_COUNT: usize = FIRST_TIME_INTRO_PAGES.len();

const FIRST_TIME_INTRO_PAGE_1: [&str; 14] = [
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

const FIRST_TIME_INTRO_PAGE_2: [&str; 8] = [
    "In profound respect and admiration to Bentley C. Griffith and his",
    "fellow pioneers, who between 1990 and 1992 forged the enduring legend",
    "of Esterian Conquest-a digital realm where star empires rose and fell",
    "across BBS screens-and to the ancient dreamers, strategists, and",
    "storytellers whose timeless visions of galactic dominion first lit the",
    "way for every commander who still dares to claim worlds among these",
    "endless stars.",
    "",
];

const FIRST_TIME_INTRO_PAGES: [&[&str]; 2] =
    [&FIRST_TIME_INTRO_PAGE_1, &FIRST_TIME_INTRO_PAGE_2];
