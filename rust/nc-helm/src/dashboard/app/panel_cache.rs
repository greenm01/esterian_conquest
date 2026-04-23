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
    pub(crate) world_index: std::collections::BTreeMap<[u8; 2], usize>,
    /// Sector coords that contain at least one viewer fleet, built once per revision.
    pub(crate) viewer_fleet_sectors: std::collections::BTreeSet<[u8; 2]>,
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
