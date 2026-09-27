//! One wind over the basin.
//!
//! A single map-wide wind direction and a deterministic gust field over
//! world-projected pixel coordinates and the visual tick.  Bank plants, the
//! water crests and the headquarters loops all read the same field, so a
//! gust visibly sweeps across the field instead of each decoration moving on
//! its own clock.  The field reads no entity or visibility state, so it is
//! fog-safe, pause-stable and replay-neutral.

/// Screen-space lean of a gust: the wind comes off the sea from the west.
pub const LEAN_X: i32 = 1;

/// A gust band travels this many world pixels per tick.
const SPEED_PX_PER_TICK: i64 = 3;
/// Distance between successive gust fronts.  At three pixels per tick this
/// is one gust every eight seconds.
const PERIOD_PX: i64 = 720;
/// The strong core of a gust and the lighter shoulders on each side.
const CORE_PX: i64 = 120;
const SHOULDER_PX: i64 = 90;

/// Gust strength at a world-projected pixel: 0 calm, 1 light, 2 strong.
pub fn strength(world_x: i64, world_y: i64, tick: u64) -> u8 {
    strength_with_jitter(world_x, world_y, tick, 0)
}

/// Gust strength with a per-object phase jitter in pixels, so a row of
/// plants does not lean in lockstep even when they share a screen column.
pub fn strength_with_jitter(world_x: i64, world_y: i64, tick: u64, jitter: i64) -> u8 {
    // The front runs down-screen and to the right, matching the 2:1 ground
    // projection, so a gust crosses a diagonal bank as a diagonal.
    let along = world_x + world_y / 2 + jitter - (tick as i64).wrapping_mul(SPEED_PX_PER_TICK);
    let phase = along.rem_euclid(PERIOD_PX);
    if phase < SHOULDER_PX {
        1
    } else if phase < SHOULDER_PX + CORE_PX {
        2
    } else if phase < SHOULDER_PX * 2 + CORE_PX {
        1
    } else {
        0
    }
}

/// Deterministic jitter for a decoration at a map cell, in pixels.
pub fn cell_jitter(x: i32, y: i32) -> i64 {
    let seed = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
    i64::from((seed ^ (seed >> 13)).wrapping_mul(1_274_126_177) % 61)
}

/// Sway frame suffix for a bank plant at the given gust strength.
pub fn plant_key(base: &str, gust: u8) -> String {
    match gust {
        0 => base.to_string(),
        1 => format!("{base}_sway_1"),
        _ => format!("{base}_sway_2"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gust_bands_travel_and_cover_every_strength() {
        let mut seen = [false; 3];
        for tick in 0..240u64 {
            seen[usize::from(strength(0, 0, tick))] = true;
        }
        assert!(seen.iter().all(|s| *s), "{seen:?}");
        // A point down-wind sees the same band later.
        let t0 = (0..240u64).find(|&t| strength(0, 0, t) == 2).unwrap();
        assert_eq!(strength(0, 0, t0), 2);
        assert_eq!(strength(SPEED_PX_PER_TICK * 30, 0, t0 + 30), 2);
    }

    #[test]
    fn calm_dominates_and_the_period_is_eight_seconds() {
        let calm = (0..PERIOD_PX).filter(|&px| strength(px, 0, 0) == 0).count() as i64;
        assert!(calm * 2 > PERIOD_PX, "calm {calm} of {PERIOD_PX}");
        assert_eq!(PERIOD_PX / SPEED_PX_PER_TICK, 240);
        assert_eq!(strength(5, 9, 100), strength(5, 9, 100 + 240));
    }

    #[test]
    fn plant_keys_and_jitter_are_deterministic() {
        assert_eq!(plant_key("reed_clump", 0), "reed_clump");
        assert_eq!(plant_key("reed_clump", 1), "reed_clump_sway_1");
        assert_eq!(plant_key("salt_bush", 2), "salt_bush_sway_2");
        assert_eq!(cell_jitter(7, 9), cell_jitter(7, 9));
        assert!((0..61).contains(&cell_jitter(3, 4)));
    }
}
