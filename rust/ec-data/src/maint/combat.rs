//! Canonical EC combat resolution.
//!
//! The structure here owes an explicit debt to *Empire of the Sun*: both sides
//! compute their blows from the same moment in time, and only then does the
//! board reckon with the cost. That simultaneous exchange fits EC's manuals
//! better than file-order skirmishes, while staying deterministic enough for
//! Rust maintenance and classic save compatibility.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{CoreGameData, DiplomaticRelation, FleetOrderValidationError, Order};

use super::{
    AssaultReportEvent, BombardEvent, ContactReportSource, DiplomacyOverride,
    EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent,
    FleetBattlePerspective, FleetDestroyedEvent, Mission, MissionEvent, MissionOutcome,
    PlanetIntelEvent, PlanetIntelSource, PlanetOwnershipChangeEvent, ScoutContactEvent, ShipLosses,
    StarbaseDestroyedEvent,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncounterContext {
    DeepSpaceTransit,
    SectorPatrol,
    SystemEntry,
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
    withdrew_under_roe: bool,
    engaged_in_battle: bool,
    /// Guards/blockades get one free hold when ROE threshold fails (per spec).
    /// Set to true once this free hold has been used.
    free_hold_used: bool,
}

/// Tracks ROE decline scenarios that may trigger pursuit fire.
/// When a fleet declines ROE before combat, and the target is a
/// guard/blockade fleet, the withdrawing fleet suffers one pursuit
/// fire exchange before escaping (per spec).
enum RoeDeclineOutcome {
    /// Clean escape - no guard/blockade to intercept.
    NoEngagement { empire: u8, target_empire: u8 },
    /// Pursuit fire - guard/blockade intercepts and fires one exchange.
    PursuitFire {
        withdrawer_empire: u8,
        pursuer_empire: u8,
        withdrawer_hits: u32,
        pursuer_hits: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostilityReason {
    DeclaredEnemy,
    DefendedSystemEntry,
    PatrolContact,
}

fn hostility_requires_forced_engagement(reason: HostilityReason) -> bool {
    matches!(reason, HostilityReason::DefendedSystemEntry)
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

fn fleet_state_from_records(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
    starbases: u32,
) -> FleetCombatState {
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

/// Pursuit fire CER: per spec, the pursuer fires at CER 0.50 flat.
/// No modifiers apply—this is a hasty shot while giving chase.
fn pursuit_cer_percent() -> u32 {
    50
}

fn hits_from(as_total: u32, cer_percent: u32) -> u32 {
    (as_total.saturating_mul(cer_percent)).div_ceil(100)
}

fn apply_hits_to_fleet(state: &mut FleetCombatState, mut hits: u32) {
    // Phase 1: Screening - remove fresh steps from all classes in priority order
    // No hulls are destroyed during screening. This represents escorts
    // absorbing initial combat shock before heavier assets take damage.
    for idx in fleet_combat_priority() {
        if hits == 0 {
            break;
        }
        let fresh_loss = hits.min(state.fresh[idx]);
        state.fresh[idx] -= fresh_loss;
        hits -= fresh_loss;
    }

    // Phase 2: Kill - once all fresh steps are gone, destroy hulls in priority order
    // Remaining hits destroy units and reduce on-disk counts.
    for idx in fleet_combat_priority() {
        if hits == 0 {
            break;
        }
        let destroyed = hits.min(state.counts[idx]);
        state.counts[idx] -= destroyed;
        hits -= destroyed;
    }
}

fn task_force_role(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: &[usize],
) -> BattleRole {
    if let Some(planet) = game_data
        .planets
        .records
        .iter()
        .find(|p| p.coords_raw() == coords)
    {
        if planet.owner_empire_slot_raw() == empire {
            return BattleRole::IncumbentDefender;
        }
    }
    let guarding = fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets.records[idx].standing_order_kind(),
            Order::GuardStarbase | Order::GuardBlockadeWorld
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

fn has_anchored_guard_order(game_data: &CoreGameData, fleet_indices: &[usize]) -> bool {
    fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets.records[idx].standing_order_kind(),
            Order::GuardStarbase | Order::GuardBlockadeWorld
        )
    })
}

fn has_patrol_order(game_data: &CoreGameData, fleet_indices: &[usize]) -> bool {
    fleet_indices
        .iter()
        .any(|&idx| game_data.fleets.records[idx].standing_order_kind() == Order::PatrolSector)
}

fn fleet_is_at_system_context(fleet: &crate::FleetRecord) -> bool {
    let coords = fleet.current_location_coords_raw();
    let target = fleet.standing_order_target_coords_raw();
    if coords != target {
        return false;
    }
    matches!(
        fleet.standing_order_kind(),
        Order::SeekHome
            | Order::GuardStarbase
            | Order::GuardBlockadeWorld
            | Order::BombardWorld
            | Order::InvadeWorld
            | Order::BlitzWorld
            | Order::ViewWorld
            | Order::ScoutSolarSystem
            | Order::Salvage
    )
}

fn task_force_encounter_context(
    game_data: &CoreGameData,
    task_force: &TaskForce,
) -> EncounterContext {
    let has_system_context_fleet = task_force
        .fleet_indices
        .iter()
        .any(|&idx| fleet_is_at_system_context(&game_data.fleets.records[idx]));

    if has_system_context_fleet || task_force.state.counts[IDX_SB] > 0 {
        EncounterContext::SystemEntry
    } else if has_patrol_order(game_data, &task_force.fleet_indices) {
        EncounterContext::SectorPatrol
    } else {
        EncounterContext::DeepSpaceTransit
    }
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

    let left_context = task_force_encounter_context(game_data, left);
    let right_context = task_force_encounter_context(game_data, right);

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
                // If one side has a guard order and the other is in transit,
                // they are not hostile unless the transit fleet is in assault posture
                // (Invade/Bombard/Blitz) which forces engagement
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
                        // Transit fleet is attacking - guard defends normally
                        Some(HostilityReason::DefendedSystemEntry)
                    } else {
                        // Transit fleet is just passing through - guard stays at station
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

fn push_contact_event_for_task_force(
    events: &mut FleetBattlePhaseEvents,
    game_data: &CoreGameData,
    coords: [u8; 2],
    task_force: &TaskForce,
    target_task_force: &TaskForce,
) {
    let (small_vessels, medium_vessels, large_vessels) =
        vessel_size_summary(&target_task_force.state);
    let target_fleet_id = single_named_fleet_id(game_data, &target_task_force.fleet_indices);

    for &idx in &task_force.fleet_indices {
        let fleet = &game_data.fleets.records[idx];
        let source = contact_reporting_kind(fleet.standing_order_kind())
            .map(ContactReportSource::FleetMission)
            .unwrap_or(ContactReportSource::Fleet(fleet.fleet_id()));
        events.scout_contact_events.push(ScoutContactEvent {
            viewer_empire_raw: fleet.owner_empire_raw(),
            source,
            reporting_fleet_id: Some(fleet.fleet_id()),
            coords,
            target_empire_raw: target_task_force.empire,
            target_fleet_id,
            small_vessels,
            medium_vessels,
            large_vessels,
            stardate_week: None,
        });
    }

    for base in game_data.bases.records.iter().filter(|base| {
        base.coords_raw() == coords
            && base.owner_empire_raw() == task_force.empire
            && base.active_flag_raw() != 0
    }) {
        events.scout_contact_events.push(ScoutContactEvent {
            viewer_empire_raw: task_force.empire,
            source: ContactReportSource::Starbase(base.base_id_raw()),
            reporting_fleet_id: None,
            coords,
            target_empire_raw: target_task_force.empire,
            target_fleet_id,
            small_vessels,
            medium_vessels,
            large_vessels,
            stardate_week: None,
        });
    }
}

fn single_named_fleet_id(game_data: &CoreGameData, fleet_indices: &[usize]) -> Option<u8> {
    let named_fleets = fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.records.get(*idx))
        .filter(|fleet| {
            fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0
        })
        .map(|fleet| fleet.fleet_id())
        .filter(|fleet_id| *fleet_id != 0)
        .collect::<Vec<_>>();

    if named_fleets.len() == 1 {
        Some(named_fleets[0])
    } else {
        None
    }
}

fn preferred_reporting_fleet_id(game_data: &CoreGameData, fleet_indices: &[usize]) -> Option<u8> {
    fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.records.get(*idx))
        .map(|fleet| fleet.fleet_id())
        .filter(|fleet_id| *fleet_id != 0)
        .min()
}

fn preferred_reporting_fleet_index(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> Option<usize> {
    fleet_indices
        .iter()
        .copied()
        .filter(|idx| game_data.fleets.records.get(*idx).is_some())
        .filter(|idx| game_data.fleets.records[*idx].fleet_id() != 0)
        .min_by_key(|idx| game_data.fleets.records[*idx].fleet_id())
}

fn report_perspective_for_mission(
    mission: Option<Mission>,
    role: BattleRole,
) -> FleetBattlePerspective {
    match mission {
        Some(Mission::GuardStarbase | Mission::GuardBlockadeWorld) => {
            FleetBattlePerspective::Intercepted
        }
        Some(
            Mission::MoveOnly
            | Mission::SeekHome
            | Mission::PatrolSector
            | Mission::ViewWorld
            | Mission::ColonizeWorld
            | Mission::ScoutSector
            | Mission::ScoutSolarSystem,
        ) => FleetBattlePerspective::Attacked,
        Some(
            Mission::BombardWorld
            | Mission::InvadeWorld
            | Mission::BlitzWorld
            | Mission::JoinAnotherFleet
            | Mission::RendezvousSector
            | Mission::Salvage,
        ) => FleetBattlePerspective::Intercepted,
        None => {
            if matches!(role, BattleRole::GuardingDefender) {
                FleetBattlePerspective::Intercepted
            } else {
                FleetBattlePerspective::Attacked
            }
        }
    }
}

fn starbase_count_at(game_data: &CoreGameData, coords: [u8; 2], owner: u8) -> u32 {
    game_data
        .bases
        .records
        .iter()
        .filter(|b| {
            b.coords_raw() == coords && b.owner_empire_raw() == owner && b.active_flag_raw() != 0
        })
        .count() as u32
}

fn build_task_forces_at_location(game_data: &CoreGameData, coords: [u8; 2]) -> Vec<TaskForce> {
    let mut by_empire: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if fleet.current_location_coords_raw() == coords {
            by_empire
                .entry(fleet.owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }

    let mut empires: HashSet<u8> = by_empire.keys().copied().collect();
    if let Some(planet) = game_data
        .planets
        .records
        .iter()
        .find(|p| p.coords_raw() == coords)
    {
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
                withdrew_under_roe: false,
                engaged_in_battle: false,
                free_hold_used: false,
            }
        })
        .collect()
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

fn distribute_fleet_losses(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    before: &FleetCombatState,
    after: &FleetCombatState,
) {
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
            *base = crate::BaseRecord::new_zeroed();
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

fn ship_losses_from_states(before: &FleetCombatState, after: &FleetCombatState) -> ShipLosses {
    ShipLosses {
        destroyers: before.counts[IDX_DD].saturating_sub(after.counts[IDX_DD]),
        cruisers: before.counts[IDX_CA].saturating_sub(after.counts[IDX_CA]),
        battleships: before.counts[IDX_BB].saturating_sub(after.counts[IDX_BB]),
        scouts: before.counts[IDX_SC].saturating_sub(after.counts[IDX_SC]),
        transports: before.counts[IDX_TT].saturating_sub(after.counts[IDX_TT]),
        etacs: before.counts[IDX_ET].saturating_sub(after.counts[IDX_ET]),
    }
}

fn ship_counts_from_state(state: &FleetCombatState) -> ShipLosses {
    ShipLosses {
        destroyers: state.counts[IDX_DD],
        cruisers: state.counts[IDX_CA],
        battleships: state.counts[IDX_BB],
        scouts: state.counts[IDX_SC],
        transports: state.counts[IDX_TT],
        etacs: state.counts[IDX_ET],
    }
}

fn tf_has_any_units(tf: &TaskForce) -> bool {
    tf.state.counts.iter().any(|&count| count > 0)
}

fn set_fleet_to_hold_current_position(fleet: &mut crate::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
    fleet.raw[0x1e] = 0x00;
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
            set_fleet_to_hold_current_position(fleet);
            fleet.set_rules_of_engagement(0);
            continue;
        }

        fleet.set_standing_order_kind(Order::SeekHome);
        fleet.set_standing_order_target_coords_raw(retreat_target);
        fleet.set_current_speed(fleet.max_speed().clamp(1, 3));
        fleet.raw[0x0d] = 0x7f;
        fleet.raw[0x0e] = 0xc0;
        fleet.raw[0x10] = 0xff;
        fleet.raw[0x11] = 0xff;
        fleet.raw[0x12] = 0x7f;
        fleet.raw[0x19] = 0x00;
        fleet.set_rules_of_engagement(0);
    }
}

fn apply_roe_retreat_to_task_force(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    retreat_target: [u8; 2],
) {
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        if fleet.destroyer_count() == 0
            && fleet.cruiser_count() == 0
            && fleet.battleship_count() == 0
            && fleet.scout_count() == 0
            && fleet.troop_transport_count() == 0
            && fleet.etac_count() == 0
        {
            continue;
        }
        fleet.set_standing_order_kind(Order::SeekHome);
        fleet.set_standing_order_target_coords_raw(retreat_target);
        fleet.set_current_speed(fleet.max_speed().clamp(1, 3));
        fleet.raw[0x0d] = 0x7f;
        fleet.raw[0x0e] = 0xc0;
        fleet.raw[0x10] = 0xff;
        fleet.raw[0x11] = 0xff;
        fleet.raw[0x12] = 0x7f;
        fleet.raw[0x19] = 0x00;
        fleet.set_rules_of_engagement(0);
    }
}

fn clear_empty_withdrawn_fleets(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        if fleet.destroyer_count() == 0
            && fleet.cruiser_count() == 0
            && fleet.battleship_count() == 0
            && fleet.scout_count() == 0
            && fleet.troop_transport_count() == 0
            && fleet.etac_count() == 0
        {
            set_fleet_to_hold_current_position(fleet);
            fleet.set_rules_of_engagement(0);
        }
    }
}

