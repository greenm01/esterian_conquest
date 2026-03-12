mod common;

use common::*;
use ec_data::*;

#[test]
fn base_record_setters_can_recreate_known_valid_guard_starbase_record() {
    let mut record = BaseRecord::new_zeroed();
    record.set_local_slot_raw(0x01);
    record.set_active_flag_raw(0x01);
    record.set_base_id_raw(0x01);
    record.set_link_word_raw(0x0000);
    record.set_chain_word_raw(0x0001);
    record.set_coords_raw([0x10, 0x0D]);
    record.set_tuple_a_payload_raw([0x80, 0x00, 0x00, 0x00, 0x00]);
    record.set_tuple_b_payload_raw([0x80, 0x00, 0x00, 0x00, 0x00]);
    record.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
    record.set_trailing_coords_raw([0x10, 0x0D]);
    record.set_owner_empire_raw(0x01);

    assert_eq!(record.local_slot_raw(), 0x01);
    assert_eq!(record.active_flag_raw(), 0x01);
    assert_eq!(record.base_id_raw(), 0x01);
    assert_eq!(record.link_word_raw(), 0x0000);
    assert_eq!(record.chain_word_raw(), 0x0001);
    assert_eq!(record.coords_raw(), [0x10, 0x0D]);
    assert_eq!(record.tuple_a_payload_raw(), [0x80, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(record.tuple_b_payload_raw(), [0x80, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(record.tuple_c_payload_raw(), [0x81, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(record.trailing_coords_raw(), [0x10, 0x0D]);
    assert_eq!(record.owner_empire_raw(), 0x01);

    assert_eq!(
        record.raw,
        [
            0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x0D,
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x81,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0D, 0x01,
        ]
    );
}

#[test]
fn guard_starbase_related_accessors_expose_linkage_words() {
    let player = PlayerDat::parse(&read_fixture("PLAYER.DAT")).unwrap();
    assert_eq!(player.records[0].fleet_chain_head_raw(), 1);
    assert_eq!(player.records[0].ipbm_count_raw(), 0);

    let fleet_bytes = read_ecmaint_starbase_pre_fixture("FLEETS.DAT");
    let fleets = FleetDat::parse(&fleet_bytes).unwrap();
    let fleet = &fleets.records[0];
    assert_eq!(fleet.local_slot_word_raw(), 1);
    assert_eq!(fleet.next_fleet_link_word_raw(), 2);
    assert_eq!(fleet.fleet_id_word_raw(), 1);
    assert_eq!(fleet.guard_starbase_index_raw(), 1);
    assert_eq!(fleet.guard_starbase_enable_raw(), 1);

    let base_bytes = read_ecmaint_starbase_pre_fixture("BASES.DAT");
    let bases = BaseDat::parse(&base_bytes).unwrap();
    let base = &bases.records[0];
    assert_eq!(base.summary_word_raw(), 1);
    assert_eq!(base.chain_word_raw(), 1);
}

#[test]
fn ipbm_record_setters_round_trip_structural_prefix_fields() {
    let mut record = IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] };
    record.set_primary_word_raw(0x1234);
    record.set_owner_empire_raw(0x02);
    record.set_gate_word_raw(0x4567);
    record.set_follow_on_word_raw(0x89ab);

    assert_eq!(record.primary_word_raw(), 0x1234);
    assert_eq!(record.owner_empire_raw(), 0x02);
    assert_eq!(record.gate_word_raw(), 0x4567);
    assert_eq!(record.follow_on_word_raw(), 0x89ab);
}

#[test]
fn ipbm_record_setters_round_trip_structural_payload_groups() {
    let mut record = IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] };
    record.set_tuple_a_tag_raw(0x11);
    record.set_tuple_b_tag_raw(0x22);
    record.set_tuple_a_payload_raw([1, 2, 3, 4, 5]);
    record.set_tuple_b_payload_raw([6, 7, 8, 9, 10]);
    record.set_tuple_c_payload_raw([11, 12, 13, 14, 15]);
    record.set_trailing_control_raw([0xAA, 0xBB, 0xCC]);

    assert_eq!(record.tuple_a_tag_raw(), 0x11);
    assert_eq!(record.tuple_b_tag_raw(), 0x22);
    assert_eq!(record.tuple_a_payload_raw(), [1, 2, 3, 4, 5]);
    assert_eq!(record.tuple_b_payload_raw(), [6, 7, 8, 9, 10]);
    assert_eq!(record.tuple_c_payload_raw(), [11, 12, 13, 14, 15]);
    assert_eq!(record.trailing_control_raw(), [0xAA, 0xBB, 0xCC]);
}

#[test]
fn can_set_maintenance_schedule_from_enabled_days() {
    let mut post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
    post_maint.set_maintenance_schedule_enabled([true, false, true, false, true, false, true]);
    assert_eq!(
        post_maint.maintenance_schedule_bytes(),
        [0x01, 0x00, 0xCA, 0x00, 0x0A, 0x00, 0x26]
    );
    assert_eq!(
        post_maint.maintenance_schedule_enabled(),
        [true, false, true, false, true, false, true]
    );
}

#[test]
fn can_toggle_snoop_enabled() {
    let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
    assert!(setup.snoop_enabled());
    setup.set_snoop_enabled(false);
    assert!(!setup.snoop_enabled());
    assert_eq!(setup.raw[512], 0);
}

#[test]
fn can_set_other_setup_program_fields() {
    let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
    assert!(setup.set_com_hardware_flow_control_enabled(0, false));
    assert!(setup.set_com_hardware_flow_control_enabled(1, false));
    assert!(setup.set_com_hardware_flow_control_enabled(2, false));
    assert!(setup.set_com_hardware_flow_control_enabled(3, false));
    setup.set_max_time_between_keys_minutes_raw(15);
    setup.set_remote_timeout_enabled(false);
    setup.set_local_timeout_enabled(true);
    setup.set_minimum_time_granted_minutes_raw(69);
    setup.set_purge_after_turns_raw(10);
    setup.set_autopilot_inactive_turns_raw(3);

    assert_eq!(setup.max_time_between_keys_minutes_raw(), 15);
    assert!(!setup.remote_timeout_enabled());
    assert!(setup.local_timeout_enabled());
    assert_eq!(setup.minimum_time_granted_minutes_raw(), 69);
    assert_eq!(setup.purge_after_turns_raw(), 10);
    assert_eq!(setup.autopilot_inactive_turns_raw(), 3);
    assert_eq!(setup.com_hardware_flow_control_enabled(0), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(1), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(2), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(3), Some(false));
}

#[test]
fn can_set_purge_after_turns_raw() {
    let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
    assert_eq!(setup.purge_after_turns_raw(), 0);
    setup.set_purge_after_turns_raw(1);
    assert_eq!(setup.purge_after_turns_raw(), 1);
    assert_eq!(setup.raw[518], 1);
}

#[test]
fn core_game_data_current_known_count_helpers_follow_player1_and_records() {
    let mut base1 = BaseRecord::new_zeroed();
    base1.set_owner_empire_raw(1);
    let mut base2 = BaseRecord::new_zeroed();
    base2.set_owner_empire_raw(1);
    let mut base3 = BaseRecord::new_zeroed();
    base3.set_owner_empire_raw(2);
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat {
            records: vec![base1, base2, base3],
        },
        ipbm: IpbmDat {
            records: vec![
                IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] },
                IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] },
                IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] },
            ],
        },
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert_eq!(data.player1_starbase_count_current_known(), 0);
    assert_eq!(data.player_owned_planet_counts_current_known(), vec![1, 1, 1, 1, 0]);
    assert_eq!(
        data.player_homeworld_seed_coords_current_known(),
        vec![Some([16, 13]), Some([4, 13]), Some([6, 5]), Some([13, 5])]
    );
    assert_eq!(data.player1_owned_base_record_count_current_known(), 2);
    assert_eq!(data.player_owned_base_record_counts_current_known(), vec![2, 1, 0, 0, 0]);
    assert_eq!(data.player1_ipbm_count_current_known(), 0);
    let initial_errors = data.current_known_core_state_errors();
    assert_eq!(initial_errors.len(), 5);
    assert!(initial_errors.contains(&"PLAYER[1]-owned BASES.DAT record count expected 0, got 2".to_string()));
    assert!(initial_errors.contains(&format!(
        "PLAYER[2]-owned BASES.DAT record count expected {}, got 1",
        data.player.records[1].starbase_count_raw()
    )));
    assert!(initial_errors.contains(&"IPBM.DAT record count expected 0, got 3".to_string()));
    assert!(initial_errors.contains(&"BASES.DAT expected empty auxiliary baseline, got 3 records".to_string()));
    assert!(initial_errors.contains(&"IPBM.DAT expected empty auxiliary baseline, got 3 records".to_string()));

    data.sync_player1_current_known_counts();

    assert_eq!(data.player.records[0].starbase_count_raw(), 2);
    assert_eq!(data.player.records[1].starbase_count_raw(), 1);
    assert_eq!(data.player.records[2].starbase_count_raw(), 0);
    assert_eq!(data.player.records[3].starbase_count_raw(), 0);
    assert_eq!(data.player.records[4].starbase_count_raw(), 0);
    assert_eq!(data.player.records[0].ipbm_count_raw(), 3);
    assert_eq!(data.player1_starbase_count_current_known(), 2);
    assert_eq!(data.player_owned_planet_counts_current_known(), vec![1, 1, 1, 1, 0]);
    assert_eq!(data.player1_owned_base_record_count_current_known(), 2);
    assert_eq!(data.player_starbase_counts_current_known(), vec![2, 1, 0, 0, 0]);
    assert_eq!(data.player_owned_base_record_counts_current_known(), vec![2, 1, 0, 0, 0]);
    assert_eq!(data.player1_ipbm_count_current_known(), 3);
    let post_sync_errors = data.current_known_core_state_errors();
    assert_eq!(
        post_sync_errors,
        vec![
            "BASES.DAT expected empty auxiliary baseline, got 3 records".to_string(),
            "IPBM.DAT expected empty auxiliary baseline, got 3 records".to_string(),
        ]
    );
}

