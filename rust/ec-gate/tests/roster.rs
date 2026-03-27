//! Regression tests for roster KDL parsing and rendering.

use ec_gate::roster::{
    Roster, Seat, SeatStatus,
    io::{load_roster, parse_roster_str, render_roster, save_roster},
};
use std::fs;

fn friday_night_roster() -> Roster {
    Roster {
        id: "friday-night".to_string(),
        name: "Friday Night EC".to_string(),
        seats: vec![
            Seat {
                player: 1,
                code: "velvet-mountain".to_string(),
                status: SeatStatus::Claimed,
                npub: Some("npub1aaa...".to_string()),
            },
            Seat {
                player: 2,
                code: "copper-sunrise".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
            Seat {
                player: 3,
                code: "amber-cascade".to_string(),
                status: SeatStatus::Claimed,
                npub: Some("npub1bbb...".to_string()),
            },
            Seat {
                player: 4,
                code: "silver-meadow".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
        ],
    }
}

#[test]
fn round_trip_render_and_parse() {
    let original = friday_night_roster();
    let kdl = render_roster(&original);
    let parsed = parse_roster_str(&kdl).expect("round-trip parse failed");
    assert_eq!(parsed, original);
}

#[test]
fn parse_canonical_kdl() {
    let kdl = r#"game id="friday-night" name="Friday Night EC" {
    seat player=1 code="velvet-mountain" status="claimed" npub="npub1aaa..."
    seat player=2 code="copper-sunrise" status="pending"
    seat player=3 code="amber-cascade" status="claimed" npub="npub1bbb..."
    seat player=4 code="silver-meadow" status="pending"
}
"#;
    let roster = parse_roster_str(kdl).expect("parse failed");
    assert_eq!(roster.id, "friday-night");
    assert_eq!(roster.name, "Friday Night EC");
    assert_eq!(roster.seats.len(), 4);

    let s1 = &roster.seats[0];
    assert_eq!(s1.player, 1);
    assert_eq!(s1.code, "velvet-mountain");
    assert_eq!(s1.status, SeatStatus::Claimed);
    assert_eq!(s1.npub.as_deref(), Some("npub1aaa..."));

    let s2 = &roster.seats[1];
    assert_eq!(s2.player, 2);
    assert_eq!(s2.status, SeatStatus::Pending);
    assert!(s2.npub.is_none());
}

#[test]
fn render_pending_seat_omits_npub() {
    let roster = Roster {
        id: "test".to_string(),
        name: "Test".to_string(),
        seats: vec![Seat {
            player: 1,
            code: "abbey-abyss".to_string(),
            status: SeatStatus::Pending,
            npub: None,
        }],
    };
    let kdl = render_roster(&roster);
    assert!(!kdl.contains("npub"));
    assert!(kdl.contains("pending"));
}

#[test]
fn render_claimed_seat_includes_npub() {
    let roster = Roster {
        id: "test".to_string(),
        name: "Test".to_string(),
        seats: vec![Seat {
            player: 1,
            code: "abbey-abyss".to_string(),
            status: SeatStatus::Claimed,
            npub: Some("npub1xyz...".to_string()),
        }],
    };
    let kdl = render_roster(&roster);
    assert!(kdl.contains("claimed"));
    assert!(kdl.contains("npub1xyz..."));
}

#[test]
fn parse_missing_game_node_is_error() {
    let result = parse_roster_str("other-node foo=\"bar\"");
    assert!(result.is_err());
}

#[test]
fn parse_missing_id_is_error() {
    let result = parse_roster_str(r#"game name="Test" { }"#);
    assert!(result.is_err());
}

#[test]
fn parse_unknown_status_is_error() {
    let kdl = r#"game id="x" name="X" {
    seat player=1 code="abbey-abyss" status="broken"
}"#;
    let result = parse_roster_str(kdl);
    assert!(result.is_err());
}

#[test]
fn kdl_escaping_handles_quotes() {
    let roster = Roster {
        id: "test".to_string(),
        name: r#"Game with "Quotes""#.to_string(),
        seats: vec![],
    };
    let kdl = render_roster(&roster);
    let parsed = parse_roster_str(&kdl).expect("round-trip with quotes failed");
    assert_eq!(parsed.name, r#"Game with "Quotes""#);
}

#[test]
fn save_and_load_roster_via_file() {
    let dir = std::env::temp_dir().join("ec-gate-roster-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roster.kdl");

    let original = friday_night_roster();
    save_roster(&path, &original).expect("save_roster failed");
    assert!(path.exists(), "roster.kdl should exist after save");

    let loaded = load_roster(&path).expect("load_roster failed");
    assert_eq!(loaded, original);

    // Cleanup
    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn save_roster_is_atomic_no_tmp_leftover() {
    let dir = std::env::temp_dir().join("ec-gate-roster-atomic-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roster.kdl");
    let tmp_path = dir.join("roster.kdl.tmp");

    save_roster(&path, &friday_night_roster()).expect("save failed");

    assert!(path.exists(), "roster.kdl should exist");
    assert!(!tmp_path.exists(), ".tmp file should be gone after rename");

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}
