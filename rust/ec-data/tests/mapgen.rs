use ec_data::{build_seeded_new_game, generate_map};

#[test]
fn generated_map_is_seed_reproducible() {
    let first = generate_map(4, 1515);
    let second = generate_map(4, 1515);
    assert_eq!(first, second);
}

#[test]
fn generated_map_produces_balanced_world_count_and_unique_coords() {
    let generated = generate_map(4, 424242);
    assert_eq!(generated.homeworld_coords.len(), 4);
    assert_eq!(generated.neutral_worlds.len(), 16);

    let mut coords = generated.homeworld_coords.clone();
    coords.extend(generated.neutral_worlds.iter().map(|world| world.coords));
    coords.sort_unstable();
    coords.dedup();
    assert_eq!(coords.len(), 20);

    for coords in generated.homeworld_coords {
        assert!(coords[0] < generated.map_size);
        assert!(coords[1] < generated.map_size);
    }
}

#[test]
fn seeded_new_game_populates_documented_planet_count_for_player_count() {
    let data = build_seeded_new_game(3, 3000, 99).expect("seeded game should build");
    let populated = data
        .planets
        .records
        .iter()
        .filter(|planet| planet.coords_raw() != [0, 0])
        .count();
    assert_eq!(populated, 15);

    for idx in 0..3 {
        assert_eq!(
            data.planets.records[idx].owner_empire_slot_raw(),
            (idx + 1) as u8
        );
    }
}
