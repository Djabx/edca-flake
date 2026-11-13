//! The galaxy map's orbit camera, in galactic coordinates (light years, Sol at the origin,
//! +y out of the galactic plane, +z toward Sagittarius A*).
//!
//! The camera circles a target point. With no yaw and the pitch at 90° it looks straight
//! down with +x to the right and the core (+z) at the top, as the in-game galaxy map does.
//! The same projection feeds the GPU ([`Camera::view_proj`]) and CPU-side picking and
//! labels ([`Camera::project`]), so a click lands on the star that was drawn there.

use eframe::egui::{pos2, Pos2, Rect};

pub type V3 = [f32; 3];

/// Vertical field of view.
const FOV_Y: f32 = 50.0 * std::f32::consts::PI / 180.0;
/// Closest the camera gets to its target, in ly.
pub const MIN_DISTANCE: f32 = 2.0;
/// Farthest: the whole charted galaxy (~100 kly across) fits the view.
pub const MAX_DISTANCE: f32 = 150_000.0;
/// Pitch stays short of straight up/down, where yaw stops meaning anything.
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn dist(a: V3, b: V3) -> f32 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

pub fn lerp(a: V3, b: V3, t: f32) -> V3 {
    add(a, scale(sub(b, a), t))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The point the camera orbits and zooms toward.
    pub target: V3,
    /// Distance from the target, in ly.
    pub distance: f32,
    /// Rotation about the galactic y axis, radians.
    pub yaw: f32,
    /// Elevation above the galactic plane, radians.
    pub pitch: f32,
}

impl Camera {
    /// Right, up and forward unit vectors of the view.
    fn basis(&self) -> (V3, V3, V3) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = [-cp * sy, -sp, cp * cy];
        let right = [cy, 0.0, sy];
        // forward × right
        let up = [
            forward[1] * right[2] - forward[2] * right[1],
            forward[2] * right[0] - forward[0] * right[2],
            forward[0] * right[1] - forward[1] * right[0],
        ];
        (right, up, forward)
    }

    pub fn eye(&self) -> V3 {
        let (_, _, f) = self.basis();
        sub(self.target, scale(f, self.distance))
    }

    /// Clipping distance in front of the eye; scales with the zoom so the depth range
    /// stays sane from a few ly to the whole galaxy.
    fn near(&self) -> f32 {
        self.distance * 0.01
    }

    /// Screen pixels per ly at unit depth, for a view `height` pixels tall.
    pub fn px_per_ly(&self, height: f32) -> f32 {
        0.5 * height / (0.5 * FOV_Y).tan()
    }

    /// World → clip-space matrix (column-major, as OpenGL takes it) for a viewport with
    /// this width / height ratio. Clip z is only used for the near plane: there is no
    /// depth buffer, the stars are blended additively.
    pub fn view_proj(&self, aspect: f32) -> [f32; 16] {
        let (r, u, f) = self.basis();
        let eye = self.eye();
        let t = (0.5 * FOV_Y).tan();
        let rows = [
            [r[0] / (aspect * t), r[1] / (aspect * t), r[2] / (aspect * t), -dot(r, eye) / (aspect * t)],
            [u[0] / t, u[1] / t, u[2] / t, -dot(u, eye) / t],
            [f[0], f[1], f[2], -dot(f, eye) - self.near()],
            [f[0], f[1], f[2], -dot(f, eye)],
        ];
        let mut m = [0.0; 16];
        for (row, vals) in rows.iter().enumerate() {
            for (col, v) in vals.iter().enumerate() {
                m[col * 4 + row] = *v;
            }
        }
        m
    }

    /// Projects a point into `rect`; `None` when it is behind the near plane.
    pub fn project(&self, p: V3, rect: Rect) -> Option<Pos2> {
        self.projector(rect).project(p)
    }

    /// The projection into `rect` with its trigonometry done once, for projecting many points.
    pub fn projector(&self, rect: Rect) -> Projector {
        let (right, up, forward) = self.basis();
        Projector { right, up, forward, eye: self.eye(), near: self.near(), k: self.px_per_ly(rect.height()), center: rect.center() }
    }

    /// Projects the segment `a`–`b`, cut at the near plane. `None` when it is wholly behind.
    pub fn project_segment(&self, a: V3, b: V3, rect: Rect) -> Option<[Pos2; 2]> {
        let (_, _, f) = self.basis();
        let eye = self.eye();
        let near = self.near();
        let (da, db) = (dot(sub(a, eye), f), dot(sub(b, eye), f));
        if da < near && db < near {
            return None;
        }
        let cut = |from: V3, to: V3, d_from: f32, d_to: f32| lerp(from, to, (near - d_from) / (d_to - d_from) + 1e-4);
        let a = if da < near { cut(a, b, da, db) } else { a };
        let b = if db < near { cut(b, a, db, da) } else { b };
        Some([self.project(a, rect)?, self.project(b, rect)?])
    }

    /// Turns the view by a mouse drag of `dx`, `dy` pixels.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Slides the target so the point under the cursor follows a drag of `dx`, `dy`
    /// pixels in a view `height` pixels tall.
    pub fn pan(&mut self, dx: f32, dy: f32, height: f32) {
        let (r, u, _) = self.basis();
        let ly_per_px = self.distance / self.px_per_ly(height);
        self.target = add(self.target, add(scale(r, -dx * ly_per_px), scale(u, dy * ly_per_px)));
    }

    /// Slides the target across the galactic plane, as WASD does in the game's map:
    /// `forward` toward the top of a top-down view (into the screen when tilted), `right`
    /// sideways. Both are in camera distances, so the speed follows the zoom.
    pub fn travel(&mut self, forward: f32, right: f32) {
        let (sy, cy) = self.yaw.sin_cos();
        let ahead = [-sy, 0.0, cy];
        let side = [cy, 0.0, sy];
        let step = add(scale(ahead, forward * self.distance), scale(side, right * self.distance));
        self.target = add(self.target, step);
    }

    /// Zooms by `factor` (< 1 moves closer).
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

