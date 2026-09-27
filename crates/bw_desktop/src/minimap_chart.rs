//! The minimap drawn as the basin chart it is named for.
//!
//! A parchment field with an ink coastline, hatched deep water, the two lane
//! marks filled by their public state, wells, wreck beds and the sluice
//! symbol.  Fog is a darker, aged wash rather than a blackout so the chart
//! stays a chart.  Shared by the native and the legacy 640x360 minimaps; the
//! callers add their own alerts, markers and the camera fix.

use crate::canvas::{Canvas, Color, GOLD};
use bw_core::{FP, Pos, Terrain};
use bw_sim::World;

pub const PAPER: Color = [190, 183, 155, 255];
pub const PAPER_AGED: Color = [137, 132, 119, 255];
pub const PAPER_LIT: Color = [231, 217, 181, 255];
pub const INK: Color = [30, 48, 63, 255];
pub const INK_SOFT: Color = [85, 114, 122, 255];
pub const WATER: Color = [77, 124, 130, 255];
pub const WATER_BAND: Color = [48, 91, 102, 255];
pub const ROCK: Color = [96, 88, 78, 255];
pub const SILT: Color = [130, 118, 101, 255];
/// The paper around a diamond chart, darker than any charted ground.
pub const MARGIN: Color = [112, 106, 94, 255];

/// Chart geometry: where map cell `(u, v)` lands inside the rectangle.
///
/// The field camera looks at the square map across its diagonal, so on
/// screen the map is a two-to-one diamond: north runs up and to the right.
/// A `field` chart draws the map in that same shape, which makes the camera's
/// footprint a plain rectangle and a chart click land where the eye expects.
/// An `axis` chart lays the map flat, west to the left and north up, for the
/// observer's whole-map picture.
#[derive(Clone, Copy, Debug)]
pub struct ChartRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub diamond: bool,
}

impl ChartRect {
    /// The flat, north-up chart filling the rectangle.
    pub fn axis(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            diamond: false,
        }
    }
    /// The largest two-to-one diamond that fits the bounds, centred in them.
    /// Its box has even sides, so the middle pixel is the middle of the map.
    pub fn field(x: i32, y: i32, w: i32, h: i32) -> Self {
        let dh = (w / 2).min(h) & !1;
        let dw = 2 * dh;
        Self {
            x: x + (w - dw) / 2,
            y: y + (h - dh) / 2,
            w: dw,
            h: dh,
            diamond: true,
        }
    }
    fn map_size(world: &World) -> (i64, i64) {
        (
            i64::from(world.map.width.max(1)),
            i64::from(world.map.height.max(1)),
        )
    }
    pub fn plot(&self, world: &World, pos: Pos) -> (i32, i32) {
        if !self.diamond {
            let (u, v) = pos.cell_xy();
            return (
                self.x + u * self.w / i32::from(world.map.width.max(1)),
                self.y + v * self.h / i32::from(world.map.height.max(1)),
            );
        }
        let (mw, mh) = Self::map_size(world);
        let fp = i64::from(FP);
        let (px, py) = (i64::from(pos.x), i64::from(pos.y));
        let span = (mw + mh) * fp;
        (
            self.x + ((px - py + mh * fp) * i64::from(self.w)).div_euclid(span) as i32,
            self.y + ((px + py) * i64::from(self.h)).div_euclid(span) as i32,
        )
    }
    /// The map point under a chart pixel, unclamped: it may lie off the map
    /// when the pixel is outside the diamond.
    fn point_at(&self, world: &World, x: i32, y: i32) -> (i64, i64) {
        let (mw, mh) = Self::map_size(world);
        let fp = i64::from(FP);
        let (dx, dy) = (i64::from(x - self.x), i64::from(y - self.y));
        if !self.diamond {
            return (
                dx * mw * fp / i64::from(self.w.max(1)),
                dy * mh * fp / i64::from(self.h.max(1)),
            );
        }
        let span = (mw + mh) * fp;
        let a = (dx * span).div_euclid(i64::from(self.w.max(1))) - mh * fp;
        let b = (dy * span).div_euclid(i64::from(self.h.max(1)));
        ((a + b).div_euclid(2), (b - a).div_euclid(2))
    }
    /// The map cell under a chart pixel, or `None` off the map.
    pub fn cell_at(&self, world: &World, x: i32, y: i32) -> Option<(i32, i32)> {
        let (mw, mh) = Self::map_size(world);
        let fp = i64::from(FP);
        let (px, py) = self.point_at(world, x, y);
        ((0..mw * fp).contains(&px) && (0..mh * fp).contains(&py))
            .then(|| ((px / fp) as i32, (py / fp) as i32))
    }
    /// The map point under a chart pixel, pulled onto the map: a click on
    /// the chart's margin goes to the nearest edge.
    pub fn pos_at(&self, world: &World, x: i32, y: i32) -> Pos {
        let (mw, mh) = Self::map_size(world);
        let fp = i64::from(FP);
        let (px, py) = self.point_at(world, x, y);
        Pos::raw(
            px.clamp(0, mw * fp - 1) as i32,
            py.clamp(0, mh * fp - 1) as i32,
        )
    }
    pub fn clamp(&self, x: i32, y: i32, margin: i32) -> (i32, i32) {
        (
            x.clamp(self.x + margin, self.x + self.w - margin - 1),
            y.clamp(self.y + margin, self.y + self.h - margin - 1),
        )
    }
}

