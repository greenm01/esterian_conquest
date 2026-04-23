use crate::dashboard::buffer::Cell;

pub(crate) struct CachedPanel {
    pub(crate) inputs_hash: u64,
    pub(crate) cells: Vec<Cell>,
}

/// Cached `PlayerStarmapProjection` for a specific (game_data_revision, player).
/// Reused across every `draw` call until the game state changes.
pub(crate) struct CachedStarmapProjection {
    pub(crate) revision: u64,
    pub(crate) player: usize,
    pub(crate) projection: nc_data::PlayerStarmapProjection,
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
