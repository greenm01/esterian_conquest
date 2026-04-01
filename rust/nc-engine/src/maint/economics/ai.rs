use crate::CoreGameData;

pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let n_players = game_data.player.records.len();

    for player_idx in 0..n_players {
        let mode = game_data.player.records[player_idx].owner_mode_raw();
        let autopilot = game_data.player.records[player_idx].autopilot_flag();

        let ai_active = mode == 0xff || (mode == 0x01 && autopilot == 0x01);
        if !ai_active {
            continue;
        }

        let owner_slot = (player_idx + 1) as u8;

        for planet_idx in 0..game_data.planets.records.len() {
            let planet = &game_data.planets.records[planet_idx];
            if planet.owner_empire_slot_raw() != owner_slot
                || planet.potential_production_high_byte_raw() != 0x87
            {
                continue;
            }

            let pot_prod = planet.potential_production_points() as u8;

            if game_data.planets.records[planet_idx].factories_word_raw() == 0x8600 {
                game_data.planets.records[planet_idx].set_factories_word_raw(0x8700);
            }

            let army_delta = (pot_prod as u16 + 3) / 6;
            let current_armies = game_data.planets.records[planet_idx].army_count_raw() as u16;
            game_data.planets.records[planet_idx]
                .set_army_count_raw(current_armies.saturating_add(army_delta).min(255) as u8);

            game_data.planets.records[planet_idx].set_economy_marker_raw(4);
        }
    }

    Ok(())
}
