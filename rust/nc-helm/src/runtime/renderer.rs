use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight, fontdb,
};
use nc_ui::{CellStyle, GameColor, PlayfieldBuffer};
use wgpu::{
    self, BindGroup, BindGroupLayout, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Sampler,
    SamplerDescriptor, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event_loop::ActiveEventLoop;

const FONT_SIZE: f32 = 18.0;
const LINE_HEIGHT: f32 = 24.0;
const LEFT_INSET: f32 = 1.0;
const TOP_INSET: f32 = 2.0;
const CELL_WIDTH: f32 = 12.0;
const CELL_HEIGHT: f32 = 24.0;
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

const PRIMARY_FONT_FAMILY: &str = "0xProto Nerd Font Mono";
const PRIMARY_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Regular.ttf"
));
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/0xProtoNerdFontMono-Bold.ttf"
));
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Regular.ttf"
));

#[derive(Clone, Copy, Debug)]
struct CellMetrics {
    width_px: usize,
    height_px: usize,
}

#[derive(Clone, Copy, Debug)]
struct TextMetrics {
    font_size_px: f32,
    line_height_px: f32,
    left_inset_px: f32,
    top_inset_px: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextBufferKey {
    text: Arc<str>,
    bold: bool,
}

struct CachedTextCell {
    buffer: GlyphBuffer,
}

struct TextPlacement {
    key: TextBufferKey,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: GameColor,
}

struct BackgroundTexture {
    texture: Texture,
    bind_group: BindGroup,
}

pub struct Renderer {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: Device,
    queue: Queue,
    surface_config: SurfaceConfiguration,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: HashMap<TextBufferKey, CachedTextCell>,
    background_pipeline: wgpu::RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    background_pixels: Vec<u8>,
    cell_metrics: CellMetrics,
    text_metrics: TextMetrics,
}

impl Renderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        event_loop: &ActiveEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let scale_factor = window.scale_factor();
        let cell_metrics = cell_metrics_for_scale(scale_factor);
        let text_metrics = text_metrics_for_scale(scale_factor);
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
            cell_metrics,
            text_metrics,
        })
    }

    pub fn render(
        &mut self,
        playfield: &PlayfieldBuffer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sync_scale_metrics();
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        self.ensure_surface_size(size.width, size.height);
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let placements = self.prepare_playfield(playfield);
        let text_areas = placements
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

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(());
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
                        load: LoadOp::Clear(wgpu::Color::BLACK),
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

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(())
    }

    pub fn cell_position_at_pixel(
        &self,
        window: &winit::window::Window,
        position: PhysicalPosition<f64>,
    ) -> Option<(u16, u16)> {
        let size = window.inner_size();
        cell_position_at_pixel(
            self.surface_config.width as usize / self.cell_metrics.width_px,
            self.surface_config.height as usize / self.cell_metrics.height_px,
            size.width,
            size.height,
            window.scale_factor(),
            position,
        )
    }

    fn sync_scale_metrics(&mut self) {
        let scale_factor = self.window.scale_factor();
        self.cell_metrics = cell_metrics_for_scale(scale_factor);
        self.text_metrics = text_metrics_for_scale(scale_factor);
    }

    fn prepare_playfield(&mut self, playfield: &PlayfieldBuffer) -> Vec<TextPlacement> {
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let body_bg = color_to_rgba(GameColor::Rgb(0x12, 0x13, 0x1c));
        self.background_pixels
            .resize(frame_width * frame_height * 4, 0);
        for pixel in self.background_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&body_bg);
        }

        let grid_pixel_width = playfield.width() * self.cell_metrics.width_px;
        let grid_pixel_height = playfield.height() * self.cell_metrics.height_px;
        let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
        let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;
        let cursor = playfield
            .cursor()
            .map(|(col, row)| (usize::from(col), usize::from(row)));
        let mut placements = Vec::new();

        for row_idx in 0..playfield.height() {
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
                let cell_x = x_offset + col_idx * self.cell_metrics.width_px;
                let cell_y = y_offset + row_idx * self.cell_metrics.height_px;
                fill_rect_rgba(
                    &mut self.background_pixels,
                    frame_width,
                    cell_x,
                    cell_y,
                    self.cell_metrics.width_px,
                    self.cell_metrics.height_px,
                    color_to_rgba(style.bg),
                );
                if source.ch == ' ' {
                    flush_run(
                        self,
                        &mut placements,
                        &mut run_start,
                        &mut run_style,
                        &mut run_text,
                        x_offset,
                        y_offset,
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
                        &mut placements,
                        &mut run_start,
                        &mut run_style,
                        &mut run_text,
                        x_offset,
                        y_offset,
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
            Some(self.cell_metrics.height_px as f32),
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
    }
}

fn build_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    let db = font_system.db_mut();
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        PRIMARY_REGULAR_FONT,
    ))));
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        PRIMARY_BOLD_FONT,
    ))));
    db.load_font_source(fontdb::Source::Binary(Arc::new(Cow::Borrowed(
        FALLBACK_REGULAR_FONT,
    ))));
    let primary_family = PRIMARY_FONT_FAMILY.to_string();
    db.set_monospace_family(primary_family);
    font_system
}

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

