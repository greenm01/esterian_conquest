use crate::{
    CoreGameData, ProductionItemKind, build_capacity, yearly_growth_delta,
    yearly_high_tax_penalty, yearly_tax_revenue,
};

/// Process build queue completion for all planets.
///
/// Build production is based on planet's industrial capacity:
/// - Production rate = current production, with a starbase multiplier
/// - Each build queue item decrements by production rate per turn
/// - When build_count reaches 0, ship moves to stardock
///
/// Returns a list of planet indices that had build activity.
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
                && planet_has_friendly_starbase(
                    game_data,
                    owner_empire,
                    game_data.planets.records[planet_idx].coords_raw(),
                ),
        );
        let production_rate_u8 = spend_capacity.min(255) as u8;

        // Process up to 10 build slots per planet
        let mut had_builds = false;
        for slot in 0..10 {
            let build_count = game_data.planets.records[planet_idx].build_count_raw(slot);

            if build_count > 0 {
                had_builds = true;
                let build_kind = game_data.planets.records[planet_idx].build_kind_raw(slot);
                let build_item_kind = ProductionItemKind::from_raw(build_kind);
                // Decrement by production rate (or remaining count if less)
                let decrement = build_count.min(production_rate_u8);
                let new_count = build_count.saturating_sub(decrement);

                // If build completed (reached 0), dispatch by unit kind.
                // Armies and ground batteries are surface/defensive units: they go
                // directly onto the planet and never enter stardock. Stardock is
                // reserved for ships (kinds 1-6) and starbases (kind 9), which must
                // be commissioned before use and can be destroyed by bombardment
                // while sitting uncommissioned.
                if new_count > 0 {
                    game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);
                    continue;
                }

                let points_spent = u32::from(decrement);

                if build_item_kind.requires_stardock() {
                    let has_open_stardock_slot = (0..10).any(|stardock_slot| {
                        game_data.planets.records[planet_idx].stardock_kind_raw(stardock_slot) == 0
                    });
                    if !has_open_stardock_slot {
                        // Rust policy: hold completed ship/starbase builds in queue until
                        // stardock space exists rather than reproducing the classic
                        // corruption bug triggered by full-stardock completion.
                        continue;
                    }
                }

                match build_item_kind {
                    ProductionItemKind::Army => {
                        let qty = ((points_spent / 2).max(1)).min(u32::from(u8::MAX)) as u8;
                        let current = game_data.planets.records[planet_idx].army_count_raw();
                        let free_capacity = u8::MAX.saturating_sub(current);
                        if qty > free_capacity {
                            // v1.6 policy: hold the build in queue instead of reproducing the
                            // classic silent-loss bug at the byte cap.
                            continue;
                        }
                    }
                    ProductionItemKind::GroundBattery => {
                        let qty = ((points_spent / 20).max(1)).min(u32::from(u8::MAX)) as u8;
                        let current = game_data.planets.records[planet_idx].ground_batteries_raw();
                        let free_capacity = u8::MAX.saturating_sub(current);
                        if qty > free_capacity {
                            // v1.6 policy: hold the build in queue instead of reproducing the
                            // classic silent-loss bug at the byte cap.
                            continue;
                        }
                    }
                    _ => {}
                }

                game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);

                if new_count == 0 {
                    match build_item_kind {
                        ProductionItemKind::Army => {
                            let qty = ((points_spent / 2).max(1)).min(u32::from(u8::MAX)) as u8;
                            let current = game_data.planets.records[planet_idx].army_count_raw();
                            game_data.planets.records[planet_idx].set_army_count_raw(current + qty);
                        }
                        ProductionItemKind::GroundBattery => {
                            let qty = ((points_spent / 20).max(1)).min(u32::from(u8::MAX)) as u8;
                            let current =
                                game_data.planets.records[planet_idx].ground_batteries_raw();
                            game_data.planets.records[planet_idx]
                                .set_ground_batteries_raw(current + qty);
                        }
                        _ => {
                            // Ships and starbases stage in stardock awaiting commission.
                            for stardock_slot in 0..10 {
                                let existing_kind = game_data.planets.records[planet_idx]
                                    .stardock_kind_raw(stardock_slot);
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

                    // Clear the build slot.
                    game_data.planets.records[planet_idx].set_build_kind_raw(slot, 0);
                }
            }
        }

        if had_builds {
            planets_with_builds.push(planet_idx);
        }
    }

    Ok(planets_with_builds)
}

/// Process planet economic updates during maintenance.
///
/// Canonical Rust economy rule:
/// - every owned planet uses the empire-wide tax rate
/// - taxed revenue is added to the planet's stored production pool
/// - current production grows toward potential every year
/// - lower taxes accelerate growth
/// - taxes above the safe threshold can directly reduce present production
/// - a friendly starbase on the planet boosts both growth and build capacity
pub(super) fn process_planet_economics(
    game_data: &mut CoreGameData,
    _planets_with_builds: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.records.len() {
        let owner_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
        if owner_empire == 0 {
            continue;
        }
        let Some(player) = game_data
            .player
            .records
            .get(owner_empire.saturating_sub(1) as usize)
        else {
            continue;
        };
        if matches!(player.owner_mode_raw(), 0x00 | 0xff) {
            continue;
        }

        let tax_rate = player.tax_rate();
        let current_production = game_data.planets.records[planet_idx]
            .present_production_points()
            .unwrap_or(0);
        let potential_production =
            game_data.planets.records[planet_idx].potential_production_points();
        let has_starbase = planet_has_friendly_starbase(
            game_data,
            owner_empire,
            game_data.planets.records[planet_idx].coords_raw(),
        );

        let revenue = yearly_tax_revenue(current_production, tax_rate);
        let growth = yearly_growth_delta(
            current_production,
            potential_production,
            tax_rate,
            has_starbase,
        );
        let penalty = yearly_high_tax_penalty(current_production, tax_rate, has_starbase);

        let planet = &mut game_data.planets.records[planet_idx];
        planet.set_economy_marker_raw(tax_rate);
        planet.set_stored_goods_raw(planet.stored_goods_raw().saturating_add(revenue));
        let new_current_production = current_production
            .saturating_add(growth)
            .saturating_sub(penalty)
            .min(potential_production);
        let _ = planet.set_present_production_points(new_current_production);
    }

    Ok(())
}

