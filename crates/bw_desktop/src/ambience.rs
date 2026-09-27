//! The world under everything: wind over the salt flats, the sea
//! breaking somewhere off the edge of the chart, gulls, and a buoy bell.
//! It runs all the time a field is open, thinner at home. The tide sets
//! how much water is heard. Original synthesis, allocation-free.

use crate::synth::{self, Noise, Pink, Svf};
use std::sync::atomic::{AtomicU8, Ordering};

/// What the game asks of the ambience.
#[derive(Default)]
pub struct AmbienceControl {
    /// 0 off, 100 full.
    level: AtomicU8,
    /// 0 no sea, 100 a flood.
    water: AtomicU8,
    /// 0 still, 100 a gale.
    wind: AtomicU8,
}

impl AmbienceControl {
    pub fn set(&self, level: u8, water: u8, wind: u8) {
        self.level.store(level.min(100), Ordering::Relaxed);
        self.water.store(water.min(100), Ordering::Relaxed);
        self.wind.store(wind.min(100), Ordering::Relaxed);
    }
    fn read(&self) -> (f32, f32, f32) {
        (
            f32::from(self.level.load(Ordering::Relaxed)) / 100.0,
            f32::from(self.water.load(Ordering::Relaxed)) / 100.0,
            f32::from(self.wind.load(Ordering::Relaxed)) / 100.0,
        )
    }
}

#[derive(Clone, Copy, Default)]
struct Wave {
    t: f32,
    length: f32,
    pan: f32,
    size: f32,
    filter: Svf,
}

#[derive(Clone, Copy, Default)]
struct Call {
    t: f32,
    length: f32,
    pan: f32,
    phase: f32,
    kind: u8,
    pitch: f32,
    /// A phase for each upper partial of a bell.
    partials: [f32; 2],
}

pub struct Ambience {
    noise: Noise,
    pink_l: Pink,
    pink_r: Pink,
    wind_l: Svf,
    wind_r: Svf,
    gust: f32,
    gust_target: f32,
    gust_timer: f32,
    level: f32,
    water: f32,
    wind: f32,
    waves: [Wave; 3],
    next_wave: f32,
    call: Call,
    next_call: f32,
    bell: Call,
    next_bell: f32,
    roar_l: Svf,
    roar_r: Svf,
    roar_lfo: f32,
    roar_rate: f32,
    /// The harbour's machinery far off: an engine that thumps in spells
    /// with an uneven stroke, and a hull that creaks now and then.
    engine: f32,
    engine_period: f32,
    engine_spell: f32,
    engine_rest: f32,
    engine_filter: Svf,
    creak: Call,
    next_creak: f32,
    creak_filter: Svf,
}

impl Default for Ambience {
    fn default() -> Self {
        Self::new()
    }
}

impl Ambience {
    pub fn new() -> Self {
        Self {
            noise: Noise(0x0bad_5eed),
            pink_l: Pink::default(),
            pink_r: Pink::default(),
            wind_l: Svf::default(),
            wind_r: Svf::default(),
            gust: 0.4,
            gust_target: 0.5,
            gust_timer: 0.0,
            level: 0.0,
            water: 0.0,
            wind: 0.0,
            waves: [Wave::default(); 3],
            next_wave: 1.0,
            call: Call::default(),
            next_call: 9.0,
            bell: Call::default(),
            next_bell: 14.0,
            roar_l: Svf::default(),
            roar_r: Svf::default(),
            roar_lfo: 0.0,
            roar_rate: 0.09,
            engine: 0.0,
            engine_period: 2.5,
            engine_spell: 12.0,
            engine_rest: 20.0,
            engine_filter: Svf::default(),
            creak: Call::default(),
            next_creak: 11.0,
            creak_filter: Svf::default(),
        }
    }

