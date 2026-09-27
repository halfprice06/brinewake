//! The score's player: a tracker-style sequencer that plays the songs in
//! `score` through chip instruments. Channels belong to layers that enter
//! and leave on bar lines as the match changes, so one song can idle,
//! build, threaten and fight without a cut. Songs cross-fade; a jingle
//! hands over to the song it names when it ends. Nothing here allocates
//! after construction.

use crate::synth::{self, Echo, Lfsr, Noise, Svf, SvfCoef};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

pub const MAX_CHANNELS: usize = 48;
const SLOTS: usize = 6;

/// The songs, in the order `score::songs` builds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SongId {
    /// Home, setup, settings and the online lobby.
    Title = 0,
    /// The Split Basin match, layered by intensity.
    Match = 1,
    Victory = 2,
    Defeat = 3,
    Draw = 4,
    /// Under the result screen, after a jingle.
    Harbour = 5,
    /// The three-seat Confluence match.
    Confluence = 6,
    /// The third match song.
    Undertow = 7,
}

impl SongId {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Title,
            1 => Self::Match,
            2 => Self::Victory,
            3 => Self::Defeat,
            4 => Self::Draw,
            5 => Self::Harbour,
            6 => Self::Confluence,
            7 => Self::Undertow,
            _ => return None,
        })
    }

    /// The match song that follows this one in the rotation (each match
    /// song's `partner`).
    pub fn rotation_next(self) -> Self {
        match self {
            Self::Match => Self::Confluence,
            Self::Confluence => Self::Undertow,
            _ => Self::Match,
        }
    }
}

/// When a channel sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Always.
    Core,
    /// Only while nothing is happening; it leaves as the match stirs.
    Calm,
    /// Intensity 1 only: working, before any threat.
    Working,
    /// Intensity 1 or 2: the melody's voice until the fighting lead takes
    /// over.
    Unfought,
    /// Intensity 1 and up: the economy is running.
    Drive,
    /// Intensity 2 and up: the enemy is near or the tide is contested.
    Tension,
    /// Intensity 3: fighting.
    Battle,
    /// Our hold count is running.
    HoldOurs,
    /// An enemy hold count is running.
    HoldTheirs,
    /// The last thirty seconds of our own count.
    HoldFinal,
    /// An enemy count between a minute and thirty seconds from its end.
    HoldNeedle,
    /// An enemy count's last thirty seconds, growing as they pass.
    HoldRace,
    /// An enemy count's last ten seconds: one held needle.
    HoldLast,
    /// Its last five: the needle trills.
    HoldEnd,
    /// Intensity 2 only: the tension before a fight.
    Pressure,
    /// The four seconds before an enemy count's last ten: a riser.
    HoldRiser,
    /// The ordinary pad: it gives way to the dread pad in an enemy count's
    /// last thirty seconds.
    Pad,
}

const LAYERS: usize = 17;

/// What a channel does, for how its level moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Pads, ostinati, bass: they swell in and ebb away.
    Texture,
    /// The voice carrying the tune: hand-offs are quick, so two voices do
    /// not sing the same notes.
    Melody,
    /// Drums: they come in at once on a bar line and drop out fast.
    Kit,
    /// Bass lines: one hands to the next in a moment, not over bars.
    Bass,
    /// The song's own ostinati and pulses (the clock, the pump, stabs,
    /// arpeggios): quick like the bass, and silent from an enemy count's
    /// last thirty seconds, when the race takes over.
    Pulse,
    /// The race's own pulses (needles, racing hats): silent for the last
    /// ten seconds so only the toll keeps time.
    Race,
}

impl Layer {
    fn index(self) -> usize {
        self as usize
    }
}

/// The instrument's sound source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Osc {
    /// A pulse; `sweep` is the rate (Hz) of a slow width modulation
    /// between `duty` and a half.
    Pulse {
        duty: f32,
        sweep: f32,
    },
    /// The console triangle (stepped) or a smooth one.
    Triangle {
        stepped: bool,
    },
    Saw,
    /// Two detuned pulses and a sub octave: the pad.
    Pad {
        duty: f32,
        detune: f32,
    },
    /// A 32-step, 4-bit wave (0..15), as a wave channel holds it.
    Wave(&'static [u8; 32]),
    /// Two-operator FM: bells, glass, a brass edge. The index falls from
    /// `index` to `floor` of it with time constant `decay`.
    Fm {
        ratio: f32,
        index: f32,
        decay: f32,
        floor: f32,
    },
    Kick,
    Snare,
    HatClosed,
    HatOpen,
    Shaker,
    Tom,
    Crash,
    Clap,
    /// A washing wave: long filtered noise that swells and breaks.
    Surf,
    /// A struck metal bar: an anvil-like clank.
    Anvil,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instrument {
    pub osc: Osc,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub gain: f32,
    /// Lowpass cutoff in Hz; 0 leaves the source open.
    pub cutoff: f32,
    pub resonance: f32,
    /// Octaves the cutoff opens at the note's start, closing with `decay`.
    pub env_octaves: f32,
    /// Cutoff follows the note: 1.0 moves it an octave per octave.
    pub key_track: f32,
    pub vibrato_delay: f32,
    pub vibrato_rate: f32,
    pub vibrato_cents: f32,
    /// Seconds a slide takes to land.
    pub glide: f32,
    /// Breath noise on the attack, dying in 60 ms.
    pub breath: f32,
}

impl Instrument {
    pub const fn new(osc: Osc) -> Self {
        Self {
            osc,
            attack: 0.002,
            decay: 0.2,
            sustain: 0.7,
            release: 0.12,
            gain: 0.5,
            cutoff: 0.0,
            resonance: 0.707,
            env_octaves: 0.0,
            key_track: 0.0,
            vibrato_delay: 0.0,
            vibrato_rate: 5.5,
            vibrato_cents: 0.0,
            glide: 0.06,
            breath: 0.0,
        }
    }
    pub const fn adsr(mut self, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        self.attack = attack;
        self.decay = decay;
        self.sustain = sustain;
        self.release = release;
        self
    }
    pub const fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }
    pub const fn filter(mut self, cutoff: f32, resonance: f32, env_octaves: f32) -> Self {
        self.cutoff = cutoff;
        self.resonance = resonance;
        self.env_octaves = env_octaves;
        self
    }
    pub const fn track(mut self, key_track: f32) -> Self {
        self.key_track = key_track;
        self
    }
    pub const fn vibrato(mut self, delay: f32, rate: f32, cents: f32) -> Self {
        self.vibrato_delay = delay;
        self.vibrato_rate = rate;
        self.vibrato_cents = cents;
        self
    }
    pub const fn glide(mut self, seconds: f32) -> Self {
        self.glide = seconds;
        self
    }
    pub const fn breath(mut self, amount: f32) -> Self {
        self.breath = amount;
        self
    }
}

