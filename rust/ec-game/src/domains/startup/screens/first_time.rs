use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::startup::StartupAction;
use crate::screen::layout::{
    CommandMessage, MenuEntry, ScreenGeometry, dismiss_prompt_row,
    draw_command_line_default_input_at, draw_command_line_prompt_text_at,
    draw_command_message_stack, draw_command_prompt_at, draw_dismiss_prompt, draw_menu_notice,
    draw_plain_prompt, draw_title_bar, menu_prompt_row, new_playfield,
};
use crate::screen::{COMMAND_LABEL, PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords};
use crate::theme::classic;

pub struct FirstTimeMenuScreen;
pub struct FirstTimeEmpiresScreen;
pub struct FirstTimeIntroScreen;

const FIRST_TIME_ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "Q", "uit back to BBS"),
    MenuEntry::new(28, "J", "oin this game"),
    MenuEntry::new(55, "V", "iew Game Introduction"),
];

impl FirstTimeMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        status: Option<&str>,
        door_mode: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "FIRST TIME MENU:");
        crate::screen::layout::draw_menu_row(&mut buffer, 1, &first_time_row_1(door_mode));
        crate::screen::layout::draw_menu_row(&mut buffer, 2, &FIRST_TIME_ROW_2);
        let command_row = menu_prompt_row(2);
        if let Some(status) = status {
            draw_menu_notice(&mut buffer, command_row, status);
        }
        draw_command_prompt_at(
            &mut buffer,
            command_row,
            "FIRST TIME COMMAND",
            first_time_command_keys(door_mode),
        );
        Ok(buffer)
    }

    pub fn handle_key_for_mode(&self, key: KeyEvent, door_mode: bool) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Startup(StartupAction::OpenFirstTimeEmpires)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Startup(StartupAction::OpenFirstTimeIntro)
            }
            KeyCode::Char('a') | KeyCode::Char('A') if door_mode => Action::ToggleAnsiMode,
            KeyCode::Char('c') | KeyCode::Char('C') if !door_mode => {
                Action::Startup(StartupAction::OpenThemePicker)
            }
            KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Startup(StartupAction::OpenFirstTimeJoinName)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::RequestQuit,
            _ => Action::Noop,
        }
    }
}

impl Screen for FirstTimeMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render(None, false)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        self.handle_key_for_mode(key, false)
    }
}

pub fn render_first_time_reserved_prompt(
    reserved_alias: Option<&str>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "RESERVED PLAYER:");
    buffer.write_text(
        2,
        0,
        "This player seat is reserved for you.",
        classic::body_style(),
    );
    if let Some(alias) = reserved_alias {
        buffer.write_text(
            4,
            0,
            &format!("Reserved caller alias: \"{alias}\"."),
            classic::body_style(),
        );
    }
    buffer.write_text(
        6,
        0,
        "You may name your empire now and finish first-time setup.",
        classic::body_style(),
    );
    draw_command_line_prompt_text_at(
        &mut buffer,
        menu_prompt_row(6),
        COMMAND_LABEL,
        "Continue with reserved setup? Y/[N] ->",
    );
    Ok(buffer)
}

pub fn render_first_time_join_name(
    rename_mode: bool,
    reserved_mode: bool,
    reserved_alias: Option<&str>,
    current_empire_name: &str,
    input: &str,
    status: Option<&str>,
    door_mode: bool,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    if rename_mode {
        draw_title_bar(&mut buffer, 0, "FIRST LOGIN:");
        buffer.write_text(
            2,
            0,
            "This empire is already joined, and this is your first login.",
            classic::body_style(),
        );
        buffer.write_text(
            3,
            0,
            &format!("Your empire is currently named \"{current_empire_name}\"."),
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "Enter a new empire name (up to 20 characters).",
            classic::body_style(),
        );
        buffer.write_text(
            6,
            0,
            "Press Esc to keep the current empire name.",
            classic::body_style(),
        );
    } else if reserved_mode {
        draw_title_bar(&mut buffer, 0, "RESERVED PLAYER:");
        buffer.write_text(
            2,
            0,
            "This player seat is reserved for you.",
            classic::body_style(),
        );
        if let Some(alias) = reserved_alias {
            buffer.write_text(
                3,
                0,
                &format!("Reserved caller alias: \"{alias}\"."),
                classic::body_style(),
            );
        }
        buffer.write_text(
            5,
            0,
            "Enter the name of your empire (up to 20 characters).",
            classic::body_style(),
        );
        buffer.write_text(
            6,
            0,
            "Press Esc to return to the reserved player notice.",
            classic::body_style(),
        );
    } else {
        draw_title_bar(&mut buffer, 0, "FIRST TIME JOIN:");
        crate::screen::layout::draw_menu_row(&mut buffer, 1, &first_time_row_1(door_mode));
        crate::screen::layout::draw_menu_row(&mut buffer, 2, &FIRST_TIME_ROW_2);
        buffer.write_text(
            4,
            0,
            "Enter the name of your empire (up to 20 characters).",
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "Press Esc to back out to the First Time Menu.",
            classic::body_style(),
        );
    }
    let last_content_row = if rename_mode || reserved_mode { 6 } else { 5 };
    let command_row = menu_prompt_row(last_content_row);
    draw_command_line_default_input_at(
        &mut buffer,
        command_row,
        "EMPIRE NAME",
        if rename_mode {
            "Rename your empire "
        } else {
            "Name your empire "
        },
        "",
        input,
    );
    if let Some(status) = status {
        draw_command_message_stack(&mut buffer, command_row, &[CommandMessage::Notice(status)]);
    }
    Ok(buffer)
}

