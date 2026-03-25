use ec_data::EmpireProductionRankingSort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpireAction {
    OpenStatus,
    OpenProfile,
    OpenRankingsTable(EmpireProductionRankingSort),
    OpenEnemies,
    ScrollEnemies(i8),
    MoveEnemies(i8),
    AppendEnemiesChar(char),
    BackspaceEnemiesInput,
    SubmitEnemiesInput,
}
