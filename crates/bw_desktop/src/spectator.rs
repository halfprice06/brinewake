//! Watching a recording: a bar with both sides' hold gauges, army value,
//! income and crew; a camera that follows the biggest fight or a running
//! hold count; a replay timeline of the sluice and the counts that seeks
//! when clicked; and a kill feed in the field's top-right corner.
//!
//! Presentation only. It reads the fog-lifted playback world and never
//! issues an order.

use crate::canvas::{EDGE, GOLD, INK, MUTED, WHITE};
use crate::game::Game;
use crate::match_summary::{MatchSummary, clock, of, ordinal, place_out};
use crate::native_ui::{Rect, small, text};
use crate::occlusion::PixelScale;
use bw_core::{Camera, Kind, Pos};
use bw_sim::{EventKind, World};
use std::collections::HashMap;

/// Fighting counts toward the follow camera for this long.
const HEAT_TICKS: u64 = 150;
/// Cells around a fight that belong to it.
const FIGHT_CELLS: i32 = 10;
/// A new fight must be this much bigger than the watched one to take the
/// camera from it.
const SWITCH_PERCENT: u32 = 150;
/// The keys that skip the replay back and forward.
pub const SKIP_TICKS: u64 = 900;
/// How long the banner saying a seat is out stands: ten seconds.
pub const OUT_BANNER_TICKS: u64 = 300;
/// How long a kill stands in the feed: six seconds, the last fading out.
pub const KILL_TICKS: u64 = 180;
/// The fade at the end of a kill's time.
const KILL_FADE_TICKS: u64 = 45;
/// Kills the feed shows at once, the newest on top.
pub const KILL_ROWS: usize = 4;
/// The same kill again this soon joins its row as a count.
const KILL_MERGE_TICKS: u64 = 30;

