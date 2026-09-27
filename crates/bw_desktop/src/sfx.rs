//! Sound effects as layered patches: each cue is up to six parts (a
//! transient, a body, a tail) with their own source, pitch sweep,
//! envelope and filter sweep, retriggered where a mechanism repeats.
//! Every play varies a little in pitch and level so repeated guns and
//! machines do not sound stamped. All original synthesis; no samples.

use crate::synth::{self, Lfsr, Noise as White, Pink, Svf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Src {
    Sine,
    Triangle,
    Square(f32),
    Saw,
    Noise,
    Pink,
    /// Console noise clocked at the part's frequency.
    Chip {
        short: bool,
    },
    /// Two-operator FM.
    Fm {
        ratio: f32,
        index: f32,
    },
    /// Six inharmonic square partials: struck plate, cymbal, clank.
    Metal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filter {
    None,
    Low,
    Band,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub src: Src,
    /// Seconds before the part starts.
    pub delay: f32,
    /// Pitch starts at `f0` and settles toward `f1` with time constant
    /// `glide` (0 holds `f0`).
    pub f0: f32,
    pub f1: f32,
    pub glide: f32,
    pub attack: f32,
    pub hold: f32,
    /// Exponential decay time constant after the hold.
    pub decay: f32,
    pub gain: f32,
    pub filter: Filter,
    pub c0: f32,
    pub c1: f32,
    pub ctc: f32,
    pub q: f32,
    pub drive: f32,
    /// Extra strikes and the gap between them.
    pub repeat: u8,
    pub every: f32,
    /// Each extra strike's level relative to the one before.
    pub fall: f32,
    /// Vibrato depth in semitones and its rate.
    pub wobble: f32,
    pub wobble_rate: f32,
    /// Level modulation depth (0..1) and rate.
    pub tremolo: f32,
    pub tremolo_rate: f32,
}

pub const fn part(src: Src) -> Part {
    Part {
        src,
        delay: 0.0,
        f0: 440.0,
        f1: 440.0,
        glide: 0.0,
        attack: 0.001,
        hold: 0.0,
        decay: 0.1,
        gain: 1.0,
        filter: Filter::None,
        c0: 0.0,
        c1: 0.0,
        ctc: 0.1,
        q: 0.707,
        drive: 0.0,
        repeat: 0,
        every: 0.0,
        fall: 1.0,
        wobble: 0.0,
        wobble_rate: 0.0,
        tremolo: 0.0,
        tremolo_rate: 0.0,
    }
}

impl Part {
    pub const fn hz(mut self, f: f32) -> Self {
        self.f0 = f;
        self.f1 = f;
        self
    }
    pub const fn sweep(mut self, f0: f32, f1: f32, glide: f32) -> Self {
        self.f0 = f0;
        self.f1 = f1;
        self.glide = glide;
        self
    }
    pub const fn env(mut self, attack: f32, hold: f32, decay: f32) -> Self {
        self.attack = attack;
        self.hold = hold;
        self.decay = decay;
        self
    }
    pub const fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }
    pub const fn at(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }
    pub const fn lp(mut self, c0: f32, c1: f32, ctc: f32, q: f32) -> Self {
        self.filter = Filter::Low;
        self.c0 = c0;
        self.c1 = c1;
        self.ctc = ctc;
        self.q = q;
        self
    }
    pub const fn bp(mut self, c0: f32, c1: f32, ctc: f32, q: f32) -> Self {
        self.filter = Filter::Band;
        self.c0 = c0;
        self.c1 = c1;
        self.ctc = ctc;
        self.q = q;
        self
    }
    pub const fn hp(mut self, c0: f32, c1: f32, ctc: f32, q: f32) -> Self {
        self.filter = Filter::High;
        self.c0 = c0;
        self.c1 = c1;
        self.ctc = ctc;
        self.q = q;
        self
    }
    pub const fn drive(mut self, drive: f32) -> Self {
        self.drive = drive;
        self
    }
    pub const fn repeat(mut self, times: u8, every: f32, fall: f32) -> Self {
        self.repeat = times;
        self.every = every;
        self.fall = fall;
        self
    }
    pub const fn wobble(mut self, semis: f32, rate: f32) -> Self {
        self.wobble = semis;
        self.wobble_rate = rate;
        self
    }
    pub const fn tremolo(mut self, depth: f32, rate: f32) -> Self {
        self.tremolo = depth;
        self.tremolo_rate = rate;
        self
    }

    /// Seconds until the part is 60 dB down.
    pub fn length(&self) -> f32 {
        self.delay
            + f32::from(self.repeat) * self.every
            + self.attack
            + self.hold
            + self.decay * 6.9
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Patch {
    pub parts: &'static [Part],
    pub gain: f32,
    pub reverb: f32,
    /// Random pitch spread per play, in semitones either way.
    pub vary: f32,
}

impl Patch {
    pub fn length(&self) -> f32 {
        self.parts
            .iter()
            .map(Part::length)
            .fold(0.0, f32::max)
            .min(7.0)
    }
}

pub const MAX_PARTS: usize = 6;

#[derive(Clone, Copy, Default)]
struct PartState {
    phase: f32,
    mod_phase: f32,
    svf: Svf,
    pink: Pink,
    chip: Lfsr,
    metal: [f32; 6],
}

/// One playing effect.
#[derive(Clone, Copy)]
pub struct Player {
    pub patch: Option<Patch>,
    t: f32,
    length: f32,
    pitch: f32,
    level: f32,
    noise: White,
    parts: [PartState; MAX_PARTS],
}

impl Default for Player {
    fn default() -> Self {
        Self {
            patch: None,
            t: 0.0,
            length: 0.0,
            pitch: 1.0,
            level: 1.0,
            noise: White(0x2545_f491),
            parts: [PartState::default(); MAX_PARTS],
        }
    }
}

impl Player {
    /// Start a patch `delay` seconds from now, `semis` semitones from its
    /// written pitch.
    pub fn start(&mut self, patch: Patch, seed: u32, delay: f32, semis: f32) {
        let mut noise = White(seed.wrapping_mul(0x9e37_79b9) | 1);
        let r = noise.next();
        let pitch = ((patch.vary * r + semis) * std::f32::consts::LN_2 / 12.0).exp();
        let level = 1.0 + 0.12 * noise.next() * f32::from(patch.vary > 0.0);
        *self = Self {
            patch: Some(patch),
            t: -delay.max(0.0),
            length: patch.length() + 0.01,
            pitch,
            level,
            noise,
            parts: [PartState::default(); MAX_PARTS],
        };
    }

    pub fn active(&self) -> bool {
        self.patch.is_some()
    }

    /// The next mono sample; the voice ends itself when done.
    #[inline]
    pub fn next(&mut self, sr: f32) -> f32 {
        let Some(patch) = self.patch else {
            return 0.0;
        };
        let t = self.t;
        self.t += 1.0 / sr;
        if t < 0.0 {
            return 0.0;
        }
        if t >= self.length {
            self.patch = None;
            return 0.0;
        }
        let mut sum = 0.0;
        for (i, p) in patch.parts.iter().take(MAX_PARTS).enumerate() {
            let local = t - p.delay;
            if local < 0.0 {
                continue;
            }
            let (strike, tl) = if p.repeat > 0 && p.every > 0.0 {
                let k = ((local / p.every) as u32).min(u32::from(p.repeat));
                (k, local - k as f32 * p.every)
            } else {
                (0, local)
            };
            let env = if tl < p.attack {
                tl / p.attack.max(0.000_1)
            } else if tl < p.attack + p.hold {
                1.0
            } else {
                (-(tl - p.attack - p.hold) / p.decay.max(0.000_5)).exp()
            };
            let strike_gain = p.fall.powi(strike as i32);
            if env * strike_gain < 0.000_2 && tl > p.attack {
                continue;
            }
            let mut f = if p.glide > 0.0 {
                p.f1 + (p.f0 - p.f1) * (-tl / p.glide).exp()
            } else {
                p.f0
            };
            if p.wobble != 0.0 {
                f *= (p.wobble * synth::sine(tl * p.wobble_rate) * std::f32::consts::LN_2 / 12.0)
                    .exp();
            }
            f *= self.pitch;
            let inc = (f / sr).min(0.45);
            let st = &mut self.parts[i];
            let white = self.noise.next();
            let raw = match p.src {
                Src::Sine => synth::sine(st.phase),
                Src::Triangle => synth::triangle(st.phase),
                Src::Square(duty) => synth::pulse(st.phase, inc, duty),
                Src::Saw => synth::saw(st.phase, inc),
                Src::Noise => white,
                Src::Pink => st.pink.next(white) * 2.5,
                Src::Chip { short } => st.chip.next(f, sr, short),
                Src::Fm { ratio, index } => {
                    // Keep the sidebands (Carson's rule) under Nyquist.
                    let room = ((0.45 / inc.max(1e-6)) - 1.0) / ratio.max(0.01) - 1.0;
                    let index = index.min(room.max(0.0));
                    let m = synth::sine(st.mod_phase);
                    st.mod_phase = (st.mod_phase + inc * ratio).fract();
                    synth::sine(st.phase + index * m / synth::TAU)
                }
                Src::Metal => {
                    const RATIOS: [f32; 6] = [1.0, 1.342, 1.768, 2.131, 2.553, 3.127];
                    let mut v = 0.0;
                    for (k, r) in RATIOS.iter().enumerate() {
                        st.metal[k] = (st.metal[k] + inc * r).fract();
                        v += synth::pulse(st.metal[k], (inc * r).min(0.45), 0.5);
                    }
                    v / 3.0
                }
            };
            st.phase = (st.phase + inc).fract();
            let mut v = raw;
            if p.filter != Filter::None {
                let c = p.c1 + (p.c0 - p.c1) * (-tl / p.ctc.max(0.001)).exp();
                let out = st.svf.process(v, c.max(20.0), p.q, sr);
                v = match p.filter {
                    Filter::Low => out.low,
                    Filter::Band => out.band,
                    Filter::High => out.high,
                    Filter::None => v,
                };
            }
            if p.drive > 0.0 {
                let k = 1.0 + p.drive;
                v = (v * k).tanh() / k.tanh();
            }
            if p.tremolo > 0.0 {
                v *= 1.0 - p.tremolo * 0.5 * (1.0 + synth::sine(tl * p.tremolo_rate));
            }
            sum += v * env * p.gain * strike_gain;
        }
        // A short fade at the very end keeps a cut tail from clicking.
        let tail = ((self.length - t) / 0.004).min(1.0);
        sum * patch.gain * self.level * tail
    }
}

// ---------------------------------------------------------------------
// The patches. Levels are set against each other so a rivet gun sits
// under a cannon, an order click under both, and alerts above all.

use Src::*;

const fn p(parts: &'static [Part], gain: f32, reverb: f32, vary: f32) -> Patch {
    Patch {
        parts,
        gain,
        reverb,
        vary,
    }
}

// Commands and interface.
pub const ORDER_SUBMIT: Patch = p(
    &[
        part(Square(0.25))
            .hz(1_760.0)
            .env(0.0005, 0.004, 0.012)
            .lp(6_000.0, 6_000.0, 1.0, 0.7),
        part(Noise)
            .env(0.0003, 0.0, 0.004)
            .hp(4_000.0, 4_000.0, 1.0, 0.7)
            .gain(0.4),
    ],
    0.09,
    0.05,
    0.3,
);
pub const ORDER_ACCEPTED: Patch = p(
    &[
        part(Square(0.5))
            .hz(880.0)
            .env(0.001, 0.022, 0.018)
            .lp(4_500.0, 2_500.0, 0.05, 0.8),
        part(Square(0.5))
            .hz(1_318.5)
            .at(0.045)
            .env(0.001, 0.03, 0.03)
            .lp(4_500.0, 2_500.0, 0.05, 0.8),
    ],
    0.075,
    0.08,
    0.0,
);
pub const ORDER_REJECTED: Patch = p(
    &[part(Square(0.5))
        .sweep(233.0, 196.0, 0.05)
        .env(0.002, 0.05, 0.03)
        .lp(1_800.0, 900.0, 0.06, 0.9)
        .repeat(1, 0.1, 0.8)],
    0.045,
    0.05,
    0.0,
);
/// Starting something: a two-note chirp up a fourth.
pub const UI_CONFIRM: Patch = p(
    &[
        part(Square(0.25))
            .hz(1_174.7)
            .env(0.001, 0.02, 0.02)
            .lp(4_000.0, 2_500.0, 0.03, 0.8),
        part(Square(0.25))
            .hz(1_568.0)
            .at(0.05)
            .env(0.001, 0.03, 0.04)
            .lp(4_000.0, 2_500.0, 0.03, 0.8),
    ],
    0.07,
    0.08,
    0.0,
);
/// Going back: one soft note falling.
pub const UI_BACK: Patch = p(
    &[part(Triangle)
        .sweep(1_318.5, 880.0, 0.03)
        .env(0.001, 0.01, 0.03)],
    0.08,
    0.05,
    0.0,
);
pub const UI_CLICK: Patch = p(
    &[
        part(Triangle)
            .sweep(2_400.0, 1_600.0, 0.01)
            .env(0.0005, 0.0, 0.012),
        part(Noise)
            .env(0.0002, 0.0, 0.003)
            .hp(5_000.0, 5_000.0, 1.0, 0.7)
            .gain(0.3),
    ],
    0.08,
    0.02,
    0.15,
);
pub const SELECT_UNION: Patch = p(
    &[
        part(Metal)
            .hz(1_900.0)
            .env(0.0005, 0.0, 0.018)
            .hp(2_500.0, 2_500.0, 1.0, 0.8)
            .gain(0.6),
        part(Square(0.25))
            .hz(587.3)
            .at(0.02)
            .env(0.001, 0.02, 0.025)
            .lp(3_000.0, 1_500.0, 0.03, 0.8),
    ],
    0.07,
    0.05,
    0.0,
);
/// A wooden tok landing on D and then A.
pub const SELECT_ASSEMBLY: Patch = p(
    &[
        part(Sine).sweep(900.0, 587.3, 0.012).env(0.0005, 0.0, 0.03),
        part(Sine)
            .sweep(1_300.0, 880.0, 0.012)
            .at(0.05)
            .env(0.0005, 0.0, 0.03)
            .gain(0.7),
    ],
    0.08,
    0.06,
    0.0,
);
pub const SELECT_COMPACT: Patch = p(
    &[part(Fm {
        ratio: 2.0,
        index: 0.9,
    })
    .hz(1_174.7)
    .env(0.0005, 0.0, 0.07)],
    0.06,
    0.12,
    0.0,
);

// Guns.
pub const SHOT: Patch = p(
    &[
        part(Triangle)
            .sweep(420.0, 110.0, 0.02)
            .env(0.0005, 0.0, 0.04)
            .drive(0.6)
            .gain(0.4),
        part(Chip { short: false })
            .sweep(4_000.0, 1_200.0, 0.03)
            .env(0.0005, 0.005, 0.03)
            .lp(7_000.0, 2_500.0, 0.03, 0.8)
            .gain(0.8),
        part(Noise)
            .env(0.0005, 0.0, 0.03)
            .bp(3_000.0, 1_200.0, 0.03, 0.9)
            .gain(0.9),
    ],
    0.13,
    0.08,
    1.0,
);
/// The Riveter: a pneumatic gun, three rivets in a breath.
pub const SHOT_RIVET: Patch = p(
    &[
        part(Metal)
            .hz(1_050.0)
            .env(0.0003, 0.0, 0.016)
            .hp(1_800.0, 1_800.0, 1.0, 0.8)
            .repeat(2, 0.055, 0.8),
        part(Square(0.5))
            .sweep(180.0, 70.0, 0.015)
            .env(0.0005, 0.0, 0.025)
            .drive(0.8)
            .repeat(2, 0.055, 0.8)
            .gain(0.8),
        part(Noise)
            .env(0.0003, 0.0, 0.012)
            .hp(3_500.0, 3_500.0, 1.0, 0.7)
            .repeat(2, 0.055, 0.8)
            .gain(0.5),
    ],
    0.12,
    0.06,
    0.8,
);
/// The Bulwark: a heavy gun that thumps the chest, with a chip-noise
/// crack a small speaker can carry.
pub const SHOT_CANNON: Patch = p(
    &[
        part(Sine)
            .sweep(150.0, 70.0, 0.05)
            .env(0.001, 0.02, 0.12)
            .drive(1.6)
            .gain(0.15),
        part(Chip { short: false })
            .sweep(2_800.0, 500.0, 0.1)
            .env(0.001, 0.04, 0.12)
            .lp(6_000.0, 2_000.0, 0.1, 0.8)
            .gain(1.2),
        part(Noise)
            .env(0.001, 0.0, 0.18)
            .lp(2_400.0, 1_200.0, 0.08, 0.8)
            .drive(0.5)
            .gain(0.7),
        // The breech clanks shut.
        part(Metal)
            .hz(900.0)
            .at(0.03)
            .env(0.0005, 0.0, 0.03)
            .bp(2_000.0, 2_000.0, 1.0, 1.0)
            .gain(0.35),
        part(Noise)
            .env(0.0003, 0.0, 0.006)
            .hp(2_000.0, 2_000.0, 1.0, 0.7)
            .gain(0.6),
    ],
    0.2,
    0.2,
    0.6,
);
/// Sounders, Skippers and scouts: a snapping light gun.
pub const SHOT_LIGHT: Patch = p(
    &[
        part(Square(0.25))
            .sweep(1_600.0, 320.0, 0.022)
            .env(0.0005, 0.0, 0.04)
            .lp(5_000.0, 1_600.0, 0.03, 0.8),
        part(Noise)
            .env(0.0003, 0.0, 0.02)
            .hp(2_500.0, 2_500.0, 1.0, 0.7)
            .gain(0.6),
    ],
    0.1,
    0.06,
    1.2,
);
/// The Reedguard: a sprung reed dart, a whip and a pop.
pub const SHOT_REED: Patch = p(
    &[
        part(Pink)
            .env(0.004, 0.0, 0.05)
            .bp(900.0, 3_600.0, 0.03, 2.0)
            .gain(1.4),
        part(Triangle)
            .sweep(700.0, 180.0, 0.03)
            .env(0.001, 0.0, 0.045),
    ],
    0.12,
    0.07,
    1.0,
);
/// The Tower: a medium mounted gun.
pub const SHOT_TOWER: Patch = p(
    &[
        part(Triangle)
            .sweep(300.0, 90.0, 0.03)
            .env(0.0005, 0.0, 0.06)
            .drive(1.0)
            .gain(0.45),
        part(Chip { short: false })
            .sweep(3_200.0, 900.0, 0.05)
            .env(0.0005, 0.015, 0.05)
            .lp(7_000.0, 1_800.0, 0.05, 0.8)
            .gain(1.0),
        part(Noise)
            .env(0.0005, 0.0, 0.06)
            .bp(1_600.0, 600.0, 0.05, 0.9),
    ],
    0.14,
    0.12,
    0.7,
);
/// The Brander's fire-lance: a short roar with crackle.
pub const SHOT_LANCE: Patch = p(
    &[
        part(Pink)
            .env(0.015, 0.05, 0.12)
            .bp(500.0, 1_700.0, 0.08, 0.9)
            .gain(1.6),
        part(Chip { short: false })
            .hz(2_800.0)
            .env(0.005, 0.03, 0.06)
            .hp(2_000.0, 2_000.0, 1.0, 0.7)
            .gain(0.35),
    ],
    0.13,
    0.08,
    0.8,
);
/// The Glinter: a shard of light, a glassy zap.
pub const SHOT_SHARD: Patch = p(
    &[
        part(Fm {
            ratio: 2.76,
            index: 3.0,
        })
        .sweep(2_600.0, 1_200.0, 0.04)
        .env(0.0005, 0.0, 0.06),
        part(Noise)
            .env(0.0003, 0.0, 0.01)
            .hp(6_000.0, 6_000.0, 1.0, 0.7)
            .gain(0.4),
    ],
    0.1,
    0.12,
    1.0,
);
/// A worker's tool striking.
pub const SHOT_TOOL: Patch = p(
    &[part(Metal)
        .hz(1_300.0)
        .env(0.0003, 0.0, 0.03)
        .bp(2_600.0, 2_600.0, 1.0, 1.2)],
    0.06,
    0.05,
    1.5,
);
/// The Loom's shell leaving: a deep thump, a rising hiss.
pub const LOOM_LAUNCH: Patch = p(
    &[
        part(Sine)
            .sweep(120.0, 58.0, 0.06)
            .env(0.001, 0.02, 0.16)
            .drive(1.2),
        part(Chip { short: false })
            .sweep(1_800.0, 500.0, 0.08)
            .env(0.001, 0.02, 0.08)
            .lp(4_000.0, 900.0, 0.08, 0.8)
            .gain(0.5),
        part(Pink)
            .env(0.02, 0.05, 0.25)
            .bp(400.0, 2_200.0, 0.3, 1.2)
            .gain(0.8),
    ],
    0.17,
    0.2,
    0.4,
);
/// A shell falling on its mark.
pub const SHELL_WHISTLE: Patch = p(
    &[
        part(Sine)
            .sweep(2_300.0, 700.0, 0.7)
            .env(0.08, 0.5, 0.15)
            .wobble(0.15, 13.0),
        part(Noise)
            .env(0.08, 0.5, 0.15)
            .bp(2_300.0, 700.0, 0.45, 3.0)
            .gain(0.5),
    ],
    0.11,
    0.25,
    0.3,
);
pub const HELIOSTAT_BEAM: Patch = p(
    &[
        part(Saw)
            .sweep(196.0, 262.0, 0.3)
            .env(0.06, 0.2, 0.022)
            .lp(900.0, 2_600.0, 0.2, 1.4),
        part(Sine)
            .sweep(394.0, 527.0, 0.3)
            .env(0.06, 0.2, 0.022)
            .gain(0.5),
        part(Fm {
            ratio: 5.03,
            index: 0.8,
        })
        .sweep(785.0, 1_050.0, 0.3)
        .env(0.12, 0.13, 0.022)
        .gain(0.25),
    ],
    0.08,
    0.15,
    0.0,
);

// Hits and deaths.
pub const IMPACT: Patch = p(
    &[
        part(Noise)
            .env(0.001, 0.0, 0.16)
            .lp(3_200.0, 400.0, 0.07, 0.8)
            .drive(0.8),
        part(Chip { short: false })
            .sweep(2_400.0, 400.0, 0.1)
            .env(0.001, 0.03, 0.12)
            .lp(5_000.0, 900.0, 0.1, 0.8)
            .gain(0.7),
        part(Sine)
            .sweep(130.0, 60.0, 0.05)
            .env(0.001, 0.0, 0.1)
            .drive(1.0)
            .gain(0.45),
        part(Metal)
            .hz(780.0)
            .at(0.07)
            .env(0.0005, 0.0, 0.02)
            .bp(2_200.0, 2_200.0, 1.0, 1.0)
            .repeat(2, 0.075, 0.6)
            .gain(0.35),
    ],
    0.18,
    0.2,
    0.8,
);
pub const EXPLOSION_LARGE: Patch = p(
    &[
        part(Noise)
            .env(0.002, 0.03, 0.4)
            .lp(2_600.0, 300.0, 0.2, 0.8)
            .drive(1.2),
        part(Sine)
            .sweep(110.0, 55.0, 0.12)
            .env(0.002, 0.04, 0.25)
            .drive(1.4)
            .gain(0.45),
        part(Chip { short: false })
            .sweep(2_200.0, 350.0, 0.25)
            .env(0.002, 0.08, 0.3)
            .lp(6_000.0, 800.0, 0.2, 0.7)
            .gain(0.9),
        part(Metal)
            .hz(640.0)
            .at(0.18)
            .env(0.0005, 0.0, 0.03)
            .bp(1_800.0, 1_800.0, 1.0, 1.0)
            .repeat(3, 0.11, 0.65)
            .gain(0.3),
    ],
    0.24,
    0.3,
    0.5,
);
pub const BUILDING_COLLAPSE: Patch = p(
    &[
        part(Pink)
            .env(0.03, 0.2, 0.7)
            .lp(1_200.0, 160.0, 0.5, 0.8)
            .drive(0.8)
            .gain(1.6),
        part(Sine)
            .sweep(90.0, 55.0, 0.5)
            .env(0.02, 0.1, 0.4)
            .drive(1.0)
            .gain(0.45),
        part(Chip { short: false })
            .sweep(1_600.0, 260.0, 0.6)
            .env(0.01, 0.2, 0.5)
            .lp(4_000.0, 700.0, 0.4, 0.7)
            .gain(0.6),
        part(Saw)
            .sweep(96.0, 58.0, 0.9)
            .env(0.2, 0.3, 0.5)
            .lp(420.0, 300.0, 0.4, 1.4)
            .wobble(0.4, 3.0)
            .gain(0.5),
        part(Metal)
            .hz(520.0)
            .at(0.25)
            .env(0.0005, 0.0, 0.04)
            .bp(1_500.0, 1_500.0, 1.0, 1.0)
            .repeat(5, 0.13, 0.75)
            .gain(0.4),
    ],
    0.24,
    0.35,
    0.3,
);

// What breaks when a machine dies, by who built it.
/// The Union: plate iron clanging and ringing.
pub const DEBRIS_METAL: Patch = p(
    &[
        part(Metal)
            .hz(740.0)
            .at(0.04)
            .env(0.0005, 0.0, 0.09)
            .bp(2_200.0, 1_600.0, 0.2, 1.4),
        part(Metal)
            .hz(1_120.0)
            .at(0.16)
            .env(0.0005, 0.0, 0.06)
            .bp(3_000.0, 2_000.0, 0.2, 1.4)
            .gain(0.6),
    ],
    0.1,
    0.2,
    0.6,
);
/// The Assembly: woven reed snapping.
pub const DEBRIS_REED: Patch = p(
    &[part(Pink)
        .at(0.03)
        .env(0.001, 0.0, 0.012)
        .bp(3_000.0, 1_500.0, 0.15, 2.0)
        .repeat(5, 0.025, 0.85)
        .gain(1.6)],
    0.12,
    0.1,
    0.8,
);
/// The Compact: glass shattering in falling notes.
pub const DEBRIS_GLASS: Patch = p(
    &[
        part(Fm {
            ratio: 2.756,
            index: 1.5,
        })
        .hz(5_800.0)
        .at(0.03)
        .env(0.0005, 0.0, 0.05),
        part(Fm {
            ratio: 2.756,
            index: 1.5,
        })
        .hz(4_700.0)
        .at(0.07)
        .env(0.0005, 0.0, 0.05)
        .gain(0.8),
        part(Fm {
            ratio: 2.756,
            index: 1.5,
        })
        .hz(3_900.0)
        .at(0.11)
        .env(0.0005, 0.0, 0.06)
        .gain(0.7),
        part(Fm {
            ratio: 2.756,
            index: 1.5,
        })
        .hz(3_100.0)
        .at(0.16)
        .env(0.0005, 0.0, 0.08)
        .gain(0.6),
        part(Noise)
            .at(0.03)
            .env(0.001, 0.0, 0.04)
            .hp(6_000.0, 6_000.0, 1.0, 0.7)
            .gain(0.4),
    ],
    0.06,
    0.2,
    0.6,
);

// Building and making.
pub const BUILD_PLACED: Patch = p(
    &[
        part(Sine)
            .sweep(220.0, 110.0, 0.02)
            .env(0.0005, 0.0, 0.06)
            .drive(0.6),
        part(Noise)
            .env(0.0005, 0.0, 0.025)
            .lp(2_600.0, 900.0, 0.02, 0.8)
            .gain(0.6),
        part(Metal)
            .hz(880.0)
            .at(0.1)
            .env(0.0005, 0.0, 0.022)
            .bp(2_000.0, 2_000.0, 1.0, 1.1)
            .repeat(2, 0.06, 0.7)
            .gain(0.35),
    ],
    0.13,
    0.12,
    0.3,
);
/// A building finished: a seated clunk, then a bright chip fanfare in D.
pub const BUILD_DONE: Patch = p(
    &[
        part(Noise)
            .env(0.001, 0.0, 0.07)
            .lp(1_800.0, 300.0, 0.05, 0.8)
            .drive(0.6),
        part(Sine)
            .sweep(150.0, 90.0, 0.04)
            .env(0.001, 0.0, 0.1)
            .drive(0.8),
        part(Square(0.25))
            .hz(587.3)
            .at(0.12)
            .env(0.001, 0.05, 0.05)
            .lp(3_600.0, 2_000.0, 0.05, 0.8)
            .gain(0.55),
        part(Square(0.25))
            .hz(880.0)
            .at(0.18)
            .env(0.001, 0.05, 0.05)
            .lp(3_600.0, 2_000.0, 0.05, 0.8)
            .gain(0.55),
        part(Square(0.25))
            .hz(1_174.7)
            .at(0.24)
            .env(0.001, 0.09, 0.14)
            .lp(3_600.0, 2_000.0, 0.05, 0.8)
            .gain(0.6),
    ],
    0.13,
    0.18,
    0.0,
);
/// A machine released: a hiss of pressure and a rising fifth.
pub const UNIT_READY: Patch = p(
    &[
        part(Pink)
            .env(0.008, 0.05, 0.1)
            .hp(1_800.0, 3_500.0, 0.1, 0.7)
            .gain(0.9),
        part(Square(0.5))
            .hz(784.0)
            .at(0.05)
            .env(0.001, 0.035, 0.03)
            .lp(3_000.0, 1_800.0, 0.04, 0.8)
            .gain(0.6),
        part(Square(0.5))
            .hz(1_174.7)
            .at(0.12)
            .env(0.001, 0.06, 0.09)
            .lp(3_000.0, 1_800.0, 0.04, 0.8)
            .gain(0.6),
    ],
    0.1,
    0.12,
    0.0,
);
/// Research or an upgrade done: a glass arpeggio rising in D major.
pub const RESEARCH_DONE: Patch = p(
    &[
        part(Fm {
            ratio: 3.5,
            index: 1.4,
        })
        .hz(1_174.7)
        .env(0.0005, 0.0, 0.25),
        part(Fm {
            ratio: 3.5,
            index: 1.4,
        })
        .hz(1_318.5)
        .at(0.07)
        .env(0.0005, 0.0, 0.25),
        part(Fm {
            ratio: 3.5,
            index: 1.4,
        })
        .hz(1_760.0)
        .at(0.14)
        .env(0.0005, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.4,
        })
        .hz(2_349.3)
        .at(0.21)
        .env(0.0005, 0.0, 0.5),
    ],
    0.06,
    0.3,
    0.0,
);

// Abilities and machinery.
pub const VENT: Patch = p(
    &[
        part(Pink)
            .env(0.015, 0.25, 0.3)
            .hp(1_200.0, 3_200.0, 0.3, 0.7)
            .gain(1.4),
        part(Noise)
            .env(0.01, 0.2, 0.25)
            .bp(4_500.0, 3_000.0, 0.3, 1.5)
            .gain(0.5),
        part(Sine)
            .sweep(160.0, 110.0, 0.2)
            .env(0.005, 0.1, 0.2)
            .gain(0.4),
    ],
    0.13,
    0.2,
    0.4,
);
/// SOUND: a sonar ping that answers itself.
pub const SONAR: Patch = p(
    &[
        part(Sine)
            .hz(1_318.5)
            .env(0.002, 0.02, 0.18)
            .wobble(0.05, 7.0),
        part(Sine).hz(2_637.0).env(0.002, 0.0, 0.06).gain(0.25),
        part(Sine)
            .hz(1_318.5)
            .at(0.42)
            .env(0.002, 0.01, 0.2)
            .gain(0.4),
        part(Sine)
            .hz(1_318.5)
            .at(0.84)
            .env(0.002, 0.0, 0.2)
            .gain(0.18),
    ],
    0.1,
    0.45,
    0.0,
);
pub const GLINT: Patch = p(
    &[
        part(Fm {
            ratio: 1.5,
            index: 3.5,
        })
        .sweep(2_600.0, 5_200.0, 0.12)
        .env(0.01, 0.05, 0.14),
        part(Noise)
            .env(0.02, 0.05, 0.1)
            .hp(7_000.0, 7_000.0, 1.0, 0.7)
            .gain(0.3),
    ],
    0.06,
    0.3,
    0.3,
);
pub const SURGE: Patch = p(
    &[
        part(Pink)
            .env(0.25, 0.2, 0.35)
            .lp(300.0, 3_200.0, 0.3, 0.9)
            .gain(1.6),
        part(Sine)
            .sweep(90.0, 180.0, 0.4)
            .env(0.2, 0.2, 0.3)
            .gain(0.4),
    ],
    0.13,
    0.25,
    0.3,
);
pub const DEPLOY: Patch = p(
    &[
        part(Metal)
            .hz(420.0)
            .env(0.0005, 0.0, 0.04)
            .bp(1_400.0, 1_400.0, 1.0, 1.0),
        part(Square(0.5))
            .sweep(95.0, 55.0, 0.08)
            .env(0.001, 0.02, 0.08)
            .lp(700.0, 300.0, 0.08, 1.0),
        part(Pink)
            .at(0.08)
            .env(0.02, 0.1, 0.12)
            .hp(2_000.0, 2_000.0, 1.0, 0.7)
            .gain(0.6),
        part(Metal)
            .hz(300.0)
            .at(0.32)
            .env(0.0005, 0.0, 0.05)
            .bp(1_000.0, 1_000.0, 1.0, 1.0)
            .gain(0.9),
    ],
    0.13,
    0.12,
    0.3,
);
pub const HATCH: Patch = p(
    &[
        part(Metal)
            .hz(610.0)
            .env(0.0005, 0.0, 0.05)
            .bp(1_800.0, 1_800.0, 1.0, 1.0),
        part(Noise)
            .at(0.03)
            .env(0.02, 0.05, 0.08)
            .bp(900.0, 1_600.0, 0.1, 1.2)
            .gain(0.5),
    ],
    0.11,
    0.1,
    0.5,
);
pub const RECLAIM: Patch = p(
    &[
        part(Metal)
            .hz(1_500.0)
            .env(0.0005, 0.0, 0.018)
            .bp(3_000.0, 3_000.0, 1.0, 1.0)
            .repeat(7, 0.05, 0.92),
        part(Saw)
            .sweep(110.0, 150.0, 0.3)
            .env(0.05, 0.25, 0.1)
            .lp(600.0, 600.0, 1.0, 1.0)
            .gain(0.3),
    ],
    0.09,
    0.1,
    0.4,
);
pub const REPAIR: Patch = p(
    &[
        part(Chip { short: true })
            .hz(5_500.0)
            .env(0.001, 0.015, 0.02)
            .hp(2_500.0, 2_500.0, 1.0, 0.7)
            .repeat(3, 0.08, 0.8),
        part(Fm {
            ratio: 1.41,
            index: 2.0,
        })
        .sweep(2_200.0, 1_400.0, 0.03)
        .env(0.001, 0.0, 0.04)
        .repeat(3, 0.08, 0.8)
        .gain(0.5),
    ],
    0.06,
    0.08,
    0.6,
);
pub const SPLASH: Patch = p(
    &[
        part(Pink)
            .env(0.003, 0.02, 0.18)
            .lp(3_500.0, 450.0, 0.1, 0.8)
            .gain(1.5),
        part(Sine)
            .sweep(520.0, 160.0, 0.08)
            .env(0.002, 0.0, 0.08)
            .wobble(1.5, 23.0)
            .gain(0.5),
        part(Sine)
            .sweep(700.0, 300.0, 0.05)
            .at(0.12)
            .env(0.002, 0.0, 0.05)
            .gain(0.3),
    ],
    0.13,
    0.2,
    1.0,
);
pub const SIZZLE: Patch = p(
    &[
        part(Pink)
            .env(0.05, 0.3, 0.3)
            .hp(2_800.0, 4_200.0, 0.4, 0.7)
            .gain(1.2),
        part(Chip { short: false })
            .hz(3_200.0)
            .env(0.05, 0.2, 0.3)
            .bp(5_000.0, 5_000.0, 1.0, 1.0)
            .gain(0.3),
    ],
    0.08,
    0.15,
    0.5,
);
pub const SALT_LAID: Patch = p(
    &[part(Noise)
        .env(0.001, 0.0, 0.035)
        .bp(2_600.0, 1_800.0, 0.03, 1.2)
        .repeat(2, 0.07, 0.8)],
    0.1,
    0.08,
    1.0,
);

// Alerts.
/// B6 and F6, a tritone inside the key, with a fast tremolo, above the
/// fight's band: nothing in the score or the guns sounds like it.
pub const UNDER_ATTACK: Patch = p(
    &[
        part(Square(0.5))
            .hz(1_975.5)
            .env(0.002, 0.1, 0.02)
            .lp(5_000.0, 5_000.0, 1.0, 0.8)
            .repeat(2, 0.26, 0.85)
            .tremolo(0.6, 14.0),
        part(Square(0.5))
            .hz(1_396.9)
            .at(0.13)
            .env(0.002, 0.1, 0.02)
            .lp(5_000.0, 5_000.0, 1.0, 0.8)
            .repeat(2, 0.26, 0.85)
            .tremolo(0.6, 14.0),
    ],
    0.18,
    0.12,
    0.0,
);
/// The buoy bell at the sluice.
pub const GATE_BELL: Patch = p(
    &[
        part(Fm {
            ratio: 2.76,
            index: 1.6,
        })
        .hz(880.0)
        .env(0.001, 0.0, 0.45),
        part(Sine).hz(440.0).env(0.001, 0.0, 0.6).gain(0.6),
        part(Sine).hz(1_320.0).env(0.001, 0.0, 0.2).gain(0.3),
        part(Noise)
            .env(0.0003, 0.0, 0.004)
            .bp(3_000.0, 3_000.0, 1.0, 1.0)
            .gain(0.4),
    ],
    0.13,
    0.35,
    0.0,
);
/// The sluice turns: iron groans, then the water goes.
pub const GATE_RUSH: Patch = p(
    &[
        part(Metal)
            .hz(210.0)
            .env(0.001, 0.0, 0.08)
            .bp(900.0, 900.0, 1.0, 1.0)
            .gain(0.6),
        part(Saw)
            .sweep(72.0, 54.0, 0.6)
            .env(0.1, 0.4, 0.3)
            .lp(380.0, 260.0, 0.3, 1.5)
            .wobble(0.3, 4.0)
            .gain(0.45),
        part(Pink)
            .at(0.25)
            .env(0.4, 0.5, 0.55)
            .lp(250.0, 4_200.0, 0.5, 0.8)
            .gain(1.8),
        part(Sine).at(0.25).hz(62.0).env(0.3, 0.5, 0.5).gain(0.4),
    ],
    0.16,
    0.35,
    0.0,
);
pub const CAPTURE_ALARM: Patch = p(
    &[part(Square(0.3))
        .sweep(311.0, 277.0, 0.1)
        .env(0.003, 0.08, 0.04)
        .lp(2_200.0, 1_200.0, 0.1, 1.2)
        .repeat(2, 0.14, 0.85)],
    0.08,
    0.1,
    0.0,
);
/// Taken: the motif rising to the Dorian sixth, A–D–E–B, the colour of
/// reclaimed land.
pub const SLUICE_TAKEN: Patch = p(
    &[
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(440.0)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(587.3)
        .at(0.1)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(659.3)
        .at(0.2)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(987.8)
        .at(0.3)
        .env(0.001, 0.0, 0.6),
        part(Square(0.25))
            .hz(293.7)
            .at(0.3)
            .env(0.01, 0.2, 0.2)
            .lp(1_500.0, 1_000.0, 0.1, 0.8)
            .gain(0.4),
    ],
    0.1,
    0.3,
    0.0,
);
/// Lost: a falling line through the flat second, A–F–Eb–D, the water
/// taking it back, and a low thud.
pub const SLUICE_LOST: Patch = p(
    &[
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(880.0)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(698.5)
        .at(0.12)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(622.3)
        .at(0.24)
        .env(0.001, 0.0, 0.3),
        part(Fm {
            ratio: 3.5,
            index: 1.2,
        })
        .hz(587.3)
        .at(0.36)
        .env(0.001, 0.0, 0.7),
        part(Sine)
            .sweep(120.0, 60.0, 0.1)
            .at(0.36)
            .env(0.002, 0.05, 0.22)
            .drive(0.8)
            .gain(0.7),
    ],
    0.1,
    0.3,
    0.0,
);
pub const SWITCH_CANCELLED: Patch = p(
    &[
        part(Metal)
            .sweep(420.0, 180.0, 0.2)
            .env(0.001, 0.05, 0.12)
            .bp(1_200.0, 500.0, 0.2, 1.4)
            .drive(0.8),
        part(Saw)
            .sweep(140.0, 60.0, 0.25)
            .env(0.005, 0.1, 0.15)
            .lp(800.0, 300.0, 0.2, 1.2)
            .gain(0.6),
        part(Noise)
            .env(0.001, 0.05, 0.1)
            .bp(2_000.0, 700.0, 0.2, 0.9)
            .gain(0.4),
    ],
    0.14,
    0.2,
    0.0,
);
pub const HOLD_BEGUN_OURS: Patch = p(
    &[
        part(Square(0.25))
            .hz(587.3)
            .env(0.004, 0.08, 0.06)
            .lp(3_000.0, 1_600.0, 0.08, 0.8),
        part(Square(0.25))
            .hz(880.0)
            .at(0.11)
            .env(0.004, 0.08, 0.06)
            .lp(3_000.0, 1_600.0, 0.08, 0.8),
        part(Square(0.25))
            .hz(1_174.7)
            .at(0.22)
            .env(0.004, 0.25, 0.25)
            .lp(3_000.0, 1_600.0, 0.08, 0.8)
            .wobble(0.12, 6.0),
        part(Fm {
            ratio: 3.5,
            index: 1.0,
        })
        .hz(1_174.7)
        .at(0.22)
        .env(0.001, 0.0, 0.6)
        .gain(0.5),
    ],
    0.1,
    0.25,
    0.0,
);
pub const HOLD_BEGUN_ENEMY: Patch = p(
    &[
        part(Fm {
            ratio: 2.76,
            index: 2.2,
        })
        .hz(146.8)
        .env(0.001, 0.0, 0.9),
        part(Sine).hz(146.8).env(0.01, 0.1, 0.8).gain(0.5),
        part(Fm {
            ratio: 2.76,
            index: 1.2,
        })
        .hz(155.6)
        .at(0.5)
        .env(0.001, 0.0, 0.8)
        .gain(0.6),
        part(Square(0.25))
            .hz(293.7)
            .env(0.01, 0.2, 0.3)
            .lp(1_200.0, 700.0, 0.2, 0.8)
            .gain(0.3),
    ],
    0.16,
    0.4,
    0.0,
);
pub const HOLD_BROKEN_OURS: Patch = p(
    &[
        part(Square(0.25))
            .hz(880.0)
            .env(0.004, 0.08, 0.06)
            .lp(2_400.0, 1_200.0, 0.08, 0.8),
        part(Square(0.25))
            .hz(698.5)
            .at(0.12)
            .env(0.004, 0.08, 0.06)
            .lp(2_400.0, 1_200.0, 0.08, 0.8),
        part(Square(0.25))
            .sweep(554.4, 523.3, 0.3)
            .at(0.24)
            .env(0.004, 0.2, 0.2)
            .lp(2_400.0, 1_000.0, 0.1, 0.8),
    ],
    0.1,
    0.2,
    0.0,
);
pub const HOLD_BROKEN_ENEMY: Patch = p(
    &[
        part(Triangle).hz(587.3).env(0.004, 0.06, 0.06),
        part(Triangle).hz(659.3).at(0.1).env(0.004, 0.06, 0.06),
        part(Triangle).hz(880.0).at(0.2).env(0.004, 0.15, 0.2),
        part(Fm {
            ratio: 3.5,
            index: 1.0,
        })
        .hz(1_760.0)
        .at(0.2)
        .env(0.001, 0.0, 0.4)
        .gain(0.35),
    ],
    0.12,
    0.2,
    0.0,
);
/// Our count ticking: a bright double strike on D6 and a tick of noise,
/// unlike anything the hold music plays.
pub const HOLD_TOLL_OURS: Patch = p(
    &[
        part(Square(0.25))
            .hz(1_174.7)
            .env(0.001, 0.03, 0.03)
            .lp(5_000.0, 3_000.0, 0.03, 0.8)
            .repeat(1, 0.09, 0.8),
        part(Noise)
            .env(0.0005, 0.0, 0.01)
            .hp(5_000.0, 5_000.0, 1.0, 0.7)
            .gain(0.5),
    ],
    0.14,
    0.15,
    0.0,
);
pub const HOLD_TOLL_ENEMY: Patch = p(
    &[
        part(Fm {
            ratio: 2.76,
            index: 2.0,
        })
        .hz(146.8)
        .env(0.001, 0.0, 0.5),
        part(Square(0.25))
            .hz(293.7)
            .env(0.002, 0.05, 0.12)
            .lp(1_400.0, 800.0, 0.1, 0.8)
            .gain(0.35),
        // A struck edge above the dread, so the toll cuts through it.
        part(Metal)
            .hz(1_600.0)
            .env(0.0003, 0.0, 0.02)
            .hp(2_500.0, 2_500.0, 1.0, 0.7)
            .gain(0.4),
    ],
    0.15,
    0.35,
    0.0,
);
/// A foghorn over the flats: D and A, opening as it sounds.
pub const MATCH_HORN: Patch = p(
    &[
        part(Saw)
            .sweep(69.0, 73.4, 0.3)
            .env(0.25, 0.9, 0.5)
            .lp(600.0, 2_500.0, 0.5, 1.0)
            .gain(0.45),
        part(Square(0.25))
            .sweep(138.0, 146.8, 0.3)
            .env(0.3, 0.8, 0.5)
            .lp(800.0, 2_500.0, 0.5, 0.9)
            .gain(0.6),
        part(Square(0.25))
            .sweep(207.0, 220.0, 0.3)
            .env(0.35, 0.7, 0.5)
            .lp(800.0, 2_500.0, 0.5, 0.9)
            .gain(0.45),
        part(Square(0.25))
            .sweep(277.0, 293.7, 0.3)
            .env(0.4, 0.6, 0.5)
            .lp(900.0, 3_000.0, 0.5, 0.9)
            .gain(0.25),
        part(Sine).hz(73.4).env(0.2, 0.9, 0.5).gain(0.25),
    ],
    0.2,
    0.45,
    0.0,
);

// Stingers the music now carries; kept short for a muted music bus.
pub const VICTORY: Patch = SLUICE_TAKEN;
pub const DEFEAT: Patch = SLUICE_LOST;
/// A seat is out of the match: a low bell and the water closing over.
pub const ELIMINATED: Patch = p(
    &[
        part(Fm {
            ratio: 2.76,
            index: 1.8,
        })
        .hz(293.7)
        .env(0.001, 0.0, 0.7),
        part(Fm {
            ratio: 2.76,
            index: 1.2,
        })
        .hz(311.1)
        .at(0.35)
        .env(0.001, 0.0, 0.8)
        .gain(0.7),
        part(Pink)
            .at(0.3)
            .env(0.3, 0.4, 0.6)
            .lp(2_000.0, 300.0, 0.6, 0.8)
            .gain(1.0),
    ],
    0.12,
    0.4,
    0.0,
);
pub const DRAW: Patch = p(
    &[part(Fm {
        ratio: 3.5,
        index: 1.0,
    })
    .hz(587.3)
    .env(0.001, 0.0, 0.6)],
    0.08,
    0.3,
    0.0,
);

// The harbour at work (the ambience bus), tuned into D so it sits in
// the music.
pub const CONDENSER_BREATH: Patch = p(
    &[
        part(Pink)
            .env(0.3, 0.3, 0.3)
            .lp(200.0, 700.0, 0.4, 1.0)
            .gain(1.2),
        part(Sine)
            .sweep(55.0, 46.0, 0.6)
            .env(0.25, 0.3, 0.3)
            .gain(0.35),
    ],
    0.1,
    0.15,
    0.4,
);
pub const WINCH_CLACK: Patch = p(
    &[
        part(Metal)
            .hz(820.0)
            .env(0.0005, 0.0, 0.02)
            .bp(1_900.0, 1_900.0, 1.0, 1.1)
            .repeat(1, 0.19, 0.8),
        part(Square(0.5))
            .sweep(120.0, 90.0, 0.02)
            .env(0.001, 0.0, 0.03)
            .lp(500.0, 500.0, 1.0, 0.8)
            .repeat(1, 0.19, 0.8)
            .gain(0.6),
    ],
    0.1,
    0.1,
    0.6,
);
pub const DEPOSIT_TAP: Patch = p(
    &[
        part(Metal)
            .hz(587.3)
            .env(0.0005, 0.0, 0.05)
            .bp(1_500.0, 1_500.0, 1.0, 1.0),
        part(Sine).hz(293.7).env(0.001, 0.0, 0.12).gain(0.5),
    ],
    0.1,
    0.1,
    0.2,
);
pub const REED_BREATH: Patch = p(
    &[
        part(Triangle)
            .sweep(293.7, 293.7, 0.0)
            .env(0.06, 0.12, 0.15)
            .wobble(0.2, 5.0)
            .lp(1_200.0, 1_200.0, 1.0, 0.8),
        part(Pink)
            .env(0.05, 0.1, 0.12)
            .bp(1_600.0, 1_600.0, 1.0, 1.5)
            .gain(0.35),
    ],
    0.08,
    0.12,
    0.3,
);
pub const WATER_WASH: Patch = p(
    &[part(Pink)
        .env(0.3, 0.2, 0.45)
        .lp(250.0, 1_400.0, 0.4, 0.8)
        .gain(1.4)],
    0.1,
    0.25,
    0.5,
);
pub const UNION_PHRASE: Patch = p(
    &[
        part(Metal)
            .hz(587.3)
            .env(0.0005, 0.0, 0.05)
            .bp(1_400.0, 1_400.0, 1.0, 1.0),
        part(Metal)
            .hz(440.0)
            .at(0.22)
            .env(0.0005, 0.0, 0.06)
            .bp(1_100.0, 1_100.0, 1.0, 1.0)
            .gain(0.8),
    ],
    0.06,
    0.15,
    0.0,
);
pub const ASSEMBLY_PHRASE: Patch = p(
    &[
        part(Triangle)
            .hz(293.7)
            .env(0.05, 0.12, 0.12)
            .wobble(0.15, 5.0),
        part(Triangle)
            .hz(261.6)
            .at(0.25)
            .env(0.05, 0.15, 0.2)
            .wobble(0.15, 5.0),
    ],
    0.06,
    0.15,
    0.0,
);
pub const GLASS_CHIME: Patch = p(
    &[part(Fm {
        ratio: 2.756,
        index: 1.3,
    })
    .hz(1_174.7)
    .env(0.0005, 0.0, 0.2)],
    0.05,
    0.3,
    0.0,
);
pub const GLASS_PAIR: Patch = p(
    &[
        part(Fm {
            ratio: 2.756,
            index: 1.3,
        })
        .hz(880.0)
        .env(0.0005, 0.0, 0.12),
        part(Fm {
            ratio: 2.756,
            index: 1.3,
        })
        .hz(1_318.5)
        .at(0.11)
        .env(0.0005, 0.0, 0.15)
        .gain(0.7),
    ],
    0.06,
    0.25,
    0.0,
);
pub const COMPACT_PHRASE: Patch = p(
    &[
        part(Fm {
            ratio: 2.756,
            index: 1.2,
        })
        .hz(880.0)
        .env(0.0005, 0.0, 0.25),
        part(Fm {
            ratio: 2.756,
            index: 1.2,
        })
        .hz(698.5)
        .at(0.22)
        .env(0.0005, 0.0, 0.35)
        .gain(0.8),
    ],
    0.045,
    0.3,
    0.0,
);
