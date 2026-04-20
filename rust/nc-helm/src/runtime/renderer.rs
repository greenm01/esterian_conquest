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

use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Weight, fontdb,
};
use wgpu::{
    self, BindGroup, BindGroupLayout, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Sampler,
    SamplerDescriptor, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::event_loop::ActiveEventLoop;

use super::primitives;
use crate::geometry::{GridMapper, GridMetrics, PhysicalRect, TextMetrics, caret_rect};
use crate::grid::{
    BackgroundMode, Cell, CellStyle, GameColor, OverlayAnchor, OverlayText, OverlayTextFamily,
    PlayfieldBuffer, Point, ScreenGeometry,
};
use crate::theme;

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
    // wgpu NDC: y=+1 is top of screen, y=-1 is bottom.
    // wgpu texture coords: (0,0) is top-left, (0,1) is bottom-left.
    // Map screen top vertices (y=+1) to texture top (v=0), and screen bottom
    // vertices (y=-1) to texture bottom (v=1) so the background_pixels buffer
    // (row 0 at top) renders right-side-up.
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

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(background_tex, background_sampler, in.uv);
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
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: GameColor,
}

/// GPU-side resources for the background image: a 2D texture sized to the
/// surface, plus a pre-built bind group ready to attach during rendering.
struct BackgroundTexture {
    texture: Texture,
    bind_group: BindGroup,
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
    FullFrame,
}

