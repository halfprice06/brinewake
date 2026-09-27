//! The soundtrack. One instrument bank and one motif run through every
//! song so they read as one record: the tide motif (A–D–E–F, a rise to
//! the crest and a fall) in D Dorian, the sea-shanty mode, with the
//! major sixth (B) as the colour of the reclaimed land and the flat
//! second (Eb) as the colour of the water taking it back.

use crate::music::{Instrument, Layer, Osc, Role, Song, SongId};
use crate::score::{Builder, midi};

pub static WAVE_SOFT: [u8; 32] = [
    8, 9, 11, 12, 14, 14, 15, 15, 15, 14, 14, 13, 12, 11, 10, 9, 8, 6, 5, 4, 3, 2, 1, 1, 0, 0, 0,
    1, 1, 3, 4, 6,
];
pub static WAVE_REED: [u8; 32] = [
    9, 13, 15, 15, 14, 12, 12, 11, 11, 11, 11, 11, 12, 12, 12, 10, 7, 4, 3, 3, 5, 6, 6, 5, 4, 3, 2,
    1, 0, 0, 1, 4,
];
pub static WAVE_HOLLOW: [u8; 32] = [
    7, 12, 15, 15, 14, 13, 13, 14, 14, 14, 13, 13, 14, 15, 15, 12, 8, 3, 0, 0, 1, 2, 2, 1, 1, 1, 2,
    2, 1, 0, 0, 3,
];

// ---------------------------------------------------------------------
// The instrument bank.

/// A breathy wave-channel flute with a late vibrato.
pub(crate) fn flute() -> Instrument {
    Instrument::new(Osc::Wave(&WAVE_SOFT))
        .adsr(0.035, 0.35, 0.82, 0.22)
        .gain(0.55)
        .filter(5_500.0, 0.7, 0.3)
        .vibrato(0.22, 5.2, 22.0)
        .glide(0.07)
        .breath(0.06)
}

/// A bowed low voice: a filtered saw that swells.
pub(crate) fn cello() -> Instrument {
    Instrument::new(Osc::Saw)
        .adsr(0.09, 0.5, 0.85, 0.35)
        .gain(0.42)
        .filter(900.0, 0.9, 0.6)
        .track(0.5)
        .vibrato(0.3, 4.8, 14.0)
}

/// Struck saltglass: inharmonic FM that rings and dies.
pub(crate) fn bell() -> Instrument {
    Instrument::new(Osc::Fm {
        ratio: 3.5,
        index: 2.4,
        decay: 0.5,
        floor: 0.15,
    })
    .adsr(0.001, 1.8, 0.0, 1.2)
    .gain(0.34)
}

/// A soft mallet for the ostinato: short FM with a woody attack.
pub(crate) fn mallet() -> Instrument {
    Instrument::new(Osc::Fm {
        ratio: 4.0,
        index: 1.8,
        decay: 0.04,
        floor: 0.15,
    })
    .adsr(0.001, 0.32, 0.0, 0.15)
    .gain(0.42)
}

/// Thin chip plucks for arpeggios.
pub(crate) fn pluck() -> Instrument {
    Instrument::new(Osc::Pulse {
        duty: 0.125,
        sweep: 0.0,
    })
    .adsr(0.001, 0.16, 0.0, 0.08)
    .gain(0.3)
    .filter(4_500.0, 0.8, 1.0)
}

pub(crate) fn pad() -> Instrument {
    Instrument::new(Osc::Pad {
        duty: 0.3,
        detune: 9.0,
    })
    .adsr(0.6, 1.2, 0.8, 1.4)
    .gain(0.2)
    .filter(3_400.0, 0.7, 0.35)
}

/// The console triangle bass.
pub(crate) fn bass() -> Instrument {
    Instrument::new(Osc::Triangle { stepped: true })
        .adsr(0.003, 0.4, 0.75, 0.07)
        .gain(0.36)
        .filter(2_400.0, 0.7, 0.0)
}

/// A driving pulse bass in the bass register: its harmonics carry it on
/// small speakers.
pub(crate) fn drive_bass() -> Instrument {
    Instrument::new(Osc::Pulse {
        duty: 0.5,
        sweep: 0.0,
    })
    .adsr(0.002, 0.12, 0.45, 0.05)
    .gain(0.5)
    .filter(900.0, 1.1, 1.5)
}

/// Brass stabs: a saw whose filter snaps open.
pub(crate) fn brass() -> Instrument {
    Instrument::new(Osc::Saw)
        .adsr(0.006, 0.12, 0.35, 0.09)
        .gain(0.38)
        .filter(2_400.0, 1.0, 3.0)
}

/// The Union's lead: a brassy pulse, its width breathing, bright on the
/// attack.
pub(crate) fn lead_union() -> Instrument {
    Instrument::new(Osc::Pulse {
        duty: 0.25,
        sweep: 1.2,
    })
    .adsr(0.008, 0.3, 0.72, 0.16)
    .gain(0.34)
    .filter(3_200.0, 0.9, 1.4)
    .vibrato(0.18, 5.6, 26.0)
}

/// The Assembly's lead: a reed with a slow swell and more vibrato.
pub(crate) fn lead_assembly() -> Instrument {
    Instrument::new(Osc::Wave(&WAVE_REED))
        .adsr(0.03, 0.4, 0.8, 0.2)
        .gain(0.46)
        .filter(3_600.0, 0.8, 0.6)
        .vibrato(0.18, 5.0, 28.0)
        .breath(0.05)
}

/// The Compact's lead: glassy FM that keeps its edge while it holds.
pub(crate) fn lead_compact() -> Instrument {
    Instrument::new(Osc::Fm {
        ratio: 2.0,
        index: 1.8,
        decay: 0.45,
        floor: 0.6,
    })
    .adsr(0.004, 0.5, 0.66, 0.22)
    .gain(0.42)
    .vibrato(0.18, 5.8, 22.0)
}

/// A hollow reed: the working melody, and the counter-line under the fight.
pub(crate) fn reed() -> Instrument {
    Instrument::new(Osc::Wave(&WAVE_HOLLOW))
        .adsr(0.02, 0.3, 0.75, 0.16)
        .gain(0.4)
        .filter(4_500.0, 0.8, 0.8)
        .vibrato(0.25, 5.0, 20.0)
        .breath(0.03)
}

/// A low saw that grinds: the enemy's count.
pub(crate) fn dread() -> Instrument {
    Instrument::new(Osc::Saw)
        .adsr(0.03, 0.4, 0.7, 0.2)
        .gain(0.55)
        .filter(700.0, 1.3, 0.8)
        .vibrato(0.1, 3.0, 8.0)
}

pub(crate) fn drum(osc: Osc, gain: f32, tail: f32) -> Instrument {
    Instrument::new(osc).adsr(0.0005, 0.1, 1.0, tail).gain(gain)
}

pub(crate) fn surf() -> Instrument {
    Instrument::new(Osc::Surf)
        .adsr(1.6, 2.2, 0.35, 2.8)
        .gain(0.13)
}

/// Each faction's clank of work: the Union's struck iron, the Assembly's
/// wood block, the Compact's glass.
pub(crate) fn work_clank() -> [Instrument; 3] {
    [
        drum(Osc::Anvil, 0.16, 0.5),
        drum(Osc::Tom, 0.22, 0.1),
        Instrument::new(Osc::Fm {
            ratio: 2.756,
            index: 1.4,
            decay: 0.08,
            floor: 0.1,
        })
        .adsr(0.001, 0.4, 0.0, 0.2)
        .gain(0.2),
    ]
}

pub fn songs() -> Vec<Song> {
    vec![
        title(),
        reclamation(),
        victory(),
        defeat(),
        draw(),
        harbour(),
        crate::score_confluence::confluence(),
        crate::score_undertow::undertow(),
    ]
}

// ---------------------------------------------------------------------
// BRINEWAKE — the title. 12/8, a slow sea-swell lilt.

