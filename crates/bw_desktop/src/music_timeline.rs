//! A long match for the music engine: a scripted stream of requests (calm,
//! work, fights, counts, rests) played through the real `MusicEngine`,
//! logged bar by bar and checked against the rules the music should keep.
//! `brinewake --music-timeline DIR` writes the log; the tests run a shorter
//! script.

use crate::music::{MusicControl, MusicEngine, MusicState, Probe, SongId, Stop};
use std::fmt::Write as _;
use std::sync::Arc;

/// A request from a given second on.
#[derive(Clone, Copy, Debug)]
pub struct Cue {
    pub at: f64,
    pub state: MusicState,
}

fn working(song: SongId, intensity: u8) -> MusicState {
    MusicState {
        song: Some(song),
        intensity,
        ..MusicState::default()
    }
}

/// Thirty minutes: calm, then working; a fight at 10–12 min; an enemy
/// count from 15 min broken with 6 s left; a rest at 21 min broken by a
/// fight; our own count at 26 min.
pub fn script_a() -> Vec<Cue> {
    let mut cues = vec![
        Cue {
            at: 0.0,
            state: working(SongId::Match, 0),
        },
        Cue {
            at: 60.0,
            state: working(SongId::Match, 1),
        },
        Cue {
            at: 480.0,
            state: working(SongId::Match, 2),
        },
        Cue {
            at: 600.0,
            state: working(SongId::Match, 3),
        },
        Cue {
            at: 720.0,
            state: working(SongId::Match, 2),
        },
        Cue {
            at: 740.0,
            state: working(SongId::Match, 1),
        },
    ];
    // The enemy count: 90 s from 15:00, broken at 6 s left.
    for second in 0..=84u32 {
        cues.push(Cue {
            at: 900.0 + f64::from(second),
            state: MusicState {
                hold_theirs: true,
                hold_left: (90 - second) as u8,
                ..working(SongId::Match, 2)
            },
        });
    }
    cues.push(Cue {
        at: 985.0,
        state: working(SongId::Match, 2),
    });
    cues.push(Cue {
        at: 1_000.0,
        state: working(SongId::Match, 1),
    });
    // A rest asked at 21:00, broken by a fight twenty seconds later.
    cues.push(Cue {
        at: 1_260.0,
        state: MusicState {
            song: None,
            stop: Stop::Phrase,
            ..MusicState::default()
        },
    });
    cues.push(Cue {
        at: 1_280.0,
        state: working(SongId::Match, 3),
    });
    cues.push(Cue {
        at: 1_320.0,
        state: working(SongId::Match, 1),
    });
    // Our count from 26:00.
    for second in 0..=90u32 {
        cues.push(Cue {
            at: 1_560.0 + f64::from(second),
            state: MusicState {
                hold_ours: true,
                hold_left: (90 - second).max(1) as u8,
                ..working(SongId::Match, 2)
            },
        });
    }
    cues.push(Cue {
        at: 1_651.0,
        state: working(SongId::Match, 1),
    });
    cues
}

/// What a run found.
pub struct Report {
    pub log: String,
    pub violations: Vec<String>,
    pub songs_heard: Vec<SongId>,
    /// Each stretch on one song (none: silence): the song, its start and
    /// its end in seconds.
    pub runs: Vec<(Option<SongId>, f64, f64)>,
}

