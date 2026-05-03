use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use swash::scale::ScaleContext;
use wgpu::{
    self, BindGroupLayout, Buffer, Device, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::fonts::{render_alpha_glyph, resolve_mono_glyph};
use crate::geometry::GridMetrics;

use super::common::{
    atlas_slot_contains_pixel, create_fullscreen_pipeline, sampler_layout_entry,
    texture_layout_entry, uniform_layout_entry,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct MatrixStateUniform {
    grid_width: f32,
    grid_height: f32,
    time_seconds: f32,
    frame_count: f32,
    fall_speed: f32,
    cycle_speed: f32,
    raindrop_length: f32,
    brightness_decay: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct MatrixFinalUniform {
    surface_width: f32,
    surface_height: f32,
    cell_width: f32,
    cell_height: f32,
    atlas_width: f32,
    atlas_height: f32,
    glyph_count: f32,
    time_seconds: f32,
    grid_width: f32,
    grid_height: f32,
    _pad0: f32,
    _pad1: f32,
}

const MATRIX_SHADER_FALL_SPEED: f32 = 0.3;
const MATRIX_SHADER_CYCLE_SPEED: f32 = 0.03;
const MATRIX_SHADER_RAINDROP_LENGTH: f32 = 0.75;
const MATRIX_SHADER_BRIGHTNESS_DECAY: f32 = 1.0;

const MATRIX_SHADER_GLYPHS: [char; 35] = [
    'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο', 'π', 'ρ', 'σ', 'ς',
    'τ', 'υ', 'φ', 'χ', 'ψ', 'ω', 'ϲ', 'ϛ', 'ϟ', 'ϡ', '´', '`', '῀', '᾿', '῾', 'ͅ',
];

const MATRIX_RAINDROP_SHADER: &str = r#"
struct StateUniform {
    grid_width: f32,
    grid_height: f32,
    time_seconds: f32,
    frame_count: f32,
    fall_speed: f32,
    cycle_speed: f32,
    raindrop_length: f32,
    brightness_decay: f32,
};

@group(0) @binding(0) var<uniform> state: StateUniform;
@group(0) @binding(1) var previous_raindrop: texture_2d<f32>;
@group(0) @binding(2) var state_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

fn random_float(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn rain_brightness(t: f32, cell: vec2<f32>) -> f32 {
    let fall_speed = max(state.fall_speed, 0.001);
    let raindrop_length = max(state.raindrop_length, 0.05);
    let column_time_offset = random_float(vec2<f32>(cell.x, 0.0)) * 1000.0;
    let column_speed_offset = random_float(vec2<f32>(cell.x + 0.1, 0.0)) * 0.5 + 0.5;
    let column_time = column_time_offset + t * fall_speed * column_speed_offset;
    let glyph_y = max(state.grid_height, 1.0) - cell.y - 1.0;
    let rain_time = (glyph_y * 0.01 + column_time) / raindrop_length;
    return 1.0 - fract(rain_time);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let grid = max(vec2<f32>(state.grid_width, state.grid_height), vec2<f32>(1.0, 1.0));
    let cell = floor(in.position.xy);
    let prev_uv = (cell + vec2<f32>(0.5, 0.5)) / grid;
    let prev = textureSample(previous_raindrop, state_sampler, prev_uv);
    var brightness = rain_brightness(state.time_seconds, cell);
    let brightness_below = rain_brightness(state.time_seconds, cell + vec2<f32>(0.0, 1.0));
    let cursor = select(0.0, 1.0, brightness > brightness_below);
    if (state.frame_count > 0.5) {
        brightness = mix(prev.r, brightness, clamp(state.brightness_decay, 0.0, 1.0));
    }
    return vec4<f32>(clamp(brightness, 0.0, 1.0), cursor, 1.0, 1.0);
}
"#;

const MATRIX_SYMBOL_SHADER: &str = r#"
struct StateUniform {
    grid_width: f32,
    grid_height: f32,
    time_seconds: f32,
    frame_count: f32,
    fall_speed: f32,
    cycle_speed: f32,
    raindrop_length: f32,
    glyph_count: f32,
};

@group(0) @binding(0) var<uniform> state: StateUniform;
@group(0) @binding(1) var previous_symbol: texture_2d<f32>;
@group(0) @binding(2) var raindrop_state: texture_2d<f32>;
@group(0) @binding(3) var state_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

fn random_float(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let grid = max(vec2<f32>(state.grid_width, state.grid_height), vec2<f32>(1.0, 1.0));
    let glyph_count = max(state.glyph_count, 1.0);
    let cell = floor(in.position.xy);
    let state_uv = (cell + vec2<f32>(0.5, 0.5)) / grid;
    let prev = textureSample(previous_symbol, state_sampler, state_uv);
    let rain = textureSample(raindrop_state, state_sampler, state_uv);
    let animation_speed = max(rain.r * 2.0, 0.15);
    let cycle_rate = max(state.cycle_speed, 0.001) * 25.0 * animation_speed;
    let phase = random_float(cell + vec2<f32>(17.0, 19.0));
    let bucket = floor((state.time_seconds + phase * 8.0) * cycle_rate);
    let symbol_seed = random_float(cell + vec2<f32>(bucket * 0.37 + 7.0, bucket * 0.11 + 13.0));
    let symbol = floor(symbol_seed * glyph_count);
    let age = fract(prev.g + cycle_rate * 0.04);
    return vec4<f32>((symbol + 0.5) / glyph_count, age, 0.0, 1.0);
}
"#;

const MATRIX_FINAL_SHADER: &str = r#"
struct FinalUniform {
    surface_width: f32,
    surface_height: f32,
    cell_width: f32,
    cell_height: f32,
    atlas_width: f32,
    atlas_height: f32,
    glyph_count: f32,
    time_seconds: f32,
    grid_width: f32,
    grid_height: f32,
    pad0: f32,
    pad1: f32,
};

@group(0) @binding(0) var<uniform> params: FinalUniform;
@group(0) @binding(1) var raindrop_state: texture_2d<f32>;
@group(0) @binding(2) var symbol_state: texture_2d<f32>;
@group(0) @binding(3) var glyph_tex: texture_2d<f32>;
@group(0) @binding(4) var state_sampler: sampler;
@group(0) @binding(5) var glyph_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let surface = vec2<f32>(params.surface_width, params.surface_height);
    let cell_size = max(vec2<f32>(params.cell_width, params.cell_height), vec2<f32>(1.0, 1.0));
    let top_pixel = in.position.xy;
    if (top_pixel.x < 0.0 || top_pixel.y < 0.0 || top_pixel.x >= surface.x || top_pixel.y >= surface.y) {
        discard;
    }
    let grid = max(vec2<f32>(params.grid_width, params.grid_height), vec2<f32>(1.0, 1.0));
    let cell = floor(top_pixel / cell_size);
    if (cell.x < 0.0 || cell.y < 0.0 || cell.x >= grid.x || cell.y >= grid.y) {
        discard;
    }
    let local = fract(top_pixel / cell_size);
    let state_uv = (cell + vec2<f32>(0.5, 0.5)) / grid;
    let rain = textureSample(raindrop_state, state_sampler, state_uv);
    let symbol_sample = textureSample(symbol_state, state_sampler, state_uv);
    let glyph_count = max(params.glyph_count, 1.0);
    let symbol = clamp(floor(clamp(symbol_sample.r, 0.0, 0.99999) * glyph_count), 0.0, glyph_count - 1.0);
    let glyph_uv = vec2<f32>((symbol + local.x) / glyph_count, local.y);
    let alpha = textureSample(glyph_tex, glyph_sampler, glyph_uv).r;
    let brightness = rain.r * 1.1 - 0.5;
    if (brightness <= 0.0 || alpha <= 0.01) {
        discard;
    }
    let trail = vec3<f32>(0.0, 0.82, 0.04) * brightness;
    let cursor = vec3<f32>(0.86, 1.0, 0.86) * max(brightness, 0.55);
    let color = mix(trail, cursor, step(0.5, rain.g));
    return vec4<f32>(color * alpha, 1.0);
}
"#;

struct MatrixGlyphAtlas {
    _texture: Texture,
    view: TextureView,
    width_px: u32,
    height_px: u32,
    glyph_count: u32,
}

struct MatrixStateTarget {
    _texture: Texture,
    view: TextureView,
}

struct MatrixStateTargets {
    grid_width: u32,
    grid_height: u32,
    raindrop: [MatrixStateTarget; 2],
    symbol: [MatrixStateTarget; 2],
}

pub(super) struct MatrixRenderer {
    raindrop_pipeline: wgpu::RenderPipeline,
    raindrop_bind_group_layout: BindGroupLayout,
    symbol_pipeline: wgpu::RenderPipeline,
    symbol_bind_group_layout: BindGroupLayout,
    final_pipeline: wgpu::RenderPipeline,
    final_bind_group_layout: BindGroupLayout,
    state_sampler: Sampler,
    glyph_sampler: Sampler,
    state_uniform_buffer: Buffer,
    symbol_uniform_buffer: Buffer,
    final_uniform_buffer: Buffer,
    glyph_atlas: MatrixGlyphAtlas,
    targets: Option<MatrixStateTargets>,
    current_raindrop: usize,
    current_symbol: usize,
    frame_count: u32,
    clock_start: Option<Instant>,
}

pub(super) struct MatrixFrameSetup {
    pub state_recreated: bool,
}

impl MatrixRenderer {
    pub(super) fn new(
        device: &Device,
        queue: &Queue,
        surface_format: TextureFormat,
        grid_metrics: GridMetrics,
    ) -> Self {
        let (raindrop_pipeline, raindrop_bind_group_layout) =
            create_matrix_raindrop_pipeline(device);
        let (symbol_pipeline, symbol_bind_group_layout) = create_matrix_symbol_pipeline(device);
        let (final_pipeline, final_bind_group_layout) =
            create_matrix_final_pipeline(device, surface_format);
        let state_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("nc-helm-matrix-state-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..SamplerDescriptor::default()
        });
        let glyph_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("nc-helm-matrix-glyph-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..SamplerDescriptor::default()
        });
        let state_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-matrix-state-uniform-buffer"),
            size: std::mem::size_of::<MatrixStateUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let symbol_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-matrix-symbol-uniform-buffer"),
            size: std::mem::size_of::<MatrixStateUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let final_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nc-helm-matrix-final-uniform-buffer"),
            size: std::mem::size_of::<MatrixFinalUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            raindrop_pipeline,
            raindrop_bind_group_layout,
            symbol_pipeline,
            symbol_bind_group_layout,
            final_pipeline,
            final_bind_group_layout,
            state_sampler,
            glyph_sampler,
            state_uniform_buffer,
            symbol_uniform_buffer,
            final_uniform_buffer,
            glyph_atlas: generate_matrix_glyph_atlas(device, queue, grid_metrics),
            targets: None,
            current_raindrop: 0,
            current_symbol: 0,
            frame_count: 0,
            clock_start: None,
        }
    }

    pub(super) fn reset_clock(&mut self) {
        self.clock_start = None;
    }

    pub(super) fn sync_scale_metrics(
        &mut self,
        device: &Device,
        queue: &Queue,
        grid_metrics: GridMetrics,
    ) {
        self.glyph_atlas = generate_matrix_glyph_atlas(device, queue, grid_metrics);
        self.targets = None;
    }

    pub(super) fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_width: u32,
        surface_height: u32,
        grid_metrics: GridMetrics,
    ) -> MatrixFrameSetup {
        let cell_width = grid_metrics.cell.width_px.max(1) as u32;
        let cell_height = grid_metrics.cell.height_px.max(1) as u32;
        let (grid_width, grid_height) =
            matrix_grid_dimensions(surface_width, surface_height, cell_width, cell_height);
        let state_recreated = self.ensure_state_targets(device, grid_width, grid_height);

        let now = Instant::now();
        let start = *self.clock_start.get_or_insert(now);
        let time_seconds = now.duration_since(start).as_secs_f32();
        let frame_count = self.frame_count as f32;

        let state_uniform = MatrixStateUniform {
            grid_width: grid_width as f32,
            grid_height: grid_height as f32,
            time_seconds,
            frame_count,
            fall_speed: MATRIX_SHADER_FALL_SPEED,
            cycle_speed: MATRIX_SHADER_CYCLE_SPEED,
            raindrop_length: MATRIX_SHADER_RAINDROP_LENGTH,
            brightness_decay: MATRIX_SHADER_BRIGHTNESS_DECAY,
        };
        queue.write_buffer(
            &self.state_uniform_buffer,
            0,
            bytemuck::bytes_of(&state_uniform),
        );
        let symbol_uniform = MatrixStateUniform {
            brightness_decay: self.glyph_atlas.glyph_count as f32,
            ..state_uniform
        };
        queue.write_buffer(
            &self.symbol_uniform_buffer,
            0,
            bytemuck::bytes_of(&symbol_uniform),
        );
        let final_uniform = MatrixFinalUniform {
            surface_width: surface_width as f32,
            surface_height: surface_height as f32,
            cell_width: cell_width as f32,
            cell_height: cell_height as f32,
            atlas_width: self.glyph_atlas.width_px as f32,
            atlas_height: self.glyph_atlas.height_px as f32,
            glyph_count: self.glyph_atlas.glyph_count as f32,
            time_seconds,
            grid_width: grid_width as f32,
            grid_height: grid_height as f32,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        queue.write_buffer(
            &self.final_uniform_buffer,
            0,
            bytemuck::bytes_of(&final_uniform),
        );

        MatrixFrameSetup { state_recreated }
    }

    pub(super) fn clear_state_targets(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(targets) = self.targets.as_ref() else {
            return;
        };
        for target in targets.raindrop.iter().chain(targets.symbol.iter()) {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-helm-matrix-clear-state-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.view,
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
        }
    }

    pub(super) fn encode_passes(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
    ) {
        let targets = self
            .targets
            .as_ref()
            .expect("matrix state targets should be initialized before rendering");
        let raindrop_src = self.current_raindrop;
        let raindrop_dst = 1 - raindrop_src;
        let symbol_src = self.current_symbol;
        let symbol_dst = 1 - symbol_src;

        let raindrop_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nc-helm-matrix-raindrop-bind-group"),
            layout: &self.raindrop_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.state_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.raindrop[raindrop_src].view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.state_sampler),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-helm-matrix-raindrop-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.raindrop[raindrop_dst].view,
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
            pass.set_pipeline(&self.raindrop_pipeline);
            pass.set_bind_group(0, &raindrop_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.current_raindrop = raindrop_dst;

        let symbol_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nc-helm-matrix-symbol-bind-group"),
            layout: &self.symbol_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.symbol_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.symbol[symbol_src].view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.raindrop[self.current_raindrop].view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.state_sampler),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-helm-matrix-symbol-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.symbol[symbol_dst].view,
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
            pass.set_pipeline(&self.symbol_pipeline);
            pass.set_bind_group(0, &symbol_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.current_symbol = symbol_dst;

        let final_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nc-helm-matrix-final-bind-group"),
            layout: &self.final_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.final_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.raindrop[self.current_raindrop].view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.symbol[self.current_symbol].view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.glyph_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.state_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.glyph_sampler),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("nc-helm-matrix-final-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: surface_view,
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
            pass.set_pipeline(&self.final_pipeline);
            pass.set_bind_group(0, &final_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    pub(super) fn finish_frame(&mut self) {
        self.frame_count = self.frame_count.saturating_add(1);
    }

    fn ensure_state_targets(&mut self, device: &Device, grid_width: u32, grid_height: u32) -> bool {
        if self.targets.as_ref().is_some_and(|targets| {
            targets.grid_width == grid_width && targets.grid_height == grid_height
        }) {
            return false;
        }
        self.targets = Some(MatrixStateTargets {
            grid_width,
            grid_height,
            raindrop: [
                create_matrix_state_target(device, grid_width, grid_height, "matrix-raindrop-a"),
                create_matrix_state_target(device, grid_width, grid_height, "matrix-raindrop-b"),
            ],
            symbol: [
                create_matrix_state_target(device, grid_width, grid_height, "matrix-symbol-a"),
                create_matrix_state_target(device, grid_width, grid_height, "matrix-symbol-b"),
            ],
        });
        self.current_raindrop = 0;
        self.current_symbol = 0;
        self.frame_count = 0;
        true
    }
}

fn create_matrix_state_target(
    device: &Device,
    width: u32,
    height: u32,
    label: &'static str,
) -> MatrixStateTarget {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    MatrixStateTarget {
        _texture: texture,
        view,
    }
}

fn create_matrix_raindrop_pipeline(device: &Device) -> (wgpu::RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nc-helm-matrix-raindrop-shader"),
        source: wgpu::ShaderSource::Wgsl(MATRIX_RAINDROP_SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-helm-matrix-raindrop-bind-group-layout"),
        entries: &[
            uniform_layout_entry(0),
            texture_layout_entry(1),
            sampler_layout_entry(2),
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nc-helm-matrix-raindrop-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = create_fullscreen_pipeline(
        device,
        &shader,
        &pipeline_layout,
        TextureFormat::Rgba8Unorm,
        "nc-helm-matrix-raindrop-pipeline",
    );
    (pipeline, bind_group_layout)
}

fn create_matrix_symbol_pipeline(device: &Device) -> (wgpu::RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nc-helm-matrix-symbol-shader"),
        source: wgpu::ShaderSource::Wgsl(MATRIX_SYMBOL_SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-helm-matrix-symbol-bind-group-layout"),
        entries: &[
            uniform_layout_entry(0),
            texture_layout_entry(1),
            texture_layout_entry(2),
            sampler_layout_entry(3),
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nc-helm-matrix-symbol-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = create_fullscreen_pipeline(
        device,
        &shader,
        &pipeline_layout,
        TextureFormat::Rgba8Unorm,
        "nc-helm-matrix-symbol-pipeline",
    );
    (pipeline, bind_group_layout)
}

fn create_matrix_final_pipeline(
    device: &Device,
    surface_format: TextureFormat,
) -> (wgpu::RenderPipeline, BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nc-helm-matrix-final-shader"),
        source: wgpu::ShaderSource::Wgsl(MATRIX_FINAL_SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nc-helm-matrix-final-bind-group-layout"),
        entries: &[
            uniform_layout_entry(0),
            texture_layout_entry(1),
            texture_layout_entry(2),
            texture_layout_entry(3),
            sampler_layout_entry(4),
            sampler_layout_entry(5),
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nc-helm-matrix-final-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = create_fullscreen_pipeline(
        device,
        &shader,
        &pipeline_layout,
        surface_format,
        "nc-helm-matrix-final-pipeline",
    );
    (pipeline, bind_group_layout)
}

fn generate_matrix_glyph_atlas(
    device: &Device,
    queue: &Queue,
    grid_metrics: GridMetrics,
) -> MatrixGlyphAtlas {
    let slot_w = grid_metrics.cell.width_px.max(1) as u32;
    let slot_h = grid_metrics.cell.height_px.max(1) as u32;
    let glyph_count = MATRIX_SHADER_GLYPHS.len() as u32;
    let atlas_width = slot_w * glyph_count;
    let atlas_height = slot_h;

    let texture = device.create_texture(&TextureDescriptor {
        label: Some("nc-helm-matrix-glyph-atlas"),
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

    for (index, ch) in MATRIX_SHADER_GLYPHS.iter().enumerate() {
        let slot_left = index as i32 * slot_w as i32;
        let slot_top = 0;
        let slot_right = slot_left + slot_w as i32;
        let slot_bottom = slot_h as i32;
        let Some(glyph) = resolve_mono_glyph(*ch, false) else {
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
                    len if len == (image.placement.width * image.placement.height * 4) as usize => {
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

    MatrixGlyphAtlas {
        _texture: texture,
        view,
        width_px: atlas_width,
        height_px: atlas_height,
        glyph_count,
    }
}

fn matrix_grid_dimensions(
    surface_width: u32,
    surface_height: u32,
    cell_width: u32,
    cell_height: u32,
) -> (u32, u32) {
    (
        (surface_width / cell_width.max(1)).max(1),
        (surface_height / cell_height.max(1)).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MATRIX_SHADER_BRIGHTNESS_DECAY, MATRIX_SHADER_CYCLE_SPEED, MATRIX_SHADER_FALL_SPEED,
        MATRIX_SHADER_GLYPHS, MATRIX_SHADER_RAINDROP_LENGTH, matrix_grid_dimensions,
    };

    #[test]
    fn matrix_shader_defaults_match_lockme_runtime_defaults() {
        assert_eq!(MATRIX_SHADER_FALL_SPEED, 0.3);
        assert_eq!(MATRIX_SHADER_CYCLE_SPEED, 0.03);
        assert_eq!(MATRIX_SHADER_RAINDROP_LENGTH, 0.75);
        assert_eq!(MATRIX_SHADER_BRIGHTNESS_DECAY, 1.0);
    }

    #[test]
    fn matrix_shader_glyphs_match_lockme_repertoire() {
        assert_eq!(
            MATRIX_SHADER_GLYPHS.iter().collect::<String>(),
            "αβγδεζηθικλμνξοπρσςτυφχψωϲϛϟϡ´`῀᾿῾ͅ"
        );
    }

    #[test]
    fn matrix_grid_dimensions_use_full_surface_cell_floor() {
        assert_eq!(matrix_grid_dimensions(1919, 1079, 8, 16), (239, 67));
        assert_eq!(matrix_grid_dimensions(1, 1, 8, 16), (1, 1));
        assert_eq!(matrix_grid_dimensions(80, 48, 0, 0), (80, 48));
    }
}
