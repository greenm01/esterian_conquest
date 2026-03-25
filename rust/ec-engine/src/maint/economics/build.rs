use crate::{CoreGameData, ProductionItemKind, build_capacity};

pub(super) fn process_build_completion(
    game_data: &mut CoreGameData,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let planet_count = game_data.planets.records.len();
    let mut planets_with_builds = Vec::new();

    for planet_idx in 0..planet_count {
        let owner_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
        let current_production = game_data.planets.records[planet_idx]
            .present_production_points()
            .unwrap_or(0);
        let spend_capacity = build_capacity(
            current_production,
            owner_empire != 0
                && super::planet_has_friendly_starbase(
                    game_data,
                    owner_empire,
                    game_data.planets.records[planet_idx].coords_raw(),
                ),
        );
        let production_rate_u8 = spend_capacity.min(255) as u8;

        let mut had_builds = false;
        for slot in 0..10 {
            let build_count = game_data.planets.records[planet_idx].build_count_raw(slot);

            if build_count == 0 {
                continue;
            }
            had_builds = true;

            let build_kind = game_data.planets.records[planet_idx].build_kind_raw(slot);
            let build_item_kind = ProductionItemKind::from_raw(build_kind);
            let decrement = build_count.min(production_rate_u8);
            let new_count = build_count.saturating_sub(decrement);

            if new_count > 0 {
                game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);
                continue;
            }

            let points_spent = u32::from(decrement);

            if build_item_kind.requires_stardock() {
                let has_open_stardock_slot = (0..crate::STARDOCK_SLOT_COUNT).any(|stardock_slot| {
                    game_data.planets.records[planet_idx].stardock_kind_raw(stardock_slot) == 0
                });
                if !has_open_stardock_slot {
                    continue;
                }
            }

            match build_item_kind {
                ProductionItemKind::Army => {
                    let qty = ((points_spent / 2).max(1)).min(u32::from(u8::MAX)) as u8;
                    let current = game_data.planets.records[planet_idx].army_count_raw();
                    let free_capacity = u8::MAX.saturating_sub(current);
                    if qty > free_capacity {
                        continue;
                    }
                }
                ProductionItemKind::GroundBattery => {
                    let qty = ((points_spent / 20).max(1)).min(u32::from(u8::MAX)) as u8;
                    let current = game_data.planets.records[planet_idx].ground_batteries_raw();
                    let free_capacity = u8::MAX.saturating_sub(current);
                    if qty > free_capacity {
                        continue;
                    }
                }
                _ => {}
            }

            game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);

            match build_item_kind {
                ProductionItemKind::Army => {
                    let qty = ((points_spent / 2).max(1)).min(u32::from(u8::MAX)) as u8;
                    let current = game_data.planets.records[planet_idx].army_count_raw();
                    game_data.planets.records[planet_idx].set_army_count_raw(current + qty);
                }
                ProductionItemKind::GroundBattery => {
                    let qty = ((points_spent / 20).max(1)).min(u32::from(u8::MAX)) as u8;
                    let current = game_data.planets.records[planet_idx].ground_batteries_raw();
                    game_data.planets.records[planet_idx].set_ground_batteries_raw(current + qty);
                }
                _ => {
                    for stardock_slot in 0..crate::STARDOCK_SLOT_COUNT {
                        let existing_kind =
                            game_data.planets.records[planet_idx].stardock_kind_raw(stardock_slot);
                        if existing_kind == 0 {
                            game_data.planets.records[planet_idx]
                                .set_stardock_kind_raw(stardock_slot, build_kind);
                            game_data.planets.records[planet_idx]
                                .set_stardock_count_raw(stardock_slot, 3);
                            break;
                        }
                    }
                }
            }

            game_data.planets.records[planet_idx].set_build_kind_raw(slot, 0);
        }

        if had_builds {
            planets_with_builds.push(planet_idx);
        }
    }

    Ok(planets_with_builds)
}
