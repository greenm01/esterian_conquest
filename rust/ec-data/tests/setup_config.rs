use std::path::PathBuf;

use ec_data::{DiplomacyConfig, SetupConfig, SetupMode};

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/setup.example.kdl")
}

#[test]
fn setup_example_kdl_parses() {
    let config = SetupConfig::load_kdl(&example_path()).expect("parse setup example");
    assert_eq!(config.player_count, 4);
    assert_eq!(config.year, 3000);
    assert_eq!(config.setup_mode, SetupMode::BuilderCompatible);
    assert_eq!(config.seed, Some(1515));
    assert!(config.maintenance_days.into_iter().all(|enabled| enabled));
}

#[test]
fn setup_example_kdl_builds_preflight_clean_game() {
    let config = SetupConfig::load_kdl(&example_path()).expect("parse setup example");
    let data = config.build_game_data(1515).expect("build game data");
    assert_eq!(data.conquest.player_count(), 4);
    assert_eq!(data.conquest.game_year(), 3000);
    assert_eq!(data.setup.com_irq_raw(0), Some(4));
    assert_eq!(data.setup.com_irq_raw(1), Some(3));
    assert_eq!(data.setup.com_hardware_flow_control_enabled(0), Some(true));
    assert!(data.ecmaint_preflight_errors().is_empty());
}

#[test]
fn setup_kdl_rejects_out_of_range_irq() {
    let invalid = r#"
game player_count=4 year=3000 setup_mode="canonical-four-player"
setup_options snoop=#true local_timeout=#false remote_timeout=#true max_key_gap_minutes=10 minimum_time_minutes=0 purge_after_turns=0 autopilot_after_turns=0
port_setup {
  com port="com1" irq=9 hardware_flow_control=#true
}
"#;
    let err = SetupConfig::parse_kdl_str(invalid).expect_err("invalid IRQ should fail");
    assert!(err.to_string().contains("COM IRQ values"));
}

#[test]
fn diplomacy_kdl_parses_enemy_relations() {
    let config = DiplomacyConfig::parse_kdl_str(
        "relation from=1 to=2 status=\"enemy\"\nrelation from=2 to=1 status=\"enemy\"\n",
    )
    .expect("diplomacy.kdl should parse")
    .validate_for_player_count(4)
    .expect("diplomacy.kdl should validate");
    assert_eq!(config.directives.len(), 2);
}

#[test]
fn diplomacy_kdl_rejects_out_of_range_empires() {
    let err = DiplomacyConfig::parse_kdl_str("relation from=1 to=5 status=\"enemy\"\n")
        .expect("diplomacy.kdl should parse")
        .validate_for_player_count(4)
        .expect_err("out of range empire should fail");
    assert!(err.to_string().contains("1..=4"));
}
