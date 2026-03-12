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
