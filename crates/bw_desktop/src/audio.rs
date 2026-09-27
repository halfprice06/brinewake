//! The sound: effects, music and the world's ambience, mixed in CPAL's
//! output callback.
//!
//! The simulation never depends on this module. `Audio` is a best-effort
//! handoff: cues go through bounded queues (a full queue drops a cue), the
//! music and ambience are steered through atomics, and a missing output
//! device means the match plays silent. The callback owns fixed voice
//! arrays and performs no heap allocation.
//!
//! Signal flow: effect voices (layered `sfx` patches) and the ambience bed
//! and the music (`music`, ducked under alerts) each have a bus level; all
//! three send to one harbour reverb; the sum goes through a glue
//! compressor and a ceiling (`synth::Master`).

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::ambience::{Ambience, AmbienceControl};
use crate::music::{MusicControl, MusicEngine, MusicState, Song, SongId};
use crate::sfx::{self, Patch, Player};
use crate::synth::{Master, Reverb};

const EFFECT_COMMAND_CAPACITY: usize = 64;
const ALERT_COMMAND_CAPACITY: usize = 16;
const AMBIENCE_COMMAND_CAPACITY: usize = 32;
const MUSIC_COMMAND_CAPACITY: usize = 16;
const MAX_DRAIN_ALERTS: usize = 8;
const MAX_DRAIN_EFFECTS: usize = 24;
const MAX_DRAIN_AMBIENCE: usize = 8;
const MAX_DRAIN_MUSIC: usize = 4;
const MAX_VOICES: usize = 24;
const MAX_AMBIENT_VOICES: usize = 8;
const ORDER_SUPPRESSION_MS: u32 = 90;
/// The ceiling every output sample stays under.
const MAX_OUTPUT: f32 = 0.90;
/// At most this many of one cue sound at once.
const MAX_PER_CUE: usize = 4;
/// Samples a hushed voice takes to fade.
const FADE_MS: f32 = 15.0;
/// Bus trims, set by measurement (design/reviews/audio): effects peak
/// around -6 dBFS, the music sits near -16 LUFS at full volume.
const EFFECTS_GAIN: f32 = 2.4;
const MUSIC_GAIN: f32 = 0.62;
const AMBIENCE_GAIN: f32 = 3.2;
const REVERB_RETURN: f32 = 0.55;
/// How far the music drops under an alert.
const DUCK_DEPTH: f32 = 0.55;
/// How long an alert holds the ducks down.
const ALERT_DUCK_SECONDS: f32 = 1.5;

/// Destination mix bus for a cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    /// Immediate commands, combat, warnings, and other tactical feedback.
    Effects,
    /// Capped world texture such as machinery and water.
    Ambience,
    /// The score, and sparse phrases from working bases.
    Music,
}

impl Bus {
    fn level(self, levels: &Levels) -> f32 {
        let value = match self {
            Self::Effects => levels.effects.load(Ordering::Relaxed),
            Self::Ambience => levels.ambience.load(Ordering::Relaxed),
            Self::Music => levels.music.load(Ordering::Relaxed),
        };
        // A perceptual taper: half the slider is about -12 dB.
        let x = f32::from(value) / 100.0;
        x * x
    }
}

/// Presentation cues emitted by the desktop layer.
///
/// The first seven names are retained for source compatibility with the
/// original desktop event mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Order,
    Shot,
    Impact,
    Complete,
    #[allow(dead_code)]
    Gate,
    Victory,
    Defeat,
    Draw,
    OrderSubmit,
    OrderAccepted,
    OrderRejected,
    CondenserBreath,
    UnionWinchClack,
    UnionDepositTap,
    AssemblyReed,
    WaterWash,
    UnionWorkPhrase,
    AssemblyWorkPhrase,
    GateWarningBell,
    GateChangeRush,
    /// The enemy has started capturing the sluice.
    SluiceCaptureAlarm,
    /// The sluice became yours.
    SluiceTaken,
    /// The enemy took the sluice.
    SluiceLost,
    /// A tide switch was cancelled by a machine at the station.
    SwitchCancelled,
    /// Your hold count started (or resumed).
    HoldBegunOurs,
    /// The enemy's hold count started: you lose when it runs out.
    HoldBegunEnemy,
    /// Your hold count stopped.
    HoldBrokenOurs,
    /// The enemy's hold count stopped.
    HoldBrokenEnemy,
    /// Your hold count passing a mark.
    HoldTollOurs,
    /// The enemy's hold count passing a mark.
    HoldTollEnemy,
    /// A Compact Raker or Works at work: a struck glass rod.
    CompactGlassChime,
    /// A Compact Raker setting down its load: a small glass pair.
    CompactDepositChime,
    /// The Compact's working base: a falling glass phrase.
    CompactWorkPhrase,
    /// A Heliostat's beam: a rising mirror hum, one per shot.
    HeliostatBeam,
    /// The Riveter's pneumatic gun.
    ShotRivet,
    /// The Bulwark's heavy gun.
    ShotCannon,
    /// Light guns: Sounders, Skippers.
    ShotLight,
    /// The Reedguard's dart.
    ShotReed,
    /// A Tower.
    ShotTower,
    /// The Brander's fire-lance.
    ShotLance,
    /// The Glinter's shard.
    ShotShard,
    /// A worker's tool.
    ShotTool,
    /// A Loom shell leaving.
    LoomLaunch,
    /// A Loom shell falling on its mark.
    ShellWhistle,
    /// A heavy machine destroyed.
    ExplosionLarge,
    /// A building destroyed.
    BuildingCollapse,
    /// A building site begun.
    BuildPlaced,
    /// A building finished.
    BuildDone,
    /// A machine trained.
    UnitReady,
    /// Research or an upgrade finished.
    ResearchDone,
    Vent,
    /// SOUND.
    Sonar,
    Glint,
    Surge,
    Deploy,
    /// Boarding or unloading a transport.
    Hatch,
    /// RECLAIM and recycling.
    Reclaim,
    Repair,
    /// A machine swamped in deep water.
    Splash,
    /// Crust melting.
    Sizzle,
    /// A salt causeway laid.
    SaltLaid,
    /// Our machines are under fire.
    UnderAttack,
    /// A menu control pressed.
    UiClick,
    /// Our machines selected, in their faction's voice.
    SelectUnion,
    SelectAssembly,
    SelectCompact,
    /// The match begins.
    MatchHorn,
    /// A seat is out.
    Eliminated,
    /// What breaks when a machine dies, by its builder.
    DebrisMetal,
    DebrisReed,
    DebrisGlass,
    /// Menus: starting something, and going back.
    UiConfirm,
    UiBack,
}

impl Cue {
    /// Returns the bus that carries this cue.
    pub fn bus(self) -> Bus {
        match self {
            Self::CondenserBreath
            | Self::UnionWinchClack
            | Self::UnionDepositTap
            | Self::AssemblyReed
            | Self::CompactGlassChime
            | Self::CompactDepositChime
            | Self::WaterWash => Bus::Ambience,
            Self::Victory
            | Self::Defeat
            | Self::Draw
            | Self::UnionWorkPhrase
            | Self::AssemblyWorkPhrase
            | Self::CompactWorkPhrase => Bus::Music,
            _ => Bus::Effects,
        }
    }

    /// Returns whether this cue belongs to the rapid-order suppression group.
    pub fn is_order(self) -> bool {
        matches!(
            self,
            Self::Order | Self::OrderSubmit | Self::OrderAccepted | Self::OrderRejected
        )
    }

    /// Cues with a pitch the music's key should carry: when the second
    /// match song (in E) plays, they move up a step with it.
    /// Tonal pings pitched above about 1 kHz.
    fn bright(self) -> bool {
        matches!(
            self,
            Self::Order
                | Self::OrderAccepted
                | Self::Complete
                | Self::BuildDone
                | Self::UnitReady
                | Self::ResearchDone
                | Self::SelectUnion
                | Self::SelectAssembly
                | Self::SelectCompact
                | Self::UiConfirm
        )
    }

    fn tonal(self) -> bool {
        matches!(
            self,
            Self::Order
                | Self::OrderAccepted
                | Self::Complete
                | Self::BuildDone
                | Self::UnitReady
                | Self::ResearchDone
                | Self::GateWarningBell
                | Self::SluiceTaken
                | Self::SluiceLost
                | Self::HoldBegunOurs
                | Self::HoldBegunEnemy
                | Self::HoldBrokenOurs
                | Self::HoldBrokenEnemy
                | Self::HoldTollOurs
                | Self::HoldTollEnemy
                | Self::SelectUnion
                | Self::SelectAssembly
                | Self::SelectCompact
                | Self::UiConfirm
                | Self::UnionWorkPhrase
                | Self::AssemblyWorkPhrase
                | Self::CompactWorkPhrase
        )
    }

    fn is_alert(self) -> bool {
        matches!(
            self,
            Self::GateWarningBell
                | Self::GateChangeRush
                | Self::SluiceCaptureAlarm
                | Self::SluiceTaken
                | Self::SluiceLost
                | Self::SwitchCancelled
                | Self::HoldBegunOurs
                | Self::HoldBegunEnemy
                | Self::HoldBrokenOurs
                | Self::HoldBrokenEnemy
                | Self::HoldTollOurs
                | Self::HoldTollEnemy
                | Self::UnderAttack
        )
    }

    fn priority(self) -> u8 {
        match self {
            cue if cue.is_alert() => 4,
            Self::Victory
            | Self::Defeat
            | Self::Draw
            | Self::UnionWorkPhrase
            | Self::AssemblyWorkPhrase
            | Self::CompactWorkPhrase => 2,
            cue if cue.bus() == Bus::Ambience => 0,
            // Small repeated clicks yield to everything.
            Self::ShotTool | Self::UiClick | Self::UiConfirm | Self::UiBack => 1,
            // Gunfire yields to deaths and news.
            Self::Shot
            | Self::ShotRivet
            | Self::ShotCannon
            | Self::ShotLight
            | Self::ShotReed
            | Self::ShotTower
            | Self::ShotLance
            | Self::ShotShard
            | Self::HeliostatBeam
            | Self::DebrisMetal
            | Self::DebrisReed
            | Self::DebrisGlass => 2,
            _ => 3,
        }
    }

    /// The patch this cue plays.
    pub fn patch(self) -> Patch {
        match self {
            Self::Order | Self::OrderAccepted => sfx::ORDER_ACCEPTED,
            Self::OrderSubmit => sfx::ORDER_SUBMIT,
            Self::OrderRejected => sfx::ORDER_REJECTED,
            Self::Shot => sfx::SHOT,
            Self::Impact => sfx::IMPACT,
            Self::Complete => sfx::BUILD_DONE,
            Self::Gate => sfx::GATE_RUSH,
            Self::Victory => sfx::VICTORY,
            Self::Defeat => sfx::DEFEAT,
            Self::Draw => sfx::DRAW,
            Self::CondenserBreath => sfx::CONDENSER_BREATH,
            Self::UnionWinchClack => sfx::WINCH_CLACK,
            Self::UnionDepositTap => sfx::DEPOSIT_TAP,
            Self::AssemblyReed => sfx::REED_BREATH,
            Self::WaterWash => sfx::WATER_WASH,
            Self::UnionWorkPhrase => sfx::UNION_PHRASE,
            Self::AssemblyWorkPhrase => sfx::ASSEMBLY_PHRASE,
            Self::GateWarningBell => sfx::GATE_BELL,
            Self::GateChangeRush => sfx::GATE_RUSH,
            Self::SluiceCaptureAlarm => sfx::CAPTURE_ALARM,
            Self::SluiceTaken => sfx::SLUICE_TAKEN,
            Self::SluiceLost => sfx::SLUICE_LOST,
            Self::SwitchCancelled => sfx::SWITCH_CANCELLED,
            Self::HoldBegunOurs => sfx::HOLD_BEGUN_OURS,
            Self::HoldBegunEnemy => sfx::HOLD_BEGUN_ENEMY,
            Self::HoldBrokenOurs => sfx::HOLD_BROKEN_OURS,
            Self::HoldBrokenEnemy => sfx::HOLD_BROKEN_ENEMY,
            Self::HoldTollOurs => sfx::HOLD_TOLL_OURS,
            Self::HoldTollEnemy => sfx::HOLD_TOLL_ENEMY,
            Self::CompactGlassChime => sfx::GLASS_CHIME,
            Self::CompactDepositChime => sfx::GLASS_PAIR,
            Self::CompactWorkPhrase => sfx::COMPACT_PHRASE,
            Self::HeliostatBeam => sfx::HELIOSTAT_BEAM,
            Self::ShotRivet => sfx::SHOT_RIVET,
            Self::ShotCannon => sfx::SHOT_CANNON,
            Self::ShotLight => sfx::SHOT_LIGHT,
            Self::ShotReed => sfx::SHOT_REED,
            Self::ShotTower => sfx::SHOT_TOWER,
            Self::ShotLance => sfx::SHOT_LANCE,
            Self::ShotShard => sfx::SHOT_SHARD,
            Self::ShotTool => sfx::SHOT_TOOL,
            Self::LoomLaunch => sfx::LOOM_LAUNCH,
            Self::ShellWhistle => sfx::SHELL_WHISTLE,
            Self::ExplosionLarge => sfx::EXPLOSION_LARGE,
            Self::BuildingCollapse => sfx::BUILDING_COLLAPSE,
            Self::BuildPlaced => sfx::BUILD_PLACED,
            Self::BuildDone => sfx::BUILD_DONE,
            Self::UnitReady => sfx::UNIT_READY,
            Self::ResearchDone => sfx::RESEARCH_DONE,
            Self::Vent => sfx::VENT,
            Self::Sonar => sfx::SONAR,
            Self::Glint => sfx::GLINT,
            Self::Surge => sfx::SURGE,
            Self::Deploy => sfx::DEPLOY,
            Self::Hatch => sfx::HATCH,
            Self::Reclaim => sfx::RECLAIM,
            Self::Repair => sfx::REPAIR,
            Self::Splash => sfx::SPLASH,
            Self::Sizzle => sfx::SIZZLE,
            Self::SaltLaid => sfx::SALT_LAID,
            Self::UnderAttack => sfx::UNDER_ATTACK,
            Self::UiClick => sfx::UI_CLICK,
            Self::SelectUnion => sfx::SELECT_UNION,
            Self::SelectAssembly => sfx::SELECT_ASSEMBLY,
            Self::SelectCompact => sfx::SELECT_COMPACT,
            Self::MatchHorn => sfx::MATCH_HORN,
            Self::Eliminated => sfx::ELIMINATED,
            Self::DebrisMetal => sfx::DEBRIS_METAL,
            Self::DebrisReed => sfx::DEBRIS_REED,
            Self::DebrisGlass => sfx::DEBRIS_GLASS,
            Self::UiConfirm => sfx::UI_CONFIRM,
            Self::UiBack => sfx::UI_BACK,
        }
    }

