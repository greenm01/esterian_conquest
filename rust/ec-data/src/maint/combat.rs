//! Canonical EC combat resolution.
//!
//! The structure here owes an explicit debt to *Empire of the Sun*: both sides
//! compute their blows from the same moment in time, and only then does the
//! board reckon with the cost. That simultaneous exchange fits EC's manuals
//! better than file-order skirmishes, while staying deterministic enough for
//! Rust maintenance and classic save compatibility.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{CoreGameData, FleetStandingOrderKind};

use super::BombardEvent;

const IDX_DD: usize = 0;
const IDX_CA: usize = 1;
const IDX_BB: usize = 2;
const IDX_SB: usize = 3;
const IDX_SC: usize = 4;
const IDX_TT: usize = 5;
const IDX_ET: usize = 6;

const AS_DD: u32 = 1;
const AS_CA: u32 = 3;
const AS_BB: u32 = 9;
const AS_SB: u32 = 10;

const DS_DD: u32 = 1;
const DS_CA: u32 = 3;
const DS_BB: u32 = 10;
const DS_SB: u32 = 12;
const DS_SC: u32 = 1;
const DS_TT: u32 = 1;
const DS_ET: u32 = 2;

const GROUND_AS_BATTERY: u32 = 9;
const GROUND_AS_ARMY: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MissionClass {
    Bombard,
    Invade,
    Blitz,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BattleRole {
    IncumbentDefender,
    GuardingDefender,
    Attacker,
    Neutral,
}

#[derive(Clone, Debug, Default)]
struct FleetCombatState {
    counts: [u32; 7],
    fresh: [u32; 7],
}

#[derive(Clone, Debug)]
struct TaskForce {
    empire: u8,
    fleet_indices: Vec<usize>,
    coords: [u8; 2],
    state: FleetCombatState,
    role: BattleRole,
}

impl FleetCombatState {
    fn total_combat_as(&self) -> u32 {
        self.counts[IDX_DD] * AS_DD
            + self.counts[IDX_CA] * AS_CA
            + self.counts[IDX_BB] * AS_BB
            + self.counts[IDX_SB] * AS_SB
    }

    fn is_mixed(&self) -> bool {
        let mut kinds = 0;
        if self.counts[IDX_DD] > 0 {
            kinds += 1;
        }
        if self.counts[IDX_CA] > 0 {
            kinds += 1;
        }
        if self.counts[IDX_BB] > 0 {
            kinds += 1;
        }
        kinds >= 2
    }

    fn has_units(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }
}

fn fresh_steps_for_ds(ds: u32) -> u32 {
    std::cmp::max(1, ds.div_ceil(6))
}

fn fleet_state_from_records(game_data: &CoreGameData, fleet_indices: &[usize], starbases: u32) -> FleetCombatState {
    let mut state = FleetCombatState::default();
    for &idx in fleet_indices {
        let fleet = &game_data.fleets.records[idx];
        state.counts[IDX_DD] += fleet.destroyer_count() as u32;
        state.counts[IDX_CA] += fleet.cruiser_count() as u32;
        state.counts[IDX_BB] += fleet.battleship_count() as u32;
        state.counts[IDX_SC] += fleet.scout_count() as u32;
        state.counts[IDX_TT] += fleet.troop_transport_count() as u32;
        state.counts[IDX_ET] += fleet.etac_count() as u32;
    }
    state.counts[IDX_SB] = starbases;
    state.fresh = [
        state.counts[IDX_DD] * fresh_steps_for_ds(DS_DD),
        state.counts[IDX_CA] * fresh_steps_for_ds(DS_CA),
        state.counts[IDX_BB] * fresh_steps_for_ds(DS_BB),
        state.counts[IDX_SB] * fresh_steps_for_ds(DS_SB),
        state.counts[IDX_SC] * fresh_steps_for_ds(DS_SC),
        state.counts[IDX_TT] * fresh_steps_for_ds(DS_TT),
        state.counts[IDX_ET] * fresh_steps_for_ds(DS_ET),
    ];
    state
}

fn fleet_combat_priority() -> [usize; 7] {
    [IDX_DD, IDX_CA, IDX_BB, IDX_SB, IDX_SC, IDX_TT, IDX_ET]
}

fn nearest_owned_planet(game_data: &CoreGameData, empire: u8, from: [u8; 2]) -> Option<[u8; 2]> {
    game_data
        .planets
        .records
        .iter()
        .filter(|p| p.owner_empire_slot_raw() == empire)
        .min_by_key(|p| {
            let [x, y] = p.coords_raw();
            let dx = (x as i32 - from[0] as i32).unsigned_abs();
            let dy = (y as i32 - from[1] as i32).unsigned_abs();
            dx + dy
        })
        .map(|p| p.coords_raw())
}

fn rule_threshold_satisfied(roe: u8, friendly_as: u32, enemy_as: u32) -> bool {
    match roe {
        0 => false,
        1 => enemy_as == 0,
        2 => friendly_as >= enemy_as.saturating_mul(4),
        3 => friendly_as >= enemy_as.saturating_mul(3),
        4 => friendly_as >= enemy_as.saturating_mul(2),
        5 => friendly_as.saturating_mul(2) >= enemy_as.saturating_mul(3),
        6 => friendly_as >= enemy_as,
        7 => friendly_as.saturating_mul(3) >= enemy_as.saturating_mul(2),
        8 => friendly_as.saturating_mul(2) >= enemy_as,
        9 => friendly_as.saturating_mul(3) >= enemy_as,
        _ => true,
    }
}

fn space_cer_percent(our_as: u32, enemy_as: u32, mixed: bool, starbase_bonus: bool) -> u32 {
    let mut cer = if enemy_as == 0 {
        150
    } else if our_as.saturating_mul(2) < enemy_as {
        50
    } else if our_as < enemy_as {
        75
    } else if our_as.saturating_mul(2) < enemy_as.saturating_mul(3) {
        100
    } else if our_as < enemy_as.saturating_mul(3) {
        125
    } else {
        150
    };

    if mixed {
        cer += 25;
    }
    if starbase_bonus {
        cer += 25;
    }
    cer.clamp(25, 150)
}

fn ground_cer_percent(our_as: u32, enemy_as: u32, bonus: i32) -> u32 {
    let base = if enemy_as == 0 {
        200
    } else if our_as.saturating_mul(2) < enemy_as {
        50
    } else if our_as < enemy_as {
        100
    } else if our_as < enemy_as.saturating_mul(2) {
        150
    } else {
        200
    } as i32;

    (base + bonus * 100).clamp(50, 200) as u32
}

fn hits_from(as_total: u32, cer_percent: u32) -> u32 {
    (as_total.saturating_mul(cer_percent)).div_ceil(100)
}

fn apply_hits_to_fleet(state: &mut FleetCombatState, mut hits: u32) {
    for idx in fleet_combat_priority() {
        if hits == 0 {
            break;
        }
        let fresh_loss = hits.min(state.fresh[idx]);
        state.fresh[idx] -= fresh_loss;
        hits -= fresh_loss;
        if hits == 0 {
            break;
        }
        let destroyed = hits.min(state.counts[idx]);
        state.counts[idx] -= destroyed;
        hits -= destroyed;
    }
}

fn task_force_role(game_data: &CoreGameData, empire: u8, coords: [u8; 2], fleet_indices: &[usize]) -> BattleRole {
    if let Some(planet) = game_data.planets.records.iter().find(|p| p.coords_raw() == coords) {
        if planet.owner_empire_slot_raw() == empire {
            return BattleRole::IncumbentDefender;
        }
    }
    let guarding = fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets.records[idx].standing_order_kind(),
            FleetStandingOrderKind::PatrolSector
                | FleetStandingOrderKind::GuardStarbase
                | FleetStandingOrderKind::GuardBlockadeWorld
        )
    });
    if guarding {
        BattleRole::GuardingDefender
    } else if !fleet_indices.is_empty() {
        BattleRole::Attacker
    } else {
        BattleRole::Neutral
    }
}

