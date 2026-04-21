//! wgpu + glyphon GPU renderer for the nc-helm character grid.
//!
//! The rigid playfield grid is rendered by a fullscreen shader fed from a
//! storage buffer of per-cell glyph/style data plus a boot-time monospace
//! atlas. glyphon is reserved for overlays and rare grid glyph fallbacks that
//! are not part of the fixed atlas repertoire.
//!
//! Coordinate conventions:
//! - `GridMapper` and glyphon both use pixel-space coordinates with row 0 at
//!   the top (y-down).
//! - The fragment shader receives `@builtin(position)` in the same
//!   framebuffer-space coordinates, so cell-local math can stay in y-down
//!   pixels without NDC conversions.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Weight, fontdb,
};
use wgpu::{
    self, BindGroup, BindGroupLayout, Buffer, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Sampler,
    SamplerDescriptor, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::event_loop::ActiveEventLoop;

use super::primitives;
use crate::geometry::{GridMapper, GridMetrics, PhysicalRect, TextMetrics};
use crate::grid::{
    BackgroundMode, Cell, CellStyle, GameColor, OverlayAnchor, OverlayText, OverlayTextFamily,
    PlayfieldBuffer, Point, ScreenGeometry,
};
use crate::theme;

/// A GPU-ready representation of a single grid cell.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GpuCell {
    ch: u32,
    fg: u32,
    bg: u32,
    // style bits: bit 0 = bold, bit 1 = text-band background
    style: u32,
}

/// Metadata about the grid layout passed to the shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GpuGrid {
    width: u32,
    height: u32,
    cell_width_px: u32,
    cell_height_px: u32,
    origin_x: u32,
    origin_y: u32,
    band_top_px: u32,
    band_height_px: u32,
    app_bg: u32,
    cursor_col: u32,
    cursor_row: u32,
    cursor_visible: u32,
}

const GPU_STYLE_BOLD: u32 = 1 << 0;
const GPU_STYLE_TEXT_BAND: u32 = 1 << 1;

const GRID_ATLAS_ASCII_START: u32 = 32;
const GRID_ATLAS_ASCII_END: u32 = 127;
const GRID_ATLAS_ASCII_COUNT: u32 = GRID_ATLAS_ASCII_END - GRID_ATLAS_ASCII_START;
const GRID_ATLAS_BOX_START: u32 = 0x2500;
const GRID_ATLAS_BOX_END: u32 = 0x2580;
const GRID_ATLAS_BOX_COUNT: u32 = GRID_ATLAS_BOX_END - GRID_ATLAS_BOX_START;
const GRID_ATLAS_GREEK_COUNT: u32 = 24;
const GRID_ATLAS_MISC_CHARS: [char; 6] = ['△', '⨁', '·', '◊', '—', '●'];
const GRID_ATLAS_MISC_COUNT: u32 = GRID_ATLAS_MISC_CHARS.len() as u32;
const GRID_ATLAS_BASE_GLYPH_COUNT: u32 =
    GRID_ATLAS_ASCII_COUNT + GRID_ATLAS_BOX_COUNT + GRID_ATLAS_GREEK_COUNT + GRID_ATLAS_MISC_COUNT;
const GRID_ATLAS_COLS: u32 = 32;
const GRID_ATLAS_ROWS: u32 = 16;

const GRID_GREEK_UPPERCASE: [char; GRID_ATLAS_GREEK_COUNT as usize] = [
    'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ', 'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ', 'Ο', 'Π', 'Ρ', 'Σ', 'Τ',
    'Υ', 'Φ', 'Χ', 'Ψ', 'Ω',
];

/// Fullscreen-quad shader that uploads the CPU `background_pixels` buffer.
///
/// Six vertices form two triangles covering NDC `[-1, +1]` on both axes.
/// The UV table maps each NDC corner to the matching texture corner so the
/// background buffer (row 0 at top) renders right-side-up:
///
/// | NDC corner       | screen pos    | UV     | texel pos      |
/// |------------------|---------------|--------|----------------|
/// | `(-1, -1)`       | bottom-left   | `(0,1)`| bottom-left    |
/// | `( 1, -1)`       | bottom-right  | `(1,1)`| bottom-right   |
/// | `(-1,  1)`       | top-left      | `(0,0)`| top-left       |
/// | `( 1,  1)`       | top-right     | `(1,0)`| top-right      |
///
/// Mismatching these (e.g. pairing NDC `(-1, -1)` with UV `(0, 0)`) sample
/// the background texture vertically mirrored, which would put cell strips
/// and the caret at the wrong rows while glyphon (which paints in
/// pixel-space, independently of this quad) keeps text in place — producing
/// labels that look offset from their input strips.
const BACKGROUND_SHADER: &str = r#"
@group(0) @binding(0) var background_tex: texture_2d<f32>;
@group(0) @binding(1) var background_sampler: sampler;
@group(0) @binding(2) var<storage, read> grid_data: array<Cell>;
@group(0) @binding(3) var<uniform> grid_config: GridConfig;
@group(0) @binding(4) var grid_atlas: texture_2d<f32>;

struct Cell {
    char_val: u32,
    fg: u32,
    bg: u32,
    style: u32,
};

struct GridConfig {
    width: u32,
    height: u32,
    cell_width_px: u32,
    cell_height_px: u32,
    origin_x: u32,
    origin_y: u32,
    band_top_px: u32,
    band_height_px: u32,
    app_bg: u32,
    cursor_col: u32,
    cursor_row: u32,
    cursor_visible: u32,
};

const STYLE_BOLD: u32 = 1u;
const STYLE_TEXT_BAND: u32 = 2u;
const ATLAS_ASCII_COUNT: u32 = 95u;
const ATLAS_BOX_COUNT: u32 = 128u;
const ATLAS_GREEK_COUNT: u32 = 24u;
const ATLAS_MISC_COUNT: u32 = 6u;
const ATLAS_GLYPH_COUNT: u32 = ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + ATLAS_MISC_COUNT;
const ATLAS_COLS: u32 = 32u;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

fn srgb_channel_to_linear(value: f32) -> f32 {
    if (value <= 0.04045) {
        return value / 12.92;
    }
    return pow((value + 0.055) / 1.055, 2.4);
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
    );
}

fn unpack_color_linear(packed: u32) -> vec4<f32> {
    let srgb = unpack4x8unorm(packed);
    return vec4<f32>(srgb_to_linear(srgb.rgb), srgb.a);
}

