use ec_connect::cli::successful_session_handoff_lines;
use ec_connect::connect::session::SessionOutcome;

#[test]
fn successful_session_handoff_lines_emit_griffith_line_without_notice() {
    let outcome = SessionOutcome::Done {
        exit_code: 0,
        notice: None,
    };

    assert_eq!(
        successful_session_handoff_lines(&outcome),
        Some(vec!["For Griffith and glory.".to_string()])
    );
}

#[test]
fn successful_session_handoff_lines_emit_notice_before_griffith_line() {
    let outcome = SessionOutcome::Done {
        exit_code: 0,
        notice: Some("Warning: unable to save starmaps.".to_string()),
    };

    assert_eq!(
        successful_session_handoff_lines(&outcome),
        Some(vec![
            "Warning: unable to save starmaps.".to_string(),
            "For Griffith and glory.".to_string(),
        ])
    );
}

#[test]
fn successful_session_handoff_lines_skip_nonzero_exit() {
    let outcome = SessionOutcome::Done {
        exit_code: 1,
        notice: None,
    };

    assert_eq!(successful_session_handoff_lines(&outcome), None);
}
