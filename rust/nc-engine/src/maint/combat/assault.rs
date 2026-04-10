use std::collections::{BTreeMap, HashSet};

use crate::{
    AssaultReportEvent, BombardEvent, CoreGameData, Mission, MissionEvent, MissionOutcome, Order,
    PlanetIntelEvent, PlanetIntelSource, PlanetOwnershipChangeEvent, ProductionItemKind,
    STARDOCK_SLOT_COUNT, ShipLosses,
};

use super::exchange::{
    COMBAT_KIND_BLITZ_COVER, COMBAT_KIND_BLITZ_GROUND, COMBAT_KIND_BOMBARD, COMBAT_KIND_GROUND,
    COMBAT_KIND_INVASION_SOFTEN, COMBAT_KIND_INVASION_SUPPRESSION, ExchangeResolution,
    GROUND_AS_BATTERY, apply_hits_to_fleet, resolve_ground_exchange, resolve_space_exchange,
    scalar_hits_with_critical,
};
use super::reporting::{
    mission_kind_for_fleet, preferred_reporting_fleet_number, push_planet_intel,
};
use super::retreat::set_fleet_to_hold_current_position;
use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, fleet_state_from_records, planet_idx_at_coords,
    ship_counts_from_state, ship_losses_from_states, starbase_count_at,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MissionClass {
    Bombard,
    Invade,
    Blitz,
    Other,
}

#[derive(Debug, Default)]
pub(crate) struct AssaultEvents {
    pub bombard_events: Vec<BombardEvent>,
    pub assault_report_events: Vec<AssaultReportEvent>,
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    pub ownership_change_events: Vec<PlanetOwnershipChangeEvent>,
    pub mission_events: Vec<MissionEvent>,
}

fn clear_arrival_and_hold(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        set_fleet_to_hold_current_position(&mut game_data.fleets.records[idx]);
    }
}

/// Keep a bombarding fleet on station with its BombardWorld order so it
/// re-executes next turn. Speed is zeroed but transit_ready_flag stays 0x80.
fn hold_bombardment_station(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        fleet.set_current_speed(0);
        fleet.set_transit_ready_flag_raw(0x80);
        nc_data::fleet_motion_state::clear_exact_position(fleet);
    }
}

fn fleet_still_ready_for_assault(game_data: &CoreGameData, fleet_idx: usize, order: Order) -> bool {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return false;
    };
    if fleet.standing_order_kind() != order {
        return false;
    }
    let target_coords = fleet.standing_order_target_coords_raw();
    game_data
        .validate_fleet_order_payload(fleet_idx + 1, order.to_raw(), target_coords, None, None)
        .is_ok()
}

fn reduce_stardock(planet: &mut nc_data::PlanetRecord, mut hits: u32) -> u32 {
    for slot in 0..STARDOCK_SLOT_COUNT {
        if hits == 0 {
            break;
        }
        let count = planet.stardock_count_raw(slot) as u32;
        if count == 0 {
            continue;
        }
        let destroyed = hits.min(count) as u16;
        planet.set_stardock_count_raw(
            slot,
            planet.stardock_count_raw(slot).saturating_sub(destroyed),
        );
        if planet.stardock_count_raw(slot) == 0 {
            planet.set_stardock_kind_raw(slot, 0);
        }
        hits -= destroyed as u32;
    }
    hits
}

fn stardock_summary(planet: &nc_data::PlanetRecord) -> nc_data::EmpireUnitSummary {
    let mut summary = nc_data::EmpireUnitSummary::default();
    for slot in 0..STARDOCK_SLOT_COUNT {
        let count = u32::from(planet.stardock_count_raw(slot));
        if count == 0 {
            continue;
        }
        match ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)) {
            ProductionItemKind::Destroyer => summary.destroyers += count,
            ProductionItemKind::Cruiser => summary.cruisers += count,
            ProductionItemKind::Battleship => summary.battleships += count,
            ProductionItemKind::Scout => summary.scouts += count,
            ProductionItemKind::Transport => summary.transports += count,
            ProductionItemKind::Etac => summary.etacs += count,
            ProductionItemKind::Starbase => summary.starbases += count,
            ProductionItemKind::Army => summary.armies += count,
            ProductionItemKind::GroundBattery => summary.ground_batteries += count,
            ProductionItemKind::Unknown(_) => {}
        }
    }
    summary
}