fn get_atlas_base_index(char_val: u32) -> i32 {
    if (char_val >= 32u && char_val < 127u) {
        return i32(char_val - 32u);
    }
    if (char_val >= 0x2500u && char_val < 0x2580u) {
        return i32(char_val - 0x2500u + 95u);
    }
    if (char_val >= 0x0391u && char_val <= 0x03A9u && char_val != 0x03A2u) {
        let offset = select(char_val - 0x0391u, char_val - 0x0392u, char_val > 0x03A1u);
        return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + offset);
    }
    switch char_val {
        case 0x25B3u: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT);
        }
        case 0x2A01u: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + 1u);
        }
        case 0x00B7u: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + 2u);
        }
        case 0x25CAu: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + 3u);
        }
        case 0x2014u: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + 4u);
        }
        case 0x25CFu: {
            return i32(ATLAS_ASCII_COUNT + ATLAS_BOX_COUNT + ATLAS_GREEK_COUNT + 5u);
        }
        default: {}
    }
    return -1;
}

fn primitive_kind(char_val: u32) -> i32 {
    switch char_val {
        case 0x2500u: { return 0; }  // ─
        case 0x2502u: { return 1; }  // │
        case 0x250Cu: { return 2; }  // ┌
        case 0x2510u: { return 3; }  // ┐
        case 0x2514u: { return 4; }  // └
        case 0x2518u: { return 5; }  // ┘
        case 0x251Cu: { return 6; }  // ├
        case 0x2524u: { return 7; }  // ┤
        case 0x252Cu: { return 8; }  // ┬
        case 0x2534u: { return 9; }  // ┴
        case 0x253Cu: { return 10; } // ┼
        case 0x256Du: { return 11; } // ╭
        case 0x256Eu: { return 12; } // ╮
        case 0x256Fu: { return 13; } // ╯
        case 0x2570u: { return 14; } // ╰
        default: { return -1; }
    }
}

fn square_primitive_alpha(kind: i32, x: u32, y: u32, cell_w: u32, cell_h: u32) -> f32 {
    let mid_x = cell_w / 2u;
    let mid_y = cell_h / 2u;
    var left = false;
    var right = false;
    var up = false;
    var down = false;
    switch kind {
        case 0: { left = true; right = true; }
        case 1: { up = true; down = true; }
        case 2: { right = true; down = true; }
        case 3: { left = true; down = true; }
        case 4: { right = true; up = true; }
        case 5: { left = true; up = true; }
        case 6: { right = true; up = true; down = true; }
        case 7: { left = true; up = true; down = true; }
        case 8: { left = true; right = true; down = true; }
        case 9: { left = true; right = true; up = true; }
        case 10: { left = true; right = true; up = true; down = true; }
        default: {}
    }
    if ((left && y == mid_y && x <= mid_x)
        || (right && y == mid_y && x >= mid_x)
        || (up && x == mid_x && y <= mid_y)
        || (down && x == mid_x && y >= mid_y)) {
        return 1.0;
    }
    return 0.0;
}

fn rounded_primitive_alpha(kind: i32, x: u32, y: u32, cell_w: u32, cell_h: u32) -> f32 {
    let sample_x = f32(x);
    let sample_y = f32(y);
    let mid_x = f32(cell_w / 2u);
    let mid_y = f32(cell_h / 2u);
    let left = 0.0;
    let top = 0.0;
    let right = f32(cell_w - 1u);
    let bottom = f32(cell_h - 1u);
    let left_rx = max(mid_x - left, 1.0);
    let right_rx = max(right - mid_x, 1.0);
    let top_ry = max(mid_y - top, 1.0);
    let bottom_ry = max(bottom - mid_y, 1.0);

    var center = vec2<f32>(0.0, 0.0);
    var radius = vec2<f32>(1.0, 1.0);
    var in_quadrant = false;

    switch kind {
        case 11: {
            center = vec2<f32>(right, bottom);
            radius = vec2<f32>(right_rx, bottom_ry);
            in_quadrant = sample_x >= mid_x && sample_y >= mid_y;
        }
        case 12: {
            center = vec2<f32>(left, bottom);
            radius = vec2<f32>(left_rx, bottom_ry);
            in_quadrant = sample_x <= mid_x && sample_y >= mid_y;
        }
        case 13: {
            center = vec2<f32>(left, top);
            radius = vec2<f32>(left_rx, top_ry);
            in_quadrant = sample_x <= mid_x && sample_y <= mid_y;
        }
        case 14: {
            center = vec2<f32>(right, top);
            radius = vec2<f32>(right_rx, top_ry);
            in_quadrant = sample_x >= mid_x && sample_y <= mid_y;
        }
        default: {}
    }

    if (!in_quadrant) {
        return 0.0;
    }

    let distance = abs(length((vec2<f32>(sample_x, sample_y) - center) / radius) - 1.0);
    let threshold = 0.75 * max(1.0 / radius.x, 1.0 / radius.y);
    return select(0.0, 1.0, distance <= threshold);
}

fn primitive_alpha(kind: i32, x: u32, y: u32, cell_w: u32, cell_h: u32) -> f32 {
    if (kind <= 10) {
        return square_primitive_alpha(kind, x, y, cell_w, cell_h);
    }
    return rounded_primitive_alpha(kind, x, y, cell_w, cell_h);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let screen_px = in.position.xy;
    let gx_px = screen_px.x - f32(grid_config.origin_x);
    let gy_px = screen_px.y - f32(grid_config.origin_y);

    if (gx_px < 0.0 || gy_px < 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let grid_x = u32(gx_px) / grid_config.cell_width_px;
    let grid_y = u32(gy_px) / grid_config.cell_height_px;

    if (grid_x >= grid_config.width || grid_y >= grid_config.height) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let cell_idx = grid_y * grid_config.width + grid_x;
    let cell = grid_data[cell_idx];

    let local_x = u32(gx_px) % grid_config.cell_width_px;
    let local_y = u32(gy_px) % grid_config.cell_height_px;
    let app_bg = unpack_color_linear(grid_config.app_bg);
    let cell_bg = unpack_color_linear(cell.bg);
    let fg_color = unpack_color_linear(cell.fg);
    let in_text_band = local_y >= grid_config.band_top_px
        && local_y < min(
            grid_config.band_top_px + max(grid_config.band_height_px, 1u),
            grid_config.cell_height_px,
        );
    let use_text_band = (cell.style & STYLE_TEXT_BAND) != 0u;

    var color = app_bg;
    if (cell.bg != grid_config.app_bg && (!use_text_band || in_text_band)) {
        color = cell_bg;
    }

    if (grid_config.cursor_visible != 0u
        && grid_x == grid_config.cursor_col
        && grid_y == grid_config.cursor_row) {
        let beam_width = select(2u, 1u, grid_config.cell_width_px <= 2u);
        if (local_x < beam_width && in_text_band) {
            color = unpack_color_linear(0xffffffffu);
        }
    }

    let primitive = primitive_kind(cell.char_val);
    if (primitive >= 0) {
        let alpha = primitive_alpha(
            primitive,
            local_x,
            local_y,
            grid_config.cell_width_px,
            grid_config.cell_height_px,
        );
        color = vec4<f32>(mix(color.rgb, fg_color.rgb, alpha), max(color.a, alpha));
        return color;
    }

    var atlas_idx = get_atlas_base_index(cell.char_val);
    if (atlas_idx >= 0) {
        if ((cell.style & STYLE_BOLD) != 0u) {
            atlas_idx = atlas_idx + i32(ATLAS_GLYPH_COUNT);
        }
        let col = u32(atlas_idx) % ATLAS_COLS;
        let row = u32(atlas_idx) / ATLAS_COLS;
        let atlas_width_px = f32(ATLAS_COLS * grid_config.cell_width_px);
        let atlas_height_px = f32(16u * grid_config.cell_height_px);
        let atlas_u = (f32(col * grid_config.cell_width_px + local_x) + 0.5) / atlas_width_px;
        let atlas_v = (f32(row * grid_config.cell_height_px + local_y) + 0.5) / atlas_height_px;
        let glyph_alpha = textureSample(grid_atlas, background_sampler, vec2<f32>(atlas_u, atlas_v)).r;
        color = vec4<f32>(mix(color.rgb, fg_color.rgb, glyph_alpha), max(color.a, glyph_alpha));
    }

    return color;
}
"#;

const PRIMARY_FONT_FAMILY: &str = "JetBrains Mono";
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
const STORMFAZE_FONT_FAMILY: &str = "Stormfaze";
const STORMFAZE_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/assets/fonts/Stormfaze.otf"
));

