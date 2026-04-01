use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{CampaignStore, DiplomaticRelation, Order, TurnSubmission};
use nc_engine::build_seeded_initialized_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

#[test]
fn turn_submission_applies_mixed_player_actions() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .records
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let fleet_indexes = data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == 1)
        .map(|(idx, _)| idx + 1)
        .collect::<Vec<_>>();
    let fleet_record_index_1_based = fleet_indexes[0];
    let host_fleet_record_index_1_based = fleet_indexes[1];
    let target_coords = data.planets.records[planet_record_index_1_based - 1].coords_raw();
    let host_destroyers_before =
        data.fleets.records[host_fleet_record_index_1_based - 1].destroyer_count();

    {
        let fleet = &mut data.fleets.records[fleet_record_index_1_based - 1];
        fleet.set_destroyer_count(2);
        fleet.set_scout_count(1);
        fleet.set_troop_transport_count(2);
        fleet.set_army_count(0);
        fleet.recompute_max_speed_from_composition();
        fleet.set_current_speed(0);
    }
    {
        let host = &mut data.fleets.records[host_fleet_record_index_1_based - 1];
        host.set_current_location_coords_raw(target_coords);
    }
    {
        let planet = &mut data.planets.records[planet_record_index_1_based - 1];
        planet.set_stardock_kind_raw(0, 1);
        planet.set_stardock_count_raw(0, 2);
    }

    let kdl = format!(
        r#"
turn player=1 year=3000
tax rate=41
diplomacy to=2 relation="enemy"
planet record={planet_record_index_1_based} {{
  rename name="New Aurora"
  clear_build_queue
  build points=4 kind="scout"
  commission slot=1
}}
fleet record={fleet_record_index_1_based} {{
  roe value=4
  order speed=3 kind="scout_system" x={target_x} y={target_y}
  transfer to={host_fleet_record_index_1_based} destroyers=1
  load_armies planet={planet_record_index_1_based} qty=2
  unload_armies planet={planet_record_index_1_based} qty=1
}}
message to=2 subject="Border" body="Holding lane."
"#,
        target_x = target_coords[0],
        target_y = target_coords[1],
    );

    let submission = TurnSubmission::parse_kdl_str(&kdl).unwrap();
    let mut queued_mail = Vec::new();
    let report = submission.apply_to(&mut data, &mut queued_mail).unwrap();

    assert!(report.tax_changed);
    assert_eq!(report.diplomacy_updates, 1);
    assert_eq!(report.planet_blocks, 1);
    assert_eq!(report.fleet_blocks, 1);
    assert_eq!(report.messages_queued, 1);

    assert_eq!(data.player.records[0].tax_rate(), 41);
    assert_eq!(
        data.stored_diplomatic_relation(1, 2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        data.planets.records[planet_record_index_1_based - 1].planet_name(),
        "New Aurora"
    );
    assert_eq!(
        data.planets.records[planet_record_index_1_based - 1].build_count_raw(0),
        4
    );
    assert_eq!(
        data.planets.records[planet_record_index_1_based - 1].build_kind_raw(0),
        4
    );
    assert_eq!(
        data.planets.records[planet_record_index_1_based - 1].stardock_kind_raw(0),
        0
    );
    assert_eq!(
        data.fleets.records[fleet_record_index_1_based - 1].standing_order_kind(),
        Order::ScoutSolarSystem
    );
    assert_eq!(
        data.fleets.records[fleet_record_index_1_based - 1].destroyer_count(),
        1
    );
    assert_eq!(
        data.fleets.records[fleet_record_index_1_based - 1].army_count(),
        1
    );
    assert_eq!(
        data.fleets.records[host_fleet_record_index_1_based - 1].destroyer_count(),
        host_destroyers_before + 1
    );
    assert_eq!(queued_mail.len(), 1);
    assert_eq!(queued_mail[0].sender_empire_id, 1);
    assert_eq!(queued_mail[0].recipient_empire_id, 2);
    assert_eq!(queued_mail[0].subject, "Border");
    assert_eq!(queued_mail[0].body, "Holding lane.");
    assert!(data.fleets.records.len() > fleet_indexes.len());
}

#[test]
fn turn_submission_rejects_year_mismatch() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let submission = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3001
tax rate=20
"#,
    )
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("mismatched year should fail");
    assert!(err.to_string().contains("turn year mismatch"));
}

#[test]
fn turn_submission_loads_from_kdl_file() {
    let dir = std::env::temp_dir().join("nc-data-turn-kdl-load");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("turn.kdl");
    std::fs::write(
        &path,
        r#"
turn player=1 year=3000
message to=2 body="hello"
"#,
    )
    .unwrap();

    let submission = TurnSubmission::load_kdl(&path).unwrap();
    assert_eq!(submission.player_record_index_1_based, 1);
    assert_eq!(submission.year, 3000);
    assert_eq!(submission.messages.len(), 1);

    let _ = CampaignStore::open_default_in_dir(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn turn_submission_runtime_helper_check_only_does_not_create_runtime_db() {
    let dir = fixture_copy("nc-data-submit-turn-check");
    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=42
"#,
    )
    .unwrap();

    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, true).unwrap();

    assert_eq!(report.player_record_index_1_based, 1);
    assert!(report.tax_changed);
    assert!(!dir.join("ncgame.db").exists());

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_runtime_helper_apply_creates_runtime_db() {
    let dir = fixture_copy("nc-data-submit-turn-apply");
    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=37
message to=2 subject="Scout" body="Watch the lane."
"#,
    )
    .unwrap();

    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, false).unwrap();

    assert_eq!(report.messages_queued, 1);
    assert!(dir.join("ncgame.db").exists());

    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.player.records[0].tax_rate(), 37);
    assert_eq!(state.queued_mail.len(), 1);

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_rejects_fourth_message_to_same_recipient_in_same_year() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let mut queued_mail = vec![
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "One".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Two".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Three".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
    ];
    let submission = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3000
message to=2 subject="Four" body="Blocked"
"#,
    )
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut queued_mail)
        .expect_err("4th message should be rejected");
    assert!(
        err.to_string()
            .contains("You may only queue 3 messages to Empire 2 this turn.")
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn fixture_copy(prefix: &str) -> PathBuf {
    let root = unique_temp_dir(prefix);
    copy_dir_files(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    root
}

fn copy_dir_files(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create target dir");
    for entry in fs::read_dir(source).expect("read source dir") {
        let entry = entry.expect("dir entry");
        if !entry.file_type().expect("file type").is_file() {
            continue;
        }
        fs::copy(entry.path(), target.join(entry.file_name())).expect("copy file");
    }
}

fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
