//! Event canonicalization pass for the maintenance engine.
//!
//! This module runs after event assembly and before returning `MaintenanceEvents`
//! to the caller.  It:
//!
//! 1. Walks each event vector and assigns `stardate_week` based on the
//!    recovered timing rules (see [`super::timing`]).
//! 2. Enforces corpus-backed same-week pairing rules (sensor-contact chains,
//!    fleet-lost / join-retarget pairs).
//! 3. Sorts each event vector by `stardate_week` ascending so that caller
//!    rendering loops produce chronologically ordered output.
//!
//! **No game-state mutation happens here.**  The pass is purely additive:
//! it only fills in `stardate_week` fields that are currently `None`.

use crate::CoreGameData;

use super::{
    MaintenanceEvents, Mission, MissionEvent, MissionOutcome,
    timing::{apply_timing_offset, event_base_week, mission_timing_code},
};

/// Run the canonicalization pass over `events`.
///
/// After this call every report-visible event has a non-`None` `stardate_week`
/// and each event vector is sorted in ascending week order.
pub fn canonicalize_events(events: &mut MaintenanceEvents, game_data: &CoreGameData) {
    assign_mission_event_weeks(events, game_data);
    assign_combat_event_weeks(events);
    assign_assault_event_weeks(events);
    assign_bombard_event_weeks(events);
    assign_scout_contact_weeks(events);
    assign_ownership_change_weeks(events);
    assign_fleet_merge_weeks(events);
    assign_colonization_weeks(events);
    assign_salvage_weeks(events);
    assign_encounter_disposition_weeks(events);
    assign_civil_disorder_weeks(events);
    assign_campaign_weeks(events);
    assign_diplomatic_escalation_weeks(events);

    // Sort each event vector by week ascending.
    events
        .mission_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_battle_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_destroyed_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .starbase_destroyed_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .assault_report_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .bombard_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .scout_contact_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .ownership_change_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_merge_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events.colonization_events.sort_by_key(colonization_week);
    events.salvage_events.sort_by_key(salvage_week);
    events
        .encounter_disposition_events
        .sort_by_key(encounter_week);
    events
        .civil_disorder_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .campaign_outlook_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .campaign_outcome_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .fleet_defection_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .diplomatic_escalation_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
}

// ---------------------------------------------------------------------------
// Week assignment helpers
// ---------------------------------------------------------------------------

/// Derive the week for a `MissionEvent` from fleet state and mission family.
fn week_for_mission_event(event: &MissionEvent, game_data: &CoreGameData) -> u8 {
    let fleet = game_data.fleets.records.get(event.fleet_idx);
    let fleet_speed = fleet.map(|f| f.current_speed()).unwrap_or(0);

    // "Arrived" outcomes represent the turn the fleet reached the destination;
    // "Succeeded"/"Failed"/"Aborted" outcomes happen after arrival.
    // Use travel_time_years = 1 when the fleet has a meaningful speed, 0 otherwise.
    let travel_time_years: u8 = if fleet_speed > 0 { 1 } else { 0 };

    let base = event_base_week(event.kind, travel_time_years, fleet_speed);
    let code = mission_timing_code(event.kind);
    apply_timing_offset(base, code)
}

fn assign_mission_event_weeks(events: &mut MaintenanceEvents, game_data: &CoreGameData) {
    for event in &mut events.mission_events {
        if event.stardate_week.is_none() {
            event.stardate_week = Some(week_for_mission_event(event, game_data));
        }
    }
}

/// Fleet battles and destruction happen at week 2 (Code 1: base=0 → +2).
fn assign_combat_event_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.fleet_battle_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
    for e in &mut events.fleet_destroyed_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
    for e in &mut events.starbase_destroyed_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}

