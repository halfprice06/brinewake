//! Writing music down: a small builder for the songs in `score_songs`,
//! with a note language, chord symbols, voice-leading for pads, and a
//! report that reads a song back (harmony per bar, clashes, rhythm
//! figures) so it can be checked without ears.

use crate::music::{Channel, Instrument, Layer, Note, Song, SongId};
use std::fmt::Write as _;

/// A chord: its pitch classes (root first) and the bass pitch class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub root: u8,
    pub bass: u8,
    pub tones: Vec<u8>,
    pub third: Option<u8>,
    pub fifth: u8,
    pub seventh: Option<u8>,
}

fn pitch_class(name: &str) -> Option<(u8, usize)> {
    let mut chars = name.chars();
    let letter = chars.next()?;
    let base: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let (shift, used) = match chars.next() {
        Some('#') => (1, 2),
        Some('b') => (-1, 2),
        _ => (0, 1),
    };
    Some((((base + shift).rem_euclid(12)) as u8, used))
}

/// Parses a chord symbol: `Dm`, `G`, `Bb`, `A7`, `Dsus4`, `Cmaj7`, `Dm/F`,
/// `Gm6`, `Eb`, `Bdim`, `Dadd9`.
pub fn chord(symbol: &str) -> Chord {
    let (main, slash) = match symbol.split_once('/') {
        Some((m, s)) => (m, Some(s)),
        None => (symbol, None),
    };
    let (root, used) = pitch_class(main).unwrap_or_else(|| panic!("bad chord root in {symbol:?}"));
    let quality = &main[used..];
    let at = |i: u8| (root + i) % 12;
    let (third, fifth, extra): (Option<u8>, u8, Vec<u8>) = match quality {
        "" => (Some(4), 7, vec![]),
        "m" => (Some(3), 7, vec![]),
        "7" => (Some(4), 7, vec![10]),
        "m7" => (Some(3), 7, vec![10]),
        "maj7" => (Some(4), 7, vec![11]),
        "6" => (Some(4), 7, vec![9]),
        "m6" => (Some(3), 7, vec![9]),
        "sus2" => (None, 7, vec![2]),
        "sus4" => (None, 7, vec![5]),
        "add9" => (Some(4), 7, vec![2]),
        "madd9" => (Some(3), 7, vec![2]),
        "dim" => (Some(3), 6, vec![]),
        "5" => (None, 7, vec![]),
        other => panic!("unknown chord quality {other:?} in {symbol:?}"),
    };
    let mut tones = vec![root];
    if let Some(t) = third {
        tones.push(at(t));
    }
    tones.push(at(fifth));
    for e in &extra {
        tones.push(at(*e));
    }
    let seventh = extra.iter().find(|e| matches!(e, 9..=11)).map(|e| at(*e));
    let bass = slash.map_or(root, |s| {
        pitch_class(s)
            .unwrap_or_else(|| panic!("bad slash bass in {symbol:?}"))
            .0
    });
    Chord {
        root,
        bass,
        tones,
        third: third.map(at),
        fifth: at(fifth),
        seventh,
    }
}

/// A note name to MIDI: `C4` is 60, `Bb3` is 58, `F#5` is 78.
pub fn midi(name: &str) -> u8 {
    let (pc, used) = pitch_class(name).unwrap_or_else(|| panic!("bad note {name:?}"));
    let octave: i32 = name[used..]
        .parse()
        .unwrap_or_else(|_| panic!("bad octave in {name:?}"));
    let value = (octave + 1) * 12 + i32::from(pc);
    u8::try_from(value).unwrap_or_else(|_| panic!("note out of range {name:?}"))
}

pub fn note_name(key: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
    ];
    format!(
        "{}{}",
        NAMES[usize::from(key % 12)],
        i32::from(key / 12) - 1
    )
}

pub struct Builder {
    pub song: Song,
    parsed: Vec<Vec<(u32, Chord)>>,
    /// The pass notes written now belong to (0 every pass).
    pass: u8,
}