    /// Every cue, for reviews and tests.
    pub const ALL: [Cue; 73] = [
        Cue::Order,
        Cue::Shot,
        Cue::Impact,
        Cue::Complete,
        Cue::Gate,
        Cue::Victory,
        Cue::Defeat,
        Cue::Draw,
        Cue::OrderSubmit,
        Cue::OrderAccepted,
        Cue::OrderRejected,
        Cue::CondenserBreath,
        Cue::UnionWinchClack,
        Cue::UnionDepositTap,
        Cue::AssemblyReed,
        Cue::WaterWash,
        Cue::UnionWorkPhrase,
        Cue::AssemblyWorkPhrase,
        Cue::GateWarningBell,
        Cue::GateChangeRush,
        Cue::SluiceCaptureAlarm,
        Cue::SluiceTaken,
        Cue::SluiceLost,
        Cue::SwitchCancelled,
        Cue::HoldBegunOurs,
        Cue::HoldBegunEnemy,
        Cue::HoldBrokenOurs,
        Cue::HoldBrokenEnemy,
        Cue::HoldTollOurs,
        Cue::HoldTollEnemy,
        Cue::CompactGlassChime,
        Cue::CompactDepositChime,
        Cue::CompactWorkPhrase,
        Cue::HeliostatBeam,
        Cue::ShotRivet,
        Cue::ShotCannon,
        Cue::ShotLight,
        Cue::ShotReed,
        Cue::ShotTower,
        Cue::ShotLance,
        Cue::ShotShard,
        Cue::ShotTool,
        Cue::LoomLaunch,
        Cue::ShellWhistle,
        Cue::ExplosionLarge,
        Cue::BuildingCollapse,
        Cue::BuildPlaced,
        Cue::BuildDone,
        Cue::UnitReady,
        Cue::ResearchDone,
        Cue::Vent,
        Cue::Sonar,
        Cue::Glint,
        Cue::Surge,
        Cue::Deploy,
        Cue::Hatch,
        Cue::Reclaim,
        Cue::Repair,
        Cue::Splash,
        Cue::Sizzle,
        Cue::SaltLaid,
        Cue::UnderAttack,
        Cue::UiClick,
        Cue::SelectUnion,
        Cue::SelectAssembly,
        Cue::SelectCompact,
        Cue::MatchHorn,
        Cue::Eliminated,
        Cue::DebrisMetal,
        Cue::DebrisReed,
        Cue::DebrisGlass,
        Cue::UiConfirm,
        Cue::UiBack,
    ];
}

/// Owns only CPAL output state, bounded cue senders and the controls the
/// callback reads.
pub struct Audio {
    alerts_sender: SyncSender<PlayCommand>,
    effects_sender: SyncSender<PlayCommand>,
    ambience_sender: SyncSender<PlayCommand>,
    music_sender: SyncSender<PlayCommand>,
    muted: Arc<AtomicBool>,
    mute_epoch: Arc<AtomicU64>,
    hush_epoch: Arc<AtomicU64>,
    levels: Arc<Levels>,
    music: Arc<MusicControl>,
    ambience: Arc<AmbienceControl>,
    // Retaining the stream keeps CPAL's callback alive. It is intentionally not
    // inspected after construction.
    _stream: Stream,
}

/// The songs, built once.
pub fn soundtrack() -> Arc<Vec<Song>> {
    Arc::new(crate::score_songs::songs())
}

impl Audio {
    /// Opens the default output device and starts the output stream.
    ///
    /// No input device is queried. Devices whose default format is not one of
    /// CPAL's common `f32`, `i16`, or `u16` output formats receive a clear error;
    /// the caller can continue the match without constructing `Audio`.
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default audio output device is available".to_owned())?;
        let supported = device
            .default_output_config()
            .map_err(|error| format!("could not query default audio output: {error}"))?;
        let sample_format = supported.sample_format();
        let config = supported.config();
        let channels = usize::from(config.channels);
        if channels == 0 {
            return Err("audio output reported zero channels".to_owned());
        }
        let sample_rate = config.sample_rate;
        if sample_rate == 0 {
            return Err("audio output reported a zero sample rate".to_owned());
        }

        let (alerts_sender, alerts_receiver) = sync_channel(ALERT_COMMAND_CAPACITY);
        let (effects_sender, effects_receiver) = sync_channel(EFFECT_COMMAND_CAPACITY);
        let (ambience_sender, ambience_receiver) = sync_channel(AMBIENCE_COMMAND_CAPACITY);
        let (music_sender, music_receiver) = sync_channel(MUSIC_COMMAND_CAPACITY);
        let shared = Shared::new();
        let queues = Queues {
            alerts: alerts_receiver,
            effects: effects_receiver,
            ambience: ambience_receiver,
            music: music_receiver,
        };
        let callback_shared = shared.clone();
        let songs = soundtrack();
        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                config,
                queues,
                callback_shared,
                songs,
                sample_rate,
                channels,
            ),
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                config,
                queues,
                callback_shared,
                songs,
                sample_rate,
                channels,
            ),
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                config,
                queues,
                callback_shared,
                songs,
                sample_rate,
                channels,
            ),
            other => Err(format!(
                "default audio output format {other} is unsupported; expected f32, i16, or u16"
            )),
        }?;

        stream
            .play()
            .map_err(|error| format!("could not start audio output stream: {error}"))?;
        Ok(Self {
            alerts_sender,
            effects_sender,
            ambience_sender,
            music_sender,
            muted: shared.muted,
            mute_epoch: shared.mute_epoch,
            hush_epoch: shared.hush_epoch,
            levels: shared.levels,
            music: shared.music,
            ambience: shared.ambience,
            _stream: stream,
        })
    }

    /// Queues a centered cue without blocking the simulation or UI thread.
    pub fn play(&self, cue: Cue) {
        self.play_spatial(cue, 0.0, 1.0);
    }

    /// Queues a cue with a clamped screen pan (`-1` left, `1` right) and gain.
    ///
    /// Effects, ambience, and music use separate bounded queues. The callback
    /// drains tactical effects first, so a busy ambient scene cannot displace a
    /// warning or order acknowledgement before it reaches the mixer.
    pub fn play_spatial(&self, cue: Cue, pan: f32, gain: f32) {
        self.play_placed(cue, pan, gain, false);
    }

    /// A cue just past the edge of the view: quieter and dulled, so the
    /// player hears a fight nearby without it standing in front.
    pub fn play_far(&self, cue: Cue, pan: f32, gain: f32) {
        self.play_placed(cue, pan, gain, true);
    }

    /// A centred cue moved `semis` semitones, at `gain`.
    pub fn play_pitched(&self, cue: Cue, semis: i8, gain: f32) {
        self.send(cue, 0.0, gain, false, semis);
    }

    fn play_placed(&self, cue: Cue, pan: f32, gain: f32, far: bool) {
        self.send(cue, pan, gain, far, 0);
    }

    fn send(&self, cue: Cue, pan: f32, gain: f32, far: bool, semis: i8) {
        if self.muted.load(Ordering::Relaxed) {
            return;
        }
        let command = PlayCommand {
            cue,
            pan: sanitize_pan(pan),
            gain: sanitize_gain(gain),
            far,
            semis,
            mute_epoch: self.mute_epoch.load(Ordering::Acquire),
            hush_epoch: self.hush_epoch.load(Ordering::Acquire),
        };
        let sender = if cue.is_alert() {
            &self.alerts_sender
        } else {
            match cue.bus() {
                Bus::Effects => &self.effects_sender,
                Bus::Ambience => &self.ambience_sender,
                Bus::Music => &self.music_sender,
            }
        };
        let _ = sender.try_send(command);
    }

    /// Sets effects, ambience, and music buses in the inclusive `0..=100`
    /// range. Out-of-range values are clamped so settings input cannot amplify
    /// a bus past its limiter.
    pub fn set_levels(&self, effects: u8, ambience: u8, music: u8) {
        self.levels
            .effects
            .store(effects.min(100), Ordering::Relaxed);
        self.levels
            .ambience
            .store(ambience.min(100), Ordering::Relaxed);
        self.levels.music.store(music.min(100), Ordering::Relaxed);
    }

    /// What the score should do; see `MusicControl::set`.
    pub fn set_music(&self, state: MusicState, restart: bool) {
        self.music.set(state, restart);
    }

    /// The song the music engine is playing.
    pub fn playing_song(&self) -> Option<SongId> {
        self.music.playing()
    }

    /// The world bed: its level, how much sea, how much wind (0..=100).
    pub fn set_ambience(&self, level: u8, water: u8, wind: u8) {
        self.ambience.set(level, water, wind);
    }

    /// Mutes future and currently playing cues without stopping the stream.
    /// Queued commands carry the mute generation and are discarded after a
    /// mute transition, including a quick mute/unmute before the next callback.
    pub fn set_muted(&self, muted: bool) {
        let was_muted = self.muted.swap(muted, Ordering::AcqRel);
        if was_muted != muted {
            self.mute_epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Drops active effect tails and invalidates queued cues for a scene
    /// transition. The music crosses over by itself.
    pub fn hush(&self) {
        self.hush_epoch.fetch_add(1, Ordering::AcqRel);
    }
}

/// State the game and the callback share.
#[derive(Clone)]
struct Shared {
    muted: Arc<AtomicBool>,
    mute_epoch: Arc<AtomicU64>,
    hush_epoch: Arc<AtomicU64>,
    levels: Arc<Levels>,
    music: Arc<MusicControl>,
    ambience: Arc<AmbienceControl>,
}

impl Shared {
    fn new() -> Self {
        Self {
            muted: Arc::new(AtomicBool::new(false)),
            mute_epoch: Arc::new(AtomicU64::new(0)),
            hush_epoch: Arc::new(AtomicU64::new(0)),
            levels: Arc::new(Levels::with_values(100, 100, 100)),
            music: Arc::new(MusicControl::new()),
            ambience: Arc::new(AmbienceControl::default()),
        }
    }
}

struct Queues {
    alerts: Receiver<PlayCommand>,
    effects: Receiver<PlayCommand>,
    ambience: Receiver<PlayCommand>,
    music: Receiver<PlayCommand>,
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    queues: Queues,
    shared: Shared,
    songs: Arc<Vec<Song>>,
    sample_rate: u32,
    channels: usize,
) -> Result<Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let mut mixer = Mixer::new(queues, shared, songs, sample_rate as f32, channels);
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| mixer.fill(output),
            move |error| eprintln!("BRINEWAKE audio stream error: {error}"),
            None,
        )
        .map_err(|error| format!("could not build audio output stream: {error}"))
}

#[derive(Clone, Copy)]
struct PlayCommand {
    cue: Cue,
    pan: f32,
    gain: f32,
    far: bool,
    semis: i8,
    mute_epoch: u64,
    hush_epoch: u64,
}

#[derive(Default)]
struct Levels {
    effects: AtomicU8,
    ambience: AtomicU8,
    music: AtomicU8,
}

