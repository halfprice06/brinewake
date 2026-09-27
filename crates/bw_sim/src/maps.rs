//! The maps: the ground, the seats, the arms of tidal water and the
//! practice AI's building sites of each.
//!
//! A map is named by its `MapId`; everything else about it lives here as
//! fixed data, so a saved world carries only its tiles and wrecks as it
//! always has. The Split Basin's data is the layout the game shipped with
//! before there was a second map, unchanged, so two-seat saves, replays and
//! state hashes stay exactly as they were.

use bw_content::{DEEP_WRECK, HOME_WRECK, LANE_WRECK, SIDE_WRECK};
use bw_core::{Pos, Terrain};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Which map a match is played on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MapId {
    /// Two seats, west and east of a lake with a north and a south lane.
    #[default]
    SplitBasin,
    /// Three seats around a Y of water with three arms.
    Confluence,
}

impl MapId {
    pub const ALL: [MapId; 2] = [MapId::SplitBasin, MapId::Confluence];

    pub fn is_split_basin(&self) -> bool {
        *self == MapId::SplitBasin
    }

    pub fn layout(self) -> &'static Layout {
        match self {
            MapId::SplitBasin => &SPLIT_BASIN,
            MapId::Confluence => &CONFLUENCE,
        }
    }
}

/// A map's fixed geography.
#[derive(Debug)]
pub struct Layout {
    pub id: MapId,
    pub name: &'static str,
    /// The grid is square, this many cells a side.
    pub size: u16,
    /// The station (the sluice), where the tide is set.
    pub station: (i32, i32),
    pub seats: &'static [SeatLayout],
    pub arms: &'static [ArmLayout],
}

/// Where a seat starts.
#[derive(Debug)]
pub struct SeatLayout {
    pub headquarters: (i32, i32),
    /// The six starting workers, outside the headquarters' footprint.
    pub workers: [(i32, i32); 6],
    /// The facing a seat's buildings are raised with and a line turns to.
    pub facing: u8,
    pub ai: AiSites,
}

/// Where the practice AI builds when it plays a seat.
#[derive(Debug)]
pub struct AiSites {
    pub works: (i32, i32),
    pub dropoff: (i32, i32),
    /// On the seat's home well.
    pub condenser: (i32, i32),
    /// Tried in order until one takes the Drydock and can launch a hull.
    pub drydocks: [(i32, i32); 4],
    /// Where its scout looks for the enemy, in turn.
    pub explore: &'static [(i32, i32)],
    /// Toward the enemy, as a direction: the machine furthest along it is
    /// the front, where its menders wait.
    pub forward: (i32, i32),
}

/// An arm of tidal water: its crossing lane, the two mouths on its banks,
/// and the wrecks beside them.
#[derive(Debug)]
pub struct ArmLayout {
    /// How the arm is named on the tide: "north", "south", or "E", "W", "S".
    pub name: &'static str,
    /// The event text when the tide opens this arm.
    pub opened: &'static str,
    /// The bank mouths of the lane; the tide victory holds a lane at either.
    pub mouths: [(i32, i32); 2],
    /// The seat whose bank each mouth stands on.
    pub banks: [u8; 2],
    /// The middle of the lane, where the practice AI reads the water.
    pub centre: (i32, i32),
    /// The two lane wrecks, one beside each mouth inside the lane.
    pub wrecks: [(i32, i32); 2],
}

impl Layout {
    pub fn seat_count(&self) -> usize {
        self.seats.len()
    }

    pub fn arm_count(&self) -> usize {
        self.arms.len()
    }

    pub fn station_pos(&self) -> Pos {
        Pos::cell(self.station.0, self.station.1)
    }

