//! Unit orders and controls asked for in the eighth trial (rules 14): keys
//! that select every worker or every building of a kind, DEPLOY and PACK as
//! separate orders with a keep-deployed switch, rallies for every selected
//! building, and placement that shows where a site can go, says when a
//! worker's site is queued, and refuses an accidental second site.
use crate::canvas::{GOLD, JADE};
use crate::game::{Game, Mode};
use bw_content::spec;
use bw_core::{Kind, Pos};
use bw_sim::{Command, Order};

/// How long a placed site counts as just placed: long enough to cover a
/// two-player match's order delay and a player's second look.
pub(crate) const PLACED_TICKS: u64 = 150;
/// A second site of the same kind this close to one just placed asks for a
/// second click first.
const DOUBLE_SITE_CELLS: i32 = 4;
/// How far around the pointer buildable ground is shaded while placing.
const SHADE_CELLS: i32 = 9;
/// How long after a placement ends or is refused a lone Escape only says
/// so, rather than opening the menu: three seconds.
pub(crate) const ESCAPE_GUARD_TICKS: u64 = 90;
/// How far past a crossing mouth's ring a building's ghost shows the ring.
const MOUTH_NOTICE_CELLS: i32 = 4;

/// Screen diamonds to shade (x, y, colour) and PLACED label points.
pub(crate) type PlacementHelp = Vec<(i32, i32, [u8; 4])>;

/// Ctrl and the key that builds a kind selects every one of them; Ctrl+H
/// selects the headquarters.
pub(crate) fn building_hotkey_kind(key: &str) -> Option<Kind> {
    Some(match key {
        "B" => Kind::Works,
        "C" => Kind::Condenser,
        "Y" => Kind::Dropoff,
        "V" => Kind::Tower,
        "N" => Kind::Drydock,
        "P" => Kind::Palisade,
        "H" => Kind::Headquarters,
        _ => return None,
    })
}

/// A machine that deploys and packs: the Bulwark, Loom and Caisson, and
/// the Compact's Heliostat and Pan.
pub(crate) fn is_specialist(kind: Kind) -> bool {
    bw_sim::is_specialist(kind)
}

impl Game {
    /// After a site is placed: with shift held, placement goes on while
    /// there is salvage for another (trial 10: a second nest ended it);
    /// otherwise it ends, and an Escape just after it does not open the
    /// menu.
    pub(crate) fn after_placement(&mut self, kind: Kind) {
        if self.ux.pointer_shift
            && self
                .action_reason(&crate::game::Action::Build(kind))
                .is_none()
        {
            self.mode = Mode::Build(kind);
            return;
        }
        self.guard_escape();
    }

    /// For a moment, a lone Escape does not open the menu.
    pub(crate) fn guard_escape(&mut self) {
        self.escape_guard_until = self.world.tick.saturating_add(ESCAPE_GUARD_TICKS);
    }

    /// Whether this Escape only follows a placement that ended or was
    /// refused a moment ago.
    pub(crate) fn escape_guarded(&self) -> bool {
        self.screen == crate::game::Screen::Match
            && self.mode == Mode::Context
            && self.world.outcome.is_none()
            && self.world.tick < self.escape_guard_until
    }

    /// The crossing mouth a building's ghost stands near while placing:
    /// its lane, the mouth, and whether the site's middle is inside the
    /// ring.  Buildings never count for the hold (`holds_mouth_unit` in
    /// the simulation); the ghost says so rather than letting a nest look
    /// like a holder.
    pub(crate) fn placement_mouth(&self, kind: Kind, origin: Pos) -> Option<(usize, Pos, bool)> {
        let n = spec(kind).footprint.max(1);
        let (x, y) = origin.cell_xy();
        let middle = Pos {
            x: Pos::cell(x, y).x + (n - 1) * bw_core::FP / 2,
            y: Pos::cell(x, y).y + (n - 1) * bw_core::FP / 2,
        };
        let ring = bw_content::CROSSING_HOLD_RADIUS_CELLS * bw_core::FP;
        let notice = ring + MOUTH_NOTICE_CELLS * bw_core::FP;
        self.world
            .crossing_mouths()
            .iter()
            .enumerate()
            .flat_map(|(lane, mouths)| mouths.iter().map(move |m| (lane, *m)))
            .map(|(lane, mouth)| (mouth.distance_sq(middle), lane, mouth))
            .filter(|(d, _, _)| *d <= i64::from(notice).pow(2))
            .min_by_key(|(d, lane, mouth)| (*d, *lane, mouth.x, mouth.y))
            .map(|(d, lane, mouth)| (lane, mouth, d <= i64::from(ring).pow(2)))
    }

