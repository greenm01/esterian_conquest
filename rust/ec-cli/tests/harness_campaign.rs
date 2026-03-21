mod common;

use std::fs;

use ec_data::{CampaignStore, DiplomaticRelation};

use common::{cleanup_dir, repo_root, run_ec_cli, unique_temp_dir};

fn write_file(path: &std::path::Path, text: &str) {
    fs::write(path, text).unwrap();
}

fn scenario_text() -> &'static str {
    r#"scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="Bot Campaign"

house record=1 handle="P1" empire="Aurora"
house record=2 handle="P2" empire="Helios"
house record=3 handle="P3" empire="Vesper"
house record=4 handle="P4" empire="Nadir"
"#
}

#[test]
fn harness_init_campaign_creates_ready_bundles_and_status_files() {
    let scenario_root = unique_temp_dir("ec-cli-harness-campaign-scenario");
    let campaign_dir = unique_temp_dir("ec-cli-harness-campaign-out");
    let game_id = format!(
        "ec-cli-campaign-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workspace_root = repo_root().join(".tmp/llm-turns").join(&game_id);
    let scenario_path = scenario_root.join("scenario.kdl");
    write_file(&scenario_path, scenario_text());

    let stdout = run_ec_cli(&[
        "harness",
        "init-campaign",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_dir.to_str().unwrap(),
        "--game-id",
        &game_id,
    ]);

    assert!(stdout.contains("Initialized bot campaign."));
    assert!(workspace_root.join("campaign/manifest.kdl").exists());
    assert!(
        workspace_root
            .join("player-1/status-turn-0001.kdl")
            .exists()
    );
    assert!(
        workspace_root
            .join("player-1/bundle-turn-0001/README.md")
            .exists()
    );

    let status = fs::read_to_string(workspace_root.join("player-1/status-turn-0001.kdl")).unwrap();
    assert!(status.contains("state=\"ready\""));
    assert!(status.contains("doctrine=\""));
    let bundle =
        fs::read_to_string(workspace_root.join("player-1/bundle-turn-0001/README.md")).unwrap();
    assert!(bundle.contains("doctrine"));
    assert!(bundle.contains("turn_file"));

    cleanup_dir(&scenario_root);
    cleanup_dir(&campaign_dir);
    cleanup_dir(&workspace_root);
}

#[test]
fn harness_scan_and_apply_turn_batch_advance_year_and_surface_player_mail() {
    let scenario_root = unique_temp_dir("ec-cli-harness-campaign-scan");
    let campaign_dir = unique_temp_dir("ec-cli-harness-campaign-scan-out");
    let game_id = format!(
        "ec-cli-campaign-mail-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workspace_root = repo_root().join(".tmp/llm-turns").join(&game_id);
    let scenario_path = scenario_root.join("scenario.kdl");
    write_file(&scenario_path, scenario_text());

    run_ec_cli(&[
        "harness",
        "init-campaign",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_dir.to_str().unwrap(),
        "--game-id",
        &game_id,
    ]);

    write_file(
        &workspace_root.join("player-1/turn-0001.kdl"),
        r#"turn player=1 year=3000

tax rate=41

diplomacy to=3 relation="enemy"

message to=2 subject="Alliance?" body="Hold the center while I scout east."
"#,
    );
    write_file(
        &workspace_root.join("player-2/turn-0001.kdl"),
        "turn player=2 year=3000\n",
    );
    write_file(
        &workspace_root.join("player-3/turn-0001.kdl"),
        "turn player=3 year=3000\n",
    );
    write_file(
        &workspace_root.join("player-4/turn-0001.kdl"),
        "turn player=4 year=3000\n",
    );

    let scan_stdout = run_ec_cli(&[
        "harness",
        "scan-turn",
        "--dir",
        campaign_dir.to_str().unwrap(),
    ]);
    assert!(scan_stdout.contains("validated=1,2,3,4"));

    let apply_stdout = run_ec_cli(&[
        "harness",
        "apply-turn-batch",
        "--dir",
        campaign_dir.to_str().unwrap(),
    ]);
    assert!(apply_stdout.contains("Applied campaign turn batch."));
    assert!(apply_stdout.contains("next_turn=2"));

    let store = CampaignStore::open_default_in_dir(&campaign_dir).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.conquest.game_year(), 3001);
    assert_eq!(
        state.game_data.stored_diplomatic_relation(1, 3).unwrap(),
        DiplomaticRelation::Enemy
    );

    let turn1_status =
        fs::read_to_string(workspace_root.join("player-1/status-turn-0001.kdl")).unwrap();
    assert!(turn1_status.contains("state=\"applied\""));

    let turn2_status =
        fs::read_to_string(workspace_root.join("player-2/status-turn-0002.kdl")).unwrap();
    assert!(turn2_status.contains("state=\"ready\""));

    let player2_bundle =
        fs::read_to_string(workspace_root.join("player-2/bundle-turn-0002/README.md")).unwrap();
    assert!(player2_bundle.contains("Alliance?"));
    assert!(player2_bundle.contains("Hold the center while I scout east."));

    cleanup_dir(&scenario_root);
    cleanup_dir(&campaign_dir);
    cleanup_dir(&workspace_root);
}

#[test]
fn harness_campaign_doctrine_assignments_vary_by_game_id() {
    let scenario_root = unique_temp_dir("ec-cli-harness-campaign-doctrine-scenario");
    let scenario_path = scenario_root.join("scenario.kdl");
    write_file(&scenario_path, scenario_text());

    let game_a = "ec-cli-doctrine-alpha";
    let game_b = "ec-cli-doctrine-beta";
    let campaign_a = unique_temp_dir("ec-cli-harness-campaign-doctrine-a");
    let campaign_b = unique_temp_dir("ec-cli-harness-campaign-doctrine-b");
    let workspace_a = repo_root().join(".tmp/llm-turns").join(game_a);
    let workspace_b = repo_root().join(".tmp/llm-turns").join(game_b);

    run_ec_cli(&[
        "harness",
        "init-campaign",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_a.to_str().unwrap(),
        "--game-id",
        game_a,
    ]);
    run_ec_cli(&[
        "harness",
        "init-campaign",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_b.to_str().unwrap(),
        "--game-id",
        game_b,
    ]);

    let status_a = fs::read_to_string(workspace_a.join("player-1/status-turn-0001.kdl")).unwrap();
    let status_b = fs::read_to_string(workspace_b.join("player-1/status-turn-0001.kdl")).unwrap();
    assert_ne!(status_a, status_b);

    cleanup_dir(&scenario_root);
    cleanup_dir(&campaign_a);
    cleanup_dir(&campaign_b);
    cleanup_dir(&workspace_a);
    cleanup_dir(&workspace_b);
}

#[test]
fn harness_claim_turn_marks_player_as_claimed_until_submission() {
    let scenario_root = unique_temp_dir("ec-cli-harness-campaign-claim");
    let campaign_dir = unique_temp_dir("ec-cli-harness-campaign-claim-out");
    let game_id = format!(
        "ec-cli-campaign-claim-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workspace_root = repo_root().join(".tmp/llm-turns").join(&game_id);
    let scenario_path = scenario_root.join("scenario.kdl");
    write_file(&scenario_path, scenario_text());

    run_ec_cli(&[
        "harness",
        "init-campaign",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_dir.to_str().unwrap(),
        "--game-id",
        &game_id,
    ]);

    let claim_stdout = run_ec_cli(&[
        "harness",
        "claim-turn",
        "--dir",
        campaign_dir.to_str().unwrap(),
        "--player",
        "2",
    ]);
    assert!(claim_stdout.contains("Claimed campaign turn."));

    let status = fs::read_to_string(workspace_root.join("player-2/status-turn-0001.kdl")).unwrap();
    assert!(status.contains("state=\"claimed\""));

    let scan_stdout = run_ec_cli(&[
        "harness",
        "scan-turn",
        "--dir",
        campaign_dir.to_str().unwrap(),
    ]);
    assert!(scan_stdout.contains("claimed=2"));
    assert!(scan_stdout.contains("blocking=1,2,3,4"));

    cleanup_dir(&scenario_root);
    cleanup_dir(&campaign_dir);
    cleanup_dir(&workspace_root);
}

#[test]
fn harness_play_until_initializes_then_blocks_when_turns_are_missing() {
    let scenario_root = unique_temp_dir("ec-cli-harness-play-until");
    let campaign_dir = unique_temp_dir("ec-cli-harness-play-until-out");
    let game_id = format!(
        "ec-cli-campaign-play-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workspace_root = repo_root().join(".tmp/llm-turns").join(&game_id);
    let scenario_path = scenario_root.join("scenario.kdl");
    write_file(&scenario_path, scenario_text());

    let stdout = run_ec_cli(&[
        "harness",
        "play-until",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        campaign_dir.to_str().unwrap(),
        "--game-id",
        &game_id,
        "--turn",
        "2",
    ]);

    assert!(stdout.contains("Campaign blocked before turn 1."));
    assert!(
        workspace_root
            .join("player-4/status-turn-0001.kdl")
            .exists()
    );

    cleanup_dir(&scenario_root);
    cleanup_dir(&campaign_dir);
    cleanup_dir(&workspace_root);
}
