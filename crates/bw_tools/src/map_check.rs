//! `bw_tools map-check`: measures a map's fairness on the grid the game
//! plays, with the design sketch's metric (8 neighbours, diagonal steps at
//! 141/100, no corner cutting, silt at 95% speed, wading at 75%).
//!
//! For every seat it measures the walk from its headquarters to the station
//! (every lane dry, and at the neutral tide), to the mouths on its own bank,
//! to the other headquarters, and to the wreck beds and wells nearest it. It
//! fails when a seat's walk differs from another's by more than the design's
//! 2%, or by more than a cell on a walk short enough that a cell is more.

use bw_core::{Faction, Pos, Terrain};
use bw_sim::{MapId, World};
use serde_json::{Value, json};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// How far apart two seats' walks may be, in percent of the longer, or in
/// hundredths of a cell for short walks, where a cell is more than 2%.
const FAIR_PERCENT: u64 = 2;
const FAIR_SLACK: u64 = 100;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tide {
    /// Every lane and rim dry.
    Dry,
    /// Every lane and rim shallow: walkers wade.
    Neutral,
}

struct Grid<'a> {
    world: &'a World,
    size: i32,
}

impl Grid<'_> {
    fn walkable(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < self.size
            && y < self.size
            && self.world.map.terrain(x, y).walkable()
    }

    fn cost(&self, x: i32, y: i32, tide: Tide) -> u64 {
        match self.world.map.terrain(x, y) {
            Terrain::Silt => 105,
            terrain if terrain.is_tidal() && tide == Tide::Neutral => 133,
            _ => 100,
        }
    }

    /// Octile walking distance from `from` to every cell, in hundredths of
    /// a cell; `u64::MAX` where it cannot walk.
    fn distances(&self, from: (i32, i32), tide: Tide) -> Vec<u64> {
        let size = self.size as usize;
        let mut dist = vec![u64::MAX; size * size];
        let index = |x: i32, y: i32| y as usize * size + x as usize;
        dist[index(from.0, from.1)] = 0;
        let mut queue = BinaryHeap::new();
        queue.push(Reverse((0u64, from.0, from.1)));
        while let Some(Reverse((d, x, y))) = queue.pop() {
            if d != dist[index(x, y)] {
                continue;
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if !self.walkable(nx, ny) {
                        continue;
                    }
                    let diagonal = dx != 0 && dy != 0;
                    if diagonal && (!self.walkable(x + dx, y) || !self.walkable(x, y + dy)) {
                        continue;
                    }
                    let cost = self.cost(nx, ny, tide);
                    let step = if diagonal { cost * 141 / 100 } else { cost };
                    let next = d + step;
                    if next < dist[index(nx, ny)] {
                        dist[index(nx, ny)] = next;
                        queue.push(Reverse((next, nx, ny)));
                    }
                }
            }
        }
        dist
    }
}

fn cells(distance: u64) -> Value {
    if distance == u64::MAX {
        Value::Null
    } else {
        json!((distance as f64 / 100.0 * 10.0).round() / 10.0)
    }
}

/// Whether two walks are within `FAIR_PERCENT` or a cell of each other.
fn close(a: u64, b: u64) -> bool {
    let (low, high) = (a.min(b), a.max(b));
    high != u64::MAX && (high - low <= FAIR_SLACK || (high - low) * 100 <= high * FAIR_PERCENT)
}

