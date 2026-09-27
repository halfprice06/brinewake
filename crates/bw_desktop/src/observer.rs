//! The dock while watching a recording: no command card and no one seat's
//! quick-select, but a row per side in its colour, with what it is making
//! (each running job's icon over its progress) and its army (an icon per
//! kind with the count), as StarCraft's observer panels show them.
//!
//! Presentation only: it reads the fog-lifted playback world.

use crate::canvas::{EDGE, GOLD, INK, MUTED, PANEL, WHITE};
use crate::game::Game;
use crate::native_ui::small;
use crate::occlusion::PixelScale;
use bw_core::Kind;

/// One cell of a row: an icon, and either a job's progress or a count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cell {
    /// Something being made, and how far along it is (per mille).
    Job { key: String, done: u32 },
    /// A kind of machine on the field, and how many.
    Army { key: String, count: usize },
}

/// A side's row: its jobs, then its army by kind with workers first.
pub(crate) fn side_cells(world: &bw_sim::World, seat: u8) -> (Vec<Cell>, Vec<Cell>) {
    let faction = world.players[usize::from(seat)].faction;
    let unit = |kind: Kind| format!("ui_unit_{}", kind.asset(faction));
    let building = |kind: Kind| format!("ui_build_{}", kind.asset(faction));
    let permille = |remaining: u32, total: u32| {
        let total = total.max(1);
        (1000 - u64::from(remaining.min(total)) * 1000 / u64::from(total)) as u32
    };
    let mut jobs = Vec::new();
    let mut army: Vec<(Kind, usize)> = Vec::new();
    let mut own: Vec<&bw_sim::Entity> = world
        .entities
        .iter()
        .filter(|e| e.owner == seat && e.hp > 0)
        .collect();
    own.sort_by_key(|e| e.id);
    for e in own {
        if e.kind.is_building() {
            if e.build_remaining > 0 {
                jobs.push(Cell::Job {
                    key: building(e.kind),
                    done: permille(e.build_remaining, bw_content::spec(e.kind).build_ticks),
                });
            } else if let Some(job) = e.queue.first() {
                jobs.push(Cell::Job {
                    key: unit(job.kind),
                    done: if job.started {
                        permille(job.remaining, bw_content::spec(job.kind).build_ticks)
                    } else {
                        0
                    },
                });
            }
        } else {
            match army.iter_mut().find(|(kind, _)| *kind == e.kind) {
                Some((_, count)) => *count += 1,
                None => army.push((e.kind, 1)),
            }
        }
    }
    if let Some(research) = &world.players[usize::from(seat)].research {
        jobs.push(Cell::Job {
            key: match research.doctrine {
                bw_content::Doctrine::Hauling => "ui_doc_hauling".into(),
                bw_content::Doctrine::FireControl => "ui_doc_fire_control".into(),
            },
            done: permille(research.remaining, bw_content::DOCTRINE_TICKS),
        });
    }
    army.sort_by_key(|(kind, _)| (!kind.is_worker(), *kind));
    let army = army
        .into_iter()
        .map(|(kind, count)| Cell::Army {
            key: unit(kind),
            count,
        })
        .collect();
    (jobs, army)
}

/// The message line while watching a recording. No one seat's own notes
/// (its refused orders, its blocked paths, "enemy Looms"), which read as if
/// the watcher were the first seat (trial 10: "HAULING II COMPLETE" over
/// the observer's field). What a side finished researching is news to a
/// watcher, named by side.
pub(crate) fn neutral_notice(world: &bw_sim::World) -> Option<String> {
    world.events.iter().rev().find_map(|event| {
        if event.kind != bw_sim::EventKind::ResearchCompleted {
            return None;
        }
        let seat = event.player?;
        let player = world.players.get(usize::from(seat))?;
        let doctrine = player.doctrine?;
        let shared = world
            .players
            .iter()
            .filter(|p| p.faction == player.faction)
            .count()
            > 1;
        let side = if shared {
            format!("{} {}", player.faction.name(), u32::from(seat) + 1)
        } else {
            player.faction.name().to_string()
        };
        Some(format!(
            "{side}: {}{} complete",
            crate::tactics::doctrine_name(doctrine),
            if player.doctrine_tier >= 2 { " II" } else { "" },
        ))
    })
}

