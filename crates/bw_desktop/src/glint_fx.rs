//! GLINT's light on the field (art v23, `art/source/v23/fx_glint_*`).
//!
//! A Glinter's heliograph flash at the mirror for the first moments, the
//! ray's spine on the ground, three motes of light running out along it
//! from the mirror's end, and a ring of light where its reach ends. These
//! are the frame and placement rules; `game.rs` draws them.

/// Ticks the heliograph flash shows at the mirror (four frames, two ticks
/// each).
pub const FLASH_TICKS: u64 = 8;
/// Ticks one mote takes to run the length of the ray.
pub const MOTE_RUN: u64 = 45;
/// Motes on a ray, a third of a run apart.
pub const MOTES: u64 = 3;
/// The motes stop this many ticks before the light goes out, so the ray
/// dims without sparks still leaving the mirror.
pub const MOTE_TAIL: u64 = 15;

/// The flash's frame `age` ticks after the order, while it shows.
pub fn flash_key(age: u64) -> Option<String> {
    (age < FLASH_TICKS).then(|| format!("fx_glint_flash_{}", age / 2))
}

/// The ring at the ray's end: four frames of four ticks, looping.
pub fn spot_key(age: u64) -> String {
    format!("fx_glint_spot_{}", (age / 4) % 4)
}

/// The motes in flight: each one's frame key and how far along the ray it
/// is, in thousandths. Mote `i` leaves the mirror `i * MOTE_RUN / MOTES`
/// ticks after the order and eases out as it runs (fast from the mirror,
/// slowing toward the end). None fly in the last `MOTE_TAIL` ticks.
pub fn motes(age: u64, left: u64) -> Vec<(String, i32)> {
    if left <= MOTE_TAIL {
        return Vec::new();
    }
    (0..MOTES)
        .filter_map(|i| {
            let start = i * MOTE_RUN / MOTES;
            let run = age.checked_sub(start)? % MOTE_RUN;
            // s = 1 - (1 - u)^2 with u = run / MOTE_RUN, in thousandths.
            let rest = MOTE_RUN - run;
            let along = 1000 - (rest * rest * 1000 / (MOTE_RUN * MOTE_RUN)) as i32;
            Some((format!("fx_glint_mote_{}", (age / 3 + i) % 3), along))
        })
        .collect()
}

/// The point `along` thousandths of the way from `a` to `b`.
pub fn along(a: (i32, i32), b: (i32, i32), along: i32) -> (i32, i32) {
    (
        a.0 + (b.0 - a.0) * along / 1000,
        a.1 + (b.1 - a.1) * along / 1000,
    )
}

/// The spine's two strokes: a pale glint core and the glaze's pale violet
/// beside it, both fading as the light runs out.
pub fn spine_colours(left: u64, total: u64) -> ([u8; 4], [u8; 4]) {
    let alpha = (40 + left.min(total) * 160 / total.max(1)) as u8;
    ([255, 251, 230, alpha], [164, 173, 224, alpha / 2 + 20])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flash_runs_its_four_frames_then_stops() {
        let keys: Vec<_> = (0..10).map(flash_key).collect();
        assert_eq!(keys[0].as_deref(), Some("fx_glint_flash_0"));
        assert_eq!(keys[3].as_deref(), Some("fx_glint_flash_1"));
        assert_eq!(keys[7].as_deref(), Some("fx_glint_flash_3"));
        assert_eq!(keys[8], None);
    }

    #[test]
    fn the_spot_loops_its_four_frames() {
        assert_eq!(spot_key(0), "fx_glint_spot_0");
        assert_eq!(spot_key(15), "fx_glint_spot_3");
        assert_eq!(spot_key(16), "fx_glint_spot_0");
    }

    #[test]
    fn motes_leave_the_mirror_in_turn_and_ease_out() {
        let total = u64::from(bw_content::GLINT_TICKS);
        assert_eq!(motes(0, total).len(), 1);
        assert_eq!(motes(0, total)[0].1, 0);
        assert_eq!(motes(15, total - 15).len(), 2);
        assert_eq!(motes(30, total - 30).len(), 3);
        // Eases out: the first third of the run covers more than a third.
        let (_, third) = motes(15, total - 15)[0].clone();
        assert!(third > 333 && third < 1000, "{third}");
        // Every mote stays on the ray and names an authored frame.
        for age in 0..total {
            for (key, at) in motes(age, total - age) {
                assert!((0..=1000).contains(&at), "{age} {at}");
                assert!(key.starts_with("fx_glint_mote_"));
            }
        }
        assert!(motes(total - MOTE_TAIL, MOTE_TAIL).is_empty());
    }

    #[test]
    fn the_spine_fades_with_the_light() {
        let total = u64::from(bw_content::GLINT_TICKS);
        let (bright, _) = spine_colours(total, total);
        let (dim, edge) = spine_colours(0, total);
        assert!(bright[3] > dim[3]);
        assert!(edge[3] > 0);
        assert_eq!(along((0, 0), (100, -50), 500), (50, -25));
    }
}