/// One scored note. Rows are the song's grid (a sixteenth, or a triplet
/// eighth in compound time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub row: u32,
    pub len: u32,
    pub key: u8,
    /// 0..1.
    pub vel: f32,
    /// Semitones the note slides in from (negative: from below).
    pub slide: i8,
    /// Chip-chord offsets cycled with the root at 50 Hz; zero for none.
    pub arp: [i8; 2],
    /// 0 plays the channel's instrument; n plays the channel's `extra[n-1]`.
    pub inst: u8,
    /// 0 plays every time through; 1 only on odd passes, 2 only on even
    /// ones, so a loop's second time differs.
    pub pass: u8,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub name: &'static str,
    pub inst: Instrument,
    /// A different lead for each faction (Union, Assembly, Compact).
    pub variants: Option<[Instrument; 3]>,
    /// Instruments a note may pick instead (`Note::inst`).
    pub extra: Vec<Instrument>,
    pub layer: Layer,
    pub pan: f32,
    pub gain: f32,
    pub echo: f32,
    pub reverb: f32,
    /// Notes on the same row sound together instead of cutting each other.
    pub poly: bool,
    /// A polyphonic channel's voices sit this far either side of `pan`.
    pub spread: f32,
    pub role: Role,
    /// A channel of one-off fills: its notes (rows counted from the beat
    /// before the bar line) play once whenever this layer comes on.
    pub entry: Option<Layer>,
    /// Sorted by row.
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug)]
pub struct Song {
    pub id: SongId,
    pub name: &'static str,
    pub bpm: f32,
    pub rows_per_beat: u32,
    pub beats_per_bar: u32,
    /// Odd rows land this fraction of a row late.
    pub swing: f32,
    pub bars: u32,
    /// The bar a looping song returns to; `None` plays once.
    pub loop_bar: Option<u32>,
    /// What follows a song that plays once.
    pub then: Option<SongId>,
    pub echo_rows: f32,
    pub echo_feedback: f32,
    pub gain: f32,
    pub channels: Vec<Channel>,
    /// Chord symbols by bar, for the score report.
    pub chords: Vec<String>,
    /// The song this one hands over to every second time through (and
    /// after a fight), while nothing threatens.
    pub partner: Option<SongId>,
    /// Bars that start a section: an entry fill is skipped into one (the
    /// section's own fill plays there).
    pub sections: Vec<u32>,
    /// Bars whose harmony leads into the partner's key: a hand-over plays
    /// out one of these and the partner starts on the next downbeat. The
    /// last is the one used at the end of a second time through.
    pub handovers: Vec<u32>,
    /// The key's offset from D, for effects that play in the music's key.
    pub key: i8,
}

impl Song {
    pub fn rows_per_bar(&self) -> u32 {
        self.rows_per_beat * self.beats_per_bar
    }
    pub fn total_rows(&self) -> u32 {
        self.bars * self.rows_per_bar()
    }
    pub fn seconds_per_row(&self) -> f32 {
        60.0 / (self.bpm * self.rows_per_beat as f32)
    }
    pub fn seconds(&self) -> f32 {
        self.total_rows() as f32 * self.seconds_per_row()
    }
}

/// What the game wants from the music. The game writes; the output
/// callback reads without locking.
#[derive(Default)]
pub struct MusicControl {
    song: AtomicU8,
    intensity: AtomicU8,
    flags: AtomicU8,
    variant: AtomicU8,
    hold_left: AtomicU8,
    stop: AtomicU8,
    /// Written back by the engine: the song now playing.
    playing: AtomicU8,
    generation: AtomicU32,
}

const FLAG_HOLD_OURS: u8 = 1;
const FLAG_HOLD_THEIRS: u8 = 2;
const FLAG_MUFFLED: u8 = 4;
const FLAG_FLOOD: u8 = 16;
const NO_SONG: u8 = 255;

/// How the music stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Over a second and a half: leaving for the menus.
    Gentle,
    /// At once: the final blow.
    Quick,
    /// At the end of the phrase, over three seconds: the music rests.
    Phrase,
}

/// A snapshot of the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicState {
    pub song: Option<SongId>,
    /// 0 calm, 1 working, 2 tense, 3 fighting.
    pub intensity: u8,
    pub hold_ours: bool,
    pub hold_theirs: bool,
    /// Seconds left on the running hold count; 0 when none runs.
    pub hold_left: u8,
    /// How the music stops when no song is asked for.
    pub stop: Stop,
    /// The tide is in flood: the echo lengthens.
    pub flood: bool,
    /// The pause menu: the music goes behind a wall.
    pub muffled: bool,
    /// The faction whose lead plays: 0 Union, 1 Assembly, 2 Compact.
    pub variant: u8,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            song: None,
            intensity: 0,
            hold_ours: false,
            hold_theirs: false,
            hold_left: 0,
            stop: Stop::Gentle,
            flood: false,
            muffled: false,
            variant: 0,
        }
    }
}

impl MusicControl {
    pub fn new() -> Self {
        let control = Self::default();
        control.song.store(NO_SONG, Ordering::Relaxed);
        control.playing.store(NO_SONG, Ordering::Relaxed);
        control
    }

    /// The song the engine is playing (it may have handed over to a
    /// partner by itself); none once it has faded to silence.
    pub fn playing(&self) -> Option<SongId> {
        SongId::from_u8(self.playing.load(Ordering::Relaxed))
    }

    /// Ask for a state. A different song (or `restart`) starts it anew;
    /// the rest takes effect at the next bar line.
    pub fn set(&self, state: MusicState, restart: bool) {
        let song = state.song.map_or(NO_SONG, |s| s as u8);
        let changed = self.song.swap(song, Ordering::AcqRel) != song;
        self.intensity
            .store(state.intensity.min(3), Ordering::Relaxed);
        let flags = (u8::from(state.hold_ours) * FLAG_HOLD_OURS)
            | (u8::from(state.hold_theirs) * FLAG_HOLD_THEIRS)
            | (u8::from(state.muffled) * FLAG_MUFFLED)
            | (u8::from(state.flood) * FLAG_FLOOD);
        self.hold_left.store(state.hold_left, Ordering::Relaxed);
        self.stop.store(state.stop as u8, Ordering::Relaxed);
        self.flags.store(flags, Ordering::Relaxed);
        self.variant.store(state.variant.min(2), Ordering::Relaxed);
        if changed || restart {
            self.generation.fetch_add(1, Ordering::AcqRel);
        }
    }

    fn read(&self) -> (MusicState, u32) {
        let flags = self.flags.load(Ordering::Relaxed);
        (
            MusicState {
                song: SongId::from_u8(self.song.load(Ordering::Acquire)),
                intensity: self.intensity.load(Ordering::Relaxed),
                hold_ours: flags & FLAG_HOLD_OURS != 0,
                hold_theirs: flags & FLAG_HOLD_THEIRS != 0,
                muffled: flags & FLAG_MUFFLED != 0,
                hold_left: self.hold_left.load(Ordering::Relaxed),
                stop: match self.stop.load(Ordering::Relaxed) {
                    1 => Stop::Quick,
                    2 => Stop::Phrase,
                    _ => Stop::Gentle,
                },
                flood: flags & FLAG_FLOOD != 0,
                variant: self.variant.load(Ordering::Relaxed),
            },
            self.generation.load(Ordering::Acquire),
        )
    }
}

fn layer_targets(state: &MusicState) -> [f32; LAYERS] {
    let i = state.intensity;
    let on = |b: bool| if b { 1.0 } else { 0.0 };
    let left = if state.hold_left == 0 {
        u8::MAX
    } else {
        state.hold_left
    };
    let race = state.hold_theirs && left <= 30;
    // The race grows from 0.6 at thirty seconds to full at ten.
    let grow = if race {
        (0.6 + 0.4 * (30.0 - f32::from(left)) / 20.0).clamp(0.6, 1.0)
    } else {
        0.0
    };
    // Assigned by name, so the order of the enum cannot slip.
    let mut t = [0.0; LAYERS];
    let mut set = |layer: Layer, value: f32| t[layer.index()] = value;
    set(Layer::Core, 1.0);
    set(Layer::Calm, on(i == 0));
    set(Layer::Working, on(i == 1));
    set(Layer::Unfought, on(i == 1 || i == 2));
    set(Layer::Drive, on(i >= 1));
    set(Layer::Tension, on(i >= 2));
    set(Layer::Battle, on(i >= 3));
    set(Layer::HoldOurs, on(state.hold_ours));
    set(Layer::HoldTheirs, on(state.hold_theirs));
    set(Layer::HoldFinal, on(state.hold_ours && left <= 30));
    set(
        Layer::HoldNeedle,
        on(state.hold_theirs && left <= 60 && left > 30),
    );
    set(Layer::HoldRace, grow);
    set(Layer::HoldLast, on(state.hold_theirs && left <= 10));
    set(Layer::HoldEnd, on(state.hold_theirs && left <= 5));
    set(Layer::Pressure, on(i == 2));
    set(
        Layer::HoldRiser,
        on(state.hold_theirs && left <= 14 && left > 10),
    );
    set(Layer::Pad, on(!race));
    t
}

