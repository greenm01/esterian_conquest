mod common;

use std::fs;

use ec_data::CampaignStore;

use common::{cleanup_dir, run_ec_cli, unique_temp_dir};

fn write_file(path: &std::path::Path, text: &str) {
    fs::write(path, text).unwrap();
}

#[test]
fn harness_run_scenario_creates_runtime_snapshot() {
    let scenario_root = unique_temp_dir("ec-cli-harness-scenario");
    let target_dir = unique_temp_dir("ec-cli-harness-scenario-out");
    let turn_path = scenario_root.join("player1-turn.kdl");
    let scenario_path = scenario_root.join("scenario.kdl");

    write_file(
        &turn_path,
        "turn player=1 year=3000\n\
         tax rate=40\n",
    );
    write_file(
        &scenario_path,
        r#"scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="CLI Scenario"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime"
planet record=1 {
  name "Aurora Prime"
  stardock slot=1 kind="destroyer" count=2
  commission slot=1
}
turn-file path="player1-turn.kdl"
results-block player=1 "Ready to launch"
"#,
    );

    let stdout = run_ec_cli(&[
        "harness",
        "run-scenario",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        target_dir.to_str().unwrap(),
    ]);

    assert!(stdout.contains("Saved runtime scenario"));
    let store = CampaignStore::open_default_in_dir(&target_dir).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.player.records[0].tax_rate(), 40);
    assert!(!state.results_bytes.is_empty());

    cleanup_dir(&scenario_root);
    cleanup_dir(&target_dir);
}

#[test]
fn harness_run_sweep_prints_case_summary() {
    let root = unique_temp_dir("ec-cli-harness-sweep");
    let combat_path = root.join("combat.kdl");
    let sweep_path = root.join("sweep.kdl");

    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="CLI Combat"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}

fleet record=5 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}
"#,
    );
    write_file(
        &sweep_path,
        r#"combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=2
fleet-ship fleet=1 kind="bb" 1 2
"#,
    );

    let stdout = run_ec_cli(&[
        "harness",
        "run-sweep",
        "--file",
        sweep_path.to_str().unwrap(),
    ]);

    assert!(stdout.contains("Executed combat sweep."));
    assert!(stdout.contains("executed_cases=2"));

    cleanup_dir(&root);
}