#[test]
fn core_game_data_initialized_fleet_block_helpers_match_known_fixtures() {
    let data = CoreGameData {
        player: PlayerDat::parse(&read_initialized_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_initialized_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_initialized_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_initialized_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_initialized_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_initialized_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_initialized_fixture("CONQUEST.DAT")).unwrap(),
    };

    assert!(data.looks_like_initialized_fleet_blocks_current_known());
    assert_eq!(data.current_known_initialized_fleet_block_head_ids(), vec![1, 5, 9, 13]);
    assert!(data.current_known_initialized_fleet_block_errors().is_empty());
    assert!(data.current_known_initialized_fleet_payload_errors().is_empty());
    assert!(data.current_known_initialized_fleet_mission_errors().is_empty());
    assert!(data.current_known_homeworld_seed_errors().is_empty());
    assert!(data.current_known_initialized_planet_ownership_errors().is_empty());
    assert!(data.current_known_homeworld_seed_payload_errors().is_empty());
    assert!(data.current_known_unowned_planet_payload_errors().is_empty());
    assert!(data.current_known_empty_auxiliary_state_errors().is_empty());
    assert!(data.current_known_initialized_homeworld_alignment_errors().is_empty());
    assert!(data.current_known_setup_baseline_errors().is_empty());
    assert!(data.current_known_conquest_baseline_errors().is_empty());
}

