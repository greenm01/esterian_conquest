#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseAction {
    OpenMenu,
    OpenList,
    OpenReviewSelect,
    OpenReview,
    OpenMovePrompt,
    MoveSelect(i8),
    AppendChar(char),
    BackspaceInput,
    SubmitReviewSelect,
    AppendMovePromptChar(char),
    BackspaceMovePromptInput,
    SubmitMovePrompt,
    CancelMovePrompt,
}
