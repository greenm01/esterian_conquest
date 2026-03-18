mod common;

use common::{
    cleanup_dir, copy_fixture_dir, run_classic_ecgame_smoke, run_ec_cli_in_dir,
    run_maint_rust_failure_after_import, run_maint_rust_with_export,
    set_mutual_enemy_in_player_dat, unique_temp_dir, write_mutual_enemy_diplomacy,
};
use ec_data::{CoreGameData, DatabaseDat, GameStateBuilder, Order};
use std::fs;

fn decode_chunked_report(bytes: &[u8]) -> String {
    const RESULTS_TEXT_SIZE: usize = 72;
    const RESULTS_TEXT_START: usize = 2;
    const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;
    bytes
        .chunks(84)
        .flat_map(|chunk| {
            if chunk.len() != 84 {
                return Vec::new();
            }
            let used = chunk[1] as usize;
            if used <= RESULTS_TEXT_SIZE
                && chunk[RESULTS_TEXT_START + used..RESULTS_TEXT_END]
                    .iter()
                    .all(|byte| *byte == 0)
            {
                return chunk[RESULTS_TEXT_START..RESULTS_TEXT_START + used].to_vec();
            }
            let text = &chunk[1..76];
            let end = text.iter().position(|b| *b == 0).unwrap_or(text.len());
            text[..end].to_vec()
        })
        .map(char::from)
        .collect::<String>()
}

fn results_records(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.chunks(84).filter(|chunk| chunk.len() == 84).collect()
}

fn result_header_record_indexes(records: &[&[u8]]) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter_map(|(idx, record)| {
            let used = record[1] as usize;
            let text = String::from_utf8_lossy(&record[2..2 + used.min(72)]);
            text.starts_with("From ").then_some(idx)
        })
        .collect()
}

fn result_record_text(record: &[u8]) -> String {
    let used = record[1] as usize;
    String::from_utf8_lossy(&record[2..2 + used.min(72)]).into_owned()
}

fn logical_result_reports(records: &[&[u8]]) -> Vec<(u8, Vec<String>)> {
    let header_indexes = result_header_record_indexes(records);
    let mut reports = Vec::new();
    for (pos, start) in header_indexes.iter().enumerate() {
        let end = header_indexes
            .get(pos + 1)
            .copied()
            .unwrap_or(records.len());
        let kind = records[*start][0];
        let lines = records[*start..end]
            .iter()
            .map(|record| result_record_text(record))
            .collect::<Vec<_>>();
        reports.push((kind, lines));
    }
    reports
}

