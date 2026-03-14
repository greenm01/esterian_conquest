use ec_data::EmpireProductionRankingSort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AdvanceStartup,
    OpenStartupIntro,
    OpenMainMenu,
    OpenGeneralMenu,
    OpenStarmap,
    OpenPartialStarmapPrompt,
    BeginStarmapDump,
    AdvanceStarmapPage,
    ExportStarmap,
    AppendPartialStarmapChar(char),
    BackspacePartialStarmapInput,
    SubmitPartialStarmapPrompt,
    MovePartialStarmap(i8, i8),
    OpenPlanetInfoPrompt,
    AppendPlanetInfoChar(char),
    BackspacePlanetInfoInput,
    SubmitPlanetInfoPrompt,
    OpenEmpireStatus,
    OpenEmpireProfile,
    OpenRankingsPrompt,
    OpenRankingsTable(EmpireProductionRankingSort),
    OpenReports,
    Quit,
    Noop,
}