fn stardock_summary_diff(
    before: nc_data::EmpireUnitSummary,
    after: nc_data::EmpireUnitSummary,
) -> nc_data::EmpireUnitSummary {
    nc_data::EmpireUnitSummary {
        destroyers: before.destroyers.saturating_sub(after.destroyers),
        cruisers: before.cruisers.saturating_sub(after.cruisers),
        battleships: before.battleships.saturating_sub(after.battleships),
        scouts: before.scouts.saturating_sub(after.scouts),
        transports: before.transports.saturating_sub(after.transports),
        etacs: before.etacs.saturating_sub(after.etacs),
        starbases: before.starbases.saturating_sub(after.starbases),
        armies: before.armies.saturating_sub(after.armies),
        ground_batteries: before
            .ground_batteries
            .saturating_sub(after.ground_batteries),
    }
}

/// Suppression damage: hits only go through stardock and batteries.
/// Armies, stored goods, and factories are shielded.
fn apply_planet_suppression_damage(planet: &mut nc_data::PlanetRecord, mut hits: u32) {
    hits = reduce_stardock(planet, hits);
    let battery_loss = hits.min(planet.ground_batteries_raw() as u32) as u8;
    planet.set_ground_batteries_raw(planet.ground_batteries_raw().saturating_sub(battery_loss));
}

/// Soften damage during invasion: only targets armies.
/// Factories and stored goods are preserved to keep the planet valuable for capture.
fn apply_planet_soften_damage(planet: &mut nc_data::PlanetRecord, hits: u32) {
    let army_loss = hits.min(planet.army_count_raw() as u32) as u8;
    planet.set_army_count_raw(planet.army_count_raw().saturating_sub(army_loss));
}

/// Full bombardment cascade: stardock -> batteries -> armies -> stored goods -> factories.
fn apply_planet_bombardment_damage(planet: &mut nc_data::PlanetRecord, mut hits: u32) {
    hits = reduce_stardock(planet, hits);

    let battery_loss = hits.min(planet.ground_batteries_raw() as u32) as u8;
    planet.set_ground_batteries_raw(planet.ground_batteries_raw().saturating_sub(battery_loss));
    hits -= battery_loss as u32;

    let army_loss = hits.min(planet.army_count_raw() as u32) as u8;
    planet.set_army_count_raw(planet.army_count_raw().saturating_sub(army_loss));
    hits -= army_loss as u32;

    if hits > 0 {
        let goods_loss = hits.saturating_mul(100);
        planet.set_stored_goods_raw(planet.stored_goods_raw().saturating_sub(goods_loss));
    }
    if hits > 0 {
        let loss = hits.min(planet.factories_word_raw() as u32) as u16;
        planet.set_factories_word_raw(planet.factories_word_raw().saturating_sub(loss));
    }
}

fn bombard_attack_as(state: &FleetCombatState) -> u32 {
    state.counts[IDX_DD] / 2 + state.counts[IDX_CA] * 3 + state.counts[IDX_BB] * 9 * 3 / 2
}

fn blitz_cover_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    acting_empire: u8,
    target_empire: u8,
    state: &FleetCombatState,
    battery_as: u32,
) -> ExchangeResolution {
    let combat_ships = state.counts[IDX_DD] + state.counts[IDX_CA] + state.counts[IDX_BB];
    if combat_ships == 0 {
        return ExchangeResolution {
            roll: 0,
            critical: false,
            hits: 0,
        };
    }

    resolve_ground_exchange(
        campaign_seed,
        game_year,
        coords,
        COMBAT_KIND_BLITZ_COVER,
        1,
        acting_empire,
        target_empire,
        bombard_attack_as(state).max(1),
        battery_as.max(1),
        0,
    )
}