fn dominant_empire_after_battle(
    task_forces: &[TaskForce],
    winner_empire: Option<u8>,
) -> Option<u8> {
    if winner_empire.is_some() {
        return winner_empire;
    }

    let mut surviving_empires = task_forces
        .iter()
        .filter(|tf| tf.state.has_units())
        .map(|tf| tf.empire)
        .collect::<Vec<_>>();
    surviving_empires.sort_unstable();
    surviving_empires.dedup();
    if surviving_empires.len() == 1 {
        Some(surviving_empires[0])
    } else {
        None
    }
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

fn planet_idx_at_coords(game_data: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.coords_raw() == coords)
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

pub(crate) fn process_fleet_battles(
    game_data: &mut CoreGameData,
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

        for (i, left) in task_forces.iter().enumerate() {
            for right in task_forces.iter().skip(i + 1) {
                if left.empire == right.empire
                    || !left.state.has_units()
                    || !right.state.has_units()
                {
                    continue;
                }
                push_contact_event_for_task_force(&mut events, game_data, coords, left, right);
                push_contact_event_for_task_force(&mut events, game_data, coords, right, left);
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

        let planet_owner = game_data
            .planets
            .records
            .iter()
            .find(|p| p.coords_raw() == coords)
            .map(|p| p.owner_empire_slot_raw());

        let mut combat_occurred = false;
        let mut roe_declined_outcomes: Vec<RoeDeclineOutcome> = Vec::new();
        for _round in 0..3 {
            let active: Vec<u8> = task_forces
                .iter()
                .filter(|tf| {
                    tf.state.has_units() && tf.state.total_combat_as() > 0 && !tf.withdrew_under_roe
                })
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
            let mut immediate_roe_retreats = Vec::new();
            let mut engaged_empires = Vec::new();
            for tf in &task_forces {
                let our_as = *combat_as_map.get(&tf.empire).unwrap_or(&0);
                if our_as == 0 {
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
                if enemy_as == 0 {
                    continue;
                }

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
                if !forced_engagement && !rule_threshold_satisfied(roe, our_as, enemy_as) {
                    if !combat_occurred {
                        if target_empire != 0 {
                            // Check if target is a guard/blockade fleet for pursuit fire
                            if let Some(target_tf) = task_forces
                                .iter()
                                .find(|other| other.empire == target_empire)
                            {
                                if matches!(
                                    target_tf.role,
                                    BattleRole::GuardingDefender | BattleRole::IncumbentDefender
                                ) {
                                    // Pursuit fire: guard/blockade intercepts fleeing fleet
                                    // Pursuer fires at CER 0.50, withdrawer fires at normal CER
                                    let withdrawer_starbase_bonus =
                                        tf.state.counts[IDX_SB] > 0 && tf.state.fresh[IDX_SB] > 0;
                                    let withdrawer_cer = space_cer_percent(
                                        our_as,
                                        enemy_as,
                                        tf.state.is_mixed(),
                                        withdrawer_starbase_bonus,
                                    );
                                    let withdrawer_hits = hits_from(our_as, withdrawer_cer);

                                    let pursuer_as = target_tf.state.total_combat_as();
                                    // Pursuer fires at flat CER 0.50, no modifiers
                                    let pursuer_cer = pursuit_cer_percent();
                                    let pursuer_hits = hits_from(pursuer_as, pursuer_cer);

                                    roe_declined_outcomes.push(RoeDeclineOutcome::PursuitFire {
                                        withdrawer_empire: tf.empire,
                                        pursuer_empire: target_empire,
                                        withdrawer_hits,
                                        pursuer_hits,
                                    });
                                } else {
                                    // No guard/blockade - clean escape
                                    roe_declined_outcomes.push(RoeDeclineOutcome::NoEngagement {
                                        empire: tf.empire,
                                        target_empire,
                                    });
                                }
                            } else {
                                // Target task force not found - treat as no engagement
                                roe_declined_outcomes.push(RoeDeclineOutcome::NoEngagement {
                                    empire: tf.empire,
                                    target_empire,
                                });
                            }
                        }
                    } else if let Some(retreat_target) =
                        nearest_owned_planet(game_data, tf.empire, coords)
                    {
                        immediate_roe_retreats.push((tf.empire, retreat_target));
                    }
                    continue;
                }
                let starbase_bonus = tf.state.counts[IDX_SB] > 0 && tf.state.fresh[IDX_SB] > 0;
                let cer = space_cer_percent(our_as, enemy_as, tf.state.is_mixed(), starbase_bonus);
                let hits = hits_from(our_as, cer);
                *pending_hits.entry(target_empire).or_default() += hits;
                engaged_empires.push(tf.empire);
            }

            for empire in engaged_empires {
                if let Some(task_force) =
                    task_forces.iter_mut().find(|other| other.empire == empire)
                {
                    task_force.engaged_in_battle = true;
                }
            }
            for (empire, retreat_target) in immediate_roe_retreats {
                if let Some(task_force) =
                    task_forces.iter_mut().find(|other| other.empire == empire)
                {
                    task_force.withdrew_under_roe = true;
                    apply_roe_retreat_to_task_force(
                        game_data,
                        &task_force.fleet_indices,
                        retreat_target,
                    );
                }
            }

            if pending_hits.is_empty() {
                break;
            }
            combat_occurred = true;

            for tf in &mut task_forces {
                if let Some(&hits) = pending_hits.get(&tf.empire) {
                    apply_hits_to_fleet(&mut tf.state, hits);
                }
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
                let enemy_as = task_forces
                    .iter()
                    .filter(|other| other.empire != tf.empire && !other.withdrew_under_roe)
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
                    // Guards/blockades get one free hold before breaking (per spec)
                    let is_guard = matches!(
                        tf.role,
                        BattleRole::GuardingDefender | BattleRole::IncumbentDefender
                    );
                    if is_guard && !tf.free_hold_used {
                        // Use the free hold - stay and fight one more round
                        free_holds_to_consume.push(tf.empire);
                        continue;
                    }
                    let target_empire = task_forces
                        .iter()
                        .filter(|other| other.empire != tf.empire)
                        .max_by_key(|other| other.state.total_combat_as())
                        .map(|other| other.empire)
                        .unwrap_or(0);
                    let retreat_target =
                        nearest_owned_planet(game_data, tf.empire, coords).unwrap_or(coords);
                    post_round_retreats.push((tf.empire, target_empire, retreat_target));
                }
            }
            // Apply free hold consumption
            for empire in free_holds_to_consume {
                if let Some(task_force) = task_forces.iter_mut().find(|t| t.empire == empire) {
                    task_force.free_hold_used = true;
                }
            }
            for (empire, _target_empire, retreat_target) in post_round_retreats {
                if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                    task_force.withdrew_under_roe = true;
                    apply_roe_retreat_to_task_force(
                        game_data,
                        &task_force.fleet_indices,
                        retreat_target,
                    );
                }
            }
        }

        if !combat_occurred {
            for outcome in roe_declined_outcomes {
                match outcome {
                    RoeDeclineOutcome::NoEngagement {
                        empire,
                        target_empire,
                    } => {
                        if let Some(tf) = task_forces.iter().find(|tf| tf.empire == empire) {
                            for &idx in &tf.fleet_indices {
                                events.encounter_disposition_events.push(
                                    EncounterDispositionEvent::NoEngagement {
                                        fleet_idx: idx,
                                        owner_empire_raw: empire,
                                        mission: mission_kind_for_order(
                                            pre_encounter_orders.get(&idx).copied(),
                                        ),
                                        coords,
                                        target_empire_raw: target_empire,
                                        target_fleet_id: task_forces
                                            .iter()
                                            .find(|other| other.empire == target_empire)
                                            .and_then(|other| {
                                                single_named_fleet_id(
                                                    game_data,
                                                    &other.fleet_indices,
                                                )
                                            }),
                                        small_vessels: vessel_size_summary(
                                            original_states
                                                .get(&target_empire)
                                                .unwrap_or(&FleetCombatState::default()),
                                        )
                                        .0,
                                        medium_vessels: vessel_size_summary(
                                            original_states
                                                .get(&target_empire)
                                                .unwrap_or(&FleetCombatState::default()),
                                        )
                                        .1,
                                        large_vessels: vessel_size_summary(
                                            original_states
                                                .get(&target_empire)
                                                .unwrap_or(&FleetCombatState::default()),
                                        )
                                        .2,
                                        reason: EncounterDispositionReason::RoeDeclined,
                                        stardate_week: None,
                                    },
                                );
                            }
                        }
                    }
                    RoeDeclineOutcome::PursuitFire {
                        withdrawer_empire,
                        pursuer_empire,
                        withdrawer_hits,
                        pursuer_hits,
                    } => {
                        // Capture pre-pursuit-fire states for loss calculation
                        let withdrawer_before = task_forces
                            .iter()
                            .find(|tf| tf.empire == withdrawer_empire)
                            .map(|tf| tf.state.clone())
                            .unwrap_or_default();
                        let pursuer_before = task_forces
                            .iter()
                            .find(|tf| tf.empire == pursuer_empire)
                            .map(|tf| tf.state.clone())
                            .unwrap_or_default();

                        // Apply pursuit fire hits simultaneously
                        if let Some(withdrawer_tf) = task_forces
                            .iter_mut()
                            .find(|tf| tf.empire == withdrawer_empire)
                        {
                            apply_hits_to_fleet(&mut withdrawer_tf.state, pursuer_hits);
                            withdrawer_tf.engaged_in_battle = true;
                        }
                        if let Some(pursuer_tf) = task_forces
                            .iter_mut()
                            .find(|tf| tf.empire == pursuer_empire)
                        {
                            apply_hits_to_fleet(&mut pursuer_tf.state, withdrawer_hits);
                            pursuer_tf.engaged_in_battle = true;
                        }
                        combat_occurred = true;

                        // Calculate losses from pursuit fire
                        let withdrawer_after = task_forces
                            .iter()
                            .find(|tf| tf.empire == withdrawer_empire)
                            .map(|tf| tf.state.clone())
                            .unwrap_or_default();
                        let pursuer_after = task_forces
                            .iter()
                            .find(|tf| tf.empire == pursuer_empire)
                            .map(|tf| tf.state.clone())
                            .unwrap_or_default();

                        let losses_sustained =
                            ship_losses_from_states(&withdrawer_before, &withdrawer_after);
                        let enemy_losses_inflicted =
                            ship_losses_from_states(&pursuer_before, &pursuer_after);
                        let enemy_initial = ship_counts_from_state(&pursuer_before);

                        // Find retreat target for withdrawer
                        let retreat_target =
                            nearest_owned_planet(game_data, withdrawer_empire, coords)
                                .unwrap_or(coords);

                        // The withdrawer will be retreated by the normal post-battle logic
                        // since it is now a combatant that didn't dominate (combat_occurred is set)

                        // Emit PursuitFire events for each fleet in the withdrawing task force
                        if let Some(withdrawer_tf) =
                            task_forces.iter().find(|tf| tf.empire == withdrawer_empire)
                        {
                            for &idx in &withdrawer_tf.fleet_indices {
                                events.encounter_disposition_events.push(
                                    EncounterDispositionEvent::PursuitFire {
                                        fleet_idx: idx,
                                        owner_empire_raw: withdrawer_empire,
                                        mission: mission_kind_for_order(
                                            pre_encounter_orders.get(&idx).copied(),
                                        ),
                                        coords,
                                        target_empire_raw: pursuer_empire,
                                        target_fleet_id: task_forces
                                            .iter()
                                            .find(|other| other.empire == pursuer_empire)
                                            .and_then(|other| {
                                                single_named_fleet_id(
                                                    game_data,
                                                    &other.fleet_indices,
                                                )
                                            }),
                                        enemy_initial,
                                        retreat_target_coords: retreat_target,
                                        losses_sustained,
                                        enemy_losses_inflicted,
                                        reason: EncounterDispositionReason::RoeDeclined,
                                        stardate_week: None,
                                    },
                                );
                            }
                        }
                    }
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
            let target_fleet_id = task_forces
                .iter()
                .find(|other| other.empire == target_empire_raw)
                .and_then(|other| single_named_fleet_id(game_data, &other.fleet_indices));
            for &idx in &tf.fleet_indices {
                events
                    .encounter_disposition_events
                    .push(EncounterDispositionEvent::Retreated {
                        fleet_idx: idx,
                        owner_empire_raw: tf.empire,
                        mission: mission_kind_for_order(pre_encounter_orders.get(&idx).copied()),
                        coords,
                        target_empire_raw,
                        target_fleet_id,
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
            let reporting_fleet_id = reporting_fleet_idx
                .map(|idx| game_data.fleets.records[idx].fleet_id())
                .filter(|fleet_id| *fleet_id != 0);
            let reporting_mission = reporting_fleet_idx
                .and_then(|idx| mission_kind_for_order(pre_encounter_orders.get(&idx).copied()));
            let primary_enemy_fleet_id = task_forces
                .iter()
                .filter(|tf| tf.empire != empire && tf.state.has_units())
                .max_by_key(|tf| tf.state.total_combat_as())
                .and_then(|tf| single_named_fleet_id(game_data, &tf.fleet_indices));
            events.fleet_battle_events.push(FleetBattleEvent {
                reporting_empire_raw: empire,
                reporting_fleet_id,
                reporting_mission,
                perspective: report_perspective_for_mission(reporting_mission, after_tf.role),
                coords,
                enemy_empires_raw,
                primary_enemy_fleet_id,
                held_field: dominant_empire == Some(empire),
                friendly_initial: ship_counts_from_state(before),
                friendly_losses,
                enemy_initial: ship_counts_from_state(&enemy_before),
                enemy_losses,
                stardate_week: None,
            });

            if !tf_has_any_units(after_tf) && !after_tf.fleet_indices.is_empty() {
                let fleet_id = after_tf
                    .fleet_indices
                    .first()
                    .map(|idx| game_data.fleets.records[*idx].fleet_id())
                    .unwrap_or(0);
                let friendly_armies = after_tf
                    .fleet_indices
                    .iter()
                    .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                    .sum();
                let primary_enemy_empire_raw = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .map(|tf| tf.empire);
                let primary_enemy_fleet_id = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .and_then(|tf| single_named_fleet_id(game_data, &tf.fleet_indices));
                events.fleet_destroyed_events.push(FleetDestroyedEvent {
                    reporting_empire_raw: empire,
                    fleet_id,
                    coords,
                    was_intercepting: matches!(after_tf.role, BattleRole::Attacker),
                    friendly_initial: ship_counts_from_state(before),
                    enemy_initial: ship_counts_from_state(&enemy_before),
                    enemy_losses,
                    friendly_armies,
                    primary_enemy_empire_raw,
                    primary_enemy_fleet_id,
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
                let primary_enemy_fleet_id = task_forces
                    .iter()
                    .filter(|tf| tf.empire != empire)
                    .max_by_key(|tf| tf.state.total_combat_as())
                    .and_then(|tf| single_named_fleet_id(game_data, &tf.fleet_indices));
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
                                primary_enemy_fleet_id,
                                stardate_week: None,
                            });
                    }
                }
            }
        }
    }

    Ok(events)
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

fn mission_kind_for_order(order: Option<Order>) -> Option<Mission> {
    match order? {
        Order::MoveOnly => Some(Mission::MoveOnly),
        Order::SeekHome => Some(Mission::SeekHome),
        Order::PatrolSector => Some(Mission::PatrolSector),
        Order::ViewWorld => Some(Mission::ViewWorld),
        Order::GuardStarbase => Some(Mission::GuardStarbase),
        Order::GuardBlockadeWorld => Some(Mission::GuardBlockadeWorld),
        Order::ScoutSector => Some(Mission::ScoutSector),
        Order::ScoutSolarSystem => Some(Mission::ScoutSolarSystem),
        Order::BombardWorld => Some(Mission::BombardWorld),
        Order::InvadeWorld => Some(Mission::InvadeWorld),
        Order::BlitzWorld => Some(Mission::BlitzWorld),
        _ => None,
    }
}

fn contact_reporting_kind(order: Order) -> Option<Mission> {
    match order {
        Order::MoveOnly => Some(Mission::MoveOnly),
        Order::SeekHome => Some(Mission::SeekHome),
        Order::PatrolSector => Some(Mission::PatrolSector),
        Order::ViewWorld => Some(Mission::ViewWorld),
        Order::ScoutSector => Some(Mission::ScoutSector),
        Order::ScoutSolarSystem => Some(Mission::ScoutSolarSystem),
        Order::BombardWorld => Some(Mission::BombardWorld),
        Order::InvadeWorld => Some(Mission::InvadeWorld),
        Order::BlitzWorld => Some(Mission::BlitzWorld),
        Order::GuardStarbase => Some(Mission::GuardStarbase),
        Order::JoinAnotherFleet => Some(Mission::JoinAnotherFleet),
        Order::RendezvousSector => Some(Mission::RendezvousSector),
        Order::GuardBlockadeWorld => Some(Mission::GuardBlockadeWorld),
        Order::Salvage => Some(Mission::Salvage),
        _ => None,
    }
}

fn vessel_size_summary(state: &FleetCombatState) -> (u32, u32, u32) {
    let small =
        state.counts[IDX_DD] + state.counts[IDX_SC] + state.counts[IDX_TT] + state.counts[IDX_ET];
    let medium = state.counts[IDX_CA];
    let large = state.counts[IDX_BB];
    (small, medium, large)
}

#[derive(Debug, Default)]
pub(crate) struct AssaultEvents {
    pub bombard_events: Vec<BombardEvent>,
    pub assault_report_events: Vec<AssaultReportEvent>,
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    pub ownership_change_events: Vec<PlanetOwnershipChangeEvent>,
    pub mission_events: Vec<MissionEvent>,
}

fn push_planet_intel(
    events: &mut AssaultEvents,
    planet_idx: usize,
    viewer_empire_raw: u8,
    source: PlanetIntelSource,
) {
    if viewer_empire_raw == 0 {
        return;
    }
    events.planet_intel_events.push(PlanetIntelEvent {
        planet_idx,
        viewer_empire_raw,
        source,
    });
}

fn mission_kind_for_fleet(
    fleet: usize,
    bombard_set: &HashSet<usize>,
    invade_set: &HashSet<usize>,
    blitz_set: &HashSet<usize>,
) -> Option<Mission> {
    if blitz_set.contains(&fleet) {
        Some(Mission::BlitzWorld)
    } else if invade_set.contains(&fleet) {
        Some(Mission::InvadeWorld)
    } else if bombard_set.contains(&fleet) {
        Some(Mission::BombardWorld)
    } else {
        None
    }
}

fn clear_arrival_and_hold(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        set_fleet_to_hold_current_position(&mut game_data.fleets.records[idx]);
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
    state.counts[IDX_DD] / 2 + state.counts[IDX_CA] * 3 + state.counts[IDX_BB] * 9 * 3 / 2
}

fn blitz_cover_hits(state: &FleetCombatState) -> u32 {
    let combat_ships = state.counts[IDX_DD] + state.counts[IDX_CA] + state.counts[IDX_BB];
    if combat_ships == 0 {
        return 0;
    }

    hits_from(bombard_attack_as(state).max(1), 50)
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
    bombard_ready: &[usize],
    invade_ready: &[usize],
    blitz_ready: &[usize],
) -> Result<AssaultEvents, Box<dyn std::error::Error>> {
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
                let state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let attack_as = bombard_attack_as(&state);
                let planet = &game_data.planets.records[planet_idx];
                let defense_as = planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY
                    + (planet.army_count_raw() as u32).div_ceil(2) * GROUND_AS_ARMY;
                let attacker_cer =
                    space_cer_percent(attack_as, defense_as, state.is_mixed(), false);
                let defender_cer = space_cer_percent(defense_as, attack_as.max(1), false, false);
                let attacker_hits = hits_from(attack_as, attacker_cer);
                let defender_hits = hits_from(defense_as, defender_cer);
                let pre_armies = game_data.planets.records[planet_idx].army_count_raw();
                let pre_batteries = game_data.planets.records[planet_idx].ground_batteries_raw();

                let before = state.clone();
                let mut after = state.clone();
                apply_hits_to_fleet(&mut after, defender_hits);
                distribute_fleet_losses(game_data, &winner_fleets, &before, &after);

                apply_planet_bombardment_damage(
                    &mut game_data.planets.records[planet_idx],
                    attacker_hits,
                );
                clear_arrival_and_hold(game_data, &winner_fleets);
                events.bombard_events.push(BombardEvent {
                    planet_idx,
                    attacker_empire_raw: winner_empire,
                    attacker_fleet_id: preferred_reporting_fleet_id(game_data, &winner_fleets),
                    defender_empire_raw: game_data.planets.records[planet_idx]
                        .owner_empire_slot_raw(),
                    attacker_initial: ship_counts_from_state(&before),
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_losses: ship_losses_from_states(&before, &after),
                    defender_battery_losses: pre_batteries.saturating_sub(
                        game_data.planets.records[planet_idx].ground_batteries_raw(),
                    ),
                    defender_army_losses: pre_armies
                        .saturating_sub(game_data.planets.records[planet_idx].army_count_raw()),
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
                let pre_batteries = planet.ground_batteries_raw();
                let pre_armies = planet.army_count_raw();
                let battery_as = planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY;
                let attacker_cer =
                    space_cer_percent(bombard_as, battery_as.max(1), state.is_mixed(), false);
                let defender_cer = space_cer_percent(battery_as, bombard_as.max(1), false, false);
                let suppression_hits = hits_from(bombard_as, attacker_cer);
                let return_hits = hits_from(battery_as, defender_cer);

                let before = state.clone();
                let mut after = state.clone();
                apply_hits_to_fleet(&mut after, return_hits);
                distribute_fleet_losses(game_data, &winner_fleets, &before, &after);

                {
                    let planet = &mut game_data.planets.records[planet_idx];
                    let battery_loss =
                        suppression_hits.min(planet.ground_batteries_raw() as u32) as u8;
                    planet.set_ground_batteries_raw(
                        planet.ground_batteries_raw().saturating_sub(battery_loss),
                    );
                }

                let batteries_cleared =
                    game_data.planets.records[planet_idx].ground_batteries_raw() == 0;
                if batteries_cleared {
                    let soft_hits = hits_from(
                        bombard_attack_as(&after),
                        ground_cer_percent(
                            bombard_attack_as(&after),
                            game_data.planets.records[planet_idx].army_count_raw() as u32,
                            0,
                        ),
                    );
                    apply_planet_bombardment_damage(
                        &mut game_data.planets.records[planet_idx],
                        soft_hits,
                    );

                    let attacking_armies: u32 = winner_fleets
                        .iter()
                        .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                        .sum();
                    let defender_armies =
                        game_data.planets.records[planet_idx].army_count_raw() as u32;
                    let atk_hits = hits_from(
                        attacking_armies,
                        ground_cer_percent(attacking_armies, defender_armies.max(1), 0),
                    );
                    let def_hits = hits_from(
                        defender_armies,
                        ground_cer_percent(defender_armies, attacking_armies.max(1), 0),
                    );
                    let attacker_survivors = attacking_armies.saturating_sub(def_hits);
                    let defender_survivors = defender_armies.saturating_sub(atk_hits);
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
                            attacker_fleet_id: preferred_reporting_fleet_id(
                                game_data,
                                &winner_fleets,
                            ),
                            planet_idx,
                            attacker_empire_raw: winner_empire,
                            defender_empire_raw: previous_owner,
                            attacker_initial: ship_counts_from_state(&before),
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
                            attacker_fleet_id: preferred_reporting_fleet_id(
                                game_data,
                                &winner_fleets,
                            ),
                            planet_idx,
                            attacker_empire_raw: winner_empire,
                            defender_empire_raw: previous_owner,
                            attacker_initial: ship_counts_from_state(&before),
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
                        attacker_fleet_id: preferred_reporting_fleet_id(game_data, &winner_fleets),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&before),
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
                push_planet_intel(&mut events, planet_idx, winner_empire, intel_source);
                push_planet_intel(&mut events, planet_idx, previous_owner, intel_source);
                let owner_after = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                push_planet_intel(&mut events, planet_idx, owner_after, intel_source);
            }
            MissionClass::Blitz => {
                let previous_owner = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                let cover_state = fleet_state_from_records(game_data, &winner_fleets, 0);
                let pre_batteries = game_data.planets.records[planet_idx].ground_batteries_raw();
                let pre_armies = game_data.planets.records[planet_idx].army_count_raw();
                let attacking_armies: u32 = winner_fleets
                    .iter()
                    .map(|idx| game_data.fleets.records[*idx].army_count() as u32)
                    .sum();
                // A blitz uses only a brief cover-fire exchange before the drop.
                // This is intentionally lighter than a full invade bombardment.
                let cover_hits = blitz_cover_hits(&cover_state);
                {
                    let planet = &mut game_data.planets.records[planet_idx];
                    let battery_loss = cover_hits.min(planet.ground_batteries_raw() as u32) as u8;
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
                let atk_hits = hits_from(
                    armies_after_landing,
                    ground_cer_percent(armies_after_landing, defender_armies.max(1), 0),
                );
                let def_hits = hits_from(
                    defender_armies,
                    ground_cer_percent(defender_armies, armies_after_landing.max(1), 1),
                );
                let attacker_survivors = armies_after_landing.saturating_sub(def_hits);
                let defender_survivors = defender_armies.saturating_sub(atk_hits);
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
                        attacker_fleet_id: preferred_reporting_fleet_id(game_data, &winner_fleets),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&cover_state),
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
                        attacker_fleet_id: preferred_reporting_fleet_id(game_data, &winner_fleets),
                        planet_idx,
                        attacker_empire_raw: winner_empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&cover_state),
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
                push_planet_intel(&mut events, planet_idx, winner_empire, intel_source);
                push_planet_intel(&mut events, planet_idx, previous_owner, intel_source);
                let owner_after = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                push_planet_intel(&mut events, planet_idx, owner_after, intel_source);
            }
            _ => {}
        }
    }

    Ok(events)
}
