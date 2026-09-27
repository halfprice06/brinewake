//! Chooses what is heard: which song, how intense, which effect each
//! simulation event makes and where, the world's bed, and the small
//! interface sounds. Presentation only; it reads the world and never
//! changes it.

use crate::audio::Cue;
use crate::game::{Game, Screen};
use crate::music::{MusicState, SongId, Stop};
use bw_core::{Camera, Faction, Kind, Pos, Terrain};
use bw_sim::{Event, EventKind, Outcome, Tide, World};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Ticks a second.
const HZ: u64 = 30;
/// A fight this hot is a battle.
const BATTLE_HEAT: f32 = 5.0;
/// A battle cools to tension below this heat.
const COOL_HEAT: f32 = 1.5;
/// The heat halves every four seconds.
const HEAT_HALF_TICKS: f32 = 4.0 * 30.0;
/// A battle lasts at least eight bars of the match song (17 s).
const BATTLE_MIN_TICKS: u64 = 17 * HZ;
/// Tension lasts at least four bars.
const TENSION_MIN_TICKS: u64 = 9 * HZ;
/// Quiet this long returns the calm.
const CALM_AFTER_TICKS: u64 = 45 * HZ;
/// An army this size keeps the music working through quiet.
const WORKING_ARMY: usize = 6;
/// After this long without a fight, the music rests.
const REST_AFTER_TICKS: u64 = 270 * HZ;
/// And rests this long, the sea and wind alone.
const REST_TICKS: u64 = 30 * HZ;
/// The match song waits for the horn.
const MATCH_ENTRY: Duration = Duration::from_millis(2_500);
/// The final blow is followed by a breath of silence before the result.
const RESULT_SILENCE_TICKS: u64 = 30;

#[derive(Clone, Debug, Default)]
pub struct SoundState {
    heat: f32,
    intensity: u8,
    /// When the current intensity began, and the last tick the fight was
    /// hot enough to hold it.
    since: u64,
    hot_until: u64,
    threat_until: u64,
    quiet_since: u64,
    /// The music's rest: when the current unbroken calm stretch began and
    /// when a rest ends.
    unbroken_since: u64,
    rest_until: u64,
    attacks_heard: u32,
    last_alarm: Option<Instant>,
    last_select: Option<Instant>,
    pub(crate) last_submit: Option<Instant>,
    field_since: Option<Instant>,
    last_played: BTreeMap<u8, u64>,
    selection: Vec<u32>,
    /// The selection heard before the current one: recalling it again is
    /// quieter.
    heard_before: Vec<u32>,
    current_heard: Vec<u32>,
    /// The last tick read, to notice a new world.
    last_tick: u64,
    /// Rests taken so far, and the song to come back on: the partner of
    /// whatever was playing when the rest began.
    rests: u32,
    resting_now: bool,
    after_rest: Option<SongId>,
    rest_noted: bool,
}

/// Where an effect is heard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// Everywhere: public news and our own completions.
    Global,
    /// At a point on the field, heard only near the camera.
    At(Pos),
}

/// The weapon a machine fires.
pub fn weapon(kind: Kind) -> Cue {
    match kind {
        Kind::Riveter => Cue::ShotRivet,
        Kind::Bulwark => Cue::ShotCannon,
        Kind::Sounder | Kind::Skipper => Cue::ShotLight,
        Kind::Reedguard => Cue::ShotReed,
        Kind::Tower => Cue::ShotTower,
        Kind::Brander => Cue::ShotLance,
        Kind::Glinter => Cue::ShotShard,
        Kind::Heliostat => Cue::HeliostatBeam,
        Kind::Hook | Kind::Wick | Kind::Raker => Cue::ShotTool,
        _ => Cue::Shot,
    }
}

/// How a thing dies: machines by weight, buildings fall.
pub fn death(kind: Kind) -> Cue {
    if kind.is_building() || matches!(kind, Kind::Palisade | Kind::Tower) {
        Cue::BuildingCollapse
    } else if bw_content::spec(kind).health >= 250 {
        Cue::ExplosionLarge
    } else {
        Cue::Impact
    }
}

/// What breaks, by who built it.
pub fn debris(faction: Faction) -> Cue {
    match faction {
        Faction::Union => Cue::DebrisMetal,
        Faction::Assembly => Cue::DebrisReed,
        Faction::Compact => Cue::DebrisGlass,
    }
}

