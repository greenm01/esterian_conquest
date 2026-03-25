use std::path::PathBuf;

use ec_data::DEFAULT_GAME_CONFIG_KDL;
use ec_data::game_config::{
    GameConfig, GameConfigError, InactivityConfig, SeatReservation, SessionConfig,
};

// ─── Default / bundled KDL ────────────────────────────────────────────────────

#[test]
fn bundled_config_kdl_parses_without_error() {
    GameConfig::parse_kdl_str(DEFAULT_GAME_CONFIG_KDL).expect("bundled config.kdl should be valid");
}

#[test]
fn bundled_config_kdl_matches_default_values() {
    let config = GameConfig::parse_kdl_str(DEFAULT_GAME_CONFIG_KDL)
        .expect("bundled config.kdl should be valid");
    let expected = GameConfig::default();

    assert_eq!(config.game_name, expected.game_name);
    assert_eq!(config.snoop, expected.snoop);
    assert_eq!(config.session, expected.session);
    assert_eq!(config.inactivity, expected.inactivity);
    assert_eq!(config.reservations, expected.reservations);
    // theme directive may be absent in bundled KDL (commented out)
    // so we just confirm it parses cleanly; we do not assert a specific path.
}

// ─── Explicit field parsing ───────────────────────────────────────────────────

#[test]
fn parses_all_fields_explicitly() {
    let kdl = r#"
game_name "Battle for the Stars"
theme "my-theme.kdl"
snoop #false
session {
    max_idle_minutes 20
    minimum_time_minutes 5
    local_timeout #true
    remote_timeout #false
}
inactivity {
    purge_after_turns 10
    autopilot_after_turns 3
}
reservations {
    seat player=1 alias="SYSOP"
    seat player=2 alias="NightShade"
}
"#;

    let config = GameConfig::parse_kdl_str(kdl).expect("should parse");

    assert_eq!(config.game_name, "Battle for the Stars");
    assert_eq!(config.theme, Some(PathBuf::from("my-theme.kdl")));
    assert!(!config.snoop);

    assert_eq!(config.session.max_idle_minutes, 20);
    assert_eq!(config.session.minimum_time_minutes, 5);
    assert!(config.session.local_timeout);
    assert!(!config.session.remote_timeout);

    assert_eq!(config.inactivity.purge_after_turns, 10);
    assert_eq!(config.inactivity.autopilot_after_turns, 3);
    assert_eq!(
        config.reservations,
        vec![
            SeatReservation {
                player_record_index_1_based: 1,
                alias: "SYSOP".to_string()
            },
            SeatReservation {
                player_record_index_1_based: 2,
                alias: "NightShade".to_string()
            }
        ]
    );
}

#[test]
fn missing_optional_fields_use_defaults() {
    let config = GameConfig::parse_kdl_str("").expect("empty KDL should yield defaults");
    assert_eq!(config, GameConfig::default());
}

#[test]
fn missing_session_block_uses_defaults() {
    let config = GameConfig::parse_kdl_str("snoop #false\n").expect("should parse");
    assert_eq!(config.session, SessionConfig::default());
}

#[test]
fn missing_inactivity_block_uses_defaults() {
    let config = GameConfig::parse_kdl_str("snoop #true\n").expect("should parse");
    assert_eq!(config.inactivity, InactivityConfig::default());
}

// ─── Validation errors ────────────────────────────────────────────────────────

#[test]
fn max_idle_minutes_over_120_is_rejected() {
    let kdl = "session {\n    max_idle_minutes 121\n}\n";
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject > 120");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("max_idle_minutes")),
        "unexpected error: {err}"
    );
}

#[test]
fn minimum_time_minutes_over_120_is_rejected() {
    let kdl = "session {\n    minimum_time_minutes 200\n}\n";
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject > 120");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("minimum_time_minutes")),
        "unexpected error: {err}"
    );
}

#[test]
fn purge_after_turns_over_100_is_rejected() {
    let kdl = "inactivity {\n    purge_after_turns 101\n}\n";
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject > 100");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("purge_after_turns")),
        "unexpected error: {err}"
    );
}

#[test]
fn autopilot_after_turns_over_100_is_rejected() {
    let kdl = "inactivity {\n    autopilot_after_turns 255\n}\n";
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject > 100");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("autopilot_after_turns")),
        "unexpected error: {err}"
    );
}

#[test]
fn invalid_kdl_is_rejected() {
    let err = GameConfig::parse_kdl_str("this {{ is not valid").expect_err("should reject bad KDL");
    assert!(
        matches!(err, GameConfigError::Parse(_)),
        "unexpected error: {err}"
    );
}

#[test]
fn duplicate_reservation_player_is_rejected() {
    let kdl = r#"
reservations {
    seat player=1 alias="SYSOP"
    seat player=1 alias="RIVAL"
}
"#;
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate seat");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("duplicate reservation for player 1")),
        "unexpected error: {err}"
    );
}

#[test]
fn duplicate_reservation_alias_is_rejected_case_insensitively() {
    let kdl = r#"
reservations {
    seat player=1 alias="Sysop"
    seat player=2 alias="SYSOP"
}
"#;
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate alias");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("duplicate reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn blank_reservation_alias_is_rejected() {
    let kdl = r#"
reservations {
    seat player=1 alias="   "
}
"#;
    let err = GameConfig::parse_kdl_str(kdl).expect_err("should reject blank alias");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn reservation_alias_lookup_is_trimmed_and_case_insensitive() {
    let kdl = r#"
reservations {
    seat player=2 alias="NightShade"
}
"#;
    let config = GameConfig::parse_kdl_str(kdl).expect("should parse");
    let reservation = config
        .reservation_for_alias("  nightshade ")
        .expect("alias should resolve");
    assert_eq!(reservation.player_record_index_1_based, 2);
}

#[test]
fn reservation_player_count_validation_rejects_out_of_range_seat() {
    let kdl = r#"
reservations {
    seat player=5 alias="SYSOP"
}
"#;
    let config = GameConfig::parse_kdl_str(kdl).expect("should parse");
    let err = config
        .validate_reservations_for_player_count(4)
        .expect_err("seat should exceed player count");
    assert!(
        matches!(err, GameConfigError::Parse(ref msg) if msg.contains("exceeds player count")),
        "unexpected error: {err}"
    );
}

// ─── load_kdl (file path) ─────────────────────────────────────────────────────

#[test]
fn load_kdl_reads_file_from_disk() {
    let dir = std::env::temp_dir().join(format!(
        "ec-game-config-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let path = dir.join("config.kdl");
    std::fs::write(&path, "game_name \"Galaxy Wars\"\n").expect("write test config.kdl");

    let config = GameConfig::load_kdl(&path).expect("load_kdl should succeed");
    assert_eq!(config.game_name, "Galaxy Wars");
}

#[test]
fn load_kdl_returns_io_error_for_missing_file() {
    let path = std::path::Path::new("/tmp/ec-nonexistent-config.kdl");
    let err = GameConfig::load_kdl(path).expect_err("should fail for missing file");
    assert!(
        matches!(err, GameConfigError::Io(_)),
        "expected IO error, got: {err}"
    );
}