pub fn render_first_time_join_name_confirm(
    rename_mode: bool,
    reserved_mode: bool,
    empire_name: &str,
    door_mode: bool,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    if rename_mode {
        draw_title_bar(&mut buffer, 0, "FIRST LOGIN:");
        buffer.write_text(
            2,
            0,
            "Would you like to rename your empire? (This is your only chance.)",
            classic::body_style(),
        );
        buffer.write_text(
            4,
            0,
            "Press N or Esc to keep the current empire name.",
            classic::body_style(),
        );
    } else if reserved_mode {
        draw_title_bar(&mut buffer, 0, "RESERVED PLAYER:");
        buffer.write_text(
            2,
            0,
            "This reserved seat will join as the empire shown below.",
            classic::body_style(),
        );
        buffer.write_text(
            4,
            0,
            "Press N or Esc to go back and edit it before joining.",
            classic::body_style(),
        );
    } else {
        draw_title_bar(&mut buffer, 0, "FIRST TIME JOIN:");
        crate::screen::layout::draw_menu_row(&mut buffer, 1, &first_time_row_1(door_mode));
        crate::screen::layout::draw_menu_row(&mut buffer, 2, &FIRST_TIME_ROW_2);
        buffer.write_text(
            4,
            0,
            "Enter the name of your empire (up to 20 characters).",
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "Press N or Esc to go back and edit it before joining.",
            classic::body_style(),
        );
    }
    draw_command_line_prompt_text_at(
        &mut buffer,
        menu_prompt_row(if rename_mode { 4 } else { 5 }),
        "EMPIRE NAME",
        &format!(
            "\"{empire_name}\" <- Is this correct? {}/N ->",
            if rename_mode { "Y/[N]" } else { "[Y]" }
        ),
    );
    Ok(buffer)
}

fn first_time_command_keys(door_mode: bool) -> &'static str {
    if door_mode {
        "? L J A V <Q>"
    } else {
        "? L J C V <Q>"
    }
}

fn first_time_row_1(door_mode: bool) -> [MenuEntry<'static>; 3] {
    [
        MenuEntry::new(2, "H", "elp with commands"),
        MenuEntry::new(28, "L", "ist current empires"),
        if door_mode {
            MenuEntry::new(55, "A", "nsi color ON/OFF")
        } else {
            MenuEntry::new(55, "C", "olor Theme")
        },
    ]
}

pub fn render_preloaded_first_login_rename_prompt(
    empire_name: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "FIRST LOGIN:");
    buffer.write_text(
        2,
        0,
        "This empire is already joined, and this is your first login.",
        classic::body_style(),
    );
    buffer.write_text(
        4,
        0,
        &format!("Your empire is currently named \"{empire_name}\"."),
        classic::body_style(),
    );
    draw_command_line_prompt_text_at(
        &mut buffer,
        menu_prompt_row(4),
        "EMPIRE NAME",
        "Rename your empire? Y/[N] ->",
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
    buffer.write_text(
        4,
        0,
        &format!("The year is: {year} A.D."),
        classic::body_style(),
    );
    buffer.write_text(6, 0, "Last year on: NEVER", classic::body_style());
    buffer.write_text(
        8,
        0,
        "You have 60 minutes left to play.",
        classic::body_style(),
    );
    buffer.write_text(10, 0, "Autopilot is off.", classic::body_style());
    draw_plain_prompt(&mut buffer, dismiss_prompt_row(10), "(slap a key)");
    Ok(buffer)
}

pub fn render_first_time_join_no_pending() -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "JOIN COMPLETE:");
    buffer.write_text(2, 0, "You have no reports pending.", classic::body_style());
    buffer.write_text(4, 0, "You have no messages pending.", classic::body_style());
    draw_plain_prompt(&mut buffer, dismiss_prompt_row(4), "(slap a key)");
    Ok(buffer)
}