fn flush_run(
    renderer: &mut Renderer,
    placements: &mut Vec<TextPlacement>,
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
    let style = run_style.take().expect("run style exists");
    if run_text.is_empty() {
        return;
    }
    let key = TextBufferKey {
        text: Arc::<str>::from(run_text.as_str()),
        bold: style.bold,
    };
    renderer.ensure_text_buffer(key.clone());
    let left = x_offset as f32
        + start_col as f32 * renderer.cell_metrics.width_px as f32
        + renderer.text_metrics.left_inset_px;
    let top = y_offset as f32
        + row_idx as f32 * renderer.cell_metrics.height_px as f32
        + renderer.text_metrics.top_inset_px;
    placements.push(TextPlacement {
        key,
        left,
        top,
        bounds: TextBounds {
            left: (x_offset + start_col * renderer.cell_metrics.width_px) as i32,
            top: (y_offset + row_idx * renderer.cell_metrics.height_px) as i32,
            right: (x_offset + end_col * renderer.cell_metrics.width_px) as i32,
            bottom: (y_offset + (row_idx + 1) * renderer.cell_metrics.height_px) as i32,
        },
        color: style.fg,
    });
    run_text.clear();
}

fn cell_metrics_for_scale(scale_factor: f64) -> CellMetrics {
    CellMetrics {
        width_px: (CELL_WIDTH * scale_factor as f32).round().max(1.0) as usize,
        height_px: (CELL_HEIGHT * scale_factor as f32).round().max(1.0) as usize,
    }
}

fn text_metrics_for_scale(scale_factor: f64) -> TextMetrics {
    let scale = scale_factor as f32;
    TextMetrics {
        font_size_px: (FONT_SIZE * scale).max(1.0),
        line_height_px: (LINE_HEIGHT * scale).max(1.0),
        left_inset_px: LEFT_INSET * scale,
        top_inset_px: TOP_INSET * scale,
    }
}

pub fn logical_window_size_for_grid(cols: usize, rows: usize) -> LogicalSize<f64> {
    LogicalSize::new(
        (cols as f32 * CELL_WIDTH) as f64,
        (rows as f32 * CELL_HEIGHT) as f64,
    )
}

pub fn terminal_grid_for_pixels(
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
) -> (u16, u16) {
    let metrics = cell_metrics_for_scale(scale_factor);
    let cols = (pixel_width.max(1) as usize / metrics.width_px).max(1);
    let rows = (pixel_height.max(1) as usize / metrics.height_px).max(1);
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
    position: PhysicalPosition<f64>,
) -> Option<(u16, u16)> {
    if !position.x.is_finite() || !position.y.is_finite() || position.x < 0.0 || position.y < 0.0 {
        return None;
    }
    let metrics = cell_metrics_for_scale(scale_factor);
    let x = position.x.floor() as usize;
    let y = position.y.floor() as usize;
    let grid_pixel_width = grid_cols.checked_mul(metrics.width_px)?;
    let grid_pixel_height = grid_rows.checked_mul(metrics.height_px)?;
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
    Some((
        (local_x / metrics.width_px) as u16,
        (local_y / metrics.height_px) as u16,
    ))
}

fn fill_rect_rgba(
    frame: &mut [u8],
    stride_px: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    for row in 0..height {
        let row_start = ((y0 + row) * stride_px + x0) * 4;
        for col in 0..width {
            let pixel = row_start + col * 4;
            if pixel + 4 <= frame.len() {
                frame[pixel..pixel + 4].copy_from_slice(&color);
            }
        }
    }
}

fn color_to_rgba(color: GameColor) -> [u8; 4] {
    let (r, g, b) = match color {
        GameColor::Black => (0x00, 0x00, 0x00),
        GameColor::Red => (0x80, 0x00, 0x00),
        GameColor::Green => (0x00, 0x80, 0x00),
        GameColor::Yellow => (0x80, 0x80, 0x00),
        GameColor::Blue => (0x00, 0x00, 0x80),
        GameColor::Magenta => (0x80, 0x00, 0x80),
        GameColor::Cyan => (0x00, 0x80, 0x80),
        GameColor::White => (0xc0, 0xc0, 0xc0),
        GameColor::BrightBlack => (0x80, 0x80, 0x80),
        GameColor::BrightRed => (0xff, 0x00, 0x00),
        GameColor::BrightGreen => (0x00, 0xff, 0x00),
        GameColor::BrightYellow => (0xff, 0xff, 0x00),
        GameColor::BrightBlue => (0x00, 0x00, 0xff),
        GameColor::BrightMagenta => (0xff, 0x00, 0xff),
        GameColor::BrightCyan => (0x00, 0xff, 0xff),
        GameColor::BrightWhite => (0xff, 0xff, 0xff),
        GameColor::Indexed(index) => ansi_indexed_rgb(index),
        GameColor::Rgb(r, g, b) => (r, g, b),
    };
    [r, g, b, 0xff]
}

fn ansi_indexed_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => match index {
            0 => (0x00, 0x00, 0x00),
            1 => (0x80, 0x00, 0x00),
            2 => (0x00, 0x80, 0x00),
            3 => (0x80, 0x80, 0x00),
            4 => (0x00, 0x00, 0x80),
            5 => (0x80, 0x00, 0x80),
            6 => (0x00, 0x80, 0x80),
            7 => (0xc0, 0xc0, 0xc0),
            8 => (0x80, 0x80, 0x80),
            9 => (0xff, 0x00, 0x00),
            10 => (0x00, 0xff, 0x00),
            11 => (0xff, 0xff, 0x00),
            12 => (0x00, 0x00, 0xff),
            13 => (0xff, 0x00, 0xff),
            14 => (0x00, 0xff, 0xff),
            _ => (0xff, 0xff, 0xff),
        },
        16..=231 => {
            let idx = index - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;
            let expand = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            (expand(r), expand(g), expand(b))
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
    }
}

fn glyphon_color(color: GameColor) -> Color {
    let [r, g, b, _] = color_to_rgba(color);
    Color::rgb(r, g, b)
}
