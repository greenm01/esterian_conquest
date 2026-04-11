use nc_data::{
    GameStateBuilder, PlayerAccessMode, PlayerActivityState, PlayerLifecycleState,
    PublicEmpireStatus, TerminalOutcome, WinnerState, default_player_lifecycle_states,
    empire_has_recovery_path, player_access_mode, player_public_status,
};

fn baseline_game() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn default_activity_states(player_count: usize) -> Vec<PlayerActivityState> {
    (1..=player_count)
        .map(|player_record_index_1_based| PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        })
        .collect()
}

#[test]
fn public_status_reports_mia_when_inactivity_autopilot_is_active() {
    let mut game_data = baseline_game();
    game_data.player.records[0].set_autopilot_flag(1);
    let mut activity = default_activity_states(game_data.player.records.len());
    activity[0].inactivity_autopilot_pending_clear = true;

    let status = player_public_status(
        &game_data,
        1,
        &activity,
        &default_player_lifecycle_states(game_data.conquest.player_count()),
    );

    assert_eq!(status, PublicEmpireStatus::Mia);
}

#[test]
fn public_status_reports_defeated_for_terminal_loss_states() {
    let game_data = baseline_game();
    let activity = default_activity_states(game_data.player.records.len());
    let mut lifecycle = default_player_lifecycle_states(game_data.conquest.player_count());
    lifecycle[0].terminal_outcome = TerminalOutcome::Defeated;
    lifecycle[1].terminal_outcome = TerminalOutcome::LostGame;

    assert_eq!(
        player_public_status(&game_data, 1, &activity, &lifecycle),
        PublicEmpireStatus::Defeated
    );
    assert_eq!(
        player_public_status(&game_data, 2, &activity, &lifecycle),
        PublicEmpireStatus::Defeated
    );
}

#[test]
fn recovery_path_requires_loaded_transports_or_etac_with_unowned_world() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
            fleet.set_etac_count(0);
        }
    }

    assert!(!empire_has_recovery_path(&game_data, 1));

    let fleet_idx = game_data
        .fleets
        .records
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 1)
        .expect("player 1 fleet");
    game_data.fleets.records[fleet_idx].set_etac_count(1);
    assert!(empire_has_recovery_path(&game_data, 1));

    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 0 {
            planet.set_owner_empire_slot_raw(2);
            planet.set_ownership_status_raw(2);
        }
    }
    assert!(!empire_has_recovery_path(&game_data, 1));

    let fleet = game_data
        .fleets
        .records
        .get_mut(fleet_idx)
        .expect("player 1 fleet");
    fleet.set_etac_count(0);
    fleet.set_troop_transport_count(1);
    fleet.set_army_count(1);
    assert!(empire_has_recovery_path(&game_data, 1));
}

#[test]
fn access_mode_distinguishes_review_survey_and_lockout() {
    let player_count = 4;
    let mut lifecycle = default_player_lifecycle_states(player_count);

    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::Normal
    );

    lifecycle[0].terminal_outcome = TerminalOutcome::Defeated;
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::ReviewOnly
    );

    lifecycle[0].terminal_review_consumed = true;
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::LockedOut
    );

    lifecycle[0] = PlayerLifecycleState {
        player_record_index_1_based: 1,
        recovery_window_turns_remaining: 0,
        terminal_outcome: TerminalOutcome::Winner,
        terminal_review_consumed: false,
    };
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::SurveyOnly
    );

    assert_eq!(
        player_access_mode(
            2,
            &lifecycle,
            WinnerState {
                winner_empire_raw: Some(1),
                winner_declared_year: Some(3005),
            }
        ),
        PlayerAccessMode::LockedOut
    );
}
