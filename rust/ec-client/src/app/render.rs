use super::state::App;
use crate::screen::ScreenId;
use crate::terminal::Terminal;

impl App {
    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::domains;

        let mut playfield = match self.current_screen {
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
            | ScreenId::FleetList(_)
            | ScreenId::FleetReviewSelect
            | ScreenId::FleetReview
            | ScreenId::FleetRoeSelect
            | ScreenId::FleetOrder
            | ScreenId::FleetGroupOrder
            | ScreenId::FleetMissionPicker
            | ScreenId::FleetMerge
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
            | ScreenId::PlanetAutoCommissionConfirm
            | ScreenId::PlanetAutoCommissionDone
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_)
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetBuildHelp
            | ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildReview
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildAbortConfirm
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity
            | ScreenId::PlanetListSortPrompt(_)
            | ScreenId::PlanetBriefList(_)
            | ScreenId::PlanetDetailList(_)
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseFilterPrompt
            | ScreenId::PlanetDatabaseDetail
            | ScreenId::PlanetInfoDetail => domains::planet::views::render(self)?,

            ScreenId::Enemies
            | ScreenId::EmpireStatus
            | ScreenId::EmpireProfile
            | ScreenId::Rankings(_) => domains::empire::views::render(self)?,

            ScreenId::DeleteReviewables
            | ScreenId::ComposeMessageRecipient
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
        if let Some(notice) = self.current_modal_notice() {
            crate::screen::draw_command_line_notice(&mut playfield, notice);
        }
        terminal.render(&playfield)
    }
}
