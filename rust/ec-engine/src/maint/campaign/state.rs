use super::super::{
    CampaignOutcomeEvent, CampaignOutlookEvent, CivilDisorderEvent, FleetDefectionEvent,
    MaintenanceEvents,
};
use crate::{CoreGameData, DiplomaticRelation};

pub(super) fn detect_campaign_outlook_events(
    _before: crate::CampaignOutlook,
    after: crate::CampaignOutlook,
    _civil_disorder_events: &[CivilDisorderEvent],
) -> Vec<CampaignOutlookEvent> {
    match after {
        crate::CampaignOutlook::SoleContender(empire_raw) => {
            vec![CampaignOutlookEvent {
                empire_raw,
                stardate_week: None,
            }]
        }
        _ => Vec::new(),
    }
}

pub(super) fn detect_campaign_outcome_events(
    _before: crate::CampaignOutcome,
    after: crate::CampaignOutcome,
) -> Vec<CampaignOutcomeEvent> {
    match after {
        crate::CampaignOutcome::RecognizedEmperor(emperor_empire_raw) => {
            vec![CampaignOutcomeEvent {
                emperor_empire_raw,
                stardate_week: None,
            }]
        }
        _ => Vec::new(),
    }
}

pub(super) fn apply_civil_disorder_fleet_defections(
    game_data: &mut CoreGameData,
    newly_disordered: &[CivilDisorderEvent],
) -> Result<Vec<FleetDefectionEvent>, Box<dyn std::error::Error>> {
    let mut to_remove = vec![false; game_data.fleets.records.len()];
    let mut events = Vec::new();

    for empire_raw in 1..=game_data.player.records.len() as u8 {
        let Some(player) = game_data
            .player
            .records
            .get(empire_raw.saturating_sub(1) as usize)
        else {
            continue;
        };
        if player.owner_mode_raw() != 0x00 {
            continue;
        }
        if newly_disordered
            .iter()
            .any(|event| event.reporting_empire_raw == empire_raw)
        {
            continue;
        }
        if game_data
            .planets
            .records
            .iter()
            .any(|planet| planet.owner_empire_slot_raw() == empire_raw)
        {
            continue;
        }

        let candidate = game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() == empire_raw && super::super::fleet_has_presence(fleet)
            })
            .max_by_key(|(_, fleet)| fleet.fleet_id());

        if let Some((fleet_idx, fleet)) = candidate {
            to_remove[fleet_idx] = true;
            events.push(FleetDefectionEvent {
                reporting_empire_raw: empire_raw,
                fleet_id: fleet.fleet_id(),
                stardate_week: None,
            });
        }
    }

    if to_remove.iter().any(|remove| *remove) {
        super::super::remove_selected_fleets(game_data, &to_remove);
    }

    Ok(events)
}

pub(super) fn apply_stored_diplomatic_escalations(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pairs = Vec::new();

    for event in &events.fleet_battle_events {
        for &enemy_empire_raw in &event.enemy_empires_raw {
            pairs.push((event.reporting_empire_raw, enemy_empire_raw));
        }
    }

    for event in &events.bombard_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for event in &events.assault_report_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for event in &events.diplomatic_escalation_events {
        pairs.push((event.left_empire_raw, event.right_empire_raw));
    }

    for (left, right) in pairs {
        if left == 0 || right == 0 || left == right {
            continue;
        }
        let _ = game_data.set_stored_diplomatic_relation(left, right, DiplomaticRelation::Enemy)?;
        let _ = game_data.set_stored_diplomatic_relation(right, left, DiplomaticRelation::Enemy)?;
    }

    Ok(())
}

pub(super) fn apply_campaign_state_transitions(
    game_data: &mut CoreGameData,
) -> Vec<CivilDisorderEvent> {
    let player_count = game_data.player.records.len() as u8;
    let mut events = Vec::new();
    for empire_raw in 1..=player_count {
        let Some(state) = game_data.empire_campaign_state(empire_raw) else {
            continue;
        };
        if matches!(
            state,
            crate::CampaignState::DefectionRisk | crate::CampaignState::Defeated
        ) {
            if let Some(player) = game_data
                .player
                .records
                .get_mut(empire_raw.saturating_sub(1) as usize)
            {
                if player.owner_mode_raw() == 0x01 {
                    let prior_label = if !player.controlled_empire_name_summary().is_empty() {
                        player.controlled_empire_name_summary()
                    } else if !player.assigned_player_handle_summary().is_empty() {
                        player.assigned_player_handle_summary()
                    } else {
                        format!("Empire #{empire_raw}")
                    };
                    player.set_civil_disorder_mode();
                    events.push(CivilDisorderEvent {
                        reporting_empire_raw: empire_raw,
                        prior_label,
                        stardate_week: None,
                    });
                }
            }
        }
    }
    events
}

pub(super) fn update_player_starbase_flag(game_data: &mut CoreGameData) {
    for player in game_data.player.records.iter_mut() {
        let sc = player.starbase_count_raw();
        player.set_starbase_presence_flag_raw(if sc > 0 { 0x01 } else { 0x00 });
    }
}
