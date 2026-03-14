use ec_data::{CanonicalFourPlayerSetup, GameStateBuilder};

#[test]
fn canonical_four_player_start_matches_documented_opening_shape() {
    let data =
        GameStateBuilder::build_canonical_four_player_start(CanonicalFourPlayerSetup::default())
            .expect("canonical four-player start should build");

    assert_eq!(data.conquest.game_year(), 3000);
    assert_eq!(data.conquest.player_count(), 4);
    assert_eq!(data.planets.records.len(), 20);
    assert_eq!(data.fleets.records.len(), 16);

    let errors = data.ecmaint_preflight_errors();
    assert!(errors.is_empty(), "Preflight errors: {errors:?}");

    let homeworlds = [[16, 13], [30, 6], [2, 25], [26, 26]];
    for (idx, coords) in homeworlds.into_iter().enumerate() {
        let planet = &data.planets.records[idx];
        assert_eq!(planet.coords_raw(), coords);
        assert_eq!(planet.owner_empire_slot_raw(), (idx + 1) as u8);
        assert_eq!(planet.ownership_status_raw(), 2);
        assert_eq!(planet.army_count_raw(), 10);
        assert_eq!(planet.ground_batteries_raw(), 4);
        assert_eq!(planet.economy_marker_raw(), 50);
        assert_eq!(planet.present_production_points(), Some(100));
    }

    for player_idx in 0..4 {
        let base = player_idx * 4;
        let coords = homeworlds[player_idx];
        let fleet_a = &data.fleets.records[base];
        let fleet_b = &data.fleets.records[base + 1];
        let fleet_c = &data.fleets.records[base + 2];
        let fleet_d = &data.fleets.records[base + 3];

        for fleet in [fleet_a, fleet_b, fleet_c, fleet_d] {
            assert_eq!(fleet.current_location_coords_raw(), coords);
            assert_eq!(fleet.standing_order_target_coords_raw(), coords);
            assert_eq!(fleet.owner_empire_raw(), (player_idx + 1) as u8);
        }

        assert_eq!(fleet_a.cruiser_count(), 1);
        assert_eq!(fleet_a.etac_count(), 1);
        assert_eq!(fleet_b.cruiser_count(), 1);
        assert_eq!(fleet_b.etac_count(), 1);
        assert_eq!(fleet_c.destroyer_count(), 1);
        assert_eq!(fleet_d.destroyer_count(), 1);
    }
}
