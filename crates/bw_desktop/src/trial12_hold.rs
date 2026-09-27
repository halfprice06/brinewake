//! Trial 12: the hold and tide interface on the Confluence.
//!
//! - The Guide states the hold length the rule gives (75 s with three seats,
//!   120 s once one is out, 90 s with two), on every seat.
//! - An enemy count names the lanes that stop it ("GUN AT E OR S STOPS IT")
//!   and rings their banks on the field and the chart: only a gun at a bank
//!   of the counting seat's own lanes stops it (trial 12: both defenders sent
//!   guns to their own W bank in the last minute).
//! - A lane chip on the sluice card that reads FOE wears a mark for what
//!   blocks it, a nest or a machine, and a click looks at the blocker.
//!
//! The vocabulary is LANE (a water crossing, by letter) and BANK (each end
//! of a lane, by its owner): "E LANE, RED BANK".

use crate::canvas::{Canvas, Color, INK, WHITE};
use crate::game::{Action, Game};
use bw_core::{EntityId, FP, Faction, Pos, TICK_HZ};
use bw_sim::{MapId, World};

/// The hold length a map's rule gives, in seconds: at the start, or once
/// a seat is out.
pub(crate) fn rule_hold_seconds(map: MapId, any_out: bool) -> u32 {
    bw_sim::hold_ticks_for(map.layout().seat_count(), any_out) / TICK_HZ as u32
}

/// A Guide page's facts as drawn: the map's own page, with the hold length
/// in force (`live`, from `World::hold_ticks`) where it differs from the
/// map's opening one, so a Guide read after a seat is out says 120S.
pub(crate) fn guide_lines(
    page: usize,
    faction: Faction,
    map: MapId,
    live: Option<u32>,
) -> Vec<String> {
    let (lines, _) = crate::menus::guide_page_on(page, faction, map);
    let opening = format!("{}S", rule_hold_seconds(map, false));
    lines
        .iter()
        .map(|line| match live {
            Some(now) if format!("{now}S") != opening && line.contains("LANES") => {
                line.replace(&opening, &format!("{now}S"))
            }
            _ => line.to_string(),
        })
        .collect()
}

/// The lanes whose banks stop `player`'s count: its own hold lanes.
pub(crate) fn stop_lanes(world: &World, player: u8) -> Vec<usize> {
    world.hold_arms(player)
}

/// "E OR S", "N OR S", "E": the letters of lanes, for a line.
pub(crate) fn lane_letters(world: &World, lanes: &[usize]) -> String {
    lanes
        .iter()
        .map(|&lane| crate::seats::arm_letter(world, lane))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// The banner's line under an enemy count: the lanes a gun stops it at.
pub(crate) fn stop_hint(world: &World, player: u8) -> String {
    format!(
        "GUN AT {} STOPS IT",
        lane_letters(world, &stop_lanes(world, player))
    )
}

/// A place on a lane's bank, for alert cards: "E LANE, RED BANK",
/// "N LANE, YOUR BANK".
pub(crate) fn bank_words(world: &World, lane: usize, bank: u8) -> String {
    let owner = match bank {
        0 => "YOUR",
        _ if world.seat_count() <= 2 => "ENEMY",
        seat => crate::seats::seat_name(world, seat),
    };
    format!(
        "{} LANE, {owner} BANK",
        crate::seats::arm_letter(world, lane)
    )
}

/// One enemy that blocks this seat's count on a lane: a machine at either
/// bank, or a finished nest whose middle stands in a bank's ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Blocker {
    pub id: EntityId,
    pub pos: Pos,
    pub owner: u8,
    pub nest: bool,
}

