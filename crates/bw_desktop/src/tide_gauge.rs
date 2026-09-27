//! The tide gauge in the top bar's centre: who holds the sluice, your two
//! crossings (each in its holder's colour, with its water drawn under the
//! letter), and the hold count.  It replaces the sluice card on the field;
//! the card, with its checklist and the tide keys, opens under the gauge on
//! hover or when the gauge is clicked, and stays open while pinned.

use crate::canvas::*;
use crate::game::{Action, Game};
use crate::native_ui::{Rect, button, small};
use crate::qol::hold_color;
use crate::seats::arm_letter;

/// Water colours for an arm's stripe: dry sand, shallow, deep.
const DRY: Color = [196, 170, 128, 255];
const SHALLOW: Color = [92, 146, 160, 255];
const DEEP: Color = [38, 70, 104, 255];

/// Logical width of the gauge.
const GAUGE_W: i32 = 132;

/// Which way the water goes when the gauge's tide timer runs out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TideArrow {
    /// An arm is about to dry: the water falls to sand.
    Drying,
    /// A flood is on its way: every crossing goes deep.
    Rising,
    /// A flood is running out.
    Ebbing,
}

impl TideArrow {
    pub(crate) fn colour(self) -> Color {
        match self {
            TideArrow::Drying => DRY,
            TideArrow::Rising | TideArrow::Ebbing => [128, 188, 214, 255],
        }
    }
}

/// A five-pixel arrow, up for rising water and down for falling, over a
/// line of water: the timer beside it is the tide's, not a lane's.
pub(crate) fn draw_tide_arrow(canvas: &mut Canvas, arrow: TideArrow, x: i32, y: i32, s: i32) {
    let colour = arrow.colour();
    let up = arrow == TideArrow::Rising;
    for row in 0..3 {
        // Rows of 5, 3 and 1 pixels: the point up or down.
        let width = if up { 1 + 2 * row } else { 5 - 2 * row };
        canvas.rect(x + (5 - width) / 2 * s, y + row * s, width * s, s, colour);
    }
    canvas.rect(x, y + 5 * s, 5 * s, s, colour);
}

impl Game {
    /// The gauge's place in the top bar as last drawn.
    #[cfg(test)]
    pub(crate) fn tide_gauge_bounds(&self) -> Option<Rect> {
        self.gauge_rect
    }

    /// Whether the sluice card shows under the gauge this frame: pinned
    /// open, hovered from the gauge, or opened by the practice step that
    /// sets the tide.  While an order or a placement waits for its click
    /// the card stands aside, so the ground under it takes the click.
    pub(crate) fn tide_card_shown(&self) -> bool {
        let Some(gauge) = self.gauge_rect else {
            return false;
        };
        if self.mode != crate::game::Mode::Context {
            return false;
        }
        if self.ux.preferences.tide_card {
            return true;
        }
        if self.ux.practice
            && self.ux.guidance_visible
            && self
                .ux
                .tutorial
                .as_ref()
                .is_some_and(|t| !t.is_complete() && t.stage() >= 6)
        {
            return true;
        }
        let (x, y) = self.cursor;
        gauge.contains(x, y) || (self.tide_card_hover && self.tide_card_reach().contains(x, y))
    }

    /// Where the pointer may travel once the gauge has opened the card:
    /// from under the gauge down through the card, so it does not close
    /// on the way.
    fn tide_card_reach(&self) -> Rect {
        let card = self.tide_card_rect();
        let top = self.gauge_rect.map_or(card.y, |g| g.y + g.h);
        Rect {
            y: top,
            h: card.y + card.h - top,
            ..card
        }
    }

    /// Follow the pointer: the gauge opens the hover card, and the card
    /// stays open while the pointer stays on the gauge or the card.  Only
    /// the gauge opens it; the field where it stands does not.
    pub(crate) fn follow_tide_card_hover(&mut self) {
        let (x, y) = self.cursor;
        let on_gauge = self.gauge_rect.is_some_and(|g| g.contains(x, y));
        self.tide_card_hover = self.mode == crate::game::Mode::Context
            && (on_gauge || (self.tide_card_hover && self.tide_card_reach().contains(x, y)));
    }

