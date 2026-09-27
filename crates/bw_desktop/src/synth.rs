//! DSP building blocks shared by the music, the effects and the ambience:
//! band-limited oscillators, a state-variable filter, noise sources, a
//! tempo echo, a harbour reverb and the master bus. Everything here is
//! allocation-free after construction, so it can run in the output callback.

pub const TAU: f32 = std::f32::consts::TAU;

/// The PolyBLEP residual for a unit step at phase 0, with phase increment
/// `dt` per sample. Subtracting it from a naive edge removes most aliasing.
#[inline]
pub fn poly_blep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// A band-limited pulse wave with its DC offset removed, so a thin duty
/// does not push the mix off centre.
#[inline]
pub fn pulse(phase: f32, dt: f32, duty: f32) -> f32 {
    let duty = duty.clamp(0.02, 0.98);
    let mut v = if phase < duty { 1.0 } else { -1.0 };
    v += poly_blep(phase, dt);
    v -= poly_blep((phase + 1.0 - duty).fract(), dt);
    v - (2.0 * duty - 1.0)
}

/// A band-limited rising saw.
#[inline]
pub fn saw(phase: f32, dt: f32) -> f32 {
    2.0 * phase - 1.0 - poly_blep(phase, dt)
}

/// A smooth triangle, peak at phase 0.5.
#[inline]
pub fn triangle(phase: f32) -> f32 {
    4.0 * (phase - 0.5).abs() - 1.0
}

/// The console triangle: sixteen steps up and down. The steps give the
/// bass its grain; the caller lowpasses what it does not want.
#[inline]
pub fn triangle_stepped(phase: f32) -> f32 {
    let step = (phase * 32.0).floor();
    let level = if step < 16.0 { step } else { 31.0 - step };
    level / 7.5 - 1.0
}

/// A small, fast sine good to about -70 dB, used where many voices run.
#[inline]
pub fn sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

/// Deterministic white noise in -1..1.
#[derive(Clone, Copy, Debug)]
pub struct Noise(pub u32);

impl Noise {
    #[inline]
    pub fn next(&mut self) -> f32 {
        let mut s = self.0;
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        self.0 = s;
        (s as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    /// A value in 0..1.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        0.5 * (self.next() + 1.0)
    }
}

/// Pink-ish noise (Paul Kellet's economy filter): a softer bed than white
/// noise for wind and surf.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pink {
    b0: f32,
    b1: f32,
    b2: f32,
}

impl Pink {
    #[inline]
    pub fn next(&mut self, white: f32) -> f32 {
        self.b0 = 0.99765 * self.b0 + white * 0.099_046;
        self.b1 = 0.96300 * self.b1 + white * 0.296_516_4;
        self.b2 = 0.57000 * self.b2 + white * 1.052_691_3;
        (self.b0 + self.b1 + self.b2 + white * 0.1848) * 0.2
    }
}

/// The console noise channel: a 15-bit shift register clocked at a rate,
/// held between clocks. Short mode (6-bit feedback) gives the metallic,
/// pitched buzz.
#[derive(Clone, Copy, Debug)]
pub struct Lfsr {
    state: u16,
    phase: f32,
    out: f32,
}

impl Default for Lfsr {
    fn default() -> Self {
        Self {
            state: 1,
            phase: 0.0,
            out: 1.0,
        }
    }
}

impl Lfsr {
    #[inline]
    pub fn next(&mut self, clock_hz: f32, sample_rate: f32, short: bool) -> f32 {
        self.phase += clock_hz / sample_rate;
        // At most a few clocks a sample; beyond that it is white anyway.
        let mut clocks = 0;
        while self.phase >= 1.0 && clocks < 8 {
            self.phase -= 1.0;
            clocks += 1;
            let tap = if short { 6 } else { 1 };
            let bit = (self.state ^ (self.state >> tap)) & 1;
            self.state = (self.state >> 1) | (bit << 14);
            self.out = if self.state & 1 == 1 { 1.0 } else { -1.0 };
        }
        if self.phase >= 1.0 {
            self.phase = self.phase.fract();
        }
        self.out
    }
}

/// Andrew Simper's trapezoidal state-variable filter: stable under fast
/// modulation, which the sweeps here rely on.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SvfOut {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

/// A state-variable filter's coefficients, for filters whose cutoff moves
/// slower than the audio: compute them now and then, run every sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct SvfCoef {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl SvfCoef {
    #[inline]
    pub fn new(cutoff: f32, q: f32, sample_rate: f32) -> Self {
        let fc = (cutoff / sample_rate).clamp(0.000_05, 0.49);
        let g = fast_tan(std::f32::consts::PI * fc);
        let k = 1.0 / q.max(0.3);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        Self {
            a1,
            a2,
            a3: g * a2,
            k,
        }
    }
}

