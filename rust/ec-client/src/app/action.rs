#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenMainMenu,
    OpenGeneralMenu,
    OpenReports,
    Quit,
    Noop,
}
