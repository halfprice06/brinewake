//! CONFLUENCE — the second match song: the three-seat map, and the one
//! after Reclamation when the music hands over (Undertow comes next). E Dorian, a step above the
//! rest of the record, at 120 BPM: three waters meeting. Its own plan: the
//! first theme moves its chords two bars at a time (Em7 A/E C D), the
//! second is long, high notes over the relative major, the third goes to a
//! key Reclamation never visits (B Dorian), and the tide line is a
//! twelve-bar phrase. The first theme is angular and syncopated on a
//! 3-3-2; the accompaniment runs straight so the syncopation pushes
//! against it. One melody passes between voices as in Reclamation.

use crate::music::{Instrument, Layer, Osc, Role, Song, SongId};
use crate::score::{Builder, midi};
use crate::score_songs::{
    bass, bell, brass, drive_bass, drum, flute, lead_assembly, lead_compact, lead_union, mallet,
    match_layers, pad, pluck, reed, split_kit, work_clank,
};

pub fn confluence() -> Song {
    let mut s = Builder::new(SongId::Confluence, "Confluence", 120.0, 4, 4, 64);
    s.song.partner = Some(SongId::Undertow);
    s.song.key = 2;
    s.song.swing = 0.1;
    s.song.echo_rows = 6.0;
    s.song.echo_feedback = 0.3;
    // A (8), A' (8), B (16), C in B Dorian (8), the tide (12), A'' (8),
    // and four bars home.
    s.chords(0, "Em7 Em7 A/E A/E C C D D");
    s.chords(8, "Em7 Em7 A/E A/E C D Em Em");
    s.chords(16, "G G D/F# D/F# Em7 Em7 C C Am7 Am7 D D G G B7 B7");
    s.chords(32, "Bm E/B Bm A G A Bm F#");
    s.chords(40, "Am7 Am7 C C D D Am7 Am7 C D B7 B7");
    s.chords(52, "Em7 Em7 A/E A/E C D B7 A7");
    // Home ends on D: the flat seventh here, and the dominant of G, so it
    // leads back to this song's start or on into Undertow.
    s.chords(60, "Em7 C D D");
    s.song.sections = vec![0, 8, 16, 32, 40, 52, 60];
    // D (G minor's dominant) leads into Undertow: the end of A and the
    // last bar.
    s.song.handovers = vec![7, 63];
    // Section lines, for fills: A, A', B, C, the tide, A'', home.
    let section_ends = [7u32, 15, 31, 39, 51, 59, 63];
    let valley = |bar: u32| (16..20).contains(&bar) || (40..44).contains(&bar);

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
        -0.1,
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
    let motor = s.channel("motor", mallet(), Layer::Core, -0.5, 0.8, 0.25, 0.25);
    s.role(motor, Role::Pulse);
    let shaker = s.channel(
        "shaker",
        drum(Osc::Shaker, 0.3, 0.1),
        Layer::Core,
        0.45,
        1.0,
        0.0,
        0.1,
    );
    s.role(shaker, Role::Kit);
    let flute_ch = s.channel("flute", flute(), Layer::Calm, 0.2, 0.95, 0.25, 0.35);
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
        -0.05,
        1.0,
        0.0,
        0.1,
    );
    s.extra(snare3, drum(Osc::Clap, 0.3, 0.2));
    s.poly(snare3);
    s.role(snare3, Role::Kit);
    let melody = s.channel("lead reed", reed(), Layer::Unfought, -0.15, 1.0, 0.22, 0.3);
    s.role(melody, Role::Melody);
    let answer = s.channel(
        "bell answers",
        bell(),
        Layer::Unfought,
        0.35,
        0.8,
        0.35,
        0.5,
    );
    let canon = s.channel(
        "pluck answers",
        pluck(),
        Layer::Unfought,
        -0.45,
        1.0,
        0.3,
        0.3,
    );
    s.poly(canon);
    s.role(answer, Role::Melody);
    s.role(canon, Role::Melody);
    let counter = s.channel("counter reed", reed(), Layer::Battle, -0.4, 0.9, 0.15, 0.28);
    s.role(counter, Role::Melody);
    let clank = s.channel(
        "clank",
        drum(Osc::Anvil, 0.16, 0.5),
        Layer::Drive,
        -0.6,
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
        -0.2,
        1.0,
        0.0,
        0.2,
    );
    s.extra(toms, drum(Osc::Crash, 0.2, 1.6));
    s.poly(toms);
    s.role(toms, Role::Kit);
    let brass_ch = s.channel("brass", brass(), Layer::Tension, 0.35, 1.0, 0.1, 0.25);
    s.role(brass_ch, Role::Pulse);
    let lead = s.channel("lead", lead_union(), Layer::Battle, 0.0, 1.0, 0.3, 0.3);
    s.variants(lead, [lead_union(), lead_assembly(), lead_compact()]);
    s.role(lead, Role::Melody);
    let arp = s.channel("arp", pluck(), Layer::Battle, 0.65, 0.9, 0.2, 0.2);
    s.role(arp, Role::Pulse);
    let battle_kit = s.channel(
        "battle kit",
        drum(Osc::Snare, 0.4, 0.3),
        Layer::Battle,
        -0.05,
        1.0,
        0.0,
        0.15,
    );
    s.extra(battle_kit, drum(Osc::HatOpen, 0.2, 0.3));
    s.poly(battle_kit);
    s.role(battle_kit, Role::Kit);

    // Bass: roots when calm; while working a figure that changes with the
    // section (3-3-2 under the first theme, walking under the second,
    // driving in B Dorian); a pump that changes too.
    for bar in 0..64u32 {
        let section = match bar {
            0..=15 => 0,
            16..=31 => 1,
            32..=39 => 2,
            40..=51 => 3,
            _ => 0,
        };
        let end = section_ends.contains(&bar);
        let roots_text = if end { "R:8 5:8," } else { "R:12 5:4," };
        s.bass(roots, bar, 1, roots_text, 40, 0.75);
        let figure_text = if valley(bar) {
            "R:8 5:8,"
        } else if end {
            // No chromatic approach: under these chords it rubs a
            // semitone against the pad and the reed.
            "R:3 R:3, 5:2 O:4 5:4"
        } else {
            [
                "R:3 R:3, R:2 O:3 5:3, R:2",
                "R:4 3:4 5:4 O:4,",
                "R:2 R:2, O:2 R:2, 5:2 R:2, O:2 5:2,",
                "R:6 5:2, O:4 5:4,",
            ][section]
        };
        if section == 1 && !valley(bar) && !end {
            // The second time through, the second theme walks differently.
            s.pass(1);
            s.bass(figure, bar, 1, figure_text, 40, 0.8);
            s.bass(edge, bar, 1, figure_text, 52, 0.8);
            s.pass(2);
            s.bass(figure, bar, 1, "R:6 5:2, 3:4 O:4,", 40, 0.8);
            s.bass(edge, bar, 1, "R:6 5:2, 3:4 O:4,", 52, 0.8);
            s.pass(0);
        } else {
            s.bass(figure, bar, 1, figure_text, 40, 0.8);
            s.bass(edge, bar, 1, figure_text, 52, 0.8);
        }
        let pump_text = [
            "R:2 R:2, O:2 R:2, R:2 R:2, O:2 R:2,",
            "R:4 O:4, R:4 O:4,",
            "R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1, R:1 R:1, O:1 R:1,",
            "R:2 O:2 R:2, O:2 R:2 O:2 R:2, O:2",
        ][section];
        s.bass(pump, bar, 1, pump_text, 40, 0.62);
    }
    s.pad(pad_ch, 0, 64, 3, 55, 71, 0.45);

    // Straight eighths in the mallet and sixteenths in the shaker, so the
    // tune's 3-3-2 pushes against a grid. Both rest in the valleys.
    let motor_figures = ["0 1 2 1", "0 2 1 2", "2 1 0 1", "0 1 2 3"];
    for bar in (0..64).filter(|&b| !valley(b)) {
        let section = match bar {
            0..=15 => 0,
            16..=31 => 1,
            32..=39 => 2,
            _ => 3,
        };
        s.arp(motor, bar, 1, motor_figures[section], 2, 64, 1, 0.42);
        s.beat(shaker, bar, 1, "XgxgXgxgXgxgXgxg", 60, 0.4);
    }

    // The themes.
    let head = "| E5:3 B4:3 E5:2, G5:3! F#5:3 E5:2, | D5:3 E5:3 G5:2! F#5:4 D5:4 | C#6:3 B5:3 A5:2, E5:3 F#5:3 A5:2 | A5:6! E5:2 C#5:4 D5:2 E5:2, ";
    let theme_a = format!(
        "{head}| E5:3 C5:3 E5:2, G5:3 C6:3 E6:2! | D6:3 C6:3 G5:2, E5:8~ | F#5:3 D5:3 A4:2, D5:3 F#5:3 A5:2 | B5:6! A5:2 F#5:4 D5:4 |"
    );
    let theme_a2 = format!(
        "{head}| E5:3 C5:3 E5:2, G5:3 C6:3 E6:2! | D6:3 A5:3 F#5:2, D5:8~ | E5:3 G5:3 B5:2, E6:8! | D6:3 B5:3 G5:2, E5:8 |"
    );
    let theme_a3 = format!(
        "{head}| E5:3 G5:3 C6:2, E6:8! | F#6:3 D6:3 A5:2, D6:8 | D#6:3 B5:3 F#5:2, A5:8 | B5:4! C#6:4 A5:4 G5:4 |"
    );
    // The second theme stays under F#6: the song's top waits for A''.
    let theme_b1 = "| D6:16~ | B5:8 G5:4 A5:4 | A5:16~ | F#5:8 A5:4 D6:4 | E6:12! D6:2 B5:2, | D6:8 B5:8 | E6:12~ D6:2 C6:2, | G5:8 C6:8 |";
    let theme_b2 = "| C6:12 B5:2 A5:2, | G5:8 E5:8 | F#5:12~ E5:2 F#5:2, | A5:8 D6:8 | B5:16~ | D6:8! B5:4 G5:4 | F#5:12 A5:4 | D#6:8 B5:8 |";
    let theme_c = "| B4:3 D5:3 F#5:2, B5:3! A5:3 F#5:2, | G#5:3 E5:3 B4:2, E5:8 | D5:3 F#5:3 B5:2, D6:8! | C#6:3 A5:3 E5:2, A5:8 \
                   | B5:3 G5:3 D5:2, G5:4 B5:4 | C#6:3 E6:3 A5:2, E5:8 | D6:3 B5:3 F#5:2, B5:8 | A#5:8! C#6:4 F#5:4 |";
    let tide = "| E5:8 G5:8 | A5:16~ | G5:8 E5:4 G5:4 | C6:16 | A5:6 B5:2, C6:8 | D6:16~ \
                | E6:6 D6:2, C6:8 | B5:6 A5:2, G5:8 | E5:4 G5:4 C6:8 | F#5:4 A5:4 D6:8! | D#6:8 B5:4 A5:4 | F#5:8 D#5:8 |";
    let home = "| E6:12~ D6:4 | C6:8 G5:8 | A5:8 F#5:8 | D6:8 A5:4 F#5:4 |";

    // Calm: the flute opens on the motif in long notes (B–E–F#–G) the
    // first time through, in other long notes the second; then the second
    // theme and the tide.
    s.pass(1);
    s.line(
        flute_ch,
        0,
        "| B4:8 E5:8~ | F#5:8 G5:8~ | A5:16~ | C#6:8 B5:8 | G5:16 | E5:8 G5:8 | A5:16~ | F#5:8 D5:8 \
         | E6:16~ | D6:8 B5:8 | C#6:16 | E6:8 A5:8 | G5:12 E5:4 | F#5:16 | G5:8 B5:8 | E5:16~ |",
        0.68,
    );
    s.pass(2);
    s.line(
        flute_ch,
        0,
        "| B5:16~ | A5:8 G5:8 | C#6:16~ | B5:8 A5:8 | G5:16 | E5:8 G5:8 | A5:16~ | F#5:8 D5:8 \
         | B4:8 E5:8~ | F#5:8 G5:8~ | C#6:16 | E6:8 A5:8 | G5:12 E5:4 | F#5:16 | G5:8 B5:8 | E5:16~ |",
        0.66,
    );
    s.pass(0);
    s.line(flute_ch, 16, theme_b1, 0.66);
    s.line(flute_ch, 24, theme_b2, 0.66);
    s.line(flute_ch, 40, tide, 0.62);
    // B Dorian, A'' and home in long notes: A'' climbs to the flute's one
    // F#6.
    s.line(flute_ch, 32, "| D5:8 F#5:8 | E5:8 G#5:8 | F#5:8 B5:8 | A5:8 C#6:8 | B5:8 G5:8 | A5:8 E5:8 | F#5:8 D5:8 | C#5:8 A#4:8 |", 0.64);
    s.line(flute_ch, 52, "| B5:8 E6:8 | D6:8 B5:8 | C#6:8 E6:8 | A5:8 C#6:8 | C6:8 E6:8 | F#6:8! D6:8 | D#6:8 B5:8 | C#6:8 G5:8 |", 0.64);
    s.line(
        flute_ch,
        60,
        "| E6:12~ D6:4 | C6:8 G5:8 | A5:8 F#5:8 | D6:8 A5:4 F#5:4 |",
        0.6,
    );

    // Working and gathering: the reed, resting at the second theme's start
    // (bars 17-24) and the tide's end (bars 45-52), where glass and a
    // plucked canon answer.
    s.line(melody, 24, theme_b2, 0.64);
    s.line(melody, 32, theme_c, 0.66);
    s.line(
        melody,
        40,
        "| E5:8 G5:8 | A5:16~ | G5:8 E5:4 G5:4 | C6:16 |",
        0.64,
    );
    s.line(melody, 60, home, 0.64);
    // The first time through the reed sings the heads; the second time it
    // ornaments the first, rests through A' and the start of A'', and glass
    // and the plucked canon answer.
    s.pass(1);
    s.line(melody, 0, &theme_a, 0.66);
    s.line(melody, 8, &theme_a2, 0.66);
    s.line(melody, 52, &theme_a3, 0.66);
    s.pass(2);
    s.line(
        melody,
        0,
        "| E5:2 D#5:1, B4:3 E5:2, G5:1, A5:1, G5:1! F#5:3 E5:2, | D5:3 E5:3 G5:2! F#5:2 G5:1, F#5:1 D5:4~ \
         | C#6:3 B5:3 A5:2, E5:3 F#5:3 A5:2~ | A5:6!~ E5:2 C#5:4 D5:2 E5:2, \
         | E5:2 D5:1, C5:3 E5:2, G5:3 C6:3 E6:2! | D6:3 C6:3 G5:2, E5:6~ F#5:1, E5:1 \
         | F#5:3 D5:3 A4:2, D5:3 F#5:3 A5:2~ | B5:5! C6:1, A5:2 F#5:4 D5:4 |",
        0.68,
    );
    s.line(
        melody,
        56,
        "| E5:3 G5:3 C6:2, E6:8! | F#6:3 D6:3 A5:2, D6:8 | D#6:3 B5:3 F#5:2, A5:8 | B5:4! C#6:4 A5:4 G5:4 |",
        0.66,
    );
    for (bar, text, plucked) in [
        (8u32, "E5:3 B5:3 D6:2, G5:8", false),
        (10, "E5:2 A5:2 C#6:4 r:8", true),
        (12, "G5:3 C6:3 E6:2, C6:8", false),
        (14, "E5:2 G5:2 B5:4 r:8", true),
        (52, "B5:3 D6:3 E6:2, B5:8", false),
        (54, "C#6:2 E6:2 A6:4 r:8", true),
    ] {
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
    }
    s.pass(0);
    let answers = [
        (16, "B5:3 D6:3 G6:2, D6:8", false),
        (18, "A5:2 D6:2 F#6:4 r:8", true),
        (20, "G5:3 B5:3 E6:2, B5:8", false),
        (22, "G5:2 C6:2 E6:4 r:8", true),
        (44, "A5:2 D6:2 F#6:4 r:8", true),
        (46, "E5:3 A5:3 C6:2, G5:8", false),
        (48, "G5:2 C6:2 E6:4 r:8", true),
        (50, "F#5:3 B5:3 D#6:2, A5:8", false),
    ];
    for (bar, text, plucked) in answers {
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
    }

    // Fighting: the lead, with chip chords spelled down from its long
    // notes in the second theme; the reed below in chord tones.
    s.line(lead, 0, &theme_a, 0.74);
    s.line(lead, 8, &theme_a2, 0.74);
    s.line(
        lead,
        16,
        "| D6:16~{-3,-7} | B5:8 G5:4 A5:4 | A5:16~{-3,-7} | F#5:8 A5:4 D6:4 | E6:12!{-5,-9} D6:2 B5:2, | D6:8 B5:8 | E6:12~{-4,-9} D6:2 C6:2, | G5:8 C6:8 \
         | C6:12{-3,-8} B5:2 A5:2, | G5:8 E5:8 | F#5:12~{-4,-9} E5:2 F#5:2, | A5:8 D6:8 | B5:16~{-4,-9} | D6:8! B5:4 G5:4 | F#5:12{-3,-7} A5:4 | D#6:8 B5:8 |",
        0.76,
    );
    s.line(lead, 32, theme_c, 0.78);
    s.line(lead, 40, tide, 0.76);
    s.line(lead, 52, &theme_a3, 0.8);
    s.line(lead, 60, home, 0.78);
    for (bar, bars, figure_text) in [
        (0u32, 8u32, "3:6 5:2, 3:8"),
        (8, 8, "5:8 3:8"),
        (16, 16, "3:8 5:4 3:4,"),
        (32, 8, "R:4 3:4 5:8"),
        (40, 12, "3:12 5:4,"),
        (52, 8, "5:6 3:2, 5:8"),
        (60, 4, "3:8 5:8"),
    ] {
        s.bass(counter, bar, bars, figure_text, 55, 0.58);
    }

    // The kick on a 3-3-2 under every working state; the snare on three
    // until the fight; the valleys thin to one and three.
    for bar in 0..64u32 {
        let pattern = if valley(bar) {
            "x.......x......."
        } else if section_ends.contains(&bar) {
            "x..x..x.1...1.11"
        } else if bar < 16 {
            "x..x..x.1......."
        } else {
            "x..x..x.1..x..x."
        };
        let (kick, rest) = split_kit(pattern);
        s.beat(kit, bar, 1, &kick, 40, 0.85);
        if !valley(bar) {
            s.beat(snare3, bar, 1, &rest, 40, 0.8);
        }
    }
    for bar in (8u32..16).chain(32..40).chain(52..60) {
        s.beat(clank, bar, 1, "...x.......x....", midi("E6"), 0.5);
    }

    // Threat: toms into each section, brass stabs that change by section.
    for (k, &end) in section_ends.iter().enumerate() {
        s.line(toms, end, "| r:8 E3:1! E3:1, B2:2 G2:2 E2:2 |", 0.62);
        if k > 0 {
            s.line(toms, section_ends[k - 1] + 1, "E4:16@1", 0.6);
        }
    }
    s.stab(brass_ch, 0, 16, &[3, 6, 11, 14], 2, 3, 59, 76, 0.55);
    s.stab(brass_ch, 16, 16, &[0, 8], 6, 3, 59, 76, 0.5);
    s.stab(brass_ch, 32, 8, &[2, 6, 10, 14], 2, 3, 59, 76, 0.58);
    s.stab(brass_ch, 40, 12, &[4, 12], 3, 3, 59, 76, 0.5);
    s.stab(brass_ch, 52, 12, &[3, 6, 11, 14], 2, 3, 59, 76, 0.55);

    // Fighting: the song's own 3-3-2 in the arpeggios under the first
    // theme and the third, open hats on its off-beats around the backbeat,
    // and fills.
    for (bar, bars) in [(0u32, 16u32), (32, 8), (52, 8)] {
        s.arp(arp, bar, bars, "0! 2 4, 0! 2 4, 1 3,", 1, 64, 1, 0.42);
    }
    s.beat(battle_kit, 0, 64, "...1x.1....1x.1.", 60, 0.5);
    for &end in &section_ends {
        s.beat(battle_kit, end, 1, "...1x.1.x..xX.XX", 60, 0.7);
    }

    // A'' is the song's climb: while the harbour works, a pump in
    // eighths, sixteenth hats, a counter-line and the pad an octave up.
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
    s.bass(climb, 52, 8, "R:2 O:2, R:2 O:2, R:2 O:2, R:2 O:2,", 40, 0.7);
    let climb_hats = s.channel(
        "climb hats",
        drum(Osc::HatClosed, 0.5, 0.04),
        Layer::Unfought,
        0.5,
        1.0,
        0.0,
        0.08,
    );
    s.role(climb_hats, Role::Kit);
    s.beat(climb_hats, 52, 8, "xgxgXgxgxgxgXgxg", 60, 0.6);
    let climb_counter = s.channel(
        "counter working",
        reed(),
        Layer::Unfought,
        -0.4,
        0.8,
        0.15,
        0.28,
    );
    s.role(climb_counter, Role::Melody);
    s.bass(climb_counter, 52, 8, "3:6 5:2, 3:4 5:4,", 59, 0.8);
    let pad_high = s.channel("pad high", pad(), Layer::Unfought, 0.0, 0.7, 0.0, 0.4);
    s.role(pad_high, Role::Pulse);
    s.pad(pad_high, 52, 8, 3, 64, 76, 0.5);
    // Where the reed rests, the counter-line and the high pad fill in.
    for bar in [16u32, 44] {
        s.bass(climb_counter, bar, 8, "3:6 5:2, 3:4 5:4,", 55, 0.6);
        s.pad(pad_high, bar, 8, 3, 64, 76, 0.5);
    }
    s.pass(2);
    s.bass(climb_counter, 8, 8, "3:6 5:2, 3:4 5:4,", 55, 0.6);
    s.pad(pad_high, 8, 8, 3, 64, 76, 0.5);
    s.pass(0);

    match_layers(&mut s, 52, 64);
    s.finish()
}