#[test]
fn core_game_data_initialized_fleet_block_errors_catch_broken_local_chain() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records[1].raw[0x03] = 9;

    assert_eq!(
        data.current_known_initialized_fleet_block_errors(),
        vec!["FLEET[2].next_fleet_id expected 3, got 9".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_payload_errors_catch_broken_slot_pattern() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records[2].raw[0x09] = 3;

    assert_eq!(
        data.current_known_initialized_fleet_payload_errors(),
        vec!["FLEET[3].max_speed expected 6, got 3".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_mission_errors_catch_wrong_order_code() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records[0].set_standing_order_code_raw(4);

    assert_eq!(
        data.current_known_initialized_fleet_mission_errors(),
        vec!["FLEET[1].standing_order expected 5 for initialized baseline, got 4".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_mission_errors_catch_wrong_aux_bytes() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records[0].set_mission_aux_bytes([0, 1]);

    assert_eq!(
        data.current_known_initialized_fleet_mission_errors(),
        vec!["FLEET[1].mission_aux expected [1, 0] for initialized baseline, got [0, 1]".to_string()]
    );
}

#[test]
fn core_game_data_owner_range_errors_catch_invalid_planet_and_base_owners() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat {
            records: vec![BaseRecord::new_zeroed()],
        },
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[0].raw[0x5D] = 9;
    data.bases.records[0].set_owner_empire_raw(0);

    assert_eq!(
        data.current_known_planet_owner_slot_errors(),
        vec!["PLANET[1].owner_empire_slot expected <= 4, got 9".to_string()]
    );
    assert_eq!(
        data.current_known_base_owner_empire_errors(),
        vec!["BASES[1].owner_empire expected 1..=4, got 0".to_string()]
    );
}

#[test]
fn core_game_data_homeworld_alignment_errors_catch_misaligned_fleet_block() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records[4].raw[0x20] = 9;

    assert_eq!(
        data.current_known_initialized_homeworld_alignment_errors(),
        vec!["FLEET block 2 target expected homeworld seed [4, 13], got [9, 13]".to_string()]
    );
}

#[test]
fn core_game_data_initialized_planet_ownership_errors_catch_non_homeworld_owner() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[0].raw[0x5D] = 2;

    assert_eq!(
        data.current_known_initialized_planet_ownership_errors(),
        vec![
            "PLANET[1] expected unowned non-homeworld baseline, got owner 2".to_string(),
            "PLAYER[2] owned_planet_count expected 1, got 2".to_string(),
        ]
    );
}

#[test]
fn core_game_data_homeworld_seed_payload_errors_catch_changed_army_marker() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[14].raw[0x5A] = 1;

    assert_eq!(
        data.current_known_homeworld_seed_payload_errors(),
        vec!["PLANET[15].likely_army_count expected 4 for homeworld seed, got 1".to_string()]
    );
}

#[test]
fn core_game_data_homeworld_seed_payload_errors_catch_changed_tax_rate() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[14].raw[0x0E] = 11;

    assert_eq!(
        data.current_known_homeworld_seed_payload_errors(),
        vec!["PLANET[15].planet_tax_rate expected 12 for homeworld seed, got 11".to_string()]
    );
}