fn title() -> Song {
    let mut s = Builder::new(SongId::Title, "Brinewake", 64.0, 3, 4, 40);
    s.song.echo_rows = 3.0;
    s.song.echo_feedback = 0.38;
    s.song.gain = 1.0;
    s.chords(0, "Dm Dm Bb C");
    s.chords(4, "Dm G/D Dm C Bb F Gm A");
    s.chords(12, "Dm G/D Dm C Bb C Dm Dm");
    s.chords(20, "F C Bb F Gm Dm Eb A");
    s.chords(28, "Dm G/D Dm C Bb C A Dm");
    s.chords(36, "Dm Bb C Dm");

    let flute = s.channel("lead flute", flute(), Layer::Core, -0.15, 1.0, 0.22, 0.32);
    let cello = s.channel("counter cello", cello(), Layer::Core, 0.35, 1.0, 0.1, 0.3);
    let bell = s.channel("bell", bell(), Layer::Core, 0.55, 0.9, 0.3, 0.5);
    let arp = s.channel("arp", pluck(), Layer::Core, -0.6, 1.0, 0.3, 0.25);
    let pad_ch = s.channel("pad", pad(), Layer::Core, 0.0, 1.0, 0.0, 0.45);
    let bass_ch = s.channel("bass", bass(), Layer::Core, 0.0, 1.0, 0.0, 0.08);
    let surf_ch = s.channel("surf", surf(), Layer::Core, 0.0, 1.0, 0.0, 0.5);
    let kit = s.channel(
        "kit",
        drum(Osc::Kick, 0.62, 0.4),
        Layer::Core,
        0.0,
        1.0,
        0.0,
        0.1,
    );
    s.extra(kit, drum(Osc::Snare, 0.32, 0.3));
    s.extra(kit, drum(Osc::Tom, 0.42, 0.4));
    s.extra(kit, drum(Osc::Crash, 0.22, 1.6));
    s.poly(kit);
    let shaker = s.channel(
        "shaker",
        drum(Osc::Shaker, 0.26, 0.1),
        Layer::Core,
        0.5,
        1.0,
        0.0,
        0.15,
    );

    // Intro: the sea, and the motif rung high on glass.
    s.line(
        bell,
        0,
        "A5:3 D6:2 E6:1, F6:6 | r:12 | F5:3 D5:3 A5:6 | G5:3 E5:3 C5:3 r:3",
        0.7,
    );
    for bar in [0, 2, 36, 38] {
        s.note(surf_ch, bar * 12, 18, midi("D3"), 0.9);
    }
    for bar in [8, 16, 24, 32] {
        s.note(surf_ch, bar * 12, 18, midi("D3"), 0.45);
    }
    s.bass(bass_ch, 0, 4, "R:12", 38, 0.7);
    s.pad(pad_ch, 0, 40, 3, 53, 69, 0.55);

    // A: the theme.
    s.line(
        flute,
        4,
        "| A4:3 D5:2 E5:1, F5:4! E5:2 | D5:2 B4:1, G4:3 A4:6~ | A4:3 D5:2 E5:1, F5:2 G5:1, A5:3! \
         | G5:4 F5:2 E5:3 C5:3 | D5:3 F5:2 D5:1, Bb4:6~ | C5:3 A4:2 C5:1, F5:6! \
         | E5:2 D5:1, Bb4:3 G4:3 A4:2 Bb4:1, | A4:6~ G4:2 A4:1, C#5:3 |",
        0.72,
    );
    s.bass(bass_ch, 4, 7, "R:6 5:3 R:3,", 38, 0.75);
    s.bass(bass_ch, 11, 1, "R:6 5:3 a:3", 38, 0.75);

    // A': ornamented, the arpeggio joins, a second ending that lands.
    s.line(
        flute,
        12,
        "| A4:2 C5:1, D5:2 E5:1, F5:4! E5:1 D5:1, | D5:2 B4:1, G4:3 A4:3 B4:2 C5:1, \
         | D5:2 E5:1, F5:2 G5:1, A5:6!~ | G5:4 E5:2 C5:3 E5:3 | D5:3 F5:2 D5:1, F5:3 G5:3 \
         | A5:4! G5:2 E5:3 G5:3 | F5:3 E5:2 D5:1, C5:3 E5:3 | D5:12~ |",
        0.74,
    );
    s.line(
        cello,
        12,
        "| D3:12 | B2:12 | D3:6 A2:6 | C3:6 E3:6 | D3:6 Bb2:6 | C3:6 E3:6 | A2:6 C3:6 | D3:12 |",
        0.5,
    );
    s.arp(arp, 12, 8, "0! 1 2, 3 2, 1,", 1, 62, 1, 0.5);
    s.bass(bass_ch, 12, 7, "R:6 5:3 O:3,", 38, 0.75);
    s.bass(bass_ch, 19, 1, "R:6 5:3 a:3", 38, 0.75);
    s.beat(shaker, 12, 8, "x.gx.gx.gx.g", 60, 0.5);

    // B: the relative major, the melody down in the cello, glass answers.
    s.line(
        cello,
        20,
        "| F4:4 E4:2 C4:6~ | E4:4 D4:2 G3:6~ | D4:4 C4:2 Bb3:3 D4:3 | C4:12~ \
         | Bb3:4 A3:2 G3:3 Bb3:3 | A3:4 F3:2 D4:6~ | G4:6! Eb4:3 Bb3:3 | A3:6 C#4:3 E4:3 |",
        0.78,
    );
    s.line(
        bell,
        20,
        "| r:12 | r:6 G5:3 E5:3 | r:12 | r:6 A5:2 G5:1, F5:3 | r:12 | r:6 F5:2 E5:1, D5:3 | r:12 | r:6 E5:3 C#5:3 |",
        0.62,
    );
    s.arp(arp, 20, 8, "0! 2 4, 2 1, 3", 1, 62, 1, 0.42);
    s.bass(bass_ch, 20, 7, "R:3 5:3, O:3 5:3,", 38, 0.72);
    s.bass(bass_ch, 27, 1, "R:3 5:3 3:3 a:3", 38, 0.72);
    s.beat(kit, 20, 8, "x...........", 38, 0.55);

    // A'': everything; the climb reaches D6; a fill into the return.
    s.line(
        flute,
        28,
        "| A4:3 D5:2 E5:1, F5:4! E5:2 | D5:2 B4:1, G4:3 A4:6~ | A4:3 D5:2 E5:1, F5:2 G5:1, A5:3 \
         | G5:4 F5:2 E5:3 C5:3 | D5:3 F5:2 A5:1, D6:6!~ | C6:4 A5:2 G5:3 E5:3 \
         | E5:3 C#5:2 D5:1, E5:3 G5:3 | F5:6 E5:3 D5:3 | D5:12~ |",
        0.8,
    );
    s.line(cello, 28, "| D3:6 F3:6 | G3:6 B2:6 | D3:6 A3:6 | C3:6 G3:6 | Bb2:6 D3:6 | C3:6 E3:6 | A2:6 C#3:6 | D3:12 |", 0.55);
    s.arp(arp, 28, 8, "0! 1 2, 3 2, 1,", 1, 62, 1, 0.55);
    s.bass(bass_ch, 28, 7, "R:3 R:2, 5:1, O:3 5:3,", 38, 0.8);
    s.bass(bass_ch, 35, 1, "R:12", 38, 0.8);
    s.beat(kit, 28, 7, "x.....1..x..", 38, 0.62);
    s.line(
        kit,
        34,
        "r:6 A2:1@2 F2:1@2 D2:1@2 A1:1@2 F1:1@2 D1:1@2",
        0.7,
    );
    s.line(kit, 28, "D4:12@3", 0.7);
    s.line(kit, 36, "D4:12@3", 0.6);
    s.beat(shaker, 28, 8, "X.gx.gX.gx.g", 60, 0.6);

    // Outro: the flute holds the tonic, glass echoes the motif, the sea.
    s.line(
        bell,
        37,
        "| D5:3 F5:3 A5:6 | G5:3 E5:3 C5:6 | A4:3 D5:3 r:6 |",
        0.55,
    );
    s.bass(bass_ch, 36, 4, "R:12", 38, 0.65);
    s.finish()
}

// ---------------------------------------------------------------------
// Layers every match song shares.

/// A kit figure split in two: the kick (`x`, `X`, `g`) and everything
/// else (digits pick the kit's extras) — so the kick can run under every
/// working state while the snare on three belongs to the unfought ones.
pub(crate) fn split_kit(pattern: &str) -> (String, String) {
    let kick = pattern
        .chars()
        .map(|c| if c.is_ascii_digit() { '.' } else { c })
        .collect();
    let rest = pattern
        .chars()
        .map(|c| match c {
            '1' => 'x',
            d if d.is_ascii_digit() => d,
            _ => '.',
        })
        .collect::<String>()
        .replace('2', "1");
    (kick, rest)
}