/// Whole roles give way late in an enemy count: the tune sinks under the
/// dread in the last thirty seconds and the drums stop for the last ten.
fn role_gain(role: Role, state: &MusicState) -> f32 {
    let left = if state.hold_left == 0 {
        u8::MAX
    } else {
        state.hold_left
    };
    match role {
        Role::Melody | Role::Pulse if state.hold_theirs && left <= 30 => 0.0,
        Role::Kit | Role::Race if state.hold_theirs && left <= 10 => 0.0,
        _ => 1.0,
    }
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    inst: Instrument,
    hz: f32,
    vel: f32,
    slide: f32,
    arp: [i8; 2],
    t: f32,
    /// Samples since the note began and the sample its gate closes on.
    elapsed: u32,
    gate: u32,
    released: bool,
    level: f32,
    rel_from: f32,
    rel_t: f32,
    phase: f32,
    phase2: f32,
    mod_phase: f32,
    svf: Svf,
    svf2: Svf,
    noise: Noise,
    lfsr: Lfsr,
    /// Control-rate state: pitch and the tone filter are worked out every
    /// sixteen samples, the envelope's decay and release by a factor.
    ctl: u8,
    inc: f32,
    hz_now: f32,
    tone: SvfCoef,
    decaying: f32,
    k_decay: f32,
    k_release: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            inst: Instrument::new(Osc::Triangle { stepped: false }),
            hz: 440.0,
            vel: 0.0,
            slide: 0.0,
            arp: [0, 0],
            t: 0.0,
            elapsed: 0,
            gate: 0,
            released: false,
            level: 0.0,
            rel_from: 0.0,
            rel_t: 0.0,
            phase: 0.0,
            phase2: 0.0,
            mod_phase: 0.0,
            svf: Svf::default(),
            svf2: Svf::default(),
            noise: Noise(0x1234_5678),
            lfsr: Lfsr::default(),
            ctl: 0,
            inc: 0.0,
            hz_now: 440.0,
            tone: SvfCoef::default(),
            decaying: -1.0,
            k_decay: 0.0,
            k_release: 0.0,
        }
    }
}

impl Voice {
    fn start(&mut self, inst: Instrument, note: &Note, gate: u32, seed: u32) {
        *self = Self {
            active: true,
            inst,
            hz: synth::midi_hz(f32::from(note.key)),
            vel: note.vel.clamp(0.0, 1.0),
            slide: f32::from(note.slide),
            arp: note.arp,
            t: 0.0,
            elapsed: 0,
            gate,
            noise: Noise(seed | 1),
            // Pads start their second oscillator elsewhere, so repeated
            // chords do not phase identically.
            phase2: (seed % 997) as f32 / 997.0,
            ..Self::default()
        };
    }

    fn release(&mut self) {
        if self.active && !self.released {
            self.released = true;
            self.rel_from = self.level;
            self.rel_t = 0.0;
        }
    }

    /// Cut quickly for a new note on the same slot's channel.
    fn choke(&mut self) {
        if self.active {
            self.released = true;
            self.rel_from = self.level;
            self.rel_t = 0.0;
            self.inst.release = self.inst.release.min(0.03);
        }
    }

    fn envelope(&mut self, dt: f32) -> f32 {
        let i = &self.inst;
        if self.k_decay == 0.0 {
            self.k_decay = (-dt / (i.decay / 3.0).max(0.001)).exp();
        }
        if self.released {
            if self.rel_t == 0.0 {
                // The release's factor is set when it begins (a choke
                // shortens the release).
                self.k_release = (-dt / (i.release * 0.25).max(0.002)).exp();
                self.level = self.rel_from;
            }
            self.rel_t += dt;
            self.level *= self.k_release;
            if self.level < 0.000_3 || self.rel_t > i.release * 2.0 + 0.01 {
                self.active = false;
            }
        } else if self.t < i.attack {
            self.level = self.t / i.attack.max(0.000_1);
        } else {
            if self.decaying < 0.0 {
                self.decaying = 1.0 - i.sustain;
            }
            self.decaying *= self.k_decay;
            self.level = i.sustain + self.decaying;
            // A note whose sustain is silent finishes by itself.
            if i.sustain <= 0.0 && self.level < 0.000_3 {
                self.active = false;
            }
        }
        self.level
    }

