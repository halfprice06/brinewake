//! Who is who in a match of two or three seats, and what the arms are
//! called: one place for the names, colours and letters every screen uses.
//!
//! Seat 0 is always the person at this screen (a network guest's world is
//! rotated so). With one opponent the words stay what they always were
//! (YOU and ENEMY in jade and red, north and south).
//!
//! With three seats every player wears one colour on every screen: its
//! own, its opponents', the replay's and the observer's. Trial 10 named the
//! seats by their order after the viewer, so RED and VIOLET were different
//! players on each screen, and the Saltglass Compact, whose machines are
//! glazed violet, saw a VIOLET enemy that looked like itself. The colour now
//! follows the player (its seat in the match, never the viewer's rotation)
//! and, where it can, its faction: the Union red, the Assembly jade, the
//! Compact violet. A seat never wears the body colour of another seat's
//! faction, so no one sees an enemy in its own colours.

use crate::canvas::{Color, JADE, RED, VIOLET};
use bw_core::Faction;
use bw_sim::World;

/// The fourth seat colour, for a mirror match: pink is no faction's body.
pub const PINK: Color = [236, 122, 178, 255];

/// A seat's colour in a match of three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Jade,
    Red,
    Violet,
    Pink,
}

impl Hue {
    pub fn colour(self) -> Color {
        match self {
            Hue::Jade => JADE,
            Hue::Red => RED,
            Hue::Violet => VIOLET,
            Hue::Pink => PINK,
        }
    }

    /// The colour's word: "VIOLET".
    pub fn word(self) -> &'static str {
        match self {
            Hue::Jade => "JADE",
            Hue::Red => "RED",
            Hue::Violet => "VIOLET",
            Hue::Pink => "PINK",
        }
    }

    /// The colour's word as an owner: "VIOLET'S".
    pub fn owner_word(self) -> &'static str {
        match self {
            Hue::Jade => "JADE'S",
            Hue::Red => "RED'S",
            Hue::Violet => "VIOLET'S",
            Hue::Pink => "PINK'S",
        }
    }
}

/// The colour a faction's own machines are painted in, which it wears as a
/// seat when no other seat shares its faction: the Union's orange-red
/// paint, the Assembly's pale jade, the Compact's violet glaze.
pub fn faction_hue(faction: Faction) -> Hue {
    match faction {
        Faction::Union => Hue::Red,
        Faction::Assembly => Hue::Jade,
        Faction::Compact => Hue::Violet,
    }
}

/// A faction's short name: "UNION", "ASSEMBLY", "COMPACT".
pub fn faction_word(faction: Faction) -> &'static str {
    let name = faction.name();
    name.rsplit(' ').next().unwrap_or(name)
}

/// Each seat's colour, by its number in the match (not by any viewer's
/// rotation). Two seats keep jade and red. With three, a faction alone in
/// the match wears its own colour; the others take the first colour no
/// other seat uses and that is not the body colour of another seat's
/// faction.
pub fn match_hues(factions: &[Faction]) -> Vec<Hue> {
    if factions.len() <= 2 {
        return [Hue::Jade, Hue::Red][..factions.len()].to_vec();
    }
    let alone = |faction: Faction| factions.iter().filter(|&&f| f == faction).count() == 1;
    let mut hues: Vec<Option<Hue>> = factions
        .iter()
        .map(|&faction| alone(faction).then(|| faction_hue(faction)))
        .collect();
    for seat in 0..factions.len() {
        if hues[seat].is_some() {
            continue;
        }
        let free = |hue: Hue, hues: &[Option<Hue>]| !hues.contains(&Some(hue));
        let clashes = |hue: Hue| {
            factions
                .iter()
                .enumerate()
                .any(|(other, &faction)| other != seat && faction_hue(faction) == hue)
        };
        let order = [Hue::Red, Hue::Violet, Hue::Jade, Hue::Pink];
        let pick = order
            .into_iter()
            .find(|&hue| free(hue, &hues) && !clashes(hue))
            .or_else(|| order.into_iter().find(|&hue| free(hue, &hues)))
            .unwrap_or(Hue::Pink);
        hues[seat] = Some(pick);
    }
    hues.into_iter()
        .map(|hue| hue.unwrap_or(Hue::Pink))
        .collect()
}