/// Everything that blocks seat 0's count on `lane`, nests first, then by
/// id.  The banks are lit for every seat, so fog does not hide them.
pub(crate) fn lane_blockers(world: &World, lane: usize) -> Vec<Blocker> {
    let radius = i64::from(FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let mouths = world.map.layout().arm_mouths(lane);
    let mut out: Vec<Blocker> = world
        .entities
        .iter()
        .filter(|e| e.owner != 0 && e.hp > 0 && e.build_remaining == 0 && e.aboard.is_none())
        .filter_map(|e| {
            let (place, nest) = if crate::qol::lane_holder(e.kind) {
                (e.pos, false)
            } else if bw_sim::blocks_mouth_building(e.kind) {
                (bw_sim::footprint_middle(e), true)
            } else {
                return None;
            };
            mouths
                .iter()
                .any(|mouth| place.distance_sq(*mouth) <= radius)
                .then_some(Blocker {
                    id: e.id,
                    pos: e.pos,
                    owner: e.owner,
                    nest,
                })
        })
        .collect();
    out.sort_by_key(|b| (!b.nest, b.id));
    out
}

/// Whether an enemy nest only slows this seat's count on a lane instead of
/// stopping it.
///
/// With three seats rules 22 halves a count's rate where an enemy nest
/// stands and no enemy gun does, rather than stopping it: the chip reads
/// SLOW.
pub(crate) fn lane_slowed(world: &World, seat: u8, lane: usize) -> bool {
    world.nests_slow_counts()
        && world
            .map
            .layout()
            .arm_mouths(lane)
            .into_iter()
            .any(|mouth| world.mouth_slowed_for(seat, mouth))
}

/// A mark for what blocks a lane, 5 by 5 logical pixels at scale `s`: a
/// nest's turret (battlements over a block), or a machine (a hull on two
/// wheels).  Drawn in code, in `color`.
pub(crate) fn blocker_mark(canvas: &mut Canvas, x: i32, y: i32, nest: bool, color: Color, s: i32) {
    let rows: [u8; 5] = if nest {
        [0b10101, 0b11111, 0b01110, 0b01110, 0b11111]
    } else {
        [0b00100, 0b01110, 0b11111, 0b11111, 0b01010]
    };
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                canvas.rect(x + col * s, y + row as i32 * s, s, s, color);
            }
        }
    }
}

/// Points of a ring of `radius` world units around `pos`, as projected.
fn ring_points(game: &Game, pos: Pos, radius: i32) -> Vec<(i32, i32)> {
    (0..24)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 24.0;
            game.camera.project(Pos {
                x: pos.x + (radius as f32 * angle.cos()) as i32,
                y: pos.y + (radius as f32 * angle.sin()) as i32,
            })
        })
        .collect()
}

impl Game {
    /// The lanes to ring while an enemy count runs: the counting seat's
    /// own, where a gun of ours stops it.  None for an observer, our own
    /// count, a broken one or a frozen one.
    pub(crate) fn stop_ring_lanes(&self) -> Option<(u8, Vec<usize>)> {
        if self.spectator_band() || self.world.outcome.is_some() {
            return None;
        }
        let (player, _) = self.hold_gauge()?;
        (player != 0 && self.world.holds_every_lane(player) && !self.world.hold_frozen())
            .then(|| (player, stop_lanes(&self.world, player)))
    }

    /// On the field: a ring that breathes out from each bank of the lanes
    /// that stop an enemy count, and four chevrons pointing into it.  The
    /// counting seat's colour and white take turns, twice a second.
    pub(crate) fn draw_stop_rings(&mut self) {
        let Some((player, lanes)) = self.stop_ring_lanes() else {
            return;
        };
        let mouths = self.world.crossing_mouths();
        let base = bw_content::CROSSING_HOLD_RADIUS_CELLS * FP;
        let phase = (self.world.tick % 30) as i32;
        let colour = if (self.world.tick / 15).is_multiple_of(2) {
            WHITE
        } else {
            crate::qol::hold_color(&self.world, player)
        };
        for lane in lanes {
            for point in mouths[lane] {
                let radius = base + FP / 2 + FP * phase / 30;
                let ring = ring_points(self, point, radius);
                for i in 0..ring.len() {
                    let (x, y) = ring[i];
                    let (xx, yy) = ring[(i + 1) % ring.len()];
                    self.canvas.line(x, y, xx, yy, colour);
                }
                let (cx, cy) = self.camera.project(point);
                for quarter in [0, 6, 12, 18] {
                    let (x, y) = ring[quarter];
                    let (dx, dy) = ((cx - x).signum(), (cy - y).signum());
                    // A chevron: two strokes meeting at the ring, opening
                    // outward, so it points in.
                    let (tx, ty) = (x + dx * 2, y + dy * 2);
                    let (ox, oy) = (-dy, dx);
                    for side in [-1, 1] {
                        let (ex, ey) = (tx - dx * 4 + side * ox * 3, ty - dy * 4 + side * oy * 3);
                        self.canvas.line(tx, ty, ex, ey, colour);
                        self.canvas.line(tx + dx, ty + dy, ex + dx, ey + dy, INK);
                    }
                }
            }
        }
    }