    #[inline]
    fn next(&mut self, sr: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let dt = 1.0 / sr;
        if !self.released && self.elapsed >= self.gate {
            self.release();
        }
        let env = self.envelope(dt);
        if !self.active {
            return 0.0;
        }
        let inst = self.inst;
        let t = self.t;
        self.t += dt;
        self.elapsed = self.elapsed.saturating_add(1);

        // Pitch and the tone filter move at a control rate.
        if self.ctl == 0 {
            let mut semis = 0.0;
            if self.slide != 0.0 {
                let k = (t / inst.glide.max(0.001)).min(1.0);
                let k = k * k * (3.0 - 2.0 * k);
                semis += self.slide * (1.0 - k);
            }
            if self.arp != [0, 0] {
                let step = ((t * 50.0) as usize) % 3;
                semis += match step {
                    0 => 0.0,
                    1 => f32::from(self.arp[0]),
                    _ => f32::from(self.arp[1]),
                };
            }
            if inst.vibrato_cents > 0.0 && t > inst.vibrato_delay {
                let ramp = ((t - inst.vibrato_delay) / 0.35).min(1.0);
                semis += inst.vibrato_cents / 100.0
                    * ramp
                    * synth::sine((t - inst.vibrato_delay) * inst.vibrato_rate);
            }
            self.hz_now = self.hz * (semis * (std::f32::consts::LN_2 / 12.0)).exp();
            self.inc = (self.hz_now / sr).min(0.45);
            if inst.cutoff > 0.0 {
                let octaves = inst.env_octaves * env
                    + inst.key_track * ((self.hz_now / 261.63).max(0.01)).log2();
                let cutoff = (inst.cutoff * octaves.exp2()).min(sr * 0.45);
                self.tone = SvfCoef::new(cutoff, inst.resonance, sr);
            }
        }
        self.ctl = (self.ctl + 1) % 16;
        let hz = self.hz_now;
        let inc = self.inc;

        let raw = match inst.osc {
            Osc::Pulse { duty, sweep } => {
                let d = if sweep > 0.0 {
                    duty + (0.5 - duty) * 0.5 * (1.0 - synth::sine(t * sweep + 0.25))
                } else {
                    duty
                };
                synth::pulse(self.phase, inc, d)
            }
            Osc::Triangle { stepped } => {
                if stepped {
                    synth::triangle_stepped(self.phase)
                } else {
                    synth::triangle(self.phase)
                }
            }
            Osc::Saw => synth::saw(self.phase, inc),
            Osc::Pad { duty, detune } => {
                let ratio = (detune / 1200.0 * std::f32::consts::LN_2).exp();
                let inc2 = inc * ratio;
                let a = synth::pulse(self.phase, inc, duty);
                let b = synth::pulse(self.phase2, inc2, duty + 0.08);
                self.phase2 = (self.phase2 + inc2).fract();
                0.45 * (a + b)
            }
            Osc::Wave(table) => {
                let pos = self.phase * 32.0;
                let i = pos as usize & 31;
                let j = (i + 1) & 31;
                let frac = pos - pos.floor();
                let a = f32::from(table[i]) / 7.5 - 1.0;
                let b = f32::from(table[j]) / 7.5 - 1.0;
                a + (b - a) * frac
            }
            Osc::Fm {
                ratio,
                index,
                decay,
                floor,
            } => {
                let idx = index * (floor + (1.0 - floor) * (-t / decay.max(0.001)).exp());
                // Keep the sidebands (Carson's rule) under Nyquist.
                let room = ((0.45 / inc.max(1e-6)) - 1.0) / ratio.max(0.01) - 1.0;
                let idx = idx.min(room.max(0.0));
                let m = synth::sine(self.mod_phase);
                self.mod_phase = (self.mod_phase + inc * ratio).fract();
                synth::sine(self.phase + idx * m / std::f32::consts::TAU)
            }
            Osc::Kick => {
                let f = hz + 160.0 * (-t / 0.03).exp();
                self.phase2 = (self.phase2 + f / sr).fract();
                let body = synth::sine(self.phase2) * (-t / 0.10).exp();
                // A beater click a laptop speaker can play.
                let n = self.noise.next();
                let click = self.svf.process(n, 2_500.0, 1.2, sr).band * (-t / 0.001).exp() * 1.4;
                (2.2 * body).tanh() + click
            }
            Osc::Snare => {
                let f = 190.0 * (1.0 + 0.6 * (-t / 0.015).exp());
                self.phase2 = (self.phase2 + f / sr).fract();
                let tone = synth::sine(self.phase2) * (-t / 0.05).exp();
                let n = self.noise.next();
                let rattle = self.svf.process(n, 4_200.0, 0.8, sr).band * 2.0
                    + self.svf2.process(n, 7_000.0, 0.7, sr).high * 0.6;
                0.55 * tone + rattle * (-t / 0.13).exp()
            }
            Osc::HatClosed | Osc::HatOpen => {
                let n = self.lfsr.next(18_000.0, sr, false) * 0.6 + self.noise.next() * 0.4;
                let hp = self.svf.process(n, 7_500.0, 0.9, sr).high;
                let tail = if inst.osc == Osc::HatOpen {
                    0.26
                } else {
                    0.035
                };
                hp * (-t / tail).exp()
            }
            Osc::Shaker => {
                let n = self.noise.next();
                let bp = self.svf.process(n, 6_500.0, 1.2, sr).band * 1.8;
                let shape = (t / 0.012).min(1.0) * (-t / 0.06).exp();
                bp * shape
            }
            Osc::Tom => {
                let f = hz * (1.0 + 0.5 * (-t / 0.04).exp());
                self.phase2 = (self.phase2 + f / sr).fract();
                let body = synth::sine(self.phase2) * (-t / 0.28).exp();
                let n = self.noise.next();
                let skin = n * (-t / 0.01).exp() * 0.25;
                // A beater click a small speaker can carry.
                let click = self.svf.process(n, 2_500.0, 1.2, sr).band * (-t / 0.001).exp() * 1.2;
                (1.3 * body + skin).tanh() + click
            }
            Osc::Crash => {
                // Six square partials at the classic inharmonic ratios.
                let base = 330.0;
                let ratios = [1.0, 1.47, 1.63, 1.94, 2.44, 3.19];
                let mut metal = 0.0;
                for (k, r) in ratios.iter().enumerate() {
                    let p = (t * base * r * (1.0 + k as f32 * 0.013)).fract();
                    metal += if p < 0.5 { 1.0 } else { -1.0 };
                }
                let n = self.noise.next();
                let mix = metal * 0.12 + n * 0.8;
                let hp = self.svf.process(mix, 5_200.0, 0.8, sr).high;
                hp * (-t / 1.1).exp() * (0.4 + 0.6 * (-t / 0.08).exp())
            }
            Osc::Clap => {
                let n = self.noise.next();
                let bp = self.svf.process(n, 1_400.0, 1.4, sr).band * 2.2;
                let bursts = [0.0, 0.011, 0.022];
                let mut shape = 0.0f32;
                for b in bursts {
                    if t >= b {
                        shape = shape.max((-(t - b) / 0.006).exp());
                    }
                }
                let tail = if t > 0.022 {
                    (-(t - 0.022) / 0.09).exp() * 0.6
                } else {
                    0.0
                };
                bp * shape.max(tail)
            }
            Osc::Surf => {
                let n = self.noise.next();
                let open = 400.0 + 2_400.0 * env;
                self.svf.process(n, open, 0.6, sr).low * 1.6
            }
            Osc::Anvil => {
                let ratios = [1.0, 2.76, 5.40, 8.93];
                let mut v = 0.0;
                for (k, r) in ratios.iter().enumerate() {
                    let p = (t * hz * r).fract();
                    v += synth::sine(p) * (-t * (3.0 + k as f32 * 6.0)).exp() / (1.0 + k as f32);
                }
                v + self.noise.next() * (-t / 0.004).exp() * 0.3
            }
        };
        self.phase = (self.phase + inc).fract();

        let mut out = raw;
        if inst.breath > 0.0 && t < 0.25 {
            out += self.noise.next() * inst.breath * (-t / 0.06).exp();
        }
        if inst.cutoff > 0.0 {
            out = self.svf2.run(out, &self.tone).low;
        }
        out * env * self.vel * inst.gain
    }
}

/// Why a song wants to hand over to its partner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Want {
    /// It has played twice through.
    Loop,
    /// A fight has ended.
    AfterFight,
}

fn is_enemy_count(layer: Layer) -> bool {
    matches!(
        layer,
        Layer::HoldTheirs
            | Layer::HoldNeedle
            | Layer::HoldRace
            | Layer::HoldLast
            | Layer::HoldEnd
            | Layer::HoldRiser
    )
}

struct Deck {
    song: Option<usize>,
    row: u32,
    t: f64,
    next_row_at: f64,
    note_idx: [usize; MAX_CHANNELS],
    voices: [[Voice; SLOTS]; MAX_CHANNELS],
    slot: [usize; MAX_CHANNELS],
    /// Layer targets and the state role gains read, both in force from a
    /// bar line; and those decided a beat before the next (so a fill can
    /// lead into it).
    layer_target: [f32; LAYERS],
    role_state: MusicState,
    pending: Option<([f32; LAYERS], MusicState)>,
    /// Each channel's level, moving toward its target at its role's pace.
    chan_gain: [f32; MAX_CHANNELS],
    /// Fill channels: the row their fill began, and the next note.
    /// The row a fill's template starts on (before 0 for a fill whose
    /// pickup would fall before the song begins).
    entry_start: [Option<i64>; MAX_CHANNELS],
    entry_idx: [usize; MAX_CHANNELS],
    fade: f32,
    fade_target: f32,
    fade_rate: f32,
    /// Stop at the next four-bar line.
    phrase_stop: bool,
    /// The arrangement holds still: a rest is waiting or fading.
    frozen: bool,
    /// No more notes: the song has handed over.
    mute_notes: bool,
    /// The rows ran out (a song that plays once).
    finished_at: Option<f64>,
    /// Reported once, so what follows can start under the last chord.
    handed_over: bool,
    seed: u32,
    /// Each deck keeps its own echo, so a song's tail keeps its tempo
    /// while the next begins.
    echo: Echo,
    /// Each channel's pan gains for its even and odd voices.
    pans: [[(f32, f32); 2]; MAX_CHANNELS],
    /// Times the song has looped; odd and even passes differ.
    loops: u32,
    /// Until this row the song holds to its calm arrangement, so a song
    /// arriving from silence builds up instead of landing. Cleared once
    /// passed.
    calm_until: u32,
    /// A fight was heard since the song began.
    battle_seen: bool,
    /// The song would like to hand over, and why.
    want: Option<Want>,
    /// The downbeat a hand-over in progress ends on.
    handing: Option<u32>,
    /// The hand-over bar has ended: the partner starts now.
    handover_due: bool,
    /// Whether each melody channel may start notes.
    melody_on: [bool; MAX_CHANNELS],
    /// A running level of the deck's output, for the timeline probe.
    level: f32,
}

