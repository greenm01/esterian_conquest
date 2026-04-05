//! Maintenance logic for ECMAINT.EXE mechanics.

mod campaign;
mod canonicalize;
mod combat;
mod economics;
pub mod gate;
mod merging;
mod movement;
pub mod recovery;
mod results;
mod retarget;
mod sanitize;
pub mod timing;

pub use nc_data::maintenance_types::*;
pub use results::{build_results_dat, build_results_report_blocks};

use crate::VisibleHazardIntel;
use nc_data::{CoreGameData, FleetRecord, Order};

/// Event produced when a fleet completes a ColonizeWorld order.
#[derive(Debug)]
struct ColonizationEvent {
    /// Fleet index in FLEETS.DAT that arrived.
    fleet_idx: usize,
    /// Target coordinates where colonization occurred.
    coords: [u8; 2],
    /// Empire that colonized (owner_empire_raw from fleet record).
    owner_empire: u8,
}

#[derive(Debug, Default)]
struct MovementEvents {
    colonization_events: Vec<ColonizationEvent>,
    planet_intel_events: Vec<PlanetIntelEvent>,
    pending_observation_events: Vec<PendingObservationEvent>,
    mission_events: Vec<MissionEvent>,
    salvage_events: Vec<SalvageResolvedEvent>,
    diplomatic_escalation_events: Vec<DiplomaticEscalationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingObservationEvent {
    fleet_idx: usize,
    owner_empire_raw: u8,
    kind: Mission,
    outcome: MissionOutcome,
    planet_idx: Option<usize>,
    location_coords: [u8; 2],
    target_coords: [u8; 2],
    intel_event: Option<PlanetIntelEvent>,
}

/// Run a single turn of maintenance processing.
///
/// This is the Rust implementation of ECMAINT.EXE behavior.
/// Currently implements:
/// - Year advancement (+1 per turn)
/// - Fleet movement (basic move orders)
/// - Planet colonization (ColonizeWorld fleet arrivals)
/// - Fleet co-location merging (friendly fleets at same coords merge into one)
///
/// Note: DATABASE.DAT regeneration is handled separately in the CLI layer
/// since it's not part of CoreGameData.
///
/// # Arguments
/// * `game_data` - Mutable reference to the game state to modify
///
/// # Returns
/// Ok(()) on success, or an error if maintenance fails
pub fn run_maintenance_turn(
    game_data: &mut CoreGameData,
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    run_maintenance_turn_with_context_and_seed(game_data, 0, &[], &[])
}

pub fn run_maintenance_turn_with_visible_hazards(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    run_maintenance_turn_with_context_and_seed(game_data, 0, visible_hazards_by_empire, &[])
}

pub fn run_maintenance_turn_with_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    run_maintenance_turn_with_context_and_seed(game_data, campaign_seed, &[], &[])
}

pub fn run_maintenance_turn_with_visible_hazards_and_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    run_maintenance_turn_with_context_and_seed(
        game_data,
        campaign_seed,
        visible_hazards_by_empire,
        &[],
    )
}

pub fn run_maintenance_turn_with_context(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    run_maintenance_turn_with_context_and_seed(
        game_data,
        0,
        visible_hazards_by_empire,
        diplomacy_overrides,
    )
}

