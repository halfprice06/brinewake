//! The match clock as daylight.
//!
//! A slow colour key over the match: the cool morning haze of the first
//! minutes, a lifted noon, then warm low light after fifteen minutes.  The
//! key is applied to the scene canvas after the ground passes and before the
//! sorted world items, so machines, buildings, effects and the interface
//! keep their exact colours.  The key is a fixed function of the public
//! visual tick, so it is replay-neutral and identical for both players.

use crate::canvas::{Canvas, Color};
use bw_core::TICK_HZ;

const MINUTE: u64 = 60 * TICK_HZ;
/// Noon begins after five minutes and evening after fifteen; each change
/// blends in over thirty seconds so no frame steps.
const NOON_START: u64 = 5 * MINUTE;
const EVENING_START: u64 = 15 * MINUTE;
const BLEND_TICKS: u64 = 30 * TICK_HZ;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Morning,
    Noon,
    Evening,
}

/// Per-channel multipliers in 1/1024 units plus a flat lift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub lift: i32,
}

const IDENTITY: Key = Key {
    r: 1024,
    g: 1024,
    b: 1024,
    lift: 0,
};
/// Noon: the salt planes lift a step; the water keeps its hue.
const NOON: Key = Key {
    r: 1064,
    g: 1054,
    b: 1010,
    lift: 5,
};
/// Evening: warm, low light.  Red holds, green and blue fall, the whole
/// field darkens slightly so the lit machines stay the strongest read.
const EVENING: Key = Key {
    r: 1040,
    g: 930,
    b: 830,
    lift: -4,
};

impl Key {
    pub fn is_identity(self) -> bool {
        self == IDENTITY
    }
    fn blend(a: Key, b: Key, t: i32) -> Key {
        let mix = |x: i32, y: i32| x + (y - x) * t / 256;
        Key {
            r: mix(a.r, b.r),
            g: mix(a.g, b.g),
            b: mix(a.b, b.b),
            lift: mix(a.lift, b.lift),
        }
    }
    pub fn apply(self, c: Color) -> Color {
        let ch = |v: u8, k: i32| ((i32::from(v) * k) / 1024 + self.lift).clamp(0, 255) as u8;
        [ch(c[0], self.r), ch(c[1], self.g), ch(c[2], self.b), c[3]]
    }
}

pub fn phase(tick: u64) -> Phase {
    if tick < NOON_START {
        Phase::Morning
    } else if tick < EVENING_START {
        Phase::Noon
    } else {
        Phase::Evening
    }
}

/// Blend progress in 0..=256 from `start` over the blend window.
fn progress(tick: u64, start: u64) -> i32 {
    let elapsed = tick.saturating_sub(start).min(BLEND_TICKS);
    (elapsed * 256 / BLEND_TICKS) as i32
}

pub fn key(tick: u64) -> Key {
    if tick < NOON_START {
        IDENTITY
    } else if tick < EVENING_START {
        Key::blend(IDENTITY, NOON, progress(tick, NOON_START))
    } else {
        Key::blend(NOON, EVENING, progress(tick, EVENING_START))
    }
}

/// Evening lamp strength in 0..=256: lamps come on with the evening blend.
pub fn lamps(tick: u64) -> i32 {
    if tick < EVENING_START {
        0
    } else {
        progress(tick, EVENING_START)
    }
}

/// Recolour every pixel of the canvas rows in `top..bottom` with the key.
/// The identity key is a no-op, so the first five minutes cost nothing.
pub fn apply(canvas: &mut Canvas, key: Key, top: i32, bottom: i32) {
    if key.is_identity() {
        return;
    }
    let width = canvas.width() as i32;
    let top = top.max(0);
    let bottom = bottom.min(canvas.height() as i32);
    for y in top..bottom {
        for x in 0..width {
            if let Some(c) = canvas.get(x, y) {
                canvas.pixel(x, y, key.apply([c[0], c[1], c[2], 255]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_follow_the_match_clock_and_blend_smoothly() {
        assert_eq!(phase(0), Phase::Morning);
        assert_eq!(phase(NOON_START), Phase::Noon);
        assert_eq!(phase(EVENING_START), Phase::Evening);
        assert!(key(0).is_identity());
        assert!(key(NOON_START - 1).is_identity());
        assert_eq!(key(NOON_START + BLEND_TICKS), NOON);
        assert_eq!(key(EVENING_START + BLEND_TICKS), EVENING);
        assert_eq!(key(EVENING_START + 10 * MINUTE), EVENING);
        // Blending is monotonic in each channel over the noon window.
        let mut last = key(NOON_START).r;
        for step in 0..=BLEND_TICKS / 30 {
            let now = key(NOON_START + step * 30).r;
            assert!(now >= last);
            last = now;
        }
        assert_eq!(lamps(0), 0);
        assert_eq!(lamps(EVENING_START + BLEND_TICKS), 256);
    }

    #[test]
    fn evening_warms_and_noon_lifts_without_clipping_the_palette() {
        let salt = [157, 143, 118, 255];
        let noon = NOON.apply(salt);
        let evening = EVENING.apply(salt);
        assert!(noon[0] > salt[0] && noon[1] > salt[1]);
        assert!(evening[2] < salt[2] && evening[0] >= evening[1] && evening[1] > evening[2]);
        let white = [255, 255, 255, 255];
        assert_eq!(NOON.apply(white)[0], 255);
        assert_eq!(IDENTITY.apply(salt), salt);
    }

    #[test]
    fn apply_touches_only_the_requested_rows_and_skips_identity() {
        let mut canvas = Canvas::default();
        canvas.clear([100, 100, 100, 255]);
        apply(&mut canvas, IDENTITY, 0, 360);
        assert_eq!(canvas.get(5, 5), Some([100, 100, 100, 255]));
        apply(&mut canvas, EVENING, 10, 20);
        assert_eq!(canvas.get(5, 5), Some([100, 100, 100, 255]));
        assert_ne!(canvas.get(5, 15), Some([100, 100, 100, 255]));
        assert_eq!(canvas.get(5, 25), Some([100, 100, 100, 255]));
    }
}
