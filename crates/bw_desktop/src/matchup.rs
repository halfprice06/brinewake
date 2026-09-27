//! The moment a match begins: a card over the field with who plays whom,
//! on which map, and the two ways to win.  Offline the field waits under
//! it for two seconds (a click or a key starts at once); online it counts
//! down to the start the seats agreed on.  Learn to play opens on a
//! simpler card with only the ways to win, which waits for a click.

use crate::canvas::{EDGE, GOLD, INK, MUTED, WHITE};
use crate::game::Game;
use crate::native_ui::{Rect, small, text};
use crate::occlusion::PixelScale;

/// How long the offline card holds the field, in ticks.
pub const INTRO_TICKS: u32 = 60;

/// The card's state for this match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intro {
    /// Offline: ticks left before the field moves.
    pub ticks: u32,
    /// Learn to play: the how-to-win card, up until a click or a key.
    pub lesson: bool,
}

impl Game {
    /// Show the matchup card: the field holds under it offline, and online
    /// it counts down to the agreed start.
    pub(crate) fn begin_intro(&mut self) {
        self.unfold_sluice_card();
        if let Some(audio) = &self.audio {
            audio.play(crate::audio::Cue::MatchHorn);
        }
        self.intro = Some(Intro {
            ticks: INTRO_TICKS,
            lesson: false,
        });
    }

    /// Every match opens with the sluice card pinned and unfolded, so its
    /// tide controls are in sight; a fold or unpin lasts for that match.
    pub(crate) fn unfold_sluice_card(&mut self) {
        self.ux.preferences.tide_card = true;
        self.ux.preferences.compact_field_cards = false;
    }

    /// Open Learn to play on the how-to-win card. The field waits under it
    /// until the player clicks or presses a key.
    pub(crate) fn begin_lesson_intro(&mut self) {
        if let Some(audio) = &self.audio {
            audio.play(crate::audio::Cue::MatchHorn);
        }
        self.intro = Some(Intro {
            ticks: INTRO_TICKS,
            lesson: true,
        });
    }

    /// Advance the card by one tick. True while the field must hold.
    pub(crate) fn intro_holds(&mut self) -> bool {
        let Some(intro) = self.intro.as_mut() else {
            return false;
        };
        if let Some(session) = &self.session {
            // The session holds itself until the start; the card stays up
            // for a moment after, reading GO.
            if session
                .starts_at()
                .checked_duration_since(std::time::Instant::now())
                .is_none()
                && session.starts_at().elapsed() > std::time::Duration::from_millis(600)
            {
                self.intro = None;
            }
            return false;
        }
        if intro.lesson {
            return true;
        }
        intro.ticks = intro.ticks.saturating_sub(1);
        if intro.ticks == 0 {
            self.intro = None;
        }
        self.intro.is_some()
    }

    /// A click or a key while the offline card is up starts the match at
    /// once. True when the input was spent on that.
    pub(crate) fn skip_intro(&mut self) -> bool {
        if self.intro.is_some() && self.session.is_none() {
            self.intro = None;
            return true;
        }
        false
    }

    /// The online countdown: 3, 2, 1, then GO.
    fn countdown(&self) -> Option<String> {
        let session = self.session.as_ref()?;
        let left = session
            .starts_at()
            .checked_duration_since(std::time::Instant::now());
        Some(match left {
            Some(left) => (left.as_millis() as u64).div_ceil(1000).max(1).to_string(),
            None => "GO".into(),
        })
    }

    /// Where the card stands: centred on the field.
    pub(crate) fn intro_rect(&self) -> Rect {
        let s = self.ui_scale();
        let v = self.world_view();
        let (w, h) = (320 * s, 154 * s);
        Rect {
            x: (v.width - w) / 2,
            y: v.top + ((v.bottom - v.top - h) / 2).max(8 * s),
            w,
            h,
        }
    }