/// The effect an event makes, its loudness and where it is heard.
/// `before` holds each entity's kind as the tick began, so the dead can be
/// named. `visible` says whether the event's place is in our sight.
pub fn event_cue(
    world: &World,
    event: &Event,
    before: &BTreeMap<u32, (Kind, u8, Pos)>,
    visible: bool,
) -> Option<(Cue, f32, Place)> {
    let ours = event.player == Some(0);
    let at = |pos: Option<Pos>| pos.map(Place::At);
    let kind_of = |id: Option<u32>| {
        id.and_then(|id| {
            world
                .entities
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.kind)
                .or_else(|| before.get(&id).map(|(k, ..)| *k))
        })
    };
    // The deciding tick is heard too: the final blow sounds.
    Some(match event.kind {
        EventKind::Shot if visible => {
            if event.text == "Loom blast" {
                (Cue::ExplosionLarge, 0.9, at(event.to)?)
            } else {
                let shooter = kind_of(event.entity);
                let cue = shooter.map_or(Cue::Shot, weapon);
                let (cue, loud) = if cue == Cue::HeliostatBeam {
                    crate::audio_scene::shot_cue(world, event)
                } else {
                    (cue, 1.0)
                };
                (cue, loud, at(event.from.or(event.to))?)
            }
        }
        EventKind::Death if visible => {
            let kind = kind_of(event.entity)?;
            (death(kind), 1.0, at(event.from)?)
        }
        EventKind::ArtilleryWarning => {
            let target_seen = event.to.is_some_and(|p| world.visible(0, p));
            let from_seen = event.from.is_some_and(|p| world.visible(0, p));
            if target_seen {
                (Cue::ShellWhistle, 1.0, at(event.to)?)
            } else if from_seen {
                (Cue::LoomLaunch, 1.0, at(event.from)?)
            } else {
                return None;
            }
        }
        EventKind::BuildStarted if ours => (
            Cue::BuildPlaced,
            1.0,
            at(event.to.or(event.from)).unwrap_or(Place::Global),
        ),
        EventKind::BuildCompleted if ours => (Cue::BuildDone, 1.0, Place::Global),
        EventKind::ProductionCompleted if ours => (Cue::UnitReady, 1.0, Place::Global),
        EventKind::ResearchCompleted | EventKind::UpgradeCompleted if ours => {
            (Cue::ResearchDone, 1.0, Place::Global)
        }
        EventKind::GateWarning => (Cue::GateWarningBell, 1.0, Place::Global),
        EventKind::GateChanged => (Cue::GateChangeRush, 1.0, Place::Global),
        EventKind::Deposit if ours => (
            crate::audio_scene::deposit_cue(world.players[0].faction),
            1.0,
            Place::At(crate::audio_scene::event_position(world, event)?),
        ),
        EventKind::Vent if visible || ours => (Cue::Vent, 1.0, place(world, event)?),
        EventKind::Sound if visible || ours => (Cue::Sonar, 1.0, place(world, event)?),
        EventKind::Glint if visible || ours => (Cue::Glint, 1.0, place(world, event)?),
        EventKind::Surge if ours => (
            Cue::Surge,
            1.0,
            place(world, event).unwrap_or(Place::Global),
        ),
        EventKind::Deploy if visible || ours => (Cue::Deploy, 1.0, place(world, event)?),
        EventKind::Boarded | EventKind::Unloaded if visible || ours => {
            (Cue::Hatch, 0.9, place(world, event)?)
        }
        EventKind::Reclaimed | EventKind::Recycled if ours => (
            Cue::Reclaim,
            1.0,
            place(world, event).unwrap_or(Place::Global),
        ),
        EventKind::Repair if ours => (Cue::Repair, 0.7, place(world, event)?),
        EventKind::Swamped if visible => (Cue::Splash, 1.0, place(world, event)?),
        EventKind::CrustMelted if visible => (Cue::Sizzle, 0.8, place(world, event)?),
        EventKind::Laid if visible || ours => (Cue::SaltLaid, 1.0, place(world, event)?),
        _ => return None,
    })
}

fn place(world: &World, event: &Event) -> Option<Place> {
    crate::audio_scene::event_position(world, event).map(Place::At)
}

