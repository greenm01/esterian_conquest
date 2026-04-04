use nc_connect::cli::{picker_exit_lines, successful_session_handoff_lines};
use nc_connect::connect::session::SessionOutcome;
use std::path::PathBuf;

#[test]
fn successful_session_handoff_lines_returns_none_without_notice_or_maps() {
    let outcome = SessionOutcome::Done {
        exit_code: 0,
        notice: None,
        maps_saved_to: None,
    };

    assert_eq!(successful_session_handoff_lines(&outcome), None);
}

#[test]
fn successful_session_handoff_lines_emit_notice() {
    let outcome = SessionOutcome::Done {
        exit_code: 0,
        notice: Some("Warning: unable to save starmaps.".to_string()),
        maps_saved_to: None,
    };

    assert_eq!(
        successful_session_handoff_lines(&outcome),
        Some(vec!["Warning: unable to save starmaps.".to_string()])
    );
}

#[test]
fn successful_session_handoff_lines_skip_nonzero_exit() {
    let outcome = SessionOutcome::Done {
        exit_code: 1,
        notice: None,
        maps_saved_to: None,
    };

    assert_eq!(successful_session_handoff_lines(&outcome), None);
}

#[test]
fn successful_session_handoff_lines_emit_maps_path() {
    let outcome = SessionOutcome::Done {
        exit_code: 0,
        notice: None,
        maps_saved_to: Some(PathBuf::from("/tmp/nc/maps/friday-night")),
    };

    assert_eq!(
        successful_session_handoff_lines(&outcome),
        Some(vec![
            "Maps downloaded to /tmp/nc/maps/friday-night".to_string()
        ])
    );
}

#[test]
fn picker_exit_lines_returns_empty() {
    assert_eq!(picker_exit_lines(), Vec::<String>::new());
}
