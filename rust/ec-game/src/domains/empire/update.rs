use crate::app::state::App;
use crate::domains::empire::EmpireAction;

pub fn update(app: &mut App, action: EmpireAction) {
    match action {
        EmpireAction::OpenStatus => app.open_empire_status(),
        EmpireAction::OpenProfile => app.open_empire_profile(),
        EmpireAction::OpenRankingsTable(sort) => app.open_rankings_table(sort),
        EmpireAction::OpenEnemies => app.open_enemies(),
        EmpireAction::ScrollEnemies(delta) => app.scroll_enemies(delta),
        EmpireAction::MoveEnemies(delta) => app.move_enemies_cursor(delta),
        EmpireAction::AppendEnemiesChar(ch) => app.append_enemies_char(ch),
        EmpireAction::BackspaceEnemiesInput => app.backspace_enemies_input(),
        EmpireAction::SubmitEnemiesInput => {
            if let Err(err) = app.submit_enemies_input() {
                eprintln!("submit enemies input failed: {err}");
            }
        }
    }
}
