    use ec_data::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../original/v1.5")
            .join(name)
    }

    fn initialized_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecutil-init/v1.5")
            .join(name)
    }

    fn post_maint_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-post/v1.5")
            .join(name)
    }

    fn f3_owner_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecutil-f3-owner/v1.5")
            .join(name)
    }

    fn ecmaint_build_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-build-pre/v1.5")
            .join(name)
    }

    fn ecmaint_build_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-build-post/v1.5")
            .join(name)
    }

    fn ecmaint_fleet_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-fleet-pre/v1.5")
            .join(name)
    }

    fn ecmaint_fleet_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-fleet-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_arrive_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-arrive/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_dev0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-dev0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_dev0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-dev0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_e0c_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-e0c-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_e0c_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-e0c-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b08_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b08-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b08_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b08-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b09_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b09-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b09_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b09-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_heavy_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-heavy-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_heavy_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-heavy-post/v1.5")
            .join(name)
    }

    fn read_fixture(name: &str) -> Vec<u8> {
        fs::read(fixture_path(name)).expect("fixture should exist")
    }

    fn read_ecmaint_starbase_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/ecmaint-starbase-pre/v1.5")
                .join(name),
        )
        .expect("ecmaint starbase-pre fixture should exist")
    }

    fn read_initialized_fixture(name: &str) -> Vec<u8> {
        fs::read(initialized_fixture_path(name)).expect("initialized fixture should exist")
    }

    fn read_post_maint_fixture(name: &str) -> Vec<u8> {
        fs::read(post_maint_fixture_path(name)).expect("post-maint fixture should exist")
    }

    fn read_f3_owner_fixture(name: &str) -> Vec<u8> {
        fs::read(f3_owner_fixture_path(name)).expect("f3 owner fixture should exist")
    }

    fn read_ecmaint_build_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_build_pre_fixture_path(name)).expect("ecmaint build-pre fixture should exist")
    }

    fn read_ecmaint_build_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_build_post_fixture_path(name)).expect("ecmaint build-post fixture should exist")
    }

    fn read_ecmaint_fleet_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_fleet_pre_fixture_path(name)).expect("ecmaint fleet-pre fixture should exist")
    }

    fn read_ecmaint_fleet_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_fleet_post_fixture_path(name)).expect("ecmaint fleet-post fixture should exist")
    }

    fn read_ecmaint_bombard_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_pre_fixture_path(name)).expect("ecmaint bombard-pre fixture should exist")
    }

    fn read_ecmaint_bombard_arrive_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_arrive_fixture_path(name)).expect("ecmaint bombard-arrive fixture should exist")
    }

    fn read_ecmaint_bombard_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_post_fixture_path(name)).expect("ecmaint bombard-post fixture should exist")
    }

    fn read_ecmaint_bombard_army0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_pre_fixture_path(name))
            .expect("ecmaint bombard-army0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_post_fixture_path(name))
            .expect("ecmaint bombard-army0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army0_dev0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_dev0_pre_fixture_path(name))
            .expect("ecmaint bombard-army0-dev0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army0_dev0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_dev0_post_fixture_path(name))
            .expect("ecmaint bombard-army0-dev0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_post_fixture_path(name))
            .expect("ecmaint bombard-army1-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_e0c_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-e0c-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_e0c_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_e0c_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-e0c-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b08_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b08_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b08-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b08_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b08_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b08-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b09_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b09_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b09-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b09_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b09_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b09-post fixture should exist")
    }

    fn read_ecmaint_bombard_heavy_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_heavy_pre_fixture_path(name))
            .expect("ecmaint bombard-heavy-pre fixture should exist")
    }

    fn read_ecmaint_bombard_heavy_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_heavy_post_fixture_path(name))
            .expect("ecmaint bombard-heavy-post fixture should exist")
    }

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
    fn planet_tail_fields_expose_owner_slot_and_likely_armies() {
        let init = PlanetDat::parse(&read_initialized_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(init.records[12].owner_empire_slot_raw(), 2);
        assert_eq!(init.records[12].ownership_status_raw(), 2);
        assert_eq!(init.records[12].likely_army_count_raw(), 4);

        assert_eq!(init.records[14].owner_empire_slot_raw(), 1);
        assert_eq!(init.records[14].ownership_status_raw(), 2);
        assert_eq!(init.records[14].likely_army_count_raw(), 4);

        let fleet_post = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(fleet_post.records[13].owner_empire_slot_raw(), 1);
        assert_eq!(fleet_post.records[13].ownership_status_raw(), 2);
        assert_eq!(fleet_post.records[13].likely_army_count_raw(), 0);
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
        assert_eq!(parsed.records[0].ownership_summary(), "rogue label='Rogues'");

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
                0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10,
                0x0D, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x81, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0D, 0x01,
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
        assert_eq!(parsed.records[0].standing_order_target_coords_raw(), [16, 13]);
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
        assert_eq!(parsed.records[2].standing_order_target_coords_raw(), [16, 13]);
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
    fn ecmaint_build_scenario_consumes_queue_and_changes_planet_state() {
        let pre = PlanetDat::parse(&read_ecmaint_build_pre_fixture("PLANETS.DAT")).unwrap();
        let post = PlanetDat::parse(&read_ecmaint_build_post_fixture("PLANETS.DAT")).unwrap();

        let pre_record = &pre.records[14];
        let post_record = &post.records[14];

        assert_eq!(pre_record.raw[0x24], 0x03);
        assert_eq!(pre_record.raw[0x2e], 0x01);

        assert_eq!(post_record.raw[0x24], 0x00);
        assert_eq!(post_record.raw[0x2e], 0x00);
        assert_eq!(post_record.raw[0x38], 0x03);
        assert_eq!(post_record.raw[0x4c], 0x01);
    }

    #[test]
    fn ecmaint_fleet_scenario_consumes_order_and_updates_fleet_and_planet_state() {
        let pre_fleets = FleetDat::parse(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap();
        let post_fleets = FleetDat::parse(&read_ecmaint_fleet_post_fixture("FLEETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();

        let pre_fleet = &pre_fleets.records[0];
        let post_fleet = &post_fleets.records[0];

        assert_eq!(pre_fleet.raw[0x0a], 0x03);
        assert_eq!(pre_fleet.raw[0x1f], 0x0c);
        assert_eq!(pre_fleet.raw[0x20], 0x0f);

        assert_eq!(post_fleet.raw[0x0b], 0x0f);
        assert_eq!(post_fleet.raw[0x19], 0x80);
        assert_eq!(post_fleet.raw[0x1a], 0xb9);
        assert_eq!(post_fleet.raw[0x1b], 0xff);
        assert_eq!(post_fleet.raw[0x1c], 0xff);
        assert_eq!(post_fleet.raw[0x1d], 0xff);
        assert_eq!(post_fleet.raw[0x1e], 0x7f);
        assert_eq!(post_fleet.raw[0x1f], 0x00);
        assert_eq!(post_fleet.raw[0x20], 0x0f);

        let post_planet = &post_planets.records[13];
        assert_eq!(post_planet.raw[0x58], 0x01);
        assert_eq!(post_planet.raw[0x5c], 0x02);
        assert_eq!(post_planet.raw[0x5d], 0x01);
    }

    #[test]
    fn ecmaint_bombard_scenario_arrival_preserves_attack_order() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_pre_fixture("FLEETS.DAT")).unwrap();
        let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let arrive_fleet = &arrive.records[2];

        assert_eq!(pre_fleet.current_speed(), 0x03);
        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.standing_order_target_coords_raw(), [0x0f, 0x0d]);

        assert_eq!(arrive_fleet.current_speed(), 0x03);
        assert_eq!(arrive_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(arrive_fleet.standing_order_target_coords_raw(), [0x0f, 0x0d]);
    }

    #[test]
    fn ecmaint_bombard_scenario_second_pass_consumes_order_and_kills_attackers() {
        let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_post_fixture("FLEETS.DAT")).unwrap();

        let arrive_fleet = &arrive.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(arrive_fleet.current_speed(), 0x03);
        assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(arrive_fleet.cruiser_count(), 0x03);
        assert_eq!(arrive_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x01);

        let arrive_planets = PlanetDat::parse(&read_ecmaint_bombard_arrive_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_post_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(arrive_planets.records[13].raw, post_planets.records[13].raw);
    }

    #[test]
    fn ecmaint_bombard_zero_army_target_changes_planet_without_attacker_losses() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_eq!(pre_target.developed_value_raw(), 0x8e);
        assert_eq!(post_target.developed_value_raw(), 0x8a);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_zero_army_zero_dev_target_changes_damage_pattern() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0x00);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army0_dev0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army0_dev0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_one_army_target_causes_partial_attacker_losses() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x02);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_eq!(pre_target.developed_value_raw(), 0x8e);
        assert_eq!(post_target.developed_value_raw(), 0x8d);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_one_army_zero_dev_target_changes_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x04);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_0e_increases_defender_damage_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x01);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x0e], 0x0c);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.raw[0x0e], 0x54);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_e0c_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_e0c_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_08_changes_defender_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x01);
        assert_eq!(post_fleet.destroyer_count(), 0x03);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x08], 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_b08_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_b08_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_09_changes_attacker_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        // army1-dev0 base: CA 3->2 (1 loss), DD 5->4 (1 loss)
        // army1-dev0-b09 (0x09=0): CA 3->1 (2 losses), DD 5->5 (0 losses)
        assert_eq!(post_fleet.cruiser_count(), 0x01);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x09], 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_b09_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_b09_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_heavy_generates_combat_report() {
        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_heavy_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_heavy_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x5A], 15); // Ground batteries mapped
        assert_eq!(pre_target.raw[0x58], 0x8E); // Armies mapped

        // Target capacity goes to 0 due to heavy bombardment
        assert_ne!(pre_target.raw, post_target.raw);

        // A report should be generated in RESULTS.DAT for player "FOO" (Empire 2)
        let results = read_ecmaint_bombard_heavy_post_fixture("RESULTS.DAT");
        assert!(!results.is_empty(), "RESULTS.DAT should contain the bombardment report");
    }