#[test]
fn core_game_data_unowned_planet_payload_errors_catch_owned_marker() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[0].raw[0x5A] = 1;

    assert_eq!(
        data.current_known_unowned_planet_payload_errors(),
        vec!["PLANET[1].likely_army_count expected 0 for unowned baseline, got 1".to_string()]
    );
}

#[test]
fn core_game_data_unowned_planet_payload_errors_catch_nonzero_stored_goods() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets.records[0].raw[0x0A] = 1;

    assert_eq!(
        data.current_known_unowned_planet_payload_errors(),
        vec!["PLANET[1].stored_goods_raw expected 0 for unowned baseline, got 1".to_string()]
    );
}

#[test]
fn core_game_data_empty_auxiliary_state_errors_catch_starbase_record() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.bases.records.push(BaseRecord::new_zeroed());

    assert_eq!(
        data.current_known_empty_auxiliary_state_errors(),
        vec!["BASES.DAT expected empty auxiliary baseline, got 1 records".to_string()]
    );
}

#[test]
fn core_game_data_setup_baseline_errors_catch_changed_timeout_flag() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.setup.set_remote_timeout_enabled(false);

    assert_eq!(
        data.current_known_setup_baseline_errors(),
        vec!["SETUP.DAT.remote_timeout expected enabled in baseline".to_string()]
    );
}

#[test]
fn core_game_data_conquest_baseline_errors_catch_changed_year() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.conquest.raw[0] = 0xB7;
    data.conquest.raw[1] = 0x0B; // 2999

    assert_eq!(
        data.current_known_conquest_baseline_errors(),
        vec!["CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got 2999".to_string()]
    );
}