/// [`Camera::project`] for one view, ready to run over thousands of points.
pub struct Projector {
    right: V3,
    up: V3,
    forward: V3,
    eye: V3,
    near: f32,
    k: f32,
    center: Pos2,
}

impl Projector {
    pub fn project(&self, p: V3) -> Option<Pos2> {
        let v = sub(p, self.eye);
        let depth = dot(v, self.forward);
        if depth < self.near {
            return None;
        }
        let k = self.k / depth;
        Some(pos2(self.center.x + dot(v, self.right) * k, self.center.y - dot(v, self.up) * k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    fn rect() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))
    }

    #[test]
    fn target_projects_to_the_centre() {
        let cam = Camera { target: [10.0, -5.0, 300.0], distance: 50.0, yaw: 0.7, pitch: 0.3 };
        let p = cam.project(cam.target, rect()).unwrap();
        assert!((p - rect().center()).length() < 1e-3);
    }

    #[test]
    fn top_down_view_has_x_right_and_core_up() {
        let cam = Camera { target: [0.0; 3], distance: 100.0, yaw: 0.0, pitch: MAX_PITCH };
        let c = rect().center();
        let east = cam.project([10.0, 0.0, 0.0], rect()).unwrap();
        let core = cam.project([0.0, 0.0, 10.0], rect()).unwrap();
        assert!(east.x > c.x + 1.0 && (east.y - c.y).abs() < 1.0);
        assert!(core.y < c.y - 1.0 && (core.x - c.x).abs() < 1.0);
    }

    #[test]
    fn points_behind_the_camera_are_not_projected() {
        let cam = Camera { target: [0.0; 3], distance: 10.0, yaw: 0.0, pitch: 0.7 };
        let behind = sub(cam.eye(), sub(cam.target, cam.eye()));
        assert!(cam.project(behind, rect()).is_none());
    }

    #[test]
    fn gpu_matrix_matches_cpu_projection() {
        let cam = Camera { target: [100.0, 20.0, -40.0], distance: 80.0, yaw: -1.1, pitch: 0.5 };
        let r = rect();
        let m = cam.view_proj(r.width() / r.height());
        let p = [130.0, 5.0, -10.0];
        let clip: Vec<f32> = (0..4).map(|row| m[row] * p[0] + m[4 + row] * p[1] + m[8 + row] * p[2] + m[12 + row]).collect();
        let (nx, ny) = (clip[0] / clip[3], clip[1] / clip[3]);
        let gpu = pos2(r.center().x + nx * r.width() / 2.0, r.center().y - ny * r.height() / 2.0);
        let cpu = cam.project(p, r).unwrap();
        assert!((gpu - cpu).length() < 0.01, "{gpu:?} vs {cpu:?}");
    }

    #[test]
    fn panning_moves_the_target_with_the_cursor() {
        let mut cam = Camera { target: [0.0; 3], distance: 100.0, yaw: 0.0, pitch: MAX_PITCH };
        let before = cam.project([0.0; 3], rect()).unwrap();
        cam.pan(40.0, 0.0, rect().height());
        let after = cam.project([0.0; 3], rect()).unwrap();
        // The world point that was under the cursor moved right with it.
        assert!((after.x - before.x - 40.0).abs() < 0.5, "{before:?} -> {after:?}");
    }

    #[test]
    fn travel_moves_up_the_screen_and_sideways_in_the_plane() {
        let mut cam = Camera { target: [0.0; 3], distance: 100.0, yaw: 0.0, pitch: MAX_PITCH };
        cam.travel(0.5, 0.0);
        // Top-down, forward is toward the core (+z), at the top of the view.
        assert!((cam.target[2] - 50.0).abs() < 1e-3 && cam.target[0].abs() < 1e-3);
        cam.travel(0.0, 0.25);
        assert!((cam.target[0] - 25.0).abs() < 1e-3);
        // Tilted or turned, the move stays in the galactic plane.
        let mut tilted = Camera { target: [0.0; 3], distance: 100.0, yaw: 1.0, pitch: 0.4 };
        tilted.travel(1.0, -1.0);
        assert!(tilted.target[1].abs() < 1e-3);
    }

    #[test]
    fn segments_are_cut_at_the_near_plane() {
        let cam = Camera { target: [0.0; 3], distance: 10.0, yaw: 0.0, pitch: 0.7 };
        let behind = sub(cam.eye(), sub(cam.target, cam.eye()));
        assert!(cam.project_segment(cam.target, behind, rect()).is_some());
        assert!(cam.project_segment(behind, scale(behind, 2.0), rect()).is_none());
    }
}