fn starbase_count_at(game_data: &CoreGameData, coords: [u8; 2], owner: u8) -> u32 {
    game_data
        .bases
        .records
        .iter()
        .filter(|b| b.coords_raw() == coords && b.owner_empire_raw() == owner && b.active_flag_raw() != 0)
        .count() as u32
}

fn build_task_forces_at_location(game_data: &CoreGameData, coords: [u8; 2]) -> Vec<TaskForce> {
    let mut by_empire: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if fleet.current_location_coords_raw() == coords {
            by_empire.entry(fleet.owner_empire_raw()).or_default().push(idx);
        }
    }

    let mut empires: HashSet<u8> = by_empire.keys().copied().collect();
    if let Some(planet) = game_data.planets.records.iter().find(|p| p.coords_raw() == coords) {
        if planet.owner_empire_slot_raw() != 0
            && starbase_count_at(game_data, coords, planet.owner_empire_slot_raw()) > 0
        {
            empires.insert(planet.owner_empire_slot_raw());
        }
    }

    empires
        .into_iter()
        .map(|empire| {
            let fleet_indices = by_empire.remove(&empire).unwrap_or_default();
            let starbases = starbase_count_at(game_data, coords, empire);
            let role = task_force_role(game_data, empire, coords, &fleet_indices);
            let state = fleet_state_from_records(game_data, &fleet_indices, starbases);
            TaskForce {
                empire,
                fleet_indices,
                coords,
                state,
                role,
            }
        })
        .collect()
}

