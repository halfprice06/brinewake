//! What peers say to each other, and the match they agree on.
//!
//! Messages travel as JSON over the reliable link. The host is the hub:
//! guests speak only to the host, and the host passes each guest's batches
//! and hashes on to every other guest. Nothing here assumes two seats; the
//! map decides how many sides a match has (`sides`), and every seat beyond
//! them watches.

use bw_core::Faction;
use bw_sim::{Command, MapId, World};
use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes shape.
pub const PROTOCOL: u32 = 7;

/// One seat in the lobby and in the match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatInfo {
    pub name: String,
    /// The side this seat plays, or `None` to watch.
    pub faction: Option<Faction>,
    pub ready: bool,
    /// Round trip to the host, for the lobby's signal bars.
    #[serde(default)]
    pub rtt_ms: Option<u32>,
}

/// Everything both sides need to build the same world.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchPlan {
    pub seed: u64,
    pub seats: Vec<SeatInfo>,
    #[serde(default)]
    pub map: MapId,
}

/// How many seats play on `map`; the rest watch.
pub fn sides(map: MapId) -> usize {
    map.layout().seat_count()
}

impl MatchPlan {
    /// The world player index of each seat, `None` for a watcher. Players
    /// are numbered in seat order.
    pub fn players(&self) -> Vec<Option<u8>> {
        let mut next = 0u8;
        self.seats
            .iter()
            .map(|seat| {
                seat.faction.map(|_| {
                    next += 1;
                    next - 1
                })
            })
            .collect()
    }

    /// Why this plan cannot start yet, if it cannot.
    pub fn check(&self) -> Result<(), String> {
        let held: Vec<Faction> = self.seats.iter().filter_map(|s| s.faction).collect();
        let wanted = sides(self.map);
        if held.len() < wanted {
            let missing = wanted - held.len();
            return Err(if missing == 1 {
                "Waiting for another player.".into()
            } else {
                format!("Waiting for {missing} more players.")
            });
        }
        if held.len() > wanted {
            return Err(format!(
                "This map holds {wanted} players; the rest can watch."
            ));
        }
        Ok(())
    }

    /// The world every seat builds from this plan.
    pub fn world(&self) -> Result<World, String> {
        self.check()?;
        let factions: Vec<Faction> = self.seats.iter().filter_map(|s| s.faction).collect();
        let mut world = World::with_map(self.seed, self.map, &factions)?;
        world.ai_enabled = false;
        Ok(world)
    }
}

/// Give a new seat a side while `map` has room, or make it a watcher: the
/// side fewest seats hold, so the first seats get one of each.
pub fn free_side(seats: &[SeatInfo], map: MapId) -> Option<Faction> {
    let taken: Vec<Faction> = seats.iter().filter_map(|s| s.faction).collect();
    if taken.len() >= sides(map) {
        return None;
    }
    Faction::ALL
        .into_iter()
        .min_by_key(|f| taken.iter().filter(|t| *t == f).count())
}

/// Seat `changed` picked `faction`: any faction may play any seat, mirrors
/// included, while the map has room for another player.
pub fn settle_sides(seats: &mut [SeatInfo], changed: usize, faction: Option<Faction>, map: MapId) {
    let before = seats[changed].faction;
    seats[changed].faction = faction;
    seats[changed].ready = false;
    if faction.is_none() {
        return;
    }
    let players = seats.iter().filter(|s| s.faction.is_some()).count();
    if players > sides(map) {
        // A watcher taking a side when every side is held: refuse it.
        seats[changed].faction = before;
    }
}

