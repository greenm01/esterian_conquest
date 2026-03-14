use std::fs;
use std::path::PathBuf;

use ec_data::{CoreGameData, DatabaseDat, build_player_starmap_projection};

use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::ReportsPreview;
use crate::screen::{
    EmpireProfileScreen, EmpireStatusScreen, GeneralMenuScreen, MainMenuScreen, PlanetInfoScreen,
    PartialStarmapScreen, RankingsScreen, RankingsView, ReportsScreen, Screen, ScreenFrame,
    ScreenId, StartupScreen, StarmapScreen,
};
use crate::startup::{StartupPhase, StartupSequence, StartupSummary};
use crate::terminal::Terminal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub game_dir: PathBuf,
    pub player_record_index_1_based: usize,
    pub export_root: Option<PathBuf>,
    pub queue_dir: Option<PathBuf>,
}

pub struct App {
    game_dir: PathBuf,
    game_data: CoreGameData,
    database: DatabaseDat,
    player: PlayerContext,
    current_screen: ScreenId,
    startup_sequence: StartupSequence,
    startup: StartupScreen,
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    starmap: StarmapScreen,
    partial_starmap: PartialStarmapScreen,
    planet_info: PlanetInfoScreen,
    empire_status: EmpireStatusScreen,
    empire_profile: EmpireProfileScreen,
    rankings: RankingsScreen,
    reports: ReportsScreen,
    planet_info_input: String,
    planet_info_error: Option<String>,
    planet_info_selected: Option<usize>,
    partial_starmap_input: String,
    partial_starmap_error: Option<String>,
    partial_starmap_center: [u8; 2],
    starmap_view_x: usize,
    starmap_view_y: usize,
    starmap_status: Option<String>,
    starmap_dump_lines: Vec<String>,
    starmap_dump_offset: usize,
    starmap_dump_active: bool,
    starmap_capture_complete: bool,
    export_root: PathBuf,
    queue_dir: Option<PathBuf>,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_dir = config.game_dir.clone();
        let export_root = config
            .export_root
            .clone()
            .unwrap_or_else(|| game_dir.join("exports"));
        let game_data = CoreGameData::load(&game_dir)?;
        let database = DatabaseDat::parse(&fs::read(game_dir.join("DATABASE.DAT"))?)?;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let pending_results = file_nonempty(game_dir.join("RESULTS.DAT"));
        let reports = ReportsPreview::load(&game_dir)?;
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
            game_dir,
            game_data,
            database,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            main_menu: MainMenuScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            starmap: StarmapScreen::new(),
            partial_starmap: PartialStarmapScreen::new(),
            planet_info: PlanetInfoScreen::new(),
            empire_status: EmpireStatusScreen::new(),
            empire_profile: EmpireProfileScreen::new(),
            rankings: RankingsScreen::new(),
            reports: ReportsScreen::new(reports, review_summary),
            planet_info_input: String::new(),
            planet_info_error: None,
            planet_info_selected: None,
            partial_starmap_input: "8,2".to_string(),
            partial_starmap_error: None,
            partial_starmap_center: [8, 2],
            starmap_view_x: 1,
            starmap_view_y: 1,
            starmap_status: None,
            starmap_dump_lines: Vec::new(),
            starmap_dump_offset: 0,
            starmap_dump_active: false,
            starmap_capture_complete: false,
            export_root,
            queue_dir: config.queue_dir,
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
            ScreenId::Starmap if self.starmap_capture_complete => self.starmap.render_complete()?,
            ScreenId::Starmap if self.starmap_dump_active => self.starmap.render_dump_page(
                &self.starmap_dump_lines,
                self.starmap_dump_offset,
            )?,
            ScreenId::Starmap => self.starmap.render_prompt(self.starmap_status.as_deref())?,
            ScreenId::PartialStarmapPrompt => self
                .partial_starmap
                .render_prompt(&self.partial_starmap_input, self.partial_starmap_error.as_deref())?,
            ScreenId::PartialStarmapView => self.partial_starmap.render_view(
                &frame,
                &self.database,
                self.partial_starmap_center,
            )?,
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
            ScreenId::Starmap if self.starmap_capture_complete => {
                self.starmap.handle_complete_key(key)
            }
            ScreenId::Starmap if self.starmap_dump_active => self.starmap.handle_dump_key(key),
            ScreenId::Starmap => self.starmap.handle_prompt_key(key),
            ScreenId::PartialStarmapPrompt => self.partial_starmap.handle_prompt_key(key),
            ScreenId::PartialStarmapView => self.partial_starmap.handle_view_key(key),
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

