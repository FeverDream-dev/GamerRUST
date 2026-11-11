//! 2D sprite batching: instanced quads through one pipeline.
//!
//! [`SpriteBatch`] collects `(position, size, color, uv-rect)` instances,
//! then flushes them all in a single draw call per texture. World space is
//! pixels with a screen-center origin, +y down — matching [`crate::camera::Camera2D`].

use super::{Graphics, Texture};
use crate::camera::Camera2D;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};
use wgpu::util::DeviceExt;

/// Pixel sub-rect of a texture.
#[derive(Clone, Copy, Debug)]
pub struct SourceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// One queued sprite.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    /// Center in world units.
    pub position: Vec2,
    pub size: Vec2,
    /// RGBA tint (multiplied with the texture); white = untinted.
    pub color: [f32; 4],
    /// Sub-rect of the texture in pixels (default: whole texture).
    pub source_rect: Option<SourceRect>,
}

impl Sprite {
    /// Untinted full-texture sprite centered at `position`.
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
            color: [1.0, 1.0, 1.0, 1.0],
            source_rect: None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    /// world position.xy + size.zw.
    pos_size: [f32; 4],
    color: [f32; 4],
    /// UV rect: (u0, v0, u1, v1).
    uv: [f32; 4],
}

impl Instance {
    fn from_sprite(s: &Sprite, tex: &Texture) -> Self {
        let uv = match s.source_rect {
            None => [0.0, 0.0, 1.0, 1.0],
            Some(r) => [
                r.x as f32 / tex.width as f32,
                r.y as f32 / tex.height as f32,
                (r.x + r.width) as f32 / tex.width as f32,
                (r.y + r.height) as f32 / tex.height as f32,
            ],
        };
        Self {
            pos_size: [s.position.x, s.position.y, s.size.x, s.size.y],
            color: s.color,
            uv,
        }
    }
}

/// Quad corner vertex (triangle strip).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Corner {
    xy: [f32; 2],
}

const CORNERS: [Corner; 4] = [
    Corner { xy: [0.0, 0.0] },
    Corner { xy: [1.0, 0.0] },
    Corner { xy: [0.0, 1.0] },
    Corner { xy: [1.0, 1.0] },
];

const SHADER: &str = "
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertexIn {
    @location(0) corner: vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    let local = (input.corner - vec2<f32>(0.5, 0.5)) * input.pos_size.zw;
    let world = input.pos_size.xy + local;
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(world, 0.0, 1.0);
    out.color = input.color;
    out.uv = mix(input.uv.xy, input.uv.zw, input.corner);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv) * in.color;
}
";

/// Batches sprites sharing one [`Texture`] into one draw call.
///
/// ponytail: one texture per batch per frame; multi-atlas sorting and
/// texture arrays when a game measurably hits the flush ceiling.
pub struct SpriteBatch {
    texture: Texture,
    instances: Vec<Instance>,
    pipeline: wgpu::RenderPipeline,
    tex_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    corner_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    capacity: usize,
    proj: Mat4,
}

const INITIAL_CAPACITY: usize = 1024;

impl SpriteBatch {
    /// Create a batcher bound to `texture`. Camera defaults to a 1:1
    /// pixel ortho matrix; call [`SpriteBatch::set_camera`] to change it.
    pub fn new(gfx: &Graphics, texture: Texture) -> Self {
        let device = &gfx.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gamerrust.sprite"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gamerrust.camera_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });

        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gamerrust.tex_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gamerrust.camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gamerrust.camera_bg"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let tex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gamerrust.tex_bg"),
            layout: &tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gamerrust.sprite_layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&tex_layout)],
            immediate_size: 0,
        });

        let vertex_buffers = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Corner>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 16,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 32,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                ],
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gamerrust.sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(vertex_buffers[0].clone()),
                    Some(vertex_buffers[1].clone()),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gfx.format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let corner_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gamerrust.corners"),
            contents: bytemuck::cast_slice(&CORNERS),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gamerrust.instances"),
            size: (INITIAL_CAPACITY * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            texture,
            instances: Vec::with_capacity(INITIAL_CAPACITY),
            pipeline,
            tex_bind_group,
            camera_bind_group,
            camera_buffer,
            corner_buffer,
            instance_buffer,
            capacity: INITIAL_CAPACITY,
            proj: Mat4::IDENTITY,
        }
    }

    /// Queue a sprite for the current frame.
    pub fn push(&mut self, sprite: Sprite) {
        self.instances
            .push(Instance::from_sprite(&sprite, &self.texture));
    }

    /// Convenience: full-texture sprite centered at `position`.
    pub fn draw(&mut self, position: Vec2, size: Vec2) {
        self.push(Sprite::new(position, size));
    }

    /// Set the camera for subsequent flushes until changed.
    pub fn set_camera(&mut self, camera: &Camera2D, width: f32, height: f32) {
        self.proj = camera.ortho_matrix(width, height);
    }

    /// Upload instances and record the draw into the in-flight frame.
    /// Call between `gfx.begin_frame()` and `gfx.end_frame()`.
    pub fn flush(&mut self, gfx: &mut Graphics) {
        if self.instances.is_empty() {
            return;
        }
        if self.instances.len() > self.capacity {
            self.capacity = self.instances.len().next_power_of_two();
            self.instance_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gamerrust.instances"),
                size: (self.capacity * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        gfx.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
        gfx.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.proj.to_cols_array()),
        );

        let n = self.instances.len() as u32;
        self.instances.clear();

        let pipeline = self.pipeline.clone();
        let tex_bg = self.tex_bind_group.clone();
        let camera_bg = self.camera_bind_group.clone();
        let corners = self.corner_buffer.clone();
        let instances = self.instance_buffer.clone();
        gfx.with_frame(move |encoder, view| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gamerrust.sprites"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &camera_bg, &[]);
            pass.set_bind_group(1, &tex_bg, &[]);
            pass.set_vertex_buffer(0, corners.slice(..));
            pass.set_vertex_buffer(1, instances.slice(..));
            pass.draw(0..4, 0..n);
        });
    }
}
