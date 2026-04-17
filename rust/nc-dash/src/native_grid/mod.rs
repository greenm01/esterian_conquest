mod primitives;

use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::OnceLock;

use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight, fontdb,
};
use tracing::info;
use wgpu::{
    self, BindGroup, BindGroupLayout, BlendState, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, FragmentState,
    Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PresentMode, PrimitiveState, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, SurfaceConfiguration,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::event_loop::ActiveEventLoop;

use crate::buffer::{CellStyle, PlayfieldBuffer};
use crate::theme;
use crate::ui::UiScene;
use crate::ui::scene::{SceneGraph, SceneNode, SceneRect, TextNode};

pub const DEFAULT_FONT_SIZE_LOGICAL_PX: f32 = 18.0;
pub const DEFAULT_LINE_HEIGHT_LOGICAL_PX: f32 = 24.0;
pub const DEFAULT_TEXT_TOP_INSET_LOGICAL_PX: f32 = 2.0;

const PRIMARY_FONT_FAMILY: &str = "0xProto Nerd Font Mono";
const PRIMARY_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Regular.ttf"
));
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Bold.ttf"
));
const PRIMARY_ITALIC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Italic.ttf"
));
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Regular.ttf"
));
const FALLBACK_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Bold.ttf"
));
const CELL_METRIC_SAMPLE_GLYPHS: &[char] = &[
    'm', 'M', 'W', '@', 'O', '#', '?', '*', '△', '⨁', '◊', '·', 'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η',
    'θ', 'λ', 'μ', 'ξ', 'π', 'σ', 'φ', 'ω', 'Δ', 'Σ', 'Ω',
];
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
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeCellMetrics {
    pub cell_width_px: usize,
    pub cell_height_px: usize,
}