pub fn run_maintenance_turn_with_context_and_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    // CONQUEST.DAT 0x0c/0x3d accumulation trigger — snapshot BEFORE processing.
    // ECMAINT increments production total (0x0c += 100) and turn counter (0x3d += 1)
    // only when raw[0x0c] == 0x64 at the start of the turn.
    //
    // State machine for 0x0c:
    //   0x00 (fresh/econ start) → first tick writes non-active prod words → becomes 0x64
    //   0x64 (initialized)      → next tick with should_accumulate=true increments → 0xc8
    //   0xc8+ (accumulated)     → no further changes
    //
    // Confirmed across all scenarios:
    // - fleet/build/move:  0x0c pre=0x64, post=0x64 (no change; stays at 0x64 each tick but
    //   the condition raw[0x0c]==0x64 IS true ... so why don't they accumulate?)
    //
    // Actually the rule is: accumulate when raw[0x0c]==0x64 AND (any fleet in-transit OR
    // any active/rogue player). Without one of those two game-activity signals, the
    // fleet/build/move scenarios don't accumulate even though 0x0c==0x64.
    //
    // Confirmed:
    // - bombard tick 2: 0x0c=0x64, fleet 2 in-transit → accumulates to 0xc8 ✓
    // - invade/econ tick 2: 0x0c=0x64, active player present → accumulates to 0xc8 ✓
    // - fleet/build/move tick N: 0x0c=0x64, no in-transit, no active → no accumulation ✓
    // Snapshot pre-turn state needed for post-movement processing.
    // These must be captured BEFORE any mutations so they reflect start-of-turn conditions.

    // CONQUEST.DAT 0x0c/0x3d accumulation: accumulate when raw[0x0c]==0x64 at start of turn
    // AND at least one fleet is in-transit (raw[0x19]==0x80) or a player is active/rogue.
    let any_fleet_in_transit = game_data
        .fleets
        .records
        .iter()
        .any(|f| f.transit_ready_flag_raw() == 0x80);
    let any_active_player = game_data
        .player
        .records
        .iter()
        .any(|p| p.is_active_or_rogue_player());
    let should_accumulate_conquest = game_data.conquest.inactive_production_slot_raw(0)
        == Some(0x0064)
        && (any_fleet_in_transit || any_active_player);

    // Bombardment execution: a BombardWorld fleet that had raw[0x19]==0x80 at start of turn
    // (i.e. it arrived last turn) executes this turn. Fleets that arrive this turn
    // will execute next turn. Snapshot indices now, before movement mutates raw[0x19].
    let bombard_ready: Vec<usize> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if f.transit_ready_flag_raw() == 0x80
                && matches!(
                    Order::from_raw(f.standing_order_code_raw()),
                    Order::BombardWorld
                )
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    let initial_campaign_outlook = game_data.campaign_outlook();
    let initial_campaign_outcome = game_data.campaign_outcome();
    let fleet_number_by_id: std::collections::HashMap<u8, u8> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| (fleet.fleet_id(), fleet.local_slot_word_raw() as u8))
        .collect();

    // InvadeWorld execution: a InvadeWorld fleet that had raw[0x19]==0x80 at start of turn
    // executes this turn. Snapshot indices now, before movement mutates raw[0x19].
    let invade_ready: Vec<usize> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if f.transit_ready_flag_raw() == 0x80
                && matches!(
                    Order::from_raw(f.standing_order_code_raw()),
                    Order::InvadeWorld
                )
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    let blitz_ready: Vec<usize> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if f.transit_ready_flag_raw() == 0x80
                && matches!(
                    Order::from_raw(f.standing_order_code_raw()),
                    Order::BlitzWorld
                )
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    // Advance game year by 1
    let current_year = game_data.conquest.game_year();
    let new_year = current_year + 1;
    game_data.conquest.set_game_year(new_year);

    // Merge co-located friendly fleets BEFORE movement.
    // Confirmed from econ fixture: the Bombard fleet (at 16,13 pre-move) is
    // included in the merge even though it moves to (15,13) this turn.
    // The merge runs before movement resolution, absorbing all same-position
    // fleets for flagged players (PLAYER raw[0x00]==0xff).
    let mut merge_events = merging::process_fleet_merging(game_data)?;

    let invalid_player_state_events = sanitize_invalid_player_inputs(game_data);

    let mut mission_retarget_events = retarget::refresh_seek_home_targets(game_data);
    mission_retarget_events.extend(retarget::refresh_join_host_targets(game_data));
    mission_retarget_events.extend(retarget::refresh_guard_starbase_targets(game_data));

    // Autopilot fleet recall: idle fleets in deep space get SeekHome before
    // movement so they can start heading home this turn.
    economics::process_autopilot_fleet_orders(game_data)?;

    // Process fleet orders; collect side-effect events
    let mut movement_events =
        movement::process_fleet_movement(game_data, visible_hazards_by_empire)?;
    merge_events.extend(merging::process_mission_fleet_merging(game_data)?);

    // Detect and resolve fleet battles: when hostile fleets co-locate after movement,
    // surviving fleets get SeekHome orders (confirmed from fleet-battle oracle).
    // This runs after movement so all fleet positions are final for this turn.
    let fleet_battle_phase_events =
        combat::process_fleet_battles(game_data, campaign_seed, diplomacy_overrides)?;

    // Apply colonization results to PLANETS.DAT and PLAYER.DAT
    let colonization_events =
        merging::process_colonizations(game_data, &movement_events.colonization_events)?;
    let newly_colonized_planets = colonization_events
        .iter()
        .filter_map(|event| match *event {
            ColonizationResolvedEvent::Succeeded { planet_idx, .. } => Some(planet_idx),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Process build queues and track which planets had activity
    let planets_with_builds = economics::process_build_completion(game_data)?;

    // Process planet economic updates for planets that had builds
    economics::process_planet_economics(
        game_data,
        &planets_with_builds,
        &newly_colonized_planets,
    )?;

    // Run autopilot / rogue AI planet economics.
    // Updates factories, armies, and raw[0x0E] for rogue and autopilot-on players.
    economics::process_autopilot_ai(game_data)?;

    // Recompute per-player planet count and production score from PLANETS.DAT.
    // ECMAINT recalculates these from scratch every turn, not as incremental deltas.
    economics::recompute_player_planet_stats(game_data);

    // A player who has lost all planets and has no realistic recovery path
    // falls into civil disorder. This preserves the empire slot and matches
    // the observed "In Civil Disorder" state already used by classic data.
    let civil_disorder_events = campaign::apply_campaign_state_transitions(game_data);
    let fleet_defection_events =
        campaign::apply_civil_disorder_fleet_defections(game_data, &civil_disorder_events)?;
    let campaign_outlook_events = campaign::detect_campaign_outlook_events(
        initial_campaign_outlook,
        game_data.campaign_outlook(),
        &civil_disorder_events,
    );
    let campaign_outcome_events = campaign::detect_campaign_outcome_events(
        initial_campaign_outcome,
        game_data.campaign_outcome(),
    );

    // Update PLAYER.DAT raw[0x46]: set to 0x01 for any player with starbase_count > 0.
    // Confirmed from starbase fixture: player 0 (starbase_count=1) gets raw[0x46]=0x01 after maint.
    campaign::update_player_starbase_flag(game_data);

    // Resolve bombardment for fleets that were already-arrived (raw[0x19]==0x80) at turn start.
    // Confirmed: bombardment executes on the tick AFTER transit-arrival, not same tick.
    let assault_events = combat::process_planetary_assaults(
        game_data,
        campaign_seed,
        &bombard_ready,
        &invade_ready,
        &blitz_ready,
    )?;

    let join_host_events =
        merging::process_join_host_updates(game_data, &merge_events, &fleet_number_by_id);

    // Normalize CONQUEST.DAT header fields
    campaign::process_conquest_header(game_data, should_accumulate_conquest)?;

    finalize_pending_observation_events(
        game_data,
        &mut movement_events,
        &fleet_battle_phase_events.mission_events,
    );

    restore_scout_orders_and_generate_on_station_observations(game_data, &mut movement_events);

    let mut mission_events = movement_events.mission_events;
    mission_events.extend(fleet_battle_phase_events.mission_events);
    mission_events.extend(assault_events.mission_events);
    for colonization in &colonization_events {
        match *colonization {
            ColonizationResolvedEvent::Succeeded {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Succeeded,
                planet_idx: Some(planet_idx),
                location_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                stardate_week: None,
            }),
            ColonizationResolvedEvent::BlockedByOwner {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Failed,
                planet_idx: Some(planet_idx),
                location_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                stardate_week: None,
            }),
            ColonizationResolvedEvent::Aborted {
                fleet_idx,
                colonizer_empire_raw,
                coords,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Aborted,
                planet_idx: game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords),
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            }),
        }
    }

    let blocked_colonization_intel_events = colonization_events
        .iter()
        .filter_map(|event| match *event {
            ColonizationResolvedEvent::BlockedByOwner {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                ..
            } => Some(PlanetIntelEvent {
                planet_idx,
                viewer_empire_raw: colonizer_empire_raw,
                source: nc_data::PlanetIntelSource::ColonizeBlockedByOwner,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: None,
                stardate_week: None,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    let events = MaintenanceEvents {
        bombard_events: assault_events.bombard_events,
        planet_intel_events: {
            let mut events = movement_events.planet_intel_events;
            events.extend(fleet_battle_phase_events.planet_intel_events);
            events.extend(assault_events.planet_intel_events);
            events.extend(blocked_colonization_intel_events);
            events
        },
        ownership_change_events: assault_events.ownership_change_events,
        fleet_battle_events: fleet_battle_phase_events.fleet_battle_events,
        fleet_destroyed_events: fleet_battle_phase_events.fleet_destroyed_events,
        starbase_destroyed_events: fleet_battle_phase_events.starbase_destroyed_events,
        assault_report_events: assault_events.assault_report_events,
        scout_contact_events: fleet_battle_phase_events.scout_contact_events,
        encounter_disposition_events: fleet_battle_phase_events.encounter_disposition_events,
        invalid_player_state_events,
        fleet_merge_events: merge_events,
        join_host_events,
        mission_retarget_events,
        colonization_events,
        mission_events,
        salvage_events: movement_events.salvage_events,
        diplomatic_escalation_events: movement_events.diplomatic_escalation_events,
        civil_disorder_events,
        campaign_outlook_events,
        campaign_outcome_events,
        fleet_defection_events,
    };

    campaign::apply_stored_diplomatic_escalations(game_data, &events)?;

    // Assign stardate week values and sort event vectors chronologically.
    let mut events = events;
    canonicalize::canonicalize_events(&mut events, game_data);

    Ok(events)
}

fn fleet_has_presence(fleet: &FleetRecord) -> bool {
    fleet.has_any_force()
}

/// Restore scout orders for fleets that arrived this turn (they were set to
/// HoldPosition by the stepper to avoid interfering with combat resolution).
/// Also generate per-turn observation reports for scout fleets already on station
/// from a previous turn.
fn restore_scout_orders_and_generate_on_station_observations(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
) {
    // Collect scout arrivals: fleet_idx → mission kind.
    let scout_arrivals: std::collections::HashMap<usize, Mission> = movement_events
        .mission_events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                Mission::ScoutSector | Mission::ScoutSolarSystem
            ) && matches!(
                e.outcome,
                MissionOutcome::Succeeded | MissionOutcome::Arrived
            )
        })
        .map(|e| (e.fleet_idx, e.kind))
        .collect();

    // Restore scout orders for fleets that just arrived and survived combat.
    for (&fleet_idx, &kind) in &scout_arrivals {
        let Some(fleet) = game_data.fleets.records.get_mut(fleet_idx) else {
            continue;
        };
        if !fleet.has_any_force() {
            continue;
        }
        let order = match kind {
            Mission::ScoutSector => Order::ScoutSector,
            Mission::ScoutSolarSystem => Order::ScoutSolarSystem,
            _ => continue,
        };
        fleet.set_standing_order_kind(order);
    }

    // Generate per-turn observations for scouts already on station from previous turns.
    for (fleet_idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if !fleet.has_any_force() {
            continue;
        }
        if scout_arrivals.contains_key(&fleet_idx) {
            continue;
        }
        let order = fleet.standing_order_kind();
        let coords = fleet.current_location_coords_raw();
        let target = fleet.standing_order_target_coords_raw();
        if coords != target {
            continue;
        }
        let owner_empire_raw = fleet.owner_empire_raw();
        match order {
            Order::ScoutSector => {
                movement_events.mission_events.push(MissionEvent {
                    fleet_idx,
                    owner_empire_raw,
                    kind: Mission::ScoutSector,
                    outcome: MissionOutcome::Succeeded,
                    planet_idx: None,
                    location_coords: Some(coords),
                    target_coords: Some(coords),
                    stardate_week: None,
                });
            }
            Order::ScoutSolarSystem => {
                let planet_idx = game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords);
                if let Some(planet_idx) = planet_idx {
                    movement_events.planet_intel_events.push(PlanetIntelEvent {
                        planet_idx,
                        viewer_empire_raw: owner_empire_raw,
                        source: nc_data::PlanetIntelSource::ScoutSolarSystem,
                        source_fleet_idx: Some(fleet_idx),
                        observed_snapshot: nc_data::build_runtime_planet_intel_snapshot(
                            game_data,
                            owner_empire_raw,
                            game_data.conquest.game_year(),
                            planet_idx,
                            nc_data::PlanetIntelSource::ScoutSolarSystem,
                        ),
                        stardate_week: None,
                    });
                }
                movement_events.mission_events.push(MissionEvent {
                    fleet_idx,
                    owner_empire_raw,
                    kind: Mission::ScoutSolarSystem,
                    outcome: MissionOutcome::Succeeded,
                    planet_idx,
                    location_coords: Some(coords),
                    target_coords: Some(coords),
                    stardate_week: None,
                });
            }
            _ => {}
        }
    }
}

