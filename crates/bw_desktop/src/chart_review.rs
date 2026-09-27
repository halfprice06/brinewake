//! Controlled chart/craft scenarios and real UI-action journeys.
use crate::field_manual::ManualSubject;
use crate::game::{Action, Game, Screen};
use crate::tidal_traces::{Trace, TraceKind};
use bw_core::{Faction, Kind, Pos, Terrain};
use bw_sim::{Event, EventKind, Order};
use std::path::{Path, PathBuf};

fn capture(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    game.screenshot(&dir.join(format!("{name}.png")))
}

fn click(game: &mut Game, action: Action) -> Result<(), String> {
    game.render();
    let b = game
        .buttons
        .iter()
        .find(|b| b.action == action && b.enabled)
        .cloned()
        .ok_or_else(|| format!("Missing chart/craft UI control {action:?}"))?;
    game.left_down(b.x + b.w / 2, b.y + b.h / 2);
    Ok(())
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    event_journey(base.clone(), dir)?;
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut game = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-data")));
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        game.message.clear();
        let hq = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .id;
        game.selected = vec![hq];
        capture(&mut game, dir, &format!("{name}-hq"))?;
        game.action(Action::MainMenu);
        click(&mut game, Action::Setup)?;
        capture(&mut game, dir, &format!("{name}-setup"))?;
        game.action(Action::Help);
        game.action(Action::GuidePage(4));
        let hash = game.world.state_hash();
        for subject in [ManualSubject::Bulwark, ManualSubject::Loom] {
            click(&mut game, Action::ManualSubject(subject))?;
            if game.ux.manual.playing {
                click(&mut game, Action::ManualPlay)?;
            }
            for phase in 0..3 {
                while game.ux.manual.phase() != phase {
                    game.ux.manual.step();
                }
                capture(&mut game, dir, &format!("{name}-guide-{subject:?}-{phase}"))?;
            }
            let before = game.ux.manual.phase_tick;
            click(&mut game, Action::ManualStep)?;
            if game.ux.manual.playing || game.ux.manual.phase_tick == before {
                return Err("Guide step did not advance and pause".into());
            }
            let paused = game.ux.manual.clone();
            for _ in 0..10 {
                game.ux.manual.advance();
                game.render();
            }
            if game.ux.manual != paused {
                return Err("Paused manual advanced".into());
            }
            game.key("P", false, false);
            for _ in 0..5 {
                game.ux.manual.advance();
            }
            if !game.ux.manual.playing {
                return Err("P did not play manual".into());
            }
        }
        if game.world.state_hash() != hash {
            return Err("Guide changed simulation".into());
        }

        crate::artistry_review::practice(&mut game)?;
        game.message.clear();
        game.camera.center(Pos::cell(62, 64));
        for phase in 0..8 {
            game.world.tick = phase * 6;
            capture(&mut game, dir, &format!("{name}-water-{phase}"))?;
        }
        for (lane, pos) in [("north", Pos::cell(58, 49)), ("south", Pos::cell(58, 79))] {
            for north_dry in [true, false] {
                game.world.gate.tide = bw_sim::Tide::Open;
                game.world.gate.dry_arm = bw_sim::Arm::from_north(north_dry);
                game.camera.center(pos);
                capture(
                    &mut game,
                    dir,
                    &format!("{name}-{lane}-north-dry-{north_dry}"),
                )?;
            }
        }
        // A known enemy building is recorded by the ordinary visibility update.
        let mut memory =
            Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-memory-data")));
        memory.faction = faction;
        memory.start();
        memory.world.ai_enabled = false;
        let enemy = memory
            .world
            .entities
            .iter()
            .find(|e| e.owner == 1 && e.kind == Kind::Headquarters)
            .unwrap()
            .clone();
        let scout = memory
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap()
            .id;
        let original = memory
            .world
            .entities
            .iter()
            .find(|e| e.id == scout)
            .unwrap()
            .pos;
        {
            let e = memory
                .world
                .entities
                .iter_mut()
                .find(|e| e.id == scout)
                .unwrap();
            e.pos = Pos::raw(enemy.pos.x - 3 * bw_core::FP, enemy.pos.y);
            e.path.clear();
            e.order = Order::Idle;
        }
        memory.world.step();
        if !memory
            .world
            .knowledge(0)
            .visible
            .iter()
            .any(|e| e.id == enemy.id)
        {
            return Err("Memory fixture never observed headquarters".into());
        }
        memory
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == scout)
            .unwrap()
            .pos = original;
        memory.world.step();
        memory.camera.center(enemy.pos);
        memory.message.clear();
        capture(&mut memory, dir, &format!("{name}-last-seen"))?;
        let image = memory.canvas.pixels.clone();
        memory
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == enemy.id)
            .unwrap()
            .pos = Pos::cell(110, 110);
        memory.render();
        if image != memory.canvas.pixels {
            return Err("Hidden building movement changed chart".into());
        }
        memory
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == scout)
            .unwrap()
            .pos = Pos::raw(enemy.pos.x - 3 * bw_core::FP, enemy.pos.y);
        memory.tick();
        if !memory.ux.chart_memory.cleared.contains_key(&enemy.id) {
            return Err("Observing an empty old site did not retire its chart drawing".into());
        }
        memory
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == scout)
            .unwrap()
            .pos = original;
        memory.tick();
        capture(&mut memory, dir, &format!("{name}-last-seen-cleared"))?;
        if !memory.save_match() {
            return Err("Memory history save failed".into());
        }
        let cleared = memory.ux.chart_memory.clone();
        memory.ux.chart_memory = Default::default();
        memory.load_match();
        if memory.ux.chart_memory != cleared {
            return Err("Empty-site observation was lost on load".into());
        }

        // The washing composition is an explicitly controlled state fixture.
        crate::artistry_review::practice(&mut game)?;
        game.world.tick = 600;
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
        game.camera.center(Pos::cell(58, 49));
        game.message.clear();
        game.ux.traces.traces = [TraceKind::JackFoot, TraceKind::Rivet, TraceKind::Binding]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| Trace {
                kind,
                pos: Pos::cell(57 + i as i32 * 4, 51),
                lane: Terrain::Lane0,
                born_tick: 600,
                wash_start: None,
                dry_tick: None,
            })
            .collect();
        capture(&mut game, dir, &format!("{name}-traces-dry"))?;
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm::from_north(false);
        let gate = Event {
            tick: 600,
            kind: EventKind::GateChanged,
            player: None,
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        };
        game.ux.traces.record_event(&game.world, &gate);
        if !game.save_match() {
            return Err("Trace fixture save failed".into());
        }
        let saved = game.ux.traces.clone();
        let hash = game.world.state_hash();
        game.ux.traces = Default::default();
        game.load_match();
        if game.ux.traces != saved || game.world.state_hash() != hash {
            return Err("Trace sidecar failed roundtrip".into());
        }
        game.camera.center(Pos::cell(58, 49));
        game.message.clear();
        for age in [0, 6, 18, 36, 54, 90, 150, 210] {
            game.world.tick = 600 + age;
            capture(&mut game, dir, &format!("{name}-wash-{age:03}"))?;
        }
        // Trace-heavy renderer timing includes clipping and the semantic surface.
        game.world.tick = 600;
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
        let initial = game.ux.traces.traces.clone();
        game.ux.traces.traces = (0..96)
            .map(|i| {
                let mut t = initial[i % initial.len()].clone();
                t.pos = Pos::cell(52 + (i % 16) as i32, 48 + (i / 16 % 3) as i32);
                t.wash_start = None;
                t
            })
            .collect();
        let mut times = Vec::new();
        for _ in 0..60 {
            let now = std::time::Instant::now();
            game.render();
            times.push(now.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        std::fs::write(dir.join(format!("{name}-trace-performance.json")),serde_json::to_vec_pretty(&serde_json::json!({"scope":"CPU compositor, controlled 96 trace fixture, 60 samples, local hardware only","p95_ms":times[57],"p99_ms":times[59]})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        game.screen = Screen::Match;
        game.start();
        if !game.ux.traces.traces.is_empty() {
            return Err("New match retained traces".into());
        }
    }
    std::fs::write(dir.join("evidence.txt"),"Both factions: real guide buttons and keys; play/pause/step; world hash unchanged; controlled water/lane/memory/wash views; hidden building position invariance; trace sidecar roundtrip; new-match reset. Static capture and automated UI evidence, not human playtest.\n").map_err(|e|e.to_string())?;
    Ok(())
}

/// Prepared armies, then only ordinary commands/ticks: all three trace births
/// must be reachable through the actual simulation-to-presentation hook.
fn event_journey(base: PathBuf, dir: &Path) -> Result<(), String> {
    let mut game = Game::new_with_data_dir(base, dir.join("event-journey-data"));
    crate::artistry_review::practice(&mut game)?;
    let target = game
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 1 && e.kind.is_worker())
        .unwrap();
    target.kind = Kind::Reedguard;
    target.pos = Pos::cell(58, 51);
    target.hp = 1;
    target.max_hp = bw_content::spec(target.kind).health;
    target.order = Order::Hold;
    target.path.clear();
    target.path_index = 0;
    target.path_target = None;
    target.waypoints.clear();
    target.carried = 0;
    target.carried_kind = None;
    let target_id = target.id;
    game.world.players[1].crew = game
        .world
        .entities
        .iter()
        .filter(|e| e.owner == 1)
        .map(|e| bw_content::spec(e.kind).crew)
        .sum();
    let riveter = game
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Riveter)
        .unwrap()
        .id;
    let bulwark = game
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Bulwark)
        .unwrap()
        .id;
    game.world.reset_fixture_origin()?;
    game.issue(bw_sim::Command::Attack {
        units: vec![riveter],
        target: target_id,
    });
    game.selected = vec![bulwark];
    game.action(Action::Deploy);
    for _ in 0..60 {
        game.tick();
    }
    for kind in [TraceKind::JackFoot, TraceKind::Rivet, TraceKind::Binding] {
        if !game.ux.traces.traces.iter().any(|t| t.kind == kind) {
            return Err(format!("Real event journey did not produce {kind:?}"));
        }
    }
    let replay = dir.join("observed-traces.replay.json");
    game.world.export_replay(&replay)?;
    if bw_sim::World::replay(&replay)?.state_hash() != game.world.state_hash() {
        return Err("Trace event journey replay diverged".into());
    }
    let observed = game.ux.traces.clone();
    let hash = game.world.state_hash();
    if !game.save_match() {
        return Err("Real trace journey save failed".into());
    }
    game.ux.traces = Default::default();
    game.load_match();
    if game.ux.traces != observed || game.world.state_hash() != hash {
        return Err("Real trace journey save/load changed state".into());
    }
    game.camera.center(Pos::cell(59, 50));
    game.message.clear();
    capture(&mut game, dir, "observed-traces-real-events")?;
    Ok(())
}
