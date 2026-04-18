use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

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
use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;

use crate::geometry::{GridMapper, GridMetrics, caret_rect};
use crate::grid::{CellStyle, GameColor, PlayfieldBuffer, Point, ScreenGeometry};

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
    viewport: glyphon::Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: HashMap<TextBufferKey, CachedTextCell>,
    background_pipeline: wgpu::RenderPipeline,
    background_bind_group_layout: BindGroupLayout,
    background_sampler: Sampler,
    background_texture: BackgroundTexture,
    background_pixels: Vec<u8>,
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
            desired_maximum_frame_latency: 2,
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
            grid_metrics,
        })
    }

    pub fn grid_geometry_for_window(&mut self, width: u32, height: u32) -> ScreenGeometry {
        self.sync_scale_metrics();
        let cols = (width.max(1) as usize / self.grid_metrics.cell.width_px).max(1);
        let rows = (height.max(1) as usize / self.grid_metrics.cell.height_px).max(1);
        ScreenGeometry::new(cols, rows)
    }

    pub fn cell_position_at_pixel(
        &mut self,
        window: &winit::window::Window,
        geometry: ScreenGeometry,
        position: PhysicalPosition<f64>,
    ) -> Option<Point> {
        self.sync_scale_metrics();
        let size = window.inner_size();
        GridMapper::centered(
            size.width as usize,
            size.height as usize,
            geometry,
            self.grid_metrics.cell,
        )
        .pixel_to_cell(position)
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

        self.window.pre_present_notify();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(())
    }

    fn sync_scale_metrics(&mut self) {
        let scale_factor = self.window.scale_factor();
        let updated = GridMetrics::for_scale(scale_factor, &mut self.font_system);
        if updated != self.grid_metrics {
            self.grid_metrics = updated;
            self.text_buffers.clear();
        }
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

        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper = GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);
        let cursor_rect = playfield
            .cursor()
            .map(|point| caret_rect(point, mapper, self.grid_metrics.text));
        let mut placements = Vec::new();

        for row_idx in 0..playfield.height() {
            let mut run_start = None;
            let mut run_style: Option<CellStyle> = None;
            let mut run_text = String::new();
            for col_idx in 0..playfield.width() {
                let source = playfield.row(row_idx)[col_idx];
                let style = source.style;
                let rect = mapper.cell_rect(Point::from_usize(col_idx, row_idx));
                fill_rect_rgba(
                    &mut self.background_pixels,
                    frame_width,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    color_to_rgba(style.bg),
                );
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
        }

        if let Some(caret) = cursor_rect {
            fill_rect_rgba(
                &mut self.background_pixels,
                frame_width,
                caret.x,
                caret.y,
                caret.width,
                caret.height,
                color_to_rgba(GameColor::BrightWhite),
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
                self.grid_metrics.text.font_size_px,
                self.grid_metrics.text.line_height_px,
            ),
        );
        buffer.set_size(
            &mut self.font_system,
            None,
            Some(self.grid_metrics.cell.height_px as f32),
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
    db.set_monospace_family(PRIMARY_FONT_FAMILY.to_string());
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
    let key = TextBufferKey {
        text: Arc::<str>::from(run_text.as_str()),
        bold: style.bold,
    };
    renderer.ensure_text_buffer(key.clone());
    let start = Point::from_usize(start_col, row_idx);
    let text_origin = mapper.text_origin(start, renderer.grid_metrics.text);
    let start_rect = mapper.cell_rect(start);
    placements.push(TextPlacement {
        key,
        left: text_origin.left,
        top: text_origin.top,
        bounds: TextBounds {
            left: start_rect.x as i32,
            top: start_rect.y as i32,
            right: (mapper.origin_x + end_col * mapper.cell.width_px) as i32,
            bottom: (start_rect.y + mapper.cell.height_px) as i32,
        },
        color: style.fg,
    });
    run_text.clear();
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

#[cfg(test)]
mod tests {
    use crate::geometry::{GridMapper, GridMetrics, caret_rect};
    use crate::grid::{Point, ScreenGeometry};

    fn mapper() -> (GridMapper, GridMetrics) {
        let metrics = GridMetrics {
            cell: crate::geometry::CellMetrics {
                width_px: 12,
                height_px: 24,
            },
            text: crate::geometry::TextMetrics {
                font_size_px: 18.0,
                line_height_px: 24.0,
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
        let rect = mapper.cell_rect(Point::from_usize(31, 16));
        let caret = caret_rect(Point::from_usize(31, 16), mapper, metrics.text);
        assert_eq!(caret.x, rect.x);
        assert_eq!(caret.y, rect.y);
        assert_eq!(caret.width, 2);
        assert_eq!(caret.height, 24);
    }
}