/// Cache key for shaped glyphon buffers. Same string + weight reuses one
/// shaped layout across cells/rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TextFamilyKey {
    Monospace,
    Named(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextBufferKey {
    text: Arc<str>,
    family: TextFamilyKey,
    font_size_bits: u32,
    line_height_bits: u32,
    bold: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextOverhang {
    left_px: usize,
    right_px: usize,
    advance_width_px: usize,
}

struct CachedTextCell {
    buffer: GlyphBuffer,
    overhang: TextOverhang,
}

/// One run of contiguous styled text staged for the glyphon `TextRenderer`.
/// `bounds` clips the run horizontally to the cells it occupies so adjacent
/// runs can't bleed into each other.
#[derive(Clone, Debug)]
struct TextPlacement {
    key: TextBufferKey,
    start_col: usize,
    end_col: usize,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: GameColor,
}

/// GPU-side resources for the background image: a 2D texture sized to the
/// surface, plus a pre-built bind group ready to attach during rendering.
struct BackgroundTexture {
    texture: Texture,
    view: TextureView,
}

struct GridAtlas {
    texture: Texture,
    view: TextureView,
}

#[derive(Clone, Debug)]
struct CachedPlayfield {
    width: usize,
    height: usize,
    rows: Vec<Vec<Cell>>,
    cursor: Option<Point>,
    overlay_texts: Vec<OverlayText>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DirtyColumnSpan {
    start_col: usize,
    end_col: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DirtyPixelRect {
    left_px: usize,
    top_px: usize,
    width_px: usize,
    height_px: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UploadStrategy {
    #[default]
    Rects,
    DirtyRows,
}

#[derive(Clone, Debug, Default)]
struct PreparedFrame {
    full_rebuild: bool,
    dirty_rows: usize,
    raw_spans: usize,
    text_rebuild_spans: usize,
    text_rebuild_cells: usize,
    text_buffer_misses: usize,
    compacted_rects: usize,
    compacted_upload_area_pct: f64,
    upload_rects: usize,
    upload_strategy: UploadStrategy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DirtyCells {
    row_spans: Vec<Vec<DirtyColumnSpan>>,
    overlay_changed: bool,
    full_rebuild: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderTimings {
    pub playfield_prepare: Duration,
    pub glyph_prepare: Duration,
    pub gpu_submit_present: Duration,
    pub total: Duration,
    pub dirty_rows: usize,
    pub raw_spans: usize,
    pub text_rebuild_spans: usize,
    pub text_rebuild_cells: usize,
    pub text_buffer_misses: usize,
    pub compacted_rects: usize,
    pub compacted_upload_area_pct: f64,
    pub upload_rects: usize,
    pub full_rebuild: bool,
    pub upload_strategy: UploadStrategy,
}

const DIRTY_SPAN_GAP_MERGE_CELLS: usize = 2;
const DIRTY_SPAN_COLLAPSE_THRESHOLD: usize = 4;
const MAX_UPLOAD_RECTS_BEFORE_ROW_FALLBACK: usize = 24;

pub struct Renderer {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: Device,
    queue: Queue,
    surface_config: SurfaceConfiguration,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: glyphon::Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: HashMap<TextBufferKey, CachedTextCell>,
    background_pipeline: wgpu::RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    grid_buffer: Buffer,
    grid_config_buffer: Buffer,
    grid_bind_group: BindGroup,
    grid_atlas: GridAtlas,
    previous_playfield: Option<CachedPlayfield>,
    grid_fallback_placements: Vec<TextPlacement>,
    overlay_placements: Vec<TextPlacement>,
    text_buffer_misses: usize,
    grid_metrics: GridMetrics,
}

impl Renderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        event_loop: &ActiveEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        )));
        let adapter =
            pollster::block_on(instance.request_adapter(&RequestAdapterOptions::default()))
                .map_err(|err| format!("unable to request wgpu adapter: {err}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default()))
                .map_err(|err| format!("unable to request wgpu device: {err}"))?;
        let surface = instance.create_surface(window.clone())?;
        let size = window.inner_size();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &surface_config);

        let mut font_system = build_font_system();
        let grid_metrics = GridMetrics::for_scale(window.scale_factor(), &mut font_system);
        let mut swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_config.format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let (background_pipeline, background_bind_group_layout) =
            create_background_pipeline(&device, surface_config.format);
        let background_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("nc-helm-background-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerDescriptor::default()
        });
        let background_texture =
            BackgroundTexture::new(&device, surface_config.width, surface_config.height);
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-grid-buffer"),
            size: (std::mem::size_of::<GpuCell>() * 256 * 128) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_config_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-grid-config-buffer"),
            size: std::mem::size_of::<GpuGrid>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_atlas = generate_grid_atlas(
            &device,
            &queue,
            &mut font_system,
            &mut swash_cache,
            grid_metrics,
        );

        let grid_bind_group = create_grid_bind_group(
            &device,
            &background_bind_group_layout,
            &background_texture,
            &background_sampler,
            &grid_buffer,
            &grid_config_buffer,
            &grid_atlas,
        );

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffers: HashMap::new(),
            background_pipeline,
            background_bind_group_layout,
            background_sampler,
            background_texture,
            grid_buffer,
            grid_config_buffer,
            grid_bind_group,
            grid_atlas,
            previous_playfield: None,
            grid_fallback_placements: Vec::new(),
            overlay_placements: Vec::new(),
            text_buffer_misses: 0,
            grid_metrics,
        })
    }

    pub fn grid_geometry_for_pixels(
        &mut self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> ScreenGeometry {
        self.sync_scale_metrics_for_scale(scale_factor);
        fit_grid_to_pixels(width, height, self.grid_metrics.cell)
    }

    pub fn grid_metrics(&self) -> GridMetrics {
        self.grid_metrics
    }

    fn upload_grid_to_gpu(&mut self, playfield: &PlayfieldBuffer) {
        let cells = playfield.get_all_cells();
        let gpu_cells: Vec<GpuCell> = cells
            .iter()
            .map(|cell| {
                let fg_rgba = primitives::color_to_rgba(cell.style.fg);
                let bg_rgba = primitives::color_to_rgba(cell.style.bg);
                GpuCell {
                    ch: cell.ch as u32,
                    fg: u32::from_le_bytes(fg_rgba),
                    bg: u32::from_le_bytes(bg_rgba),
                    style: pack_gpu_style(cell.style),
                }
            })
            .collect();

        self.queue
            .write_buffer(&self.grid_buffer, 0, bytemuck::cast_slice(&gpu_cells));

        let frame_width = self.surface_config.width;
        let frame_height = self.surface_config.height;
        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper = GridMapper::centered(
            frame_width as usize,
            frame_height as usize,
            geometry,
            self.grid_metrics.cell,
        );

        let config = GpuGrid {
            width: playfield.width() as u32,
            height: playfield.height() as u32,
            cell_width_px: self.grid_metrics.cell.width_px as u32,
            cell_height_px: self.grid_metrics.cell.height_px as u32,
            origin_x: mapper.origin_x as u32,
            origin_y: mapper.origin_y as u32,
            band_top_px: self.grid_metrics.text.band_top_px as u32,
            band_height_px: self.grid_metrics.text.band_height_px as u32,
            app_bg: u32::from_le_bytes(primitives::color_to_rgba(theme::app_background())),
            cursor_col: playfield
                .cursor()
                .map_or(0, |point| point.column.as_usize() as u32),
            cursor_row: playfield
                .cursor()
                .map_or(0, |point| point.row.as_usize() as u32),
            cursor_visible: u32::from(playfield.cursor().is_some()),
        };
        self.queue
            .write_buffer(&self.grid_config_buffer, 0, bytemuck::bytes_of(&config));
    }

    fn rebuild_grid_bind_group(&mut self) {
        self.grid_bind_group = create_grid_bind_group(
            &self.device,
            &self.background_bind_group_layout,
            &self.background_texture,
            &self.background_sampler,
            &self.grid_buffer,
            &self.grid_config_buffer,
            &self.grid_atlas,
        );
    }

    /// Render one frame of `playfield`.
    ///
    /// The grid itself is drawn by the fullscreen shader. glyphon is prepared
    /// only for overlays and rare per-cell fallbacks that are outside the
    /// fixed monospace atlas.
    pub fn render(
        &mut self,
        playfield: &PlayfieldBuffer,
    ) -> Result<RenderTimings, Box<dyn std::error::Error>> {
        let total_start = Instant::now();
        self.sync_scale_metrics();
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(RenderTimings::default());
        }
        self.ensure_surface_size(size.width, size.height);
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let prepare_start = Instant::now();
        let prepared = self.prepare_playfield(playfield);
        let playfield_prepare = prepare_start.elapsed();
        let glyph_prepare_start = Instant::now();
        let text_areas = self
            .grid_fallback_placements
            .iter()
            .chain(self.overlay_placements.iter())
            .map(|placement| TextArea {
                buffer: self
                    .text_buffers
                    .get(&placement.key)
                    .map(|cached| &cached.buffer)
                    .expect("text buffer exists"),
                left: placement.left,
                top: placement.top,
                scale: 1.0,
                bounds: placement.bounds,
                default_color: glyphon_color(placement.color),
                custom_glyphs: &[],
            })
            .collect::<Vec<_>>();
        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;
        let glyph_prepare = glyph_prepare_start.elapsed();

        self.upload_grid_to_gpu(playfield);

        let gpu_start = Instant::now();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.window.request_redraw();
                return Ok(RenderTimings {
                    playfield_prepare,
                    glyph_prepare,
                    total: total_start.elapsed(),
                    dirty_rows: prepared.dirty_rows,
                    raw_spans: prepared.raw_spans,
                    text_rebuild_spans: prepared.text_rebuild_spans,
                    text_rebuild_cells: prepared.text_rebuild_cells,
                    text_buffer_misses: prepared.text_buffer_misses,
                    compacted_rects: prepared.compacted_rects,
                    compacted_upload_area_pct: prepared.compacted_upload_area_pct,
                    upload_rects: prepared.upload_rects,
                    full_rebuild: prepared.full_rebuild,
                    upload_strategy: prepared.upload_strategy,
                    ..RenderTimings::default()
                });
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(RenderTimings {
                    playfield_prepare,
                    glyph_prepare,
                    total: total_start.elapsed(),
                    dirty_rows: prepared.dirty_rows,
                    raw_spans: prepared.raw_spans,
                    text_rebuild_spans: prepared.text_rebuild_spans,
                    text_rebuild_cells: prepared.text_rebuild_cells,
                    text_buffer_misses: prepared.text_buffer_misses,
                    compacted_rects: prepared.compacted_rects,
                    compacted_upload_area_pct: prepared.compacted_upload_area_pct,
                    upload_rects: prepared.upload_rects,
                    full_rebuild: prepared.full_rebuild,
                    upload_strategy: prepared.upload_strategy,
                    ..RenderTimings::default()
                });
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".into());
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-helm-render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color(theme::app_background())),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.background_pipeline);
            pass.set_bind_group(0, &self.grid_bind_group, &[]);
            pass.draw(0..6, 0..1);
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)?;
        }

        self.window.pre_present_notify();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(RenderTimings {
            playfield_prepare,
            glyph_prepare,
            gpu_submit_present: gpu_start.elapsed(),
            total: total_start.elapsed(),
            dirty_rows: prepared.dirty_rows,
            raw_spans: prepared.raw_spans,
            text_rebuild_spans: prepared.text_rebuild_spans,
            text_rebuild_cells: prepared.text_rebuild_cells,
            text_buffer_misses: prepared.text_buffer_misses,
            compacted_rects: prepared.compacted_rects,
            compacted_upload_area_pct: prepared.compacted_upload_area_pct,
            upload_rects: prepared.upload_rects,
            full_rebuild: prepared.full_rebuild,
            upload_strategy: prepared.upload_strategy,
        })
    }

    /// Re-probe glyphon at the window's current DPI scale. If the resulting
    /// metrics differ from the cached set, drop the shaped-text cache so
    /// every run is reshaped at the new size on the next frame.
    fn sync_scale_metrics(&mut self) {
        self.sync_scale_metrics_for_scale(self.window.scale_factor());
    }

    fn sync_scale_metrics_for_scale(&mut self, scale_factor: f64) {
        let updated = GridMetrics::for_scale(scale_factor, &mut self.font_system);
        if updated != self.grid_metrics {
            self.grid_metrics = updated;
            self.grid_atlas = generate_grid_atlas(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.swash_cache,
                self.grid_metrics,
            );
            self.rebuild_grid_bind_group();
            self.text_buffers.clear();
            self.previous_playfield = None;
            self.grid_fallback_placements.clear();
            self.overlay_placements.clear();
        }
    }

    /// Refresh the overlay text set and collect any rare grid glyphs that
    /// need glyphon fallback because they are outside the fixed atlas
    /// repertoire.
    fn prepare_playfield(&mut self, playfield: &PlayfieldBuffer) -> PreparedFrame {
        self.text_buffer_misses = 0;
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper =
            GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);

        let dirty_overlay = self
            .previous_playfield
            .as_ref()
            .map_or(true, |prev| prev.overlay_texts != playfield.overlay_texts());

        if dirty_overlay {
            let mut overlay_placements = Vec::new();
            prepare_overlay_texts(
                self,
                mapper,
                playfield.overlay_texts(),
                &mut overlay_placements,
            );
            self.overlay_placements = overlay_placements;
        }
        self.grid_fallback_placements = collect_grid_fallback_placements(self, mapper, playfield);

        self.previous_playfield = Some(snapshot_playfield(playfield));

        PreparedFrame {
            text_buffer_misses: self.text_buffer_misses,
            ..Default::default()
        }
    }

    /// Shape `key` into a glyphon `Buffer` and cache it. No-op if already
    /// cached. Buffers are sized to one cell row in height; horizontal size
    /// is left unconstrained so wider runs lay out on a single line.
    fn ensure_text_buffer(&mut self, key: TextBufferKey) -> bool {
        let inserted = ensure_text_buffer_cached(
            &mut self.font_system,
            &mut self.swash_cache,
            &mut self.text_buffers,
            key,
        );
        if inserted {
            self.text_buffer_misses += 1;
        }
        inserted
    }

    /// Resize the swapchain surface and the background texture if the
    /// window dimensions changed. Width/height are clamped to at least 1
    /// pixel so wgpu never sees a zero-sized surface.
    fn ensure_surface_size(&mut self, width: u32, height: u32) {
        if self.surface_config.width == width && self.surface_config.height == height {
            return;
        }
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        self.background_texture = BackgroundTexture::new(
            &self.device,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.rebuild_grid_bind_group();
        self.previous_playfield = None;
        self.grid_fallback_placements.clear();
        self.overlay_placements.clear();
    }
}