/// The holds, the tension tick and the battle entry, pitched to a tonic
/// (`tonic` is its MIDI note in the third octave: D3 is 50).
pub(crate) fn match_layers(s: &mut Builder, tonic: u8, bars: u32) {
    let name = |k: u8| crate::score::note_name(k);
    // Tension (intensity 2 only): a thin tick high above everything, on
    // the chord, and hats in sixteenths with their opens on the off-beat
    // eighths.
    let tick = s.channel(
        "tension tick",
        Instrument::new(Osc::Pulse {
            duty: 0.125,
            sweep: 0.0,
        })
        .adsr(0.001, 0.07, 0.0, 0.03)
        .gain(0.8)
        .filter(6_000.0, 0.8, 0.8),
        Layer::Pressure,
        0.55,
        1.0,
        0.15,
        0.15,
    );
    s.role(tick, Role::Pulse);
    // Three octaves up, or two when that would pass E6.
    let tick_low = if tonic + 36 > 88 {
        tonic + 24
    } else {
        tonic + 36
    };
    s.arp(tick, 0, bars, "2! 0, 0, 2 0, 0, 2 0,", 1, tick_low, 1, 0.75);
    let tension_hats = s.channel(
        "tension hats",
        drum(Osc::HatClosed, 0.34, 0.04),
        Layer::Tension,
        -0.55,
        1.0,
        0.0,
        0.08,
    );
    s.extra(tension_hats, drum(Osc::HatOpen, 0.16, 0.2));
    s.poly(tension_hats);
    s.role(tension_hats, Role::Kit);
    s.beat(tension_hats, 0, bars, "xg1gxg1gxg1gxg1g", 60, 0.6);

    // Our count: a bright clock of bells; its last thirty seconds race.
    let ours = s.channel(
        "bell hold ours",
        bell(),
        Layer::HoldOurs,
        0.55,
        0.8,
        0.35,
        0.4,
    );
    s.poly(ours);
    // Two octaves up, or one when that would pass E5 (the clock reaches
    // an octave and a fifth above it).
    let bell_low = if tonic + 24 > 76 {
        tonic + 12
    } else {
        tonic + 24
    };
    s.arp(
        ours,
        0,
        bars,
        "4! - 2 - 3 - 1, - 4 - 2 - 3 - 2, -",
        1,
        bell_low,
        2,
        0.5,
    );
    let final_hats = s.channel(
        "final hats",
        drum(Osc::HatClosed, 0.36, 0.04),
        Layer::HoldFinal,
        0.35,
        1.0,
        0.0,
        0.05,
    );
    s.role(final_hats, Role::Kit);
    s.beat(final_hats, 0, bars, "XxxxXxxxXxxxXxxx", 60, 0.45);

    // The enemy's count escalates: a grinding semitone from the start; a
    // needle of eighths from a minute out; from thirty seconds, sixteenths,
    // racing hats and a dread pad that grow as the seconds go, with the
    // tune silenced; for the last ten, every pulse stops and one needle
    // holds (trilling for the last five) under the dread, so the toll is
    // the only clock.
    let dread_ch = s.channel(
        "dread hold theirs",
        dread(),
        Layer::HoldTheirs,
        -0.2,
        1.0,
        0.1,
        0.3,
    );
    let dread_bar = format!("| {}:8 {}:8 ", name(tonic), name(tonic + 1));
    s.line(dread_ch, 0, &(dread_bar.repeat(bars as usize) + "|"), 0.5);
    let needle = |gain: f32| {
        Instrument::new(Osc::Pulse {
            duty: 0.125,
            sweep: 0.0,
        })
        .adsr(0.001, 0.06, 0.3, 0.03)
        .gain(gain)
        .filter(8_000.0, 0.8, 1.5)
    };
    let high = tonic + 31;
    let needle8 = s.channel(
        "needle hold theirs",
        needle(0.55),
        Layer::HoldNeedle,
        0.45,
        1.0,
        0.25,
        0.2,
    );
    s.role(needle8, Role::Race);
    let bar8 = "| ".to_string() + &format!("{}:2 {}:2, ", name(high), name(high + 1)).repeat(4);
    s.line(needle8, 0, &(bar8.repeat(bars as usize) + "|"), 0.6);
    let needle16 = s.channel(
        "needle race",
        needle(1.0),
        Layer::HoldRace,
        -0.45,
        1.0,
        0.25,
        0.2,
    );
    s.role(needle16, Role::Race);
    let bar16 = "| ".to_string() + &format!("{}:1 {}:1, ", name(high), name(high + 1)).repeat(8);
    s.line(needle16, 0, &(bar16.repeat(bars as usize) + "|"), 0.6);
    let race_hats = s.channel(
        "race hats",
        drum(Osc::HatClosed, 0.7, 0.04),
        Layer::HoldRace,
        0.25,
        1.0,
        0.0,
        0.05,
    );
    s.role(race_hats, Role::Race);
    s.beat(race_hats, 0, bars, "XxxxXxxxXxxxXxxx", 60, 0.9);
    // War drums under the race; they stop with the other drums at ten.
    let war = s.channel(
        "war drums",
        drum(Osc::Tom, 1.0, 0.4),
        Layer::HoldRace,
        0.0,
        1.0,
        0.0,
        0.2,
    );
    s.role(war, Role::Kit);
    s.poly(war);
    // An octave up from the toms of the song, so a laptop hears them.
    let (hi, lo) = (name(tonic + 12), name(tonic + 7));
    let war_bar =
        format!("| {hi}:2! {lo}:2, {hi}:2 {lo}:2, {hi}:2! {lo}:2, {hi}:1 {hi}:1 {lo}:2! ");
    s.line(war, 0, &(war_bar.repeat(bars as usize) + "|"), 0.95);
    // And a snare roll that tightens toward ten seconds.
    let roll = s.channel(
        "race roll",
        drum(Osc::Snare, 0.75, 0.2).filter(4_500.0, 0.8, 0.0),
        Layer::HoldRace,
        -0.1,
        1.0,
        0.0,
        0.2,
    );
    s.role(roll, Role::Kit);
    s.poly(roll);
    // The roll leans toward the downbeat.
    let rpb_roll = s.rows_per_bar();
    for bar in 0..bars {
        for k in 0..rpb_roll {
            let vel = 0.35 + 0.55 * k as f32 / rpb_roll as f32;
            if k % 4 != 1 {
                s.note(roll, bar * rpb_roll + k, 1, 60, vel);
            }
        }
    }
    // The song's bass folds to a pedal on the tonic under the race.
    let pedal = s.channel(
        "race pedal",
        drive_bass(),
        Layer::HoldRace,
        0.0,
        1.0,
        0.0,
        0.04,
    );
    s.role(pedal, Role::Race);
    let pedal_bar =
        "| ".to_string() + &format!("{}:2 {}:2, ", name(tonic - 12), name(tonic)).repeat(4);
    s.line(pedal, 0, &(pedal_bar.repeat(bars as usize) + "|"), 0.7);
    // A riser into the last ten seconds.
    let riser = s.channel(
        "riser",
        Instrument::new(Osc::Surf)
            .adsr(3.5, 1.0, 1.0, 0.4)
            .gain(0.3),
        Layer::HoldRiser,
        0.0,
        1.0,
        0.0,
        0.3,
    );
    s.role(riser, Role::Race);
    for bar in 0..bars {
        s.note(riser, bar * s.rows_per_bar(), s.rows_per_bar(), tonic, 0.9);
    }
    // The race's dread pad stops with the race's pulses at ten seconds; a
    // quieter one holds the last ten.
    let dread_pad = s.channel("dread pad", pad(), Layer::HoldRace, 0.0, 0.9, 0.0, 0.4);
    s.role(dread_pad, Role::Race);
    let last_pad = s.channel("dread pad last", pad(), Layer::HoldLast, 0.0, 0.3, 0.0, 0.4);
    // The tonic minor and the flat second's major, a bar each.
    for bar in (0..bars).step_by(2) {
        // Voiced up in D4–A4, where the tune was.
        for (k, keys) in [
            [tonic + 12, tonic + 15, tonic + 19],
            [tonic + 13, tonic + 17, tonic + 20],
        ]
        .iter()
        .enumerate()
        {
            for &key in keys {
                s.note(
                    dread_pad,
                    (bar + k as u32) * s.rows_per_bar(),
                    s.rows_per_bar(),
                    key,
                    0.7,
                );
                s.note(
                    last_pad,
                    (bar + k as u32) * s.rows_per_bar(),
                    s.rows_per_bar(),
                    key,
                    0.7,
                );
            }
        }
    }
    s.poly(dread_pad);
    s.spread(dread_pad, 0.5);
    s.poly(last_pad);
    s.spread(last_pad, 0.5);
    let held = s.channel(
        "needle held",
        needle(0.2).adsr(0.02, 0.5, 0.8, 0.3),
        Layer::HoldLast,
        0.3,
        1.0,
        0.3,
        0.3,
    );
    let trill = s.channel(
        "needle trill",
        needle(0.22).adsr(0.02, 0.5, 0.8, 0.3),
        Layer::HoldEnd,
        -0.3,
        1.0,
        0.3,
        0.3,
    );
    let rpb = s.rows_per_bar();
    for bar in 0..bars {
        s.note(held, bar * rpb, rpb, high, 0.6);
        s.push(
            trill,
            crate::music::Note {
                row: bar * rpb,
                len: rpb,
                key: high,
                vel: 0.55,
                slide: 0,
                arp: [1, 0],
                inst: 0,
                pass: 0,
            },
        );
    }

    // Battle comes in with a tom pickup on the last beat and a crash.
    let entry = s.channel(
        "battle entry",
        drum(Osc::Tom, 0.5, 0.4),
        Layer::Core,
        0.15,
        1.0,
        0.0,
        0.25,
    );
    s.extra(entry, drum(Osc::Crash, 0.26, 1.8));
    s.extra(entry, drum(Osc::Kick, 0.7, 0.4));
    s.poly(entry);
    let fill = format!(
        "{}:1! {}:1 {}:1, {}:1 D2:16@2",
        name(tonic),
        name(tonic - 5),
        name(tonic - 9),
        name(tonic - 12)
    );
    s.line(entry, 0, &fill, 0.7);
    s.line(entry, 0, "r:4 D4:16@1", 0.7);
    s.entry(entry, Layer::Battle);
}