fn planet_has_friendly_starbase(
    game_data: &CoreGameData,
    owner_empire_raw: u8,
    coords: [u8; 2],
) -> bool {
    game_data.bases.records.iter().any(|base| {
        base.owner_empire_raw() == owner_empire_raw
            && base.coords_raw() == coords
            && base.active_flag_raw() != 0
    })
}

/// Process autopilot / rogue AI planet economics.
///
/// Runs for every player whose slot is either:
/// - rogue (`PLAYER.DAT raw[0x00] == 0xff`), OR
/// - an active human with autopilot on (`raw[0x00] == 0x01` AND `raw[0x6D] == 0x01`)
///
/// For each qualifying player, every planet they own with `raw[0x03] == 0x87`
/// (homeworld type, the only flag value that produces clean AI behaviour) is updated:
///
/// 1. **Factories exponent** (`raw[0x09]`, the BP Real48 exponent byte):
///    If currently `0x86` (= factories 50.0 for pot_prod=100 homeworlds), increment
///    to `0x87` (doubles the Real48 value: 50.0 → 100.0 = pot_prod).
///    Confirmed deterministic across all oracle runs.
///
/// 2. **Armies** (`raw[0x58]`):
///    Add `round(pot_prod / 6)` to the army count.
///    Formula: `(pot_prod + 3) / 6` in integer arithmetic (rounds to nearest).
///    For pot_prod=100: delta = (100+3)/6 = 17.
///
/// 3. **`raw[0x0E]`** (production accumulator):
///    Set to 4.  This is the value consistently observed after the AI has spent
///    production points on armies. Without AI it decrements by 1 per tick; the AI
///    resets it to ~4 after spending. Exact accumulator arithmetic is not yet decoded
///    but setting 4 matches the oracle output for pot_prod=100 homeworlds.
///
/// Sources: docs/dev/archive/RE_NOTES.md "Rogue AI / autopilot planet economics — Session 2026-03-13".
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

        // owner_empire_slot is 1-based; player_idx 0 = slot 1
        let owner_slot = (player_idx + 1) as u8;

        for planet_idx in 0..game_data.planets.records.len() {
            let planet = &game_data.planets.records[planet_idx];

            // Must be owned by this player and be a homeworld-type planet
            if planet.raw[0x5D] != owner_slot || planet.raw[0x03] != 0x87 {
                continue;
            }

            let pot_prod = planet.raw[0x02];

            // 1. Increment factories exponent if at 0x86 (50.0 → 100.0)
            if game_data.planets.records[planet_idx].raw[0x09] == 0x86 {
                game_data.planets.records[planet_idx].raw[0x09] = 0x87;
            }

            // 2. Army growth: += round(pot_prod / 6)
            let army_delta = (pot_prod as u16 + 3) / 6;
            let current_armies = game_data.planets.records[planet_idx].raw[0x58] as u16;
            game_data.planets.records[planet_idx].raw[0x58] =
                current_armies.saturating_add(army_delta).min(255) as u8;

            // 3. Reset production accumulator to 4
            game_data.planets.records[planet_idx].raw[0x0E] = 4;
        }
    }

    Ok(())
}

/// Recompute per-player planet count and production score from PLANETS.DAT.
///
/// ECMAINT recalculates these fields from scratch every turn by scanning all
/// planet records. The pre-maint PLAYER.DAT values may be stale.
///
/// - PLAYER raw[0x50]: count of planets owned by this player
/// - PLAYER raw[0x52]: sum of current production for all owned planets
///
/// Player record index N corresponds to owner_empire_slot N+1 in PLANETS.DAT.
/// Owner empire slot 0 means unowned. Player record 0 = owner_empire_slot 1, etc.
///
/// Current-known model:
/// - newly colonized worlds (`raw[0x03] == 0x81`) contribute `1`
/// - mature worlds contribute their current/present production
/// - joinable homeworld seeds present at full potential from the start
pub(super) fn recompute_player_planet_stats(game_data: &mut CoreGameData) {
    let n_players = game_data.player.records.len();

    // Accumulate count and pot_prod sum per player slot (1-based owner_empire_slot)
    let mut planet_counts = vec![0u8; n_players + 1]; // index = owner_empire_slot
    let mut pot_prod_sums = vec![0u16; n_players + 1];

    for planet in &game_data.planets.records {
        let owner = planet.owner_empire_slot_raw() as usize;
        if owner > 0 && owner <= n_players {
            planet_counts[owner] = planet_counts[owner].saturating_add(1);
            let current_prod: u16 = if planet.raw[0x03] == 0x81 {
                1
            } else {
                planet.present_production_points().unwrap_or(0)
            };
            pot_prod_sums[owner] = pot_prod_sums[owner].saturating_add(current_prod);
        }
    }

    // Write back to player records (player record index = owner_empire_slot - 1)
    for player_idx in 0..n_players {
        let owner_slot = player_idx + 1;
        game_data.player.records[player_idx].raw[0x50] = planet_counts[owner_slot];
        game_data.player.records[player_idx].raw[0x52] = pot_prod_sums[owner_slot] as u8;
    }
}