    /// Each arm's two mouths, in arm order.
    pub fn mouths(&self) -> impl Iterator<Item = [Pos; 2]> + '_ {
        self.arms.iter().map(|arm| {
            [
                Pos::cell(arm.mouths[0].0, arm.mouths[0].1),
                Pos::cell(arm.mouths[1].0, arm.mouths[1].1),
            ]
        })
    }

    /// The mouths of one arm.
    pub fn arm_mouths(&self, arm: usize) -> [Pos; 2] {
        let arm = &self.arms[arm.min(self.arms.len() - 1)];
        [
            Pos::cell(arm.mouths[0].0, arm.mouths[0].1),
            Pos::cell(arm.mouths[1].0, arm.mouths[1].1),
        ]
    }

    /// The arms whose lane touches a seat's land: the ones it must hold for
    /// the tide victory. On the Split Basin that is both arms for either seat.
    pub fn arms_of(&self, seat: u8) -> impl Iterator<Item = usize> + '_ {
        self.arms
            .iter()
            .enumerate()
            .filter(move |(_, arm)| arm.banks.contains(&seat))
            .map(|(index, _)| index)
    }

    /// The arm whose lane joins two seats' banks, if one does. On the
    /// Split Basin both arms join the two seats; this gives the first.
    pub fn arm_between(&self, a: u8, b: u8) -> Option<usize> {
        self.arms
            .iter()
            .position(|arm| arm.banks.contains(&a) && arm.banks.contains(&b) && a != b)
    }

    /// Whether a cell is a lane wreck's.
    pub fn is_lane_wreck(&self, cell: (i32, i32)) -> bool {
        self.arms.iter().any(|arm| arm.wrecks.contains(&cell))
    }

    pub fn headquarters(&self, seat: u8) -> Pos {
        let (x, y) = self.seats[usize::from(seat) % self.seats.len()].headquarters;
        Pos::cell(x, y)
    }

    /// The facing of a seat's buildings and lines.
    pub fn facing(&self, seat: u8) -> u8 {
        self.seats
            .get(usize::from(seat))
            .map_or(0, |layout| layout.facing)
    }

    /// Whether a cell is the station's island: salt in the water around the
    /// station, where nothing may be built.
    pub fn island_cell(&self, x: i32, y: i32) -> bool {
        match self.id {
            MapId::SplitBasin => (62..=66).contains(&x) && (52..=76).contains(&y),
            MapId::Confluence => confluence_grid().island(x, y),
        }
    }
}

// The Split Basin: two seats on the west and east banks of a lake, a north
// and a south lane across it, and the station on an island in the middle.
static SPLIT_BASIN: Layout = Layout {
    id: MapId::SplitBasin,
    name: "THE SPLIT BASIN",
    size: 128,
    station: (64, 64),
    seats: &[
        SeatLayout {
            headquarters: (10, 64),
            workers: [(15, 61), (16, 61), (17, 61), (15, 67), (16, 67), (17, 67)],
            facing: 0,
            // Never used: the practice AI has only played the east seat.
            // Mirrors of the east seat's sites, for when it plays either.
            ai: AiSites {
                works: (22, 56),
                dropoff: (23, 64),
                condenser: (19, 69),
                drydocks: [(7, 58), (7, 69), (15, 48), (15, 76)],
                explore: &[(87, 64), (87, 45), (87, 83), (79, 120), (63, 121)],
                forward: (1, 0),
            },
        },
        SeatLayout {
            headquarters: (117, 64),
            workers: [
                (112, 61),
                (111, 61),
                (110, 61),
                (112, 67),
                (111, 67),
                (110, 67),
            ],
            facing: 4,
            ai: AiSites {
                works: (105, 56),
                dropoff: (104, 64),
                condenser: (108, 69),
                drydocks: [(120, 58), (120, 69), (112, 48), (112, 76)],
                explore: &[(40, 64), (40, 45), (40, 83), (48, 120), (64, 121)],
                forward: (-1, 0),
            },
        },
    ],
    arms: &[
        ArmLayout {
            name: "north",
            opened: "north open",
            mouths: [(50, 49), (77, 49)],
            banks: [0, 1],
            centre: (64, 49),
            wrecks: [(54, 49), (73, 49)],
        },
        ArmLayout {
            name: "south",
            opened: "south open",
            mouths: [(50, 79), (77, 79)],
            banks: [0, 1],
            centre: (64, 79),
            wrecks: [(54, 79), (73, 79)],
        },
    ],
};