impl Deck {
    fn new(sr: f32) -> Self {
        Self {
            song: None,
            row: 0,
            t: 0.0,
            next_row_at: 0.0,
            note_idx: [0; MAX_CHANNELS],
            voices: [[Voice::default(); SLOTS]; MAX_CHANNELS],
            slot: [0; MAX_CHANNELS],
            layer_target: [0.0; LAYERS],
            role_state: MusicState::default(),
            pending: None,
            chan_gain: [0.0; MAX_CHANNELS],
            entry_start: [None; MAX_CHANNELS],
            entry_idx: [0; MAX_CHANNELS],
            fade: 0.0,
            fade_target: 0.0,
            fade_rate: 0.0,
            phrase_stop: false,
            frozen: false,
            mute_notes: false,
            finished_at: None,
            handed_over: false,
            seed: 0x51ed_27a1,
            echo: Echo::new(sr),
            pans: [[(0.707, 0.707); 2]; MAX_CHANNELS],
            loops: 0,
            calm_until: 0,
            battle_seen: false,
            want: None,
            handing: None,
            handover_due: false,
            melody_on: [false; MAX_CHANNELS],
            level: 0.0,
        }
    }

    /// The layer targets for the bar starting at `row`: calm while the
    /// song is still building up.
    fn targets_at(&self, row: u32, state: &MusicState) -> [f32; LAYERS] {
        if row < self.calm_until {
            layer_targets(&MusicState {
                intensity: 0,
                ..*state
            })
        } else {
            layer_targets(state)
        }
    }

