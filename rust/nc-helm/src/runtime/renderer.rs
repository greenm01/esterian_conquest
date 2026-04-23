//! wgpu GPU renderer for the nc-helm character grid.
//!
//! The rigid playfield grid is rendered by a fullscreen shader fed from a
//! storage buffer of per-cell glyph/style data plus boot-time monospace and
//! logo atlases.
//!
//! Coordinate conventions:
//! - `GridMapper` uses pixel-space coordinates with row 0 at the top
//!   (y-down).
//! - The fragment shader receives `@builtin(position)` in the same
//!   framebuffer-space coordinates, so cell-local math can stay in y-down
//!   pixels without NDC conversions.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use wgpu::{
    self, BindGroup, BindGroupLayout, Buffer, BufferAddress, CommandEncoderDescriptor,
    CompositeAlphaMode, Device, DeviceDescriptor, Instance, InstanceDescriptor, LoadOp, Operations,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Sampler,
    SamplerDescriptor, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::event_loop::ActiveEventLoop;

use super::primitives;
use crate::fonts::{ResolvedGlyph, render_alpha_glyph, resolve_mono_glyph, shape_stormfaze_text};
use crate::geometry::{GridMapper, GridMetrics, PhysicalRect};
use crate::grid::{
    BackgroundMode, Cell, CellStyle, GameColor, OverlayLogo, OverlayLogoKind, OverlaySelection,
    PlayfieldBuffer, Point, ScreenGeometry,
};
use crate::theme as chrome_theme;

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
    selection_left_col: u32,
    selection_right_col: u32,
    selection_top_row: u32,
    selection_bottom_row: u32,
    selection_visible: u32,
    selection_color: u32,
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
/// and the caret at the wrong rows while the grid shader still paints in
/// framebuffer space — producing labels that look offset from their input
/// strips.
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
    selection_left_col: u32,
    selection_right_col: u32,
    selection_top_row: u32,
    selection_bottom_row: u32,
    selection_visible: u32,
    selection_color: u32,
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

fn selection_corner_kind(
    grid_x: u32,
    grid_y: u32,
    config: GridConfig,
) -> i32 {
    if (config.selection_visible == 0u) {
        return -1;
    }

    let on_left   = grid_x == config.selection_left_col;
    let on_right  = grid_x == config.selection_right_col;
    let on_top    = grid_y == config.selection_top_row;
    let on_bottom = grid_y == config.selection_bottom_row;

    if (!(on_left || on_right) || !(on_top || on_bottom)) {
        return -1;
    }

    if (on_left && on_top) {
        return 0;
    }
    if (on_right && on_top) {
        return 1;
    }
    if (on_left && on_bottom) {
        return 2;
    }
    if (on_right && on_bottom) {
        return 3;
    }

    return -1;
}

fn selection_alpha(
    corner_kind: i32,
    local_x: u32,
    local_y: u32,
    config: GridConfig,
) -> f32 {
    if (corner_kind < 0) {
        return 0.0;
    }

    // Replace each selected corner lattice cell with a red square bracket.
    // The bracket sits on the same centerlines as the ASCII `|`/`-` lattice
    // it replaces so the highlight is visually anchored to the sector bounds.
    let mid_x = config.cell_width_px / 2u;
    let mid_y = config.cell_height_px / 2u;
    let thick_x = select(1u, 0u, config.cell_width_px <= 2u);
    let thick_y = select(1u, 0u, config.cell_height_px <= 2u);
    let on_vertical = abs(i32(local_x) - i32(mid_x)) <= i32(thick_x);
    let on_horizontal = abs(i32(local_y) - i32(mid_y)) <= i32(thick_y);

    var horizontal_arm = false;
    var vertical_arm = false;
    switch corner_kind {
        case 0: { // top-left
            horizontal_arm = local_x >= mid_x;
            vertical_arm = local_y >= mid_y;
        }
        case 1: { // top-right
            horizontal_arm = local_x <= mid_x;
            vertical_arm = local_y >= mid_y;
        }
        case 2: { // bottom-left
            horizontal_arm = local_x >= mid_x;
            vertical_arm = local_y <= mid_y;
        }
        case 3: { // bottom-right
            horizontal_arm = local_x <= mid_x;
            vertical_arm = local_y <= mid_y;
        }
        default: {}
    }

    return select(
        0.0,
        1.0,
        (on_horizontal && horizontal_arm) || (on_vertical && vertical_arm),
    );
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

    let selection_corner = selection_corner_kind(grid_x, grid_y, grid_config);
    let selection = unpack_color_linear(grid_config.selection_color);
    let selection_mix = selection_alpha(selection_corner, local_x, local_y, grid_config);
    if (selection_mix > 0.0) {
        color = vec4<f32>(
            mix(color.rgb, selection.rgb, selection_mix),
            max(color.a, selection_mix),
        );
    }

    if (selection_corner < 0) {
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
    }

    return color;
}
"#;

