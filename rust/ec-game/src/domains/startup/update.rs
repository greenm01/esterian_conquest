use crate::app::state::App;
use crate::domains::startup::StartupAction;

pub fn update(app: &mut App, action: StartupAction) {
    match action {
        StartupAction::Advance => app.advance_startup(),
        StartupAction::ScrollReview(delta) => app.scroll_startup_review(delta),
        StartupAction::SkipIntro => app.skip_startup_intro(),
        StartupAction::AcceptDefault => {
            if let Err(err) = app.startup_accept_default() {
                eprintln!("startup accept failed: {err}");
            }
        }
        StartupAction::RejectChoice => {
            if let Err(err) = app.startup_reject_choice() {
                eprintln!("startup reject failed: {err}");
            }
        }
        StartupAction::EnableNonstop => {
            if let Err(err) = app.startup_enable_nonstop() {
                eprintln!("startup nonstop transition failed: {err}");
            }
        }
        StartupAction::OpenFirstTimeMenu => app.open_first_time_menu(),
        StartupAction::OpenFirstTimeHelp => app.open_first_time_help(),
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