/// Play `cues` for `seconds` at `sr` through the engine. The requested
/// song follows the engine's own hand-overs, as the director does.
pub fn run(cues: &[Cue], seconds: f64, sr: f32) -> Report {
    let songs = crate::audio::soundtrack();
    let control = Arc::new(MusicControl::new());
    let mut engine = MusicEngine::new(songs, Arc::clone(&control), sr);
    let mut log = String::from(
        "time    song        pass bar  asked  plays  melody               decks race\n",
    );
    let mut violations = Vec::new();
    let mut songs_heard: Vec<SongId> = Vec::new();
    let mut runs: Vec<(Option<SongId>, f64, f64)> = Vec::new();
    let mut next_cue = 0usize;
    let mut asked = MusicState::default();
    let mut last: Option<Probe> = None;
    let mut two_decks_since: Option<f64> = None;
    let mut last_song_change = 0.0f64;
    let mut count_broken_at: Option<f64> = None;
    let mut after_rest: Option<SongId> = None;
    let total = (seconds * f64::from(sr)) as u64;
    let mut asked_at_bar: Vec<u8> = Vec::new();
    for n in 0..total {
        let now = n as f64 / f64::from(sr);
        while next_cue < cues.len() && now >= cues[next_cue].at {
            let mut state = cues[next_cue].state;
            let was_theirs = asked.hold_theirs;
            // The director asks for what is playing, not the script's
            // song, once the engine has handed over; after a rest that
            // fell silent, the next song in the rotation.
            if state.song.is_none() && state.stop == Stop::Phrase {
                after_rest = control.playing().map(SongId::rotation_next);
            } else if state.song.is_some() {
                if let Some(playing) = control.playing() {
                    state.song = Some(playing);
                } else if let Some(next) = after_rest.take() {
                    state.song = Some(next);
                }
            }
            if was_theirs && !state.hold_theirs {
                count_broken_at = Some(now);
            }
            asked = state;
            control.set(state, false);
            next_cue += 1;
        }
        engine.next();
        if n % 64 != 0 {
            continue;
        }
        let probe = engine.probe();
        // A broken enemy count's race is gone within half a second.
        if let Some(at) = count_broken_at
            && now - at > 0.5
        {
            if probe.race > 0.05 {
                violations.push(format!(
                    "{now:.1}s: the race still sounds after the count broke at {at:.1}s"
                ));
            }
            count_broken_at = None;
        }
        // Two songs overlapping more than half a second after a hand-over.
        if probe.decks >= 2 {
            let since = *two_decks_since.get_or_insert(now);
            if now - since > 2.0
                && now - last_song_change < 30.0
                && probe.song != last.as_ref().and_then(|p| p.song)
            {
                violations.push(format!(
                    "{now:.1}s: two decks sound for {:.1}s",
                    now - since
                ));
            }
        } else {
            two_decks_since = None;
        }
        let bar_changed = last
            .as_ref()
            .is_none_or(|p| p.bar != probe.bar || p.song != probe.song);
        if bar_changed {
            if last.as_ref().is_some_and(|p| p.song != probe.song) || last.is_none() {
                if let Some(song) = probe.song {
                    songs_heard.push(song);
                }
                if let Some(run) = runs.last_mut() {
                    run.2 = now;
                }
                runs.push((probe.song, now, now));
                last_song_change = now;
                // No hand-over while a count runs.
                if (asked.hold_ours || asked.hold_theirs)
                    && last.as_ref().is_some_and(|p| p.song.is_some())
                    && probe.song.is_some()
                {
                    violations.push(format!("{now:.1}s: the song changed during a count"));
                }
            }
            asked_at_bar.push(asked.intensity);
            // The layers in force match what was asked at one of the last
            // three bar lines: up to two bars of latency (a request is taken
            // a beat before a bar line, and a resumed rest or a hand-over
            // bar can hold it one bar more); the only exception is a calm
            // build-up from silence.
            if probe.song.is_some() && probe.pass > 1 || probe.bar >= 5 {
                let recent = &asked_at_bar[asked_at_bar.len().saturating_sub(3)..];
                if !recent.contains(&probe.intensity)
                    && asked.song.is_some()
                    && asked.stop != Stop::Phrase
                {
                    violations.push(format!(
                        "{now:.1}s: {:?} bar {} pass {} plays intensity {} while {:?} was asked",
                        probe.song,
                        probe.bar + 1,
                        probe.pass,
                        probe.intensity,
                        recent
                    ));
                }
            }
            let _ = writeln!(
                log,
                "{:>6.1}  {:<10}  {:>4} {:>3}  {:>5}  {:>5}  {:<20} {:>5} {:.2}",
                now,
                probe.song.map_or("-".to_string(), |s| format!("{s:?}")),
                probe.pass,
                (probe.bar % 64) + 1,
                asked.intensity,
                probe.intensity,
                probe.melody.unwrap_or("-"),
                probe.decks,
                probe.race
            );
            last = Some(probe);
        }
    }
    if let Some(run) = runs.last_mut() {
        run.2 = seconds;
    }
    Report {
        log,
        violations,
        songs_heard,
        runs,
    }
}