/// Where a point sits for the ear: its pan, its gain, and whether it is
/// just past the edge of the view (heard dulled and quieter) rather than
/// on screen. Points further out are not heard.
pub fn hear(camera: Camera, pos: Pos) -> Option<(f32, f32, bool)> {
    if let Some((pan, gain)) = crate::audio_scene::spatial(camera, pos) {
        return Some((pan.clamp(-0.75, 0.75), gain, false));
    }
    let (x, y) = camera.project(pos);
    if !(-160..800).contains(&x) || !(-36..324).contains(&y) {
        return None;
    }
    let pan = ((x - 320) as f32 / 320.0).clamp(-0.75, 0.75);
    // -9 dB.
    Some((pan, 0.35, true))
}

/// A selection's pitch in semitones from the patch's own note, by role,
/// with a variant from three chord tones.
pub fn select_pitch(kind: Kind, frame: u64) -> i8 {
    let spec = bw_content::spec(kind);
    let base: i8 = if kind.is_worker() {
        5
    } else if kind.is_transport() {
        -2
    } else if spec.damage == 0 {
        2
    } else if spec.health >= 250 || spec.range >= 6 {
        -5
    } else {
        0
    };
    // Three chord tones of D minor near the role's note.
    const VARIANTS: [i8; 3] = [0, 3, 7];
    let pitch = base + VARIANTS[(frame % 3) as usize];
    // Keep it on D minor's triad (D F A): step down to the nearest.
    const TRIAD: [i8; 3] = [0, 3, 7];
    let within = pitch.rem_euclid(12);
    let nearest = TRIAD
        .iter()
        .copied()
        .filter(|t| *t <= within)
        .max()
        .unwrap_or(0);
    pitch - within + nearest
}

/// The shortest gap between two plays of a cue, in ticks, so a stream of
/// events is one texture and not a pile.
fn min_gap(cue: Cue) -> u64 {
    match cue {
        Cue::Repair | Cue::Reclaim => 12,
        Cue::Hatch | Cue::Splash | Cue::SaltLaid | Cue::Sizzle => 5,
        Cue::BuildPlaced | Cue::UnitReady | Cue::BuildDone | Cue::ResearchDone => 3,
        Cue::ShotTool => 3,
        _ => 0,
    }
}