impl Svf {
    #[inline]
    pub fn run(&mut self, x: f32, c: &SvfCoef) -> SvfOut {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        if !self.ic1.is_finite() || !self.ic2.is_finite() {
            self.ic1 = 0.0;
            self.ic2 = 0.0;
        }
        SvfOut {
            low: v2,
            band: v1,
            high: x - c.k * v1 - v2,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32, cutoff: f32, q: f32, sample_rate: f32) -> SvfOut {
        let fc = (cutoff / sample_rate).clamp(0.000_05, 0.49);
        let g = fast_tan(std::f32::consts::PI * fc);
        let k = 1.0 / q.max(0.3);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        // A runaway state (NaN from a bad parameter) resets instead of
        // poisoning the bus.
        if !self.ic1.is_finite() || !self.ic2.is_finite() {
            self.ic1 = 0.0;
            self.ic2 = 0.0;
        }
        SvfOut {
            low: v2,
            band: v1,
            high: x - k * v1 - v2,
        }
    }
}

/// tan(x) for 0..~1.54 by a rational approximation, accurate to a fraction
/// of a percent across the audio band, which is all a filter's cutoff needs.
#[inline]
fn fast_tan(x: f32) -> f32 {
    let x2 = x * x;
    x * (1.0 - x2 * (1.0 / 9.0 + x2 * (2.0 / 945.0))) / (1.0 - x2 * (4.0 / 9.0 - x2 / 63.0))
}

/// A one-pole lowpass smoother.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole {
    pub z: f32,
    /// The coefficient for the last cutoff asked, kept so a steady filter
    /// costs no exponential per sample.
    key: f32,
    a: f32,
}

impl OnePole {
    #[inline]
    pub fn lowpass(&mut self, x: f32, cutoff: f32, sample_rate: f32) -> f32 {
        let key = cutoff / sample_rate;
        if key != self.key {
            self.key = key;
            self.a = (-TAU * key).exp();
        }
        self.z = x + self.a * (self.z - x);
        self.z
    }
}

/// Removes DC and subsonic rumble below ~20 Hz.
#[derive(Clone, Copy, Debug, Default)]
pub struct DcBlock {
    x1: f32,
    y1: f32,
}

impl DcBlock {
    #[inline]
    pub fn process(&mut self, x: f32, r: f32) -> f32 {
        let y = x - self.x1 + r * self.y1;
        self.x1 = x;
        self.y1 = if y.is_finite() { y } else { 0.0 };
        self.y1
    }
}

/// A fractional delay line with a fixed capacity.
pub struct Delay {
    buffer: Vec<f32>,
    write: usize,
}

impl Delay {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity.max(4)],
            write: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.buffer[self.write] = x;
        self.write = (self.write + 1) % self.buffer.len();
    }
    /// The sample written `delay` samples ago (fractional, linear).
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        let len = self.buffer.len();
        let d = delay.clamp(1.0, (len - 2) as f32);
        let whole = d as usize;
        let frac = d - whole as f32;
        let a = self.buffer[(self.write + len - whole) % len];
        let b = self.buffer[(self.write + len - whole - 1) % len];
        a + (b - a) * frac
    }
    pub fn clear(&mut self) {
        self.buffer.iter_mut().for_each(|s| *s = 0.0);
    }
}

/// A ping-pong echo in the manner of the 16-bit consoles: the repeats
/// cross sides and darken as they go.
pub struct Echo {
    left: Delay,
    right: Delay,
    damp_l: OnePole,
    damp_r: OnePole,
    pub delay_samples: f32,
    pub feedback: f32,
    pub tone_hz: f32,
}

impl Echo {
    pub fn new(sample_rate: f32) -> Self {
        let cap = (sample_rate * 1.6) as usize;
        Self {
            left: Delay::new(cap),
            right: Delay::new(cap),
            damp_l: OnePole::default(),
            damp_r: OnePole::default(),
            delay_samples: sample_rate * 0.4,
            feedback: 0.35,
            tone_hz: 3_200.0,
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, sample_rate: f32) -> (f32, f32) {
        let out_l = self.left.read(self.delay_samples);
        let out_r = self.right.read(self.delay_samples);
        let fb_l = self.damp_l.lowpass(out_r, self.tone_hz, sample_rate);
        let fb_r = self.damp_r.lowpass(out_l, self.tone_hz, sample_rate);
        // Mono input enters on the left; the feedback crosses sides.
        self.left
            .push(soft(0.5 * (in_l + in_r) + fb_l * self.feedback));
        self.right.push(soft(fb_r * self.feedback));
        (out_l, out_r)
    }

    pub fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
    }
}

