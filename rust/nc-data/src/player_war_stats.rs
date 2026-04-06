use crate::storage::PlayerWarStatsState;
use crate::{EmpireUnitSummary, MaintenanceEvents, Mission, MissionOutcome, ShipLosses};

pub fn default_player_war_stats_states(player_count: u8) -> Vec<PlayerWarStatsState> {
    (1..=player_count as usize)
        .map(PlayerWarStatsState::for_player)
        .collect()
}

pub fn apply_maintenance_events_to_player_war_stats(
    player_war_stats: &mut [PlayerWarStatsState],
    events: &MaintenanceEvents,
) {
    for event in &events.colonization_events {
        if let crate::ColonizationResolvedEvent::Succeeded {
            colonizer_empire_raw,
            ..
        } = event
        {
            if let Some(stats) = state_for_empire(player_war_stats, *colonizer_empire_raw) {
                stats.colonies_established += 1;
            }
        }
    }

    for event in &events.ownership_change_events {
        if let Some(stats) = state_for_empire(player_war_stats, event.new_owner_empire_raw) {
            stats.worlds_taken += 1;
        }
        if event.previous_owner_empire_raw != 0 {
            if let Some(stats) = state_for_empire(player_war_stats, event.previous_owner_empire_raw)
            {
                stats.worlds_lost += 1;
            }
        }
    }

    for event in &events.bombard_events {
        if let Some(stats) = state_for_empire(player_war_stats, event.attacker_empire_raw) {
            stats.bombardments_launched += 1;
            add_ship_losses(&mut stats.units_lost, event.attacker_losses);
            add_unit_summary(&mut stats.enemy_units_destroyed, event.docked_losses);
            stats.enemy_units_destroyed.armies += u32::from(event.defender_army_losses);
            stats.enemy_units_destroyed.ground_batteries +=
                u32::from(event.defender_battery_losses);
        }
        if event.defender_empire_raw != 0 {
            if let Some(stats) = state_for_empire(player_war_stats, event.defender_empire_raw) {
                stats.bombardments_suffered += 1;
                stats.units_lost.armies += u32::from(event.defender_army_losses);
                stats.units_lost.ground_batteries += u32::from(event.defender_battery_losses);
                add_unit_summary(&mut stats.units_lost, event.docked_losses);
                add_ship_losses(&mut stats.enemy_units_destroyed, event.attacker_losses);
            }
        }
    }

    for event in &events.assault_report_events {
        if let Some(stats) = state_for_empire(player_war_stats, event.attacker_empire_raw) {
            match event.kind {
                Mission::InvadeWorld => {
                    stats.invade_attempts += 1;
                    if event.outcome == MissionOutcome::Succeeded {
                        stats.invade_successes += 1;
                    }
                }
                Mission::BlitzWorld => {
                    stats.blitz_attempts += 1;
                    if event.outcome == MissionOutcome::Succeeded {
                        stats.blitz_successes += 1;
                    }
                }
                _ => {}
            }
            add_ship_losses(&mut stats.units_lost, event.attacker_ship_losses);
            stats.units_lost.armies += event.attacker_army_losses;
            stats.enemy_units_destroyed.armies += u32::from(event.defender_army_losses);
            stats.enemy_units_destroyed.ground_batteries +=
                u32::from(event.defender_battery_losses);
        }
        if event.defender_empire_raw != 0 {
            if let Some(stats) = state_for_empire(player_war_stats, event.defender_empire_raw) {
                stats.units_lost.armies += u32::from(event.defender_army_losses);
                stats.units_lost.ground_batteries += u32::from(event.defender_battery_losses);
                add_ship_losses(&mut stats.enemy_units_destroyed, event.attacker_ship_losses);
                stats.enemy_units_destroyed.armies += event.attacker_army_losses;
                if event.outcome != MissionOutcome::Succeeded {
                    stats.attacks_repelled += 1;
                }
            }
        }
    }

    for event in &events.fleet_battle_events {
        if let Some(stats) = state_for_empire(player_war_stats, event.reporting_empire_raw) {
            add_ship_losses(&mut stats.units_lost, event.friendly_losses);
            stats.units_lost.starbases += event.friendly_starbases_lost;
            add_ship_losses(&mut stats.enemy_units_destroyed, event.enemy_losses);
            stats.enemy_units_destroyed.starbases += event.enemy_starbases_destroyed;
        }
    }
}

fn state_for_empire(
    player_war_stats: &mut [PlayerWarStatsState],
    empire_raw: u8,
) -> Option<&mut PlayerWarStatsState> {
    player_war_stats.get_mut(empire_raw.saturating_sub(1) as usize)
}

fn add_ship_losses(summary: &mut EmpireUnitSummary, losses: ShipLosses) {
    summary.destroyers += losses.destroyers;
    summary.cruisers += losses.cruisers;
    summary.battleships += losses.battleships;
    summary.scouts += losses.scouts;
    summary.transports += losses.transports;
    summary.etacs += losses.etacs;
}

fn add_unit_summary(summary: &mut EmpireUnitSummary, add: EmpireUnitSummary) {
    summary.destroyers += add.destroyers;
    summary.cruisers += add.cruisers;
    summary.battleships += add.battleships;
    summary.scouts += add.scouts;
    summary.transports += add.transports;
    summary.etacs += add.etacs;
    summary.starbases += add.starbases;
    summary.armies += add.armies;
    summary.ground_batteries += add.ground_batteries;
}
