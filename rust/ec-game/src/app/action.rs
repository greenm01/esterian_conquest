use crate::domains::empire::EmpireAction;
use crate::domains::fleet::FleetAction;
use crate::domains::messaging::MessagingAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
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
    OpenMainMenu,
    OpenMainHelp,
    OpenGeneralMenu,
    OpenGeneralHelp,
    OpenPopupHelp,
    DismissPopupHelp,
    ToggleAnsiMode,
    ToggleExpertMode,
    ReturnToCommandMenu,
    ToggleAutopilot,
    RequestQuit,
    CancelQuitPrompt,
    Quit,
    Noop,
}
