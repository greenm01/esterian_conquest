use crate::CoreGameData;

pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let n_players = game_data.player.records.len();

    for player_idx in 0..n_players {
        let mode = game_data.player.records[player_idx].raw[0x00];
        let autopilot = game_data.player.records[player_idx].raw[0x6D];

        let ai_active = mode == 0xff || (mode == 0x01 && autopilot == 0x01);
        if !ai_active {
            continue;
        }

        let owner_slot = (player_idx + 1) as u8;

        for planet_idx in 0..game_data.planets.records.len() {
            let planet = &game_data.planets.records[planet_idx];
            if planet.raw[0x5D] != owner_slot || planet.raw[0x03] != 0x87 {
                continue;
            }

            let pot_prod = planet.raw[0x02];

            if game_data.planets.records[planet_idx].raw[0x09] == 0x86 {
                game_data.planets.records[planet_idx].raw[0x09] = 0x87;
            }

            let army_delta = (pot_prod as u16 + 3) / 6;
            let current_armies = game_data.planets.records[planet_idx].raw[0x58] as u16;
            game_data.planets.records[planet_idx].raw[0x58] =
                current_armies.saturating_add(army_delta).min(255) as u8;

            game_data.planets.records[planet_idx].raw[0x0E] = 4;
        }
    }

    Ok(())
}