fn apply_blitz_landing_fire(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    surviving_batteries: u8,
) -> (u32, ShipLosses) {
    let mut transport_hits_remaining = surviving_batteries as u32;
    let mut landed_army_losses = 0u32;
    let mut ship_losses = ShipLosses::default();

    for &idx in fleet_indices {
        if transport_hits_remaining == 0 {
            break;
        }
        let fleet = &mut game_data.fleets.records[idx];
        let transport_loss =
            transport_hits_remaining.min(fleet.troop_transport_count() as u32) as u16;
        if transport_loss == 0 {
            continue;
        }
        fleet.set_troop_transport_count(
            fleet.troop_transport_count().saturating_sub(transport_loss),
        );
        fleet.set_army_count(fleet.army_count().saturating_sub(transport_loss));
        ship_losses.transports += transport_loss as u32;
        landed_army_losses += transport_loss as u32;
        transport_hits_remaining -= transport_loss as u32;
    }

    if transport_hits_remaining > 0 {
        for &idx in fleet_indices {
            if transport_hits_remaining == 0 {
                break;
            }
            let fleet = &mut game_data.fleets.records[idx];
            let destroyer_loss =
                transport_hits_remaining.min(fleet.destroyer_count() as u32) as u16;
            if destroyer_loss > 0 {
                fleet.set_destroyer_count(fleet.destroyer_count().saturating_sub(destroyer_loss));
                ship_losses.destroyers += destroyer_loss as u32;
                transport_hits_remaining -= destroyer_loss as u32;
            }

            let cruiser_loss = transport_hits_remaining.min(fleet.cruiser_count() as u32) as u16;
            if cruiser_loss > 0 {
                fleet.set_cruiser_count(fleet.cruiser_count().saturating_sub(cruiser_loss));
                ship_losses.cruisers += cruiser_loss as u32;
                transport_hits_remaining -= cruiser_loss as u32;
            }

            let battleship_loss =
                transport_hits_remaining.min(fleet.battleship_count() as u32) as u16;
            if battleship_loss > 0 {
                fleet
                    .set_battleship_count(fleet.battleship_count().saturating_sub(battleship_loss));
                ship_losses.battleships += battleship_loss as u32;
                transport_hits_remaining -= battleship_loss as u32;
            }
        }
    }

    if transport_hits_remaining > 0 {
        landed_army_losses += transport_hits_remaining;
    }

    (landed_army_losses, ship_losses)
}

fn select_orbital_supremacy_empire(
    game_data: &CoreGameData,
    planet_idx: usize,
    entrants: &BTreeMap<u8, Vec<usize>>,
) -> Option<u8> {
    let coords = game_data.planets.records[planet_idx].coords_raw();
    let owner = game_data.planets.records[planet_idx].owner_empire_slot_raw();

    let mut contenders: Vec<(u8, u32, bool)> = entrants
        .iter()
        .map(|(&empire, fleet_indices)| {
            let starbases = if owner == empire {
                starbase_count_at(game_data, coords, empire)
            } else {
                0
            };
            let state = fleet_state_from_records(game_data, fleet_indices, starbases);
            let retreating = fleet_indices.iter().all(|&idx| {
                game_data.fleets.records[idx].standing_order_kind() == Order::SeekHome
                    && game_data.fleets.records[idx].current_speed() > 0
            });
            (empire, state.total_combat_as(), retreating)
        })
        .filter(|(_, as_total, retreating)| *as_total > 0 && !*retreating)
        .collect();

    if contenders.is_empty() {
        return None;
    }
    contenders.sort_by_key(|(empire, as_total, _)| {
        (
            std::cmp::Reverse(*as_total),
            if *empire == owner { 0u8 } else { 1u8 },
            *empire,
        )
    });
    if contenders.len() > 1 && contenders[0].1 == contenders[1].1 {
        if contenders
            .iter()
            .any(|(emp, as_total, _)| *emp == owner && *as_total == contenders[0].1)
        {
            return Some(owner);
        }
        return None;
    }
    Some(contenders[0].0)
}

fn mission_priority(class: MissionClass) -> u8 {
    match class {
        MissionClass::Blitz => 0,
        MissionClass::Invade => 1,
        MissionClass::Bombard => 2,
        MissionClass::Other => 3,
    }
}

