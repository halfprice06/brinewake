//! Controlled engine captures for the art and artistry v15 pass.
//!
//! Each capture sets only presentation clocks and public world state (hull,
//! positions, the gate change tick) so every new treatment is visible in an
//! ordinary render.  Nothing here changes rules, saves or replays.
use crate::game::{Game, Screen};
use crate::zoom::Zoom;
use bw_core::{Faction, Kind, Pos};
use std::path::{Path, PathBuf};

fn fresh(base: &Path, dir: &Path, faction: Faction) -> Game {
    let mut g = Game::new_with_data_dir(base.to_path_buf(), dir.join("session"));
    g.resize_view(1920, 1080);
    g.faction = faction;
    g.start();
    g.world.ai_enabled = false;
    g.message.clear();
    g.cursor = (-999, -999);
    g
}

fn own_units(g: &Game) -> Vec<u32> {
    g.world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && !e.kind.is_building())
        .map(|e| e.id)
        .collect()
}

fn place(g: &mut Game, id: u32, pos: Pos, facing: u8) {
    if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
        e.pos = pos;
        e.facing = facing;
    }
}

fn look(g: &mut Game, at: Pos, zoom: Zoom) {
    g.zoom = zoom;
    g.camera.center(at);
}

fn shoot(g: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    g.screenshot(&dir.join(format!("{name}.png")))
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut g = fresh(&base, dir, Faction::Union);

    // Home: the living vignette with gulls mid-flight.
    g.screen = Screen::Menu;
    g.frame = 45;
    shoot(&mut g, dir, "home")?;
    g.frame = 400;
    shoot(&mut g, dir, "home-calm")?;
    g.screen = Screen::Match;

    // The ordinary start for both factions, chart minimap included, and the
    // overview where the chart contours, hatch and rim carry the screen.
    for faction in [Faction::Union, Faction::Assembly] {
        let mut g = fresh(&base, dir, faction);
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        g.set_zoom(Zoom::Wide, g.world_view().center());
        shoot(&mut g, dir, &format!("{name}-2x"))?;
        g.set_zoom(Zoom::Overview, g.world_view().center());
        shoot(&mut g, dir, &format!("{name}-1x"))?;
    }

    // Wind: the same bank at three ticks; the plants lean as the gust passes.
    let hq = g
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .map(|e| e.pos)
        .unwrap_or(Pos::cell(20, 64));
    // The nearest reed clump or marsh grass to the headquarters: the tall
    // plants show the lean best.  Ticks are chosen from that plant's own
    // gust field, jitter included, so each capture shows one strength.
    let (hx, hy) = hq.cell_xy();
    let mut plant = None;
    'search: for radius in 1..40 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (x, y) = (hx + dx, hy + dy);
                if matches!(
                    crate::game::bank_plant(&g.world.map, x, y),
                    Some("reed_clump" | "marsh_grass")
                ) {
                    plant = Some(Pos::cell(x, y));
                    break 'search;
                }
            }
        }
    }
    let bank = plant.unwrap_or(Pos::cell(hx - 6, hy - 4));
    look(&mut g, bank, Zoom::Detail);
    let (bx, by) = g.camera.project(bank);
    let (wx, wy) = (
        i64::from(bx) + i64::from(g.camera.x),
        i64::from(by) + i64::from(g.camera.y),
    );
    let (cx, cy) = bank.cell_xy();
    let jitter = crate::wind::cell_jitter(cx, cy);
    for strength in 0..3u8 {
        let tick = (0..720u64)
            .find(|&t| crate::wind::strength_with_jitter(wx, wy, t, jitter) == strength)
            .unwrap_or(0);
        g.world.tick = tick;
        look(&mut g, bank, Zoom::Detail);
        shoot(&mut g, dir, &format!("wind-{strength}"))?;
    }
    g.world.tick = 0;

    // Daylight: the same base at morning, noon and evening.
    let mut timings = Vec::new();
    for (name, tick) in [
        ("morning", 0u64),
        ("noon", 12 * 60 * 30),
        ("evening", 20 * 60 * 30),
    ] {
        g.world.tick = tick;
        g.set_zoom(Zoom::Wide, g.world_view().center());
        g.camera.center(hq);
        shoot(&mut g, dir, &format!("daylight-{name}"))?;
        let mut samples = Vec::new();
        for _ in 0..40 {
            let t = std::time::Instant::now();
            g.render();
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        timings.push(serde_json::json!({
            "phase": name,
            "daylight": format!("{:?}", crate::daylight::phase(tick)),
            "world_scale": 2,
            "size": [1920, 1080],
            "p95_ms": samples[37],
            "p99_ms": samples[39],
            "samples": 40,
            "scope": "CPU compositor, ordinary starting field on local hardware"
        }));
    }
    g.world.tick = 0;

    // Damage: the headquarters at a quarter hull with a fresh repair patch,
    // one machine failing and one dented.
    let units = own_units(&g);
    if let Some(e) = g
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
    {
        e.hp = e.max_hp / 4;
    }
    let hq_id = g
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .map(|e| e.id)
        .unwrap_or(1);
    g.repair_marks.record(hq_id, 0);
    g.repair_marks.record(hq_id, 0);
    if let Some(&id) = units.first()
        && let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id)
    {
        e.hp = e.max_hp / 5;
    }
    if let Some(&id) = units.get(1)
        && let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id)
    {
        e.hp = e.max_hp * 45 / 100;
    }
    look(&mut g, hq, Zoom::Detail);
    shoot(&mut g, dir, "damage-3x")?;
    g.world.tick = 20 * 60 * 30;
    shoot(&mut g, dir, "damage-evening-3x")?;
    g.world.tick = 0;

    // A machine behind a building: the ghost through the headquarters and
    // the worker parked on its north face.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    let (hx, hy) = hq.cell_xy();
    for (i, &id) in units.iter().enumerate() {
        place(
            &mut g,
            id,
            if i == 0 {
                Pos::cell(hx + 1, hy - 1)
            } else {
                Pos::cell(hx - 10 + i as i32, hy + 10)
            },
            5,
        );
    }
    look(&mut g, hq, Zoom::Detail);
    shoot(&mut g, dir, "ghost-behind-building-3x")?;

    // Enemy edge and grounded shadows: enemy machines beside our own at the
    // base, both factions' machinery in one frame.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    let (hx, hy) = hq.cell_xy();
    for (i, &id) in units.iter().enumerate() {
        place(
            &mut g,
            id,
            Pos::cell(hx + 4 + (i as i32 % 3) * 2, hy + 2 + (i as i32 / 3) * 2),
            1,
        );
    }
    let enemies: Vec<u32> = g
        .world
        .entities
        .iter()
        .filter(|e| e.owner == 1 && !e.kind.is_building())
        .map(|e| e.id)
        .take(3)
        .collect();
    for (i, &id) in enemies.iter().enumerate() {
        place(&mut g, id, Pos::cell(hx + 11 + i as i32 * 2, hy + 3), 5);
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.surge_remaining = 0;
            e.order = bw_sim::Order::Hold;
        }
    }
    look(&mut g, Pos::cell(hx + 8, hy + 3), Zoom::Detail);
    shoot(&mut g, dir, "enemy-edge-3x")?;

    // Lane memory: the north lane drying through its four stages, seen with
    // a worker beside it, then the flood front on the south lane and the
    // receding sheen on the north two seconds after a change.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    let watcher = units.first().copied();
    if let Some(id) = watcher {
        place(&mut g, id, Pos::cell(57, 47), 2);
    }
    for stage in 0..4u64 {
        g.tide_change = Some(crate::lane_memory::TideChange::from_change(
            &g.world.gate,
            "north open",
            1000,
        ));
        g.world.tick = 1000 + stage * crate::lane_memory::STAGE_TICKS + 100;
        look(&mut g, Pos::cell(58, 49), Zoom::Detail);
        shoot(&mut g, dir, &format!("lane-drying-{stage}"))?;
    }
    if let Some(&id) = units.get(1) {
        place(&mut g, id, Pos::cell(57, 81), 2);
    }
    g.tide_change = Some(crate::lane_memory::TideChange::from_change(
        &g.world.gate,
        "north open",
        1000,
    ));
    g.world.tick = 1030;
    look(&mut g, Pos::cell(60, 64), Zoom::Wide);
    shoot(&mut g, dir, "lane-fronts-2x")?;
    g.world.tick = 1048;
    shoot(&mut g, dir, "lane-fronts-late-2x")?;

    // Wading: machines standing and moving in the flooded south lane.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    for (i, &id) in units.iter().take(4).enumerate() {
        place(
            &mut g,
            id,
            Pos::cell(54 + i as i32 * 2, 79),
            (i as u8 * 3) % 8,
        );
    }
    if let Some(&id) = units.get(1) {
        let pos = g.world.entities.iter().find(|e| e.id == id).map(|e| e.pos);
        if let Some(pos) = pos {
            g.motion.insert(id, (pos, 64, g.world.tick));
        }
    }
    look(&mut g, Pos::cell(57, 79), Zoom::Detail);
    shoot(&mut g, dir, "wading-3x")?;

    // The drowned town: submerged shapes beside a scout at the cut's edge.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    if let Some(&id) = units.first() {
        place(&mut g, id, Pos::cell(51, 41), 2);
    }
    if let Some(&id) = units.get(1) {
        place(&mut g, id, Pos::cell(51, 31), 2);
    }
    look(&mut g, Pos::cell(57, 36), Zoom::Wide);
    shoot(&mut g, dir, "submerged-2x")?;
    look(&mut g, Pos::cell(60, 38), Zoom::Detail);
    shoot(&mut g, dir, "submerged-3x")?;

    // Living coast: the birds lifting off and the crab hiding as a machine
    // arrives, then the empty sites.
    let mut g = fresh(&base, dir, Faction::Union);
    let units = own_units(&g);
    if let Some(&id) = units.first() {
        place(&mut g, id, Pos::cell(51, 45), 2);
    }
    if let Some(&id) = units.get(1) {
        place(&mut g, id, Pos::cell(51, 89), 2);
    }
    g.world.tick = 100;
    for (i, age) in [6u64, 18, 30].into_iter().enumerate() {
        g.coast_flush.insert(0, (100 - age, None));
        look(&mut g, Pos::cell(49, 44), Zoom::Detail);
        shoot(&mut g, dir, &format!("coast-birds-{i}"))?;
    }
    for (i, age) in [3u64, 12].into_iter().enumerate() {
        g.coast_flush.insert(1, (100 - age, None));
        look(&mut g, Pos::cell(49, 88), Zoom::Detail);
        shoot(&mut g, dir, &format!("coast-crab-{i}"))?;
    }

    // Idle acting: a worker group at rest caught on both acted frames.
    let mut g = fresh(&base, dir, Faction::Assembly);
    let ids = own_units(&g);
    for (i, &id) in ids.iter().enumerate() {
        place(
            &mut g,
            id,
            Pos::cell(
                hq.cell_xy().0 + 4 + (i as i32 % 3) * 2,
                hq.cell_xy().1 - 2 + (i as i32 / 3) * 2,
            ),
            1,
        );
    }
    look(
        &mut g,
        Pos::cell(hq.cell_xy().0 + 6, hq.cell_xy().1),
        Zoom::Detail,
    );
    for tick in 0..120u64 {
        g.world.tick = tick;
        let acting = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && !e.kind.is_building())
            .any(|e| (tick + u64::from(e.id) * 17) % (72 + u64::from(e.id % 5) * 12) < 6);
        if acting {
            shoot(&mut g, dir, "idle-acting-3x")?;
            break;
        }
    }

    std::fs::write(
        dir.join("perf.json"),
        serde_json::to_vec_pretty(&timings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
