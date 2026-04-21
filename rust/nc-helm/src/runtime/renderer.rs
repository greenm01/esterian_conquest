//! wgpu + glyphon GPU renderer for the nc-helm character grid.
//!
//! Each frame the renderer paints two layers:
//!
//! 1. **Background pixel buffer** (`background_pixels`): a CPU-side RGBA
//!    image the size of the surface. The playfield's per-cell background
//!    colour, `BackgroundMode::TextBand` strips, and the caret are written
//!    into this buffer with `fill_rect_rgba`. Unchanged cells are reused
//!    across frames; dirty rectangles are uploaded back to the GPU texture
//!    and drawn first as a fullscreen quad via the [`BACKGROUND_SHADER`]
//!    pipeline.
//! 2. **Glyph layer**: text runs are batched per (text, weight) into shaped
//!    glyphon `Buffer`s, cached in `text_buffers`, and drawn on top by the
//!    `glyphon::TextRenderer` using pixel-space coordinates from
//!    `GridMapper::text_origin`.
//!
//! Coordinate conventions:
//! - The CPU background buffer is row-major with row 0 at the top
//!   (y-down), matching `GridMapper`.
//! - wgpu NDC has y=+1 at the top of the screen and texture coords have
//!   v=0 at the top, so the WGSL vertex shader pairs each NDC corner with
//!   the matching texture corner (see [`BACKGROUND_SHADER`]).
//! - glyphon takes pixel coordinates in the same y-down space.

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
use crate::geometry::{GridMapper, GridMetrics, TextMetrics};
use crate::grid::{
    Cell, GameColor, OverlayAnchor, OverlayText, OverlayTextFamily, PlayfieldBuffer, Point,
    ScreenGeometry,
};
use crate::theme;

/// A GPU-ready representation of a single grid cell.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GpuCell {
    ch: u32,
    fg: u32,
    bg: u32,
    // style bits: bit 0: bold, bit 1: dim, bit 2: italic, bit 3: underline
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
    origin_x: f32,
    origin_y: f32,
    _padding: [u32; 2],
}

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
    origin_x: f32,
    origin_y: f32,
};

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