/// Ground assaults happen in the same week as the corresponding mission event.
fn assign_assault_event_weeks(events: &mut MaintenanceEvents) {
    // Match by planet_idx and attacker_empire to find a corresponding MissionEvent.
    let mission_weeks: Vec<(usize, u8, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if matches!(e.kind, Mission::InvadeWorld | Mission::BlitzWorld) {
                Some((
                    e.planet_idx.unwrap_or(usize::MAX),
                    e.owner_empire_raw,
                    e.stardate_week.unwrap_or(1),
                ))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.assault_report_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = mission_weeks
            .iter()
            .find(|(pidx, emp, _)| *pidx == e.planet_idx && *emp == e.attacker_empire_raw)
            .map(|(_, _, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

/// Bombardment events share the week of the BombardWorld mission event.
fn assign_bombard_event_weeks(events: &mut MaintenanceEvents) {
    let bombard_weeks: Vec<(usize, u8, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::BombardWorld && e.outcome == MissionOutcome::Succeeded {
                Some((
                    e.planet_idx.unwrap_or(usize::MAX),
                    e.owner_empire_raw,
                    e.stardate_week.unwrap_or(1),
                ))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.bombard_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = bombard_weeks
            .iter()
            .find(|(pidx, emp, _)| *pidx == e.planet_idx && *emp == e.attacker_empire_raw)
            .map(|(_, _, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

/// Scout contact events use Code 1 (+2 from base 0) → week 2.
/// Corpus: sensor-contact → identified chains land in the same week.
fn assign_scout_contact_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.scout_contact_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}

/// Ownership changes are same-week as the assault that caused them.
fn assign_ownership_change_weeks(events: &mut MaintenanceEvents) {
    let assault_weeks: Vec<(usize, u8)> = events
        .assault_report_events
        .iter()
        .filter_map(|e| {
            if matches!(e.outcome, super::MissionOutcome::Succeeded) {
                Some((e.planet_idx, e.stardate_week.unwrap_or(2)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.ownership_change_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = assault_weeks
            .iter()
            .find(|(pidx, _)| *pidx == e.planet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

/// Fleet merges happen at Code 4 (standing: +0 from base 1) → week 1.
fn assign_fleet_merge_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.fleet_merge_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(1);
        }
    }
}

fn assign_colonization_weeks(events: &mut MaintenanceEvents) {
    // Colonization weeks are matched from mission_events (ColonizeWorld Succeeded/Failed).
    let colonize_weeks: Vec<(usize, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::ColonizeWorld {
                Some((e.fleet_idx, e.stardate_week.unwrap_or(1)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.colonization_events {
        if colonization_week(e) != 1 {
            continue; // already set
        }
        let fleet_idx = match *e {
            super::ColonizationResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            super::ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. } => fleet_idx,
            super::ColonizationResolvedEvent::Aborted { fleet_idx, .. } => fleet_idx,
        };
        let week = colonize_weeks
            .iter()
            .find(|(fi, _)| *fi == fleet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(1);
        match e {
            super::ColonizationResolvedEvent::Succeeded { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
            super::ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
            super::ColonizationResolvedEvent::Aborted { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
        }
    }
}

fn assign_salvage_weeks(events: &mut MaintenanceEvents) {
    // Match from mission_events (Salvage).
    let salvage_weeks: Vec<(usize, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::Salvage {
                Some((e.fleet_idx, e.stardate_week.unwrap_or(1)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.salvage_events {
        let fleet_idx = match *e {
            super::SalvageResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            super::SalvageResolvedEvent::Failed { fleet_idx, .. } => fleet_idx,
        };
        let week = salvage_weeks
            .iter()
            .find(|(fi, _)| *fi == fleet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(1);
        match e {
            super::SalvageResolvedEvent::Succeeded { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
            super::SalvageResolvedEvent::Failed { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
        }
    }
}

fn assign_encounter_disposition_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.encounter_disposition_events {
        let already_set = match e {
            super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
                stardate_week.is_some()
            }
        };
        if !already_set {
            match e {
                super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
                super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
                super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
            }
        }
    }
}

/// Civil disorder and campaign events are end-of-year administrative events → week 52.
fn assign_civil_disorder_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.civil_disorder_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.fleet_defection_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
}

fn assign_campaign_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.campaign_outlook_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.campaign_outcome_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
}

/// Diplomatic escalations happen in the same week as the hostile action.
fn assign_diplomatic_escalation_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.diplomatic_escalation_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}

// ---------------------------------------------------------------------------
// Sort-key extractors for enum event types
// ---------------------------------------------------------------------------

fn colonization_week(e: &super::ColonizationResolvedEvent) -> u8 {
    match e {
        super::ColonizationResolvedEvent::Succeeded { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
        super::ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
        super::ColonizationResolvedEvent::Aborted { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
    }
}

fn salvage_week(e: &super::SalvageResolvedEvent) -> u8 {
    match e {
        super::SalvageResolvedEvent::Succeeded { stardate_week, .. } => stardate_week.unwrap_or(1),
        super::SalvageResolvedEvent::Failed { stardate_week, .. } => stardate_week.unwrap_or(1),
    }
}

fn encounter_week(e: &super::EncounterDispositionEvent) -> u8 {
    match e {
        super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
        super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
        super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::{
        CampaignOutlookEvent, CivilDisorderEvent, ColonizationResolvedEvent, FleetBattleEvent,
        MissionEvent, MissionOutcome, ShipLosses,
    };
    use super::*;
    use crate::builder::GameStateBuilder;

    fn minimal_game_data() -> crate::CoreGameData {
        GameStateBuilder::default()
            .build_initialized_baseline()
            .expect("minimal game data")
    }

    fn make_mission_event(kind: Mission, outcome: MissionOutcome) -> MissionEvent {
        MissionEvent {
            fleet_idx: 0,
            owner_empire_raw: 1,
            kind,
            outcome,
            planet_idx: None,
            location_coords: None,
            target_coords: None,
            stardate_week: None,
        }
    }

    #[test]
    fn canonicalize_populates_mission_event_weeks() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        events.mission_events.push(make_mission_event(
            Mission::MoveOnly,
            MissionOutcome::Succeeded,
        ));
        events.mission_events.push(make_mission_event(
            Mission::PatrolSector,
            MissionOutcome::Arrived,
        ));
        canonicalize_events(&mut events, &game_data);
        assert!(
            events
                .mission_events
                .iter()
                .all(|e| e.stardate_week.is_some())
        );
    }

    #[test]
    fn canonicalize_sorts_mission_events_by_week() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        // PatrolSector is a standing mission (week 1); ScoutSector gets code 1 offset.
        events.mission_events.push(make_mission_event(
            Mission::ScoutSector,
            MissionOutcome::Succeeded,
        ));
        events.mission_events.push(make_mission_event(
            Mission::PatrolSector,
            MissionOutcome::Arrived,
        ));
        canonicalize_events(&mut events, &game_data);
        let weeks: Vec<u8> = events
            .mission_events
            .iter()
            .map(|e| e.stardate_week.unwrap())
            .collect();
        for pair in weeks.windows(2) {
            assert!(
                pair[0] <= pair[1],
                "events should be sorted ascending: {:?}",
                weeks
            );
        }
    }

    #[test]
    fn fleet_battle_event_gets_week_2() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        events.fleet_battle_events.push(FleetBattleEvent {
            reporting_empire_raw: 1,
            reporting_fleet_id: Some(1),
            reporting_mission: Some(Mission::GuardBlockadeWorld),
            perspective: crate::maint::events::FleetBattlePerspective::Intercepted,
            coords: [5, 5],
            enemy_empires_raw: vec![2],
            primary_enemy_fleet_id: Some(5),
            held_field: true,
            friendly_initial: ShipLosses::default(),
            friendly_losses: ShipLosses::default(),
            enemy_initial: ShipLosses::default(),
            enemy_losses: ShipLosses::default(),
            stardate_week: None,
        });
        canonicalize_events(&mut events, &game_data);
        assert_eq!(events.fleet_battle_events[0].stardate_week, Some(2));
    }

    #[test]
    fn civil_disorder_events_get_week_52() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        events.civil_disorder_events.push(CivilDisorderEvent {
            reporting_empire_raw: 1,
            prior_label: "Empire Alpha".to_string(),
            stardate_week: None,
        });
        canonicalize_events(&mut events, &game_data);
        assert_eq!(events.civil_disorder_events[0].stardate_week, Some(52));
    }

    #[test]
    fn campaign_outlook_gets_week_52() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        events.campaign_outlook_events.push(CampaignOutlookEvent {
            empire_raw: 1,
            stardate_week: None,
        });
        canonicalize_events(&mut events, &game_data);
        assert_eq!(events.campaign_outlook_events[0].stardate_week, Some(52));
    }

    #[test]
    fn colonization_event_gets_week_from_mission_event() {
        let game_data = minimal_game_data();
        let mut events = MaintenanceEvents::default();
        // Add a MissionEvent for colonize with a pre-set week.
        events.mission_events.push(MissionEvent {
            fleet_idx: 3,
            owner_empire_raw: 1,
            kind: Mission::ColonizeWorld,
            outcome: MissionOutcome::Succeeded,
            planet_idx: Some(5),
            location_coords: Some([10, 10]),
            target_coords: Some([10, 10]),
            stardate_week: Some(30),
        });
        events
            .colonization_events
            .push(ColonizationResolvedEvent::Succeeded {
                fleet_idx: 3,
                planet_idx: 5,
                colonizer_empire_raw: 1,
                stardate_week: None,
            });
        canonicalize_events(&mut events, &game_data);
        let col_week = match events.colonization_events[0] {
            ColonizationResolvedEvent::Succeeded { stardate_week, .. } => stardate_week,
            _ => None,
        };
        assert_eq!(col_week, Some(30));
    }
}
