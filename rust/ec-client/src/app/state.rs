use std::fs;
use std::path::PathBuf;

use ec_data::CoreGameData;

use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::ReportsPreview;
use crate::screen::{
    EmpireProfileScreen, EmpireStatusScreen, GeneralMenuScreen, MainMenuScreen, PlanetInfoScreen,
    RankingsScreen, RankingsView, ReportsScreen, Screen, ScreenFrame, ScreenId, StartupScreen,
};
use crate::startup::{StartupPhase, StartupSequence, StartupSummary};
use crate::terminal::Terminal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub game_dir: PathBuf,
    pub player_record_index_1_based: usize,
}

pub struct App {
    game_dir: PathBuf,
    game_data: CoreGameData,
    player: PlayerContext,
    current_screen: ScreenId,
    startup_sequence: StartupSequence,
    startup: StartupScreen,
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    planet_info: PlanetInfoScreen,
    empire_status: EmpireStatusScreen,
    empire_profile: EmpireProfileScreen,
    rankings: RankingsScreen,
    reports: ReportsScreen,
    planet_info_input: String,
    planet_info_error: Option<String>,
    planet_info_selected: Option<usize>,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_data = CoreGameData::load(&config.game_dir)?;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let pending_results = file_nonempty(config.game_dir.join("RESULTS.DAT"));
        let reports = ReportsPreview::load(&config.game_dir)?;
        let main_menu_summary = MainMenuSummary::from_game_data(
            &game_data,
            config.player_record_index_1_based,
            pending_results,
        );
        let review_summary = ReviewSummary::from_main_menu(&main_menu_summary);
        let startup_summary = StartupSummary::from_reports(
            main_menu_summary.game_year,
            main_menu_summary.pending_results,
            main_menu_summary.pending_messages,
            &reports,
        );
        let startup_sequence = StartupSequence::new(&startup_summary);

        Ok(Self {
            game_dir: config.game_dir,
            game_data,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            main_menu: MainMenuScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            planet_info: PlanetInfoScreen::new(),
            empire_status: EmpireStatusScreen::new(),
            empire_profile: EmpireProfileScreen::new(),
            rankings: RankingsScreen::new(),
            reports: ReportsScreen::new(reports, review_summary),
            planet_info_input: String::new(),
            planet_info_error: None,
            planet_info_selected: None,
        })
    }

    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame = ScreenFrame {
            game_dir: &self.game_dir,
            game_data: &self.game_data,
            player: &self.player,
        };

        let playfield = match self.current_screen {
            ScreenId::Startup(phase) => self.startup.render_phase(&frame, phase)?,
            ScreenId::MainMenu => self.main_menu.render(&frame)?,
            ScreenId::GeneralMenu => self.general_menu.render(&frame)?,
            ScreenId::PlanetInfoPrompt => self
                .planet_info
                .render_prompt(&self.planet_info_input, self.planet_info_error.as_deref())?,
            ScreenId::PlanetInfoDetail => self.planet_info.render_detail(
                &frame,
                self.planet_info_selected.ok_or("planet info detail not selected")?,
            )?,
            ScreenId::EmpireStatus => self.empire_status.render(&frame)?,
            ScreenId::EmpireProfile => self.empire_profile.render(&frame)?,
            ScreenId::Rankings(RankingsView::Prompt) => self.rankings.render_prompt(&frame)?,
            ScreenId::Rankings(RankingsView::Table(sort)) => {
                self.rankings.render_table(&frame, sort)?
            }
            ScreenId::Reports => self.reports.render(&frame)?,
        };
        terminal.render(&playfield)
    }

    pub fn current_screen(&self) -> ScreenId {
        self.current_screen
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenId {
        &mut self.current_screen
    }

    pub fn advance_startup(&mut self) {
        let next = self.startup_sequence.advance();
        self.current_screen = match next {
            StartupPhase::Complete => ScreenId::MainMenu,
            phase => ScreenId::Startup(phase),
        };
    }

    pub fn open_startup_intro(&mut self) {
        let next = self.startup_sequence.open_intro();
        self.current_screen = ScreenId::Startup(next);
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        match self.current_screen {
            ScreenId::Startup(phase) => self.startup.handle_key(phase, key),
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::PlanetInfoPrompt => self.handle_planet_info_prompt_key(key),
            ScreenId::PlanetInfoDetail => self.planet_info.handle_detail_key(key),
            ScreenId::EmpireStatus => self.empire_status.handle_key(key),
            ScreenId::EmpireProfile => self.empire_profile.handle_key(key),
            ScreenId::Rankings(RankingsView::Prompt) => self.rankings.handle_prompt_key(key),
            ScreenId::Rankings(RankingsView::Table(_)) => self.rankings.handle_table_key(key),
            ScreenId::Reports => self.reports.handle_key(key),
        }
    }

    pub fn open_planet_info_prompt(&mut self) {
        self.planet_info_input = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| {
                let [x, y] = planet.coords_raw();
                format!("{x},{y}")
            })
            .unwrap_or_default();
        self.planet_info_error = None;
        self.planet_info_selected = None;
        self.current_screen = ScreenId::PlanetInfoPrompt;
    }

    pub fn append_planet_info_char(&mut self, ch: char) {
        if self.planet_info_input.len() < 16 {
            self.planet_info_input.push(ch);
            self.planet_info_error = None;
        }
    }

    pub fn backspace_planet_info_input(&mut self) {
        self.planet_info_input.pop();
        self.planet_info_error = None;
    }

    pub fn submit_planet_info_prompt(&mut self) {
        let Some(coords) = crate::screen::parse_planet_coords(&self.planet_info_input) else {
            self.planet_info_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            self.planet_info_error = Some(format!(
                "No world found at X={}, Y={}",
                coords[0], coords[1]
            ));
            return;
        };

        self.planet_info_selected = Some(planet_idx);
        self.planet_info_error = None;
        self.current_screen = ScreenId::PlanetInfoDetail;
    }

    pub fn planet_info_input(&self) -> &str {
        &self.planet_info_input
    }

    pub fn selected_planet_info(&self) -> Option<usize> {
        self.planet_info_selected
    }

    fn handle_planet_info_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::OpenGeneralMenu
            }
            KeyCode::Enter => crate::app::Action::SubmitPlanetInfoPrompt,
            KeyCode::Backspace => crate::app::Action::BackspacePlanetInfoInput,
            KeyCode::Char(ch)
                if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | ':' | '/' | '-') =>
            {
                crate::app::Action::AppendPlanetInfoChar(ch)
            }
            _ => crate::app::Action::Noop,
        }
    }
}

fn file_nonempty(path: PathBuf) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
}