fn create_grid_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    background_texture: &BackgroundTexture,
    background_sampler: &Sampler,
    grid_buffer: &Buffer,
    grid_config_buffer: &Buffer,
    grid_atlas: &GridAtlas,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nc-helm-grid-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&background_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(background_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: grid_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: grid_config_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&grid_atlas.view),
            },
        ],
    })
}

fn pack_gpu_style(style: CellStyle) -> u32 {
    let mut bits = 0;
    if style.bold {
        bits |= GPU_STYLE_BOLD;
    }
    if style.bg_mode == BackgroundMode::TextBand {
        bits |= GPU_STYLE_TEXT_BAND;
    }
    bits
}

fn snapshot_playfield(playfield: &PlayfieldBuffer) -> CachedPlayfield {
    let mut rows = Vec::with_capacity(playfield.height());
    for row_idx in 0..playfield.height() {
        rows.push(playfield.row(row_idx).to_vec());
    }
    CachedPlayfield {
        width: playfield.width(),
        height: playfield.height(),
        rows,
        cursor: playfield.cursor(),
        overlay_texts: playfield.overlay_texts().to_vec(),
    }
}

/// Build a `FontSystem` preloaded with the bundled monospace face plus a
/// fallback. The primary family is set so glyphon resolves `Family::Monospace`
/// to the bundled face rather than whatever the OS picks.
fn build_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        PRIMARY_REGULAR_FONT,
    ))));
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        PRIMARY_BOLD_FONT,
    ))));
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        FALLBACK_REGULAR_FONT,
    ))));
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        STORMFAZE_REGULAR_FONT,
    ))));
    db.set_monospace_family(PRIMARY_FONT_FAMILY.to_string());
    FontSystem::new_with_locale_and_db(String::from("en-US"), db)
}

