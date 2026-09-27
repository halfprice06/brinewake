use bw_core::{Faction, Kind, Pos};
use bw_sim::{Command, EventKind, Formation, Order, World};
use std::path::PathBuf;
use std::time::Instant;

mod analyse;
mod depth;
mod map_check;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("trace") => {
            let path = args
                .get(2)
                .ok_or("usage: bw_tools trace SAVE ENTITY [ticks]")?;
            let id = args
                .get(3)
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or("missing entity ID")?;
            let ticks = args
                .get(4)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(30)
                .min(600);
            let mut world = World::load(path)?;
            for _ in 0..ticks {
                if let Some(e) = world.entities.iter().find(|e| e.id == id) {
                    println!(
                        "tick={} pos={:?} carried={} order={:?} path={}/{} next={:?} goal={:?} blocked={}",
                        world.tick,
                        e.pos,
                        e.carried,
                        e.order,
                        e.path_index,
                        e.path.len(),
                        e.path.get(e.path_index),
                        e.path_target,
                        e.blocked_ticks
                    );
                }
                world.step();
            }
        }
        Some("skirmish") => {
            let out = args
                .iter()
                .position(|s| s == "--out")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("output"));
            let ticks = args
                .get(2)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(36000)
                .min(100_000);
            let faction = if args.iter().any(|s| s == "--assembly") {
                Faction::Assembly
            } else {
                Faction::Union
            };
            // `--map confluence`: the practice AI plays seats 1 and 2 and
            // seat 0 stands idle, so the two fight each other and the idle
            // seat; the scripted player knows only the Split Basin.
            let map = match args
                .iter()
                .position(|s| s == "--map")
                .and_then(|i| args.get(i + 1))
            {
                Some(name) => map_check::parse_map(name)?,
                None => bw_sim::MapId::SplitBasin,
            };
            let mut world = if map.is_split_basin() {
                World::new(17, faction)
            } else {
                let rival = match faction {
                    Faction::Union => Faction::Assembly,
                    Faction::Assembly | Faction::Compact => Faction::Union,
                };
                let factions = [faction, rival, faction];
                World::with_map(17, map, &factions[..map.layout().seat_count()])?
            };
            let mut counters = std::collections::BTreeMap::<String, u64>::new();
            let start = Instant::now();
            for _ in 0..ticks {
                if map.is_split_basin() && world.tick.is_multiple_of(60) {
                    scripted_player(&mut world);
                }
                world.step();
                for event in &world.events {
                    if matches!(
                        event.kind,
                        EventKind::Deposit
                            | EventKind::BuildCompleted
                            | EventKind::ProductionCompleted
                            | EventKind::GateCaptured
                            | EventKind::GateChanged
                            | EventKind::Deploy
                            | EventKind::ArtilleryWarning
                            | EventKind::ResearchStarted
                            | EventKind::ResearchCompleted
                            | EventKind::ResearchCancelled
                            | EventKind::Surge
                            | EventKind::Shot
                            | EventKind::Damage
                            | EventKind::Death
                    ) {
                        *counters.entry(format!("{:?}", event.kind)).or_default() += 1;
                    }
                }
                if world.tick.is_multiple_of(3000) {
                    println!(
                        "tick={} entities={} salvage=[{},{}] crews=[{},{}] gate={:?}",
                        world.tick,
                        world.entities.len(),
                        world.players[0].salvage,
                        world.players[1].salvage,
                        world.players[0].crew,
                        world.players[1].crew,
                        world.gate.owner
                    );
                }
                if world.outcome.is_some() {
                    break;
                }
            }
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            world.save(out.join("skirmish-final.json"))?;
            world.export_replay(out.join("skirmish.replay.json"))?;
            let report = serde_json::json!({"ticks":world.tick,"elapsed_seconds":start.elapsed().as_secs_f64(),"outcome":format!("{:?}",world.outcome),"events":counters,"hash":world.state_hash(),"evidence":"scripted control versus built-in AI; not human fun or balance evidence"});
            std::fs::write(
                out.join("skirmish-report.json"),
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            println!("{report}");
        }
        Some("replay") => {
            let path = args.get(2).ok_or("usage: bw_tools replay FILE")?;
            let w = World::replay(path)?;
            println!("tick={} state={}", w.tick, w.state_hash());
        }
        Some("replay-hashes") => {
            // Play a recording tick by tick: the state hash every N ticks and
            // the slowest ticks, so two builds can be shown to play it alike
            // and a slow stretch can be found.
            let path = args
                .get(2)
                .ok_or("usage: bw_tools replay-hashes FILE [EVERY] [FROM]")?;
            let every = args
                .get(3)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000)
                .max(1);
            let from = args.get(4).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let mut player = bw_sim::ReplayPlayer::open(path)?;
            let mut slowest: Vec<(u128, u64)> = Vec::new();
            loop {
                let t = Instant::now();
                if !player.step()? {
                    break;
                }
                let tick = player.tick();
                if tick >= from {
                    slowest.push((t.elapsed().as_micros(), tick));
                    if tick % every == 0 {
                        println!("tick={tick} state={}", player.world().state_hash());
                    }
                }
            }
            println!(
                "end tick={} state={}",
                player.tick(),
                player.world().state_hash()
            );
            slowest.sort_unstable_by(|a, b| b.cmp(a));
            for (micros, tick) in slowest.iter().take(5) {
                println!("slow tick={tick} ms={:.1}", *micros as f64 / 1000.0);
            }
        }
        Some("analyse") | Some("analyze") => {
            let path = args
                .get(2)
                .ok_or("usage: bw_tools analyse REPLAY [--csv DEATHS.csv]")?;
            let analysis = analyse::analyse(std::path::Path::new(path))?;
            print!("{}", analysis.report());
            if let Some(csv) = args
                .iter()
                .position(|s| s == "--csv")
                .and_then(|i| args.get(i + 1))
            {
                std::fs::write(csv, analysis.deaths_csv()).map_err(|e| e.to_string())?;
                println!("\nwrote {csv} ({} deaths)", analysis.deaths.len());
            }
        }
        Some("inspect") => {
            let path = args.get(2).ok_or("usage: bw_tools inspect SAVE")?;
            let w = World::load(path)?;
            println!(
                "tick={} entities={} state={}",
                w.tick,
                w.entities.len(),
                w.state_hash()
            );
        }
        Some("benchmark") => {
            let ticks = args
                .get(2)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(9000);
            if ticks > 1_000_000 {
                return Err("benchmark tick count exceeds safety bound".into());
            }
            let mut w = World::new(42, Faction::Union);
            let mut samples = Vec::new();
            let start = Instant::now();
            for _ in 0..ticks {
                let t = Instant::now();
                w.step();
                samples.push(t.elapsed().as_nanos() as u64);
                if w.outcome.is_some() {
                    break;
                }
            }
            samples.sort_unstable();
            let pct = |p: usize| {
                samples
                    .get(samples.len().saturating_sub(1) * p / 100)
                    .copied()
                    .unwrap_or(0) as f64
                    / 1e6
            };
            println!(
                "ticks={} elapsed_s={:.3} entities={} p50_ms={:.4} p95_ms={:.4} p99_ms={:.4} hash={}",
                w.tick,
                start.elapsed().as_secs_f64(),
                w.entities.len(),
                pct(50),
                pct(95),
                pct(99),
                w.state_hash()
            );
        }
        Some("map-check") => {
            let maps = match args.get(2) {
                Some(name) => vec![map_check::parse_map(name)?],
                None => bw_sim::MapId::ALL.to_vec(),
            };
            for map in maps {
                let report = map_check::map_check(map)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
                );
            }
        }
        Some("depth-check") => {
            let out = args
                .iter()
                .position(|s| s == "--out")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("output/depth-check"));
            depth::run(&out)?;
        }
        Some("save-check") => {
            let path = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("output/save-check.json"));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut a = World::new(23, Faction::Assembly);
            for _ in 0..300 {
                a.step()
            }
            a.save(&path)?;
            let mut b = World::load(&path)?;
            if a.state_hash() != b.state_hash() {
                return Err("roundtrip state mismatch".into());
            }
            for _ in 0..300 {
                a.step();
                b.step();
                if a.state_hash() != b.state_hash() {
                    return Err(format!("save divergence at tick {}", a.tick));
                }
            }
            println!("Save/resume matched through tick {}", a.tick);
        }
        _ => println!(
            "BRINEWAKE tools\n  skirmish [ticks] [--assembly] [--map confluence]\n  benchmark [ticks]\n  depth-check [--out DIR]\n  save-check [path]\n  replay <path>\n  analyse <replay> [--csv deaths.csv]\n  inspect <save>"
        ),
    };
    Ok(())
}

