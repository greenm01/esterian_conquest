use std::collections::{HashMap, HashSet};

use crate::{
    CoreGameData, DiplomacyOverride, DiplomaticRelation, EncounterDispositionEvent,
    EncounterDispositionReason, FleetBattleEvent, FleetDestroyedEvent, FleetOrderValidationError,
    MissionEvent, MissionOutcome, Order, ScoutContactEvent, StarbaseDestroyedEvent,
};

use super::exchange::{
    COMBAT_GUARDRAIL_MAX_ROUNDS, COMBAT_KIND_FLEET, RoundAction, RoundActionKind,
    apply_hits_to_fleet, fleet_state_changed, has_starbase_column_bonus, resolve_space_exchange,
    resolve_withdrawal_exchange, rule_threshold_satisfied,
};
use super::reporting::{
    loaded_armies_for_fleet_indices, mission_kind_for_order, preferred_reporting_fleet_index,
    push_contact_event_for_task_force, report_perspective_for_mission, single_named_fleet_number,
};
use super::retreat::{
    apply_roe_retreat_to_task_force, clear_empty_withdrawn_fleets, dominant_empire_after_battle,
    nearest_owned_planet, retreat_task_force, set_fleet_to_hold_current_position,
};
use super::state::{
    BattleRole, EncounterContext, FleetCombatState, IDX_SB, TaskForce,
    build_task_forces_at_location, has_anchored_guard_order, planet_idx_at_coords,
    ship_counts_from_state, ship_losses_from_states, tf_has_any_units,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostilityReason {
    DeclaredEnemy,
    DefendedSystemEntry,
    PatrolContact,
}

fn hostility_requires_forced_engagement(reason: HostilityReason) -> bool {
    matches!(reason, HostilityReason::DefendedSystemEntry)
}

fn effective_diplomatic_relation(
    game_data: &CoreGameData,
    diplomacy_overrides: &[DiplomacyOverride],
    from_empire_raw: u8,
    to_empire_raw: u8,
) -> Option<DiplomaticRelation> {
    diplomacy_overrides
        .iter()
        .find(|directive| {
            directive.from_empire_raw == from_empire_raw && directive.to_empire_raw == to_empire_raw
        })
        .map(|directive| directive.relation)
        .or_else(|| game_data.stored_diplomatic_relation(from_empire_raw, to_empire_raw))
}

fn hostility_reason_between(
    game_data: &CoreGameData,
    diplomacy_overrides: &[DiplomacyOverride],
    coords: [u8; 2],
    left: &TaskForce,
    right: &TaskForce,
) -> Option<HostilityReason> {
    if left.empire == right.empire {
        return None;
    }

    let left_context = super::state::task_force_encounter_context(game_data, left);
    let right_context = super::state::task_force_encounter_context(game_data, right);

    if matches!(
        effective_diplomatic_relation(game_data, diplomacy_overrides, left.empire, right.empire),
        Some(DiplomaticRelation::Enemy)
    ) || matches!(
        effective_diplomatic_relation(game_data, diplomacy_overrides, right.empire, left.empire),
        Some(DiplomaticRelation::Enemy)
    ) {
        return match (left_context, right_context) {
            (EncounterContext::SystemEntry, EncounterContext::SystemEntry) => {
                Some(HostilityReason::DefendedSystemEntry)
            }
            (EncounterContext::SystemEntry, EncounterContext::SectorPatrol)
            | (EncounterContext::SectorPatrol, EncounterContext::SystemEntry)
            | (EncounterContext::SectorPatrol, EncounterContext::SectorPatrol) => {
                Some(HostilityReason::PatrolContact)
            }
            (EncounterContext::DeepSpaceTransit, EncounterContext::DeepSpaceTransit)
            | (EncounterContext::DeepSpaceTransit, EncounterContext::SectorPatrol)
            | (EncounterContext::SectorPatrol, EncounterContext::DeepSpaceTransit) => {
                Some(HostilityReason::DeclaredEnemy)
            }
            (EncounterContext::SystemEntry, EncounterContext::DeepSpaceTransit)
            | (EncounterContext::DeepSpaceTransit, EncounterContext::SystemEntry) => {
                let transit_side = if left_context == EncounterContext::DeepSpaceTransit {
                    left
                } else {
                    right
                };
                let has_assault_posture = transit_side.fleet_indices.iter().any(|&idx| {
                    matches!(
                        game_data.fleets.records[idx].standing_order_kind(),
                        Order::InvadeWorld | Order::BombardWorld | Order::BlitzWorld
                    )
                });

                if has_anchored_guard_order(game_data, &left.fleet_indices)
                    || has_anchored_guard_order(game_data, &right.fleet_indices)
                {
                    if has_assault_posture {
                        Some(HostilityReason::DefendedSystemEntry)
                    } else {
                        None
                    }
                } else {
                    Some(HostilityReason::DeclaredEnemy)
                }
            }
        };
    }

    if let Some(planet) = game_data
        .planets
        .records
        .iter()
        .find(|p| p.coords_raw() == coords)
    {
        let owner = planet.owner_empire_slot_raw();
        if owner != 0
            && (owner == left.empire || owner == right.empire)
            && matches!(
                (left_context, right_context),
                (EncounterContext::SystemEntry, _) | (_, EncounterContext::SystemEntry)
            )
        {
            return Some(HostilityReason::DefendedSystemEntry);
        }
    }

    None
}

fn hostile_target_priority(
    our_empire: u8,
    our_role: BattleRole,
    candidates: &[(&TaskForce, HostilityReason)],
    planet_owner: Option<u8>,
) -> Option<(u8, HostilityReason)> {
    let _ = our_role;
    candidates
        .iter()
        .copied()
        .filter(|(tf, _)| tf.empire != our_empire && tf.state.has_units())
        .min_by_key(|(tf, _)| {
            let guarding = matches!(
                tf.role,
                BattleRole::IncumbentDefender | BattleRole::GuardingDefender
            );
            let threatens_ours =
                planet_owner == Some(our_empire) && matches!(tf.role, BattleRole::Attacker);
            (
                !guarding,
                !threatens_ours,
                std::cmp::Reverse(tf.state.total_combat_as()),
                tf.empire,
            )
        })
        .map(|(tf, reason)| (tf.empire, reason))
}

pub(super) fn distribute_fleet_losses(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    before: &FleetCombatState,
    after: &FleetCombatState,
) {
    let losses = [
        before.counts[super::state::IDX_DD].saturating_sub(after.counts[super::state::IDX_DD]),
        before.counts[super::state::IDX_CA].saturating_sub(after.counts[super::state::IDX_CA]),
        before.counts[super::state::IDX_BB].saturating_sub(after.counts[super::state::IDX_BB]),
        before.counts[super::state::IDX_SC].saturating_sub(after.counts[super::state::IDX_SC]),
        before.counts[super::state::IDX_TT].saturating_sub(after.counts[super::state::IDX_TT]),
        before.counts[super::state::IDX_ET].saturating_sub(after.counts[super::state::IDX_ET]),
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
        if tt_loss > 0 {
            fleet.set_army_count(fleet.army_count().saturating_sub(tt_loss));
        }
        remaining[4] -= tt_loss as u32;

        let et_loss = remaining[5].min(fleet.etac_count() as u32) as u16;
        fleet.set_etac_count(fleet.etac_count().saturating_sub(et_loss));
        remaining[5] -= et_loss as u32;
    }
}

fn remove_destroyed_starbases(
    game_data: &mut CoreGameData,
    coords: [u8; 2],
    owner: u8,
    destroyed: u32,
) -> Vec<u8> {
    let mut remaining = destroyed;
    let mut destroyed_ids = Vec::new();
    for base in &mut game_data.bases.records {
        if remaining == 0 {
            break;
        }
        if base.coords_raw() == coords
            && base.owner_empire_raw() == owner
            && base.active_flag_raw() != 0
        {
            destroyed_ids.push(base.base_id_raw());
            *base = nc_data::BaseRecord::new_zeroed();
            remaining -= 1;
        }
    }

    if let Some(player) = game_data
        .player
        .records
        .get_mut(owner.saturating_sub(1) as usize)
    {
        player.set_starbase_count_raw(
            player
                .starbase_count_raw()
                .saturating_sub(destroyed_ids.len() as u16),
        );
    }

    destroyed_ids
}

fn is_ship_loss_abort_reason(order: Order, reason: FleetOrderValidationError) -> bool {
    matches!(
        (order, reason),
        (
            Order::BombardWorld,
            FleetOrderValidationError::MissingCombatShips
        ) | (
            Order::InvadeWorld,
            FleetOrderValidationError::MissingCombatShips
                | FleetOrderValidationError::MissingLoadedTroopTransports,
        ) | (
            Order::BlitzWorld,
            FleetOrderValidationError::MissingLoadedTroopTransports,
        ) | (
            Order::ScoutSector | Order::ScoutSolarSystem,
            FleetOrderValidationError::MissingScoutShip,
        )
    )
}

fn abort_invalid_dominant_missions_after_battle(
    game_data: &mut CoreGameData,
    events: &mut FleetBattlePhaseEvents,
    task_forces: &[TaskForce],
    dominant_empire: Option<u8>,
    pre_retreat_orders: &HashMap<usize, Order>,
    coords: [u8; 2],
) {
    let Some(dominant_empire) = dominant_empire else {
        return;
    };

    for task_force in task_forces {
        if task_force.empire != dominant_empire || task_force.withdrew_under_roe {
            continue;
        }
        for &fleet_idx in &task_force.fleet_indices {
            let Some(order) = pre_retreat_orders.get(&fleet_idx).copied() else {
                continue;
            };
            let Some(kind) = mission_kind_for_order(Some(order)) else {
                continue;
            };
            let target_coords =
                game_data.fleets.records[fleet_idx].standing_order_target_coords_raw();
            let validation = game_data.validate_fleet_order_payload(
                fleet_idx + 1,
                order.to_raw(),
                target_coords,
                None,
                None,
            );
            let Err(reason) = validation else {
                continue;
            };
            if !is_ship_loss_abort_reason(order, reason) {
                continue;
            }

            let owner_empire_raw = game_data.fleets.records[fleet_idx].owner_empire_raw();
            {
                let fleet = &mut game_data.fleets.records[fleet_idx];
                set_fleet_to_hold_current_position(fleet);
            }
            events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw,
                kind,
                outcome: MissionOutcome::Aborted,
                planet_idx: planet_idx_at_coords(game_data, coords),
                location_coords: Some(coords),
                target_coords: Some(
                    game_data.fleets.records[fleet_idx].standing_order_target_coords_raw(),
                ),
                stardate_week: None,
            });
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct FleetBattlePhaseEvents {
    pub fleet_battle_events: Vec<FleetBattleEvent>,
    pub fleet_destroyed_events: Vec<FleetDestroyedEvent>,
    pub starbase_destroyed_events: Vec<StarbaseDestroyedEvent>,
    pub scout_contact_events: Vec<ScoutContactEvent>,
    pub encounter_disposition_events: Vec<EncounterDispositionEvent>,
    pub mission_events: Vec<MissionEvent>,
}

pub(crate) fn process_fleet_battles(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<FleetBattlePhaseEvents, Box<dyn std::error::Error>> {
    let mut coord_set = HashSet::new();
    for fleet in &game_data.fleets.records {
        coord_set.insert(fleet.current_location_coords_raw());
    }
    let mut events = FleetBattlePhaseEvents::default();

    for coords in coord_set {
        let mut task_forces = build_task_forces_at_location(game_data, coords);
        let pre_encounter_orders: HashMap<usize, Order> = task_forces
            .iter()
            .flat_map(|tf| tf.fleet_indices.iter().copied())
            .map(|idx| (idx, game_data.fleets.records[idx].standing_order_kind()))
            .collect();
        let participants: Vec<u8> = task_forces
            .iter()
            .filter(|tf| tf.state.has_units())
            .map(|tf| tf.empire)
            .collect();
        if participants.len() < 2 {
            continue;
        }

        let original_states: HashMap<u8, FleetCombatState> = task_forces
            .iter()
            .map(|tf| (tf.empire, tf.state.clone()))
            .collect();
        let original_loaded_armies: HashMap<u8, u32> = task_forces
            .iter()
            .map(|tf| {
                (
                    tf.empire,
                    loaded_armies_for_fleet_indices(game_data, &tf.fleet_indices),
                )
            })
            .collect();

        for (i, left) in task_forces.iter().enumerate() {
            for right in task_forces.iter().skip(i + 1) {
                if left.empire == right.empire
                    || !left.state.has_units()
                    || !right.state.has_units()
                {
                    continue;
                }
                push_contact_event_for_task_force(
                    &mut events.scout_contact_events,
                    game_data,
                    coords,
                    left,
                    right,
                );
                push_contact_event_for_task_force(
                    &mut events.scout_contact_events,
                    game_data,
                    coords,
                    right,
                    left,
                );
            }
        }

        let has_hostile_pair = task_forces.iter().enumerate().any(|(i, left)| {
            task_forces.iter().skip(i + 1).any(|right| {
                hostility_reason_between(game_data, diplomacy_overrides, coords, left, right)
                    .is_some()
            })
        });
        if !has_hostile_pair {
            continue;
        }

        let battle_year = game_data.conquest.game_year();
        let planet_owner = game_data
            .planets
            .records
            .iter()
            .find(|p| p.coords_raw() == coords)
            .map(|p| p.owner_empire_slot_raw());
        let mut combat_occurred = false;
        let mut resolved_within_guardrail = false;

        for round in 1..=COMBAT_GUARDRAIL_MAX_ROUNDS {
            let active_empires: Vec<u8> = task_forces
                .iter()
                .filter(|tf| {
                    tf.state.has_units() && tf.state.total_combat_as() > 0 && !tf.withdrew_under_roe
                })
                .map(|tf| tf.empire)
                .collect();
            if active_empires.len() < 2 {
                resolved_within_guardrail = true;
                break;
            }

            let combat_as_map: HashMap<u8, u32> = task_forces
                .iter()
                .map(|tf| (tf.empire, tf.state.total_combat_as()))
                .collect();
            let mut actions = Vec::new();
            for tf in &task_forces {
                let our_as = *combat_as_map.get(&tf.empire).unwrap_or(&0);
                if our_as == 0 || tf.withdrew_under_roe || !tf.state.has_units() {
                    continue;
                }
                let hostile_opponents = task_forces
                    .iter()
                    .filter_map(|other| {
                        (other.empire != tf.empire)
                            .then(|| {
                                hostility_reason_between(
                                    game_data,
                                    diplomacy_overrides,
                                    coords,
                                    tf,
                                    other,
                                )
                                .map(|reason| (other, reason))
                            })
                            .flatten()
                    })
                    .collect::<Vec<_>>();
                let enemy_as = hostile_opponents
                    .iter()
                    .map(|(other, _)| other.state.total_combat_as())
                    .max()
                    .unwrap_or(0);
                let Some((target_empire, hostility_reason)) =
                    hostile_target_priority(tf.empire, tf.role, &hostile_opponents, planet_owner)
                else {
                    continue;
                };
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
                let forced_engagement = hostility_requires_forced_engagement(hostility_reason);
                let kind = if !forced_engagement && !rule_threshold_satisfied(roe, our_as, enemy_as)
                {
                    RoundActionKind::Withdraw
                } else {
                    RoundActionKind::Fight
                };
                actions.push(RoundAction {
                    empire: tf.empire,
                    target_empire,
                    kind,
                });
            }

            if actions.is_empty() {
                resolved_within_guardrail = true;
                break;
            }

            let before_round_states: HashMap<u8, FleetCombatState> = task_forces
                .iter()
                .map(|tf| (tf.empire, tf.state.clone()))
                .collect();
            let mut pending_hits: HashMap<u8, u32> = HashMap::new();
            let mut pending_criticals: HashMap<u8, u32> = HashMap::new();
            let mut engaged_empires = HashSet::new();
            let mut pre_round_withdrawals = HashSet::new();
            let mut reciprocal_withdrawal_replies = HashSet::new();

            let mut ordered_actions = actions;
            ordered_actions.sort_by_key(|action| action.empire);
            for action in ordered_actions {
                if reciprocal_withdrawal_replies.remove(&(action.empire, action.target_empire)) {
                    continue;
                }

                let Some(actor_tf) = task_forces.iter().find(|tf| tf.empire == action.empire)
                else {
                    continue;
                };
                let Some(target_tf) = task_forces
                    .iter()
                    .find(|tf| tf.empire == action.target_empire)
                else {
                    continue;
                };
                let our_as = actor_tf.state.total_combat_as();
                if our_as == 0 {
                    continue;
                }
                let enemy_as = target_tf.state.total_combat_as();
                engaged_empires.insert(action.empire);
                engaged_empires.insert(action.target_empire);

                match action.kind {
                    RoundActionKind::Fight => {
                        let result = resolve_space_exchange(
                            campaign_seed,
                            battle_year,
                            coords,
                            COMBAT_KIND_FLEET,
                            round,
                            action.empire,
                            action.target_empire,
                            our_as,
                            enemy_as,
                            actor_tf.state.is_mixed(),
                            has_starbase_column_bonus(&actor_tf.state),
                        );
                        *pending_hits.entry(action.target_empire).or_default() += result.hits;
                        *pending_criticals.entry(action.target_empire).or_default() +=
                            u32::from(result.critical);
                    }
                    RoundActionKind::Withdraw => {
                        pre_round_withdrawals.insert(action.empire);
                        let outbound = resolve_withdrawal_exchange(
                            campaign_seed,
                            battle_year,
                            coords,
                            round,
                            action.empire,
                            action.target_empire,
                            our_as,
                        );
                        *pending_hits.entry(action.target_empire).or_default() += outbound.hits;
                        *pending_criticals.entry(action.target_empire).or_default() +=
                            u32::from(outbound.critical);

                        let target_as = target_tf.state.total_combat_as();
                        if target_as > 0 {
                            let reply = resolve_withdrawal_exchange(
                                campaign_seed,
                                battle_year,
                                coords,
                                round,
                                action.target_empire,
                                action.empire,
                                target_as,
                            );
                            *pending_hits.entry(action.empire).or_default() += reply.hits;
                            *pending_criticals.entry(action.empire).or_default() +=
                                u32::from(reply.critical);
                            reciprocal_withdrawal_replies
                                .insert((action.target_empire, action.empire));
                        }
                    }
                }
            }

            for empire in engaged_empires {
                if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                    task_force.engaged_in_battle = true;
                }
            }

            for tf in &mut task_forces {
                let hits = pending_hits.get(&tf.empire).copied().unwrap_or(0);
                let critical_hits = pending_criticals.get(&tf.empire).copied().unwrap_or(0);
                if hits > 0 || critical_hits > 0 {
                    apply_hits_to_fleet(&mut tf.state, hits, critical_hits);
                }
            }

            let mut any_withdrawal = false;
            for empire in pre_round_withdrawals {
                if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                    task_force.withdrew_under_roe = true;
                    let retreat_target =
                        nearest_owned_planet(game_data, empire, coords).unwrap_or(coords);
                    apply_roe_retreat_to_task_force(
                        game_data,
                        &task_force.fleet_indices,
                        retreat_target,
                    );
                    any_withdrawal = true;
                }
            }

            let any_round_state_change = task_forces.iter().any(|tf| {
                before_round_states
                    .get(&tf.empire)
                    .is_some_and(|before| fleet_state_changed(before, &tf.state))
            });
            combat_occurred |= any_round_state_change || any_withdrawal;

            let remaining_active_after_exchange = task_forces
                .iter()
                .filter(|tf| {
                    tf.state.has_units() && tf.state.total_combat_as() > 0 && !tf.withdrew_under_roe
                })
                .count();
            if remaining_active_after_exchange < 2 {
                resolved_within_guardrail = true;
                break;
            }

            let current_as_map: HashMap<u8, u32> = task_forces
                .iter()
                .map(|tf| (tf.empire, tf.state.total_combat_as()))
                .collect();
            let mut post_round_retreats = Vec::new();
            let mut free_holds_to_consume = Vec::new();
            for tf in &task_forces {
                if !tf.engaged_in_battle || tf.withdrew_under_roe || !tf.state.has_units() {
                    continue;
                }
                let our_as = *current_as_map.get(&tf.empire).unwrap_or(&0);
                if our_as == 0 {
                    continue;
                }
                let hostile_opponents = task_forces
                    .iter()
                    .filter(|other| !other.withdrew_under_roe && other.empire != tf.empire)
                    .filter_map(|other| {
                        hostility_reason_between(game_data, diplomacy_overrides, coords, tf, other)
                            .map(|_| other)
                    })
                    .collect::<Vec<_>>();
                let enemy_as = hostile_opponents
                    .iter()
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
                if !rule_threshold_satisfied(roe, our_as, enemy_as) {
                    let is_guard = matches!(
                        tf.role,
                        BattleRole::GuardingDefender | BattleRole::IncumbentDefender
                    );
                    if is_guard && !tf.free_hold_used {
                        free_holds_to_consume.push(tf.empire);
                        continue;
                    }
                    let retreat_target =
                        nearest_owned_planet(game_data, tf.empire, coords).unwrap_or(coords);
                    post_round_retreats.push((tf.empire, retreat_target));
                }
            }

            for empire in free_holds_to_consume {
                if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                    task_force.free_hold_used = true;
                }
            }

            let mut any_post_round_withdrawal = false;
            for (empire, retreat_target) in post_round_retreats {
                if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                    task_force.withdrew_under_roe = true;
                    apply_roe_retreat_to_task_force(
                        game_data,
                        &task_force.fleet_indices,
                        retreat_target,
                    );
                    any_post_round_withdrawal = true;
                }
            }
            combat_occurred |= any_post_round_withdrawal;

            let remaining_active_after_retreats = task_forces
                .iter()
                .filter(|tf| {
                    tf.state.has_units() && tf.state.total_combat_as() > 0 && !tf.withdrew_under_roe
                })
                .count();
            if remaining_active_after_retreats < 2 {
                resolved_within_guardrail = true;
                break;
            }
        }

        if !resolved_within_guardrail {
            return Err(format!(
                "combat at ({},{}) exceeded {} rounds",
                coords[0], coords[1], COMBAT_GUARDRAIL_MAX_ROUNDS
            )
            .into());
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
        let dominant_empire = dominant_empire_after_battle(&task_forces, winner_empire);

        let mut destroyed_starbases_by_empire: HashMap<u8, Vec<u8>> = HashMap::new();
        for tf in &task_forces {
            if let Some(before) = original_states.get(&tf.empire) {
                distribute_fleet_losses(game_data, &tf.fleet_indices, before, &tf.state);
                let destroyed_starbases =
                    before.counts[IDX_SB].saturating_sub(tf.state.counts[IDX_SB]);
                if destroyed_starbases > 0 {
                    destroyed_starbases_by_empire.insert(
                        tf.empire,
                        remove_destroyed_starbases(
                            game_data,
                            coords,
                            tf.empire,
                            destroyed_starbases,
                        ),
                    );
                }
            }
        }

        let pre_retreat_orders: HashMap<usize, Order> = task_forces
            .iter()
            .flat_map(|tf| tf.fleet_indices.iter().copied())
            .map(|idx| (idx, game_data.fleets.records[idx].standing_order_kind()))
            .collect();

        for tf in &task_forces {
            if Some(tf.empire) != dominant_empire && !tf.withdrew_under_roe {
                retreat_task_force(game_data, tf);
                for &idx in &tf.fleet_indices {
                    if let Some(kind) =
                        mission_kind_for_order(pre_retreat_orders.get(&idx).copied())
                    {
                        let fleet = &game_data.fleets.records[idx];
                        events.mission_events.push(MissionEvent {
                            fleet_idx: idx,
                            owner_empire_raw: fleet.owner_empire_raw(),
                            kind,
                            outcome: MissionOutcome::Aborted,
                            planet_idx: None,
                            location_coords: Some(coords),
                            target_coords: Some(fleet.standing_order_target_coords_raw()),
                            stardate_week: None,
                        });
                    }
                }
            }
        }
        abort_invalid_dominant_missions_after_battle(
            game_data,
            &mut events,
            &task_forces,
            dominant_empire,
            &pre_retreat_orders,
            coords,
        );
        for tf in &task_forces {
            if !tf.withdrew_under_roe {
                continue;
            }
            if !tf_has_any_units(tf) {
                clear_empty_withdrawn_fleets(game_data, &tf.fleet_indices);
                continue;
            }
            let Some(before) = original_states.get(&tf.empire) else {
                continue;
            };
            let losses = ship_losses_from_states(before, &tf.state);
            let mut enemy_before = FleetCombatState::default();
            let mut enemy_after = FleetCombatState::default();
            for other in &task_forces {
                if other.empire == tf.empire {
                    continue;
                }
                if let Some(orig) = original_states.get(&other.empire) {
                    for idx in 0..7 {
                        enemy_before.counts[idx] += orig.counts[idx];
                        enemy_after.counts[idx] += other.state.counts[idx];
                    }
                }
            }
            let enemy_losses_inflicted = ship_losses_from_states(&enemy_before, &enemy_after);
            let target_empire_raw = task_forces
                .iter()
                .filter(|other| other.empire != tf.empire)
                .max_by_key(|other| other.state.total_combat_as())
                .map(|other| other.empire)
                .unwrap_or(0);
            let target_fleet_number = task_forces
                .iter()
                .find(|other| other.empire == target_empire_raw)
                .and_then(|other| single_named_fleet_number(game_data, &other.fleet_indices));
            for &idx in &tf.fleet_indices {
                events
                    .encounter_disposition_events
                    .push(EncounterDispositionEvent::Retreated {
                        fleet_idx: idx,
                        owner_empire_raw: tf.empire,
                        mission: mission_kind_for_order(pre_encounter_orders.get(&idx).copied()),
                        coords,
                        target_empire_raw,
                        target_fleet_number,
                        enemy_initial: ship_counts_from_state(&enemy_before),
                        retreat_target_coords: game_data.fleets.records[idx]
                            .standing_order_target_coords_raw(),
                        losses_sustained: losses,
                        enemy_losses_inflicted,
                        reason: EncounterDispositionReason::RoeWithdrawal,
                        stardate_week: None,
                    });
            }
        }
        for empire in participants {
            if !combat_occurred {
                continue;
            }
            let Some(before) = original_states.get(&empire) else {
                continue;
            };
            let Some(after_tf) = task_forces.iter().find(|tf| tf.empire == empire) else {
                continue;
            };
            let after = &after_tf.state;
            let friendly_losses = ship_losses_from_states(before, after);
            let mut enemy_before = FleetCombatState::default();
            let mut enemy_after = FleetCombatState::default();
            for tf in &task_forces {
                if tf.empire == empire {
                    continue;
                }
                if let Some(orig) = original_states.get(&tf.empire) {
                    for idx in 0..7 {
                        enemy_before.counts[idx] += orig.counts[idx];
                        enemy_after.counts[idx] += tf.state.counts[idx];
                    }
                }
            }
            let enemy_losses = ship_losses_from_states(&enemy_before, &enemy_after);
            let enemy_loaded_armies_initial = task_forces
                .iter()
                .filter(|tf| tf.empire != empire)
                .map(|tf| original_loaded_armies.get(&tf.empire).copied().unwrap_or(0))
                .sum();
            let enemy_empires_raw = task_forces
                .iter()
                .filter(|tf| {
                    tf.empire != empire
                        && original_states
                            .get(&tf.empire)
                            .is_some_and(FleetCombatState::has_units)
                })
                .map(|tf| tf.empire)
                .collect();
            let reporting_fleet_idx =
                preferred_reporting_fleet_index(game_data, &after_tf.fleet_indices);
            let reporting_fleet_number = reporting_fleet_idx
                .map(|idx| game_data.fleets.records[idx].local_slot_word_raw() as u8)
                .filter(|fleet_number| *fleet_number != 0);
            let reporting_mission = reporting_fleet_idx
                .and_then(|idx| mission_kind_for_order(pre_encounter_orders.get(&idx).copied()));
            let primary_enemy_fleet_number = task_forces
                .iter()
                .filter(|tf| tf.empire != empire && tf.state.has_units())
                .max_by_key(|tf| tf.state.total_combat_as())
                .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
            events.fleet_battle_events.push(FleetBattleEvent {
                reporting_empire_raw: empire,
                reporting_fleet_number,
                reporting_mission,
                perspective: report_perspective_for_mission(reporting_mission, after_tf.role),
                coords,
                enemy_empires_raw,
                primary_enemy_fleet_number,
                held_field: dominant_empire == Some(empire),
                friendly_initial: ship_counts_from_state(before),
                friendly_loaded_armies_initial: original_loaded_armies
                    .get(&empire)
                    .copied()
                    .unwrap_or(0),
                friendly_losses,
                enemy_initial: ship_counts_from_state(&enemy_before),
                enemy_initial_starbases: enemy_before.counts[IDX_SB],
                enemy_loaded_armies_initial,
                enemy_losses,
                stardate_week: None,
            });

            if !tf_has_any_units(after_tf) && !after_tf.fleet_indices.is_empty() {
                let fleet_number = reporting_fleet_number.unwrap_or(0);
                let primary_enemy_empire_raw = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .map(|tf| tf.empire);
                let primary_enemy_fleet_number = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
                events.fleet_destroyed_events.push(FleetDestroyedEvent {
                    reporting_empire_raw: empire,
                    fleet_number,
                    coords,
                    was_intercepting: matches!(after_tf.role, BattleRole::Attacker),
                    friendly_initial: ship_counts_from_state(before),
                    friendly_loaded_armies_initial: original_loaded_armies
                        .get(&empire)
                        .copied()
                        .unwrap_or(0),
                    enemy_initial: ship_counts_from_state(&enemy_before),
                    enemy_loaded_armies_initial,
                    enemy_losses,
                    primary_enemy_empire_raw,
                    primary_enemy_fleet_number,
                    stardate_week: None,
                });
            }

            let destroyed_starbases = before.counts[IDX_SB].saturating_sub(after.counts[IDX_SB]);
            if destroyed_starbases > 0 {
                let primary_enemy_empire_raw = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .map(|tf| tf.empire);
                let primary_enemy_fleet_number = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
                if let Some(lost_ids) = destroyed_starbases_by_empire.get(&empire) {
                    for &starbase_id in lost_ids {
                        events
                            .starbase_destroyed_events
                            .push(StarbaseDestroyedEvent {
                                reporting_empire_raw: empire,
                                starbase_id,
                                coords,
                                enemy_initial: ship_counts_from_state(&enemy_before),
                                enemy_losses,
                                primary_enemy_empire_raw,
                                primary_enemy_fleet_number,
                                stardate_week: None,
                            });
                    }
                }
            }
        }
    }

    Ok(events)
}
