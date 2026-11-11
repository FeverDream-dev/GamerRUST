//! 2D collision primitives: AABB + circle overlap, resolution, and swept tests.
//!
//! Convention used throughout: **touching is not overlapping** — shapes whose
//! boundaries exactly meet report `false` from `overlap_*` (edges are shared,
//! so two tiles side by side never "collide"). Point containment is inclusive.

use glam::Vec2;

/// Axis-aligned bounding box defined by its min (bottom-left) and max
/// (top-right) corners.
///
/// // ponytail: axis-aligned only, no rotation support — add OBB when a game needs it
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

/// Circle defined by center and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Aabb {
    /// Strict overlap test; exact edge contact returns `false`.
    pub fn overlap_aabb(&self, other: &Aabb) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    /// Overlap against a circle (closest-point-on-box method).
    pub fn overlap_circle(&self, circle: &Circle) -> bool {
        let closest = circle.center.clamp(self.min, self.max);
        circle.center.distance_squared(closest) < circle.radius * circle.radius
    }

    /// Inclusive point test (points on the boundary count as inside).
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Minimum translation vector pushing `self` out of `other`, or `None`
    /// when not overlapping. The push is the smallest separation, on one axis;
    /// `velocity` only breaks ties (equal-cost faces resolve against travel).
    // ponytail: single-axis MTV only — corner hits snap to one axis; combined
    // diagonal push if a game ever visibly pops on corners
    pub fn resolve_aabb(&self, other: &Aabb, velocity: Vec2) -> Option<Vec2> {
        if !self.overlap_aabb(other) {
            return None;
        }
        // Cost to separate by pushing `self` out through each face of `other`.
        let push_right = self.max.x - other.min.x;
        let push_left = other.max.x - self.min.x;
        let push_up = self.max.y - other.min.y;
        let push_down = other.max.y - self.min.y;
        let (x_push, x_cost) =
            if push_left < push_right || (push_left == push_right && velocity.x > 0.0) {
                (-push_left, push_left)
            } else {
                (push_right, push_right)
            };
        let (y_push, y_cost) = if push_down < push_up || (push_down == push_up && velocity.y > 0.0)
        {
            (-push_down, push_down)
        } else {
            (push_up, push_up)
        };
        if x_cost <= y_cost {
            Some(Vec2::new(x_push, 0.0))
        } else {
            Some(Vec2::new(0.0, y_push))
        }
    }
}

impl Circle {
    /// Strict overlap test; exact edge contact returns `false`.
    pub fn overlap_circle(&self, other: &Circle) -> bool {
        self.center.distance_squared(other.center)
            < (self.radius + other.radius) * (self.radius + other.radius)
    }

    /// Inclusive point test (points on the boundary count as inside).
    pub fn contains_point(&self, point: Vec2) -> bool {
        self.center.distance_squared(point) <= self.radius * self.radius
    }
}