/// Build the wgpu render pipeline that draws the background texture as a
/// fullscreen quad using [`BACKGROUND_SHADER`]. The pipeline owns no vertex
/// buffer; six vertex IDs are generated by `pass.draw(0..6, 0..1)` and the
/// shader synthesises positions/UVs from `vertex_index`.
fn create_background_pipeline(
    device: &Device,
    surface_format: TextureFormat,
) -> (wgpu::RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nc-helm-background-shader"),
        source: wgpu::ShaderSource::Wgsl(BACKGROUND_SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-helm-background-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nc-helm-background-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nc-helm-background-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, bind_group_layout)
}

impl BackgroundTexture {
    fn new(device: &Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("nc-helm-background-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self { texture, view }
    }
}

fn fit_grid_to_pixels(
    width: u32,
    height: u32,
    cell: crate::geometry::CellMetrics,
) -> ScreenGeometry {
    let cols = (width.max(1) as usize / cell.width_px).max(1);
    let rows = (height.max(1) as usize / cell.height_px).max(1);
    ScreenGeometry::new(cols, rows)
}

fn collect_grid_fallback_placements(
    renderer: &mut Renderer,
    mapper: GridMapper,
    playfield: &PlayfieldBuffer,
) -> Vec<TextPlacement> {
    let mut placements = Vec::new();
    for row_idx in 0..playfield.height() {
        for (col_idx, cell) in playfield.row(row_idx).iter().copied().enumerate() {
            if !needs_grid_glyph_fallback(cell) {
                continue;
            }
            let key = make_text_key(
                Arc::<str>::from(cell.ch.to_string()),
                TextFamilyKey::Monospace,
                renderer.grid_metrics.text.font_size_px,
                renderer.grid_metrics.text.line_height_px,
                cell.style.bold,
            );
            renderer.ensure_text_buffer(key.clone());
            let overhang = renderer
                .text_buffers
                .get(&key)
                .map(|cached| cached.overhang)
                .expect("text buffer exists after shaping");
            let point = Point::from_usize(col_idx, row_idx);
            let text_origin = mapper.text_origin(point, renderer.grid_metrics.text);
            let fill_rect =
                text_cell_fill_rect(mapper, point, cell.style, renderer.grid_metrics.text);
            placements.push(TextPlacement {
                key,
                start_col: col_idx,
                end_col: col_idx + 1,
                left: text_origin.left,
                top: text_origin.top,
                bounds: expanded_text_bounds(
                    text_origin.left.floor().max(0.0) as usize,
                    fill_rect.x,
                    fill_rect.y,
                    fill_rect.x + fill_rect.width,
                    mapper.cell.height_px,
                    renderer.surface_config.width as usize,
                    renderer.surface_config.height as usize,
                    overhang,
                ),
                color: cell.style.fg,
            });
        }
    }
    placements
}

fn needs_grid_glyph_fallback(cell: Cell) -> bool {
    cell.ch != ' '
        && !primitives::should_draw_as_primitive(cell.ch)
        && char_atlas_base_index(cell.ch).is_none()
}

fn text_cell_fill_rect(
    mapper: GridMapper,
    point: Point,
    style: CellStyle,
    text_metrics: TextMetrics,
) -> PhysicalRect {
    if style.bg_mode == BackgroundMode::TextBand {
        mapper.text_band_rect(point, text_metrics)
    } else {
        mapper.cell_rect(point)
    }
}

fn char_atlas_base_index(ch: char) -> Option<u32> {
    let code = ch as u32;
    if (GRID_ATLAS_ASCII_START..GRID_ATLAS_ASCII_END).contains(&code) {
        return Some(code - GRID_ATLAS_ASCII_START);
    }
    if (GRID_ATLAS_BOX_START..GRID_ATLAS_BOX_END).contains(&code) {
        return Some(GRID_ATLAS_ASCII_COUNT + code - GRID_ATLAS_BOX_START);
    }
    if let Some(index) = GRID_GREEK_UPPERCASE
        .iter()
        .position(|candidate| *candidate == ch)
    {
        return Some(GRID_ATLAS_ASCII_COUNT + GRID_ATLAS_BOX_COUNT + index as u32);
    }
    GRID_ATLAS_MISC_CHARS
        .iter()
        .position(|candidate| *candidate == ch)
        .map(|index| {
            GRID_ATLAS_ASCII_COUNT + GRID_ATLAS_BOX_COUNT + GRID_ATLAS_GREEK_COUNT + index as u32
        })
}

fn prepare_overlay_texts(
    renderer: &mut Renderer,
    mapper: GridMapper,
    overlays: &[OverlayText],
    placements: &mut Vec<TextPlacement>,
) {
    for overlay in overlays {
        if overlay.text.is_empty() {
            continue;
        }
        match &overlay.anchor {
            OverlayAnchor::CellRect {
                left_col,
                top_row,
                width_cols,
                height_rows,
            } => {
                prepare_cell_rect_overlay(
                    renderer,
                    mapper,
                    overlay,
                    *left_col,
                    *top_row,
                    *width_cols,
                    *height_rows,
                    placements,
                );
            }
            OverlayAnchor::FractionalCell {
                center_col,
                center_row,
                font_size_cells,
            } => {
                prepare_fractional_cell_overlay(
                    renderer,
                    mapper,
                    overlay,
                    *center_col,
                    *center_row,
                    *font_size_cells,
                    placements,
                );
            }
        }
    }
}

/// Render a fit-to-bounds overlay (e.g. the Stormfaze wordmark).
fn prepare_cell_rect_overlay(
    renderer: &mut Renderer,
    mapper: GridMapper,
    overlay: &OverlayText,
    left_col: usize,
    top_row: usize,
    width_cols: usize,
    height_rows: usize,
    placements: &mut Vec<TextPlacement>,
) {
    if width_cols == 0 || height_rows == 0 {
        return;
    }
    let bounds = overlay_cell_rect_pixel_bounds(mapper, left_col, top_row, width_cols, height_rows);
    if bounds.width <= 1 || bounds.height <= 1 {
        return;
    }
    let family = match overlay.family {
        OverlayTextFamily::Stormfaze => TextFamilyKey::Named(STORMFAZE_FONT_FAMILY),
        OverlayTextFamily::Monospace => TextFamilyKey::Monospace,
    };
    let font_size = fit_overlay_font_size(
        &mut renderer.font_system,
        &overlay.text,
        family,
        bounds.width as f32,
        bounds.height as f32,
    );
    if font_size <= 0.0 {
        return;
    }
    let key = make_text_key(
        Arc::<str>::from(overlay.text.as_str()),
        family,
        font_size,
        font_size,
        overlay.style.bold,
    );
    renderer.ensure_text_buffer(key.clone());
    let cached = renderer
        .text_buffers
        .get(&key)
        .expect("overlay text buffer exists after shaping");
    let measured_width = cached
        .buffer
        .layout_runs()
        .next()
        .map(|run| run.line_w)
        .unwrap_or(0.0);
    let line_height = f32::from_bits(key.line_height_bits);
    placements.push(TextPlacement {
        key,
        start_col: 0,
        end_col: 0,
        left: bounds.x as f32 + ((bounds.width as f32 - measured_width).max(0.0) / 2.0),
        top: bounds.y as f32 + ((bounds.height as f32 - line_height).max(0.0) / 2.0),
        bounds: TextBounds {
            left: bounds.x as i32,
            top: bounds.y as i32,
            right: bounds.x.saturating_add(bounds.width) as i32,
            bottom: bounds.y.saturating_add(bounds.height) as i32,
        },
        color: overlay.style.fg,
    });
}

/// Render a single glyph floating at a fractional cell centre.
fn prepare_fractional_cell_overlay(
    renderer: &mut Renderer,
    mapper: GridMapper,
    overlay: &OverlayText,
    center_col: f32,
    center_row: f32,
    font_size_cells: f32,
    placements: &mut Vec<TextPlacement>,
) {
    let font_size_px = font_size_cells * renderer.grid_metrics.text.font_size_px;
    if font_size_px <= 0.0 {
        return;
    }
    // Pixel centre of the glyph.
    let px_x = mapper.origin_x as f32 + center_col * mapper.cell.width_px as f32;
    let px_y = mapper.origin_y as f32 + center_row * mapper.cell.height_px as f32;

    let key = make_text_key(
        Arc::<str>::from(overlay.text.as_str()),
        TextFamilyKey::Monospace,
        font_size_px,
        font_size_px,
        overlay.style.bold,
    );
    renderer.ensure_text_buffer(key.clone());
    let cached = renderer
        .text_buffers
        .get(&key)
        .expect("fractional glyph buffer exists after shaping");
    let measured_width = cached
        .buffer
        .layout_runs()
        .next()
        .map(|run| run.line_w)
        .unwrap_or(0.0);
    let line_height = f32::from_bits(key.line_height_bits);

    // Centre the glyph on (px_x, px_y).
    let left = px_x - measured_width / 2.0;
    let top = px_y - line_height / 2.0;

    // Clip to a one-cell box centred at the pixel position.
    let half_w = mapper.cell.width_px as f32 / 2.0;
    let half_h = mapper.cell.height_px as f32 / 2.0;
    let surface_w = renderer.surface_config.width as i32;
    let surface_h = renderer.surface_config.height as i32;
    placements.push(TextPlacement {
        key,
        start_col: 0,
        end_col: 0,
        left,
        top,
        bounds: TextBounds {
            left: ((px_x - half_w).max(0.0) as i32).min(surface_w),
            top: ((px_y - half_h).max(0.0) as i32).min(surface_h),
            right: ((px_x + half_w) as i32).min(surface_w),
            bottom: ((px_y + half_h) as i32).min(surface_h),
        },
        color: overlay.style.fg,
    });
}

fn overlay_cell_rect_pixel_bounds(
    mapper: GridMapper,
    left_col: usize,
    top_row: usize,
    width_cols: usize,
    height_rows: usize,
) -> crate::geometry::PhysicalRect {
    let top_left = mapper.cell_rect(Point::from_usize(left_col, top_row));
    crate::geometry::PhysicalRect {
        x: top_left.x,
        y: top_left.y,
        width: width_cols.saturating_mul(mapper.cell.width_px),
        height: height_rows.saturating_mul(mapper.cell.height_px),
    }
}

fn make_text_key(
    text: Arc<str>,
    family: TextFamilyKey,
    font_size_px: f32,
    line_height_px: f32,
    bold: bool,
) -> TextBufferKey {
    TextBufferKey {
        text,
        family,
        font_size_bits: font_size_px.to_bits(),
        line_height_bits: line_height_px.to_bits(),
        bold,
    }
}

fn fit_overlay_font_size(
    font_system: &mut FontSystem,
    text: &str,
    family: TextFamilyKey,
    max_width_px: f32,
    max_height_px: f32,
) -> f32 {
    let max_height_px = max_height_px.floor().max(1.0);
    let mut low = 1usize;
    let mut high = max_height_px as usize;
    let mut best = 0usize;
    while low <= high {
        let mid = (low + high) / 2;
        let width = measure_single_line_width(font_system, text, family, mid as f32);
        if width <= max_width_px.max(1.0) {
            best = mid;
            low = mid.saturating_add(1);
        } else {
            high = mid.saturating_sub(1);
        }
    }
    best as f32
}

fn measure_single_line_width(
    font_system: &mut FontSystem,
    text: &str,
    family: TextFamilyKey,
    font_size_px: f32,
) -> f32 {
    let mut buffer = GlyphBuffer::new(font_system, Metrics::new(font_size_px, font_size_px));
    buffer.set_size(font_system, None, Some(font_size_px.max(1.0)));
    let attrs = Attrs::new().family(match family {
        TextFamilyKey::Monospace => Family::Monospace,
        TextFamilyKey::Named(name) => Family::Name(name),
    });
    buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
        .layout_runs()
        .next()
        .map(|run| run.line_w)
        .unwrap_or(0.0)
}

fn ensure_text_buffer_cached(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text_buffers: &mut HashMap<TextBufferKey, CachedTextCell>,
    key: TextBufferKey,
) -> bool {
    if text_buffers.contains_key(&key) {
        return false;
    }
    let mut buffer = GlyphBuffer::new(
        font_system,
        Metrics::new(
            f32::from_bits(key.font_size_bits),
            f32::from_bits(key.line_height_bits),
        ),
    );
    buffer.set_size(
        font_system,
        None,
        Some(f32::from_bits(key.line_height_bits).max(1.0)),
    );
    let attrs = Attrs::new()
        .family(match key.family {
            TextFamilyKey::Monospace => Family::Monospace,
            TextFamilyKey::Named(name) => Family::Name(name),
        })
        .weight(if key.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        });
    buffer.set_text(
        font_system,
        key.text.as_ref(),
        &attrs,
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);
    let overhang = measure_text_overhang(font_system, swash_cache, &buffer);
    text_buffers.insert(key, CachedTextCell { buffer, overhang });
    true
}

