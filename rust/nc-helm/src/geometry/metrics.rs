//! Grid + glyph measurement.
//!
//! The renderer paints to a fixed character grid (cols × rows of equal-sized
//! cells). Cell width comes from the rendered advance of `M` in the bundled
//! monospace font; cell height comes from the configured line height. The
//! "text band" is the inked vertical slice of a line (ascender top to
//! descender bottom) used for `BackgroundMode::TextBand` strips so highlight
//! fills hug the glyphs instead of the full line box.
//!
//! All values are produced from real shaping/rasterisation through glyphon +
//! cosmic-text + swash, so they track whatever monospace face the renderer
//! actually loads at the current DPI scale.

use glyphon::{
    Attrs, Buffer as GlyphBuffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use winit::dpi::LogicalSize;

/// Logical font size, scaled by the window DPI factor at probe time.
pub const FONT_SIZE: f32 = 18.0;
/// Logical line height (cell height before DPI scaling).
pub const LINE_HEIGHT: f32 = 24.0;
/// Approximate width-to-font-size ratio for a monospace cell, used only for
/// the initial window sizing hint before a real font has been measured.
const NOMINAL_CELL_WIDTH_RATIO: f64 = 2.0 / 3.0;

/// Pixel size of one grid cell at the current DPI scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellMetrics {
    pub width_px: usize,
    pub height_px: usize,
}

/// Glyph layout measurements for one cell row.
///
/// `baseline_px` is the y-offset (from the top of the line box) at which
/// glyphon places the baseline. `band_top_px` and `band_height_px` describe
/// the inked vertical slice of the line — the region a `TextBand` background
/// fill should cover so the highlight tracks the glyphs rather than the full
/// line box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub font_size_px: f32,
    pub line_height_px: f32,
    pub baseline_px: usize,
    pub band_top_px: usize,
    pub band_height_px: usize,
}

/// Bundled cell + text measurements for the current DPI scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMetrics {
    pub cell: CellMetrics,
    pub text: TextMetrics,
}

impl GridMetrics {
    /// Probe glyphon at the given DPI scale and return measurements suitable
    /// for the current font. Calls into `probe_cell_width_px` and
    /// `probe_text_band` so cell size and band geometry come from the real
    /// bundled monospace face rather than constants.
    pub fn for_scale(scale_factor: f64, font_system: &mut FontSystem) -> Self {
        let scale = scale_factor as f32;
        let text = TextMetrics {
            font_size_px: (FONT_SIZE * scale).max(1.0),
            line_height_px: (LINE_HEIGHT * scale).max(1.0),
            baseline_px: 0,
            band_top_px: 0,
            band_height_px: 0,
        };
        let (baseline_px, band_top_px, band_height_px) = probe_text_band(font_system, text);
        let width_px = probe_cell_width_px(font_system, text);
        let text = TextMetrics {
            baseline_px,
            band_top_px,
            band_height_px,
            ..text
        };
        let height_px = text.line_height_px.round().max(1.0) as usize;
        let m = Self {
            cell: CellMetrics {
                width_px,
                height_px,
            },
            text,
        };
        m
    }
}

/// Pre-shaping estimate of cell width, used to size the window before any
/// real font has been loaded. Real cell width comes from `probe_cell_width_px`.
pub fn nominal_cell_width_px() -> f64 {
    f64::from(FONT_SIZE) * NOMINAL_CELL_WIDTH_RATIO
}

/// Logical window size that fits a `cols × rows` grid using the nominal cell
/// metrics. The actual surface size at runtime is whatever the OS hands us;
/// this is just the requested initial size.
pub fn logical_window_size_for_grid(cols: usize, rows: usize) -> LogicalSize<f64> {
    LogicalSize::new(
        cols as f64 * nominal_cell_width_px(),
        rows as f64 * f64::from(LINE_HEIGHT),
    )
}

/// Shape a single `M` and return its rounded advance in pixels. Monospace
/// fonts give every cell the same advance, so this is also the cell width.
fn probe_cell_width_px(font_system: &mut FontSystem, text: TextMetrics) -> usize {
    let mut buffer = GlyphBuffer::new(
        font_system,
        Metrics::new(text.font_size_px, text.line_height_px),
    );
    buffer.set_size(font_system, None, Some(text.line_height_px));
    buffer.set_text(
        font_system,
        "M",
        &Attrs::new().family(Family::Monospace).weight(Weight::NORMAL),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);
    let width = buffer
        .layout_runs()
        .next()
        .map(|run| run.line_w.max(1.0))
        .unwrap_or(1.0);
    width.round().max(1.0) as usize
}

