//! Tide cues: the sluice changing hands, a switch cancelled, and a hold count
//! starting, tolling and breaking (trial 8, item 36).
//!
//! Everything here is read from the simulation's events and its public tide
//! state. The station and the crossing mouths are lit for both sides, so a
//! cue never tells the player anything the fog hides. The tracker keeps the
//! last few cues as text so a muted seat (an agent, or a player with the
//! sound off) can still read what it would have heard.
use std::collections::VecDeque;

use bw_core::TICK_HZ;
use bw_sim::{EventKind, World};

use crate::audio::Cue;

/// A count must hold (or stay broken) this long before its cue plays, so a
/// machine stepping in and out of a mouth ring does not stutter.
const CONFIRM_TICKS: u32 = TICK_HZ as u32 / 2;
/// How long a cue stays in the readable list.
const RECENT_TICKS: u64 = 10 * TICK_HZ;
const MAX_RECENT: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    pub tick: u64,
    pub cue: Cue,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct TideCues {
    /// Whether each side's count is running, after debouncing.
    counting: Vec<bool>,
    /// Ticks the raw state has disagreed with `counting`.
    pending: Vec<u32>,
    /// Whole seconds left on each side's count at the last tick.
    seconds_left: Vec<u32>,
    /// The station's owner at the last tick, so a capture can tell the
    /// seat that lost it from the seats that only watched it change hands.
    owner: Option<u8>,
    recent: VecDeque<Heard>,
}

/// The arms a tide leaves deep when `arm` dries, in words: "the south" on
/// the Split Basin, "the east and south" on the Confluence.
pub(crate) fn deep_words(world: &World, arm: usize) -> String {
    let others: Vec<&str> = (0..crate::seats::arm_count(world))
        .filter(|&other| other != arm)
        .map(|other| crate::seats::arm_word(world, other))
        .collect();
    match others.as_slice() {
        [] => "no other arm".into(),
        [one] => format!("the {one}"),
        [rest @ .., last] => format!("the {} and {last}", rest.join(", ")),
    }
}

/// The prompt for a tide switch: which arm dries and which go deep, for
/// two arms or three (trial 10: DRY W on the Confluence read "the south
/// dries ...; the other side goes deep").
pub(crate) fn set_tide_sentence(world: &World, arm: bw_sim::Arm, pressure: u32) -> String {
    let dry = crate::seats::arm_word(world, arm.index());
    let deep = deep_words(world, arm.index());
    let verb = if crate::seats::arm_count(world) > 2 {
        "go"
    } else {
        "goes"
    };
    format!("The {dry} dries in ten seconds for {pressure} pressure; {deep} {verb} deep.")
}