pub fn render_first_time_homeworld_name(
    coords: [u8; 2],
    present_production: u16,
    potential_production: u16,
    is_preloaded_first_login: bool,
    input: &str,
    status: Option<&str>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "HOMEWORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!(
            "You have a world in the solar system at {}. Its current production",
            format_sector_coords(coords)
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
    if is_preloaded_first_login {
        buffer.write_text(
            5,
            0,
            "This joined empire still needs its first homeworld name.",
            classic::body_style(),
        );
    }
    let last_content_row = if is_preloaded_first_login { 5 } else { 3 };
    let command_row = menu_prompt_row(last_content_row);
    draw_command_line_default_input_at(
        &mut buffer,
        command_row,
        "HOMEWORLD",
        "Name this world (20 chars or less) ",
        "",
        input,
    );
    if let Some(status) = status {
        draw_command_message_stack(&mut buffer, command_row, &[CommandMessage::Notice(status)]);
    }
    Ok(buffer)
}

pub fn render_first_time_homeworld_confirm(
    coords: [u8; 2],
    present_production: u16,
    potential_production: u16,
    is_preloaded_first_login: bool,
    homeworld_name: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "HOMEWORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!(
            "You have a world in the solar system at {}. Its current production",
            format_sector_coords(coords)
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
    if is_preloaded_first_login {
        buffer.write_text(
            5,
            0,
            "This joined empire still needs its first homeworld name.",
            classic::body_style(),
        );
    }
    buffer.write_text(
        if is_preloaded_first_login { 6 } else { 5 },
        0,
        "Press N or Esc to go back and edit the homeworld name.",
        classic::body_style(),
    );
    draw_command_line_prompt_text_at(
        &mut buffer,
        menu_prompt_row(if is_preloaded_first_login { 6 } else { 5 }),
        "HOMEWORLD",
        &format!("\"{homeworld_name}\" <- Is this correct? Y/[N] ->"),
    );
    Ok(buffer)
}

pub fn render_colony_world_name(
    coords: [u8; 2],
    present_production: u16,
    potential_production: u16,
    input: &str,
    status: Option<&str>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "WORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!(
            "You own a newly colonized world in the solar system at {}. Its current",
            format_sector_coords(coords)
        ),
        classic::body_style(),
    );
    buffer.write_text(
        3,
        0,
        &format!(
            "production level is {} out of a possible {} points.",
            present_production, potential_production
        ),
        classic::body_style(),
    );
    let command_row = menu_prompt_row(3);
    draw_command_line_default_input_at(
        &mut buffer,
        command_row,
        "WORLD NAME",
        "Name this world (20 chars or less) ",
        "",
        input,
    );
    if let Some(status) = status {
        draw_command_message_stack(&mut buffer, command_row, &[CommandMessage::Notice(status)]);
    }
    Ok(buffer)
}

pub fn render_colony_world_confirm(
    coords: [u8; 2],
    planet_name: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield();
    draw_title_bar(&mut buffer, 0, "WORLD NAMING:");
    buffer.write_text(
        2,
        0,
        &format!(
            "Confirm the new name for your world at {}.",
            format_sector_coords(coords)
        ),
        classic::body_style(),
    );
    buffer.write_text(
        4,
        0,
        "Press N or Esc to go back and edit the world name.",
        classic::body_style(),
    );
    draw_command_line_prompt_text_at(
        &mut buffer,
        menu_prompt_row(4),
        "WORLD NAME",
        &format!("\"{planet_name}\" <- Is this correct? [Y]/N ->"),
    );
    Ok(buffer)
}

impl FirstTimeEmpiresScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_rows(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[String],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        draw_title_bar(&mut buffer, 0, "CURRENT EMPIRES:");
        let start_row = 2usize;
        let visible_rows =
            crate::screen::layout::command_line_row_for(geometry).saturating_sub(start_row + 2);
        let mut last_content_row = 0usize;
        for (idx, row) in rows.iter().take(visible_rows).enumerate() {
            let render_row = start_row + idx;
            buffer.write_text(render_row, 0, row, classic::body_style());
            last_content_row = render_row;
        }
        let last_content_row = if rows.is_empty() { 0 } else { last_content_row };
        draw_dismiss_prompt(
            &mut buffer,
            crate::screen::layout::dismiss_prompt_row_for(geometry, last_content_row),
        );
        Ok(buffer)
    }
}

impl Screen for FirstTimeEmpiresScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_rows(frame.geometry, &[])
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Startup(StartupAction::OpenFirstTimeMenu)
    }
}

impl FirstTimeIntroScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_page(
        &mut self,
        geometry: ScreenGeometry,
        page: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        crate::screen::startup::render_game_intro_page(
            geometry,
            page,
            "(Slap a key to return to the First Time Menu)",
        )
    }
}

impl Screen for FirstTimeIntroScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_page(frame.geometry, 0)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Startup(StartupAction::OpenFirstTimeMenu)
    }
}

pub const FIRST_TIME_INTRO_PAGE_COUNT: usize = crate::screen::startup::STARTUP_INTRO_PAGE_COUNT;
