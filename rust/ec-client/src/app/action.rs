use ec_data::EmpireProductionRankingSort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AdvanceStartup,
    OpenStartupIntro,
    OpenMainMenu,
    OpenGeneralMenu,
    OpenEmpireStatus,
    OpenEmpireProfile,
    OpenRankingsPrompt,
    OpenRankingsTable(EmpireProductionRankingSort),
    OpenReports,
    Quit,
    Noop,
}