#[derive(Clone, Copy, Debug)]
struct TextRasterMetrics {
    font_size_px: f32,
    line_height_px: f32,
    left_inset_px: f32,
    top_inset_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalTextMetrics {
    pub line_height_px: f32,
    pub left_inset_px: f32,
    pub top_inset_px: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextBufferKey {
    text: std::sync::Arc<str>,
    bold: bool,
}

struct CachedTextCell {
    buffer: GlyphBuffer,
}

#[derive(Clone, Debug)]
struct TextCellPlacement {
    key: TextBufferKey,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: crate::buffer::GameColor,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PlayfieldSnapshot {
    width: usize,
    height: usize,
    row_fingerprints: Vec<u64>,
    cursor: Option<(usize, usize)>,
}

impl PlayfieldSnapshot {
    fn capture_from_buffer(&mut self, buffer: &PlayfieldBuffer) {
        self.width = buffer.width();
        self.height = buffer.height();
        self.cursor = buffer
            .cursor()
            .map(|(col, row)| (usize::from(col), usize::from(row)));
        self.row_fingerprints.clear();
        self.row_fingerprints.reserve(self.height);
        for row_idx in 0..self.height {
            self.row_fingerprints
                .push(fingerprint_row(buffer.row(row_idx)));
        }
    }
}

static DEFAULT_CELL_METRICS: OnceLock<NativeCellMetrics> = OnceLock::new();

pub struct CellGridWindowRenderer {
    device: Device,
    queue: Queue,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfiguration,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: HashMap<TextBufferKey, CachedTextCell>,
    background_pipeline: RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    background_pixels: Vec<u8>,
    previous_playfield: PlayfieldSnapshot,
    has_previous_playfield: bool,
    prepared_text_hash: u64,
    has_prepared_text: bool,
    previous_scene_hash: Option<u64>,
    cell_metrics: NativeCellMetrics,
    text_metrics: TextRasterMetrics,
    scale_factor: f64,
    window: Arc<winit::window::Window>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RendererFrameStats {
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) grid_cols: usize,
    pub(crate) grid_rows: usize,
    pub(crate) text_area_count: usize,
    pub(crate) unique_text_buffer_count: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceAcquireVariant {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

const fn should_reconfigure_after_present(variant: SurfaceAcquireVariant) -> bool {
    matches!(variant, SurfaceAcquireVariant::Suboptimal)
}

struct BackgroundTexture {
    texture: Texture,
    bind_group: BindGroup,
}

#[derive(Clone, Copy, Debug)]
struct SceneRenderLayout {
    logical_width: f32,
    logical_height: f32,
    scale_factor: f64,
    x_offset_px: f32,
    y_offset_px: f32,
}

impl SceneRenderLayout {
    fn new(
        logical_size: crate::ui::scene::SceneSize,
        frame_width: u32,
        frame_height: u32,
        scale_factor: f64,
    ) -> Self {
        let physical_width = logical_size.width * scale_factor as f32;
        let physical_height = logical_size.height * scale_factor as f32;
        Self {
            logical_width: logical_size.width,
            logical_height: logical_size.height,
            scale_factor,
            x_offset_px: ((frame_width as f32 - physical_width).max(0.0) / 2.0).floor(),
            y_offset_px: ((frame_height as f32 - physical_height).max(0.0) / 2.0).floor(),
        }
    }

    fn logical_x_to_physical(self, x: f32) -> f32 {
        self.x_offset_px + x * self.scale_factor as f32
    }

    fn logical_y_to_physical(self, y: f32) -> f32 {
        self.y_offset_px + y * self.scale_factor as f32
    }

    fn physical_point(self, point: crate::ui::scene::ScenePoint) -> (usize, usize) {
        (
            self.logical_x_to_physical(point.x).floor().max(0.0) as usize,
            self.logical_y_to_physical(point.y).floor().max(0.0) as usize,
        )
    }

    fn physical_rect(self, rect: SceneRect) -> PhysicalRect {
        PhysicalRect {
            x: self.logical_x_to_physical(rect.x).floor().max(0.0) as usize,
            y: self.logical_y_to_physical(rect.y).floor().max(0.0) as usize,
            width: (rect.width * self.scale_factor as f32).ceil().max(0.0) as usize,
            height: (rect.height * self.scale_factor as f32).ceil().max(0.0) as usize,
        }
    }

    fn text_bounds(self, rect: SceneRect) -> TextBounds {
        let physical = self.physical_rect(rect);
        TextBounds {
            left: physical.x as i32,
            top: physical.y as i32,
            right: physical.x.saturating_add(physical.width) as i32,
            bottom: physical.y.saturating_add(physical.height) as i32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PhysicalRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl CellGridWindowRenderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        event_loop: &ActiveEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let scale_factor = window.scale_factor();
        let cell_metrics = cell_metrics_for_scale(scale_factor);
        let text_metrics = text_raster_metrics_for_scale(scale_factor);
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
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let font_system = build_font_system();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_config.format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let (background_pipeline, background_bind_group_layout) =
            create_background_pipeline(&device, surface_config.format);
        let background_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("nc-dash-background-sampler"),
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
            device,
            queue,
            surface,
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
            previous_playfield: PlayfieldSnapshot::default(),
            has_previous_playfield: false,
            prepared_text_hash: 0,
            has_prepared_text: false,
            previous_scene_hash: None,
            cell_metrics,
            text_metrics,
            scale_factor,
            window,
        })
    }

    pub fn render(
        &mut self,
        scene: &UiScene,
        window_pixel_width: u32,
        window_pixel_height: u32,
        diagnostic_mode: bool,
    ) -> Result<RendererFrameStats, Box<dyn std::error::Error>> {
        self.sync_scale_metrics();
        if window_pixel_width == 0 || window_pixel_height == 0 {
            let (grid_cols, grid_rows) = match scene {
                UiScene::Playfield(playfield) => (playfield.width(), playfield.height()),
                UiScene::Graph(graph) => self.scene_grid_dimensions(graph),
            };
            return Ok(RendererFrameStats {
                window_width: window_pixel_width,
                window_height: window_pixel_height,
                grid_cols,
                grid_rows,
                text_area_count: 0,
                unique_text_buffer_count: self.text_buffers.len(),
            });
        }
        self.ensure_surface_size(window_pixel_width, window_pixel_height, diagnostic_mode);
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let text_placements = match scene {
            UiScene::Playfield(playfield) => self.draw_playfield_background(playfield)?,
            UiScene::Graph(graph) => {
                let scene_hash = hash_scene_graph(graph);
                if self.previous_scene_hash == Some(scene_hash) {
                    // Scene unchanged: skip CPU repaint + write_texture.
                    // Collect text placements from existing buffers for the prepare-skip hash check.
                    self.collect_existing_scene_text_placements(graph)
                } else {
                    let placements = self.draw_scene_background(graph)?;
                    self.previous_scene_hash = Some(scene_hash);
                    placements
                }
            }
        };
        let (grid_cols, grid_rows) = match scene {
            UiScene::Playfield(playfield) => (playfield.width(), playfield.height()),
            UiScene::Graph(graph) => self.scene_grid_dimensions(graph),
        };
        let frame_stats = RendererFrameStats {
            window_width: self.surface_config.width,
            window_height: self.surface_config.height,
            grid_cols,
            grid_rows,
            text_area_count: text_placements.len(),
            unique_text_buffer_count: self.text_buffers.len(),
        };
        let placement_hash = hash_text_placements(&text_placements);
        if diagnostic_mode {
            info!(
                target: "nc_dash::native_grid",
                window_width = frame_stats.window_width,
                window_height = frame_stats.window_height,
                grid_cols = frame_stats.grid_cols,
                grid_rows = frame_stats.grid_rows,
                text_areas = frame_stats.text_area_count,
                unique_text_buffers = frame_stats.unique_text_buffer_count,
                "renderer frame begin"
            );
        }
        let text_areas = text_placements
            .iter()
            .map(|placement| TextArea {
                buffer: self
                    .text_buffers
                    .get(&placement.key)
                    .map(|cached| &cached.buffer)
                    .expect("text buffer should exist before prepare"),
                left: placement.left,
                top: placement.top,
                scale: 1.0,
                bounds: placement.bounds,
                default_color: glyphon_color(placement.color),
                custom_glyphs: &[],
            })
            .collect::<Vec<_>>();
        let needs_prepare = !self.has_prepared_text || self.prepared_text_hash != placement_hash;
        if needs_prepare {
            if diagnostic_mode {
                info!(
                    target: "nc_dash::native_grid",
                    text_areas = frame_stats.text_area_count,
                    "renderer prepare begin"
                );
            }
            self.text_renderer.prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )?;
            self.prepared_text_hash = placement_hash;
            self.has_prepared_text = true;
            if diagnostic_mode {
                info!(target: "nc_dash::native_grid", "renderer prepare end");
            }
        } else if diagnostic_mode {
            info!(
                target: "nc_dash::native_grid",
                text_areas = frame_stats.text_area_count,
                "renderer prepare skipped"
            );
        }
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer acquire begin");
        }

        let (frame, acquire_variant) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, SurfaceAcquireVariant::Success),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "suboptimal",
                        width = self.surface_config.width,
                        height = self.surface_config.height,
                        "renderer surface reconfigure deferred"
                    );
                }
                // `Suboptimal(frame)` still carries a live `SurfaceTexture`, so
                // reconfigure must wait until after `present()` consumes and drops it.
                (frame, SurfaceAcquireVariant::Suboptimal)
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "timeout_or_occluded",
                        "renderer acquire retry"
                    );
                }
                self.window.request_redraw();
                return Ok(frame_stats);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "timeout_or_occluded",
                        "renderer acquire retry"
                    );
                }
                self.window.request_redraw();
                return Ok(frame_stats);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "outdated_or_lost",
                        width = self.surface_config.width,
                        height = self.surface_config.height,
                        "renderer surface reconfigure"
                    );
                }
                // Reconfigure rather than recreate. Recreating via
                // `self.surface = instance.create_surface(...)` evaluates the new
                // surface while the old one is still alive, which triggers the wgpu
                // validation error "SurfaceOutput must be dropped before a new
                // Surface is made" on Wayland/mesa-vk when the Lost event arrives
                // with an acquired surface texture still internally tracked.
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(frame_stats);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".into());
            }
        };
        let reconfigure_after_present = should_reconfigure_after_present(acquire_variant);
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer acquire end");
        }

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-dash-native-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(theme_wgpu_background()),
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
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer submit begin");
        }
        self.queue.submit(Some(encoder.finish()));
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer submit end");
            info!(target: "nc_dash::native_grid", "renderer present begin");
        }
        frame.present();
        if reconfigure_after_present {
            if diagnostic_mode {
                info!(
                    target: "nc_dash::native_grid",
                    surface_state = "suboptimal",
                    width = self.surface_config.width,
                    height = self.surface_config.height,
                    "renderer surface reconfigure after present"
                );
            }
            self.surface.configure(&self.device, &self.surface_config);
            self.window.request_redraw();
        }
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer present end");
        }
        self.atlas.trim();
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer frame end");
        }
        Ok(frame_stats)
    }

    fn ensure_surface_size(&mut self, width: u32, height: u32, diagnostic_mode: bool) {
        if self.surface_config.width == width && self.surface_config.height == height {
            return;
        }
        if diagnostic_mode {
            info!(
                target: "nc_dash::native_grid",
                old_width = self.surface_config.width,
                old_height = self.surface_config.height,
                new_width = width.max(1),
                new_height = height.max(1),
                "renderer surface resize"
            );
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
        self.has_prepared_text = false;
        self.previous_scene_hash = None;
    }

    fn draw_playfield_background(
        &mut self,
        playfield: &PlayfieldBuffer,
    ) -> Result<Vec<TextCellPlacement>, Box<dyn std::error::Error>> {
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let body_bg = primitives::color_to_rgba(theme::body_style().bg);
        let grid_pixel_width = playfield.width() * self.cell_metrics.cell_width_px;
        let grid_pixel_height = playfield.height() * self.cell_metrics.cell_height_px;
        let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
        let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;
        let cursor = playfield
            .cursor()
            .map(|(col, row)| (usize::from(col), usize::from(row)));
        let mut current_snapshot = PlayfieldSnapshot::default();
        current_snapshot.capture_from_buffer(playfield);
        let full_repaint = !self.has_previous_playfield
            || self.previous_playfield.width != current_snapshot.width
            || self.previous_playfield.height != current_snapshot.height
            || self.background_pixels.len() != frame_width * frame_height * 4;
        if full_repaint {
            self.background_pixels.fill(0);
            for pixel in self.background_pixels.chunks_exact_mut(4) {
                pixel.copy_from_slice(&body_bg);
            }
        }
        let mut placements = Vec::new();
        let mut dirty_row_ranges = Vec::new();
        let mut current_dirty_range: Option<(usize, usize)> = None;

        for row_idx in 0..playfield.height() {
            let row_changed = full_repaint
                || row_needs_repaint(&self.previous_playfield, &current_snapshot, row_idx, cursor);
            if row_changed {
                repaint_playfield_row(
                    &mut self.background_pixels,
                    frame_width,
                    x_offset,
                    y_offset,
                    row_idx,
                    playfield,
                    cursor,
                    self.cell_metrics,
                    body_bg,
                );
                match current_dirty_range.as_mut() {
                    Some((_, end)) if *end == row_idx => *end += 1,
                    Some(_) => {
                        dirty_row_ranges
                            .push(current_dirty_range.take().expect("dirty range exists"));
                        current_dirty_range = Some((row_idx, row_idx + 1));
                    }
                    None => current_dirty_range = Some((row_idx, row_idx + 1)),
                }
            }

            let mut run_start = None;
            let mut run_style: Option<CellStyle> = None;
            let mut run_text = String::new();

            for col_idx in 0..playfield.width() {
                let source = playfield.row(row_idx)[col_idx];
                let style = if cursor == Some((col_idx, row_idx)) {
                    CellStyle::new(source.style.bg, source.style.fg, source.style.bold)
                } else {
                    source.style
                };
                let can_join_run = source.ch != ' '
                    && !primitives::should_draw_as_primitive(source.ch)
                    && run_style == Some(style);
                if can_join_run {
                    run_text.push(source.ch);
                    continue;
                }
                self.flush_playfield_text_run(
                    &mut placements,
                    &mut run_start,
                    &mut run_style,
                    &mut run_text,
                    x_offset,
                    y_offset,
                    row_idx,
                    col_idx,
                );
                if source.ch != ' ' && !primitives::should_draw_as_primitive(source.ch) {
                    run_start = Some(col_idx);
                    run_style = Some(style);
                    run_text.push(source.ch);
                }
            }
            self.flush_playfield_text_run(
                &mut placements,
                &mut run_start,
                &mut run_style,
                &mut run_text,
                x_offset,
                y_offset,
                row_idx,
                playfield.width(),
            );
        }
        if let Some(range) = current_dirty_range {
            dirty_row_ranges.push(range);
        }

        self.previous_playfield = current_snapshot;
        self.has_previous_playfield = true;

        if full_repaint {
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
        } else {
            for (start_row, end_row) in dirty_row_ranges {
                let band_top = y_offset + start_row * self.cell_metrics.cell_height_px;
                let band_height = (end_row - start_row) * self.cell_metrics.cell_height_px;
                let byte_offset = band_top * frame_width * 4;
                let byte_len = band_height * frame_width * 4;
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.background_texture.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: band_top as u32,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &self.background_pixels[byte_offset..byte_offset + byte_len],
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(self.surface_config.width * 4),
                        rows_per_image: Some(band_height as u32),
                    },
                    wgpu::Extent3d {
                        width: self.surface_config.width,
                        height: band_height as u32,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        Ok(placements)
    }

    fn draw_scene_background(
        &mut self,
        scene: &SceneGraph,
    ) -> Result<Vec<TextCellPlacement>, Box<dyn std::error::Error>> {
        let frame_width = self.surface_config.width as usize;
        let body_bg = primitives::color_to_rgba(theme::body_style().bg);
        self.background_pixels.fill(0);
        for pixel in self.background_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&body_bg);
        }

        let layout = SceneRenderLayout::new(
            scene.logical_size(),
            self.surface_config.width,
            self.surface_config.height,
            self.window.scale_factor(),
        );
        let mut placements = Vec::new();

        for node in scene.nodes() {
            match node {
                SceneNode::Quad(node) => {
                    let rect = layout.physical_rect(node.rect);
                    primitives::fill_rect_rgba(
                        &mut self.background_pixels,
                        frame_width,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        primitives::color_to_rgba(node.color),
                    );
                }
                SceneNode::Caret(node) => {
                    let rect = layout.physical_rect(node.rect);
                    primitives::fill_rect_rgba(
                        &mut self.background_pixels,
                        frame_width,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        primitives::color_to_rgba(node.color),
                    );
                }
                SceneNode::Line(node) => {
                    let thickness = (node.thickness * layout.scale_factor as f32)
                        .ceil()
                        .max(1.0);
                    let start = layout.physical_point(node.start);
                    let end = layout.physical_point(node.end);
                    draw_scene_line(
                        &mut self.background_pixels,
                        frame_width,
                        start,
                        end,
                        thickness as usize,
                        primitives::color_to_rgba(node.color),
                    );
                }
                SceneNode::Text(node) => self.push_scene_text_node(&layout, node, &mut placements),
            }
        }

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

        Ok(placements)
    }

    fn collect_existing_scene_text_placements(
        &mut self,
        scene: &SceneGraph,
    ) -> Vec<TextCellPlacement> {
        let layout = SceneRenderLayout::new(
            scene.logical_size(),
            self.surface_config.width,
            self.surface_config.height,
            self.window.scale_factor(),
        );
        let mut placements = Vec::new();
        for node in scene.nodes() {
            if let SceneNode::Text(node) = node {
                self.push_scene_text_node(&layout, node, &mut placements);
            }
        }
        placements
    }

    fn ensure_text_buffer(&mut self, key: TextBufferKey) {
        if self.text_buffers.contains_key(&key) {
            return;
        }
        let mut buffer = GlyphBuffer::new(
            &mut self.font_system,
            Metrics::new(
                self.text_metrics.font_size_px,
                self.text_metrics.line_height_px,
            ),
        );
        buffer.set_size(
            &mut self.font_system,
            None,
            Some(self.cell_metrics.cell_height_px as f32),
        );
        let attrs = Attrs::new().family(Family::Monospace).weight(if key.bold {
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
        self.text_buffers.insert(key, CachedTextCell { buffer });
    }

    fn sync_scale_metrics(&mut self) {
        let scale_factor = self.window.scale_factor();
        if (self.scale_factor - scale_factor).abs() < f64::EPSILON {
            return;
        }
        self.scale_factor = scale_factor;
        self.cell_metrics = cell_metrics_for_scale(scale_factor);
        self.text_metrics = text_raster_metrics_for_scale(scale_factor);
        self.text_buffers.clear();
        self.has_prepared_text = false;
        self.previous_scene_hash = None;
    }

    fn push_scene_text_node(
        &mut self,
        layout: &SceneRenderLayout,
        node: &TextNode,
        placements: &mut Vec<TextCellPlacement>,
    ) {
        if node.text.is_empty() {
            return;
        }
        let key = TextBufferKey {
            text: Arc::<str>::from(node.text.as_str()),
            bold: node.style.bold,
        };
        self.ensure_text_buffer(key.clone());
        let bounds = node
            .clip
            .map(|clip| layout.text_bounds(clip))
            .unwrap_or_else(|| {
                layout.text_bounds(SceneRect::new(
                    0.0,
                    0.0,
                    layout.logical_width,
                    layout.logical_height,
                ))
            });
        placements.push(TextCellPlacement {
            key,
            left: layout.logical_x_to_physical(node.origin.x) + self.text_metrics.left_inset_px,
            top: layout.logical_y_to_physical(node.origin.y) + self.text_metrics.top_inset_px,
            bounds,
            color: node.style.fg,
        });
    }

    fn scene_grid_dimensions(&self, scene: &SceneGraph) -> (usize, usize) {
        let logical_metrics = logical_cell_metrics();
        (
            (scene.logical_size().width / logical_metrics.cell_width_px as f32).round() as usize,
            (scene.logical_size().height / logical_metrics.cell_height_px as f32).round() as usize,
        )
    }

    fn flush_playfield_text_run(
        &mut self,
        placements: &mut Vec<TextCellPlacement>,
        run_start: &mut Option<usize>,
        run_style: &mut Option<CellStyle>,
        run_text: &mut String,
        x_offset: usize,
        y_offset: usize,
        row_idx: usize,
        end_col: usize,
    ) {
        let Some(start_col) = run_start.take() else {
            return;
        };
        let style = run_style.take().expect("run style exists when run starts");
        if run_text.is_empty() {
            return;
        }
        let key = TextBufferKey {
            text: Arc::<str>::from(run_text.as_str()),
            bold: style.bold,
        };
        self.ensure_text_buffer(key.clone());
        let left = x_offset as f32
            + start_col as f32 * self.cell_metrics.cell_width_px as f32
            + self.text_metrics.left_inset_px;
        let top = y_offset as f32
            + row_idx as f32 * self.cell_metrics.cell_height_px as f32
            + self.text_metrics.top_inset_px;
        placements.push(TextCellPlacement {
            key,
            left,
            top,
            bounds: TextBounds {
                left: (x_offset + start_col * self.cell_metrics.cell_width_px) as i32,
                top: (y_offset + row_idx * self.cell_metrics.cell_height_px) as i32,
                right: (x_offset + end_col * self.cell_metrics.cell_width_px) as i32,
                bottom: (y_offset + (row_idx + 1) * self.cell_metrics.cell_height_px) as i32,
            },
            color: style.fg,
        });
        run_text.clear();
    }
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
            label: Some("nc-dash-background-texture"),
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
            label: Some("nc-dash-background-bind-group"),
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

fn repaint_playfield_row(
    pixels: &mut [u8],
    frame_width: usize,
    x_offset: usize,
    y_offset: usize,
    row_idx: usize,
    playfield: &PlayfieldBuffer,
    cursor: Option<(usize, usize)>,
    cell_metrics: NativeCellMetrics,
    body_bg: [u8; 4],
) {
    let row_y = y_offset + row_idx * cell_metrics.cell_height_px;
    primitives::fill_rect_rgba(
        pixels,
        frame_width,
        0,
        row_y,
        frame_width,
        cell_metrics.cell_height_px,
        body_bg,
    );
    for col_idx in 0..playfield.width() {
        let source = playfield.row(row_idx)[col_idx];
        let style = if cursor == Some((col_idx, row_idx)) {
            CellStyle::new(source.style.bg, source.style.fg, source.style.bold)
        } else {
            source.style
        };
        let cell_x = x_offset + col_idx * cell_metrics.cell_width_px;
        primitives::fill_rect_rgba(
            pixels,
            frame_width,
            cell_x,
            row_y,
            cell_metrics.cell_width_px,
            cell_metrics.cell_height_px,
            primitives::color_to_rgba(style.bg),
        );
        if source.ch != ' ' && primitives::should_draw_as_primitive(source.ch) {
            primitives::draw_cell_primitive(
                pixels,
                frame_width,
                cell_x,
                row_y,
                cell_metrics.cell_width_px,
                cell_metrics.cell_height_px,
                source.ch,
                primitives::color_to_rgba(style.fg),
            );
        }
    }
}

fn row_needs_repaint(
    previous: &PlayfieldSnapshot,
    current: &PlayfieldSnapshot,
    row_idx: usize,
    current_cursor: Option<(usize, usize)>,
) -> bool {
    if previous.row_fingerprints.get(row_idx) != current.row_fingerprints.get(row_idx) {
        return true;
    }
    let previous_cursor_row = previous.cursor.filter(|(_, row)| *row == row_idx);
    let current_cursor_row = current_cursor.filter(|(_, row)| *row == row_idx);
    previous_cursor_row != current_cursor_row
}

fn fingerprint_row(row: &[crate::buffer::Cell]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for cell in row {
        hash = mix_u32(hash, cell.ch as u32);
        hash = mix_u32(hash, color_code(cell.style.fg));
        hash = mix_u32(hash, color_code(cell.style.bg));
        hash = mix_u32(hash, u32::from(cell.style.bold));
    }
    hash
}

fn mix_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn color_code(color: crate::buffer::GameColor) -> u32 {
    match color {
        crate::buffer::GameColor::Black => 0,
        crate::buffer::GameColor::Red => 1,
        crate::buffer::GameColor::Green => 2,
        crate::buffer::GameColor::Yellow => 3,
        crate::buffer::GameColor::Blue => 4,
        crate::buffer::GameColor::Magenta => 5,
        crate::buffer::GameColor::Cyan => 6,
        crate::buffer::GameColor::White => 7,
        crate::buffer::GameColor::BrightBlack => 8,
        crate::buffer::GameColor::BrightRed => 9,
        crate::buffer::GameColor::BrightGreen => 10,
        crate::buffer::GameColor::BrightYellow => 11,
        crate::buffer::GameColor::BrightBlue => 12,
        crate::buffer::GameColor::BrightMagenta => 13,
        crate::buffer::GameColor::BrightCyan => 14,
        crate::buffer::GameColor::BrightWhite => 15,
        crate::buffer::GameColor::Indexed(idx) => 0x0100_0000 | u32::from(idx),
        crate::buffer::GameColor::Rgb(r, g, b) => {
            0x0200_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
        }
    }
}

fn hash_text_placements(placements: &[TextCellPlacement]) -> u64 {
    let mut hasher = DefaultHasher::new();
    placements.len().hash(&mut hasher);
    for placement in placements {
        placement.key.hash(&mut hasher);
        placement.left.to_bits().hash(&mut hasher);
        placement.top.to_bits().hash(&mut hasher);
        placement.bounds.left.hash(&mut hasher);
        placement.bounds.top.hash(&mut hasher);
        placement.bounds.right.hash(&mut hasher);
        placement.bounds.bottom.hash(&mut hasher);
        color_code(placement.color).hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_scene_graph(graph: &SceneGraph) -> u64 {
    let mut hasher = DefaultHasher::new();
    let size = graph.logical_size();
    size.width.to_bits().hash(&mut hasher);
    size.height.to_bits().hash(&mut hasher);
    graph.nodes().len().hash(&mut hasher);
    for node in graph.nodes() {
        match node {
            SceneNode::Quad(n) => {
                0u8.hash(&mut hasher);
                n.rect.x.to_bits().hash(&mut hasher);
                n.rect.y.to_bits().hash(&mut hasher);
                n.rect.width.to_bits().hash(&mut hasher);
                n.rect.height.to_bits().hash(&mut hasher);
                color_code(n.color).hash(&mut hasher);
            }
            SceneNode::Line(n) => {
                1u8.hash(&mut hasher);
                n.start.x.to_bits().hash(&mut hasher);
                n.start.y.to_bits().hash(&mut hasher);
                n.end.x.to_bits().hash(&mut hasher);
                n.end.y.to_bits().hash(&mut hasher);
                n.thickness.to_bits().hash(&mut hasher);
                color_code(n.color).hash(&mut hasher);
            }
            SceneNode::Caret(n) => {
                2u8.hash(&mut hasher);
                n.rect.x.to_bits().hash(&mut hasher);
                n.rect.y.to_bits().hash(&mut hasher);
                n.rect.width.to_bits().hash(&mut hasher);
                n.rect.height.to_bits().hash(&mut hasher);
                color_code(n.color).hash(&mut hasher);
            }
            SceneNode::Text(n) => {
                3u8.hash(&mut hasher);
                n.text.hash(&mut hasher);
                n.origin.x.to_bits().hash(&mut hasher);
                n.origin.y.to_bits().hash(&mut hasher);
                color_code(n.style.fg).hash(&mut hasher);
                color_code(n.style.bg).hash(&mut hasher);
                n.style.bold.hash(&mut hasher);
                n.clip.is_some().hash(&mut hasher);
                if let Some(clip) = n.clip {
                    clip.x.to_bits().hash(&mut hasher);
                    clip.y.to_bits().hash(&mut hasher);
                    clip.width.to_bits().hash(&mut hasher);
                    clip.height.to_bits().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

fn draw_scene_line(
    frame: &mut [u8],
    stride_px: usize,
    start: (usize, usize),
    end: (usize, usize),
    thickness: usize,
    color: [u8; 4],
) {
    let thickness = thickness.max(1);
    if start.0 == end.0 {
        let x = start.0.saturating_sub(thickness / 2);
        let top = start.1.min(end.1);
        let height = start
            .1
            .max(end.1)
            .saturating_sub(top)
            .saturating_add(thickness);
        primitives::fill_rect_rgba(frame, stride_px, x, top, thickness, height, color);
    } else {
        let y = start.1.saturating_sub(thickness / 2);
        let left = start.0.min(end.0);
        let width = start
            .0
            .max(end.0)
            .saturating_sub(left)
            .saturating_add(thickness);
        primitives::fill_rect_rgba(frame, stride_px, left, y, width, thickness, color);
    }
}

pub fn logical_cell_metrics() -> NativeCellMetrics {
    *DEFAULT_CELL_METRICS.get_or_init(|| cell_metrics_for_scale(1.0))
}

pub fn logical_text_metrics() -> LogicalTextMetrics {
    let metrics = text_raster_metrics_for_scale(1.0);
    LogicalTextMetrics {
        line_height_px: metrics.line_height_px,
        left_inset_px: metrics.left_inset_px,
        top_inset_px: metrics.top_inset_px,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn default_cell_metrics() -> NativeCellMetrics {
    logical_cell_metrics()
}

pub fn logical_window_size_for_grid(cols: usize, rows: usize) -> winit::dpi::LogicalSize<f64> {
    let metrics = logical_cell_metrics();
    winit::dpi::LogicalSize::new(
        (cols * metrics.cell_width_px) as f64,
        (rows * metrics.cell_height_px) as f64,
    )
}

pub fn terminal_grid_for_pixels(
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
) -> (u16, u16) {
    let metrics = cell_metrics_for_scale(scale_factor);
    let cols = (pixel_width.max(1) as usize / metrics.cell_width_px).max(1);
    let rows = (pixel_height.max(1) as usize / metrics.cell_height_px).max(1);
    (
        cols.min(u16::MAX as usize) as u16,
        rows.min(u16::MAX as usize) as u16,
    )
}

pub fn cell_position_at_pixel(
    grid_cols: usize,
    grid_rows: usize,
    window_pixel_width: u32,
    window_pixel_height: u32,
    scale_factor: f64,
    position: winit::dpi::PhysicalPosition<f64>,
) -> Option<(u16, u16)> {
    if !position.x.is_finite() || !position.y.is_finite() || position.x < 0.0 || position.y < 0.0 {
        return None;
    }

    let metrics = cell_metrics_for_scale(scale_factor);
    let x = position.x.floor() as usize;
    let y = position.y.floor() as usize;
    let grid_pixel_width = grid_cols.checked_mul(metrics.cell_width_px)?;
    let grid_pixel_height = grid_rows.checked_mul(metrics.cell_height_px)?;
    let x_offset = (window_pixel_width as usize).saturating_sub(grid_pixel_width) / 2;
    let y_offset = (window_pixel_height as usize).saturating_sub(grid_pixel_height) / 2;

    if x < x_offset || y < y_offset {
        return None;
    }
    let local_x = x - x_offset;
    let local_y = y - y_offset;
    if local_x >= grid_pixel_width || local_y >= grid_pixel_height {
        return None;
    }

    let col = local_x / metrics.cell_width_px;
    let row = local_y / metrics.cell_height_px;
    Some((
        col.min(u16::MAX as usize) as u16,
        row.min(u16::MAX as usize) as u16,
    ))
}

fn build_font_system() -> FontSystem {
    let fonts = [
        PRIMARY_REGULAR_FONT,
        PRIMARY_BOLD_FONT,
        PRIMARY_ITALIC_FONT,
        FALLBACK_REGULAR_FONT,
        FALLBACK_BOLD_FONT,
    ]
    .into_iter()
    .map(|font| fontdb::Source::Binary(Arc::new(Vec::from(font))));
    let mut system = FontSystem::new_with_fonts(fonts);
    system.db_mut().set_monospace_family(PRIMARY_FONT_FAMILY);
    system
}

fn create_background_pipeline(
    device: &Device,
    surface_format: TextureFormat,
) -> (RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("nc-dash-background-shader"),
        source: ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_SHADER)),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-dash-background-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("nc-dash-background-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("nc-dash-background-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    (pipeline, bind_group_layout)
}

fn measured_buffer_width(buffer: &GlyphBuffer) -> f32 {
    buffer
        .layout_runs()
        .fold(0.0f32, |width, run| width.max(run.line_w))
}

fn cell_metrics_for_scale(scale_factor: f64) -> NativeCellMetrics {
    let mut font_system = build_font_system();
    let text_metrics = text_raster_metrics_for_scale(scale_factor);
    let horizontal_padding = (2.0 * scale_factor as f32).ceil().max(1.0) as usize;
    NativeCellMetrics {
        cell_width_px: CELL_METRIC_SAMPLE_GLYPHS
            .iter()
            .flat_map(|ch| [false, true].into_iter().map(move |bold| (*ch, bold)))
            .map(|(ch, bold)| measure_sample_glyph_width(&mut font_system, ch, bold, text_metrics))
            .max()
            .unwrap_or(1)
            .saturating_add(horizontal_padding)
            .max(1),
        cell_height_px: text_metrics.line_height_px.ceil().max(1.0) as usize,
    }
}

fn text_raster_metrics_for_scale(scale_factor: f64) -> TextRasterMetrics {
    let scale = scale_factor.max(1.0) as f32;
    let font_size_px = DEFAULT_FONT_SIZE_LOGICAL_PX * scale;
    let line_height_px = DEFAULT_LINE_HEIGHT_LOGICAL_PX * scale;
    let left_inset_px = scale.ceil().max(1.0);
    let top_inset_px = DEFAULT_TEXT_TOP_INSET_LOGICAL_PX * scale;
    TextRasterMetrics {
        font_size_px,
        line_height_px,
        left_inset_px,
        top_inset_px,
    }
}

fn measure_sample_glyph_width(
    font_system: &mut FontSystem,
    ch: char,
    bold: bool,
    text_metrics: TextRasterMetrics,
) -> usize {
    let mut buffer = GlyphBuffer::new(
        font_system,
        Metrics::new(text_metrics.font_size_px, text_metrics.line_height_px),
    );
    buffer.set_size(font_system, None, Some(text_metrics.line_height_px));
    let attrs = Attrs::new().family(Family::Monospace).weight(if bold {
        Weight::BOLD
    } else {
        Weight::NORMAL
    });
    let text = ch.to_string();
    buffer.set_text(font_system, &text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    measured_buffer_width(&buffer).ceil().max(1.0) as usize
}

fn glyphon_color(color: crate::buffer::GameColor) -> Color {
    let [r, g, b, _] = primitives::color_to_rgba(color);
    Color::rgb(r, g, b)
}

fn theme_wgpu_background() -> wgpu::Color {
    let [r, g, b, _] = primitives::color_to_rgba(theme::body_style().bg);
    wgpu::Color {
        r: f64::from(r) / 255.0,
        g: f64::from(g) / 255.0,
        b: f64::from(b) / 255.0,
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CELL_METRIC_SAMPLE_GLYPHS, SurfaceAcquireVariant, cell_position_at_pixel,
        default_cell_metrics, measure_sample_glyph_width, should_reconfigure_after_present,
        terminal_grid_for_pixels, text_raster_metrics_for_scale,
    };

    #[test]
    fn terminal_grid_uses_measured_font_dimensions() {
        let metrics = default_cell_metrics();
        assert_eq!(
            terminal_grid_for_pixels(
                (metrics.cell_width_px * 10) as u32,
                (metrics.cell_height_px * 3) as u32,
                1.0
            ),
            (10, 3)
        );
    }

    #[test]
    fn pixel_position_maps_back_to_grid_cell() {
        let metrics = default_cell_metrics();
        assert_eq!(
            cell_position_at_pixel(
                10,
                4,
                (metrics.cell_width_px * 10) as u32,
                (metrics.cell_height_px * 4) as u32,
                1.0,
                winit::dpi::PhysicalPosition::new(
                    (metrics.cell_width_px * 2 + metrics.cell_width_px / 2) as f64,
                    (metrics.cell_height_px + metrics.cell_height_px / 2) as f64
                )
            ),
            Some((2, 1))
        );
    }

    #[test]
    fn position_in_centering_gutter_maps_to_none() {
        let metrics = default_cell_metrics();
        let window_width = (metrics.cell_width_px * 10 + 7) as u32;
        let window_height = (metrics.cell_height_px * 4 + 9) as u32;

        assert_eq!(
            cell_position_at_pixel(
                10,
                4,
                window_width,
                window_height,
                1.0,
                winit::dpi::PhysicalPosition::new(2.0, 3.0)
            ),
            None
        );
    }

    #[test]
    fn default_metrics_cover_sample_glyph_catalog() {
        let metrics = default_cell_metrics();
        let mut font_system = super::build_font_system();
        let text_metrics = text_raster_metrics_for_scale(1.0);
        for ch in CELL_METRIC_SAMPLE_GLYPHS {
            for bold in [false, true] {
                let width = measure_sample_glyph_width(&mut font_system, *ch, bold, text_metrics);
                assert!(
                    width < metrics.cell_width_px,
                    "sample glyph {ch} bold={bold} width {width} exceeded cell width {}",
                    metrics.cell_width_px
                );
            }
        }
        assert_eq!(
            metrics.cell_height_px,
            super::DEFAULT_LINE_HEIGHT_LOGICAL_PX.ceil() as usize
        );
    }

    #[test]
    fn only_suboptimal_reconfigures_after_present() {
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Success
        ));
        assert!(should_reconfigure_after_present(
            SurfaceAcquireVariant::Suboptimal
        ));
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Timeout
        ));
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Occluded
        ));
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Outdated
        ));
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Lost
        ));
        assert!(!should_reconfigure_after_present(
            SurfaceAcquireVariant::Validation
        ));
    }
}
