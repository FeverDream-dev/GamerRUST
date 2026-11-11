//! 3D mesh loading (.obj) and GPU mesh upload.

use crate::gfx::Graphics;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// CPU-side mesh: indexed triangles with per-vertex normal.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// A mesh uploaded to the GPU, ready for [`crate::gfx::render3d`].
pub struct Mesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
}

impl MeshData {
    /// Unit cube (±0.5) with flat normals — 24 verts, 36 indices.
    pub fn cube() -> Self {
        let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
            // (corners, normal)
            (
                [
                    [-0.5, -0.5, 0.5],
                    [0.5, -0.5, 0.5],
                    [0.5, 0.5, 0.5],
                    [-0.5, 0.5, 0.5],
                ],
                [0.0, 0.0, 1.0],
            ),
            (
                [
                    [0.5, -0.5, -0.5],
                    [-0.5, -0.5, -0.5],
                    [-0.5, 0.5, -0.5],
                    [0.5, 0.5, -0.5],
                ],
                [0.0, 0.0, -1.0],
            ),
            (
                [
                    [0.5, -0.5, 0.5],
                    [0.5, -0.5, -0.5],
                    [0.5, 0.5, -0.5],
                    [0.5, 0.5, 0.5],
                ],
                [1.0, 0.0, 0.0],
            ),
            (
                [
                    [-0.5, -0.5, -0.5],
                    [-0.5, -0.5, 0.5],
                    [-0.5, 0.5, 0.5],
                    [-0.5, 0.5, -0.5],
                ],
                [-1.0, 0.0, 0.0],
            ),
            (
                [
                    [-0.5, 0.5, 0.5],
                    [0.5, 0.5, 0.5],
                    [0.5, 0.5, -0.5],
                    [-0.5, 0.5, -0.5],
                ],
                [0.0, 1.0, 0.0],
            ),
            (
                [
                    [-0.5, -0.5, -0.5],
                    [0.5, -0.5, -0.5],
                    [0.5, -0.5, 0.5],
                    [-0.5, -0.5, 0.5],
                ],
                [0.0, -1.0, 0.0],
            ),
        ];
        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (i, (corners, normal)) in faces.iter().enumerate() {
            let base = (i * 4) as u32;
            for c in corners {
                positions.push(*c);
                normals.push(*normal);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self {
            positions,
            normals,
            indices,
        }
    }

    /// Load triangle meshes from .obj text. Later objects are merged into
    /// one mesh (first-come normals; missing normals default to +Y).
    ///
    /// ponytail: no mtl/material support; real material pipelines when a
    /// game needs them.
    pub fn from_obj(text: &str) -> Result<Self, String> {
        let mut cursor = std::io::Cursor::new(text.as_bytes());
        let (models, _) = tobj::load_obj_buf(
            &mut cursor,
            &tobj::LoadOptions {
                triangulate: true,
                ..Default::default()
            },
            |_| Ok(Default::default()),
        )
        .map_err(|e| format!("obj parse: {e}"))?;

        let mut data = Self::default();
        for model in &models {
            let mesh = &model.mesh;
            for i in 0..mesh.positions.len() / 3 {
                let p = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                let n = if !mesh.normals.is_empty() {
                    [
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                };
                data.positions.push(p);
                data.normals.push(n);
            }
            data.indices.extend(mesh.indices.iter().copied());
        }
        Ok(data)
    }

    /// Upload to the GPU.
    pub fn upload(&self, gfx: &Graphics) -> Mesh {
        use wgpu::util::DeviceExt;
        let verts: Vec<MeshVertex> = self
            .positions
            .iter()
            .zip(&self.normals)
            .map(|(p, n)| MeshVertex {
                position: *p,
                normal: *n,
            })
            .collect();
        let vertex_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gamerrust.mesh.v"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gamerrust.mesh.i"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Mesh {
            vertex_buffer,
            index_buffer,
            index_count: self.indices.len() as u32,
        }
    }

    /// Recompute smooth vertex normals from triangle geometry (for meshes
    /// that shipped without usable normals).
    pub fn with_recomputed_normals(mut self) -> Self {
        let mut acc = vec![[0.0f32; 3]; self.positions.len()];
        for tri in self.indices.chunks_exact(3) {
            let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let pa = Vec3::from_array(self.positions[a]);
            let pb = Vec3::from_array(self.positions[b]);
            let pc = Vec3::from_array(self.positions[c]);
            let n = (pb - pa).cross(pc - pa);
            for &v in &[a, b, c] {
                let cur = Vec3::from_array(acc[v]);
                acc[v] = (cur + n).to_array();
            }
        }
        self.normals = acc
            .iter()
            .map(|n| Vec3::from_array(*n).normalize_or_zero().to_array())
            .collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_counts() {
        let c = MeshData::cube();
        assert_eq!(c.positions.len(), 24);
        assert_eq!(c.indices.len(), 36);
        // All normals unit length.
        for n in &c.normals {
            let l = Vec3::from_array(*n).length();
            assert!((l - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn obj_round_trip() {
        let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let m = MeshData::from_obj(obj).expect("parse");
        assert_eq!(m.positions.len(), 3);
        assert_eq!(m.indices, vec![0, 1, 2]);
    }

    #[test]
    fn recompute_normals_points_out() {
        // CCW triangle in XY plane → normal +Z.
        let m = MeshData {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0; 3]; 3],
            indices: vec![0, 1, 2],
        }
        .with_recomputed_normals();
        assert!((m.normals[0][2] - 1.0).abs() < 1e-6, "{:?}", m.normals[0]);
    }
}