    /// What stands on a footprint and refuses it: own machines on their
    /// feet step off a new site, so it is a building, a deployed machine
    /// or an enemy.  In trial 10 an OCCUPIED site inside a crowd of own
    /// workers read as the workers' fault; the ghost now names the nest
    /// under them.
    pub(crate) fn placement_occupant(&self, kind: Kind, origin: Pos) -> Option<String> {
        let (x, y) = origin.cell_xy();
        let n = spec(kind).footprint.max(1);
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.hp > 0
                    && e.aboard.is_none()
                    && (e.owner == 0 || self.world.entity_visible(0, e.id))
                    && !matches!(spec(e.kind).movement, bw_core::Movement::Air)
                    && (e.owner != 0
                        || e.kind.is_building()
                        || e.deployed
                        || e.deploy_remaining > 0)
            })
            .find(|e| {
                let (ex, ey) = e.pos.cell_xy();
                let size = spec(e.kind).footprint.max(1);
                ex < x + n && x < ex + size && ey < y + n && y < ey + size
            })
            .map(|e| {
                let whose = if e.owner == 0 { "YOUR" } else { "ENEMY" };
                let state = if e.build_remaining > 0 {
                    " SITE"
                } else if !e.kind.is_building() {
                    " (DEPLOYED)"
                } else {
                    ""
                };
                let name = match e.kind {
                    Kind::Tower => "NEST",
                    Kind::Dropoff => "YARD",
                    k => k.name(),
                };
                format!("{whose} {name}{state}").to_uppercase()
            })
    }

    /// Why no selected machine could walk to `target`, or None when one
    /// can: an order across a deep lane says so instead of doing nothing
    /// (trial 10), and names that lane (trial 12).
    pub(crate) fn no_way_there(&self, units: &[u32], target: Pos) -> Option<String> {
        if units.is_empty() || self.world.any_can_reach(units, target) {
            return None;
        }
        Some(self.no_way_words(|world| world.any_can_reach(units, target)))
    }

    /// Whether a worker is on a site or saving sites for later.
    pub(crate) fn building_now(&self, id: u32) -> bool {
        self.world.entities.iter().any(|e| {
            e.id == id
                && (!e.build_queue.is_empty()
                    || matches!(e.order, Order::Build { target } if self
                        .world
                        .entities
                        .iter()
                        .any(|s| s.id == target && s.hp > 0 && s.build_remaining > 0)))
        })
    }

    /// Deployed machines in `units`: whether any is kept deployed and
    /// whether any will pack for the order.
    fn deployed_split(
        &self,
        units: &[u32],
        goes: impl Fn(&bw_sim::Entity) -> bool,
    ) -> (bool, bool) {
        let deployed = self.world.entities.iter().filter(|e| {
            units.contains(&e.id) && is_specialist(e.kind) && (e.deployed || e.deploy_remaining > 0)
        });
        deployed.fold((false, false), |(kept, packs), e| {
            (
                kept || e.keep_deployed,
                packs || (!e.keep_deployed && goes(e)),
            )
        })
    }

    fn pack_words(kept: bool, packs: bool) -> &'static str {
        match (kept, packs) {
            (true, true) => " KEPT ONES STAY; REST PACKED.",
            (true, false) => " Kept-deployed stay.",
            (false, true) => crate::trial12_orders::PACKED_NOTE,
            (false, false) => "",
        }
    }

    /// What CAPTURE does with deployed machines (rules 19): they pack
    /// first and go, unless kept deployed.
    pub(crate) fn capture_pack_note(&self, units: &[u32]) -> &'static str {
        let (kept, packs) = self.deployed_split(units, |_| true);
        Self::pack_words(kept, packs)
    }

    /// What an ATTACK does with deployed guns (rules 19): one with the
    /// target out of its reach packs and goes, unless kept deployed.
    pub(crate) fn attack_pack_note(&self, units: &[u32], target: u32) -> &'static str {
        let Some(aim) = self
            .world
            .entities
            .iter()
            .find(|e| e.id == target)
            .map(|e| e.pos)
        else {
            return "";
        };
        let out_of_reach = |e: &bw_sim::Entity| {
            e.pos.distance_sq(aim) > i64::from(self.world.weapon_range(e)).pow(2)
        };
        let (kept, packs) = self.deployed_split(units, out_of_reach);
        let kept = kept
            && self
                .world
                .entities
                .iter()
                .any(|e| units.contains(&e.id) && e.keep_deployed && e.deployed && out_of_reach(e));
        Self::pack_words(kept, packs)
    }

    /// Whether the selection holds a machine that deploys.
    pub(crate) fn selected_specialists(&self) -> bool {
        !self.gameplay_ids(is_specialist).is_empty()
    }

    /// Whether every selected specialist is kept deployed.
    pub(crate) fn keeps_deployed(&self) -> bool {
        let ids = self.gameplay_ids(is_specialist);
        !ids.is_empty()
            && ids.iter().all(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && e.keep_deployed)
            })
    }

    /// Select `ids`; the same key again, with nothing changed, centres on
    /// them.
    fn select_all_of(&mut self, ids: Vec<u32>) {
        let mut sorted = ids;
        sorted.sort_unstable();
        let again = !sorted.is_empty() && sorted == self.selected;
        // The selection panel shows the count, so the prompt row stays
        // quiet.
        self.select_recovered(sorted, again);
    }

    /// F7: every worker, idle or not.
    pub(crate) fn select_workers(&mut self) {
        let ids: Vec<u32> = self
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        if ids.is_empty() {
            self.notify("No workers. Train them at the headquarters with Q.");
            return;
        }
        self.select_all_of(ids);
    }

    /// Ctrl and a build key: every own building of that kind, sites too.
    pub(crate) fn select_buildings(&mut self, kind: Kind) {
        let ids: Vec<u32> = self
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.kind == kind)
            .map(|e| e.id)
            .collect();
        let name = match kind {
            Kind::Tower => "defense nests",
            Kind::Dropoff => "salvage yards",
            Kind::Headquarters => "headquarters",
            // WORKS is already plural: "2 workss selected" (ninth trial).
            Kind::Works => "works",
            _ => "",
        };
        let plural = if name.is_empty() {
            format!("{}S", kind.name())
        } else {
            name.to_string()
        };
        if ids.is_empty() {
            self.notify(&format!("You have no {}.", plural.to_lowercase()));
            return;
        }
        self.select_all_of(ids);
    }

    /// Set the rally of every selected building that trains, not just the
    /// first.  True when there was one.
    pub(crate) fn rally_selected(&mut self, pos: Pos) -> bool {
        let buildings: Vec<u32> = self
            .selected
            .iter()
            .copied()
            .filter(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && e.owner == 0 && e.hp > 0 && e.kind.produces())
            })
            .collect();
        if buildings.is_empty() {
            return false;
        }
        let count = buildings.len();
        let mut set = 0;
        for building in buildings {
            if self.issue_accepted(Command::Rally { building, pos }) {
                set += 1;
            }
        }
        if count > 1 && set > 0 {
            self.notify(&format!("Rally set for {set} buildings."));
        }
        true
    }

    /// The worker to build a site, and whether the site waits behind the
    /// one it is on: a worker already building (or given a site a moment
    /// ago, before the order shows) keeps its first site and queues this.
    pub(crate) fn builder_and_queue(&self, site: Pos) -> Option<(u32, bool)> {
        let worker = self.builder_for(site)?;
        let tick = self.world.tick;
        let busy = self.world.entities.iter().any(|e| {
            e.id == worker
                && matches!(e.order, Order::Build { target } if self
                    .world
                    .entities
                    .iter()
                    .any(|s| s.id == target && s.hp > 0 && s.build_remaining > 0))
        }) || self
            .recent_builders
            .iter()
            .any(|(id, at)| *id == worker && tick.saturating_sub(*at) < 90);
        Some((worker, busy || self.ux.pointer_shift))
    }

    /// A site of `kind` placed a moment ago near `origin`: its order may
    /// not have reached the field yet, or it has only just.
    fn recent_site_near(&self, kind: Kind, origin: Pos) -> bool {
        let (x, y) = origin.cell_xy();
        let near = |pos: Pos| {
            let (px, py) = pos.cell_xy();
            (px - x).abs().max((py - y).abs()) <= DOUBLE_SITE_CELLS
        };
        let tick = self.world.tick;
        self.placed_sites
            .iter()
            .any(|(k, pos, at)| *k == kind && near(*pos) && tick.saturating_sub(*at) < PLACED_TICKS)
    }

    /// Whether a click that places `kind` at `origin` should first be
    /// confirmed: one visible click once gave two salvage yards.  The
    /// second click on the same spot places it.
    pub(crate) fn confirm_double_site(&mut self, kind: Kind, origin: Pos) -> bool {
        if !self.recent_site_near(kind, origin) {
            self.double_site = None;
            return false;
        }
        if self.double_site == Some((kind, origin)) {
            self.double_site = None;
            return false;
        }
        self.double_site = Some((kind, origin));
        self.notify(&format!(
            "A {} was just placed here. Click again to place a second.",
            crate::ux::building_name(kind, self.world.players[0].faction)
        ));
        true
    }

    /// Remember a site this side has ordered, until it stands on the field.
    pub(crate) fn remember_placed_site(&mut self, kind: Kind, pos: Pos) {
        self.placed_sites.push((kind, pos, self.world.tick));
    }

    /// Placed sites whose order has not reached the field yet.
    pub(crate) fn pending_sites(&self) -> Vec<(Kind, Pos)> {
        let tick = self.world.tick;
        self.placed_sites
            .iter()
            .filter(|(kind, pos, at)| {
                tick.saturating_sub(*at) < PLACED_TICKS
                    && !self
                        .world
                        .entities
                        .iter()
                        .any(|e| e.owner == 0 && e.kind == *kind && e.pos == *pos)
            })
            .map(|(kind, pos, _)| (*kind, *pos))
            .collect()
    }

    pub(crate) fn prune_placed_sites(&mut self) {
        let tick = self.world.tick;
        self.placed_sites
            .retain(|(_, _, at)| tick.saturating_sub(*at) < PLACED_TICKS && *at <= tick);
    }

    /// While placing: the buildable ground around the pointer to shade,
    /// and sites already ordered but not yet on the field, as screen
    /// diamonds and labels.
    pub(crate) fn placement_help(&self, camera: bw_core::Camera, origin: Pos) -> PlacementHelp {
        let mut diamonds = Vec::new();
        let Mode::Build(kind) = self.mode else {
            return diamonds;
        };
        let (cx, cy) = origin.cell_xy();
        let n = spec(kind).footprint;
        if kind != Kind::Condenser {
            for dy in -SHADE_CELLS..=SHADE_CELLS + n {
                for dx in -SHADE_CELLS..=SHADE_CELLS + n {
                    let cell = Pos::cell(cx + dx, cy + dy);
                    if self.placement_cell_refusal(cell, true).is_none() {
                        let (px, py) = camera.project(cell);
                        diamonds.push((px, py, [JADE[0], JADE[1], JADE[2], 64]));
                    }
                }
            }
        }
        for (pending, pos) in self.pending_sites() {
            let (x, y) = pos.cell_xy();
            let size = spec(pending).footprint;
            for a in 0..size {
                for b in 0..size {
                    let (qx, qy) = camera.project(Pos::cell(x + a, y + b));
                    diamonds.push((qx, qy, [GOLD[0], GOLD[1], GOLD[2], 80]));
                }
            }
        }
        diamonds
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Action, Game, Mode};
    use bw_core::{Kind, Pos};
    use bw_sim::{Command, Order, SiteBuilder};
    use std::path::PathBuf;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn place(g: &mut Game, owner: u8, kind: Kind, cell: (i32, i32)) -> u32 {
        let id = g
            .world
            .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
        }
        id
    }

    fn own_hq(g: &Game) -> (u32, Pos) {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| (e.id, e.pos))
            .expect("hq")
    }

    #[test]
    fn f7_selects_every_worker_and_ctrl_build_keys_select_buildings() {
        let mut g = game();
        let workers: Vec<u32> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        assert!(!workers.is_empty());
        g.key("F7", false, false);
        assert_eq!(g.selected, {
            let mut w = workers.clone();
            w.sort_unstable();
            w
        });
        g.key("H", false, true);
        assert_eq!(g.selected, vec![own_hq(&g).0]);
        g.key("B", false, true);
        assert_eq!(g.selected, vec![own_hq(&g).0], "no Works: selection kept");
    }

    #[test]
    fn d_deploys_p_packs_and_k_keeps_deployed() {
        let mut g = game();
        let (_, hq) = own_hq(&g);
        let (x, y) = hq.cell_xy();
        let bulwark = place(&mut g, 0, Kind::Bulwark, (x + 6, y + 6));
        g.selected = vec![bulwark];
        g.key("D", false, false);
        for _ in 0..40 {
            g.tick();
        }
        let e = |g: &Game| {
            g.world
                .entities
                .iter()
                .find(|e| e.id == bulwark)
                .cloned()
                .unwrap()
        };
        assert!(e(&g).deployed);
        g.key("D", false, false);
        g.tick();
        assert!(e(&g).deployed, "a second D does not pack");
        g.key("K", false, false);
        g.tick();
        assert!(e(&g).keep_deployed);
        assert!(g.keeps_deployed());
        g.key("P", false, false);
        for _ in 0..40 {
            g.tick();
        }
        assert!(!e(&g).deployed, "P packs");
    }

    #[test]
    fn a_right_click_sets_the_rally_of_every_selected_building() {
        let mut g = game();
        let (hq, pos) = own_hq(&g);
        let (x, y) = pos.cell_xy();
        let dock = place(&mut g, 0, Kind::Drydock, (x + 8, y));
        g.selected = vec![hq, dock];
        let target = Pos::cell(x + 3, y + 10);
        assert!(g.rally_selected(target));
        g.tick();
        for id in [hq, dock] {
            assert_eq!(
                g.world.entities.iter().find(|e| e.id == id).unwrap().rally,
                Some(target)
            );
        }
    }

    #[test]
    fn a_second_site_for_a_busy_worker_is_queued_and_labelled() {
        let mut g = game();
        g.world.players[0].salvage = 2000;
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        let (_, hq) = own_hq(&g);
        let (x, y) = hq.cell_xy();
        let mut sites = Vec::new();
        'find: for dy in -10..10 {
            for dx in -10..10 {
                let pos = Pos::cell(x + dx, y + dy);
                if g.placement_reason(Kind::Palisade, pos).is_none()
                    && sites
                        .iter()
                        .all(|p: &Pos| (p.cell_xy().0 - pos.cell_xy().0).abs() > 5)
                {
                    sites.push(pos);
                    if sites.len() == 2 {
                        break 'find;
                    }
                }
            }
        }
        for site in &sites {
            let (w, queued) = g.builder_and_queue(*site).expect("builder");
            assert_eq!(w, worker);
            g.issue(Command::Build {
                worker: w,
                kind: Kind::Palisade,
                pos: *site,
                queued,
            });
            g.recent_builders.push((w, g.world.tick));
            g.tick();
        }
        let ids: Vec<u32> = sites
            .iter()
            .map(|p| {
                g.world
                    .entities
                    .iter()
                    .find(|e| e.kind == Kind::Palisade && e.pos == *p)
                    .unwrap()
                    .id
            })
            .collect();
        assert_eq!(g.world.site_builder(ids[0]), SiteBuilder::Building);
        assert_eq!(g.world.site_builder(ids[1]), SiteBuilder::Queued);
    }

    #[test]
    fn a_second_site_of_the_same_kind_right_beside_the_first_asks_again() {
        let mut g = game();
        let origin = Pos::cell(30, 60);
        g.remember_placed_site(Kind::Dropoff, origin);
        assert!(g.confirm_double_site(Kind::Dropoff, Pos::cell(31, 61)));
        assert!(!g.confirm_double_site(Kind::Dropoff, Pos::cell(31, 61)));
        assert!(!g.confirm_double_site(Kind::Works, Pos::cell(31, 61)));
        assert_eq!(g.pending_sites().len(), 1);
        let _ = Action::SelectWorkers;
        let _ = Mode::Context;
    }
}