// The Confluence: three seats around a Y of water. The grid, the cells and
// the distances come from tools/maps/confluence_sketch.py; see
// docs/THIRD-FACTION-AND-CONFLUENCE.md, section 2. Seat 0 (T) sits on the
// grid's diagonal, and seats 1 (R) and 2 (L) are mirror images across it.
// Arm 0 (E) joins T and R, arm 1 (W) T and L, and arm 2 (S) R and L.
static CONFLUENCE: Layout = Layout {
    id: MapId::Confluence,
    name: "THE CONFLUENCE",
    size: 176,
    station: (88, 88),
    seats: &[
        SeatLayout {
            headquarters: (45, 45),
            workers: [(51, 46), (51, 47), (52, 48), (46, 51), (47, 51), (48, 52)],
            facing: 1,
            ai: AiSites {
                works: (48, 59),
                dropoff: (54, 54),
                condenser: (48, 55),
                drydocks: [(35, 35), (30, 43), (43, 30), (30, 51)],
                // R's front, L's front, the far mouths of E and W, and the
                // S lane between the other two.
                explore: &[(130, 70), (70, 130), (105, 70), (70, 105), (103, 103)],
                forward: (1, 1),
            },
        },
        SeatLayout {
            headquarters: (147, 73),
            workers: [
                (143, 77),
                (142, 77),
                (141, 78),
                (141, 71),
                (140, 72),
                (139, 72),
            ],
            facing: 4,
            ai: AiSites {
                works: (133, 68),
                dropoff: (134, 76),
                condenser: (137, 70),
                drydocks: [(161, 69), (156, 61), (161, 79), (149, 57)],
                explore: &[(55, 55), (70, 130), (81, 64), (94, 111), (67, 93)],
                forward: (-4, 1),
            },
        },
        SeatLayout {
            headquarters: (73, 147),
            workers: [
                (77, 143),
                (77, 142),
                (78, 141),
                (71, 141),
                (72, 140),
                (72, 139),
            ],
            facing: 6,
            ai: AiSites {
                works: (68, 133),
                dropoff: (76, 134),
                condenser: (70, 137),
                drydocks: [(69, 161), (61, 156), (79, 161), (57, 149)],
                explore: &[(55, 55), (130, 70), (64, 81), (111, 94), (93, 67)],
                forward: (1, -4),
            },
        },
    ],
    arms: &[
        ArmLayout {
            name: "E",
            opened: "E open",
            mouths: [(81, 64), (105, 70)],
            banks: [0, 1],
            centre: (93, 67),
            wrecks: [(85, 65), (101, 69)],
        },
        ArmLayout {
            name: "W",
            opened: "W open",
            mouths: [(64, 81), (70, 105)],
            banks: [0, 2],
            centre: (67, 93),
            wrecks: [(65, 85), (69, 101)],
        },
        ArmLayout {
            name: "S",
            opened: "S open",
            mouths: [(111, 94), (94, 111)],
            banks: [1, 2],
            centre: (103, 103),
            wrecks: [(108, 97), (97, 108)],
        },
    ],
};

/// The Confluence's economy, seat by seat: home beds, side beds and wells.
const CONFLUENCE_HOME_BEDS: [[(i32, i32); 2]; 3] = [
    [(44, 54), (54, 44)],
    [(140, 67), (141, 80)],
    [(67, 140), (80, 141)],
];
const CONFLUENCE_SIDE_BEDS: [[(i32, i32); 4]; 3] = [
    [(46, 80), (51, 78), (80, 46), (78, 51)],
    [(118, 57), (117, 60), (129, 101), (124, 99)],
    [(57, 118), (60, 117), (101, 129), (99, 124)],
];
const CONFLUENCE_WELLS: [[(i32, i32); 3]; 3] = [
    [(48, 55), (53, 85), (85, 53)],
    [(137, 70), (108, 63), (120, 105)],
    [(70, 137), (63, 108), (105, 120)],
];
const CONFLUENCE_DEEP_WRECKS: [(i32, i32); 3] = [(91, 74), (74, 91), (97, 97)];