    fn start(
        &mut self,
        index: usize,
        song: &Song,
        state: &MusicState,
        fade_in: f32,
        sr: f32,
        calm_bars: u32,
    ) {
        self.song = Some(index);
        self.loops = 0;
        self.calm_until = calm_bars * song.rows_per_bar();
        self.battle_seen = false;
        self.want = None;
        self.handing = None;
        self.handover_due = false;
        self.row = 0;
        self.t = 0.0;
        self.next_row_at = 0.0;
        self.note_idx = [0; MAX_CHANNELS];
        for slots in &mut self.voices {
            for v in slots {
                v.active = false;
            }
        }
        self.layer_target = self.targets_at(0, state);
        self.role_state = *state;
        self.pending = None;
        for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
            self.chan_gain[c] =
                self.layer_target[channel.layer.index()] * role_gain(channel.role, state);
            self.melody_on[c] = self.chan_gain[c] > 0.5;
            if channel.role == Role::Melody {
                self.chan_gain[c] = 1.0;
            }
            self.pans[c] = [
                pan_gains(channel.pan - channel.spread),
                pan_gains(channel.pan + channel.spread),
            ];
        }
        self.entry_start = [None; MAX_CHANNELS];
        self.entry_idx = [0; MAX_CHANNELS];
        // Straight into a fight: the entry's downbeat hit plays on the
        // first row (its pickup would fall before the song).
        if fade_in <= 0.0 {
            for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
                if channel
                    .entry
                    .is_some_and(|layer| self.layer_target[layer.index()] > 0.5)
                {
                    self.entry_start[c] = Some(-i64::from(song.rows_per_beat));
                }
            }
        }
        self.fade = if fade_in > 0.0 { 0.0 } else { 1.0 };
        self.fade_target = 1.0;
        self.fade_rate = 1.0 / (fade_in.max(0.01) * sr);
        self.phrase_stop = false;
        self.frozen = false;
        self.mute_notes = false;
        self.finished_at = None;
        self.handed_over = false;
        self.level = 0.0;
        self.echo.clear();
        self.echo.delay_samples = song.echo_rows * song.seconds_per_row() * sr;
        self.echo.feedback = song.echo_feedback;
    }

    fn fade_to(&mut self, target: f32, seconds: f32, sr: f32) {
        self.fade_target = target;
        self.fade_rate = (self.fade - target).abs().max(0.01) / (seconds.max(0.01) * sr);
    }

    fn fade_out(&mut self, seconds: f32, sr: f32) {
        self.fade_target = 0.0;
        self.fade_rate = 1.0 / (seconds.max(0.01) * sr);
    }

    fn row_time(song: &Song, row: u32, spr: f64) -> f64 {
        let swing = if row % 2 == 1 {
            f64::from(song.swing)
        } else {
            0.0
        };
        (f64::from(row) + swing) * spr
    }

    fn trigger(&mut self, c: usize, channel: &Channel, note: &Note, gate: u32, state: &MusicState) {
        let inst = if note.inst > 0 {
            channel
                .extra
                .get(usize::from(note.inst) - 1)
                .copied()
                .unwrap_or(channel.inst)
        } else if let Some(variants) = channel.variants {
            variants[usize::from(state.variant.min(2))]
        } else {
            channel.inst
        };
        let slot = if channel.poly {
            // A free voice; else the quietest letting go; else the oldest
            // still held. Never one begun this row.
            let slots = &self.voices[c];
            slots.iter().position(|v| !v.active).or_else(|| {
                (0..SLOTS)
                    .filter(|&k| slots[k].released && slots[k].elapsed > 0)
                    .min_by(|&a, &b| {
                        slots[a]
                            .level
                            .partial_cmp(&slots[b].level)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .or_else(|| {
                        (0..SLOTS)
                            .filter(|&k| slots[k].elapsed > 0)
                            .max_by_key(|&k| slots[k].elapsed)
                    })
            })
        } else {
            self.voices[c][self.slot[c]].choke();
            Some((self.slot[c] + 1) % SLOTS)
        };
        let Some(slot) = slot else {
            return;
        };
        self.slot[c] = slot;
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.voices[c][slot].start(inst, note, gate, self.seed);
    }

    /// Advance one sample: (left, right, reverb send, rows finished just
    /// now, tail finished).
    fn next(
        &mut self,
        songs: &[Song],
        state: &MusicState,
        sr: f32,
    ) -> Option<(f32, f32, f32, bool, bool)> {
        let index = self.song?;
        let song = &songs[index];
        let spr = f64::from(sr) * f64::from(song.seconds_per_row());
        let rows_per_bar = song.rows_per_bar();
        let beat = song.rows_per_beat;
        let total = song.total_rows();
        let mut ended = false;
        let mut rows_done = false;

        // Trigger every row that is due.
        while self.finished_at.is_none() && self.t >= self.next_row_at {
            if self.row >= total {
                match song.loop_bar {
                    Some(bar) => {
                        let loop_row = bar * rows_per_bar;
                        let span =
                            Self::row_time(song, total, spr) - Self::row_time(song, loop_row, spr);
                        self.t -= span;
                        self.row = loop_row;
                        self.loops += 1;
                        // A hand-over that ends on the song's last downbeat
                        // hands over here.
                        if self.handing.is_some_and(|end| end >= total) {
                            self.handing = None;
                            self.handover_due = true;
                            self.mute_notes = true;
                            self.frozen = true;
                        }
                        for (c, channel) in song.channels.iter().enumerate() {
                            self.note_idx[c] = channel.notes.partition_point(|n| n.row < loop_row);
                        }
                    }
                    None => {
                        self.finished_at = Some(self.t);
                        break;
                    }
                }
            }
            let row = self.row;
            let bar = row / rows_per_bar;
            let in_bar = row % rows_per_bar;
            // The build-up is used once.
            if self.calm_until > 0 && row >= self.calm_until {
                self.calm_until = 0;
            }
            // A beat before the bar line: decide what comes on, and lead
            // into a fight (or back from a count's silence) with a fill.
            // A rest in waiting keeps the arrangement as it is.
            if in_bar == rows_per_bar - beat && !self.frozen {
                let next = self.targets_at(row + beat, state);
                let next_bar = (row + beat) / rows_per_bar % song.bars;
                let section_start = song.sections.contains(&next_bar);
                let kit_back = role_gain(Role::Kit, &self.role_state) < 0.5
                    && role_gain(Role::Kit, state) > 0.5;
                for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
                    if let Some(layer) = channel.entry {
                        let li = layer.index();
                        let comes_on = next[li] > 0.5 && (self.layer_target[li] < 0.5 || kit_back);
                        if comes_on && !section_start && self.handing.is_none() {
                            self.entry_start[c] = Some(i64::from(row));
                            self.entry_idx[c] = 0;
                        }
                    }
                }
                self.pending = Some((next, *state));
            }
            // Roles follow the request on each beat: an enemy count's last
            // seconds empty the music on the beat, not up to a bar late.
            if in_bar.is_multiple_of(beat) && !self.frozen {
                self.role_state = *state;
            }
            if in_bar == 0 {
                if !self.frozen {
                    let (targets, _) = self
                        .pending
                        .take()
                        .unwrap_or_else(|| (self.targets_at(row, state), *state));
                    self.layer_target = targets;
                }
                if self.phrase_stop && bar.is_multiple_of(4) {
                    self.phrase_stop = false;
                    self.fade_out(3.0, sr);
                }
                if self.layer_target[Layer::Battle.index()] > 0.5 {
                    self.battle_seen = true;
                }
                // Wanting to hand over: after two times through, or when a
                // fight has ended.
                if song.partner.is_some() {
                    if self.loops % 2 == 1 {
                        self.want.get_or_insert(Want::Loop);
                    }
                    if self.battle_seen && state.intensity <= 1 {
                        self.battle_seen = false;
                        self.want = Some(Want::AfterFight);
                    }
                }
                // A hand-over plays out a bar whose harmony leads into the
                // partner's key, never in a fight or while a count runs:
                // after two times through it may happen under tension too,
                // after a fight only once the harbour is working again.
                let calm_enough = match self.want {
                    Some(Want::Loop) => state.intensity <= 2,
                    _ => state.intensity <= 1,
                };
                let quiet = calm_enough && !state.hold_ours && !state.hold_theirs;
                let point = match self.want {
                    Some(Want::Loop) => song.handovers.last() == Some(&bar),
                    Some(Want::AfterFight) => song.handovers.contains(&bar),
                    None => false,
                };
                if point && quiet && !self.frozen && self.handing.is_none() && !self.mute_notes {
                    self.handing = Some(row + rows_per_bar);
                    self.want = None;
                }
                if let Some(end) = self.handing
                    && row >= end
                {
                    self.handing = None;
                    self.handover_due = true;
                    self.mute_notes = true;
                    self.frozen = true;
                }
                // Melody voices hand over by note: the outgoing voice
                // starts nothing new and lets its note go; the incoming one
                // starts at full level.
                for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
                    if channel.role != Role::Melody {
                        continue;
                    }
                    let on = self.layer_target[channel.layer.index()]
                        * role_gain(channel.role, &self.role_state)
                        > 0.5;
                    if !on && self.melody_on[c] {
                        for v in &mut self.voices[c] {
                            v.release();
                        }
                    }
                    self.melody_on[c] = on;
                }
            }
            // A rest, a count or a fight asked during the hand-over bar
            // calls it off: this song carries on.
            if self.handing.is_some()
                && (self.phrase_stop
                    || state.hold_ours
                    || state.hold_theirs
                    || state.intensity >= 3)
            {
                self.handing = None;
                if !self.phrase_stop && self.fade_target < 1.0 {
                    self.fade_to(1.0, 0.2, sr);
                }
            }
            // The hand-over bar's second half: drums and pulses stop, the
            // song falls to -30 dB by the downbeat.
            let closing = self.handing.is_some_and(|end| row + 2 * beat >= end);
            if self.handing.is_some_and(|end| row + 2 * beat == end) {
                let seconds = (2 * beat) as f32 * song.seconds_per_row();
                self.fade_to(0.03, seconds, sr);
            }
            let row_at = Self::row_time(song, row, spr);
            for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
                if self.mute_notes {
                    break;
                }
                if channel.entry.is_some() {
                    // Fill channels play their template from where it began.
                    let Some(start) = self.entry_start[c] else {
                        continue;
                    };
                    while let Some(note) = channel.notes.get(self.entry_idx[c]) {
                        let at = start + i64::from(note.row);
                        if at > i64::from(row) {
                            break;
                        }
                        self.entry_idx[c] += 1;
                        if at == i64::from(row) {
                            let gate = (spr * f64::from(note.len)).round().max(1.0) as u32;
                            let note = *note;
                            self.trigger(c, channel, &note, gate, state);
                        }
                    }
                    if self.entry_idx[c] >= channel.notes.len() {
                        self.entry_start[c] = None;
                    }
                    continue;
                }
                let due = channel
                    .notes
                    .get(self.note_idx[c])
                    .is_some_and(|n| n.row == row);
                if due {
                    // Notes whose length ends here let go before the new
                    // ones take voices, whatever the clock's rounding.
                    for v in &mut self.voices[c] {
                        if v.active && !v.released && v.elapsed.saturating_add(2) >= v.gate {
                            v.release();
                        }
                    }
                }
                let pass = (self.loops % 2) as u8 + 1;
                let silenced =
                    closing && matches!(channel.role, Role::Kit | Role::Pulse | Role::Race);
                while let Some(note) = channel.notes.get(self.note_idx[c]) {
                    if note.row > row {
                        break;
                    }
                    self.note_idx[c] += 1;
                    if note.row < row || (note.pass != 0 && note.pass != pass) || silenced {
                        continue;
                    }
                    if channel.role == Role::Melody && !self.melody_on[c] {
                        continue;
                    }
                    let end_at = Self::row_time(song, row + note.len, spr);
                    let gate = (end_at - row_at).round().max(1.0) as u32;
                    let note = *note;
                    self.trigger(c, channel, &note, gate, state);
                }
            }
            self.row += 1;
            self.next_row_at = Self::row_time(song, self.row, spr);
        }
        if self.finished_at.is_some() && !self.handed_over {
            self.handed_over = true;
            rows_done = true;
        }
        self.t += 1.0;

        if self.fade > self.fade_target {
            self.fade = (self.fade - self.fade_rate).max(self.fade_target);
        } else if self.fade < self.fade_target {
            self.fade = (self.fade + self.fade_rate).min(self.fade_target);
        }

        let mut l = 0.0;
        let mut r = 0.0;
        let mut echo = 0.0;
        let mut verb = 0.0;
        let mut sounding = false;
        for (c, channel) in song.channels.iter().enumerate().take(MAX_CHANNELS) {
            // Each channel moves toward its target at its role's pace: a
            // tune hands over by note; a pad swells in over a bar and ebbs
            // over three seconds. An enemy count that breaks empties its
            // layers at once.
            let count_broken = is_enemy_count(channel.layer) && !state.hold_theirs;
            let target = if count_broken {
                0.0
            } else if channel.role == Role::Melody {
                1.0
            } else if channel.entry.is_some() {
                role_gain(channel.role, &self.role_state)
            } else {
                self.layer_target[channel.layer.index()] * role_gain(channel.role, &self.role_state)
            };
            let (up, down) = match channel.role {
                _ if count_broken => (0.3, 0.3),
                Role::Melody => (0.25, 0.25),
                Role::Kit => (0.01, 0.6),
                Role::Bass | Role::Pulse | Role::Race => (0.05, 0.3),
                Role::Texture => (1.2, 3.0),
            };
            let gain = &mut self.chan_gain[c];
            if *gain < target {
                *gain = (*gain + 1.0 / (up * sr)).min(target);
            } else if *gain > target {
                *gain = (*gain - 1.0 / (down * sr)).max(target);
            }
            let g = *gain * channel.gain;
            let slots = &mut self.voices[c];
            if g <= 0.000_1 {
                // Silent layers keep time without spending the work.
                for v in slots.iter_mut() {
                    if v.active {
                        v.t += 1.0 / sr;
                        v.elapsed = v.elapsed.saturating_add(1);
                        if v.elapsed > v.gate.saturating_add((v.inst.release * sr) as u32 + 2_000) {
                            v.active = false;
                        }
                    }
                }
                continue;
            }
            let mut sum = 0.0;
            let pans = self.pans[c];
            for (k, v) in slots.iter_mut().enumerate() {
                if !v.active {
                    continue;
                }
                sounding = true;
                let x = v.next(sr) * g;
                let (pl, pr) = pans[k % 2];
                l += x * pl;
                r += x * pr;
                sum += x;
            }
            echo += sum * channel.echo;
            verb += sum * channel.reverb;
        }
        // The echo keeps ringing after the last voice.
        let flood = if state.flood { 0.1 } else { 0.0 };
        self.echo.feedback = (song.echo_feedback + flood).min(0.6);
        let (el, er) = self.echo.process(echo, echo, sr);
        l += el * 0.55;
        r += er * 0.55;
        verb += (el + er) * 0.15;
        if self.finished_at.is_some() && !sounding {
            ended = self
                .finished_at
                .is_some_and(|at| self.t - at > f64::from(sr) * 2.5);
        }
        if self
            .finished_at
            .is_some_and(|at| self.t - at > f64::from(sr) * 8.0)
        {
            ended = true;
        }
        let f = self.fade * song.gain;
        let (l, r) = (l * f, r * f);
        self.level += (l * l + r * r - self.level) * (1.0 / (0.2 * sr));
        Some((l, r, verb * f, rows_done, ended))
    }

    /// What the probe reports: the effective intensity from the layers in
    /// force.
    fn effective_intensity(&self) -> u8 {
        let on = |layer: Layer| self.layer_target[layer.index()] > 0.5;
        if on(Layer::Battle) {
            3
        } else if on(Layer::Tension) {
            2
        } else if on(Layer::Drive) {
            1
        } else {
            0
        }
    }
}

