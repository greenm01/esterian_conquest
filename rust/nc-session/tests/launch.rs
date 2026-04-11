use nc_data::{GameStateBuilder, SeatReservation};
use nc_session::launch::{
    LaunchBindingRequest, LaunchPlayerBinding, LaunchPlayerBindingSource,
    resolve_launch_player_binding, session_lease_ttl_seconds,
};
use nc_session::onboarding::{
    FirstTimeOnboardingMode, HostedFirstTimeStatus, first_time_onboarding_mode,
    hosted_first_time_status,
};

#[test]
fn resolve_launch_binding_prefers_reserved_alias() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");

    let binding = resolve_launch_player_binding(LaunchBindingRequest {
        explicit_player_record_index_1_based: None,
        dropfile_alias: Some("sysop"),
        use_door_terminal: true,
        reservations: &[SeatReservation {
            player_record_index_1_based: 2,
            alias: "SYSOP".to_string(),
        }],
        game_data: &game_data,
    })
    .expect("resolve binding");

    assert_eq!(
        binding,
        LaunchPlayerBinding::Bound {
            player_record_index_1_based: 2,
            source: LaunchPlayerBindingSource::ReservedAlias,
        }
    );
}

#[test]
fn resolve_launch_binding_returns_unbound_for_new_door_caller() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");

    let binding = resolve_launch_player_binding(LaunchBindingRequest {
        explicit_player_record_index_1_based: None,
        dropfile_alias: Some("newcaller"),
        use_door_terminal: true,
        reservations: &[],
        game_data: &game_data,
    })
    .expect("resolve binding");

    assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);
}

#[test]
fn shared_session_ttl_prefers_explicit_timeout_then_idle_then_fallback() {
    assert_eq!(session_lease_ttl_seconds(Some(45), Some(600)), 45);
    assert_eq!(session_lease_ttl_seconds(None, Some(600)), 600);
    assert_eq!(session_lease_ttl_seconds(None, None), 120);
}

#[test]
fn onboarding_helpers_choose_expected_modes() {
    assert_eq!(
        first_time_onboarding_mode(true, false),
        FirstTimeOnboardingMode::HostedInvite
    );
    assert_eq!(
        first_time_onboarding_mode(false, true),
        FirstTimeOnboardingMode::BbsReserved
    );
    assert_eq!(
        first_time_onboarding_mode(false, false),
        FirstTimeOnboardingMode::Generic
    );
}

#[test]
fn hosted_first_time_status_detects_pending_joinable_seat() {
    let joinable = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");
    assert_eq!(
        hosted_first_time_status(&joinable),
        HostedFirstTimeStatus::NeedsEmpireName
    );

    let mut joined = joinable.clone();
    joined.join_player(1, "Empire One").expect("join player 1");
    joined.join_player(2, "Empire Two").expect("join player 2");
    joined
        .join_player(3, "Empire Three")
        .expect("join player 3");
    joined.join_player(4, "Empire Four").expect("join player 4");
    assert_eq!(
        hosted_first_time_status(&joined),
        HostedFirstTimeStatus::NoPendingSeat
    );
}

#[test]
fn hosted_first_time_status_ignores_used_civil_disorder_seats() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");
    for player_idx in 1..=4 {
        game_data.player.records[player_idx - 1].set_last_run_year_raw(3005);
        game_data.player.records[player_idx - 1].set_planet_count_raw(0);
    }
    for planet in &mut game_data.planets.records {
        planet.set_owner_empire_slot_raw(0);
        planet.set_ownership_status_raw(0);
    }

    assert_eq!(
        hosted_first_time_status(&game_data),
        HostedFirstTimeStatus::NoPendingSeat
    );
}