impl SoundState {
    /// Reads one tick's events into the fight's heat and the threat, and
    /// moves the intensity with some patience: a battle holds for eight
    /// bars and until the heat has cooled for six seconds; tension holds
    /// four bars; the calm returns after 45 quiet seconds.
    pub fn observe(&mut self, world: &World, hold: bool) {
        // A new world (a new match, a load, a replay) starts from calm.
        if world.tick < self.last_tick {
            *self = self.fresh();
        }
        self.last_tick = world.tick;
        let decay = 0.5f32.powf(1.0 / HEAT_HALF_TICKS);
        self.heat *= decay;
        let tick = world.tick;
        for event in &world.events {
            let seen = event.to.or(event.from).is_some_and(|p| world.visible(0, p));
            match event.kind {
                EventKind::Shot if seen => {
                    // Our own machines in it make it matter more.
                    let involved = event.player == Some(0)
                        || event
                            .other
                            .and_then(|id| world.entities.iter().find(|e| e.id == id))
                            .is_some_and(|e| e.owner == 0);
                    self.heat += if involved { 1.0 } else { 0.5 };
                }
                EventKind::Death if seen => self.heat += 2.5,
                EventKind::EnemySeen if event.player == Some(0) => {
                    self.threat_until = tick + 25 * HZ;
                }
                EventKind::ArtilleryWarning | EventKind::GateCaptureStarted => {
                    self.threat_until = tick + 20 * HZ;
                }
                _ => {}
            }
        }
        let enemy_near = world.entities.iter().any(|e| {
            e.owner != 0
                && (e.owner as usize) < world.seat_count()
                && !e.kind.is_building()
                && bw_content::spec(e.kind).damage > 0
                && world.visible(0, e.pos)
        });
        if enemy_near {
            self.threat_until = self.threat_until.max(tick + 6 * HZ);
        }
        let contested =
            world.gate.capture_player.is_some_and(|p| p != 0) && world.gate.capture_progress > 0;
        let threatened = tick < self.threat_until || contested || hold;
        if self.heat >= COOL_HEAT {
            self.hot_until = tick + 6 * HZ;
        }
        if self.heat >= 0.5 || threatened {
            self.quiet_since = tick;
        }
        // A standing army keeps the harbour working; without one, quiet
        // brings the calm back.
        let army = world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && !e.kind.is_building()
                    && !e.kind.is_worker()
                    && bw_content::spec(e.kind).damage > 0
            })
            .count();
        let held = tick.saturating_sub(self.since);
        // Reaching a level and holding it are separate cases.
        #[allow(clippy::if_same_then_else)]
        let wanted = if self.heat >= BATTLE_HEAT {
            3
        } else if self.intensity == 3 && (held < BATTLE_MIN_TICKS || tick < self.hot_until) {
            3
        } else if threatened || self.heat >= COOL_HEAT {
            2
        } else if self.intensity == 2 && held < TENSION_MIN_TICKS {
            2
        } else if tick < 60 * HZ
            || (tick.saturating_sub(self.quiet_since) >= CALM_AFTER_TICKS && army < WORKING_ARMY)
        {
            0
        } else {
            1
        };
        if wanted != self.intensity {
            self.intensity = wanted;
            self.since = tick;
        }
        // A long unbroken stretch without a fight earns the music a rest.
        if self.intensity >= 2 {
            self.unbroken_since = tick;
            self.rest_until = 0;
        } else if tick.saturating_sub(self.unbroken_since) >= REST_AFTER_TICKS {
            self.rest_until = tick + REST_TICKS;
            self.unbroken_since = tick + REST_TICKS;
        }
        let resting = self.resting(tick);
        if self.resting_now && !resting {
            self.rests += 1;
        }
        self.resting_now = resting;
    }

    pub fn intensity(&self) -> u8 {
        self.intensity
    }

    /// A state for a new match, keeping only the interface's clocks.
    pub fn fresh(&self) -> Self {
        Self {
            last_submit: self.last_submit,
            last_select: self.last_select,
            last_alarm: self.last_alarm,
            ..Self::default()
        }
    }

    fn resting(&self, tick: u64) -> bool {
        tick < self.rest_until
    }

    /// Whether a cue may play now, and note that it did.
    pub fn allow(&mut self, cue: Cue, tick: u64) -> bool {
        let gap = min_gap(cue);
        if gap == 0 {
            return true;
        }
        let key = cue as u8;
        if self
            .last_played
            .get(&key)
            .is_some_and(|&last| tick.saturating_sub(last) < gap && last <= tick)
        {
            return false;
        }
        self.last_played.insert(key, tick);
        true
    }
}

fn is_match_song(song: SongId) -> bool {
    matches!(song, SongId::Match | SongId::Confluence | SongId::Undertow)
}

/// When a rest begins: the song to come back on is the next in the
/// rotation after the one playing (or the map's own).
fn song_after_rest(playing: Option<SongId>, map_song: SongId) -> SongId {
    match playing {
        Some(song) if is_match_song(song) => song.rotation_next(),
        _ => map_song,
    }
}

/// When a rest ends: if a match song still sounds (the rest was called off
/// before the music fell silent), keep it rather than cross-fade into
/// another key; otherwise the one chosen when the rest began.
fn song_when_rest_ends(planned: Option<SongId>, playing: Option<SongId>) -> Option<SongId> {
    match playing {
        Some(song) if is_match_song(song) => Some(song),
        _ => planned,
    }
}

impl Game {
    /// Once a frame: the song, its intensity and the world bed.
    pub(crate) fn update_sound(&mut self) {
        if self.in_field() {
            self.sound.field_since.get_or_insert_with(Instant::now);
        } else {
            self.sound.field_since = None;
        }
        // When a rest begins, note the song to come back on.
        let resting = self.sound.resting(self.world.tick);
        if resting && !self.sound.rest_noted {
            self.sound.rest_noted = true;
            let playing = self.audio.as_ref().and_then(|a| a.playing_song());
            self.sound.after_rest = Some(song_after_rest(playing, self.match_song()));
        } else if !resting {
            if self.sound.rest_noted {
                let playing = self.audio.as_ref().and_then(|a| a.playing_song());
                self.sound.after_rest = song_when_rest_ends(self.sound.after_rest, playing);
            }
            self.sound.rest_noted = false;
        }
        let state = self.music_state();
        let bed = self.ambience_state();
        self.selection_sound();
        self.attack_alarm();
        if let Some(audio) = &self.audio {
            audio.set_music(state, false);
            audio.set_ambience(bed.0, bed.1, bed.2);
        }
    }