impl TideCues {
    /// Reads one authoritative tick and returns the cues it produced, from
    /// the local player's (player 0's) side.  A tide switch and a flood are
    /// heard as lines only: their bell and rush already play from the
    /// simulation's events.
    pub fn observe(&mut self, world: &World) -> Vec<Cue> {
        let mut heard = Vec::new();
        let mut lines: Vec<(Cue, String)> = Vec::new();
        // An opponent by name: "enemy" with one, "red" or "violet" with two.
        let rival = |player: Option<u8>| {
            crate::seats::seat_name(world, player.unwrap_or(1).max(1)).to_lowercase()
        };
        for event in &world.events {
            match event.kind {
                EventKind::GateCaptureStarted if event.player.is_some_and(|p| p != 0) => {
                    heard.push((
                        Cue::SluiceCaptureAlarm,
                        format!("{} capturing the sluice", rival(event.player)),
                    ));
                }
                EventKind::GateCaptured if event.player == Some(0) => {
                    heard.push((Cue::SluiceTaken, "sluice taken".into()));
                }
                // Only the seat that owned the station lost it; the others
                // hear who took it (trial 10).
                EventKind::GateCaptured if self.owner == Some(0) => {
                    heard.push((
                        Cue::SluiceLost,
                        format!("sluice lost to {}", rival(event.player)),
                    ));
                }
                EventKind::GateCaptured if event.player.is_some_and(|p| p != 0) => {
                    lines.push((
                        Cue::SluiceLost,
                        format!("{} took the sluice", rival(event.player)),
                    ));
                }
                EventKind::GateWarning => {
                    let whose = match event.player {
                        Some(0) => "your".to_string(),
                        Some(_) => rival(event.player),
                        None => "the".to_string(),
                    };
                    let text = if world.gate.flood_pending {
                        format!("{whose} flood: every crossing deep in 5s")
                    } else if world.gate.ebb_pending {
                        // Rules 22: a DRY on the Confluence ebbs on its own.
                        format!(
                            "the {} ebbs: every crossing shallow in 10s",
                            crate::seats::arm_word(world, world.gate.dry_arm.index())
                        )
                    } else {
                        let arm = world.gate.switch_target.unwrap_or_default().index();
                        format!(
                            "{whose} switch: the {} dries in 10s",
                            crate::seats::arm_word(world, arm)
                        )
                    };
                    lines.push((Cue::GateWarningBell, text));
                }
                EventKind::GateChanged => {
                    let text = match world.gate.tide {
                        bw_sim::Tide::Flood => format!(
                            "flood: every crossing deep for {}s",
                            bw_content::FLOOD_TICKS / TICK_HZ as u32
                        ),
                        bw_sim::Tide::Open => {
                            let arm = world.gate.dry_arm.index();
                            format!(
                                "tide: the {} dry, {} deep",
                                crate::seats::arm_word(world, arm),
                                deep_words(world, arm)
                            )
                        }
                        bw_sim::Tide::Neutral => "tide: every crossing shallow".into(),
                    };
                    lines.push((Cue::GateChangeRush, text));
                }
                EventKind::SwitchCancelled => {
                    let whose = if event.player == Some(0) {
                        "your".to_string()
                    } else {
                        rival(event.player)
                    };
                    heard.push((Cue::SwitchCancelled, format!("{whose} switch cancelled")));
                }
                _ => {}
            }
        }
        let seats = world.players.len();
        self.counting.resize(seats, false);
        self.pending.resize(seats, 0);
        self.seconds_left.resize(seats, 0);
        for player in world.seats() {
            let slot = player as usize;
            let held = world.outcome.is_none() && world.holds_every_lane(player);
            let left = world
                .hold_ticks()
                .saturating_sub(world.lane_hold[slot])
                .div_ceil(TICK_HZ as u32);
            // "enemy" with one opponent, "red" or "violet" with two.
            let who = crate::seats::seat_name(world, player).to_lowercase();
            if held != self.counting[slot] {
                self.pending[slot] += 1;
                if self.pending[slot] >= CONFIRM_TICKS {
                    self.counting[slot] = held;
                    self.pending[slot] = 0;
                    // A count that kept part of its run resumes, and says so
                    // (trial 11: "90s to win" after a break read as a reset).
                    let resumed = world.lane_hold[slot] > CONFIRM_TICKS + TICK_HZ as u32;
                    let begun = |own: bool, out: bool| {
                        crate::trial11_words::hold_begun_words(&who, own, left, resumed, out)
                    };
                    heard.push(match (player, held) {
                        (0, true) => (Cue::HoldBegunOurs, begun(true, false)),
                        (0, false) => (Cue::HoldBrokenOurs, "your hold is broken".into()),
                        // A seat already out has nothing left to lose.
                        (_, true) if world.is_eliminated(0) => {
                            (Cue::HoldBegunEnemy, begun(false, true))
                        }
                        (_, true) => (Cue::HoldBegunEnemy, begun(false, false)),
                        (_, false) => (Cue::HoldBrokenEnemy, format!("{who} hold broken")),
                    });
                }
            } else {
                self.pending[slot] = 0;
                // The count tolls every ten seconds, then every second at the
                // end: the last ten for the enemy's, the last five for yours.
                let crossed = left < self.seconds_left[slot] && left > 0;
                let last = if player == 0 { 5 } else { 10 };
                if self.counting[slot] && crossed && (left.is_multiple_of(10) || left <= last) {
                    heard.push(if player == 0 {
                        (Cue::HoldTollOurs, format!("your hold: {left}s"))
                    } else {
                        (Cue::HoldTollEnemy, format!("{who} hold: {left}s"))
                    });
                }
            }
            self.seconds_left[slot] = left;
        }
        self.owner = world.gate.owner;
        for (cue, text) in heard.iter().chain(&lines) {
            self.recent.push_back(Heard {
                tick: world.tick,
                cue: *cue,
                text: text.clone(),
            });
        }
        while self.recent.len() > MAX_RECENT
            || self
                .recent
                .front()
                .is_some_and(|h| h.tick.saturating_add(RECENT_TICKS) < world.tick)
        {
            self.recent.pop_front();
        }
        heard.into_iter().map(|(cue, _)| cue).collect()
    }

