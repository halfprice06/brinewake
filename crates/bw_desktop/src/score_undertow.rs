//! UNDERTOW — the third match song. G Dorian, a fourth above the record's
//! home, at 100 BPM on a heavy half-time: the tide pulling back. The motif
//! is here as D–G–A–B♭ (a rise to the crest), E natural is the Dorian
//! colour and E♭ (the flat sixth) the pull under it. A dub echo on the
//! dotted eighth. Its form is its own (six eight-bar sections: A, B, A',
//! a bright C that turns toward F and C, B', and a climbing D whose G6 is
//! the song's one top note), and it ends on D minor, so it hands straight
//! back to Reclamation. The same layers as the other match songs.

use crate::music::{Instrument, Layer, Osc, Role, Song, SongId};
use crate::score::{Builder, midi};
use crate::score_songs::{
    bass, bell, brass, drive_bass, drum, flute, lead_assembly, lead_compact, lead_union, mallet,
    match_layers, pad, pluck, reed, split_kit, work_clank,
};

pub fn undertow() -> Song {
    let mut s = Builder::new(SongId::Undertow, "Undertow", 100.0, 4, 4, 48);
    s.song.partner = Some(SongId::Match);
    s.song.key = 5;
    s.song.swing = 0.06;
    s.song.echo_rows = 6.0;
    s.song.echo_feedback = 0.38;
    s.chords(0, "Gm C/G Gm F Eb F Gm D");
    s.chords(8, "Bb F C Gm Bb F Eb D");
    s.chords(16, "Gm C/G Gm F Eb Bb Cm D7");
    s.chords(24, "C Bb F C Am Dm Bb C");
    s.chords(32, "Bb F C Gm Bb F Eb D");
    s.chords(40, "Gm F Eb D Gm F Bb/D Dm");
    s.song.sections = vec![0, 8, 16, 24, 32, 40];
    // C (the flat seventh of D) and D minor lead into Reclamation.
    s.song.handovers = vec![31, 47];
    let valley = |bar: u32| (20..24).contains(&bar) || (36..40).contains(&bar);
    let section_ends = [7u32, 15, 23, 31, 39, 47];

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
    let pad_ch = s.channel("pad", pad(), Layer::Pad, 0.0, 0.8, 0.0, 0.45);
    let motor = s.channel("motor", mallet(), Layer::Core, 0.45, 0.8, 0.35, 0.3);
    s.role(motor, Role::Pulse);
    let hats = s.channel(
        "hats",
        drum(Osc::HatClosed, 0.38, 0.05),
        Layer::Core,
        -0.45,
        1.0,
        0.0,
        0.1,
    );
    s.role(hats, Role::Kit);
    let flute_ch = s.channel("flute", flute(), Layer::Calm, -0.2, 0.95, 0.3, 0.4);
    s.role(flute_ch, Role::Melody);
    let kit = s.channel(
        "kit",
        drum(Osc::Kick, 0.5, 0.4),
        Layer::Drive,
        0.0,
        1.0,
        0.0,
        0.1,
    );
    s.role(kit, Role::Kit);
    let snare3 = s.channel(
        "snare on three",
        drum(Osc::Snare, 0.45, 0.35),
        Layer::Unfought,
        0.05,
        1.0,
        0.25,
        0.2,
    );
    s.extra(snare3, drum(Osc::Clap, 0.3, 0.2));
    s.poly(snare3);
    s.role(snare3, Role::Kit);
    let melody = s.channel("lead reed", reed(), Layer::Unfought, 0.15, 1.0, 0.25, 0.3);
    s.role(melody, Role::Melody);
    let answer = s.channel(
        "bell answers",
        bell(),
        Layer::Unfought,
        -0.35,
        0.8,
        0.4,
        0.5,
    );
    let canon = s.channel(
        "pluck answers",
        pluck(),
        Layer::Unfought,
        0.45,
        1.0,
        0.35,
        0.3,
    );
    s.poly(canon);
    s.role(answer, Role::Melody);
    s.role(canon, Role::Melody);
    let counter = s.channel("counter reed", reed(), Layer::Battle, 0.4, 0.9, 0.15, 0.28);
    s.role(counter, Role::Melody);
    let clank = s.channel(
        "clank",
        drum(Osc::Anvil, 0.16, 0.5),
        Layer::Drive,
        0.65,
        1.0,
        0.35,
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
    let brass_ch = s.channel("brass", brass(), Layer::Tension, -0.35, 1.0, 0.2, 0.25);
    s.role(brass_ch, Role::Pulse);
    let lead = s.channel("lead", lead_union(), Layer::Battle, 0.0, 1.0, 0.3, 0.3);
    s.variants(lead, [lead_union(), lead_assembly(), lead_compact()]);
    s.role(lead, Role::Melody);
    let arp = s.channel("arp", pluck(), Layer::Battle, -0.65, 0.9, 0.3, 0.2);
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
    s.extra(battle_kit, drum(Osc::Tom, 0.32, 0.3));
    s.poly(battle_kit);
    s.role(battle_kit, Role::Kit);

    // Bass: whole-note roots when calm; a heavy half-time figure while
    // working, doubled above for small speakers; an eighth-note pump under
    // threat. G2 is the floor.
    for bar in 0..48u32 {
        let end = section_ends.contains(&bar);
        s.bass(
            roots,
            bar,
            1,
            if end { "R:8 5:8," } else { "R:12 5:4," },
            43,
            0.75,
        );
        let text = if valley(bar) {
            "R:8 5:8,"
        } else if bar == 31 || bar == 47 {
            // The hand-over bars: root and fifth only.
            "R:6 R:2, 5:8"
        } else if end {
            "R:6 R:2, 5:4 a:4"
        } else if (24..32).contains(&bar) {
            "R:3 R:3, O:2 R:4 5:4,"
        } else {
            "R:6 R:2, O:4 5:4,"
        };
        s.bass(figure, bar, 1, text, 43, 0.8);
        s.bass(edge, bar, 1, text, 55, 0.8);
        s.bass(
            pump,
            bar,
            1,
            "R:2 R:2, O:2 R:2, R:2 R:2, O:2 R:2,",
            43,
            0.62,
        );
    }
    s.pad(pad_ch, 0, 48, 3, 55, 71, 0.45);
    // The clock rests in the valleys; where the reed rests (B' the first
    // time through, A the second) it steps forward to sixteenths an octave
    // up, and keeps going softly through B's valley.
    for bar in 0..48u32 {
        let figure_text = if (24..32).contains(&bar) {
            "0 2 1 3"
        } else {
            "0 2 1 2"
        };
        let plain = |s: &mut Builder| s.arp(motor, bar, 1, figure_text, 2, 67, 1, 0.42);
        let busy =
            |s: &mut Builder, vel: f32| s.arp(motor, bar, 1, "0 2, 1 2, 3 2, 1 2,", 1, 79, 1, vel);
        if !valley(bar) {
            s.beat(hats, bar, 1, "x.g.x.ggx.g.x.gg", 60, 0.34);
        }
        let rest_pass = if (32..40).contains(&bar) {
            Some(1)
        } else if bar < 8 {
            Some(2)
        } else {
            None
        };
        match rest_pass {
            Some(pass) => {
                s.pass(pass);
                busy(&mut s, if valley(bar) { 0.45 } else { 0.55 });
                if !valley(bar) {
                    s.pass(3 - pass);
                    plain(&mut s);
                }
                s.pass(0);
            }
            None if !valley(bar) => plain(&mut s),
            None => {}
        }
    }

    // The themes.
    // The head leaves each downbeat to the kick and the echo: a beat's
    // rest, a pickup, then long-short.
    let theme_a = "| r:4 D5:2 G5:2 A5:6 Bb5:2 | Bb5:6! A5:2 G5:2 E5:6 | r:4 D5:2 G5:2 A5:6 C6:2 | C6:6! Bb5:2 A5:8 \
                   | r:4 G5:2 Bb5:2 Eb6:6! D6:2 | C6:6 A5:2 F5:8 | r:4 G5:2 A5:2 Bb5:4 A5:2 G5:2 | F#5:12 r:4 |";
    let theme_b = "| F5:12 D5:4 | C5:8 F5:4 A5:4 | G5:12 E5:4 | D5:16~ | F5:8 Bb5:8 | A5:12 C6:4 | Bb5:8 G5:4 Eb5:4 | F#5:8 D5:8 |";
    let theme_a2 = "| r:4 D5:2 G5:2 A5:6 Bb5:2 | Bb5:6! A5:2 G5:2 E5:6 | r:4 D5:2 G5:2 A5:6 C6:2 | C6:6 D6:2 C6:8 \
                    | r:4 Bb5:2 C6:2 Eb6:6! D6:2 | D6:6 C6:2 Bb5:8 | r:4 Eb5:2 G5:2 C6:6 Bb5:2 | A5:6 F#5:2 C6:8 |";
    let theme_c = "| r:4 E5:2 G5:2 C6:6 E6:2 | D6:8 Bb5:4 F5:4 | C6:6 A5:2 F5:4 A5:4 | E6:8! D6:4 C6:4 \
                   | r:4 E5:2 A5:2 C6:6 A5:2 | F5:4 A5:4 D6:8 | Bb5:4 D6:4 C6:4 Bb5:4 | C6:6 G5:2 E5:8 |";
    let theme_b2 = "| F5:10 G5:1, F5:1 D5:4 | C5:8 F5:3 G5:1, A5:4 | G5:10 A5:1, G5:1 E5:4 | D5:16~ \
                    | F5:6 A5:1, Bb5:1 Bb5:8 | A5:12 C6:4~ | Bb5:8 G5:4 Eb5:4 | F#5:8 D5:8 |";
    let theme_d = "| G5:4 Bb5:4 D6:8! | C6:4 A5:4 F5:8 | Eb6:6! D6:2, Bb5:8 | A5:4 F#5:4 D6:8 \
                   | G6:8! F6:4 D6:4 | C6:8 A5:8 | Bb5:6 A5:2, F5:8 | D5:16~ |";

    // Calm: the flute through the whole song in long notes, the first
    // theme with the head's rest and pickup, other notes the second time
    // through, and the climb to G6 at bar 45.
    s.pass(1);
    s.line(flute_ch, 0, "| r:4 D5:4 G5:8 | G5:8 E5:8 | r:4 D5:4 Bb5:8 | C6:12 A5:4 | r:4 G5:4 Bb5:8 | A5:8 F5:8 | G5:8 Bb5:8 | A5:8 F#5:8 |", 0.68);
    s.line(flute_ch, 16, "| r:4 D5:4 G5:8 | G5:8 E5:8 | r:4 D5:4 Bb5:8 | C6:8 A5:8 | r:4 G5:4 Eb6:8 | D6:8 Bb5:8 | C6:8 G5:8 | F#5:8 C6:8 |", 0.66);
    s.pass(2);
    s.line(flute_ch, 0, "| r:4 G5:4 D6:8 | C6:8 G5:8 | r:4 Bb5:4 D6:8 | C6:8 F5:8 | r:4 Eb5:4 G5:8 | F5:8 C6:8 | Bb5:8 G5:8 | F#5:8 D5:8 |", 0.68);
    s.line(flute_ch, 16, "| Bb5:8 A5:4 G5:4 | E5:8 C6:8 | D6:8 Bb5:8 | A5:8 C6:8 | Bb5:8 G5:8 | F5:8 D6:8 | Eb6:8 C6:8 | A5:8 F#5:8 |", 0.66);
    s.pass(0);
    s.line(flute_ch, 8, theme_b, 0.64);
    s.line(flute_ch, 24, "| E5:8 G5:8 | F5:8 D5:8 | A5:8 C6:8 | E6:8 C6:8 | C6:8 A5:8 | D6:8 A5:8 | Bb5:8 F5:8 | E5:8 G5:8 |", 0.64);
    s.line(flute_ch, 32, theme_b2, 0.64);
    s.line(flute_ch, 40, "| G5:8 Bb5:8 | A5:8 C6:8 | Bb5:8 Eb6:8 | D6:8 F#5:8 | G5:4 D6:4 G6:8! | F6:8 C6:8 | D6:8 Bb5:8 | A5:8 D5:8 |", 0.66);

    // Working: the reed, resting in B' the first time through and in A the
    // second, where glass and a plucked canon answer.
    s.line(melody, 8, theme_b, 0.66);
    s.line(melody, 16, theme_a2, 0.66);
    s.line(melody, 24, theme_c, 0.66);
    s.line(melody, 40, theme_d, 0.68);
    let answer_line = |s: &mut Builder, bar: u32, text: &str, plucked: bool| {
        if plucked {
            s.line(canon, bar, text, 0.75);
            s.line(
                canon,
                bar,
                &format!("r:2 {}", text.trim_end_matches(" r:8")),
                0.4,
            );
        } else {
            s.line(answer, bar, text, 0.6);
        }
    };
    s.pass(1);
    s.line(melody, 0, theme_a, 0.66);
    answer_line(&mut s, 32, "r:4 F5:2 Bb5:2 D6:8", false);
    answer_line(&mut s, 34, "E5:2 G5:2 C6:4 r:8", true);
    answer_line(&mut s, 36, "r:4 D6:2 Bb5:2 F5:8", false);
    answer_line(&mut s, 38, "Eb5:2 G5:2 Bb5:4 r:8", true);
    s.pass(2);
    s.line(melody, 32, theme_b2, 0.66);
    answer_line(&mut s, 0, "r:4 D6:2 Bb5:2 G5:8", false);
    answer_line(&mut s, 2, "G5:2 Bb5:2 D6:4 r:8", true);
    answer_line(&mut s, 4, "r:4 G5:2 Bb5:2 Eb6:8", false);
    answer_line(&mut s, 6, "G5:2 Bb5:2 D6:4 r:8", true);
    s.pass(0);

    // Fighting: the lead, with chip chords spelled down from the second
    // theme's long notes; the reed below in chord tones.
    let battle_b = "| F5:12{-3,-7} D5:4 | C5:8{-3,-7} F5:4 A5:4 | G5:12{-3,-7} E5:4 | D5:16~{-4,-7} \
                    | F5:8 Bb5:8 | A5:12{-4,-9} C6:4 | Bb5:8{-3,-7} G5:4 Eb5:4 | F#5:8{-4,-9} D5:8 |";
    s.line(lead, 0, theme_a, 0.74);
    s.line(lead, 8, battle_b, 0.74);
    s.line(lead, 16, theme_a2, 0.76);
    s.line(lead, 24, theme_c, 0.78);
    s.line(lead, 32, battle_b, 0.76);
    s.line(lead, 40, theme_d, 0.8);
    for (bar, bars, figure_text) in [
        (0u32, 8u32, "3:6 5:2, 3:8"),
        (8, 8, "5:8 3:8"),
        (16, 8, "3:8 5:4 3:4,"),
        (24, 8, "R:4 3:4 5:8"),
        (32, 8, "5:8 3:8"),
        (40, 8, "3:6 5:2, 5:8"),
    ] {
        s.bass(counter, bar, bars, figure_text, 55, 0.58);
    }

    // Half-time: the kick on one and the "and" of three while the harbour
    // works, and the snare on three (the fight keeps it there).
    for bar in 0..48u32 {
        let pattern = if valley(bar) {
            "x..............."
        } else if section_ends.contains(&bar) {
            "x.......1.x.1.11"
        } else {
            "x.......1.x....."
        };
        let (kick, rest) = split_kit(pattern);
        s.beat(kit, bar, 1, &kick, 43, 0.9);
        if !valley(bar) {
            s.beat(snare3, bar, 1, &rest, 43, 0.85);
        }
    }
    for bar in (24u32..32).chain(40..48) {
        s.beat(clank, bar, 1, "......x.......x.", midi("G5"), 0.5);
    }

    // Threat: toms into each section, brass on the off-beats of two and
    // four.
    for (k, &end) in section_ends.iter().enumerate() {
        s.line(toms, end, "| r:8 G3:1! G3:1, D3:2 Bb2:2 G2:2 |", 0.62);
        if k > 0 {
            s.line(toms, section_ends[k - 1] + 1, "G4:16@1", 0.6);
        }
    }
    s.stab(brass_ch, 0, 48, &[6, 14], 2, 3, 55, 72, 0.58);

    // Fighting: arpeggios in eighths under A, A' and D. The fight stays
    // half-time: the snare only on three, open hats on the off-beats and
    // toms answering in the fourth beat.
    for (bar, bars) in [(0u32, 8u32), (16, 8), (40, 8)] {
        s.arp(
            arp,
            bar,
            bars,
            "0! - 2 - 4 - 2, - 1 - 3 - 5 - 3, -",
            1,
            67,
            2,
            0.42,
        );
    }
    s.beat(battle_kit, 0, 48, "..1...1.X.1.2.22", 60, 0.5);
    for &end in &section_ends {
        s.beat(battle_kit, end, 1, "..1...1.X.2222XX", 60, 0.7);
    }

    // D is the song's climb: while the harbour works, a pump in eighths,
    // sixteenth hats, a counter-line and the pad an octave up. Where the
    // reed rests, the counter-line and the high pad fill in.
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
    s.bass(climb, 40, 8, "R:2 O:2, R:2 O:2, R:2 O:2, R:2 O:2,", 43, 0.7);
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
    s.beat(climb_hats, 40, 8, "xgxgXgxgxgxgXgxg", 60, 0.6);
    let working_counter = s.channel(
        "counter working",
        reed(),
        Layer::Unfought,
        0.4,
        0.8,
        0.15,
        0.28,
    );
    s.role(working_counter, Role::Melody);
    s.bass(working_counter, 40, 8, "3:6 5:2, 3:4 5:4,", 62, 0.8);
    let pad_high = s.channel("pad high", pad(), Layer::Unfought, 0.0, 0.7, 0.0, 0.4);
    s.role(pad_high, Role::Pulse);
    s.pad(pad_high, 40, 8, 3, 65, 77, 0.5);
    for (pass, bar) in [(1u8, 32u32), (2, 0)] {
        s.pass(pass);
        s.bass(working_counter, bar, 8, "3:6 5:2, 3:4 5:4,", 55, 0.6);
        s.pad(pad_high, bar, 8, 3, 65, 77, 0.5);
        s.pass(0);
    }

    match_layers(&mut s, 55, 48);
    s.finish()
}