    fn in_field(&self) -> bool {
        self.has_session()
            && match self.screen {
                Screen::Match | Screen::Pause => true,
                Screen::Confirm => matches!(self.ux.confirm_return, Screen::Match | Screen::Pause),
                Screen::Help => matches!(self.paused_from_help, Screen::Match | Screen::Pause),
                _ => false,
            }
    }

    /// Whether our count runs, whether an enemy's does, and the seconds
    /// left on the running one (0 when none).
    fn hold_state(&self) -> (bool, bool, u8) {
        let Some((seat, ticks)) = self.hold_gauge() else {
            return (false, false, 0);
        };
        let holding = self.world.holds_every_lane(seat);
        let left = self.world.hold_ticks().saturating_sub(ticks);
        let secs = if holding {
            (u64::from(left).div_ceil(HZ)).clamp(1, 250) as u8
        } else {
            0
        };
        if seat == 0 {
            (holding, false, secs)
        } else {
            (false, holding, secs)
        }
    }

    /// The match song: the map's own, and after a rest the partner of the
    /// one that was playing (the engine also hands over by itself).
    fn match_song(&self) -> SongId {
        self.sound
            .after_rest
            .unwrap_or(if self.world.map.id == bw_sim::MapId::Confluence {
                SongId::Confluence
            } else {
                SongId::Match
            })
    }

    pub(crate) fn music_state(&self) -> MusicState {
        if !self.in_field() {
            return MusicState {
                song: Some(SongId::Title),
                ..MusicState::default()
            };
        }
        let variant = match self
            .world
            .players
            .first()
            .map_or(self.faction, |p| p.faction)
        {
            Faction::Union => 0,
            Faction::Assembly => 1,
            Faction::Compact => 2,
        };
        let muffled = self.screen != Screen::Match;
        if let Some(outcome) = &self.world.outcome {
            // A breath of silence after the final blow, then the result.
            let song = if self.aftermath_ticks < RESULT_SILENCE_TICKS {
                None
            } else {
                Some(match outcome {
                    Outcome::Victory(0) => SongId::Victory,
                    Outcome::Victory(_) => SongId::Defeat,
                    _ => SongId::Draw,
                })
            };
            return MusicState {
                song,
                variant,
                muffled,
                stop: Stop::Quick,
                ..MusicState::default()
            };
        }
        if self.ux.practice_review {
            return MusicState {
                song: Some(SongId::Harbour),
                muffled,
                ..MusicState::default()
            };
        }
        // The horn sounds alone before the match song comes in.
        let entering = self
            .sound
            .field_since
            .is_none_or(|since| since.elapsed() < MATCH_ENTRY);
        let resting = self.sound.resting(self.world.tick);
        if entering || resting {
            return MusicState {
                song: None,
                muffled,
                variant,
                // A rest ends the phrase first.
                stop: if resting { Stop::Phrase } else { Stop::Gentle },
                ..MusicState::default()
            };
        }
        let (hold_ours, hold_theirs, hold_left) = self.hold_state();
        let mut intensity = if self.intro.is_some() {
            0
        } else {
            self.sound.intensity()
        };
        if self.ux.practice {
            // Learning is calm unless there is a real fight.
            intensity = intensity.min(if self.sound.intensity() >= 3 { 2 } else { 1 });
        }
        MusicState {
            song: Some(self.match_song()),
            intensity,
            hold_ours,
            hold_theirs,
            hold_left,
            stop: Stop::Gentle,
            flood: self.world.gate.tide == Tide::Flood,
            muffled,
            variant,
        }
    }

