#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AdvanceStartup,
    OpenStartupIntro,
    OpenMainMenu,
    OpenGeneralMenu,
    OpenReports,
    Quit,
    Noop,
}