/// Swept AABB vs static AABB: moves the box from `start` to `end` and returns
/// `(time_of_impact, normal)` with time in `[0, 1]` and the unit axis normal
/// opposing the motion on the axis first entered. Handles tunneling — a fast
/// box that would skip a thin wall in a discrete test is still caught.
///
/// Returns `None` when the sweep misses. If already overlapping and not
/// moving, returns `(0.0, Vec2::ZERO)` — use [`Aabb::resolve_aabb`] for a
/// push-out direction in that case.
// ponytail: pure translation assumed (start/end must be the same size);
// rotating or resizing mid-sweep is unsupported
pub fn swept_aabb(start: Aabb, end: Aabb, obstacle: &Aabb) -> Option<(f32, Vec2)> {
    let d = end.min - start.min;
    let mut t_enter = 0.0f32;
    let mut t_exit = 1.0f32;
    let mut normal = Vec2::ZERO;
    let mut gating_enter = f32::NEG_INFINITY;

    let axes = [
        (
            start.min.x,
            start.max.x,
            obstacle.min.x,
            obstacle.max.x,
            d.x,
            Vec2::X,
            Vec2::NEG_X,
        ),
        (
            start.min.y,
            start.max.y,
            obstacle.min.y,
            obstacle.max.y,
            d.y,
            Vec2::Y,
            Vec2::NEG_Y,
        ),
    ];
    for (s_lo, s_hi, o_lo, o_hi, dv, n_pos, n_neg) in axes {
        if dv == 0.0 {
            // No motion on this axis: it must already overlap, else never hit.
            if s_hi <= o_lo || s_lo >= o_hi {
                return None;
            }
            continue;
        }
        // Slab method: overlap window on this axis is [t1, t2] (unordered).
        let t1 = (o_lo - s_hi) / dv;
        let t2 = (o_hi - s_lo) / dv;
        let (axis_enter, axis_exit) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        // The axis that gates entry last is the face we hit → normal source.
        if axis_enter > gating_enter {
            gating_enter = axis_enter;
            normal = if dv > 0.0 { n_neg } else { n_pos };
        }
        t_enter = t_enter.max(axis_enter);
        t_exit = t_exit.min(axis_exit);
        if t_enter > t_exit {
            return None;
        }
    }
    // Started overlapped: first contact is "now" (normal may be ZERO when stationary).
    Some((t_enter.max(0.0), normal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
        Aabb {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }

    #[test]
    fn aabb_overlap_positives_and_negatives() {
        assert!(aabb(0.0, 0.0, 2.0, 2.0).overlap_aabb(&aabb(1.0, 1.0, 3.0, 3.0)));
        assert!(aabb(0.0, 0.0, 2.0, 2.0).overlap_aabb(&aabb(-1.0, -1.0, 0.5, 0.5)));
        assert!(!aabb(0.0, 0.0, 1.0, 1.0).overlap_aabb(&aabb(2.0, 2.0, 3.0, 3.0)));
        assert!(!aabb(0.0, 0.0, 1.0, 1.0).overlap_aabb(&aabb(0.0, 5.0, 1.0, 6.0)));
    }

    #[test]
    fn aabb_exact_touch_is_not_overlap() {
        // Share the x edge, touch only there.
        assert!(!aabb(0.0, 0.0, 1.0, 1.0).overlap_aabb(&aabb(1.0, 0.0, 2.0, 1.0)));
        // Share the y edge.
        assert!(!aabb(0.0, 0.0, 1.0, 1.0).overlap_aabb(&aabb(0.0, 1.0, 1.0, 2.0)));
        // Corner touch only.
        assert!(!aabb(0.0, 0.0, 1.0, 1.0).overlap_aabb(&aabb(1.0, 1.0, 2.0, 2.0)));
    }

    #[test]
    fn aabb_circle_overlap_positives_negatives_touch() {
        let class = aabb(0.0, 0.0, 4.0, 4.0);
        assert!(class.overlap_circle(&Circle {
            center: Vec2::new(5.0, 2.0),
            radius: 1.5,
        }));
        assert!(class.overlap_circle(&Circle {
            center: Vec2::new(2.0, 2.0),
            radius: 0.1,
        })); // fully inside
        assert!(!class.overlap_circle(&Circle {
            center: Vec2::new(5.0, 2.0),
            radius: 0.5,
        }));
        // Exact touch: closest point (4,2) is exactly radius away.
        assert!(!class.overlap_circle(&Circle {
            center: Vec2::new(5.0, 2.0),
            radius: 1.0,
        }));
    }

    #[test]
    fn circle_overlap_positives_negatives_touch() {
        let a = Circle {
            center: Vec2::new(0.0, 0.0),
            radius: 1.6,
        };
        assert!(a.overlap_circle(&Circle {
            center: Vec2::new(3.0, 0.0),
            radius: 1.6,
        })); // 3.0 < 3.2
        assert!(!a.overlap_circle(&Circle {
            center: Vec2::new(3.0, 0.0),
            radius: 1.4,
        })); // exact touch: 1.6 + 1.4 == 3.0 == distance
        assert!(!a.overlap_circle(&Circle {
            center: Vec2::new(10.0, 0.0),
            radius: 1.6,
        }));
    }

    #[test]
    fn resolve_pushes_out_along_minimum_axis() {
        // Overlaps 2 in +x, 8..10 on y: minimum translation is +x by 2.
        let a = aabb(0.0, 0.0, 10.0, 10.0);
        let b = aabb(8.0, 0.0, 18.0, 10.0);
        assert_eq!(
            a.resolve_aabb(&b, Vec2::new(5.0, 0.0)),
            Some(Vec2::new(2.0, 0.0))
        );
    }

    #[test]
    fn resolve_tie_prefers_against_travel() {
        // Perfectly centered: all four pushes cost 4; moving +x → pushed -x.
        let inner = aabb(-1.0, -1.0, 1.0, 1.0);
        let outer = aabb(-3.0, -3.0, 3.0, 3.0);
        assert_eq!(
            inner.resolve_aabb(&outer, Vec2::new(1.0, 0.0)),
            Some(Vec2::new(-4.0, 0.0))
        );
    }

    #[test]
    fn resolve_none_when_separate() {
        let a = aabb(0.0, 0.0, 1.0, 1.0);
        assert_eq!(a.resolve_aabb(&aabb(2.0, 0.0, 3.0, 1.0), Vec2::ONE), None);
    }

    #[test]
    fn swept_catches_tunneling() {
        // Thin, tall wall; a discrete check at the end position sees no hit.
        let wall = aabb(4.9, -100.0, 5.1, 100.0);
        let start = aabb(0.0, 0.0, 1.0, 1.0);
        let end = aabb(10.0, 0.0, 11.0, 1.0);
        assert!(!end.overlap_aabb(&wall)); // discrete check would tunnel
        let (t, normal) = swept_aabb(start, end, &wall).expect("sweep must catch the tunnel");
        assert!((t - 0.39).abs() < 1e-6, "toi {t}");
        assert_eq!(normal, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn swept_time_of_impact_is_exact() {
        // Box travels right 4 units; wall face at x=3 → contact at t = 0.5.
        let start = aabb(0.0, 0.0, 1.0, 1.0);
        let end = aabb(4.0, 0.0, 5.0, 1.0);
        let obstacle = aabb(3.0, -1.0, 4.0, 3.0);
        let (t, normal) = swept_aabb(start, end, &obstacle).expect("should hit");
        assert_eq!(t, 0.5);
        assert_eq!(normal, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn swept_miss_when_passing_above() {
        let start = aabb(0.0, 0.0, 1.0, 1.0);
        let end = aabb(4.0, 0.0, 5.0, 1.0);
        // Wall's y range [2,3] never intersects the box's [0,1]: miss.
        assert_eq!(swept_aabb(start, end, &aabb(3.0, 2.0, 4.0, 3.0)), None);
    }

    #[test]
    fn swept_none_when_stops_short() {
        let start = aabb(0.0, 0.0, 1.0, 1.0);
        let end = aabb(1.0, 0.0, 2.0, 1.0); // moves 1 unit, wall face at x=3
        assert_eq!(swept_aabb(start, end, &aabb(3.0, -1.0, 4.0, 3.0)), None);
    }

    #[test]
    fn aabb_contains_point_boundaries() {
        let a = aabb(0.0, 0.0, 2.0, 2.0);
        assert!(a.contains_point(Vec2::new(1.0, 1.0)));
        assert!(a.contains_point(Vec2::new(0.0, 0.0))); // corner inclusive
        assert!(a.contains_point(Vec2::new(2.0, 2.0))); // opposite corner
        assert!(!a.contains_point(Vec2::new(2.0001, 1.0)));
        assert!(!a.contains_point(Vec2::new(-0.0001, 1.0)));
    }

    #[test]
    fn circle_contains_point_boundaries() {
        let c = Circle {
            center: Vec2::ZERO,
            radius: 2.0,
        };
        assert!(c.contains_point(Vec2::ZERO));
        assert!(c.contains_point(Vec2::new(2.0, 0.0))); // on the rim
        assert!(c.contains_point(Vec2::new(1.2, 1.6))); // 1.44+2.56 = 4.0
        assert!(!c.contains_point(Vec2::new(2.0001, 0.0)));
        assert!(!c.contains_point(Vec2::new(1.5, 1.5))); // 4.5 > 4.0
    }
}