/// The Confluence's grid, one character a cell, as the sketch wrote it.
const CONFLUENCE_MAP: &str = include_str!("../maps/confluence.map");

struct ParsedGrid {
    size: usize,
    tiles: Vec<Terrain>,
    island: Vec<bool>,
}

impl ParsedGrid {
    fn island(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x as usize >= self.size || y as usize >= self.size {
            return false;
        }
        self.island[y as usize * self.size + x as usize]
    }
}

fn confluence_grid() -> &'static ParsedGrid {
    static GRID: OnceLock<ParsedGrid> = OnceLock::new();
    GRID.get_or_init(|| {
        let size = usize::from(CONFLUENCE.size);
        let mut tiles = Vec::with_capacity(size * size);
        let mut island = Vec::with_capacity(size * size);
        for line in CONFLUENCE_MAP.lines().take(size) {
            for byte in line.bytes().take(size) {
                tiles.push(match byte {
                    b'.' | b'i' => Terrain::Salt,
                    b',' => Terrain::Silt,
                    b'e' => Terrain::Lane0,
                    b'w' => Terrain::Lane1,
                    b's' => Terrain::Lane2,
                    b'E' => Terrain::Rim0,
                    b'W' => Terrain::Rim1,
                    b'S' => Terrain::Rim2,
                    b'#' => Terrain::Rock,
                    _ => Terrain::Deep,
                });
                island.push(byte == b'i');
            }
        }
        // A short file reads as sea rather than failing: the tests check the
        // grid is whole.
        tiles.resize(size * size, Terrain::Deep);
        island.resize(size * size, false);
        ParsedGrid {
            size,
            tiles,
            island,
        }
    })
}

/// A map's ground, wrecks and wells as a new match starts on it.
pub(crate) struct Ground {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<Terrain>,
    /// Wreck beds, in id order: `(x, y, salvage)`.
    pub wrecks: Vec<(i32, i32, u32)>,
    pub wells: Vec<Pos>,
    pub station: Pos,
}

pub(crate) fn ground(id: MapId) -> Ground {
    match id {
        MapId::SplitBasin => split_basin_ground(),
        MapId::Confluence => confluence_ground(),
    }
}

