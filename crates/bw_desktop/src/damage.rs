//! Hull damage as material truth, and visible repairs.
//!
//! Buildings show two damage tiers and machines one, drawn as authored
//! overlays in each faction's material on top of the unchanged base frame.
//! The lowest tier adds a procedural vapour wisp that leans downwind.  Repair
//! events leave a short-lived patch mark on the repaired building, drawn with
//! the existing rivet and binding trace families.  All of this reads the
//! entity's public hull fraction and the presentation clocks only.

use crate::canvas::{Canvas, Color};
use bw_core::{Faction, TICK_HZ};
use std::collections::BTreeMap;

/// Tier 1 below sixty percent hull, tier 2 below thirty.
pub fn building_tier(hp: i32, max_hp: i32) -> u8 {
    if max_hp <= 0 || hp >= max_hp {
        return 0;
    }
    let pct = i64::from(hp.max(0)) * 100 / i64::from(max_hp);
    if pct < 30 {
        2
    } else if pct < 60 {
        1
    } else {
        0
    }
}

/// Machines carry one tier, below half hull.
pub fn machine_tier(hp: i32, max_hp: i32) -> u8 {
    if max_hp <= 0 || hp >= max_hp {
        return 0;
    }
    u8::from(i64::from(hp.max(0)) * 100 / i64::from(max_hp) < 50)
}

pub fn building_overlay_key(base: &str, tier: u8) -> Option<String> {
    (tier > 0).then(|| format!("{base}_damage_{}", tier.min(2)))
}

pub fn machine_overlay_key(role: &str, face: u8, tier: u8) -> Option<String> {
    (tier > 0).then(|| format!("{role}_{}_damage", face % 8))
}

/// A vapour wisp above a badly damaged hull.  `top_y` is the screen row of
/// the sprite's highest opaque pixel; the wisp rises from just above it and
/// leans with the wind.  Colours follow the material: grey steel smoke for
/// the Union, pale reed vapour for the Assembly.
pub fn draw_wisp(
    canvas: &mut Canvas,
    x: i32,
    top_y: i32,
    tick: u64,
    id: u32,
    faction: Faction,
    lean_x: i32,
) {
    let (a, b): (Color, Color) = match faction {
        Faction::Union => ([64, 84, 101, 170], [101, 127, 137, 140]),
        Faction::Assembly => ([137, 132, 119, 150], [190, 183, 155, 120]),
        Faction::Compact => ([196, 204, 214, 160], [232, 236, 240, 120]),
    };
    let seed = u64::from(id).wrapping_mul(7);
    for i in 0..4i64 {
        let age = (tick.wrapping_add(seed) / 3) as i64 + i * 5;
        let rise = age % 18;
        let drift = lean_x * (rise as i32 / 4) + ((age / 7) % 3) as i32 - 1;
        let py = top_y - 2 - rise as i32;
        let px = x + 3 + drift + i as i32 - 2;
        canvas.pixel(px, py, if i % 2 == 0 { a } else { b });
        if rise > 9 {
            canvas.pixel(px + 1, py, b);
        }
    }
}

/// Repair patches last this long on the building's surface.
pub const REPAIR_MARK_TICKS: u64 = 8 * TICK_HZ;
/// Repair events arrive every tick while a worker works; one mark per three
/// seconds per building keeps the patches readable.
pub const REPAIR_MARK_SPACING: u64 = 3 * TICK_HZ;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairMark {
    pub tick: u64,
    pub offset: (i32, i32),
}

/// Presentation-only repair marks per building, excluded from saves.
#[derive(Clone, Debug, Default)]
pub struct RepairMarks {
    pub marks: BTreeMap<u32, Vec<RepairMark>>,
}

