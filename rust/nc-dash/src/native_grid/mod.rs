mod legacy;
mod primitives;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight, fontdb,
};
use rustybuzz::{Face, ttf_parser::GlyphId};
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
use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, ModifiersState, NamedKey};
use tracing::info;

use crate::buffer::{CellStyle, PlayfieldBuffer};
use crate::theme;

pub use legacy::build_native_terminal;

pub const DEFAULT_FONT_HEIGHT_PX: u32 = 20;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TextCellKey {
    ch: char,
    bold: bool,
}

#[derive(Clone, Copy, Debug)]
struct TextCellPlacement {
    key: TextCellKey,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: Color,
}

static DEFAULT_CELL_METRICS: OnceLock<NativeCellMetrics> = OnceLock::new();

pub struct CellGridWindowRenderer {
    instance: Instance,
    device: Device,
    queue: Queue,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfiguration,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: HashMap<TextCellKey, GlyphBuffer>,
    background_pipeline: RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    background_pixels: Vec<u8>,
    cell_metrics: NativeCellMetrics,
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

struct BackgroundTexture {
    texture: Texture,
    bind_group: BindGroup,
}

impl CellGridWindowRenderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        event_loop: &ActiveEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cell_metrics = default_cell_metrics();
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
            present_mode: PresentMode::Fifo,
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
            instance,
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
            cell_metrics,
            window,
        })
    }

    pub fn render(
        &mut self,
        playfield: &PlayfieldBuffer,
        window_pixel_width: u32,
        window_pixel_height: u32,
        diagnostic_mode: bool,
    ) -> Result<RendererFrameStats, Box<dyn std::error::Error>> {
        if window_pixel_width == 0 || window_pixel_height == 0 {
            return Ok(RendererFrameStats {
                window_width: window_pixel_width,
                window_height: window_pixel_height,
                grid_cols: playfield.width(),
                grid_rows: playfield.height(),
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

        let text_placements = self.draw_background(playfield)?;
        let frame_stats = RendererFrameStats {
            window_width: self.surface_config.width,
            window_height: self.surface_config.height,
            grid_cols: playfield.width(),
            grid_rows: playfield.height(),
            text_area_count: text_placements.len(),
            unique_text_buffer_count: self.text_buffers.len(),
        };
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
                    .expect("text buffer should exist before prepare"),
                left: placement.left,
                top: placement.top,
                scale: 1.0,
                bounds: placement.bounds,
                default_color: placement.color,
                custom_glyphs: &[],
            })
            .collect::<Vec<_>>();

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
        if diagnostic_mode {
            info!(target: "nc_dash::native_grid", "renderer prepare end");
            info!(target: "nc_dash::native_grid", "renderer acquire begin");
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
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
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "outdated_or_suboptimal",
                        width = self.surface_config.width,
                        height = self.surface_config.height,
                        "renderer surface reconfigure"
                    );
                }
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(frame_stats);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if diagnostic_mode {
                    info!(
                        target: "nc_dash::native_grid",
                        surface_state = "lost",
                        "renderer surface recreate"
                    );
                }
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(frame_stats);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".into());
            }
        };
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
    }

    fn draw_background(
        &mut self,
        playfield: &PlayfieldBuffer,
    ) -> Result<Vec<TextCellPlacement>, Box<dyn std::error::Error>> {
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let body_bg = primitives::color_to_rgba(theme::body_style().bg);
        self.background_pixels.fill(0);
        for pixel in self.background_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&body_bg);
        }

        let grid_pixel_width = playfield.width() * self.cell_metrics.cell_width_px;
        let grid_pixel_height = playfield.height() * self.cell_metrics.cell_height_px;
        let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
        let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;
        let cursor = playfield
            .cursor()
            .map(|(col, row)| (usize::from(col), usize::from(row)));
        let mut placements = Vec::new();

        for row_idx in 0..playfield.height() {
            for col_idx in 0..playfield.width() {
                let source = playfield.row(row_idx)[col_idx];
                let style = if cursor == Some((col_idx, row_idx)) {
                    CellStyle::new(source.style.bg, source.style.fg, source.style.bold)
                } else {
                    source.style
                };
                let cell_x = x_offset + col_idx * self.cell_metrics.cell_width_px;
                let cell_y = y_offset + row_idx * self.cell_metrics.cell_height_px;
                primitives::fill_rect_rgba(
                    &mut self.background_pixels,
                    frame_width,
                    cell_x,
                    cell_y,
                    self.cell_metrics.cell_width_px,
                    self.cell_metrics.cell_height_px,
                    primitives::color_to_rgba(style.bg),
                );
                if source.ch == ' ' {
                    continue;
                }
                if primitives::should_draw_as_primitive(source.ch) {
                    primitives::draw_cell_primitive(
                        &mut self.background_pixels,
                        frame_width,
                        cell_x,
                        cell_y,
                        self.cell_metrics.cell_width_px,
                        self.cell_metrics.cell_height_px,
                        source.ch,
                        primitives::color_to_rgba(style.fg),
                    );
                    continue;
                }

                let key = TextCellKey {
                    ch: source.ch,
                    bold: style.bold,
                };
                self.ensure_text_buffer(key);
                placements.push(TextCellPlacement {
                    key,
                    left: cell_x as f32,
                    top: cell_y as f32,
                    bounds: TextBounds {
                        left: cell_x as i32,
                        top: cell_y as i32,
                        right: (cell_x + self.cell_metrics.cell_width_px) as i32,
                        bottom: (cell_y + self.cell_metrics.cell_height_px) as i32,
                    },
                    color: glyphon_color(style.fg),
                });
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

    fn ensure_text_buffer(&mut self, key: TextCellKey) {
        if self.text_buffers.contains_key(&key) {
            return;
        }
        let mut buffer = GlyphBuffer::new(
            &mut self.font_system,
            Metrics::new(
                DEFAULT_FONT_HEIGHT_PX as f32,
                self.cell_metrics.cell_height_px as f32,
            ),
        );
        buffer.set_size(
            &mut self.font_system,
            Some(self.cell_metrics.cell_width_px as f32),
            Some(self.cell_metrics.cell_height_px as f32),
        );
        let attrs = Attrs::new().family(Family::Monospace).weight(if key.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        });
        let text = key.ch.to_string();
        buffer.set_text(
            &mut self.font_system,
            &text,
            &attrs,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        self.text_buffers.insert(key, buffer);
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

pub fn default_cell_metrics() -> NativeCellMetrics {
    *DEFAULT_CELL_METRICS.get_or_init(measure_default_cell_metrics)
}

pub fn logical_window_size_for_grid(cols: usize, rows: usize) -> winit::dpi::LogicalSize<f64> {
    let metrics = default_cell_metrics();
    winit::dpi::LogicalSize::new(
        (cols * metrics.cell_width_px) as f64,
        (rows * metrics.cell_height_px) as f64,
    )
}

pub fn terminal_grid_for_pixels(pixel_width: u32, pixel_height: u32) -> (u16, u16) {
    let metrics = default_cell_metrics();
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
    position: winit::dpi::PhysicalPosition<f64>,
) -> Option<(u16, u16)> {
    if !position.x.is_finite() || !position.y.is_finite() || position.x < 0.0 || position.y < 0.0 {
        return None;
    }

    let metrics = default_cell_metrics();
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

pub fn is_key_press(event: &WinitKeyEvent) -> bool {
    event.state == ElementState::Pressed
}

pub fn crossterm_key_event_from_winit(
    event: &WinitKeyEvent,
    modifiers: ModifiersState,
) -> Option<KeyEvent> {
    if !is_key_press(event) {
        return None;
    }
    let key_modifiers = modifiers_to_crossterm(modifiers);
    let code = match &event.logical_key {
        Key::Named(NamedKey::ArrowUp) => KeyCode::Up,
        Key::Named(NamedKey::ArrowDown) => KeyCode::Down,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::Left,
        Key::Named(NamedKey::ArrowRight) => KeyCode::Right,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Escape) => KeyCode::Esc,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Named(NamedKey::Insert) => KeyCode::Insert,
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => KeyCode::BackTab,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::F1) => KeyCode::F(1),
        Key::Named(NamedKey::F2) => KeyCode::F(2),
        Key::Named(NamedKey::F3) => KeyCode::F(3),
        Key::Named(NamedKey::F4) => KeyCode::F(4),
        Key::Named(NamedKey::F5) => KeyCode::F(5),
        Key::Named(NamedKey::F6) => KeyCode::F(6),
        Key::Named(NamedKey::F7) => KeyCode::F(7),
        Key::Named(NamedKey::F8) => KeyCode::F(8),
        Key::Named(NamedKey::F9) => KeyCode::F(9),
        Key::Named(NamedKey::F10) => KeyCode::F(10),
        Key::Named(NamedKey::F11) => KeyCode::F(11),
        Key::Named(NamedKey::F12) => KeyCode::F(12),
        _ => {
            let ch = event
                .text
                .as_ref()
                .and_then(|text| text.chars().next())
                .filter(|ch| !ch.is_control())
                .or_else(|| match &event.logical_key {
                    Key::Character(text) => text.chars().next(),
                    _ => None,
                })?;
            let ch = if key_modifiers.contains(KeyModifiers::CONTROL) {
                ch.to_ascii_lowercase()
            } else {
                ch
            };
            KeyCode::Char(ch)
        }
    };
    Some(KeyEvent::new(code, key_modifiers))
}

fn build_font_system() -> FontSystem {
    let fonts = [
        PRIMARY_REGULAR_FONT,
        PRIMARY_BOLD_FONT,
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

fn measured_char_width(font_bytes: &[u8]) -> usize {
    let face = Face::from_slice(font_bytes, 0).expect("embedded native font should parse");
    let glyph = face.glyph_index('m').unwrap_or_else(|| GlyphId(0));
    let advance = face.glyph_hor_advance(glyph).unwrap_or(0) as f32;
    let scale = DEFAULT_FONT_HEIGHT_PX as f32 / face.height() as f32;
    (advance * scale).ceil().max(1.0) as usize
}

fn measure_default_cell_metrics() -> NativeCellMetrics {
    NativeCellMetrics {
        cell_width_px: [
            PRIMARY_REGULAR_FONT,
            PRIMARY_BOLD_FONT,
            PRIMARY_ITALIC_FONT,
            FALLBACK_REGULAR_FONT,
            FALLBACK_BOLD_FONT,
        ]
        .into_iter()
        .map(measured_char_width)
        .max()
        .unwrap_or(1)
        .max(1),
        cell_height_px: DEFAULT_FONT_HEIGHT_PX as usize,
    }
}

fn glyphon_color(color: crate::buffer::GameColor) -> Color {
    let [r, g, b, _] = primitives::color_to_rgba(color);
    Color::rgb(r, g, b)
}

fn modifiers_to_crossterm(modifiers: ModifiersState) -> KeyModifiers {
    let mut mapped = KeyModifiers::empty();
    if modifiers.shift_key() {
        mapped.insert(KeyModifiers::SHIFT);
    }
    if modifiers.control_key() {
        mapped.insert(KeyModifiers::CONTROL);
    }
    if modifiers.alt_key() {
        mapped.insert(KeyModifiers::ALT);
    }
    mapped
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
        FALLBACK_BOLD_FONT, FALLBACK_REGULAR_FONT, PRIMARY_BOLD_FONT, PRIMARY_ITALIC_FONT,
        PRIMARY_REGULAR_FONT, cell_position_at_pixel, default_cell_metrics,
        terminal_grid_for_pixels,
    };
    use rustybuzz::{Face, ttf_parser::GlyphId};

    #[test]
    fn terminal_grid_uses_measured_font_dimensions() {
        let metrics = default_cell_metrics();
        assert_eq!(
            terminal_grid_for_pixels(
                (metrics.cell_width_px * 10) as u32,
                (metrics.cell_height_px * 3) as u32
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
                winit::dpi::PhysicalPosition::new(2.0, 3.0)
            ),
            None
        );
    }

    #[test]
    fn default_metrics_match_active_font_profile() {
        let expected = [
            PRIMARY_REGULAR_FONT,
            PRIMARY_BOLD_FONT,
            PRIMARY_ITALIC_FONT,
            FALLBACK_REGULAR_FONT,
            FALLBACK_BOLD_FONT,
        ]
        .into_iter()
        .map(|bytes| {
            let face = Face::from_slice(bytes, 0).expect("font");
            let glyph = face.glyph_index('m').unwrap_or_else(|| GlyphId(0));
            let advance = face.glyph_hor_advance(glyph).unwrap_or(0) as f32;
            let scale = super::DEFAULT_FONT_HEIGHT_PX as f32 / face.height() as f32;
            (advance * scale).ceil().max(1.0) as usize
        })
        .max()
        .expect("width");

        let metrics = default_cell_metrics();
        assert_eq!(metrics.cell_width_px, expected);
        assert_eq!(
            metrics.cell_height_px,
            super::DEFAULT_FONT_HEIGHT_PX as usize
        );
    }
}
