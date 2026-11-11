//! 3D pipeline: meshes + directional light + ambient, depth-tested.
//!
//! ponytail: single hardcoded directional light + ambient, Blinn-Phong-ish;
//! PBR/material system when a game needs it.

use crate::camera::Camera3D;
use crate::gfx::Graphics;
use crate::mesh::{Mesh, MeshVertex};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [f32; 16],
    light_dir: [f32; 4], // xyz + unused
    light_color: [f32; 4],
    ambient: [f32; 4],
}

/// One queued draw: mesh + model transform + base color.
#[derive(Clone)]
pub struct Draw3d<'a> {
    pub mesh: &'a Mesh,
    /// Model-to-world transform.
    pub model: Mat4,
    /// sRGB-ish base color.
    pub color: [f32; 3],
}

/// The 3D renderer: create once, queue draws, flush once per frame.
pub struct Renderer3d {
    pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
    depth_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    depth_format: wgpu::TextureFormat,
    queued: Vec<QueuedDraw>,
}

struct QueuedDraw {
    mesh_vertex: wgpu::Buffer,
    mesh_index: wgpu::Buffer,
    index_count: u32,
    model: Mat4,
    color: [f32; 3],
}

const SHADER: &str = "
struct Globals {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct Model {
    matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0) var<uniform> model: Model;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    let world = model.matrix * vec4<f32>(input.position, 1.0);
    var out: VertexOut;
    out.position = globals.view_proj * world;
    out.world_normal = (model.normal_matrix * vec4<f32>(input.normal, 0.0)).xyz;
    out.world_pos = world.xyz;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(input.world_normal);
    let l = normalize(-globals.light_dir.xyz);
    let diffuse = max(dot(n, l), 0.0);
    // View from +Z-ish: cheap half-vector specular.
    let v = vec3<f32>(0.0, 0.0, 1.0);
    let h = normalize(l + v);
    let spec = pow(max(dot(n, h), 0.0), 32.0) * 0.25;
    let lit = globals.ambient.rgb * model.color.rgb
        + globals.light_color.rgb * model.color.rgb * diffuse
        + vec3<f32>(spec);
    return vec4<f32>(lit, 1.0);
}
";

/// Directional-light setup for [`Renderer3d`].
#[derive(Clone, Copy, Debug)]
pub struct DirectionalLight {
    /// Direction the light travels (pointing away from the source).
    pub direction: Vec3,
    /// RGB intensity.
    pub color: [f32; 3],
    /// RGB ambient floor.
    pub ambient: [f32; 3],
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.4),
            color: [1.0, 0.95, 0.9],
            ambient: [0.15, 0.15, 0.2],
        }
    }
}

impl Renderer3d {
    pub fn new(gfx: &Graphics) -> Self {
        let device = &gfx.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gamerrust.mesh3d"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gamerrust.globals_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gamerrust.model_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gamerrust.globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gamerrust.globals_bg"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        // ponytail: one model uniform, rewritten per draw; instancing when
        // draw-call count matters. Size = mat4 + mat4 + vec4 padded to
        // 192 bytes (WGSL uniform layout, two 64B rows + one 16B row).
        let model_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gamerrust.model"),
            size: 192,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gamerrust.model_bg"),
            layout: &model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gamerrust.mesh3d_layout"),
            bind_group_layouts: &[Some(&globals_layout), Some(&model_layout)],
            immediate_size: 0,
        });

        let depth_format = wgpu::TextureFormat::Depth32Float;
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gamerrust.mesh3d_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gfx.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            globals_buffer,
            globals_bind_group,
            model_buffer,
            model_bind_group,
            depth_texture: None,
            depth_format,
            queued: Vec::new(),
        }
    }

    /// Queue a mesh draw for this frame.
    pub fn draw(&mut self, d: Draw3d<'_>) {
        self.queued.push(QueuedDraw {
            mesh_vertex: d.mesh.vertex_buffer.clone(),
            mesh_index: d.mesh.index_buffer.clone(),
            index_count: d.mesh.index_count,
            model: d.model,
            color: d.color,
        });
    }

    /// Ensure the depth target exists and matches the frame size.
    fn ensure_depth(&mut self, gfx: &Graphics, w: u32, h: u32) -> wgpu::TextureView {
        let recreate = self
            .depth_texture
            .as_ref()
            .map(|(t, _)| t.width() != w || t.height() != h)
            .unwrap_or(true);
        if recreate {
            let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gamerrust.depth"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.depth_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_texture = Some((texture, view));
        }
        self.depth_texture.as_ref().unwrap().1.clone()
    }

    /// Upload uniforms and record the 3D pass. Between begin/end frame,
    /// after sprites/text (3D owns the depth buffer).
    pub fn flush(&mut self, gfx: &mut Graphics, camera: &Camera3D, light: &DirectionalLight) {
        if self.queued.is_empty() {
            return;
        }
        let (w, h) = gfx.size();
        let depth = self.ensure_depth(gfx, w.max(1), h.max(1));
        let aspect = w as f32 / h.max(1) as f32;

        let globals = Globals {
            view_proj: camera.view_projection(aspect).to_cols_array(),
            light_dir: [light.direction.x, light.direction.y, light.direction.z, 0.0],
            light_color: [light.color[0], light.color[1], light.color[2], 0.0],
            ambient: [light.ambient[0], light.ambient[1], light.ambient[2], 0.0],
        };
        gfx.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        let draws = std::mem::take(&mut self.queued);
        // ponytail: one pass per mesh (model uniform rewritten between
        // passes); merge into instanced draws when draw-call count matters.
        for d in &draws {
            let depth = depth.clone();
            let model_arr = [
                d.model.to_cols_array(),
                d.model.inverse().transpose().to_cols_array(),
                [
                    d.color[0], d.color[1], d.color[2], 1.0, // color @ byte 128
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                ],
            ];
            let bytes: Vec<u8> = model_arr
                .iter()
                .flat_map(|m| m.iter().flat_map(|f| f.to_le_bytes()))
                .collect();
            gfx.queue.write_buffer(&self.model_buffer, 0, &bytes);

            let pipeline = self.pipeline.clone();
            let globals_bg = self.globals_bind_group.clone();
            let model_bg = self.model_bind_group.clone();
            let vertex = d.mesh_vertex.clone();
            let index = d.mesh_index.clone();
            let count = d.index_count;
            gfx.with_frame(move |encoder, view| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("gamerrust.mesh3d"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &globals_bg, &[]);
                pass.set_bind_group(1, &model_bg, &[]);
                pass.set_vertex_buffer(0, vertex.slice(..));
                pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..count, 0, 0..1);
            });
        }
    }
}
