use crate::screen::CommandMenu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarmapAction {
    OpenFull,
    OpenPartialPrompt(CommandMenu),
    BeginDump,
    AdvancePage,
    Export,
    AppendPartialChar(char),
    BackspacePartialInput,
    SubmitPartialPrompt,
    MovePartial(i8, i8),
}
