use wgpu::{self, Device, TextureFormat};

use super::primitives;
use crate::grid::GameColor;

pub(super) fn create_fullscreen_pipeline(
    device: &Device,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    format: TextureFormat,
    label: &'static str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) fn uniform_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(super) fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

pub(super) fn sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub(super) fn linear_color_f32(color: GameColor) -> [f32; 4] {
    let [r, g, b, a] = primitives::color_to_rgba(color);
    [
        linear_channel_from_srgb_u8(r) as f32,
        linear_channel_from_srgb_u8(g) as f32,
        linear_channel_from_srgb_u8(b) as f32,
        f32::from(a) / 255.0,
    ]
}

pub(super) fn pixel_to_ndc_x(x_px: usize, surface_width_px: u32) -> f32 {
    (x_px as f32 / surface_width_px.max(1) as f32) * 2.0 - 1.0
}

pub(super) fn pixel_to_ndc_y(y_px: usize, surface_height_px: u32) -> f32 {
    1.0 - (y_px as f32 / surface_height_px.max(1) as f32) * 2.0
}

pub(super) fn linear_channel_from_srgb_u8(value: u8) -> f64 {
    let srgb = f64::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn clear_color(color: GameColor) -> wgpu::Color {
    let [r, g, b, a] = primitives::color_to_rgba(color);
    wgpu::Color {
        r: linear_channel_from_srgb_u8(r),
        g: linear_channel_from_srgb_u8(g),
        b: linear_channel_from_srgb_u8(b),
        a: f64::from(a) / 255.0,
    }
}

pub(super) fn atlas_slot_contains_pixel(
    slot_left: i32,
    slot_top: i32,
    slot_right: i32,
    slot_bottom: i32,
    dst_x: i32,
    dst_y: i32,
) -> bool {
    dst_x >= slot_left && dst_x < slot_right && dst_y >= slot_top && dst_y < slot_bottom
}