/// An eight-line feedback delay network: a damp, wide harbour space.
pub struct Reverb {
    lines: [Delay; 8],
    lengths: [f32; 8],
    damp: [OnePole; 8],
    pre: [Delay; 2],
    pre_len: [f32; 2],
    pub decay: f32,
    pub tone_hz: f32,
    lfo: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        // Mutually prime lengths in milliseconds, spread for density.
        let ms = [29.7, 37.1, 41.1, 43.7, 53.3, 59.9, 67.7, 73.1];
        let scale = sample_rate / 1000.0;
        let lengths = ms.map(|m| m * scale);
        let lines = lengths.map(|l| Delay::new(l as usize + 64));
        let pre_len = [7.3 * scale, 11.9 * scale];
        let pre = pre_len.map(|l| Delay::new(l as usize + 8));
        Self {
            lines,
            lengths,
            damp: [OnePole::default(); 8],
            pre,
            pre_len,
            decay: 0.86,
            tone_hz: 5_200.0,
            lfo: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, sample_rate: f32) -> (f32, f32) {
        // Two allpass diffusers smear the input before the network.
        let mut x = 0.5 * (in_l + in_r);
        for (line, len) in self.pre.iter_mut().zip(self.pre_len) {
            let delayed = line.read(len);
            let v = x + 0.6 * delayed;
            line.push(v);
            x = delayed - 0.6 * v;
        }
        self.lfo = (self.lfo + 0.35 / sample_rate).fract();
        let wobble = sine(self.lfo) * 6.0;
        let mut taps = [0.0f32; 8];
        for (i, tap) in taps.iter_mut().enumerate() {
            // Two lines drift slightly to break up metallic ringing.
            let len = self.lengths[i] + if i == 2 || i == 5 { wobble } else { 0.0 };
            *tap = self.damp[i].lowpass(self.lines[i].read(len), self.tone_hz, sample_rate);
        }
        // Householder feedback: each line gets the others back, mixed.
        let sum: f32 = taps.iter().sum::<f32>() * 0.25;
        for (i, tap) in taps.iter().enumerate() {
            let feedback = (tap - sum) * self.decay;
            let input = if i % 2 == 0 { x } else { -x };
            self.lines[i].push(soft(input * 0.35 + feedback));
        }
        let left = taps[0] - taps[2] + taps[4] - taps[6];
        let right = taps[1] - taps[3] + taps[5] - taps[7];
        (left * 0.5, right * 0.5)
    }

    pub fn clear(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        for line in &mut self.pre {
            line.clear();
        }
    }
}

/// A gentle saturator: unity near zero, rounding peaks.
#[inline]
pub fn soft(x: f32) -> f32 {
    if x.abs() < 0.5 { x } else { x.tanh() }
}

/// The master bus: a gentle RMS glue compressor whose detector ignores the
/// sub-bass (so an explosion does not pump the music), then a look-ahead
/// peak limiter at the ceiling. Allocation-free after construction.
pub struct Master {
    dc_l: DcBlock,
    dc_r: DcBlock,
    side_hp_l: OnePole,
    side_hp_r: OnePole,
    rms: f32,
    rms_attack: f32,
    rms_release: f32,
    comp_gain: f32,
    pub threshold: f32,
    pub ratio: f32,
    pub makeup: f32,
    pub ceiling: f32,
    delay_l: Delay,
    delay_r: Delay,
    /// Required gain for each of the last `look` samples.
    window: Vec<f32>,
    head: usize,
    look: usize,
    limit_gain: f32,
    /// The lowest the limiter's gain has gone, for reviews.
    deepest: f32,
    limit_attack: f32,
    limit_release: f32,
    sr: f32,
}

impl Master {
    pub fn new(sample_rate: f32) -> Self {
        let look = ((sample_rate * 0.003) as usize).max(8);
        Self {
            dc_l: DcBlock::default(),
            dc_r: DcBlock::default(),
            side_hp_l: OnePole::default(),
            side_hp_r: OnePole::default(),
            rms: 0.0,
            rms_attack: (-1.0 / (0.008 * sample_rate)).exp(),
            rms_release: (-1.0 / (0.25 * sample_rate)).exp(),
            comp_gain: 1.0,
            threshold: 0.25,
            ratio: 2.0,
            makeup: 1.0,
            ceiling: 0.891,
            delay_l: Delay::new(look + 4),
            delay_r: Delay::new(look + 4),
            window: vec![1.0; look],
            head: 0,
            look,
            limit_gain: 1.0,
            deepest: 1.0,
            limit_attack: 1.0 - (-4.0 / look as f32).exp(),
            limit_release: 1.0 - (-1.0 / (0.08 * sample_rate)).exp(),
            sr: sample_rate,
        }
    }

    /// The lowest gain the limiter has applied.
    pub fn deepest(&self) -> f32 {
        self.deepest
    }