impl Levels {
    fn with_values(effects: u8, ambience: u8, music: u8) -> Self {
        Self {
            effects: AtomicU8::new(effects.min(100)),
            ambience: AtomicU8::new(ambience.min(100)),
            music: AtomicU8::new(music.min(100)),
        }
    }
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    cue: Cue,
    bus: Bus,
    priority: u8,
    pan: f32,
    gain: f32,
    reverb: f32,
    player: Player,
    age: u64,
    /// Far voices pass through a dull lowpass.
    far: bool,
    dull: crate::synth::OnePole,
    /// Samples left of a hush fade, when fading.
    fading: Option<u32>,
    /// The mixer clock when it began.
    started: u64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            cue: Cue::Order,
            bus: Bus::Effects,
            priority: 0,
            pan: 0.0,
            gain: 1.0,
            reverb: 0.0,
            player: Player::default(),
            age: 0,
            far: false,
            dull: crate::synth::OnePole::default(),
            fading: None,
            started: 0,
        }
    }
}

struct Mixer {
    queues: Queues,
    muted: Arc<AtomicBool>,
    mute_epoch: Arc<AtomicU64>,
    hush_epoch: Arc<AtomicU64>,
    levels: Arc<Levels>,
    ambience_control: Arc<AmbienceControl>,
    sample_rate: f32,
    channels: usize,
    voices: [Voice; MAX_VOICES],
    active_count: usize,
    ambient_count: usize,
    next_age: u64,
    sample_clock: u64,
    seen_mute_epoch: u64,
    seen_hush_epoch: u64,
    last_order: Option<Cue>,
    last_order_frame: u64,
    music: MusicEngine,
    ambience: Ambience,
    reverb: Reverb,
    master: Master,
    duck: f32,
    /// The effects bus loses its sub-bass; the reverb only hears above
    /// 200 Hz.
    fx_hp: [crate::synth::OnePole; 2],
    verb_hp: crate::synth::OnePole,
    /// Samples left of the fade into a mute.
    mute_fade: u32,
    was_muted: bool,
    /// Voices replaced while sounding, fading out.
    tails: [Voice; 4],
    /// The music loses anything under 40 Hz.
    music_hp: [crate::synth::OnePole; 2],
    /// Counts for reviews: voices taken over, tails used, the most voices
    /// at once.
    stats: MixStats,
    /// The other effects' level under an alert.
    fx_duck: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct MixStats {
    steals: u64,
    tails: u64,
    peak_voices: usize,
}

impl Mixer {
    fn new(
        queues: Queues,
        shared: Shared,
        songs: Arc<Vec<Song>>,
        sample_rate: f32,
        channels: usize,
    ) -> Self {
        let sample_rate = sample_rate.max(1.0);
        Self {
            queues,
            muted: shared.muted,
            mute_epoch: shared.mute_epoch,
            hush_epoch: shared.hush_epoch,
            levels: shared.levels,
            ambience_control: shared.ambience,
            sample_rate,
            channels,
            voices: [Voice::default(); MAX_VOICES],
            active_count: 0,
            ambient_count: 0,
            next_age: 0,
            sample_clock: 0,
            seen_mute_epoch: 0,
            seen_hush_epoch: 0,
            last_order: None,
            last_order_frame: 0,
            music: MusicEngine::new(songs, shared.music, sample_rate),
            ambience: Ambience::new(),
            reverb: Reverb::new(sample_rate),
            master: Master::new(sample_rate),
            duck: 1.0,
            fx_hp: [crate::synth::OnePole::default(); 2],
            verb_hp: crate::synth::OnePole::default(),
            mute_fade: 0,
            was_muted: false,
            tails: [Voice::default(); 4],
            music_hp: [crate::synth::OnePole::default(); 2],
            stats: MixStats::default(),
            fx_duck: 1.0,
        }
    }

    fn fill<T>(&mut self, output: &mut [T])
    where
        T: Sample + FromSample<f32>,
    {
        let channels = self.channels.max(1);
        let (muted, transitioned) = self.sync_state();
        if muted || transitioned {
            // Drain through `accept` instead of blindly clearing the queues:
            // commands tagged before a transition are dropped, while a new
            // result/order sent immediately after `hush` is retained.
            self.drain_all_commands();
        } else {
            self.drain_commands();
        }

        if muted && self.mute_fade == 0 {
            let silent_frames = output.chunks(channels).count() as u64;
            for sample in output.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            // The sample clock advances through silence as well. Otherwise a
            // stream of repeated order cues could remain suppressed forever
            // after the previous voice ended.
            self.sample_clock = self.sample_clock.wrapping_add(silent_frames);
            return;
        }

        let fade_len = self.fade_samples();
        for frame in output.chunks_mut(channels) {
            let (mut left, mut right) = self.next_frame();
            if muted {
                // Into a mute over a moment, not a click.
                let g = self.mute_fade as f32 / fade_len as f32;
                left *= g;
                right *= g;
                self.mute_fade = self.mute_fade.saturating_sub(1);
                if self.mute_fade == 0 {
                    self.clear_voices();
                    self.clear_bus();
                }
            }
            let mono = 0.70710677 * (left + right);
            let center = 0.5 * (left + right);
            for (index, sample) in frame.iter_mut().enumerate() {
                let value = if channels == 1 {
                    mono.clamp(-MAX_OUTPUT, MAX_OUTPUT)
                } else if index == 0 {
                    left
                } else if index == 1 {
                    right
                } else {
                    center
                };
                *sample = T::from_sample(value);
            }
            self.sample_clock = self.sample_clock.wrapping_add(1);
        }
    }

    /// Applies asynchronous transitions before draining any queue. Exact epoch
    /// tags on commands handle a mute or hush racing with this callback.
    fn sync_state(&mut self) -> (bool, bool) {
        let mut transitioned = false;
        let muted = self.muted.load(Ordering::Acquire);
        let mute_epoch = self.mute_epoch.load(Ordering::Acquire);
        if mute_epoch != self.seen_mute_epoch {
            if muted && !self.was_muted {
                self.mute_fade = self.fade_samples();
            } else {
                self.mute_fade = 0;
                self.clear_voices();
                self.clear_bus();
            }
            self.seen_mute_epoch = mute_epoch;
            transitioned = true;
        }
        self.was_muted = muted;
        let hush_epoch = self.hush_epoch.load(Ordering::Acquire);
        if hush_epoch != self.seen_hush_epoch {
            self.fade_voices();
            self.seen_hush_epoch = hush_epoch;
            transitioned = true;
        }
        (muted, transitioned)
    }

    fn drain(&mut self, which: u8, limit: usize) {
        for _ in 0..limit {
            let queue = match which {
                0 => &self.queues.alerts,
                1 => &self.queues.effects,
                2 => &self.queues.music,
                _ => &self.queues.ambience,
            };
            match queue.try_recv() {
                Ok(command) => self.accept(command),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn drain_commands(&mut self) {
        self.drain(0, MAX_DRAIN_ALERTS);
        self.drain(1, MAX_DRAIN_EFFECTS);
        self.drain(2, MAX_DRAIN_MUSIC);
        self.drain(3, MAX_DRAIN_AMBIENCE);
    }

    fn drain_all_commands(&mut self) {
        self.drain(0, ALERT_COMMAND_CAPACITY);
        self.drain(1, EFFECT_COMMAND_CAPACITY);
        self.drain(2, MUSIC_COMMAND_CAPACITY);
        self.drain(3, AMBIENCE_COMMAND_CAPACITY);
    }

    fn accept(&mut self, command: PlayCommand) {
        if self.muted.load(Ordering::Acquire)
            || command.mute_epoch != self.mute_epoch.load(Ordering::Acquire)
            || command.hush_epoch != self.hush_epoch.load(Ordering::Acquire)
        {
            return;
        }
        self.start_placed(
            command.cue,
            command.pan,
            command.gain,
            command.far,
            command.semis,
        );
    }

    #[cfg(test)]
    fn start(&mut self, cue: Cue, pan: f32, gain: f32) {
        self.start_placed(cue, pan, gain, false, 0);
    }

    fn start_placed(&mut self, cue: Cue, pan: f32, gain: f32, far: bool, semis: i8) {
        let order_suppression_frames =
            (self.sample_rate * ORDER_SUPPRESSION_MS as f32 / 1000.0).max(1.0) as u64;
        if cue.is_order()
            && self.last_order == Some(cue)
            && self.sample_clock.saturating_sub(self.last_order_frame) < order_suppression_frames
        {
            return;
        }
        if cue.is_order() {
            self.last_order = Some(cue);
            self.last_order_frame = self.sample_clock;
        }

        if cue.bus() == Bus::Ambience && self.ambient_count >= MAX_AMBIENT_VOICES {
            return;
        }
        // A cue already sounding four times replaces its own oldest.
        let mut count = 0usize;
        let mut oldest: Option<usize> = None;
        let mut recent = false;
        let recent_frames = (self.sample_rate * 0.012) as u64;
        for (index, voice) in self.voices.iter().enumerate() {
            if voice.active && voice.cue == cue && voice.fading.is_none() {
                count += 1;
                if oldest.is_none_or(|o| voice.age < self.voices[o].age) {
                    oldest = Some(index);
                }
                if self.sample_clock.saturating_sub(voice.started) < recent_frames {
                    recent = true;
                }
            }
        }
        let slot = if count >= MAX_PER_CUE {
            self.stats.steals += 1;
            oldest
        } else {
            self.select_slot(cue.priority())
        };
        // Each further copy of a cue already sounding is 2 dB quieter, so a
        // volley thickens without piling up. Alerts keep their level: a
        // count's toll is the clock.
        let gain = if cue.priority() >= 4 {
            gain
        } else {
            gain * 0.8f32.powi(count.min(MAX_PER_CUE) as i32)
        };
        let Some(slot) = slot else {
            return;
        };
        if self.voices[slot].active {
            if count < MAX_PER_CUE {
                self.stats.steals += 1;
            }
            if self.voices[slot].bus == Bus::Ambience {
                self.ambient_count = self.ambient_count.saturating_sub(1);
            }
            // The voice it replaces fades over 5 ms in a tail slot
            // instead of stopping dead.
            if let Some(tail) = self.tails.iter_mut().find(|t| !t.active) {
                *tail = self.voices[slot];
                tail.fading = Some(((self.sample_rate * 0.005) as u32).max(1));
                self.stats.tails += 1;
            }
        } else {
            self.active_count += 1;
        }
        self.next_age = self.next_age.wrapping_add(1);
        let patch = cue.patch();
        let voice = &mut self.voices[slot];
        voice.active = true;
        voice.cue = cue;
        voice.bus = cue.bus();
        voice.priority = cue.priority();
        voice.pan = sanitize_pan(pan);
        voice.gain = sanitize_gain(gain);
        voice.reverb = patch.reverb;
        voice.age = self.next_age;
        voice.far = far;
        voice.dull = crate::synth::OnePole::default();
        voice.fading = None;
        voice.started = self.sample_clock;
        let seed = self.next_age as u32 ^ (cue as u32).wrapping_mul(0x045d_9f3b);
        // Copies of one cue in the same moment spread over 30 ms rather
        // than stacking on one sample.
        let delay = if recent {
            0.004 + 0.026 * ((seed.wrapping_mul(2_654_435_761) >> 16) as f32 / 65_535.0)
        } else {
            0.0
        };
        // In a song pitched up a fourth or more, the bright pings go down
        // a fifth instead of up a fourth.
        let key = match (cue.tonal(), self.music.key()) {
            (false, _) => 0,
            (true, k) if k > 2 && cue.bright() => k - 12,
            (true, k) => k,
        };
        let key = f32::from(key);
        voice
            .player
            .start(patch, seed, delay, f32::from(semis) + key);
        if cue.bus() == Bus::Ambience {
            self.ambient_count += 1;
        }
    }

    fn select_slot(&self, incoming_priority: u8) -> Option<usize> {
        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .find(|(_, voice)| !voice.active)
        {
            return Some(index);
        }
        self.voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.active && voice.priority <= incoming_priority)
            .min_by_key(|(_, voice)| (voice.priority, voice.age))
            .map(|(index, _)| index)
    }

    fn fade_samples(&self) -> u32 {
        ((self.sample_rate * FADE_MS / 1000.0) as u32).max(1)
    }

    /// Every sounding effect fades out over a few milliseconds.
    fn fade_voices(&mut self) {
        let n = self.fade_samples();
        for voice in &mut self.voices {
            if voice.active && voice.fading.is_none() {
                voice.fading = Some(n);
            }
        }
        self.last_order = None;
    }

    /// The bus's memory (reverb, filters, limiter) back to silence.
    fn clear_bus(&mut self) {
        self.reverb.clear();
        self.master.clear();
        self.fx_hp = [crate::synth::OnePole::default(); 2];
        self.verb_hp = crate::synth::OnePole::default();
    }

    fn clear_voices(&mut self) {
        for voice in self.voices.iter_mut().chain(self.tails.iter_mut()) {
            voice.active = false;
            voice.fading = None;
            voice.player = Player::default();
        }
        self.active_count = 0;
        self.ambient_count = 0;
        self.last_order = None;
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let sr = self.sample_rate;
        let mut left = 0.0;
        let mut right = 0.0;
        let mut send = 0.0;
        let mut duck_depth: f32 = 1.0;
        let clock = self.sample_clock;
        let duck_frames = (ALERT_DUCK_SECONDS * sr) as u64;
        if self.active_count > 0 {
            let mut active_count = self.active_count;
            let mut ambient_count = self.ambient_count;
            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }
                let bus = voice.bus;
                let trim = match bus {
                    Bus::Effects => EFFECTS_GAIN,
                    Bus::Ambience => EFFECTS_GAIN * 0.8,
                    Bus::Music => EFFECTS_GAIN * 0.8,
                };
                let mut sample =
                    voice.player.next(sr) * voice.gain * trim * bus.level(&self.levels);
                if voice.far {
                    sample = voice.dull.lowpass(sample, 2_000.0, sr);
                }
                // Under an alert the fight steps back too.
                if voice.priority < 4 {
                    sample *= self.fx_duck;
                }
                if let Some(left_samples) = voice.fading {
                    sample *= left_samples as f32 / (sr * FADE_MS / 1000.0).max(1.0);
                    voice.fading = Some(left_samples.saturating_sub(1));
                    if left_samples <= 1 {
                        voice.player = Player::default();
                    }
                }
                // For its first 1.5 s an alert dips the music: a count's
                // toll and the tide's own bells and rush a little, real news
                // more (and only news steps the fight back).
                if voice.priority >= 4 && clock.wrapping_sub(voice.started) < duck_frames {
                    let depth = match voice.cue {
                        Cue::HoldTollOurs
                        | Cue::HoldTollEnemy
                        | Cue::GateWarningBell
                        | Cue::GateChangeRush => 0.8,
                        _ => DUCK_DEPTH,
                    };
                    duck_depth = duck_depth.min(depth);
                }
                let (left_gain, right_gain) = pan_gains(voice.pan);
                left += sample * left_gain;
                right += sample * right_gain;
                send += sample * voice.reverb;
                if !voice.player.active() {
                    voice.fading = None;
                    voice.active = false;
                    active_count = active_count.saturating_sub(1);
                    if bus == Bus::Ambience {
                        ambient_count = ambient_count.saturating_sub(1);
                    }
                }
            }
            self.active_count = active_count;
            self.ambient_count = ambient_count;
            self.stats.peak_voices = self.stats.peak_voices.max(active_count);
        }

        for tail in &mut self.tails {
            if !tail.active {
                continue;
            }
            let Some(left_samples) = tail.fading else {
                tail.active = false;
                continue;
            };
            let n = ((sr * 0.005) as u32).max(1);
            let trim = match tail.bus {
                Bus::Effects => EFFECTS_GAIN,
                Bus::Ambience | Bus::Music => EFFECTS_GAIN * 0.8,
            };
            let mut sample = tail.player.next(sr) * tail.gain * trim * tail.bus.level(&self.levels);
            if tail.far {
                sample = tail.dull.lowpass(sample, 2_000.0, sr);
            }
            let sample = sample * left_samples as f32 / n as f32;
            let (lg, rg) = pan_gains(tail.pan);
            left += sample * lg;
            right += sample * rg;
            send += sample * tail.reverb;
            if left_samples <= 1 {
                tail.active = false;
                tail.player = Player::default();
            } else {
                tail.fading = Some(left_samples - 1);
            }
        }

        // No sub-bass rumble from the effects: small speakers cannot play
        // it and it pumps the master.
        left -= self.fx_hp[0].lowpass(left, 35.0, sr);
        right -= self.fx_hp[1].lowpass(right, 35.0, sr);

        // The fight steps back 6 dB under an alert (a news cue, not a
        // count's toll) and returns over 0.4 s.
        let fx_target = if duck_depth < 0.7 { 0.5 } else { 1.0 };
        let fx_rate = if fx_target < self.fx_duck {
            1.0 / (0.02 * sr)
        } else {
            1.0 / (0.4 * sr)
        };
        self.fx_duck += (fx_target - self.fx_duck).clamp(-fx_rate, fx_rate);

        // The score steps back under an alert and returns slowly.
        let duck_target = duck_depth;
        let rate = if duck_target < self.duck {
            1.0 / (0.05 * sr)
        } else {
            1.0 / (0.8 * sr)
        };
        self.duck += (duck_target - self.duck).clamp(-rate, rate);

        let music_level = Bus::Music.level(&self.levels) * MUSIC_GAIN * self.duck;
        let (ml, mr, mv) = self.music.next();
        let ml = ml - self.music_hp[0].lowpass(ml, 40.0, sr);
        let mr = mr - self.music_hp[1].lowpass(mr, 40.0, sr);
        left += ml * music_level;
        right += mr * music_level;
        send += mv * music_level;

        let ambience_level = Bus::Ambience.level(&self.levels) * AMBIENCE_GAIN;
        let (al, ar, av) = self.ambience.next(&self.ambience_control, sr);
        left += al * ambience_level;
        right += ar * ambience_level;
        send += av * ambience_level;

        let send = send - self.verb_hp.lowpass(send, 200.0, sr);
        let (rl, rr) = self.reverb.process(send, send, sr);
        left += rl * REVERB_RETURN;
        right += rr * REVERB_RETURN;
        self.master.process(left, right)
    }
}

fn sanitize_pan(pan: f32) -> f32 {
    if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(0.0, 2.0)
    } else {
        1.0
    }
}

