use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::fleet_motion_state::reset_motion_state_for_new_orders;
use nc_data::{CampaignStore, CoreGameData, IntelTier, Order};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir should have rust workspace parent")
        .parent()
        .expect("rust workspace should have repo root parent")
        .to_path_buf()
}

fn load_fixture(name: &str) -> CoreGameData {
    let fixture_dir = repo_root().join("fixtures").join(name).join("v1.5");
    CoreGameData::load(&fixture_dir).expect("fixture should load")
}

fn join_player(
    game_data: &mut CoreGameData,
    player: usize,
    empire_name: &str,
    homeworld_name: &str,
) {
    game_data
        .join_player(player, empire_name)
        .expect("join player");
    game_data
        .rename_player_homeworld(player, homeworld_name)
        .expect("rename homeworld");
}

fn hold_all_fleets_in_place(game_data: &mut CoreGameData) {
    for fleet in &mut game_data.fleets.records {
        let coords = fleet.current_location_coords_raw();
        fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
        fleet.set_standing_order_target_coords_raw(coords);
    }
}

fn seed_runtime_snapshot(dir: &Path, game_data: &CoreGameData) {
    let store = CampaignStore::open_default_in_dir(dir).expect("open campaign store");
    store
        .save_runtime_state_structured(game_data, &BTreeSet::new(), &[], &[])
        .expect("seed runtime snapshot");
}