fn measure_text_overhang(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    buffer: &GlyphBuffer,
) -> TextOverhang {
    let advance_width = buffer
        .layout_runs()
        .next()
        .map(|run| run.line_w.ceil().max(1.0) as i32)
        .unwrap_or(1);
    let mut ink_left = i32::MAX;
    let mut ink_right = i32::MIN;

    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let Some(image) = swash_cache
                .get_image(font_system, physical.cache_key)
                .as_ref()
            else {
                continue;
            };
            let glyph_left = physical.x + image.placement.left as i32;
            let glyph_right = glyph_left + image.placement.width as i32;
            ink_left = ink_left.min(glyph_left);
            ink_right = ink_right.max(glyph_right);
        }
    }

    if ink_left == i32::MAX || ink_right <= ink_left {
        return TextOverhang {
            left_px: 0,
            right_px: 0,
            advance_width_px: advance_width as usize,
        };
    }

    TextOverhang {
        left_px: (-ink_left).max(0) as usize,
        right_px: (ink_right - advance_width).max(0) as usize,
        advance_width_px: advance_width as usize,
    }
}

/// Map a `GameColor` to a glyphon `Color` (alpha is dropped — the glyph
/// renderer uses its own coverage for anti-aliasing).
fn glyphon_color(color: GameColor) -> Color {
    let [r, g, b, _] = primitives::color_to_rgba(color);
    Color::rgb(r, g, b)
}