// ---------------------------------------------------------------------
// RECLAMATION — the Split Basin match. 4/4, one melody passed between
// voices as the match changes: the flute when all is calm, the reed while
// the harbour works or the enemy gathers (resting two sections in eight,
// where glass answers it), the faction's own lead in a fight, in a
// sharper rhythm, with the reed below it. The bass is roots when calm, a
// walking figure while working, and a pumping pulse under threat.

fn reclamation() -> Song {
    let mut s = Builder::new(SongId::Match, "Reclamation", 112.0, 4, 4, 64);
    s.song.partner = Some(SongId::Confluence);
    s.song.sections = (0..8).map(|k| k * 8).collect();
    // Bars on A (the dominant here, the subdominant of E) lead into
    // Confluence; the last is the song's end.
    s.song.handovers = vec![7, 23, 31, 55, 63];
    s.song.swing = 0.08;
    s.song.echo_rows = 4.0;
    s.song.echo_feedback = 0.32;
    let sections = [
        "Dm G/D Bb C Dm G/D Bb A",
        "Dm F C G Dm F Bb C",
        "Gm Dm Bb F Gm Dm Eb A",
        "Bb C Dm Dm Bb C A A7",
        "Dm G/D Bb C Dm G/D Bb G/D",
        "F C Dm Bb F C Gm Bb",
        "Gm Dm Bb F Eb Bb Gm A",
        "Bb C Dm Bb Gm A Dm A",
    ];
    for (i, section) in sections.iter().enumerate() {
        s.chords(i as u32 * 8, section);
    }
    // The valleys, where the clock and the drums thin out.
    let valley = |bar: u32| (24..28).contains(&bar) || (56..60).contains(&bar);

    let roots = s.channel("bass roots", bass(), Layer::Calm, 0.0, 1.0, 0.0, 0.05);
    let figure = s.channel("bass figure", bass(), Layer::Working, 0.0, 1.0, 0.0, 0.05);
    let edge = s.channel(
        "bass edge",
        Instrument::new(Osc::Pulse {
            duty: 0.25,
            sweep: 0.0,
        })
        .adsr(0.002, 0.2, 0.5, 0.05)
        .gain(0.16)
        .filter(1_400.0, 0.8, 0.5),
        Layer::Working,
        0.1,
        1.0,
        0.0,
        0.05,
    );
    let pump = s.channel(
        "pump bass",
        drive_bass(),
        Layer::Tension,
        0.0,
        1.0,
        0.0,
        0.04,
    );
    for ch in [roots, figure, edge] {
        s.role(ch, Role::Bass);
    }
    s.role(pump, Role::Pulse);
    let pad_ch = s.channel("pad", pad(), Layer::Pad, 0.0, 0.8, 0.0, 0.4);
    let motor = s.channel("motor", mallet(), Layer::Core, 0.5, 0.8, 0.2, 0.25);
    s.role(motor, Role::Pulse);
    let hats = s.channel(
        "hats",
        drum(Osc::HatClosed, 0.41, 0.05),
        Layer::Core,
        -0.45,
        1.0,
        0.0,
        0.1,
    );
    s.role(hats, Role::Kit);
    let flute_ch = s.channel("flute", flute(), Layer::Calm, -0.2, 0.95, 0.25, 0.35);
    s.role(flute_ch, Role::Melody);
    let kit = s.channel(
        "kit",
        drum(Osc::Kick, 0.45, 0.4),
        Layer::Drive,
        0.0,
        1.0,
        0.0,
        0.08,
    );
    s.role(kit, Role::Kit);
    let snare3 = s.channel(
        "snare on three",
        drum(Osc::Snare, 0.42, 0.3),
        Layer::Unfought,
        0.05,
        1.0,
        0.0,
        0.1,
    );
    s.extra(snare3, drum(Osc::Clap, 0.3, 0.2));
    s.poly(snare3);
    s.role(snare3, Role::Kit);
    let melody = s.channel("lead reed", reed(), Layer::Unfought, 0.15, 1.0, 0.22, 0.3);
    s.role(melody, Role::Melody);
    let answer = s.channel(
        "bell answers",
        bell(),
        Layer::Unfought,
        -0.35,
        0.8,
        0.35,
        0.5,
    );
    let canon = s.channel(
        "pluck answers",
        pluck(),
        Layer::Unfought,
        0.45,
        1.0,
        0.3,
        0.3,
    );
    s.poly(canon);
    // While the harbour works: a counter-line where the reed rests and all
    // through section 7 (its climb), and the pad voiced up in the rests.
    let working_counter = s.channel(
        "counter working",
        reed(),
        Layer::Unfought,
        0.4,
        0.8,
        0.15,
        0.28,
    );
    let pad_high = s.channel("pad high", pad(), Layer::Unfought, 0.0, 0.7, 0.0, 0.4);
    // All of it gives way to an enemy count's race.
    for ch in [answer, canon, working_counter] {
        s.role(ch, Role::Melody);
    }
    s.role(pad_high, Role::Pulse);
    let counter = s.channel("counter reed", reed(), Layer::Battle, 0.4, 0.9, 0.15, 0.28);
    s.role(counter, Role::Melody);
    let clank = s.channel(
        "clank",
        drum(Osc::Anvil, 0.16, 0.5),
        Layer::Drive,
        0.65,
        1.0,
        0.3,
        0.3,
    );
    s.variants(clank, work_clank());
    s.role(clank, Role::Kit);
    let toms = s.channel(
        "toms",
        drum(Osc::Tom, 0.5, 0.4),
        Layer::Tension,
        0.2,
        1.0,
        0.0,
        0.2,
    );
    s.extra(toms, drum(Osc::Crash, 0.2, 1.6));
    s.poly(toms);
    s.role(toms, Role::Kit);
    let brass_ch = s.channel("brass", brass(), Layer::Tension, -0.35, 1.0, 0.1, 0.25);
    s.role(brass_ch, Role::Pulse);
    let lead = s.channel("lead", lead_union(), Layer::Battle, 0.0, 1.0, 0.3, 0.3);
    s.variants(lead, [lead_union(), lead_assembly(), lead_compact()]);
    s.role(lead, Role::Melody);
    let arp = s.channel("arp", pluck(), Layer::Battle, -0.65, 0.9, 0.2, 0.2);
    s.role(arp, Role::Pulse);
    let battle_kit = s.channel(
        "battle kit",
        drum(Osc::Snare, 0.4, 0.3),
        Layer::Battle,
        0.05,
        1.0,
        0.0,
        0.15,
    );
    s.extra(battle_kit, drum(Osc::HatOpen, 0.2, 0.3));
    s.poly(battle_kit);
    s.role(battle_kit, Role::Kit);

    // Bass: roots when calm, a figure while working (doubled an octave up
    // by a thin pulse so small speakers carry it), a pump under threat.
    let figures = [
        "R:4 r:2 R:2, 5:4 R:4",
        "R:2 R:2, O:2 R:2, 5:2 R:2, O:2 5:2,",
        "R:3 R:3 R:2 5:3 5:3, O:2",
        "R:4 R:2, 5:2 O:4 5:2, R:2",
        "R:4 r:2 R:2, 5:2 O:2, R:4",
        "R:2 r:2 R:2 O:2, r:2 5:2 R:2 5:2,",
        "R:3 R:3 R:2 5:3 5:3, O:2",
        "R:3 R:3 R:2 5:3 O:3 5:2,",
    ];
    let pumps = [
        "R:2 R:2, O:2 R:2, R:2 R:2, O:2 R:2,",
        "R:2 O:2 R:2, O:2 R:2 O:2 R:2, O:2",
        "R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1,",
    ];
    for (i, figure_text) in figures.iter().enumerate() {
        let bar = i as u32 * 8;
        s.bass(roots, bar, 7, "R:12 5:4,", 38, 0.75);
        s.bass(roots, bar + 7, 1, "R:8 5:8,", 38, 0.75);
        // Into G minor the chromatic step would rub the melody's F.
        let into = if i == 1 || i == 5 {
            "R:4 r:2 R:2 5:4 5:4,"
        } else {
            "R:4 r:2 R:2 5:4 a:4"
        };
        for b in bar..bar + 8 {
            let text = if valley(b) {
                "R:8 5:8,"
            } else if b == bar + 7 {
                into
            } else {
                figure_text
            };
            s.bass(figure, b, 1, text, 38, 0.8);
            s.bass(edge, b, 1, text, 50, 0.8);
        }
        let pump_text = pumps[[0usize, 1, 2, 1, 0, 1, 2, 2][i]];
        s.bass(pump, bar, 7, pump_text, 38, 0.66);
        s.bass(
            pump,
            bar + 7,
            1,
            "R:2 R:2, O:2 R:2, R:2 5:2, O:2 a:2",
            38,
            0.66,
        );
    }
    s.pad(pad_ch, 0, 64, 3, 53, 69, 0.45);

    // The tide clock, resting in the two valleys (bars 25-28, 57-60).
    // Where the reed rests (sections 5 and 7 the first time through,
    // 1 and 6 the second) the clock steps forward.
    let motor_figures = [
        "0 1 2 1", "0 2 1 2", "0 1 2 3", "2 1 0 1", "0 1 2 1", "0 2 3 2", "0 1 2 3", "3 2 1 0",
    ];
    for (i, figure_text) in motor_figures.iter().enumerate() {
        let bar = i as u32 * 8;
        for b in (bar..bar + 8).filter(|&b| !valley(b)) {
            s.beat(hats, b, 1, "x.g.x.g.x.g.x.gg", 60, 0.34);
            let rests_first = i == 4 || i == 6;
            let rests_second = i == 0 || i == 5;
            if rests_first || rests_second {
                // Resting sections get a sixteenth-note clock an octave up.
                let busy =
                    |s: &mut Builder| s.arp(motor, b, 1, "0 2, 1 2, 3 2, 1 2,", 1, 74, 1, 0.55);
                let plain = |s: &mut Builder| s.arp(motor, b, 1, figure_text, 2, 62, 1, 0.42);
                s.pass(1);
                if rests_first {
                    busy(&mut s)
                } else {
                    plain(&mut s)
                }
                s.pass(2);
                if rests_second {
                    busy(&mut s)
                } else {
                    plain(&mut s)
                }
                s.pass(0);
            } else {
                s.arp(motor, b, 1, figure_text, 2, 62, 1, 0.42);
            }
        }
    }

    // Calm: the flute alone with the tide.
    // The calls stay under D6: the song saves its top for later.
    let calls = "| r:8 F5:2 G5:2, A5:4 | C6:6! A5:2 F5:8 | r:8 E5:2 F5:2, G5:4 | B5:6! A5:2 G5:8 \
                 | r:8 A5:2 Bb5:2, C6:4 | C6:4 A5:4 F5:8 | r:4 C6:4 Bb5:4 A5:4 | G5:8 E5:4 C5:4 |";
    s.line(
        flute_ch,
        0,
        "| A4:4 D5:4 E5:2, F5:6! | D5:3 B4:1, G4:4 A4:8~ | F5:4 D5:4 Bb4:8~ | C5:4 E5:4 G5:8 \
         | A4:4 D5:4 E5:2, F5:6 | B5:6! A5:2 G5:8 | F5:6 E5:2, D5:8 | C#5:8 E5:4 A4:4 |",
        0.7,
    );
    s.line(flute_ch, 8, calls, 0.6);
    let tide_line = "| D5:6 C5:2, Bb4:8 | E5:6 D5:2, C5:8 | F5:4 E5:4 D5:4 A4:4 | D5:16~ \
                     | D5:6 F5:2, D5:8 | E5:6 G5:2, E5:8 | C#5:8 E5:8 | G5:8! E5:4 C#5:4 |";
    s.line(flute_ch, 24, tide_line, 0.62);
    s.line(
        flute_ch,
        32,
        "| A4:3 C#5:1, D5:4 E5:2, F5:6! | D5:3 B4:1, G4:4 A4:8~ | F5:4 D5:4 Bb4:8 | C5:4 E5:4 G5:8 \
         | A4:4 D5:4 E5:2, F5:6 | B5:6! A5:2 G5:8 | F5:6 E5:2, D5:8 | B4:8 D5:4 G4:4 |",
        0.72,
    );
    let theme_b = "| A5:6 G5:2, F5:8 | G5:6 E5:2, C5:8 | D5:4 F5:4 A5:4 C6:4 | D6:8! C6:4 Bb5:4 \
                   | A5:6 F5:2, C5:8 | E5:4 G5:4 C6:8! | Bb5:4 A5:4 G5:4 F5:4 | D5:8 F5:8~ |";
    s.line(flute_ch, 40, theme_b, 0.68);
    s.line(
        flute_ch,
        56,
        "| D5:8 F5:8 | E5:8 G5:8 | F5:4 A5:4 D5:8 | D6:8 Bb5:8 |",
        0.62,
    );
    s.line(
        flute_ch,
        60,
        "| Bb5:6 A5:2, G5:8 | A5:16~ | D5:16 | r:16 |",
        0.6,
    );
    // Sections three and seven in long notes: theme C, then the climb to
    // the flute's one F6. The second time through, other long notes.
    s.pass(1);
    s.line(flute_ch, 16, "| D5:8 G5:8 | A5:8 F5:8 | Bb5:8 D6:8 | C6:8 A5:8 | G5:8 Bb5:8 | A5:8 F5:8 | G5:8 Eb5:8 | E5:8 C#5:8 |", 0.64);
    s.line(flute_ch, 48, "| G5:8 Bb5:8 | A5:8 D6:8 | D6:8 Bb5:8 | C6:8 A5:8 | Bb5:8 Eb6:8 | D6:8 F6:8! | D6:8 Bb5:8 | C#6:8 E6:8 |", 0.66);
    s.pass(2);
    s.line(flute_ch, 16, "| Bb5:8 A5:4 G5:4 | F5:8 A5:8 | D6:12 C6:4 | A5:16 | G5:8 Bb5:8 | C6:4 A5:4 F5:8 | Bb5:8 G5:4 Eb5:4 | E5:8 C#5:8 |", 0.64);
    s.line(flute_ch, 48, "| D6:8 Bb5:4 G5:4 | A5:4 F5:4 D6:8 | F5:8 Bb5:4 D6:4 | C6:12 A5:4 | G5:4 Bb5:4 Eb6:8 | D6:4 F6:12! | D6:8 G5:8 | A5:4 C#6:4 E6:8 |", 0.66);
    s.pass(0);

    // Working and gathering: the reed carries the themes and rests in
    // sections five and seven, where a bell answers with the motif.
    let reed_a = "| A4:4 D5:3 E5:1, F5:6! E5:2 | D5:3 B4:1, G4:4 A4:8~ | D5:4 F5:3 D5:1, Bb4:8 | G5:6! F5:2 E5:4 C5:4 \
                  | A4:4 D5:3 E5:1, F5:3 G5:1, A5:4! | G5:6 F5:2 E5:4 D5:4 | D5:4 F5:3 G5:1, F5:4 D5:4 | E5:8 C#5:4 A4:4 |";
    let theme_c = "| D5:4 G5:4 A5:2, Bb5:6! | A5:6 F5:2, D5:8 | F5:4 Bb5:4 C6:2, D6:6! | C6:6 A5:2, F5:8 \
                   | D5:4 G5:4 A5:2, Bb5:6 | A5:4 G5:2 F5:2, E5:4 D5:4 | G5:4 Bb5:4 G5:4 Eb5:4! | E5:6 C#5:2, A4:8 |";
    let ending = "| D5:4 F5:4 Bb5:8 | C6:6! Bb5:2 G5:8 | A5:4 F5:4 D5:8 | D5:4 F5:4 G5:4 A5:4 \
                  | Bb5:6 A5:2, G5:8 | A5:8! C#6:4 E6:4 | D6:12~ C6:2 A5:2, | E5:8 C#5:8 |";
    let reed_a2 = "| A4:3 C#5:1, D5:3 E5:1, F5:6! E5:2 | D5:3 B4:1, G4:4 A4:8~ | D5:4 F5:3 D5:1, Bb4:8 | G5:6! F5:2 E5:4 C5:4 \
                   | A4:4 D5:3 E5:1, F5:3 G5:1, A5:4! | G5:6 F5:2 E5:4 D5:4 | D5:4 F5:3 G5:1, F5:4 D5:4 | D5:8 B4:4 G4:4 |";
    let theme_c2 = "| D5:4 G5:4 A5:2, Bb5:6! | A5:6 F5:2, D5:8 | F5:4 Bb5:4 C6:2, D6:6! | C6:6 A5:2, F5:8 \
                    | G5:4 Bb5:3 C6:1, Bb5:4 G5:4 | F5:4 Bb5:3 D6:1, F6:8!~ | D6:6 C6:2, Bb5:4 G5:4 | A5:6 G5:2, E5:4 C#5:4 |";
    s.line(melody, 8, calls, 0.66);
    s.line(melody, 16, theme_c, 0.64);
    s.line(melody, 24, tide_line, 0.64);
    s.line(melody, 56, ending, 0.66);
    // Each rest is filled every two bars: glass, then a plucked echo.
    let answers =
        |s: &mut Builder, bar: u32, bell_a: &str, pluck_a: &str, bell_b: &str, pluck_b: &str| {
            s.line(answer, bar, bell_a, 0.6);
            s.line(canon, bar + 2, pluck_a, 0.75);
            s.line(
                canon,
                bar + 2,
                &format!("r:2 {}", pluck_a.trim_end_matches(" r:8")),
                0.4,
            );
            s.line(answer, bar + 4, bell_b, 0.6);
            s.line(canon, bar + 6, pluck_b, 0.75);
            s.line(
                canon,
                bar + 6,
                &format!("r:2 {}", pluck_b.trim_end_matches(" r:8")),
                0.4,
            );
        };
    // First time through: the reed sings sections 1 and 6 and rests in 5
    // and 7.
    s.pass(1);
    s.line(melody, 0, reed_a, 0.66);
    s.line(melody, 40, theme_b, 0.64);
    answers(
        &mut s,
        32,
        "A5:3 D6:3 E6:2, F6:8",
        "D6:2 F6:2 Bb6:4 r:8",
        "F6:3 E6:3 D6:2, A5:8",
        "F5:2 Bb5:2 D6:4 r:8",
    );
    answers(
        &mut s,
        48,
        "D5:3 G5:3 A5:2, Bb5:8",
        "D6:2 F6:2 Bb6:4 r:8",
        "Eb5:3 G5:3 Bb5:2, G5:8",
        "Bb5:2 D6:2 G6:4 r:8",
    );
    // Second time: it sings 5 and 7 and rests in 1 and 6.
    s.pass(2);
    s.line(melody, 32, reed_a2, 0.66);
    s.line(melody, 48, theme_c2, 0.64);
    answers(
        &mut s,
        0,
        "A5:3 D6:3 E6:2, F6:8",
        "D6:2 F6:2 Bb6:4 r:8",
        "F6:3 E6:3 D6:2, A5:8",
        "F5:2 Bb5:2 D6:4 r:8",
    );
    answers(
        &mut s,
        40,
        "A5:3 C6:3 F6:2, C6:8",
        "D6:2 F6:2 A6:4 r:8",
        "C6:3 A5:3 F5:2, A5:8",
        "D6:2 G6:2 Bb6:4 r:8",
    );
    s.pass(0);
    // Section 7 is the song's climb: while the harbour works, a pumping
    // bass in eighths and sixteenth hats join it.
    let climb = s.channel(
        "climb pump",
        drive_bass(),
        Layer::Unfought,
        0.0,
        0.8,
        0.0,
        0.04,
    );
    s.role(climb, Role::Pulse);
    s.bass(climb, 48, 8, "R:2 O:2, R:2 O:2, R:2 O:2, R:2 O:2,", 38, 0.7);
    let climb_hats = s.channel(
        "climb hats",
        drum(Osc::HatClosed, 0.5, 0.04),
        Layer::Unfought,
        -0.5,
        1.0,
        0.0,
        0.08,
    );
    s.role(climb_hats, Role::Kit);
    s.beat(climb_hats, 48, 8, "xgxgXgxgxgxgXgxg", 60, 0.6);
    // Section 7 always has the working counter-line and the pad an octave
    // up; the rests have them too.
    s.bass(working_counter, 48, 8, "3:6 5:2, 3:4 5:4,", 62, 0.8);
    s.pad(pad_high, 48, 8, 3, 65, 77, 0.5);
    for (pass, bars) in [(1u8, &[32u32][..]), (2, &[0, 40])] {
        s.pass(pass);
        for &bar in bars.iter() {
            s.bass(working_counter, bar, 8, "3:6 5:2, 3:4 5:4,", 55, 0.6);
            s.pad(pad_high, bar, 8, 3, 65, 77, 0.5);
        }
        s.pass(0);
    }

    // Fighting: the lead in a sharper, 3-3-2 rhythm with chip chords on
    // its held notes; the calls an octave up; the reed below it.
    let battle_a = "| A4:3 D5:3 E5:2, F5:3! E5:3 D5:2, | D5:3 B4:3 G4:2, A4:6~ G4:1, A4:1 | D5:3 F5:3 Bb5:2! A5:3 F5:3 D5:2, \
                    | E5:3 G5:3 C6:2! Bb5:3 G5:3 E5:2, | A4:3 D5:3 E5:2, F5:3 G5:3 A5:2! | B5:6! A5:2 G5:3 F5:3 D5:2, \
                    | D5:3 F5:3 G5:2, F5:3 D5:3 Bb4:2 | C#5:3 E5:3 A5:2! G5:4 E5:4 |";
    s.line(lead, 0, battle_a, 0.74);
    s.line(lead, 8, calls, 0.74);
    s.line(
        lead,
        16,
        "| D5:4 G5:4 A5:2, Bb5:6! | A5:6 F5:2, D5:8{3,7} | F5:4 Bb5:4 C6:2, D6:6! | C6:6 A5:2, F5:8{4,7} \
         | D5:4 G5:4 A5:2, Bb5:6 | A5:4 G5:2 F5:2, E5:4 D5:4 | G5:4 Bb5:4 G5:4 Eb5:4! | E5:6 C#5:2, A4:8{4,7} |",
        0.76,
    );
    s.line(
        lead,
        24,
        "| F5:3 D5:3 Bb4:2, D5:8 | G5:3 E5:3 C5:2, E5:8 | A5:4 G5:4 F5:4 E5:4 | D5:16~ \
         | F5:6 D5:2, Bb4:8{4,7} | G5:6 E5:2, C5:8{4,7} | A5:8! G5:4 E5:4 | C#5:8 E5:4 G5:4 |",
        0.74,
    );
    s.line(
        lead,
        32,
        "| A4:3 D5:3 E5:2, F5:3! E5:3 D5:2, | D5:3 B4:3 G4:2, A4:6~ G4:1, A4:1 | D5:3 F5:3 Bb5:2! A5:3 F5:3 D5:2, \
         | E5:3 G5:3 C6:2! Bb5:3 G5:3 E5:2, | A4:3 D5:3 E5:2, F5:3 G5:3 A5:2! | B5:6! A5:2 G5:3 F5:3 D5:2, \
         | D5:3 F5:3 G5:2, F5:3 D5:3 Bb4:2 | B4:3 D5:3 G5:2! D5:4 B4:4 |",
        0.76,
    );
    s.line(
        lead,
        40,
        "| A5:6 G5:2, F5:8{4,7} | G5:6 E5:2, C5:8{4,7} | D5:4 F5:4 A5:4 C6:4 | D6:8! C6:4 Bb5:4 \
         | A5:6 F5:2, C5:8 | E5:4 G5:4 C6:8! | Bb5:4 A5:4 G5:4 F5:4 | D5:8 F5:8~ |",
        0.74,
    );
    s.line(
        lead,
        48,
        "| C#5:1, D5:3 G5:4 A5:2, Bb5:6! | A5:6 F5:2, D5:8{3,7} | F5:4 Bb5:4 C6:2, D6:6! | C6:6 A5:2, F5:8{4,7} \
         | G5:4 Bb5:3 C6:1, Bb5:4 G5:4 | F5:4 Bb5:3 C6:1, D6:8!~ | D6:6 C6:2, Bb5:4 G5:4 | A5:6 G5:2, E5:4 C#5:4 |",
        0.8,
    );
    s.line(
        lead,
        56,
        "| D5:4 F5:4 Bb5:8{4,7} | C6:6 Bb5:2, G5:8 | A5:4 F5:4 D5:8{3,7} | D5:4 F5:4 G5:4 A5:4 \
         | Bb5:6 A5:2, G5:8 | A5:8! C#6:4 E6:4 | D6:12~ C6:2 A5:2, | E5:8 C#5:8 |",
        0.78,
    );
    let counter_a = "| D4:8 F4:4 A4:4 | B3:8 D4:4 G4:4 | Bb3:6 D4:2, F4:8 | C4:6 E4:2, G4:8 \
                     | A4:8 F4:4 D4:4 | G4:8 B4:4 G4:4 | F4:6 D4:2, Bb3:8 | A3:6 C#4:2, E4:8 |";
    let counter_c = "| G4:6 A4:2, Bb4:8 | A4:6 G4:2, F4:8 | F4:4 D4:4 F4:4 Bb4:4 | A4:6 G4:2, C5:8 \
                     | Bb4:6 A4:2, G4:8 | F4:4 E4:4, D4:4 A4:4 | G4:6 F4:2, Eb4:8 | E4:4 A4:4 C#5:8 |";
    s.line(counter, 0, counter_a, 0.6);
    s.line(counter, 8, "| D4:8 F4:4 A4:4 | C4:8 F4:4 A4:4 | C4:8 E4:4 G4:4 | B3:8 D4:4 G4:4 | D4:8 F4:4 A4:4 | C4:8 F4:4 A4:4 | D4:8 F4:4 Bb4:4 | C4:8 E4:4 G4:4 |", 0.6);
    s.line(counter, 16, counter_c, 0.58);
    s.line(counter, 24, "| Bb3:8 D4:8 | C4:8 E4:8 | D4:8 F4:8 | A3:8 D4:8 | D4:8 F4:8 | E4:8 G4:8 | C#4:8 E4:8 | E4:8 C#4:4 A3:4 |", 0.6);
    s.line(counter, 32, "| D4:8 F4:4 A4:4 | B3:8 D4:4 G4:4 | Bb3:6 D4:2, F4:8 | C4:6 E4:2, G4:8 | A4:8 F4:4 D4:4 | G4:8 B4:4 G4:4 | F4:6 D4:2, Bb3:8 | G3:6 B3:2, D4:8 |", 0.6);
    s.line(counter, 40, "| C4:8 F4:8 | C4:8 E4:8 | F4:8 A4:8 | F4:8 D4:8 | C4:8 A3:8 | E4:8 G4:8 | D4:8 G4:8 | F4:8 Bb3:8 |", 0.58);
    s.line(counter, 48, "| G4:6 A4:2, Bb4:8 | A4:6 G4:2, F4:8 | F4:4 D4:4 F4:4 Bb4:4 | A4:6 G4:2, C5:8 | Eb4:8 G4:8 | D4:6 Eb4:2, F4:8 | G4:6 F4:2, D4:8 | E4:8 C#4:8 |", 0.58);
    s.line(counter, 56, "| F4:8 D4:8 | E4:8 G4:8 | F4:8 A4:8 | F4:8 D4:8 | G4:8 Bb4:8 | C#5:8 E5:8 | A4:12 G4:2 F4:2 | E4:8 C#4:8 |", 0.58);

    // The kick runs whenever the harbour works; the snare on three only
    // until the fight, when the backbeat moves to two and four.
    let kit_bars: [(u32, u32, &str); 10] = [
        (0, 7, "x.......1......."),
        (7, 1, "x.......1...1.11"),
        (8, 8, "x.....x.1.....x."),
        (16, 8, "x..x..x.1..x..1."),
        (24, 8, "x.......1.......x.....x.1.....2."),
        (32, 8, "x.....x.1.....x."),
        (40, 8, "x.....x.1...x..."),
        // The climb: a backbeat on two and four joins the snare on three.
        (48, 8, "x..x1.x.1..x1.1."),
        (56, 7, "x..x..x.1..x..1."),
        (63, 1, "x..x..x.1.1.1111"),
    ];
    for (bar, bars, pattern) in kit_bars {
        let (kick, rest) = split_kit(pattern);
        for b in bar..bar + bars {
            let k = if valley(b) {
                "x.......x......."
            } else {
                &kick[(((b - bar) as usize * 16) % kick.len())..][..16]
            };
            s.beat(kit, b, 1, k, 38, 0.85);
            if !valley(b) {
                s.beat(
                    snare3,
                    b,
                    1,
                    &rest[(((b - bar) as usize * 16) % rest.len())..][..16],
                    38,
                    0.8,
                );
            }
        }
    }
    for bar in [0, 8, 32, 40] {
        s.beat(clank, bar, 8, "......x.......x.", midi("D6"), 0.5);
    }
    for bar in [16, 48, 56] {
        s.beat(clank, bar, 8, "..x...x...x.g.x.", midi("D6"), 0.5);
    }

    // Threat: the pump (above), toms into each section, brass stabs.
    for section in 0..8u32 {
        let end = section * 8 + 7;
        s.line(toms, end, "| r:8 D3:1! D3:1, A2:2 F2:2 D2:2 |", 0.62);
        if section > 0 {
            s.line(toms, section * 8, "D4:16@1", 0.6);
        }
    }
    for section in 0..7u32 {
        s.stab(brass_ch, section * 8, 8, &[2, 10], 2, 3, 57, 74, 0.6);
    }
    s.stab(brass_ch, 56, 8, &[2, 6, 10, 14], 2, 3, 57, 74, 0.55);

    // Fighting: arpeggios, open hats on the off-beats, the snare on two
    // and four, fills.
    for bar in [0u32, 16, 32, 48, 56] {
        s.arp(arp, bar, 8, "0! 1 2, 3 4, 3 2, 1,", 1, 62, 1, 0.42);
    }
    s.beat(battle_kit, 0, 64, "..1.x.1...1.x.1.", 60, 0.5);
    for section in 0..8u32 {
        s.beat(battle_kit, section * 8 + 3, 1, "............x.xx", 60, 0.6);
        s.beat(battle_kit, section * 8 + 7, 1, "........x.xxXxXX", 60, 0.7);
    }

    match_layers(&mut s, 50, 64);
    s.finish()
}

