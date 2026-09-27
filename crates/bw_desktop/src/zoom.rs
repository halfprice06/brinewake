//! World scale, independent of window size and UI scale. The world always
//! draws at its native grid; the enlargement to the window is a whole-pixel
//! block copy at 1×, 2× and 3×, and a sharp-edged resample in between.
use bw_core::{Camera, Pos, WORLD_TOP};

pub const MIN_SCALE: f64 = 1.0;
pub const MAX_SCALE: f64 = 3.0;
/// One wheel click: three clicks double the scale, so from 1× the wheel
/// passes through 2× exactly.
pub const WHEEL_RATIO: f64 = 1.259_921_049_894_873_2;

/// An eased zoom on its way.
#[derive(Clone, Copy, Debug)]
pub struct ZoomGoal {
    pub zoom: Zoom,
    /// The window pixel that keeps its place in the world.
    pub anchor: (i32, i32),
    /// Set by the wheel, which may still be turning.
    pub wheel: bool,
    /// The last input that moved the goal.
    pub since: std::time::Instant,
    pub last_frame: std::time::Instant,
}

/// The world's enlargement. The whole steps keep their old names.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Zoom(f64);
#[allow(non_upper_case_globals)]
impl Zoom {
    pub const Detail: Self = Self(3.0);
    pub const Wide: Self = Self(2.0);
    pub const Overview: Self = Self(1.0);
}
impl Default for Zoom {
    fn default() -> Self {
        Self::Overview
    }
}
impl Zoom {
    pub fn new(scale: f64) -> Self {
        Self(if scale.is_finite() {
            scale.clamp(MIN_SCALE, MAX_SCALE)
        } else {
            2.0
        })
    }
    pub const fn scale(self) -> f64 {
        self.0
    }
    /// The nearest whole step, for names and reports.
    pub fn factor(self) -> i32 {
        self.0.round() as i32
    }
    /// The whole step this scale is, if it is one.
    pub fn whole(self) -> Option<i32> {
        let r = self.0.round();
        ((self.0 - r).abs() < 1e-6).then_some(r as i32)
    }
    /// The readout on the zoom control: "2X", or "2.4X" between steps.
    pub fn label(self) -> String {
        match self.whole() {
            Some(k) => format!("{k}X"),
            None => format!("{:.1}X", self.0),
        }
    }
    pub fn percent(self) -> i32 {
        (self.0 * 100.0).round() as i32
    }
    /// Read the saved preference, including the older percentages.
    pub fn from_preference(value: f32) -> Self {
        match value {
            50.0 => Self::Overview,
            100.0 => Self::Detail,
            v if (1.0..=3.0).contains(&v) => Self::new(f64::from(v)),
            _ => Self::Wide,
        }
    }
    /// The next whole step farther out, or this one if it is the widest.
    pub fn out(self) -> Self {
        Self::new((self.0 - 1e-6).ceil() - 1.0)
    }
    /// The next whole step closer in, or this one if it is the nearest.
    pub fn closer(self) -> Self {
        Self::new((self.0 + 1e-6).floor() + 1.0)
    }
    /// A whole step within `margin` (a ratio, 0.07 for 7%), if one is near.
    pub fn nearby_step(self, margin: f64) -> Option<Self> {
        let r = self.0.round();
        ((self.0 / r).ln().abs() < margin.ln_1p()).then(|| Self::new(r))
    }
}

