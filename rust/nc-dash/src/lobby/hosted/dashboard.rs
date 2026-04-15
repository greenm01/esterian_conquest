use crate::geometry::ScreenGeometry;
use nc_data::TurnSubmission;
use nc_nostr::state_sync::GameState;

use crate::app::state::DashApp;
use crate::dashboard_launch::DashLaunchState;

pub fn build_hosted_dash_app(
    snapshot: &GameState,
    geometry: ScreenGeometry,
) -> Result<DashApp, Box<dyn std::error::Error>> {
    DashLaunchState::from_hosted_snapshot(snapshot)?.into_app(geometry)
}

pub fn replay_hosted_draft(
    dashboard: &mut DashApp,
    draft: &TurnSubmission,
) -> Result<(), nc_data::TurnSubmissionError> {
    draft.apply_to(&mut dashboard.game_data, &mut dashboard.queued_mail)?;
    dashboard.hosted_turn_draft = Some(draft.clone());
    Ok(())
}