fn run_nc_sysop(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_nc-sysop"))
        .args(args)
        .output()
        .expect("nc-sysop should run");
    assert!(
        output.status.success(),
        "nc-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

#[test]
fn nc_sysop_maint_persists_view_world_reports_and_runtime_intel() {
    let target = unique_temp_dir("nc-sysop-maint-view-world");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(runtime
        .report_block_rows
        .iter()
        .any(|row| row.decoded_text.contains("Viewing mission report")));

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    let target_world = &runtime.game_data.planets.records[13];
    let snapshot = intel
        .iter()
        .find(|snapshot| snapshot.planet_record_index_1_based == 14)
        .expect("viewed planet intel should be stored");
    assert_eq!(snapshot.intel_tier, IntelTier::Partial);
    assert_eq!(
        snapshot.known_name.as_deref(),
        Some(target_world.planet_name().as_str())
    );
    assert_eq!(
        snapshot.known_owner_empire_id,
        Some(target_world.owner_empire_slot_raw())
    );
    assert_eq!(
        snapshot.known_potential_production,
        Some(target_world.potential_production_points_current_known())
    );
    assert_eq!(snapshot.known_armies, None);
    assert_eq!(snapshot.known_ground_batteries, None);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_scout_system_reports_and_runtime_intel() {
    let target = unique_temp_dir("nc-sysop-maint-scout-system");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    game_data.planets.records[13].set_stardock_kind_raw(0, 1);
    game_data.planets.records[13].set_stardock_count_raw(0, 2);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(runtime
        .report_block_rows
        .iter()
        .any(|row| row.decoded_text.contains("Scouting mission report")));

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    let target_world = &runtime.game_data.planets.records[13];
    let snapshot = intel
        .iter()
        .find(|snapshot| snapshot.planet_record_index_1_based == 14)
        .expect("scouted planet intel should be stored");
    assert_eq!(snapshot.intel_tier, IntelTier::Full);
    assert_eq!(
        snapshot.known_owner_empire_id,
        Some(target_world.owner_empire_slot_raw())
    );
    assert_eq!(
        snapshot.known_potential_production,
        Some(target_world.potential_production_points_current_known())
    );
    assert_eq!(snapshot.known_armies, Some(target_world.army_count_raw()));
    assert_eq!(
        snapshot.known_ground_batteries,
        Some(target_world.ground_batteries_raw())
    );
    assert_eq!(snapshot.known_starbase_count, Some(0));
    assert_eq!(
        snapshot.known_docked_summary.as_deref(),
        Some("2 destroyers")
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_non_intel_player_reports() {
    let target = unique_temp_dir("nc-sysop-maint-colonize-report");
    let game_data = load_fixture("ecmaint-fleet-pre");
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(runtime.report_block_rows.iter().any(|row| {
        row.decoded_text.contains("successfully terraformed")
            && row.decoded_text.contains("started a new colony")
    }));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scopes_runtime_reports_and_intel_per_viewer() {
    let target = unique_temp_dir("nc-sysop-maint-viewer-scoping");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    join_player(&mut game_data, 1, "Empire One", "Forge");
    join_player(&mut game_data, 4, "Empire Four", "Bastion");
    hold_all_fleets_in_place(&mut game_data);

    let player_1_coords = game_data.planets.records[13].coords_raw();
    let player_4_coords = game_data.planets.records[6].coords_raw();
    let player_1_speed = game_data.fleets.records[0].max_speed();
    let player_4_speed = game_data.fleets.records[12].max_speed();
    game_data.fleets.records[0].set_standing_order_kind(Order::ViewWorld);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(player_1_coords);
    game_data.fleets.records[0].set_current_speed(player_1_speed);
    game_data.fleets.records[12].set_standing_order_kind(Order::ViewWorld);
    game_data.fleets.records[12].set_standing_order_target_coords_raw(player_4_coords);
    game_data.fleets.records[12].set_current_speed(player_4_speed);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    let player_1_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(1))
        .map(|row| row.decoded_text.as_str())
        .collect::<Vec<_>>();
    let player_4_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(4))
        .map(|row| row.decoded_text.as_str())
        .collect::<Vec<_>>();
    let viewing_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.decoded_text.contains("Viewing mission report"))
        .collect::<Vec<_>>();

    assert!(runtime.report_block_rows.iter().any(
        |row| row.viewer_empire_id == 1 && row.decoded_text.contains("Viewing mission report")
    ));
    assert_eq!(viewing_reports.len(), 2);
    assert!(!player_1_reports
        .iter()
        .any(|text| text.contains("13th Fleet")));
    assert!(runtime.report_block_rows.iter().any(
        |row| row.viewer_empire_id == 4 && row.decoded_text.contains("Viewing mission report")
    ));
    assert!(!player_4_reports.iter().any(|text| text.contains(&format!(
        "System({},{})",
        player_1_coords[0], player_1_coords[1]
    ))));

    let player_1_intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load player 1 intel");
    assert_eq!(
        player_1_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 14)
            .expect("player 1 viewed world")
            .intel_tier,
        IntelTier::Partial
    );
    assert_eq!(
        player_1_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 7)
            .expect("player 4 target should stay unknown for player 1")
            .intel_tier,
        IntelTier::Unknown
    );

    let player_4_intel = store
        .latest_planet_intel_for_viewer(4)
        .expect("load player 4 intel");
    assert_eq!(
        player_4_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 7)
            .expect("player 4 viewed world")
            .intel_tier,
        IntelTier::Partial
    );
    assert_eq!(
        player_4_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 14)
            .expect("player 1 target should stay unknown for player 4")
            .intel_tier,
        IntelTier::Unknown
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_view_world_on_station_generates_report() {
    // Regression test for Bug 1: a ViewWorld fleet whose current position
    // already equals its target (no travel needed) must still emit a viewing
    // mission report and update intel on that turn.
    let target = unique_temp_dir("nc-sysop-maint-view-world-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Place fleet 0 at the coords of planet 13 with ViewWorld order and
    // speed=0 so it has nothing to travel — it is already on station.
    let planet_coords = game_data.planets.records[13].coords_raw();
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_current_location_coords_raw(planet_coords);
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw(planet_coords);
    viewer.set_current_speed(0);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Viewing mission report")),
        "on-station ViewWorld fleet should generate a viewing report"
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    assert!(
        intel
            .iter()
            .any(|s| s.planet_record_index_1_based == 14 && s.intel_tier == IntelTier::Partial),
        "on-station ViewWorld fleet should refresh planet intel"
    );

    // ViewWorld is one-shot: the order must revert to HoldPosition after firing.
    assert_eq!(
        runtime.game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "on-station ViewWorld must reset to HoldPosition after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scout_sector_on_station_persists_report() {
    // A ScoutSector fleet already on station must keep Order::ScoutSector and
    // not crash.  The on-station ScoutSector event deliberately suppresses the
    // repeating status message ("no news is good news"), so no report block row
    // is expected.
    let target = unique_temp_dir("nc-sysop-maint-scout-sector-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let planet_coords = game_data.planets.records[13].coords_raw();
    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw(planet_coords);
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw(planet_coords);
    scout.set_current_speed(0);
    scout.set_scout_count(1);
    reset_motion_state_for_new_orders(scout);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    // On-station ScoutSector suppresses the repeating status message.
    assert!(
        !runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("scout this sector")),
        "on-station ScoutSector must not emit a repeating status report"
    );
    assert_eq!(
        runtime.game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSector,
        "ScoutSector must persist on station after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scout_system_on_station_persists_report_and_intel() {
    // A ScoutSolarSystem fleet already on station must emit a scouting report,
    // update planet intel to Full, and keep Order::ScoutSolarSystem.
    let target = unique_temp_dir("nc-sysop-maint-scout-system-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let planet_coords = game_data.planets.records[13].coords_raw();
    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw(planet_coords);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw(planet_coords);
    scout.set_current_speed(0);
    scout.set_scout_count(1);
    reset_motion_state_for_new_orders(scout);
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Scouting mission report")),
        "on-station ScoutSolarSystem fleet should generate a scouting report"
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    assert!(
        intel
            .iter()
            .any(|s| s.planet_record_index_1_based == 14 && s.intel_tier == IntelTier::Full),
        "on-station ScoutSolarSystem should produce Full intel for the target planet"
    );

    assert_eq!(
        runtime.game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSolarSystem,
        "ScoutSolarSystem must persist on station after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_view_world_reports_when_integer_coords_match_but_exact_not_arrived() {
    // Regression test for Bug 2: a ViewWorld fleet whose integer coordinates
    // round to the target sector on turn 1 but whose sub-grid exact position
    // hasn't quite reached the target must still generate a report on that
    // turn via the on-station observation path.
    //
    // Geometry: fleet at (5,3), target (7,4), speed=3.
    // The fixture fleet[0] has a cruiser with max_speed=3.
    // Single-turn movement = floor(3*8/9) = 2 grid units.
    // Straight-line distance = sqrt((7-5)²+(4-3)²) = sqrt(5) ≈ 2.236 > 2,
    // so the exact position does not reach the target on turn 1.
    // The exact endpoint ≈ (6.789, 3.894) rounds to (7,4) == target, so
    // integer coords match the target without an exact arrival.
    let target_dir = unique_temp_dir("nc-sysop-maint-view-world-rounded");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // The fixture fleet[0] has a cruiser (and originally an ETAC). We zero
    // the ETAC so only the cruiser remains; has_any_force() stays true.
    // max_speed raw byte stays at 3 (set by the fixture), so speed=3 passes
    // the SpeedExceedsMaximum check and the fleet moves this turn.
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_current_location_coords_raw([5, 3]);
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([7, 4]);
    viewer.set_current_speed(3);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    // Reset in-transit motion state from the fixture so the stepper starts
    // fresh from (5,3) rather than continuing a prior exact-position path.
    reset_motion_state_for_new_orders(viewer);
    seed_runtime_snapshot(&target_dir, &game_data);

    let stdout = run_nc_sysop(&["maint", target_dir.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target_dir).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    // The fleet's integer coords round to (7,4) == target on turn 1.
    // The on-station path must fire a viewing report even though the
    // sub-grid exact position has not fully reached the target.
    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Viewing mission report")),
        "ViewWorld fleet rounded to target sector must report on turn 1 via on-station path"
    );

    let _ = fs::remove_dir_all(&target_dir);
}
