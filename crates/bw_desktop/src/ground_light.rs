//! Ground light: broad tonal planes for the salt floor and a damp waterline.
//!
//! The tone of a cell is a deterministic function of its coordinates and the
//! map's terrain classes only.  It never reads entities, visibility, or the
//! presentation clock, so it is fog-safe, pause-stable and replay-neutral.
//! Values are chosen so that low and high areas form connected patches of
//! several cells rather than per-tile speckle.
use bw_core::Terrain;
use bw_sim::Map;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    Low,
    Mid,
    High,
}

/// Hash one lattice point into `0..=1023`.
fn lattice(x: i64, y: i64, salt: u64) -> i64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt;
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 1023) as i64
}

/// Bilinear value noise in `0..=1023` for a lattice of `period` cells.
fn value_noise(x: i32, y: i32, period: i32, salt: u64) -> i64 {
    let (x, y) = (i64::from(x), i64::from(y));
    let p = i64::from(period.max(1));
    let (gx, gy) = (x.div_euclid(p), y.div_euclid(p));
    let (fx, fy) = (x.rem_euclid(p), y.rem_euclid(p));
    let a = lattice(gx, gy, salt);
    let b = lattice(gx + 1, gy, salt);
    let c = lattice(gx, gy + 1, salt);
    let d = lattice(gx + 1, gy + 1, salt);
    // Smoothstep weights in 0..=p*p to stay in exact integer arithmetic.
    let wx = fx * fx * (3 * p - 2 * fx);
    let wy = fy * fy * (3 * p - 2 * fy);
    let p3 = p * p * p;
    let top = a * (p3 - wx) + b * wx;
    let bottom = c * (p3 - wx) + d * wx;
    (top * (p3 - wy) + bottom * wy) / (p3 * p3)
}

/// The tonal plane for a floor cell.
pub fn tone(x: i32, y: i32) -> Tone {
    let broad = value_noise(x, y, 9, 0x5A17);
    let fine = value_noise(x, y, 4, 0xB12E);
    let v = (broad * 7 + fine * 3) / 10;
    if v < 360 {
        Tone::Low
    } else if v >= 690 {
        Tone::High
    } else {
        Tone::Mid
    }
}

const SIDES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn touches_deep(map: &Map, x: i32, y: i32) -> bool {
    SIDES
        .iter()
        .any(|(dx, dy)| map.terrain(x + dx, y + dy) == Terrain::Deep)
}

/// True when a walkable floor cell is part of the damp waterline: every cell
/// that touches deep water, plus an irregular second row so the line does
/// not read as a uniform strip.
pub fn is_damp(map: &Map, x: i32, y: i32) -> bool {
    if !matches!(map.terrain(x, y), Terrain::Salt | Terrain::Silt) {
        return false;
    }
    if touches_deep(map, x, y) {
        return true;
    }
    lattice(i64::from(x), i64::from(y), 0x77D1) % 5 < 2
        && SIDES.iter().any(|(dx, dy)| {
            matches!(map.terrain(x + dx, y + dy), Terrain::Salt | Terrain::Silt)
                && touches_deep(map, x + dx, y + dy)
        })
}

/// True when a low or high cell borders a cell of another tone; such cells
/// take the intermediate step so a plane ramps over two cells.
fn on_tone_boundary(x: i32, y: i32, t: Tone) -> bool {
    SIDES.iter().any(|(dx, dy)| tone(x + dx, y + dy) != t)
}

/// Atlas key for a floor cell, or `None` when the cell keeps its existing key.
pub fn floor_key(map: &Map, x: i32, y: i32, variant: i32) -> Option<String> {
    if is_damp(map, x, y) {
        return Some(format!("terrain_damp_{variant}"));
    }
    if !matches!(map.terrain(x, y), Terrain::Salt | Terrain::Rock) {
        return None;
    }
    let t = tone(x, y);
    let family = match (t, on_tone_boundary(x, y, t)) {
        (Tone::Low, true) => "dim",
        (Tone::Low, false) => "low",
        (Tone::High, true) => "pale",
        (Tone::High, false) => "high",
        (Tone::Mid, _) => return None,
    };
    Some(format!("terrain_salt_{family}_{variant}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_sim::World;

    #[test]
    fn tone_is_deterministic_and_forms_broad_patches() {
        let mut counts = [0usize; 3];
        let mut same_as_east = 0usize;
        let mut total = 0usize;
        for y in 0..128 {
            for x in 0..128 {
                let t = tone(x, y);
                assert_eq!(t, tone(x, y));
                counts[match t {
                    Tone::Low => 0,
                    Tone::Mid => 1,
                    Tone::High => 2,
                }] += 1;
                if tone(x + 1, y) == t {
                    same_as_east += 1;
                }
                total += 1;
            }
        }
        // Every tone appears, the mid tone stays the largest floor area, and
        // neighbouring cells usually share a tone (patches, not speckle).
        assert!(counts.iter().all(|&n| n > total / 20), "{counts:?}");
        assert!(
            counts[1] >= counts[0] && counts[1] >= counts[2],
            "{counts:?}"
        );
        assert!(
            same_as_east * 100 / total >= 85,
            "{same_as_east} of {total}"
        );
    }

    #[test]
    fn damp_cells_touch_deep_water_and_lanes_keep_their_keys() {
        let world = World::new(7, bw_core::Faction::Union);
        let map = &world.map;
        let mut damp = 0usize;
        for y in 0..i32::from(map.height) {
            for x in 0..i32::from(map.width) {
                let key = floor_key(map, x, y, 0);
                let t = map.terrain(x, y);
                if let Some(k) = &key {
                    assert!(
                        k.starts_with("terrain_damp_") || k.starts_with("terrain_salt_"),
                        "{k}"
                    );
                }
                if key.as_deref() == Some("terrain_damp_0") {
                    damp += 1;
                    assert!(is_damp(map, x, y));
                    assert!(matches!(t, Terrain::Salt | Terrain::Silt));
                }
                if matches!(
                    t,
                    Terrain::Deep | Terrain::Lane0 | Terrain::Lane1 | Terrain::Rim1 | Terrain::Rim0
                ) {
                    assert_eq!(key, None, "{t:?} at {x},{y} must keep its key");
                }
                if t == Terrain::Silt && !is_damp(map, x, y) {
                    assert_eq!(key, None);
                }
            }
        }
        assert!(damp > 100, "expected a full waterline, found {damp}");
    }
}