/// Write the thirty-minute run's log and findings to `dir`.
pub fn export(dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {dir:?}: {e}"))?;
    let report = run(&script_a(), 1_800.0, 8_000.0);
    let mut summary = String::new();
    let _ = writeln!(summary, "songs in order: {:?}", report.songs_heard);
    let _ = writeln!(summary, "violations: {}", report.violations.len());
    for v in &report.violations {
        let _ = writeln!(summary, "- {v}");
    }
    std::fs::write(dir.join("timeline.txt"), &report.log).map_err(|e| format!("{e}"))?;
    std::fs::write(dir.join("timeline-findings.txt"), &summary).map_err(|e| format!("{e}"))?;
    print!("{summary}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Six minutes of battle: every pass keeps the battle layers (the
    /// build-up is used once, not at every loop).
    #[test]
    fn a_long_battle_stays_a_battle_across_loops() {
        let cues = vec![Cue {
            at: 0.0,
            state: working(SongId::Match, 3),
        }];
        let report = run(&cues, 330.0, 2_000.0);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        let later: Vec<&str> = report.log.lines().filter(|l| l.contains("   2 ")).collect();
        assert!(!later.is_empty(), "the song looped");
        assert!(
            later
                .iter()
                .all(|l| l.split_whitespace().nth(5) == Some("3")),
            "pass 2 is still a battle"
        );
    }

    /// A count across the point where the song would hand over keeps the
    /// song; after it, the song may change.
    #[test]
    fn no_hand_over_during_a_count() {
        let mut cues = vec![Cue {
            at: 0.0,
            state: working(SongId::Match, 1),
        }];
        for second in 0..=90u32 {
            cues.push(Cue {
                at: 230.0 + f64::from(second),
                state: MusicState {
                    hold_theirs: true,
                    hold_left: (90 - second).max(1) as u8,
                    ..working(SongId::Match, 2)
                },
            });
        }
        cues.push(Cue {
            at: 321.0,
            state: working(SongId::Match, 1),
        });
        let report = run(&cues, 360.0, 2_000.0);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
    }

    /// A rest called off two seconds after it was asked (before the
    /// phrase ends) keeps the song: one deck, no change of key.
    #[test]
    fn a_rest_called_off_early_keeps_the_song() {
        let cues = vec![
            Cue {
                at: 0.0,
                state: working(SongId::Match, 1),
            },
            Cue {
                at: 20.0,
                state: MusicState {
                    song: None,
                    stop: Stop::Phrase,
                    ..MusicState::default()
                },
            },
            Cue {
                at: 22.0,
                state: working(SongId::Match, 2),
            },
        ];
        let report = run(&cues, 40.0, 2_000.0);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(report.songs_heard, vec![SongId::Match], "{}", report.log);
        let late = report.log.lines().last().unwrap_or_default().to_string();
        assert!(late.contains("Match"), "still playing at 40 s: {late}");
    }

    /// Under tension (intensity 2) all the time, every song still hands
    /// over at the end of its second pass.
    #[test]
    fn rotation_goes_on_under_tension() {
        let cues = vec![Cue {
            at: 0.0,
            state: working(SongId::Match, 2),
        }];
        let report = run(&cues, 560.0, 1_000.0);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(
            report.songs_heard,
            vec![SongId::Match, SongId::Confluence, SongId::Undertow]
        );
    }

    /// A count that begins during the hand-over bar calls the hand-over
    /// off: the song carries on through the count.
    #[test]
    fn a_count_during_the_hand_over_bar_keeps_the_song() {
        // Reclamation's second pass ends at 274.3 s; its last bar starts
        // at 270.0 s.
        let mut cues = vec![Cue {
            at: 0.0,
            state: working(SongId::Match, 1),
        }];
        for second in 0..=40u32 {
            cues.push(Cue {
                at: 272.0 + f64::from(second),
                state: MusicState {
                    hold_theirs: true,
                    hold_left: (90 - second) as u8,
                    ..working(SongId::Match, 2)
                },
            });
        }
        let report = run(&cues, 310.0, 1_000.0);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(report.songs_heard, vec![SongId::Match], "{}", report.log);
    }

    /// The engine reports its song while a rest waits for the phrase and
    /// while it fades, and none once it is silent: the director's rest
    /// choice depends on it.
    #[test]
    fn the_song_is_reported_until_silence() {
        let sr = 2_000.0;
        let control = Arc::new(MusicControl::new());
        let mut engine = MusicEngine::new(crate::audio::soundtrack(), Arc::clone(&control), sr);
        let mut play = |seconds: f64| {
            for _ in 0..(seconds * f64::from(sr)) as u64 {
                engine.next();
            }
        };
        control.set(working(SongId::Match, 1), false);
        play(3.0);
        assert_eq!(control.playing(), Some(SongId::Match));
        control.set(
            MusicState {
                song: None,
                stop: Stop::Phrase,
                ..MusicState::default()
            },
            false,
        );
        play(1.0);
        assert_eq!(
            control.playing(),
            Some(SongId::Match),
            "waiting for the phrase"
        );
        // The phrase ends on the next four-bar line (bar 5 at 8.6 s), then
        // a 3-s fade.
        play(5.0);
        assert_eq!(control.playing(), Some(SongId::Match), "fading");
        play(4.0);
        assert_eq!(control.playing(), None, "silent");
    }

    /// The whole thirty minutes, at a low rate for speed:
    /// `cargo test --release -p bw_desktop -- --ignored thirty_minutes`.
    #[test]
    #[ignore]
    fn thirty_minutes_keep_the_rules() {
        let report = run(&script_a(), 1_800.0, 4_000.0);
        assert!(report.violations.is_empty(), "{:#?}", report.violations);
        assert!(report.songs_heard.len() >= 3, "{:?}", report.songs_heard);
    }
}