fn finalize_pending_observation_events(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    combat_mission_events: &[MissionEvent],
) {
    let pending_events = std::mem::take(&mut movement_events.pending_observation_events);
    for pending in pending_events {
        let Some(fleet) = game_data.fleets.records.get_mut(pending.fleet_idx) else {
            continue;
        };
        if !fleet_has_presence(fleet) {
            continue;
        }
        let mission_aborted = combat_mission_events.iter().any(|event| {
            event.fleet_idx == pending.fleet_idx
                && event.owner_empire_raw == pending.owner_empire_raw
                && event.kind == pending.kind
                && event.outcome == MissionOutcome::Aborted
        });
        if mission_aborted {
            continue;
        }
        if pending.kind == Mission::ViewWorld && pending.outcome == MissionOutcome::Succeeded {
            movement::set_view_world_completion_hold(fleet);
        }
        if let Some(intel_event) = pending.intel_event {
            movement_events.planet_intel_events.push(intel_event);
        }
        movement_events.mission_events.push(MissionEvent {
            fleet_idx: pending.fleet_idx,
            owner_empire_raw: pending.owner_empire_raw,
            kind: pending.kind,
            outcome: pending.outcome,
            planet_idx: pending.planet_idx,
            location_coords: Some(pending.location_coords),
            target_coords: Some(pending.target_coords),
            stardate_week: None,
        });
    }
}

