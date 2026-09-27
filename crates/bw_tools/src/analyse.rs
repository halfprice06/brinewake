//! `bw_tools analyse REPLAY`: play a recording through and print what
//! decided it — deaths by kind, cause and place, sluice captures, tide
//! switches, hold counts, seats knocked out, and each side's orders per
//! minute — so a referee does not have to piece the timeline together by
//! hand.  Any number of seats: the Split Basin's two or the Confluence's
//! three.  Every hold count start and break is followed by who stood at
//! each crossing mouth at that tick, so a count can be traced to the
//! machines that made or broke it.

use bw_core::Kind;
use bw_sim::{EventKind, Outcome, ReplayPlayer, World};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

const TICK_HZ: u64 = 30;

fn clock(tick: u64) -> String {
    let s = tick / TICK_HZ;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub struct Death {
    pub tick: u64,
    pub owner: u8,
    pub kind: String,
    pub cause: Option<Kind>,
    pub cell: (i32, i32),
}

#[derive(Default)]
pub struct Analysis {
    pub end_tick: u64,
    pub outcome: Option<Outcome>,
    /// One per seat, in seat order.
    pub factions: Vec<String>,
    pub deaths: Vec<Death>,
    /// (tick, text) in the order they happened.
    pub timeline: Vec<(u64, String)>,
    /// Orders sent by each seat, and how many the game refused.
    pub orders: Vec<u64>,
    pub refused: Vec<u64>,
    /// Orders per seat per match minute.
    pub orders_by_minute: Vec<BTreeMap<u64, u64>>,
}

/// What counts at a crossing mouth: every gun machine, and the Caisson.
/// Mirrors the simulation's (private) `holds_mouth_unit` rule.
fn holds_mouth(kind: Kind) -> bool {
    !kind.is_worker()
        && !kind.is_building()
        && (bw_content::spec(kind).damage > 0 || kind == Kind::Caisson)
}

/// A mouth's name: "N west" on the Split Basin; on a map of more seats the
/// arm's letter and whose bank the mouth is on, "E union bank".
fn mouth_name(world: &World, factions: &[String], arm: usize, index: usize) -> String {
    let layout = world.map.layout();
    let Some(arm) = layout.arms.get(arm) else {
        return "?".to_string();
    };
    let letter = match arm.name {
        "north" => "N",
        "south" => "S",
        other => other,
    };
    if factions.len() <= 2 {
        format!("{letter} {}", ["west", "east"][index])
    } else {
        let bank = factions
            .get(usize::from(arm.banks[index]))
            .map_or("?".to_string(), |f| f.to_lowercase());
        format!("{letter} {bank} bank")
    }
}

/// Per arm and mouth (in the map's order), per seat: the machines that
/// count toward a hold standing within `CROSSING_HOLD_RADIUS_CELLS` of the
/// mouth, by kind, and (rules 20) the Defense Nests whose footprint's
/// middle is in the ring, which block an enemy's count without holding.
/// Uses the same exclusions as `World::holds_lane`.
pub fn mouth_standing(world: &World) -> Vec<[Vec<BTreeMap<Kind, u32>>; 2]> {
    let radius = i64::from(bw_core::FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let seats = world.seat_count();
    let mouths: Vec<[bw_core::Pos; 2]> = world.map.layout().mouths().collect();
    let mut out: Vec<[Vec<BTreeMap<Kind, u32>>; 2]> = mouths
        .iter()
        .map(|_| [vec![BTreeMap::new(); seats], vec![BTreeMap::new(); seats]])
        .collect();
    for e in &world.entities {
        let place = if holds_mouth(e.kind) {
            e.pos
        } else if bw_sim::blocks_mouth_building(e.kind) {
            bw_sim::footprint_middle(e)
        } else {
            continue;
        };
        if e.hp <= 0 || e.build_remaining > 0 || e.aboard.is_some() {
            continue;
        }
        if usize::from(e.owner) >= seats {
            continue;
        }
        for (lane, pair) in mouths.iter().enumerate() {
            for (index, mouth) in pair.iter().enumerate() {
                if place.distance_sq(*mouth) <= radius {
                    *out[lane][index][e.owner as usize]
                        .entry(e.kind)
                        .or_default() += 1;
                }
            }
        }
    }
    out
}

/// One line naming who stood at each crossing mouth, `holder` (whose count
/// it is) first as "own", every other seat as "enemy".
pub fn mouths_line(world: &World, factions: &[String], holder: usize) -> String {
    let standing = mouth_standing(world);
    let words = |kinds: &BTreeMap<Kind, u32>| {
        if kinds.is_empty() {
            "-".to_string()
        } else {
            kinds
                .iter()
                .map(|(k, n)| format!("{n} {}", k.name().to_lowercase()))
                .collect::<Vec<_>>()
                .join(" ")
        }
    };
    let mut parts = Vec::new();
    for (lane, mouths) in standing.iter().enumerate() {
        for (index, at) in mouths.iter().enumerate() {
            let mut line = format!(
                "{}: own({}) {}",
                mouth_name(world, factions, lane, index),
                factions[holder],
                words(&at[holder])
            );
            for (seat, faction) in factions.iter().enumerate() {
                if seat != holder {
                    let _ = write!(line, " / enemy({faction}) {}", words(&at[seat]));
                }
            }
            parts.push(line);
        }
    }
    format!("  at the mouths: {}", parts.join("; "))
}

pub fn analyse(path: &Path) -> Result<Analysis, String> {
    let mut player = ReplayPlayer::open(path)?;
    let seats = player.world().seat_count();
    let mut a = Analysis {
        factions: player
            .world()
            .players
            .iter()
            .map(|p| format!("{:?}", p.faction).to_uppercase())
            .collect(),
        orders: vec![0; seats],
        refused: vec![0; seats],
        orders_by_minute: vec![BTreeMap::new(); seats],
        ..Default::default()
    };
    let name = |a: &Analysis, p: Option<u8>| {
        p.and_then(|p| a.factions.get(p as usize))
            .map_or("NOBODY".to_string(), Clone::clone)
    };
    let mut hold_before = vec![0u32; seats];
    let mut hold_peak = vec![0u32; seats];
    // A dead entity may be gone from the world by the time its Death is
    // read, so remember every kind seen.
    let mut kinds: BTreeMap<u32, Kind> = BTreeMap::new();
    loop {
        kinds.extend(player.world().entities.iter().map(|e| (e.id, e.kind)));
        if !player.step()? {
            break;
        }
        let world = player.world();
        for event in &world.events {
            match event.kind {
                EventKind::Death => {
                    let owner = event.player.unwrap_or(0);
                    a.deaths.push(Death {
                        tick: event.tick,
                        owner,
                        kind: event.text.clone(),
                        cause: event.cause,
                        cell: event.from.map_or((-1, -1), |p| p.cell_xy()),
                    });
                    let is_building = event
                        .entity
                        .and_then(|id| kinds.get(&id))
                        .is_some_and(|k| k.is_building());
                    if is_building {
                        let line = format!("{} lost its {}", name(&a, event.player), event.text);
                        a.timeline.push((event.tick, line));
                    }
                }
                EventKind::GateCaptureStarted => {
                    let line = format!("{} starts capturing the sluice", name(&a, event.player));
                    a.timeline.push((event.tick, line));
                }
                EventKind::GateCaptured => {
                    let line = format!("{} captures the sluice", name(&a, event.player));
                    a.timeline.push((event.tick, line));
                }
                EventKind::GateChanged => {
                    let line = format!("tide switch by {}: {}", name(&a, event.player), event.text);
                    a.timeline.push((event.tick, line));
                }
                EventKind::SwitchCancelled => {
                    let line = format!("{}: {}", name(&a, event.player), event.text);
                    a.timeline.push((event.tick, line));
                }
                EventKind::Victory | EventKind::Draw => {
                    let line = match event.player {
                        Some(p) => format!("result: {} ({})", name(&a, Some(p)), event.text),
                        None => format!("result: {}", event.text),
                    };
                    a.timeline.push((event.tick, line));
                }
                EventKind::Eliminated => {
                    a.timeline.push((event.tick, event.text.clone()));
                }
                EventKind::CommandAccepted | EventKind::CommandRejected => {
                    if let Some(p) = event.player.filter(|p| usize::from(*p) < seats) {
                        a.orders[p as usize] += 1;
                        *a.orders_by_minute[p as usize]
                            .entry(event.tick / (60 * TICK_HZ))
                            .or_default() += 1;
                        if event.kind == EventKind::CommandRejected {
                            a.refused[p as usize] += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        // A hold count starts when the gauge leaves zero and is broken when
        // it stops rising; the gauge drains rather than resets.
        for p in 0..seats {
            let now = world.lane_hold.get(p).copied().unwrap_or(0);
            let before = hold_before[p];
            if before == 0 && now > 0 {
                a.timeline
                    .push((world.tick, format!("{} starts a hold count", a.factions[p])));
                a.timeline
                    .push((world.tick, mouths_line(world, &a.factions, p)));
                hold_peak[p] = now;
            } else if now > before {
                hold_peak[p] = now;
            } else if now < before && before == hold_peak[p] {
                let line = format!(
                    "{}'s hold count broken at {}s of {}s",
                    a.factions[p],
                    before as u64 / TICK_HZ,
                    u64::from(world.hold_ticks()) / TICK_HZ
                );
                a.timeline.push((world.tick, line));
                a.timeline
                    .push((world.tick, mouths_line(world, &a.factions, p)));
            }
            hold_before[p] = now;
        }
    }
    a.end_tick = player.tick();
    a.outcome = player.world().outcome.clone();
    Ok(a)
}

impl Analysis {
    pub fn report(&self) -> String {
        let mut out = String::new();
        let outcome = match &self.outcome {
            Some(Outcome::Victory(p)) => format!(
                "{} won",
                self.factions.get(*p as usize).map_or("?", String::as_str)
            ),
            Some(Outcome::Draw) => "draw".to_string(),
            None => "no result in the recording".to_string(),
        };
        let _ = writeln!(
            out,
            "match: {} at {} (tick {})",
            outcome,
            clock(self.end_tick),
            self.end_tick
        );
        let minutes = (self.end_tick as f64 / (60.0 * TICK_HZ as f64)).max(1.0 / 60.0);
        for p in 0..self.factions.len() {
            let _ = writeln!(
                out,
                "{}: {} orders ({:.1} a minute), {} refused; lost {}",
                self.factions[p],
                self.orders[p],
                self.orders[p] as f64 / minutes,
                self.refused[p],
                self.deaths.iter().filter(|d| d.owner as usize == p).count()
            );
        }
        let _ = writeln!(out, "\nlosses by kind (and what killed them):");
        for p in 0..self.factions.len() {
            let mut by_kind: BTreeMap<&str, (u32, BTreeMap<String, u32>)> = BTreeMap::new();
            for d in self.deaths.iter().filter(|d| d.owner as usize == p) {
                let slot = by_kind.entry(&d.kind).or_default();
                slot.0 += 1;
                let cause = d
                    .cause
                    .map_or("unknown".to_string(), |k| k.name().to_string());
                *slot.1.entry(cause).or_default() += 1;
            }
            let _ = writeln!(out, "  {}:", self.factions[p]);
            for (kind, (n, causes)) in by_kind {
                let causes: Vec<_> = causes.iter().map(|(c, n)| format!("{c} {n}")).collect();
                let _ = writeln!(out, "    {n:>3} {kind} ({})", causes.join(", "));
            }
        }
        let _ = writeln!(
            out,
            "\nwhere they fell (8x8 cell blocks, every side, top 8):"
        );
        let seats = self.factions.len();
        let mut places: BTreeMap<(i32, i32), Vec<u32>> = BTreeMap::new();
        for d in &self.deaths {
            let row = places
                .entry((d.cell.0 / 8 * 8, d.cell.1 / 8 * 8))
                .or_insert_with(|| vec![0; seats]);
            if let Some(n) = row.get_mut(d.owner as usize) {
                *n += 1;
            }
        }
        let mut places: Vec<_> = places.into_iter().collect();
        places.sort_by_key(|(_, n)| std::cmp::Reverse(n.iter().sum::<u32>()));
        for ((x, y), n) in places.into_iter().take(8) {
            let counts: Vec<String> = self
                .factions
                .iter()
                .zip(&n)
                .map(|(f, n)| format!("{f} {n}"))
                .collect();
            let _ = writeln!(
                out,
                "  cells {x}-{} x {y}-{}: {}",
                x + 7,
                y + 7,
                counts.join(", ")
            );
        }
        let _ = writeln!(out, "\norders per minute:");
        let last = self.end_tick / (60 * TICK_HZ);
        for p in 0..self.factions.len() {
            let row: Vec<_> = (0..=last)
                .map(|m| {
                    self.orders_by_minute[p]
                        .get(&m)
                        .copied()
                        .unwrap_or(0)
                        .to_string()
                })
                .collect();
            let _ = writeln!(out, "  {}: {}", self.factions[p], row.join(" "));
        }
        let _ = writeln!(
            out,
            "\ntimeline (captures, switches, holds, buildings lost, seats out):"
        );
        for (tick, line) in &self.timeline {
            let _ = writeln!(out, "  {} {line}", clock(*tick));
        }
        out
    }

    pub fn deaths_csv(&self) -> String {
        let mut out = String::from("tick,clock,owner,kind,cause,x,y\n");
        for d in &self.deaths {
            let _ = writeln!(
                out,
                "{},{},{},{},{},{},{}",
                d.tick,
                clock(d.tick),
                d.owner,
                d.kind,
                d.cause.map_or("", |k| k.name()),
                d.cell.0,
                d.cell.1
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;
    use bw_sim::{CROSSING_MOUTHS, Command, MapId, World};

    #[test]
    fn a_recording_is_played_through_and_summarised() {
        let dir = std::env::temp_dir().join(format!("bw-analyse-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("match.replay.json");
        let mut world = World::new(31, Faction::Union);
        for _ in 0..600 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        let analysis = analyse(&path).unwrap();
        assert_eq!(analysis.end_tick, 600);
        let report = analysis.report();
        assert!(report.contains("UNION"), "{report}");
        assert!(report.contains("orders per minute"), "{report}");
        assert!(analysis.deaths_csv().starts_with("tick,clock,owner"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_machines_at_each_crossing_mouth_are_named() {
        let mut world = World::new(31, Faction::Union);
        let factions = ["UNION".to_string(), "ASSEMBLY".to_string()];
        let empty = mouths_line(&world, &factions, 0);
        assert!(
            empty.contains("N west: own(UNION) - / enemy(ASSEMBLY) -"),
            "{empty}"
        );
        world.spawn_for_tests(0, Kind::Riveter, CROSSING_MOUTHS[0][0]);
        world.spawn_for_tests(0, Kind::Riveter, CROSSING_MOUTHS[0][0]);
        world.spawn_for_tests(1, Kind::Caisson, CROSSING_MOUTHS[1][1]);
        // A worker does not count toward a hold.
        world.spawn_for_tests(1, Kind::Hook, CROSSING_MOUTHS[0][1]);
        let standing = mouth_standing(&world);
        assert_eq!(standing[0][0][0].get(&Kind::Riveter), Some(&2));
        assert!(standing[0][0][1].is_empty());
        assert_eq!(standing[1][1][1].get(&Kind::Caisson), Some(&1));
        assert!(standing[0][1][1].is_empty());
        let line = mouths_line(&world, &factions, 1);
        assert!(
            line.contains("N west: own(ASSEMBLY) - / enemy(UNION) 2 riveter"),
            "{line}"
        );
        assert!(line.contains("S east: own(ASSEMBLY) 1 caisson"), "{line}");
    }

    #[test]
    fn a_three_seat_recording_is_summarised_with_every_seat() {
        let dir = std::env::temp_dir().join(format!("bw-analyse-3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("match.replay.json");
        let factions = [Faction::Union, Faction::Assembly, Faction::Compact];
        let mut world = World::with_map(31, MapId::Confluence, &factions).unwrap();
        world.ai_enabled = false;
        for _ in 0..300 {
            world.step();
        }
        world.issue(2, Command::Surrender).unwrap();
        for _ in 0..300 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        let analysis = analyse(&path).unwrap();
        assert_eq!(analysis.factions, ["UNION", "ASSEMBLY", "COMPACT"]);
        let report = analysis.report();
        assert!(report.contains("COMPACT: "), "{report}");
        assert!(report.contains("IS OUT: SURRENDERED"), "{report}");
        assert_eq!(report.matches("orders (").count(), 3, "{report}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_confluence_mouths_are_named_by_arm_and_bank() {
        let factions = [Faction::Union, Faction::Assembly, Faction::Compact];
        let mut world = World::with_map(31, MapId::Confluence, &factions).unwrap();
        let names: Vec<String> = factions
            .iter()
            .map(|f| format!("{f:?}").to_uppercase())
            .collect();
        let mouth = world.map.layout().mouths().next().unwrap()[0];
        world.spawn_for_tests(2, Kind::Brander, mouth);
        let line = mouths_line(&world, &names, 2);
        assert!(
            line.contains("E union bank: own(COMPACT) 1 brander"),
            "{line}"
        );
        assert!(
            line.contains("/ enemy(UNION) - / enemy(ASSEMBLY) -"),
            "{line}"
        );
        assert_eq!(
            line.matches("bank:").count(),
            6,
            "three arms, two mouths each: {line}"
        );
    }
}