    /// Where the sluice card stands when it shows: under the gauge.
    pub(crate) fn tide_card_rect(&self) -> Rect {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let cw = 218 * s;
        let cx = self.gauge_rect.map_or(w / 2, |g| g.x + g.w / 2);
        Rect {
            x: (cx - cw / 2).clamp(8 * s, (w - 8 * s - cw).max(8 * s)),
            y: self.world_view().top + 4 * s,
            w: cw,
            h: if self.ux.preferences.compact_field_cards {
                // Folded, the holder keeps the tide controls.
                if self.world.gate.owner == Some(0) {
                    64 * s
                } else {
                    36 * s
                }
            } else if self.world.gate.owner == Some(0) {
                93 * s
            } else {
                71 * s
            },
        }
    }

    /// Draw the gauge in the top bar, starting no further left than
    /// `after`, and push its control.
    pub(crate) fn draw_tide_gauge(&mut self, after: i32, before: i32) {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let gw = GAUGE_W * s;
        let x0 = ((w - gw) / 2).max(after + 12 * s);
        if self.spectator_band() || x0 + gw > before - 6 * s {
            self.gauge_rect = None;
            return;
        }
        let r = Rect {
            x: x0,
            y: 2 * s,
            w: gw,
            h: 14 * s,
        };
        self.gauge_rect = Some(r);
        let open = self.tide_card_shown();
        self.canvas.rect(
            r.x,
            r.y,
            r.w,
            r.h,
            if open { [34, 53, 60, 255] } else { INK },
        );
        self.canvas
            .frame(r.x, r.y, r.w, r.h, if open { GOLD } else { EDGE });
        let gate = &self.world.gate;
        // The sluice: a gate between two posts in its holder's colour.
        let (gx, gy) = (r.x + 3 * s, r.y + 2 * s);
        let owner = gate
            .owner
            .map(|seat| hold_color(&self.world, seat))
            .unwrap_or(MUTED);
        self.canvas.rect(gx, gy, 2 * s, 10 * s, owner);
        self.canvas.rect(gx + 8 * s, gy, 2 * s, 10 * s, owner);
        self.canvas.rect(gx, gy + s, 10 * s, 2 * s, owner);
        self.canvas
            .rect(gx + 3 * s, gy + 4 * s, 4 * s, 5 * s, owner);
        // A capture in progress fills a line under it in the capturer's
        // colour.
        if let Some(player) = gate.capture_player.filter(|_| gate.capture_progress > 0) {
            let work = self.world.capture_work().max(1);
            let fill = (10 * s * gate.capture_progress.min(work) as i32) / work as i32;
            self.canvas.rect(gx, gy + 11 * s, 10 * s, s, EDGE);
            self.canvas.rect(
                gx,
                gy + 11 * s,
                fill.max(s),
                s,
                hold_color(&self.world, player),
            );
        }
        // Your crossings: the letter on its holder's colour, the water
        // under it.  A contested mouth is framed in the rival's colour.
        let seats_at = crate::qol::mouth_seats(&self.world);
        let tide = gate.tide;
        let dry_arm = gate.dry_arm.index();
        let warning = gate.warning_until.is_some();
        let switching = gate.switch_target.map(|a| a.index());
        let lit = (self.frame / 15).is_multiple_of(2);
        for (i, lane) in self.world.arms_of(0).into_iter().take(2).enumerate() {
            let cx = r.x + (17 + i as i32 * 17) * s;
            let holder = crate::qol::lane_holder_seat(&self.world, lane);
            let (own_present, enemy_mouth) = crate::qol::lane_presence(&self.world, lane);
            let rival = seats_at
                .get(lane)
                .map_or(0, |pair| (pair[0] | pair[1]) & !1)
                .trailing_zeros()
                .min(7) as u8;
            let (fill, ink) = match holder {
                Some(seat) => (hold_color(&self.world, seat), INK),
                None => (PANEL, WHITE),
            };
            self.canvas.rect(cx, r.y + 2 * s, 15 * s, 10 * s, fill);
            if own_present && enemy_mouth.is_some() {
                self.canvas.frame(
                    cx,
                    r.y + 2 * s,
                    15 * s,
                    10 * s,
                    hold_color(&self.world, rival.max(1)),
                );
            }
            small(
                &mut self.canvas,
                arm_letter(&self.world, lane),
                cx + 5 * s,
                r.y + 3 * s,
                ink,
                s,
            );
            let water = match tide {
                bw_sim::Tide::Flood => DEEP,
                bw_sim::Tide::Neutral => SHALLOW,
                bw_sim::Tide::Open if lane == dry_arm => DRY,
                bw_sim::Tide::Open => DEEP,
            };
            // The arm about to dry blinks between its water and sand.
            let water = if warning && switching == Some(lane) && lit {
                DRY
            } else {
                water
            };
            self.canvas.rect(cx, r.y + 10 * s, 15 * s, 2 * s, water);
        }
        // A tide change or a flood counts down beside the crossings, behind
        // an arrow in the colour of the water to come: sand falling for an
        // arm about to dry, blue rising for a flood on its way, blue falling
        // while a flood runs out.  Trial 10 read a bare "W 1S" as a
        // lane's number.
        let state = self.console_state();
        let tide_timer = match (
            state.route.warning_ticks_remaining,
            state.route.flood_ticks_remaining,
        ) {
            (Some(t), _) if state.route.flood_pending || state.route.ebb_pending => {
                Some((t, TideArrow::Rising))
            }
            (Some(t), _) => Some((t, TideArrow::Drying)),
            (None, Some(t)) => Some((t, TideArrow::Ebbing)),
            (None, None) => None,
        };
        let capture = crate::trial11_words::capture_status(&self.world);
        if let (None, Some(capture)) = (tide_timer, capture) {
            // Your claim: machines claiming of the four that count, in the
            // colour of how it goes (trial 11).
            small(
                &mut self.canvas,
                &capture.chip,
                r.x + 50 * s,
                r.y + 4 * s,
                capture.color,
                s,
            );
        }
        if let Some((ticks, arrow)) = tide_timer {
            let colour = arrow.colour();
            draw_tide_arrow(&mut self.canvas, arrow, r.x + 50 * s, r.y + 4 * s, s);
            let timer = format!("{}S", ticks.div_ceil(30));
            small(
                &mut self.canvas,
                &timer,
                r.x + 57 * s,
                r.y + 4 * s,
                colour,
                s,
            );
        }
        // The hold count: a bar in the counting seat's colour and the
        // seconds left, or an empty bar and the full count.
        let total = self.world.hold_ticks();
        let (bx, bw) = (r.x + 74 * s, 32 * s);
        self.canvas.rect(bx, r.y + 6 * s, bw, 3 * s, PANEL);
        let (label, color) = match self.hold_gauge() {
            Some((player, ticks)) => {
                let color = hold_color(&self.world, player);
                let fill = (i64::from(bw) * i64::from(ticks.min(total)) / i64::from(total)) as i32;
                self.canvas.rect(bx, r.y + 6 * s, fill, 3 * s, color);
                (
                    format!("{}S", total.saturating_sub(ticks).div_ceil(30)),
                    color,
                )
            }
            None => (format!("{}S", total / 30), MUTED),
        };
        small(
            &mut self.canvas,
            &label,
            r.x + 110 * s,
            r.y + 4 * s,
            color,
            s,
        );
        self.buttons.push(button(r, "", "", Action::TideCard, true));
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};

