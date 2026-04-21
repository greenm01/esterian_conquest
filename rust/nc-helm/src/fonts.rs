use swash::scale::{Render, ScaleContext, Source};
use swash::shape::{Direction, ShapeContext};
use swash::text::Script;
use swash::zeno::Format;
use swash::{FontRef, GlyphId};

const PRIMARY_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Regular.ttf"
));
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Bold.ttf"
));
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Regular.ttf"
));
const STORMFAZE_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/assets/fonts/Stormfaze.otf"
));

const FALLBACK_BOLD_EMBOLDEN: f32 = 0.75;

#[derive(Clone, Copy)]
pub(crate) struct ResolvedGlyph {
    pub font: FontRef<'static>,
    pub glyph_id: GlyphId,
    pub embolden: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PositionedGlyph {
    pub glyph_id: GlyphId,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct ShapedRun {
    pub glyphs: Vec<PositionedGlyph>,
    pub width_px: f32,
    pub ascent_px: f32,
    pub descent_px: f32,
}

pub(crate) fn primary_mono_font(bold: bool) -> FontRef<'static> {
    font_ref(if bold {
        PRIMARY_BOLD_FONT
    } else {
        PRIMARY_REGULAR_FONT
    })
}

pub(crate) fn fallback_mono_font() -> FontRef<'static> {
    font_ref(FALLBACK_REGULAR_FONT)
}

pub(crate) fn stormfaze_font() -> FontRef<'static> {
    font_ref(STORMFAZE_REGULAR_FONT)
}

pub(crate) fn resolve_mono_glyph(ch: char, bold: bool) -> Option<ResolvedGlyph> {
    let primary = primary_mono_font(bold);
    let primary_glyph_id = primary.charmap().map(ch);
    if primary_glyph_id != 0 {
        return Some(ResolvedGlyph {
            font: primary,
            glyph_id: primary_glyph_id,
            embolden: 0.0,
        });
    }

    let fallback = fallback_mono_font();
    let fallback_glyph_id = fallback.charmap().map(ch);
    if fallback_glyph_id != 0 {
        return Some(ResolvedGlyph {
            font: fallback,
            glyph_id: fallback_glyph_id,
            embolden: if bold { FALLBACK_BOLD_EMBOLDEN } else { 0.0 },
        });
    }

    None
}

pub(crate) fn render_alpha_glyph(
    scale_context: &mut ScaleContext,
    glyph: ResolvedGlyph,
    size_px: f32,
    hint: bool,
) -> Option<swash::scale::image::Image> {
    let mut scaler = scale_context
        .builder(glyph.font)
        .size(size_px.max(1.0))
        .hint(hint)
        .build();
    let mut render = Render::new(&[Source::Outline]);
    render.format(Format::Alpha);
    if glyph.embolden > 0.0 {
        render.embolden(glyph.embolden);
    }
    render.render(&mut scaler, glyph.glyph_id)
}

pub(crate) fn shape_stormfaze_text(
    shape_context: &mut ShapeContext,
    text: &str,
    size_px: f32,
) -> ShapedRun {
    let font = stormfaze_font();
    let mut shaper = shape_context
        .builder(font)
        .script(Script::Latin)
        .direction(Direction::LeftToRight)
        .size(size_px.max(1.0))
        .build();
    shaper.add_str(text);
    let metrics = shaper.metrics();
    let mut pen_x = 0.0;
    let mut glyphs = Vec::new();
    shaper.shape_with(|cluster| {
        for glyph in cluster.glyphs {
            glyphs.push(PositionedGlyph {
                glyph_id: glyph.id,
                x: pen_x + glyph.x,
                y: glyph.y,
            });
            pen_x += glyph.advance;
        }
    });
    ShapedRun {
        glyphs,
        width_px: pen_x,
        ascent_px: metrics.ascent,
        descent_px: metrics.descent,
    }
}

fn font_ref(data: &'static [u8]) -> FontRef<'static> {
    FontRef::from_index(data, 0).expect("bundled font should be valid")
}