    pub fn open_partial_starmap_prompt(&mut self) {
        let default = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| planet.coords_raw())
            .unwrap_or([8, 2]);
        self.partial_starmap_input = format!("{},{}", default[0], default[1]);
        self.partial_starmap_error = None;
        self.partial_starmap_center = default;
        self.current_screen = ScreenId::PartialStarmapPrompt;
    }

    pub fn append_partial_starmap_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PartialStarmapPrompt && self.partial_starmap_input.len() < 16 {
            self.partial_starmap_input.push(ch);
            self.partial_starmap_error = None;
        }
    }

    pub fn backspace_partial_starmap_input(&mut self) {
        if self.current_screen == ScreenId::PartialStarmapPrompt {
            self.partial_starmap_input.pop();
            self.partial_starmap_error = None;
        }
    }

    pub fn submit_partial_starmap_prompt(&mut self) {
        let Some(coords) = crate::screen::parse_planet_coords(&self.partial_starmap_input) else {
            self.partial_starmap_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let map_size = ec_data::map_size_for_player_count(self.game_data.conquest.player_count());
        if coords[0] == 0 || coords[1] == 0 || coords[0] > map_size || coords[1] > map_size {
            self.partial_starmap_error =
                Some(format!("Enter coordinates within 1..{map_size}"));
            return;
        }
        self.partial_starmap_center = coords;
        self.partial_starmap_error = None;
        self.current_screen = ScreenId::PartialStarmapView;
    }

    pub fn move_partial_starmap(&mut self, dx: i8, dy: i8) {
        let map_size = ec_data::map_size_for_player_count(self.game_data.conquest.player_count());
        self.partial_starmap_center[0] = self.partial_starmap_center[0]
            .saturating_add_signed(dx)
            .clamp(1, map_size);
        self.partial_starmap_center[1] = self.partial_starmap_center[1]
            .saturating_add_signed(dy)
            .clamp(1, map_size);
    }

    pub fn open_starmap(&mut self) {
        self.starmap_view_x = 1;
        self.starmap_view_y = 1;
        self.starmap_status = None;
        self.starmap_dump_lines.clear();
        self.starmap_dump_offset = 0;
        self.starmap_dump_active = false;
        self.starmap_capture_complete = false;
        self.current_screen = ScreenId::Starmap;
    }

    pub fn export_starmap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection(
            &self.game_data,
            &self.database,
            self.player.record_index_1_based as u8,
        );
        std::fs::create_dir_all(&self.export_root)?;
        let filename = format!(
            "ECMAP-P{}-Y{}.TXT",
            self.player.record_index_1_based,
            self.game_data.conquest.game_year()
        );
        let export_path = self.export_root.join(&filename);
        let csv_path = self.export_root.join(filename.replace(".TXT", ".CSV"));
        let details_csv_path = self
            .export_root
            .join(filename.replace(".TXT", "-DETAILS.CSV"));
        std::fs::write(&export_path, projection.render_ascii_export())?;
        std::fs::write(&csv_path, projection.render_csv_export())?;
        std::fs::write(&details_csv_path, projection.render_csv_details_export())?;
        if let Some(queue_dir) = &self.queue_dir {
            std::fs::create_dir_all(queue_dir)?;
            std::fs::copy(&export_path, queue_dir.join(&filename))?;
            std::fs::copy(&csv_path, queue_dir.join(csv_path.file_name().unwrap()))?;
            std::fs::copy(
                &details_csv_path,
                queue_dir.join(details_csv_path.file_name().unwrap()),
            )?;
            self.starmap_status = Some(format!(
                "Exported TXT + grid CSV + details CSV and queued copies in {}",
                queue_dir.display()
            ));
        } else {
            self.starmap_status = Some(format!(
                "Exported {}, {}, and {}",
                export_path.display(),
                csv_path.display(),
                details_csv_path.display()
            ));
        }
        Ok(())
    }

    pub fn starmap_dump_text(&self) -> String {
        build_player_starmap_projection(
            &self.game_data,
            &self.database,
            self.player.record_index_1_based as u8,
        )
        .render_ascii_map()
    }

    pub fn begin_starmap_dump(&mut self) {
        self.starmap_dump_lines = self
            .starmap_dump_text()
            .lines()
            .map(|line| line.to_string())
            .collect();
        self.starmap_dump_offset = 0;
        self.starmap_dump_active = true;
        self.starmap_capture_complete = false;
    }

    pub fn advance_starmap_page(&mut self) {
        const PAGE_LINES: usize = 16;
        if !self.starmap_dump_active {
            return;
        }
        let next_offset = self.starmap_dump_offset.saturating_add(PAGE_LINES);
        if next_offset >= self.starmap_dump_lines.len() {
            self.starmap_dump_active = false;
            self.starmap_capture_complete = true;
        } else {
            self.starmap_dump_offset = next_offset;
        }
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