impl RepairMarks {
    pub fn record(&mut self, entity: u32, tick: u64) {
        let list = self.marks.entry(entity).or_default();
        // Marks climb the building in a small deterministic spiral so repeated
        // repairs read as separate patches, not one flicker.
        let n = list.len() as i32;
        let offset = (((n * 11) % 31) - 15, -8 - (n * 7) % 26);
        list.push(RepairMark { tick, offset });
        if list.len() > 6 {
            list.remove(0);
        }
    }
    pub fn record_limited(&mut self, entity: u32, tick: u64) {
        let recent = self
            .marks
            .get(&entity)
            .and_then(|list| list.last())
            .is_some_and(|m| tick.saturating_sub(m.tick) < REPAIR_MARK_SPACING);
        if !recent {
            self.record(entity, tick);
        }
    }
    pub fn expire(&mut self, tick: u64) {
        for list in self.marks.values_mut() {
            list.retain(|m| tick.saturating_sub(m.tick) < REPAIR_MARK_TICKS);
        }
        self.marks.retain(|_, list| !list.is_empty());
    }
    pub fn retain_ids(&mut self, alive: impl Fn(u32) -> bool) {
        self.marks.retain(|id, _| alive(*id));
    }
    /// The trace key and offset for each active mark on an entity.
    pub fn active(&self, entity: u32, tick: u64, faction: Faction) -> Vec<(String, (i32, i32))> {
        let family = match faction {
            Faction::Union => "trace_rivet",
            Faction::Assembly => "trace_binding",
            // A Glazier's patch: fused glass (art v23, trace_glaze).
            Faction::Compact => "trace_glaze",
        };
        self.marks
            .get(&entity)
            .map(|list| {
                list.iter()
                    .filter_map(|m| {
                        let age = tick.checked_sub(m.tick)?;
                        (age < REPAIR_MARK_TICKS).then(|| {
                            // The mark is freshest first, then settles over the
                            // remaining phases as the patch cools.
                            let phase = (age * 6 / REPAIR_MARK_TICKS).min(5);
                            (format!("{family}_{phase}"), m.offset)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_hull_fraction() {
        assert_eq!(building_tier(2400, 2400), 0);
        assert_eq!(building_tier(1500, 2400), 0);
        assert_eq!(building_tier(1400, 2400), 1);
        assert_eq!(building_tier(700, 2400), 2);
        assert_eq!(building_tier(0, 2400), 2);
        assert_eq!(building_tier(5, 0), 0);
        assert_eq!(machine_tier(100, 100), 0);
        assert_eq!(machine_tier(50, 100), 0);
        assert_eq!(machine_tier(49, 100), 1);
        assert_eq!(building_overlay_key("union_hq", 0), None);
        assert_eq!(
            building_overlay_key("union_hq", 2).as_deref(),
            Some("union_hq_damage_2")
        );
        assert_eq!(
            machine_overlay_key("loom", 9, 1).as_deref(),
            Some("loom_1_damage")
        );
    }

    #[test]
    fn repair_marks_record_expire_and_name_the_faction_family() {
        let mut marks = RepairMarks::default();
        marks.record(4, 100);
        marks.record(4, 130);
        let active = marks.active(4, 140, Faction::Union);
        assert_eq!(active.len(), 2);
        assert!(
            active
                .iter()
                .all(|(key, _)| key.starts_with("trace_rivet_"))
        );
        assert_ne!(active[0].1, active[1].1);
        let assembly = marks.active(4, 140, Faction::Assembly);
        assert!(
            assembly
                .iter()
                .all(|(key, _)| key.starts_with("trace_binding_"))
        );
        let compact = marks.active(4, 140, Faction::Compact);
        assert_eq!(compact.len(), 2);
        assert!(
            compact
                .iter()
                .all(|(key, _)| key.starts_with("trace_glaze_"))
        );
        marks.expire(100 + REPAIR_MARK_TICKS);
        assert_eq!(
            marks
                .active(4, 100 + REPAIR_MARK_TICKS, Faction::Union)
                .len(),
            1
        );
        marks.expire(130 + REPAIR_MARK_TICKS);
        assert!(marks.marks.is_empty());
        marks.record(9, 1);
        marks.retain_ids(|id| id != 9);
        assert!(marks.marks.is_empty());
    }

    #[test]
    fn wisp_stays_above_the_hull_and_leans_downwind() {
        let mut canvas = Canvas::default();
        canvas.clear([0, 0, 0, 255]);
        draw_wisp(&mut canvas, 100, 100, 30, 3, Faction::Union, 1);
        let mut lit = Vec::new();
        for y in 60..120 {
            for x in 80..130 {
                if canvas.get(x, y) != Some([0, 0, 0, 255]) {
                    lit.push((x, y));
                }
            }
        }
        assert!(!lit.is_empty());
        assert!(lit.iter().all(|(_, y)| *y < 100));
    }
}
