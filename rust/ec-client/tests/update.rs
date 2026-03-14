use ec_client::app::{Action, AppOutcome, apply_action};
use ec_client::screen::ScreenId;

#[test]
fn apply_action_switches_between_client_screens() {
    let mut screen = ScreenId::MainMenu;

    assert_eq!(
        apply_action(&mut screen, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(screen, ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut screen, Action::OpenReports),
        AppOutcome::Continue
    );
    assert_eq!(screen, ScreenId::Reports);

    assert_eq!(
        apply_action(&mut screen, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(screen, ScreenId::MainMenu);
}

#[test]
fn apply_action_quit_exits_loop() {
    let mut screen = ScreenId::MainMenu;
    assert_eq!(apply_action(&mut screen, Action::Quit), AppOutcome::Quit);
    assert_eq!(screen, ScreenId::MainMenu);
}