#[derive(Clone, Debug, Default)]
struct PreparedFrame {
    placements: Vec<TextPlacement>,
    full_rebuild: bool,
    dirty_rows: usize,
    raw_spans: usize,
    upload_rects: usize,
    upload_strategy: UploadStrategy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DirtyCells {
    row_spans: Vec<Vec<DirtyColumnSpan>>,
    overlay_changed: bool,
    full_rebuild: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderTimings {
    pub playfield_prepare: Duration,
    pub glyph_prepare: Duration,
    pub gpu_submit_present: Duration,
    pub total: Duration,
    pub dirty_rows: usize,
    pub raw_spans: usize,
    pub upload_rects: usize,
    pub full_rebuild: bool,
    pub upload_strategy: UploadStrategy,
}

const DIRTY_SPAN_GAP_MERGE_CELLS: usize = 2;
const DIRTY_SPAN_COLLAPSE_THRESHOLD: usize = 4;
const MAX_UPLOAD_RECTS_BEFORE_ROW_FALLBACK: usize = 64;

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
    previous_playfield: Option<CachedPlayfield>,
    row_placements: Vec<Vec<TextPlacement>>,
    overlay_placements: Vec<TextPlacement>,
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
        let swash_cache = SwashCache::new();
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
            &background_bind_group_layout,
            &background_sampler,
            surface_config.width,
            surface_config.height,
        );
        let background_pixels =
            vec![0; surface_config.width as usize * surface_config.height as usize * 4];

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
            previous_playfield: None,
            row_placements: Vec::new(),
            overlay_placements: Vec::new(),
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
        let text_areas = prepared
            .placements
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
            pass.set_bind_group(0, &self.background_texture.bind_group, &[]);
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
            self.row_placements.clear();
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
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper =
            GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);
        let dirty = dirty_cells(
            self.previous_playfield.as_ref(),
            playfield,
            frame_width,
            frame_height,
        );

        let dirty_rows = dirty
            .row_spans
            .iter()
            .filter(|spans| !spans.is_empty())
            .count();
        let raw_spans = count_dirty_spans(&dirty.row_spans);

        if dirty.full_rebuild {
            self.fill_body_background(frame_width, frame_height);
            self.row_placements = vec![Vec::new(); playfield.height()];
            for row_idx in 0..playfield.height() {
                self.repaint_row_spans(
                    playfield,
                    mapper,
                    row_idx,
                    &[DirtyColumnSpan {
                        start_col: 0,
                        end_col: playfield.width(),
                    }],
                );
                self.row_placements[row_idx] =
                    self.collect_row_placements(playfield, mapper, row_idx);
            }
            let mut overlay_placements = Vec::new();
            prepare_overlay_texts(
                self,
                mapper,
                playfield.overlay_texts(),
                &mut overlay_placements,
            );
            self.overlay_placements = overlay_placements;
            if let Some(cursor) = playfield.cursor() {
                self.paint_cursor(frame_width, mapper, cursor);
            }
            self.write_full_background_texture();
            self.previous_playfield = Some(snapshot_playfield(playfield));
            let mut placements = Vec::new();
            for row in &self.row_placements {
                placements.extend(row.iter().cloned());
            }
            placements.extend(self.overlay_placements.iter().cloned());
            return PreparedFrame {
                placements,
                full_rebuild: true,
                dirty_rows,
                raw_spans,
                upload_rects: 1,
                upload_strategy: UploadStrategy::FullFrame,
            };
        } else {
            let compacted_row_spans = compact_row_spans(&dirty.row_spans);
            for (row_idx, spans) in compacted_row_spans.iter().enumerate() {
                if spans.is_empty() {
                    continue;
                }
                self.repaint_row_spans(playfield, mapper, row_idx, spans);
            }
            for (row_idx, spans) in dirty.row_spans.iter().enumerate() {
                if spans.is_empty() {
                    continue;
                }
                self.row_placements[row_idx] =
                    self.collect_row_placements(playfield, mapper, row_idx);
            }
            if dirty.overlay_changed {
                let mut overlay_placements = Vec::new();
                prepare_overlay_texts(
                    self,
                    mapper,
                    playfield.overlay_texts(),
                    &mut overlay_placements,
                );
                self.overlay_placements = overlay_placements;
            }
            if let Some(cursor) = playfield.cursor() {
                if dirty
                    .row_spans
                    .get(cursor.row.as_usize())
                    .is_some_and(|spans| !spans.is_empty())
                {
                    self.paint_cursor(frame_width, mapper, cursor);
                }
            }
            let compacted_rects = dirty_rectangles(&compacted_row_spans, mapper);
            let upload_strategy =
                choose_upload_strategy(compacted_rects.len(), dirty_rows, playfield.height());
            let upload_rects = match upload_strategy {
                UploadStrategy::Rects => {
                    if !compacted_rects.is_empty() {
                        self.write_dirty_rects(frame_width, &compacted_rects);
                    }
                    compacted_rects.len()
                }
                UploadStrategy::DirtyRows => {
                    let dirty_row_rects =
                        dirty_row_upload_rectangles(&compacted_row_spans, mapper, frame_width);
                    if !dirty_row_rects.is_empty() {
                        self.write_dirty_rects(frame_width, &dirty_row_rects);
                    }
                    dirty_row_rects.len()
                }
                UploadStrategy::FullFrame => {
                    self.write_full_background_texture();
                    1
                }
            };
            self.previous_playfield = Some(snapshot_playfield(playfield));
            let mut placements = Vec::new();
            for row in &self.row_placements {
                placements.extend(row.iter().cloned());
            }
            placements.extend(self.overlay_placements.iter().cloned());
            return PreparedFrame {
                placements,
                full_rebuild: false,
                dirty_rows,
                raw_spans,
                upload_rects,
                upload_strategy,
            };
        }
    }

    /// Shape `key` into a glyphon `Buffer` and cache it. No-op if already
    /// cached. Buffers are sized to one cell row in height; horizontal size
    /// is left unconstrained so wider runs lay out on a single line.
    fn ensure_text_buffer(&mut self, key: TextBufferKey) {
        if self.text_buffers.contains_key(&key) {
            return;
        }
        let mut buffer = GlyphBuffer::new(
            &mut self.font_system,
            Metrics::new(
                f32::from_bits(key.font_size_bits),
                f32::from_bits(key.line_height_bits),
            ),
        );
        buffer.set_size(
            &mut self.font_system,
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
            &mut self.font_system,
            key.text.as_ref(),
            &attrs,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        let overhang = measure_text_overhang(&mut self.font_system, &mut self.swash_cache, &buffer);
        self.text_buffers
            .insert(key, CachedTextCell { buffer, overhang });
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
            &self.background_bind_group_layout,
            &self.background_sampler,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.background_pixels.resize(
            self.surface_config.width as usize * self.surface_config.height as usize * 4,
            0,
        );
        self.previous_playfield = None;
        self.row_placements.clear();
        self.overlay_placements.clear();
    }

    fn fill_body_background(&mut self, frame_width: usize, frame_height: usize) {
        let body_bg = primitives::color_to_rgba(theme::app_background());
        self.background_pixels
            .resize(frame_width * frame_height * 4, 0);
        for pixel in self.background_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&body_bg);
        }
    }

    fn collect_row_placements(
        &mut self,
        playfield: &PlayfieldBuffer,
        mapper: GridMapper,
        row_idx: usize,
    ) -> Vec<TextPlacement> {
        let mut placements = Vec::new();
        let mut run_start = None;
        let mut run_style: Option<CellStyle> = None;
        let mut run_text = String::new();
        for col_idx in 0..playfield.width() {
            let source = playfield.row(row_idx)[col_idx];
            let style = source.style;
            if source.ch == ' ' {
                flush_run(
                    self,
                    mapper,
                    &mut placements,
                    &mut run_start,
                    &mut run_style,
                    &mut run_text,
                    row_idx,
                    col_idx,
                );
                continue;
            }
            if primitives::should_draw_as_primitive(source.ch) {
                flush_run(
                    self,
                    mapper,
                    &mut placements,
                    &mut run_start,
                    &mut run_style,
                    &mut run_text,
                    row_idx,
                    col_idx,
                );
                continue;
            }
            if run_style == Some(style) {
                if run_start.is_none() {
                    run_start = Some(col_idx);
                }
                run_text.push(source.ch);
            } else {
                flush_run(
                    self,
                    mapper,
                    &mut placements,
                    &mut run_start,
                    &mut run_style,
                    &mut run_text,
                    row_idx,
                    col_idx,
                );
                run_start = Some(col_idx);
                run_style = Some(style);
                run_text.push(source.ch);
            }
        }
        flush_run(
            self,
            mapper,
            &mut placements,
            &mut run_start,
            &mut run_style,
            &mut run_text,
            row_idx,
            playfield.width(),
        );
        placements
    }

    fn repaint_row_spans(
        &mut self,
        playfield: &PlayfieldBuffer,
        mapper: GridMapper,
        row_idx: usize,
        spans: &[DirtyColumnSpan],
    ) {
        let frame_width = self.surface_config.width as usize;
        let body_bg_color = theme::app_background();
        let body_bg = primitives::color_to_rgba(body_bg_color);
        for span in spans {
            if span.start_col >= span.end_col {
                continue;
            }
            let start_rect = mapper.cell_rect(Point::from_usize(span.start_col, row_idx));
            primitives::fill_rect_rgba(
                &mut self.background_pixels,
                frame_width,
                start_rect.x,
                start_rect.y,
                (span.end_col - span.start_col) * mapper.cell.width_px,
                mapper.cell.height_px,
                body_bg,
            );
            let mut background_run_start = None;
            let mut background_run_style = None;
            for col_idx in span.start_col..span.end_col {
                let source = playfield.row(row_idx)[col_idx];
                let style = source.style;
                let background_key = background_fill_key(style, body_bg_color);
                if background_key != background_run_style {
                    self.flush_background_run(
                        frame_width,
                        mapper,
                        row_idx,
                        &mut background_run_start,
                        &mut background_run_style,
                        col_idx,
                    );
                    if background_key.is_some() {
                        background_run_start = Some(col_idx);
                        background_run_style = background_key;
                    }
                }
                if primitives::should_draw_as_primitive(source.ch) {
                    let point = Point::from_usize(col_idx, row_idx);
                    let rect = if style.bg_mode == BackgroundMode::TextBand {
                        mapper.text_band_rect(point, self.grid_metrics.text)
                    } else {
                        mapper.cell_rect(point)
                    };
                    primitives::draw_cell_primitive(
                        &mut self.background_pixels,
                        frame_width,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        source.ch,
                        primitives::color_to_rgba(style.fg),
                    );
                }
            }
            self.flush_background_run(
                frame_width,
                mapper,
                row_idx,
                &mut background_run_start,
                &mut background_run_style,
                span.end_col,
            );
        }
    }

    fn flush_background_run(
        &mut self,
        frame_width: usize,
        mapper: GridMapper,
        row_idx: usize,
        run_start: &mut Option<usize>,
        run_style: &mut Option<(GameColor, BackgroundMode)>,
        end_col: usize,
    ) {
        let (Some(start_col), Some((bg, bg_mode))) = (run_start.take(), run_style.take()) else {
            return;
        };
        let rect = span_fill_rect(
            mapper,
            row_idx,
            start_col,
            end_col,
            bg_mode,
            self.grid_metrics.text,
        );
        primitives::fill_rect_rgba(
            &mut self.background_pixels,
            frame_width,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            primitives::color_to_rgba(bg),
        );
    }

    fn paint_cursor(&mut self, frame_width: usize, mapper: GridMapper, point: Point) {
        let caret = caret_rect(point, mapper, self.grid_metrics.text);
        primitives::fill_rect_rgba(
            &mut self.background_pixels,
            frame_width,
            caret.x,
            caret.y,
            caret.width,
            caret.height,
            primitives::color_to_rgba(GameColor::BrightWhite),
        );
    }

    fn write_full_background_texture(&self) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.background_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.background_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.surface_config.width * 4),
                rows_per_image: Some(self.surface_config.height),
            },
            wgpu::Extent3d {
                width: self.surface_config.width,
                height: self.surface_config.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn write_dirty_rects(&self, frame_width: usize, rects: &[DirtyPixelRect]) {
        for rect in rects {
            let start = (rect.top_px * frame_width + rect.left_px) * 4;
            let len = if rect.height_px == 0 {
                0
            } else {
                (rect.height_px - 1) * frame_width * 4 + rect.width_px * 4
            };
            if len == 0 {
                continue;
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.background_texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: rect.left_px as u32,
                        y: rect.top_px as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &self.background_pixels[start..start + len],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.surface_config.width * 4),
                    rows_per_image: Some(rect.height_px as u32),
                },
                wgpu::Extent3d {
                    width: rect.width_px as u32,
                    height: rect.height_px as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
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

fn dirty_cells(
    previous: Option<&CachedPlayfield>,
    playfield: &PlayfieldBuffer,
    frame_width: usize,
    frame_height: usize,
) -> DirtyCells {
    let Some(previous) = previous else {
        return DirtyCells {
            row_spans: vec![
                vec![DirtyColumnSpan {
                    start_col: 0,
                    end_col: playfield.width(),
                }];
                playfield.height()
            ],
            overlay_changed: true,
            full_rebuild: true,
        };
    };
    if previous.width != playfield.width()
        || previous.height != playfield.height()
        || frame_width == 0
        || frame_height == 0
    {
        return DirtyCells {
            row_spans: vec![
                vec![DirtyColumnSpan {
                    start_col: 0,
                    end_col: playfield.width(),
                }];
                playfield.height()
            ],
            overlay_changed: true,
            full_rebuild: true,
        };
    }

    let mut row_spans = vec![Vec::new(); playfield.height()];
    for row_idx in 0..playfield.height() {
        record_dirty_spans_for_row(
            &previous.rows[row_idx],
            playfield.row(row_idx),
            &mut row_spans[row_idx],
        );
    }
    if previous.cursor != playfield.cursor() {
        if let Some(cursor) = previous.cursor {
            push_dirty_cell(
                &mut row_spans,
                cursor.row.as_usize(),
                cursor.column.as_usize(),
                playfield.width(),
            );
        }
        if let Some(cursor) = playfield.cursor() {
            push_dirty_cell(
                &mut row_spans,
                cursor.row.as_usize(),
                cursor.column.as_usize(),
                playfield.width(),
            );
        }
    }
    for spans in &mut row_spans {
        merge_column_spans(spans);
    }
    DirtyCells {
        row_spans,
        overlay_changed: previous.overlay_texts != playfield.overlay_texts(),
        full_rebuild: false,
    }
}

fn count_dirty_spans(row_spans: &[Vec<DirtyColumnSpan>]) -> usize {
    row_spans.iter().map(Vec::len).sum()
}

fn compact_row_spans(row_spans: &[Vec<DirtyColumnSpan>]) -> Vec<Vec<DirtyColumnSpan>> {
    row_spans
        .iter()
        .map(|spans| compact_spans_for_row(spans))
        .collect()
}

fn compact_spans_for_row(spans: &[DirtyColumnSpan]) -> Vec<DirtyColumnSpan> {
    if spans.is_empty() {
        return Vec::new();
    }
    let mut compacted = Vec::with_capacity(spans.len());
    let mut current = spans[0];
    for span in spans.iter().copied().skip(1) {
        if span.start_col <= current.end_col.saturating_add(DIRTY_SPAN_GAP_MERGE_CELLS) {
            current.end_col = current.end_col.max(span.end_col);
        } else {
            compacted.push(current);
            current = span;
        }
    }
    compacted.push(current);
    if compacted.len() > DIRTY_SPAN_COLLAPSE_THRESHOLD {
        return vec![DirtyColumnSpan {
            start_col: compacted.first().map(|span| span.start_col).unwrap_or(0),
            end_col: compacted.last().map(|span| span.end_col).unwrap_or(0),
        }];
    }
    compacted
}

fn record_dirty_spans_for_row(previous: &[Cell], next: &[Cell], spans: &mut Vec<DirtyColumnSpan>) {
    let mut start_col = None;
    for col_idx in 0..next.len() {
        if previous[col_idx] != next[col_idx] {
            start_col.get_or_insert(col_idx);
        } else if let Some(start_col) = start_col.take() {
            spans.push(DirtyColumnSpan {
                start_col,
                end_col: col_idx,
            });
        }
    }
    if let Some(start_col) = start_col {
        spans.push(DirtyColumnSpan {
            start_col,
            end_col: next.len(),
        });
    }
}

fn push_dirty_cell(
    row_spans: &mut [Vec<DirtyColumnSpan>],
    row_idx: usize,
    col_idx: usize,
    width: usize,
) {
    if row_idx >= row_spans.len() || col_idx >= width {
        return;
    }
    row_spans[row_idx].push(DirtyColumnSpan {
        start_col: col_idx,
        end_col: col_idx + 1,
    });
}

fn merge_column_spans(spans: &mut Vec<DirtyColumnSpan>) {
    if spans.len() <= 1 {
        return;
    }
    spans.sort_unstable_by_key(|span| span.start_col);
    let mut merged = Vec::with_capacity(spans.len());
    let mut current = spans[0];
    for span in spans.iter().copied().skip(1) {
        if span.start_col <= current.end_col {
            current.end_col = current.end_col.max(span.end_col);
        } else {
            merged.push(current);
            current = span;
        }
    }
    merged.push(current);
    *spans = merged;
}

fn choose_upload_strategy(
    compacted_rect_count: usize,
    dirty_rows: usize,
    playfield_height: usize,
) -> UploadStrategy {
    if dirty_rows == 0 {
        return UploadStrategy::Rects;
    }
    if dirty_rows.saturating_mul(2) > playfield_height {
        return UploadStrategy::FullFrame;
    }
    if compacted_rect_count > MAX_UPLOAD_RECTS_BEFORE_ROW_FALLBACK {
        return UploadStrategy::DirtyRows;
    }
    UploadStrategy::Rects
}

fn dirty_row_upload_rectangles(
    row_spans: &[Vec<DirtyColumnSpan>],
    mapper: GridMapper,
    frame_width: usize,
) -> Vec<DirtyPixelRect> {
    let mut rects = Vec::new();
    let mut current_start = None;
    for (row_idx, spans) in row_spans.iter().enumerate() {
        if spans.is_empty() {
            if let Some(start_row) = current_start.take() {
                rects.push(DirtyPixelRect {
                    left_px: 0,
                    top_px: mapper.origin_y + start_row * mapper.cell.height_px,
                    width_px: frame_width,
                    height_px: (row_idx - start_row) * mapper.cell.height_px,
                });
            }
            continue;
        }
        current_start.get_or_insert(row_idx);
    }
    if let Some(start_row) = current_start {
        rects.push(DirtyPixelRect {
            left_px: 0,
            top_px: mapper.origin_y + start_row * mapper.cell.height_px,
            width_px: frame_width,
            height_px: (row_spans.len() - start_row) * mapper.cell.height_px,
        });
    }
    rects
}

fn dirty_rectangles(row_spans: &[Vec<DirtyColumnSpan>], mapper: GridMapper) -> Vec<DirtyPixelRect> {
    #[derive(Clone, Copy, Debug)]
    struct GridRect {
        left_col: usize,
        right_col: usize,
        top_row: usize,
        bottom_row: usize,
    }

    let mut rects = Vec::new();
    let mut active: Vec<GridRect> = Vec::new();
    for (row_idx, spans) in row_spans.iter().enumerate() {
        let mut next_active: Vec<GridRect> = Vec::new();
        for span in spans {
            let mut merged = GridRect {
                left_col: span.start_col,
                right_col: span.end_col,
                top_row: row_idx,
                bottom_row: row_idx + 1,
            };
            let mut carry = Vec::new();
            for active_rect in active.drain(..) {
                if merged.left_col <= active_rect.right_col.saturating_add(1)
                    && active_rect.left_col <= merged.right_col.saturating_add(1)
                {
                    merged.left_col = merged.left_col.min(active_rect.left_col);
                    merged.right_col = merged.right_col.max(active_rect.right_col);
                    merged.top_row = merged.top_row.min(active_rect.top_row);
                    merged.bottom_row = merged.bottom_row.max(active_rect.bottom_row);
                } else {
                    carry.push(active_rect);
                }
            }
            active = carry;
            let mut idx = 0;
            while idx < next_active.len() {
                let existing = next_active[idx];
                if merged.left_col <= existing.right_col.saturating_add(1)
                    && existing.left_col <= merged.right_col.saturating_add(1)
                {
                    merged.left_col = merged.left_col.min(existing.left_col);
                    merged.right_col = merged.right_col.max(existing.right_col);
                    merged.top_row = merged.top_row.min(existing.top_row);
                    merged.bottom_row = merged.bottom_row.max(existing.bottom_row);
                    next_active.swap_remove(idx);
                } else {
                    idx += 1;
                }
            }
            next_active.push(merged);
        }
        rects.extend(active.drain(..).map(|rect| DirtyPixelRect {
            left_px: mapper.origin_x + rect.left_col * mapper.cell.width_px,
            top_px: mapper.origin_y + rect.top_row * mapper.cell.height_px,
            width_px: (rect.right_col - rect.left_col) * mapper.cell.width_px,
            height_px: (rect.bottom_row - rect.top_row) * mapper.cell.height_px,
        }));
        active = next_active;
    }
    rects.extend(active.drain(..).map(|rect| DirtyPixelRect {
        left_px: mapper.origin_x + rect.left_col * mapper.cell.width_px,
        top_px: mapper.origin_y + rect.top_row * mapper.cell.height_px,
        width_px: (rect.right_col - rect.left_col) * mapper.cell.width_px,
        height_px: (rect.bottom_row - rect.top_row) * mapper.cell.height_px,
    }));
    rects
}

fn background_fill_key(
    style: CellStyle,
    body_bg_color: GameColor,
) -> Option<(GameColor, BackgroundMode)> {
    if style.bg_mode == BackgroundMode::Cell && style.bg == body_bg_color {
        return None;
    }
    if style.bg == body_bg_color {
        return None;
    }
    Some((style.bg, style.bg_mode))
}

fn span_fill_rect(
    mapper: GridMapper,
    row_idx: usize,
    start_col: usize,
    end_col: usize,
    bg_mode: BackgroundMode,
    text_metrics: TextMetrics,
) -> PhysicalRect {
    let start = Point::from_usize(start_col, row_idx);
    let end = Point::from_usize(end_col.saturating_sub(1), row_idx);
    let start_rect = if bg_mode == BackgroundMode::TextBand {
        mapper.text_band_rect(start, text_metrics)
    } else {
        mapper.cell_rect(start)
    };
    let end_rect = if bg_mode == BackgroundMode::TextBand {
        mapper.text_band_rect(end, text_metrics)
    } else {
        mapper.cell_rect(end)
    };
    PhysicalRect {
        x: start_rect.x,
        y: start_rect.y,
        width: end_rect.x + end_rect.width - start_rect.x,
        height: start_rect.height,
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
        bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nc-helm-background-bind-group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self {
            texture,
            bind_group,
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

/// Finalise an in-progress text run, if any.
///
/// Looks up the text origin and bounding box for the run's cell range and
/// pushes a `TextPlacement` so glyphon can draw it. The bounds clip the run
/// to its cell range horizontally and to the cell row vertically — this
/// prevents glyphs that overshoot their cell (wide italics, ligatures) from
/// painting into a neighbour's strip.
///
/// Resets `run_start`, `run_style`, and `run_text` so the caller can begin
/// the next run.
fn flush_run(
    renderer: &mut Renderer,
    mapper: GridMapper,
    placements: &mut Vec<TextPlacement>,
    run_start: &mut Option<usize>,
    run_style: &mut Option<CellStyle>,
    run_text: &mut String,
    row_idx: usize,
    end_col: usize,
) {
    let Some(start_col) = run_start.take() else {
        return;
    };
    let style = run_style.take().expect("run style exists");
    if run_text.is_empty() {
        return;
    }
    let key = make_text_key(
        Arc::<str>::from(run_text.as_str()),
        TextFamilyKey::Monospace,
        renderer.grid_metrics.text.font_size_px,
        renderer.grid_metrics.text.line_height_px,
        style.bold,
    );
    renderer.ensure_text_buffer(key.clone());
    let overhang = renderer
        .text_buffers
        .get(&key)
        .map(|cached| cached.overhang)
        .expect("text buffer exists after shaping");
    let start = Point::from_usize(start_col, row_idx);
    let text_origin = mapper.text_origin(start, renderer.grid_metrics.text);
    let cell_rect = mapper.cell_rect(start);
    let start_rect = if style.bg_mode == BackgroundMode::TextBand {
        mapper.text_band_rect(start, renderer.grid_metrics.text)
    } else {
        cell_rect
    };
    placements.push(TextPlacement {
        key,
        left: text_origin.left,
        top: text_origin.top,
        bounds: expanded_text_bounds(
            text_origin.left.floor().max(0.0) as usize,
            start_rect.x,
            start_rect.y,
            mapper.origin_x + end_col * mapper.cell.width_px,
            mapper.cell.height_px,
            renderer.surface_config.width as usize,
            renderer.surface_config.height as usize,
            overhang,
        ),
        color: style.fg,
    });
    run_text.clear();
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
            let Some(image) = swash_cache.get_image_uncached(font_system, physical.cache_key)
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

#[cfg(test)]
mod tests {
    use glyphon::{Attrs, Buffer as GlyphBuffer, Family, Metrics, Shaping, fontdb};

    use crate::geometry::{GridMapper, GridMetrics, caret_rect};
    use crate::grid::{
        CellStyle, GameColor, OverlayTextFamily, PlayfieldBuffer, Point, ScreenGeometry,
    };

    use super::{
        DIRTY_SPAN_COLLAPSE_THRESHOLD, DirtyColumnSpan, DirtyPixelRect, PRIMARY_FONT_FAMILY,
        STORMFAZE_FONT_FAMILY, TextFamilyKey, TextOverhang, UploadStrategy, build_font_system,
        choose_upload_strategy, compact_row_spans, dirty_cells, dirty_rectangles,
        dirty_row_upload_rectangles, expanded_text_bounds, fit_grid_to_pixels,
        measure_single_line_width, snapshot_playfield,
    };

    fn base_style() -> CellStyle {
        CellStyle::new(GameColor::White, GameColor::Black, false)
    }

    fn mapper() -> (GridMapper, GridMetrics) {
        let metrics = GridMetrics {
            cell: crate::geometry::CellMetrics {
                width_px: 12,
                height_px: 24,
            },
            text: crate::geometry::TextMetrics {
                font_size_px: 18.0,
                line_height_px: 24.0,
                baseline_px: 18,
                band_top_px: 5,
                band_height_px: 14,
            },
        };
        (
            GridMapper::centered(1200, 900, ScreenGeometry::new(100, 36), metrics.cell),
            metrics,
        )
    }

    #[test]
    fn caret_rect_tracks_columns_by_cell_width() {
        let (mapper, metrics) = mapper();
        let start = caret_rect(Point::from_usize(31, 16), mapper, metrics.text);
        let next = caret_rect(Point::from_usize(32, 16), mapper, metrics.text);
        assert_eq!(next.x - start.x, metrics.cell.width_px);
        assert_eq!(start.y, next.y);
    }

    #[test]
    fn caret_rect_tracks_rows_by_cell_height() {
        let (mapper, metrics) = mapper();
        let handle = caret_rect(Point::from_usize(31, 16), mapper, metrics.text);
        let password = caret_rect(Point::from_usize(31, 18), mapper, metrics.text);
        assert_eq!(password.x, handle.x);
        assert_eq!(password.y - handle.y, metrics.cell.height_px * 2);
    }

    #[test]
    fn caret_rect_uses_text_origin_from_cell_mapping() {
        let (mapper, metrics) = mapper();
        let rect = mapper.text_band_rect(Point::from_usize(31, 16), metrics.text);
        let caret = caret_rect(Point::from_usize(31, 16), mapper, metrics.text);
        assert_eq!(caret.x, rect.x);
        assert_eq!(caret.y, rect.y);
        assert_eq!(caret.width, 2);
        assert_eq!(caret.height, rect.height);
    }

    #[test]
    fn expanded_text_bounds_include_measured_overhang() {
        let bounds = expanded_text_bounds(
            101,
            100,
            50,
            148,
            24,
            500,
            400,
            TextOverhang {
                left_px: 1,
                right_px: 2,
                advance_width_px: 48,
            },
        );
        assert_eq!(bounds.left, 99);
        assert_eq!(bounds.right, 151);
        assert_eq!(bounds.top, 50);
        assert_eq!(bounds.bottom, 74);
    }

    #[test]
    fn expanded_text_bounds_clamp_to_frame_edges() {
        let bounds = expanded_text_bounds(
            2,
            0,
            0,
            120,
            24,
            120,
            80,
            TextOverhang {
                left_px: 2,
                right_px: 4,
                advance_width_px: 120,
            },
        );
        assert_eq!(bounds.left, 0);
        assert_eq!(bounds.right, 120);
        assert_eq!(bounds.bottom, 24);
    }

    #[test]
    fn stormfaze_wordmark_shapes_with_nonzero_width() {
        let mut font_system = build_font_system();
        let width = measure_single_line_width(
            &mut font_system,
            "NOSTRIAN",
            TextFamilyKey::Named(STORMFAZE_FONT_FAMILY),
            48.0,
        );
        assert!(
            width > 0.0,
            "Stormfaze wordmark should shape to a visible width"
        );
    }

    #[test]
    fn stormfaze_query_resolves_to_bundled_face() {
        let font_system = build_font_system();
        let face_id = font_system.db().query(&fontdb::Query {
            families: &[fontdb::Family::Name(STORMFAZE_FONT_FAMILY)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        });
        let face = face_id
            .and_then(|id| font_system.db().face(id))
            .expect("Stormfaze face should be present in the bundled font database");
        assert!(
            face.families
                .iter()
                .any(|(family, _)| family == STORMFAZE_FONT_FAMILY),
            "resolved face should belong to the Stormfaze family"
        );
    }

    #[test]
    fn monospace_query_resolves_to_jetbrains_mono() {
        let font_system = build_font_system();
        let face_id = font_system.db().query(&fontdb::Query {
            families: &[fontdb::Family::Monospace],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        });
        let face = face_id
            .and_then(|id| font_system.db().face(id))
            .expect("monospace query should resolve to the bundled primary face");
        assert!(
            face.families
                .iter()
                .any(|(family, _)| family == PRIMARY_FONT_FAMILY),
            "monospace query should resolve to JetBrains Mono"
        );
    }

    #[test]
    fn fit_grid_to_pixels_is_stable_for_repeated_inputs() {
        let cell = crate::geometry::CellMetrics {
            width_px: 12,
            height_px: 24,
        };
        let first = fit_grid_to_pixels(1200, 900, cell);
        let second = fit_grid_to_pixels(1200, 900, cell);
        assert_eq!(first, second);
        assert_eq!(first, ScreenGeometry::new(100, 37));
    }

    #[test]
    fn dirty_cells_track_changed_rows_and_cursor_rows() {
        let mut previous = PlayfieldBuffer::new(4, 3, base_style());
        previous.write_text(1, 0, "AB", base_style());
        previous.set_cursor(Point::from_usize(0, 0));
        let previous = snapshot_playfield(&previous);

        let mut next = PlayfieldBuffer::new(4, 3, base_style());
        next.write_text(1, 0, "AX", base_style());
        next.set_cursor(Point::from_usize(1, 2));

        let dirty = dirty_cells(Some(&previous), &next, 640, 480);
        assert!(!dirty.full_rebuild);
        assert_eq!(
            dirty.row_spans,
            vec![
                vec![DirtyColumnSpan {
                    start_col: 0,
                    end_col: 1
                }],
                vec![DirtyColumnSpan {
                    start_col: 1,
                    end_col: 2
                }],
                vec![DirtyColumnSpan {
                    start_col: 1,
                    end_col: 2
                }],
            ]
        );
    }

    #[test]
    fn dirty_cells_keep_background_incremental_when_only_overlay_text_changes() {
        let mut previous = PlayfieldBuffer::new(4, 3, base_style());
        previous.push_overlay_text("NC", OverlayTextFamily::Monospace, base_style(), 0, 0, 2, 1);
        let previous = snapshot_playfield(&previous);

        let next = PlayfieldBuffer::new(4, 3, base_style());
        let dirty = dirty_cells(Some(&previous), &next, 640, 480);
        assert!(!dirty.full_rebuild);
        assert!(dirty.overlay_changed);
        assert!(dirty.row_spans.iter().all(|spans| spans.is_empty()));
    }

    #[test]
    fn compact_row_spans_merge_small_gaps() {
        let row_spans = vec![vec![
            DirtyColumnSpan {
                start_col: 1,
                end_col: 2,
            },
            DirtyColumnSpan {
                start_col: 4,
                end_col: 5,
            },
            DirtyColumnSpan {
                start_col: 8,
                end_col: 9,
            },
        ]];
        assert_eq!(
            compact_row_spans(&row_spans),
            vec![vec![
                DirtyColumnSpan {
                    start_col: 1,
                    end_col: 5,
                },
                DirtyColumnSpan {
                    start_col: 8,
                    end_col: 9,
                },
            ]]
        );
    }

    #[test]
    fn compact_row_spans_collapse_busy_rows() {
        let row_spans = vec![vec![
            DirtyColumnSpan {
                start_col: 0,
                end_col: 1,
            },
            DirtyColumnSpan {
                start_col: 4,
                end_col: 5,
            },
            DirtyColumnSpan {
                start_col: 8,
                end_col: 9,
            },
            DirtyColumnSpan {
                start_col: 12,
                end_col: 13,
            },
            DirtyColumnSpan {
                start_col: 16,
                end_col: 17,
            },
        ]];
        let compacted = compact_row_spans(&row_spans);
        assert_eq!(compacted[0].len(), 1);
        assert!(DIRTY_SPAN_COLLAPSE_THRESHOLD < row_spans[0].len());
        assert_eq!(
            compacted[0][0],
            DirtyColumnSpan {
                start_col: 0,
                end_col: 17,
            }
        );
    }

    #[test]
    fn dirty_rectangles_merge_adjacent_rows_with_overlapping_columns() {
        let (mapper, _) = mapper();
        let mut row_spans = vec![Vec::new(); 6];
        row_spans[2].push(DirtyColumnSpan {
            start_col: 0,
            end_col: 2,
        });
        row_spans[3].push(DirtyColumnSpan {
            start_col: 1,
            end_col: 3,
        });
        row_spans[5].push(DirtyColumnSpan {
            start_col: 2,
            end_col: 3,
        });
        let rects = dirty_rectangles(&row_spans, mapper);
        assert_eq!(
            rects,
            vec![
                DirtyPixelRect {
                    left_px: mapper.origin_x,
                    top_px: mapper.origin_y + 2 * mapper.cell.height_px,
                    width_px: 3 * mapper.cell.width_px,
                    height_px: 2 * mapper.cell.height_px,
                },
                DirtyPixelRect {
                    left_px: mapper.origin_x + 2 * mapper.cell.width_px,
                    top_px: mapper.origin_y + 5 * mapper.cell.height_px,
                    width_px: mapper.cell.width_px,
                    height_px: mapper.cell.height_px,
                },
            ]
        );
    }

    #[test]
    fn dirty_row_upload_rectangles_merge_contiguous_rows() {
        let (mapper, _) = mapper();
        let mut row_spans = vec![Vec::new(); 5];
        row_spans[1].push(DirtyColumnSpan {
            start_col: 1,
            end_col: 2,
        });
        row_spans[2].push(DirtyColumnSpan {
            start_col: 4,
            end_col: 6,
        });
        row_spans[4].push(DirtyColumnSpan {
            start_col: 2,
            end_col: 3,
        });
        let rects = dirty_row_upload_rectangles(&row_spans, mapper, 1200);
        assert_eq!(
            rects,
            vec![
                DirtyPixelRect {
                    left_px: 0,
                    top_px: mapper.origin_y + mapper.cell.height_px,
                    width_px: 1200,
                    height_px: 2 * mapper.cell.height_px,
                },
                DirtyPixelRect {
                    left_px: 0,
                    top_px: mapper.origin_y + 4 * mapper.cell.height_px,
                    width_px: 1200,
                    height_px: mapper.cell.height_px,
                },
            ]
        );
    }

    #[test]
    fn upload_strategy_uses_dirty_rows_when_rects_explode() {
        assert_eq!(choose_upload_strategy(65, 8, 40), UploadStrategy::DirtyRows);
    }

    #[test]
    fn upload_strategy_uses_full_frame_when_rows_exceed_half_height() {
        assert_eq!(
            choose_upload_strategy(10, 11, 20),
            UploadStrategy::FullFrame
        );
    }

    #[test]
    fn stormfaze_wordmark_glyphs_use_stormfaze_face() {
        let mut font_system = build_font_system();
        let mut buffer = GlyphBuffer::new(&mut font_system, Metrics::new(48.0, 48.0));
        buffer.set_size(&mut font_system, None, Some(48.0));
        buffer.set_text(
            &mut font_system,
            "NOSTRIAN",
            &Attrs::new().family(Family::Name(STORMFAZE_FONT_FAMILY)),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        let font_id = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .next()
            .map(|glyph| glyph.font_id)
            .expect("Stormfaze wordmark should produce at least one glyph");
        let face = font_system
            .db()
            .face(font_id)
            .expect("shaped Stormfaze glyph should resolve to a known face");
        assert!(
            face.families
                .iter()
                .any(|(family, _)| family == STORMFAZE_FONT_FAMILY),
            "glyphs should resolve to the Stormfaze face instead of a fallback family"
        );
    }
}
