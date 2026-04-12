use nc_nostr::state_sync::GameState;
use nc_ui::ScreenGeometry;

use crate::app::state::DashApp;
use crate::dashboard_launch::DashLaunchState;

pub fn build_hosted_dash_app(
    snapshot: &GameState,
    geometry: ScreenGeometry,
) -> Result<DashApp, Box<dyn std::error::Error>> {
    DashLaunchState::from_hosted_snapshot(snapshot)?.into_app(geometry)
}