/// Each seat's colour as a view numbers its seats: `factions` in the view's
/// order, `turn` the view's rotation (the local seat's number in the
/// match). The colours follow the players, so they turn with the seats.
pub fn view_hues(factions: &[Faction], turn: u8) -> Vec<Hue> {
    let seats = factions.len();
    if seats <= 2 {
        return match_hues(factions);
    }
    let turn = usize::from(turn) % seats;
    // The match's order: view seat v is match seat (v + turn) % seats.
    let mut in_match = factions.to_vec();
    in_match.rotate_right(turn);
    let mut hues = match_hues(&in_match);
    hues.rotate_left(turn);
    hues
}

/// A seat's colour in this world.
pub fn seat_hue(world: &World, seat: u8) -> Hue {
    if world.players.len() <= 2 {
        return if seat == 0 { Hue::Jade } else { Hue::Red };
    }
    let factions: Vec<Faction> = world.players.iter().map(|p| p.faction).collect();
    view_hues(&factions, world.view_turn)
        .get(usize::from(seat))
        .copied()
        .unwrap_or(if seat == 0 { Hue::Jade } else { Hue::Red })
}

/// The colour a seat wears on every mark: jade for you and red for the
/// enemy with two seats, the player's own colour with three.
pub fn seat_colour(world: &World, seat: u8) -> Color {
    seat_hue(world, seat).colour()
}

/// A seat colour darkened for a blinking field behind light text.
pub fn shade(colour: Color) -> Color {
    let dark = |c: u8| (u16::from(c) * 3 / 10) as u8;
    [dark(colour[0]), dark(colour[1]), dark(colour[2]), 255]
}

/// A seat's colour where no world is at hand: a map preview's seat order.
pub fn preview_colour(seat: u8) -> Color {
    match seat {
        0 => JADE,
        1 => RED,
        2 => VIOLET,
        _ => PINK,
    }
}

/// A seat's name on screen: YOU, ENEMY, or its colour with two opponents.
pub fn seat_name(world: &World, seat: u8) -> &'static str {
    match seat {
        0 => "YOU",
        _ if world.seat_count() <= 2 => "ENEMY",
        _ => seat_hue(world, seat).word(),
    }
}

/// A seat's name as an owner: YOUR, ENEMY, RED'S, VIOLET'S.
pub fn seat_owner_word(world: &World, seat: u8) -> &'static str {
    match seat {
        0 => "YOUR",
        _ if world.seat_count() <= 2 => "ENEMY",
        _ => seat_hue(world, seat).owner_word(),
    }
}

/// A player's colour and faction, the same on every screen: "VIOLET
/// COMPACT". With two seats, the faction's name: "SILT ASSEMBLY".
pub fn seat_title(world: &World, seat: u8) -> String {
    let Some(player) = world.players.get(usize::from(seat)) else {
        return seat_name(world, seat).to_string();
    };
    if world.seat_count() <= 2 {
        return player.faction.name().to_string();
    }
    format!(
        "{} {}",
        seat_hue(world, seat).word(),
        faction_word(player.faction)
    )
}

/// This world's seat for a seat's number in the match (a session's seat).
pub fn seat_of_match_seat(world: &World, match_seat: u8) -> u8 {
    let seats = world.players.len().max(1);
    let turn = usize::from(world.view_turn) % seats;
    ((usize::from(match_seat) + seats - turn) % seats) as u8
}

/// An arm's letter on chips and buttons: N and S on the Split Basin, E, W
/// and S on the Confluence.
pub fn arm_letter(world: &World, arm: usize) -> &'static str {
    match world.map.layout().arms.get(arm).map(|arm| arm.name) {
        Some("north") => "N",
        Some("south") => "S",
        Some("E") => "E",
        Some("W") => "W",
        Some("S") => "S",
        _ => "?",
    }
}

/// An arm's name in a sentence: "north", "south", "east", "west".
pub fn arm_word(world: &World, arm: usize) -> &'static str {
    match world.map.layout().arms.get(arm).map(|arm| arm.name) {
        Some("north") => "north",
        Some("south") => "south",
        Some("E") => "east",
        Some("W") => "west",
        Some("S") => "south",
        _ => "far",
    }
}

/// The tide button that dries an arm: "DRY N", "DRY E".
pub fn dry_label(world: &World, arm: usize) -> &'static str {
    match arm_letter(world, arm) {
        "N" => "DRY N",
        "S" => "DRY S",
        "E" => "DRY E",
        "W" => "DRY W",
        _ => "DRY",
    }
}