fn pan_gains(pan: f32) -> (f32, f32) {
    let position = 0.5 * (sanitize_pan(pan) + 1.0);
    ((1.0 - position).sqrt(), position.sqrt())
}

// ---------------------------------------------------------------------
// Reviews: deterministic renders through the same mixer, no device.

/// Exports a short, deterministic harbor sound sketch through the same Mixer
/// used by the CPAL callback. No device is opened and no external samples are
/// read.
pub fn export_review(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|error| format!("could not create audio review directory: {error}"))?;

    let normal = render_review(Levels::with_values(75, 45, 35))?;
    let quiet = render_review(Levels::with_values(42, 25, 20))?;
    write_wav(
        &dir.join("harbor-normal.wav"),
        &normal.samples,
        normal.sample_rate,
        2,
    )?;
    write_wav(
        &dir.join("harbor-quiet.wav"),
        &quiet.samples,
        quiet.sample_rate,
        2,
    )?;
    let tide = render_timeline(
        Levels::with_values(75, 45, 35),
        &TIDE_TIMELINE,
        TIDE_REVIEW_DURATION_MS,
    )?;
    write_wav(
        &dir.join("tide-cues.wav"),
        &tide.samples,
        tide.sample_rate,
        2,
    )?;
    let compact = render_timeline(
        Levels::with_values(75, 45, 35),
        &COMPACT_TIMELINE,
        COMPACT_REVIEW_DURATION_MS,
    )?;
    write_wav(
        &dir.join("compact-cues.wav"),
        &compact.samples,
        compact.sample_rate,
        2,
    )?;

    let rendered: Vec<String> = HARBOR_TIMELINE
        .iter()
        .map(|(_, cue, ..)| format!("\"{cue:?}\""))
        .collect();
    let available: Vec<String> = Cue::ALL.iter().map(|cue| format!("\"{cue:?}\"")).collect();
    let metadata = format!(
        "{{\n  \"format\": \"BRINEWAKE harbor audio review\",\n  \"source\": \"crates/bw_desktop/src/audio.rs, sfx.rs\",\n  \"synthesis\": \"original deterministic procedural synthesis; no recorded samples\",\n  \"sample_rate\": {},\n  \"channels\": 2,\n  \"duration_ms\": {},\n  \"normal\": {{\"file\": \"harbor-normal.wav\", \"effects\": 75, \"ambience\": 45, \"music\": 35, \"peak\": {:.6}, \"rms\": {:.6}}},\n  \"quiet\": {{\"file\": \"harbor-quiet.wav\", \"effects\": 42, \"ambience\": 25, \"music\": 20, \"peak\": {:.6}, \"rms\": {:.6}}},\n  \"compact\": {{\"file\": \"compact-cues.wav\", \"duration_ms\": {}, \"cues\": [\"CompactGlassChime\", \"CompactDepositChime\", \"CompactWorkPhrase\", \"HeliostatBeam\"], \"peak\": {:.6}, \"rms\": {:.6}}},\n  \"cues_rendered\": [{}],\n  \"available_cues\": [{}]\n}}\n",
        normal.sample_rate,
        REVIEW_DURATION_MS,
        normal.peak,
        normal.rms,
        quiet.peak,
        quiet.rms,
        COMPACT_REVIEW_DURATION_MS,
        compact.peak,
        compact.rms,
        rendered.join(", "),
        available.join(", "),
    );
    fs::write(dir.join("harbor-metadata.json"), metadata)
        .map_err(|error| format!("could not write audio review metadata: {error}"))?;
    Ok(())
}

const REVIEW_SAMPLE_RATE: u32 = 48_000;
const REVIEW_DURATION_MS: u32 = 15_000;
const REVIEW_BLOCK_FRAMES: usize = 256;

struct RenderedReview {
    samples: Vec<i16>,
    sample_rate: u32,
    peak: f32,
    rms: f32,
}

fn render_review(levels: Levels) -> Result<RenderedReview, String> {
    render_timeline(levels, &HARBOR_TIMELINE, REVIEW_DURATION_MS)
}

/// The harbor sketch: work, a gate change, a skirmish, and the result.
const HARBOR_TIMELINE: [(usize, Cue, f32, f32); 17] = [
    (320, Cue::CondenserBreath, -0.55, 0.82),
    (1_200, Cue::UnionWinchClack, -0.35, 0.90),
    (2_050, Cue::UnionDepositTap, -0.22, 0.90),
    (2_750, Cue::AssemblyReed, 0.45, 0.78),
    (3_100, Cue::UnionWorkPhrase, -0.24, 0.62),
    (3_350, Cue::AssemblyWorkPhrase, 0.28, 0.58),
    (5_000, Cue::GateWarningBell, 0.0, 0.90),
    (6_200, Cue::GateChangeRush, 0.0, 0.90),
    (7_100, Cue::WaterWash, 0.10, 0.86),
    (10_050, Cue::OrderSubmit, -0.05, 0.80),
    (10_120, Cue::OrderAccepted, -0.05, 0.80),
    (10_520, Cue::Shot, -0.40, 0.85),
    (11_000, Cue::Impact, -0.25, 0.90),
    (11_520, Cue::OrderRejected, 0.30, 0.85),
    (12_300, Cue::WaterWash, 0.0, 0.68),
    (13_000, Cue::Victory, 0.0, 0.68),
    (14_100, Cue::Draw, 0.0, 0.62),
];

const COMPACT_REVIEW_DURATION_MS: u32 = 9_000;

/// The Compact's harbor: two Rakers at work, a deposit, the base's
/// phrase, then a Heliostat burning one target for eight shots.
const COMPACT_TIMELINE: [(usize, Cue, f32, f32); 13] = [
    (300, Cue::CompactGlassChime, -0.40, 0.85),
    (1_100, Cue::CompactGlassChime, 0.35, 0.80),
    (1_900, Cue::CompactDepositChime, -0.20, 0.90),
    (2_800, Cue::CompactWorkPhrase, 0.0, 0.70),
    (4_600, Cue::HeliostatBeam, 0.10, 0.60),
    (5_000, Cue::HeliostatBeam, 0.10, 0.66),
    (5_400, Cue::HeliostatBeam, 0.10, 0.72),
    (5_800, Cue::HeliostatBeam, 0.10, 0.78),
    (6_200, Cue::HeliostatBeam, 0.10, 0.84),
    (6_600, Cue::HeliostatBeam, 0.10, 0.90),
    (7_000, Cue::HeliostatBeam, 0.10, 0.95),
    (7_400, Cue::HeliostatBeam, 0.10, 1.0),
    (8_100, Cue::CompactGlassChime, 0.0, 0.70),
];

const TIDE_REVIEW_DURATION_MS: u32 = 26_000;

/// Every tide cue in the order a match might play them.
const TIDE_TIMELINE: [(usize, Cue, f32, f32); 19] = [
    (300, Cue::SluiceCaptureAlarm, 0.0, 1.0),
    (1_600, Cue::SluiceLost, 0.0, 1.0),
    (3_300, Cue::SluiceTaken, 0.0, 1.0),
    (5_000, Cue::SwitchCancelled, 0.0, 1.0),
    (6_800, Cue::HoldBegunOurs, 0.0, 1.0),
    (8_300, Cue::HoldTollOurs, 0.0, 1.0),
    (9_300, Cue::HoldTollOurs, 0.0, 1.0),
    (10_300, Cue::HoldTollOurs, 0.0, 1.0),
    (11_500, Cue::HoldBrokenOurs, 0.0, 1.0),
    (13_300, Cue::HoldBegunEnemy, 0.0, 1.0),
    (15_300, Cue::HoldTollEnemy, 0.0, 1.0),
    (17_300, Cue::HoldTollEnemy, 0.0, 1.0),
    (18_300, Cue::HoldTollEnemy, 0.0, 1.0),
    (19_300, Cue::HoldTollEnemy, 0.0, 1.0),
    (20_800, Cue::HoldBrokenEnemy, 0.0, 1.0),
    (22_300, Cue::GateWarningBell, 0.0, 1.0),
    (23_300, Cue::GateWarningBell, 0.0, 1.0),
    (24_100, Cue::GateChangeRush, 0.0, 1.0),
    (24_900, Cue::WaterWash, 0.0, 0.6),
];

