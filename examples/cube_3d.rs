//! M8 demo — 3D: a lit, spinning cube. WASD orbits, ESC exits.

use gamerrust::gfx::render3d::{DirectionalLight, Draw3d, Renderer3d};
use gamerrust::mesh::MeshData;
use gamerrust::prelude::*;
use glam::Vec3;

struct CubeDemo {
    renderer: Option<Renderer3d>,
    mesh: Option<gamerrust::mesh::Mesh>,
    angle: f32,
    distance: f32,
}

impl Game for CubeDemo {
    fn init(ctx: &mut Context) -> Self {
        let gfx = ctx.gfx();
        let renderer = gfx.map(Renderer3d::new);
        let mesh = gfx.map(|g| MeshData::cube().upload(g));
        Self {
            renderer,
            mesh,
            angle: 0.0,
            distance: 3.0,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };
        if input.pressed(Key::Escape) {
            ctx.exit();
            return;
        }
        if input.pressed(Key::A) {
            self.angle -= dt;
        }
        if input.pressed(Key::D) {
            self.angle += dt;
        }
        if input.pressed(Key::W) {
            self.distance = (self.distance - dt).max(1.5);
        }
        if input.pressed(Key::S) {
            self.distance = (self.distance + dt).min(10.0);
        }
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        let Some(gfx) = ctx.gfx() else { return };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(mesh) = self.mesh.as_ref() else {
            return;
        };

        gfx.clear_color([0.05, 0.06, 0.1, 1.0]);

        let cam = Camera3D {
            position: Vec3::new(
                self.distance * self.angle.sin(),
                1.2,
                self.distance * self.angle.cos(),
            ),
            looking_at: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: 60.0f32.to_radians(),
            z_near: 0.1,
            z_far: 100.0,
        };
        let model =
            glam::Mat4::from_rotation_y(self.angle * 0.5) * glam::Mat4::from_rotation_x(0.5);
        renderer.draw(Draw3d {
            mesh,
            model,
            color: [0.2, 0.7, 0.9],
        });
        renderer.flush(gfx, &cam, &DirectionalLight::default());
    }
}

fn main() {
    env_logger::init();
    Engine::new()
        .title("GamerRUST cube 3D")
        .size(800, 600)
        .run::<CubeDemo>()
        .expect("engine run");
}
