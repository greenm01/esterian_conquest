mod common;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use common::*;
use ec_data::*;

#[test]
fn round_trip_player_dat() {
    let bytes = read_fixture("PLAYER.DAT");
    let parsed = PlayerDat::parse(&bytes).unwrap();
    assert_eq!(parsed.to_bytes(), bytes);
}

#[test]
fn round_trip_planets_dat() {
    let bytes = read_fixture("PLANETS.DAT");
    let parsed = PlanetDat::parse(&bytes).unwrap();
    assert_eq!(parsed.to_bytes(), bytes);
}

#[test]
fn initialized_planets_expose_named_homeworld_seeds() {
    let bytes = read_initialized_fixture("PLANETS.DAT");
    let parsed = PlanetDat::parse(&bytes).unwrap();
    let seeds = parsed
        .records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.is_named_homeworld_seed())
        .map(|(idx, record)| (idx + 1, record.coords_raw(), record.header_value_raw()))
        .collect::<Vec<_>>();

    assert_eq!(
        seeds,
        vec![
            (5, [6, 5], 100),
            (6, [13, 5], 100),
            (13, [4, 13], 100),
            (15, [16, 13], 100),
        ]
    );
}

#[test]
fn planet_tail_fields_expose_owner_slot_army_and_battery_fields() {
    let init = PlanetDat::parse(&read_initialized_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(init.records[12].owner_empire_slot_raw(), 2);
    assert_eq!(init.records[12].ownership_status_raw(), 2);
    assert_eq!(init.records[12].army_count_raw(), 10);
    assert_eq!(init.records[12].ground_batteries_raw(), 4);

    assert_eq!(init.records[14].owner_empire_slot_raw(), 1);
    assert_eq!(init.records[14].ownership_status_raw(), 2);
    assert_eq!(init.records[14].army_count_raw(), 10);
    assert_eq!(init.records[14].ground_batteries_raw(), 4);

    let fleet_post = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(fleet_post.records[13].owner_empire_slot_raw(), 1);
    assert_eq!(fleet_post.records[13].ownership_status_raw(), 2);
    assert_eq!(fleet_post.records[13].army_count_raw(), 1);
    assert_eq!(fleet_post.records[13].ground_batteries_raw(), 0);
}

#[test]
fn round_trip_setup_dat() {
    let bytes = read_fixture("SETUP.DAT");
    let parsed = SetupDat::parse(&bytes).unwrap();
    assert_eq!(parsed.to_bytes(), bytes);
    assert_eq!(parsed.version_tag(), b"EC151");
    assert_eq!(parsed.option_prefix(), &[4, 3, 4, 3, 1, 1, 1, 1]);
    assert_eq!(parsed.com_irq_raw(0), Some(4));
    assert_eq!(parsed.com_irq_raw(1), Some(3));
    assert_eq!(parsed.com_irq_raw(2), Some(4));
    assert_eq!(parsed.com_irq_raw(3), Some(3));
    assert_eq!(parsed.com_hardware_flow_control_enabled(0), Some(true));
    assert_eq!(parsed.com_hardware_flow_control_enabled(1), Some(true));
    assert_eq!(parsed.com_hardware_flow_control_enabled(2), Some(true));
    assert_eq!(parsed.com_hardware_flow_control_enabled(3), Some(true));
    assert!(parsed.snoop_enabled());
    assert_eq!(parsed.max_time_between_keys_minutes_raw(), 10);
    assert!(parsed.remote_timeout_enabled());
    assert!(!parsed.local_timeout_enabled());
    assert_eq!(parsed.minimum_time_granted_minutes_raw(), 0);
    assert_eq!(parsed.purge_after_turns_raw(), 0);
    assert_eq!(parsed.autopilot_inactive_turns_raw(), 0);
}

#[test]
fn round_trip_conquest_dat() {
    let bytes = read_fixture("CONQUEST.DAT");
    let parsed = ConquestDat::parse(&bytes).unwrap();
    assert_eq!(parsed.to_bytes(), bytes);
    assert_eq!(parsed.control_header().len(), 0x55);
    assert_eq!(parsed.header_words()[0], 0x0bce);
    assert_eq!(parsed.game_year(), 3022);
    assert_eq!(parsed.player_count(), 4);
    assert_eq!(parsed.player_config_word(), 0x0104);
}

#[test]
fn player_tax_rate_matches_current_notes() {
    let bytes = read_fixture("PLAYER.DAT");
    let parsed = PlayerDat::parse(&bytes).unwrap();
    assert_eq!(parsed.records[0].tax_rate(), 65);
}

#[test]
fn f3_owner_fixture_exposes_rogue_and_player_controlled_empire_summaries() {
    let bytes = read_f3_owner_fixture("PLAYER.DAT");
    let parsed = PlayerDat::parse(&bytes).unwrap();

    assert_eq!(parsed.records[0].owner_mode_raw(), 0xff);
    assert_eq!(parsed.records[0].legacy_status_name_len_raw(), 6);
    assert_eq!(parsed.records[0].legacy_status_name_summary(), "Rogues");
    assert_eq!(
        parsed.records[0].ownership_summary(),
        "rogue label='Rogues'"
    );

    assert_eq!(parsed.records[1].assigned_player_flag_raw(), 1);
    assert_eq!(parsed.records[1].assigned_player_handle_summary(), "FOO");
    assert_eq!(parsed.records[1].controlled_empire_name_len_raw(), 3);
    assert_eq!(parsed.records[1].controlled_empire_name_summary(), "foo");
    assert_eq!(
        parsed.records[1].ownership_summary(),
        "player handle='FOO' empire='foo'"
    );
}

#[test]
fn shipped_fleets_dat_uses_a_variable_record_count() {
    let bytes = read_fixture("FLEETS.DAT");
    let parsed = FleetDat::parse(&bytes).unwrap();
    assert_eq!(parsed.records.len(), 13);
}

#[test]
fn round_trip_initialized_fleets_dat() {
    let bytes = read_initialized_fixture("FLEETS.DAT");
    let parsed = FleetDat::parse(&bytes).unwrap();
    assert_eq!(parsed.to_bytes(), bytes);
    assert_eq!(parsed.records.len(), INITIALIZED_FLEET_RECORD_COUNT);
    assert_eq!(parsed.records[0].fleet_id(), 1);
    assert_eq!(parsed.records[0].local_slot(), 1);
    assert_eq!(parsed.records[0].next_fleet_id(), 2);
    assert_eq!(parsed.records[0].previous_fleet_id(), 0);
    assert_eq!(parsed.records[0].max_speed(), 3);
    assert_eq!(parsed.records[0].rules_of_engagement(), 6);
    assert_eq!(parsed.records[0].cruiser_count(), 1);
    assert_eq!(parsed.records[0].destroyer_count(), 0);
    assert_eq!(parsed.records[0].etac_count(), 1);
    assert_eq!(parsed.records[0].standing_order_code_raw(), 5);
    assert_eq!(
        parsed.records[0].standing_order_kind(),
        FleetStandingOrderKind::GuardBlockadeWorld
    );
    assert_eq!(
        parsed.records[0].standing_order_target_coords_raw(),
        [16, 13]
    );
    assert_eq!(
        parsed.records[0].standing_order_summary(),
        "Guard/blockade world in System (16,13)"
    );
    assert_eq!(parsed.records[0].ship_composition_summary(), "CA=1 ET=1");

    assert_eq!(parsed.records[2].fleet_id(), 3);
    assert_eq!(parsed.records[2].local_slot(), 3);
    assert_eq!(parsed.records[2].next_fleet_id(), 4);
    assert_eq!(parsed.records[2].previous_fleet_id(), 2);
    assert_eq!(parsed.records[2].max_speed(), 6);
    assert_eq!(parsed.records[2].rules_of_engagement(), 6);
    assert_eq!(parsed.records[2].cruiser_count(), 0);
    assert_eq!(parsed.records[2].destroyer_count(), 1);
    assert_eq!(parsed.records[2].etac_count(), 0);
    assert_eq!(parsed.records[2].standing_order_code_raw(), 5);
    assert_eq!(
        parsed.records[2].standing_order_kind(),
        FleetStandingOrderKind::GuardBlockadeWorld
    );
    assert_eq!(
        parsed.records[2].standing_order_target_coords_raw(),
        [16, 13]
    );
    assert_eq!(
        parsed.records[2].standing_order_summary(),
        "Guard/blockade world in System (16,13)"
    );
    assert_eq!(parsed.records[2].ship_composition_summary(), "DD=1");
}

#[test]
fn post_maintenance_matches_init_for_core_state_but_not_global_summaries() {
    assert_eq!(
        read_initialized_fixture("PLAYER.DAT"),
        read_post_maint_fixture("PLAYER.DAT")
    );
    assert_eq!(
        read_initialized_fixture("PLANETS.DAT"),
        read_post_maint_fixture("PLANETS.DAT")
    );
    assert_eq!(
        read_initialized_fixture("FLEETS.DAT"),
        read_post_maint_fixture("FLEETS.DAT")
    );
    assert_eq!(
        read_initialized_fixture("SETUP.DAT"),
        read_post_maint_fixture("SETUP.DAT")
    );

    assert_ne!(
        read_initialized_fixture("CONQUEST.DAT"),
        read_post_maint_fixture("CONQUEST.DAT")
    );
    assert_ne!(
        read_initialized_fixture("DATABASE.DAT"),
        read_post_maint_fixture("DATABASE.DAT")
    );
}

#[test]
fn preserved_conquest_year_progression_matches_docs() {
    let original = ConquestDat::parse(&read_fixture("CONQUEST.DAT")).unwrap();
    let initialized = ConquestDat::parse(&read_initialized_fixture("CONQUEST.DAT")).unwrap();
    let post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();

    assert_eq!(initialized.game_year(), 3000);
    assert_eq!(post_maint.game_year(), 3001);
    assert_eq!(original.game_year(), 3022);
    assert_eq!(initialized.player_count(), 4);
    assert_eq!(post_maint.player_count(), 4);
    assert_eq!(original.player_count(), 4);
}

#[test]
fn post_maintenance_fixture_exposes_known_schedule_bytes() {
    let post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
    assert_eq!(post_maint.maintenance_schedule_bytes(), [0x01; 7]);
}

#[test]
fn core_game_data_round_trips_post_maintenance_directory() {
    let source_dir = repo_root().join("fixtures/ecmaint-post/v1.5");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("ec-data-core-game-{unique}"));
    fs::create_dir_all(&temp_dir).unwrap();

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
    ] {
        fs::copy(source_dir.join(name), temp_dir.join(name)).unwrap();
    }

    let parsed = CoreGameData::load(&temp_dir).unwrap();
    parsed.save(&temp_dir).unwrap();

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
    ] {
        assert_eq!(
            fs::read(temp_dir.join(name)).unwrap(),
            fs::read(source_dir.join(name)).unwrap(),
            "{name} should round-trip through CoreGameData unchanged"
        );
    }

    fs::remove_dir_all(temp_dir).unwrap();
}