impl Game {
    /// Draw the observer's dock right of the minimap: a row per side.
    pub(crate) fn draw_observer_panel(&mut self, top: i32, bottom: i32) {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let seats = self.world.seat_count().max(1) as i32;
        let x0 = crate::lean_hud::QUICK_X * s;
        let row_h = (bottom - top - 8 * s) / seats;
        let cell = (crate::lean_hud::CELL * s).min(row_h - 2 * s);
        let icon = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        // Each seat by its colour and name as on every screen, "VIOLET
        // COMPACT", the column as wide as the longest.
        let titles: Vec<String> = self
            .world
            .seats()
            .map(|seat| crate::seats::seat_title(&self.world, seat))
            .collect();
        let name_w =
            (titles.iter().map(String::len).max().unwrap_or(0) as i32 * 6 * s + 12 * s).max(64 * s);
        for seat in self.world.seats() {
            let y = top + 4 * s + i32::from(seat) * row_h;
            let colour = crate::seats::seat_colour(&self.world, seat);
            let out = self.world.is_eliminated(seat);
            let name = titles[usize::from(seat)].as_str();
            let cy = y + (row_h - cell) / 2;
            self.canvas
                .rect(x0, cy, 3 * s, cell, if out { EDGE } else { colour });
            small(
                &mut self.canvas,
                name,
                x0 + 7 * s,
                cy + (cell - 7 * s) / 2,
                if out { MUTED } else { colour },
                s,
            );
            if out {
                small(
                    &mut self.canvas,
                    "OUT",
                    x0 + name_w,
                    cy + (cell - 7 * s) / 2,
                    MUTED,
                    s,
                );
                continue;
            }
            let (jobs, army) = side_cells(&self.world, seat);
            let mut x = x0 + name_w;
            let right = w - 8 * s;
            let pitch = cell + 2 * s;
            let groups = [jobs, army];
            for (g, cells) in groups.iter().enumerate() {
                if g == 1 && !groups[0].is_empty() {
                    // A rule between what is being made and what stands.
                    x += 4 * s;
                    self.canvas.rect(x, cy + 3 * s, s, cell - 6 * s, EDGE);
                    x += 6 * s;
                }
                for (i, c) in cells.iter().enumerate() {
                    if x + cell > right {
                        let more = cells.len() - i;
                        small(
                            &mut self.canvas,
                            &format!("+{more}"),
                            x,
                            cy + (cell - 7 * s) / 2,
                            MUTED,
                            s,
                        );
                        break;
                    }
                    let key = match c {
                        Cell::Job { key, .. } | Cell::Army { key, .. } => key,
                    };
                    self.canvas.rect(x, cy, cell, cell, PANEL);
                    let inset = (cell - 24 * s).max(0) / 2;
                    if let Some(atlas) = &self.atlas {
                        atlas.draw_scaled(
                            &mut self.canvas,
                            key,
                            x + inset,
                            cy + inset,
                            false,
                            icon,
                        );
                    }
                    match c {
                        Cell::Job { done, .. } => {
                            let bar = (cell - 2 * s) * *done as i32 / 1000;
                            self.canvas
                                .rect(x + s, cy + cell - 3 * s, cell - 2 * s, 2 * s, INK);
                            self.canvas
                                .rect(x + s, cy + cell - 3 * s, bar.max(s), 2 * s, GOLD);
                            self.canvas.frame(x, cy, cell, cell, GOLD);
                        }
                        Cell::Army { count, .. } => {
                            let text = count.to_string();
                            let tw = text.len() as i32 * 6 * s + s;
                            let (tx, ty) = (x + cell - tw - s, cy + cell - 9 * s);
                            self.canvas.rect(tx, ty, tw + s, 9 * s, INK);
                            small(&mut self.canvas, &text, tx + s, ty + s, WHITE, s);
                            self.canvas.frame(x, cy, cell, cell, EDGE);
                        }
                    }
                    x += pitch;
                }
            }
            if groups.iter().all(Vec::is_empty) {
                small(
                    &mut self.canvas,
                    "NOTHING ON THE FIELD",
                    x,
                    cy + (cell - 7 * s) / 2,
                    MUTED,
                    s,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;

    #[test]
    fn a_replay_dock_has_no_seats_controls() {
        let dir = std::env::temp_dir().join(format!("brinewake-observer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("watch.replay.json");
        let mut world = bw_sim::World::new(9, Faction::Union);
        for _ in 0..90 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = crate::game::Game::new(base);
        g.resize_view(1280, 720);
        g.start_playback(&path, false).unwrap();
        g.render();
        use crate::game::Action;
        assert!(
            !g.buttons.iter().any(|b| matches!(
                b.action,
                Action::Train(_) | Action::IdleWorker | Action::SelectArmy | Action::Build(_)
            )),
            "no command card or quick-select while watching"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_watcher_hears_no_seats_own_notes_but_every_sides_research() {
        let mut world = bw_sim::World::new(7, Faction::Union);
        world.events.clear();
        let event = |kind, player: u8| bw_sim::Event {
            tick: 0,
            kind,
            player: Some(player),
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        };
        world
            .events
            .push(event(bw_sim::EventKind::CommandRejected, 0));
        assert_eq!(neutral_notice(&world), None, "a seat's refusal is its own");
        let seat = 1u8;
        world.players[usize::from(seat)].doctrine = Some(bw_content::Doctrine::Hauling);
        world.players[usize::from(seat)].doctrine_tier = 2;
        world
            .events
            .push(event(bw_sim::EventKind::ResearchCompleted, seat));
        let line = neutral_notice(&world).expect("research is news");
        let faction = world.players[usize::from(seat)].faction;
        assert!(line.starts_with(faction.name()), "{line}");
        assert!(line.contains(" II complete"), "{line}");
    }

    #[test]
    fn each_side_lists_its_jobs_and_its_army_workers_first() {
        let mut world = bw_sim::World::new(7, Faction::Union);
        world.ai_enabled = false;
        let (jobs, army) = side_cells(&world, 1);
        assert!(jobs.is_empty());
        let Some(Cell::Army { key, count }) = army.first() else {
            panic!("an army row: {army:?}");
        };
        assert!(key.starts_with("ui_unit_"), "{key}");
        assert!(*count > 0);
        // A queued machine shows as a job at nought until it starts.
        let hq = world
            .entities
            .iter_mut()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap();
        hq.queue.push(bw_sim::Production {
            kind: Kind::Hook,
            remaining: 10,
            started: false,
            cost_salvage: 0,
            cost_pressure: 0,
        });
        let (jobs, _) = side_cells(&world, 0);
        assert_eq!(jobs.len(), 1);
        assert!(matches!(&jobs[0], Cell::Job { done: 0, .. }));
    }
}