fn shade(c: Color, pct: u16) -> Color {
    let mut out = c;
    for ch in out.iter_mut().take(3) {
        *ch = (u16::from(*ch) * pct / 100).min(255) as u8;
    }
    out
}

/// Paper colour for one chart pixel from its map cell and neighbours.
fn field_color(world: &World, u: i32, v: i32, px: i32, py: i32) -> Color {
    let t = world.map.terrain(u, v);
    let depth = world.depth_at(u, v);
    match t {
        Terrain::Deep => {
            // Soundings: a sparse diagonal hatch on paper, denser at the shore.
            let near_land = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| world.map.terrain(u + dx, v + dy) != Terrain::Deep);
            let period = if near_land { 3 } else { 5 };
            if (px + py).rem_euclid(period) == 0 {
                INK_SOFT
            } else {
                PAPER
            }
        }
        // Deep tidal water is sounded like the lake: a wall on the chart.
        _ if depth == Some(bw_sim::Depth::Deep) => {
            if (px + py).rem_euclid(3) == 0 {
                INK_SOFT
            } else {
                PAPER
            }
        }
        _ if depth == Some(bw_sim::Depth::Shallow) => {
            if (px / 2 + py).rem_euclid(2) == 0 {
                WATER
            } else {
                WATER_BAND
            }
        }
        Terrain::Lane0 | Terrain::Lane1 | Terrain::Lane2 => {
            if (px + py).rem_euclid(2) == 0 {
                PAPER_LIT
            } else {
                PAPER
            }
        }
        Terrain::Rim1 | Terrain::Rim0 | Terrain::Rim2 => {
            if py.rem_euclid(2) == 0 {
                PAPER_AGED
            } else {
                PAPER
            }
        }
        Terrain::Rock => ROCK,
        Terrain::Silt => {
            if (px + py).rem_euclid(2) == 0 {
                SILT
            } else {
                PAPER
            }
        }
        Terrain::Salt => PAPER_LIT,
    }
}

