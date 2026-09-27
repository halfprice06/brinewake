//! The eleventh trial's control and selection fixes: moving machines are
//! caught where they were drawn, a group key recalls its group while an
//! enemy is inspected, F2 takes only machines with guns, workers gather a
//! wreck under enemies, Surge counts what it moves, a short batch says why
//! it is short, shift placement ends with Shift, the chart moves the camera
//! during an order, the scout keeps one key, a type row shows its remove
//! control, an A-click on an enemy attacks it and KEEP is a card toggle.
use crate::game::{Action, Game, Mode};
use bw_content::spec;
use bw_core::{EntityId, Faction, Kind, Pos};
use bw_sim::Command;

/// How long a picture's positions stay good for a click read off it: a
/// seat reads a frame, thinks and clicks while the match runs on.
const SEEN_TICKS: u64 = 300;

/// Control state the eleventh trial added: where machines stood in the
/// last picture a seat was shown, and a placement kept going by Shift.
#[derive(Clone, Debug, Default)]
pub(crate) struct ControlsState {
    seen: Option<SeenUnits>,
    /// The kind being placed with Shift held and how many sites so far.
    shift_placing: Option<(Kind, u32)>,
}

#[derive(Clone, Debug)]
struct SeenUnits {
    tick: u64,
    positions: Vec<(EntityId, Pos)>,
}

/// A machine that F2 and ARMY take: one with a gun.  Workers, Dredgers,
/// scouts, menders, transports, Salters and Pans stay where they are.
pub(crate) fn is_army_kind(kind: Kind) -> bool {
    !kind.is_worker() && !kind.is_building() && spec(kind).damage > 0
}

/// The Drydock's keys: Q the mender, W the scout (as at headquarters), E
/// the water role and R the transport, so one machine keeps one key.
pub(crate) fn drydock_keys(faction: Faction) -> [Kind; 4] {
    let [scout, mender, water, transport] = faction.drydock_roles();
    [mender, scout, water, transport]
}

/// The partial-batch line: how many went in, how many did not and why.
pub(crate) fn batch_words(name: &str, queued: usize, requested: usize, reason: &str) -> String {
    let left = requested.saturating_sub(queued);
    format!(
        "Queued {queued} of {requested} {name}; {left} not queued. {}",
        reason.trim_end()
    )
}

impl Game {
    /// Remember where each machine stands in the picture a seat is shown,
    /// so a click read off it still finds a machine that has walked on.
    pub(crate) fn note_seen(&mut self) {
        let positions = self
            .world
            .entities
            .iter()
            .filter(|e| e.hp > 0 && e.aboard.is_none() && !e.kind.is_building())
            .map(|e| (e.id, e.pos))
            .collect();
        self.controls11.seen = Some(SeenUnits {
            tick: self.world.tick,
            positions,
        });
    }

