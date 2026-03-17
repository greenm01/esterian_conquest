use crate::app::state::App;
use crate::domains::starmap::StarmapAction;

pub fn update(app: &mut App, action: StarmapAction) {
    match action {
        StarmapAction::OpenFull => app.open_starmap(),
        StarmapAction::OpenPartialPrompt(menu) => app.open_partial_starmap_prompt(menu),
        StarmapAction::BeginDump => app.begin_starmap_dump(),
        StarmapAction::AdvancePage => app.advance_starmap_page(),
        StarmapAction::Export => {
            if let Err(err) = app.export_starmap() {
                eprintln!("export starmap failed: {err}");
            }
        }
        StarmapAction::AppendPartialChar(ch) => app.append_partial_starmap_char(ch),
        StarmapAction::BackspacePartialInput => app.backspace_partial_starmap_input(),
        StarmapAction::SubmitPartialPrompt => app.submit_partial_starmap_prompt(),
        StarmapAction::MovePartial(dx, dy) => app.move_partial_starmap(dx, dy),
    }
}
