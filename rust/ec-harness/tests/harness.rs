use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_harness::{
    CombatScenarioSpec, CombatSweepSpec, ScenarioSpec, build_scenario, run_combat_scenario,
    run_combat_sweep,
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&target).unwrap();
    target
}

fn write_file(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

#[test]
fn scenario_build_applies_turn_files_reports_and_commissions() {
    let dir = unique_temp_dir("ec-harness-scenario");
    let turn_path = dir.join("player1-turn.kdl");
    write_file(
        &turn_path,
        "turn player=1 year=3000\n\
         tax rate=35\n",
    );

    let scenario_path = dir.join("scenario.kdl");
    write_file(
        &scenario_path,
        r#"scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="Playtest"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime" tax=25
house record=2 handle="RIVAL" empire="Helios"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

planet record=1 {
  name "Aurora Prime"
  production potential=140 present=120 stored=80 economy_marker=25
  defenses armies=14 batteries=6
  stardock slot=1 kind="destroyer" count=3
  commission slot=1
}

fleet record=1 {
  coords x=10 y=10
  ships bb=0 ca=1 dd=2 sc=0 tt=1 armies=1 etac=0
  roe value=8
  order kind="hold" speed=0 x=10 y=10
}

turn-file path="player1-turn.kdl"
queued-mail from=1 to=2 year=3000 subject="Border" body="Hold line."
results-block player=1 "Command summary\nTurn 3 ready."
messages-block player=1 "Incoming traffic\nStand by."
"#,
    );

    let spec = ScenarioSpec::load_kdl(&scenario_path).unwrap();
    let built = build_scenario(&spec).unwrap();

    assert_eq!(built.game_data.player.records[0].tax_rate(), 35);
    assert_eq!(built.game_data.player.records[0].assigned_player_handle_summary(), "SYSOP");
    assert_eq!(
        built.game_data.player.records[0].controlled_empire_name_summary(),
        "Aurora"
    );
    assert_eq!(built.game_data.fleets.records.len(), 17);
    assert_eq!(built.queued_mail.len(), 1);
    assert!(!built.results_bytes.is_empty());
    assert!(!built.messages_bytes.is_empty());
    assert!(
        built.game_data.player.records[0].has_classic_results_review_state()
            || built.game_data.player.records[0].classic_reports_pending_flag_raw() != 0
    );
}

#[test]
fn combat_scenario_runs_and_reports_battle_metrics() {
    let dir = unique_temp_dir("ec-harness-combat");
    let combat_path = dir.join("combat.kdl");
    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Skirmish"

house record=1 empire="Aurora"
house record=2 empire="Helios"

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

    let spec = CombatScenarioSpec::load_kdl(&combat_path).unwrap();
    let run = run_combat_scenario(&spec).unwrap();

    assert!(run.report.fleet_battle_events >= 1);
    assert_eq!(run.report.maintenance_turns, 1);
    assert!(run.report.elapsed_millis <= u128::MAX);
}

#[test]
fn combat_sweep_caps_cases_and_reports_summary() {
    let dir = unique_temp_dir("ec-harness-sweep");
    let combat_path = dir.join("combat.kdl");
    let sweep_path = dir.join("sweep.kdl");
    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Sweep Base"

house record=1 empire="Aurora"
house record=2 empire="Helios"

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
        r#"combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=3
fleet-ship fleet=1 kind="bb" 1 2
fleet-roe fleet=5 6 10
"#,
    );

    let spec = CombatSweepSpec::load_kdl(&sweep_path).unwrap();
    let report = run_combat_sweep(&spec).unwrap();

    assert_eq!(report.total_possible_cases, 4);
    assert_eq!(report.executed_cases, 3);
    assert_eq!(report.cases.len(), 3);
}