#[test]
fn core_game_data_sync_current_known_baseline_controls_and_counts_repairs_mutated_fields() {
    let mut base1 = BaseRecord::new_zeroed();
    base1.set_owner_empire_raw(1);
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat { records: vec![base1] },
        ipbm: IpbmDat { records: vec![IpbmRecord { raw: [0u8; IPBM_RECORD_SIZE] }] },
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.player.records[0].set_starbase_count_raw(9);
    data.player.records[0].set_ipbm_count_raw(9);
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.setup.set_remote_timeout_enabled(false);
    data.conquest.raw[0..2].copy_from_slice(&2999u16.to_le_bytes());
    data.conquest.raw[2] = 9;
    data.conquest.set_maintenance_schedule_enabled([false; 7]);

    data.sync_current_known_baseline_controls_and_counts();

    assert_eq!(data.player.records[0].starbase_count_raw(), 1);
    assert_eq!(data.player.records[0].ipbm_count_raw(), 1);
    assert!(data.current_known_setup_baseline_errors().is_empty());
    assert!(data.current_known_conquest_baseline_errors().is_empty());
}

#[test]
fn core_game_data_sync_current_known_initialized_fleet_baseline_repairs_mutated_fleets() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.records.clear();
    data.fleets.records.push(FleetRecord::new_zeroed());

    data.sync_current_known_initialized_fleet_baseline();

    assert!(data.looks_like_initialized_fleet_blocks_current_known());
    assert!(data.current_known_initialized_fleet_payload_errors().is_empty());
    assert!(data.current_known_initialized_fleet_mission_errors().is_empty());
    assert!(data.current_known_initialized_homeworld_alignment_errors().is_empty());
}