// ---------------------------------------------------------------------
// Results.

fn victory() -> Song {
    let mut s = Builder::new(SongId::Victory, "Victory", 104.0, 4, 4, 4);
    s.song.loop_bar = None;
    s.song.then = Some(SongId::Harbour);
    s.song.echo_rows = 4.0;
    s.song.echo_feedback = 0.4;
    s.chords(0, "Bb C D D");
    let lead = s.channel("lead", lead_union(), Layer::Core, 0.0, 1.0, 0.3, 0.35);
    s.variants(lead, [lead_union(), lead_assembly(), lead_compact()]);
    let harmony = s.channel(
        "counter",
        Instrument::new(Osc::Pulse {
            duty: 0.5,
            sweep: 0.0,
        })
        .adsr(0.006, 0.3, 0.6, 0.2)
        .gain(0.2)
        .filter(2_400.0, 0.8, 1.0),
        Layer::Core,
        0.35,
        1.0,
        0.2,
        0.3,
    );
    let bass_ch = s.channel("bass", bass(), Layer::Core, 0.0, 1.0, 0.0, 0.05);
    let arp = s.channel("arp", pluck(), Layer::Core, -0.5, 1.0, 0.4, 0.3);
    let pad_ch = s.channel(
        "pad",
        pad().adsr(0.08, 1.0, 0.8, 1.6),
        Layer::Core,
        0.0,
        1.0,
        0.0,
        0.5,
    );
    let kit = s.channel(
        "kit",
        drum(Osc::Kick, 0.7, 0.4),
        Layer::Core,
        0.0,
        1.0,
        0.0,
        0.1,
    );
    s.extra(kit, drum(Osc::Snare, 0.45, 0.3));
    s.extra(kit, drum(Osc::Crash, 0.28, 2.0));
    s.poly(kit);
    s.line(
        lead,
        0,
        "| D5:3 D5:1, F5:4 Bb5:8! | E5:3 E5:1, G5:4 C6:8! | A4:3 D5:2 E5:1, F#5:6 A5:4 | D6:16!~ |",
        0.8,
    );
    s.line(
        harmony,
        0,
        "| Bb4:3 Bb4:1, D5:4 F5:8 | C5:3 C5:1, E5:4 G5:8 | F#4:3 A4:2 A4:1, D5:6 F#5:4 | A5:16 |",
        0.7,
    );
    s.line(
        bass_ch,
        0,
        "| Bb2:3 Bb2:1, Bb2:4 F2:8 | C3:3 C3:1, C3:4 G2:8 | D3:4 D3:4 A2:4 D3:4 | D2:16 |",
        0.8,
    );
    s.arp(arp, 3, 1, "0 1 2 3 4 5 6 5 4 3 2 1 2 3 4 5", 1, 62, 1, 0.5);
    s.pad(pad_ch, 0, 4, 3, 57, 74, 0.5);
    s.line(kit, 0, "| D2:3 D2:1, D4:4@1 D2:4 D4:4@1 | D2:3 D2:1, D4:4@1 D2:4 D4:1@1 D4:1@1, D4:1@1 D4:1@1! | D2:16@2 | D2:16@2 |", 0.8);
    s.finish()
}

