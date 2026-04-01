use crate::app::state::App;
use crate::domains::starbase::StarbaseAction;

pub fn update(app: &mut App, action: StarbaseAction) {
    match action {
        StarbaseAction::OpenMenu => app.open_starbase_menu(),
        StarbaseAction::OpenList => app.open_starbase_list(),
        StarbaseAction::OpenReviewSelect => app.open_starbase_review_select(),
        StarbaseAction::OpenReview => app.open_starbase_review(),
        StarbaseAction::OpenMovePrompt => app.open_starbase_move_prompt(),
        StarbaseAction::MoveSelect(delta) => app.move_starbase_select(delta),
        StarbaseAction::AppendChar(ch) => app.append_starbase_char(ch),
        StarbaseAction::BackspaceInput => app.backspace_starbase_input(),
        StarbaseAction::SubmitReviewSelect => app.submit_starbase_review_select(),
        StarbaseAction::AppendMovePromptChar(ch) => app.append_starbase_move_prompt_char(ch),
        StarbaseAction::BackspaceMovePromptInput => app.backspace_starbase_move_prompt_input(),
        StarbaseAction::SubmitMovePrompt => {
            if let Err(err) = app.submit_starbase_move_prompt() {
                app.starbase.move_prompt_status = Some(err);
            }
        }
        StarbaseAction::CancelMovePrompt => app.cancel_starbase_move_prompt(),
    }
}