    /// Forget everything: after a mute the next sound starts from silence.
    pub fn clear(&mut self) {
        self.dc_l = DcBlock::default();
        self.dc_r = DcBlock::default();
        self.rms = 0.0;
        self.comp_gain = 1.0;
        self.delay_l.clear();
        self.delay_r.clear();
        self.window.iter_mut().for_each(|w| *w = 1.0);
        self.limit_gain = 1.0;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let l = self.dc_l.process(l, 0.9975);
        let r = self.dc_r.process(r, 0.9975);
        // The detector hears above 120 Hz only.
        let sl = l - self.side_hp_l.lowpass(l, 120.0, self.sr);
        let sr = r - self.side_hp_r.lowpass(r, 120.0, self.sr);
        let power = 0.5 * (sl * sl + sr * sr);
        let coeff = if power > self.rms {
            self.rms_attack
        } else {
            self.rms_release
        };
        self.rms = power + coeff * (self.rms - power);
        let level = self.rms.sqrt();
        let target = if level > self.threshold {
            (level / self.threshold).powf(1.0 / self.ratio - 1.0)
        } else {
            1.0
        };
        // Smooth the compressor's own gain a little more.
        self.comp_gain += (target - self.comp_gain) * 0.002;
        let g = self.comp_gain * self.makeup;
        let (l, r) = (l * g, r * g);

        // Look-ahead limiter: the gain this sample needs, remembered for
        // the window; the delayed sample gets the least of them.
        let peak = l.abs().max(r.abs());
        // Aim a little under the ceiling so the ramp's last 2 % is covered.
        let need = if peak > self.ceiling * 0.98 {
            self.ceiling * 0.98 / peak
        } else {
            1.0
        };
        self.window[self.head] = need;
        self.head = (self.head + 1) % self.look;
        let mut floor = 1.0f32;
        for &w in &self.window {
            floor = floor.min(w);
        }
        if floor < self.limit_gain {
            // Ramp down across the look-ahead window rather than stepping:
            // the window holds the need until the peak arrives.
            self.limit_gain += (floor - self.limit_gain) * self.limit_attack;
        } else {
            self.limit_gain += (floor - self.limit_gain) * self.limit_release;
        }
        self.deepest = self.deepest.min(self.limit_gain);
        self.delay_l.push(l);
        self.delay_r.push(r);
        let dl = self.delay_l.read(self.look as f32);
        let dr = self.delay_r.read(self.look as f32);
        let c = self.ceiling;
        let out_l = (dl * self.limit_gain).clamp(-c, c);
        let out_r = (dr * self.limit_gain).clamp(-c, c);
        (
            if out_l.is_finite() { out_l } else { 0.0 },
            if out_r.is_finite() { out_r } else { 0.0 },
        )
    }
}

/// MIDI note number to Hz.
#[inline]
pub fn midi_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillators_stay_bounded_and_centred() {
        for duty in [0.125, 0.25, 0.5] {
            let mut sum = 0.0;
            let mut peak = 0.0f32;
            let n = 48_000;
            let dt = 440.0 / 48_000.0;
            let mut phase = 0.0;
            for _ in 0..n {
                let v = pulse(phase, dt, duty);
                sum += v;
                peak = peak.max(v.abs());
                phase = (phase + dt).fract();
            }
            assert!((sum / n as f32).abs() < 0.02, "pulse {duty} is centred");
            assert!(peak < 2.2, "pulse {duty} bounded: {peak}");
        }
        for i in 0..64 {
            let p = i as f32 / 64.0;
            assert!(triangle(p).abs() <= 1.0);
            assert!(triangle_stepped(p).abs() <= 1.0 + 1e-6);
        }
    }

    #[test]
    fn the_filter_passes_lows_and_stops_highs() {
        let sr = 48_000.0;
        let rms = |hz: f32| {
            let mut f = Svf::default();
            let mut sum = 0.0;
            for i in 0..sr as usize {
                let x = sine(hz * i as f32 / sr);
                let y = f.process(x, 500.0, 0.707, sr).low;
                if i > 4_800 {
                    sum += y * y;
                }
            }
            sum.sqrt()
        };
        assert!(rms(100.0) > 10.0 * rms(8_000.0));
    }

    #[test]
    fn effects_and_master_stay_finite_and_under_the_ceiling() {
        let sr = 48_000.0;
        let mut echo = Echo::new(sr);
        let mut verb = Reverb::new(sr);
        let mut master = Master::new(sr);
        let mut noise = Noise(7);
        for _ in 0..96_000 {
            let x = noise.next() * 3.0;
            let (el, er) = echo.process(x, x, sr);
            let (rl, rr) = verb.process(x, -x, sr);
            let (l, r) = master.process(x + el + rl, x + er + rr);
            assert!(l.is_finite() && r.is_finite());
            assert!(l.abs() <= 0.892 && r.abs() <= 0.892);
        }
    }
}