pub fn map_check(map: MapId) -> Result<Value, String> {
    let layout = map.layout();
    let factions = vec![Faction::Union; layout.seat_count()];
    let world = World::with_map(1, map, &factions)?;
    let grid = Grid {
        world: &world,
        size: i32::from(world.map.width),
    };
    let size = grid.size as usize;
    let at = |dist: &[u64], pos: Pos| {
        let (x, y) = pos.cell_xy();
        dist[y as usize * size + x as usize]
    };
    let seats: Vec<u8> = world.seats().collect();
    let dry: Vec<Vec<u64>> = seats
        .iter()
        .map(|&seat| grid.distances(layout.headquarters(seat).cell_xy(), Tide::Dry))
        .collect();
    let neutral: Vec<Vec<u64>> = seats
        .iter()
        .map(|&seat| grid.distances(layout.headquarters(seat).cell_xy(), Tide::Neutral))
        .collect();

    // The wreck beds and wells on land, each given to the seat nearest it.
    let land = |pos: Pos| {
        let (x, y) = pos.cell_xy();
        !world.map.terrain(x, y).is_tidal() && !world.map.island_cell(x, y)
    };
    let nearest = |pos: Pos| {
        seats
            .iter()
            .copied()
            .min_by_key(|&seat| (at(&dry[usize::from(seat)], pos), seat))
            .unwrap_or(0)
    };
    let mut economy: Vec<Vec<u64>> = vec![Vec::new(); seats.len()];
    let sites = world
        .map
        .resources
        .iter()
        .map(|resource| resource.pos)
        .chain(world.map.wells.iter().copied())
        .filter(|&pos| land(pos));
    for pos in sites {
        let seat = usize::from(nearest(pos));
        economy[seat].push(at(&dry[seat], pos));
    }
    for walks in &mut economy {
        walks.sort_unstable();
    }
    let mut area = vec![0u64; seats.len()];
    for y in 0..grid.size {
        for x in 0..grid.size {
            let pos = Pos::cell(x, y);
            if matches!(world.map.terrain(x, y), Terrain::Salt | Terrain::Silt) && land(pos) {
                let seat = nearest(pos);
                if at(&dry[usize::from(seat)], pos) != u64::MAX {
                    area[usize::from(seat)] += 1;
                }
            }
        }
    }

    let station = layout.station_pos();
    let mut problems = Vec::new();
    let mut rows = Vec::new();
    for &seat in &seats {
        let s = usize::from(seat);
        let own_mouths: Vec<u64> = layout
            .arms_of(seat)
            .map(|arm| {
                let side = usize::from(layout.arms[arm].banks[1] == seat);
                at(&dry[s], layout.arm_mouths(arm)[side])
            })
            .collect();
        let others: Vec<u64> = seats
            .iter()
            .filter(|&&other| other != seat)
            .map(|&other| at(&dry[s], layout.headquarters(other)))
            .collect();
        rows.push(json!({
            "seat": seat,
            "headquarters": layout.seats[s].headquarters,
            "to_station_dry": cells(at(&dry[s], station)),
            "to_station_neutral": cells(at(&neutral[s], station)),
            "to_own_mouths": own_mouths.iter().map(|&d| cells(d)).collect::<Vec<_>>(),
            "to_other_headquarters": others.iter().map(|&d| cells(d)).collect::<Vec<_>>(),
            "economy": economy[s].iter().map(|&d| cells(d)).collect::<Vec<_>>(),
            "land_nearest": area[s],
        }));
        for &d in own_mouths.iter().chain(&others).chain(&economy[s]) {
            if d == u64::MAX {
                problems.push(format!("seat {seat} cannot walk to part of its map"));
            }
        }
    }
    // Every seat against seat 0, measure by measure.
    for &seat in seats.iter().skip(1) {
        let s = usize::from(seat);
        let pairs = [
            (
                "station, lanes dry",
                at(&dry[0], station),
                at(&dry[s], station),
            ),
            (
                "station, neutral tide",
                at(&neutral[0], station),
                at(&neutral[s], station),
            ),
        ];
        for (what, a, b) in pairs {
            if !close(a, b) {
                problems.push(format!("seat {seat} {what}: {} against {}", b, a));
            }
        }
        if economy[s].len() != economy[0].len() {
            problems.push(format!(
                "seat {seat} has {} wreck beds and wells, seat 0 {}",
                economy[s].len(),
                economy[0].len()
            ));
        }
        for (a, b) in economy[0].iter().zip(&economy[s]) {
            if !close(*a, *b) {
                problems.push(format!("seat {seat} economy walk {b} against {a}"));
            }
        }
        let low = area[0].min(area[s]);
        let high = area[0].max(area[s]);
        if (high - low) * 100 > high * FAIR_PERCENT {
            problems.push(format!("seat {seat} land {} against {}", area[s], area[0]));
        }
    }
    let mut hq_pairs = Vec::new();
    for (i, &a) in seats.iter().enumerate() {
        for &b in &seats[i + 1..] {
            hq_pairs.push(at(&dry[usize::from(a)], layout.headquarters(b)));
        }
    }
    for pair in hq_pairs.windows(2) {
        if !close(pair[0], pair[1]) {
            problems.push(format!(
                "headquarters apart: {} against {}",
                pair[1], pair[0]
            ));
        }
    }
    let report = json!({
        "map": layout.name,
        "metric": "octile walk in cells: 8 neighbours, diagonal 1.41, no corner cutting, silt 1.05, wading 1.33",
        "fair_percent": FAIR_PERCENT,
        "fair_slack_cells": FAIR_SLACK / 100,
        "seats": rows,
        "problems": problems,
    });
    if problems.is_empty() {
        Ok(report)
    } else {
        Err(serde_json::to_string_pretty(&report).unwrap_or_default())
    }
}

pub fn parse_map(name: &str) -> Result<MapId, String> {
    match name {
        "split-basin" | "split_basin" | "basin" => Ok(MapId::SplitBasin),
        "confluence" => Ok(MapId::Confluence),
        other => Err(format!(
            "unknown map {other}: use split-basin or confluence"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_maps_are_fair_to_every_seat() {
        for map in MapId::ALL {
            let report = map_check(map).unwrap_or_else(|error| panic!("{map:?}: {error}"));
            assert_eq!(
                report["seats"].as_array().map(Vec::len),
                Some(map.layout().seat_count())
            );
        }
    }

    #[test]
    fn the_confluence_keeps_the_designed_walks() {
        let report = map_check(MapId::Confluence).expect("fair");
        // docs/THIRD-FACTION-AND-CONFLUENCE.md, section 2.
        // The sketch rounds half down where this rounds half up, so allow
        // a tenth of a cell.
        let seat = |s: usize, key: &str| report["seats"][s][key].as_f64().unwrap_or(0.0);
        for (s, key, designed) in [
            (0, "to_station_dry", 75.2),
            (1, "to_station_dry", 74.2),
            (2, "to_station_dry", 74.2),
            (0, "to_station_neutral", 78.0),
            (1, "to_station_neutral", 76.8),
        ] {
            assert!((seat(s, key) - designed).abs() <= 0.11, "seat {s} {key}");
        }
        assert_eq!(report["seats"][0]["to_own_mouths"], json!([43.8, 43.8]));
        assert_eq!(
            report["seats"][1]["to_other_headquarters"],
            json!([113.6, 112.0])
        );
    }
}