    pub(crate) fn draw_matchup(&mut self) {
        let Some(intro) = self.intro else {
            return;
        };
        if intro.lesson {
            self.draw_lesson_intro();
            return;
        }
        let s = self.ui_scale();
        let r = self.intro_rect();
        self.canvas.rect(r.x, r.y, r.w, r.h, INK);
        self.canvas.frame(r.x, r.y, r.w, r.h, EDGE);
        self.canvas.rect(r.x, r.y, r.w, 2 * s, GOLD);
        // The map, and how it is played.
        let map = match self.world.map.id {
            bw_sim::MapId::SplitBasin => "SPLIT BASIN",
            bw_sim::MapId::Confluence => "CONFLUENCE",
        };
        let seats = self.world.seat_count() as u8;
        let kind = if self.session.is_some() {
            "ONLINE"
        } else if seats > 2 {
            "FREE FOR ALL"
        } else {
            "SKIRMISH"
        };
        let head = format!("{map} / {kind}");
        let hw = head.len() as i32 * 6 * s;
        small(
            &mut self.canvas,
            &head,
            r.x + (r.w - hw) / 2,
            r.y + 7 * s,
            MUTED,
            s,
        );
        // One column per seat: its headquarters, name and faction.
        let col_w = r.w / i32::from(seats.max(1));
        let icon = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        for seat in 0..seats {
            let cx = r.x + col_w * i32::from(seat) + col_w / 2;
            let faction = self
                .world
                .players
                .get(usize::from(seat))
                .map_or(self.faction, |p| p.faction);
            let colour = crate::seats::seat_colour(&self.world, seat);
            // The faction's crest, framed in the seat's colour.
            let key = match faction {
                bw_core::Faction::Union => "portrait_union",
                bw_core::Faction::Assembly => "portrait_assembly",
                bw_core::Faction::Compact => "portrait_compact",
            };
            let top = r.y + 20 * s;
            self.canvas
                .rect(cx - 29 * s, top - s, 58 * s, 58 * s, colour);
            self.canvas
                .rect(cx - 28 * s, top, 56 * s, 56 * s, [23, 38, 46, 255]);
            let drawn = self.atlas.as_ref().is_some_and(|atlas| {
                atlas.draw_scaled(&mut self.canvas, key, cx - 28 * s, top, false, icon)
            });
            if !drawn {
                self.canvas
                    .diamond(cx, top + 28 * s, 14 * s, 18 * s, colour);
            }
            let name = crate::seats::seat_name(&self.world, seat);
            let nw = crate::canvas::text_readable_width(name) * s;
            text(&mut self.canvas, name, cx - nw / 2, top + 61 * s, colour, s);
            // With three seats YOU learn your own colour here: every
            // screen shows each player in one colour, yours included.
            let title =
                (seats > 2 && seat == 0).then(|| crate::seats::seat_title(&self.world, seat));
            let side = title.as_deref().unwrap_or(faction.name());
            let side = if side.len() as i32 * 6 * s > col_w - 6 * s {
                side.rsplit(' ').next().unwrap_or(side)
            } else {
                side
            };
            let sw = side.len() as i32 * 6 * s;
            small(&mut self.canvas, side, cx - sw / 2, top + 72 * s, WHITE, s);
            if seat + 1 < seats {
                let vx = r.x + col_w * i32::from(seat + 1);
                small(&mut self.canvas, "VS", vx - 6 * s, top + 25 * s, MUTED, s);
            }
        }
        // The two ways to win.
        let wy = r.y + 114 * s;
        self.canvas
            .line(r.x + 8 * s, wy - 5 * s, r.x + r.w - 9 * s, wy - 5 * s, EDGE);
        // The length for this match (rules 20): 75 s with three seats.
        let hold = format!(
            "HOLD {} LANES {}S",
            if seats > 2 { "YOUR" } else { "BOTH" },
            self.world.hold_ticks() / 30
        );
        let ways = [("DESTROY THE HQ", false), (hold.as_str(), true)];
        let width = |words: &str| words.len() as i32 * 6 * s + 14 * s;
        let gap = 18 * s;
        let mut x = r.x + (r.w - width(ways[0].0) - gap - width(ways[1].0)) / 2;
        for (words, crossings) in ways {
            draw_win_mark(&mut self.canvas, crossings, x, wy, s);
            small(&mut self.canvas, words, x + 14 * s, wy + s, WHITE, s);
            x += width(words) + gap;
        }
        // Offline: the time left and how to start now. Online: the count.
        match self.countdown() {
            Some(count) => {
                let big = 3 * s;
                let cw = crate::canvas::text_readable_width(&count) * big;
                let y = r.y + r.h + 4 * s;
                self.canvas
                    .rect(r.x + (r.w - cw) / 2 - 8 * s, y, cw + 16 * s, 31 * s, INK);
                text(
                    &mut self.canvas,
                    &count,
                    r.x + (r.w - cw) / 2,
                    y + 2 * s,
                    GOLD,
                    big,
                );
            }
            None => {
                let left = self.intro.map_or(0, |i| i.ticks) as i32;
                let bar = (r.w - 16 * s) * left / INTRO_TICKS as i32;
                self.canvas
                    .rect(r.x + 8 * s, r.y + r.h - 7 * s, bar, s, GOLD);
                let hint = "CLICK TO START NOW";
                let hw = hint.len() as i32 * 6 * s;
                small(
                    &mut self.canvas,
                    hint,
                    r.x + (r.w - hw) / 2,
                    r.y + r.h - 17 * s,
                    MUTED,
                    s,
                );
            }
        }
    }
}