/// How many arms the tide has on this map.
pub fn arm_count(world: &World) -> usize {
    world.map.layout().arm_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;
    use bw_sim::MapId;

    fn confluence(factions: &[Faction]) -> World {
        World::with_map(1, MapId::Confluence, factions).expect("world")
    }

    #[test]
    fn two_seats_keep_their_old_words_and_three_are_named_by_colour() {
        let basin = World::new(1, Faction::Union);
        assert_eq!(seat_name(&basin, 1), "ENEMY");
        assert_eq!(seat_colour(&basin, 0), JADE);
        assert_eq!(seat_colour(&basin, 1), RED);
        assert_eq!(arm_letter(&basin, 0), "N");
        assert_eq!(dry_label(&basin, 1), "DRY S");
        // A two-seat guest's swapped view keeps YOU jade and ENEMY red.
        let guest = basin.relabeled_for(1);
        assert_eq!(seat_colour(&guest, 0), JADE);
        assert_eq!(seat_name(&guest, 1), "ENEMY");
        let confluence = confluence(&[Faction::Union, Faction::Assembly, Faction::Compact]);
        assert_eq!(seat_name(&confluence, 0), "YOU");
        assert_eq!(seat_name(&confluence, 1), "JADE");
        assert_eq!(seat_owner_word(&confluence, 2), "VIOLET'S");
        assert_eq!(seat_title(&confluence, 2), "VIOLET COMPACT");
        assert_eq!(seat_title(&confluence, 0), "RED UNION");
        assert_eq!(
            (0..3)
                .map(|arm| arm_letter(&confluence, arm))
                .collect::<Vec<_>>(),
            ["E", "W", "S"]
        );
        assert_eq!(arm_word(&confluence, 1), "west");
    }

    /// Trial 10: Union saw the Assembly as RED and the Compact as VIOLET,
    /// the Assembly saw the Compact as RED, and the Compact saw the Union
    /// as RED. Every screen now names every player alike.
    #[test]
    fn every_screen_gives_a_player_the_same_colour_and_name() {
        let factions = [Faction::Union, Faction::Assembly, Faction::Compact];
        let host = confluence(&factions);
        for local in 0..3u8 {
            let view = host.relabeled_for(local);
            for match_seat in 0..3u8 {
                let seat = seat_of_match_seat(&view, match_seat);
                assert_eq!(view.layout_seat(seat), match_seat);
                assert_eq!(
                    seat_hue(&view, seat),
                    seat_hue(&host, match_seat),
                    "seat {match_seat} seen from seat {local}"
                );
                assert_eq!(
                    seat_title(&view, seat),
                    seat_title(&host, match_seat),
                    "seat {match_seat} seen from seat {local}"
                );
            }
            // Each viewer is YOU on its own screen, in its own colour.
            assert_eq!(seat_name(&view, 0), "YOU");
            assert_eq!(seat_hue(&view, 0), seat_hue(&host, local));
        }
        assert_eq!(seat_hue(&host, 0), Hue::Red, "the Union's orange-red");
        assert_eq!(seat_hue(&host, 1), Hue::Jade, "the Assembly's jade");
        assert_eq!(seat_hue(&host, 2), Hue::Violet, "the Compact's glaze");
    }

    /// No player sees an enemy wearing its own faction's body colour, in
    /// any mix of factions, mirror matches included.
    #[test]
    fn no_enemy_wears_your_factions_colour() {
        for a in Faction::ALL {
            for b in Faction::ALL {
                for c in Faction::ALL {
                    let factions = [a, b, c];
                    let hues = match_hues(&factions);
                    for (seat, hue) in hues.iter().enumerate() {
                        assert_eq!(
                            hues.iter().filter(|h| *h == hue).count(),
                            1,
                            "{factions:?}: colours are unique"
                        );
                        for (viewer, faction) in factions.iter().enumerate() {
                            if viewer != seat {
                                assert_ne!(
                                    faction_hue(*faction),
                                    *hue,
                                    "{factions:?}: seat {viewer} sees seat {seat} in its own colour"
                                );
                            }
                        }
                    }
                }
            }
        }
        // The rotation of a view never changes who wears what.
        let factions = [Faction::Compact, Faction::Compact, Faction::Union];
        let hues = match_hues(&factions);
        for turn in 0..3u8 {
            let mut view = factions.to_vec();
            view.rotate_left(usize::from(turn));
            let mut expected = hues.clone();
            expected.rotate_left(usize::from(turn));
            assert_eq!(view_hues(&view, turn), expected);
        }
    }
}