/// One row of the kill feed: who killed what, both as (seat, kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kill {
    pub tick: u64,
    pub killer: (u8, Kind),
    pub victim: (u8, Kind),
    /// How many of the same kill landed together.
    pub count: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Spectator {
    /// The camera follows the action.
    pub follow: bool,
    /// Recent damage and deaths: tick, place, weight.
    heat: Vec<(u64, Pos, u32)>,
    /// Where the camera is heading and why.
    pub focus: Option<(Pos, &'static str)>,
    /// The timeline as last drawn, for clicks.
    pub timeline: Option<Rect>,
    /// The follow switch as last drawn, for clicks.
    pub follow_switch: Option<Rect>,
    /// The kill feed, oldest first.
    pub kills: Vec<Kill>,
    /// Every machine and building as of the last tick, (seat, kind) by id:
    /// a death names what died after it has left the world, and its killer
    /// may have fallen in the same exchange.
    known: HashMap<u32, (u8, Kind)>,
}

impl Spectator {
    /// Take in one tick of the watched match.
    pub fn observe(&mut self, world: &World) {
        for event in &world.events {
            let weight = match event.kind {
                EventKind::Damage => event.amount.max(1) as u32,
                EventKind::Death => 60,
                _ => continue,
            };
            if let Some(pos) = event.to.or(event.from) {
                self.heat.push((event.tick, pos, weight));
            }
        }
        self.heat
            .retain(|(tick, ..)| world.tick.saturating_sub(*tick) <= HEAT_TICKS);
        self.observe_kills(world);
    }

    /// Take the tick's deaths into the kill feed: a seat's machine or
    /// building killed by another seat's. Deaths with no killer (the
    /// tide, a sunk transport's riders) are not kills.
    fn observe_kills(&mut self, world: &World) {
        let seats = world.seat_count();
        let lookup = |known: &HashMap<u32, (u8, Kind)>, id: u32| {
            world
                .entities
                .iter()
                .find(|e| e.id == id)
                .map(|e| (e.owner, e.kind))
                .or_else(|| known.get(&id).copied())
        };
        for event in &world.events {
            if event.kind != EventKind::Death {
                continue;
            }
            let Some(victim) = event.entity.and_then(|id| lookup(&self.known, id)) else {
                continue;
            };
            let Some((killer_seat, killer_kind)) =
                event.other.and_then(|id| lookup(&self.known, id))
            else {
                continue;
            };
            let killer = (killer_seat, event.cause.unwrap_or(killer_kind));
            if usize::from(victim.0) >= seats
                || usize::from(killer.0) >= seats
                || killer.0 == victim.0
            {
                continue;
            }
            match self.kills.last_mut() {
                Some(last)
                    if last.killer == killer
                        && last.victim == victim
                        && world.tick.saturating_sub(last.tick) <= KILL_MERGE_TICKS =>
                {
                    last.count += 1;
                    last.tick = world.tick;
                }
                _ => self.kills.push(Kill {
                    tick: world.tick,
                    killer,
                    victim,
                    count: 1,
                }),
            }
        }
        self.kills
            .retain(|kill| world.tick.saturating_sub(kill.tick) < KILL_TICKS);
        if self.kills.len() > KILL_ROWS {
            self.kills.drain(..self.kills.len() - KILL_ROWS);
        }
        self.known.clear();
        self.known.extend(
            world
                .entities
                .iter()
                .filter(|e| e.hp > 0)
                .map(|e| (e.id, (e.owner, e.kind))),
        );
    }

    /// Forget the fighting: after a seek it belongs to another moment.
    pub fn clear(&mut self) {
        self.heat.clear();
        self.focus = None;
        self.kills.clear();
        self.known.clear();
    }

    fn heat_near(&self, pos: Pos) -> u32 {
        let radius = i64::from(FIGHT_CELLS * bw_core::FP).pow(2);
        self.heat
            .iter()
            .filter(|(_, at, _)| at.distance_sq(pos) <= radius)
            .map(|(.., weight)| *weight)
            .sum()
    }

    /// The biggest fight: the heat point with the most heat around it, and
    /// the weighted centre of that heat.
    fn biggest_fight(&self) -> Option<(Pos, u32)> {
        let radius = i64::from(FIGHT_CELLS * bw_core::FP).pow(2);
        let (_, centre, score) = self
            .heat
            .iter()
            .map(|(_, pos, _)| (*pos, *pos, self.heat_near(*pos)))
            .max_by_key(|(pos, _, score)| (*score, pos.x, pos.y))?;
        let (mut sx, mut sy, mut sw) = (0i64, 0i64, 0i64);
        for (_, at, weight) in &self.heat {
            if at.distance_sq(centre) <= radius {
                sx += i64::from(at.x) * i64::from(*weight);
                sy += i64::from(at.y) * i64::from(*weight);
                sw += i64::from(*weight);
            }
        }
        let sw = sw.max(1);
        Some((Pos::raw((sx / sw) as i32, (sy / sw) as i32), score))
    }

    /// Choose where the camera should be: the biggest fight, unless a hold
    /// count is running and nothing is being fought, in which case the
    /// crossing mouth the count depends on.
    pub fn choose_focus(&mut self, world: &World) -> Option<Pos> {
        let fight = self.biggest_fight();
        let current = self.focus.map(|(pos, _)| (pos, self.heat_near(pos)));
        if let Some((pos, score)) = fight {
            let keep = current.is_some_and(|(_, held)| {
                held > 0 && u64::from(score) * 100 <= u64::from(held) * u64::from(SWITCH_PERCENT)
            });
            if !keep {
                self.focus = Some((pos, "FIGHT"));
            }
            return self.focus.map(|(pos, _)| pos);
        }
        if let Some(player) = counting_seat(world) {
            // The count's weak point is the mouth, of the lanes it needs,
            // with the most opponents' machines near it: that is where it
            // will be broken.
            let seats = world.seat_count();
            let radius = i64::from(12 * bw_core::FP).pow(2);
            let mouths = world.crossing_mouths();
            let arms = world.arms_of(player);
            let mouth = arms
                .iter()
                .filter(|arm| world.holds_lane(player, **arm))
                .filter_map(|arm| mouths.get(*arm))
                .flatten()
                .copied()
                .max_by_key(|mouth| {
                    world
                        .entities
                        .iter()
                        .filter(|e| {
                            e.owner != player
                                && usize::from(e.owner) < seats
                                && e.hp > 0
                                && !e.kind.is_building()
                                && e.pos.distance_sq(*mouth) <= radius
                        })
                        .count()
                })
                .or_else(|| {
                    arms.first()
                        .and_then(|arm| mouths.get(*arm))
                        .map(|pair| pair[0])
                })
                .unwrap_or(world.map.gate_pos);
            self.focus = Some((mouth, "COUNT"));
            return Some(mouth);
        }
        self.focus.map(|(pos, _)| pos)
    }
}

/// The seat whose hold count the camera watches: with two seats the first
/// with a running count, with three the fullest (the lower seat on a tie).
fn counting_seat(world: &World) -> Option<u8> {
    let gauge = |seat: u8| of(&world.lane_hold, seat);
    if world.seat_count() <= 2 {
        world.seats().find(|&seat| gauge(seat) > 0)
    } else {
        world
            .seats()
            .filter(|&seat| gauge(seat) > 0)
            .max_by_key(|&seat| (gauge(seat), std::cmp::Reverse(seat)))
    }
}

/// A kill row's width: two tiles and the chevron between.
fn kill_row_width(k: i32) -> i32 {
    2 * (24 * k + 2 * k) + 10 * k
}

/// How far the camera may go on a map, as (min x, max x, min y, max y): a
/// cell is 16 pixels across and 8 down in the projection, and the bounds
/// leave a margin past the corners. The 128-cell Split Basin gives the
/// old -2400..2400 by -200..2400.
pub(crate) fn camera_bounds(world: &World) -> (i32, i32, i32, i32) {
    let [left, right, top, bottom] =
        crate::game::camera_limits(world.map.width.max(world.map.height));
    (left, right, top, bottom)
}

impl Game {
    /// The top band is taller while watching: the spectator bar and the
    /// replay timeline sit in it, never over the field.
    pub(crate) fn spectator_band(&self) -> bool {
        self.observing()
    }

    /// This seat was knocked out of a match of three that goes on without
    /// it: it watches the rest with the fog lifted, as an observer would.
    pub(crate) fn knocked_out(&self) -> bool {
        self.playback.is_none() && self.world.outcome.is_none() && self.world.is_eliminated(0)
    }

    /// Watching rather than playing: a recording, or a seat knocked out.
    /// The seat's own header, prompts, groups and command card give way to
    /// the spectator band and the observer's dock.
    pub(crate) fn observing(&self) -> bool {
        self.playback.is_some() || self.knocked_out()
    }

    /// The tick this seat went out, while it is out of a match that goes on.
    pub(crate) fn out_since(&self) -> Option<u64> {
        if !self.knocked_out() {
            return None;
        }
        self.world
            .eliminated
            .iter()
            .find(|(seat, _)| *seat == 0)
            .map(|(_, tick)| *tick)
    }

    /// Once, when this seat goes out: its place large, as a hold count's
    /// seconds are, over what to do now. It stands for OUT_BANNER_TICKS,
    /// then the band's own OUT column says it for the rest of the match.
    pub(crate) fn draw_out_banner(&mut self) {
        let Some(out_at) = self.out_since() else {
            return;
        };
        if self.world.tick.saturating_sub(out_at) >= OUT_BANNER_TICKS {
            return;
        }
        let s = self.ui_scale();
        let place = place_out(&self.world.eliminated, self.world.seat_count(), 0)
            .map_or_else(|| "OUT".to_string(), ordinal);
        let w = self.canvas.width() as i32;
        let bw = 232 * s;
        let r = Rect {
            x: (w - bw) / 2,
            y: self.world_view().top + 8 * s,
            w: bw,
            h: 34 * s,
        };
        self.canvas.rect(r.x, r.y, r.w, r.h, INK);
        for i in 0..s {
            self.canvas
                .frame(r.x + i, r.y + i, r.w - 2 * i, r.h - 2 * i, MUTED);
        }
        self.canvas.rect(r.x, r.y, 4 * s, r.h, MUTED);
        text(
            &mut self.canvas,
            &place,
            r.x + 10 * s,
            r.y + 8 * s,
            WHITE,
            2 * s,
        );
        let x = r.x + 64 * s;
        text(&mut self.canvas, "YOU ARE OUT", x, r.y + 6 * s, WHITE, s);
        small(
            &mut self.canvas,
            "WATCHING THE REST",
            x,
            r.y + 21 * s,
            MUTED,
            s,
        );
    }

    /// The kill feed's corner: the field's top right, under the band,
    /// clear of the centre where the station and the fighting are watched.
    /// (x, y, row pitch, icon scale), rows stacking down from y.
    pub(crate) fn kill_feed_origin(&self) -> (i32, i32, i32, i32) {
        let s = self.ui_scale();
        // Icons at their own 24 pixels up to scale three, doubled above:
        // a glance, not a panel.
        let k = (s / 2).max(1);
        let tile = 24 * k + 2 * k;
        let right = self.canvas.width() as i32 - 8 * s;
        (
            right - kill_row_width(k),
            self.world_view().top + 6 * s,
            tile + 3 * k,
            k,
        )
    }

    /// The kill feed: a row a kill, the killer's icon in its seat's frame,
    /// a chevron, then what it killed in its own seat's frame; a count when
    /// several fell together. Rows fade out over their last second and a
    /// half.
    pub(crate) fn draw_kill_feed(&mut self) {
        if self.spectator.kills.is_empty() {
            return;
        }
        let (x0, y0, pitch, k) = self.kill_feed_origin();
        let tile = 24 * k + 2 * k;
        let row_w = kill_row_width(k);
        let now = self.world.tick;
        let kills: Vec<Kill> = self.spectator.kills.iter().rev().copied().collect();
        let mut row = crate::canvas::Canvas::new(row_w as u32, tile as u32);
        let backing = [INK[0], INK[1], INK[2], 235];
        for (i, kill) in kills.iter().enumerate() {
            let age = now.saturating_sub(kill.tick);
            if age >= KILL_TICKS {
                continue;
            }
            let alpha = if age > KILL_TICKS - KILL_FADE_TICKS {
                (255 * (KILL_TICKS - age) / KILL_FADE_TICKS) as u32
            } else {
                255
            };
            row.clear([0, 0, 0, 0]);
            let mut x = 0;
            for (n, (seat, kind)) in [kill.killer, kill.victim].into_iter().enumerate() {
                let colour = crate::seats::seat_colour(&self.world, seat);
                // The seat's colour, darkened behind the icon and full in
                // the frame.
                let [r, g, b, _] = crate::seats::shade(colour);
                row.rect(x, 0, tile, tile, [r, g, b, 235]);
                for j in 0..k {
                    row.frame(x + j, j, tile - 2 * j, tile - 2 * j, colour);
                }
                let faction = self.world.players[usize::from(seat)].faction;
                let key = if kind.is_building() {
                    format!("ui_build_{}", kind.asset(faction))
                } else {
                    format!("ui_unit_{}", kind.asset(faction))
                };
                if let Some(atlas) = &self.atlas {
                    atlas.draw_scaled(
                        &mut row,
                        &key,
                        x + k,
                        k,
                        false,
                        PixelScale {
                            numerator: k as u16,
                            denominator: 1,
                        },
                    );
                }
                x += tile;
                if n == 0 {
                    // The chevron, in the killer's colour.
                    let cx = x + 3 * k;
                    let cy = tile / 2;
                    for d in 0..4 * k {
                        row.rect(cx + d, cy - 4 * k + d, 2 * k, k, colour);
                        row.rect(cx + d, cy + 4 * k - d - 1, 2 * k, k, colour);
                    }
                    x += 10 * k;
                }
            }
            if kill.count > 1 {
                // A count in the victim's corner, as the dock counts an army.
                let words = kill.count.to_string();
                let tw = words.len() as i32 * 6 * k + k;
                let (tx, ty) = (x - tw - 2 * k, tile - 9 * k - k);
                row.rect(tx, ty, tw + k, 9 * k, backing);
                small(&mut row, &words, tx + k, ty + k, WHITE, k);
            }
            let y = y0 + i as i32 * pitch;
            for yy in 0..tile {
                for xx in 0..row_w {
                    if let Some(c) = row.get(xx, yy).filter(|c| c[3] != 0) {
                        let a = (u32::from(c[3]) * alpha / 255) as u8;
                        self.canvas.pixel(x0 + xx, y + yy, [c[0], c[1], c[2], a]);
                    }
                }
            }
        }
    }

    pub(crate) fn toggle_follow(&mut self) {
        self.spectator.follow = !self.spectator.follow;
        self.spectator.focus = None;
        self.notify(if self.spectator.follow {
            "Camera follows the biggest fight or a running count. Pan to stop."
        } else {
            "Camera is yours."
        });
    }

    /// Ease the camera toward the followed action, one step a tick.
    pub(crate) fn follow_camera(&mut self) {
        if !self.spectator.follow {
            return;
        }
        let Some(target) = self.spectator.choose_focus(&self.world) else {
            return;
        };
        let mut wanted = Camera::default();
        wanted.center(target);
        let ease = |from: i32, to: i32| {
            let d = to - from;
            if d == 0 {
                from
            } else {
                from + (d / 8).clamp(-40, 40) + d.signum()
            }
        };
        let (x0, x1, y0, y1) = camera_bounds(&self.world);
        self.camera.x = ease(self.camera.x, wanted.x).clamp(x0, x1);
        self.camera.y = ease(self.camera.y, wanted.y).clamp(y0, y1);
    }

    /// Row one with three seats: a column per seat in its colour with its
    /// faction, hold gauge, army and economy. A seat that is out is dimmed,
    /// its chip hollow, and says where it placed.
    fn draw_seat_columns(
        &mut self,
        usable: i32,
        clock_w: i32,
        army: &[u32],
        sample: Option<&crate::match_summary::Sample>,
    ) {
        let s = self.ui_scale();
        let char_w = 6 * s;
        let seats = self.world.seat_count();
        let start = 12 * s + clock_w + 14 * s;
        // The pace and menu controls take a little more than the band
        // leaves them: stop short of them.
        let end = usable - 26 * s;
        let col_w = (end - start) / seats as i32;
        // Every column names its seat as every other screen does, "RED
        // ASSEMBLY" (seat_title), or by its colour alone when a standing
        // seat would lose the room for its gauge, so the gauges line up.
        // The dock's rows name each seat in full either way.
        // The gauge's words are its seconds and a falling mark while it
        // drains; an empty gauge is no count.
        let words_w = 3 * char_w + 8 * s;
        let room = col_w - 12 * s - 10 * s - 24 * s - 12 * s - words_w;
        let names = |form: usize, seat: u8| match form {
            0 => crate::seats::seat_title(&self.world, seat),
            _ => crate::seats::seat_hue(&self.world, seat).word().to_string(),
        };
        let standing: Vec<u8> = self
            .world
            .seats()
            .filter(|&seat| !self.world.is_eliminated(seat))
            .collect();
        let widest = |form: usize| {
            standing
                .iter()
                .map(|&seat| names(form, seat).len() as i32)
                .max()
                .unwrap_or(0)
        };
        let form = usize::from(widest(0) * char_w > room);
        let name_cols = widest(form);
        let names: Vec<String> = self.world.seats().map(|seat| names(form, seat)).collect();
        for seat in self.world.seats() {
            let x0 = start + i32::from(seat) * col_w;
            let right = x0 + col_w - 12 * s;
            let colour = crate::seats::seat_colour(&self.world, seat);
            let word = crate::seats::seat_hue(&self.world, seat).word();
            let name = &names[seat as usize];
            self.canvas
                .line(x0 - 7 * s, 3 * s, x0 - 7 * s, 29 * s, EDGE);
            let name_x = x0 + 10 * s;
            if let Some(place) = place_out(&self.world.eliminated, seats, seat) {
                self.canvas.frame(x0, 4 * s, 6 * s, 6 * s, colour);
                let tag = ordinal(place);
                let room = right - name_x - (tag.len() as i32 + 1) * char_w;
                let name = Some(crate::seats::seat_title(&self.world, seat))
                    .filter(|name| name.len() as i32 * char_w <= room)
                    .unwrap_or_else(|| word.to_string());
                small(&mut self.canvas, &name, name_x, 4 * s, MUTED, s);
                let tag_x = name_x + (name.len() as i32 + 1) * char_w;
                small(&mut self.canvas, &tag, tag_x, 4 * s, colour, s);
                let went = self
                    .world
                    .eliminated
                    .iter()
                    .find(|(out, _)| *out == seat)
                    .map_or(0, |(_, tick)| *tick);
                let line = crate::native_ui::fit_segments(
                    &format!("OUT AT {}", clock(went)),
                    right - x0,
                    s,
                );
                small(&mut self.canvas, &line, x0, 18 * s, MUTED, s);
                continue;
            }
            self.canvas.rect(x0, 4 * s, 6 * s, 6 * s, colour);
            small(&mut self.canvas, name, name_x, 4 * s, colour, s);
            let gauge_x = name_x + name_cols * char_w + 6 * s;
            let gauge_w = (right - gauge_x - 6 * s - words_w).clamp(24 * s, 90 * s);
            let total = self.world.hold_ticks();
            let held = of(&self.world.lane_hold, seat).min(total);
            self.canvas
                .rect(gauge_x, 3 * s, gauge_w, 8 * s, [23, 38, 46, 255]);
            self.canvas.frame(gauge_x, 3 * s, gauge_w, 8 * s, EDGE);
            let fill = (i64::from(gauge_w) * i64::from(held) / i64::from(total)) as i32;
            if fill > 0 {
                self.canvas.rect(gauge_x, 3 * s, fill, 8 * s, colour);
            }
            if held > 0 {
                let secs = format!("{}S", total.saturating_sub(held).div_ceil(30));
                let tx = gauge_x + gauge_w + 6 * s;
                let frozen = self.world.hold_frozen();
                // A flood freezes every count: the seconds turn water blue.
                let secs_colour = if frozen { [110, 170, 210, 255] } else { colour };
                small(&mut self.canvas, &secs, tx, 4 * s, secs_colour, s);
                if !frozen && !self.world.holds_every_lane(seat) {
                    // Draining: a small falling triangle.
                    let mx = tx + secs.len() as i32 * char_w + s;
                    for row in 0..3 {
                        self.canvas.rect(
                            mx + row * s,
                            5 * s + row * s,
                            (5 - 2 * row) * s,
                            s,
                            colour,
                        );
                    }
                }
            }
            let income = sample.map_or(0, |sample| of(&sample.income, seat));
            self.draw_band_marks(seat, of(army, seat), income, x0, 17 * s, right - x0);
        }
    }

    /// A side's numbers in the band as marks: the sluice when it holds it,
    /// army value, salvage with the minute's income, and crew. Where they
    /// do not fit the salvage in hand and the crew cap go first, leaving
    /// the income and the crew; then marks are left off from the end.
    fn draw_band_marks(&mut self, seat: u8, army: u32, income: u32, x: i32, y: i32, room: i32) {
        use crate::hover_card::{Chip, Mark};
        let s = self.ui_scale();
        let player = &self.world.players[usize::from(seat)];
        let mut x = x;
        let mut room = room;
        if self.world.gate.owner == Some(seat) {
            // The sluice: a gate between two posts, as on the tide gauge.
            let colour = crate::seats::seat_colour(&self.world, seat);
            self.canvas.rect(x, y, s, 8 * s, colour);
            self.canvas.rect(x + 6 * s, y, s, 8 * s, colour);
            self.canvas.rect(x, y + s, 7 * s, s, colour);
            self.canvas.rect(x + 2 * s, y + 3 * s, 3 * s, 4 * s, colour);
            x += 12 * s;
            room -= 12 * s;
        }
        let chip = |mark, value: String| Chip {
            mark,
            value,
            short: false,
        };
        let width = |chips: &[Chip]| -> i32 {
            chips
                .iter()
                .map(|c| (10 + c.value.chars().count() as i32 * 6 + 7) * s)
                .sum()
        };
        let forms = [
            (
                format!("{} +{income}/M", player.salvage),
                format!("{}/{}", player.crew, player.cap),
            ),
            (
                format!("+{income}/M"),
                format!("{}/{}", player.crew, player.cap),
            ),
            (format!("+{income}/M"), player.crew.to_string()),
            (format!("+{income}"), player.crew.to_string()),
        ];
        let set = |(salvage, crew): &(String, String)| {
            vec![
                chip(Mark::Damage, army.to_string()),
                chip(Mark::Salvage, salvage.clone()),
                chip(Mark::Crew, crew.clone()),
            ]
        };
        let mut chips = forms
            .iter()
            .map(set)
            .find(|chips| width(chips) <= room)
            .unwrap_or_else(|| set(&forms[forms.len() - 1]));
        while chips.len() > 1 && width(&chips) > room {
            chips.pop();
        }
        self.draw_chips(&chips, x, y, false);
    }

    /// Jump the replay to a tick, from the timeline or the skip keys.
    pub(crate) fn seek_replay(&mut self, tick: u64) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        if !playback.can_seek() {
            self.notify("A live match cannot be rewound.");
            return;
        }
        let mut dock = std::mem::take(&mut self.ux.dock);
        let sought = playback.seek(tick, &mut dock);
        self.ux.dock = dock;
        match sought {
            Ok(reached) => {
                self.world = playback.view();
                self.aftermath_ticks = 0;
                self.effects.clear();
                self.motion.clear();
                self.fire_starts.clear();
                self.unload_starts.clear();
                self.gate_foam_start = None;
                self.spectator.clear();
                self.result_page = Default::default();
                self.result_log_scroll = 0;
                // The dock log came along with the seek: whole up to here.
                self.summary = MatchSummary::default();
                self.screen = crate::game::Screen::Match;
                self.notify(&format!("Replay at {}.", clock(reached)));
            }
            Err(e) => self.notify(&e),
        }
    }

    /// A click on the spectator band: the timeline seeks, the switch
    /// toggles the follow camera. Returns true when the click was taken.
    pub(crate) fn spectator_click(&mut self, x: i32, y: i32) -> bool {
        if !self.observing() || !self.native_ui() {
            return false;
        }
        if self
            .spectator
            .follow_switch
            .is_some_and(|r| r.contains(x, y))
        {
            self.toggle_follow();
            return true;
        }
        let Some(r) = self.spectator.timeline.filter(|r| r.contains(x, y)) else {
            return false;
        };
        let Some((first, last)) = self.timeline_span() else {
            return true;
        };
        let offset = u64::try_from((x - r.x).clamp(0, r.w)).unwrap_or(0);
        let tick = first + (last - first) * offset / r.w.max(1) as u64;
        self.seek_replay(tick);
        true
    }

    /// The summary the timeline draws: the scan's whole match when there is
    /// one, otherwise what has been watched.
    fn timeline_summary(&self) -> (MatchSummary, Option<(u64, bool)>) {
        match self.playback.as_ref().and_then(|p| p.index_summary()) {
            Some((summary, scanned, done)) => (summary, Some((scanned, done))),
            None => (self.summary.clone(), None),
        }
    }

    fn timeline_span(&self) -> Option<(u64, u64)> {
        let (summary, _) = self.timeline_summary();
        let first = summary.first_tick();
        let last = summary.last_tick().max(first + 1);
        (summary.samples.len() >= 2).then_some((first, last))
    }

    /// Draw the spectator band: row one is both sides side by side, row two
    /// the replay timeline. `top` is the band's height.
    pub(crate) fn draw_spectator_band(&mut self, top: i32) {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        self.canvas.rect(0, 0, w, top, INK);
        self.canvas.line(0, top - 1, w, top - 1, EDGE);
        self.canvas.line(0, 32 * s, w, 32 * s, EDGE);
        let army = crate::match_summary::army_value(&self.world);
        let (timeline, _) = self.timeline_summary();
        // Income is a minute's earnings, so it comes from the samples: the
        // scan's when there is one, as the watched stretch may be short.
        let sample = timeline
            .samples
            .iter()
            .rev()
            .find(|sample| sample.tick <= self.world.tick)
            .or(self.summary.latest())
            .cloned();
        let clock_text = clock(self.world.tick);
        let clock_w = crate::canvas::text_readable_width(&clock_text) * s;
        // The pace and menu controls keep the right end of the band.
        let usable = w - 130 * s;
        // With three seats the clock leads the row; with two it sits
        // between the halves.
        let clock_x = if self.world.seat_count() > 2 {
            12 * s
        } else {
            (usable - clock_w) / 2
        };
        text(&mut self.canvas, &clock_text, clock_x, 12 * s, WHITE, s);
        if self.world.seat_count() > 2 {
            self.draw_seat_columns(usable, clock_w, &army, sample.as_ref());
        }
        let side_w = usable / 2 - 48 * s;
        let two = self.world.seat_count() <= 2;
        for side in (0..2u8).filter(|_| two) {
            let color = crate::seats::seat_colour(&self.world, side);
            let player = &self.world.players[side as usize];
            let x0 = if side == 0 {
                12 * s
            } else {
                usable / 2 + 36 * s
            };
            small(&mut self.canvas, player.faction.name(), x0, 4 * s, color, s);
            // The hold gauge beside the name.
            let gauge_x = x0 + 104 * s;
            let gauge_w = (side_w - 104 * s - 76 * s).clamp(24 * s, 90 * s);
            let total = self.world.hold_ticks();
            let held = self.world.lane_hold[side as usize].min(total);
            self.canvas
                .rect(gauge_x, 3 * s, gauge_w, 8 * s, [23, 38, 46, 255]);
            self.canvas.frame(gauge_x, 3 * s, gauge_w, 8 * s, EDGE);
            let fill = (i64::from(gauge_w) * i64::from(held) / i64::from(total)) as i32;
            if fill > 0 {
                self.canvas.rect(gauge_x, 3 * s, fill, 8 * s, color);
            }
            let holding = self.world.holds_every_lane(side);
            let gauge_text = if held == 0 {
                "NO COUNT".to_string()
            } else {
                format!(
                    "{}S {}",
                    total.saturating_sub(held).div_ceil(30),
                    if self.world.hold_frozen() {
                        "FROZEN"
                    } else if holding {
                        "LEFT"
                    } else {
                        "DRAINING"
                    }
                )
            };
            small(
                &mut self.canvas,
                &gauge_text,
                gauge_x + gauge_w + 6 * s,
                4 * s,
                if held > 0 { color } else { MUTED },
                s,
            );
            let income = sample.as_ref().map_or(0, |sample| of(&sample.income, side));
            self.draw_band_marks(side, army[side as usize], income, x0, 17 * s, side_w);
        }
        // Row two: the timeline and the follow switch.
        let row = 32 * s;
        let follow_label = if self.spectator.follow {
            match self.spectator.focus {
                Some((_, why)) => format!("F FOLLOW: {why}"),
                None => "F FOLLOW: ON".into(),
            }
        } else {
            "F FOLLOW: OFF".into()
        };
        let switch = Rect {
            x: 12 * s,
            y: row + 6 * s,
            w: 108 * s,
            h: 14 * s,
        };
        self.canvas
            .rect(switch.x, switch.y, switch.w, switch.h, [34, 53, 60, 255]);
        self.canvas.frame(
            switch.x,
            switch.y,
            switch.w,
            switch.h,
            if self.spectator.follow { GOLD } else { EDGE },
        );
        small(
            &mut self.canvas,
            &follow_label,
            switch.x + 4 * s,
            switch.y + 4 * s,
            if self.spectator.follow { GOLD } else { MUTED },
            s,
        );
        self.spectator.follow_switch = Some(switch);
        let (summary, scan) = self.timeline_summary();
        let seekable = self.playback.as_ref().is_some_and(|p| p.can_seek());
        // A live match says so; a finished one explains seeking on hover.
        let hint = if seekable { "" } else { "LIVE" };
        let hint_w = if hint.is_empty() {
            0
        } else {
            hint.len() as i32 * 6 * s + 12 * s
        };
        let strip = Rect {
            x: switch.x + switch.w + 12 * s,
            y: row + 6 * s,
            w: (w - (switch.x + switch.w + 12 * s) - hint_w - 12 * s).max(40),
            h: 14 * s,
        };
        crate::match_summary::draw_sluice_strip(
            &mut self.canvas,
            &summary,
            [strip.x, strip.y, strip.w, strip.h],
            (summary.samples.len() >= 2).then_some(self.world.tick),
        );
        if let Some((scanned, false)) = scan {
            small(
                &mut self.canvas,
                &format!("READING THE RECORDING {}", clock(scanned)),
                strip.x + 4 * s,
                strip.y + 4 * s,
                MUTED,
                s,
            );
        }
        if !hint.is_empty() {
            small(&mut self.canvas, hint, w - hint_w, row + 10 * s, MUTED, s);
        }
        // Hovering the timeline: a hairline at the pointer, and under it
        // the time a click would seek to and the skip keys.
        let (cx, cy) = self.cursor;
        if seekable
            && strip.contains(cx, cy)
            && let Some((first, last)) = self.timeline_span()
        {
            self.canvas.rect(cx, strip.y, s, strip.h, WHITE);
            let offset = u64::try_from((cx - strip.x).clamp(0, strip.w)).unwrap_or(0);
            let at = first + (last - first) * offset / strip.w.max(1) as u64;
            let words = format!("SEEK TO {}  [ ] SKIP 30S", clock(at));
            let tw = words.len() as i32 * 6 * s + 8 * s;
            let tx = (cx - tw / 2).clamp(4 * s, w - tw - 4 * s);
            let ty = top + 4 * s;
            self.canvas.rect(tx, ty, tw, 13 * s, INK);
            self.canvas.frame(tx, ty, tw, 13 * s, EDGE);
            small(&mut self.canvas, &words, tx + 4 * s, ty + 3 * s, WHITE, s);
        }
        self.spectator.timeline = Some(strip);
    }
}

