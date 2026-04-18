//! wgpu + glyphon GPU renderer for the nc-helm character grid.
//!
//! Each frame the renderer paints two layers:
//!
//! 1. **Background pixel buffer** (`background_pixels`): a CPU-side RGBA
//!    image the size of the surface. The playfield's per-cell background
//!    colour, `BackgroundMode::TextBand` strips, and the caret are all
//!    written into this buffer with `fill_rect_rgba`. It is uploaded to a
//!    GPU texture and drawn first as a fullscreen quad via the
//!    [`BACKGROUND_SHADER`] pipeline.
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

use glyphon::{
    fontdb, Attrs, Buffer as GlyphBuffer, Cache, Color, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Weight,
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

use super::primitives;
use crate::geometry::{caret_rect, GridMapper, GridMetrics};
use crate::grid::{
    BackgroundMode, CellStyle, GameColor, OverlayText, OverlayTextFamily, PlayfieldBuffer, Point,
    ScreenGeometry,
};

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

    pub fn grid_geometry_for_pixels(
        &mut self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> ScreenGeometry {
        self.sync_scale_metrics_for_scale(scale_factor);
        fit_grid_to_pixels(width, height, self.grid_metrics.cell)
    }

    /// Map a window-local pixel position to a grid cell, or `None` if the
    /// pixel falls outside the centred grid area.
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

    /// Render one frame of `playfield`.
    ///
    /// Steps, in order:
    /// 1. Sync DPI metrics and reconfigure the surface if the window resized.
    /// 2. Walk the playfield, paint per-cell backgrounds + caret into
    ///    `background_pixels`, and collect text runs as `TextPlacement`s
    ///    (see `prepare_playfield`).
    /// 3. Upload `background_pixels` to the GPU texture and `prepare` the
    ///    glyphon `TextRenderer` with the staged runs.
    /// 4. Acquire the swapchain frame, draw the background quad, then the
    ///    glyph layer on top, and present.
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
        }
    }

    /// Paint per-cell backgrounds + caret into `background_pixels`, batch
    /// adjacent same-style cells into glyphon text runs, and upload the
    /// background image to the GPU.
    ///
    /// Returns the staged text placements; the caller hands them to
    /// `glyphon::TextRenderer::prepare`.
    ///
    /// Run batching: contiguous non-space cells with identical `CellStyle`
    /// become one shaped run. Spaces flush the current run because they
    /// don't need glyph rendering — the background fill alone suffices, and
    /// breaking on spaces lets adjacent runs differ in style without a
    /// per-cell shaping cost.
    fn prepare_playfield(&mut self, playfield: &PlayfieldBuffer) -> Vec<TextPlacement> {
        let frame_width = self.surface_config.width as usize;
        let frame_height = self.surface_config.height as usize;
        let body_bg = primitives::color_to_rgba(GameColor::Rgb(0x12, 0x13, 0x1c));
        // Start every frame from the base body colour so cells outside the
        // grid (when the window doesn't divide evenly into cells) have a
        // defined colour rather than stale pixels from the previous frame.
        self.background_pixels
            .resize(frame_width * frame_height * 4, 0);
        for pixel in self.background_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&body_bg);
        }

        let geometry = ScreenGeometry::new(playfield.width(), playfield.height());
        let mapper =
            GridMapper::centered(frame_width, frame_height, geometry, self.grid_metrics.cell);
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
                let point = Point::from_usize(col_idx, row_idx);
                let rect = if style.bg_mode == BackgroundMode::TextBand {
                    mapper.text_band_rect(point, self.grid_metrics.text)
                } else {
                    mapper.cell_rect(point)
                };
                primitives::fill_rect_rgba(
                    &mut self.background_pixels,
                    frame_width,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    primitives::color_to_rgba(style.bg),
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

        prepare_overlay_texts(self, mapper, playfield.overlay_texts(), &mut placements);

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
        let bounds = overlay_pixel_bounds(mapper, overlay);
        if bounds.width <= 1 || bounds.height <= 1 {
            continue;
        }
        let family = match overlay.family {
            OverlayTextFamily::Stormfaze => TextFamilyKey::Named(STORMFAZE_FONT_FAMILY),
        };
        let font_size = fit_overlay_font_size(
            &mut renderer.font_system,
            &overlay.text,
            family,
            bounds.width as f32,
            bounds.height as f32,
        );
        if font_size <= 0.0 {
            continue;
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
}

fn overlay_pixel_bounds(
    mapper: GridMapper,
    overlay: &OverlayText,
) -> crate::geometry::PhysicalRect {
    let top_left = mapper.cell_rect(Point::from_usize(overlay.left_col, overlay.top_row));
    crate::geometry::PhysicalRect {
        x: top_left.x,
        y: top_left.y,
        width: overlay.width_cols.saturating_mul(mapper.cell.width_px),
        height: overlay.height_rows.saturating_mul(mapper.cell.height_px),
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

#[cfg(test)]
mod tests {
    use glyphon::{fontdb, Attrs, Buffer as GlyphBuffer, Family, Metrics, Shaping};

    use crate::geometry::{caret_rect, GridMapper, GridMetrics};
    use crate::grid::{Point, ScreenGeometry};

    use super::{
        build_font_system, expanded_text_bounds, fit_grid_to_pixels, measure_single_line_width,
        TextFamilyKey, TextOverhang, PRIMARY_FONT_FAMILY, STORMFAZE_FONT_FAMILY,
    };

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