fn apply_fleet_removal_remap(game_data: &mut CoreGameData, to_remove: &[bool]) {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 || to_remove.len() != fleet_count {
        return;
    }

    let pre_removal_owner: Vec<u8> = game_data
        .fleets
        .records
        .iter()
        .map(|f| f.owner_empire_raw())
        .collect();
    let pre_removal_fleet_id: Vec<u16> = game_data
        .fleets
        .records
        .iter()
        .map(|f| f.fleet_id_word_raw())
        .collect();

    let removed_before: Vec<u16> = {
        let mut count = 0u16;
        (0..fleet_count)
            .map(|i| {
                let current = count;
                if to_remove[i] {
                    count = count.saturating_add(1);
                }
                current
            })
            .collect()
    };

    let remap_id = |old_id: u16| -> u16 {
        if old_id == 0 {
            return 0;
        }
        let orig_idx = (old_id as usize).saturating_sub(1);
        if orig_idx >= fleet_count || to_remove[orig_idx] {
            0
        } else {
            old_id.saturating_sub(removed_before[orig_idx])
        }
    };

    // Surviving local fleet numbers stay unchanged. Only global linkage IDs compress.
    game_data.fleets.records = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter(|(i, _)| !to_remove[*i])
        .map(|(_, fleet)| {
            let mut f = fleet.clone();
            f.set_fleet_id_word_raw(remap_id(fleet.fleet_id_word_raw()));
            f.set_next_fleet_link_word_raw(remap_id(fleet.next_fleet_link_word_raw()));
            f.set_previous_fleet_id(remap_id(u16::from(fleet.previous_fleet_id())) as u8);
            f
        })
        .collect();

    for player_idx in 0..game_data.player.records.len() {
        let owner_raw = (player_idx + 1) as u8;
        let first_id = game_data.player.records[player_idx].fleet_chain_head_raw() as u8;
        let last_id = game_data.player.records[player_idx].fleet_chain_tail_raw() as u8;
        let new_first = remap_id(u16::from(first_id)) as u8;
        let new_last = remap_id(u16::from(last_id)) as u8;
        game_data.player.records[player_idx].set_fleet_chain_head_raw(u16::from(new_first));
        game_data.player.records[player_idx].set_fleet_chain_tail_raw(
            if new_last == 0 && new_first != 0 {
                let mut max_new_id: u8 = new_first;
                for orig_idx in 0..fleet_count {
                    if pre_removal_owner[orig_idx] == owner_raw && !to_remove[orig_idx] {
                        let mapped = remap_id(pre_removal_fleet_id[orig_idx]) as u8;
                        if mapped > max_new_id {
                            max_new_id = mapped;
                        }
                    }
                }
                max_new_id
            } else {
                new_last
            }
            .into(),
        );
    }
}

fn remove_selected_fleets(game_data: &mut CoreGameData, to_remove: &[bool]) {
    apply_fleet_removal_remap(game_data, to_remove);
}

fn sanitize_invalid_player_inputs(game_data: &mut CoreGameData) -> Vec<InvalidPlayerStateEvent> {
    sanitize::sanitize_invalid_player_inputs(game_data)
}

pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    economics::process_autopilot_ai(game_data)
}

pub fn run_maintenance_turns(
    game_data: &mut CoreGameData,
    turns: u16,
) -> Result<u16, Box<dyn std::error::Error>> {
    for _ in 0..turns {
        run_maintenance_turn_with_seed(game_data, 0)?;
    }
    Ok(game_data.conquest.game_year())
}
