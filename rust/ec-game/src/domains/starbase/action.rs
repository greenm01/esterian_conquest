#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseAction {
    OpenMenu,
    OpenHelp,
    OpenList,
    OpenReviewSelect,
    OpenReview,
    ShowMoveNotice,
    MoveSelect(i8),
    AppendChar(char),
    BackspaceInput,
    SubmitReviewSelect,
}