/// A scripted scene for a review render: cues, and changes to the music
/// and ambience at given milliseconds.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Cue(Cue, f32, f32),
    Music(MusicState),
    Ambience(u8, u8, u8),
}

/// Renders a scene through the real mixer.
struct Scene {
    mixer: Mixer,
    senders: [SyncSender<PlayCommand>; 4],
    music: Arc<MusicControl>,
    ambience: Arc<AmbienceControl>,
    lenient: bool,
}

impl Scene {
    fn new(levels: Levels, songs: Arc<Vec<Song>>) -> Self {
        let (a_tx, a_rx) = sync_channel(ALERT_COMMAND_CAPACITY);
        let (e_tx, e_rx) = sync_channel(EFFECT_COMMAND_CAPACITY);
        let (m_tx, m_rx) = sync_channel(MUSIC_COMMAND_CAPACITY);
        let (b_tx, b_rx) = sync_channel(AMBIENCE_COMMAND_CAPACITY);
        let mut shared = Shared::new();
        shared.levels = Arc::new(levels);
        let music = Arc::clone(&shared.music);
        let ambience = Arc::clone(&shared.ambience);
        let queues = Queues {
            alerts: a_rx,
            effects: e_rx,
            ambience: b_rx,
            music: m_rx,
        };
        Self {
            mixer: Mixer::new(queues, shared, songs, REVIEW_SAMPLE_RATE as f32, 2),
            senders: [a_tx, e_tx, m_tx, b_tx],
            music,
            ambience,
            lenient: false,
        }
    }

    fn apply(&self, step: Step) -> Result<(), String> {
        match step {
            Step::Cue(cue, pan, gain) => {
                let command = PlayCommand {
                    cue,
                    pan: sanitize_pan(pan),
                    gain: sanitize_gain(gain),
                    far: false,
                    semis: 0,
                    mute_epoch: 0,
                    hush_epoch: 0,
                };
                let sender = if cue.is_alert() {
                    &self.senders[0]
                } else {
                    match cue.bus() {
                        Bus::Effects => &self.senders[1],
                        Bus::Music => &self.senders[2],
                        Bus::Ambience => &self.senders[3],
                    }
                };
                match sender.try_send(command) {
                    Err(_) if self.lenient => Ok(()),
                    result => result
                        .map_err(|error| format!("could not queue review cue {cue:?}: {error}")),
                }
            }
            Step::Music(state) => {
                self.music.set(state, false);
                Ok(())
            }
            Step::Ambience(level, water, wind) => {
                self.ambience.set(level, water, wind);
                Ok(())
            }
        }
    }

    fn render_with_stats(
        self,
        steps: &[(usize, Step)],
        duration_ms: u32,
    ) -> Result<(RenderedReview, MixStats, f32), String> {
        let mut scene = self;
        let review = scene.render_mut(steps, duration_ms)?;
        Ok((review, scene.mixer.stats, scene.mixer.master.deepest()))
    }

    fn render(
        mut self,
        steps: &[(usize, Step)],
        duration_ms: u32,
    ) -> Result<RenderedReview, String> {
        self.render_mut(steps, duration_ms)
    }

    /// As `render`, but a cue the queues cannot take is dropped (a trailer
    /// fires volleys faster than a match).
    fn render_lenient(
        mut self,
        steps: &[(usize, Step)],
        duration_ms: u32,
    ) -> Result<RenderedReview, String> {
        self.lenient = true;
        self.render_mut(steps, duration_ms)
    }

    fn render_mut(
        &mut self,
        steps: &[(usize, Step)],
        duration_ms: u32,
    ) -> Result<RenderedReview, String> {
        let total_frames = REVIEW_SAMPLE_RATE as usize * duration_ms as usize / 1000;
        let mut output = Vec::with_capacity(total_frames * 2);
        let mut frame = 0usize;
        let mut index = 0usize;
        let mut peak = 0.0f32;
        let mut sum_squares = 0.0f64;
        let mut block = vec![0.0f32; REVIEW_BLOCK_FRAMES * 2];
        while frame < total_frames {
            while index < steps.len()
                && frame >= steps[index].0 * REVIEW_SAMPLE_RATE as usize / 1000
            {
                self.apply(steps[index].1)?;
                index += 1;
            }
            let frames = REVIEW_BLOCK_FRAMES.min(total_frames - frame);
            let block = &mut block[..frames * 2];
            self.mixer.fill(block);
            for &sample in block.iter() {
                let value = sample.clamp(-1.0, 1.0);
                peak = peak.max(value.abs());
                sum_squares += f64::from(value) * f64::from(value);
                output.push((value * f32::from(i16::MAX)) as i16);
            }
            frame += frames;
        }
        let rms = (sum_squares / output.len().max(1) as f64).sqrt() as f32;
        Ok(RenderedReview {
            samples: output,
            sample_rate: REVIEW_SAMPLE_RATE,
            peak,
            rms,
        })
    }
}

fn render_timeline(
    levels: Levels,
    timeline: &[(usize, Cue, f32, f32)],
    duration_ms: u32,
) -> Result<RenderedReview, String> {
    let steps: Vec<(usize, Step)> = timeline
        .iter()
        .map(|&(at, cue, pan, gain)| (at, Step::Cue(cue, pan, gain)))
        .collect();
    Scene::new(levels, Arc::new(Vec::new())).render(&steps, duration_ms)
}