    /// The world bed: its level, the sea and the wind. More sea when the
    /// view holds water, more still in a flood.
    fn ambience_state(&self) -> (u8, u8, u8) {
        if !self.in_field() {
            // Home hears only a little wind under the title.
            return (35, 25, 35);
        }
        let mut wet = 0u32;
        for row in 0..5 {
            for col in 0..7 {
                let pos = self.camera.unproject(32 + col * 96, 40 + row * 48);
                let (x, y) = pos.cell_xy();
                let terrain = self.world.map.terrain(x, y);
                if terrain == Terrain::Deep
                    || crate::presentation::cell_is_wet(&self.world, terrain)
                {
                    wet += 1;
                }
            }
        }
        let view = wet as f32 / 35.0;
        let tide = match self.world.gate.tide {
            // A flood is heard, but stays well under the music.
            Tide::Flood => 0.55,
            Tide::Neutral => 0.6,
            Tide::Open => 0.45,
        };
        let water = (100.0 * tide * (0.45 + 0.55 * view)).clamp(10.0, 100.0) as u8;
        // The result screen lets the music lead.
        let level = if self.world.outcome.is_some() || self.ux.practice_review {
            60
        } else if self.screen == Screen::Match {
            100
        } else {
            45
        };
        (level, water, 45)
    }

    /// Our own machines newly selected speak in their faction's voice.
    fn selection_sound(&mut self) {
        if self.screen != Screen::Match {
            self.sound.selection.clear();
            return;
        }
        if self.selected == self.sound.selection {
            return;
        }
        let fresh = self
            .selected
            .iter()
            .any(|id| !self.sound.selection.contains(id));
        self.sound.selection = self.selected.clone();
        if !fresh
            || self
                .sound
                .last_select
                .is_some_and(|t| t.elapsed() < Duration::from_millis(100))
        {
            return;
        }
        let ours = self.selected.iter().any(|id| {
            self.world
                .entities
                .iter()
                .any(|e| e.id == *id && e.owner == 0)
        });
        if !ours {
            return;
        }
        self.sound.last_select = Some(Instant::now());
        let faction = self
            .world
            .players
            .first()
            .map_or(self.faction, |p| p.faction);
        let cue = match faction {
            Faction::Union => Cue::SelectUnion,
            Faction::Assembly => Cue::SelectAssembly,
            Faction::Compact => Cue::SelectCompact,
        };
        // The role of what was picked sets the pitch, inside D Dorian:
        // workers high, scouts and harassers above the line, heavies and
        // artillery low, the rest in the middle; three nearby notes vary it.
        let kinds: Vec<Kind> = self
            .selected
            .iter()
            .filter_map(|id| {
                self.world
                    .entities
                    .iter()
                    .find(|e| e.id == *id && e.owner == 0)
            })
            .map(|e| e.kind)
            .collect();
        let semis = kinds
            .first()
            .map_or(0, |&kind| select_pitch(kind, self.frame));
        let mut sorted = self.selected.clone();
        sorted.sort_unstable();
        let recalled = sorted == self.sound.heard_before;
        let previous = std::mem::replace(&mut self.sound.current_heard, sorted);
        self.sound.heard_before = previous;
        if let Some(audio) = &self.audio {
            audio.play_pitched(cue, semis, if recalled { 0.5 } else { 1.0 });
        }
    }

    /// An attack card raised: the alarm, unless we are looking at the fight
    /// or heard one lately.
    fn attack_alarm(&mut self) {
        let raised = self.ux.alerts.attacks_raised;
        if raised == self.sound.attacks_heard {
            return;
        }
        self.sound.attacks_heard = raised;
        let Some(pos) = self.ux.alerts.last_attack else {
            return;
        };
        if self.screen != Screen::Match || self.world.outcome.is_some() {
            return;
        }
        if crate::audio_scene::spatial(self.camera, pos).is_some() {
            return;
        }
        if self
            .sound
            .last_alarm
            .is_some_and(|t| t.elapsed() < Duration::from_secs(12))
        {
            return;
        }
        self.sound.last_alarm = Some(Instant::now());
        if let Some(audio) = &self.audio {
            audio.play(Cue::UnderAttack);
        }
    }

