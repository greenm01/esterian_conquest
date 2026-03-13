mod common;

use ec_data::CoreGameData;

use common::{run_ec_cli, unique_temp_dir};

#[test]
fn init_canonical_four_player_start_writes_expected_directory_shape() {
    let target = unique_temp_dir("ec-cli-canonical-4p-start");
    let target_str = target.to_string_lossy().into_owned();

    let stdout = run_ec_cli(&["init-canonical-four-player-start", &target_str]);
    assert!(stdout.contains("Initialized canonical four-player start"));

    let data = CoreGameData::load(&target).expect("load generated state");
    assert_eq!(data.conquest.game_year(), 3000);
    assert_eq!(data.conquest.player_count(), 4);
    assert_eq!(data.fleets.records.len(), 16);
    assert!(data.ecmaint_preflight_errors().is_empty());

    assert!(target.join("DATABASE.DAT").exists());
    assert!(target.join("MESSAGES.DAT").exists());
    assert!(target.join("RESULTS.DAT").exists());
}
