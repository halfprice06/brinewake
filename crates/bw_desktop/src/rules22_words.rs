//! Rules 22 in the interface: DELIVER on a right-click at an own drop-off;
//! the Confluence Guide and nest hover say a nest slows a count; the DRY
//! tooltip and Guide on the Confluence say it ebbs after 180 s; the agent
//! /state carries the slowed count; and every "YOUR ANSWER" the hover card
//! and the Guide roster give is checked against a headless duel.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};
    use crate::menus::guide_page_on;
    use bw_content::spec;
    use bw_core::{FP, Faction, Kind, Pos, Terrain};
    use bw_sim::{Command, MapId, Order, World};
    use std::path::PathBuf;

    fn game_on(faction: Faction, map: MapId) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-rules22-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.map = map;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn at(g: &mut Game, pos: Pos) -> (i32, i32) {
        g.camera.center(pos);
        g.render();
        g.project(pos)
    }

    fn page_text(page: usize, map: MapId) -> String {
        guide_page_on(page, Faction::Union, map).0.join(" ")
    }

    #[test]
    fn a_right_click_on_the_own_headquarters_with_loaded_workers_delivers() {
        let mut g = game_on(Faction::Compact, MapId::SplitBasin);
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .cloned()
            .expect("the Kiln");
        let workers: Vec<u32> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind == Kind::Raker)
            .map(|e| e.id)
            .take(2)
            .collect();
        // Both stand loaded, as the pulled-back Rakers of trial 12 did.
        for e in g.world.entities.iter_mut() {
            if workers.contains(&e.id) {
                e.carried = 11;
                e.carried_kind = Some(bw_sim::ResourceKind::Salvage);
                e.order = Order::Move { target: e.pos };
            }
        }
        g.selected = workers.clone();
        let (x, y) = at(&mut g, bw_sim::footprint_middle(&hq));
        let (command, verb, _) = g.context_order(x, y, false).expect("an order");
        assert_eq!(verb, "Deliver");
        assert_eq!(
            command,
            Command::Deliver {
                units: workers.clone(),
                target: hq.id
            }
        );
        assert_eq!(g.order_hint(&command, verb), "RIGHT CLICK: Deliver");
        g.right_click(x, y, false);
        assert!(g.message.starts_with("DELIVER"), "{}", g.message);
        // Empty-handed workers on the HQ get no DELIVER: it would do nothing.
        for e in g.world.entities.iter_mut() {
            if workers.contains(&e.id) {
                e.carried = 0;
            }
        }
        let verb = g.context_order(x, y, false).map(|order| order.1);
        assert_ne!(verb, Some("Deliver"));
    }

    #[test]
    fn the_confluence_guide_and_nest_hover_say_a_nest_slows_a_count() {
        let three = page_text(3, MapId::Confluence);
        assert!(three.contains("NESTS SLOW A COUNT TO HALF"), "{three}");
        assert!(!three.contains("NESTS BLOCK"), "{three}");
        assert!(three.contains("40P: 180S DRY"), "{three}");
        // Two seats keep the rules 20 words.
        let two = page_text(3, MapId::SplitBasin);
        assert!(two.contains("NESTS BLOCK, BUT DON'T HOLD"), "{two}");
        assert!(!two.contains("180S"), "{two}");
        // Every Guide line still fits its page.
        for map in [MapId::Confluence, MapId::SplitBasin] {
            for line in guide_page_on(3, Faction::Union, map).0 {
                assert!(line.len() <= 76, "{line}");
            }
        }
    }

    #[test]
    fn the_dry_tooltip_on_the_confluence_says_it_ebbs_after_180s() {
        let g = game_on(Faction::Union, MapId::Confluence);
        let tip = g.action_description(&Action::SetTide(bw_sim::Arm(1)));
        assert!(tip.contains("Ebbs back to shallow after 180s."), "{tip}");
        let g = game_on(Faction::Union, MapId::SplitBasin);
        let tip = g.action_description(&Action::SetTide(bw_sim::Arm::SOUTH));
        assert!(!tip.contains("Ebbs"), "{tip}");
    }

    #[test]
    fn the_sluice_card_names_the_ebb_while_it_warns() {
        let mut g = game_on(Faction::Union, MapId::Confluence);
        g.world.gate.tide = bw_sim::Tide::Open;
        g.world.gate.opened = true;
        g.world.gate.dry_arm = bw_sim::Arm(1);
        g.world.gate.ebb_pending = true;
        g.world.gate.warning_until = Some(g.world.tick + 300);
        let state = g.console_state();
        assert!(state.route.ebb_pending);
        // The station swings to the neutral pose: every arm half full.
        assert_eq!(
            crate::station3::warning_target(&g.world.gate, &[]).map(|pose| pose.levels),
            Some([10; 3])
        );
    }

    #[test]
    fn with_three_seats_a_nest_lights_no_foe_and_the_state_says_slowed() {
        let mut g = game_on(Faction::Union, MapId::Confluence);
        g.world.gate.owner = Some(0);
        for arm in g.world.arms_of(0) {
            let mouth = g.world.own_mouth(0, arm).expect("own bank");
            g.world.spawn_for_tests(0, Kind::Riveter, mouth);
        }
        let arm = g.world.arm_between(0, 1).expect("a shared lane");
        let far = g.world.own_mouth(1, arm).expect("their bank");
        g.world.spawn_for_tests(1, Kind::Tower, far);
        assert!(g.world.hold_slowed(0));
        let holders = crate::qol::mouth_holders(&g.world);
        assert!(
            holders[arm].iter().all(|(_, enemy)| !enemy),
            "a nest is no FOE with three seats"
        );
        assert!(crate::qol::lane_presence(&g.world, arm).1.is_none());
        let state = crate::agent::state_for_tests(&g);
        assert_eq!(state["hold"]["slowed"], true, "{}", state["hold"]);
        assert_eq!(state["seats"][0]["hold_slowed"], true);
        assert_eq!(state["seats"][1]["hold_slowed"], false);
    }

    // --- The counter hints against the simulation ---------------------------

    /// A duel's outcome: machines and hull left on each side.
    #[derive(Debug, Clone, Copy)]
    struct Duel {
        answer_left: usize,
        answer_hull: i32,
        enemy_left: usize,
        enemy_hull: i32,
        seconds: u64,
    }

    impl Duel {
        fn answer_wins(self) -> bool {
            self.enemy_left == 0 && self.answer_left > 0
        }
    }

    /// Machines that deploy to fight: they defend a line deployed.
    fn deploys(kind: Kind) -> bool {
        matches!(kind, Kind::Loom | Kind::Heliostat | Kind::Bulwark)
    }

    /// How many of `kind` a budget of `salvage` buys, at least one.
    fn count_for(kind: Kind, salvage: u32) -> usize {
        ((salvage + spec(kind).salvage / 2) / spec(kind).salvage).max(1) as usize
    }

    /// Open dry ground `w` by `h` cells far from every building.
    fn open_ground(world: &World, w: i32, h: i32) -> (i32, i32) {
        let clear = i64::from(FP * (16 + w)).pow(2);
        for y0 in 4..i32::from(world.map.height) - h - 4 {
            for x0 in 4..i32::from(world.map.width) - w - 4 {
                let dry = (y0..y0 + h).all(|y| {
                    (x0..x0 + w)
                        .all(|x| matches!(world.map.terrain(x, y), Terrain::Salt | Terrain::Silt))
                });
                let centre = Pos::cell(x0 + w / 2, y0 + h / 2);
                if dry
                    && world
                        .entities
                        .iter()
                        .all(|e| !e.kind.is_building() || e.pos.distance_sq(centre) > clear)
                {
                    return (x0, y0);
                }
            }
        }
        panic!("no open ground");
    }

    /// A duel on open dry ground of the Split Basin: `mover` (seat 0)
    /// attack-moves into `holder` (seat 1), which stands in a line, deployed
    /// and facing the mover if it deploys. Each side gets `salvage` worth.
    /// Every 10 s the movers are sent at the holders again, so none walk off.
    fn duel(mover: Kind, holder: Kind, salvage: u32) -> (usize, usize, Duel) {
        duel_at(mover, salvage, holder, salvage, false)
    }

    /// A duel with a budget for each side, and SIEGE for the mover if asked.
    fn duel_at(
        mover: Kind,
        mover_salvage: u32,
        holder: Kind,
        holder_salvage: u32,
        siege: bool,
    ) -> (usize, usize, Duel) {
        let side = |kind: Kind, other: Kind| {
            if kind == Kind::Tower {
                // A nest is anyone's: give it a side the mover is not.
                [Faction::Union, Faction::Assembly, Faction::Compact]
                    .into_iter()
                    .find(|f| *f != faction_of(other))
                    .expect("another side")
            } else {
                faction_of(kind)
            }
        };
        let factions = [side(mover, holder), side(holder, mover)];
        let mut world = World::with_map(2210, MapId::SplitBasin, &factions).expect("two seats");
        world.ai_enabled = false;
        world.entities.retain(|e| e.kind.is_building());
        if siege {
            world.players[0].upgrades.push(bw_content::Upgrade::Siege);
        }
        let (x0, y0) = open_ground(&world, 36, 18);
        let mid = y0 + 9;
        let line_x = x0 + 28;
        let (n, m) = (
            count_for(mover, mover_salvage),
            count_for(holder, holder_salvage),
        );
        let mut holders = Vec::new();
        for i in 0..m {
            let col = (i / 12) as i32;
            let y = mid - (m.min(12) as i32) / 2 + (i % 12) as i32;
            let id = world.spawn_for_tests(1, holder, Pos::cell(line_x + col, y));
            if deploys(holder)
                && let Some(e) = world.entities.iter_mut().find(|e| e.id == id)
            {
                e.deployed = true;
                e.deploy_remaining = 0;
                e.facing = 6;
            }
            holders.push(id);
        }
        let mut movers = Vec::new();
        for i in 0..n {
            let col = (i % 3) as i32;
            let y = mid - (n as i32 / 3) / 2 + (i / 3) as i32;
            movers.push(world.spawn_for_tests(0, mover, Pos::cell(x0 + 2 + col, y)));
        }
        let alive = |world: &World, ids: &[u32]| -> (usize, i32) {
            ids.iter()
                .filter_map(|id| world.entities.iter().find(|e| e.id == *id))
                .fold((0, 0), |(k, hull), e| (k + 1, hull + e.hp))
        };
        let start = world.tick;
        for step in 0..150 * 30 {
            if step % (10 * 30) == 0 {
                let left: Vec<u32> = movers
                    .iter()
                    .copied()
                    .filter(|id| world.entities.iter().any(|e| e.id == *id))
                    .collect();
                let target = holders
                    .iter()
                    .find_map(|id| world.entities.iter().find(|e| e.id == *id))
                    .map_or(Pos::cell(line_x, mid), |e| e.pos);
                let _ = world.issue(
                    0,
                    Command::AttackMove {
                        units: left,
                        target,
                        queued: false,
                    },
                );
            }
            world.step();
            if alive(&world, &movers).0 == 0 || alive(&world, &holders).0 == 0 {
                break;
            }
        }
        let (mover_left, mover_hull) = alive(&world, &movers);
        let (holder_left, holder_hull) = alive(&world, &holders);
        (
            n,
            m,
            Duel {
                answer_left: mover_left,
                answer_hull: mover_hull,
                enemy_left: holder_left,
                enemy_hull: holder_hull,
                seconds: (world.tick - start) / 30,
            },
        )
    }

    /// A claimed answer against an enemy machine, fought the way the claim
    /// means it: a deployed gun holds its line and the other side comes to
    /// it; two mobile sides fight both ways round and the answer must win
    /// both.
    fn claim_duels(answer: Kind, enemy: Kind) -> Vec<(String, Duel)> {
        const SALVAGE: u32 = 1_200;
        let flip = |d: Duel| Duel {
            answer_left: d.enemy_left,
            answer_hull: d.enemy_hull,
            enemy_left: d.answer_left,
            enemy_hull: d.answer_hull,
            seconds: d.seconds,
        };
        let mut out = Vec::new();
        if enemy == Kind::Tower {
            // "MASSED": twice the cost of two nests.
            let (n, m, d) = duel_at(answer, 720, Kind::Tower, 360, false);
            out.push((format!("{n} {} attack {m} NEST", answer.name()), d));
            return out;
        }
        if deploys(enemy) || !deploys(answer) {
            let (n, m, d) = duel(answer, enemy, SALVAGE);
            out.push((
                format!(
                    "{n} {} attack {m} {}{}",
                    answer.name(),
                    if deploys(enemy) { "deployed " } else { "" },
                    enemy.name()
                ),
                d,
            ));
        }
        if !deploys(enemy) || deploys(answer) {
            let (m, n, d) = duel(enemy, answer, SALVAGE);
            out.push((
                format!(
                    "{m} {} attack {n} {}{}",
                    enemy.name(),
                    if deploys(answer) { "deployed " } else { "" },
                    answer.name()
                ),
                flip(d),
            ));
        }
        out
    }

    /// The side that fields a combat machine.
    fn faction_of(kind: Kind) -> Faction {
        [Faction::Union, Faction::Assembly, Faction::Compact]
            .into_iter()
            .find(|f| f.army().contains(&kind))
            .expect("a combat machine")
    }

    /// Every combat machine a faction fields.
    fn fighters() -> Vec<Kind> {
        [Faction::Union, Faction::Assembly, Faction::Compact]
            .into_iter()
            .flat_map(|f| f.army())
            .collect()
    }

    /// The whole table, for the review record: every pair of two sides'
    /// combat machines, each side attacking in turn, and each machine
    /// against two Defense Nests. Run with --ignored --nocapture.
    #[test]
    #[ignore = "a report, not a check: about a minute in a debug build"]
    fn duel_table() {
        for a in fighters() {
            for b in fighters() {
                if faction_of(a) == faction_of(b) || a > b {
                    continue;
                }
                for (mover, holder) in [(a, b), (b, a)] {
                    let (n, m, d) = duel(mover, holder, 1_200);
                    println!(
                        "{n:>2} {:<9} attack {m:>2} {}{:<9} -> {}: {} left ({} hull) vs {} left ({} hull), {}s",
                        mover.name(),
                        if deploys(holder) { "deployed " } else { "" },
                        holder.name(),
                        if d.enemy_left == 0 && d.answer_left > 0 {
                            mover.name()
                        } else if d.answer_left == 0 && d.enemy_left > 0 {
                            holder.name()
                        } else {
                            "DRAW"
                        },
                        d.answer_left,
                        d.answer_hull,
                        d.enemy_left,
                        d.enemy_hull,
                        d.seconds
                    );
                }
            }
        }
        for (budget, siege) in [(360, false), (720, false), (720, true)] {
            for mover in fighters() {
                if siege && !matches!(mover, Kind::Loom | Kind::Heliostat) {
                    continue;
                }
                let (n, m, d) = duel_at(mover, budget, Kind::Tower, 360, siege);
                println!(
                    "{n:>2} {:<9}{} attack {m} NEST -> {} left ({} hull) vs {} nests left ({} hull), {}s",
                    mover.name(),
                    if siege { " +SIEGE" } else { "" },
                    d.answer_left,
                    d.answer_hull,
                    d.enemy_left,
                    d.enemy_hull,
                    d.seconds
                );
            }
        }
    }

    /// Every "YOUR ANSWER" the hover card and the Guide give between two
    /// sides' combat machines wins its duel for equal salvage (rules 22):
    /// in trial 12 the card sent 39 Sounders at Heliostats and they killed
    /// one. Run with --nocapture for the table.
    #[test]
    fn every_named_answer_wins_its_duel_for_equal_salvage() {
        let mut failures = Vec::new();
        let mut enemies = fighters();
        enemies.push(Kind::Tower);
        for enemy in enemies {
            for yours in [Faction::Union, Faction::Assembly, Faction::Compact] {
                if enemy != Kind::Tower && yours == faction_of(enemy) {
                    continue;
                }
                for answer in crate::ux::answers(enemy, yours) {
                    if spec(answer).damage == 0 {
                        continue;
                    }
                    for (setup, d) in claim_duels(answer, enemy) {
                        println!(
                            "{:>10} vs {:<10} {setup:<44} answer {:>2} left ({:>4} hull), enemy {:>2} left ({:>4} hull), {:>3}s{}",
                            answer.name(),
                            enemy.name(),
                            d.answer_left,
                            d.answer_hull,
                            d.enemy_left,
                            d.enemy_hull,
                            d.seconds,
                            if d.answer_wins() {
                                ""
                            } else {
                                "  <-- NOT BACKED"
                            }
                        );
                        if !d.answer_wins() {
                            failures.push(format!("{answer:?} vs {enemy:?}: {setup}: {d:?}"));
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