fn empire_target_priority(
    our_empire: u8,
    our_role: BattleRole,
    candidates: &[TaskForce],
    planet_owner: Option<u8>,
) -> Option<u8> {
    let _ = our_role;
    candidates
        .iter()
        .filter(|tf| tf.empire != our_empire && tf.state.has_units())
        .min_by_key(|tf| {
            let guarding = matches!(tf.role, BattleRole::IncumbentDefender | BattleRole::GuardingDefender);
            let threatens_ours = planet_owner == Some(our_empire)
                && matches!(tf.role, BattleRole::Attacker);
            (
                !guarding,
                !threatens_ours,
                std::cmp::Reverse(tf.state.total_combat_as()),
                tf.empire,
            )
        })
        .map(|tf| tf.empire)
}

fn distribute_fleet_losses(game_data: &mut CoreGameData, fleet_indices: &[usize], before: &FleetCombatState, after: &FleetCombatState) {
    let losses = [
        before.counts[IDX_DD].saturating_sub(after.counts[IDX_DD]),
        before.counts[IDX_CA].saturating_sub(after.counts[IDX_CA]),
        before.counts[IDX_BB].saturating_sub(after.counts[IDX_BB]),
        before.counts[IDX_SC].saturating_sub(after.counts[IDX_SC]),
        before.counts[IDX_TT].saturating_sub(after.counts[IDX_TT]),
        before.counts[IDX_ET].saturating_sub(after.counts[IDX_ET]),
    ];

    let mut remaining = losses;
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];

        let dd_loss = remaining[0].min(fleet.destroyer_count() as u32) as u16;
        fleet.set_destroyer_count(fleet.destroyer_count().saturating_sub(dd_loss));
        remaining[0] -= dd_loss as u32;

        let ca_loss = remaining[1].min(fleet.cruiser_count() as u32) as u16;
        fleet.set_cruiser_count(fleet.cruiser_count().saturating_sub(ca_loss));
        remaining[1] -= ca_loss as u32;

        let bb_loss = remaining[2].min(fleet.battleship_count() as u32) as u16;
        fleet.set_battleship_count(fleet.battleship_count().saturating_sub(bb_loss));
        remaining[2] -= bb_loss as u32;

        let sc_loss = remaining[3].min(fleet.scout_count() as u32) as u8;
        fleet.set_scout_count(fleet.scout_count().saturating_sub(sc_loss));
        remaining[3] -= sc_loss as u32;

        let tt_loss = remaining[4].min(fleet.troop_transport_count() as u32) as u16;
        fleet.set_troop_transport_count(fleet.troop_transport_count().saturating_sub(tt_loss));
        // Each destroyed transport takes one loaded army with it.
        if tt_loss > 0 {
            fleet.set_army_count(fleet.army_count().saturating_sub(tt_loss));
        }
        remaining[4] -= tt_loss as u32;

        let et_loss = remaining[5].min(fleet.etac_count() as u32) as u16;
        fleet.set_etac_count(fleet.etac_count().saturating_sub(et_loss));
        remaining[5] -= et_loss as u32;
    }
}

