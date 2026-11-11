//! M8 harness — headless 3D pipeline test: a lit cube must render brighter
//! where the light points and darker (ambient-only) on the far side.

use gamerrust::camera::Camera3D;
use gamerrust::gfx::render3d::{DirectionalLight, Draw3d, Renderer3d};
use gamerrust::gfx::Graphics;
use gamerrust::mesh::MeshData;
use glam::{Mat4, Vec3};

const W: u32 = 128;
const H: u32 = 128;

fn camera() -> Camera3D {
    Camera3D {
        position: Vec3::new(0.0, 0.0, 3.0),
        looking_at: Vec3::ZERO,
        up: Vec3::Y,
        fov_y: 60.0f32.to_radians(),
        z_near: 0.1,
        z_far: 100.0,
    }
}

fn pixel(data: &[u8], x: u32, y: u32) -> [u8; 4] {
    let bpr = (W * 4).next_multiple_of(256);
    let i = (y * bpr + x * 4) as usize;
    [data[i], data[i + 1], data[i + 2], data[i + 3]]
}

#[test]
fn lit_cube_shades_by_light_direction() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let mesh = MeshData::cube().upload(&gfx);
    let mut renderer = Renderer3d::new(&gfx);

    // Light from +Z (toward the camera-facing face): front face bright.
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        ambient: [0.1, 0.1, 0.1],
    };

    gfx.clear_color([0.0, 0.0, 0.0, 1.0]);
    gfx.begin_frame();
    renderer.draw(Draw3d {
        mesh: &mesh,
        model: Mat4::IDENTITY,
        color: [1.0, 1.0, 1.0],
    });
    renderer.flush(&mut gfx, &camera(), &light);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    let center = pixel(&data, W / 2, H / 2);
    // Center = front face, fully lit: white * (ambient + diffuse) ≈ bright.
    assert!(center[0] > 180, "front face too dark: {center:?}");
    // Cube corners project off-center; a point on the cube's top edge is
    // lit by diffuse only from its angle — sample an off-center point on
    // the same face for consistency.
    let near_center = pixel(&data, W / 2 - 8, H / 2 - 8);
    assert!(
        (near_center[0] as i32 - center[0] as i32).abs() < 40,
        "same-face pixels differ too much: {near_center:?} vs {center:?}"
    );
}

#[test]
fn ambient_only_cube_is_dark() {
    let Some(mut gfx) = Graphics::headless(W, H) else {
        eprintln!("skip: no GPU adapter");
        return;
    };
    let mesh = MeshData::cube().upload(&gfx);
    let mut renderer = Renderer3d::new(&gfx);

    // No directional light at all: cube must be ambient-dark but visible.
    let light = DirectionalLight {
        direction: Vec3::new(0.0, -1.0, 0.0),
        color: [0.0, 0.0, 0.0], // dark side toward camera
        ambient: [0.2, 0.2, 0.2],
    };

    gfx.clear_color([0.0, 0.0, 0.0, 1.0]);
    gfx.begin_frame();
    renderer.draw(Draw3d {
        mesh: &mesh,
        model: Mat4::IDENTITY,
        color: [1.0, 1.0, 1.0],
    });
    renderer.flush(&mut gfx, &camera(), &light);
    gfx.end_frame();

    let data = gfx.read_pixels().expect("read pixels");
    let center = pixel(&data, W / 2, H / 2);
    // Ambient 0.2 → ~51/255 linear... but target is linear Rgba8Unorm and
    // shader writes raw floats: 0.2 * 1.0 = 51ish. Must be visible but dim.
    assert!(
        center[0] > 15 && center[0] < 120,
        "ambient-only face should be dim: {center:?}"
    );
}