/// Renders the whole soundtrack and every effect for review: each song
/// at each intensity, a scripted match that climbs and falls, the
/// results, the effects one by one, the ambience, and a battle montage.
/// Writes WAVs, a cue sheet, and each song's score report.
pub fn export_soundtrack(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| format!("could not create {dir:?}: {error}"))?;
    let songs = soundtrack();
    let full = || Levels::with_values(100, 100, 100);
    // The shipped defaults (ux.rs).
    let game = || Levels::with_values(75, 65, 60);
    let music = |song: SongId, intensity: u8| MusicState {
        song: Some(song),
        intensity,
        ..MusicState::default()
    };
    let mut sheet = String::from("{\n");
    let mut write = |name: &str, review: RenderedReview, note: &str| -> Result<(), String> {
        write_wav(&dir.join(name), &review.samples, review.sample_rate, 2)?;
        sheet.push_str(&format!(
            "  \"{name}\": {{\"peak\": {:.4}, \"rms\": {:.4}, \"note\": \"{note}\"}},\n",
            review.peak, review.rms
        ));
        Ok(())
    };

    for song in songs.iter() {
        let report = crate::score::report(song);
        fs::write(
            dir.join(format!(
                "score-{}.md",
                song.name.to_lowercase().replace(' ', "-")
            )),
            report,
        )
        .map_err(|error| format!("could not write score report: {error}"))?;
    }

    let title_ms = (songs
        .iter()
        .find(|s| s.id == SongId::Title)
        .map_or(60.0, |s| s.seconds())
        * 1000.0) as u32
        + 4_000;
    let r = Scene::new(full(), Arc::clone(&songs))
        .render(&[(0, Step::Music(music(SongId::Title, 0)))], title_ms)?;
    write(
        "music-title.wav",
        r,
        "the title theme, once through and into the loop",
    )?;

    let match_ms = (songs
        .iter()
        .find(|s| s.id == SongId::Match)
        .map_or(60.0, |s| s.seconds())
        * 1000.0) as u32;
    for intensity in 0..=3u8 {
        let r = Scene::new(full(), Arc::clone(&songs)).render(
            &[(0, Step::Music(music(SongId::Match, intensity)))],
            match_ms,
        )?;
        write(
            &format!("music-match-i{intensity}.wav"),
            r,
            "the match song held at one intensity",
        )?;
    }
    // The same song at the shipped default levels, for loudness targets.
    for intensity in [1u8, 3] {
        let r = Scene::new(game(), Arc::clone(&songs)).render(
            &[
                (0, Step::Music(music(SongId::Match, intensity))),
                (0, Step::Ambience(100, 55, 45)),
            ],
            60_000,
        )?;
        write(
            &format!("defaults-match-i{intensity}.wav"),
            r,
            "match song with the bed at the default levels (75/65/60)",
        )?;
    }
    let r = Scene::new(game(), Arc::new(Vec::new()))
        .render(&[(0, Step::Ambience(100, 55, 45))], 30_000)?;
    write(
        "defaults-ambience.wav",
        r,
        "the bed alone at the default ambience level",
    )?;
    // A whole count, second by second from 90 to 0, at intensity 2: the
    // enemy's escalates at 60 and 30 seconds and empties at 10.
    // With the tolls the game plays: every ten seconds, and each of the
    // last ten (the enemy's) or five (ours).
    // The enemy's count is also rendered in the other two match songs'
    // keys (E and G), music only at the defaults.
    for (name, ours, theirs, song) in [
        ("ours", true, false, SongId::Match),
        ("theirs", false, true, SongId::Match),
        ("theirs-confluence", false, true, SongId::Confluence),
        ("theirs-undertow", false, true, SongId::Undertow),
    ] {
        let mut steps = Vec::new();
        for second in 0..=90u32 {
            let left = (90 - second).max(1);
            steps.push((
                second as usize * 1_000,
                Step::Music(MusicState {
                    song: Some(song),
                    intensity: 2,
                    hold_ours: ours,
                    hold_theirs: theirs,
                    hold_left: left as u8,
                    ..MusicState::default()
                }),
            ));
            if second > 0 && (left.is_multiple_of(10) || left <= if ours { 5 } else { 10 }) {
                let toll = if ours {
                    Cue::HoldTollOurs
                } else {
                    Cue::HoldTollEnemy
                };
                steps.push((second as usize * 1_000 + 1, Step::Cue(toll, 0.0, 1.0)));
            }
        }
        if song == SongId::Match {
            let r = Scene::new(full(), Arc::clone(&songs)).render(&steps, 92_000)?;
            write(
                &format!("music-match-hold-{name}.wav"),
                r,
                "a 90-second hold count at intensity 2 with its tolls, seconds left counting down",
            )?;
        }
        let quiet: Vec<(usize, Step)> = steps
            .iter()
            .copied()
            .filter(|(_, s)| matches!(s, Step::Music(_)))
            .collect();
        let r = Scene::new(game(), Arc::clone(&songs)).render(&quiet, 92_000)?;
        write(
            &format!("defaults-hold-{name}-music.wav"),
            r,
            "the same count at the default levels, music only (no tolls)",
        )?;
    }
    let confluence_ms = (songs
        .iter()
        .find(|s| s.id == SongId::Confluence)
        .map_or(64.0, |s| s.seconds())
        * 1000.0) as u32;
    for intensity in 0..=3u8 {
        let r = Scene::new(full(), Arc::clone(&songs)).render(
            &[(0, Step::Music(music(SongId::Confluence, intensity)))],
            confluence_ms,
        )?;
        write(
            &format!("music-confluence-i{intensity}.wav"),
            r,
            "the second match song, once through, held at one intensity",
        )?;
    }
    let r = Scene::new(game(), Arc::clone(&songs)).render(
        &[(0, Step::Music(music(SongId::Confluence, 1)))],
        2 * confluence_ms + 6_000,
    )?;
    write(
        "music-confluence-two-passes.wav",
        r,
        "Confluence twice through at intensity 1 (the second pass differs), at defaults",
    )?;
    let undertow_ms = (songs
        .iter()
        .find(|s| s.id == SongId::Undertow)
        .map_or(115.0, |s| s.seconds())
        * 1000.0) as u32;
    for intensity in 0..=3u8 {
        let r = Scene::new(full(), Arc::clone(&songs)).render(
            &[(0, Step::Music(music(SongId::Undertow, intensity)))],
            undertow_ms,
        )?;
        write(
            &format!("music-undertow-i{intensity}.wav"),
            r,
            "the third match song, once through, held at one intensity",
        )?;
    }
    let r = Scene::new(game(), Arc::clone(&songs)).render(
        &[(0, Step::Music(music(SongId::Undertow, 1)))],
        2 * undertow_ms + 20_000,
    )?;
    write(
        "music-undertow-rotation.wav",
        r,
        "Undertow twice through at intensity 1 (the second pass differs), then it hands over to Reclamation, at defaults",
    )?;
    let r = Scene::new(game(), Arc::clone(&songs)).render(
        &[(0, Step::Music(music(SongId::Confluence, 1)))],
        2 * confluence_ms + 20_000,
    )?;
    write(
        "music-confluence-rotation.wav",
        r,
        "Confluence twice through at intensity 1, then it hands over to Undertow, at defaults",
    )?;
    // An enemy count broken with six seconds left.
    let mut broken = Vec::new();
    for second in 0..=84u32 {
        broken.push((
            second as usize * 1_000,
            Step::Music(MusicState {
                song: Some(SongId::Match),
                intensity: 2,
                hold_theirs: true,
                hold_left: (90 - second) as u8,
                ..MusicState::default()
            }),
        ));
    }
    broken.push((84_500, Step::Music(music(SongId::Match, 2))));
    broken.push((84_500, Step::Cue(Cue::HoldBrokenEnemy, 0.0, 1.0)));
    let r = Scene::new(game(), Arc::clone(&songs)).render(&broken, 95_000)?;
    write(
        "music-hold-broken.wav",
        r,
        "an enemy count broken with six seconds left, at defaults: the race must stop at once",
    )?;
    // The music rests after a long calm and comes back on the other song.
    let rest = vec![
        (0usize, Step::Music(music(SongId::Match, 1))),
        (
            20_000,
            Step::Music(MusicState {
                song: None,
                stop: crate::music::Stop::Phrase,
                ..MusicState::default()
            }),
        ),
        (45_000, Step::Music(music(SongId::Confluence, 1))),
    ];
    let r = Scene::new(game(), Arc::clone(&songs)).render(&rest, 70_000)?;
    write(
        "music-rest.wav",
        r,
        "working at defaults; a rest asked at 20 s ends the phrase; Confluence at 45 s",
    )?;
    for intensity in [1u8, 3] {
        let back = vec![
            (0usize, Step::Music(music(SongId::Match, 1))),
            (
                10_000,
                Step::Music(MusicState {
                    song: None,
                    stop: crate::music::Stop::Phrase,
                    ..MusicState::default()
                }),
            ),
            (30_000, Step::Music(music(SongId::Confluence, intensity))),
        ];
        let r = Scene::new(game(), Arc::clone(&songs)).render(&back, 60_000)?;
        write(
            &format!("music-rest-back-i{intensity}.wav"),
            r,
            "a rest, then Confluence returning at 30 s: at i1 four calm bars first, at i3 straight in on the battle entry's hit",
        )?;
    }
    // Twice through Reclamation at intensity 1, then the hand-over.
    let r = Scene::new(game(), Arc::clone(&songs))
        .render(&[(0, Step::Music(music(SongId::Match, 1)))], 300_000)?;
    write(
        "music-rotation.wav",
        r,
        "Reclamation at intensity 1 twice through (274 s), then it hands over to Confluence",
    )?;
    let r = Scene::new(game(), Arc::clone(&songs))
        .render(&[(0, Step::Music(music(SongId::Match, 2)))], 290_000)?;
    write(
        "music-rotation-i2.wav",
        r,
        "Reclamation at intensity 2 twice through (274 s), then it hands over to Confluence at intensity 2",
    )?;
    for (variant, name) in [(1u8, "assembly"), (2u8, "compact")] {
        let state = MusicState {
            song: Some(SongId::Match),
            intensity: 3,
            variant,
            ..MusicState::default()
        };
        let r =
            Scene::new(full(), Arc::clone(&songs)).render(&[(0, Step::Music(state))], 36_000)?;
        write(
            &format!("music-match-lead-{name}.wav"),
            r,
            "the fighting lead in another faction's voice",
        )?;
    }

    // A match that climbs and falls, with the pause menu in the middle.
    let mut dynamic = vec![(0usize, Step::Music(music(SongId::Match, 0)))];
    for (at, i) in [
        (20_000, 1u8),
        (45_000, 2),
        (60_000, 3),
        (90_000, 2),
        (105_000, 1),
    ] {
        dynamic.push((at, Step::Music(music(SongId::Match, i))));
    }
    dynamic.push((
        115_000,
        Step::Music(MusicState {
            muffled: true,
            ..music(SongId::Match, 1)
        }),
    ));
    dynamic.push((122_000, Step::Music(music(SongId::Match, 1))));
    dynamic.push((130_000, Step::Music(music(SongId::Victory, 1))));
    let r = Scene::new(game(), Arc::clone(&songs)).render(&dynamic, 160_000)?;
    write(
        "music-match-dynamic.wav",
        r,
        "0→1 at 20s, 2 at 45s, 3 at 60s, 2 at 90s, 1 at 105s, paused 115-122s, victory at 130s into the harbour",
    )?;

    for (id, name) in [
        (SongId::Victory, "victory"),
        (SongId::Defeat, "defeat"),
        (SongId::Draw, "draw"),
    ] {
        let r = Scene::new(full(), Arc::clone(&songs))
            .render(&[(0, Step::Music(music(id, 0)))], 30_000)?;
        write(
            &format!("music-{name}.wav"),
            r,
            "the jingle, then the harbour loop",
        )?;
    }

    // Every effect, a second and a half apart, with a cue sheet.
    let mut steps = Vec::new();
    let mut labels = String::from("[\n");
    let mut at = 300usize;
    for cue in Cue::ALL {
        steps.push((at, Step::Cue(cue, 0.0, 1.0)));
        labels.push_str(&format!(
            "  {{\"ms\": {at}, \"cue\": \"{cue:?}\", \"length_ms\": {}}},\n",
            (cue.patch().length() * 1000.0) as u32
        ));
        at += 600 + (cue.patch().length() * 1000.0) as usize;
    }
    labels.push_str("  {}\n]\n");
    fs::write(dir.join("sfx-gallery.json"), labels)
        .map_err(|error| format!("could not write gallery sheet: {error}"))?;
    let r = Scene::new(full(), Arc::new(Vec::new())).render(&steps, at as u32 + 1_000)?;
    write(
        "sfx-gallery.wav",
        r,
        "every cue at full effects level, labelled in sfx-gallery.json",
    )?;

    for (name, water, wind) in [("neutral", 55u8, 45u8), ("flood", 100, 60), ("dry", 20, 70)] {
        let r = Scene::new(full(), Arc::new(Vec::new()))
            .render(&[(0, Step::Ambience(100, water, wind))], 45_000)?;
        write(
            &format!("ambience-{name}.wav"),
            r,
            "the world bed alone at full ambience level",
        )?;
    }

    // A dense fight: 25 shots a second from six guns, two deaths a second,
    // three big explosions, the alarm and an enemy count starting.
    let mut dense = vec![
        (0usize, Step::Music(music(SongId::Match, 3))),
        (0, Step::Ambience(100, 55, 45)),
    ];
    let guns = [
        Cue::ShotRivet,
        Cue::ShotReed,
        Cue::ShotLight,
        Cue::ShotCannon,
        Cue::ShotLance,
        Cue::ShotShard,
    ];
    for k in 0..500usize {
        let at = 500 + k * 40 + (k * 17) % 23;
        let pan = ((k * 29) % 19) as f32 / 9.5 - 1.0;
        dense.push((at, Step::Cue(guns[k % guns.len()], pan * 0.75, 0.9)));
        if k % 12 == 5 {
            dense.push((
                at + 15,
                Step::Cue(
                    if k % 24 == 5 {
                        Cue::Impact
                    } else {
                        Cue::ExplosionLarge
                    },
                    pan * 0.6,
                    1.0,
                ),
            ));
        }
    }
    for at in [4_000usize, 9_000, 15_000] {
        dense.push((at, Step::Cue(Cue::ExplosionLarge, 0.0, 1.0)));
    }
    dense.push((6_000, Step::Cue(Cue::UnderAttack, 0.0, 1.0)));
    dense.push((12_000, Step::Cue(Cue::HoldBegunEnemy, 0.0, 1.0)));
    dense.sort_by_key(|(at, _)| *at);
    let scene = Scene::new(game(), Arc::clone(&songs));
    let (r, stats, limit) = scene.render_with_stats(&dense, 21_000)?;
    write(
        "montage-dense.wav",
        r,
        &format!(
            "25 shots a second at defaults; peak voices {}, voices taken over {}, tails {}, deepest limiter gain {:.2}",
            stats.peak_voices, stats.steals, stats.tails, limit
        ),
    )?;

    // A fight in context: music at the game's default levels, the bed,
    // and a skirmish's worth of effects.
    let r = Scene::new(game(), Arc::clone(&songs)).render(&montage(), 40_000)?;
    write(
        "montage-battle.wav",
        r,
        "match music climbing to battle under a skirmish at default levels",
    )?;

    sheet.push_str("  \"_\": {}\n}\n");
    fs::write(dir.join("soundtrack-review.json"), sheet)
        .map_err(|error| format!("could not write review sheet: {error}"))?;
    Ok(())
}

/// A skirmish: work, a sighting, a fight with every gun, losses, a
/// sluice change, a hold.
fn montage() -> Vec<(usize, Step)> {
    let music = |intensity: u8, hold_ours: bool| {
        Step::Music(MusicState {
            song: Some(SongId::Match),
            intensity,
            hold_ours,
            ..MusicState::default()
        })
    };
    let mut steps = vec![(0, music(1, false)), (0, Step::Ambience(100, 55, 45))];
    let work = [
        (800, Cue::UnionWinchClack, -0.4),
        (1_900, Cue::UnionDepositTap, -0.3),
        (2_600, Cue::BuildPlaced, 0.2),
        (3_400, Cue::OrderSubmit, 0.0),
        (3_420, Cue::OrderAccepted, 0.0),
        (4_800, Cue::UnitReady, 0.0),
        (5_600, Cue::SelectUnion, 0.0),
        (6_300, Cue::UnionWinchClack, 0.3),
    ];
    for (at, cue, pan) in work {
        steps.push((at, Step::Cue(cue, pan, 0.9)));
    }
    steps.push((8_000, music(2, false)));
    steps.push((9_000, Step::Cue(Cue::UnderAttack, 0.0, 1.0)));
    steps.push((10_500, music(3, false)));
    let guns = [
        Cue::ShotRivet,
        Cue::ShotReed,
        Cue::ShotLight,
        Cue::ShotCannon,
        Cue::ShotRivet,
        Cue::ShotLight,
        Cue::ShotReed,
    ];
    let mut t = 10_500usize;
    let mut k = 0usize;
    while t < 24_000 {
        let cue = guns[k % guns.len()];
        let pan = ((k * 37) % 17) as f32 / 8.5 - 1.0;
        steps.push((t, Step::Cue(cue, pan * 0.8, 0.8)));
        if k % 5 == 3 {
            steps.push((t + 120, Step::Cue(Cue::Impact, pan * 0.6, 0.9)));
        }
        t += 180 + (k * 53) % 260;
        k += 1;
    }
    steps.push((14_000, Step::Cue(Cue::LoomLaunch, 0.5, 0.9)));
    steps.push((15_600, Step::Cue(Cue::ShellWhistle, -0.2, 1.0)));
    steps.push((16_800, Step::Cue(Cue::ExplosionLarge, -0.2, 1.0)));
    steps.push((19_000, Step::Cue(Cue::Sonar, 0.3, 0.9)));
    steps.push((21_500, Step::Cue(Cue::BuildingCollapse, 0.4, 1.0)));
    steps.push((24_500, Step::Cue(Cue::GateWarningBell, 0.0, 1.0)));
    steps.push((26_500, Step::Cue(Cue::GateWarningBell, 0.0, 1.0)));
    steps.push((28_000, Step::Cue(Cue::GateChangeRush, 0.0, 1.0)));
    steps.push((28_000, Step::Ambience(100, 100, 55)));
    steps.push((30_000, music(2, false)));
    steps.push((31_000, Step::Cue(Cue::SluiceTaken, 0.0, 1.0)));
    steps.push((32_500, Step::Cue(Cue::HoldBegunOurs, 0.0, 1.0)));
    steps.push((32_500, music(2, true)));
    for (i, at) in [35_000usize, 36_000, 37_000, 38_000].iter().enumerate() {
        let _ = i;
        steps.push((*at, Step::Cue(Cue::HoldTollOurs, 0.0, 1.0)));
    }
    steps.sort_by_key(|(at, _)| *at);
    steps
}

