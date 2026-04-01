use crate::app::state::App;
use crate::domains::startup::StartupAction;

pub fn update(app: &mut App, action: StartupAction) {
    match action {
        StartupAction::Advance => app.advance_startup(),
        StartupAction::ScrollReview(delta) => app.scroll_startup_review(delta),
        StartupAction::SkipIntro => app.skip_startup_intro(),
        StartupAction::AcceptDefault => {
            app.startup_state.startup_status = None;
            if let Err(err) = app.startup_accept_default() {
                app.log_action_error("startup_accept_default", err.as_ref());
                app.startup_state.startup_status =
                    Some("Unable to continue startup right now. Please try again.".to_string());
            }
        }
        StartupAction::RejectChoice => {
            app.startup_state.startup_status = None;
            if let Err(err) = app.startup_reject_choice() {
                app.log_action_error("startup_reject_choice", err.as_ref());
                app.startup_state.startup_status =
                    Some("Unable to continue startup right now. Please try again.".to_string());
            }
        }
        StartupAction::EnableNonstop => {
            app.startup_state.startup_status = None;
            if let Err(err) = app.startup_enable_nonstop() {
                app.log_action_error("startup_enable_nonstop", err.as_ref());
                app.startup_state.startup_status =
                    Some("Unable to continue startup right now. Please try again.".to_string());
            }
        }
        StartupAction::OpenFirstTimeMenu => app.open_first_time_menu(),
        StartupAction::OpenFirstTimeEmpires => app.open_first_time_empires(),
        StartupAction::OpenFirstTimeIntro => app.open_first_time_intro(),
        StartupAction::OpenThemePicker => app.open_theme_picker(),
        StartupAction::MoveThemePicker(delta) => app.move_theme_picker_cursor(delta),
        StartupAction::AppendThemePickerChar(ch) => app.append_theme_picker_char(ch),
        StartupAction::BackspaceThemePickerInput => app.backspace_theme_picker_input(),
        StartupAction::ApplyThemePickerSelection => app.apply_theme_picker_selection(),
        StartupAction::ExitThemePicker => app.exit_theme_picker(),
        StartupAction::OpenFirstTimeJoinName => app.open_first_time_join_name(),
        StartupAction::AppendFirstTimeInputChar(ch) => app.append_first_time_input_char(ch),
        StartupAction::BackspaceFirstTimeInput => app.backspace_first_time_input(),
        StartupAction::SubmitFirstTimeInput => app.submit_first_time_input(),
        StartupAction::AcceptFirstTimePrompt => app.accept_first_time_prompt(),
        StartupAction::RejectFirstTimePrompt => app.reject_first_time_prompt(),
        StartupAction::OpenReports => app.open_reports(),
    }
}
