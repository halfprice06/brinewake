//! Tests for the fixes from the eighth two-agent trial (rules 13): the hold
//! count is impossible to miss, the sluice card spells out the hold rule,
//! a Loom's fire shows where it came from, the Guide names both ways to
//! win, Shift-train queues five in a two-player match, a tide switch is
//! never a pressure trap, F2 leaves a mouth's keepers, and workers are
//! reported and protected apart from the army.
#[cfg(test)]
mod tests {
    use crate::field_alerts::{AlertHistory, AlertKind};
    use crate::game::{Action, Game, Screen};
    use bw_core::{Camera, Faction, Kind, Pos};
    use bw_sim::{CROSSING_MOUTHS, Event, EventKind, Order};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        // These checks read the sluice card, so it is pinned open.
        game.ux.preferences.tide_card = true;
        game
    }

    /// Save the frame when `BW_TRIAL8_FRAMES` names a folder, for review.
    fn keep_frame(g: &Game, name: &str) {
        if let Ok(dir) = std::env::var("BW_TRIAL8_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&PathBuf::from(dir).join(format!("{name}.png")))
                .expect("frame");
        }
    }

    fn place(g: &mut Game, owner: u8, kind: Kind, cell: Pos) -> u32 {
        let id = g.world.spawn_for_tests(owner, kind, cell);
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
        }
        id
    }

    /// The enemy owns the sluice and stands at the east mouth of each lane.
    fn enemy_holding(g: &mut Game, counted_seconds: u32) {
        g.world.gate.owner = Some(1);
        place(g, 1, Kind::Reedguard, CROSSING_MOUTHS[0][1]);
        place(g, 1, Kind::Reedguard, CROSSING_MOUTHS[1][1]);
        g.world.lane_hold[1] = counted_seconds * 30;
    }

    fn banner(g: &Game) -> Option<crate::game::Button> {
        g.buttons
            .iter()
            .find(|b| b.action == Action::FocusHold && b.label != "HOLD CHECKLIST")
            .cloned()
    }

    #[test]
    fn an_enemy_count_shows_a_large_banner_with_its_timer() {
        let mut g = game();
        g.render();
        assert!(banner(&g).is_none(), "no banner without a count");
        enemy_holding(&mut g, 12);
        for _ in 0..3 {
            g.tick();
        }
        assert!((0..2).all(|lane| g.world.holds_lane(1, lane)));
        g.render();
        keep_frame(&g, "enemy-count");
        let b = banner(&g).expect("the banner");
        assert_eq!(b.label, "ENEMY WINS IN");
        let s = g.ui_scale();
        assert!(b.w >= 200 * s && b.h >= 30 * s, "the banner is large");
        let brief = g.hold_brief();
        assert!(brief.contains("wins in 78s"), "{brief}");
        assert!(
            brief.contains("at a bank of lane N or S stops it"),
            "{brief}"
        );
        // The status line says what the count is doing.
        let card = g.route_bounds();
        assert!(
            b.x >= card.x + card.w || card.x >= b.x + b.w || b.y >= card.y + card.h,
            "clear of the sluice card"
        );
        // The draining count reads as draining.
        g.world
            .entities
            .retain(|e| e.owner == 0 || e.kind != Kind::Reedguard);
        g.tick();
        g.render();
        keep_frame(&g, "enemy-drains");
        let b = banner(&g).expect("a draining count keeps its banner");
        assert_eq!(b.label, "ENEMY DRAINING");
        assert!(g.hold_brief().contains("drains"));
    }

    #[test]
    fn a_recording_shows_no_banner_for_either_side() {
        let dir = std::env::temp_dir().join(format!("brinewake-trial8-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hold.replay.json");
        let mut recorded = bw_sim::World::new(29, Faction::Assembly);
        recorded.ai_enabled = false;
        recorded.step();
        recorded.export_replay(&path).unwrap();
        let mut g = game();
        g.start_playback(&path, false).unwrap();
        enemy_holding(&mut g, 12);
        g.render();
        assert!(g.hold_gauge().is_some(), "a count is running");
        assert!(
            banner(&g).is_none(),
            "the spectator bar carries both counts"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_banner_stays_clear_of_every_control_at_every_size() {
        for (practice, pinned) in [(false, false), (false, true), (true, true)] {
            for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
                let mut g = game();
                g.ux.practice = practice;
                g.ux.preferences.tide_card = pinned;
                g.resize_view(w, h);
                enemy_holding(&mut g, 20);
                g.tick();
                g.render();
                let b = banner(&g).unwrap_or_else(|| panic!("{w}x{h}: a banner"));
                let view = g.world_view();
                assert!(
                    b.y >= view.top
                        && b.y + b.h <= view.bottom
                        && b.x >= 0
                        && b.x + b.w <= w as i32,
                    "{w}x{h}: the banner sits on the field"
                );
                for other in &g.buttons {
                    if other.action == Action::FocusHold {
                        continue;
                    }
                    assert!(
                        b.x + b.w <= other.x
                            || other.x + other.w <= b.x
                            || b.y + b.h <= other.y
                            || other.y + other.h <= b.y,
                        "{w}x{h} practice {practice}: the banner overlaps {:?}",
                        other.action
                    );
                }
                let card = g.route_bounds();
                assert!(
                    card.w == 0
                        || b.x >= card.x + card.w
                        || card.x >= b.x + b.w
                        || card.y >= b.y + b.h
                        || b.y >= card.y + card.h,
                    "{w}x{h} practice {practice}: the banner overlaps the sluice card"
                );
            }
        }
    }

    #[test]
    fn f3_and_the_banner_go_to_the_mouth_that_stops_an_enemy_count() {
        let mut g = game();
        enemy_holding(&mut g, 30);
        for _ in 0..3 {
            g.tick();
        }
        g.home();
        g.key("F3", false, false);
        let mut there = Camera::default();
        // Both east mouths are as far from a western headquarters; the
        // north one wins the tie.
        there.center(CROSSING_MOUTHS[0][1]);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
        assert!(
            g.message.contains("at a bank of lane N or S stops it"),
            "{}",
            g.message
        );
        // The card for the count outlives an ordinary red card.
        for _ in 0..(45 * 30) {
            g.tick();
        }
        assert!(
            g.ux.alerts
                .entries
                .iter()
                .any(|a| a.kind == AlertKind::EnemyHold),
            "the hold card stays while the enemy counts"
        );
        g.home();
        g.action(Action::FocusHold);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
    }

    #[test]
    fn our_own_count_and_its_checklist() {
        let mut g = game();
        g.world.gate.owner = Some(0);
        place(&mut g, 0, Kind::Riveter, CROSSING_MOUTHS[0][0]);
        g.tick();
        g.render();
        keep_frame(&g, "checklist-one-lane");
        let holders = crate::qol::mouth_holders(&g.world);
        assert!(holders[0][0].0 && !holders[1][0].0 && !holders[1][1].0);
        let checklist = g
            .buttons
            .iter()
            .find(|b| b.label == "HOLD CHECKLIST")
            .cloned()
            .expect("the checklist row");
        let card = g.route_bounds();
        assert!(checklist.y > card.y && checklist.y + checklist.h <= card.y + card.h);
        // Its click shows the lane that is not held yet.
        g.action(Action::FocusHold);
        let mut there = Camera::default();
        there.center(CROSSING_MOUTHS[1][0]);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
        // The tide buttons sit under the checklist, inside the card.
        for b in g
            .buttons
            .iter()
            .filter(|b| matches!(b.action, Action::SetTide(_) | Action::Flood))
        {
            assert!(b.y >= checklist.y + checklist.h, "{:?}", b.action);
            assert!(b.y + b.h <= card.y + card.h, "{:?}", b.action);
        }
        place(&mut g, 0, Kind::Riveter, CROSSING_MOUTHS[1][1]);
        g.tick();
        g.render();
        keep_frame(&g, "own-count");
        assert_eq!(banner(&g).expect("our banner").label, "YOU WIN IN");
        // An enemy steps onto the north mouth: our count drains, the banner
        // names the lane and the blocker is marked on the field.
        g.world.lane_hold[0] = 40 * 30;
        place(&mut g, 1, Kind::Reedguard, CROSSING_MOUTHS[0][1]);
        g.tick();
        g.render();
        keep_frame(&g, "own-drains-blocked");
        g.camera.center(CROSSING_MOUTHS[0][1]);
        g.render();
        keep_frame(&g, "blocker-marked");
        let label = banner(&g).expect("our banner").label;
        assert!(
            label.starts_with("YOUR ") && label.ends_with("/90 DRAINS"),
            "{label}"
        );
        assert!(g.hold_brief().contains("drains"), "{}", g.hold_brief());
    }

    #[test]
    fn a_loom_shell_draws_back_to_the_loom_it_came_from() {
        let mut g = game();
        g.world.players[1].upgrades = vec![bw_content::Upgrade::Siege];
        // Rules 21: a Siege Loom reaches eight cells, a Riveter's sight, so
        // the unseen shot falls on a worker, which sees seven.
        let worker = g.world.players[0].faction.worker();
        let target = g.world.spawn_for_tests(0, worker, Pos::cell(40, 60));
        let loom = g.world.spawn_for_tests(1, Kind::Loom, Pos::cell(48, 60));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == loom) {
            e.deployed = true;
            e.order = Order::Hold;
        }
        let loom_pos = g.world.entities.iter().find(|e| e.id == loom).unwrap().pos;
        assert!(!g.world.visible(0, loom_pos), "past the worker's sight");
        let mut fired = false;
        for _ in 0..90 {
            g.tick();
            if !g.world.artillery.is_empty() {
                fired = true;
                break;
            }
        }
        assert!(fired, "the Loom fires");
        assert!(g.world.visible(0, loom_pos), "the shot lights the Loom");
        assert!(
            g.ux.alerts
                .entries
                .iter()
                .any(|a| a.kind == AlertKind::LoomFire && a.pos == loom_pos),
            "a LOOM FIRE card on the Loom"
        );
        g.camera.center(Pos::cell(45, 60));
        g.render();
        keep_frame(&g, "loom-tracer");
        let _ = target;
    }

    #[test]
    fn an_off_screen_loom_gets_an_edge_arrow_and_a_capture_shows_its_ring() {
        use crate::tactics::screen_edge_point;
        assert_eq!(screen_edge_point((100, 100), (150, 150), 640, 360), None);
        assert_eq!(
            screen_edge_point((100, 100), (1000, 100), 640, 360),
            Some((632, 100))
        );
        let mut g = game();
        g.world.players[1].upgrades = vec![bw_content::Upgrade::Siege];
        let worker = g.world.players[0].faction.worker();
        g.world.spawn_for_tests(0, worker, Pos::cell(40, 60));
        let loom = g.world.spawn_for_tests(1, Kind::Loom, Pos::cell(48, 60));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == loom) {
            e.deployed = true;
            e.order = Order::Hold;
        }
        for _ in 0..90 {
            g.tick();
            if !g.world.artillery.is_empty() {
                break;
            }
        }
        assert!(!g.world.artillery.is_empty(), "the Loom fires");
        g.camera.center(Pos::cell(45, 60));
        let loom_pos = g.world.entities.iter().find(|e| e.id == loom).unwrap().pos;
        let target_pos = g.world.artillery[0].target;
        g.render();
        for _ in 0..200 {
            if g.project(loom_pos).0 > g.canvas.width() as i32 + 30 {
                break;
            }
            g.pan(-10, 0);
            g.render();
        }
        keep_frame(&g, "loom-edge-arrow");
        let (w, h) = (g.canvas.width() as i32, g.canvas.height() as i32);
        let (tx, ty) = g.project(target_pos);
        assert!((0..w).contains(&tx), "the shell's target stays on screen");
        let (fx, fy) = g.project(loom_pos);
        let (ex, ey) = screen_edge_point((tx, ty), (fx, fy), w, h).expect("off screen");
        // The arrow's body sits just behind its tip, toward the target.
        let red = (ex - 10..=ex)
            .flat_map(|x| (ey - 3..=ey + 3).map(move |y| (x, y)))
            .filter(|&(x, y)| g.canvas.get(x, y) == Some(crate::canvas::RED))
            .count();
        assert!(red > 10, "a red arrow on the right edge ({red})");
        // A capture under way draws the contest ring in the capturer's colour.
        let mut g = game();
        g.world.gate.capture_player = Some(1);
        g.world.gate.capture_progress = 200;
        let gate = g.world.map.gate_pos;
        g.camera.center(gate);
        g.render();
        keep_frame(&g, "capture-ring");
        let west = Pos {
            x: gate.x - bw_core::FP * bw_content::CAPTURE_CONTEST_RADIUS_CELLS,
            y: gate.y,
        };
        let (wx, wy) = g.project(west);
        let red = (wx - 6..=wx + 6)
            .flat_map(|x| (wy - 6..=wy + 6).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                g.canvas
                    .get(x, y)
                    .is_some_and(|c| c[0] > 180 && c[1] < 140 && c[2] < 120)
            })
            .count();
        assert!(red > 0, "the contest ring crosses the west of the station");
    }

    #[test]
    fn f2_leaves_machines_holding_a_crossing_mouth() {
        let mut g = game();
        let keeper = place(&mut g, 0, Kind::Riveter, CROSSING_MOUTHS[1][0]);
        let roamer = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(30, 60));
        // On hold away from a mouth is still army.
        let guard = place(&mut g, 0, Kind::Riveter, Pos::cell(32, 60));
        let army = g.army_ids();
        assert!(army.contains(&roamer) && army.contains(&guard));
        assert!(!army.contains(&keeper), "a mouth keeper stays out of F2");
        g.key("F2", false, false);
        assert!(!g.selected.contains(&keeper));
        assert!(g.message.contains("1 at a crossing mouth"), "{}", g.message);
        // Idle at the mouth it still stays out (trial 9); walked away from
        // the mouth, it is army again.
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == keeper) {
            e.order = Order::Idle;
        }
        assert!(!g.army_ids().contains(&keeper));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == keeper) {
            e.pos = Pos::cell(34, 60);
        }
        assert!(g.army_ids().contains(&keeper));
    }

    #[test]
    fn a_switch_with_an_enemy_at_the_station_is_refused_with_its_reason() {
        let mut g = game();
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 200;
        let gate = g.world.map.gate_pos;
        let (gx, gy) = gate.cell_xy();
        g.world
            .spawn_for_tests(1, Kind::Reedguard, Pos::cell(gx + 1, gy));
        let reason = g
            .action_reason(&Action::SetTide(bw_sim::Arm::NORTH))
            .expect("refused");
        assert!(reason.contains("enemy stands at the station"), "{reason}");
        assert!(g.action_reason(&Action::Flood).is_some());
        g.render();
        let dry = g
            .buttons
            .iter()
            .find(|b| b.action == Action::SetTide(bw_sim::Arm::NORTH))
            .expect("the DRY N button");
        assert!(!dry.enabled);
        let pressure = g.world.players[0].pressure;
        g.issue(bw_sim::Command::SetTide {
            arm: bw_sim::Arm::NORTH,
        });
        assert_eq!(g.world.players[0].pressure, pressure, "nothing spent");
        assert!(g.message.contains("clear it first"), "{}", g.message);
    }

    fn before_of(g: &Game) -> BTreeMap<u32, (Kind, u8, Pos)> {
        g.world
            .entities
            .iter()
            .map(|e| (e.id, (e.kind, e.owner, e.pos)))
            .collect()
    }

    fn event(kind: EventKind) -> Event {
        Event {
            tick: 0,
            kind,
            player: Some(0),
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        }
    }

    #[test]
    fn workers_lost_loom_fire_and_refunds_get_their_own_cards() {
        let mut g = game();
        let hook = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Hook)
            .map(|e| e.id)
            .expect("a worker");
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(60, 40));
        let before = before_of(&g);
        let mut history = AlertHistory::default();
        let mut death = event(EventKind::Death);
        death.entity = Some(hook);
        death.cause = Some(Kind::Loom);
        let mut shell = event(EventKind::ArtilleryWarning);
        shell.player = Some(1);
        shell.entity = Some(9999);
        shell.other = Some(riveter);
        shell.from = Some(Pos::cell(90, 40));
        shell.to = Some(Pos::cell(60, 40));
        let mut fled = event(EventKind::WorkersFled);
        fled.from = Some(Pos::cell(64, 60));
        fled.amount = 3;
        let mut cancel = event(EventKind::SwitchCancelled);
        cancel.amount = 40;
        g.world.events = vec![death, shell, fled, cancel];
        history.observe(&g.world, &before);
        let card = |kind: AlertKind| {
            history
                .entries
                .iter()
                .find(|a| a.kind == kind)
                .unwrap_or_else(|| panic!("a {kind:?} card"))
                .clone()
        };
        let lost = card(AlertKind::WorkersLost);
        assert_eq!(lost.caption(2), "WORKER LOST AT BASE TO LOOMS 2S AGO");
        assert!(
            !history
                .entries
                .iter()
                .any(|a| a.kind == AlertKind::MachinesLost),
            "a worker is not counted as a machine"
        );
        let fire = card(AlertKind::LoomFire);
        assert_eq!(fire.pos, Pos::cell(90, 40), "the card is on the Loom");
        assert!(
            fire.caption(0)
                .starts_with("LOOM FIRE FROM N LANE, ENEMY BANK"),
            "{}",
            fire.caption(0)
        );
        assert!(
            card(AlertKind::WorkersFled)
                .caption(0)
                .starts_with("WORKERS PULLED BACK")
        );
        assert!(
            card(AlertKind::SwitchCancelled)
                .caption(0)
                .contains("REFUNDED")
        );
    }

    #[test]
    fn a_rally_on_the_island_a_lane_or_the_far_bank_warns() {
        let mut g = game();
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .expect("headquarters");
        let wreck_at = |pred: &dyn Fn(i32, i32) -> bool| {
            g.world
                .map
                .resources
                .iter()
                .find(|r| {
                    let (x, y) = r.pos.cell_xy();
                    r.remaining > 0 && pred(x, y)
                })
                .map(|r| r.pos)
        };
        let island = wreck_at(&|x, y| g.world.map.island_cell(x, y)).expect("island wreck");
        assert!(g.rally_words(hq, island).contains("sluice island"));
        let far = wreck_at(&|x, y| {
            x > 80 && !g.world.map.terrain(x, y).is_tidal() && !g.world.map.island_cell(x, y)
        })
        .expect("a wreck on the far bank");
        assert!(g.rally_words(hq, far).contains("across the lake"));
        let home = wreck_at(&|x, _| x < 40).expect("a home wreck");
        assert_eq!(
            g.rally_words(hq, home),
            "Rally point set on a wreck: new workers gather there."
        );
        let works = g.world.spawn_for_tests(0, Kind::Works, Pos::cell(24, 50));
        assert_eq!(
            g.rally_words(works, home),
            "Rally point set. New machines move here.",
            "a Works trains machines, not workers"
        );
    }

    #[test]
    fn the_guide_names_both_ways_to_win_and_draws_the_sluice() {
        let (basics, _) = crate::menus::guide_page(0, Faction::Union);
        assert!(
            basics
                .iter()
                .any(|line| line.contains("DESTROY THE ENEMY HQ")
                    && line.contains("HOLD BOTH LANES FOR 90S")),
            "{basics:?}"
        );
        let (_, keys) = crate::menus::guide_page(3, Faction::Union);
        assert!(keys.iter().any(|(key, _)| *key == "H"));
        let mut g = game();
        g.screen = Screen::Help;
        g.ux.help_page = 3;
        g.render();
        keep_frame(&g, "guide-sluice");
        let mut canvas = crate::canvas::Canvas::default();
        let model = g.menu_model();
        crate::menus::help(&mut canvas, None, &model);
        // The station on the diagram: its gold frame at map cell 64, 64.
        let (x, y) = (336 + (64 - 40) * 11 / 2 - 5, 160 + (64 - 40) * 8 / 3);
        let i = ((y as u32 * canvas.width() + x as u32) * 4) as usize;
        assert_eq!(
            &canvas.pixels[i..i + 4],
            &crate::canvas::GOLD,
            "the station is drawn on the SLUICE page"
        );
    }

    #[test]
    fn shift_train_queues_five_in_a_two_player_match() {
        let (host, guest) = crate::net::local_pair(7, Faction::Union, 3);
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data =
            |tag: &str| std::env::temp_dir().join(format!("bw-t8-{tag}-{}", std::process::id()));
        let mut a = Game::new_with_data_dir(base.clone(), data("host"));
        let mut b = Game::new_with_data_dir(base, data("guest"));
        a.resize_view(1280, 720);
        b.resize_view(1280, 720);
        a.start_network(host);
        b.start_network(guest);
        for g in [&mut a, &mut b] {
            let hq = g
                .world
                .entities
                .iter()
                .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
                .map(|e| e.id)
                .expect("headquarters");
            g.selected = vec![hq];
        }
        // The guest trains; its orders go to the network, not the world.
        let worker = b.world.players[0].faction.worker();
        b.train_selection(worker, 5);
        assert!(b.message.starts_with("Queued 5"), "{}", b.message);
        assert_eq!(
            b.session.as_ref().expect("session").pending_local().len(),
            5,
            "five orders wait for their tick"
        );
        // The batch reaches the headquarters' queue on both seats.
        let mut queued = 0;
        for _ in 0..20_000 {
            a.tick();
            b.tick();
            queued = b
                .world
                .entities
                .iter()
                .filter(|e| e.owner == 0 && e.kind == Kind::Headquarters)
                .map(|e| e.queue.len())
                .sum();
            if queued >= 5 || b.world.tick > 60 {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(queued, 5, "the whole batch reached the queue");
        a.session.as_mut().expect("session").close();
        b.session.as_mut().expect("session").close();
    }
}