/// Render a trailer's soundtrack through the real mixer from a JSON
/// timeline:
/// `{"duration": s, "levels": [effects, ambience, music], "steps": [...]}`,
/// each step `{"t": s, ...}` with one of `"music": {"song": "Match",
/// "intensity": 2, "hold_theirs": true, "hold_left": 12, "stop": "Phrase"}`
/// (`"song": null` stops it), `"cue": "ExplosionLarge", "gain": 1,
/// "pan": 0`, or `"ambience": [level, water, wind]`. Writes a 48 kHz WAV.
pub fn render_trailer_audio(timeline: &Path, out: &Path) -> Result<(), String> {
    use crate::music::{SongId, Stop};
    let text = fs::read_to_string(timeline).map_err(|e| format!("{}: {e}", timeline.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", timeline.display()))?;
    let duration = json["duration"]
        .as_f64()
        .ok_or("the timeline needs a duration")?;
    let level = |i: usize, d: u8| {
        json["levels"]
            .get(i)
            .and_then(|v| v.as_u64())
            .map_or(d, |v| v.min(100) as u8)
    };
    let levels = Levels::with_values(level(0, 100), level(1, 100), level(2, 100));
    let song = |name: &str| -> Option<SongId> {
        (0..=7)
            .filter_map(SongId::from_u8)
            .find(|s| format!("{s:?}") == name)
    };
    let mut steps: Vec<(usize, Step)> = Vec::new();
    for step in json["steps"].as_array().ok_or("the timeline needs steps")? {
        let at = (step["t"].as_f64().ok_or("every step needs t")? * 1000.0)
            .round()
            .max(0.0) as usize;
        if let Some(name) = step["cue"].as_str() {
            let cue = *Cue::ALL
                .iter()
                .find(|c| format!("{c:?}") == name)
                .ok_or(format!("unknown cue {name}"))?;
            let pan = step["pan"].as_f64().unwrap_or(0.0) as f32;
            let gain = step["gain"].as_f64().unwrap_or(1.0) as f32;
            steps.push((at, Step::Cue(cue, pan, gain)));
        } else if let Some(m) = step.get("music") {
            let state = MusicState {
                song: m["song"].as_str().and_then(song),
                intensity: m["intensity"].as_u64().unwrap_or(0).min(3) as u8,
                hold_ours: m["hold_ours"].as_bool().unwrap_or(false),
                hold_theirs: m["hold_theirs"].as_bool().unwrap_or(false),
                hold_left: m["hold_left"].as_u64().unwrap_or(0).min(250) as u8,
                stop: match m["stop"].as_str() {
                    Some("Quick") => Stop::Quick,
                    Some("Phrase") => Stop::Phrase,
                    _ => Stop::Gentle,
                },
                flood: m["flood"].as_bool().unwrap_or(false),
                muffled: false,
                variant: m["variant"].as_u64().unwrap_or(0).min(2) as u8,
            };
            steps.push((at, Step::Music(state)));
        } else if let Some(a) = step["ambience"].as_array() {
            let v = |i: usize| a.get(i).and_then(|x| x.as_u64()).unwrap_or(0).min(100) as u8;
            steps.push((at, Step::Ambience(v(0), v(1), v(2))));
        }
    }
    steps.sort_by_key(|(at, _)| *at);
    let scene = Scene::new(levels, soundtrack());
    let review = scene.render_lenient(&steps, (duration * 1000.0) as u32)?;
    write_wav(out, &review.samples, review.sample_rate, 2)?;
    println!(
        "wrote {} ({:.1} s, peak {:.3})",
        out.display(),
        duration,
        review.peak
    );
    Ok(())
}

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> Result<(), String> {
    if channels == 0 || !samples.len().is_multiple_of(usize::from(channels)) {
        return Err("audio review sample count does not match channel count".to_owned());
    }
    let data_bytes = samples
        .len()
        .checked_mul(std::mem::size_of::<i16>())
        .ok_or_else(|| "audio review is too large".to_owned())?;
    let riff_size = 36usize
        .checked_add(data_bytes)
        .ok_or_else(|| "audio review WAV size overflow".to_owned())?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .and_then(|rate| rate.checked_mul(2))
        .ok_or_else(|| "audio review byte rate overflow".to_owned())?;
    let block_align = channels
        .checked_mul(2)
        .ok_or_else(|| "audio review block alignment overflow".to_owned())?;
    let data_len =
        u32::try_from(data_bytes).map_err(|_| "audio review data is too large".to_owned())?;
    let riff_len =
        u32::try_from(riff_size).map_err(|_| "audio review RIFF is too large".to_owned())?;
    let mut bytes = Vec::with_capacity(44 + data_bytes);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_len.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let mut file =
        File::create(path).map_err(|error| format!("could not create {path:?}: {error}"))?;
    file.write_all(&bytes).map_err(io_error)
}

fn io_error(error: std::io::Error) -> String {
    format!("could not write audio review WAV: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mixer(
        channels: usize,
        levels: (u8, u8, u8),
    ) -> (
        Mixer,
        SyncSender<PlayCommand>,
        SyncSender<PlayCommand>,
        SyncSender<PlayCommand>,
    ) {
        let (_alerts_sender, alerts_receiver) = sync_channel(ALERT_COMMAND_CAPACITY);
        let (effects_sender, effects_receiver) = sync_channel(EFFECT_COMMAND_CAPACITY);
        let (ambience_sender, ambience_receiver) = sync_channel(AMBIENCE_COMMAND_CAPACITY);
        let (music_sender, music_receiver) = sync_channel(MUSIC_COMMAND_CAPACITY);
        let mut shared = Shared::new();
        shared.levels = Arc::new(Levels::with_values(levels.0, levels.1, levels.2));
        let queues = Queues {
            alerts: alerts_receiver,
            effects: effects_receiver,
            ambience: ambience_receiver,
            music: music_receiver,
        };
        let mixer = Mixer::new(queues, shared, Arc::new(Vec::new()), 48_000.0, channels);
        (mixer, effects_sender, ambience_sender, music_sender)
    }

    fn command(cue: Cue, pan: f32, gain: f32) -> PlayCommand {
        PlayCommand {
            cue,
            pan: sanitize_pan(pan),
            gain: sanitize_gain(gain),
            far: false,
            semis: 0,
            mute_epoch: 0,
            hush_epoch: 0,
        }
    }

    fn alert_mixer() -> (Mixer, SyncSender<PlayCommand>, SyncSender<PlayCommand>) {
        let (alerts_sender, alerts_receiver) = sync_channel(ALERT_COMMAND_CAPACITY);
        let (effects_sender, effects_receiver) = sync_channel(EFFECT_COMMAND_CAPACITY);
        let (_ambience_sender, ambience_receiver) = sync_channel(AMBIENCE_COMMAND_CAPACITY);
        let (_music_sender, music_receiver) = sync_channel(MUSIC_COMMAND_CAPACITY);
        let queues = Queues {
            alerts: alerts_receiver,
            effects: effects_receiver,
            ambience: ambience_receiver,
            music: music_receiver,
        };
        let mixer = Mixer::new(queues, Shared::new(), Arc::new(Vec::new()), 48_000.0, 2);
        (mixer, alerts_sender, effects_sender)
    }

    #[test]
    fn cue_patches_are_finite_bounded_and_distinct() {
        for cue in Cue::ALL {
            let patch = cue.patch();
            assert!(
                patch.gain.is_finite() && patch.gain > 0.0 && patch.gain <= 0.3,
                "{cue:?} gain"
            );
            assert!(
                !patch.parts.is_empty() && patch.parts.len() <= sfx::MAX_PARTS,
                "{cue:?} parts"
            );
            let length = patch.length();
            assert!(length > 0.0 && length <= 7.0, "{cue:?} length {length}");
            for part in patch.parts {
                assert!(part.f0.is_finite() && part.f1.is_finite() && part.gain.is_finite());
                assert!(part.decay > 0.0 && part.attack >= 0.0);
            }
            // Rendered alone at full level each stays under the ceiling.
            let (mut mixer, _, _, _) = test_mixer(2, (100, 100, 100));
            mixer.start(cue, 0.0, 1.0);
            let mut output = vec![0.0f32; 48_000 * 2];
            mixer.fill(&mut output);
            assert!(
                output
                    .iter()
                    .all(|s| s.is_finite() && s.abs() <= MAX_OUTPUT + 1e-6),
                "{cue:?} bounded"
            );
            assert!(output.iter().any(|s| s.abs() > 0.001), "{cue:?} audible");
        }
        assert_ne!(Cue::OrderSubmit.patch(), Cue::OrderRejected.patch());
        assert_ne!(Cue::GateWarningBell.patch(), Cue::GateChangeRush.patch());
    }

    #[test]
    fn mixer_output_is_finite_and_clamped_for_mono_stereo_and_surround() {
        for channels in [1, 2, 6] {
            let (mut mixer, effects, ambience, music) = test_mixer(channels, (100, 100, 100));
            effects
                .try_send(command(Cue::GateWarningBell, -0.7, 1.0))
                .unwrap();
            ambience
                .try_send(command(Cue::CondenserBreath, 0.7, 1.0))
                .unwrap();
            music.try_send(command(Cue::Draw, 0.0, 1.0)).unwrap();
            let mut output = vec![0.0f32; 2_048 * channels];
            mixer.fill(&mut output);
            assert!(output.iter().all(|sample| sample.is_finite()));
            assert!(
                output
                    .iter()
                    .all(|sample| sample.abs() <= MAX_OUTPUT + f32::EPSILON)
            );
            assert!(output.iter().any(|sample| sample.abs() > 0.0001));
        }
    }

    #[test]
    fn cues_finish_within_their_declared_length() {
        let (mut mixer, _, _, _) = test_mixer(1, (100, 100, 100));
        mixer.start(Cue::Order, 0.0, 1.0);
        let frames = (Cue::Order.patch().length() * 48_000.0) as usize + 1_000;
        let mut output = vec![0.0f32; frames];
        mixer.fill(&mut output);
        assert_eq!(mixer.active_count, 0);
    }

    #[test]
    fn muted_mixer_discards_pending_cues_and_outputs_silence() {
        let (mut mixer, effects, _, _) = test_mixer(2, (100, 100, 100));
        effects.try_send(command(Cue::Shot, 0.0, 1.0)).unwrap();
        mixer.muted.store(true, Ordering::Relaxed);
        mixer.mute_epoch.fetch_add(1, Ordering::Relaxed);
        let mut output = vec![1.0f32; 128];
        mixer.fill(&mut output);
        assert!(output.iter().all(|sample| *sample == 0.0));
        assert_eq!(mixer.active_count, 0);
    }

    #[test]
    fn muting_active_voice_drops_tail_before_unmute() {
        let (mut mixer, _, _, _) = test_mixer(2, (100, 100, 100));
        mixer.start(Cue::Impact, 0.0, 1.0);
        assert!(mixer.active_count > 0);

        mixer.muted.store(true, Ordering::Relaxed);
        mixer.mute_epoch.fetch_add(1, Ordering::Relaxed);
        let mut muted_output = vec![1.0f32; 48_000];
        mixer.fill(&mut muted_output);
        // A 15 ms fade (720 frames, 1,440 samples) and then silence.
        assert!(muted_output[1_500..].iter().all(|sample| *sample == 0.0));
        assert_eq!(mixer.active_count, 0);

        mixer.muted.store(false, Ordering::Relaxed);
        mixer.mute_epoch.fetch_add(1, Ordering::Relaxed);
        let mut unmuted_output = vec![1.0f32; 48_000];
        mixer.fill(&mut unmuted_output);
        assert!(unmuted_output.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn bus_levels_can_silence_one_bus_without_affecting_others() {
        let (mut mixer, effects, ambience, _) = test_mixer(2, (0, 100, 100));
        effects.try_send(command(Cue::Shot, 0.0, 1.0)).unwrap();
        ambience
            .try_send(command(Cue::WaterWash, 0.0, 1.0))
            .unwrap();
        let mut output = vec![0.0f32; 1_024 * 2];
        mixer.fill(&mut output);
        assert!(output.iter().any(|sample| sample.abs() > 0.0001));

        let (mut effects_only, effects_sender, ambience_sender, _) = test_mixer(2, (100, 0, 100));
        effects_sender
            .try_send(command(Cue::Shot, 0.0, 1.0))
            .unwrap();
        ambience_sender
            .try_send(command(Cue::WaterWash, 0.0, 1.0))
            .unwrap();
        let mut effects_output = vec![0.0f32; 1_024 * 2];
        effects_only.fill(&mut effects_output);
        assert!(effects_output.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    fn tactical_voice_steals_ambient_before_another_tactical_voice() {
        let (mut mixer, _, _, _) = test_mixer(2, (100, 100, 100));
        for index in 0..MAX_AMBIENT_VOICES {
            mixer.start(
                if index % 2 == 0 {
                    Cue::CondenserBreath
                } else {
                    Cue::WaterWash
                },
                0.0,
                1.0,
            );
        }
        assert_eq!(mixer.ambient_count, MAX_AMBIENT_VOICES);
        // Four of each gun: the per-cue cap is four.
        let guns = [Cue::Shot, Cue::ShotRivet, Cue::ShotLight, Cue::ShotReed];
        for index in 0..(MAX_VOICES - MAX_AMBIENT_VOICES) {
            mixer.start(guns[index % guns.len()], 0.0, 1.0);
        }
        assert_eq!(mixer.active_count, MAX_VOICES);
        mixer.start(Cue::GateWarningBell, 0.0, 1.0);
        assert_eq!(mixer.active_count, MAX_VOICES);
        assert!(mixer.ambient_count < MAX_AMBIENT_VOICES);
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.cue == Cue::GateWarningBell)
        );
    }

    #[test]
    fn alert_queue_precedes_a_flooded_effect_queue() {
        let (mut mixer, alerts, effects) = alert_mixer();
        for _ in 0..EFFECT_COMMAND_CAPACITY {
            effects.try_send(command(Cue::Shot, 0.0, 1.0)).unwrap();
        }
        alerts
            .try_send(command(Cue::GateWarningBell, 0.0, 1.0))
            .unwrap();
        mixer.fill(&mut vec![0.0f32; 256 * 2]);
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.active && voice.cue == Cue::GateWarningBell)
        );
    }

    #[test]
    fn repeated_order_cues_are_suppressed_but_distinct_ack_is_audible() {
        let (mut mixer, effects, _, _) = test_mixer(2, (100, 100, 100));
        for _ in 0..8 {
            effects
                .try_send(command(Cue::OrderSubmit, 0.0, 1.0))
                .unwrap();
        }
        mixer.fill(&mut vec![0.0f32; 64 * 2]);
        assert_eq!(
            mixer
                .voices
                .iter()
                .filter(|voice| voice.active && voice.cue == Cue::OrderSubmit)
                .count(),
            1
        );

        effects
            .try_send(command(Cue::OrderAccepted, 0.0, 1.0))
            .unwrap();
        mixer.fill(&mut vec![0.0f32; 64 * 2]);
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.active && voice.cue == Cue::OrderAccepted)
        );

        effects
            .try_send(command(Cue::OrderRejected, 0.0, 1.0))
            .unwrap();
        mixer.fill(&mut vec![0.0f32; 64 * 2]);
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.active && voice.cue == Cue::OrderRejected)
        );
    }

    #[test]
    fn panning_and_gain_are_sanitized_and_directional() {
        let (mut left_mixer, left_sender, _, _) = test_mixer(2, (100, 100, 100));
        left_sender
            .try_send(command(Cue::Shot, -2.0, f32::NAN))
            .unwrap();
        let mut left_output = vec![0.0f32; 1_024 * 2];
        left_mixer.fill(&mut left_output);
        let left_peak = left_output
            .chunks_exact(2)
            .map(|frame| frame[0].abs())
            .fold(0.0, f32::max);
        let left_right_peak = left_output
            .chunks_exact(2)
            .map(|frame| frame[1].abs())
            .fold(0.0, f32::max);
        assert!(left_peak > left_right_peak);

        let (mut silent_mixer, silent_sender, _, _) = test_mixer(2, (100, 100, 100));
        silent_sender
            .try_send(command(Cue::Shot, 0.0, -4.0))
            .unwrap();
        let mut silent_output = vec![1.0f32; 256 * 2];
        silent_mixer.fill(&mut silent_output);
        assert!(silent_output.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn hush_drops_active_and_queued_tails() {
        let (mut mixer, effects, ambience, music) = test_mixer(2, (100, 100, 100));
        mixer.start(Cue::Impact, 0.0, 1.0);
        effects.try_send(command(Cue::Shot, 0.0, 1.0)).unwrap();
        ambience
            .try_send(command(Cue::WaterWash, 0.0, 1.0))
            .unwrap();
        let (mut unhushed, _, _, _) = test_mixer(2, (100, 100, 100));
        unhushed.start(Cue::Impact, 0.0, 1.0);
        mixer.hush_epoch.fetch_add(1, Ordering::Relaxed);
        let mut output = vec![1.0f32; 2_048 * 2];
        mixer.fill(&mut output);
        let mut reference = vec![0.0f32; 2_048 * 2];
        unhushed.fill(&mut reference);
        // The voice fades in 15 ms; after that only a little reverb is left.
        let energy = |x: &[f32]| x.iter().map(|v| v * v).sum::<f32>();
        assert!(energy(&output[1_600..]) < 0.05 * energy(&reference[1_600..]));
        assert_eq!(mixer.active_count, 0);

        let mut next_scene = command(Cue::Draw, 0.0, 1.0);
        next_scene.hush_epoch = 1;
        music.try_send(next_scene).unwrap();
        let mut next_output = vec![0.0f32; 512 * 2];
        mixer.fill(&mut next_output);
        assert!(next_output.iter().any(|sample| sample.abs() > 0.0001));
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.active && voice.cue == Cue::Draw)
        );
    }

    #[test]
    fn hush_resets_order_debounce_for_the_next_scene() {
        let (mut mixer, effects, _, _) = test_mixer(2, (100, 100, 100));
        mixer.start(Cue::OrderSubmit, 0.0, 1.0);
        mixer.hush_epoch.fetch_add(1, Ordering::Relaxed);
        mixer.fill(&mut vec![0.0f32; 32 * 2]);
        let mut next_order = command(Cue::OrderSubmit, 0.0, 1.0);
        next_order.hush_epoch = 1;
        effects.try_send(next_order).unwrap();
        mixer.fill(&mut vec![0.0f32; 32 * 2]);
        assert!(
            mixer
                .voices
                .iter()
                .any(|voice| voice.active && voice.cue == Cue::OrderSubmit)
        );
    }

    #[test]
    fn review_export_has_wav_headers_and_metadata() {
        let dir =
            std::env::temp_dir().join(format!("brinewake-audio-review-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        export_review(&dir).expect("review export");
        let wav = fs::read(dir.join("harbor-normal.wav")).expect("normal wav");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let metadata = fs::read_to_string(dir.join("harbor-metadata.json")).expect("metadata");
        assert!(metadata.contains("CondenserBreath"));
        assert!(metadata.contains("harbor-quiet.wav"));
        let tide = fs::read(dir.join("tide-cues.wav")).expect("tide cues wav");
        assert_eq!(&tide[0..4], b"RIFF");
        let compact = fs::read(dir.join("compact-cues.wav")).expect("compact cues wav");
        assert_eq!(&compact[0..4], b"RIFF");
        for cue in [
            "CompactGlassChime",
            "CompactDepositChime",
            "CompactWorkPhrase",
            "HeliostatBeam",
        ] {
            assert!(metadata.contains(cue), "{cue} is in the review");
        }
        assert!(metadata.contains("compact-cues.wav"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn compact_cues_are_their_own_sounds_and_ring_out() {
        // Glass (FM at a free bar's ratio), not the Assembly's reed or
        // the Union's metal.
        let glass = |cue: Cue| {
            cue.patch().parts.iter().all(
                |p| matches!(p.src, sfx::Src::Fm { ratio, .. } if (ratio - 2.756).abs() < 0.01),
            )
        };
        for cue in [
            Cue::CompactGlassChime,
            Cue::CompactDepositChime,
            Cue::CompactWorkPhrase,
        ] {
            assert!(glass(cue), "{cue:?} is glass");
        }
        for other in [
            Cue::AssemblyReed,
            Cue::AssemblyWorkPhrase,
            Cue::UnionWinchClack,
            Cue::UnionDepositTap,
            Cue::UnionWorkPhrase,
        ] {
            assert!(!glass(other), "{other:?} is not glass");
        }
        assert_eq!(Cue::CompactWorkPhrase.bus(), Bus::Music);
        assert_eq!(Cue::CompactGlassChime.bus(), Bus::Ambience);
        assert_eq!(Cue::HeliostatBeam.bus(), Bus::Effects);
        // The beam hums no longer than a Heliostat's 12-tick cooldown and
        // a little, so shots join into one rising tone rather than pile up.
        let cooldown_ms =
            bw_content::spec(bw_core::Kind::Heliostat).cooldown as f32 * 1000.0 / 30.0;
        let beam = Cue::HeliostatBeam.patch();
        let length_ms = beam.length() * 1000.0;
        assert!(
            length_ms >= cooldown_ms && length_ms <= cooldown_ms + 60.0,
            "{length_ms} vs {cooldown_ms}"
        );
        assert!(
            beam.parts[0].f1 > beam.parts[0].f0,
            "the beam rises as it focuses"
        );
        let compact = render_timeline(
            Levels::with_values(100, 100, 100),
            &COMPACT_TIMELINE,
            COMPACT_REVIEW_DURATION_MS,
        )
        .expect("compact render");
        assert!(compact.peak > 0.01 && compact.peak <= MAX_OUTPUT + f32::EPSILON);
        let chime = render_timeline(
            Levels::with_values(100, 100, 100),
            &[(0, Cue::CompactGlassChime, 0.0, 1.0)],
            600,
        )
        .unwrap();
        let energy = |range: std::ops::Range<usize>| -> f64 {
            chime.samples[range]
                .iter()
                .map(|s| f64::from(*s).powi(2))
                .sum()
        };
        let quarter = chime.samples.len() / 4;
        assert!(energy(0..quarter) > 4.0 * energy(3 * quarter..4 * quarter));
    }

    #[test]
    fn review_mix_is_repeatable_and_quiet_mix_is_lower() {
        let normal = render_review(Levels::with_values(75, 45, 35)).expect("normal render");
        let normal_again = render_review(Levels::with_values(75, 45, 35)).expect("repeat render");
        let quiet = render_review(Levels::with_values(42, 25, 20)).expect("quiet render");
        assert_eq!(normal.samples, normal_again.samples);
        assert!(quiet.rms < normal.rms);
        assert!(quiet.peak < normal.peak);
    }

    #[test]
    fn the_score_plays_through_the_mixer_and_ducks_under_alerts() {
        let songs = soundtrack();
        let scene = Scene::new(Levels::with_values(100, 100, 100), songs);
        let state = MusicState {
            song: Some(SongId::Match),
            intensity: 3,
            ..MusicState::default()
        };
        let r = scene
            .render(&[(0, Step::Music(state))], 6_000)
            .expect("render");
        assert!(r.rms > 0.01, "the match song is audible: {}", r.rms);
        assert!(r.peak <= MAX_OUTPUT + 1e-6);
    }

    /// The enemy count's last ten tolls, one a second while earlier ones
    /// still ring, each strike as loud as the first (within 1 dB).
    #[test]
    fn the_last_tolls_keep_their_level() {
        let steps: Vec<(usize, Step)> = (0..10)
            .map(|k| (k * 1_000, Step::Cue(Cue::HoldTollEnemy, 0.0, 1.0)))
            .collect();
        let r = Scene::new(Levels::with_values(100, 100, 100), Arc::new(Vec::new()))
            .render(&steps, 10_500)
            .expect("render");
        let sr = r.sample_rate as usize;
        // The loudest 20 ms in the first 150 ms of each strike.
        let peaks: Vec<f32> = (0..10)
            .map(|k| {
                let a = k * sr;
                let w = sr / 50;
                (a..a + sr * 3 / 20 - w)
                    .step_by(w / 2)
                    .map(|i| {
                        let seg = &r.samples[i * 2..(i + w) * 2];
                        (seg.iter().map(|&s| f32::from(s).powi(2)).sum::<f32>() / seg.len() as f32)
                            .sqrt()
                    })
                    .fold(0.0, f32::max)
            })
            .collect();
        for (k, p) in peaks.iter().enumerate() {
            let db = 20.0 * (p / peaks[0]).log10();
            assert!(
                db > -1.0,
                "toll {k} is {db:.1} dB under the first: {peaks:?}"
            );
        }
    }

    /// The heaviest scene (battle music, a full fight, the bed) must mix
    /// at least eight times faster than real time (an eighth of one core
    /// on the audio thread at most). Release builds only:
    /// `cargo test --release -p bw_desktop -- --ignored mixer_is_fast`.
    #[test]
    #[ignore]
    fn mixer_is_fast_enough_for_the_output_callback() {
        let songs = soundtrack();
        let scene = Scene::new(Levels::with_values(100, 100, 100), songs);
        let start = std::time::Instant::now();
        let r = scene.render(&montage(), 40_000).expect("render");
        let seconds = start.elapsed().as_secs_f64();
        assert!(r.rms > 0.0);
        let speed = 40.0 / seconds;
        eprintln!("montage mixed at {speed:.0}x real time");
        assert!(speed > 8.0, "only {speed:.1}x real time");
    }

    /// Where the mixer's time goes: `cargo test --release -p bw_desktop
    /// -- --ignored where_the_time_goes --nocapture`.
    #[test]
    #[ignore]
    fn where_the_time_goes() {
        let songs = soundtrack();
        let time = |steps: &[(usize, Step)], songs: Arc<Vec<Song>>| {
            let scene = Scene::new(Levels::with_values(100, 100, 100), songs);
            let start = std::time::Instant::now();
            scene.render(steps, 20_000).expect("render");
            20.0 / start.elapsed().as_secs_f64()
        };
        let music = |i: u8| {
            Step::Music(MusicState {
                song: Some(SongId::Match),
                intensity: i,
                ..MusicState::default()
            })
        };
        eprintln!("silence: {:.0}x", time(&[], Arc::new(Vec::new())));
        eprintln!(
            "ambience: {:.0}x",
            time(&[(0, Step::Ambience(100, 60, 50))], Arc::new(Vec::new()))
        );
        eprintln!(
            "music i0: {:.0}x",
            time(&[(0, music(0))], Arc::clone(&songs))
        );
        eprintln!(
            "music i3: {:.0}x",
            time(&[(0, music(3))], Arc::clone(&songs))
        );
        eprintln!(
            "montage without music: {:.0}x",
            time(
                &montage()
                    .into_iter()
                    .filter(|(_, s)| !matches!(s, Step::Music(_)))
                    .collect::<Vec<_>>(),
                Arc::new(Vec::new())
            )
        );
    }
}
