use ec_data::{DatabaseDat, GameStateBuilder, build_player_starmap_projection};

#[test]
fn player_starmap_projection_shows_full_geometry_but_only_known_details() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets.records[0].set_coords_raw([5, 2]);
    game_data.planets.records[1].set_coords_raw([10, 15]);

    let planet_count = game_data.planets.records.len();
    let mut database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let record = database.record_mut(0, 0, planet_count);
    record.set_planet_name("Home");
    record.raw[0x15] = 1;
    record.raw[0x1c] = 100;
    record.raw[0x23] = 10;
    record.raw[0x25] = 4;

    let projection = build_player_starmap_projection(&game_data, &database, 1);
    assert_eq!(projection.worlds.len(), planet_count);
    assert!(projection.worlds.iter().any(|world| world.coords == [5, 2]));
    assert!(projection.worlds.iter().any(|world| world.coords == [10, 15]));

    let known = projection
        .worlds
        .iter()
        .find(|world| world.coords == [5, 2])
        .expect("known world should exist");
    assert_eq!(known.known_name.as_deref(), Some("Home"));

    let unknown = projection
        .worlds
        .iter()
        .find(|world| world.coords == [10, 15])
        .expect("unknown world should exist");
    assert_eq!(unknown.known_name, None);
    assert_eq!(unknown.known_owner_empire_id, None);
}

#[test]
fn ascii_map_export_uses_printable_paged_grid() {
    for (width, height, expect_formfeed) in [(18, 18, false), (27, 27, true), (36, 36, true), (45, 45, true)] {
        let projection = ec_data::PlayerStarmapProjection {
            map_width: width,
            map_height: height,
            year: 3000,
            viewer_empire_id: 1,
            worlds: vec![
                ec_data::PlayerStarmapWorld {
                    planet_record_index_1_based: 1,
                    coords: [1, 1],
                    known_name: None,
                    known_owner_empire_id: None,
                    known_owner_empire_name: None,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                },
                ec_data::PlayerStarmapWorld {
                    planet_record_index_1_based: 2,
                    coords: [width, height],
                    known_name: None,
                    known_owner_empire_id: None,
                    known_owner_empire_name: None,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                },
            ],
        };

        let rendered = projection.render_ascii_map();
        assert_eq!(rendered.contains('\u{0c}'), expect_formfeed);
        assert!(rendered.contains('*'));
        assert!(rendered.contains("   1"));
        assert!(rendered.contains(&format!("{height:>4}")));
    }
}
