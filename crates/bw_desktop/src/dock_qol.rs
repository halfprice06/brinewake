//! Selection and production presentation; all commands still use the simulation.
use crate::game::{Game, Mode};
use bw_content::spec;
use bw_core::{EntityId, Kind};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RosterEntry {
    pub kind: Kind,
    pub count: usize,
}

pub(crate) fn composition(kinds: impl IntoIterator<Item = Kind>) -> Vec<RosterEntry> {
    let mut rows: Vec<RosterEntry> = Vec::new();
    for kind in kinds {
        if let Some(row) = rows.iter_mut().find(|r| r.kind == kind) {
            row.count += 1;
        } else {
            rows.push(RosterEntry { kind, count: 1 });
        }
    }
    rows.sort_by_key(|r| r.kind as u8);
    rows
}

pub(crate) fn short_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Headquarters => "HQ",
        Kind::Dropoff => "YARD",
        Kind::Tower => "DEFENSE",
        _ => kind.name(),
    }
}

pub(crate) fn composition_label(rows: &[RosterEntry]) -> String {
    rows.iter()
        .map(|r| format!("{} {}", r.count, short_name(r.kind)))
        .collect::<Vec<_>>()
        .join(" / ")
}

pub(crate) struct ProductionInfo {
    pub heading: String,
    pub status: String,
    pub fraction: (u32, u32),
    /// The kinds in the queue, the running one first, for the icon row.
    pub queued: Vec<Kind>,
}

