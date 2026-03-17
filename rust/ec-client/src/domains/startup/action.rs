#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    Advance,
    OpenIntro,
    AcceptDefault,
    RejectChoice,
    EnableNonstop,
    OpenFirstTimeMenu,
    OpenFirstTimeHelp,
    OpenFirstTimeEmpires,
    OpenFirstTimeIntro,
    OpenFirstTimeJoinName,
    AppendFirstTimeInputChar(char),
    BackspaceFirstTimeInput,
    SubmitFirstTimeInput,
    AcceptFirstTimePrompt,
    RejectFirstTimePrompt,
    OpenReports,
}