fn linear_channel_from_srgb_u8(value: u8) -> f64 {
    let srgb = f64::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn clear_color(color: GameColor) -> wgpu::Color {
    let [r, g, b, a] = primitives::color_to_rgba(color);
    wgpu::Color {
        r: linear_channel_from_srgb_u8(r),
        g: linear_channel_from_srgb_u8(g),
        b: linear_channel_from_srgb_u8(b),
        a: f64::from(a) / 255.0,
    }
}

fn expanded_text_bounds(
    text_left_px: usize,
    left_px: usize,
    top_px: usize,
    right_px: usize,
    cell_height_px: usize,
    frame_width_px: usize,
    frame_height_px: usize,
    overhang: TextOverhang,
) -> TextBounds {
    let left = left_px.saturating_sub(overhang.left_px).min(frame_width_px) as i32;
    let ink_right = text_left_px
        .saturating_add(overhang.advance_width_px)
        .saturating_add(overhang.right_px)
        .min(frame_width_px);
    let right = right_px.max(ink_right).min(frame_width_px) as i32;
    let top = top_px.min(frame_height_px) as i32;
    let bottom = top_px.saturating_add(cell_height_px).min(frame_height_px) as i32;
    TextBounds {
        left,
        top,
        right,
        bottom,
    }
}

fn generate_grid_atlas(
    device: &Device,
    queue: &Queue,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    grid_metrics: GridMetrics,
) -> GridAtlas {
    let slot_w = grid_metrics.cell.width_px.max(1) as u32;
    let slot_h = grid_metrics.cell.height_px.max(1) as u32;
    let atlas_width = slot_w * GRID_ATLAS_COLS;
    let atlas_height = slot_h * GRID_ATLAS_ROWS;

    let texture = device.create_texture(&TextureDescriptor {
        label: Some("nc-helm-grid-atlas"),
        size: wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];

    for bold in [false, true] {
        let bold_offset = if bold { GRID_ATLAS_BASE_GLYPH_COUNT } else { 0 };
        for ch in atlas_repertoire_chars() {
            let Some(base_index) = char_atlas_base_index(ch) else {
                continue;
            };
            let atlas_index = base_index + bold_offset;
            let col = atlas_index % GRID_ATLAS_COLS;
            let row = atlas_index / GRID_ATLAS_COLS;
            let slot_left = col as i32 * slot_w as i32;
            let slot_top = row as i32 * slot_h as i32;
            let slot_right = slot_left + slot_w as i32;
            let slot_bottom = slot_top + slot_h as i32;
            let mut buffer = GlyphBuffer::new(
                font_system,
                Metrics::new(
                    grid_metrics.text.font_size_px,
                    grid_metrics.text.line_height_px,
                ),
            );
            buffer.set_size(
                font_system,
                Some(slot_w as f32),
                Some(grid_metrics.text.line_height_px),
            );
            buffer.set_text(
                font_system,
                &ch.to_string(),
                &Attrs::new().family(Family::Monospace).weight(if bold {
                    Weight::BOLD
                } else {
                    Weight::NORMAL
                }),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);

            for run in buffer.layout_runs() {
                let baseline = run.line_y.round() as i32;
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((0.0, 0.0), 1.0);
                    let Some(img) = swash_cache
                        .get_image(font_system, physical.cache_key)
                        .as_ref()
                    else {
                        continue;
                    };
                    let glyph_left = physical.x + img.placement.left;
                    let glyph_top = baseline + physical.y - img.placement.top as i32;

                    for y in 0..img.placement.height {
                        for x in 0..img.placement.width {
                            let src_idx = (y * img.placement.width + x) as usize;
                            let alpha = match img.data.len() {
                                len if len
                                    == (img.placement.width * img.placement.height) as usize =>
                                {
                                    img.data[src_idx]
                                }
                                len if len
                                    == (img.placement.width * img.placement.height * 4)
                                        as usize =>
                                {
                                    img.data[src_idx * 4 + 3]
                                }
                                _ => continue,
                            };
                            if alpha == 0 {
                                continue;
                            }
                            let dst_x = slot_left + glyph_left + x as i32;
                            let dst_y = slot_top + glyph_top + y as i32;
                            if !atlas_slot_contains_pixel(
                                slot_left,
                                slot_top,
                                slot_right,
                                slot_bottom,
                                dst_x,
                                dst_y,
                            ) {
                                continue;
                            }
                            if dst_x < 0
                                || dst_y < 0
                                || dst_x >= atlas_width as i32
                                || dst_y >= atlas_height as i32
                            {
                                continue;
                            }
                            let dst_idx = dst_y as usize * atlas_width as usize + dst_x as usize;
                            atlas_data[dst_idx] = atlas_data[dst_idx].max(alpha);
                        }
                    }
                }
            }
        }
    }

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_width),
            rows_per_image: Some(atlas_height),
        },
        wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
    );

    GridAtlas { texture, view }
}

