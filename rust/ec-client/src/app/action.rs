use crate::domains::fleet::FleetAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::empire::EmpireAction;
use crate::domains::messaging::MessagingAction;
use crate::domains::starmap::StarmapAction;
use crate::domains::startup::StartupAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Fleet(FleetAction),
    Planet(PlanetAction),
    Starbase(StarbaseAction),
    Empire(EmpireAction),
    Messaging(MessagingAction),
    Starmap(StarmapAction),
    Startup(StartupAction),
    
    // Top-level / Generic App Actions
    DismissModalNotice,
    OpenMainMenu,
    OpenMainHelp,
    OpenGeneralMenu,
    OpenGeneralHelp,
    ShowAnsiAlwaysOnNotice,
    ShowAnsiAlwaysOnMainMenu,
    ShowFleetExpertModeNotice,
    ReturnToCommandMenu,
    ToggleAutopilot,
    Quit,
    Noop,
}