/// One shared transform for rendering, picking, culling and camera anchoring.
/// Camera's stored center stays in native artwork pixels, preserving saves;
/// `sub` is the presentation's part of a texel on top of it.
#[derive(Clone, Copy, Debug)]
pub struct WorldView {
    pub width: i32,
    pub top: i32,
    pub bottom: i32,
    pub zoom: Zoom,
    pub sub: (f64, f64),
}
impl WorldView {
    pub fn center(self) -> (i32, i32) {
        (self.width / 2, (self.top + self.bottom) / 2)
    }
    pub fn contains(self, x: i32, y: i32) -> bool {
        (0..self.width).contains(&x) && (self.top..self.bottom).contains(&y)
    }
    /// The scene camera's offset from the stored camera, in whole texels,
    /// and the scene's remaining shift in texels. At a whole step the shift
    /// is a whole number of window pixels, so every texel stays a block.
    fn grid(self) -> ((i32, i32), (f64, f64)) {
        let s = self.zoom.scale();
        let (cx, cy) = self.center();
        let axis = |origin: f64| {
            let whole = origin.floor();
            let mut shift = origin - whole;
            if self.zoom.whole().is_some() {
                shift = (shift * s).round() / s;
            }
            if shift >= 1.0 {
                (whole as i32 + 1, 0.0)
            } else {
                (whole as i32, shift)
            }
        };
        let (ox, ex) = axis(self.sub.0 + 320.0 - f64::from(cx) / s);
        let (oy, ey) =
            axis(self.sub.1 + 156.0 - f64::from(WORLD_TOP) - f64::from(cy - self.top) / s);
        ((ox, oy), (ex, ey))
    }
    /// The scene canvas: every texel the window can show, plus one more on
    /// each far edge for the resample's neighbour.
    pub fn size(self) -> (u32, u32) {
        let s = self.zoom.scale();
        (
            (f64::from(self.width) / s).ceil() as u32 + 2,
            WORLD_TOP as u32 + (f64::from(self.bottom - self.top) / s).ceil() as u32 + 2,
        )
    }
    /// The scene texel under a window pixel's centre, in scene coordinates.
    pub fn source_exact(self, point: (i32, i32)) -> (f64, f64) {
        let s = self.zoom.scale();
        let (_, (ex, ey)) = self.grid();
        (
            (f64::from(point.0) + 0.5) / s + ex,
            f64::from(WORLD_TOP) + (f64::from(point.1 - self.top) + 0.5) / s + ey,
        )
    }
    pub fn source(self, point: (i32, i32)) -> (i32, i32) {
        let (x, y) = self.source_exact(point);
        (x.floor() as i32, y.floor() as i32)
    }
    /// The first window pixel a scene texel covers.
    pub fn screen(self, point: (i32, i32)) -> (i32, i32) {
        let s = self.zoom.scale();
        let (_, (ex, ey)) = self.grid();
        (
            ((f64::from(point.0) - ex) * s - 0.5 + 1e-9).ceil() as i32,
            self.top + ((f64::from(point.1 - WORLD_TOP) - ey) * s - 0.5 + 1e-9).ceil() as i32,
        )
    }
    pub fn scene_camera(self, camera: Camera) -> Camera {
        let ((ox, oy), _) = self.grid();
        Camera {
            x: camera.x + ox,
            y: camera.y + oy,
        }
    }
    pub fn project(self, camera: Camera, pos: Pos) -> (i32, i32) {
        self.screen(self.scene_camera(camera).project(pos))
    }
    pub fn unproject(self, camera: Camera, point: (i32, i32)) -> Pos {
        let p = self.source(point);
        self.scene_camera(camera).unproject(p.0, p.1)
    }
    /// For each window column (or row) from `from` to `to`, the scene texel
    /// to its left (or above) and the right-hand texel's weight out of 256.
    /// Inside a texel the weight is 0; only the one window pixel that a
    /// texel edge crosses is blended, so edges stay one pixel sharp.
    pub fn taps(self, from: i32, to: i32, vertical: bool, limit: usize) -> Vec<(usize, u16)> {
        let s = self.zoom.scale();
        (from..to)
            .map(|p| {
                let (x, y) = self.source_exact(if vertical { (0, p) } else { (p, self.top) });
                let u = if vertical { y } else { x };
                let seam = u.round();
                let t = (0.5 + (u - seam) * s).clamp(0.0, 1.0);
                let w = (t * 256.0).round() as u16;
                let left = seam as i64 - 1;
                let clamp = |i: i64| i.clamp(0, limit as i64 - 1) as usize;
                if w == 0 {
                    (clamp(left), 0)
                } else if w >= 256 {
                    (clamp(left + 1), 0)
                } else {
                    (clamp(left), w)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_zoom_preserves_each_source_texel_as_an_integer_block() {
        for zoom in [Zoom::Overview, Zoom::Wide, Zoom::Detail] {
            let k = zoom.whole().unwrap();
            for sub in [(0.0, 0.0), (0.5, 1.0 / 3.0), (2.0 / 3.0, 0.25)] {
                let view = WorldView {
                    width: 1919,
                    top: 64,
                    bottom: 841,
                    zoom,
                    sub,
                };
                for p in [(0, WORLD_TOP), (50, 60), (100, 120)] {
                    let d = view.screen(p);
                    for y in 0..k {
                        for x in 0..k {
                            assert_eq!(view.source((d.0 + x, d.1 + y)), p);
                        }
                    }
                    assert_ne!(view.source((d.0 - 1, d.1)), p);
                }
                for (_, w) in view.taps(0, view.width, false, 2000) {
                    assert_eq!(w, 0, "a whole step never blends");
                }
                let last = view.source((1918, 840));
                let size = view.size();
                assert!(last.0 + 1 < size.0 as i32 && last.1 + 1 < size.1 as i32);
            }
            let view = WorldView {
                width: 1920,
                top: 64,
                bottom: 840,
                zoom,
                sub: (0.0, 0.0),
            };
            let c = Camera { x: 30, y: 400 };
            assert_eq!(view.unproject(c, view.center()), c.unproject(320, 156));
        }
    }
    #[test]
    fn between_steps_each_texel_edge_blends_one_pixel_at_most() {
        for scale in [1.25, 1.5, 2.37, 2.9] {
            let view = WorldView {
                width: 1920,
                top: 64,
                bottom: 840,
                zoom: Zoom::new(scale),
                sub: (0.3, 0.7),
            };
            let taps = view.taps(0, view.width, false, 4000);
            for pair in taps.windows(2) {
                assert!(pair[1].0 >= pair[0].0);
            }
            // Each texel keeps within a pixel of the same solid width; the
            // rest of its width is the blended pixel at each edge.
            let mut runs = vec![0; 4000];
            for (i, w) in &taps {
                if *w == 0 {
                    runs[*i] += 1;
                }
            }
            let inner: Vec<_> = runs.iter().copied().filter(|&n| n > 0).collect();
            for n in &inner[1..inner.len() - 1] {
                assert!((*n as f64 - (scale - 1.0)).abs() < 1.0, "{scale}: {n}");
            }
        }
    }
    #[test]
    fn steps_and_preferences() {
        assert_eq!(Zoom::Overview.closer(), Zoom::Wide);
        assert_eq!(Zoom::new(1.4).closer(), Zoom::Wide);
        assert_eq!(Zoom::new(2.6).out(), Zoom::Wide);
        assert_eq!(Zoom::Detail.closer(), Zoom::Detail);
        assert_eq!(Zoom::Overview.out(), Zoom::Overview);
        assert_eq!(Zoom::new(2.4).label(), "2.4X");
        assert_eq!(Zoom::Wide.label(), "2X");
        assert_eq!(Zoom::from_preference(50.0), Zoom::Overview);
        assert_eq!(Zoom::from_preference(100.0), Zoom::Detail);
        assert_eq!(Zoom::from_preference(2.5), Zoom::new(2.5));
        assert_eq!(Zoom::from_preference(7.0), Zoom::Wide);
        assert_eq!(Zoom::new(1.95).nearby_step(0.07), Some(Zoom::Wide));
        assert_eq!(Zoom::new(2.4).nearby_step(0.07), None);
    }
}