/// Draw the chart field: paper, coastline, lanes, wells, beds, sluice and
/// the visible machines.  Returns nothing; callers layer their marks on top.
pub fn draw_field(canvas: &mut Canvas, world: &World, r: ChartRect, s: i32) {
    let map = &world.map;
    for y in 0..r.h {
        for x in 0..r.w {
            // Off the diamond is the chart's margin: the caller's panel shows.
            let Some((u, v)) = r.cell_at(world, r.x + x, r.y + y) else {
                continue;
            };
            let mut c = field_color(world, u, v, x, y);
            // Coastline ink where land meets deep water in the sampled grid.
            // The crossings are not coast: their fill carries the lane state.
            if !matches!(
                map.terrain(u, v),
                Terrain::Deep | Terrain::Lane0 | Terrain::Lane1 | Terrain::Lane2
            ) {
                let coast = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    r.cell_at(world, r.x + x + dx, r.y + y + dy)
                        .is_some_and(|(nu, nv)| {
                            (nu, nv) != (u, v) && map.terrain(nu, nv) == Terrain::Deep
                        })
                });
                if coast {
                    c = INK;
                }
            }
            if !world.visible(0, Pos::cell(u, v)) {
                c = shade(c, 62);
            }
            canvas.pixel(r.x + x, r.y + y, c);
        }
    }
    // Wells and wreck beds are map features, not intelligence: the chart
    // names where they are at all times so a player can plan gathering and
    // find a condenser site.  Sonar still adds what is left in each.
    for well in &map.wells {
        let (x, y) = r.plot(world, *well);
        well_mark(canvas, x, y, s);
    }
    for bed in &map.resources {
        // The map's own beds are geography and always inked; wreckage a
        // fight left is only inked where the player can see it, so the
        // chart never reports a battle it did not witness.
        if bed.scrap && !world.visible(0, bed.pos) {
            continue;
        }
        let (x, y) = r.plot(world, bed.pos);
        wreck_mark(canvas, x, y, s);
    }
    for e in &world.entities {
        if e.hp <= 0 || !(e.owner == 0 || world.entity_visible(0, e.id)) {
            continue;
        }
        // Marks carry an ink rim so they read on the paper and in a small
        // picture; enemies are one step larger than own machines.
        let (x, y) = r.plot(world, e.pos);
        let size = if e.kind.is_building() {
            4
        } else if e.owner == 0 {
            2
        } else {
            3
        };
        canvas.rect(x - s, y - s, (size + 2) * s, (size + 2) * s, INK);
        canvas.rect(
            x,
            y,
            size * s,
            size * s,
            crate::seats::seat_colour(world, e.owner),
        );
    }
    // The sluice: a framed amber square on the spine, lane letters beside
    // each crossing, and a gold frame on the committed target during the
    // public warning.
    let (gx, gy) = r.plot(world, map.gate_pos);
    canvas.rect(gx - s, gy - s, 3 * s, 3 * s, GOLD);
    canvas.frame(gx - 2 * s, gy - 2 * s, 5 * s, 5 * s, INK);
    if map.id != bw_sim::MapId::SplitBasin {
        draw_arm_letters(canvas, world, r, s);
        return;
    }
    // Lane letters sit on the paper just east of each cut's east mouth,
    // where ink reads over either lane state.
    let north = r.plot(world, Pos::cell(64, 49));
    let south = r.plot(world, Pos::cell(64, 79));
    if r.diamond {
        for (letter, row) in [("N", 49), ("S", 79)] {
            let (x, y) = r.plot(world, Pos::cell(86, row));
            canvas.text(letter, x - 2, y - 3, INK);
        }
    } else {
        let label_x = r.plot(world, Pos::cell(78, 64)).0;
        canvas.text("N", label_x, north.1 - 3, INK);
        canvas.text("S", label_x, south.1 - 3, INK);
    }
    if let Some(target) = world.gate.switch_target.map(bw_sim::Arm::is_north)
        && world.gate.warning_until.is_some()
    {
        let (tx, ty) = if target { north } else { south };
        canvas.frame(tx - 3 * s, ty - 3 * s, 7 * s, 7 * s, GOLD);
    }
    if world.gate.flood_pending && world.gate.warning_until.is_some() {
        for (tx, ty) in [north, south] {
            canvas.frame(tx - 3 * s, ty - 3 * s, 7 * s, 7 * s, GOLD);
        }
    }
}

