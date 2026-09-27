//! Quiet harbor scheduling from public infrastructure and local-player activity.
//! This module returns presentation requests. It never changes the world.
use crate::audio::Cue;
use bw_core::{Camera, Faction, Kind, Pos, Terrain};
use bw_sim::{Event, EventKind, Order, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarborSound {
    Condenser,
    Winch,
    Reed,
    Water,
    UnionPhrase,
    AssemblyPhrase,
    /// The Compact's work: struck glass.
    Chime,
    CompactPhrase,
}

/// The cue a harbor sound plays.
pub fn harbor_cue(sound: HarborSound) -> Cue {
    match sound {
        HarborSound::Condenser => Cue::CondenserBreath,
        HarborSound::Winch => Cue::UnionWinchClack,
        HarborSound::Reed => Cue::AssemblyReed,
        HarborSound::Water => Cue::WaterWash,
        HarborSound::UnionPhrase => Cue::UnionWorkPhrase,
        HarborSound::AssemblyPhrase => Cue::AssemblyWorkPhrase,
        HarborSound::Chime => Cue::CompactGlassChime,
        HarborSound::CompactPhrase => Cue::CompactWorkPhrase,
    }
}

/// The sound a faction's workers and Works make at work.
pub fn work_sound(faction: Faction) -> HarborSound {
    match faction {
        Faction::Union => HarborSound::Winch,
        Faction::Assembly => HarborSound::Reed,
        Faction::Compact => HarborSound::Chime,
    }
}

/// The phrase a faction's working base plays.
pub fn phrase_sound(faction: Faction) -> HarborSound {
    match faction {
        Faction::Union => HarborSound::UnionPhrase,
        Faction::Assembly => HarborSound::AssemblyPhrase,
        Faction::Compact => HarborSound::CompactPhrase,
    }
}

/// The cue a worker's deposit plays for its faction.
pub fn deposit_cue(faction: Faction) -> Cue {
    match faction {
        Faction::Union => Cue::UnionDepositTap,
        Faction::Assembly => Cue::AssemblyReed,
        Faction::Compact => Cue::CompactDepositChime,
    }
}

/// The cue a shot plays and its gain: a Heliostat's beam hums, louder as
/// it builds on one target; every other gun fires the ordinary shot.
pub fn shot_cue(world: &World, event: &Event) -> (Cue, f32) {
    let heliostat = event.kind == EventKind::Shot
        && event
            .entity
            .and_then(|id| world.entities.iter().find(|e| e.id == id))
            .is_some_and(|e| e.kind == Kind::Heliostat);
    if !heliostat {
        return (Cue::Shot, 1.0);
    }
    let first = bw_content::spec(Kind::Heliostat).damage;
    let span = (bw_content::HELIOSTAT_BEAM_MAX - first).max(1);
    // The shot's amount has the owner's upgrades in it; its build-up is
    // read against the plain beam, capped.
    let built = (event.amount - first).clamp(0, span) as f32 / span as f32;
    (Cue::HeliostatBeam, 0.6 + 0.4 * built)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarborVoice {
    pub sound: HarborSound,
    pub pan: f32,
    pub gain: f32,
}

/// Economy events such as Deposit identify their worker but carry no position.
/// Resolve only owned sources; a hidden enemy ID must not become an audio cue.
pub fn event_position(world: &World, event: &Event) -> Option<Pos> {
    event.to.or(event.from).or_else(|| {
        if event.player != Some(0) {
            return None;
        }
        let id = event.entity?;
        world
            .entities
            .iter()
            .find(|e| e.id == id && e.owner == 0)
            .map(|e| e.pos)
    })
}

/// Offscreen private activity is silent; the public gate has its own alert path.
pub fn spatial(camera: Camera, pos: Pos) -> Option<(f32, f32)> {
    let (x, y) = camera.project(pos);
    if !(0..640).contains(&x) || !(24..264).contains(&y) {
        return None;
    }
    let pan = ((x - 320) as f32 / 320.0).clamp(-1.0, 1.0);
    let distance = (((x - 320) as f32 / 400.0).powi(2) + ((y - 152) as f32 / 220.0).powi(2))
        .sqrt()
        .min(1.0);
    Some((pan, 0.2 + 0.8 * (1.0 - distance)))
}

/// A deterministic sparse score: at most three work voices and one water bed
/// or phrase per tick. The audio mixer separately caps simultaneous voices.
pub fn scheduled(world: &World, camera: Camera, clock: u64, aftermath: bool) -> Vec<HarborVoice> {
    let mut voices = Vec::with_capacity(4);
    if !aftermath {
        for entity in world
            .entities
            .iter()
            .filter(|entity| entity.owner == 0 && entity.hp > 0 && entity.build_remaining == 0)
        {
            let Some((pan, gain)) = spatial(camera, entity.pos) else {
                continue;
            };
            let phase = clock.wrapping_add(u64::from(entity.id) * 17);
            let sound = match entity.kind {
                Kind::Condenser if phase.is_multiple_of(156) => Some(HarborSound::Condenser),
                Kind::Hook | Kind::Wick | Kind::Raker
                    if matches!(entity.order, Order::Gather { .. })
                        && entity.gather_ticks > 0
                        && phase.is_multiple_of(123) =>
                {
                    Some(match entity.kind {
                        Kind::Hook => HarborSound::Winch,
                        Kind::Wick => HarborSound::Reed,
                        _ => HarborSound::Chime,
                    })
                }
                Kind::Works
                    if entity
                        .queue
                        .first()
                        .is_some_and(|item| item.started && item.remaining > 1)
                        && phase.is_multiple_of(141) =>
                {
                    Some(work_sound(world.players[0].faction))
                }
                _ => None,
            };
            if let Some(sound) = sound {
                voices.push(HarborVoice {
                    sound,
                    pan,
                    gain: gain * 0.7,
                });
                if voices.len() == 3 {
                    break;
                }
            }
        }
    }
    // Public terrain is sufficient for this bed. No enemy or resource count
    // influences it, including when the terminal field keeps the water alive.
    if clock % 120 == 1 {
        let water = (0..5)
            .flat_map(|row| (0..7).map(move |col| (row, col)))
            .find_map(|(row, col)| {
                let pos = camera.unproject(32 + col * 96, 40 + row * 48);
                let (x, y) = pos.cell_xy();
                let terrain = world.map.terrain(x, y);
                let wet = crate::presentation::cell_is_wet(world, terrain);
                (terrain == Terrain::Deep || wet).then_some(pos)
            });
        if let Some(pos) = water
            && let Some((pan, gain)) = spatial(camera, pos)
        {
            voices.push(HarborVoice {
                sound: HarborSound::Water,
                pan,
                gain: if aftermath { 0.85 } else { gain * 0.65 },
            });
        }
    } else if !aftermath && clock % 420 == 90 {
        // A phrase belongs to the selected faction's visible working base;
        // it is not a meter of global or concealed economic strength.
        if let Some(pos) = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters && e.hp > 0)
            .map(|e| e.pos)
            && let Some((pan, gain)) = spatial(camera, pos)
        {
            voices.push(HarborVoice {
                sound: phrase_sound(world.players[0].faction),
                pan,
                gain: gain * 0.7,
            });
        }
    }
    voices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_deposits_resolve_the_owned_worker_for_spatial_audio() {
        let mut world = World::new(17, Faction::Union);
        world.ai_enabled = false;
        for _ in 0..1200 {
            world.step();
            if let Some(event) = world
                .events
                .iter()
                .find(|e| e.kind == bw_sim::EventKind::Deposit && e.player == Some(0))
            {
                assert!(event.from.is_none() && event.to.is_none());
                let pos = event_position(&world, event).expect("deposit worker supplies position");
                let mut camera = Camera::default();
                camera.center(pos);
                assert!(spatial(camera, pos).is_some());
                let mut hidden = event.clone();
                hidden.player = Some(1);
                assert_eq!(event_position(&world, &hidden), None);
                return;
            }
        }
        panic!("expected an ordinary worker deposit");
    }
    fn assembly_cue(cue: Cue) -> bool {
        matches!(cue, Cue::AssemblyReed | Cue::AssemblyWorkPhrase)
    }

    #[test]
    fn each_faction_works_to_its_own_sounds() {
        for (i, a) in Faction::ALL.iter().enumerate() {
            for b in &Faction::ALL[i + 1..] {
                assert_ne!(deposit_cue(*a), deposit_cue(*b));
                assert_ne!(harbor_cue(work_sound(*a)), harbor_cue(work_sound(*b)));
                assert_ne!(harbor_cue(phrase_sound(*a)), harbor_cue(phrase_sound(*b)));
            }
        }
        assert!(!assembly_cue(deposit_cue(Faction::Compact)));
        assert!(!assembly_cue(harbor_cue(work_sound(Faction::Compact))));
        assert!(!assembly_cue(harbor_cue(phrase_sound(Faction::Compact))));
    }

    /// A Compact base at work, heard as the game hears it: the harbor score
    /// over its base and its own deposits. Nothing it plays is the
    /// Assembly's, and its Rakers, deposits and base phrase are heard.
    #[test]
    fn no_compact_event_plays_an_assembly_cue() {
        let mut world = World::new(23, Faction::Compact);
        world.ai_enabled = false;
        let hq = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .unwrap();
        let mut camera = Camera::default();
        camera.center(hq);
        let mut heard = Vec::new();
        for _ in 0..2400 {
            world.step();
            for voice in scheduled(&world, camera, world.tick, false) {
                heard.push(harbor_cue(voice.sound));
            }
            for event in &world.events {
                match event.kind {
                    EventKind::Deposit if event.player == Some(0) => {
                        heard.push(deposit_cue(world.players[0].faction));
                    }
                    EventKind::Shot => heard.push(shot_cue(&world, event).0),
                    _ => {}
                }
            }
        }
        assert!(!heard.iter().any(|cue| assembly_cue(*cue)), "{heard:?}");
        for cue in [
            Cue::CompactGlassChime,
            Cue::CompactDepositChime,
            Cue::CompactWorkPhrase,
        ] {
            assert!(heard.contains(&cue), "{cue:?} not heard");
        }
    }

    #[test]
    fn a_heliostat_shot_hums_and_swells_as_its_beam_builds() {
        let mut world = World::new(23, Faction::Compact);
        let (id, target) = {
            let mut own = world.entities.iter_mut().filter(|e| e.owner == 0);
            let heliostat = own.find(|e| !e.kind.is_building()).unwrap();
            heliostat.kind = Kind::Heliostat;
            (heliostat.id, heliostat.pos)
        };
        let shot = |entity, amount| Event {
            tick: 0,
            kind: EventKind::Shot,
            player: Some(0),
            entity: Some(entity),
            other: None,
            from: Some(target),
            to: Some(target),
            amount,
            text: "shot".into(),
            cause: None,
        };
        let first = bw_content::spec(Kind::Heliostat).damage;
        let (cue, soft) = shot_cue(&world, &shot(id, first));
        assert_eq!(cue, Cue::HeliostatBeam);
        let (_, loud) = shot_cue(&world, &shot(id, bw_content::HELIOSTAT_BEAM_MAX));
        assert!(loud > soft && loud <= 1.0 && soft >= 0.6);
        // Any other gun is an ordinary shot.
        let other = world
            .entities
            .iter()
            .find(|e| e.kind != Kind::Heliostat)
            .unwrap()
            .id;
        assert_eq!(shot_cue(&world, &shot(other, 5)), (Cue::Shot, 1.0));
        world.entities.retain(|e| e.id != id);
        assert_eq!(shot_cue(&world, &shot(id, 5)).0, Cue::Shot);
    }

    #[test]
    fn offscreen_sources_are_silent_and_spatial_gain_is_bounded() {
        let mut camera = Camera::default();
        camera.center(Pos::cell(30, 30));
        assert!(spatial(camera, Pos::cell(110, 110)).is_none());
        for x in 15..45 {
            for y in 15..45 {
                if let Some((pan, gain)) = spatial(camera, Pos::cell(x, y)) {
                    assert!((-1.0..=1.0).contains(&pan));
                    assert!((0.2..=1.0).contains(&gain));
                }
            }
        }
    }
    #[test]
    fn concealed_enemy_activity_never_changes_the_harbor_score() {
        let world = World::new(17, Faction::Union);
        let mut altered = world.clone();
        let mut camera = Camera::default();
        camera.center(Pos::cell(25, 25));
        for e in altered.entities.iter_mut().filter(|e| e.owner == 1) {
            e.pos = Pos::cell(25, 25);
            e.kind = Kind::Condenser;
            e.build_remaining = 0;
            e.hp = 100;
        }
        let original_hash = world.state_hash();
        for tick in 0..500 {
            assert_eq!(
                scheduled(&world, camera, tick, false),
                scheduled(&altered, camera, tick, false)
            );
            assert!(scheduled(&world, camera, tick, false).len() <= 4);
            assert!(
                scheduled(&world, camera, tick, true)
                    .iter()
                    .all(|v| v.sound == HarborSound::Water)
            );
        }
        assert_eq!(world.state_hash(), original_hash);
    }
}
