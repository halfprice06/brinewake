//! Last-seen buildings as chart observations. No current hidden position is used.
use crate::canvas::{Atlas, Canvas, INK, MUTED};
use bw_core::{Camera, FP, Pos};
use bw_sim::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Negative observations are presentation history too: after seeing an empty
/// former site, do not resurrect its old drawing when the scout leaves.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryState {
    pub cleared: BTreeMap<u32, u64>,
    /// Ground this seat has ever had in sight.
    #[serde(default)]
    pub explored: Explored,
}

impl MemoryState {
    pub fn observe(&mut self, world: &World) {
        for o in world.knowledge(0).stale {
            if world.visible(0, o.pos) {
                self.cleared.insert(o.id, o.last_seen);
            }
        }
        self.explored.observe(world);
        self.sanitize(world);
    }

    pub fn sanitize(&mut self, world: &World) {
        self.cleared
            .retain(|id, tick| *id != 0 && *tick <= world.tick);
        while self.cleared.len() > 512 {
            if let Some(id) = self.cleared.keys().next().copied() {
                self.cleared.remove(&id);
            }
        }
    }
}

/// Rows of the map re-checked each tick for sight that no machine carries:
/// the station's tidal view, the lit mouths and SOUND pings.
const SCAN_ROWS: u16 = 8;

/// Explored ground, one bit per cell.  Presentation history, like the rest
/// of the chart memory: it never enters the world or its hash.  The trial
/// seats could not tell ground they had never seen from ground merely out
/// of sight, and learned it only from a NOT EXPLORED refusal.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Explored {
    pub width: u16,
    pub height: u16,
    bits: Vec<u64>,
    #[serde(skip)]
    scan_row: u16,
}

impl Explored {
    fn fit(&mut self, world: &World) {
        let (w, h) = (world.map.width, world.map.height);
        let words = (usize::from(w) * usize::from(h)).div_ceil(64);
        if self.width != w || self.height != h || self.bits.len() != words {
            *self = Self {
                width: w,
                height: h,
                bits: vec![0; words],
                scan_row: 0,
            };
        }
    }

    /// The memory, once it has watched a field; a memory that never has
    /// (a staged fixture, a menu backdrop) claims nothing is unexplored.
    pub fn known(&self) -> Option<&Self> {
        (self.width > 0 && self.height > 0).then_some(self)
    }

    /// Whether the cell has been in sight.  Off-map is never explored.
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.height) {
            return false;
        }
        let i = y as usize * usize::from(self.width) + x as usize;
        self.bits
            .get(i / 64)
            .is_some_and(|w| w & (1 << (i % 64)) != 0)
    }

    fn set(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.height) {
            return;
        }
        let i = y as usize * usize::from(self.width) + x as usize;
        if let Some(w) = self.bits.get_mut(i / 64) {
            *w |= 1 << (i % 64);
        }
    }

    fn check(&mut self, world: &World, x: i32, y: i32) {
        if !self.get(x, y) && world.visible(0, Pos::cell(x, y)) {
            self.set(x, y);
        }
    }

    /// Mark what is in sight now.  Every machine's own sight square is
    /// checked each tick; a few rows of the whole map are swept in turn for
    /// the sight that belongs to no machine.  An observer's lifted fog
    /// explores nothing.
    pub fn observe(&mut self, world: &World) {
        self.fit(world);
        if world.revealed || self.width == 0 || self.height == 0 {
            return;
        }
        for e in &world.entities {
            if e.owner != 0 || e.hp <= 0 || e.build_remaining > 0 || e.aboard.is_some() {
                continue;
            }
            let reach = world.sight_of(e) / FP + 1;
            let (cx, cy) = e.pos.cell_xy();
            for y in (cy - reach).max(0)..=(cy + reach).min(i32::from(self.height) - 1) {
                for x in (cx - reach).max(0)..=(cx + reach).min(i32::from(self.width) - 1) {
                    self.check(world, x, y);
                }
            }
        }
        let start = self.scan_row % self.height;
        for y in start..(start + SCAN_ROWS).min(self.height) {
            for x in 0..i32::from(self.width) {
                self.check(world, x, i32::from(y));
            }
        }
        self.scan_row = (start + SCAN_ROWS) % self.height;
    }
}

pub fn draw(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    world: &World,
    camera: Camera,
    state: &MemoryState,
) {
    let Some(atlas) = atlas else { return };
    let view_width = i32::try_from(canvas.width()).expect("Canvas width fits i32");
    let view_height = i32::try_from(canvas.height()).expect("Canvas height fits i32");
    let mut observations = world.knowledge(0).stale;
    observations.sort_by_key(|o| (i64::from(o.pos.x) + i64::from(o.pos.y), o.id));
    for o in observations {
        if usize::from(o.owner) >= world.players.len()
            || !o.kind.is_building()
            || world.visible(0, o.pos)
            || state
                .cleared
                .get(&o.id)
                .is_some_and(|tick| *tick >= o.last_seen)
        {
            continue;
        }
        let faction = world.players[usize::from(o.owner)].faction;
        let transform = crate::occlusion::render_transform(o.kind, o.pos, faction);
        let (px, py) = camera.project(transform.anchor);
        let (x, y) = (
            px + transform.render_x_offset,
            py + transform.render_y_offset,
        );
        let key = o.kind.asset(faction);
        let Some(sprite) = atlas.sprites.get(key) else {
            continue;
        };
        let n = u32::from(transform.scale.numerator);
        let d = u32::from(transform.scale.denominator);
        let width = (sprite.w * n).div_ceil(d);
        let height = (sprite.h * n).div_ceil(d);
        let left = x - transform.scale.apply(sprite.anchor_x);
        let top = y - transform.scale.apply(sprite.anchor_y);
        if left >= view_width
            || left + width as i32 <= 0
            || top >= view_height
            || top + height as i32 <= 0
        {
            continue;
        }
        for sy in 0..height {
            let py = top + sy as i32;
            if !(0..view_height).contains(&py) {
                continue;
            }
            for sx in 0..width {
                let px = left + sx as i32;
                if !(0..view_width).contains(&px) {
                    continue;
                }
                let tx = sx * d / n;
                let ty = sy * d / n;
                if let Some(c) = atlas.sample(sprite, tx, ty).filter(|c| c[3] != 0) {
                    let mut ink = crate::chart_surface::memory_color(c);
                    // A quiet printed silhouette belongs to the survey chart;
                    // horizontal scanlines made it look like a live hologram.
                    let edge = tx == 0
                        || ty == 0
                        || tx + 1 == sprite.w
                        || ty + 1 == sprite.h
                        || [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)]
                            .into_iter()
                            .any(|(nx, ny)| atlas.sample(sprite, nx, ny).is_none_or(|p| p[3] == 0));
                    ink[3] = if edge { 165 } else { 65 };
                    canvas.pixel(px, py, ink);
                }
            }
        }
        let seconds = o.last_seen / 30;
        let label = format!("LAST SEEN {}:{:02}", seconds / 60, seconds % 60);
        let width = label.len() as i32 * 6;
        let lx = (x - width / 2).clamp(0, (view_width - width).max(0));
        let ly = y + 7;
        if (0..view_height.saturating_sub(10)).contains(&ly) {
            canvas.rect(lx - 2, ly - 1, width + 4, 9, INK);
            canvas.text(&label, lx, ly, MUTED);
        }
    }
}
