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
        Action::OpenReports => {
            *app.current_screen_mut() = crate::screen::ScreenId::Reports;
            AppOutcome::Continue
        }
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
