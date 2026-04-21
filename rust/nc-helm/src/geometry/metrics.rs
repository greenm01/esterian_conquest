//! Grid + glyph measurement.
//!
//! The renderer paints to a fixed character grid (cols × rows of equal-sized
//! cells). Cell width comes from the rendered advance of `M` in the bundled
//! monospace font; cell height comes from the configured line height. The
//! "text band" is the inked vertical slice of a line (ascender top to
//! descender bottom) used for `BackgroundMode::TextBand` strips so highlight
//! fills hug the glyphs instead of the full line box.
//!
//! All values are produced from direct swash metric probing and rasterisation
//! of the bundled fonts so they track the same monospace face the GPU atlas
//! generator uses at the current DPI scale.

use swash::scale::ScaleContext;
use winit::dpi::LogicalSize;

use crate::fonts::{primary_mono_font, render_alpha_glyph, resolve_mono_glyph};

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
    /// Probe swash at the given DPI scale and return measurements suitable
    /// for the current font.
    pub fn for_scale(scale_factor: f64) -> Self {
        let scale = scale_factor as f32;
        let text = TextMetrics {
            font_size_px: (FONT_SIZE * scale).max(1.0),
            line_height_px: (LINE_HEIGHT * scale).max(1.0),
            baseline_px: 0,
            band_top_px: 0,
            band_height_px: 0,
        };
        let (baseline_px, band_top_px, band_height_px) = probe_text_band(text);
        let width_px = probe_cell_width_px(text);
        let text = TextMetrics {
            baseline_px,
            band_top_px,
            band_height_px,
            ..text
        };
        let height_px = text.line_height_px.round().max(1.0) as usize;
        Self {
            cell: CellMetrics {
                width_px,
                height_px,
            },
            text,
        }
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

fn probe_cell_width_px(text: TextMetrics) -> usize {
    probe_advance_px(text, "M")
}

fn probe_advance_px(text: TextMetrics, sample: &str) -> usize {
    let font = primary_mono_font(false);
    let glyph_metrics = font.glyph_metrics(&[]).scale(text.font_size_px);
    let width = sample
        .chars()
        .filter_map(|ch| resolve_mono_glyph(ch, false))
        .map(|glyph| {
            glyph
                .font
                .glyph_metrics(&[])
                .scale(text.font_size_px)
                .advance_width(glyph.glyph_id)
        })
        .sum::<f32>()
        .max(glyph_metrics.advance_width(font.charmap().map('M')))
        .max(1.0);
    width.round().max(1.0) as usize
}

fn probe_text_band(text: TextMetrics) -> (usize, usize, usize) {
    let font = primary_mono_font(false);
    let metrics = font.metrics(&[]).scale(text.font_size_px);
    let baseline_px = metrics.ascent.round().max(0.0) as usize;
    let line_height_px = text.line_height_px.round().max(1.0) as i32;
    let mut scale_context = ScaleContext::new();
    let mut band_top = i32::MAX;
    let mut band_bottom = i32::MIN;

    for ch in ['H', 'g', 'y', 'p', '|'] {
        let Some(glyph) = resolve_mono_glyph(ch, false) else {
            continue;
        };
        let Some(image) = render_alpha_glyph(&mut scale_context, glyph, text.font_size_px, true)
        else {
            continue;
        };
        let glyph_top = baseline_px as i32 - image.placement.top as i32;
        let glyph_bottom = glyph_top + image.placement.height as i32;
        band_top = band_top.min(glyph_top);
        band_bottom = band_bottom.max(glyph_bottom);
    }

    if band_top == i32::MAX || band_bottom <= band_top {
        return (baseline_px, 0, line_height_px.max(1) as usize);
    }

    let top = band_top.clamp(0, line_height_px.saturating_sub(1));
    let bottom = band_bottom.clamp(top + 1, line_height_px.max(top + 1));
    (baseline_px, top as usize, (bottom - top) as usize)
}

#[cfg(test)]
mod tests {
    use super::{FONT_SIZE, GridMetrics, LINE_HEIGHT, TextMetrics, probe_advance_px};

    #[test]
    fn text_band_sits_in_upper_half_of_line_box() {
        let m = GridMetrics::for_scale(1.0);
        let line_height = m.cell.height_px;

        assert!(
            m.text.band_top_px < line_height / 2,
            "band_top_px ({}) should be in the upper half of the line box (height {})",
            m.text.band_top_px,
            line_height,
        );
    }

    #[test]
    fn text_band_fits_within_cell_height() {
        let m = GridMetrics::for_scale(1.0);
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

    #[test]
    fn text_band_covers_majority_of_line() {
        let m = GridMetrics::for_scale(1.0);
        let line_height = m.cell.height_px;
        let min_expected = line_height / 2;

        assert!(
            m.text.band_height_px >= min_expected,
            "band_height_px ({}) should cover at least half the cell height ({})",
            m.text.band_height_px,
            min_expected,
        );
    }

    #[test]
    fn box_drawing_glyphs_match_cell_advance() {
        let text = TextMetrics {
            font_size_px: FONT_SIZE,
            line_height_px: LINE_HEIGHT,
            baseline_px: 0,
            band_top_px: 0,
            band_height_px: 0,
        };
        let cell_width = probe_advance_px(text, "M");

        for glyph in ["─", "│", "╭", "╮", "╰", "╯", "┐", "┌", "┘", "└"] {
            let width = probe_advance_px(text, glyph);
            assert_eq!(
                width, cell_width,
                "box glyph {glyph:?} should keep the same advance as the monospace cell width",
            );
        }
    }
}
