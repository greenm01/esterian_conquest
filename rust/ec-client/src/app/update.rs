use crate::app::action::Action;
use crate::screen::ScreenId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOutcome {
    Continue,
    Quit,
}

pub fn apply_action(current: &mut ScreenId, action: Action) -> AppOutcome {
    match action {
        Action::OpenMainMenu => {
            *current = ScreenId::MainMenu;
            AppOutcome::Continue
        }
        Action::OpenGeneralMenu => {
            *current = ScreenId::GeneralMenu;
            AppOutcome::Continue
        }
        Action::OpenReports => {
            *current = ScreenId::Reports;
            AppOutcome::Continue
        }
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
