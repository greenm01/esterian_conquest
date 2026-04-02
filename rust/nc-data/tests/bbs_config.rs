use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{BbsGameConfig, BbsGameConfigError, SeatReservation};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn parses_players_seed_and_reservations() {
    let kdl = r#"
players 4
seed 1515
reservations {
    seat player=1 alias="SYSOP"
    seat player=2 alias="NightShade"
}
"#;

    let config = BbsGameConfig::parse_kdl_str(kdl).expect("should parse");

    assert_eq!(config.players, 4);
    assert_eq!(config.seed, Some(1515));
    assert_eq!(
        config.reservations,
        vec![
            SeatReservation {
                player_record_index_1_based: 1,
                alias: "SYSOP".to_string(),
            },
            SeatReservation {
                player_record_index_1_based: 2,
                alias: "NightShade".to_string(),
            },
        ]
    );
}

#[test]
fn seed_is_optional() {
    let config = BbsGameConfig::parse_kdl_str("players 4\n").expect("should parse");
    assert_eq!(config.players, 4);
    assert_eq!(config.seed, None);
    assert!(config.reservations.is_empty());
}

#[test]
fn unsupported_legacy_fields_are_rejected() {
    for field in [
        "year",
        "game_name",
        "theme",
        "snoop",
        "session",
        "inactivity",
    ] {
        let kdl = format!("players 4\n{field} 1\n");
        let err = BbsGameConfig::parse_kdl_str(&kdl).expect_err("should reject unsupported field");
        assert!(
            matches!(err, BbsGameConfigError::Parse(ref message) if message.contains(field)),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn players_is_required() {
    let err = BbsGameConfig::parse_kdl_str("").expect_err("should reject missing players");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("players")),
        "unexpected error: {err}"
    );
}

#[test]
fn players_must_be_in_supported_range() {
    let err = BbsGameConfig::parse_kdl_str("players 26\n").expect_err("should reject 26 players");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("1..=25")),
        "unexpected error: {err}"
    );
}

#[test]
fn duplicate_reservation_player_is_rejected() {
    let kdl = r#"
players 4
reservations {
    seat player=1 alias="SYSOP"
    seat player=1 alias="RIVAL"
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate player");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("duplicate reservation for player 1")),
        "unexpected error: {err}"
    );
}

#[test]
fn duplicate_reservation_alias_is_rejected_case_insensitively() {
    let kdl = r#"
players 4
reservations {
    seat player=1 alias="Sysop"
    seat player=2 alias="SYSOP"
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate alias");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("duplicate reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn blank_reservation_alias_is_rejected() {
    let kdl = r#"
players 4
reservations {
    seat player=1 alias="   "
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject blank alias");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn reservation_lookup_is_trimmed_and_case_insensitive() {
    let config = BbsGameConfig::parse_kdl_str(
        r#"
players 4
reservations {
    seat player=2 alias="NightShade"
}
"#,
    )
    .expect("should parse");

    let reservation = config
        .reservation_for_alias("  nightshade ")
        .expect("alias should resolve");
    assert_eq!(reservation.player_record_index_1_based, 2);
}

#[test]
fn reservation_player_count_validation_rejects_out_of_range_seat() {
    let config = BbsGameConfig::parse_kdl_str(
        r#"
players 6
reservations {
    seat player=5 alias="SYSOP"
}
"#,
    )
    .expect("should parse");

    let err = config
        .validate_reservations_for_player_count(4)
        .expect_err("seat should exceed player count");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("exceeds player count")),
        "unexpected error: {err}"
    );
}

#[test]
fn save_and_load_round_trip() {
    let dir = temp_dir("nc-data-bbs-config");
    let path = dir.join("config.kdl");
    let config = BbsGameConfig {
        players: 4,
        seed: Some(4242),
        reservations: vec![SeatReservation {
            player_record_index_1_based: 1,
            alias: "SYSOP".to_string(),
        }],
    };

    config.save_kdl(&path).expect("save config");
    let loaded = BbsGameConfig::load_kdl(&path).expect("load config");

    assert_eq!(loaded, config);

    let _ = std::fs::remove_dir_all(&dir);
}