/// Trial 10: the sluice card and alert cards no longer take world clicks,
/// Escape after a placement stays in the match, workers resume a site
/// under a crowd, shift keeps placing and queues gathers, the ghost shows
/// the mouth ring, orders across deep water say so, the pump art takes a
/// condenser, and an enemy can be read.
#[cfg(test)]
mod trial10_tests {
    use crate::game::{Action, Game, Mode, Screen};
    use bw_content::spec;
    use bw_core::{FP, Kind, Pos, Terrain};
    use bw_sim::{Command, Order};
    use std::path::PathBuf;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game.render();
        game
    }

    fn worker(g: &Game) -> u32 {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .expect("a worker")
    }

    fn hq(g: &Game) -> Pos {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .expect("hq")
    }

    /// Dry open ground `dx`, `dy` cells from the headquarters, or the
    /// nearest such cell.
    fn ground(g: &Game, dx: i32, dy: i32) -> Pos {
        let (x, y) = hq(g).cell_xy();
        for r in 0..8 {
            for oy in -r..=r {
                for ox in -r..=r {
                    let pos = Pos::cell(x + dx + ox, y + dy + oy);
                    if g.placement_reason(Kind::Palisade, pos).is_none()
                        && matches!(
                            g.world.map.terrain(x + dx + ox, y + dy + oy),
                            Terrain::Salt | Terrain::Silt
                        )
                    {
                        return pos;
                    }
                }
            }
        }
        panic!("no open ground near the headquarters");
    }

    /// Free sites for a palisade near the headquarters, far apart.
    fn palisade_sites(g: &Game, n: usize) -> Vec<Pos> {
        let (x, y) = hq(g).cell_xy();
        let mut sites: Vec<Pos> = Vec::new();
        for dy in -10..10 {
            for dx in -10..10 {
                let pos = Pos::cell(x + dx, y + dy);
                if g.placement_reason(Kind::Palisade, pos).is_none()
                    && sites.iter().all(|p| {
                        let (a, b) = (p.cell_xy(), pos.cell_xy());
                        (a.0 - b.0).abs().max((a.1 - b.1).abs()) > 5
                    })
                {
                    sites.push(pos);
                    if sites.len() == n {
                        return sites;
                    }
                }
            }
        }
        panic!("no room for {n} palisades");
    }

    fn at(g: &mut Game, pos: Pos) -> (i32, i32) {
        g.camera.center(pos);
        g.render();
        g.project(pos)
    }

    fn pos_of(g: &Game, id: u32) -> Pos {
        g.world
            .entities
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.pos)
            .expect("entity")
    }

    #[test]
    fn the_sluice_card_opens_from_the_gauge_and_never_from_the_field() {
        let mut g = game();
        // Unpinned (it starts pinned open since trial 11).
        g.ux.preferences.tide_card = false;
        g.render();
        let gauge = g.tide_gauge_bounds().expect("gauge");
        let card = g.tide_card_rect();
        let under = (card.x + card.w / 2, card.y + card.h / 2);
        // Trial 10: crossing the upper middle of the field opened the card
        // and it took the clicks meant for the ground.
        g.pointer_moved(under.0, under.1);
        g.render();
        assert!(!g.tide_card_shown());
        assert!(g.world_pointer_allowed(under.0, under.1));
        // From the gauge it opens, and stays while the pointer goes down
        // onto it.
        g.pointer_moved(gauge.x + gauge.w / 2, gauge.y + gauge.h / 2);
        g.render();
        assert!(g.tide_card_shown());
        g.pointer_moved(under.0, under.1);
        g.render();
        assert!(g.tide_card_shown(), "the card stays under the pointer");
        g.pointer_moved(10, 400);
        g.render();
        assert!(!g.tide_card_shown());
        g.pointer_moved(under.0, under.1);
        g.render();
        assert!(
            !g.tide_card_shown(),
            "and does not come back from the field"
        );
    }

    #[test]
    fn a_pinned_card_lets_orders_and_right_clicks_through() {
        let mut g = game();
        g.ux.preferences.tide_card = true;
        g.render();
        let card = g.route_bounds();
        assert!(card.h > 0, "pinned open");
        let (px, py) = (card.x + 12, card.y + card.h - 6);
        assert!(!g.world_pointer_allowed(px, py), "left clicks look at it");
        // An order waiting for its click: the card stands aside.
        let at_ground = ground(&g, 6, 6);
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, at_ground);
        g.selected = vec![riveter];
        g.action(Action::Attack);
        assert_eq!(g.mode, Mode::Attack);
        g.render();
        assert!(!g.tide_card_shown());
        assert!(g.world_pointer_allowed(px, py));
        g.key("Escape", false, false);
        g.render();
        assert!(g.tide_card_shown());
        // A right-click on the card's body is an order for the ground under
        // it (or says why none can go there).
        let before = g.world.command_log.len();
        g.message.clear();
        g.right_click(px, py, false);
        assert!(
            g.world.command_log.len() > before || g.message.contains("No way"),
            "the right-click was swallowed: {}",
            g.message
        );
    }

    #[test]
    fn an_alert_card_lets_a_placement_click_through() {
        let mut g = game();
        let site = palisade_sites(&g, 1)[0];
        let (x, y) = at(&mut g, site);
        // An alert card standing right where the site is clicked.
        g.buttons.push(crate::game::Button {
            x: x - 20,
            y: y - 10,
            w: 40,
            h: 20,
            label: "ENEMY SEEN".into(),
            hint: String::new(),
            action: Action::FocusAlert(None),
            enabled: true,
        });
        g.selected = vec![worker(&g)];
        g.world.players[0].salvage = 2000;
        g.mode = Mode::Build(Kind::Palisade);
        let camera = (g.camera.x, g.camera.y);
        assert!(g.world_pointer_allowed(x, y));
        g.left_down(x, y);
        assert_eq!((g.camera.x, g.camera.y), camera, "no jump to the alert");
        assert!(
            g.world.command_log.iter().any(|r| matches!(
                r.command,
                Command::Build {
                    kind: Kind::Palisade,
                    ..
                }
            )),
            "the placement landed: {}",
            g.message
        );
    }

    #[test]
    fn escape_after_a_refused_or_finished_placement_stays_in_the_match() {
        let mut g = game();
        g.selected = vec![worker(&g)];
        g.world.players[0].salvage = 0;
        g.action(Action::Build(Kind::Tower));
        assert_eq!(g.mode, Mode::Context, "refused: no salvage");
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Match, "trial 10: this opened the menu");
        assert!(g.message.contains("Esc again"), "{}", g.message);
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Pause, "a second Escape opens it");
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Match);
        // Shift keeps placing; a plain click after it (Shift let go)
        // ends it without another site (trial 11: a third nest), and the
        // Escape right after does not open the menu.
        g.world.players[0].salvage = 2000;
        let sites = palisade_sites(&g, 2);
        g.action(Action::Build(Kind::Palisade));
        let (x, y) = at(&mut g, sites[0]);
        g.ux.pointer_shift = true;
        g.left_down(x, y);
        assert_eq!(g.mode, Mode::Build(Kind::Palisade), "shift keeps placing");
        g.ux.pointer_shift = false;
        let (x, y) = at(&mut g, sites[1]);
        g.left_down(x, y);
        assert_eq!(g.mode, Mode::Context);
        let built = g
            .world
            .command_log
            .iter()
            .filter(|r| matches!(r.command, Command::Build { .. }))
            .count();
        assert_eq!(built, 1, "{}", g.message);
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Match);
        // The guard lapses after a moment.
        for _ in 0..=super::ESCAPE_GUARD_TICKS {
            g.tick();
        }
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Pause);
    }

    /// An own site with nobody on it, and a crowd of workers standing on it.
    fn abandoned_site(g: &mut Game) -> (u32, Vec<u32>) {
        let (x, y) = palisade_sites(g, 1)[0].cell_xy();
        let site = g.world.spawn_for_tests(0, Kind::Tower, Pos::cell(x, y));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == site) {
            e.build_remaining = spec(Kind::Tower).build_ticks / 2;
            e.hp = e.max_hp / 3;
            e.builder = None;
        }
        let crowd: Vec<u32> = (0..6)
            .map(|i| {
                let id = g
                    .world
                    .spawn_for_tests(0, Kind::Hook, Pos::cell(x + i % 2, y + i / 3));
                if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
                    e.order = Order::Idle;
                }
                id
            })
            .collect();
        (site, crowd)
    }

    #[test]
    fn a_right_click_with_workers_resumes_a_site_under_a_crowd() {
        let mut g = game();
        let (site, crowd) = abandoned_site(&mut g);
        g.selected = crowd.clone();
        let site_pos = pos_of(&g, site);
        let middle = Pos {
            x: site_pos.x + FP / 2,
            y: site_pos.y + FP / 2,
        };
        let (x, y) = at(&mut g, middle);
        g.cursor = (x, y);
        let (command, verb, _) = g.context_order(x, y, false).expect("an order");
        assert_eq!(verb, "Resume", "trial 10: this read MOVE");
        assert!(matches!(command, Command::Repair { target, .. } if target == site));
        assert_eq!(
            g.pointer_kind((0, 0)),
            crate::pointer::PointerKind::Interact
        );
        g.right_click(x, y, false);
        assert!(g.message.contains("resumed"), "{}", g.message);
        // A finished, damaged building reads REPAIR.
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == site) {
            e.build_remaining = 0;
        }
        let (_, verb, _) = g.context_order(x, y, false).expect("an order");
        assert_eq!(verb, "Repair");
    }

    #[test]
    fn own_workers_step_off_a_site_and_a_refused_one_names_what_is_there() {
        let mut g = game();
        let (x, y) = palisade_sites(&g, 1)[0].cell_xy();
        // A crowd of own workers on open ground does not refuse a nest.
        let crowd: Vec<u32> = (0..4)
            .map(|i| {
                g.world
                    .spawn_for_tests(0, Kind::Hook, Pos::cell(x + i % 2, y + i / 2))
            })
            .collect();
        let origin = Pos::cell(x, y);
        assert_eq!(g.placement_reason(Kind::Tower, origin), None);
        {
            g.world.players[0].salvage = 2000;
            assert!(g.issue_accepted(Command::Build {
                worker: crowd[0],
                kind: Kind::Tower,
                pos: origin,
                queued: false,
            }));
            g.tick();
            g.tick();
            let (sx, sy) = origin.cell_xy();
            for id in &crowd[1..] {
                let (wx, wy) = pos_of(&g, *id).cell_xy();
                assert!(
                    !((sx..sx + 2).contains(&wx) && (sy..sy + 2).contains(&wy)),
                    "own workers step off the site"
                );
            }
            // Now the site itself is what refuses a second nest there, and
            // the ghost says so instead of blaming the crowd.
            assert_eq!(
                g.placement_occupant(Kind::Tower, origin).as_deref(),
                Some("YOUR NEST SITE")
            );
        }
    }

    #[test]
    fn shift_with_a_gather_keeps_a_builder_on_its_site() {
        let mut g = game();
        let w = worker(&g);
        g.selected = vec![w];
        let home = hq(&g);
        let wreck = g
            .world
            .map
            .resources
            .iter()
            .filter(|r| r.remaining > 0 && r.kind == bw_sim::ResourceKind::Salvage)
            .min_by_key(|r| r.pos.distance_sq(home))
            .cloned()
            .expect("a wreck");
        let (x, y) = at(&mut g, wreck.pos);
        assert_eq!(g.resource_under_pointer(x, y), Some(wreck.id));
        g.mode = Mode::Gather;
        g.ux.pointer_shift = true;
        g.left_down(x, y);
        assert!(
            matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(Command::QueueGather { resource, .. }) if *resource == wreck.id
            ),
            "{:?}",
            g.world.command_log.last()
        );
        g.ux.pointer_shift = false;
        let (command, _, _) = g.context_order(x, y, true).expect("gather");
        assert!(matches!(command, Command::QueueGather { .. }));
        let (command, _, _) = g.context_order(x, y, false).expect("gather");
        assert!(matches!(command, Command::Gather { .. }));
    }

    #[test]
    fn the_ghost_near_a_mouth_shows_the_ring_and_that_buildings_do_not_hold() {
        let mut g = game();
        let mouth = g.world.crossing_mouths()[0][0];
        let (mx, my) = mouth.cell_xy();
        let inside = g.placement_mouth(Kind::Tower, Pos::cell(mx, my));
        assert!(matches!(inside, Some((0, _, true))), "{inside:?}");
        let ring = bw_content::CROSSING_HOLD_RADIUS_CELLS;
        let outside = (0..8)
            .flat_map(|d| {
                [
                    Pos::cell(mx + ring + 1 + d, my),
                    Pos::cell(mx - ring - 2 - d, my),
                    Pos::cell(mx, my + ring + 1 + d),
                    Pos::cell(mx, my - ring - 2 - d),
                ]
            })
            .find(|p| g.placement_mouth(Kind::Tower, *p).is_some_and(|m| !m.2));
        assert!(outside.is_some(), "just outside the ring still shows it");
        // A good site in the ring says so on the ghost.
        let good = (-ring..=ring)
            .flat_map(|dy| (-ring..=ring).map(move |dx| Pos::cell(mx + dx, my + dy)))
            .find(|p| {
                g.placement_reason(Kind::Palisade, *p).is_none()
                    && g.placement_mouth(Kind::Palisade, *p).is_some_and(|m| m.2)
            });
        assert!(good.is_some(), "a free site inside the ring");
        if let Some(site) = good {
            g.selected = vec![worker(&g)];
            g.world.players[0].salvage = 2000;
            g.mode = Mode::Build(Kind::Palisade);
            let (x, y) = at(&mut g, site);
            g.cursor = (x, y);
            g.render();
            let tip = g.hover_tip.as_ref().expect("the ghost's words");
            assert!(tip.card.title.contains("MOUTH RING"), "{}", tip.card.title);
            assert!(
                tip.card
                    .reason
                    .as_deref()
                    .is_some_and(|r| r.contains("don't hold")),
                "{:?}",
                tip.card.reason
            );
        }
    }

    #[test]
    fn an_order_across_deep_water_says_so_and_is_not_sent() {
        let mut g = game();
        let start = ground(&g, 6, 6);
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, start);
        g.selected = vec![riveter];
        g.world.gate.tide = bw_sim::Tide::Flood;
        let enemy_hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 1 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .expect("enemy hq");
        assert!(!g.world.any_can_reach(&[riveter], enemy_hq));
        let before = g.world.command_log.len();
        g.mode = Mode::Attack;
        g.attack_destination(enemy_hq);
        assert_eq!(
            g.world.command_log.len(),
            before,
            "no order that does nothing"
        );
        assert!(g.message.contains("lanes are deep"), "{}", g.message);
        assert_eq!(g.mode, Mode::Attack, "another point may be picked");
    }

    #[test]
    fn an_enemy_on_the_pointed_ground_is_the_target() {
        let mut g = game();
        let own = g.world.spawn_for_tests(0, Kind::Riveter, ground(&g, 6, 6));
        let foe = g.world.spawn_for_tests(1, Kind::Riveter, ground(&g, 8, 6));
        g.tick();
        g.selected = vec![own];
        let foe_pos = pos_of(&g, foe);
        assert_eq!(
            g.enemy_on_ground(Pos {
                x: foe_pos.x + FP * 3 / 4,
                y: foe_pos.y + FP / 2,
            }),
            Some(foe),
            "trial 10: a right-click beside a distant enemy was a plain move"
        );
        let (x, y) = at(&mut g, foe_pos);
        let (command, verb, _) = g.context_order(x, y + 4, false).expect("an order");
        assert_eq!(verb, "Attack");
        assert!(matches!(command, Command::Attack { target, .. } if target == foe));
    }

    #[test]
    fn a_click_on_the_pump_takes_its_well() {
        let mut g = game();
        let well = g.world.map.wells[0];
        let (x, y) = at(&mut g, well);
        // A point on the pump's picture above its site, where the ground
        // under the pointer is more than two cells from the well.
        let (wx, wy) = well.cell_xy();
        let mut point = None;
        'find: for dy in -90..-4 {
            for dx in -24..=24 {
                g.cursor = (x + dx, y + dy);
                let ground = g.unproject(x + dx, y + dy).cell_xy();
                if g.well_art_under_pointer() == Some(well)
                    && (ground.0 - wx).abs().max((ground.1 - wy).abs()) > 2
                {
                    point = Some((x + dx, y + dy));
                    break 'find;
                }
            }
        }
        let (px, py) = point.expect("the pump stands above ground two cells off");
        g.cursor = (px, py);
        assert_eq!(
            g.build_origin(Kind::Condenser, g.unproject(px, py)),
            well,
            "trial 10: a click on the pump art did nothing"
        );
    }

    #[test]
    fn a_click_on_an_enemy_reads_it_in_the_panel() {
        let mut g = game();
        let own = g.world.spawn_for_tests(0, Kind::Riveter, ground(&g, 6, 6));
        let foe = g.world.spawn_for_tests(1, Kind::Loom, ground(&g, 8, 6));
        g.tick();
        g.selected = vec![own];
        let foe_pos = pos_of(&g, foe);
        let (x, y) = at(&mut g, foe_pos);
        let (x, y) = (x, y - 6);
        assert_eq!(g.unit_at(x, y), Some(foe));
        g.left_down(x, y);
        g.left_up(x, y, false);
        assert_eq!(g.selected, vec![own], "the own selection keeps its orders");
        assert_eq!(g.inspected_enemy().map(|e| e.id), Some(foe));
        g.render();
        g.key("Escape", false, false);
        assert!(g.inspected_enemy().is_none());
        assert_eq!(g.screen, Screen::Match);
        // A new selection ends the reading too.
        g.left_down(x, y);
        g.left_up(x, y, false);
        assert!(g.inspected_enemy().is_some());
        g.selected.clear();
        assert!(g.inspected_enemy().is_none());
    }
}
