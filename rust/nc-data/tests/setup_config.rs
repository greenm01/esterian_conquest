use nc_data::DiplomacyConfig;

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