    /// On the chart: a white diamond around each bank of the lanes that
    /// stop an enemy count, on while the held banks' squares are off.
    pub(crate) fn draw_stop_marks_on_chart(&mut self, chart: crate::minimap_chart::ChartRect) {
        let Some((_, lanes)) = self.stop_ring_lanes() else {
            return;
        };
        if (self.world.tick / 8).is_multiple_of(2) {
            return;
        }
        let s = self.ui_scale();
        let mouths = self.world.crossing_mouths();
        for lane in lanes {
            for point in mouths[lane] {
                let (x, y) = chart.plot(&self.world, point);
                let r = 5 * s;
                self.canvas.line(x - r, y, x, y - r, WHITE);
                self.canvas.line(x, y - r, x + r, y, WHITE);
                self.canvas.line(x + r, y, x, y + r, WHITE);
                self.canvas.line(x, y + r, x - r, y, WHITE);
            }
        }
    }

    /// A lane chip's click: the blocker, cycling through them while the
    /// camera already looks at one; else our bank of the lane.
    pub(crate) fn focus_lane(&mut self, lane: usize) {
        if lane >= self.world.crossing_mouths().len() {
            return;
        }
        let letter = crate::seats::arm_letter(&self.world, lane);
        let blockers = lane_blockers(&self.world, lane);
        let (target, message) = if blockers.is_empty() {
            let bank = self
                .world
                .own_mouth(0, lane)
                .unwrap_or(self.world.crossing_mouths()[lane][0]);
            let message = if self.world.holds_lane(0, lane) {
                format!("{letter} lane: held.")
            } else {
                format!("{letter} lane: put a gun at either bank.")
            };
            (bank, message)
        } else {
            let next = blockers
                .iter()
                .position(|b| self.looks_at(b.pos))
                .map_or(0, |i| (i + 1) % blockers.len());
            let b = blockers[next];
            let whose = if self.world.seat_count() > 2 {
                let name = crate::seats::seat_name(&self.world, b.owner);
                let mut chars = name.chars();
                chars.next().map_or(String::new(), |first| {
                    first.to_string() + &chars.as_str().to_lowercase()
                })
            } else {
                "Enemy".into()
            };
            let what = if b.nest { "nest" } else { "machine" };
            // With three seats a nest only halves the count (rules 22).
            let does = if b.nest && self.world.nests_slow_counts() {
                "slows"
            } else {
                "blocks"
            };
            let more = if blockers.len() > 1 {
                format!(" ({} of {})", next + 1, blockers.len())
            } else {
                String::new()
            };
            (
                b.pos,
                format!("{letter} lane: {whose} {what} {does} your count{more}."),
            )
        };
        self.camera.center(target);
        self.camera_sub = (0.0, 0.0);
        self.drag = None;
        self.ux.minimap_drag = false;
        self.mode = crate::game::Mode::Context;
        self.last_click = None;
        self.notify(&message);
    }

    /// Whether the camera centres on `pos`, within a couple of cells.
    fn looks_at(&self, pos: Pos) -> bool {
        let mut there = self.camera;
        there.center(pos);
        (there.x - self.camera.x).abs() <= 8 && (there.y - self.camera.y).abs() <= 8
    }
}

/// The hover card's title and one line for the sluice card's hold row and
/// its lane chips (trial 12: "HOLD CHECKLIST: SHOW THE CROSSING MOUTH THAT
/// DECIDES THE HOLD." read as a developer's note).
pub(crate) fn hold_card_words(game: &Game, b: &crate::game::Button) -> Option<(String, String)> {
    match b.action {
        Action::FocusHold if b.label == "HOLD CHECKLIST" => Some((
            "YOUR HOLD".into(),
            "The sluice, then your lanes: GUN is yours at a bank, FOE blocks you. Click to look."
                .into(),
        )),
        Action::FocusLane(lane) => {
            let world = &game.world;
            let letter = crate::seats::arm_letter(world, lane);
            let blockers = lane_blockers(world, lane);
            let body = if let Some(b) = blockers.first() {
                format!(
                    "Blocked by {} {}. Click to look.",
                    crate::seats::seat_owner_word(world, b.owner).to_lowercase(),
                    if b.nest { "nest" } else { "machine" }
                )
            } else if world.holds_lane(0, lane) {
                "Held. Click to look.".into()
            } else {
                "A gun of yours at either bank holds it. Click to look.".into()
            };
            Some((format!("{letter} LANE"), body))
        }
        _ => None,
    }
}