fn split_basin_ground() -> Ground {
    let layout = &SPLIT_BASIN;
    let width = layout.size;
    let height = layout.size;
    let mut tiles = vec![Terrain::Salt; usize::from(width) * usize::from(height)];
    for y in 0..i32::from(height) {
        for x in 0..i32::from(width) {
            let index = y as usize * width as usize + x as usize;
            if x < 3 || y < 3 || x >= i32::from(width) - 3 || y >= i32::from(height) - 3 {
                tiles[index] = Terrain::Deep;
            } else if (47..=51).contains(&y) && (52..=75).contains(&x) {
                tiles[index] = Terrain::Lane0;
            } else if (77..=81).contains(&y) && (52..=75).contains(&x) {
                tiles[index] = Terrain::Lane1;
            } else if layout.island_cell(x, y) {
                tiles[index] = Terrain::Salt;
            } else if (118..=124).contains(&y) && (52..=75).contains(&x) {
                // The south rim runs from the lake to the border sea, as
                // the north rim does: at full flood nothing crosses.
                tiles[index] = Terrain::Rim1;
            } else if (3..=9).contains(&y) && (52..=75).contains(&x) {
                tiles[index] = Terrain::Rim0;
            } else if (52..=75).contains(&x) && (10..=117).contains(&y) {
                tiles[index] = Terrain::Deep;
            } else if (x == 61 || x == 66) && (58..=70).contains(&y) {
                tiles[index] = Terrain::Rock;
            } else if x % 19 == 0 && (30..=95).contains(&y) {
                tiles[index] = Terrain::Silt;
            }
        }
    }
    // Home beds are small, the side beds hold, the lane wrecks at the
    // crossing mouths are gathered only while their lane is dry, and the
    // deep wrecks lie on the sluice island.
    let lane = |arm: usize, side: usize| {
        let (x, y) = layout.arms[arm].wrecks[side];
        (x, y, LANE_WRECK)
    };
    let wrecks = vec![
        (16, 57, HOME_WRECK),
        (16, 71, HOME_WRECK),
        (112, 57, HOME_WRECK),
        (112, 71, HOME_WRECK),
        (39, 42, SIDE_WRECK),
        (39, 86, SIDE_WRECK),
        (88, 42, SIDE_WRECK),
        (88, 86, SIDE_WRECK),
        (34, 40, SIDE_WRECK),
        (94, 40, SIDE_WRECK),
        (34, 88, SIDE_WRECK),
        (94, 88, SIDE_WRECK),
        lane(0, 0),
        lane(0, 1),
        lane(1, 0),
        lane(1, 1),
        (64, 56, DEEP_WRECK),
        (64, 72, DEEP_WRECK),
    ];
    let wells = vec![
        Pos::cell(19, 69),
        Pos::cell(108, 69),
        Pos::cell(44, 40),
        Pos::cell(84, 40),
        Pos::cell(44, 88),
        Pos::cell(84, 88),
    ];
    Ground {
        width,
        height,
        tiles,
        wrecks,
        wells,
        station: layout.station_pos(),
    }
}