#[test]
fn maint_rust_econ_updates_database_owner_intel_from_post_combat_planet_state() {
    let target = unique_temp_dir("ec-cli-maint-rust-econ");
    copy_fixture_dir("fixtures/ecmaint-econ-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Running Rust maintenance on:"));
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");

    let (planet_idx, planet) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.coords_raw() == [15, 13])
        .expect("econ combat target should exist");
    let year_bytes = (game_data.conquest.game_year() - 1).to_le_bytes();
    let owner_player = planet.owner_empire_slot_raw().saturating_sub(1) as usize;
    let planet_name = planet.planet_name();

    let owner_record = database.record(planet_idx, owner_player, game_data.planets.records.len());
    assert_eq!(owner_record.planet_name_bytes(), planet_name.as_bytes());
    assert_eq!(owner_record.raw[0x15], planet.owner_empire_slot_raw());
    assert_eq!(owner_record.raw[0x16], year_bytes[0]);
    assert_eq!(owner_record.raw[0x17], year_bytes[1]);
    assert_eq!(
        owner_record.raw[0x1e],
        0x40 + planet.owner_empire_slot_raw()
    );
    assert_eq!(owner_record.raw[0x23], planet.army_count_raw());
    assert_eq!(owner_record.raw[0x25], planet.ground_batteries_raw());

    let unrelated_player = (owner_player + 1) % 4;
    let unrelated_record = database.record(
        planet_idx,
        unrelated_player,
        game_data.planets.records.len(),
    );
    assert_eq!(unrelated_record.planet_name_bytes(), b"UNKNOWN");
    assert_eq!(unrelated_record.raw[0x15], 0xff);
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    let message_text = decode_chunked_report(&messages);
    assert!(
        message_text.contains("Bombardment mission report"),
        "MESSAGES.DAT decoded text was: {:?}",
        message_text
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_projects_latest_snapshot_back_into_working_directory() {
    let target = unique_temp_dir("ec-cli-maint-rust-in-place-classic");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let pre = CoreGameData::load(&target).expect("fixture should load");
    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust should project latest snapshot");
    assert_eq!(post.conquest.game_year(), pre.conquest.game_year() + 1);
    assert!(target.join("DATABASE.DAT").exists());
    assert!(target.join("RESULTS.DAT").exists());
    assert!(target.join("MESSAGES.DAT").exists());
    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reimports_live_classic_directory_changes_before_processing() {
    let target = unique_temp_dir("ec-cli-maint-rust-live-classic-sync");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    common::import_campaign_db(&target);

    let mut classic_side = CoreGameData::load(&target).expect("fixture should load");
    classic_side.player.records[1].set_assigned_player_handle_raw("SYSOP");
    classic_side.player.records[1].set_controlled_empire_name_raw("foo");
    classic_side
        .save(&target)
        .expect("classic-side edit should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        post.player.records[1].assigned_player_handle_summary(),
        "SYSOP"
    );
    assert_eq!(
        post.player.records[1].controlled_empire_name_summary(),
        "foo"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_preserves_prepared_classic_login_classification_across_turns() {
    let target = unique_temp_dir("ec-cli-maint-rust-classic-login-classification");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let prepare = run_ec_cli_in_dir(
        &[
            "classic-login-prepare",
            target.to_str().unwrap(),
            "2",
            "SYSOP",
            "foo",
        ],
        common::rust_workspace(),
    );
    assert!(prepare.contains("Prepared classic login for player 2"));

    let before = run_ec_cli_in_dir(
        &["inspect-classic-login", target.to_str().unwrap(), "SYSOP"],
        common::rust_workspace(),
    );
    assert!(before.contains("slot 2: classification=matched-preloaded-first-login"));

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let after = run_ec_cli_in_dir(
        &["inspect-classic-login", target.to_str().unwrap(), "SYSOP"],
        common::rust_workspace(),
    );
    assert!(after.contains("slot 2: classification=matched-preloaded-first-login"));
    assert!(after.contains("handle='SYSOP'"));

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn maint_rust_output_reopens_in_classic_ecgame_smoke() {
    let target = unique_temp_dir("ec-cli-maint-rust-classic-ecgame");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    run_classic_ecgame_smoke(&target, 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_fleet_battle_generates_results_report_from_battle_events() {
    let target = unique_temp_dir("ec-cli-maint-rust-fleet-battle");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain battle summaries"
    );
    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Fleet battle report"));
    assert!(text.contains("<end of transmission>"));
    assert!(text.contains("System("));
    assert!(text.contains("Initial observed hostile composition:"));
    assert_eq!(
        &results[82..84],
        &post.conquest.game_year().to_le_bytes(),
        "battle fixture should stamp the current report year into the results tail"
    );
    let records = results_records(&results);
    assert!(!records.is_empty(), "expected RESULTS.DAT records");
    let header_indexes = result_header_record_indexes(&records);
    assert!(
        header_indexes.len() >= 2,
        "expected multiple logical reports in RESULTS.DAT"
    );
    let first_chain_id = u16::from_le_bytes([records[0][74], records[0][75]]);
    let first_next_id = u16::from_le_bytes([records[0][78], records[0][79]]);
    assert_eq!(first_chain_id, 0, "first logical report should start with cursor id 0");
    assert_eq!(
        first_next_id,
        (header_indexes[1] + 1) as u16,
        "header should point at the next header record index plus one"
    );
    assert_eq!(
        u16::from_le_bytes([records[1][74], records[1][75]]),
        first_chain_id,
        "continuation should stay on the same report chain id"
    );
    assert_eq!(
        u16::from_le_bytes([records[1][78], records[1][79]]),
        first_next_id,
        "continuation records should preserve the report's next header id"
    );
    let eot = records
        .iter()
        .find(|record| record[1] == 21 && &record[2..23] == b"<end of transmission>")
        .expect("expected explicit end-of-transmission record");
    assert_eq!(
        u16::from_le_bytes([eot[74], eot[75]]),
        first_chain_id,
        "EOT should remain part of the same report chain"
    );
    assert_eq!(
        u16::from_le_bytes([eot[78], eot[79]]),
        first_next_id,
        "EOT should preserve the report's next header id"
    );
    assert_eq!(post.player.records[0].classic_results_chain_flag_raw(), 1);
    assert_eq!(
        u16::from_le_bytes([
            records[*header_indexes.last().unwrap()][74],
            records[*header_indexes.last().unwrap()][75],
        ]),
        (header_indexes[header_indexes.len() - 2] + 1) as u16,
        "later headers should inherit the previous header index plus one"
    );
    assert_eq!(
        post.player.records[0].classic_results_chain_next_free_raw(),
        (header_indexes.last().copied().unwrap() + 1) as u16,
        "player review state should advertise the last header index plus one"
    );
    assert!(
        post.player.records[0].classic_results_chain_next_free_raw() >= first_next_id,
        "player should advertise classic undeleted results"
    );
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    let text = decode_chunked_report(&messages);
    assert!(
        text.contains("Fleet battle report"),
        "MESSAGES.DAT decoded text was: {:?}",
        text
    );
    assert!(text.contains("Initial observed hostile composition:"));
    assert!(text.contains("For Empire #"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_contact_and_identify_use_classic_result_kinds() {
    let target = unique_temp_dir("ec-cli-maint-rust-classic-scout-kinds");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);

    let sensor_contact = records
        .iter()
        .find(|record| {
            let used = record[1] as usize;
            String::from_utf8_lossy(&record[2..2 + used.min(72)])
                .contains("Sensor contact shows an alien fleet")
        })
        .expect("expected sensor contact report");
    assert_eq!(
        sensor_contact[0], 0x05,
        "initial scout contact should use classic kind 0x05"
    );

    let identified = records
        .iter()
        .find(|record| {
            let used = record[1] as usize;
            String::from_utf8_lossy(&record[2..2 + used.min(72)])
                .contains("We have located and identified the alien fleet")
        })
        .expect("expected identified scout report");
    assert_eq!(
        identified[0], 0x06,
        "identified scout follow-up should use classic kind 0x06"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_results_emit_left_justified_wrapped_lines_without_generic_fleet_headers() {
    let target = unique_temp_dir("ec-cli-maint-rust-classic-results-lines");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);
    for record in &records {
        let line = result_record_text(record);
        assert!(
            line.chars().count() <= 72,
            "classic RESULTS line exceeded width: {:?}",
            line
        );
        if !line.starts_with("From ") && line != "<end of transmission>" {
            assert!(
                !line.starts_with(' '),
                "body lines should be left-justified: {:?}",
                line
            );
        }
    }

    let normalized = decode_chunked_report(&results);
    assert!(
        !normalized.contains("From your fleet, located"),
        "fleet-origin reports should name a specific fleet: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_contact_and_identify_are_separate_classic_reports() {
    let target = unique_temp_dir("ec-cli-maint-rust-classic-scout-boundaries");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let reports = logical_result_reports(&results_records(&results));
    let sensor_idx = reports
        .iter()
        .position(|(_, lines)| {
            lines
                .iter()
                .any(|line| line.contains("Sensor contact shows an alien fleet"))
        })
        .expect("expected sensor contact report");
    let identify_idx = reports
        .iter()
        .position(|(_, lines)| {
            lines
                .iter()
                .any(|line| line.contains("We have located and identified the alien fleet"))
        })
        .expect("expected identified report");

    assert_eq!(sensor_idx + 1, identify_idx);
    assert_eq!(reports[sensor_idx].0, 0x05);
    assert_eq!(reports[identify_idx].0, 0x06);
    assert_eq!(
        reports[sensor_idx].1.last().map(String::as_str),
        Some("<end of transmission>")
    );
    assert_eq!(
        reports[identify_idx].1.last().map(String::as_str),
        Some("<end of transmission>")
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_routed_reports_set_classic_pending_flags() {
    let target = unique_temp_dir("ec-cli-maint-rust-pending-flags");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.player.records[0].classic_reports_pending_flag_raw(), 1);
    assert_eq!(game_data.player.records[0].classic_messages_pending_flag_raw(), 1);
    assert_eq!(game_data.player.records[0].classic_results_review_word_raw(), 1);
    assert_eq!(game_data.player.records[0].classic_message_review_word_raw(), 1);
    assert_eq!(game_data.player.records[1].classic_reports_pending_flag_raw(), 1);
    assert_eq!(game_data.player.records[1].classic_messages_pending_flag_raw(), 1);
    assert_eq!(game_data.player.records[1].classic_results_review_word_raw(), 1);
    assert_eq!(game_data.player.records[1].classic_message_review_word_raw(), 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_uses_stored_player_diplomacy_without_sidecar() {
    let target = unique_temp_dir("ec-cli-maint-rust-player-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    set_mutual_enemy_in_player_dat(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Fleet battle report"));
    assert!(text.contains("We lost all contact") || text.contains("held the field"));

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "stored PLAYER.DAT diplomacy should not require a sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_absorbs_small_game_sidecar_diplomacy_into_player_dat() {
    let target = unique_temp_dir("ec-cli-maint-rust-sidecar-persist-four");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(ec_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(ec_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    let sidecar_text =
        fs::read_to_string(&diplomacy_sidecar).expect("diplomacy.kdl should still exist");
    assert!(
        sidecar_text.trim().is_empty(),
        "persistable small-game diplomacy should migrate into PLAYER.DAT and clear the sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_uses_stored_player_diplomacy_without_sidecar_for_large_games() {
    let target = unique_temp_dir("ec-cli-maint-rust-player-diplomacy-nine");
    let stdout = run_ec_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "ec-data/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    let mut game_data = CoreGameData::load(&target).expect("generated game should load");
    let fleet_a = &mut game_data.fleets.records[0];
    fleet_a.set_current_location_coords_raw([8, 8]);
    fleet_a.set_standing_order_kind(Order::ScoutSector);
    fleet_a.set_standing_order_target_coords_raw([8, 8]);
    fleet_a.set_current_speed(0);
    fleet_a.raw[0x19] = 0x81;
    fleet_a.set_destroyer_count(1);
    fleet_a.set_cruiser_count(0);
    fleet_a.set_battleship_count(0);
    fleet_a.set_troop_transport_count(0);
    fleet_a.set_army_count(0);
    fleet_a.set_scout_count(1);
    fleet_a.set_etac_count(0);
    fleet_a.set_rules_of_engagement(10);

    let fleet_b = &mut game_data.fleets.records[(8 * 4) as usize];
    fleet_b.set_current_location_coords_raw([8, 8]);
    fleet_b.set_standing_order_kind(Order::HoldPosition);
    fleet_b.set_standing_order_target_coords_raw([8, 8]);
    fleet_b.set_current_speed(0);
    fleet_b.raw[0x19] = 0x81;
    fleet_b.set_destroyer_count(1);
    fleet_b.set_cruiser_count(0);
    fleet_b.set_battleship_count(0);
    fleet_b.set_troop_transport_count(0);
    fleet_b.set_army_count(0);
    fleet_b.set_scout_count(0);
    fleet_b.set_etac_count(0);
    fleet_b.set_rules_of_engagement(10);

    game_data
        .set_stored_diplomatic_relation(1, 9, ec_data::DiplomaticRelation::Enemy)
        .expect("player 1 -> 9 diplomacy should set");
    game_data
        .set_stored_diplomatic_relation(9, 1, ec_data::DiplomaticRelation::Enemy)
        .expect("player 9 -> 1 diplomacy should set");
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Fleet battle report"));

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "stored PLAYER.DAT diplomacy should cover larger tiers without a sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_updates_large_game_database_from_scout_intel_event() {
    let target = unique_temp_dir("ec-cli-maint-rust-large-database-intel");
    let stdout = run_ec_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "ec-data/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    let mut game_data = CoreGameData::load(&target).expect("generated game should load");
    let (planet_idx, coords, owner_empire_raw) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw(), planet.owner_empire_slot_raw()))
        .expect("generated game should contain an empire 2 world");

    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw(coords);
    scout.set_current_speed(3);
    scout.raw[0x19] = 0x00;
    scout.set_scout_count(1);
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let planet_count = game_data.planets.records.len();

    let viewer_record = database.record(planet_idx, 0, planet_count);
    assert_ne!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert!(!viewer_record.planet_name_bytes().is_empty());
    assert_eq!(viewer_record.raw[0x15], owner_empire_raw);

    let unrelated_viewer_record = database.record(planet_idx, 2, planet_count);
    assert_eq!(unrelated_viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert_ne!(unrelated_viewer_record.raw[0x15], owner_empire_raw);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blockade_arrival_persists_enemy_relation_in_player_dat() {
    let target = unique_temp_dir("ec-cli-maint-rust-blockade-escalation");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 2)
        .map(|planet| planet.coords_raw())
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x00;
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(ec_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(ec_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "blockade escalation should persist directly into PLAYER.DAT"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_without_enemy_declaration_reports_contact_without_forcing_battle() {
    let target = unique_temp_dir("ec-cli-maint-rust-peaceful-contact");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet_a = &mut game_data.fleets.records[0];
    fleet_a.set_current_location_coords_raw([8, 8]);
    fleet_a.set_standing_order_kind(Order::ScoutSector);
    fleet_a.set_standing_order_target_coords_raw([8, 8]);
    fleet_a.set_current_speed(0);
    fleet_a.raw[0x19] = 0x81;
    fleet_a.set_destroyer_count(1);
    fleet_a.set_cruiser_count(0);
    fleet_a.set_battleship_count(0);
    fleet_a.set_troop_transport_count(0);
    fleet_a.set_army_count(0);
    fleet_a.set_scout_count(1);
    fleet_a.set_etac_count(0);
    fleet_a.set_rules_of_engagement(10);

    let fleet_b = &mut game_data.fleets.records[4];
    fleet_b.set_current_location_coords_raw([8, 8]);
    fleet_b.set_standing_order_kind(Order::HoldPosition);
    fleet_b.set_standing_order_target_coords_raw([8, 8]);
    fleet_b.set_current_speed(0);
    fleet_b.raw[0x19] = 0x81;
    fleet_b.set_destroyer_count(1);
    fleet_b.set_cruiser_count(0);
    fleet_b.set_battleship_count(0);
    fleet_b.set_troop_transport_count(0);
    fleet_b.set_army_count(0);
    fleet_b.set_scout_count(0);
    fleet_b.set_etac_count(0);
    fleet_b.set_rules_of_engagement(10);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));
    assert!(!text.contains("Fleet battle report"));
    assert!(!text.contains("We lost all contact"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rejects_invalid_diplomacy_sidecar() {
    let target = unique_temp_dir("ec-cli-maint-rust-invalid-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    fs::write(
        target.join("diplomacy.kdl"),
        "relation from=1 to=99 status=\"enemy\"\n",
    )
    .expect("invalid diplomacy.kdl should write");

    let stderr = run_maint_rust_failure_after_import(&target, 1);
    assert!(stderr.contains("1..=4") || stderr.contains("1..=25"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_persists_sidecar_diplomacy_into_player_dat_for_large_games() {
    let target = unique_temp_dir("ec-cli-maint-rust-sidecar-persist-nine");
    let stdout = run_ec_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "ec-data/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    write_mutual_enemy_diplomacy(&target, 1, 9);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(9),
        Some(ec_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[8].diplomatic_relation_toward(1),
        Some(ec_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    let sidecar_text =
        fs::read_to_string(&diplomacy_sidecar).expect("diplomacy.kdl should still exist");
    assert!(
        sidecar_text.trim().is_empty(),
        "persistable diplomacy should migrate into PLAYER.DAT and clear the sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_fleet_generates_lost_contact_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-lost-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("We lost all contact with the"));
    assert!(text.contains("Fleet Command Center"));
    assert!(text.contains("flight recorder"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_starbase_generates_lost_contact_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-lost-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let starbase_coords = game_data.bases.records[0].coords_raw();
    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw(starbase_coords);
    attacker.set_standing_order_kind(Order::MoveOnly);
    attacker.set_standing_order_target_coords_raw(starbase_coords);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x81;
    attacker.set_rules_of_engagement(10);
    attacker.set_destroyer_count(20);
    attacker.set_cruiser_count(10);
    attacker.set_battleship_count(5);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From Starbase"));
    assert!(text.contains("We lost all contact with Starbase"));
    assert!(text.contains("burnt flight recorder"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.player.records[0].starbase_count_raw(), 0);
    assert!(
        game_data
            .bases
            .records
            .iter()
            .all(|base| !(base.coords_raw() == starbase_coords
                && base.owner_empire_raw() == 1
                && base.active_flag_raw() != 0))
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_generates_results_report_from_colony_event() {
    let target = unique_temp_dir("ec-cli-maint-rust-colonize");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain colonization summaries"
    );
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your 1st Fleet, located in System("));
    assert!(text.contains("successfully established"));
    assert!(text.contains("Not Named Yet"));
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    let message_text = decode_chunked_report(&messages);
    assert!(
        message_text.contains("successfully established"),
        "MESSAGES.DAT decoded text was: {:?}",
        message_text
    );
    assert!(message_text.contains("For Empire #"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_preserves_existing_classic_player_mail_when_no_rust_messages_are_emitted() {
    let target = unique_temp_dir("ec-cli-maint-rust-preserve-classic-mail");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    for fleet in &mut game_data.fleets.records {
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_current_speed(0);
        fleet.raw[0x19] = 0x00;
    }
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let classic_mail = b"\x18this is a message to you\x00classic-payload".to_vec();
    fs::write(target.join("MESSAGES.DAT"), &classic_mail).expect("should seed classic mail");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert_eq!(
        messages, classic_mail,
        "maint-rust should not erase pending classic player mail when it has no routed maint messages to add"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_blocked_by_owner_generates_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-colonize-blocked");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let blocked = &mut game_data.planets.records[13];
    blocked.set_owner_empire_slot_raw(2);
    blocked.set_ownership_status_raw(2);
    blocked.set_planet_name("TargetPrime");
    blocked.set_army_count_raw(10);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain blocked colonization summaries"
    );
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your 1st Fleet, located in System("));
    assert!(text.contains("ot establish a colony on planet"));
    assert!(text.contains("already occupie"));
    // Stardate header takes the first 75-byte chunk; planet name may span record
    // boundaries depending on empire-label length. Check independently for both halves.
    assert!(text.contains("Targ") || text.contains("etPrime"), "Planet name should appear in report");
    assert!(text.contains("Empire #2"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_sector_generates_results_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-sector");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain scout summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    // "beginning to scout this sector" may span record boundaries with the new Stardate header.
    assert!(text.contains("arrived at our destination") || text.contains("beginning to scout"));
    assert!(text.contains("Sector(15,13)"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_system_generates_results_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-system");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain scout summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("Owner:"));
    assert!(text.contains("Ground batteries:"));
    assert!(text.contains("System(15,13)"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.records.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );
    assert_eq!(
        viewer_record.raw[0x15],
        game_data.planets.records[13].owner_empire_slot_raw()
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_view_world_generates_results_and_database_intel() {
    let target = unique_temp_dir("ec-cli-maint-rust-view-world");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Viewing mission report"));
    // Strings near the 75-byte chunk boundary may be split across records in the new Stardate
    // header format. Check for unambiguous early-body content instead.
    assert!(text.contains("entered System(15,13)") || text.contains("long range"));
    assert!(text.contains("has a") || text.contains("potential"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.records.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_refreshes_database_between_turns_for_route_hazards() {
    let target = unique_temp_dir("ec-cli-maint-rust-routing-refresh");

    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let foreign_world = &mut game_data.planets.records[4];
    foreign_world.set_coords_raw([4, 2]);
    foreign_world.set_owner_empire_slot_raw(2);
    foreign_world.set_ownership_status_raw(2);
    foreign_world.set_planet_name("TargetPrime");
    foreign_world.set_ground_batteries_raw(3);
    foreign_world.set_army_count_raw(9);

    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw([2, 2]);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([4, 2]);
    scout.set_current_speed(3);
    scout.set_scout_count(1);

    let mover = &mut game_data.fleets.records[1];
    mover.set_current_location_coords_raw([0, 2]);
    mover.set_standing_order_kind(Order::MoveOnly);
    mover.set_standing_order_target_coords_raw([6, 2]);
    mover.set_current_speed(3);

    game_data.save(&target).expect("baseline should save");

    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.planet_name())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year(),
        game_data.conquest.player_count() as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Turn 1: year 3001"));
    assert!(stdout.contains("Turn 2: year 3002"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let mover_location = game_data.fleets.records[1].current_location_coords_raw();
    assert_ne!(
        mover_location,
        [4, 2],
        "second-turn routing should avoid the now-known foreign world"
    );

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(4, 0, game_data.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_starbase_generates_arrival_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data.bases.records[0].coords_raw();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_sub(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard Starbase mission report"));
    assert!(text.contains("guard/escort"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_blockade_generates_arrival_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-blockade");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let guard = &mut game_data.fleets.records[0];
    guard.set_standing_order_kind(Order::GuardBlockadeWorld);
    guard.set_standing_order_target_coords_raw([15, 13]);
    guard.set_scout_count(0);
    guard.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("arrived at planet"));
    assert!(text.contains("assignment"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_salvage_generates_report_and_removes_fleet() {
    let target = unique_temp_dir("ec-cli-maint-rust-salvage");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let (planet_idx, target_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = game_data.planets.records[planet_idx].stored_production_points();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;
    game_data
        .save(&target)
        .expect("mutated fixture should save");
    let db_path = target.join("ecgame.db");
    if db_path.exists() {
        fs::remove_file(&db_path).expect("stale ecgame.db should be removable");
    }

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Salvage mission report"));
    assert!(text.contains("yield 10 production point(s)"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.fleets.records.len(), 15);
    assert_eq!(
        game_data.planets.records[planet_idx].stored_production_points(),
        stored_before + 10
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_bombardment_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-bombard-report");
    copy_fixture_dir("fixtures/ecmaint-bombard-arrive/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Bombardment mission report"));
    assert!(text.contains("bombing run"));
    assert!(text.contains("The defending world initially contained"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invade_failure_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-invade-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        40,
        0,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Invasion mission report"));
    assert!(text.contains("repulsed") || text.contains("landing was"));
    // "defending world initially contained" may span a record boundary with the Stardate header.
    assert!(text.contains("defending world") || text.contains("initially contained"));
    assert!(text.contains("ground batteries") || text.contains("armies"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_success_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-blitz-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(10);
    attacker.set_army_count(10);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Blitz mission report"));
    assert!(text.contains("defending world initially contained"));
    assert!(text.contains("Friendly losses:"));
    assert!(text.contains("Enemy losses:"));
    assert!(text.contains("during the landing"));
    assert_eq!(text.matches("Blitz mission report").count(), 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_generates_move_abort_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-battle-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::MoveOnly);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Move mission report"));
    assert!(text.contains("abort our mission") || text.contains("abort our"));
    // "seek safety" may span a record boundary with the Stardate header.
    assert!(text.contains("seek safe") || text.contains("abort our mission"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_roe_withdrawal_generates_composition_and_loss_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-roe-withdrawal");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = [15, 13];

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(Order::PatrolSector);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    fleet.set_destroyer_count(6);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.set_rules_of_engagement(8);

    let hostile = &mut game_data.fleets.records[4];
    hostile.set_current_location_coords_raw(coords);
    hostile.set_standing_order_kind(Order::MoveOnly);
    hostile.set_standing_order_target_coords_raw(coords);
    hostile.set_current_speed(3);
    hostile.set_destroyer_count(2);
    hostile.set_cruiser_count(2);
    hostile.set_battleship_count(0);
    hostile.set_scout_count(0);
    hostile.set_troop_transport_count(0);
    hostile.set_army_count(0);
    hostile.set_etac_count(0);
    hostile.set_rules_of_engagement(10);

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let lines = results_records(&results)
        .into_iter()
        .map(result_record_text)
        .collect::<Vec<_>>();
    let normalized = lines.join(" ");
    assert!(normalized.contains("withdrew under our ROE"));
    assert!(normalized.contains("Initial observed hostile composition:"));
    assert!(normalized.contains("We observed enemy losses of"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalid_fleet_order_generates_sanitization_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-invalid-fleet-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([15, 13]);
    fleet.set_standing_order_kind(Order::BombardWorld);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x80;
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Order validation report"));
    assert!(text.contains("required combat ships"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalid_planet_inputs_generate_admin_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-invalid-planet-input");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let planet_idx = game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("fixture should contain owned planet");
    game_data.planets.records[planet_idx].set_build_count_raw(0, 12);
    game_data.planets.records[planet_idx].set_build_kind_raw(0, 0xfe);
    game_data.player.records[0].set_tax_rate_raw(255);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Administration report"));
    assert!(text.contains("Tax rate input 255%"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_sanitizes_mixed_invalid_player_inputs_and_exports_loadable_state() {
    let target = unique_temp_dir("ec-cli-maint-rust-mixed-invalid-inputs");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([15, 13]);
    fleet.set_standing_order_kind(Order::BombardWorld);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    fleet.set_current_speed(fleet.max_speed().saturating_add(3));
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(1);
    fleet.set_army_count(3);
    fleet.set_rules_of_engagement(6);
    game_data.planets.records[0].set_build_count_raw(0, 9);
    game_data.planets.records[0].set_build_kind_raw(0, 0xfe);
    game_data.player.records[0].set_tax_rate_raw(255);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should remain loadable");
    assert_eq!(
        reloaded.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(reloaded.fleets.records[0].current_speed(), 0);
    assert_eq!(reloaded.fleets.records[0].army_count(), 1);
    assert_eq!(reloaded.fleets.records[0].rules_of_engagement(), 0);
    assert_eq!(reloaded.player.records[0].tax_rate(), 100);
    assert_eq!(reloaded.planets.records[0].build_count_raw(0), 0);
    assert_eq!(reloaded.planets.records[0].build_kind_raw(0), 0);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let result_text = decode_chunked_report(&results);
    assert!(result_text.contains("Order validation report"));
    assert!(result_text.contains("Fleet readiness report"));
    assert!(result_text.contains("Administration report"));
    assert!(result_text.contains("Tax rate input 255%"));

    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    let message_text = decode_chunked_report(&messages);
    assert!(message_text.contains("Fleet readiness report"));
    assert!(message_text.contains("Order validation report"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_survives_deterministic_malformed_directory_matrix() {
    for order_code in [16u8, 17, 24, 31] {
        let target = unique_temp_dir(&format!("ec-cli-maint-rust-invalid-matrix-{order_code}"));
        copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

        let mut game_data = CoreGameData::load(&target).expect("fixture should load");
        let fleet = &mut game_data.fleets.records[0];
        fleet.set_current_location_coords_raw([15, 13]);
        fleet.set_standing_order_code_raw(order_code);
        fleet.set_standing_order_target_coords_raw([15, 13]);
        fleet.set_current_speed(99);
        fleet.set_mission_aux_bytes([0xfe, 0xfe]);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(1);
        fleet.set_troop_transport_count(1);
        fleet.set_army_count(4);
        fleet.set_rules_of_engagement(42);
        game_data.planets.records[0].set_build_count_raw(0, 9);
        game_data.planets.records[0].set_build_kind_raw(0, 0xfe);
        game_data.planets.records[0].set_stardock_count_raw(0, 2);
        game_data.planets.records[0].set_stardock_kind_raw(0, 0xfe);
        game_data.player.records[0].set_tax_rate_raw(255);
        game_data
            .save(&target)
            .expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(
            stdout.contains("Rust maintenance complete."),
            "maint-rust failed for order code {order_code:#04x}: {stdout}"
        );

        let reloaded =
            CoreGameData::load(&target).expect("maint-rust output should remain loadable");
        assert_eq!(
            reloaded.player.records[0].tax_rate(),
            100,
            "tax rate should clamp for order code {order_code:#04x}"
        );

        let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
        let result_text = decode_chunked_report(&results);
        assert!(
            result_text.contains("Order validation report")
                || result_text.contains("Fleet readiness report"),
            "RESULTS.DAT decoded text was: {:?}",
            result_text
        );

        cleanup_dir(&target);
    }
}

#[test]
fn maint_rust_survives_multi_fixture_invalid_input_sweep() {
    for fixture in [
        "fixtures/ecmaint-post/v1.5",
        "fixtures/ecmaint-fleet-pre/v1.5",
        "fixtures/ecmaint-fleet-battle-pre/v1.5",
    ] {
        let slug = fixture
            .split('/')
            .nth(1)
            .unwrap_or("fixture")
            .replace("ecmaint-", "");
        let target = unique_temp_dir(&format!("ec-cli-maint-rust-sweep-{slug}"));
        copy_fixture_dir(fixture, &target);

        let mut game_data = CoreGameData::load(&target).expect("fixture should load");
        if let Some(fleet) = game_data.fleets.records.get_mut(0) {
            fleet.set_standing_order_code_raw(0xfe);
            fleet.set_current_speed(99);
            fleet.set_troop_transport_count(1);
            fleet.set_army_count(4);
            fleet.set_rules_of_engagement(42);
        }
        if let Some(planet) = game_data.planets.records.get_mut(0) {
            planet.set_build_count_raw(0, 9);
            planet.set_build_kind_raw(0, 0xfe);
            planet.set_stardock_count_raw(0, 2);
            planet.set_stardock_kind_raw(0, 0xfe);
        }
        if let Some(player) = game_data.player.records.get_mut(0) {
            player.set_tax_rate_raw(255);
            player.raw[0x54] = 0x01;
            player.raw[0x55] = 0xfe;
        }
        game_data
            .save(&target)
            .expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(stdout.contains("Rust maintenance complete."));

        let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
        assert!(
            reloaded.player.records[0].tax_rate() <= 100,
            "fixture {fixture} left invalid tax rate {}",
            reloaded.player.records[0].tax_rate()
        );
        let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
        let text = decode_chunked_report(&results);
        assert!(
            text.contains("Order validation report")
                || text.contains("Fleet readiness report")
                || text.contains("foreign ministry"),
            "fixture {fixture} produced unexpected RESULTS.DAT text: {:?}",
            text
        );

        cleanup_dir(&target);
    }
}

#[test]
fn maint_rust_reports_invalid_diplomacy_input_sanitization() {
    let target = unique_temp_dir("ec-cli-maint-rust-invalid-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x54] = 0x01;
    game_data.player.records[0].raw[0x55] = 0xfe;
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should remain loadable");
    assert_eq!(reloaded.player.records[0].raw[0x54], 0x00);
    assert_eq!(reloaded.player.records[0].raw[0x55], 0x00);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("foreign ministry"));
    assert!(text.contains("invalid diplomacy input"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_scout_report_mentions_retreat_destination() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::ScoutSector);
    game_data.fleets.records[0].set_scout_count(1);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));
    assert!(text.contains("identified the alien fleet") || text.contains("located and ident"));
    assert!(text.contains("From your 1st Fleet, located in"));
    assert!(text.contains("the ") && text.contains(" Fleet of Empire #"));
    assert!(text.contains("withdraw toward") || text.contains("seeking safety"));
    assert!(text.contains("planet \"") || text.contains("System("));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_arrival_generates_waiting_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-rendezvous-wait");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_kind(Order::RendezvousSector);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("waiting for more fleets"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_merge_generates_join_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-join-merge");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Join mission report"));
    assert!(text.contains("From your 2nd Fleet, located in"));
    assert!(text.contains("now merging"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_merge_generates_absorbing_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-rendezvous-absorb");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("absorbing the") || text.contains("merging with the"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_contact_uses_join_report_label() {
    let target = unique_temp_dir("ec-cli-maint-rust-join-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::JoinAnotherFleet);
    let host_id = game_data.fleets.records[1].fleet_id();
    game_data.fleets.records[0].set_join_host_fleet_id_raw(host_id);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Join mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_contact_uses_guard_report_label() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::GuardBlockadeWorld);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_empire_falling_into_civil_disorder() {
    let target = unique_temp_dir("ec-cli-maint-rust-civil-disorder");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(post.player.records[0].owner_mode_raw(), 0x00);
    assert_eq!(
        post.player.records[0].legacy_status_name_summary(),
        "In Civil Disorder"
    );

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized.to_ascii_lowercase().contains("civil disorder"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_when_one_serious_contender_remains() {
    let target = unique_temp_dir("ec-cli-maint-rust-campaign-outlook");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        post.campaign_outlook(),
        ec_data::CampaignOutlook::SoleContender(1)
    );

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized
            .to_ascii_lowercase()
            .contains("sole remaining serious contender"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_emperor_recognition_when_only_stable_empire_remains() {
    let target = unique_temp_dir("ec-cli-maint-rust-emperor-recognition");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        post.campaign_outcome(),
        ec_data::CampaignOutcome::RecognizedEmperor(1)
    );

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized
            .to_ascii_lowercase()
            .contains("recognized as emperor"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_fleet_defection_after_civil_disorder() {
    let target = unique_temp_dir("ec-cli-maint-rust-fleet-defection");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    let remaining_player_fleets = post
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .count();
    assert_eq!(remaining_player_fleets, 3);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized
            .to_ascii_lowercase()
            .contains("crews have defected"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_seeded_games_survive_five_turns_across_manual_player_tiers() {
    for (player_count, seed) in [(4u8, 1515u64), (9, 2025), (16, 4242), (25, 5151)] {
        let target = unique_temp_dir(&format!("ec-cli-maint-rust-multiturn-{player_count}p"));

        let stdout = run_ec_cli_in_dir(
            &[
                "sysop",
                "new-game",
                target.to_str().unwrap(),
                "--config",
                "ec-data/config/setup.example.kdl",
                "--players",
                &player_count.to_string(),
                "--seed",
                &seed.to_string(),
            ],
            common::rust_workspace(),
        );
        assert!(
            stdout.contains("Initialized new game at:"),
            "sysop new-game stdout was: {stdout:?}"
        );

        let stdout = run_maint_rust_with_export(&target, 5);
        assert!(stdout.contains("Rust maintenance complete."));

        let post = CoreGameData::load(&target).expect("multi-turn maint output should load");
        assert_eq!(post.conquest.game_year(), 3005);
        assert!(
            post.ecmaint_preflight_errors().is_empty(),
            "preflight errors after five turns for {player_count} players: {:?}",
            post.ecmaint_preflight_errors()
        );

        let database_bytes =
            fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
        let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
        assert_eq!(
            database.records.len(),
            post.player.records.len() * post.planets.records.len()
        );

        cleanup_dir(&target);
    }
}
