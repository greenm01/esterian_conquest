use glyphon::{Attrs, Buffer as GlyphBuffer, Family, FontSystem, Metrics, Shaping, Weight};
use winit::dpi::LogicalSize;

pub const FONT_SIZE: f32 = 18.0;
pub const LINE_HEIGHT: f32 = 24.0;
const NOMINAL_CELL_WIDTH_RATIO: f64 = 2.0 / 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellMetrics {
    pub width_px: usize,
    pub height_px: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub font_size_px: f32,
    pub line_height_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMetrics {
    pub cell: CellMetrics,
    pub text: TextMetrics,
}

impl GridMetrics {
    pub fn for_scale(scale_factor: f64, font_system: &mut FontSystem) -> Self {
        let scale = scale_factor as f32;
        let text = TextMetrics {
            font_size_px: (FONT_SIZE * scale).max(1.0),
            line_height_px: (LINE_HEIGHT * scale).max(1.0),
        };
        let width_px = probe_cell_width_px(font_system, text);
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

pub fn nominal_cell_width_px() -> f64 {
    f64::from(FONT_SIZE) * NOMINAL_CELL_WIDTH_RATIO
}

pub fn logical_window_size_for_grid(cols: usize, rows: usize) -> LogicalSize<f64> {
    LogicalSize::new(
        cols as f64 * nominal_cell_width_px(),
        rows as f64 * f64::from(LINE_HEIGHT),
    )
}

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