pub(crate) fn process_planetary_assaults(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    bombard_ready: &[usize],
    invade_ready: &[usize],
    blitz_ready: &[usize],
) -> Result<AssaultEvents, Box<dyn std::error::Error>> {
    let battle_year = game_data.conquest.game_year();
    let mut by_planet: BTreeMap<usize, BTreeMap<u8, Vec<usize>>> = BTreeMap::new();
    for &idx in bombard_ready {
        if !fleet_still_ready_for_assault(game_data, idx, Order::BombardWorld) {
            continue;
        }
        let coords = game_data.fleets.records[idx].standing_order_target_coords_raw();
        if let Some(planet_idx) = planet_idx_at_coords(game_data, coords) {
            by_planet
                .entry(planet_idx)
                .or_default()
                .entry(game_data.fleets.records[idx].owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }
    for &idx in invade_ready {
        if !fleet_still_ready_for_assault(game_data, idx, Order::InvadeWorld) {
            continue;
        }
        let coords = game_data.fleets.records[idx].standing_order_target_coords_raw();
        if let Some(planet_idx) = planet_idx_at_coords(game_data, coords) {
            by_planet
                .entry(planet_idx)
                .or_default()
                .entry(game_data.fleets.records[idx].owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }
    for &idx in blitz_ready {
        if !fleet_still_ready_for_assault(game_data, idx, Order::BlitzWorld) {
            continue;
        }
        let coords = game_data.fleets.records[idx].standing_order_target_coords_raw();
        if let Some(planet_idx) = planet_idx_at_coords(game_data, coords) {
            by_planet
                .entry(planet_idx)
                .or_default()
                .entry(game_data.fleets.records[idx].owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }

    let bombard_set: HashSet<usize> = bombard_ready.iter().copied().collect();
    let invade_set: HashSet<usize> = invade_ready.iter().copied().collect();
    let blitz_set: HashSet<usize> = blitz_ready.iter().copied().collect();

    let mut events = AssaultEvents::default();

    for (planet_idx, entrants) in by_planet {
        let Some(winner_empire) = select_orbital_supremacy_empire(game_data, planet_idx, &entrants)
        else {
            for (empire, fleets) in &entrants {
                for &fleet_idx in fleets {
                    if let Some(kind) =
                        mission_kind_for_fleet(fleet_idx, &bombard_set, &invade_set, &blitz_set)
                    {
                        events.mission_events.push(MissionEvent {
                            fleet_idx,
                            owner_empire_raw: *empire,
                            kind,
                            outcome: MissionOutcome::Aborted,
                            planet_idx: Some(planet_idx),
                            location_coords: Some(
                                game_data.planets.records[planet_idx].coords_raw(),
                            ),
                            target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                            stardate_week: None,
                        });
                    }
                }
            }
            continue;
        };

        let winner_fleets = entrants.get(&winner_empire).cloned().unwrap_or_default();
        let winner_class = winner_fleets
            .iter()
            .map(|idx| {
                let class = if blitz_set.contains(idx) {
                    MissionClass::Blitz
                } else if invade_set.contains(idx) {
                    MissionClass::Invade
                } else if bombard_set.contains(idx) {
                    MissionClass::Bombard
                } else {
                    MissionClass::Other
                };
                (class, *idx)
            })
            .min_by_key(|(class, idx)| (mission_priority(*class), *idx))
            .map(|(class, _)| class)
            .unwrap_or(MissionClass::Other);

        match winner_class {
            MissionClass::Bombard => {
                let coords = game_data.planets.records[planet_idx].coords_raw();
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                let pre_armies = game_data.planets.records[planet_idx].army_count_raw();
                let pre_batteries = game_data.planets.records[planet_idx].ground_batteries_raw();
                let pre_stored_goods = game_data.planets.records[planet_idx].stored_goods_raw();
                let pre_factories = game_data.planets.records[planet_idx].factories_word_raw();
                let pre_stardock_items: u32 = (0..STARDOCK_SLOT_COUNT)
                    .map(|s| game_data.planets.records[planet_idx].stardock_count_raw(s) as u32)
                    .sum();
                let pre_docked_summary = stardock_summary(&game_data.planets.records[planet_idx]);

                let before = fleet_state_from_records(game_data, &winner_fleets, 0);
                let mut fleet_state = before.clone();
                let mut breakthrough = false;

                // Three-round bombardment: rounds 1-2 suppression, round 3 breakthrough if batteries cleared.
                for round in 1..=3u32 {
                    let attack_as = bombard_attack_as(&fleet_state);
                    if attack_as == 0 {
                        break; // fleet has no bombardment capability left
                    }
                    let batteries_at_round_start =
                        game_data.planets.records[planet_idx].ground_batteries_raw();
                    let is_breakthrough = round == 3 && batteries_at_round_start == 0;
                    if is_breakthrough {
                        breakthrough = true;
                    }

                    let defense_as = batteries_at_round_start as u32 * GROUND_AS_BATTERY;
                    let attacker_exchange = resolve_space_exchange(
                        campaign_seed,
                        battle_year,
                        coords,
                        COMBAT_KIND_BOMBARD,
                        round,
                        winner_empire,
                        defender_empire,
                        attack_as,
                        defense_as.max(1),
                        fleet_state.is_mixed(),
                        false,
                    );

                    // Batteries fire back each round they exist.
                    if defense_as > 0 {
                        let defender_exchange = resolve_space_exchange(
                            campaign_seed,
                            battle_year,
                            coords,
                            COMBAT_KIND_BOMBARD,
                            round,
                            defender_empire,
                            winner_empire,
                            defense_as,
                            attack_as.max(1),
                            false,
                            false,
                        );
                        let mut next_state = fleet_state.clone();
                        apply_hits_to_fleet(
                            &mut next_state,
                            0,
                            defender_exchange.hits,
                            u32::from(defender_exchange.critical),
                        );
                        super::fleet_battle::distribute_fleet_losses(
                            game_data,
                            &winner_fleets,
                            &fleet_state,
                            &next_state,
                        );
                        fleet_state = fleet_state_from_records(game_data, &winner_fleets, 0);
                    }

                    // Apply planet damage: suppression or breakthrough.
                    let hits = scalar_hits_with_critical(attacker_exchange);
                    if is_breakthrough {
                        apply_planet_bombardment_damage(
                            &mut game_data.planets.records[planet_idx],
                            hits,
                        );
                    } else {
                        apply_planet_suppression_damage(
                            &mut game_data.planets.records[planet_idx],
                            hits,
                        );
                    }
                }

                hold_bombardment_station(game_data, &winner_fleets);
                let post_planet = &game_data.planets.records[planet_idx];
                let post_docked_summary = stardock_summary(post_planet);
                events.bombard_events.push(BombardEvent {
                    planet_idx,
                    attacker_empire_raw: winner_empire,
                    attacker_fleet_number: preferred_reporting_fleet_number(
                        game_data,
                        &winner_fleets,
                    ),
                    defender_empire_raw: post_planet.owner_empire_slot_raw(),
                    attacker_initial: ship_counts_from_state(&before),
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_losses: ship_losses_from_states(&before, &fleet_state),
                    defender_battery_losses: pre_batteries
                        .saturating_sub(post_planet.ground_batteries_raw()),
                    defender_army_losses: pre_armies.saturating_sub(post_planet.army_count_raw()),
                    breakthrough,
                    docked_losses: stardock_summary_diff(pre_docked_summary, post_docked_summary),
                    stardock_items_destroyed: pre_stardock_items.saturating_sub(
                        (0..STARDOCK_SLOT_COUNT)
                            .map(|s| post_planet.stardock_count_raw(s) as u32)
                            .sum::<u32>(),
                    ),
                    stored_goods_destroyed: pre_stored_goods
                        .saturating_sub(post_planet.stored_goods_raw()),
                    factories_destroyed: pre_factories
                        .saturating_sub(post_planet.factories_word_raw()),
                    stardate_week: None,
                });
                for &fleet_idx in &winner_fleets {
                    if bombard_set.contains(&fleet_idx) {
                        events.mission_events.push(MissionEvent {
                            fleet_idx,
                            owner_empire_raw: winner_empire,
                            kind: Mission::BombardWorld,
                            outcome: MissionOutcome::Succeeded,
                            planet_idx: Some(planet_idx),
                            location_coords: Some(
                                game_data.planets.records[planet_idx].coords_raw(),
                            ),
                            target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                            stardate_week: None,
                        });
                    }
                }
            }
            MissionClass::Invade => {
                let state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let bombard_as = bombard_attack_as(&state);
                let initial_attacking_armies: u32 = winner_fleets
                    .iter()
                    .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                    .sum();
                let previous_owner = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                let planet = &game_data.planets.records[planet_idx];
                let coords = planet.coords_raw();
                let pre_batteries = planet.ground_batteries_raw();
                let pre_armies = planet.army_count_raw();
                let battery_as = planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY;
                let suppression_exchange = resolve_space_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_INVASION_SUPPRESSION,
                    1,
                    winner_empire,
                    previous_owner,
                    bombard_as,
                    battery_as.max(1),
                    state.is_mixed(),
                    false,
                );
                let return_exchange = resolve_space_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_INVASION_SUPPRESSION,
                    1,
                    previous_owner,
                    winner_empire,
                    battery_as,
                    bombard_as.max(1),
                    false,
                    false,
                );

                let before = state.clone();
                let mut after = state.clone();
                apply_hits_to_fleet(
                    &mut after,
                    0,
                    return_exchange.hits,
                    u32::from(return_exchange.critical),
                );
                super::fleet_battle::distribute_fleet_losses(
                    game_data,
                    &winner_fleets,
                    &before,
                    &after,
                );

                {
                    let planet = &mut game_data.planets.records[planet_idx];
                    let battery_loss = scalar_hits_with_critical(suppression_exchange)
                        .min(planet.ground_batteries_raw() as u32)
                        as u8;
                    planet.set_ground_batteries_raw(
                        planet.ground_batteries_raw().saturating_sub(battery_loss),
                    );
                }

                let batteries_cleared =
                    game_data.planets.records[planet_idx].ground_batteries_raw() == 0;
                if batteries_cleared {
                    let soft_exchange = resolve_ground_exchange(
                        campaign_seed,
                        battle_year,
                        coords,
                        COMBAT_KIND_INVASION_SOFTEN,
                        1,
                        winner_empire,
                        previous_owner,
                        bombard_attack_as(&after),
                        game_data.planets.records[planet_idx].army_count_raw() as u32,
                        0,
                    );
                    // Soften targets armies only — factories and stored goods are
                    // preserved so the captured planet retains its production value.
                    apply_planet_soften_damage(
                        &mut game_data.planets.records[planet_idx],
                        scalar_hits_with_critical(soft_exchange),
                    );

                    let attacking_armies: u32 = winner_fleets
                        .iter()
                        .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                        .sum();
                    let defender_armies =
                        game_data.planets.records[planet_idx].army_count_raw() as u32;
                    let attacker_ground = resolve_ground_exchange(
                        campaign_seed,
                        battle_year,
                        coords,
                        COMBAT_KIND_GROUND,
                        1,
                        winner_empire,
                        previous_owner,
                        attacking_armies,
                        defender_armies.max(1),
                        0,
                    );
                    let defender_ground = resolve_ground_exchange(
                        campaign_seed,
                        battle_year,
                        coords,
                        COMBAT_KIND_GROUND,
                        1,
                        previous_owner,
                        winner_empire,
                        defender_armies,
                        attacking_armies.max(1),
                        0,
                    );
                    let attacker_survivors =
                        attacking_armies.saturating_sub(scalar_hits_with_critical(defender_ground));
                    let defender_survivors =
                        defender_armies.saturating_sub(scalar_hits_with_critical(attacker_ground));
                    let defender_battery_losses = pre_batteries.saturating_sub(
                        game_data.planets.records[planet_idx].ground_batteries_raw(),
                    );
                    let defender_army_losses =
                        pre_armies.saturating_sub(defender_survivors.min(255) as u8);

                    for &idx in &winner_fleets {
                        game_data.fleets.records[idx].set_army_count(0);
                    }
                    if attacker_survivors > 0 && defender_survivors == 0 {
                        let planet = &mut game_data.planets.records[planet_idx];
                        planet.set_owner_empire_slot_raw(winner_empire);
                        planet.set_ownership_status_raw(2);
                        planet.set_army_count_raw(attacker_survivors.min(255) as u8);
                        planet.set_ground_batteries_raw(0);
                        planet.set_conversion_countdown_raw(2);
                        events
                            .ownership_change_events
                            .push(PlanetOwnershipChangeEvent {
                                planet_idx,
                                reporting_empire_raw: previous_owner,
                                previous_owner_empire_raw: previous_owner,
                                new_owner_empire_raw: winner_empire,
                                stardate_week: None,
                            });
                        for &fleet_idx in &winner_fleets {
                            if invade_set.contains(&fleet_idx) {
                                events.mission_events.push(MissionEvent {
                                    fleet_idx,
                                    owner_empire_raw: winner_empire,
                                    kind: Mission::InvadeWorld,
                                    outcome: MissionOutcome::Succeeded,
                                    planet_idx: Some(planet_idx),
                                    location_coords: Some(
                                        game_data.planets.records[planet_idx].coords_raw(),
                                    ),
                                    target_coords: Some(
                                        game_data.planets.records[planet_idx].coords_raw(),
                                    ),
                                    stardate_week: None,
                                });
                            }
                        }
                        events.assault_report_events.push(AssaultReportEvent {
                            kind: Mission::InvadeWorld,
                            attacker_fleet_number: preferred_reporting_fleet_number(
                                game_data,
                                &winner_fleets,
                            ),
                            planet_idx,
                            attacker_empire_raw: winner_empire,
                            defender_empire_raw: previous_owner,
                            attacker_initial: ship_counts_from_state(&before),
                            attacker_loaded_armies_initial: initial_attacking_armies,
                            defender_batteries_initial: pre_batteries,
                            defender_armies_initial: pre_armies,
                            attacker_ship_losses: ship_losses_from_states(&before, &after),
                            attacker_army_losses: attacking_armies
                                .saturating_sub(attacker_survivors),
                            transport_army_losses: 0,
                            defender_battery_losses,
                            defender_army_losses,
                            outcome: MissionOutcome::Succeeded,
                            stardate_week: None,
                        });
                    } else {
                        game_data.planets.records[planet_idx]
                            .set_army_count_raw(defender_survivors.min(255) as u8);
                        for &fleet_idx in &winner_fleets {
                            if invade_set.contains(&fleet_idx) {
                                events.mission_events.push(MissionEvent {
                                    fleet_idx,
                                    owner_empire_raw: winner_empire,
                                    kind: Mission::InvadeWorld,
                                    outcome: MissionOutcome::Failed,
                                    planet_idx: Some(planet_idx),
                                    location_coords: Some(
                                        game_data.planets.records[planet_idx].coords_raw(),
                                    ),
                                    target_coords: Some(
                                        game_data.planets.records[planet_idx].coords_raw(),
                                    ),
                                    stardate_week: None,
                                });
                            }
                        }
                        events.assault_report_events.push(AssaultReportEvent {
                            kind: Mission::InvadeWorld,
                            attacker_fleet_number: preferred_reporting_fleet_number(
                                game_data,
                                &winner_fleets,
                            ),
                            planet_idx,
                            attacker_empire_raw: winner_empire,
                            defender_empire_raw: previous_owner,
                            attacker_initial: ship_counts_from_state(&before),
                            attacker_loaded_armies_initial: initial_attacking_armies,
                            defender_batteries_initial: pre_batteries,
                            defender_armies_initial: pre_armies,
                            attacker_ship_losses: ship_losses_from_states(&before, &after),
                            attacker_army_losses: attacking_armies,
                            transport_army_losses: 0,
                            defender_battery_losses,
                            defender_army_losses,
                            outcome: MissionOutcome::Failed,
                            stardate_week: None,
                        });
                    }
                } else {
                    for &idx in &winner_fleets {
                        game_data.fleets.records[idx].set_army_count(0);
                    }
                    for &fleet_idx in &winner_fleets {
                        if invade_set.contains(&fleet_idx) {
                            events.mission_events.push(MissionEvent {
                                fleet_idx,
                                owner_empire_raw: winner_empire,
                                kind: Mission::InvadeWorld,
                                outcome: MissionOutcome::Aborted,
                                planet_idx: Some(planet_idx),
                                location_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                target_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                stardate_week: None,
                            });
                        }
                    }
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::InvadeWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(
                            game_data,
                            &winner_fleets,
                        ),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&before),
                        attacker_loaded_armies_initial: initial_attacking_armies,
                        defender_batteries_initial: pre_batteries,
                        defender_armies_initial: pre_armies,
                        attacker_ship_losses: ship_losses_from_states(&before, &after),
                        attacker_army_losses: initial_attacking_armies,
                        transport_army_losses: 0,
                        defender_battery_losses: pre_batteries.saturating_sub(
                            game_data.planets.records[planet_idx].ground_batteries_raw(),
                        ),
                        defender_army_losses: 0,
                        outcome: MissionOutcome::Aborted,
                        stardate_week: None,
                    });
                }

                let intel_source = if game_data.planets.records[planet_idx].owner_empire_slot_raw()
                    == winner_empire
                {
                    PlanetIntelSource::AssaultSuccess
                } else {
                    PlanetIntelSource::AssaultFailure
                };

                clear_arrival_and_hold(game_data, &winner_fleets);
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    winner_empire,
                    intel_source,
                );
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    previous_owner,
                    intel_source,
                );
                let owner_after = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    owner_after,
                    intel_source,
                );
            }
            MissionClass::Blitz => {
                let previous_owner = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                let cover_state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let coords = game_data.planets.records[planet_idx].coords_raw();
                let pre_batteries = game_data.planets.records[planet_idx].ground_batteries_raw();
                let pre_armies = game_data.planets.records[planet_idx].army_count_raw();
                let attacking_armies: u32 = winner_fleets
                    .iter()
                    .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                    .sum();
                let cover_exchange = blitz_cover_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    winner_empire,
                    previous_owner,
                    &cover_state,
                    pre_batteries as u32 * GROUND_AS_BATTERY,
                );
                {
                    let planet = &mut game_data.planets.records[planet_idx];
                    let battery_loss = scalar_hits_with_critical(cover_exchange)
                        .min(planet.ground_batteries_raw() as u32)
                        as u8;
                    planet.set_ground_batteries_raw(
                        planet.ground_batteries_raw().saturating_sub(battery_loss),
                    );
                }
                let surviving_batteries =
                    game_data.planets.records[planet_idx].ground_batteries_raw();
                let (landing_army_losses, ship_losses) =
                    apply_blitz_landing_fire(game_data, &winner_fleets, surviving_batteries);
                let armies_after_landing = attacking_armies.saturating_sub(landing_army_losses);
                let defender_armies = game_data.planets.records[planet_idx].army_count_raw() as u32;
                let attacker_ground = resolve_ground_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_BLITZ_GROUND,
                    1,
                    winner_empire,
                    previous_owner,
                    armies_after_landing,
                    defender_armies.max(1),
                    0,
                );
                let defender_ground = resolve_ground_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_BLITZ_GROUND,
                    1,
                    previous_owner,
                    winner_empire,
                    defender_armies,
                    armies_after_landing.max(1),
                    1,
                );
                let attacker_survivors =
                    armies_after_landing.saturating_sub(scalar_hits_with_critical(defender_ground));
                let defender_survivors =
                    defender_armies.saturating_sub(scalar_hits_with_critical(attacker_ground));
                let defender_battery_losses = pre_batteries
                    .saturating_sub(game_data.planets.records[planet_idx].ground_batteries_raw());
                let defender_army_losses =
                    pre_armies.saturating_sub(defender_survivors.min(255) as u8);

                for &idx in &winner_fleets {
                    game_data.fleets.records[idx].set_army_count(0);
                }
                if attacker_survivors > 0 && defender_survivors == 0 {
                    let batteries = game_data.planets.records[planet_idx].ground_batteries_raw();
                    let planet = &mut game_data.planets.records[planet_idx];
                    planet.set_owner_empire_slot_raw(winner_empire);
                    planet.set_ownership_status_raw(2);
                    planet.set_army_count_raw(attacker_survivors.min(255) as u8);
                    planet.set_ground_batteries_raw(batteries);
                    planet.set_conversion_countdown_raw(2);
                    events
                        .ownership_change_events
                        .push(PlanetOwnershipChangeEvent {
                            planet_idx,
                            reporting_empire_raw: previous_owner,
                            previous_owner_empire_raw: previous_owner,
                            new_owner_empire_raw: winner_empire,
                            stardate_week: None,
                        });
                    for &fleet_idx in &winner_fleets {
                        if blitz_set.contains(&fleet_idx) {
                            events.mission_events.push(MissionEvent {
                                fleet_idx,
                                owner_empire_raw: winner_empire,
                                kind: Mission::BlitzWorld,
                                outcome: MissionOutcome::Succeeded,
                                planet_idx: Some(planet_idx),
                                location_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                target_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                stardate_week: None,
                            });
                        }
                    }
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::BlitzWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(
                            game_data,
                            &winner_fleets,
                        ),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&cover_state),
                        attacker_loaded_armies_initial: attacking_armies,
                        defender_batteries_initial: pre_batteries,
                        defender_armies_initial: pre_armies,
                        attacker_ship_losses: ship_losses,
                        attacker_army_losses: attacking_armies.saturating_sub(attacker_survivors),
                        transport_army_losses: landing_army_losses,
                        defender_battery_losses,
                        defender_army_losses,
                        outcome: MissionOutcome::Succeeded,
                        stardate_week: None,
                    });
                } else {
                    game_data.planets.records[planet_idx]
                        .set_army_count_raw(defender_survivors.min(255) as u8);
                    for &fleet_idx in &winner_fleets {
                        if blitz_set.contains(&fleet_idx) {
                            events.mission_events.push(MissionEvent {
                                fleet_idx,
                                owner_empire_raw: winner_empire,
                                kind: Mission::BlitzWorld,
                                outcome: MissionOutcome::Failed,
                                planet_idx: Some(planet_idx),
                                location_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                target_coords: Some(
                                    game_data.planets.records[planet_idx].coords_raw(),
                                ),
                                stardate_week: None,
                            });
                        }
                    }
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::BlitzWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(
                            game_data,
                            &winner_fleets,
                        ),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&cover_state),
                        attacker_loaded_armies_initial: attacking_armies,
                        defender_batteries_initial: pre_batteries,
                        defender_armies_initial: pre_armies,
                        attacker_ship_losses: ship_losses,
                        attacker_army_losses: attacking_armies,
                        transport_army_losses: landing_army_losses,
                        defender_battery_losses,
                        defender_army_losses,
                        outcome: MissionOutcome::Failed,
                        stardate_week: None,
                    });
                }

                let intel_source = if game_data.planets.records[planet_idx].owner_empire_slot_raw()
                    == winner_empire
                {
                    PlanetIntelSource::AssaultSuccess
                } else {
                    PlanetIntelSource::AssaultFailure
                };

                clear_arrival_and_hold(game_data, &winner_fleets);
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    winner_empire,
                    intel_source,
                );
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    previous_owner,
                    intel_source,
                );
                let owner_after = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                push_planet_intel(
                    &mut events.planet_intel_events,
                    planet_idx,
                    owner_after,
                    intel_source,
                );
            }
            _ => {}
        }
    }

    Ok(events)
}
