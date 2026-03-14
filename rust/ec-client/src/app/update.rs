use crate::app::action::Action;
use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOutcome {
    Continue,
    Quit,
}

pub fn apply_action(app: &mut App, action: Action) -> AppOutcome {
    match action {
        Action::AdvanceStartup => {
            app.advance_startup();
            AppOutcome::Continue
        }
        Action::OpenStartupIntro => {
            app.open_startup_intro();
            AppOutcome::Continue
        }
        Action::OpenMainMenu => {
            *app.current_screen_mut() = crate::screen::ScreenId::MainMenu;
            AppOutcome::Continue
        }
        Action::OpenGeneralMenu => {
            *app.current_screen_mut() = crate::screen::ScreenId::GeneralMenu;
            AppOutcome::Continue
        }
        Action::OpenStarmap => {
            app.open_starmap();
            AppOutcome::Continue
        }
        Action::BeginStarmapDump => AppOutcome::Continue,
        Action::ExportStarmap => match app.export_starmap() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::OpenPlanetInfoPrompt => {
            app.open_planet_info_prompt();
            AppOutcome::Continue
        }
        Action::AppendPlanetInfoChar(ch) => {
            app.append_planet_info_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetInfoInput => {
            app.backspace_planet_info_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetInfoPrompt => {
            app.submit_planet_info_prompt();
            AppOutcome::Continue
        }
        Action::OpenEmpireStatus => {
            *app.current_screen_mut() = crate::screen::ScreenId::EmpireStatus;
            AppOutcome::Continue
        }
        Action::OpenEmpireProfile => {
            *app.current_screen_mut() = crate::screen::ScreenId::EmpireProfile;
            AppOutcome::Continue
        }
        Action::OpenRankingsPrompt => {
            *app.current_screen_mut() =
                crate::screen::ScreenId::Rankings(crate::screen::RankingsView::Prompt);
            AppOutcome::Continue
        }
        Action::OpenRankingsTable(sort) => {
            *app.current_screen_mut() =
                crate::screen::ScreenId::Rankings(crate::screen::RankingsView::Table(sort));
            AppOutcome::Continue
        }
        Action::OpenReports => {
            *app.current_screen_mut() = crate::screen::ScreenId::Reports;
            AppOutcome::Continue
        }
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