fn pan_gains(pan: f32) -> (f32, f32) {
    let p = 0.5 * (pan.clamp(-1.0, 1.0) + 1.0);
    ((1.0 - p).sqrt(), p.sqrt())
}

const DECKS: usize = 3;

/// A look inside the engine, for the timeline tool and its tests.
#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    pub song: Option<SongId>,
    pub bar: u32,
    pub pass: u32,
    pub intensity: u8,
    /// The melody channel allowed to sing, if any.
    pub melody: Option<&'static str>,
    /// Decks sounding above -40 dB.
    pub decks: usize,
    /// The loudest enemy-count race channel's level, 0..1.
    pub race: f32,
}

/// Plays the songs. Owned by the output callback.
pub struct MusicEngine {
    songs: Arc<Vec<Song>>,
    control: Arc<MusicControl>,
    decks: [Deck; DECKS],
    current: usize,
    seen_generation: u32,
    muffle: Svf,
    muffle_amount: f32,
    sr: f32,
}

impl MusicEngine {
    pub fn new(songs: Arc<Vec<Song>>, control: Arc<MusicControl>, sr: f32) -> Self {
        Self {
            songs,
            control,
            decks: [Deck::new(sr), Deck::new(sr), Deck::new(sr)],
            current: 0,
            seen_generation: 0,
            muffle: Svf::default(),
            muffle_amount: 0.0,
            sr,
        }
    }

    fn song_index(&self, id: SongId) -> Option<usize> {
        self.songs.iter().position(|s| s.id == id)
    }

    /// Start a song (or silence): the playing one fades, the new one comes
    /// in on a free deck, so nothing still fading is cut. Asking for the
    /// song already playing keeps it (and calls off a waiting rest). A
    /// jingle cuts in; the harbour after a jingle rises under its last
    /// chord; a rest waits for the end of the phrase.
    fn begin(&mut self, id: Option<SongId>, state: &MusicState, after_jingle: bool) {
        let incoming = id.and_then(|id| self.song_index(id));
        let current = &mut self.decks[self.current];
        if id.is_none() && state.stop == Stop::Phrase {
            current.phrase_stop = true;
            current.frozen = true;
            return;
        }
        let same = incoming.is_some() && current.song == incoming && !current.mute_notes;
        if same && (current.fade_target > 0.0 || current.frozen) {
            current.phrase_stop = false;
            current.frozen = false;
            let sr = self.sr;
            current.fade_to(1.0, 0.5, sr);
            return;
        }
        let (out_seconds, in_seconds) = match id {
            Some(SongId::Victory | SongId::Defeat | SongId::Draw) => (0.3, 0.0),
            Some(SongId::Harbour) if after_jingle => (5.0, 3.0),
            Some(SongId::Harbour) => (2.5, 2.0),
            None if state.stop == Stop::Quick => (0.4, 0.0),
            None => (1.4, 0.0),
            // A match song arriving into a fight comes in at full level
            // on the entry's hit.
            Some(_)
                if state.intensity >= 3
                    && incoming.is_some_and(|i| self.songs[i].partner.is_some()) =>
            {
                (1.4, 0.0)
            }
            _ => (1.4, 2.0),
        };
        self.start_on_free_deck(incoming, state, out_seconds, in_seconds);
    }

