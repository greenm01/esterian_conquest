use super::state::App;
use crate::app::help::popup_for_screen;
use crate::screen::{CommandMenu, PlanetListMode, ScreenId};

impl App {
    pub(crate) fn log_action_error(&self, action: &'static str, err: &dyn std::error::Error) {
        tracing::error!(
            action,
            player = self.player.record_index_1_based,
            screen = ?self.current_screen,
            error = %err,
            "nc-game action failed"
        );
    }

    pub fn current_screen(&self) -> ScreenId {
        self.current_screen
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenId {
        &mut self.current_screen
    }

    pub fn classic_login_state(&self) -> crate::model::ClassicLoginState {
        self.player.classic_login_state
    }

    pub fn clear_command_menu_notice(&mut self) {
        self.command_menu_notice = None;
    }

    pub fn open_popup_help(&mut self) {
        self.popup_help = popup_for_screen(self.current_screen, self.door_mode);
    }

    pub fn dismiss_popup_help(&mut self) {
        self.popup_help = None;
    }

    pub fn show_command_menu_notice(&mut self, menu: CommandMenu, message: impl Into<String>) {
        self.command_menu_notice = Some(message.into());
        self.command_return_menu = menu;
        self.return_screen = None;
        self.current_screen = match menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub fn open_main_menu(&mut self) {
        self.clear_command_menu_notice();
        self.return_screen = None;
        self.popup_help = None;
        self.current_screen = ScreenId::MainMenu;
    }

    pub fn open_general_menu(&mut self) {
        self.clear_command_menu_notice();
        self.return_screen = None;
        self.popup_help = None;
        self.current_screen = ScreenId::GeneralMenu;
    }

    pub fn toggle_expert_mode(&mut self) {
        self.expert_mode = !self.expert_mode;
    }

    pub fn return_to_command_menu(&mut self) {
        if let Some(screen) = self.return_screen.take() {
            self.popup_help = None;
            self.current_screen = screen;
            return;
        }
        self.popup_help = None;
        self.current_screen = match self.command_return_menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub(crate) fn origin_command_menu(&self) -> CommandMenu {
        match self.current_screen {
            ScreenId::MainMenu
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseFilterPrompt => CommandMenu::Main,
            ScreenId::FleetMenu
            | ScreenId::FleetList
            | ScreenId::FleetReview
            | ScreenId::FleetOrder
            | ScreenId::FleetGroupOrder
            | ScreenId::FleetMissionPicker
            | ScreenId::FleetTransfer
            | ScreenId::FleetDetach
            | ScreenId::FleetEta => CommandMenu::Fleet,
            ScreenId::StarbaseMenu
            | ScreenId::StarbaseList
            | ScreenId::StarbaseReviewSelect
            | ScreenId::StarbaseReview => CommandMenu::Starbase,
            ScreenId::GeneralMenu
            | ScreenId::Enemies
            | ScreenId::ComposeMessageRecipient
            | ScreenId::ComposeMessageSubject
            | ScreenId::ComposeMessageBody
            | ScreenId::ComposeMessageOutbox
            | ScreenId::ComposeMessageDiscardConfirm
            | ScreenId::ComposeMessageSendConfirm
            | ScreenId::ComposeMessageSent
            | ScreenId::EmpireStatus
            | ScreenId::EmpireProfile
            | ScreenId::Rankings(_)
            | ScreenId::Reports
            | ScreenId::Starmap => CommandMenu::General,
            ScreenId::PlanetMenu
            | ScreenId::PlanetCommissionPicker
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetCommissionDraft
            | ScreenId::PlanetCommissionResult
            | ScreenId::PlanetAutoCommissionReport
            | ScreenId::PlanetListSortPrompt(PlanetListMode::Brief)
            | ScreenId::PlanetListSortPrompt(PlanetListMode::Stub(_))
            | ScreenId::PlanetBriefList(PlanetListMode::Brief, _)
            | ScreenId::PlanetBriefList(PlanetListMode::Stub(_), _)
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_) => CommandMenu::Planet,
            ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity
            | ScreenId::PlanetListSortPrompt(PlanetListMode::BuildSelect)
            | ScreenId::PlanetBriefList(PlanetListMode::BuildSelect, _) => CommandMenu::PlanetBuild,
            ScreenId::Startup(_)
            | ScreenId::FirstTimeMenu
            | ScreenId::FirstTimeEmpires
            | ScreenId::FirstTimeIntro
            | ScreenId::FirstTimeReservedPrompt
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
            | ScreenId::PartialStarmapView
            | ScreenId::PlanetInfoDetail => self.command_return_menu,
        }
    }
}