impl Builder {
    pub fn new(
        id: SongId,
        name: &'static str,
        bpm: f32,
        rows_per_beat: u32,
        beats_per_bar: u32,
        bars: u32,
    ) -> Self {
        Self {
            song: Song {
                id,
                name,
                bpm,
                rows_per_beat,
                beats_per_bar,
                swing: 0.0,
                bars,
                loop_bar: Some(0),
                then: None,
                echo_rows: 3.0,
                echo_feedback: 0.3,
                gain: 1.0,
                channels: Vec::new(),
                chords: vec![String::new(); bars as usize],
                partner: None,
                sections: vec![],
                handovers: vec![],
                key: 0,
            },
            parsed: vec![Vec::new(); bars as usize],
            pass: 0,
        }
    }

    pub fn rows_per_bar(&self) -> u32 {
        self.song.rows_per_bar()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn channel(
        &mut self,
        name: &'static str,
        inst: Instrument,
        layer: Layer,
        pan: f32,
        gain: f32,
        echo: f32,
        reverb: f32,
    ) -> usize {
        assert!(
            self.song.channels.len() < crate::music::MAX_CHANNELS,
            "too many channels"
        );
        self.song.channels.push(Channel {
            name,
            inst,
            variants: None,
            extra: Vec::new(),
            layer,
            pan,
            gain,
            echo,
            reverb,
            poly: false,
            spread: 0.0,
            role: crate::music::Role::Texture,
            entry: None,
            notes: Vec::new(),
        });
        self.song.channels.len() - 1
    }

    pub fn poly(&mut self, ch: usize) {
        self.song.channels[ch].poly = true;
    }

    pub fn role(&mut self, ch: usize, role: crate::music::Role) {
        self.song.channels[ch].role = role;
    }

    /// Make a channel a fill that plays when `layer` comes on; its notes'
    /// rows count from the last beat before the bar line.
    pub fn entry(&mut self, ch: usize, layer: Layer) {
        self.song.channels[ch].entry = Some(layer);
        self.song.channels[ch].role = crate::music::Role::Kit;
    }

    /// Spread a channel's voices either side of its pan.
    pub fn spread(&mut self, ch: usize, spread: f32) {
        self.song.channels[ch].spread = spread;
    }

    pub fn variants(&mut self, ch: usize, insts: [Instrument; 3]) {
        self.song.channels[ch].variants = Some(insts);
    }

    /// Another instrument for the channel; notes pick it by the number
    /// returned.
    pub fn extra(&mut self, ch: usize, inst: Instrument) -> u8 {
        self.song.channels[ch].extra.push(inst);
        self.song.channels[ch].extra.len() as u8
    }

    /// Chord symbols from `bar` on, one per bar. `A+B` splits a bar in
    /// halves; `-` repeats the previous bar's chord.
    pub fn chords(&mut self, bar: u32, symbols: &str) {
        let half = self.rows_per_bar() / 2;
        let mut last: Option<String> = None;
        for (i, symbol) in symbols.split_whitespace().enumerate() {
            let b = (bar as usize) + i;
            assert!(b < self.parsed.len(), "chords past the end at bar {b}");
            let symbol = if symbol == "-" {
                last.clone().expect("a repeat needs a chord before it")
            } else {
                symbol.to_string()
            };
            let parts: Vec<&str> = symbol.split('+').collect();
            self.parsed[b] = parts
                .iter()
                .enumerate()
                .map(|(k, s)| (k as u32 * half, chord(s)))
                .collect();
            self.song.chords[b] = symbol.clone();
            last = Some(symbol);
        }
    }

    /// The chord sounding at an absolute row.
    pub fn chord_at(&self, row: u32) -> &Chord {
        let rpb = self.rows_per_bar();
        let bar = (row / rpb) as usize;
        let within = row % rpb;
        let list = &self.parsed[bar.min(self.parsed.len() - 1)];
        assert!(!list.is_empty(), "no chord at bar {bar}");
        list.iter()
            .rev()
            .find(|(start, _)| *start <= within)
            .map(|(_, c)| c)
            .unwrap_or(&list[0].1)
    }

    /// Notes written after this play on every pass (0), only the first
    /// time through and every odd time (1), or every even time (2).
    pub fn pass(&mut self, pass: u8) {
        self.pass = pass;
    }

    pub fn push(&mut self, ch: usize, mut note: Note) {
        note.pass = self.pass;
        assert!(note.len > 0, "zero-length note");
        assert!(
            note.row < self.song.total_rows(),
            "{}: note past the end at row {}",
            self.song.channels[ch].name,
            note.row
        );
        self.song.channels[ch].notes.push(note);
    }

    pub fn note(&mut self, ch: usize, row: u32, len: u32, key: u8, vel: f32) {
        self.push(
            ch,
            Note {
                row,
                len,
                key,
                vel,
                slide: 0,
                arp: [0, 0],
                inst: 0,
                pass: 0,
            },
        );
    }

    /// A melodic line from `bar`: `A4:3 D5:2 E5 | F5:6! r:2 G5:4~` where
    /// `:n` is a length in rows (it carries forward), `r` rests, `|`
    /// checks a bar line, `!` accents, `,` ghosts, `'` shortens to half,
    /// `~` slides in from two below, `^` falls in from one above, `{3,7}`
    /// arpeggiates, and `@n` plays the channel's extra instrument n.
    /// Returns the row after the line.
    pub fn line(&mut self, ch: usize, bar: u32, text: &str, vel: f32) -> u32 {
        let rpb = self.rows_per_bar();
        let mut row = bar * rpb;
        let mut len = 1u32;
        for token in text.split_whitespace() {
            if token == "|" {
                assert!(
                    row.is_multiple_of(rpb),
                    "{} bar check failed at row {row} (bar {}, row {} in bar) in {text:?}",
                    self.song.name,
                    row / rpb,
                    row % rpb
                );
                continue;
            }
            // A chip chord may be written before or after the length.
            let (token, braces) = match (token.find('{'), token.find('}')) {
                (Some(a), Some(b)) if b > a => (
                    format!("{}{}", &token[..a], &token[b + 1..]),
                    Some(token[a..=b].to_string()),
                ),
                _ => (token.to_string(), None),
            };
            let token = token.as_str();
            let (head, mods) = split_mods(token);
            let (body, dur) = match head.split_once(':') {
                Some((b, d)) => (
                    b,
                    Some(
                        d.parse::<u32>()
                            .unwrap_or_else(|_| panic!("bad length in {token:?}")),
                    ),
                ),
                None => (head, None),
            };
            let body_owned = match &braces {
                Some(b) => format!("{body}{b}"),
                None => body.to_string(),
            };
            let body = body_owned.as_str();
            if let Some(d) = dur {
                len = d;
            }
            if body == "r" {
                row += len;
                continue;
            }
            let (name, arp) = match body.split_once('{') {
                Some((n, rest)) => {
                    let inner = rest.trim_end_matches('}');
                    let mut offsets = inner
                        .split(',')
                        .map(|v| v.parse::<i8>().expect("arp offset"));
                    (
                        n,
                        [offsets.next().unwrap_or(0), offsets.next().unwrap_or(0)],
                    )
                }
                None => (body, [0, 0]),
            };
            let mut v = vel;
            let mut l = len;
            let mut slide = 0i8;
            let mut inst = 0u8;
            let mut chars = mods.chars().peekable();
            while let Some(m) = chars.next() {
                match m {
                    '!' => v = (v * 1.25).min(1.0),
                    ',' => v *= 0.6,
                    '\'' => l = (len / 2).max(1),
                    '~' => slide = -2,
                    '^' => slide = 1,
                    '@' => {
                        let digit = chars.next().and_then(|d| d.to_digit(10)).expect("@n");
                        inst = digit as u8;
                    }
                    other => panic!("unknown modifier {other:?} in {token:?}"),
                }
            }
            self.push(
                ch,
                Note {
                    row,
                    len: l,
                    key: midi(name),
                    vel: v,
                    slide,
                    arp,
                    inst,
                    pass: 0,
                },
            );
            row += len;
        }
        row
    }

    /// A rhythm, one character a row from `bar`, repeated over `bars`:
    /// `x` a hit, `X` an accent, `g` a ghost, `.` nothing; any digit 1-9
    /// plays extra instrument n.
    pub fn beat(&mut self, ch: usize, bar: u32, bars: u32, pattern: &str, key: u8, vel: f32) {
        let pattern: Vec<char> = pattern.chars().filter(|c| !c.is_whitespace()).collect();
        let rpb = self.rows_per_bar();
        assert!(
            pattern.len() as u32 == rpb || (pattern.len() as u32).is_multiple_of(rpb),
            "{} beat length {} is not bars of {rpb}",
            self.song.name,
            pattern.len()
        );
        let span = pattern.len() as u32;
        let start = bar * rpb;
        let end = (bar + bars) * rpb;
        let mut row = start;
        while row < end {
            let c = pattern[((row - start) % span) as usize];
            let (v, inst) = match c {
                'x' => (vel, 0),
                'X' => ((vel * 1.3).min(1.0), 0),
                'g' => (vel * 0.45, 0),
                d if d.is_ascii_digit() => (vel, d.to_digit(10).unwrap_or(0) as u8),
                _ => (0.0, 0),
            };
            if v > 0.0 {
                self.push(
                    ch,
                    Note {
                        row,
                        len: 1,
                        key,
                        vel: v,
                        slide: 0,
                        arp: [0, 0],
                        inst,
                        pass: 0,
                    },
                );
            }
            row += 1;
        }
    }

    /// A bass figure over the chords, repeated each bar: `R:4 5:2 O:2`
    /// where R is the chord's bass, T its root, 3/5/7 its chord tones,
    /// O the bass an octave up, L the fifth below, `a`/`A` a semitone
    /// below/above the next bar's bass, `r` a rest.
    pub fn bass(&mut self, ch: usize, bar: u32, bars: u32, pattern: &str, low: u8, vel: f32) {
        let rpb = self.rows_per_bar();
        for b in bar..bar + bars {
            let mut row = b * rpb;
            for token in pattern.split_whitespace() {
                let (head, mods) = split_mods(token);
                let (deg, dur) = head
                    .split_once(':')
                    .unwrap_or_else(|| panic!("bass token {token:?} needs a length"));
                let dur: u32 = dur.parse().expect("bass length");
                let chord = self.chord_at(row).clone();
                let place = |pc: u8, floor: u8| -> u8 {
                    let mut k = floor - floor % 12 + pc;
                    if k < floor {
                        k += 12;
                    }
                    k
                };
                let bass_key = place(chord.bass, low);
                let key = match deg {
                    "r" => None,
                    "R" => Some(bass_key),
                    "T" => Some(place(chord.root, low)),
                    "O" => Some(bass_key + 12),
                    "3" => Some(place(chord.third.unwrap_or(chord.root), bass_key)),
                    "5" => Some(place(chord.fifth, bass_key)),
                    "7" => Some(place(
                        chord.seventh.unwrap_or((chord.root + 10) % 12),
                        bass_key,
                    )),
                    "L" => Some(
                        place(chord.fifth, low.saturating_sub(12)).max(bass_key.saturating_sub(7)),
                    ),
                    "a" | "A" => {
                        let next_row = ((row / rpb) + 1) * rpb;
                        // At the end, the bar the song loops to.
                        let next_row = if next_row < self.song.total_rows() {
                            next_row
                        } else {
                            self.song.loop_bar.unwrap_or(0) * rpb
                        };
                        let next = self.chord_at(next_row).bass;
                        let target = place(next, low);
                        Some(if deg == "a" { target - 1 } else { target + 1 })
                    }
                    other => panic!("unknown bass degree {other:?}"),
                };
                if let Some(key) = key {
                    let mut v = vel;
                    let mut l = dur;
                    for m in mods.chars() {
                        match m {
                            '!' => v = (v * 1.25).min(1.0),
                            ',' => v *= 0.6,
                            '\'' => l = (dur / 2).max(1),
                            _ => panic!("unknown bass modifier {m:?}"),
                        }
                    }
                    self.note(ch, row, l, key, v);
                }
                row += dur;
            }
            assert_eq!(
                row,
                (b + 1) * rpb,
                "{} bass figure {pattern:?} does not fill a bar",
                self.song.name
            );
        }
    }

    /// Broken chords: indices into the chord's tones stacked upward from
    /// `low` (0 the lowest), one every `step` rows, `-` a rest; repeated
    /// each bar over `bars`.
    #[allow(clippy::too_many_arguments)]
    pub fn arp(
        &mut self,
        ch: usize,
        bar: u32,
        bars: u32,
        pattern: &str,
        step: u32,
        low: u8,
        len: u32,
        vel: f32,
    ) {
        let rpb = self.rows_per_bar();
        let steps: Vec<&str> = pattern.split_whitespace().collect();
        for b in bar..bar + bars {
            let mut row = b * rpb;
            let mut i = 0usize;
            while row < (b + 1) * rpb {
                let token = steps[i % steps.len()];
                i += 1;
                if token != "-" {
                    let (idx, mods) = split_mods(token);
                    let idx: usize = idx.parse().expect("arp index");
                    let chord = self.chord_at(row).clone();
                    let ladder = ladder(&chord, low, 4);
                    let key = ladder[idx.min(ladder.len() - 1)];
                    let v = if mods.contains('!') {
                        (vel * 1.25).min(1.0)
                    } else if mods.contains(',') {
                        vel * 0.6
                    } else {
                        vel
                    };
                    self.note(ch, row, len, key, v);
                }
                row += step;
            }
        }
    }

    /// Held chords, voice-led: each bar takes the chord tones nearest the
    /// previous voicing within `low..=high`. A voicing that does not
    /// change is held on. Notes go to one polyphonic channel.
    #[allow(clippy::too_many_arguments)]
    pub fn pad(
        &mut self,
        ch: usize,
        bar: u32,
        bars: u32,
        voices: usize,
        low: u8,
        high: u8,
        vel: f32,
    ) {
        let rpb = self.rows_per_bar();
        let mut previous: Vec<u8> = Vec::new();
        let mut held: Vec<usize> = Vec::new();
        for b in bar..bar + bars {
            let segments: Vec<u32> = self.parsed[b as usize].iter().map(|(s, _)| *s).collect();
            for (k, start) in segments.iter().enumerate() {
                let row = b * rpb + start;
                let end = b * rpb + segments.get(k + 1).copied().unwrap_or(rpb);
                let chord = self.chord_at(row).clone();
                let voicing = voice(&chord, &previous, voices, low, high);
                if voicing == previous && !held.is_empty() {
                    for &i in &held {
                        self.song.channels[ch].notes[i].len += end - row;
                    }
                } else {
                    held.clear();
                    for &key in &voicing {
                        self.note(ch, row, end - row, key, vel);
                        held.push(self.song.channels[ch].notes.len() - 1);
                    }
                    previous = voicing;
                }
            }
        }
        self.poly(ch);
        self.spread(ch, 0.55);
    }

    /// Chord stabs: the voiced chord at `rows` within each bar, `len` rows
    /// long. The channel becomes polyphonic.
    #[allow(clippy::too_many_arguments)]
    pub fn stab(
        &mut self,
        ch: usize,
        bar: u32,
        bars: u32,
        rows: &[u32],
        len: u32,
        voices: usize,
        low: u8,
        high: u8,
        vel: f32,
    ) {
        let rpb = self.rows_per_bar();
        let mut previous: Vec<u8> = Vec::new();
        for b in bar..bar + bars {
            for &r in rows {
                let row = b * rpb + r;
                let chord = self.chord_at(row).clone();
                let voicing = voice(&chord, &previous, voices, low, high);
                for &key in &voicing {
                    self.note(ch, row, len, key, vel);
                }
                previous = voicing;
            }
        }
        self.poly(ch);
        self.spread(ch, 0.4);
    }

    pub fn finish(mut self) -> Song {
        assert!(
            self.song.channels.len() <= crate::music::MAX_CHANNELS,
            "{} has {} channels; the engine plays {}",
            self.song.name,
            self.song.channels.len(),
            crate::music::MAX_CHANNELS
        );
        for channel in &mut self.song.channels {
            channel.notes.sort_by_key(|n| n.row);
        }
        for (b, chords) in self.parsed.iter().enumerate() {
            assert!(
                !chords.is_empty(),
                "{}: bar {b} has no chord",
                self.song.name
            );
        }
        self.song
    }
}

fn split_mods(token: &str) -> (&str, &str) {
    let cut = token
        .char_indices()
        .find(|(_, c)| matches!(c, '!' | ',' | '\'' | '~' | '^' | '@'))
        .map_or(token.len(), |(i, _)| i);
    token.split_at(cut)
}

/// The chord's tones from `low` upward over `octaves`.
fn ladder(chord: &Chord, low: u8, octaves: u8) -> Vec<u8> {
    let mut out = Vec::new();
    for key in low..low.saturating_add(12 * octaves) {
        if chord.tones.contains(&(key % 12)) {
            out.push(key);
        }
    }
    out
}

/// A voicing of `n` distinct chord tones within `low..=high` nearest the
/// previous one; the third (or the suspension) and the colour tone are
/// kept when there is room.
fn voice(chord: &Chord, previous: &[u8], n: usize, low: u8, high: u8) -> Vec<u8> {
    let candidates: Vec<u8> = (low..=high)
        .filter(|k| chord.tones.contains(&(k % 12)))
        .collect();
    let mut best: Option<(i32, Vec<u8>)> = None;
    let mut combo = vec![0usize; n];
    fn search(
        start: usize,
        depth: usize,
        combo: &mut Vec<usize>,
        candidates: &[u8],
        chord: &Chord,
        previous: &[u8],
        best: &mut Option<(i32, Vec<u8>)>,
    ) {
        let n = combo.len();
        if depth == n {
            let keys: Vec<u8> = combo.iter().map(|&i| candidates[i]).collect();
            let mut pcs: Vec<u8> = keys.iter().map(|k| k % 12).collect();
            pcs.sort_unstable();
            pcs.dedup();
            if pcs.len() < n.min(chord.tones.len()) {
                return;
            }
            let mut cost = 0i32;
            // The defining tones: the third, else the first colour.
            let defining = chord
                .third
                .or_else(|| chord.tones.get(2).copied())
                .unwrap_or(chord.root);
            if !pcs.contains(&defining) {
                cost += 30;
            }
            if let Some(seventh) = chord.seventh
                && !pcs.contains(&seventh)
            {
                cost += 8;
            }
            if previous.len() == keys.len() {
                for (a, b) in keys.iter().zip(previous) {
                    cost += (i32::from(*a) - i32::from(*b)).abs() * 2;
                }
            } else {
                // First chord: sit in the middle of the range.
                let mid = (i32::from(candidates[0])
                    + i32::from(*candidates.last().unwrap_or(&candidates[0])))
                    / 2;
                for k in &keys {
                    cost += (i32::from(*k) - mid).abs();
                }
            }
            // Close seconds at the bottom are mud.
            for w in keys.windows(2) {
                if w[1] - w[0] <= 2 && w[0] < 55 {
                    cost += 20;
                }
            }
            if best.as_ref().is_none_or(|(c, _)| cost < *c) {
                *best = Some((cost, keys));
            }
            return;
        }
        for i in start..candidates.len() {
            combo[depth] = i;
            search(i + 1, depth + 1, combo, candidates, chord, previous, best);
        }
    }
    search(0, 0, &mut combo, &candidates, chord, previous, &mut best);
    best.map(|(_, keys)| keys)
        .unwrap_or_else(|| panic!("no voicing for {chord:?} in {low}..={high}"))
}

/// Reads a song back as text: the plan, each channel's figures, the
/// harmony bar by bar, and anything that looks wrong.
pub fn report(song: &Song) -> String {
    let mut out = String::new();
    let rpb = song.rows_per_bar();
    let beat = song.rows_per_beat;
    let _ = writeln!(
        out,
        "# {} — {:.0} BPM, {}/{} ({} rows a beat), {} bars, {:.1} s{}\n",
        song.name,
        song.bpm,
        if song.rows_per_beat == 3 {
            song.beats_per_bar * 3
        } else {
            song.beats_per_bar
        },
        if song.rows_per_beat == 3 { 8 } else { 4 },
        song.rows_per_beat,
        song.bars,
        song.seconds(),
        match song.loop_bar {
            Some(b) => format!(", loops to bar {}", b + 1),
            None => ", plays once".into(),
        }
    );
    let drum = |c: &Channel| {
        matches!(
            c.inst.osc,
            crate::music::Osc::Kick
                | crate::music::Osc::Snare
                | crate::music::Osc::HatClosed
                | crate::music::Osc::HatOpen
                | crate::music::Osc::Shaker
                | crate::music::Osc::Crash
                | crate::music::Osc::Clap
                | crate::music::Osc::Surf
                | crate::music::Osc::Tom
        )
    };
    let _ = writeln!(
        out,
        "| channel | layer | notes | range | off-beat onsets | mean length (rows) |\n|---|---|---:|---|---:|---:|"
    );
    for c in &song.channels {
        let n = c.notes.len().max(1);
        let lo = c.notes.iter().map(|n| n.key).min().unwrap_or(0);
        let hi = c.notes.iter().map(|n| n.key).max().unwrap_or(0);
        let off = c.notes.iter().filter(|n| n.row % beat != 0).count();
        let mean = c.notes.iter().map(|n| n.len).sum::<u32>() as f32 / n as f32;
        let _ = writeln!(
            out,
            "| {} | {:?} | {} | {} | {:.0}% | {:.1} |",
            c.name,
            c.layer,
            c.notes.len(),
            if drum(c) {
                "drum".into()
            } else {
                format!("{}–{}", note_name(lo), note_name(hi))
            },
            100.0 * off as f32 / n as f32,
            mean
        );
    }

    // Harmony: the chord and each pitched channel's notes, bar by bar.
    let _ = writeln!(
        out,
        "\n## Bars\n\n(¹ first time through only, ² second time only.)\n"
    );
    for b in 0..song.bars {
        let _ = write!(
            out,
            "{:>3} {:<10}",
            b + 1,
            song.chords.get(b as usize).cloned().unwrap_or_default()
        );
        for c in song.channels.iter().filter(|c| !drum(c)) {
            let notes: Vec<String> = c
                .notes
                .iter()
                .filter(|n| n.row / rpb == b)
                .map(|n| {
                    let pass = match n.pass {
                        1 => "¹",
                        2 => "²",
                        _ => "",
                    };
                    format!("{}:{}{pass}", note_name(n.key), n.len)
                })
                .collect();
            if !notes.is_empty() {
                let _ = write!(out, " | {}: {}", c.name, notes.join(" "));
            }
        }
        let _ = writeln!(out);
    }

    // Clashes: two pitched notes a semitone (or a major seventh) apart
    // sounding together for at least a beat, and tritones outside a
    // dominant or diminished chord.
    let _ = writeln!(out, "\n## Possible clashes (≥ 1 beat of overlap)\n");
    let pitched: Vec<&Channel> = song.channels.iter().filter(|c| !drum(c)).collect();
    // A note played on a drum from a channel's extras is not a pitch.
    let is_drum_note = |c: &Channel, n: &Note| {
        n.inst > 0
            && c.extra.get(usize::from(n.inst) - 1).is_some_and(|i| {
                let probe = Channel {
                    inst: *i,
                    ..c.clone()
                };
                drum(&probe)
            })
    };
    let mut clashes = 0;
    // Layers that never sound together are not compared.
    let meet = |a: Layer, b: Layer| {
        use Layer::*;
        let range = |l: Layer| match l {
            Core | Pad | HoldOurs | HoldTheirs | HoldFinal | HoldNeedle | HoldRace | HoldLast
            | HoldEnd | HoldRiser => (0u8, 3u8),
            Pressure => (2, 2),
            Calm => (0, 0),
            Working => (1, 1),
            Unfought => (1, 2),
            Drive => (1, 3),
            Tension => (2, 3),
            Battle => (3, 3),
        };
        let (a0, a1) = range(a);
        let (b0, b1) = range(b);
        // Hold layers force intensity two or more.
        let holds = |l: Layer| {
            matches!(
                l,
                HoldOurs
                    | HoldTheirs
                    | HoldFinal
                    | HoldNeedle
                    | HoldRace
                    | HoldLast
                    | HoldEnd
                    | HoldRiser
            )
        };
        let (a0, b0) = (
            if holds(b) { a0.max(2) } else { a0 },
            if holds(a) { b0.max(2) } else { b0 },
        );
        a0.max(b0) <= a1.min(b1) && !(holds(a) && a1 < 2) && !(holds(b) && b1 < 2)
    };
    // The enemy's count grinds on purpose; its rubs are counted apart.
    let dread = |l: Layer| {
        matches!(
            l,
            Layer::HoldTheirs
                | Layer::HoldNeedle
                | Layer::HoldRace
                | Layer::HoldLast
                | Layer::HoldEnd
                | Layer::HoldRiser
        )
    };
    let mut intended = 0usize;
    for (i, a) in pitched.iter().enumerate() {
        for b in pitched
            .iter()
            .skip(i + 1)
            .filter(|b| meet(a.layer, b.layer))
        {
            if dread(a.layer) || dread(b.layer) {
                for na in a.notes.iter().filter(|n| !is_drum_note(a, n)) {
                    for nb in b.notes.iter().filter(|n| !is_drum_note(b, n)) {
                        let start = na.row.max(nb.row);
                        let end = (na.row + na.len).min(nb.row + nb.len);
                        if end > start
                            && end - start >= beat
                            && matches!(
                                (i32::from(na.key) - i32::from(nb.key)).rem_euclid(12),
                                1 | 11
                            )
                        {
                            intended += 1;
                        }
                    }
                }
                continue;
            }
            for na in a.notes.iter().filter(|n| !is_drum_note(a, n)) {
                for nb in b.notes.iter().filter(|n| !is_drum_note(b, n)) {
                    let start = na.row.max(nb.row);
                    let end = (na.row + na.len).min(nb.row + nb.len);
                    if end <= start || end - start < beat {
                        continue;
                    }
                    let interval = (i32::from(na.key) - i32::from(nb.key)).rem_euclid(12);
                    if matches!(interval, 1 | 11) {
                        clashes += 1;
                        if clashes <= 60 {
                            let _ = writeln!(
                                out,
                                "- bar {} beat {}: {} {} against {} {} ({})",
                                start / rpb + 1,
                                (start % rpb) / beat + 1,
                                a.name,
                                note_name(na.key),
                                b.name,
                                note_name(nb.key),
                                song.chords
                                    .get((start / rpb) as usize)
                                    .cloned()
                                    .unwrap_or_default()
                            );
                        }
                    }
                }
            }
        }
    }
    if clashes == 0 {
        let _ = writeln!(out, "none");
    } else if clashes > 60 {
        let _ = writeln!(out, "- … {} in all", clashes);
    }
    if intended > 0 {
        let _ = writeln!(
            out,
            "\n({intended} intended rubs against the enemy-count layers not listed.)"
        );
    }

    // Leads on a strong beat outside the chord for a beat or more.
    let _ = writeln!(
        out,
        "\n## Lead tones outside the chord on strong beats (≥ 1 beat)\n"
    );
    let mut outside = 0;
    for c in song.channels.iter().filter(|c| {
        c.name.starts_with("lead")
            || c.name.starts_with("counter")
            || c.name.starts_with("flute")
            || c.name.starts_with("bell")
    }) {
        for n in &c.notes {
            let pos = n.row % rpb;
            let strong = pos == 0 || (song.beats_per_bar == 4 && pos == 2 * beat);
            if !strong || n.len < beat {
                continue;
            }
            let bar = (n.row / rpb) as usize;
            let Some(symbol) = song.chords.get(bar) else {
                continue;
            };
            let half = rpb / 2;
            let symbol = symbol
                .split('+')
                .nth(if pos >= half { 1 } else { 0 })
                .unwrap_or(symbol);
            let ch = chord(symbol);
            if !ch.tones.contains(&(n.key % 12)) {
                outside += 1;
                let _ = writeln!(
                    out,
                    "- {} bar {} beat {}: {} over {}",
                    c.name,
                    bar + 1,
                    pos / beat + 1,
                    note_name(n.key),
                    symbol
                );
            }
        }
    }
    if outside == 0 {
        let _ = writeln!(out, "none");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_chords_read_as_written() {
        assert_eq!(midi("C4"), 60);
        assert_eq!(midi("Bb3"), 58);
        assert_eq!(midi("F#5"), 78);
        assert_eq!(note_name(62), "D4");
        let d = chord("Dm");
        assert_eq!(d.tones, vec![2, 5, 9]);
        let g = chord("G/D");
        assert_eq!(g.bass, 2);
        assert_eq!(g.tones, vec![7, 11, 2]);
        let a = chord("A7");
        assert_eq!(a.seventh, Some(7));
    }

    #[test]
    fn voicings_move_little() {
        let first = voice(&chord("Dm"), &[], 3, 57, 74);
        let next = voice(&chord("G"), &first, 3, 57, 74);
        let moved: i32 = first
            .iter()
            .zip(&next)
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .sum();
        assert!(moved <= 6, "{first:?} to {next:?}");
    }
}