fn get_atlas_index(char_val: u32) -> i32 {
    if (char_val >= 32u && char_val < 127u) {
        return i32(char_val - 32u);
    }
    if (char_val >= 0x2500u && char_val < 0x2580u) {
        return i32(char_val - 0x2500u + 95u);
    }
    return -1;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let screen_px = in.position.xy;
    
    let gx_px = screen_px.x - grid_config.origin_x;
    let gy_px = screen_px.y - grid_config.origin_y;
    
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
    
    var color = unpack4x8unorm(cell.bg);
    
    let atlas_idx = get_atlas_index(cell.char_val);
    if (atlas_idx >= 0) {
        let cols = 1024u / 32u;
        let col = u32(atlas_idx) % cols;
        let row = u32(atlas_idx) / cols;
        
        let cell_local_x = gx_px % f32(grid_config.cell_width_px);
        let cell_local_y = gy_px % f32(grid_config.cell_height_px);
        
        // Slot is 32x64. Assume glyphs were centered in these slots.
        let slot_u_offset = (32.0 - f32(grid_config.cell_width_px)) / 2.0;
        let slot_v_offset = (64.0 - f32(grid_config.cell_height_px)) / 2.0;
        
        let atlas_u = (f32(col * 32u) + slot_u_offset + cell_local_x) / 1024.0;
        let atlas_v = (f32(row * 64u) + slot_v_offset + cell_local_y) / 1024.0;
        
        let glyph_alpha = textureSample(grid_atlas, background_sampler, vec2<f32>(atlas_u, atlas_v)).r;
        let fg_color = unpack4x8unorm(cell.fg);
        
        // Simple alpha blending
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
    background_pixels: Vec<u8>,
    grid_buffer: Buffer,
    grid_config_buffer: Buffer,
    grid_bind_group: BindGroup,
    grid_atlas: GridAtlas,
    previous_playfield: Option<CachedPlayfield>,
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
        let background_texture = BackgroundTexture::new(
            &device,
            surface_config.width,
            surface_config.height,
        );
        let background_pixels =
            vec![0; surface_config.width as usize * surface_config.height as usize * 4];

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

        let grid_atlas =
            generate_grid_atlas(&device, &queue, &mut font_system, &mut swash_cache, grid_metrics.text);

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nc-helm-grid-bind-group"),
            layout: &background_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&background_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&background_sampler),
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
        });

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
            background_pixels,
            grid_buffer,
            grid_config_buffer,
            grid_bind_group,
            grid_atlas,
            previous_playfield: None,
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
                    style: if cell.style.bold { 1 } else { 0 },
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
            origin_x: mapper.origin_x as f32,
            origin_y: mapper.origin_y as f32,
            _padding: [0; 2],
        };
        self.queue
            .write_buffer(&self.grid_config_buffer, 0, bytemuck::bytes_of(&config));
    }

    /// Render one frame of `playfield`.
    ///
    /// Steps, in order:
    /// 1. Sync DPI metrics and reconfigure the surface if the window resized.
    /// 2. Diff the playfield against the previous frame, repaint dirty rows
    ///    into `background_pixels`, and collect text runs as
    ///    `TextPlacement`s (see `prepare_playfield`).
    /// 3. Upload the changed background rows to the GPU texture and
    ///    `prepare` the glyphon `TextRenderer` with the staged runs.
    /// 4. Acquire the swapchain frame, draw the background quad, then the
    ///    glyph layer on top, and present.
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
            .overlay_placements
            .iter()
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
            self.text_buffers.clear();
            self.previous_playfield = None;
            self.overlay_placements.clear();
        }
    }

    /// Paint dirty playfield rows into `background_pixels`, batch adjacent
    /// same-style cells into glyphon text runs, and upload the changed
    /// background rows to the GPU.
    ///
    /// Returns the staged text placements; the caller hands them to
    /// `glyphon::TextRenderer::prepare`.
    ///
    /// Run batching: contiguous non-space cells with identical `CellStyle`
    /// become one shaped run. Spaces flush the current run because they
    /// don't need glyph rendering — the background fill alone suffices, and
    /// breaking on spaces lets adjacent runs differ in style without a
    /// per-cell shaping cost.
    fn prepare_playfield(&mut self, playfield: &PlayfieldBuffer) -> PreparedFrame {
        self.text_buffer_misses = 0;
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper =
            GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);

        let dirty_overlay = self.previous_playfield.as_ref().map_or(true, |prev| {
            prev.overlay_texts != playfield.overlay_texts()
        });

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
        self.background_pixels.resize(
            self.surface_config.width as usize * self.surface_config.height as usize * 4,
            0,
        );
        self.previous_playfield = None;
        self.overlay_placements.clear();
    }
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
    fn new(
        device: &Device,
        width: u32,
        height: u32,
    ) -> Self {
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
        Self {
            texture,
            view,
        }
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

fn clear_color(color: GameColor) -> wgpu::Color {
    let [r, g, b, a] = primitives::color_to_rgba(color);
    wgpu::Color {
        r: f64::from(r) / 255.0,
        g: f64::from(g) / 255.0,
        b: f64::from(b) / 255.0,
        a: f64::from(a) / 255.0,
    }
}

fn generate_grid_atlas(
    device: &Device,
    queue: &Queue,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    metrics: TextMetrics,
) -> GridAtlas {
    let atlas_width = 1024u32;
    let atlas_height = 1024u32;

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

    // We'll map characters to 32x64 slots in the atlas for simplicity.
    let slot_w = 32u32;
    let slot_h = 64u32;
    let cols = atlas_width / slot_w;

    // Rasterize ASCII range and Box Drawing range
    let chars_to_atlas: Vec<u32> = (32..127).chain(0x2500..0x2580).collect();

    for (i, &ch) in chars_to_atlas.iter().enumerate() {
        let mut buffer = GlyphBuffer::new(
            font_system,
            Metrics::new(metrics.font_size_px, metrics.line_height_px),
        );
        buffer.set_size(font_system, None, Some(metrics.line_height_px));
        buffer.set_text(
            font_system,
            &char::from_u32(ch).unwrap_or(' ').to_string(),
            &Attrs::new()
                .family(Family::Monospace)
                .weight(Weight::NORMAL),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(font_system, false);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                let Some(img) = swash_cache.get_image_uncached(font_system, physical.cache_key)
                else {
                    continue;
                };

                // Check if the image is a mask (1 byte per pixel) to avoid
                // direct swash dependency and potential API version mismatches.
                if img.data.len() == (img.placement.width * img.placement.height) as usize {
                    let col = (i as u32) % cols;
                    let row = (i as u32) / cols;

                    // Center the glyph in the 32x64 slot.
                    let ox = col * slot_w + (slot_w.saturating_sub(img.placement.width) / 2);
                    let oy = row * slot_h + (slot_h.saturating_sub(img.placement.height) / 2);

                    for y in 0..img.placement.height {
                        for x in 0..img.placement.width {
                            let src_idx = (y * img.placement.width + x) as usize;
                            let dst_x = ox + x;
                            let dst_y = oy + y;
                            if dst_x < atlas_width && dst_y < atlas_height {
                                let dst_idx = (dst_y * atlas_width + dst_x) as usize;
                                atlas_data[dst_idx] = img.data[src_idx];
                            }
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