impl Game {
    /// Learn to play's first card: how to win, and nothing else.
    fn draw_lesson_intro(&mut self) {
        let s = self.ui_scale();
        let r = self.intro_rect();
        self.canvas.rect(r.x, r.y, r.w, r.h, INK);
        self.canvas.frame(r.x, r.y, r.w, r.h, EDGE);
        self.canvas.rect(r.x, r.y, r.w, 2 * s, GOLD);
        let centre = |w: i32| r.x + (r.w - w) / 2;
        let title = "HOW TO WIN";
        let big = 2 * s;
        let tw = crate::canvas::text_readable_width(title) * big;
        text(&mut self.canvas, title, centre(tw), r.y + 16 * s, GOLD, big);
        let hold = format!("HOLD BOTH LANES FOR {}S", self.world.hold_ticks() / 30);
        let ways = [
            ("DESTROY THE ENEMY HQ", false, r.y + 58 * s),
            (hold.as_str(), true, r.y + 90 * s),
        ];
        let row = |words: &str| words.len() as i32 * 6 * s + 14 * s;
        let x = centre(row(ways[0].0).max(row(ways[1].0)));
        for (words, crossings, y) in ways {
            draw_win_mark(&mut self.canvas, crossings, x, y, s);
            small(&mut self.canvas, words, x + 14 * s, y + s, WHITE, s);
        }
        small(
            &mut self.canvas,
            "OR",
            centre(12 * s),
            r.y + 75 * s,
            MUTED,
            s,
        );
        let hint = "CLICK TO BEGIN";
        let hw = hint.len() as i32 * 6 * s;
        small(
            &mut self.canvas,
            hint,
            centre(hw),
            r.y + r.h - 17 * s,
            MUTED,
            s,
        );
    }
}

/// A headquarters, or two crossings with water between.
fn draw_win_mark(canvas: &mut crate::canvas::Canvas, crossings: bool, x: i32, y: i32, s: i32) {
    if crossings {
        canvas.rect(x, y + s, 10 * s, 2 * s, GOLD);
        canvas.rect(x, y + 6 * s, 10 * s, 2 * s, GOLD);
        canvas.rect(x + 2 * s, y + 4 * s, 2 * s, s, GOLD);
        canvas.rect(x + 6 * s, y + 4 * s, 2 * s, s, GOLD);
    } else {
        canvas.rect(x + s, y + 3 * s, 8 * s, 6 * s, GOLD);
        canvas.rect(x + 3 * s, y, 4 * s, 3 * s, GOLD);
        canvas.rect(x + 4 * s, y + 6 * s, 2 * s, 3 * s, INK);
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Game;

    fn game() -> Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        g
    }

    #[test]
    fn a_skirmish_opens_on_the_matchup_and_the_field_waits() {
        let mut g = game();
        g.begin_intro();
        g.render();
        if let Ok(dir) = std::env::var("BW_MATCHUP_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&std::path::PathBuf::from(dir).join("matchup.png"))
                .expect("frame");
        }
        let tick = g.world.tick;
        for _ in 0..30 {
            g.tick();
        }
        assert_eq!(g.world.tick, tick, "the field waits under the card");
        assert!(g.intro.is_some());
        for _ in 0..40 {
            g.tick();
        }
        assert!(g.intro.is_none(), "two seconds and it is gone");
        assert!(g.world.tick > tick, "then the field moves");
    }

    #[test]
    fn a_click_starts_the_match_at_once() {
        let mut g = game();
        g.begin_intro();
        g.render();
        let r = g.intro_rect();
        let selected = g.selected.clone();
        g.left_down(r.x + 4, r.y + 4);
        g.left_up(r.x + 4, r.y + 4, false);
        assert!(g.intro.is_none());
        assert_eq!(g.selected, selected, "the click did nothing else");
        let tick = g.world.tick;
        g.tick();
        assert!(g.world.tick > tick);
    }

    #[test]
    fn learn_to_play_opens_on_how_to_win_and_waits_for_a_click() {
        let mut g = game();
        g.begin_practice();
        g.render();
        if let Ok(dir) = std::env::var("BW_MATCHUP_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&std::path::PathBuf::from(dir).join("lesson.png"))
                .expect("frame");
        }
        assert!(g.intro.is_some_and(|i| i.lesson));
        let tick = g.world.tick;
        for _ in 0..300 {
            g.tick();
        }
        assert_eq!(g.world.tick, tick, "the card waits for the player");
        let r = g.intro_rect();
        g.left_down(r.x + 4, r.y + 4);
        g.left_up(r.x + 4, r.y + 4, false);
        assert!(g.intro.is_none());
        g.tick();
        assert!(g.world.tick > tick);
    }
}