const LOGO_SHADER: &str = r#"
@group(0) @binding(0) var logo_tex: texture_2d<f32>;
@group(0) @binding(1) var logo_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let alpha = textureSample(logo_tex, logo_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

const LOGO_KIND_COUNT: usize = OverlayLogoKind::ALL.len();

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct LogoVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
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

#[derive(Clone, Copy, Debug, Default)]
struct LogoSprite {
    x_px: u32,
    y_px: u32,
    width_px: u32,
    height_px: u32,
}

struct LogoAtlas {
    texture: Texture,
    view: TextureView,
    width_px: u32,
    height_px: u32,
    sprites: [LogoSprite; LOGO_KIND_COUNT],
}

#[derive(Clone, Copy, Debug)]
struct LogoPlacement {
    sprite: LogoSprite,
    rect: PhysicalRect,
    color: GameColor,
}

#[derive(Clone, Debug)]
struct CachedPlayfield {
    width: usize,
    height: usize,
    rows: Vec<Vec<Cell>>,
    cursor: Option<Point>,
    overlay_logos: Vec<OverlayLogo>,
    overlay_selection: Option<OverlaySelection>,
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
    dirty_cells: DirtyCells,
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
    selection_changed: bool,
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
    background_pipeline: wgpu::RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    grid_buffer: Buffer,
    grid_config_buffer: Buffer,
    grid_bind_group: BindGroup,
    grid_atlas: GridAtlas,
    logo_pipeline: wgpu::RenderPipeline,
    logo_bind_group_layout: BindGroupLayout,
    logo_sampler: Sampler,
    logo_bind_group: BindGroup,
    logo_atlas: LogoAtlas,
    logo_vertex_buffer: Buffer,
    logo_vertex_capacity: usize,
    previous_playfield: Option<CachedPlayfield>,
    logo_placements: Vec<LogoPlacement>,
    reported_unsupported_grid_chars: HashSet<char>,
    grid_metrics: GridMetrics,
    ui_scale_multiplier: f64,
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

        let grid_metrics = GridMetrics::for_scale(window.scale_factor());
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

        let grid_atlas = generate_grid_atlas(&device, &queue, grid_metrics);
        let logo_atlas = generate_logo_atlas(&device, &queue, grid_metrics);
        let (logo_pipeline, logo_bind_group_layout) =
            create_logo_pipeline(&device, surface_config.format);
        let logo_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("nc-helm-logo-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        let grid_bind_group = create_grid_bind_group(
            &device,
            &background_bind_group_layout,
            &background_texture,
            &background_sampler,
            &grid_buffer,
            &grid_config_buffer,
            &grid_atlas,
        );
        let logo_bind_group =
            create_logo_bind_group(&device, &logo_bind_group_layout, &logo_atlas, &logo_sampler);
        let initial_logo_vertex_capacity = 6 * 16;
        let logo_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-logo-vertex-buffer"),
            size: (std::mem::size_of::<LogoVertex>() * initial_logo_vertex_capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            background_pipeline,
            background_bind_group_layout,
            background_sampler,
            background_texture,
            grid_buffer,
            grid_config_buffer,
            grid_bind_group,
            grid_atlas,
            logo_pipeline,
            logo_bind_group_layout,
            logo_sampler,
            logo_bind_group,
            logo_atlas,
            logo_vertex_buffer,
            logo_vertex_capacity: initial_logo_vertex_capacity,
            previous_playfield: None,
            logo_placements: Vec::new(),
            reported_unsupported_grid_chars: HashSet::new(),
            grid_metrics,
            ui_scale_multiplier: 1.0,
        })
    }

    pub fn set_ui_scale_multiplier(&mut self, scale_factor: f64, ui_scale_multiplier: f64) {
        self.ui_scale_multiplier = ui_scale_multiplier.max(0.1);
        self.sync_scale_metrics_for_scale(scale_factor * self.ui_scale_multiplier);
    }

    pub fn grid_metrics(&self) -> GridMetrics {
        self.grid_metrics
    }

    fn upload_grid_to_gpu(&mut self, playfield: &PlayfieldBuffer, dirty: &DirtyCells) {
        if dirty.full_rebuild {
            let gpu_cells: Vec<GpuCell> = playfield
                .get_all_cells()
                .iter()
                .map(|cell| self.encode_gpu_cell(*cell))
                .collect();
            self.queue
                .write_buffer(&self.grid_buffer, 0, bytemuck::cast_slice(&gpu_cells));
        } else {
            for (row_idx, spans) in dirty.row_spans.iter().enumerate() {
                if spans.is_empty() {
                    continue;
                }
                let row = playfield.row(row_idx);
                for span in spans {
                    let gpu_cells = row[span.start_col..=span.end_col]
                        .iter()
                        .copied()
                        .map(|cell| self.encode_gpu_cell(cell))
                        .collect::<Vec<_>>();
                    let offset = ((row_idx * playfield.width()) + span.start_col)
                        * std::mem::size_of::<GpuCell>();
                    self.queue.write_buffer(
                        &self.grid_buffer,
                        offset as u64,
                        bytemuck::cast_slice(&gpu_cells),
                    );
                }
            }
        }

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
            app_bg: u32::from_le_bytes(primitives::color_to_rgba(chrome_theme::app_background())),
            cursor_col: playfield
                .cursor()
                .map_or(0, |point| point.column.as_usize() as u32),
            cursor_row: playfield
                .cursor()
                .map_or(0, |point| point.row.as_usize() as u32),
            cursor_visible: u32::from(playfield.cursor().is_some()),
            selection_left_col: playfield
                .overlay_selection()
                .map_or(0, |overlay| overlay.left_col as u32),
            selection_right_col: playfield
                .overlay_selection()
                .map_or(0, |overlay| overlay.right_col as u32),
            selection_top_row: playfield
                .overlay_selection()
                .map_or(0, |overlay| overlay.top_row as u32),
            selection_bottom_row: playfield
                .overlay_selection()
                .map_or(0, |overlay| overlay.bottom_row as u32),
            selection_visible: u32::from(playfield.overlay_selection().is_some()),
            selection_color: u32::from_le_bytes(primitives::color_to_rgba(
                playfield
                    .overlay_selection()
                    .map_or(GameColor::BrightRed, |overlay| overlay.fg),
            )),
        };
        self.queue
            .write_buffer(&self.grid_config_buffer, 0, bytemuck::bytes_of(&config));
    }

    fn encode_gpu_cell(&mut self, cell: Cell) -> GpuCell {
        let fg_rgba = primitives::color_to_rgba(cell.style.fg);
        let bg_rgba = primitives::color_to_rgba(cell.style.bg);
        GpuCell {
            ch: self.canonical_grid_char(cell.ch).unwrap_or('?') as u32,
            fg: u32::from_le_bytes(fg_rgba),
            bg: u32::from_le_bytes(bg_rgba),
            style: pack_gpu_style(cell.style),
        }
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

    fn rebuild_logo_bind_group(&mut self) {
        self.logo_bind_group = create_logo_bind_group(
            &self.device,
            &self.logo_bind_group_layout,
            &self.logo_atlas,
            &self.logo_sampler,
        );
    }

    /// Render one frame of `playfield`.
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

        let prepare_start = Instant::now();
        let prepared = self.prepare_playfield(playfield);
        let playfield_prepare = prepare_start.elapsed();
        let glyph_prepare = Duration::ZERO;

        self.upload_grid_to_gpu(playfield, &prepared.dirty_cells);
        let logo_vertices = self.build_logo_vertices();
        self.ensure_logo_vertex_capacity(logo_vertices.len());
        if !logo_vertices.is_empty() {
            self.queue.write_buffer(
                &self.logo_vertex_buffer,
                0,
                bytemuck::cast_slice(&logo_vertices),
            );
        }

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
                        load: LoadOp::Clear(clear_color(chrome_theme::app_background())),
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
            if !logo_vertices.is_empty() {
                pass.set_pipeline(&self.logo_pipeline);
                pass.set_bind_group(0, &self.logo_bind_group, &[]);
                pass.set_vertex_buffer(0, self.logo_vertex_buffer.slice(..));
                pass.draw(0..logo_vertices.len() as u32, 0..1);
            }
        }

        self.window.pre_present_notify();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
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

    fn sync_scale_metrics(&mut self) {
        self.sync_scale_metrics_for_scale(self.window.scale_factor() * self.ui_scale_multiplier);
    }

    fn sync_scale_metrics_for_scale(&mut self, scale_factor: f64) {
        let updated = GridMetrics::for_scale(scale_factor);
        if updated != self.grid_metrics {
            self.grid_metrics = updated;
            self.grid_atlas = generate_grid_atlas(&self.device, &self.queue, self.grid_metrics);
            self.logo_atlas = generate_logo_atlas(&self.device, &self.queue, self.grid_metrics);
            self.rebuild_grid_bind_group();
            self.rebuild_logo_bind_group();
            self.previous_playfield = None;
            self.logo_placements.clear();
        }
    }

    fn prepare_playfield(&mut self, playfield: &PlayfieldBuffer) -> PreparedFrame {
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper =
            GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);
        let dirty_cells = compute_dirty_cells(self.previous_playfield.as_ref(), playfield);

        if dirty_cells.overlay_changed {
            self.logo_placements = playfield
                .overlay_logos()
                .iter()
                .map(|overlay| LogoPlacement {
                    sprite: self.logo_atlas.sprites[overlay.kind as usize],
                    rect: logo_rect(mapper, *overlay),
                    color: overlay.fg,
                })
                .collect();
        }

        update_snapshot_in_place(&mut self.previous_playfield, playfield, &dirty_cells);
        let raw_spans = dirty_cells
            .row_spans
            .iter()
            .map(|row| row.len())
            .sum::<usize>();
        let dirty_rows = dirty_cells
            .row_spans
            .iter()
            .filter(|row| !row.is_empty())
            .count();
        PreparedFrame {
            full_rebuild: dirty_cells.full_rebuild,
            dirty_rows,
            raw_spans,
            upload_rects: raw_spans,
            upload_strategy: if dirty_cells.full_rebuild {
                UploadStrategy::Rects
            } else {
                UploadStrategy::DirtyRows
            },
            dirty_cells,
            ..PreparedFrame::default()
        }
    }

    fn build_logo_vertices(&self) -> Vec<LogoVertex> {
        let mut vertices = Vec::with_capacity(self.logo_placements.len() * 6);
        for placement in &self.logo_placements {
            if placement.rect.width == 0 || placement.rect.height == 0 {
                continue;
            }
            let left = pixel_to_ndc_x(placement.rect.x, self.surface_config.width);
            let right = pixel_to_ndc_x(
                placement.rect.x.saturating_add(placement.rect.width),
                self.surface_config.width,
            );
            let top = pixel_to_ndc_y(placement.rect.y, self.surface_config.height);
            let bottom = pixel_to_ndc_y(
                placement.rect.y.saturating_add(placement.rect.height),
                self.surface_config.height,
            );
            let color = linear_color_f32(placement.color);
            let sprite = placement.sprite;
            let u0 = sprite.x_px as f32 / self.logo_atlas.width_px as f32;
            let v0 = sprite.y_px as f32 / self.logo_atlas.height_px as f32;
            let u1 = (sprite.x_px + sprite.width_px) as f32 / self.logo_atlas.width_px as f32;
            let v1 = (sprite.y_px + sprite.height_px) as f32 / self.logo_atlas.height_px as f32;
            vertices.extend_from_slice(&[
                LogoVertex {
                    position: [left, top],
                    uv: [u0, v0],
                    color,
                },
                LogoVertex {
                    position: [right, top],
                    uv: [u1, v0],
                    color,
                },
                LogoVertex {
                    position: [left, bottom],
                    uv: [u0, v1],
                    color,
                },
                LogoVertex {
                    position: [left, bottom],
                    uv: [u0, v1],
                    color,
                },
                LogoVertex {
                    position: [right, top],
                    uv: [u1, v0],
                    color,
                },
                LogoVertex {
                    position: [right, bottom],
                    uv: [u1, v1],
                    color,
                },
            ]);
        }
        vertices
    }

    fn ensure_logo_vertex_capacity(&mut self, vertex_count: usize) {
        if vertex_count <= self.logo_vertex_capacity {
            return;
        }
        self.logo_vertex_capacity = vertex_count.next_power_of_two().max(6);
        self.logo_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-logo-vertex-buffer"),
            size: (std::mem::size_of::<LogoVertex>() * self.logo_vertex_capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn canonical_grid_char(&mut self, ch: char) -> Option<char> {
        if ch == ' '
            || primitives::should_draw_as_primitive(ch)
            || char_atlas_base_index(ch).is_some()
        {
            return Some(ch);
        }
        debug_assert!(false, "unsupported nc-helm grid glyph: {ch:?}");
        if self.reported_unsupported_grid_chars.insert(ch) {
            eprintln!("nc-helm diagnostic: unsupported grid glyph {ch:?}; substituting '?'.");
        }
        Some('?')
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
        self.rebuild_logo_bind_group();
        self.previous_playfield = None;
        self.logo_placements.clear();
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

fn create_logo_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    logo_atlas: &LogoAtlas,
    logo_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nc-helm-logo-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&logo_atlas.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(logo_sampler),
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

/// Update the cached playfield snapshot in-place, reusing existing row `Vec`s.
///
/// On a normal incremental frame this avoids re-allocating `height` `Vec<Cell>`
/// objects and only copies the cells that actually changed.  The full-clone
/// path (`snapshot_playfield`) is only needed on the very first frame or when
/// the playfield dimensions change.
fn update_snapshot_in_place(
    previous: &mut Option<CachedPlayfield>,
    playfield: &PlayfieldBuffer,
    dirty: &DirtyCells,
) {
    let reusable = previous
        .as_ref()
        .map(|prev| prev.width == playfield.width() && prev.height == playfield.height())
        .unwrap_or(false);

    if !reusable {
        *previous = Some(snapshot_playfield(playfield));
        return;
    }

    let prev = previous.as_mut().expect("checked above");
    if dirty.full_rebuild {
        for row_idx in 0..playfield.height() {
            prev.rows[row_idx].copy_from_slice(playfield.row(row_idx));
        }
    } else {
        for (row_idx, spans) in dirty.row_spans.iter().enumerate() {
            if spans.is_empty() {
                continue;
            }
            let curr = playfield.row(row_idx);
            for span in spans {
                prev.rows[row_idx][span.start_col..=span.end_col]
                    .copy_from_slice(&curr[span.start_col..=span.end_col]);
            }
        }
    }
    prev.cursor = playfield.cursor();
    prev.overlay_logos = playfield.overlay_logos().to_vec();
    prev.overlay_selection = playfield.overlay_selection();
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
        overlay_logos: playfield.overlay_logos().to_vec(),
        overlay_selection: playfield.overlay_selection(),
    }
}

fn compute_dirty_cells(
    previous: Option<&CachedPlayfield>,
    playfield: &PlayfieldBuffer,
) -> DirtyCells {
    let height = playfield.height();
    let width = playfield.width();
    let Some(previous) = previous else {
        return DirtyCells {
            row_spans: full_rebuild_row_spans(height, width),
            overlay_changed: true,
            full_rebuild: true,
            selection_changed: playfield.overlay_selection().is_some(),
        };
    };

    if previous.width != width || previous.height != height {
        return DirtyCells {
            row_spans: full_rebuild_row_spans(height, width),
            overlay_changed: true,
            full_rebuild: true,
            selection_changed: previous.overlay_selection != playfield.overlay_selection(),
        };
    }

    let mut row_spans = Vec::with_capacity(height);
    for row_idx in 0..height {
        row_spans.push(diff_row_spans(
            &previous.rows[row_idx],
            playfield.row(row_idx),
        ));
    }

    DirtyCells {
        row_spans,
        overlay_changed: previous.overlay_logos != playfield.overlay_logos(),
        full_rebuild: false,
        selection_changed: previous.overlay_selection != playfield.overlay_selection(),
    }
}

fn full_rebuild_row_spans(height: usize, width: usize) -> Vec<Vec<DirtyColumnSpan>> {
    if width == 0 {
        return vec![Vec::new(); height];
    }
    vec![
        vec![DirtyColumnSpan {
            start_col: 0,
            end_col: width - 1,
        }];
        height
    ]
}

fn diff_row_spans(previous: &[Cell], current: &[Cell]) -> Vec<DirtyColumnSpan> {
    let changed_cols = previous
        .iter()
        .zip(current.iter())
        .enumerate()
        .filter_map(|(col, (left, right))| (*left != *right).then_some(col))
        .collect::<Vec<_>>();
    if changed_cols.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut start_col = changed_cols[0];
    let mut end_col = changed_cols[0];
    for &col in &changed_cols[1..] {
        if col <= end_col + DIRTY_SPAN_GAP_MERGE_CELLS + 1 {
            end_col = col;
            continue;
        }
        spans.push(DirtyColumnSpan { start_col, end_col });
        start_col = col;
        end_col = col;
    }
    spans.push(DirtyColumnSpan { start_col, end_col });

    if spans.len() > DIRTY_SPAN_COLLAPSE_THRESHOLD {
        vec![DirtyColumnSpan {
            start_col: changed_cols[0],
            end_col: *changed_cols
                .last()
                .expect("changed cols should not be empty"),
        }]
    } else {
        spans
    }
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

fn create_logo_pipeline(
    device: &Device,
    surface_format: TextureFormat,
) -> (wgpu::RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nc-helm-logo-shader"),
        source: wgpu::ShaderSource::Wgsl(LOGO_SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-helm-logo-bind-group-layout"),
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
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nc-helm-logo-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<LogoVertex>() as BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as BufferAddress,
                shader_location: 2,
            },
        ],
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nc-helm-logo-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_layout],
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

fn logo_rect(mapper: GridMapper, overlay: OverlayLogo) -> PhysicalRect {
    let top_left = mapper.cell_rect(Point::from_usize(overlay.left_col, overlay.top_row));
    let (width_cols, height_rows) = overlay.kind.cell_size();
    PhysicalRect {
        x: top_left.x,
        y: top_left.y,
        width: width_cols.saturating_mul(mapper.cell.width_px),
        height: height_rows.saturating_mul(mapper.cell.height_px),
    }
}

fn linear_color_f32(color: GameColor) -> [f32; 4] {
    let [r, g, b, a] = primitives::color_to_rgba(color);
    [
        linear_channel_from_srgb_u8(r) as f32,
        linear_channel_from_srgb_u8(g) as f32,
        linear_channel_from_srgb_u8(b) as f32,
        f32::from(a) / 255.0,
    ]
}

fn pixel_to_ndc_x(x_px: usize, surface_width_px: u32) -> f32 {
    (x_px as f32 / surface_width_px.max(1) as f32) * 2.0 - 1.0
}

fn pixel_to_ndc_y(y_px: usize, surface_height_px: u32) -> f32 {
    1.0 - (y_px as f32 / surface_height_px.max(1) as f32) * 2.0
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

fn generate_grid_atlas(device: &Device, queue: &Queue, grid_metrics: GridMetrics) -> GridAtlas {
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

    let mut scale_context = ScaleContext::new();
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
            let Some(glyph) = resolve_mono_glyph(ch, bold) else {
                continue;
            };
            let Some(image) = render_alpha_glyph(
                &mut scale_context,
                glyph,
                grid_metrics.text.font_size_px,
                true,
            ) else {
                continue;
            };
            let glyph_left = image.placement.left;
            let glyph_top = grid_metrics.text.baseline_px as i32 - image.placement.top as i32;

            for y in 0..image.placement.height {
                for x in 0..image.placement.width {
                    let src_idx = (y * image.placement.width + x) as usize;
                    let alpha = match image.data.len() {
                        len if len == (image.placement.width * image.placement.height) as usize => {
                            image.data[src_idx]
                        }
                        len if len
                            == (image.placement.width * image.placement.height * 4) as usize =>
                        {
                            image.data[src_idx * 4 + 3]
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

fn generate_logo_atlas(device: &Device, queue: &Queue, grid_metrics: GridMetrics) -> LogoAtlas {
    let mut shape_context = ShapeContext::new();
    let mut scale_context = ScaleContext::new();
    let rasterized = OverlayLogoKind::ALL.map(|kind| {
        rasterize_logo_sprite(kind, grid_metrics, &mut shape_context, &mut scale_context)
    });
    let atlas_width = rasterized
        .iter()
        .map(|sprite| sprite.width_px)
        .max()
        .unwrap_or(1)
        .max(1);
    let atlas_height = rasterized
        .iter()
        .map(|sprite| sprite.height_px)
        .sum::<u32>()
        .max(1);
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("nc-helm-logo-atlas"),
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
    let mut sprites = [LogoSprite::default(); LOGO_KIND_COUNT];
    let mut y_cursor = 0u32;
    for (index, raster) in rasterized.iter().enumerate() {
        sprites[index] = LogoSprite {
            x_px: 0,
            y_px: y_cursor,
            width_px: raster.width_px,
            height_px: raster.height_px,
        };
        for row in 0..raster.height_px {
            let dst_start = ((y_cursor + row) * atlas_width) as usize;
            let src_start = (row * raster.width_px) as usize;
            atlas_data[dst_start..dst_start + raster.width_px as usize]
                .copy_from_slice(&raster.pixels[src_start..src_start + raster.width_px as usize]);
        }
        y_cursor += raster.height_px;
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
    LogoAtlas {
        texture,
        view,
        width_px: atlas_width,
        height_px: atlas_height,
        sprites,
    }
}

#[derive(Clone, Debug)]
struct RasterizedLogo {
    width_px: u32,
    height_px: u32,
    pixels: Vec<u8>,
}

fn rasterize_logo_sprite(
    kind: OverlayLogoKind,
    grid_metrics: GridMetrics,
    shape_context: &mut ShapeContext,
    scale_context: &mut ScaleContext,
) -> RasterizedLogo {
    let (text, width_cols, height_rows) = logo_spec(kind);
    let width_px = (width_cols * grid_metrics.cell.width_px).max(1) as u32;
    let height_px = (height_rows * grid_metrics.cell.height_px).max(1) as u32;
    let font_size_px = fit_logo_font_size(text, width_px, height_px, shape_context, scale_context);
    if font_size_px <= 0.0 {
        return RasterizedLogo {
            width_px,
            height_px,
            pixels: vec![0; (width_px * height_px) as usize],
        };
    }
    let measured = measure_logo(text, font_size_px, shape_context, scale_context);
    let mut pixels = vec![0u8; (width_px * height_px) as usize];
    let offset_x = ((width_px as i32 - measured.bounds_width()) / 2) - measured.left_px;
    let offset_y = ((height_px as i32 - measured.bounds_height()) / 2) - measured.top_px;
    for glyph in measured.glyphs {
        blit_alpha_image(
            &mut pixels,
            width_px,
            height_px,
            glyph.left_px + offset_x,
            glyph.top_px + offset_y,
            &glyph.image,
        );
    }
    RasterizedLogo {
        width_px,
        height_px,
        pixels,
    }
}

fn fit_logo_font_size(
    text: &str,
    width_px: u32,
    height_px: u32,
    shape_context: &mut ShapeContext,
    scale_context: &mut ScaleContext,
) -> f32 {
    let mut low = 1usize;
    let mut high = height_px.saturating_mul(2) as usize;
    let mut best = 0usize;
    while low <= high {
        let mid = (low + high) / 2;
        let measured = measure_logo(text, mid as f32, shape_context, scale_context);
        if measured.bounds_width() <= width_px as i32
            && measured.bounds_height() <= height_px as i32
        {
            best = mid;
            low = mid.saturating_add(1);
        } else {
            high = mid.saturating_sub(1);
        }
    }
    best as f32
}

#[derive(Clone)]
struct MeasuredLogoGlyph {
    image: swash::scale::image::Image,
    left_px: i32,
    top_px: i32,
}

#[derive(Clone)]
struct MeasuredLogo {
    glyphs: Vec<MeasuredLogoGlyph>,
    left_px: i32,
    top_px: i32,
    right_px: i32,
    bottom_px: i32,
}

impl MeasuredLogo {
    fn bounds_width(&self) -> i32 {
        (self.right_px - self.left_px).max(1)
    }

    fn bounds_height(&self) -> i32 {
        (self.bottom_px - self.top_px).max(1)
    }
}

fn measure_logo(
    text: &str,
    font_size_px: f32,
    shape_context: &mut ShapeContext,
    scale_context: &mut ScaleContext,
) -> MeasuredLogo {
    let shaped = shape_stormfaze_text(shape_context, text, font_size_px);
    let baseline_px = shaped.ascent_px.round() as i32;
    let mut glyphs = Vec::new();
    let mut left_px = i32::MAX;
    let mut top_px = i32::MAX;
    let mut right_px = i32::MIN;
    let mut bottom_px = i32::MIN;
    for glyph in shaped.glyphs {
        let resolved = ResolvedGlyph {
            font: crate::fonts::stormfaze_font(),
            glyph_id: glyph.glyph_id,
            embolden: 0.0,
        };
        let Some(image) = render_alpha_glyph(scale_context, resolved, font_size_px, false) else {
            continue;
        };
        let glyph_left = glyph.x.round() as i32 + image.placement.left;
        let glyph_top = baseline_px + glyph.y.round() as i32 - image.placement.top as i32;
        left_px = left_px.min(glyph_left);
        top_px = top_px.min(glyph_top);
        right_px = right_px.max(glyph_left + image.placement.width as i32);
        bottom_px = bottom_px.max(glyph_top + image.placement.height as i32);
        glyphs.push(MeasuredLogoGlyph {
            image,
            left_px: glyph_left,
            top_px: glyph_top,
        });
    }
    if glyphs.is_empty() {
        return MeasuredLogo {
            glyphs,
            left_px: 0,
            top_px: 0,
            right_px: shaped.width_px.round().max(1.0) as i32,
            bottom_px: (shaped.ascent_px + shaped.descent_px).round().max(1.0) as i32,
        };
    }
    MeasuredLogo {
        glyphs,
        left_px,
        top_px,
        right_px,
        bottom_px,
    }
}

fn blit_alpha_image(
    dst: &mut [u8],
    dst_width_px: u32,
    dst_height_px: u32,
    dst_left_px: i32,
    dst_top_px: i32,
    image: &swash::scale::image::Image,
) {
    for y in 0..image.placement.height {
        for x in 0..image.placement.width {
            let src_idx = (y * image.placement.width + x) as usize;
            let alpha = match image.data.len() {
                len if len == (image.placement.width * image.placement.height) as usize => {
                    image.data[src_idx]
                }
                len if len == (image.placement.width * image.placement.height * 4) as usize => {
                    image.data[src_idx * 4 + 3]
                }
                _ => continue,
            };
            if alpha == 0 {
                continue;
            }
            let dst_x = dst_left_px + x as i32;
            let dst_y = dst_top_px + y as i32;
            if dst_x < 0
                || dst_y < 0
                || dst_x >= dst_width_px as i32
                || dst_y >= dst_height_px as i32
            {
                continue;
            }
            let dst_idx = dst_y as usize * dst_width_px as usize + dst_x as usize;
            dst[dst_idx] = dst[dst_idx].max(alpha);
        }
    }
}

fn logo_spec(kind: OverlayLogoKind) -> (&'static str, usize, usize) {
    match kind {
        OverlayLogoKind::HeaderWordmark => ("Nostrian Conquest", 22, 1),
        OverlayLogoKind::GateNostrian54x4 => ("NOSTRIAN", 54, 4),
        OverlayLogoKind::GateConquest54x4 => ("CONQUEST", 54, 4),
        OverlayLogoKind::GateNostrian62x4 => ("NOSTRIAN", 62, 4),
        OverlayLogoKind::GateConquest62x4 => ("CONQUEST", 62, 4),
        OverlayLogoKind::GateNostrian66x4 => ("NOSTRIAN", 66, 4),
        OverlayLogoKind::GateConquest66x4 => ("CONQUEST", 66, 4),
    }
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
        BACKGROUND_SHADER, GPU_STYLE_BOLD, GPU_STYLE_TEXT_BAND, GRID_ATLAS_BASE_GLYPH_COUNT,
        GRID_ATLAS_COLS, GRID_ATLAS_MISC_CHARS, GRID_ATLAS_ROWS, atlas_repertoire_chars,
        atlas_slot_contains_pixel, char_atlas_base_index, clear_color, linear_channel_from_srgb_u8,
        pack_gpu_style,
    };
    use crate::grid::{BackgroundMode, CellStyle, GameColor};

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
    fn unsupported_grid_glyphs_stay_out_of_the_gpu_repertoire() {
        assert!(char_atlas_base_index('A').is_some());
        assert!(char_atlas_base_index('┌').is_some());
        assert!(char_atlas_base_index('🙂').is_none());
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

    #[test]
    fn selection_shader_uses_directional_half_cell_vertical_stems() {
        assert!(BACKGROUND_SHADER.contains("var vertical_arm = false;"));
        assert_eq!(
            BACKGROUND_SHADER.matches("vertical_arm = local_y >= mid_y;").count(),
            2
        );
        assert_eq!(
            BACKGROUND_SHADER.matches("vertical_arm = local_y <= mid_y;").count(),
            2
        );
        assert!(BACKGROUND_SHADER.contains(
            "(on_horizontal && horizontal_arm) || (on_vertical && vertical_arm)"
        ));
        assert!(!BACKGROUND_SHADER.contains(
            "return select(0.0, 1.0, on_vertical || (on_horizontal && horizontal_arm));"
        ));
    }
}