/// The host changed the map: fit the seats to it. Players beyond its sides
/// watch, the latest to join first; with room to spare, watchers who joined
/// as such stay watching. Anyone moved is un-readied.
pub fn fit_sides(seats: &mut [SeatInfo], map: MapId) {
    let mut kept = 0;
    for seat in seats.iter_mut() {
        if seat.faction.is_some() {
            kept += 1;
            if kept > sides(map) {
                seat.faction = None;
                seat.ready = false;
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Wire {
    /// A guest introduces itself. `rejoin` is the token from its last
    /// `Welcome` when it comes back to a match in progress; without one, a
    /// match in progress gives it back the seat of the player gone by `name`.
    Hello {
        protocol: u32,
        rules_digest: String,
        build: String,
        name: String,
        #[serde(default)]
        rejoin: Option<u64>,
    },
    /// The host will not have this guest, and says why.
    Refuse { reason: String },
    /// The host took this guest in: its seat and its token for rejoining.
    Welcome { seat: u8, token: u64 },
    /// The lobby as it stands, sent to each guest with its own seat.
    Lobby {
        seats: Vec<SeatInfo>,
        seed: u64,
        you: u8,
        #[serde(default)]
        map: MapId,
    },
    /// A guest's choices in the lobby.
    Choose {
        faction: Option<Faction>,
        ready: bool,
    },
    /// The match begins `start_in_ms` after this is sent.
    Start {
        plan: MatchPlan,
        start_in_ms: u32,
        delay: u64,
    },
    /// One seat's orders for one tick. `now` is the tick the seat stood on
    /// when it sent them, which the others use to keep pace.
    Batch {
        seat: u8,
        tick: u64,
        now: u64,
        commands: Vec<Command>,
    },
    /// One seat's state hash after `tick`.
    Hash { seat: u8, tick: u64, hash: String },
    /// The host gave up on `seat`: its batches end before `from_tick`, and
    /// it surrenders at `from_tick`.
    Dropped { seat: u8, from_tick: u64 },
    /// Each seat's round trip to the host, so a guest can size its input
    /// delay for the longest path its orders travel, and how long each
    /// seat's connection has been silent, so a guest can tell a guest that
    /// is gone (the host's drop clock runs) from one that is only slow.
    ///
    /// `away_ms`: for each seat the host keeps open while its player is
    /// away (and stands in for, sending its batches), the time before it is
    /// dropped (protocol 7).
    Paths {
        rtt_ms: Vec<Option<u32>>,
        #[serde(default)]
        quiet_ms: Vec<Option<u32>>,
        #[serde(default)]
        away_ms: Vec<Option<u32>>,
    },
    /// A rejoining guest's way back in: the world as the host has it and
    /// every batch still to be applied.
    Resume {
        seat: u8,
        plan: MatchPlan,
        world: Box<World>,
        batches: Vec<(u64, u8, Vec<Command>)>,
        upto: Vec<u64>,
        dropped: Vec<Option<u64>>,
        delay: u64,
    },
    /// This seat is leaving.
    Bye { seat: u8 },
}

impl Wire {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn decode(bytes: &[u8]) -> Result<Wire, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("bad message: {e}"))
    }
}

/// A short name for this seat: the computer's user name, or "PLAYER".
pub fn local_name() -> String {
    let raw = std::env::var("BRINEWAKE_NAME")
        .or_else(|_| std::env::var("USER"))
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    clean_name(&raw)
}

pub fn clean_name(raw: &str) -> String {
    let name: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .map(|c| c.to_ascii_uppercase())
        .take(10)
        .collect();
    let name = name.trim().to_string();
    if name.is_empty() || name == "ROOT" {
        "PLAYER".into()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(name: &str, faction: Option<Faction>) -> SeatInfo {
        SeatInfo {
            name: name.into(),
            faction,
            ready: true,
            rtt_ms: None,
        }
    }

    #[test]
    fn a_third_seat_watches_and_players_are_numbered_in_seat_order() {
        let mut seats = vec![seat("A", Some(Faction::Assembly))];
        seats.push(seat("B", free_side(&seats, MapId::SplitBasin)));
        seats.push(seat("C", free_side(&seats, MapId::SplitBasin)));
        assert_eq!(seats[1].faction, Some(Faction::Union));
        assert_eq!(seats[2].faction, None, "the map is full, so C watches");
        let plan = MatchPlan {
            seed: 9,
            seats,
            map: MapId::SplitBasin,
        };
        assert_eq!(plan.players(), vec![Some(0), Some(1), None]);
        plan.check().unwrap();
        let world = plan.world().unwrap();
        assert_eq!(world.players[0].faction, Faction::Assembly);
        assert_eq!(world.players[1].faction, Faction::Union);
        assert!(!world.ai_enabled);
    }

    #[test]
    fn any_side_may_be_picked_and_a_watcher_waits_for_room() {
        let mut seats = vec![
            seat("A", Some(Faction::Union)),
            seat("B", Some(Faction::Assembly)),
            seat("C", None),
        ];
        // A mirror on the Split Basin: nobody else moves.
        settle_sides(&mut seats, 1, Some(Faction::Union), MapId::SplitBasin);
        assert_eq!(seats[0].faction, Some(Faction::Union));
        assert_eq!(seats[1].faction, Some(Faction::Union));
        assert!(
            seats[0].ready && !seats[1].ready,
            "only the picker unreadies"
        );
        assert!(seats[2].ready, "a watcher is not disturbed");
        // A watcher cannot take a side while both are held.
        settle_sides(&mut seats, 2, Some(Faction::Union), MapId::SplitBasin);
        assert_eq!(seats[2].faction, None);
        assert_eq!(seats[1].faction, Some(Faction::Union));
        // A player stepping back to watch frees a side for the watcher.
        settle_sides(&mut seats, 0, None, MapId::SplitBasin);
        settle_sides(&mut seats, 2, Some(Faction::Assembly), MapId::SplitBasin);
        assert_eq!(seats[2].faction, Some(Faction::Assembly));
    }

    #[test]
    fn a_plan_that_cannot_start_says_why() {
        let lonely = MatchPlan {
            seed: 1,
            seats: vec![seat("A", Some(Faction::Union)), seat("B", None)],
            map: MapId::SplitBasin,
        };
        assert!(lonely.check().unwrap_err().contains("another player"));
        // Mirrors play, the Compact included.
        let mirror = MatchPlan {
            seed: 1,
            seats: vec![
                seat("A", Some(Faction::Compact)),
                seat("B", Some(Faction::Compact)),
            ],
            map: MapId::SplitBasin,
        };
        let world = mirror.world().unwrap();
        assert_eq!(world.players[1].faction, Faction::Compact);
    }

    #[test]
    fn the_confluence_seats_three_and_allows_mirrors() {
        let map = MapId::Confluence;
        let mut seats = vec![seat("A", Some(Faction::Union))];
        for name in ["B", "C", "D"] {
            let side = free_side(&seats, map);
            seats.push(seat(name, side));
        }
        assert_eq!(seats[1].faction, Some(Faction::Assembly));
        assert!(seats[2].faction.is_some(), "the Confluence holds three");
        assert_eq!(seats[3].faction, None, "the fourth watches");
        // Picking a held side moves nobody; a watcher cannot take a fourth.
        settle_sides(&mut seats, 1, Some(Faction::Union), map);
        assert_eq!(seats[0].faction, Some(Faction::Union));
        assert_eq!(seats[1].faction, Some(Faction::Union));
        settle_sides(&mut seats, 3, Some(Faction::Union), map);
        assert_eq!(seats[3].faction, None);
        let plan = MatchPlan {
            seed: 5,
            seats: seats.clone(),
            map,
        };
        plan.check().unwrap();
        assert_eq!(plan.players(), vec![Some(0), Some(1), Some(2), None]);
        let world = plan.world().unwrap();
        assert_eq!(world.seat_count(), 3);
        assert_eq!(world.map.id, MapId::Confluence);
        assert!(!world.ai_enabled);
        let short = MatchPlan {
            seed: 5,
            seats: seats[..2].to_vec(),
            map,
        };
        assert!(short.check().unwrap_err().contains("another player"));
    }

    #[test]
    fn switching_to_the_split_basin_fits_the_seats() {
        let mut seats = vec![
            seat("A", Some(Faction::Union)),
            seat("B", Some(Faction::Union)),
            seat("C", Some(Faction::Assembly)),
            seat("D", None),
        ];
        fit_sides(&mut seats, MapId::SplitBasin);
        assert_eq!(seats[0].faction, Some(Faction::Union));
        assert!(seats[0].ready, "the first player keeps its side and ready");
        assert_eq!(seats[1].faction, Some(Faction::Union), "a mirror stands");
        assert!(seats[1].ready);
        assert_eq!(seats[2].faction, None, "the latest player now watches");
        assert!(!seats[2].ready);
        assert_eq!(seats[3].faction, None);
        let plan = MatchPlan {
            seed: 1,
            seats: seats.clone(),
            map: MapId::SplitBasin,
        };
        plan.check().unwrap();
        // And back: nobody is pulled out of watching.
        fit_sides(&mut seats, MapId::Confluence);
        assert_eq!(seats.iter().filter(|s| s.faction.is_some()).count(), 2);
    }

    #[test]
    fn an_older_plan_or_lobby_reads_as_the_split_basin() {
        let plan: MatchPlan = serde_json::from_str(r#"{"seed":3,"seats":[]}"#).unwrap();
        assert_eq!(plan.map, MapId::SplitBasin);
    }

    #[test]
    fn names_are_short_plain_and_never_empty() {
        assert_eq!(clean_name("marin"), "MARIN");
        assert_eq!(clean_name("a.very-long_user.name"), "AVERYLONGU");
        assert_eq!(clean_name("  "), "PLAYER");
        assert_eq!(clean_name("root"), "PLAYER");
    }
}
