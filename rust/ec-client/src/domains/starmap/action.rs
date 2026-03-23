use crate::screen::CommandMenu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarmapAction {
    OpenFull,
    OpenPartialView(CommandMenu),
    BeginDump,
    AdvancePage,
    Export,
    MovePartial(i8, i8),
    OpenPlanetInfoAtCenter,
}