    fn start_on_free_deck(
        &mut self,
        incoming: Option<usize>,
        state: &MusicState,
        out_seconds: f32,
        in_seconds: f32,
    ) {
        let sr = self.sr;
        self.decks[self.current].fade_out(out_seconds, sr);
        if let Some(index) = incoming {
            let next = (0..DECKS)
                .filter(|&d| d != self.current)
                .min_by(|&a, &b| {
                    let key = |d: usize| (self.decks[d].song.is_some(), self.decks[d].fade);
                    key(a)
                        .partial_cmp(&key(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or((self.current + 1) % DECKS);
            let song = &self.songs[index];
            // A match song arriving while the harbour is calm builds up
            // over four bars; asked for at a higher intensity it starts
            // there.
            let calm = if song.partner.is_some() && state.intensity <= 1 {
                4
            } else {
                0
            };
            self.decks[next].start(index, song, state, in_seconds, sr, calm);
            self.current = next;
            self.control.playing.store(song.id as u8, Ordering::Relaxed);
        } else {
            self.control.playing.store(NO_SONG, Ordering::Relaxed);
        }
    }

    /// The playing song's key offset from D, for effects in its key.
    pub fn key(&self) -> i8 {
        self.decks[self.current]
            .song
            .map_or(0, |i| self.songs[i].key)
    }

    /// Where the engine is.
    pub fn probe(&self) -> Probe {
        let deck = &self.decks[self.current];
        let song = deck.song.map(|i| &self.songs[i]);
        let melody = song.and_then(|song| {
            song.channels
                .iter()
                .enumerate()
                .find(|(c, ch)| ch.role == Role::Melody && deck.melody_on[*c])
                .map(|(_, ch)| ch.name)
        });
        Probe {
            song: song.map(|s| s.id),
            bar: song.map_or(0, |s| deck.row / s.rows_per_bar()),
            pass: deck.loops + 1,
            intensity: deck.effective_intensity(),
            melody,
            decks: self
                .decks
                .iter()
                .filter(|d| d.song.is_some() && d.level > 1e-4)
                .count(),
            race: song.map_or(0.0, |song| {
                song.channels
                    .iter()
                    .enumerate()
                    .filter(|(_, ch)| ch.layer == Layer::HoldRace)
                    .map(|(c, _)| deck.chan_gain[c])
                    .fold(0.0, f32::max)
            }),
        }
    }

    /// One stereo frame and the reverb send.
    pub fn next(&mut self) -> (f32, f32, f32) {
        let (state, generation) = self.control.read();
        if generation != self.seen_generation {
            self.seen_generation = generation;
            self.begin(state.song, &state, false);
        }
        let songs = Arc::clone(&self.songs);
        let mut l = 0.0;
        let mut r = 0.0;
        let mut verb = 0.0;
        let mut follow = None;
        for d in 0..DECKS {
            let deck = &mut self.decks[d];
            let Some((dl, dr, dv, rows_done, ended)) = deck.next(&songs, &state, self.sr) else {
                continue;
            };
            l += dl;
            r += dr;
            verb += dv;
            if rows_done && d == self.current {
                follow = deck.song.and_then(|i| songs[i].then);
            }
            if ended || (deck.fade <= 0.0 && deck.fade_target <= 0.0) {
                deck.song = None;
                deck.level = 0.0;
                // Silent now: nothing is playing until a song is asked.
                if d == self.current {
                    self.control.playing.store(NO_SONG, Ordering::Relaxed);
                }
            }
        }
        if let Some(id) = follow {
            self.begin(Some(id), &state, true);
        }
        // A hand-over bar has ended: the partner starts on this downbeat,
        // at once, and the old song's tails ring out.
        if self.decks[self.current].handover_due {
            let deck = &mut self.decks[self.current];
            deck.handover_due = false;
            let partner = deck
                .song
                .and_then(|i| songs[i].partner)
                .and_then(|id| self.song_index(id));
            self.start_on_free_deck(partner, &state, 0.3, 0.0);
        }

        let target = if state.muffled { 1.0 } else { 0.0 };
        self.muffle_amount += (target - self.muffle_amount) * (1.0 / (0.25 * self.sr));
        if self.muffle_amount > 0.001 {
            let cutoff = 18_000.0 * (1.0 - self.muffle_amount) + 700.0 * self.muffle_amount;
            let mono = 0.5 * (l + r);
            let side = 0.5 * (l - r) * (1.0 - self.muffle_amount);
            let low = self.muffle.process(mono, cutoff, 0.8, self.sr).low;
            let g = 1.0 - 0.45 * self.muffle_amount;
            l = (low + side) * g;
            r = (low - side) * g;
            verb *= 1.0 - 0.6 * self.muffle_amount;
        }
        (l, r, verb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_song() -> Song {
        let lead = Instrument::new(Osc::Pulse {
            duty: 0.25,
            sweep: 0.0,
        })
        .adsr(0.005, 0.1, 0.6, 0.1);
        let notes = (0..8)
            .map(|i| Note {
                row: i * 4,
                len: 3,
                key: 60 + (i % 5) as u8,
                vel: 0.8,
                slide: 0,
                arp: [0, 0],
                inst: 0,
                pass: 0,
            })
            .collect();
        Song {
            id: SongId::Title,
            name: "test",
            bpm: 120.0,
            rows_per_beat: 4,
            beats_per_bar: 4,
            swing: 0.0,
            bars: 2,
            loop_bar: Some(0),
            then: None,
            echo_rows: 3.0,
            echo_feedback: 0.3,
            gain: 1.0,
            channels: vec![Channel {
                name: "lead",
                inst: lead,
                variants: None,
                extra: vec![],
                layer: Layer::Core,
                pan: 0.0,
                gain: 1.0,
                echo: 0.2,
                reverb: 0.2,
                poly: false,
                spread: 0.0,
                role: Role::Melody,
                entry: None,
                notes,
            }],
            chords: vec![],
            partner: None,
            sections: vec![],
            handovers: vec![],
            key: 0,
        }
    }

    #[test]
    fn each_layer_answers_to_its_own_state() {
        let calm = layer_targets(&MusicState::default());
        assert_eq!(calm[Layer::Pad.index()], 1.0, "the pad plays when calm");
        assert_eq!(calm[Layer::HoldLast.index()], 0.0, "no count, no needle");
        assert_eq!(calm[Layer::Pressure.index()], 0.0);
        let last = layer_targets(&MusicState {
            intensity: 2,
            hold_theirs: true,
            hold_left: 8,
            ..MusicState::default()
        });
        assert_eq!(last[Layer::HoldLast.index()], 1.0);
        assert_eq!(last[Layer::HoldEnd.index()], 0.0);
        assert_eq!(
            last[Layer::Pad.index()],
            0.0,
            "the dread pad has taken over"
        );
        assert!(last[Layer::HoldRace.index()] > 0.9);
        assert_eq!(last[Layer::Pressure.index()], 1.0);
    }

    #[test]
    fn a_song_plays_loops_and_stops() {
        let control = Arc::new(MusicControl::new());
        let mut engine =
            MusicEngine::new(Arc::new(vec![tiny_song()]), Arc::clone(&control), 48_000.0);
        let mut energy = 0.0;
        for _ in 0..48_000 {
            let (l, r, _) = engine.next();
            energy += l * l + r * r;
        }
        assert_eq!(energy, 0.0, "silent until asked");
        control.set(
            MusicState {
                song: Some(SongId::Title),
                ..MusicState::default()
            },
            false,
        );
        let mut energy = 0.0;
        // Five seconds: past the two-bar (four second) loop point.
        for _ in 0..240_000 {
            let (l, r, v) = engine.next();
            assert!(l.is_finite() && r.is_finite() && v.is_finite());
            energy += l * l + r * r;
        }
        assert!(energy > 1.0, "the song sounds: {energy}");
        control.set(MusicState::default(), false);
        for _ in 0..240_000 {
            engine.next();
        }
        let mut tail = 0.0;
        for _ in 0..4_800 {
            let (l, r, _) = engine.next();
            tail += l.abs() + r.abs();
        }
        assert!(tail < 1e-3, "stopping fades to silence: {tail}");
    }
}