fn confluence_ground() -> Ground {
    let layout = &CONFLUENCE;
    let grid = confluence_grid();
    let mut wrecks = Vec::new();
    for beds in CONFLUENCE_HOME_BEDS {
        wrecks.extend(beds.iter().map(|&(x, y)| (x, y, HOME_WRECK)));
    }
    for beds in CONFLUENCE_SIDE_BEDS {
        wrecks.extend(beds.iter().map(|&(x, y)| (x, y, SIDE_WRECK)));
    }
    for arm in layout.arms {
        wrecks.extend(arm.wrecks.iter().map(|&(x, y)| (x, y, LANE_WRECK)));
    }
    wrecks.extend(
        CONFLUENCE_DEEP_WRECKS
            .iter()
            .map(|&(x, y)| (x, y, DEEP_WRECK)),
    );
    let wells = CONFLUENCE_WELLS
        .iter()
        .flatten()
        .map(|&(x, y)| Pos::cell(x, y))
        .collect();
    Ground {
        width: layout.size,
        height: layout.size,
        tiles: grid.tiles.clone(),
        wrecks,
        wells,
        station: layout.station_pos(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(ground: &Ground, x: i32, y: i32) -> Terrain {
        ground.tiles[y as usize * usize::from(ground.width) + x as usize]
    }

    #[test]
    fn the_confluence_grid_is_whole_and_mirrored_across_the_diagonal() {
        let lines: Vec<&str> = CONFLUENCE_MAP.lines().collect();
        assert_eq!(lines.len(), 176);
        assert!(lines.iter().all(|line| line.len() == 176));
        let ground = ground(MapId::Confluence);
        let mirrored = |terrain: Terrain| match terrain {
            Terrain::Lane0 => Terrain::Lane1,
            Terrain::Lane1 => Terrain::Lane0,
            Terrain::Rim0 => Terrain::Rim1,
            Terrain::Rim1 => Terrain::Rim0,
            other => other,
        };
        for y in 0..176 {
            for x in 0..176 {
                assert_eq!(
                    tile(&ground, x, y),
                    mirrored(tile(&ground, y, x)),
                    "({x},{y})"
                );
                assert_eq!(
                    CONFLUENCE.island_cell(x, y),
                    CONFLUENCE.island_cell(y, x),
                    "island ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn every_confluence_arm_has_a_lane_between_its_mouths_and_a_rim() {
        let ground = ground(MapId::Confluence);
        for (index, arm) in CONFLUENCE.arms.iter().enumerate() {
            let arm_index = index as u8;
            let (cx, cy) = arm.centre;
            assert_eq!(
                tile(&ground, cx, cy),
                Terrain::lane(arm_index),
                "{}",
                arm.name
            );
            for &(x, y) in &arm.wrecks {
                assert_eq!(
                    tile(&ground, x, y),
                    Terrain::lane(arm_index),
                    "{} wreck",
                    arm.name
                );
            }
            for &(x, y) in &arm.mouths {
                assert!(
                    matches!(tile(&ground, x, y), Terrain::Salt | Terrain::Silt),
                    "{} mouth ({x},{y}) is on land",
                    arm.name
                );
            }
            let rim = ground
                .tiles
                .iter()
                .filter(|&&terrain| terrain == Terrain::rim(arm_index))
                .count();
            assert!(rim > 100, "{} rim has {rim} cells", arm.name);
        }
    }

    #[test]
    fn confluence_seats_start_on_their_own_land() {
        let ground = ground(MapId::Confluence);
        for (seat, layout) in CONFLUENCE.seats.iter().enumerate() {
            let cells = std::iter::once(layout.headquarters)
                .chain(layout.workers)
                .chain([layout.ai.works, layout.ai.dropoff, layout.ai.condenser])
                .chain(CONFLUENCE_HOME_BEDS[seat])
                .chain(CONFLUENCE_SIDE_BEDS[seat])
                .chain(CONFLUENCE_WELLS[seat]);
            for (x, y) in cells {
                assert!(
                    matches!(tile(&ground, x, y), Terrain::Salt | Terrain::Silt)
                        && !CONFLUENCE.island_cell(x, y),
                    "seat {seat} cell ({x},{y})"
                );
            }
            assert_eq!(layout.ai.condenser, CONFLUENCE_WELLS[seat][0]);
        }
        for &(x, y) in &CONFLUENCE_DEEP_WRECKS {
            assert!(CONFLUENCE.island_cell(x, y), "deep wreck ({x},{y})");
        }
        assert!(CONFLUENCE.island_cell(88, 88));
    }

    #[test]
    fn every_scout_point_is_ground_a_machine_can_reach_at_some_tide() {
        for map in MapId::ALL {
            let layout = map.layout();
            let ground = ground(map);
            for (seat, seat_layout) in layout.seats.iter().enumerate() {
                for &(x, y) in seat_layout.ai.explore {
                    assert!(
                        tile(&ground, x, y).walkable(),
                        "{map:?} seat {seat} ({x},{y})"
                    );
                }
            }
        }
        assert_eq!(CONFLUENCE.arm_between(0, 1), Some(0));
        assert_eq!(CONFLUENCE.arm_between(2, 0), Some(1));
        assert_eq!(CONFLUENCE.arm_between(1, 2), Some(2));
        assert_eq!(CONFLUENCE.arm_between(1, 1), None);
    }

    #[test]
    fn each_confluence_seat_borders_two_arms_and_each_arm_two_seats() {
        for seat in 0..3u8 {
            assert_eq!(CONFLUENCE.arms_of(seat).count(), 2, "seat {seat}");
        }
        for seat in 0..2u8 {
            assert_eq!(SPLIT_BASIN.arms_of(seat).count(), 2, "seat {seat}");
        }
        // Each mouth is nearer its own bank's headquarters than the other's.
        for arm in CONFLUENCE.arms {
            for side in 0..2 {
                let mouth = Pos::cell(arm.mouths[side].0, arm.mouths[side].1);
                let own = CONFLUENCE.headquarters(arm.banks[side]);
                let other = CONFLUENCE.headquarters(arm.banks[1 - side]);
                assert!(
                    mouth.distance_sq(own) < mouth.distance_sq(other),
                    "{} mouth {side}",
                    arm.name
                );
            }
        }
    }
}
