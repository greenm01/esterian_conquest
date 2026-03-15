mod common;

use common::{cleanup_dir, run_ec_cli, unique_temp_dir};

#[test]
fn player_name_updates_handle_and_empire() {
    let target = unique_temp_dir("ec-cli-player-name");
    run_ec_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3010",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);

    let stdout = run_ec_cli(&[
        "player-name",
        target.to_str().unwrap(),
        "1",
        "tester01",
        "Auroran_Combine",
    ]);
    assert!(stdout.contains("Player 1 renamed"));

    let data = ec_data::CoreGameData::load(&target).unwrap();
    assert_eq!(data.player.records[0].assigned_player_handle_summary(), "tester01");
    assert_eq!(
        data.player.records[0].controlled_empire_name_summary(),
        "Auroran_Combine"
    );

    cleanup_dir(&target);
}

#[test]
fn fleet_ships_and_detach_create_varied_extra_fleet() {
    let target = unique_temp_dir("ec-cli-fleet-setup");
    run_ec_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3010",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);

    run_ec_cli(&[
        "fleet-ships",
        target.to_str().unwrap(),
        "1",
        "12",
        "8",
        "10",
        "14",
        "9",
        "4",
        "2",
    ]);

    let stdout = run_ec_cli(&[
        "fleet-detach",
        target.to_str().unwrap(),
        "1",
        "1",
        "1",
        "2",
        "3",
        "1",
        "1",
        "2",
        "1",
        "3",
        "6",
    ]);
    assert!(stdout.contains("Detached fleet 1 -> new fleet 17"));

    let data = ec_data::CoreGameData::load(&target).unwrap();
    assert_eq!(data.fleets.records.len(), 17);
    let donor = &data.fleets.records[0];
    let new_fleet = &data.fleets.records[16];
    assert_eq!(donor.battleship_count(), 7);
    assert_eq!(donor.cruiser_count(), 8);
    assert_eq!(donor.destroyer_count(), 11);
    assert_eq!(donor.troop_transport_count(), 7);
    assert_eq!(donor.army_count(), 3);
    assert_eq!(donor.scout_count(), 10);
    assert_eq!(donor.etac_count(), 1);
    assert_eq!(new_fleet.battleship_count(), 1);
    assert_eq!(new_fleet.cruiser_count(), 2);
    assert_eq!(new_fleet.destroyer_count(), 3);
    assert_eq!(new_fleet.troop_transport_count(), 2);
    assert_eq!(new_fleet.army_count(), 1);
    assert_eq!(new_fleet.scout_count(), 2);
    assert_eq!(new_fleet.etac_count(), 1);

    cleanup_dir(&target);
}