/// What an upgrade still lacks once the orders already sent have spent
/// theirs, naming the resource: "Need 450 more salvage." (trial 12: three
/// upgrades ordered in one tick, the third refused as "INSUFFICIENT
/// RESOURCES FOR THE UPGRADE" after it had read "started").
pub(crate) fn upgrade_shortfall(game: &Game, upgrade: bw_content::Upgrade) -> Option<String> {
    let (salvage, pressure, _) = upgrade.cost();
    let (held_salvage, held_pressure) = crate::production_qol::pending_spend(game);
    let player = &game.world.players[0];
    let s = salvage.saturating_sub(player.salvage.saturating_sub(held_salvage));
    let p = pressure.saturating_sub(player.pressure.saturating_sub(held_pressure));
    let held = |amount: u32| {
        if amount > 0 {
            format!(": {amount} goes to orders already sent")
        } else {
            String::new()
        }
    };
    match (s, p) {
        (0, 0) => None,
        (s, 0) => Some(format!("Need {s} more salvage{}.", held(held_salvage))),
        (0, p) => Some(format!("Need {p} more pressure{}.", held(held_pressure))),
        (s, p) => Some(format!("Need {s} salvage and {p} pressure.")),
    }
}

/// A refusal that comes back from the world a few ticks after the order
/// was accepted, in words that name what was short.
pub(crate) fn late_refusal_words(game: &Game, refusal: &str) -> String {
    if !refusal.starts_with("insufficient resources for the upgrade") {
        return refusal.to_string();
    }
    let refused = game.world.command_log.iter().rev().find_map(|record| {
        match (&record.command, record.player, record.applied) {
            (bw_sim::Command::Upgrade { upgrade, .. }, 0, Some(false)) => Some(*upgrade),
            _ => None,
        }
    });
    let Some(upgrade) = refused else {
        return "Upgrade refused: another order spent its salvage first.".into();
    };
    let (salvage, pressure, _) = upgrade.cost();
    let player = &game.world.players[0];
    let s = salvage.saturating_sub(player.salvage);
    let p = pressure.saturating_sub(player.pressure);
    match (s, p) {
        (0, p) if p > 0 => format!("{} refused: need {p} more pressure.", upgrade.name()),
        (s, 0) if s > 0 => format!("{} refused: need {s} more salvage.", upgrade.name()),
        (0, 0) => format!(
            "{} refused: another order spent its salvage first.",
            upgrade.name()
        ),
        (s, p) => format!(
            "{} refused: need {s} salvage and {p} pressure.",
            upgrade.name()
        ),
    }
}

