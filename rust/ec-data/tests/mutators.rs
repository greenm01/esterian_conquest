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
    let mut data = CoreGameData {
        player: PlayerDat::parse(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: PlanetDat::parse(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: FleetDat::parse(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: BaseDat { records: vec![BaseRecord::new_zeroed(), BaseRecord::new_zeroed()] },
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
    assert_eq!(data.player1_ipbm_count_current_known(), 0);
    assert_eq!(
        data.current_known_core_state_errors(),
        vec![
            "BASES.DAT record count expected 0, got 2".to_string(),
            "IPBM.DAT record count expected 0, got 3".to_string(),
        ]
    );

    data.sync_player1_current_known_counts();

    assert_eq!(data.player.records[0].starbase_count_raw(), 2);
    assert_eq!(data.player.records[0].ipbm_count_raw(), 3);
    assert_eq!(data.player1_starbase_count_current_known(), 2);
    assert_eq!(data.player1_ipbm_count_current_known(), 3);
    assert!(data.current_known_core_state_errors().is_empty());
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
