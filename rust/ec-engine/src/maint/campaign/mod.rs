mod conquest;
mod state;

use super::{
    CampaignOutcomeEvent, CampaignOutlookEvent, CivilDisorderEvent, FleetDefectionEvent,
    MaintenanceEvents,
};
use crate::CoreGameData;

pub(super) fn process_conquest_header(
    game_data: &mut CoreGameData,
    should_accumulate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    conquest::process_conquest_header(game_data, should_accumulate)
}

pub(super) fn detect_campaign_outlook_events(
    before: crate::CampaignOutlook,
    after: crate::CampaignOutlook,
    civil_disorder_events: &[CivilDisorderEvent],
) -> Vec<CampaignOutlookEvent> {
    state::detect_campaign_outlook_events(before, after, civil_disorder_events)
}

pub(super) fn detect_campaign_outcome_events(
    before: crate::CampaignOutcome,
    after: crate::CampaignOutcome,
) -> Vec<CampaignOutcomeEvent> {
    state::detect_campaign_outcome_events(before, after)
}

pub(super) fn apply_civil_disorder_fleet_defections(
    game_data: &mut CoreGameData,
    newly_disordered: &[CivilDisorderEvent],
) -> Result<Vec<FleetDefectionEvent>, Box<dyn std::error::Error>> {
    state::apply_civil_disorder_fleet_defections(game_data, newly_disordered)
}

pub(super) fn apply_stored_diplomatic_escalations(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    state::apply_stored_diplomatic_escalations(game_data, events)
}

pub(super) fn apply_campaign_state_transitions(
    game_data: &mut CoreGameData,
) -> Vec<CivilDisorderEvent> {
    state::apply_campaign_state_transitions(game_data)
}

pub(super) fn update_player_starbase_flag(game_data: &mut CoreGameData) {
    state::update_player_starbase_flag(game_data)
}