/// Every arm's letter on a map other than the Split Basin, inked on the
/// middle of its lane, and the gold frame on the committed target (or on
/// every arm before a flood) during the public warning.
fn draw_arm_letters(canvas: &mut Canvas, world: &World, r: ChartRect, s: i32) {
    let layout = world.map.layout();
    let warning = world.gate.warning_until.is_some();
    for (arm, info) in layout.arms.iter().enumerate() {
        let (x, y) = r.plot(world, Pos::cell(info.centre.0, info.centre.1));
        let letter = crate::seats::arm_letter(world, arm);
        // A paper patch under the letter so it reads over any lane state.
        canvas.rect(x - 3, y - 4, 7, 9, PAPER_LIT);
        canvas.text(letter, x - 2, y - 3, INK);
        let targeted = world.gate.switch_target.map(bw_sim::Arm::index) == Some(arm)
            || world.gate.flood_pending
            || (world.gate.ebb_pending && world.gate.dry_arm.index() == arm);
        if warning && targeted {
            canvas.frame(x - 3 * s, y - 3 * s, 7 * s, 7 * s, GOLD);
        }
    }
}

/// The margin of a diamond chart: aged paper in the four corners the map
/// leaves free, an ink rule along the map's edge, a key to the marks both
/// seats failed to read (wells, wreck beds, crossing mouths) and a compass
/// whose north points where north lies on the field.  `bounds` is the whole
/// minimap rectangle; `r` the diamond inside it.
pub fn draw_margin(canvas: &mut Canvas, world: &World, bounds: ChartRect, r: ChartRect, s: i32) {
    if !r.diamond {
        return;
    }
    for y in bounds.y..bounds.y + bounds.h {
        for x in bounds.x..bounds.x + bounds.w {
            if r.cell_at(world, x, y).is_some() {
                continue;
            }
            let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| r.cell_at(world, x + dx, y + dy).is_some());
            canvas.pixel(x, y, if edge { INK } else { MARGIN });
        }
    }
    let line = 9 * s;
    // Top left: wells.
    let (x, y) = (bounds.x + 2 * s, bounds.y + 2 * s);
    key_swatch(canvas, x, y, s);
    well_mark(canvas, x + 3 * s, y + 3 * s, s);
    canvas.text_scaled("WELL", x + 9 * s, y, INK, s);
    // Bottom left: wreck beds.
    let (x, y) = (bounds.x + 2 * s, bounds.y + bounds.h - line);
    key_swatch(canvas, x, y, s);
    wreck_mark(canvas, x + 3 * s, y + 3 * s, s);
    canvas.text_scaled("WRECK", x + 9 * s, y, INK, s);
    // Bottom right: lane banks, where a hold is kept.
    let label = "BANK";
    let lw = (label.len() as i32 * 6 - 1) * s;
    let (x, y) = (
        bounds.x + bounds.w - 2 * s - lw - 9 * s,
        bounds.y + bounds.h - line,
    );
    key_swatch(canvas, x, y, s);
    let (cx, cy) = (x + 3 * s, y + 3 * s);
    canvas.line(cx - 2 * s, cy, cx, cy - 2 * s, INK);
    canvas.line(cx, cy - 2 * s, cx + 2 * s, cy, INK);
    canvas.line(cx + 2 * s, cy, cx, cy + 2 * s, INK);
    canvas.line(cx, cy + 2 * s, cx - 2 * s, cy, INK);
    canvas.text_scaled(label, x + 9 * s, y, INK, s);
    // Three arms: each lane wears its letter where it runs, in place of
    // the compass (trial 10: a chart pointing N beside lanes called E, W
    // and S read as a fourth direction).
    if world.map.layout().arm_count() > 2 {
        for (arm, lane) in world.map.layout().arms.iter().enumerate() {
            let (px, py) = r.plot(world, Pos::cell(lane.centre.0, lane.centre.1));
            let letter = crate::seats::arm_letter(world, arm);
            key_swatch(canvas, px - 3 * s, py - 3 * s, s);
            canvas.text_scaled(letter, px - 2 * s, py - 3 * s, INK, s);
        }
        return;
    }
    // Top right: the compass.  North on the field runs up the diamond's
    // north-east edge, two across for one up; east runs down the other.
    let (cx, cy) = (bounds.x + bounds.w - 24 * s, bounds.y + 12 * s);
    // East to west, faint: down and to the right is east.  The small
    // window's chart has no room for it beside the diamond.
    if s > 1 {
        canvas.line(cx - 6 * s, cy - 3 * s, cx + 6 * s, cy + 3 * s, INK_SOFT);
        canvas.text_scaled("E", cx + 8 * s, cy + 3 * s, INK_SOFT, s);
    }
    // South to north, doubled, with a head on the north end.
    for d in 0..s.max(1) {
        canvas.line(cx - 8 * s, cy + 4 * s + d, cx + 8 * s, cy - 4 * s + d, INK);
    }
    canvas.line(cx + 8 * s, cy - 4 * s, cx + 4 * s, cy - 4 * s, INK);
    canvas.line(cx + 8 * s, cy - 4 * s, cx + 7 * s, cy - s, INK);
    canvas.text_scaled("N", cx + 10 * s, cy - 11 * s, INK, s);
}