fn defeat() -> Song {
    let mut s = Builder::new(SongId::Defeat, "Defeat", 68.0, 4, 4, 4);
    s.song.loop_bar = None;
    s.song.then = Some(SongId::Harbour);
    s.song.echo_feedback = 0.42;
    s.chords(0, "Dm Bb Eb Dm");
    let lead = s.channel("lead flute", flute(), Layer::Core, 0.0, 1.0, 0.35, 0.45);
    let pad_ch = s.channel("pad", pad(), Layer::Core, 0.0, 1.0, 0.0, 0.5);
    let bass_ch = s.channel("bass", bass(), Layer::Core, 0.0, 1.0, 0.0, 0.1);
    let bell_ch = s.channel("bell", bell(), Layer::Core, 0.3, 1.0, 0.3, 0.6);
    s.line(
        lead,
        0,
        "| A4:4 D5:3 E5:1, F5:8~ | F5:4 D5:4 C5:4 Bb4:4 | G4:4 Bb4:4 Eb5:8!~ | E4:4, F4:4 D4:8~ |",
        0.66,
    );
    s.pad(pad_ch, 0, 4, 3, 50, 65, 0.5);
    s.line(bass_ch, 0, "| D3:16 | Bb2:16 | Eb2:16 | D2:16 |", 0.7);
    s.line(bell_ch, 3, "| r:8 D4:8 |", 0.6);
    s.finish()
}

