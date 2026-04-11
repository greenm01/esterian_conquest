use nc_data::{GameStateBuilder, SeatReservation};
use nc_session::launch::{
    LaunchBindingRequest, LaunchPlayerBinding, LaunchPlayerBindingSource, resolve_launch_player_binding,
};
use nc_session::onboarding::{FirstTimeOnboardingMode, first_time_onboarding_mode};

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
fn onboarding_helpers_choose_expected_modes() {
    assert_eq!(
        first_time_onboarding_mode(true),
        FirstTimeOnboardingMode::BbsReserved
    );
    assert_eq!(
        first_time_onboarding_mode(false),
        FirstTimeOnboardingMode::Generic
    );
}