/// Measure the inked vertical extent of a line by rasterising probe glyphs
/// that exercise both the ascender (`H`) and the descenders (`gyp`), plus a
/// box-drawing-style `|` for tall vertical strokes.
///
/// Returns `(baseline_px, band_top_px, band_height_px)`, all measured from
/// the top of the line box (y-down). The band is what
/// `BackgroundMode::TextBand` paints so highlight strips hug the glyph
/// silhouette instead of the entire `line_height_px`.
///
/// Geometry note: glyphon hands us
///   - `run.line_y`: baseline y in line-box coords (y-down).
///   - `glyph.physical(...).y`: sub-pixel residual after quantisation.
///   - `image.placement.top`: distance from baseline *upward* to the top of
///     the rasterised bitmap (positive = above the baseline).
///
/// In a y-down coordinate system the bitmap top is therefore
/// `baseline + physical.y - placement.top`. Note the **subtraction** of
/// `placement.top`: an earlier version added it, which placed the band below
/// the baseline and pushed it out of the cell. The regression tests below
/// guard against that sign flip.
fn probe_text_band(font_system: &mut FontSystem, text: TextMetrics) -> (usize, usize, usize) {
    let mut buffer = GlyphBuffer::new(
        font_system,
        Metrics::new(text.font_size_px, text.line_height_px),
    );
    buffer.set_size(font_system, None, Some(text.line_height_px));
    buffer.set_text(
        font_system,
        "Hgyp|",
        &Attrs::new().family(Family::Monospace).weight(Weight::NORMAL),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    let line_height_px = text.line_height_px.round().max(1.0) as i32;
    let baseline_px = buffer
        .layout_runs()
        .next()
        .map(|run| run.line_y.round().max(0.0) as usize)
        .unwrap_or(line_height_px.max(1) as usize);

    let mut swash_cache = SwashCache::new();
    let mut band_top = i32::MAX;
    let mut band_bottom = i32::MIN;

    for run in buffer.layout_runs() {
        let baseline = run.line_y.round() as i32;
        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let Some(image) = swash_cache.get_image_uncached(font_system, physical.cache_key) else {
                continue;
            };
            // Screen-space (y-down) bitmap top: baseline minus the upward
            // distance from the baseline to the bitmap top edge.
            let glyph_top = baseline + physical.y - image.placement.top as i32;
            let glyph_bottom = glyph_top + image.placement.height as i32;
            band_top = band_top.min(glyph_top);
            band_bottom = band_bottom.max(glyph_bottom);
        }
    }

    if band_top == i32::MAX || band_bottom <= band_top {
        // No glyphs rasterised (font missing, etc.) — fall back to a band
        // that covers the full line so highlights are still visible.
        return (baseline_px, 0, line_height_px.max(1) as usize);
    }

    // Clamp into the line box. `band_top` must leave room for at least one
    // pixel of height; `band_bottom` cannot exceed the line height.
    let top = band_top.clamp(0, line_height_px.saturating_sub(1));
    let bottom = band_bottom.clamp(top + 1, line_height_px.max(top + 1));
    (baseline_px, top as usize, (bottom - top) as usize)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::sync::Arc;

    use super::GridMetrics;

    // Load the same fonts the renderer uses so the probe sees a real monospace face.
    const PRIMARY_REGULAR: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Regular.ttf"
    ));

    fn font_system_with_bundled_font() -> super::FontSystem {
        let mut fs = super::FontSystem::new();
        fs.db_mut().load_font_source(glyphon::fontdb::Source::Binary(
            Arc::new(Cow::Borrowed(PRIMARY_REGULAR)),
        ));
        fs.db_mut().set_monospace_family("0xProto Nerd Font Mono".to_string());
        fs
    }

    /// The probe must place the band in the upper portion of the line box, not at or
    /// near the bottom. Prior to the sign fix, `band_top_px` was ~line_height/2 or
    /// higher; after the fix it should sit close to the ascenders (well under half).
    #[test]
    fn text_band_sits_in_upper_half_of_line_box() {
        let mut fs = font_system_with_bundled_font();
        let m = GridMetrics::for_scale(1.0, &mut fs);
        let line_height = m.cell.height_px;

        assert!(
            m.text.band_top_px < line_height / 2,
            "band_top_px ({}) should be in the upper half of the line box (height {})",
            m.text.band_top_px,
            line_height,
        );
    }

    /// The band must stay entirely within the cell row.
    #[test]
    fn text_band_fits_within_cell_height() {
        let mut fs = font_system_with_bundled_font();
        let m = GridMetrics::for_scale(1.0, &mut fs);
        let line_height = m.cell.height_px;
        let band_bottom = m.text.band_top_px + m.text.band_height_px;

        assert!(
            band_bottom <= line_height,
            "band_top_px ({}) + band_height_px ({}) = {} exceeds cell height {}",
            m.text.band_top_px,
            m.text.band_height_px,
            band_bottom,
            line_height,
        );
    }

    /// The band must cover a meaningful portion of the line (ascenders to descenders).
    /// A correctly measured band should span at least half the cell height.
    #[test]
    fn text_band_covers_majority_of_line() {
        let mut fs = font_system_with_bundled_font();
        let m = GridMetrics::for_scale(1.0, &mut fs);
        let line_height = m.cell.height_px;
        let min_expected = line_height / 2;

        assert!(
            m.text.band_height_px >= min_expected,
            "band_height_px ({}) should cover at least half the cell height ({})",
            m.text.band_height_px,
            min_expected,
        );
    }
}