fn draw() -> Song {
    let mut s = Builder::new(SongId::Draw, "Draw", 72.0, 4, 4, 3);
    s.song.loop_bar = None;
    s.song.then = Some(SongId::Harbour);
    s.chords(0, "Dsus4 Dsus2 Dsus2");
    let lead = s.channel("lead flute", flute(), Layer::Core, 0.0, 1.0, 0.35, 0.45);
    let pad_ch = s.channel("pad", pad(), Layer::Core, 0.0, 1.0, 0.0, 0.5);
    let bass_ch = s.channel("bass", bass(), Layer::Core, 0.0, 1.0, 0.0, 0.1);
    s.line(
        lead,
        0,
        "| A4:4 D5:3 E5:1, G5:8~ | E5:8 A4:8 | E5:16~ |",
        0.62,
    );
    s.pad(pad_ch, 0, 3, 3, 50, 67, 0.5);
    s.line(bass_ch, 0, "| D3:16 | D3:16 | D2:16 |", 0.65);
    s.finish()
}

/// Under the result: the harbour after, glass and the sea.
fn harbour() -> Song {
    let mut s = Builder::new(SongId::Harbour, "Low Tide", 60.0, 3, 4, 8);
    s.song.echo_feedback = 0.4;
    s.song.gain = 1.6;
    s.chords(0, "Dm Bb F C Dm Bb Gm A");
    let pad_ch = s.channel("pad", pad(), Layer::Core, 0.0, 1.0, 0.0, 0.5);
    let bell_ch = s.channel("bell", bell(), Layer::Core, 0.3, 0.9, 0.4, 0.6);
    let bass_ch = s.channel("bass", bass(), Layer::Core, 0.0, 0.8, 0.0, 0.1);
    let edge = s.channel(
        "bass edge",
        Instrument::new(Osc::Pulse {
            duty: 0.25,
            sweep: 0.0,
        })
        .adsr(0.01, 0.6, 0.4, 0.3)
        .gain(0.14)
        .filter(1_400.0, 0.8, 0.5),
        Layer::Core,
        0.1,
        1.0,
        0.0,
        0.1,
    );
    let surf_ch = s.channel("surf", surf(), Layer::Core, 0.0, 1.0, 0.0, 0.5);
    s.pad(pad_ch, 0, 8, 3, 53, 69, 0.45);
    s.bass(edge, 0, 8, "R:12", 50, 0.6);
    s.pass(1);
    s.line(bell_ch, 0, "| A5:3 D6:2 E6:1, F6:6 | r:12 | F5:3 A5:3 C6:6 | r:12 | D5:3 F5:3 A5:6 | r:12 | G5:3 Bb5:3 D6:6 | E6:6 C#6:6 |", 0.5);
    s.pass(2);
    s.line(bell_ch, 0, "| r:12 | F5:3 D5:3 A4:6 | r:12 | E5:3 G5:3 C6:6 | r:12 | Bb5:3 A5:3 F5:6 | r:12 | C#6:6 A5:6 |", 0.45);
    s.pass(0);
    s.bass(bass_ch, 0, 8, "R:12", 38, 0.6);
    for bar in [0, 4] {
        s.note(surf_ch, bar * 12, 18, midi("D3"), 0.8);
    }
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_song_builds_and_stays_in_its_bars() {
        let songs = songs();
        for song in &songs {
            let total = song.total_rows();
            for c in &song.channels {
                for n in &c.notes {
                    assert!(n.row < total, "{} {} note past the end", song.name, c.name);
                    assert!(
                        n.key >= 24 && n.key <= 100,
                        "{} {} key {}",
                        song.name,
                        c.name,
                        n.key
                    );
                }
            }
        }
        assert!(songs.iter().any(|s| s.id == SongId::Match));
    }

    fn match_songs() -> Vec<Song> {
        songs()
            .into_iter()
            .filter(|s| s.partner.is_some())
            .collect()
    }

    /// A bar of a tune as (row in bar, length, interval from its first
    /// note) for bars of four notes or more that lie inside the bar (three
    /// long notes down a triad are common property).
    /// A bar's shape: (row in bar, length, interval from its first note).
    type Shape = Vec<(u32, u32, i16)>;

    fn tune_bars(song: &Song, names: &[&str]) -> Vec<(String, u32, Shape)> {
        let rpb = song.rows_per_bar();
        let mut out = Vec::new();
        for c in song.channels.iter().filter(|c| names.contains(&c.name)) {
            for bar in 0..song.bars {
                let notes: Vec<_> = c
                    .notes
                    .iter()
                    .filter(|n| {
                        n.row / rpb == bar && n.row + n.len <= (bar + 1) * rpb && n.pass != 3
                    })
                    .collect();
                if notes.len() < 4 {
                    continue;
                }
                // Each pass is its own bar.
                for pass in [1u8, 2] {
                    let shape: Vec<(u32, u32, i16)> = notes
                        .iter()
                        .filter(|n| n.pass == 0 || n.pass == pass)
                        .map(|n| {
                            (
                                n.row % rpb,
                                n.len,
                                i16::from(n.key) - i16::from(notes[0].key),
                            )
                        })
                        .collect();
                    if shape.len() >= 4 {
                        out.push((c.name.to_string(), bar + 1, shape));
                    }
                }
            }
        }
        out
    }

    /// No bar of one match song's reed or lead is another's, at any
    /// transposition.
    #[test]
    fn no_match_song_borrows_a_bar_of_tune() {
        let songs = match_songs();
        let tunes = ["lead reed", "lead"];
        for (i, a) in songs.iter().enumerate() {
            for b in &songs[i + 1..] {
                let theirs = tune_bars(b, &tunes);
                for (name, bar, shape) in tune_bars(a, &tunes) {
                    if let Some((other, other_bar, _)) = theirs.iter().find(|(_, _, s)| *s == shape)
                    {
                        panic!(
                            "{} {name} bar {bar} is {} {other} bar {other_bar}",
                            a.name, b.name
                        );
                    }
                }
            }
        }
    }

    /// The calm flute differs between passes in at least 16 bars of each
    /// match song.
    #[test]
    fn the_calm_flute_changes_the_second_time_through() {
        for song in match_songs() {
            let rpb = song.rows_per_bar();
            let flute = song
                .channels
                .iter()
                .find(|c| c.name == "flute")
                .expect("flute");
            let mut bars: Vec<u32> = flute
                .notes
                .iter()
                .filter(|n| n.pass != 0)
                .map(|n| n.row / rpb)
                .collect();
            bars.dedup();
            assert!(
                bars.len() >= 16,
                "{}: {} bars differ",
                song.name,
                bars.len()
            );
        }
    }

    /// Each song fights with its own battle kit, and Undertow's keeps the
    /// half-time: no snare on two or four.
    #[test]
    fn each_song_fights_with_its_own_kit() {
        let songs = match_songs();
        // Rows, instruments and intervals from the first note.
        let first_bar = |song: &Song, name: &str| -> Vec<(u32, u8, i16)> {
            let c = song.channels.iter().find(|c| c.name == name).expect(name);
            let notes: Vec<_> = c
                .notes
                .iter()
                .filter(|n| n.row < song.rows_per_bar())
                .collect();
            notes
                .iter()
                .map(|n| (n.row, n.inst, i16::from(n.key) - i16::from(notes[0].key)))
                .collect()
        };
        for (i, a) in songs.iter().enumerate() {
            for b in &songs[i + 1..] {
                for name in ["battle kit", "arp"] {
                    assert_ne!(
                        first_bar(a, name),
                        first_bar(b, name),
                        "{} and {} share the {name}",
                        a.name,
                        b.name
                    );
                }
            }
        }
        let undertow = songs
            .iter()
            .find(|s| s.id == SongId::Undertow)
            .expect("Undertow");
        let kit = undertow
            .channels
            .iter()
            .find(|c| c.name == "battle kit")
            .expect("kit");
        let rpb = undertow.rows_per_bar();
        assert!(
            kit.notes
                .iter()
                .all(|n| n.inst != 0 || !matches!(n.row % rpb, 4 | 12)),
            "a snare on two or four"
        );
    }

    /// Each match song hands over to the next in the rotation, and the
    /// last bar it hands over from is its last bar, so the second time
    /// through ends in the partner's key.
    #[test]
    fn match_songs_rotate_through_all_three() {
        let songs = songs();
        let mut id = SongId::Match;
        for _ in 0..3 {
            let song = songs.iter().find(|s| s.id == id).expect("match song");
            assert_eq!(song.partner, Some(id.rotation_next()), "{}", song.name);
            assert_eq!(
                song.handovers.last().copied(),
                Some(song.bars - 1),
                "{}",
                song.name
            );
            id = id.rotation_next();
        }
        assert_eq!(id, SongId::Match);
    }
}
