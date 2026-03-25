use super::state::App;
use crate::screen::ScreenId;
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
            | ScreenId::PlanetBuildReview
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
        terminal.render(&playfield)
    }
}
