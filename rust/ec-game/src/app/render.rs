use super::state::App;
use crate::screen::ScreenId;
use crate::screen::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::terminal::Terminal;

impl App {
    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::domains;

        let playfield = match self.current_screen {
            ScreenId::Startup(_)
            | ScreenId::FirstTimeMenu
            | ScreenId::FirstTimeHelp
            | ScreenId::FirstTimeEmpires
            | ScreenId::FirstTimeIntro
            | ScreenId::FirstTimePreloadedRenamePrompt
            | ScreenId::FirstTimeJoinEmpireName
            | ScreenId::FirstTimeJoinEmpireConfirm
            | ScreenId::FirstTimeJoinSummary
            | ScreenId::FirstTimeJoinNoPending
            | ScreenId::FirstTimeHomeworldName
            | ScreenId::FirstTimeHomeworldConfirm
            | ScreenId::ColonyWorldName
            | ScreenId::ColonyWorldConfirm
            | ScreenId::ThemePicker
            | ScreenId::MainMenu
            | ScreenId::MainHelp
            | ScreenId::GeneralMenu
            | ScreenId::GeneralHelp
            | ScreenId::Reports => domains::startup::views::render(self)?,

            ScreenId::FleetHelp
            | ScreenId::FleetMenu
            | ScreenId::FleetList
            | ScreenId::FleetReview
            | ScreenId::FleetOrder
            | ScreenId::FleetGroupOrder
            | ScreenId::FleetMissionPicker
            | ScreenId::FleetTransfer
            | ScreenId::FleetDetach
            | ScreenId::FleetEta => domains::fleet::views::render(self)?,

            ScreenId::StarbaseMenu
            | ScreenId::StarbaseHelp
            | ScreenId::StarbaseList
            | ScreenId::StarbaseReviewSelect
            | ScreenId::StarbaseReview => domains::starbase::views::render(self)?,

            ScreenId::PlanetMenu
            | ScreenId::PlanetHelp
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_)
            | ScreenId::PlanetCommissionPicker
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetCommissionDraft
            | ScreenId::PlanetCommissionResult
            | ScreenId::PlanetAutoCommissionReport
            | ScreenId::PlanetBuildHelp
            | ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity
            | ScreenId::PlanetListSortPrompt(_)
            | ScreenId::PlanetBriefList(_, _)
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseFilterPrompt
            | ScreenId::PlanetInfoDetail => domains::planet::views::render(self)?,

            ScreenId::Enemies
            | ScreenId::EmpireStatus
            | ScreenId::EmpireProfile
            | ScreenId::Rankings(_) => domains::empire::views::render(self)?,

            ScreenId::ComposeMessageRecipient
            | ScreenId::ComposeMessageSubject
            | ScreenId::ComposeMessageBody
            | ScreenId::ComposeMessageOutbox
            | ScreenId::ComposeMessageDiscardConfirm
            | ScreenId::ComposeMessageSendConfirm
            | ScreenId::ComposeMessageSent => domains::messaging::views::render(self)?,

            ScreenId::Starmap | ScreenId::PartialStarmapView => {
                domains::starmap::views::render(self)?
            }
        };
        assert_eq!(
            playfield.width(),
            PLAYFIELD_WIDTH,
            "screen {:?} rendered width {} instead of {}",
            self.current_screen,
            playfield.width(),
            PLAYFIELD_WIDTH
        );
        assert_eq!(
            playfield.height(),
            PLAYFIELD_HEIGHT,
            "screen {:?} rendered height {} instead of {}",
            self.current_screen,
            playfield.height(),
            PLAYFIELD_HEIGHT
        );
        if let Some((column, row)) = playfield.cursor() {
            assert!(
                usize::from(column) < PLAYFIELD_WIDTH && usize::from(row) < PLAYFIELD_HEIGHT,
                "screen {:?} set cursor ({column},{row}) outside {PLAYFIELD_WIDTH}x{PLAYFIELD_HEIGHT}",
                self.current_screen
            );
        }
        terminal.render(&playfield)
    }
}