    /// An own moving machine under the pointer, as a last try after the
    /// drawn hit test missed: where it stood in the last picture, or a
    /// little wider around where it walks now.  Only own machines, and only
    /// moving ones, so a click on empty ground stays a ground click.
    pub(crate) fn pick_moving(&self, x: i32, y: i32) -> Option<EntityId> {
        let (camera, (x, y)) = self.native_pointer(x, y);
        let tick = self.world.tick;
        let seen = self
            .controls11
            .seen
            .as_ref()
            .filter(|s| tick >= s.tick && tick - s.tick <= SEEN_TICKS);
        let mut best: Option<(i64, EntityId)> = None;
        for e in self
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && !e.kind.is_building())
        {
            let then = seen.and_then(|s| {
                s.positions
                    .iter()
                    .find(|(id, _)| *id == e.id)
                    .map(|(_, pos)| *pos)
            });
            let moved = then.is_some_and(|p| p != e.pos);
            let walking = !e.path.is_empty();
            if !moved && !walking {
                continue;
            }
            let faction = self.world.players[e.owner as usize].faction;
            let mut places = vec![(e.pos, 14, 36)];
            if let Some(p) = then.filter(|_| moved) {
                places.push((p, 12, 32));
            }
            for (pos, half, height) in places {
                let t = crate::occlusion::render_transform(e.kind, pos, faction);
                let (ax, ay) = camera.project(t.anchor);
                let (sx, sy) = (ax + t.render_x_offset, ay + t.render_y_offset);
                let (half, height) = (t.scale.apply(half), t.scale.apply(height));
                if (x - sx).abs() <= half && y >= sy - height && y <= sy + t.scale.apply(6) {
                    let mid = sy - height / 2;
                    let d = i64::from(x - sx).pow(2) + i64::from(y - mid).pow(2);
                    if best.is_none_or(|(b, id)| (d, e.id) < (b, id)) {
                        best = Some((d, e.id));
                    }
                }
            }
        }
        best.map(|(_, id)| id)
    }

    /// Every machine F2 leaves out, by kind, beside the mouth keepers.
    pub(crate) fn army_left_out(&self) -> Vec<(Kind, usize)> {
        let mut out: Vec<(Kind, usize)> = Vec::new();
        for e in self.world.entities.iter().filter(|e| {
            e.owner == 0
                && e.hp > 0
                && e.aboard.is_none()
                && !e.kind.is_building()
                && !e.kind.is_worker()
                && !is_army_kind(e.kind)
        }) {
            match out.iter_mut().find(|(k, _)| *k == e.kind) {
                Some((_, n)) => *n += 1,
                None => out.push((e.kind, 1)),
            }
        }
        out.sort_by_key(|(k, _)| *k);
        out
    }

    /// The F2 line: how many guns, and what stays behind.
    pub(crate) fn army_words(&self, count: usize, keepers: usize) -> String {
        let mut stay: Vec<String> = self
            .army_left_out()
            .iter()
            .map(|(k, n)| format!("{n} {}", k.name()))
            .collect();
        if keepers > 0 {
            stay.push(format!("{keepers} at a crossing mouth"));
        }
        let head = format!(
            "{count} combat machine{} selected.",
            if count == 1 { "" } else { "s" }
        );
        if stay.is_empty() {
            head
        } else {
            format!("{head} Staying: {}.", stay.join(", "))
        }
    }

    /// A right-click with only gatherers selected on a wreck under an
    /// enemy: they gather it (trial 11: workers took an attack order,
    /// then a repair on an enemy wall, and walked into the enemy army).
    pub(crate) fn gather_under_enemy(
        &self,
        x: i32,
        y: i32,
        queued: bool,
    ) -> Option<(Command, &'static str, Pos)> {
        let mobiles = self.gameplay_ids(|k| !k.is_building());
        let gatherers = self.gameplay_ids(|k| k.gathers());
        if gatherers.is_empty() || gatherers.len() != mobiles.len() {
            return None;
        }
        let resource = self.resource_under_pointer(x, y)?;
        let pos = self
            .world
            .map
            .resources
            .iter()
            .find(|r| r.id == resource)?
            .pos;
        let command = if queued {
            Command::QueueGather {
                units: gatherers,
                resource,
            }
        } else {
            Command::Gather {
                units: gatherers,
                resource,
            }
        };
        Some((command, "Gather", pos))
    }

    /// The selected machines Surge leaves out, by reason: deployed or
    /// changing, on cooldown or running, or without a gun.
    pub(crate) fn surge_left_out(&self) -> Vec<String> {
        let mut deployed = 0;
        let mut cooling = 0;
        let mut unarmed = 0;
        for e in self.world.entities.iter().filter(|e| {
            e.owner == 0
                && e.hp > 0
                && e.aboard.is_none()
                && !e.kind.is_building()
                && self.selected.contains(&e.id)
        }) {
            if e.kind.is_worker() || spec(e.kind).damage == 0 {
                unarmed += 1;
            } else if e.deployed || e.deploy_remaining > 0 {
                deployed += 1;
            } else if e.surge_remaining > 0 || e.surge_cooldown > 0 {
                cooling += 1;
            }
        }
        [
            (deployed, "deployed"),
            (cooling, "cooling down"),
            (unarmed, "without guns"),
        ]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, why)| format!("{n} {why}"))
        .collect()
    }

    /// Surge's price line: the machines it moves out of those selected,
    /// the price, and which stay (trial 11: "8 selected = 80P" with 11
    /// selected).
    pub(crate) fn surge_words(&self) -> String {
        let able = self.surge_able_count();
        let total = self
            .selected
            .iter()
            .filter(|id| {
                self.world.entities.iter().any(|e| {
                    e.id == **id
                        && e.owner == 0
                        && e.hp > 0
                        && e.aboard.is_none()
                        && !e.kind.is_building()
                })
            })
            .count();
        let price = format!(
            "{} P per machine: {able} of {total} surge = {} P.",
            bw_content::SURGE_PRESSURE,
            self.surge_selection_cost()
        );
        let out = self.surge_left_out();
        if out.is_empty() {
            price
        } else {
            format!("{price} Left out: {}.", out.join(", "))
        }
    }

    /// Shift was let go: a placement Shift kept going ends there, as it
    /// does in other strategy games.
    pub(crate) fn shift_released(&mut self) {
        if let Some((kind, placed)) = self.controls11.shift_placing.take()
            && self.mode == Mode::Build(kind)
        {
            self.mode = Mode::Context;
            self.guard_escape();
            self.notify(&format!(
                "{placed} {} site{} placed. Shift released: placing ended.",
                crate::ux::building_name(kind, self.world.players[0].faction),
                if placed == 1 { "" } else { "s" }
            ));
        }
    }

    /// A placement starts from its key or button: no Shift run yet.
    pub(crate) fn start_placing(&mut self) {
        self.controls11.shift_placing = None;
    }

    /// A click in attack mode on a seen enemy: an attack on it.  False
    /// when the click is on ground, or Shift queues waypoints, which stay
    /// attack-moves.
    pub(crate) fn attack_click(&mut self, x: i32, y: i32) -> bool {
        if self.ux.pointer_shift {
            return false;
        }
        let Some(target) = self.attack_click_target(x, y) else {
            return false;
        };
        let units = self.gameplay_ids(|k| !k.is_building());
        if units.is_empty() {
            return false;
        }
        self.issue(Command::Attack { units, target });
        true
    }

    /// A placement after one made with Shift: count it, and keep going
    /// only while Shift is held.
    pub(crate) fn note_placed(&mut self, kind: Kind) {
        if self.mode == Mode::Build(kind) && self.ux.pointer_shift {
            let placed = match self.controls11.shift_placing {
                Some((k, n)) if k == kind => n + 1,
                _ => 1,
            };
            self.controls11.shift_placing = Some((kind, placed));
        } else {
            self.controls11.shift_placing = None;
        }
    }

    /// A click without Shift during a placement Shift kept going: Shift
    /// was let go before it, so the placement is over and the click places
    /// nothing (trial 11: a plain click after two shift-clicks placed a
    /// third nest).  True when the click was taken here.
    pub(crate) fn plain_click_ends_shift_placing(&mut self) -> bool {
        if self.ux.pointer_shift
            || !matches!(self.controls11.shift_placing, Some((k, _)) if self.mode == Mode::Build(k))
        {
            return false;
        }
        self.shift_released();
        true
    }

    /// Placement or another order waits for a target: a click on the chart
    /// looks there and keeps the order waiting (trial 11: during a
    /// placement the chart did nothing).
    pub(crate) fn chart_click_during_order(&mut self, x: i32, y: i32) {
        self.ux.minimap_drag = true;
        self.minimap_click(x, y);
    }

    /// An A-click on a seen enemy: attack that one.  A deployed gun with it
    /// in reach stays deployed and fires, which an attack-move to its
    /// ground would not do (trial 11: the beam turned off).
    pub(crate) fn attack_click_target(&self, x: i32, y: i32) -> Option<EntityId> {
        let id = self
            .unit_at(x, y)
            .or_else(|| self.enemy_on_ground(self.unproject(x, y)))?;
        self.world
            .entities
            .iter()
            .any(|e| e.id == id && e.owner != 0 && e.hp > 0 && self.world.entity_visible(0, e.id))
            .then_some(id)
    }

    /// Group recall with an enemy inspected: the panel goes back to the
    /// group even when the group is the selection it was opened over.
    pub(crate) fn recall_ends_inspection(&mut self) {
        if self.inspected.take().is_some() && self.message.starts_with("ENEMY ") {
            self.message.clear();
        }
    }

    /// Whether an order would pack a deployed gun that is not kept
    /// deployed: a move, attack-move or capture packs every one; an attack
    /// packs those without the target in reach.
    pub(crate) fn order_packs(&self, command: &Command) -> bool {
        let (units, target) = match command {
            Command::Move { units, .. }
            | Command::AttackMove { units, .. }
            | Command::Capture { units } => (units, None),
            Command::Attack { units, target } => (units, Some(*target)),
            _ => return false,
        };
        match target {
            Some(target) => self.attack_pack_note(units, target).contains("PACKED"),
            None => self.world.entities.iter().any(|e| {
                units.contains(&e.id)
                    && crate::controls::is_specialist(e.kind)
                    && (e.deployed || e.deploy_remaining > 0)
                    && !e.keep_deployed
            }),
        }
    }

    /// The prompt row before a click: the order, and a warning when it
    /// will pack deployed guns (trial 11: attack orders packed the
    /// Compact's Heliostat line three times).
    pub(crate) fn order_hint(&self, command: &Command, verb: &str) -> String {
        if self.order_packs(command) {
            format!("RIGHT CLICK: {verb} / DEPLOYED GUNS PACK, K KEEPS THEM")
        } else {
            format!("RIGHT CLICK: {verb}")
        }
    }

    /// The attack-move prompt, warning when deployed guns will pack.
    pub(crate) fn attack_mode_prompt(&self) -> String {
        let units = self.gameplay_ids(|k| !k.is_building());
        let packs = self.order_packs(&Command::AttackMove {
            units,
            target: Pos::default(),
            queued: false,
        });
        match (self.ux.pointer_shift, packs) {
            (true, _) => "ATTACK-MOVE / SHIFT+CLICK ADDS WAYPOINTS / ESC CANCELS".into(),
            (false, true) => {
                "ATTACK / ON AN ENEMY GUNS IN REACH STAY DEPLOYED / ON GROUND THEY PACK".into()
            }
            (false, false) => {
                "ATTACK-MOVE / CLICK A DESTINATION / SHIFT QUEUES / ESC CANCELS".into()
            }
        }
    }

    /// The KEEP toggle for the command card: its hint row names the state.
    pub(crate) fn keep_hint(&self) -> &'static str {
        if self.keeps_deployed() {
            "K\nKEPT"
        } else {
            "K\nOFF"
        }
    }

    /// Each type row of a mixed selection gets a small remove control in
    /// its corner: the "-" a shift-click stood for unseen.
    pub(crate) fn push_remove_control(&mut self, kind: Kind, cell_x: i32, cell_y: i32, cell: i32) {
        let s = self.ui_scale();
        let size = 9 * s;
        self.buttons.push(crate::game::Button {
            x: cell_x + cell - size,
            y: cell_y,
            w: size,
            h: size,
            label: "-".into(),
            hint: String::new(),
            action: Action::FilterSelection(kind, true),
            enabled: true,
        });
    }

    /// Draw a remove control: a dark square with a bar, gold when hovered.
    pub(crate) fn draw_remove_control(&mut self, b: &crate::game::Button, hover: bool) {
        use crate::canvas::{EDGE, GOLD, INK, WHITE};
        let s = self.ui_scale();
        self.canvas.rect(b.x, b.y, b.w, b.h, INK);
        self.canvas
            .frame(b.x, b.y, b.w, b.h, if hover { GOLD } else { EDGE });
        self.canvas.rect(
            b.x + 2 * s,
            b.y + b.h / 2 - s / 2,
            b.w - 4 * s,
            s.max(1),
            if hover { GOLD } else { WHITE },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock_log::ResultPage;
    use bw_sim::{Order, Outcome};
    use std::path::PathBuf;

    fn game(faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "brinewake-trial11-controls-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut g = Game::new_with_data_dir(base, data);
        g.faction = faction;
        g.opponent = Some(crate::game::other_faction(faction));
        g.start();
        g.intro = None;
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        g.ux.preferences.pause_unfocused = false;
        g.world.revealed = true;
        g.selected.clear();
        g.cursor = (-999, -999);
        g
    }

    fn unit(g: &mut Game, owner: u8, kind: Kind, cell: (i32, i32)) -> u32 {
        let id = g
            .world
            .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
        }
        id
    }

    fn entity(g: &Game, id: u32) -> &bw_sim::Entity {
        g.world.entities.iter().find(|e| e.id == id).unwrap()
    }

    fn worker(g: &Game) -> u32 {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap()
            .id
    }

    fn click(g: &mut Game, (x, y): (i32, i32), shift: bool) {
        g.pointer_moved(x, y);
        g.ux.pointer_shift = shift;
        g.left_down(x, y);
        g.left_up(x, y, shift);
    }

    /// A screen point that picks `id` where it stands now.
    fn point_on(g: &Game, id: u32) -> (i32, i32) {
        let (cx, cy) = g.project(entity(g, id).pos);
        for dy in 0..40 {
            for dx in [0, -2, 2, -4, 4] {
                let p = (cx + dx, cy - dy);
                if g.unit_at(p.0, p.1) == Some(id) {
                    return p;
                }
            }
        }
        panic!("no point picks {id}");
    }

    #[test]
    fn a_click_on_the_picture_catches_a_machine_that_walked_on() {
        let mut g = game(Faction::Union);
        let caisson = unit(&mut g, 0, Kind::Caisson, (40, 70));
        g.camera.center(Pos::cell(42, 70));
        g.render();
        let seen_at = point_on(&g, caisson);
        g.note_seen();
        // It walks three cells on before the click lands.
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == caisson) {
            e.pos = Pos::cell(43, 70);
            e.path = vec![Pos::cell(46, 70)];
        }
        g.render();
        assert_ne!(point_on(&g, caisson), seen_at);
        click(&mut g, seen_at, false);
        assert_eq!(g.selected, vec![caisson], "the click finds it");

        // Without the picture's positions the old spot is empty ground.
        g.controls11 = ControlsState::default();
        g.selected.clear();
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == caisson) {
            e.path.clear();
        }
        click(&mut g, seen_at, false);
        assert!(
            g.selected.is_empty(),
            "a still machine keeps its drawn bounds"
        );
        // Well clear of any machine the ground stays ground.
        g.note_seen();
        let (fx, fy) = g.project(Pos::cell(52, 60));
        click(&mut g, (fx, fy), false);
        assert!(g.selected.is_empty());
    }

    #[test]
    fn a_group_key_recalls_its_group_while_an_enemy_is_inspected() {
        let mut g = game(Faction::Union);
        let a = unit(&mut g, 0, Kind::Riveter, (40, 70));
        let b = unit(&mut g, 0, Kind::Riveter, (41, 70));
        let enemy = unit(&mut g, 1, Kind::Brander, (44, 70));
        g.camera.center(Pos::cell(42, 70));
        g.selected = vec![a, b];
        g.key("2", false, true);
        g.render();
        let at = point_on(&g, enemy);
        click(&mut g, at, false);
        assert!(g.inspected_enemy().is_some(), "the enemy is inspected");
        g.key("2", false, false);
        assert!(g.inspected_enemy().is_none(), "2 brings the group back");
        assert_eq!(g.selected, vec![a, b]);
        assert!(
            !g.native_prompt().contains("ENEMY"),
            "{}",
            g.native_prompt()
        );
    }

    #[test]
    fn f2_takes_only_machines_with_guns_and_names_what_stays() {
        let mut g = game(Faction::Compact);
        let guns = [
            unit(&mut g, 0, Kind::Brander, (40, 70)),
            unit(&mut g, 0, Kind::Heliostat, (41, 70)),
            unit(&mut g, 0, Kind::Glinter, (42, 70)),
        ];
        for kind in [Kind::Pan, Kind::Salter, Kind::Glazier, Kind::Stilt] {
            unit(&mut g, 0, kind, (40, 72));
        }
        g.key("F2", false, false);
        let mut want = guns.to_vec();
        want.sort_unstable();
        assert_eq!(g.selected, want);
        assert_eq!(g.army_ids().len(), 3, "the ARMY count agrees");
        let prompt = g.native_prompt().to_uppercase();
        for word in ["3 COMBAT MACHINES", "PAN", "SALTER", "GLAZIER", "STILT"] {
            assert!(prompt.contains(word), "{prompt}");
        }
    }

    #[test]
    fn workers_gather_a_wreck_an_enemy_stands_on() {
        let mut g = game(Faction::Compact);
        let wreck = g.world.map.resources[0].clone();
        let (cx, cy) = wreck.pos.cell_xy();
        let enemy = unit(&mut g, 1, Kind::Riveter, (cx, cy));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == enemy) {
            e.pos = wreck.pos;
        }
        g.camera.center(wreck.pos);
        g.render();
        let hand = worker(&g);
        g.selected = vec![hand];
        let (x, y) = g.project(wreck.pos);
        let order = g.context_order(x, y, false).expect("an order");
        assert!(
            matches!(&order.0, bw_sim::Command::Gather { resource, .. } if *resource == wreck.id),
            "{:?}",
            order.0
        );
        // An armed machine in the selection still attacks.
        let gun = unit(&mut g, 0, Kind::Brander, (cx - 3, cy));
        g.selected = vec![hand, gun];
        let order = g.context_order(x, y, false).expect("an order");
        assert!(
            matches!(order.0, bw_sim::Command::Attack { .. }),
            "{:?}",
            order.0
        );
    }

    #[test]
    fn surge_counts_what_it_moves_and_names_the_rest() {
        let mut g = game(Faction::Compact);
        let mut ids = vec![
            unit(&mut g, 0, Kind::Brander, (40, 70)),
            unit(&mut g, 0, Kind::Brander, (41, 70)),
            unit(&mut g, 0, Kind::Glinter, (42, 70)),
        ];
        let helio = unit(&mut g, 0, Kind::Heliostat, (43, 70));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == helio) {
            e.deployed = true;
        }
        ids.push(helio);
        ids.push(unit(&mut g, 0, Kind::Glazier, (44, 70)));
        g.selected = ids;
        let words = g.surge_words();
        assert!(words.contains("3 of 5 surge = 30 P"), "{words}");
        assert!(words.contains("1 deployed"), "{words}");
        assert!(words.contains("1 without guns"), "{words}");
        assert!(
            g.action_description(&Action::Surge)
                .contains("3 of 5 surge")
        );
    }

    #[test]
    fn a_short_batch_says_how_many_went_in_and_why_the_rest_did_not() {
        let mut g = game(Faction::Compact);
        let works = unit(&mut g, 0, Kind::Works, (30, 60));
        let cost = spec(Kind::Heliostat);
        g.world.players[0].salvage = 5000;
        g.world.players[0].pressure = cost.pressure + 5;
        g.selected = vec![works];
        g.train_selection(Kind::Heliostat, 5);
        assert!(
            g.message
                .starts_with("Queued 1 of 5 HELIOSTAT; 4 not queued. Need"),
            "{}",
            g.message
        );
        assert!(g.message.contains("pressure"), "{}", g.message);
    }

    /// Clear spots for a nest near headquarters, as screen points.
    fn nest_spots(g: &mut Game, n: usize) -> Vec<(i32, i32)> {
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        g.camera.center(hq);
        g.render();
        let mut spots: Vec<(i32, i32)> = Vec::new();
        for dy in (-10..=10).step_by(4) {
            for dx in (-10..=10).step_by(4) {
                let (x, y) = g.project(Pos::raw(hq.x + dx * bw_core::FP, hq.y + dy * bw_core::FP));
                if !g.world_pointer_allowed(x, y) || g.unit_at(x, y).is_some() {
                    continue;
                }
                let origin = g.build_origin(Kind::Tower, g.unproject(x, y));
                if g.placement_reason(Kind::Tower, origin).is_none() {
                    spots.push((x, y));
                }
                if spots.len() == n {
                    return spots;
                }
            }
        }
        panic!("only {} clear spots", spots.len());
    }

    fn sites(g: &Game) -> usize {
        g.placed_sites
            .iter()
            .filter(|(k, _, _)| *k == Kind::Tower)
            .count()
    }

    #[test]
    fn shift_placement_ends_when_shift_is_let_go() {
        let mut g = game(Faction::Compact);
        g.world.players[0].salvage = 5000;
        let spots = nest_spots(&mut g, 6);
        let hand = worker(&g);
        g.selected = vec![hand];
        g.action(Action::Build(Kind::Tower));
        click(&mut g, spots[0], true);
        click(&mut g, spots[2], true);
        assert_eq!(sites(&g), 2);
        assert_eq!(g.mode, Mode::Build(Kind::Tower), "Shift keeps placing");
        // The next click comes without Shift: Shift was let go before it.
        click(&mut g, spots[4], false);
        assert_eq!(sites(&g), 2, "no third site");
        assert_eq!(g.mode, Mode::Context);
        assert!(g.message.starts_with("2 "), "{}", g.message);

        // In a window the release itself ends it.  (Forget the sites so
        // far, so the check against a second site on one spot stays out.)
        g.placed_sites.clear();
        g.selected = vec![hand];
        g.action(Action::Build(Kind::Tower));
        click(&mut g, spots[4], true);
        assert_eq!(g.mode, Mode::Build(Kind::Tower));
        g.ux.pointer_shift = false;
        g.shift_released();
        assert_eq!(g.mode, Mode::Context);

        // A placement begun afresh takes a plain click as ever.
        g.selected = vec![hand];
        // (The sites so far are forgotten, so no second-site check asks.)
        g.placed_sites.clear();
        g.action(Action::Build(Kind::Tower));
        click(&mut g, spots[5], false);
        assert_eq!(sites(&g), 1, "{}", g.message);
        assert_eq!(g.mode, Mode::Context);
    }

    #[test]
    fn the_chart_moves_the_camera_during_a_placement() {
        let mut g = game(Faction::Compact);
        let hand = worker(&g);
        g.selected = vec![hand];
        g.action(Action::Build(Kind::Works));
        g.render();
        let chart = g.minimap_bounds();
        let before = g.camera;
        let (x, y) = (chart.x + chart.w / 4, chart.y + chart.h / 2);
        g.left_down(x, y);
        g.left_up(x, y, false);
        assert_ne!(
            (g.camera.x, g.camera.y),
            (before.x, before.y),
            "the camera moved"
        );
        assert_eq!(g.mode, Mode::Build(Kind::Works), "the placement waits");
    }

    #[test]
    fn the_whole_dock_log_tab_turns_the_page() {
        let mut g = game(Faction::Assembly);
        g.world.outcome = Some(Outcome::Victory(0));
        g.render();
        let tab = g
            .buttons
            .iter()
            .find(|b| b.action == Action::ResultPage(ResultPage::Log))
            .cloned()
            .expect("tab");
        // The right end of the tab, well past its words.
        let (x, y) = (tab.x + tab.w - 3, tab.y + tab.h / 2);
        g.pointer_moved(x, y);
        g.left_down(x, y);
        g.left_up(x, y, false);
        assert_eq!(g.result_page, ResultPage::Log);
    }

    #[test]
    fn a_mixed_selection_shows_a_remove_control_on_each_type() {
        let mut g = game(Faction::Compact);
        let a = unit(&mut g, 0, Kind::Brander, (40, 70));
        let b = unit(&mut g, 0, Kind::Heliostat, (41, 70));
        g.selected = vec![a, b];
        g.render();
        let minus = g
            .buttons
            .iter()
            .find(|b| b.action == Action::FilterSelection(Kind::Heliostat, true))
            .cloned()
            .expect("a remove control");
        let row = g
            .buttons
            .iter()
            .find(|b| b.action == Action::FilterSelection(Kind::Heliostat, false))
            .cloned()
            .expect("the row");
        assert!(minus.x >= row.x && minus.x + minus.w <= row.x + row.w && minus.y == row.y);
        let (x, y) = (minus.x + minus.w / 2, minus.y + minus.h / 2);
        g.pointer_moved(x, y);
        g.left_down(x, y);
        g.left_up(x, y, false);
        assert_eq!(g.selected, vec![a], "the Heliostats are removed");
        // One type alone has nothing to remove.
        g.render();
        assert!(
            !g.buttons
                .iter()
                .any(|b| matches!(b.action, Action::FilterSelection(_, true)))
        );
    }

    #[test]
    fn the_scout_is_w_at_headquarters_and_at_the_drydock() {
        for faction in [Faction::Union, Faction::Assembly, Faction::Compact] {
            let mut g = game(faction);
            let hq = g
                .world
                .entities
                .iter()
                .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
                .unwrap()
                .id;
            let dock = unit(&mut g, 0, Kind::Drydock, (30, 60));
            g.selected = vec![hq];
            assert_eq!(g.production_hotkey_kind("W"), Some(faction.scout()));
            g.selected = vec![dock];
            assert_eq!(g.production_hotkey_kind("W"), Some(faction.scout()));
            assert_eq!(g.production_hotkey_for(faction.scout()), Some("W"));
            let mender = faction.drydock_roles()[1];
            assert_eq!(g.production_hotkey_kind("Q"), Some(mender));
            assert_eq!(g.production_hotkey_for(mender), Some("Q"));
            // The card's key reads the same.
            g.render();
            let train_w = g
                .buttons
                .iter()
                .find(|b| b.action == Action::Train(faction.scout()))
                .expect("scout button");
            assert_eq!(crate::lean_hud::button_key(train_w).as_deref(), Some("W"));
            assert_eq!(
                crate::menus::roster_train(faction, faction.scout()),
                "W AT HQ\nW AT DRYDOCK"
            );
        }
    }

    #[test]
    fn an_a_click_on_an_enemy_in_reach_keeps_a_heliostat_firing() {
        let mut g = game(Faction::Compact);
        let helio = unit(&mut g, 0, Kind::Heliostat, (40, 70));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == helio) {
            e.deployed = true;
            e.order = Order::Idle;
        }
        let enemy = unit(&mut g, 1, Kind::Riveter, (44, 70));
        g.camera.center(Pos::cell(42, 70));
        g.selected = vec![helio];
        g.render();
        let at = point_on(&g, enemy);
        g.action(Action::Attack);
        click(&mut g, at, false);
        assert!(
            matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(bw_sim::Command::Attack { target, .. }) if *target == enemy
            ),
            "an attack on the enemy, not an attack-move"
        );
        for _ in 0..20 {
            g.tick();
        }
        assert!(entity(&g, helio).deployed, "still deployed");
    }

    #[test]
    fn keep_is_a_toggle_on_the_card() {
        let mut g = game(Faction::Compact);
        let helio = unit(&mut g, 0, Kind::Heliostat, (40, 70));
        g.selected = vec![helio];
        g.render();
        let keep = |g: &Game| {
            g.buttons
                .iter()
                .find(|b| b.action == Action::KeepDeployed)
                .cloned()
                .expect("KEEP on the card")
        };
        let b = keep(&g);
        assert_eq!(crate::lean_hud::state_chip(&b.hint).unwrap().0, "OFF");
        g.key("K", false, false);
        for _ in 0..4 {
            g.tick();
        }
        g.render();
        let b = keep(&g);
        assert_eq!(crate::lean_hud::state_chip(&b.hint).unwrap().0, "KEPT");
        assert_eq!(crate::lean_hud::button_key(&b).as_deref(), Some("K"));
    }

    #[test]
    fn the_prompt_warns_before_an_order_packs_deployed_guns() {
        let mut g = game(Faction::Compact);
        let helio = unit(&mut g, 0, Kind::Heliostat, (40, 70));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == helio) {
            e.deployed = true;
            e.order = Order::Idle;
        }
        g.camera.center(Pos::cell(42, 70));
        g.selected = vec![helio];
        g.render();
        let ground = g.project(Pos::cell(48, 64));
        g.pointer_moved(ground.0, ground.1);
        assert!(
            g.native_prompt().contains("DEPLOYED GUNS PACK"),
            "{}",
            g.native_prompt()
        );
        g.action(Action::Attack);
        assert!(
            g.native_prompt().contains("THEY PACK"),
            "{}",
            g.native_prompt()
        );
        g.key("Escape", false, false);
        // Kept deployed, nothing packs and nothing warns.
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == helio) {
            e.keep_deployed = true;
        }
        g.mode = Mode::Context;
        g.message.clear();
        assert!(!g.native_prompt().contains("PACK"), "{}", g.native_prompt());
    }

    #[test]
    fn batch_words_read_whole() {
        assert_eq!(
            batch_words("HELIOSTAT", 1, 5, "Need 30 more pressure. "),
            "Queued 1 of 5 HELIOSTAT; 4 not queued. Need 30 more pressure."
        );
        assert!(is_army_kind(Kind::Brander) && !is_army_kind(Kind::Pan));
        assert!(!is_army_kind(Kind::Glazier) && !is_army_kind(Kind::Salter));
    }
}