    /// The cues of the last ten seconds, oldest first.
    pub fn recent(&self) -> impl Iterator<Item = &Heard> {
        self.recent.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_content::TIDE_HOLD_TICKS;
    use bw_core::{Faction, Kind};
    use bw_sim::Event;

    fn event(kind: EventKind, player: u8) -> Event {
        Event {
            tick: 1,
            kind,
            player: Some(player),
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        }
    }

    fn quiet_world() -> World {
        let mut world = World::new(17, Faction::Union);
        world.ai_enabled = false;
        world
    }

    fn step(world: &mut World, cues: &mut TideCues) -> Vec<Cue> {
        world.step();
        cues.observe(world)
    }

    /// Puts a gun machine of `owner` at every crossing mouth and gives them
    /// the sluice, which is a full hold under rules 12.
    fn stage_hold(world: &mut World, owner: u8) {
        world.gate.owner = Some(owner);
        for mouth in bw_sim::CROSSING_MOUTHS.iter().flatten() {
            world.spawn_for_tests(owner, Kind::Bulwark, *mouth);
        }
    }

    #[test]
    fn captures_and_cancelled_switches_are_heard_from_their_events() {
        let mut world = quiet_world();
        let mut cues = TideCues::default();
        // The station was ours at the last tick.
        world.gate.owner = Some(0);
        let _ = cues.observe(&world);
        world.events = vec![
            event(EventKind::GateCaptureStarted, 1),
            event(EventKind::GateCaptureStarted, 0),
            event(EventKind::GateCaptured, 0),
            event(EventKind::GateCaptured, 1),
            event(EventKind::SwitchCancelled, 1),
        ];
        assert_eq!(
            cues.observe(&world),
            vec![
                Cue::SluiceCaptureAlarm,
                Cue::SluiceTaken,
                Cue::SluiceLost,
                Cue::SwitchCancelled
            ]
        );
        let texts: Vec<_> = cues.recent().map(|h| h.text.as_str()).collect();
        assert!(texts.contains(&"enemy switch cancelled"), "{texts:?}");
        assert!(texts.contains(&"sluice lost to enemy"), "{texts:?}");
    }

    fn confluence() -> World {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("world");
        world.ai_enabled = false;
        world
    }

    #[test]
    fn only_the_owner_hears_the_sluice_lost_and_the_others_hear_who_took_it() {
        // Trial 10: every seat heard "sluice lost" when red took violet's
        // station.
        let mut world = confluence();
        let mut cues = TideCues::default();
        world.gate.owner = Some(2);
        let _ = cues.observe(&world);
        world.gate.owner = Some(1);
        world.events = vec![event(EventKind::GateCaptured, 1)];
        assert!(cues.observe(&world).is_empty(), "no loss alarm for us");
        let red = crate::seats::seat_name(&world, 1).to_lowercase();
        assert!(
            cues.recent()
                .any(|h| h.text == format!("{red} took the sluice")),
            "{:?}",
            cues.recent().collect::<Vec<_>>()
        );
        assert!(!cues.recent().any(|h| h.text.contains("sluice lost")));
        // Ours, then taken: the loss is ours to hear.
        world.gate.owner = Some(0);
        world.events.clear();
        let _ = cues.observe(&world);
        world.gate.owner = Some(2);
        world.events = vec![event(EventKind::GateCaptured, 2)];
        assert_eq!(cues.observe(&world), vec![Cue::SluiceLost]);
    }

    #[test]
    fn every_switch_and_flood_is_heard_naming_the_arms() {
        let mut world = confluence();
        let mut cues = TideCues::default();
        world.gate.owner = Some(0);
        world.players[0].pressure = 500;
        // DRY W: arm 1 on the Confluence.
        world
            .issue(
                0,
                bw_sim::Command::SetTide {
                    arm: bw_sim::Arm(1),
                },
            )
            .expect("switch");
        let mut sounds = Vec::new();
        for _ in 0..TICK_HZ as usize {
            sounds.extend(step(&mut world, &mut cues));
        }
        let texts: Vec<_> = cues.recent().map(|h| h.text.clone()).collect();
        assert!(
            texts
                .iter()
                .any(|t| t == "your switch: the west dries in 10s"),
            "{texts:?}"
        );
        for _ in 0..(TICK_HZ as usize * 10) {
            sounds.extend(step(&mut world, &mut cues));
        }
        let texts: Vec<_> = cues.recent().map(|h| h.text.clone()).collect();
        assert!(
            texts
                .iter()
                .any(|t| t == "tide: the west dry, the east and south deep"),
            "{texts:?}"
        );
        // The bell and the rush play from the events, not twice from here.
        assert!(sounds.is_empty(), "{sounds:?}");
        // A flood, once the station unlocks.
        world.gate.locked_until = world.tick;
        world.issue(0, bw_sim::Command::Flood).expect("flood");
        for _ in 0..(TICK_HZ as usize * 6) {
            step(&mut world, &mut cues);
        }
        let texts: Vec<_> = cues.recent().map(|h| h.text.clone()).collect();
        assert!(
            texts
                .iter()
                .any(|t| t == "your flood: every crossing deep in 5s"),
            "{texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("flood: every crossing deep for")),
            "{texts:?}"
        );
    }

    #[test]
    fn the_switch_prompt_names_the_arm_that_dries_and_the_ones_that_go_deep() {
        let world = confluence();
        assert_eq!(
            set_tide_sentence(&world, bw_sim::Arm(1), 40),
            "The west dries in ten seconds for 40 pressure; the east and south go deep."
        );
        assert_eq!(
            set_tide_sentence(&world, bw_sim::Arm(0), 40),
            "The east dries in ten seconds for 40 pressure; the west and south go deep."
        );
        let basin = World::new(1, Faction::Union);
        assert_eq!(
            set_tide_sentence(&basin, bw_sim::Arm::NORTH, 40),
            "The north dries in ten seconds for 40 pressure; the south goes deep."
        );
    }

    #[test]
    fn an_enemy_hold_starts_tolls_down_and_breaks_once() {
        let mut world = quiet_world();
        let mut cues = TideCues::default();
        stage_hold(&mut world, 1);
        let mut heard = Vec::new();
        for _ in 0..(TICK_HZ as usize * 25) {
            heard.extend(step(&mut world, &mut cues));
        }
        assert_eq!(
            heard.iter().filter(|c| **c == Cue::HoldBegunEnemy).count(),
            1,
            "{heard:?}"
        );
        // 90 s count, 25 s in: tolls at 80 and 70 left.
        assert_eq!(
            heard.iter().filter(|c| **c == Cue::HoldTollEnemy).count(),
            2
        );
        // A brief blip at the mouth does not break it.
        world.gate.owner = None;
        heard.clear();
        for _ in 0..(CONFIRM_TICKS - 2) {
            heard.extend(step(&mut world, &mut cues));
        }
        world.gate.owner = Some(1);
        for _ in 0..TICK_HZ as usize {
            heard.extend(step(&mut world, &mut cues));
        }
        assert!(heard.is_empty(), "{heard:?}");
        // Losing the sluice breaks the count, once.
        world.gate.owner = Some(0);
        for _ in 0..(TICK_HZ as usize * 3) {
            heard.extend(step(&mut world, &mut cues));
        }
        assert_eq!(heard, vec![Cue::HoldBrokenEnemy]);
    }

    #[test]
    fn the_last_seconds_of_a_count_toll_every_second() {
        let mut world = quiet_world();
        let mut cues = TideCues::default();
        stage_hold(&mut world, 1);
        world.lane_hold[1] = TIDE_HOLD_TICKS - 12 * TICK_HZ as u32;
        let mut heard = Vec::new();
        while world.outcome.is_none() && world.tick < 20 * TICK_HZ {
            heard.extend(step(&mut world, &mut cues));
        }
        assert!(world.outcome.is_some(), "the enemy hold wins");
        let tolls = heard.iter().filter(|c| **c == Cue::HoldTollEnemy).count();
        // Begun at about 11.5 s left, then 10, 9, ... 1.
        assert_eq!(tolls, 10, "{heard:?}");
        let text: Vec<_> = cues.recent().map(|h| h.text.clone()).collect();
        assert!(text.iter().any(|t| t == "enemy hold: 1s"), "{text:?}");
    }

    #[test]
    fn our_own_hold_is_heard_with_its_own_cues() {
        let mut world = quiet_world();
        let mut cues = TideCues::default();
        stage_hold(&mut world, 0);
        let mut heard = Vec::new();
        for _ in 0..(TICK_HZ as usize * 2) {
            heard.extend(step(&mut world, &mut cues));
        }
        assert!(
            cues.recent()
                .any(|h| h.text == "you hold both lanes: 90s to win")
        );
        for _ in 0..(TICK_HZ as usize * 9) {
            heard.extend(step(&mut world, &mut cues));
        }
        assert_eq!(heard, vec![Cue::HoldBegunOurs, Cue::HoldTollOurs]);
        // The readable list forgets what is more than ten seconds old.
        let texts: Vec<_> = cues.recent().map(|h| h.text.as_str()).collect();
        assert_eq!(texts, vec!["your hold: 80s"]);
    }

    #[test]
    fn a_cue_never_changes_the_world() {
        let mut world = quiet_world();
        stage_hold(&mut world, 1);
        let hash = world.state_hash();
        let mut cues = TideCues::default();
        for _ in 0..(TICK_HZ as usize * 2) {
            let _ = cues.observe(&world);
        }
        assert_eq!(world.state_hash(), hash);
        // An enemy's accepted command is not a tide event.
        world.events = vec![event(EventKind::CommandAccepted, 1)];
        cues.pending = vec![0, 0];
        cues.counting = vec![false, true];
        assert!(cues.observe(&world).is_empty());
    }

    #[test]
    fn a_confluence_hold_of_its_two_lanes_is_heard_and_named() {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("world");
        world.ai_enabled = false;
        // Violet (the Compact, in its own glaze) holds the two lanes beside its land: the west and the south.
        world.gate.owner = Some(2);
        for arm in world.arms_of(2) {
            let mouth = world.own_mouth(2, arm).expect("mouth");
            world.spawn_for_tests(2, Kind::Bulwark, mouth);
        }
        let mut cues = TideCues::default();
        let mut heard = Vec::new();
        for _ in 0..(TICK_HZ as usize * 2) {
            heard.extend(step(&mut world, &mut cues));
        }
        assert_eq!(heard, vec![Cue::HoldBegunEnemy]);
        assert!(
            cues.recent()
                .any(|h| h.text.starts_with("violet holds both lanes")),
            "{:?}",
            cues.recent().collect::<Vec<_>>()
        );
    }
}
