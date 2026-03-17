use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::startup::StartupAction;
use crate::domains::planet::PlanetAction;
use crate::domains::messaging::MessagingAction;
use crate::domains::empire::EmpireAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    CMD_COL_1, CMD_COL_2, MenuEntry, draw_command_center, draw_wrapped_status, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};

pub struct GeneralMenuScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "I", "nfo about a Planet"),
    MenuEntry::new(51, "C", "ommunicate (send message)"),
];

const ROW_1_RIGHT: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "A", ""),
    MenuEntry::new(51, "R", "eview messages/results"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to main menu"),
    MenuEntry::new(CMD_COL_2, "S", "tatus, your"),
    MenuEntry::new(51, "D", "elete ALL messages/results"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(CMD_COL_2, "P", "rofile of your empire"),
    MenuEntry::new(51, "O", "ther empires (rankings)"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew Partial Starmap"),
    MenuEntry::new(CMD_COL_2, "M", "ap of the galaxy"),
    MenuEntry::new(51, "E", "nemies, declare or list"),
];

impl GeneralMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        frame: &ScreenFrame<'_>,
        notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_command_center(
            &mut buffer,
            "GENERAL COMMAND CENTER:",
            &TOP_ROW,
            &[&autopilot_row(frame), &ROW_2, &ROW_3, &ROW_4],
            "GENERAL COMMAND",
            "H,Q,X,V,I,A,S,P,M,C,R,D,O,E",
        );
        if let Some(notice) = notice {
            draw_wrapped_status(&mut buffer, 16, 3, "Notice: ", notice);
        }
        Ok(buffer)
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(frame, None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenGeneralHelp,
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::General))
            }
            KeyCode::Char('a') | KeyCode::Char('A') => Action::ToggleAutopilot,
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Messaging(MessagingAction::OpenComposeRecipient)
            }
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Empire(EmpireAction::OpenEnemies),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Starmap(StarmapAction::OpenFull),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialPrompt(CommandMenu::General))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Empire(EmpireAction::OpenStatus),
            KeyCode::Char('p') | KeyCode::Char('P') => Action::Empire(EmpireAction::OpenProfile),
            KeyCode::Char('d') | KeyCode::Char('D') => {
                Action::Messaging(MessagingAction::OpenDeleteReviewables)
            }
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Empire(EmpireAction::OpenRankingsTable(
                ec_data::EmpireProductionRankingSort::Production,
            )),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Startup(StartupAction::OpenReports),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}

fn autopilot_row(_frame: &ScreenFrame<'_>) -> [MenuEntry<'static>; 3] {
    [
        MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
        MenuEntry::new(CMD_COL_2, "A", "utopilot ON/OFF"),
        ROW_1_RIGHT[1],
    ]
}