fn retreat_task_force(game_data: &mut CoreGameData, task_force: &TaskForce) {
    let retreat_target = nearest_owned_planet(game_data, task_force.empire, task_force.coords)
        .unwrap_or(task_force.coords);

    for &idx in &task_force.fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        if fleet.destroyer_count() == 0
            && fleet.cruiser_count() == 0
            && fleet.battleship_count() == 0
            && fleet.scout_count() == 0
            && fleet.troop_transport_count() == 0
            && fleet.etac_count() == 0
        {
            fleet.set_current_speed(0);
            fleet.set_standing_order_code_raw(0);
            fleet.set_rules_of_engagement(0);
            continue;
        }

        fleet.set_standing_order_code_raw(0x02);
        fleet.set_standing_order_target_coords_raw(retreat_target);
        fleet.set_current_speed(fleet.max_speed().min(3).max(1));
        fleet.raw[0x0d] = 0x7f;
        fleet.raw[0x0e] = 0xc0;
        fleet.raw[0x10] = 0xff;
        fleet.raw[0x11] = 0xff;
        fleet.raw[0x12] = 0x7f;
        fleet.raw[0x19] = 0x00;
        fleet.set_rules_of_engagement(0);
    }
}

pub(crate) fn process_fleet_battles(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut coord_set = HashSet::new();
    for fleet in &game_data.fleets.records {
        coord_set.insert(fleet.current_location_coords_raw());
    }

    for coords in coord_set {
        let mut task_forces = build_task_forces_at_location(game_data, coords);
        if task_forces.iter().filter(|tf| tf.state.has_units()).count() < 2 {
            continue;
        }

        let planet_owner = game_data
            .planets
            .records
            .iter()
            .find(|p| p.coords_raw() == coords)
            .map(|p| p.owner_empire_slot_raw());

        let original_states: HashMap<u8, FleetCombatState> = task_forces
            .iter()
            .map(|tf| (tf.empire, tf.state.clone()))
            .collect();

        for _round in 0..3 {
            let active: Vec<u8> = task_forces
                .iter()
                .filter(|tf| tf.state.has_units() && tf.state.total_combat_as() > 0)
                .map(|tf| tf.empire)
                .collect();
            if active.len() < 2 {
                break;
            }

            let combat_as_map: HashMap<u8, u32> = task_forces
                .iter()
                .map(|tf| (tf.empire, tf.state.total_combat_as()))
                .collect();

            let mut pending_hits: HashMap<u8, u32> = HashMap::new();
            for tf in &task_forces {
                let our_as = *combat_as_map.get(&tf.empire).unwrap_or(&0);
                if our_as == 0 {
                    continue;
                }

                let enemy_as = task_forces
                    .iter()
                    .filter(|other| other.empire != tf.empire)
                    .map(|other| other.state.total_combat_as())
                    .max()
                    .unwrap_or(0);
                if enemy_as == 0 {
                    continue;
                }

                let roe = tf
                    .fleet_indices
                    .iter()
                    .filter_map(|idx| {
                        let fleet = &game_data.fleets.records[*idx];
                        (fleet.destroyer_count() > 0
                            || fleet.cruiser_count() > 0
                            || fleet.battleship_count() > 0)
                            .then_some(fleet.rules_of_engagement())
                    })
                    .max()
                    .unwrap_or(0);
                if !rule_threshold_satisfied(roe, our_as, enemy_as) && tf.role == BattleRole::Attacker
                {
                    continue;
                }

                let target = empire_target_priority(tf.empire, tf.role, &task_forces, planet_owner);
                let Some(target_empire) = target else {
                    continue;
                };
                let starbase_bonus = tf.state.counts[IDX_SB] > 0 && tf.state.fresh[IDX_SB] > 0;
                let cer = space_cer_percent(our_as, enemy_as, tf.state.is_mixed(), starbase_bonus);
                let hits = hits_from(our_as, cer);
                *pending_hits.entry(target_empire).or_default() += hits;
            }

            if pending_hits.is_empty() {
                break;
            }

            for tf in &mut task_forces {
                if let Some(&hits) = pending_hits.get(&tf.empire) {
                    apply_hits_to_fleet(&mut tf.state, hits);
                }
            }
        }

        let winner_empire = {
            let mut survivors: Vec<&TaskForce> = task_forces
                .iter()
                .filter(|tf| tf.state.has_units() && tf.state.total_combat_as() > 0)
                .collect();
            if survivors.len() == 1 {
                Some(survivors[0].empire)
            } else if survivors.is_empty() {
                None
            } else {
                survivors.sort_by_key(|tf| {
                    (
                        match tf.role {
                            BattleRole::IncumbentDefender => 0u8,
                            BattleRole::GuardingDefender => 1u8,
                            BattleRole::Attacker => 2u8,
                            BattleRole::Neutral => 3u8,
                        },
                        std::cmp::Reverse(tf.state.total_combat_as()),
                        tf.empire,
                    )
                });
                Some(survivors[0].empire)
            }
        };

        for tf in &task_forces {
            if let Some(before) = original_states.get(&tf.empire) {
                distribute_fleet_losses(game_data, &tf.fleet_indices, before, &tf.state);
            }
        }

        for tf in &task_forces {
            if Some(tf.empire) != winner_empire {
                retreat_task_force(game_data, tf);
            }
        }
    }

    Ok(())
}