/// A well: a pool of water ringed in ink, whole cells at the chart's scale.
fn well_mark(canvas: &mut Canvas, x: i32, y: i32, s: i32) {
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        canvas.rect(x + dx * s, y + dy * s, s, s, INK);
    }
    canvas.rect(x, y, s, s, WATER);
}

/// A wreck bed: a cross of rust-dark cells, the chart's X for salvage.
fn wreck_mark(canvas: &mut Canvas, x: i32, y: i32, s: i32) {
    for d in -1..=1 {
        canvas.rect(x + d * s, y + d * s, s, s, ROCK);
        canvas.rect(x + d * s, y - d * s, s, s, ROCK);
    }
}

/// A small square of chart paper framed in ink: the ground a key mark sits on.
fn key_swatch(canvas: &mut Canvas, x: i32, y: i32, s: i32) {
    canvas.rect(x, y, 7 * s, 7 * s, PAPER);
    canvas.frame(
        x - s.min(1),
        y - s.min(1),
        7 * s + 2 * s.min(1),
        7 * s + 2 * s.min(1),
        INK_SOFT,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;

    fn count(canvas: &Canvas, r: ChartRect, c: Color) -> usize {
        let mut n = 0;
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                if canvas.get(x, y) == Some(c) {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn chart_is_paper_with_ink_and_reads_only_public_state() {
        let world = World::new(3, Faction::Union);
        let r = ChartRect::axis(10, 10, 140, 74);
        let mut canvas = Canvas::default();
        draw_field(&mut canvas, &world, r, 1);
        assert!(count(&canvas, r, INK) > 40, "coastline ink present");
        let paper = count(&canvas, r, PAPER_LIT) + count(&canvas, r, shade(PAPER_LIT, 62));
        assert!(paper > r.w as usize * r.h as usize / 4, "paper dominates");
        // A hidden enemy that moves does not change the chart.
        let mut moved = world.clone();
        if let Some(enemy) = moved
            .entities
            .iter_mut()
            .find(|e| e.owner == 1 && !e.kind.is_building())
        {
            enemy.pos = Pos::cell(100, 100);
        }
        let mut again = Canvas::default();
        draw_field(&mut again, &moved, r, 1);
        assert_eq!(canvas.pixels, again.pixels);
    }

    #[test]
    fn lane_fill_follows_the_public_gate_state() {
        let mut world = World::new(3, Faction::Union);
        let r = ChartRect::axis(0, 0, 140, 74);
        let mut before = Canvas::default();
        draw_field(&mut before, &world, r, 1);
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(false);
        let mut after = Canvas::default();
        draw_field(&mut after, &world, r, 1);
        assert_ne!(before.pixels, after.pixels);
        // The north side is deep now: sounded like the lake, not shallow water.
        let north = r.plot(&world, Pos::cell(60, 49));
        let deep = |c: Option<Color>| matches!(c, Some(c) if c == INK_SOFT || c == PAPER || c == shade(INK_SOFT, 62) || c == shade(PAPER, 62));
        assert!(deep(after.get(north.0, north.1)));
        // At the neutral tide both lanes are shallow water.
        assert!(
            matches!(before.get(north.0, north.1), Some(c) if c == WATER || c == WATER_BAND || c == shade(WATER, 62) || c == shade(WATER_BAND, 62))
        );
    }
}