/// Whether a kind is one this module marks as a blocker: for tests.
#[cfg(test)]
fn blocks(kind: bw_core::Kind) -> bool {
    crate::qol::lane_holder(kind) || bw_sim::blocks_mouth_building(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{Camera, Kind};
    use bw_sim::Order;

    fn game_on(map: MapId) -> Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "brinewake-trial12-hold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let mut g = Game::new_with_data_dir(base, data);
        g.faction = Faction::Union;
        g.map = map;
        g.start();
        g.world.ai_enabled = false;
        g.intro = None;
        g.resize_view(1280, 720);
        g.selected.clear();
        g
    }

    fn place(g: &mut Game, owner: u8, kind: Kind, at: Pos) -> EntityId {
        let id = g.world.spawn_for_tests(owner, kind, at);
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
            e.build_remaining = 0;
        }
        id
    }

    #[test]
    fn the_guide_takes_its_hold_length_from_the_rule() {
        // The page's opening numbers are the rule's, on both maps.
        let three = crate::menus::guide_page_on(0, Faction::Union, MapId::Confluence).0;
        let text = three.join(" ");
        assert!(text.contains(&format!("{}S", rule_hold_seconds(MapId::Confluence, false))));
        assert!(text.contains(&format!(
            "OUT: {}S",
            rule_hold_seconds(MapId::Confluence, true)
        )));
        assert_eq!(rule_hold_seconds(MapId::Confluence, false), 75);
        assert_eq!(rule_hold_seconds(MapId::Confluence, true), 120);
        let two = crate::menus::guide_page_on(0, Faction::Union, MapId::SplitBasin).0;
        assert!(
            two.join(" ")
                .contains(&format!("{}S", rule_hold_seconds(MapId::SplitBasin, false)))
        );
        let sluice = crate::menus::guide_page_on(3, Faction::Union, MapId::SplitBasin).0;
        assert!(sluice.join(" ").contains("90S"));
        // Once a seat is out the Guide states the hold in force.
        let after = guide_lines(0, Faction::Union, MapId::Confluence, Some(120));
        assert!(
            after.iter().any(|l| l.contains("TWO LANES 120S")),
            "{after:?}"
        );
        assert!(after.iter().any(|l| l.contains("OUT: 120S")));
        let page3 = guide_lines(3, Faction::Union, MapId::Confluence, Some(120));
        assert!(page3.iter().any(|l| l.contains("120S WINS")), "{page3:?}");
        assert!(!page3.join(" ").contains("75S"));
        // At the opening value nothing changes.
        let same = guide_lines(3, Faction::Union, MapId::Confluence, Some(75));
        let (raw, _) = crate::menus::guide_page_on(3, Faction::Union, MapId::Confluence);
        assert_eq!(same, raw.iter().map(|l| l.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn a_game_carried_on_a_live_seat_reads_the_rule_in_force() {
        let mut g = game_on(MapId::Confluence);
        g.world.eliminated.push((2, 1));
        g.ux.help_page = 0;
        let model = g.menu_model();
        assert_eq!(model.hold_seconds, Some(120));
        let lines = guide_lines(
            0,
            model.selected_faction,
            model.selected_map,
            model.hold_seconds,
        );
        assert!(lines.iter().any(|l| l.contains("120S")), "{lines:?}");
    }

    /// A joined seat (no --map) on a Confluence session: the Guide says
    /// 75S and draws the three lanes.
    #[test]
    fn a_joined_confluence_seat_reads_the_confluence_guide() {
        let mut g = game_on(MapId::SplitBasin);
        assert_eq!(g.map, MapId::SplitBasin, "launched without --map");
        let (_host, guest, _third) = crate::net::tests::confluence_three(1207);
        assert_eq!(guest.local_seat, 1);
        g.start_network(guest);
        assert_eq!(g.world.map.id, MapId::Confluence);
        assert_eq!(g.map, MapId::Confluence);
        let model = g.menu_model();
        assert_eq!(model.selected_map, MapId::Confluence);
        assert_eq!(model.selected_map.layout().arm_count(), 3, "three lanes");
        let lines = guide_lines(
            0,
            model.selected_faction,
            model.selected_map,
            model.hold_seconds,
        );
        let text = lines.join(" ");
        assert!(text.contains("75S"), "{text}");
        assert!(!text.contains("90S"), "{text}");
        let sluice = guide_lines(
            3,
            model.selected_faction,
            model.selected_map,
            model.hold_seconds,
        );
        let text = sluice.join(" ");
        assert!(text.contains("DRY E / W / S"), "{text}");
        assert!(!text.contains("DRY N / DRY S"), "{text}");
        // The page draws: the three-lane diagram is chosen by the map.
        g.open_screen(crate::game::Screen::Help);
        g.ux.help_page = crate::menus::SLUICE_PAGE;
        g.render();
    }

    #[test]
    fn playback_takes_the_recordings_map() {
        let mut g = game_on(MapId::SplitBasin);
        let mut world = World::with_map(1, MapId::Confluence, &[Faction::Union; 3]).expect("world");
        world.ai_enabled = false;
        for _ in 0..30 {
            world.step();
        }
        let dir = std::env::temp_dir().join(format!("bw-t12-playback-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("m.replay.json");
        world.export_replay(&path).expect("replay");
        g.start_playback(&path, false).expect("playback");
        assert_eq!(g.map, MapId::Confluence);
    }

    fn confluence_count(g: &mut Game, counter: u8) {
        g.world.gate.owner = Some(counter);
        for lane in g.world.hold_arms(counter) {
            let mouth = g.world.own_mouth(counter, lane).expect("bank");
            place(g, counter, Kind::Reedguard, mouth);
        }
        g.world.lane_hold[usize::from(counter)] = 30 * 30;
        for _ in 0..3 {
            g.tick();
        }
        assert!(g.world.holds_every_lane(counter));
    }

    #[test]
    fn an_enemy_count_names_the_lanes_that_stop_it() {
        let mut g = game_on(MapId::Confluence);
        confluence_count(&mut g, 1);
        let lanes = g.world.hold_arms(1);
        assert_eq!(lanes.len(), 2);
        let hint = stop_hint(&g.world, 1);
        for &lane in &lanes {
            assert!(
                hint.contains(crate::seats::arm_letter(&g.world, lane)),
                "{hint}"
            );
        }
        // The lane that is only ours and the other foe's is not named.
        let other = (0..3).find(|l| !lanes.contains(l)).expect("third lane");
        assert!(
            !hint.contains(crate::seats::arm_letter(&g.world, other)),
            "{hint}"
        );
        assert!(!hint.contains("ANY"), "{hint}");
        let brief = g.hold_brief();
        assert!(!brief.contains("any crossing mouth"), "{brief}");
        assert!(brief.contains("lane"), "{brief}");
        // Those two lanes are ringed while it runs.
        assert_eq!(g.stop_ring_lanes(), Some((1, lanes.clone())));
        g.render();
        // Two seats: both lanes stop it, and the line stays true.
        let mut b = game_on(MapId::SplitBasin);
        confluence_count(&mut b, 1);
        let hint = stop_hint(&b.world, 1);
        assert_eq!(hint, "GUN AT N OR S STOPS IT");
    }

    /// Pixels a pass of the stop rings draws on a cleared canvas, with the
    /// camera on a bank of the first stop lane (or our first lane).
    fn ring_pixels(g: &mut Game, lane: usize) -> usize {
        let mouth = g.world.crossing_mouths()[lane][0];
        g.camera.center(mouth);
        g.canvas.clear([0, 0, 0, 255]);
        g.draw_stop_rings();
        let (w, h) = (g.canvas.width() as i32, g.canvas.height() as i32);
        let mut count = 0;
        for y in 0..h {
            for x in 0..w {
                if g.canvas.get(x, y) != Some([0, 0, 0, 255]) {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn the_stop_rings_draw_only_while_an_enemy_count_runs() {
        let mut g = game_on(MapId::Confluence);
        assert_eq!(g.stop_ring_lanes(), None);
        confluence_count(&mut g, 2);
        let lanes = g.stop_ring_lanes().expect("rings").1;
        assert_eq!(lanes, g.world.hold_arms(2));
        assert!(ring_pixels(&mut g, lanes[0]) > 50, "a ring and chevrons");
        // A frozen count, our own count and a broken one draw none.
        g.world.gate.flood_until = Some(g.world.tick + 300);
        assert_eq!(g.stop_ring_lanes(), None);
        assert_eq!(ring_pixels(&mut g, lanes[0]), 0);
        let mut own = game_on(MapId::Confluence);
        confluence_count(&mut own, 0);
        assert_eq!(own.stop_ring_lanes(), None);
        let lane = own.world.hold_arms(0)[0];
        assert_eq!(ring_pixels(&mut own, lane), 0);
        // The frames render with the rings and the chart marks.
        confluence_count(&mut g, 2);
        g.world.gate.flood_until = None;
        g.render();
    }

    #[test]
    fn a_foe_chip_shows_and_looks_at_what_blocks_the_lane() {
        let mut g = game_on(MapId::Confluence);
        let lane = g.world.hold_arms(0)[0];
        let their_bank = {
            let banks = g.world.arm_banks(lane);
            let other = banks.into_iter().find(|&b| b != 0).expect("a foe's bank");
            g.world.own_mouth(other, lane).expect("bank")
        };
        let owner = g
            .world
            .arm_banks(lane)
            .into_iter()
            .find(|&b| b != 0)
            .unwrap();
        let nest = place(&mut g, owner, Kind::Tower, their_bank);
        let blockers = lane_blockers(&g.world, lane);
        assert_eq!(blockers.len(), 1);
        assert!(blockers[0].nest);
        assert_eq!(blockers[0].id, nest);
        // The chip is a button of its own, with a proper card.
        g.ux.preferences.tide_card = true;
        g.render();
        let chip = g
            .buttons
            .iter()
            .find(|b| b.action == Action::FocusLane(lane))
            .cloned()
            .expect("the lane chip");
        let (title, body) = hold_card_words(&g, &chip).expect("words");
        assert_eq!(
            title,
            format!("{} LANE", crate::seats::arm_letter(&g.world, lane))
        );
        assert!(body.contains("nest"), "{body}");
        // A click goes to the nest itself.
        g.home();
        g.left_down(chip.x + chip.w / 2, chip.y + chip.h / 2);
        g.left_up(chip.x + chip.w / 2, chip.y + chip.h / 2, false);
        let mut there = Camera::default();
        there.center(g.world.entities.iter().find(|e| e.id == nest).unwrap().pos);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
        assert!(g.message.contains("nest slows your count"), "{}", g.message);
        // A machine too: two blockers, a second click moves on.
        let gun = place(&mut g, owner, Kind::Riveter, their_bank);
        assert_eq!(lane_blockers(&g.world, lane).len(), 2);
        g.action(Action::FocusLane(lane));
        there.center(g.world.entities.iter().find(|e| e.id == gun).unwrap().pos);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
        assert!(g.message.contains("machine"), "{}", g.message);
        assert!(blocks(Kind::Riveter) && blocks(Kind::Tower) && !blocks(Kind::Works));
        assert!(
            !lane_slowed(&g.world, 0, lane),
            "a gun at the bank stops the count, so it is not merely slowed"
        );
    }

    #[test]
    fn f3_on_a_broken_own_count_looks_at_the_blocker() {
        let mut g = game_on(MapId::Confluence);
        g.world.gate.owner = Some(0);
        let lanes = g.world.hold_arms(0);
        for &lane in &lanes {
            let bank = g.world.own_mouth(0, lane).expect("bank");
            place(&mut g, 0, Kind::Riveter, bank);
        }
        g.world.lane_hold[0] = 20 * 30;
        let lane = lanes[0];
        let owner = g
            .world
            .arm_banks(lane)
            .into_iter()
            .find(|&b| b != 0)
            .unwrap();
        let far = g.world.own_mouth(owner, lane).expect("bank");
        // With three seats a nest only slows a count (rules 22); a gun at
        // the far bank breaks it.
        let nest = place(&mut g, owner, Kind::Riveter, far);
        g.tick();
        assert!(!g.world.holds_lane(0, lane));
        g.home();
        g.key("F3", false, false);
        let mut there = Camera::default();
        there.center(g.world.entities.iter().find(|e| e.id == nest).unwrap().pos);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
    }

    #[test]
    fn the_hold_row_has_a_proper_card() {
        let mut g = game_on(MapId::Confluence);
        g.ux.preferences.tide_card = true;
        g.render();
        let row = g
            .buttons
            .iter()
            .find(|b| b.label == "HOLD CHECKLIST")
            .cloned()
            .expect("the hold row");
        let card = g.hover_card(&row).expect("a card");
        assert_eq!(card.title, "YOUR HOLD");
        let text = card.body.join(" ");
        assert!(
            !text.to_uppercase().contains("SHOW THE CROSSING MOUTH"),
            "{text}"
        );
        assert!(text.contains("Click"), "{text}");
    }

    #[test]
    fn alert_places_name_the_lane_and_the_bank() {
        let g = game_on(MapId::Confluence);
        let lane = g.world.hold_arms(0)[0];
        let owner = g
            .world
            .arm_banks(lane)
            .into_iter()
            .find(|&b| b != 0)
            .unwrap();
        let words = bank_words(&g.world, lane, owner);
        let letter = crate::seats::arm_letter(&g.world, lane);
        let name = crate::seats::seat_name(&g.world, owner);
        assert_eq!(words, format!("{letter} LANE, {name} BANK"));
        assert_eq!(
            bank_words(&g.world, lane, 0),
            format!("{letter} LANE, YOUR BANK")
        );
        assert!(!words.contains("'S"), "no possessive letters");
        // Plain letters the pixel font draws.
        assert!(words.is_ascii());
    }

    #[test]
    fn tied_losers_read_as_tied_and_knocked_out_seats_keep_their_order() {
        use crate::match_summary::{Loss, MatchSummary};
        let mut summary = MatchSummary::default();
        summary.factions = vec![
            Some(Faction::Union),
            Some(Faction::Assembly),
            Some(Faction::Compact),
        ];
        summary.outcome = Some((71_441, bw_sim::Outcome::Victory(1)));
        summary.tide = true;
        // Nobody out: both beaten seats share second.
        assert_eq!(summary.place_word(1).as_deref(), Some("1ST"));
        assert_eq!(summary.place_word(0).as_deref(), Some("=2ND"));
        assert_eq!(summary.place_word(2).as_deref(), Some("=2ND"));
        let (_, line, _) = crate::dock_log::three_seat_words(
            &summary,
            &bw_sim::Outcome::Victory(1),
            Some(0),
            false,
            true,
        );
        assert!(line.contains("YOU TIED FOR 2ND OF 3."), "{line}");
        // One knocked out: the order stands, no tie.
        summary.eliminated.push((2, 40_000));
        assert_eq!(summary.place_word(0).as_deref(), Some("2ND"));
        assert_eq!(summary.place_word(2).as_deref(), Some("3RD"));
        let (_, line, _) = crate::dock_log::three_seat_words(
            &summary,
            &bw_sim::Outcome::Victory(1),
            Some(0),
            false,
            true,
        );
        assert!(line.contains("YOU PLACED 2ND OF 3."), "{line}");

        // The turning point is a beaten side's worst minute, not the
        // winner's own worst loss.
        let loss = |tick: u64, owner: u8, kind: Kind| Loss {
            tick,
            owner,
            kind,
            cause: Some(Kind::Riveter),
            pos: None,
            killer: None,
        };
        for i in 0..6 {
            summary.losses.push(loss(9_000 + i * 30, 1, Kind::Loom));
        }
        for i in 0..3 {
            summary.losses.push(loss(20_000 + i * 30, 0, Kind::Riveter));
        }
        let (_, owner, ..) = summary.turning_point().expect("a turning point");
        assert_eq!(owner, 0, "the beaten side's minute");
        assert!(!summary.turning_point_is_winners());
        let line = summary.turning_line_for(Some(1)).expect("a line");
        assert!(
            line.contains("YOU LOST") || line.contains("LOST 3"),
            "{line}"
        );
        // Only the winner lost three in a minute: it stands, and is named
        // as the heaviest loss instead.
        summary.losses.retain(|l| l.owner == 1);
        assert!(summary.turning_point_is_winners());
    }

    #[test]
    fn an_upgrade_is_ordered_and_a_shortfall_names_the_resource() {
        let mut g = game_on(MapId::SplitBasin);
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .expect("hq");
        g.selected = vec![hq];
        g.world.players[0].salvage = 1_500;
        g.action(Action::Upgrade(bw_content::Upgrade::Overhaul1));
        assert_eq!(g.message, "OVERHAUL I ordered.");
        // The next level spends against what the first has spoken for.
        let short = upgrade_shortfall(&g, bw_content::Upgrade::Overhaul2).expect("short");
        assert!(short.starts_with("Need 1000 more salvage"), "{short}");
        assert!(short.contains("orders already sent"), "{short}");
        // Nothing pending: the plain shortfall.
        let mut fresh = game_on(MapId::SplitBasin);
        fresh.world.players[0].salvage = 550;
        assert_eq!(
            upgrade_shortfall(&fresh, bw_content::Upgrade::Overhaul1).as_deref(),
            Some("Need 450 more salvage.")
        );
        // A refusal the world sends back names what was short.
        g.world.players[0].salvage = 500;
        g.world.command_log.push(bw_sim::CommandRecord {
            tick: g.world.tick,
            player: 0,
            sequence: 99,
            command: bw_sim::Command::Upgrade {
                building: hq,
                upgrade: bw_content::Upgrade::Overhaul2,
            },
            accepted: true,
            applied: Some(false),
            reason: Some("insufficient resources for the upgrade at execution".into()),
        });
        let words = late_refusal_words(&g, "insufficient resources for the upgrade at execution");
        assert_eq!(words, "OVERHAUL II refused: need 1000 more salvage.");
        assert_eq!(
            late_refusal_words(&g, "building missing"),
            "building missing"
        );
    }

    #[test]
    fn the_dock_log_marks_an_upgrade_started_and_done() {
        let event = |kind| bw_sim::Event {
            tick: 10,
            kind,
            player: Some(0),
            entity: Some(1),
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: "OVERHAUL I".into(),
            cause: None,
        };
        assert_eq!(
            crate::dock_log::research_words(&event(bw_sim::EventKind::UpgradeStarted)),
            "OVERHAUL I STARTED"
        );
        assert_eq!(
            crate::dock_log::research_words(&event(bw_sim::EventKind::UpgradeCompleted)),
            "OVERHAUL I DONE"
        );
        // Through the dock itself: both lines, told apart.
        let mut world = World::new(3, Faction::Union);
        world.step();
        world.events.push(event(bw_sim::EventKind::UpgradeStarted));
        world
            .events
            .push(event(bw_sim::EventKind::UpgradeCompleted));
        let mut dock = crate::dock_log::DockState::default();
        dock.after_authoritative_tick(&world);
        let rows: Vec<String> = dock.log.entries.iter().map(|e| e.detail.clone()).collect();
        assert!(rows.iter().any(|r| r == "OVERHAUL I STARTED"), "{rows:?}");
        assert!(rows.iter().any(|r| r == "OVERHAUL I DONE"), "{rows:?}");
    }
}