    /// One tick's events as effects.
    pub(crate) fn play_events(&mut self, before: &BTreeMap<u32, (Kind, u8, Pos)>) {
        let (hold_ours, hold_theirs, _) = self.hold_state();
        self.sound.observe(&self.world, hold_ours || hold_theirs);
        let Some(audio) = &self.audio else { return };
        let mut per_cue: BTreeMap<u8, u8> = BTreeMap::new();
        for event in &self.world.events {
            let visible = event
                .to
                .or(event.from)
                .is_some_and(|p| self.world.visible(0, p));
            let Some((cue, loud, place)) = event_cue(&self.world, event, before, visible) else {
                continue;
            };
            // A volley is three shots loud, not ten.
            let count = per_cue.entry(cue as u8).or_insert(0);
            *count += 1;
            if *count > 3 {
                continue;
            }
            if !self.sound.allow(cue, self.world.tick) {
                continue;
            }
            // Machines break in their builder's material.
            let debris = (event.kind == EventKind::Death
                && matches!(cue, Cue::Impact | Cue::ExplosionLarge))
            .then(|| {
                event
                    .player
                    .and_then(|seat| self.world.players.get(usize::from(seat)))
            })
            .flatten()
            .map(|p| debris(p.faction));
            match place {
                Place::Global => audio.play_spatial(cue, 0.0, loud),
                Place::At(pos) => match hear(self.camera, pos) {
                    Some((pan, gain, false)) => {
                        audio.play_spatial(cue, pan, gain * loud);
                        if let Some(d) = debris {
                            audio.play_spatial(d, pan, gain * 0.8);
                        }
                    }
                    Some((pan, gain, true)) => audio.play_far(cue, pan, gain * loud),
                    None => {}
                },
            }
        }
    }
}