impl Game {
    pub(crate) fn selection_roster(&self) -> Vec<RosterEntry> {
        composition(self.gameplay_ids(|_| true).iter().filter_map(|id| {
            self.world
                .entities
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.kind)
        }))
    }

    pub(crate) fn filter_selection(&mut self, kind: Kind, remove: bool) {
        self.selected = self.gameplay_ids(|k| (k == kind) != remove);
        self.selected.sort_unstable();
        self.selected.dedup();
        self.mode = Mode::Context;
        self.drag = None;
        self.ux.minimap_drag = false;
        self.last_click = None;
        self.last_group = None;
        // The roster page keeps its place; the per-frame clamp handles a
        // page that no longer exists.
        self.notify(&format!(
            "{} {} / {} selected.",
            if remove { "Removed" } else { "Selected" },
            kind.name(),
            self.selected.len()
        ));
        self.update_tutorial();
    }

    pub(crate) fn live_group(&self, n: usize) -> Vec<EntityId> {
        let mut ids: Vec<_> = self
            .groups
            .get(n)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && e.owner == 0 && e.hp > 0 && e.aboard.is_none())
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(crate) fn group_description(&self, n: usize) -> String {
        let ids = self.live_group(n);
        let summary = composition_label(&composition(ids.iter().filter_map(|id| {
            self.world
                .entities
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.kind)
        })));
        format!(
            "{}. Click or {n}: recall. Shift: center. Ctrl+{n}: store the selection.",
            if summary.is_empty() {
                "Empty group"
            } else {
                &summary
            }
        )
    }

    pub(crate) fn selected_production_info(&self) -> Option<ProductionInfo> {
        let id = *self.selected.first()?;
        let e = self.world.entities.iter().find(|e| {
            e.id == id
                && e.owner == 0
                && e.hp > 0
                && e.aboard.is_none()
                && e.build_remaining == 0
                && e.kind.produces()
        })?;
        if self.selected.len() != 1 {
            return None;
        }
        let research = self.world.players[0]
            .research
            .as_ref()
            .filter(|r| r.building == id);
        let first = e.queue.first();
        // An order is on its way to the world for a few ticks (and longer
        // in a two-player match): the panel counts it at once, so a queue
        // key never answers QUEUE EMPTY.
        let pending = crate::production_qol::pending_trains(self, id);
        let queued: Vec<Kind> = e
            .queue
            .iter()
            .map(|p| p.kind)
            .chain(pending.iter().copied())
            .collect();
        let heading = queued.first().map_or_else(
            || "QUEUE EMPTY".into(),
            |kind| format!("{} / {} QUEUED", kind.name(), queued.len()),
        );
        let status = if let Some(r) = research {
            // A doctrine pauses the headquarters' training; the queue
            // stays and says so.
            if queued.is_empty() {
                format!("RESEARCH {}S / TRAINING PAUSED", r.remaining.div_ceil(30))
            } else {
                format!(
                    "RESEARCH {}S / {} WAIT, PAUSED",
                    r.remaining.div_ceil(30),
                    queued.len()
                )
            }
        } else if let Some(job) = e.upgrade {
            format!("{} {}S", job.upgrade.name(), job.remaining.div_ceil(30))
        } else if let Some(p) = first {
            if !p.started {
                "PAID / WAITING TO START".into()
            } else if p.remaining <= 1 {
                if self.world.players[0].crew.saturating_add(spec(p.kind).crew)
                    > self.world.players[0].cap
                {
                    "READY / CREW LIMIT".into()
                } else if self.exit_blocked(id) {
                    "EXIT BLOCKED / CLEAR THE DOOR".into()
                } else {
                    "FINISHING / CHECKING EXIT".into()
                }
            } else {
                format!("{}S REMAINING", p.remaining.div_ceil(30))
            }
        } else if !pending.is_empty() {
            "ORDER SENT / STARTING".into()
        } else {
            String::new()
        };
        let fraction = first.map_or((0, 1), |p| {
            (
                spec(p.kind).build_ticks.saturating_sub(p.remaining),
                spec(p.kind).build_ticks.max(1),
            )
        });
        Some(ProductionInfo {
            heading,
            status,
            fraction,
            queued,
        })
    }

    pub(crate) fn selected_queue_summary(&self) -> String {
        let ids = self.gameplay_ids(|k| matches!(k, Kind::Headquarters | Kind::Works));
        let rows = composition(
            self.world
                .entities
                .iter()
                .filter(|e| ids.contains(&e.id))
                .flat_map(|e| e.queue.iter().map(|p| p.kind)),
        );
        if rows.is_empty() {
            String::new()
        } else {
            format!("QUEUED: {}", composition_label(&rows))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Action, Screen};
    use bw_core::{Faction, Pos};
    use bw_sim::{Command, Production};
    use std::path::PathBuf;

    fn game(name: &str) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("brinewake-dock-{name}-{}", std::process::id())),
        );
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        g.cursor = (-999, -999);
        g.message.clear();
        g
    }

    #[test]
    fn roster_filters_only_living_local_members_without_orders_or_camera_jumps() {
        let mut g = game("filter");
        let own: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0)
            .map(|e| e.id)
            .collect();
        g.selected = own.clone();
        g.groups[3] = own.clone();
        g.selected.extend(
            g.world
                .entities
                .iter()
                .filter(|e| e.owner != 0)
                .map(|e| e.id),
        );
        g.selected.push(u32::MAX);
        let dead = own[1];
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == dead)
            .unwrap()
            .hp = 0;
        let expected = g.gameplay_ids(|k| k.is_worker());
        let hash = g.world.state_hash();
        let camera = g.camera;
        let log = g.world.command_log.len();
        g.mode = Mode::Attack;
        g.drag = Some((400, 400));
        g.ux.minimap_drag = true;
        g.action(Action::FilterSelection(Faction::Union.worker(), false));
        assert_eq!(g.selected, expected);
        assert_eq!(g.groups[3], own);
        assert_eq!(g.mode, Mode::Context);
        assert!(g.drag.is_none());
        assert!(!g.ux.minimap_drag);
        assert_eq!(
            g.camera.project(Pos::cell(20, 60)),
            camera.project(Pos::cell(20, 60))
        );
        assert_eq!(g.world.state_hash(), hash);
        assert_eq!(g.world.command_log.len(), log);
        g.action(Action::FilterSelection(Faction::Union.worker(), true));
        assert!(g.selected.is_empty());
    }

    #[test]
    fn groups_report_live_unique_members_and_mouse_recall_matches_keys() {
        let mut g = game("groups");
        let own = g.selected[0];
        let enemy = g.world.entities.iter().find(|e| e.owner == 1).unwrap().id;
        g.groups[1] = vec![own, own, enemy, u32::MAX];
        assert_eq!(g.live_group(1), vec![own]);
        g.selected.clear();
        g.render();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == Action::RecallGroup(1))
            .unwrap()
            .clone();
        let hash = g.world.state_hash();
        g.left_down(b.x + 2, b.y + 2);
        g.left_up(b.x + 2, b.y + 2, false);
        assert_eq!(g.selected, vec![own]);
        assert_eq!(g.world.state_hash(), hash);
        let saved = g.selected.clone();
        g.action(Action::RecallGroup(9));
        assert_eq!(g.selected, saved);
        // A stored mouse Shift state must not turn keyboard recall into centering.
        g.camera.center(Pos::cell(60, 60));
        let camera = g.camera;
        g.last_group = None;
        g.ux.pointer_shift = true;
        g.ux.keyboard_navigation = true;
        g.ux.focused = Some(Action::RecallGroup(1));
        g.key("Enter", false, false);
        assert_eq!(
            g.camera.project(Pos::cell(20, 60)),
            camera.project(Pos::cell(20, 60))
        );
        assert_eq!(g.selected, vec![own]);
    }

    #[test]
    fn pointer_shift_remove_and_keyboard_minus_do_the_same_thing() {
        let mut g = game("parity");
        let ids: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0)
            .map(|e| e.id)
            .collect();
        let worker = Faction::Union.worker();
        g.selected = ids.clone();
        g.render();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == Action::FilterSelection(worker, false))
            .unwrap()
            .clone();
        g.ux.pointer_shift = true;
        g.left_down(b.x + 2, b.y + 2);
        g.left_up(b.x + 2, b.y + 2, true);
        g.ux.pointer_shift = false;
        let mouse = g.selected.clone();
        g.selected = ids;
        g.ux.keyboard_navigation = true;
        g.ux.focused = Some(Action::FilterSelection(worker, false));
        g.key("Enter", true, false);
        assert_eq!(g.selected, mouse);
        assert_eq!(g.selected.len(), 1);
    }

    #[test]
    fn queue_readout_distinguishes_paid_waiting_research_crew_and_exit() {
        let mut g = game("production");
        let id = g.selected[0];
        let kind = Faction::Union.worker();
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap()
            .queue = vec![Production {
            kind,
            remaining: 300,
            started: false,
            cost_salvage: 50,
            cost_pressure: 0,
        }];
        assert_eq!(
            g.selected_production_info().unwrap().status,
            "PAID / WAITING TO START"
        );
        // Research costs more pressure than a fresh field holds; fund it so
        // the readout can be checked while research pauses worker production.
        g.world.players[0].salvage = bw_content::DOCTRINE_SALVAGE;
        g.world.players[0].pressure = bw_content::DOCTRINE_PRESSURE;
        g.world
            .issue(
                0,
                Command::Research {
                    building: id,
                    doctrine: bw_content::Doctrine::Hauling,
                },
            )
            .unwrap();
        g.tick();
        assert!(
            g.selected_production_info()
                .unwrap()
                .status
                .contains("1 WAIT, PAUSED")
        );
        g.world.players[0].research = None;
        let p = &mut g
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap()
            .queue[0];
        p.started = true;
        p.remaining = 1;
        g.world.players[0].crew = g.world.players[0].cap;
        assert_eq!(
            g.selected_production_info().unwrap().status,
            "READY / CREW LIMIT"
        );
        g.world.players[0].crew = 0;
        assert_eq!(
            g.selected_production_info().unwrap().status,
            "FINISHING / CHECKING EXIT"
        );
    }

    #[test]
    fn controls_fit_minimum_and_scaled_windows_without_overlap() {
        let mut g = game("bounds");
        crate::depth_review::practice(&mut g).unwrap();
        g.ux.guidance_visible = true;
        g.selected = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0)
            .map(|e| e.id)
            .collect();
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.render();
            for b in &g.buttons {
                assert!(
                    b.x >= 0 && b.y >= 0 && b.x + b.w <= w as i32 && b.y + b.h <= h as i32,
                    "{w}: {:?}",
                    b.action
                );
                if matches!(
                    b.action,
                    Action::RecallGroup(_) | Action::FilterSelection(..) | Action::RosterPage(_)
                ) {
                    assert!(!g.world_pointer_allowed(b.x + 2, b.y + 2));
                    for other in &g.buttons {
                        // A type's remove control sits in its own cell.
                        let own_minus = matches!(
                            (&b.action, &other.action),
                            (Action::FilterSelection(k, _), Action::FilterSelection(o, _)) if k == o
                        );
                        if b.action != other.action && !own_minus {
                            assert!(
                                !(b.x < other.x + other.w
                                    && b.x + b.w > other.x
                                    && b.y < other.y + other.h
                                    && b.y + b.h > other.y),
                                "{w}: {:?} overlaps {:?}",
                                b.action,
                                other.action
                            );
                        }
                    }
                }
            }
        }
        g.screen = Screen::Settings;
        g.render();
        for b in &g.buttons {
            assert!(b.y + b.h <= g.canvas.height() as i32);
        }
    }

    #[test]
    fn health_preference_persists_without_changing_simulation_or_hidden_pixels() {
        let mut g = game("health");
        g.selected.clear();
        g.ux.preferences.always_health_bars = false;
        g.render();
        let default = g.scene_canvas.pixels.clone();
        let hash = g.world.state_hash();
        g.key("O", false, false);
        g.render();
        assert_ne!(default, g.scene_canvas.pixels);
        assert_eq!(hash, g.world.state_hash());
        assert!(
            crate::ux::UxState::load(&g.data_dir)
                .preferences
                .always_health_bars
        );
        let before = g.scene_canvas.pixels.clone();
        let hidden: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner != 0 && !g.world.entity_visible(0, e.id))
            .map(|e| e.id)
            .collect();
        assert!(!hidden.is_empty());
        g.world.entities.retain(|e| !hidden.contains(&e.id));
        g.render();
        assert_eq!(
            before, g.scene_canvas.pixels,
            "Hidden enemies must produce no health pixels"
        );
        let legacy: crate::ux::Preferences = serde_json::from_str("{}").unwrap();
        assert!(!legacy.always_health_bars);
    }

    #[test]
    fn control_groups_survive_save_and_load_without_dead_or_foreign_members() {
        let mut g = game("groups-save");
        let own: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        let enemy = g.world.entities.iter().find(|e| e.owner == 1).unwrap().id;
        let dead = own[0];
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == dead)
            .unwrap()
            .hp = 0;
        // The simulation removes the destroyed worker on its next step; the
        // stale id then stands for a member lost before the save.
        g.tick();
        g.groups[2] = vec![own[1], own[1], enemy, dead, u32::MAX, own[2]];
        g.groups[7] = vec![own[3]];
        let hash = g.world.state_hash();
        assert!(g.save_match());
        g.groups = Default::default();
        g.load_match();
        assert_eq!(g.world.state_hash(), hash);
        assert_eq!(g.groups[2], vec![own[1], own[2]]);
        assert_eq!(g.groups[7], vec![own[3]]);
        assert!(g.groups[1].is_empty());
        // A sidecar from another match must not carry its groups across.
        let saves = g.data_dir.join("saves");
        let mut guide: serde_json::Value =
            serde_json::from_slice(&std::fs::read(saves.join("quick-save-ui.json")).unwrap())
                .unwrap();
        guide["world_hash"] = serde_json::Value::String("stale".into());
        std::fs::write(
            saves.join("quick-save-ui.json"),
            serde_json::to_vec(&guide).unwrap(),
        )
        .unwrap();
        g.load_match();
        assert!(g.groups.iter().all(|group| group.is_empty()));
    }
}
