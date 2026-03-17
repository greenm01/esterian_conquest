use crate::app::action::Action;
use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOutcome {
    Continue,
    Quit,
}

pub fn apply_action(app: &mut App, action: Action) -> AppOutcome {
    match action {
        // Domain dispatch
        Action::Startup(act) => {
            crate::domains::startup::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Fleet(act) => {
            crate::domains::fleet::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Planet(act) => {
            crate::domains::planet::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Starbase(act) => {
            crate::domains::starbase::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Empire(act) => {
            crate::domains::empire::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Messaging(act) => {
            crate::domains::messaging::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Starmap(act) => {
            crate::domains::starmap::update::update(app, act);
            AppOutcome::Continue
        }

        // Top-level app actions
        Action::DismissModalNotice => {
            app.dismiss_modal_notice();
            AppOutcome::Continue
        }
        Action::OpenMainMenu => {
            app.open_main_menu();
            AppOutcome::Continue
        }
        Action::OpenMainHelp => {
            app.open_main_help();
            AppOutcome::Continue
        }
        Action::OpenGeneralMenu => {
            app.open_general_menu();
            AppOutcome::Continue
        }
        Action::OpenGeneralHelp => {
            *app.current_screen_mut() = crate::screen::ScreenId::GeneralHelp;
            AppOutcome::Continue
        }
        Action::ShowAnsiAlwaysOnNotice => {
            app.show_first_time_ansi_notice();
            AppOutcome::Continue
        }
        Action::ShowAnsiAlwaysOnMainMenu => {
            app.show_main_menu_ansi_notice();
            AppOutcome::Continue
        }
        Action::ShowFleetExpertModeNotice => {
            app.show_fleet_expert_mode_notice();
            AppOutcome::Continue
        }
        Action::ReturnToCommandMenu => {
            app.return_to_command_menu();
            AppOutcome::Continue
        }
        Action::ToggleAutopilot => match app.toggle_autopilot() {
            Ok(()) | Err(_) => AppOutcome::Continue,
        },
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
