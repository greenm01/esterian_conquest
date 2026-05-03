use bytemuck::{Pod, Zeroable};
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use wgpu::{
    self, BindGroup, BindGroupLayout, BufferAddress, Device, Queue, Sampler, Texture,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::fonts::{ResolvedGlyph, render_alpha_glyph, shape_stormfaze_text};
use crate::geometry::{GridMapper, GridMetrics, PhysicalRect};
use crate::grid::{GameColor, OverlayLogo, OverlayLogoKind, Point};

use super::common::{linear_color_f32, pixel_to_ndc_x, pixel_to_ndc_y};

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
pub(super) struct LogoVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LogoSprite {
    x_px: u32,
    y_px: u32,
    width_px: u32,
    height_px: u32,
}

pub(super) struct LogoAtlas {
    _texture: Texture,
    view: TextureView,
    width_px: u32,
    height_px: u32,
    pub(super) sprites: [LogoSprite; LOGO_KIND_COUNT],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LogoPlacement {
    pub(super) sprite: LogoSprite,
    pub(super) rect: PhysicalRect,
    pub(super) color: GameColor,
}

pub(super) fn create_logo_pipeline(
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

pub(super) fn create_logo_bind_group(
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

pub(super) fn build_logo_vertices(
    placements: &[LogoPlacement],
    atlas: &LogoAtlas,
    surface_width: u32,
    surface_height: u32,
) -> Vec<LogoVertex> {
    let mut vertices = Vec::with_capacity(placements.len() * 6);
    for placement in placements {
        if placement.rect.width == 0 || placement.rect.height == 0 {
            continue;
        }
        let left = pixel_to_ndc_x(placement.rect.x, surface_width);
        let right = pixel_to_ndc_x(
            placement.rect.x.saturating_add(placement.rect.width),
            surface_width,
        );
        let top = pixel_to_ndc_y(placement.rect.y, surface_height);
        let bottom = pixel_to_ndc_y(
            placement.rect.y.saturating_add(placement.rect.height),
            surface_height,
        );
        let color = linear_color_f32(placement.color);
        let sprite = placement.sprite;
        let u0 = sprite.x_px as f32 / atlas.width_px as f32;
        let v0 = sprite.y_px as f32 / atlas.height_px as f32;
        let u1 = (sprite.x_px + sprite.width_px) as f32 / atlas.width_px as f32;
        let v1 = (sprite.y_px + sprite.height_px) as f32 / atlas.height_px as f32;
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

pub(super) fn logo_rect(mapper: GridMapper, overlay: OverlayLogo) -> PhysicalRect {
    let top_left = mapper.cell_rect(Point::from_usize(overlay.left_col, overlay.top_row));
    let (width_cols, height_rows) = overlay.kind.cell_size();
    PhysicalRect {
        x: top_left.x,
        y: top_left.y,
        width: width_cols.saturating_mul(mapper.cell.width_px),
        height: height_rows.saturating_mul(mapper.cell.height_px),
    }
}

pub(super) fn generate_logo_atlas(
    device: &Device,
    queue: &Queue,
    grid_metrics: GridMetrics,
) -> LogoAtlas {
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
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("nc-helm-logo-atlas"),
        size: wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
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
        _texture: texture,
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