    /// One stereo frame and a reverb send.
    pub fn next(&mut self, control: &AmbienceControl, sr: f32) -> (f32, f32, f32) {
        let dt = 1.0 / sr;
        let (level, water, wind) = control.read();
        // Parameters glide over a couple of seconds.
        let k = dt / 1.5;
        self.level += (level - self.level) * k;
        self.water += (water - self.water) * k;
        self.wind += (wind - self.wind) * k;
        if self.level < 0.000_5 {
            return (0.0, 0.0, 0.0);
        }

        // Wind: two decorrelated pink noises through a band that moves
        // with the gusts.
        self.gust_timer -= dt;
        if self.gust_timer <= 0.0 {
            self.gust_timer = 1.5 + 4.0 * self.noise.unit();
            self.gust_target = 0.25 + 0.75 * self.noise.unit();
        }
        self.gust += (self.gust_target - self.gust) * dt / 1.8;
        let g = self.gust * (0.35 + 0.65 * self.wind);
        let centre = 260.0 + 700.0 * g;
        let wl = self.pink_l.next(self.noise.next());
        let wr = self.pink_r.next(self.noise.next());
        let bl = self.wind_l.process(wl, centre, 1.3, sr).band;
        let br = self.wind_r.process(wr, centre * 1.07, 1.3, sr).band;
        let wind_gain = 0.10 * (0.25 + g * g);
        let mut l = bl * wind_gain;
        let mut r = br * wind_gain;
        // The far surf's roar under it all, in a band small speakers play:
        // two uncorrelated sides that breathe slowly and swell with each
        // breaking wave.
        // Its breathing drifts: each cycle takes its own time.
        self.roar_lfo += self.roar_rate / sr;
        if self.roar_lfo >= 1.0 {
            self.roar_lfo -= 1.0;
            self.roar_rate = 0.06 + 0.06 * self.noise.unit();
        }
        let swell: f32 = self
            .waves
            .iter()
            .filter(|w| w.t < w.length)
            .map(|w| {
                let p = w.t / w.length;
                if p < 0.35 {
                    p / 0.35
                } else {
                    (-(p - 0.35) * 4.0).exp()
                }
            })
            .fold(0.0, f32::max);
        // +-4 dB of slow breathing, and up to +4 dB more at a break.
        let breathe = synth::sine(self.roar_lfo) * 0.66;
        let roar_gain = 0.2 * self.water * (breathe + 0.66 * swell).exp2().min(4.0);
        l += self.roar_l.process(self.noise.next(), 250.0, 0.9, sr).band * roar_gain;
        r += self.roar_r.process(self.noise.next(), 275.0, 0.9, sr).band * roar_gain;

        // A pump engine somewhere across the flats: it runs in spells of
        // a few strokes with an uneven period, then stops a while.
        // A spell of strokes, then a rest of ten to forty seconds; the
        // last stroke of a spell dies away rather than stopping.
        self.engine_spell -= dt;
        if self.engine_spell < -self.engine_rest {
            self.engine_spell = 8.0 + 10.0 * self.noise.unit();
            self.engine_rest = 10.0 + 30.0 * self.noise.unit();
        }
        let stroking = self.engine_spell > 0.0;
        if stroking || self.engine < 1.0 {
            self.engine += dt / self.engine_period;
            if self.engine >= 1.0 && stroking {
                self.engine = 0.0;
                self.engine_period = 2.1 + 0.8 * self.noise.unit();
            }
            let beat = (-self.engine * self.engine_period * 5.6).exp();
            let thump = self
                .engine_filter
                .process(self.noise.next(), 180.0, 1.6, sr)
                .band
                * beat
                * 0.05;
            l += thump * 0.8;
            r += thump;
        }

        // Waves: each swells, breaks bright, and washes back.
        self.next_wave -= dt;
        if self.next_wave <= 0.0 && self.water > 0.02 {
            if let Some(slot) = self.waves.iter_mut().find(|w| w.t >= w.length) {
                slot.t = 0.0;
                slot.length = 6.0 + 4.0 * self.noise.unit();
                slot.pan = self.noise.next() * 0.7;
                slot.size = 0.5 + 0.5 * self.noise.unit();
            }
            self.next_wave = 4.0 + 5.0 * self.noise.unit() - 2.5 * self.water;
        }
        let mut verb = 0.0;
        for w in &mut self.waves {
            if w.t >= w.length {
                continue;
            }
            let p = w.t / w.length;
            // Swell to the break at 35%, then a long wash.
            let env = if p < 0.35 {
                (p / 0.35).powf(2.0)
            } else {
                (-(p - 0.35) * 4.5).exp()
            };
            let bright = if p < 0.35 {
                p / 0.35
            } else {
                (-(p - 0.35) * 6.0).exp()
            };
            let n = self.noise.next();
            let cutoff = 250.0 + 2_600.0 * bright * w.size;
            let s = w.filter.process(n, cutoff, 0.6, sr).low * env * w.size * 0.42 * self.water;
            let (pl, pr) = pan_gains(w.pan);
            l += s * pl;
            r += s * pr;
            verb += s * 0.2;
            w.t += dt;
        }

        // A gull now and then over open water.
        self.next_call -= dt;
        if self.next_call <= 0.0 {
            self.next_call = 14.0 + 26.0 * self.noise.unit();
            if self.water > 0.1 {
                self.call = Call {
                    t: 0.0,
                    length: 1.1,
                    pan: self.noise.next() * 0.9,
                    phase: 0.0,
                    kind: (self.noise.unit() * 3.0) as u8,
                    pitch: 0.9 + 0.25 * self.noise.unit(),
                    partials: [0.0; 2],
                };
            }
        }
        if self.call.t < self.call.length {
            let c = &mut self.call;
            // Two or three "kee" calls, each a falling glide.
            let calls = 2 + u32::from(c.kind % 2);
            let span = c.length / calls as f32;
            let within = c.t % span;
            let q = within / span;
            let f = (1_900.0 - 700.0 * q) * c.pitch * (1.0 + 0.03 * synth::sine(c.t * 31.0));
            c.phase = (c.phase + f / sr).fract();
            let env = if q < 0.12 {
                q / 0.12
            } else {
                (1.0 - q).max(0.0).powf(1.5)
            };
            let tone = synth::pulse(c.phase, f / sr, 0.18) * 0.6 + synth::sine(c.phase * 2.0) * 0.3;
            let s = tone * env * 0.012;
            let (pl, pr) = pan_gains(c.pan);
            l += s * pl;
            r += s * pr;
            verb += s * 1.2;
            c.t += dt;
        }

        // The buoy bell, far out, rung by the swell.
        self.next_bell -= dt;
        if self.next_bell <= 0.0 {
            self.next_bell = 18.0 + 20.0 * self.noise.unit();
            if self.water > 0.05 {
                // The swell rings it harder or softer, and now and then
                // twice.
                let twice = self.noise.unit() < 0.3;
                self.bell = Call {
                    t: 0.0,
                    length: if twice { 3.7 } else { 3.2 },
                    pan: self.noise.next() * 0.6,
                    phase: 0.0,
                    kind: u8::from(twice),
                    pitch: 0.55 + 0.45 * self.noise.unit(),
                    partials: [0.0; 2],
                };
            }
        }
        if self.bell.t < self.bell.length {
            let b = &mut self.bell;
            let f = 293.66;
            b.phase = (b.phase + f / sr).fract();
            b.partials[0] = (b.partials[0] + f * 2.76 / sr).fract();
            b.partials[1] = (b.partials[1] + f * 5.4 / sr).fract();
            let t = b.t;
            // `pitch` holds the strike's strength: a soft strike has less
            // of the upper partials. A second strike 0.45 s on is half as
            // hard.
            let strength = b.pitch;
            let again = if b.kind == 1 && t >= 0.45 {
                Some(t - 0.45)
            } else {
                None
            };
            let env = |tau: f32| (-t / tau).exp() + again.map_or(0.0, |t2| 0.5 * (-t2 / tau).exp());
            let tone = synth::sine(b.phase) * env(1.1)
                + synth::sine(b.partials[0]) * 0.5 * strength * env(0.5)
                + synth::sine(b.partials[1]) * 0.25 * strength * strength * env(0.2);
            let s = tone * 0.014 * strength;
            let (pl, pr) = pan_gains(b.pan);
            l += s * pl;
            r += s * pr;
            verb += s * 1.6;
            b.t += dt;
        }

        // A moored hull creaks against its fenders.
        self.next_creak -= dt;
        if self.next_creak <= 0.0 {
            self.next_creak = 20.0 + 20.0 * self.noise.unit();
            self.creak = Call {
                t: 0.0,
                length: 0.9 + 0.6 * self.noise.unit(),
                pan: self.noise.next() * 0.8,
                phase: 0.0,
                kind: 0,
                pitch: 0.8 + 0.4 * self.noise.unit(),
                partials: [0.0; 2],
            };
        }
        if self.creak.t < self.creak.length {
            let c = &mut self.creak;
            let q = c.t / c.length;
            // A rubbing saw that bends as the hull rolls.
            let f = (95.0 + 40.0 * synth::sine(q * 0.5)) * c.pitch;
            c.phase = (c.phase + f / sr).fract();
            let rub = synth::saw(c.phase, f / sr) * (0.6 + 0.4 * self.noise.unit());
            let body = self
                .creak_filter
                .process(rub, 700.0 * c.pitch, 3.0, sr)
                .band;
            let env = (q * 6.0).min(1.0) * (1.0 - q).max(0.0);
            let s = body * env * 0.03;
            let (pl, pr) = pan_gains(c.pan);
            l += s * pl;
            r += s * pr;
            verb += s * 0.8;
            c.t += dt;
        }

        (l * self.level, r * self.level, verb * self.level)
    }
}

fn pan_gains(pan: f32) -> (f32, f32) {
    let p = 0.5 * (pan.clamp(-1.0, 1.0) + 1.0);
    ((1.0 - p).sqrt(), p.sqrt())
}