fn atlas_slot_contains_pixel(
    slot_left: i32,
    slot_top: i32,
    slot_right: i32,
    slot_bottom: i32,
    dst_x: i32,
    dst_y: i32,
) -> bool {
    dst_x >= slot_left && dst_x < slot_right && dst_y >= slot_top && dst_y < slot_bottom
}

fn atlas_repertoire_chars() -> Vec<char> {
    let mut chars = (GRID_ATLAS_ASCII_START..GRID_ATLAS_ASCII_END)
        .filter_map(char::from_u32)
        .collect::<Vec<_>>();
    chars.extend((GRID_ATLAS_BOX_START..GRID_ATLAS_BOX_END).filter_map(char::from_u32));
    chars.extend(GRID_GREEK_UPPERCASE);
    chars.extend(GRID_ATLAS_MISC_CHARS);
    chars
}

#[cfg(test)]
mod tests {
    use super::{
        GPU_STYLE_BOLD, GPU_STYLE_TEXT_BAND, GRID_ATLAS_BASE_GLYPH_COUNT, GRID_ATLAS_COLS,
        GRID_ATLAS_MISC_CHARS, GRID_ATLAS_ROWS, atlas_repertoire_chars, atlas_slot_contains_pixel,
        char_atlas_base_index, clear_color, linear_channel_from_srgb_u8, needs_grid_glyph_fallback,
        pack_gpu_style,
    };
    use crate::grid::{BackgroundMode, Cell, CellStyle, GameColor};

    #[test]
    fn atlas_repertoire_fits_configured_grid() {
        let capacity = GRID_ATLAS_COLS * GRID_ATLAS_ROWS;
        assert!(
            GRID_ATLAS_BASE_GLYPH_COUNT * 2 <= capacity,
            "atlas capacity {capacity} must fit both normal and bold glyph sets"
        );
    }

    #[test]
    fn atlas_repertoire_covers_expected_special_glyphs() {
        for ch in ['Α', 'Ω', '△', '⨁', '·', '◊', '—', '●'] {
            assert!(
                char_atlas_base_index(ch).is_some(),
                "{ch:?} should be part of the GPU atlas repertoire"
            );
        }
        assert!(atlas_repertoire_chars().contains(&GRID_ATLAS_MISC_CHARS[0]));
    }

    #[test]
    fn unsupported_grid_glyphs_use_fallback() {
        let style = CellStyle::new(GameColor::White, GameColor::Black, false);
        assert!(!needs_grid_glyph_fallback(Cell::new('A', style)));
        assert!(!needs_grid_glyph_fallback(Cell::new('┌', style)));
        assert!(needs_grid_glyph_fallback(Cell::new('🙂', style)));
    }

    #[test]
    fn pack_gpu_style_sets_bold_and_text_band_bits() {
        let style = CellStyle::new(GameColor::White, GameColor::Black, true)
            .with_background_mode(BackgroundMode::TextBand);
        let bits = pack_gpu_style(style);
        assert_ne!(bits & GPU_STYLE_BOLD, 0);
        assert_ne!(bits & GPU_STYLE_TEXT_BAND, 0);
    }

    #[test]
    fn clear_color_linearizes_srgb_theme_values() {
        let clear = clear_color(GameColor::Rgb(128, 128, 128));
        let expected = linear_channel_from_srgb_u8(128);
        assert!((clear.r - expected).abs() < 0.000_001);
        assert!(clear.r < 0.25);
        assert_eq!(clear.a, 1.0);
    }

    #[test]
    fn atlas_slot_clipping_rejects_neighbor_slot_pixels() {
        let slot_left = 32;
        let slot_top = 48;
        let slot_right = 44;
        let slot_bottom = 72;

        assert!(atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_left,
            slot_top
        ));
        assert!(atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_right - 1,
            slot_bottom - 1
        ));
        assert!(!atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_left - 1,
            slot_top
        ));
        assert!(!atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_right,
            slot_top
        ));
        assert!(!atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_left,
            slot_top - 1
        ));
        assert!(!atlas_slot_contains_pixel(
            slot_left,
            slot_top,
            slot_right,
            slot_bottom,
            slot_left,
            slot_bottom
        ));
    }
}