    fn skirmish() -> Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.start();
        g.resize_view(1280, 720);
        g.render();
        g
    }

    fn press(g: &mut Game, action: Action) {
        g.render();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == action)
            .cloned()
            .unwrap_or_else(|| panic!("{action:?} on screen"));
        assert!(b.enabled, "{action:?} enabled");
        g.left_down(b.x + b.w / 2, b.y + b.h / 2);
        g.left_up(b.x + b.w / 2, b.y + b.h / 2, false);
    }

    /// Owner: after drying one side, a folded card still offers the other
    /// side, and it flips the tide once the gate's lock has run out.
    #[test]
    fn a_folded_card_still_turns_the_tide_the_other_way() {
        use bw_sim::{Arm, Tide};
        let mut g = skirmish();
        g.world.ai_enabled = false;
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 500;
        press(&mut g, Action::SetTide(Arm::NORTH));
        for _ in 0..6000 {
            g.tick();
            if g.world.gate.tide == Tide::Open
                && g.world.gate.north_dry()
                && g.world.tick >= g.world.gate.locked_until
            {
                break;
            }
        }
        assert!(g.world.gate.north_dry());
        press(&mut g, Action::CompactFieldCards);
        assert!(g.ux.preferences.compact_field_cards);
        g.render();
        if let Ok(dir) = std::env::var("BW_MATCHUP_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&std::path::PathBuf::from(dir).join("folded-owner.png"))
                .expect("frame");
        }
        press(&mut g, Action::SetTide(Arm::SOUTH));
        for _ in 0..6000 {
            g.tick();
            if g.world.gate.tide == Tide::Open && !g.world.gate.north_dry() {
                break;
            }
        }
        assert!(!g.world.gate.north_dry(), "the south dries");
    }

    /// Every match opens with the card pinned and unfolded.
    #[test]
    fn a_new_match_unfolds_the_sluice_card() {
        let mut g = skirmish();
        g.ux.preferences.compact_field_cards = true;
        g.ux.preferences.tide_card = false;
        g.start();
        assert!(!g.ux.preferences.compact_field_cards);
        assert!(g.ux.preferences.tide_card);
    }

    /// Trial 10 read the header's "W 1S" as a number of the W lane: the
    /// tide timer now stands behind an arrow in the colour of the water to
    /// come, up for a flood, down for an arm drying or a flood running out.
    #[test]
    fn the_tide_timer_wears_an_arrow_of_the_coming_water() {
        use super::{TideArrow, draw_tide_arrow};
        let mut canvas = crate::canvas::Canvas::new(32, 32);
        draw_tide_arrow(&mut canvas, TideArrow::Rising, 0, 0, 1);
        // Point up: one pixel on top, five on the third row.
        assert_eq!(canvas.get(2, 0), Some(TideArrow::Rising.colour()));
        assert_eq!(canvas.get(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(canvas.get(0, 2), Some(TideArrow::Rising.colour()));
        let mut canvas = crate::canvas::Canvas::new(32, 32);
        draw_tide_arrow(&mut canvas, TideArrow::Drying, 0, 0, 1);
        assert_eq!(canvas.get(0, 0), Some(TideArrow::Drying.colour()));
        assert_eq!(canvas.get(0, 2), Some([0, 0, 0, 0]));
        assert_ne!(TideArrow::Drying.colour(), TideArrow::Rising.colour());
        // The gauge shows it during a switch warning.
        let mut g = skirmish();
        g.world.gate.warning_until = Some(g.world.tick + 300);
        g.world.gate.switch_target = Some(bw_sim::Arm::SOUTH);
        assert!(g.console_state().route.warning_ticks_remaining.is_some());
        g.render();
        let r = g.tide_gauge_bounds().expect("gauge");
        let s = g.ui_scale();
        let colour = TideArrow::Drying.colour();
        let lit = (0..6 * s)
            .flat_map(|dx| (0..7 * s).map(move |dy| (dx, dy)))
            .any(|(dx, dy)| g.canvas.get(r.x + 50 * s + dx, r.y + 4 * s + dy) == Some(colour));
        assert!(lit, "the arrow stands before the timer");
    }

    #[test]
    fn the_field_starts_clear_and_the_gauge_opens_the_card() {
        let mut g = skirmish();
        let gauge = g.tide_gauge_bounds().expect("the gauge is in the top bar");
        assert!(gauge.y + gauge.h <= g.world_view().top, "in the top bar");
        // The card starts pinned open (trial 11: both seats found it by
        // accident); unpinned, the field is clear.
        assert!(g.tide_card_shown(), "the card is open at the start");
        g.action(Action::TideCard);
        g.cursor = (10, 400);
        g.render();
        assert!(!g.tide_card_shown(), "unpinned, no card on the field");
        assert_eq!(g.route_bounds().w, 0);
        // Hovering the gauge shows the card under it.
        g.cursor = (gauge.x + gauge.w / 2, gauge.y + gauge.h / 2);
        g.render();
        assert!(g.tide_card_shown());
        let card = g.route_bounds();
        assert!(card.w > 0 && card.y >= g.world_view().top);
        // A click pins it open; moving away keeps it.
        g.action(Action::TideCard);
        g.cursor = (10, 400);
        g.render();
        assert!(g.tide_card_shown());
        g.action(Action::TideCard);
        g.render();
        assert!(!g.tide_card_shown());
    }
}