/// `--spectate-review DIR --replay FILE`: watch a finished recording and save
/// what an observer and the players see after it: the spectator band while
/// following, the band during a hold count, and both result pages as a
/// player and as an observer.
pub fn export_review(
    game: &mut Game,
    replay: &std::path::Path,
    out: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    game.resize_view(1920, 1080);
    game.start_playback(replay, false)?;
    let started = std::time::Instant::now();
    loop {
        let done = game
            .playback
            .as_ref()
            .and_then(|p| p.index_summary())
            .is_some_and(|(_, _, done)| done);
        if done {
            break;
        }
        if started.elapsed() > std::time::Duration::from_secs(900) {
            return Err("the replay scan did not finish".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let (summary, ..) = game
        .playback
        .as_ref()
        .and_then(|p| p.index_summary())
        .ok_or("no replay summary")?;
    let end = summary.last_tick();
    // The biggest fight and the last count, then the end.
    let mut moments = vec![("fight", summary.turning_point().map(|t| t.0 + 300))];
    moments.push(("count", summary.holds.last().map(|run| run.start + 600)));
    for (name, tick) in moments {
        let Some(tick) = tick else { continue };
        game.seek_replay(tick.saturating_sub(150));
        game.spectator.follow = true;
        for _ in 0..150 {
            game.tick();
        }
        game.screenshot(&out.join(format!("spectate-{name}.png")))?;
    }
    // The smallest window with the full interface at scale two.
    game.resize_view(1280, 720);
    game.screenshot(&out.join("spectate-1280.png"))?;
    game.resize_view(1920, 1080);
    game.seek_replay(end.saturating_sub(60));
    for _ in 0..240 {
        game.tick();
        if game.world.outcome.is_some() {
            break;
        }
    }
    game.screenshot(&out.join("result-observer-summary.png"))?;
    game.action(crate::game::Action::ResultPage(
        crate::dock_log::ResultPage::Log,
    ));
    game.screenshot(&out.join("result-observer-log.png"))?;
    // The same finish seen from a seat, the replay's first side, with the
    // whole match's dock log behind it: play it from the start.
    game.seek_replay(0);
    game.ux.reset_dock_for_new_match();
    while game.world.outcome.is_none() && game.world.tick < end + 30 {
        game.tick();
    }
    game.playback = None;
    game.result_page = Default::default();
    game.screenshot(&out.join("result-player-summary.png"))?;
    game.action(crate::game::Action::ResultPage(
        crate::dock_log::ResultPage::Log,
    ));
    game.screenshot(&out.join("result-player-log.png"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock_log::ResultPage;
    use bw_core::Faction;

    fn damage(world: &mut World, at: Pos, amount: i32) {
        world.events.push(bw_sim::Event {
            tick: world.tick,
            kind: EventKind::Damage,
            player: Some(0),
            entity: None,
            other: None,
            from: None,
            to: Some(at),
            amount,
            text: String::new(),
            cause: None,
        });
    }

    #[test]
    fn the_camera_follows_the_bigger_fight_and_does_not_flicker() {
        let mut world = World::new(5, Faction::Union);
        let mut spectator = Spectator::default();
        world.events.clear();
        damage(&mut world, Pos::cell(30, 30), 10);
        damage(&mut world, Pos::cell(31, 30), 10);
        damage(&mut world, Pos::cell(90, 90), 12);
        spectator.observe(&world);
        let focus = spectator.choose_focus(&world).unwrap();
        assert!(focus.distance_sq(Pos::cell(30, 30)) < i64::from(4 * bw_core::FP).pow(2));
        // A slightly bigger fight elsewhere does not take the camera.
        world.events.clear();
        damage(&mut world, Pos::cell(90, 90), 12);
        spectator.observe(&world);
        let again = spectator.choose_focus(&world).unwrap();
        assert_eq!(again, focus);
        // A much bigger one does.
        world.events.clear();
        damage(&mut world, Pos::cell(90, 90), 60);
        spectator.observe(&world);
        let moved = spectator.choose_focus(&world).unwrap();
        assert!(moved.distance_sq(Pos::cell(90, 90)) < i64::from(4 * bw_core::FP).pow(2));
        // Quiet fighting ages out.
        world.events.clear();
        world.tick += HEAT_TICKS + 1;
        spectator.observe(&world);
        assert!(spectator.biggest_fight().is_none());
    }

    fn confluence(seed: u64) -> World {
        World::with_map(
            seed,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .expect("confluence")
    }

    #[test]
    fn the_camera_is_bounded_by_the_map_it_watches() {
        assert_eq!(
            camera_bounds(&World::new(1, Faction::Union)),
            (-2400, 2400, -200, 2400)
        );
        let (x0, x1, y0, y1) = camera_bounds(&confluence(1));
        // The Confluence's far corners are inside the bounds.
        for (cx, cy) in [(0, 0), (175, 0), (0, 175), (175, 175)] {
            let mut camera = Camera::default();
            camera.center(Pos::cell(cx, cy));
            assert!((x0..=x1).contains(&camera.x), "{cx},{cy} x {}", camera.x);
            assert!((y0..=y1).contains(&camera.y), "{cx},{cy} y {}", camera.y);
        }
    }

    #[test]
    fn a_three_seat_count_is_watched_at_one_of_its_own_mouths() {
        let mut world = confluence(2);
        world.lane_hold = vec![0, 40, 90];
        let mut spectator = Spectator::default();
        let focus = spectator.choose_focus(&world).expect("a count to watch");
        // Seat 2 has the fuller gauge; its lanes are the two beside it.
        let mouths = world.crossing_mouths();
        let own: Vec<Pos> = world
            .arms_of(2)
            .into_iter()
            .flat_map(|arm| mouths[arm])
            .collect();
        assert!(own.contains(&focus), "{focus:?} not at seat 2's lanes");
        assert_eq!(spectator.focus.map(|(_, why)| why), Some("COUNT"));
    }

    fn death(
        world: &World,
        victim: u32,
        killer: Option<u32>,
        cause: Option<Kind>,
    ) -> bw_sim::Event {
        bw_sim::Event {
            tick: world.tick,
            kind: EventKind::Death,
            player: None,
            entity: Some(victim),
            other: killer,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause,
        }
    }

    #[test]
    fn the_kill_feed_names_both_seats_merges_repeats_and_fades() {
        let mut world = confluence(3);
        world.ai_enabled = false;
        world.events.clear();
        let mut spectator = Spectator::default();
        spectator.observe(&world);
        let unit = |world: &World, seat: u8, nth: usize| {
            world
                .entities
                .iter()
                .filter(|e| e.owner == seat && !e.kind.is_building())
                .nth(nth)
                .map(|e| (e.id, e.kind))
                .unwrap()
        };
        let (killer, killer_kind) = unit(&world, 1, 0);
        let (victim, victim_kind) = unit(&world, 2, 0);
        // Another of the same kind, for a repeat of the same kill.
        let second = world
            .entities
            .iter()
            .find(|e| e.owner == 2 && e.kind == victim_kind && e.id != victim)
            .map(|e| e.id)
            .unwrap();
        let (own, _) = unit(&world, 1, 1);
        // The victim and its killer both fell in the exchange: the feed
        // still knows them from the tick before.
        world.tick += 1;
        world.entities.retain(|e| e.id != victim && e.id != killer);
        world.events = vec![
            death(&world, victim, Some(killer), Some(killer_kind)),
            // A machine lost with its own seat's transport is no kill.
            death(&world, own, Some(unit(&world, 1, 2).0), None),
            // Nor is a death with no killer.
            death(&world, second, None, None),
        ];
        spectator.observe(&world);
        assert_eq!(
            spectator.kills,
            vec![Kill {
                tick: world.tick,
                killer: (1, killer_kind),
                victim: (2, victim_kind),
                count: 1,
            }]
        );
        // The same kill again straight after joins the row as a count.
        world.tick += 10;
        let again = world
            .entities
            .iter()
            .find(|e| e.owner == 1 && e.kind == killer_kind)
            .map(|e| e.id)
            .unwrap();
        world.events = vec![death(&world, second, Some(again), Some(killer_kind))];
        spectator.observe(&world);
        assert_eq!(spectator.kills.len(), 1);
        assert_eq!(spectator.kills[0].count, 2);
        // At most KILL_ROWS rows, the newest kept.
        let (a, a_kind) = unit(&world, 0, 0);
        for _ in 0..KILL_ROWS + 2 {
            world.tick += KILL_MERGE_TICKS + 1;
            let (v, _) = unit(&world, 1, 0);
            world.events = vec![death(&world, v, Some(a), Some(a_kind))];
            spectator.observe(&world);
        }
        assert_eq!(spectator.kills.len(), KILL_ROWS);
        assert!(spectator.kills.iter().all(|k| k.killer == (0, a_kind)));
        // They age out.
        world.events.clear();
        world.tick += KILL_TICKS;
        spectator.observe(&world);
        assert!(spectator.kills.is_empty());
        // A seek forgets them.
        spectator.kills.push(Kill {
            tick: world.tick,
            killer: (0, a_kind),
            victim: (1, victim_kind),
            count: 1,
        });
        spectator.clear();
        assert!(spectator.kills.is_empty());
    }

    #[test]
    fn the_kill_feed_draws_in_seat_colours_clear_of_the_centre() {
        let (mut game, data) = scratch_game("kills");
        game.map = bw_sim::MapId::Confluence;
        game.start();
        game.resize_view(1280, 720);
        game.world.ai_enabled = false;
        for entity in &mut game.world.entities {
            if entity.owner == 0 && entity.kind == Kind::Headquarters {
                entity.hp = 0;
            }
        }
        game.world.entities.retain(|entity| entity.hp > 0);
        for _ in 0..3 {
            game.tick();
        }
        assert!(game.observing());
        let tick = game.world.tick;
        let kill = Kill {
            tick,
            killer: (1, Kind::Riveter),
            victim: (2, Kind::Hook),
            count: 3,
        };
        game.spectator.kills = vec![kill];
        game.render();
        let (x, y, pitch, k) = game.kill_feed_origin();
        let v = game.world_view();
        let w = game.canvas.width() as i32;
        let (row_w, tile) = (kill_row_width(k), 26 * k);
        // In the field's top-right corner: right of its middle two thirds
        // and above its middle, whatever is at the station there.
        assert!(x >= w * 2 / 3 && x + row_w <= w, "{x}");
        assert!(y >= v.top && y + KILL_ROWS as i32 * pitch <= (v.top + v.bottom) / 2);
        // The killer's tile wears its seat's colour, the victim's its own.
        let killer_colour = crate::seats::seat_colour(&game.world, 1);
        let victim_colour = crate::seats::seat_colour(&game.world, 2);
        let at = |game: &Game, x: i32, y: i32| game.canvas.get(x, y).unwrap();
        assert_eq!(at(&game, x, y + tile / 2), killer_colour);
        assert_eq!(at(&game, x + row_w - 1, y + tile / 2), victim_colour);
        // Faded out at the end of its time: what is under it shows.
        game.spectator.kills.clear();
        game.render();
        let under = at(&game, x, y + tile / 2);
        assert_ne!(under, killer_colour);
        game.spectator.kills = vec![kill];
        game.world.tick = tick + KILL_TICKS - 1;
        game.render();
        let faded = at(&game, x, y + tile / 2);
        let near = |a: u8, b: u8| a.abs_diff(b) <= 8;
        assert!(
            (0..3).all(|c| near(faded[c], under[c])),
            "{faded:?} {under:?}"
        );
        let _ = std::fs::remove_dir_all(data);
    }

    fn scratch_game(label: &str) -> (Game, std::path::PathBuf) {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data = std::env::temp_dir().join(format!(
            "brinewake-spectator-{label}-{}-{stamp}",
            std::process::id()
        ));
        (Game::new_with_data_dir(base, data.clone()), data)
    }

    #[test]
    fn a_seat_knocked_out_watches_the_rest_without_its_own_controls() {
        let (mut game, data) = scratch_game("out");
        game.map = bw_sim::MapId::Confluence;
        game.start();
        game.resize_view(1280, 720);
        game.world.ai_enabled = false;
        let worker = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        game.groups[1] = vec![worker];
        game.selected = vec![worker];
        assert!(!game.observing());
        for entity in &mut game.world.entities {
            if entity.owner == 0 && entity.kind == bw_core::Kind::Headquarters {
                entity.hp = 0;
            }
        }
        game.world.entities.retain(|entity| entity.hp > 0);
        for _ in 0..3 {
            game.tick();
        }
        assert!(game.world.is_eliminated(0) && game.world.outcome.is_none());
        assert!(game.knocked_out() && game.observing() && game.spectator_band());
        assert!(game.world.revealed, "the fog lifts");
        assert!(game.selected.is_empty());
        assert!(game.groups.iter().all(Vec::is_empty), "no groups left");
        assert!(
            game.message.contains("YOU ARE OUT: 3RD"),
            "{}",
            game.message
        );
        let out_at = game.out_since().unwrap();
        // The watcher's band and dock stand in for the seat's header and
        // card: no button trains, builds or orders anything.
        game.render();
        assert!(game.buttons.iter().all(|b| !matches!(
            b.action,
            crate::game::Action::Train(_)
                | crate::game::Action::Build(_)
                | crate::game::Action::RecallGroup(_)
                | crate::game::Action::Reclaim
        )));
        // A full pressure bar says nothing to a seat that is out, and the
        // seat earns nothing more.
        game.world.players[0].pressure = game.world.players[0].pressure_cap;
        let salvage = game.world.players[0].salvage;
        for _ in 0..(OUT_BANNER_TICKS + 30) {
            game.tick();
        }
        assert!(
            !game.message.starts_with("Pressure is full"),
            "{}",
            game.message
        );
        assert_eq!(game.world.players[0].salvage, salvage);
        assert!(
            game.world.tick >= out_at + OUT_BANNER_TICKS,
            "the banner is gone"
        );
        // Orders are refused with a reason.
        game.issue(bw_sim::Command::Stop {
            units: vec![worker],
        });
        assert_eq!(game.message, "You are out: watching the rest.");
        let _ = std::fs::remove_dir_all(data);
    }

    /// `BW_SEATS_REVIEW=DIR cargo test --release -p bw_desktop
    /// three_seat_screens_review -- --ignored`: plays a Confluence match
    /// with the practice AI on the other seats, knocks seats out by
    /// surrender, and saves the spectator band and the result pages.
    #[test]
    #[ignore]
    fn three_seat_screens_review() {
        let Ok(out) = std::env::var("BW_SEATS_REVIEW") else {
            return;
        };
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = out.join("data");
        let mut game = Game::new_with_data_dir(base, data);
        game.map = bw_sim::MapId::Confluence;
        game.start();
        game.resize_view(1920, 1080);
        let step = |game: &mut Game, ticks: u64| {
            for _ in 0..ticks {
                game.tick();
            }
        };
        // Seat 0 idles and the computer plays red and violet. The person's
        // headquarters falls first; the match goes on without it, and then
        // violet gives up, leaving red.
        step(&mut game, 9000);
        for entity in &mut game.world.entities {
            if entity.owner == 0 && entity.kind == bw_core::Kind::Headquarters {
                entity.hp = 0;
            }
        }
        game.world.entities.retain(|entity| entity.hp > 0);
        step(&mut game, 60);
        assert!(game.world.is_eliminated(0) && game.world.outcome.is_none());
        game.world.revealed = true;
        game.screenshot(&out.join("you-are-out.png")).unwrap();
        step(&mut game, 1800);
        game.world.issue(2, bw_sim::Command::Surrender).unwrap();
        step(&mut game, 5);
        assert_eq!(game.world.outcome, Some(bw_sim::Outcome::Victory(1)));
        assert_eq!(game.summary.place_of(0), Some(3));
        assert_eq!(game.summary.place_of(2), Some(2));
        game.screenshot(&out.join("result-player-summary.png"))
            .unwrap();
        game.action(crate::game::Action::ResultPage(ResultPage::Log));
        game.screenshot(&out.join("result-player-log.png")).unwrap();
        // An observer's match, all by recorded orders: violet gives up at
        // five minutes, red at seven, and the first seat is left.
        let mut world = confluence(4);
        for tick in 1..=12_600u64 {
            if tick == 9000 {
                world.issue(2, bw_sim::Command::Surrender).unwrap();
            }
            if tick == 12_600 {
                world.issue(1, bw_sim::Command::Surrender).unwrap();
            }
            world.step();
        }
        assert_eq!(world.outcome, Some(bw_sim::Outcome::Victory(0)));
        let replay = out.join("three.replay.json");
        world.export_replay(&replay).unwrap();
        game.start_playback(&replay, false).unwrap();
        let started = std::time::Instant::now();
        while !game
            .playback
            .as_ref()
            .and_then(|p| p.index_summary())
            .is_some_and(|(_, _, done)| done)
        {
            assert!(started.elapsed().as_secs() < 300, "scan finishes");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        game.seek_replay(8000);
        step(&mut game, 60);
        game.screenshot(&out.join("spectate-three.png")).unwrap();
        game.seek_replay(9500);
        step(&mut game, 60);
        game.screenshot(&out.join("spectate-one-out.png")).unwrap();
        game.resize_view(1280, 720);
        game.screenshot(&out.join("spectate-one-out-1280.png"))
            .unwrap();
        game.resize_view(1920, 1080);
        let count = game
            .playback
            .as_ref()
            .and_then(|p| p.index_summary())
            .and_then(|(summary, ..)| summary.holds.last().copied());
        if let Some(run) = count {
            game.seek_replay(run.start + 300);
            step(&mut game, 300);
            game.screenshot(&out.join("spectate-count.png")).unwrap();
        }
        let end = game
            .playback
            .as_ref()
            .and_then(|p| p.index_summary())
            .map(|(summary, ..)| summary.last_tick())
            .unwrap();
        game.seek_replay(end.saturating_sub(30));
        step(&mut game, 90);
        game.screenshot(&out.join("result-observer-summary.png"))
            .unwrap();
        game.action(crate::game::Action::ResultPage(ResultPage::Log));
        game.screenshot(&out.join("result-observer-log.png"))
            .unwrap();
    }
}
