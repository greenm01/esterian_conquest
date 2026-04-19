use crate::dashboard::buffer::{Cell, OverlayGlyph};

pub(crate) struct CachedPanel {
    pub(crate) inputs_hash: u64,
    pub(crate) cells: Vec<Cell>,
    /// Overlay glyphs produced by the panel's draw function. Cached alongside
    /// the cell grid so a cache hit restores both the cell snapshot and the
    /// floating-overlay glyphs (planet/fleet markers, etc.).
    pub(crate) overlay_glyphs: Vec<OverlayGlyph>,
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
