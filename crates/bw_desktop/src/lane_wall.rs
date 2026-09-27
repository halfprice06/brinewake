//! A deep lane drawn as a wall on the field.
//!
//! In the eighth trial Union believed a deep lane only slowed machines; it
//! is impassable, and only the chart said so.  Wherever a deep tidal cell
//! meets ground a machine could stand on, a boom now lies across the edge:
//! floating timbers between banded marker posts, the way a harbour closes
//! a channel.  The mouths also carry a plate saying the lane is closed.
//! Presentation only: it reads the public tide state and the map.

use crate::canvas::{Canvas, Color};
use bw_core::{Camera, Pos, Terrain};
use bw_sim::World;
use std::collections::BTreeSet;

const TIMBER: Color = [132, 92, 56, 255];
const TIMBER_LIGHT: Color = [176, 128, 76, 255];
const TIMBER_SHADOW: Color = [30, 34, 36, 255];
const POST: Color = [30, 34, 36, 255];
const BAND_RED: Color = [206, 72, 58, 255];
const BAND_WHITE: Color = [226, 218, 196, 255];
/// The warning a closed mouth's hover card carries.
pub const CLOSED_TEXT: &str = "DEEP: NO WAY ACROSS";

/// Whether a tidal cell is deep now: a wall to every walking machine.
pub fn deep_lane(world: &World, terrain: Terrain) -> bool {
    matches!(world.depth(terrain), Some(bw_sim::Depth::Deep))
}

/// Whether an arm's lane is deep now (on the Split Basin arm 0 is the north
/// lane and arm 1 the south).  An arm the map lacks is never closed.
pub fn lane_closed(world: &World, arm: usize) -> bool {
    arm < world.map.layout().arm_count()
        && u8::try_from(arm).is_ok_and(|arm| deep_lane(world, Terrain::lane(arm)))
}

fn standable(t: Terrain) -> bool {
    matches!(t, Terrain::Salt | Terrain::Silt | Terrain::Rock)
}

/// A screen segment and whether its tidal cell is in sight.
type Edge = ((i32, i32), (i32, i32), bool);

/// The edges between deep tidal cells and ground, as screen segments.
fn edges(world: &World, camera: Camera, view: (i32, i32)) -> Vec<Edge> {
    let map = &world.map;
    let mut out = Vec::new();
    for y in 0..i32::from(map.height) {
        for x in 0..i32::from(map.width) {
            let t = map.terrain(x, y);
            if !t.is_tidal() || !deep_lane(world, t) || world.is_crusted(x, y) {
                continue;
            }
            let (sx, sy) = camera.project(Pos::cell(x, y));
            if !(-40..view.0 + 40).contains(&sx) || !(-40..view.1 + 40).contains(&sy) {
                continue;
            }
            let visible = world.visible(0, Pos::cell(x, y));
            for (dx, dy, a, b) in [
                (1, 0, (sx + 16, sy), (sx, sy + 8)),
                (-1, 0, (sx - 16, sy), (sx, sy - 8)),
                (0, 1, (sx, sy + 8), (sx - 16, sy)),
                (0, -1, (sx, sy - 8), (sx + 16, sy)),
            ] {
                if standable(map.terrain(x + dx, y + dy)) || world.is_crusted(x + dx, y + dy) {
                    out.push((a, b, visible));
                }
            }
        }
    }
    out
}

fn dim(c: Color, visible: bool) -> Color {
    if visible {
        c
    } else {
        crate::chart_surface::memory_color(c)
    }
}

/// Lay the booms.  Call after the ground and shores, before machines.
pub fn draw(canvas: &mut Canvas, world: &World, camera: Camera) {
    let view = (canvas.width() as i32, canvas.height() as i32);
    let edges = edges(world, camera, view);
    let mut posts: BTreeSet<(i32, i32, bool)> = BTreeSet::new();
    for &(a, b, visible) in &edges {
        // Two timbers side by side ride the water along the edge.
        canvas.line(a.0, a.1 + 1, b.0, b.1 + 1, dim(TIMBER_SHADOW, visible));
        canvas.line(a.0, a.1, b.0, b.1, dim(TIMBER, visible));
        canvas.line(a.0, a.1 - 1, b.0, b.1 - 1, dim(TIMBER_LIGHT, visible));
        posts.insert((a.0, a.1, visible));
        posts.insert((b.0, b.1, visible));
        posts.insert(((a.0 + b.0) / 2, (a.1 + b.1) / 2, visible));
    }
    // Marker posts: dark poles banded red and white at the top.
    for (x, y, visible) in posts {
        canvas.rect(x - 1, y - 9, 2, 10, dim(POST, visible));
        canvas.rect(x - 1, y - 9, 2, 2, dim(BAND_RED, visible));
        canvas.rect(x - 1, y - 7, 2, 2, dim(BAND_WHITE, visible));
        canvas.rect(x - 1, y - 5, 2, 2, dim(BAND_RED, visible));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;

    #[test]
    fn a_deep_lane_is_walled_at_its_banks_and_a_shallow_one_is_not() {
        let mut world = World::new(5, Faction::Union);
        world.revealed = true;
        // A camera far up and left keeps every cell on the positive side.
        let camera = Camera { x: -3000, y: -100 };
        assert!(edges(&world, camera, (100_000, 100_000)).is_empty());
        world.gate.tide = bw_sim::Tide::Flood;
        assert!(lane_closed(&world, 0) && lane_closed(&world, 1));
        let walls = edges(&world, camera, (100_000, 100_000));
        // Both lanes meet both banks and the island.
        assert!(walls.len() >= 20, "{}", walls.len());
    }

    #[test]
    fn each_confluence_arm_closes_on_its_own() {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .unwrap();
        assert!((0..3).all(|arm| !lane_closed(&world, arm)));
        world.gate.tide = bw_sim::Tide::Open;
        for dry in 0..3u8 {
            world.gate.dry_arm = bw_sim::Arm(dry);
            for arm in 0..3 {
                assert_eq!(lane_closed(&world, arm), arm != usize::from(dry));
            }
        }
        world.gate.tide = bw_sim::Tide::Flood;
        assert!((0..3).all(|arm| lane_closed(&world, arm)));
        assert!(!lane_closed(&world, 3), "no fourth arm");
        let two = World::new(1, Faction::Union);
        assert!(!lane_closed(&two, 2), "the Split Basin has two arms");
    }
}
