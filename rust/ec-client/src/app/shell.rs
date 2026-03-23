use super::state::App;
use crate::screen::{
    CommandMenu, FleetDetachMode, FleetEtaMode, FleetMergeMode, FleetTransferMode, ScreenId,
};

impl App {
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
        self.current_screen = ScreenId::MainMenu;
    }

    pub fn open_main_help(&mut self) {
        self.clear_command_menu_notice();
        self.return_screen = None;
        self.current_screen = ScreenId::MainHelp;
    }

    pub fn open_general_menu(&mut self) {
        self.clear_command_menu_notice();
        self.return_screen = None;
        self.current_screen = ScreenId::GeneralMenu;
    }

    pub fn toggle_expert_mode(&mut self) {
        self.expert_mode = !self.expert_mode;
    }

    pub fn return_to_command_menu(&mut self) {
        if let Some(screen) = self.return_screen.take() {
            self.current_screen = screen;
            return;
        }
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
            | ScreenId::MainHelp
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseFilterPrompt
            | ScreenId::PlanetDatabaseDetail => CommandMenu::Main,
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
            | ScreenId::FleetEta => CommandMenu::Fleet,
            ScreenId::StarbaseMenu
            | ScreenId::StarbaseHelp
            | ScreenId::StarbaseList
            | ScreenId::StarbaseReviewSelect
            | ScreenId::StarbaseReview => CommandMenu::Starbase,
            ScreenId::GeneralMenu
            | ScreenId::GeneralHelp
            | ScreenId::Enemies
            | ScreenId::DeleteReviewables
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
            | ScreenId::PlanetHelp
            | ScreenId::PlanetAutoCommissionConfirm
            | ScreenId::PlanetAutoCommissionDone
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetListSortPrompt(_)
            | ScreenId::PlanetBriefList(_)
            | ScreenId::PlanetDetailList(_)
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_) => CommandMenu::Planet,
            ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildHelp
            | ScreenId::PlanetBuildReview
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildAbortConfirm
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity => CommandMenu::PlanetBuild,
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
            | ScreenId::PartialStarmapView
            | ScreenId::PlanetInfoDetail => self.command_return_menu,
        }
    }

    pub(crate) fn status_if_no_modal<'a>(&self, status: Option<&'a str>) -> Option<&'a str> {
        if self.current_modal_notice().is_some() {
            None
        } else {
            status
        }
    }

    pub(crate) fn current_modal_notice(&self) -> Option<&str> {
        match self.current_screen {
            ScreenId::StarbaseReviewSelect => self.starbase.review_status.as_deref(),
            ScreenId::FleetReviewSelect => self.fleet.review_status.as_deref(),
            ScreenId::FleetRoeSelect => self.fleet.roe_status.as_deref(),
            ScreenId::FleetOrder => self.fleet.order_status.as_deref(),
            ScreenId::FleetGroupOrder => self.fleet.group_status.as_deref(),
            ScreenId::FleetMissionPicker => self.fleet.mission_picker_status.as_deref(),
            ScreenId::FleetMerge => self.fleet.merge_status.as_deref(),
            ScreenId::FleetTransfer => self.fleet.transfer_status.as_deref(),
            ScreenId::FleetDetach => self.fleet.detach_status.as_deref(),
            ScreenId::FleetEta if self.fleet.eta_mode != FleetEtaMode::ShowingResult => {
                self.fleet.eta_status.as_deref()
            }
            ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseFilterPrompt => {
                self.planet.database_status.as_deref()
            }
            ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_) => self.planet.transport_status.as_deref(),
            ScreenId::Enemies => self.empire.enemies_status.as_deref(),
            ScreenId::ComposeMessageRecipient => self.messaging.compose_recipient_status.as_deref(),
            ScreenId::ComposeMessageOutbox => self.messaging.compose_outbox_status.as_deref(),
            _ => None,
        }
    }

    pub fn dismiss_modal_notice(&mut self) {
        match self.current_screen {
            ScreenId::StarbaseReviewSelect => {
                self.starbase.review_status = None;
                self.starbase.review_input.clear();
            }
            ScreenId::FleetReviewSelect => {
                self.fleet.review_status = None;
                self.fleet.review_select_input.clear();
            }
            ScreenId::FleetRoeSelect => {
                self.fleet.roe_status = None;
                if self.fleet.roe_editing {
                    self.fleet.roe_input.clear();
                } else {
                    self.fleet.roe_select_input.clear();
                }
            }
            ScreenId::FleetOrder => {
                self.fleet.order_status = None;
                self.fleet.order_input.clear();
            }
            ScreenId::FleetGroupOrder => {
                self.fleet.group_status = None;
                self.fleet.group_input.clear();
            }
            ScreenId::FleetMissionPicker => {
                self.fleet.mission_picker_status = None;
                self.fleet.mission_picker_input.clear();
            }
            ScreenId::FleetMerge => {
                self.fleet.merge_status = None;
                match self.fleet.merge_mode {
                    FleetMergeMode::SelectingSource => self.fleet.merge_source_input.clear(),
                    FleetMergeMode::SelectingHost => self.fleet.merge_host_input.clear(),
                }
            }
            ScreenId::FleetTransfer => {
                self.fleet.transfer_status = None;
                if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
                    self.fleet.transfer_select_input.clear();
                } else {
                    self.fleet.transfer_input.clear();
                }
            }
            ScreenId::FleetDetach => {
                self.fleet.detach_status = None;
                if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
                    self.fleet.detach_select_input.clear();
                } else {
                    self.fleet.detach_input.clear();
                }
            }
            ScreenId::FleetEta => {
                self.fleet.eta_status = None;
                match self.fleet.eta_mode {
                    FleetEtaMode::SelectingFleet => self.fleet.eta_select_input.clear(),
                    FleetEtaMode::EnteringDestination => self.fleet.eta_destination_input.clear(),
                    FleetEtaMode::ConfirmingSystemEntry => {
                        self.fleet.eta_include_system_input.clear()
                    }
                    FleetEtaMode::ShowingResult => {}
                }
            }
            ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseFilterPrompt => {
                self.planet.database_status = None;
                self.planet.database_input.clear();
            }
            ScreenId::PlanetTransportPlanetSelect(_) => {
                self.planet.transport_status = None;
                self.planet.transport_planet_input.clear();
            }
            ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_) => {
                self.planet.transport_status = None;
                self.planet.transport_qty_input.clear();
            }
            ScreenId::Enemies => {
                self.empire.enemies_status = None;
                self.empire.enemies_input.clear();
            }
            ScreenId::ComposeMessageRecipient => {
                self.messaging.compose_recipient_status = None;
                self.messaging.compose_recipient_input.clear();
            }
            ScreenId::ComposeMessageOutbox => {
                self.messaging.compose_outbox_status = None;
                self.messaging.compose_outbox_input.clear();
            }
            _ => {}
        }
    }
}