fn clear_arrival_and_hold(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        fleet.set_standing_order_code_raw(0);
        fleet.set_current_speed(0);
        fleet.raw[0x19] = 0x81;
        fleet.raw[0x1a] = 0x00;
        fleet.raw[0x1b] = 0x00;
        fleet.raw[0x1c] = 0x00;
        fleet.raw[0x1d] = 0x00;
        fleet.raw[0x1e] = 0x00;
    }
}

fn reduce_stardock(planet: &mut crate::PlanetRecord, mut hits: u32) -> u32 {
    for slot in 0..10 {
        if hits == 0 {
            break;
        }
        let count = planet.stardock_count_raw(slot) as u32;
        if count == 0 {
            continue;
        }
        let destroyed = hits.min(count) as u16;
        planet.set_stardock_count_raw(slot, planet.stardock_count_raw(slot).saturating_sub(destroyed));
        if planet.stardock_count_raw(slot) == 0 {
            planet.set_stardock_kind_raw(slot, 0);
        }
        hits -= destroyed as u32;
    }
    hits
}

fn apply_planet_bombardment_damage(planet: &mut crate::PlanetRecord, mut hits: u32) {
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
    state.counts[IDX_DD] * 1 / 2 + state.counts[IDX_CA] * 3 + state.counts[IDX_BB] * 9 * 3 / 2
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
                game_data.fleets.records[idx].standing_order_kind() == FleetStandingOrderKind::SeekHome
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
        if contenders.iter().any(|(emp, as_total, _)| *emp == owner && *as_total == contenders[0].1)
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
    bombard_ready: &[usize],
    invade_ready: &[usize],
    blitz_ready: &[usize],
) -> Result<Vec<BombardEvent>, Box<dyn std::error::Error>> {
    let mut by_planet: BTreeMap<usize, BTreeMap<u8, Vec<usize>>> = BTreeMap::new();
    for &idx in bombard_ready.iter().chain(invade_ready).chain(blitz_ready) {
        let coords = game_data.fleets.records[idx].standing_order_target_coords_raw();
        if let Some(planet_idx) = game_data.planets.records.iter().position(|p| p.coords_raw() == coords) {
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

    let mut bombard_events = Vec::new();

    for (planet_idx, entrants) in by_planet {
        let Some(winner_empire) = select_orbital_supremacy_empire(game_data, planet_idx, &entrants) else {
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
                let state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let attack_as = bombard_attack_as(&state);
                let planet = &game_data.planets.records[planet_idx];
                let defense_as =
                    planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY
                        + (planet.army_count_raw() as u32).div_ceil(2) * GROUND_AS_ARMY;
                let attacker_cer = space_cer_percent(attack_as, defense_as, state.is_mixed(), false);
                let defender_cer = space_cer_percent(defense_as, attack_as.max(1), false, false);
                let attacker_hits = hits_from(attack_as, attacker_cer);
                let defender_hits = hits_from(defense_as, defender_cer);

                let before = state.clone();
                let mut after = state.clone();
                apply_hits_to_fleet(&mut after, defender_hits);
                distribute_fleet_losses(game_data, &winner_fleets, &before, &after);

                apply_planet_bombardment_damage(&mut game_data.planets.records[planet_idx], attacker_hits);
                clear_arrival_and_hold(game_data, &winner_fleets);
                bombard_events.push(BombardEvent {
                    planet_idx,
                    attacker_empire_raw: winner_empire,
                });
            }
            MissionClass::Invade => {
                let state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let bombard_as = bombard_attack_as(&state);
                let planet = &game_data.planets.records[planet_idx];
                let battery_as = planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY;
                let attacker_cer = space_cer_percent(bombard_as, battery_as.max(1), state.is_mixed(), false);
                let defender_cer = space_cer_percent(battery_as, bombard_as.max(1), false, false);
                let suppression_hits = hits_from(bombard_as, attacker_cer);
                let return_hits = hits_from(battery_as, defender_cer);

                let before = state.clone();
                let mut after = state.clone();
                apply_hits_to_fleet(&mut after, return_hits);
                distribute_fleet_losses(game_data, &winner_fleets, &before, &after);

                {
                    let planet = &mut game_data.planets.records[planet_idx];
                    let battery_loss = suppression_hits.min(planet.ground_batteries_raw() as u32) as u8;
                    planet.set_ground_batteries_raw(planet.ground_batteries_raw().saturating_sub(battery_loss));
                }

                let batteries_cleared = game_data.planets.records[planet_idx].ground_batteries_raw() == 0;
                if batteries_cleared {
                    let soft_hits = hits_from(bombard_attack_as(&after), ground_cer_percent(bombard_attack_as(&after), game_data.planets.records[planet_idx].army_count_raw() as u32, 0));
                    apply_planet_bombardment_damage(&mut game_data.planets.records[planet_idx], soft_hits);

                    let attacking_armies: u32 = winner_fleets
                        .iter()
                        .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                        .sum();
                    let defender_armies = game_data.planets.records[planet_idx].army_count_raw() as u32;
                    let atk_hits = hits_from(attacking_armies, ground_cer_percent(attacking_armies, defender_armies.max(1), 0));
                    let def_hits = hits_from(defender_armies, ground_cer_percent(defender_armies, attacking_armies.max(1), 0));
                    let attacker_survivors = attacking_armies.saturating_sub(def_hits);
                    let defender_survivors = defender_armies.saturating_sub(atk_hits);

                    for &idx in &winner_fleets {
                        game_data.fleets.records[idx].set_army_count(0);
                    }
                    if attacker_survivors > 0 && defender_survivors == 0 {
                        let planet = &mut game_data.planets.records[planet_idx];
                        planet.set_owner_empire_slot_raw(winner_empire);
                        planet.set_ownership_status_raw(2);
                        planet.set_army_count_raw(attacker_survivors.min(255) as u8);
                        planet.set_ground_batteries_raw(0);
                    } else {
                        game_data.planets.records[planet_idx]
                            .set_army_count_raw(defender_survivors.min(255) as u8);
                    }
                } else {
                    for &idx in &winner_fleets {
                        game_data.fleets.records[idx].set_army_count(0);
                    }
                }

                clear_arrival_and_hold(game_data, &winner_fleets);
            }
            MissionClass::Blitz => {
                let attacking_armies: u32 = winner_fleets
                    .iter()
                    .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                    .sum();
                let planet = &game_data.planets.records[planet_idx];
                let landing_fire = hits_from(
                    planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY,
                    ground_cer_percent(
                        planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY,
                        attacking_armies.max(1),
                        0,
                    ),
                );

                let mut armies_after_landing = attacking_armies;
                let mut tt_losses = landing_fire;
                for &idx in &winner_fleets {
                    if tt_losses == 0 {
                        break;
                    }
                    let fleet = &mut game_data.fleets.records[idx];
                    let loss = tt_losses.min(fleet.troop_transport_count() as u32) as u16;
                    fleet.set_troop_transport_count(fleet.troop_transport_count().saturating_sub(loss));
                    fleet.set_army_count(fleet.army_count().saturating_sub(loss));
                    armies_after_landing = armies_after_landing.saturating_sub(loss as u32);
                    tt_losses -= loss as u32;
                }
                armies_after_landing = armies_after_landing.saturating_sub(tt_losses);

                let defender_armies = game_data.planets.records[planet_idx].army_count_raw() as u32;
                let atk_hits = hits_from(armies_after_landing, ground_cer_percent(armies_after_landing, defender_armies.max(1), 0));
                let def_hits = hits_from(defender_armies, ground_cer_percent(defender_armies, armies_after_landing.max(1), 1));
                let attacker_survivors = armies_after_landing.saturating_sub(def_hits);
                let defender_survivors = defender_armies.saturating_sub(atk_hits);

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
                } else {
                    game_data.planets.records[planet_idx]
                        .set_army_count_raw(defender_survivors.min(255) as u8);
                }

                clear_arrival_and_hold(game_data, &winner_fleets);
            }
            _ => {}
        }
    }

    Ok(bombard_events)
}
