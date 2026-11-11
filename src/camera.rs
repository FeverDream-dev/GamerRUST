//! 2D and 3D cameras.
//!
//! [`Camera2D`] places the world on screen with a center origin, +y down,
//! `zoom` = world units per screen pixel (bigger zoom out = more world per
//! pixel). [`Camera3D`] is a standard look-at perspective camera.

use glam::{Mat4, Vec2, Vec3};

/// Orthographic 2D camera: center of screen = `position`, +y down.
#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    /// Camera center in world units.
    pub position: Vec2,
    /// World units per screen pixel (1.0 = 1:1). >1 zooms out.
    pub zoom: f32,
    /// Rotation in radians (screen rotates around the camera center).
    pub rotation: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

impl Camera2D {
    /// Ortho matrix mapping world to clip space for a `width` x `height`
    /// pixel target. Screen center lands at `position`, +y down.
    pub fn ortho_matrix(&self, width: f32, height: f32) -> Mat4 {
        let half_w = width * self.zoom * 0.5;
        let half_h = height * self.zoom * 0.5;
        // Center-origin ortho, then translate by camera position, then
        // rotate around the camera center.
        let proj = Mat4::orthographic_rh(-half_w, half_w, half_h, -half_h, -1.0, 1.0);
        let rot = Mat4::from_rotation_z(self.rotation);
        let trans = Mat4::from_translation((-self.position).extend(0.0));
        proj * rot * trans
    }

    /// Convert a screen pixel coordinate to world space.
    pub fn screen_to_world(&self, screen: Vec2, screen_size: Vec2) -> Vec2 {
        // Pixel → centered pixel → undo rotation → scale by zoom → offset.
        let centered = screen - screen_size * 0.5;
        let rotated = Mat4::from_rotation_z(-self.rotation) * centered.extend(0.0).extend(1.0);
        let unrotated = Vec2::new(rotated.x, rotated.y);
        unrotated * self.zoom + self.position
    }
}

/// Perspective 3D camera (right-handed, look-at).
#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub position: Vec3,
    pub looking_at: Vec3,
    pub up: Vec3,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera3D {
    /// Combined view-projection for the given aspect ratio (w/h).
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.z_near, self.z_far);
        let view = Mat4::look_at_rh(self.position, self.looking_at, self.up);
        proj * view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec4;

    /// World point at the screen center maps to NDC (0, 0) — identity cam.
    #[test]
    fn ortho_center_maps_to_ndc_center() {
        let cam = Camera2D::default();
        let m = cam.ortho_matrix(800.0, 600.0);
        let c = m * Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!((c.x - 0.0).abs() < 1e-5 && (c.y - 0.0).abs() < 1e-5);
    }

    /// The world x = +half-width edge of an 800px viewport is NDC x = +1.
    #[test]
    fn ortho_right_edge_maps_to_ndc_one() {
        let cam = Camera2D::default();
        let m = cam.ortho_matrix(800.0, 600.0);
        let c = m * Vec4::new(400.0, 0.0, 0.0, 1.0);
        assert!((c.x - 1.0).abs() < 1e-5, "got NDC x = {}", c.x);
    }

    /// +y in world (down on screen) maps to NDC -y... actually with our
    /// convention world +y is down and NDC +y is up, so world +y → NDC -y.
    #[test]
    fn ortho_down_maps_correctly() {
        let cam = Camera2D::default();
        let m = cam.ortho_matrix(800.0, 600.0);
        let c = m * Vec4::new(0.0, 300.0, 0.0, 1.0);
        assert!((c.y - (-1.0)).abs() < 1e-5, "got NDC y = {}", c.y);
    }

    /// Zoom=2 shows twice the world: the x=800 point is only NDC +1 now.
    #[test]
    fn zoom_doubles_world_extents() {
        let cam = Camera2D {
            zoom: 2.0,
            ..Default::default()
        };
        let m = cam.ortho_matrix(800.0, 600.0);
        let c = m * Vec4::new(800.0, 0.0, 0.0, 1.0);
        assert!((c.x - 1.0).abs() < 1e-5, "got NDC x = {}", c.x);
    }

    /// screen_to_world inverts the ortho matrix at the same screen size.
    #[test]
    fn screen_to_world_round_trip() {
        let cam = Camera2D {
            position: Vec2::new(37.0, -11.0),
            zoom: 1.7,
            rotation: 0.6,
        };
        let size = Vec2::new(1280.0, 720.0);
        // Project a world point to screen via the matrix, then back.
        let m = cam.ortho_matrix(size.x, size.y);
        let world = Vec2::new(123.0, 45.0);
        let clip = m * world.extend(0.0).extend(1.0);
        let ndc = Vec2::new(clip.x / clip.w, clip.y / clip.w);
        let screen = (ndc + Vec2::ONE) * 0.5 * size;
        // y flips: screen y grows down, NDC y grows up.
        let screen = Vec2::new(screen.x, size.y - screen.y);
        let back = cam.screen_to_world(screen, size);
        assert!(
            (back - world).length() < 1e-3,
            "round trip drifted: {back:?} vs {world:?}"
        );
    }

    /// A point straight ahead of a Camera3D projects to NDC (0, 0).
    #[test]
    fn perspective_center_point_at_ndc_center() {
        let cam = Camera3D {
            position: Vec3::new(0.0, 0.0, 1.0),
            looking_at: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: std::f32::consts::FRAC_PI_2,
            z_near: 0.1,
            z_far: 100.0,
        };
        let vp = cam.view_projection(1.0);
        let c = vp * Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!(c.x.abs() < 1e-5 && c.y.abs() < 1e-5);
    }

    /// A point up-right of the view axis projects up-right in NDC.
    #[test]
    fn perspective_offaxis_direction() {
        let cam = Camera3D {
            position: Vec3::new(0.0, 0.0, 1.0),
            looking_at: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: std::f32::consts::FRAC_PI_2,
            z_near: 0.1,
            z_far: 100.0,
        };
        let vp = cam.view_projection(1.0);
        // 90° fov, 1:1 aspect, point at z=0 plane x=+0.5 y=+0.5 (right/up of
        // axis, distance 1 in front): tan(45°)=1 so it lands at NDC (.5, .5).
        let c = vp * Vec4::new(0.5, 0.5, 0.0, 1.0);
        assert!(
            (c.x - 0.5).abs() < 1e-4 && (c.y - 0.5).abs() < 1e-4,
            "got {c:?}"
        );
    }
}
