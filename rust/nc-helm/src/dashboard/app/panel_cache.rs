use std::collections::{BTreeMap, BTreeSet};

use crate::dashboard::buffer::Cell;

pub(crate) struct CachedPanel {
    pub(crate) inputs_hash: u64,
    pub(crate) cells: Vec<Cell>,
}

/// Cached starmap data for a specific (game_data_revision, player).
/// Reused across every `draw` call until the game state changes.
/// Stores the projection, its world-index lookup, and the viewer's
/// fleet sector set so none of those are rebuilt on each frame.
pub(crate) struct CachedStarmapProjection {
    pub(crate) revision: u64,
    pub(crate) player: usize,
    pub(crate) projection: nc_data::PlayerStarmapProjection,
    /// `[x, y] → index into projection.worlds`, built once per revision.
    pub(crate) world_index: BTreeMap<[u8; 2], usize>,
    /// Sector coords that contain at least one viewer fleet, built once per revision.
    pub(crate) viewer_fleet_sectors: BTreeSet<[u8; 2]>,
}

/// Cached sector-detail data for a specific `(game_data_revision, player)`.
/// Reused across layout measurement, sector-detail panel draws, and planet
/// popups so crosshair-only redraws do not rebuild every projected world.
pub(crate) struct CachedSectorDetails {
    pub(crate) revision: u64,
    pub(crate) player: usize,
    pub(crate) details_by_planet_index:
        BTreeMap<usize, crate::dashboard::planet_view::SelectedPlanetDetail>,
    pub(crate) preferred_body_width: usize,
    pub(crate) preferred_body_rows: usize,
}

#[derive(Default)]
pub(crate) struct PanelCache {
    pub(crate) economy: Option<CachedPanel>,
    pub(crate) planets: Option<CachedPanel>,
    pub(crate) fleets: Option<CachedPanel>,
    pub(crate) war_record: Option<CachedPanel>,
    pub(crate) starmap: Option<CachedPanel>,
    pub(crate) comms: Option<CachedPanel>,
    pub(crate) known_galaxy: Option<CachedPanel>,
    pub(crate) diplomacy: Option<CachedPanel>,
    pub(crate) sector_detail: Option<CachedPanel>,
}