/// Deliberately simple fair command fixture, not a second game-rule implementation.
fn scripted_player(w: &mut World) {
    let own: Vec<_> = w
        .entities
        .iter()
        .filter(|e| e.owner == 0 && e.hp > 0)
        .cloned()
        .collect();
    let Some(hq) = own.iter().find(|e| e.kind == Kind::Headquarters) else {
        return;
    };
    let faction = w.players[0].faction;
    let workers: Vec<_> = own.iter().filter(|e| e.kind.is_worker()).collect();
    for worker in workers.iter().filter(|e| matches!(e.order, Order::Idle)) {
        if let Some(resource) = w
            .map
            .resources
            .iter()
            .filter(|r| r.remaining > 0 && w.visible(0, r.pos) && w.gatherable(worker.kind, r))
            .min_by_key(|r| {
                let assigned = own
                    .iter()
                    .filter(|e| matches!(e.order,Order::Gather{resource} if resource==r.id))
                    .count();
                (assigned >= 6, worker.pos.distance_sq(r.pos), r.id)
            })
            .map(|r| r.id)
        {
            let _ = w.issue(
                0,
                Command::Gather {
                    units: vec![worker.id],
                    resource,
                },
            );
        }
    }
    if workers.len() < 12 && hq.queue.is_empty() {
        let _ = w.issue(
            0,
            Command::Train {
                building: hq.id,
                kind: faction.worker(),
            },
        );
    }
    if !own.iter().any(|e| e.kind == Kind::Works)
        && let Some(worker) = workers.first()
    {
        let (x, y) = hq.pos.cell_xy();
        let _ = w.issue(
            0,
            Command::Build {
                worker: worker.id,
                kind: Kind::Works,
                pos: Pos::cell(x + 8, y - 4),
                queued: false,
            },
        );
    }
    for works in own
        .iter()
        .filter(|e| e.kind == Kind::Works && e.build_remaining == 0 && e.queue.len() < 2)
    {
        let i = match (w.tick / 120) % 5 {
            0 => 1,
            1 => 2,
            _ => 0,
        };
        let _ = w.issue(
            0,
            Command::Train {
                building: works.id,
                kind: faction.army()[i],
            },
        );
    }
    let army: Vec<_> = own
        .iter()
        .filter(|e| !e.kind.is_worker() && !e.kind.is_building())
        .collect();
    // Give specialists a real commitment window.  A Bulwark faces the
    // opponent before deploying; the Loom deploys into the same advancing
    // position and relies on its normal visible-target acquisition.  Both
    // actions use the public command path and are issued only once per unit.
    if w.tick >= 1_800 && w.tick.is_multiple_of(600) {
        for specialist in army
            .iter()
            .filter(|e| matches!(e.kind, Kind::Bulwark | Kind::Loom))
            .filter(|e| !e.deployed && e.deploy_remaining == 0)
        {
            if specialist.kind == Kind::Bulwark {
                let _ = w.issue(
                    0,
                    Command::Face {
                        units: vec![specialist.id],
                        target: Pos::cell(110, 64),
                    },
                );
            }
            let _ = w.issue(
                0,
                Command::Deploy {
                    units: vec![specialist.id],
                },
            );
        }
    }
    if w.tick >= 2_400 && w.tick.is_multiple_of(1_800) && !army.is_empty() {
        let ids: Vec<_> = army.iter().map(|e| e.id).collect();
        let formation = if (w.tick / 1_800).is_multiple_of(2) {
            Formation::Line
        } else {
            Formation::Loose
        };
        let _ = w.issue(
            0,
            Command::SetFormation {
                units: ids,
                formation,
            },
        );
    }
    if w.tick >= 3_000 && w.tick.is_multiple_of(900) {
        let surge_ids: Vec<_> = army
            .iter()
            .filter(|e| {
                e.deploy_remaining == 0
                    && !e.deployed
                    && e.surge_remaining == 0
                    && e.surge_cooldown == 0
                    && !matches!(e.order, Order::Capture)
            })
            .take(3)
            .map(|e| e.id)
            .collect();
        if !surge_ids.is_empty()
            && w.players[0].pressure
                >= bw_content::SURGE_PRESSURE.saturating_mul(surge_ids.len() as u32)
        {
            let _ = w.issue(0, Command::Surge { units: surge_ids });
        }
    }
    if w.tick >= 1_800
        && w.players[0].doctrine.is_none()
        && w.players[0].research.is_none()
        && w.players[0].salvage >= bw_content::DOCTRINE_SALVAGE
        && w.players[0].pressure >= bw_content::DOCTRINE_PRESSURE
    {
        let doctrine = if faction == Faction::Union {
            bw_content::Doctrine::Hauling
        } else {
            bw_content::Doctrine::FireControl
        };
        let _ = w.issue(
            0,
            Command::Research {
                building: hq.id,
                doctrine,
            },
        );
    }
    if army.len() >= 6 {
        let idle: Vec<_> = army
            .iter()
            .filter(|e| {
                matches!(e.order, Order::Idle | Order::Hold)
                    && e.deploy_remaining == 0
                    && !e.deployed
            })
            .map(|e| e.id)
            .collect();
        if w.gate.owner != Some(0) {
            if !idle.is_empty() {
                let _ = w.issue(
                    0,
                    Command::Capture {
                        units: idle.into_iter().take(3).collect(),
                    },
                );
            }
        } else {
            if w.gate.warning_until.is_none()
                && w.tick >= w.gate.locked_until
                && w.tick.is_multiple_of(1800)
            {
                let _ = w.issue(0, Command::SwitchGate);
            }
            if !idle.is_empty() {
                // Opposing start is charted public scenario geometry; do not query hidden enemies.
                let _ = w.issue(
                    0,
                    Command::AttackMove {
                        units: idle,
                        target: Pos::cell(110, 64),
                        queued: false,
                    },
                );
            }
        }
    }
    if w.tick > 9000 && army.len() >= 10 && w.tick.is_multiple_of(900) {
        let ids = army
            .iter()
            .filter(|e| e.deploy_remaining == 0 && !e.deployed)
            .map(|e| e.id)
            .collect();
        let _ = w.issue(
            0,
            Command::AttackMove {
                units: ids,
                target: Pos::cell(110, 64),
                queued: false,
            },
        );
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