/// Runs recorded matches through the music director and a model of the
/// engine's hand-overs: time at each intensity, rests, song changes, and
/// the longest stretch on one song. `brinewake --sound-replay FILE...`.
pub fn measure_replays(paths: &[String]) -> Result<String, String> {
    use crate::music::{MusicState, Stop};
    use crate::music_timeline::{Cue as TimelineCue, run};
    let mut out = String::new();
    for path in paths {
        let mut player = match bw_sim::ReplayPlayer::open(path) {
            Ok(p) => p,
            Err(error) => {
                out.push_str(&format!("{path}: not measured ({error})\n"));
                continue;
            }
        };
        // The requests the director would make, tick by tick, played
        // through the real engine.
        let mut sound = SoundState::default();
        let mut at = [0u64; 4];
        let mut cues: Vec<TimelineCue> = Vec::new();
        let mut last: Option<MusicState> = None;
        let mut ticks = 0u64;
        while !player.finished() {
            if !player.step()? {
                break;
            }
            let world = player.world();
            let ours = world.holds_every_lane(0);
            let holder = world
                .seats()
                .skip(1)
                .find(|&seat| world.holds_every_lane(seat));
            sound.observe(world, ours || holder.is_some());
            let i = sound.intensity();
            at[usize::from(i)] += 1;
            ticks = world.tick;
            let left = |seat: u8| {
                let gauge = world.lane_hold.get(usize::from(seat)).copied().unwrap_or(0);
                (u64::from(world.hold_ticks().saturating_sub(gauge)).div_ceil(HZ)).clamp(1, 250)
                    as u8
            };
            let map_song = if world.map.id == bw_sim::MapId::Confluence {
                SongId::Confluence
            } else {
                SongId::Match
            };
            let state = if sound.resting(world.tick) {
                MusicState {
                    song: None,
                    stop: Stop::Phrase,
                    ..MusicState::default()
                }
            } else {
                MusicState {
                    song: Some(map_song),
                    intensity: i,
                    hold_ours: ours,
                    hold_theirs: holder.is_some() && !ours,
                    hold_left: if ours {
                        left(0)
                    } else {
                        holder.map_or(0, left)
                    },
                    ..MusicState::default()
                }
            };
            if last != Some(state) {
                cues.push(TimelineCue {
                    at: world.tick as f64 / HZ as f64,
                    state,
                });
                last = Some(state);
            }
        }
        let seconds = ticks as f64 / HZ as f64;
        let report = run(&cues, seconds, 1_000.0);
        let minutes = |t: u64| t as f64 / HZ as f64 / 60.0;
        let mut per_song = String::new();
        for song in [SongId::Match, SongId::Confluence, SongId::Undertow] {
            let total: f64 = report
                .runs
                .iter()
                .filter(|r| r.0 == Some(song))
                .map(|r| r.2 - r.1)
                .sum();
            per_song.push_str(&format!(" {song:?} {:.1}", total / 60.0));
        }
        let longest = report
            .runs
            .iter()
            .filter(|r| r.0.is_some())
            .map(|r| r.2 - r.1)
            .fold(0.0, f64::max);
        out.push_str(&format!(
            "{path}: {:.1} min; calm {:.1}, working {:.1}, tense {:.1}, fighting {:.1} min; {} rests; songs heard (min):{per_song}; {} song changes; longest on one song {:.1} min; {} engine rule violations\n",
            seconds / 60.0,
            minutes(at[0]),
            minutes(at[1]),
            minutes(at[2]),
            minutes(at[3]),
            sound.rests,
            report.songs_heard.len().saturating_sub(1),
            longest / 60.0,
            report.violations.len()
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The song after a rest, in its three cases: the rest ran its course
    /// (the music had fallen silent), or it was called off during the
    /// phrase wait or the fade (the song still sounds).
    #[test]
    fn the_song_after_a_rest() {
        let planned = song_after_rest(Some(SongId::Confluence), SongId::Match);
        assert_eq!(planned, SongId::Undertow);
        assert_eq!(
            song_after_rest(Some(SongId::Title), SongId::Confluence),
            SongId::Confluence
        );
        // Ran its course: silent at the end.
        assert_eq!(
            song_when_rest_ends(Some(planned), None),
            Some(SongId::Undertow)
        );
        // Called off while Confluence still waits for its phrase or fades.
        assert_eq!(
            song_when_rest_ends(Some(planned), Some(SongId::Confluence)),
            Some(SongId::Confluence)
        );
    }

    #[test]
    fn every_gun_has_its_voice_and_weight_decides_a_death() {
        assert_eq!(weapon(Kind::Riveter), Cue::ShotRivet);
        assert_eq!(weapon(Kind::Heliostat), Cue::HeliostatBeam);
        assert_eq!(death(Kind::Headquarters), Cue::BuildingCollapse);
        assert_eq!(death(Kind::Bulwark), Cue::ExplosionLarge);
        assert_eq!(death(Kind::Hook), Cue::Impact);
        assert_eq!(debris(Faction::Compact), Cue::DebrisGlass);
    }

    #[test]
    fn repeated_repairs_are_one_texture() {
        let mut s = SoundState::default();
        assert!(s.allow(Cue::Repair, 100));
        assert!(!s.allow(Cue::Repair, 105));
        assert!(s.allow(Cue::Repair, 113));
        assert!(s.allow(Cue::ShotRivet, 113));
        assert!(s.allow(Cue::ShotRivet, 113));
    }

    #[test]
    fn home_plays_the_title_and_a_match_its_song_after_the_horn() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        assert_eq!(g.music_state().song, Some(SongId::Title));
        g.start();
        g.open_screen(Screen::Match);
        g.update_sound();
        assert_eq!(g.music_state().song, None, "the horn sounds alone first");
        g.sound.field_since = Some(Instant::now() - Duration::from_secs(3));
        let state = g.music_state();
        assert_eq!(state.song, Some(SongId::Match));
        assert!(!state.muffled);
        g.open_screen(Screen::Pause);
        assert!(g.music_state().muffled, "the pause menu muffles the field");
    }

    #[test]
    fn a_new_match_starts_calm_whatever_the_last_one_did() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.start();
        let mut s = SoundState::default();
        g.world.tick = 36_000;
        s.heat = 6.0;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 3);
        g.start();
        g.world.tick = 30;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 0, "a rematch opens calm");
    }

    #[test]
    fn intensity_climbs_holds_and_comes_home() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.start();
        let mut s = SoundState::default();
        // Quiet start: calm, and calm while nothing has happened.
        g.world.tick = 10 * HZ;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 0);
        g.world.tick = 50 * HZ;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 0);
        // A hot fight: battle, held eight bars even when it cools at once.
        s.heat = 6.0;
        g.world.tick = 60 * HZ;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 3);
        s.heat = 0.0;
        g.world.tick = 70 * HZ;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 3, "a battle lasts eight bars");
        g.world.tick = 80 * HZ;
        s.observe(&g.world, false);
        assert!(s.intensity() < 3);
        // Long quiet: calm again, whatever the clock says.
        g.world.tick = 200 * HZ;
        s.observe(&g.world, false);
        assert_eq!(s.intensity(), 0, "the calm comes back");
    }
}