#[test]
fn core_game_data_can_apply_current_known_scenario_mutations() {
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let aux = data
        .set_fleet_order(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
        .unwrap();
    assert_eq!(data.fleets.records[0].current_speed(), 0x03);
    assert_eq!(data.fleets.records[0].standing_order_code_raw(), 0x0C);
    assert_eq!(data.fleets.records[0].standing_order_target_coords_raw(), [0x0F, 0x0D]);
    assert_eq!(aux, data.fleets.records[0].mission_aux_bytes());

    data.set_planet_build(15, 0x03, 0x01).unwrap();
    assert_eq!(data.planets.records[14].build_count_raw(0), 0x03);
    assert_eq!(data.planets.records[14].build_kind_raw(0), 0x01);

    data.set_guard_starbase_onebase([0x10, 0x0D]).unwrap();
    assert_eq!(data.player.records[0].starbase_count_raw(), 1);
    assert_eq!(data.fleets.records[0].standing_order_code_raw(), 0x04);
    assert_eq!(data.fleets.records[0].standing_order_target_coords_raw(), [0x10, 0x0D]);
    assert_eq!(data.fleets.records[0].mission_aux_bytes(), [0x01, 0x01]);
    assert_eq!(data.bases.records.len(), 1);
    assert_eq!(data.bases.records[0].summary_word_raw(), data.fleets.records[0].local_slot_word_raw());
    assert_eq!(data.bases.records[0].chain_word_raw(), data.fleets.records[0].fleet_id_word_raw());
    assert_eq!(data.bases.records[0].coords_raw(), [0x10, 0x0D]);

    data.set_ipbm_zero_records(2);
    assert_eq!(data.player.records[0].ipbm_count_raw(), 2);
    assert_eq!(data.ipbm.records.len(), 2);
    assert_eq!(data.ipbm.to_bytes().len(), 2 * IPBM_RECORD_SIZE);

    data.set_ipbm_record_prefix(2, 0x1234, 0x02, 0x4567, 0x89ab)
        .unwrap();
    let ipbm = &data.ipbm.records[1];
    assert_eq!(ipbm.primary_word_raw(), 0x1234);
    assert_eq!(ipbm.owner_empire_raw(), 0x02);
    assert_eq!(ipbm.gate_word_raw(), 0x4567);
    assert_eq!(ipbm.follow_on_word_raw(), 0x89ab);
}

#[test]
fn core_game_data_current_known_validation_helpers_match_known_fixtures() {
    let fleet_data = CoreGameData {
        player: PlayerDat::parse(&read_ecmaint_fleet_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_ecmaint_fleet_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_ecmaint_fleet_pre_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_ecmaint_fleet_pre_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_ecmaint_fleet_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_ecmaint_fleet_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(fleet_data
        .fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
        .is_empty());

    let build_data = CoreGameData {
        player: PlayerDat::parse(&read_ecmaint_build_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_ecmaint_build_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_ecmaint_build_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_ecmaint_build_pre_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_ecmaint_build_pre_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_ecmaint_build_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_ecmaint_build_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(build_data
        .planet_build_errors_current_known(15, 0x03, 0x01)
        .is_empty());

    let starbase_data = CoreGameData {
        player: PlayerDat::parse(&read_ecmaint_starbase_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_ecmaint_starbase_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_ecmaint_starbase_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_ecmaint_starbase_pre_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_ecmaint_starbase_pre_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_ecmaint_starbase_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_ecmaint_starbase_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(starbase_data.guard_starbase_onebase_errors_current_known().is_empty());
    assert_eq!(
        starbase_data.current_known_compliance_status(),
        CurrentKnownComplianceStatus {
            fleet_order: false,
            planet_build: false,
            guard_starbase: true,
            ipbm: true,
        }
    );
    assert_eq!(
        starbase_data.current_known_key_word_summary(),
        CurrentKnownKeyWordSummary {
            player_starbase_count: 1,
            player_ipbm_count: 0,
            fleet1_local_slot: Some(1),
            fleet1_id: Some(1),
            fleet1_guard_index: Some(1),
            fleet1_guard_enable: Some(1),
            fleet1_target: Some([0x10, 0x0D]),
            base1_summary: Some(1),
            base1_id: Some(1),
            base1_chain: Some(1),
            base1_coords: Some([0x10, 0x0D]),
            ipbm_record_count: 0,
        }
    );
    assert_eq!(
        starbase_data.guard_starbase_linkage_summary_current_known(1, 1).unwrap(),
        CurrentKnownGuardStarbaseLinkageSummary {
            player_record_index_1_based: 1,
            fleet_record_index_1_based: 1,
            player_starbase_count: 1,
            fleet_order: 0x04,
            fleet_local_slot: 1,
            fleet_id: 1,
            guard_index: 1,
            guard_enable: 1,
            target_coords: [0x10, 0x0D],
            selected_base_present: true,
            selected_base_summary_word: Some(1),
            selected_base_id: Some(1),
            selected_base_chain_word: Some(1),
            selected_base_coords: Some([0x10, 0x0D]),
            selected_base_trailing_coords: Some([0x10, 0x0D]),
            selected_base_owner_empire: Some(1),
        }
    );
    assert!(starbase_data
        .guard_starbase_linkage_errors_current_known(1, 1)
        .is_empty());
    assert_eq!(
        starbase_data.guarding_fleet_record_indexes_current_known(),
        vec![1]
    );
    assert_eq!(
        starbase_data
            .guard_starbase_linkage_summaries_for_guarding_fleets_current_known(1)
            .len(),
        1
    );
    assert!(starbase_data
        .guard_starbase_linkage_errors_for_guarding_fleets_current_known(1)
        .is_empty());

    let post_data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat::parse(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbm: IpbmDat::parse(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: SetupDat::parse(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(post_data.ipbm_count_length_errors_current_known().is_empty());
    assert!(post_data
        .guarding_fleet_record_indexes_current_known()
        .is_empty());
    assert!(post_data
        .guard_starbase_linkage_summaries_for_guarding_fleets_current_known(1)
        .is_empty());
    assert_eq!(
        post_data.guard_starbase_linkage_errors_for_guarding_fleets_current_known(1),
        vec!["no guarding fleets found".to_string()]
    );
    assert!(post_data
        .guard_starbase_linkage_errors_current_known(1, 1)
        .iter()
        .any(|error| error.contains("guard enable expected 0x01")));
}
