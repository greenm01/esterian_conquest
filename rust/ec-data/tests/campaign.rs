use ec_data::{CampaignState, GameStateBuilder, run_maintenance_turn};

fn baseline_game() -> ec_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build")
}

#[test]
fn owned_planets_mean_stable_campaign_state() {
    let game_data = baseline_game();
    assert_eq!(game_data.empire_campaign_state(1), Some(CampaignState::Stable));
}

#[test]
fn no_planets_but_etac_means_marginal_existence() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::MarginalExistence)
    );
}

#[test]
fn no_planets_but_loaded_transports_mean_marginal_existence() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_cruiser_count(0);
            fleet.set_destroyer_count(0);
            fleet.set_battleship_count(0);
            fleet.set_scout_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    let fleet = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1)
        .expect("player 1 fleet");
    fleet.set_troop_transport_count(2);
    fleet.set_army_count(2);

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::MarginalExistence)
    );
}

#[test]
fn no_planets_and_no_recovery_path_means_defection_risk() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::DefectionRisk)
    );
}

#[test]
fn no_planets_and_no_fleet_presence_means_defeated() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
            fleet.set_destroyer_count(0);
            fleet.set_cruiser_count(0);
            fleet.set_battleship_count(0);
            fleet.set_scout_count(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::Defeated)
    );
}

#[test]
fn rogue_and_civil_disorder_states_are_preserved() {
    let mut game_data = baseline_game();
    game_data.player.records[0].set_owner_empire_raw(0xff);
    game_data.player.records[1].set_owner_empire_raw(0x00);

    assert_eq!(game_data.empire_campaign_state(1), Some(CampaignState::Rogue));
    assert_eq!(
        game_data.empire_campaign_state(2),
        Some(CampaignState::CivilDisorder)
    );
}

#[test]
fn maintenance_moves_empire_without_recovery_path_into_civil_disorder() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let player = &game_data.player.records[0];
    assert_eq!(player.owner_mode_raw(), 0x00);
    assert_eq!(player.legacy_status_name_summary(), "In Civil Disorder");
    assert_eq!(events.civil_disorder_events.len(), 1);
    assert_eq!(events.civil_disorder_events[0].reporting_empire_raw, 1);
    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::CivilDisorder)
    );
}
