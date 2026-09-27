//! Practice coaching on the field: each step is a short bubble beside the
//! thing it is about (a worker, a wreck, a command-card key), with a pulsing
//! ring on that thing.  It replaces a large card in the corner that covered
//! a quarter of the field, often including the target it described.

use crate::canvas::*;
use crate::game::{Action, Game, Mode};
use crate::native_ui::{Rect, button, small, text, wrap_small};
use crate::onboarding::{TutorialTarget, TutorialView};
use bw_core::Kind;

/// What the coach points at this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoachRing {
    /// A place on the field, in screen pixels.
    Field(i32, i32),
    /// A control, by its bounds.
    Control(Rect),
}

impl CoachRing {
    /// The target's centre x, top and height, for placing the bubble.
    fn anchor(self, s: i32) -> (i32, i32, i32) {
        match self {
            // A field target is a ground point; the bubble stands clear of
            // the machine drawn above it and the ring below it.
            CoachRing::Field(x, y) => (x, y - 26 * s, 38 * s),
            CoachRing::Control(r) => (r.x + r.w / 2, r.y, r.h),
        }
    }
}

const BUBBLE_W: i32 = 260;

impl Game {
    /// The own machine or building of a kind nearest the middle of the
    /// view, when one is on screen.
    fn coach_entity(&self, want: impl Fn(&bw_sim::Entity) -> bool) -> Option<(i32, i32)> {
        let v = self.world_view();
        let (cx, cy) = v.center();
        self.world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && want(e))
            .map(|e| self.project(e.pos))
            .filter(|&(x, y)| v.contains(x, y))
            .min_by_key(|&(x, y)| (x - cx).pow(2) + (y - cy).pow(2))
    }

    fn coach_control(&self, action: &Action) -> Option<CoachRing> {
        self.coach_control_where(|a| a == action)
    }

    fn coach_control_where(&self, want: impl Fn(&Action) -> bool) -> Option<CoachRing> {
        self.buttons
            .iter()
            .find(|b| want(&b.action) && b.enabled)
            .map(|b| {
                CoachRing::Control(Rect {
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                })
            })
    }

    fn selected_kind(&self, want: impl Fn(Kind) -> bool) -> bool {
        self.selected.iter().any(|id| {
            self.world
                .entities
                .iter()
                .any(|e| e.id == *id && e.owner == 0 && want(e.kind))
        })
    }

    /// The target of the current step, if it can be pointed at now.
    pub(crate) fn coach_target(&self, view: &TutorialView) -> Option<CoachRing> {
        if view.complete {
            return None;
        }
        let field = |p: Option<(i32, i32)>| p.map(|(x, y)| CoachRing::Field(x, y));
        let worker = || field(self.coach_entity(|e| e.kind.is_worker()));
        let army = || {
            field(self.coach_entity(|e| {
                !e.kind.is_worker() && !e.kind.is_building() && e.build_remaining == 0
            }))
        };
        let army_selected = self.selected_kind(|k| !k.is_worker() && !k.is_building());
        let worker_selected = self.selected_kind(|k| k.is_worker());
        match view.step {
            1 => worker(),
            2 if worker_selected => {
                // The nearest wreck the worker could haul from, on screen.
                let v = self.world_view();
                let (cx, cy) = v.center();
                self.world
                    .map
                    .resources
                    .iter()
                    .filter(|r| r.remaining > 0 && self.world.visible(0, r.pos))
                    .map(|r| self.project(r.pos))
                    .filter(|&(x, y)| v.contains(x, y))
                    .min_by_key(|&(x, y)| (x - cx).pow(2) + (y - cy).pow(2))
                    .map(|(x, y)| CoachRing::Field(x, y))
            }
            2 => worker(),
            3 => {
                if matches!(self.mode, Mode::Build(_)) {
                    return None;
                }
                if let Some(p) =
                    self.coach_entity(|e| e.kind == Kind::Works && e.build_remaining > 0)
                {
                    return field(Some(p));
                }
                if worker_selected {
                    self.coach_control(&Action::Build(Kind::Works))
                } else {
                    worker()
                }
            }
            4 => {
                let first = self.world.players[0].faction.army()[0];
                self.coach_control(&Action::Train(first))
                    .filter(|_| self.selected_kind(|k| k == Kind::Works))
                    .or_else(|| field(self.coach_entity(|e| e.kind == Kind::Works)))
            }
            5 if army_selected => None,
            5 => army(),
            6 if army_selected => self.coach_control(&Action::Capture),
            6 => army(),
            7 => self
                .coach_control_where(|a| matches!(a, Action::SetTide(_)))
                .or_else(|| field(Some(self.project(self.world.map.gate_pos)))),
            _ => None,
        }
    }

    /// Draw the coach bubble and its controls; the ring is drawn after the
    /// controls by `draw_coach_ring`, so a key it rings is not painted over.
    pub(crate) fn draw_coach(&mut self) {
        let s = self.ui_scale();
        let v = self.world_view();
        self.coach_rect = None;
        self.coach_ring = None;
        if !self.ux.guidance_visible {
            // Folded: one small tab at the field's top right.
            let label = match self.ux.tutorial.as_ref().map(|t| t.view(&self.world)) {
                Some(view) if !view.complete => format!("GUIDE {}/{}", view.step, view.total),
                _ => "GUIDE".into(),
            };
            let w = (label.chars().count() as i32 * 6 + 12) * s;
            let r = Rect {
                x: v.width - 12 * s - w,
                y: v.top + 12 * s,
                w,
                h: 16 * s,
            };
            self.coach_rect = Some(r);
            self.buttons
                .push(button(r, &label, "", Action::EndGuidance, true));
            return;
        }
        let view = self.ux.tutorial.as_ref().map(|t| t.view(&self.world));
        let (title, body, find): (String, String, Option<(&str, Action)>) = match &view {
            None => (
                "PRACTICE".into(),
                "No enemy attacks. Start a lesson any time.".into(),
                Some(("NEW LESSON", Action::StartPractice)),
            ),
            Some(view) if view.complete => (
                view.title.to_ascii_uppercase(),
                view.body.clone(),
                Some(("TRY SKIRMISH", Action::Setup)),
            ),
            Some(view) => (
                view.title.to_ascii_uppercase(),
                view.body.clone(),
                match view.target {
                    TutorialTarget::Worker => Some(("FIND", Action::FocusWorker)),
                    TutorialTarget::Works => Some(("FIND", Action::FocusWorks)),
                    TutorialTarget::Army => Some(("FIND", Action::FocusArmy)),
                    TutorialTarget::Gate => Some(("FIND", Action::FocusGate)),
                    TutorialTarget::None => None,
                },
            ),
        };
        let ring = view.as_ref().and_then(|view| self.coach_target(view));
        // The FIND control sits at the right of the body; the body wraps
        // beside it.
        let find_w = find
            .as_ref()
            .map_or(0, |(label, _)| label.len() as i32 * 8 + 12);
        let body_room = BUBBLE_W - 16 - if find_w > 0 { find_w + 8 } else { 0 };
        let lines = wrap_small(&body.to_ascii_uppercase(), (body_room / 6) as usize, 3);
        let w = BUBBLE_W * s;
        let body_h = (lines.len() as i32 * 10).max(if find_w > 0 { 16 } else { 0 });
        let h = (24 + body_h + 6) * s;
        // Beside the target when there is one: above it, or below when the
        // field has no room above.  Otherwise at the top of the field.
        let wedge = 8 * s;
        let (x, y, below) = match ring.map(|r| r.anchor(s)) {
            Some((ax, ay, ah)) => {
                let above = ay - wedge - h;
                let (y, below) = if above >= v.top + 4 * s {
                    (above, false)
                } else {
                    (ay + ah + wedge, true)
                };
                let x = (ax - w / 2).clamp(8 * s, v.width - 8 * s - w);
                // Clear of the prompt row along the field's lower edge.
                (x, y.min(v.bottom - h - 16 * s), below)
            }
            None => ((v.width - w) / 2, v.top + 12 * s, false),
        };
        // Never over the sluice card or a running hold banner: beside
        // them where there is room, otherwise under them.
        let mut obstacles = vec![self.route_bounds()];
        if self.hold_gauge().is_some() && !self.spectator_band() {
            obstacles.push(self.hold_banner_bounds());
        }
        let clear = |x: i32, y: i32| {
            x >= 8 * s
                && x + w <= v.width - 8 * s
                && obstacles
                    .iter()
                    .all(|o| x >= o.x + o.w || x + w <= o.x || y >= o.y + o.h || y + h <= o.y)
        };
        let mut candidates = vec![(x, y)];
        for o in &obstacles {
            candidates.push((o.x + o.w + 6 * s, y));
            candidates.push((o.x - 6 * s - w, y));
            candidates.push((x, o.y + o.h + 6 * s));
        }
        let (x, y) = candidates
            .iter()
            .copied()
            .find(|&(x, y)| clear(x, y))
            .unwrap_or((x, y));
        let r = Rect { x, y, w, h };
        self.coach_rect = Some(r);
        self.coach_ring = ring;
        self.canvas.rect(r.x, r.y, r.w, r.h, [16, 27, 33, 245]);
        self.canvas.frame(r.x, r.y, r.w, r.h, JADE);
        // A wedge from the bubble toward the target.
        if let Some((ax, _, _)) = ring.map(|r| r.anchor(s)) {
            let tip_x = ax.clamp(r.x + wedge + 2 * s, r.x + r.w - wedge - 2 * s);
            for i in 0..wedge {
                let half = wedge - i;
                let yy = if below { r.y - 1 - i } else { r.y + r.h + i };
                self.canvas.line(tip_x - half, yy, tip_x + half, yy, JADE);
            }
        }
        // Progress as seven dots, then the step's name.
        let mut tx = r.x + 8 * s;
        if let Some(view) = view.as_ref().filter(|v| !v.complete) {
            for i in 0..view.total {
                let c = if i + 1 < view.step {
                    JADE
                } else if i + 1 == view.step {
                    WHITE
                } else {
                    EDGE
                };
                self.canvas
                    .rect(tx + i as i32 * 6 * s, r.y + 9 * s, 4 * s, 4 * s, c);
            }
            tx += view.total as i32 * 6 * s + 4 * s;
        }
        let hide_w = 16 * s;
        text(
            &mut self.canvas,
            &crate::native_ui::fit(&title, r.x + r.w - hide_w - 8 * s - tx, s),
            tx,
            r.y + 7 * s,
            WHITE,
            s,
        );
        for (i, line) in lines.iter().enumerate() {
            small(
                &mut self.canvas,
                line,
                r.x + 8 * s,
                r.y + (24 + i as i32 * 10) * s,
                MUTED,
                s,
            );
        }
        self.buttons.push(button(
            Rect {
                x: r.x + r.w - hide_w - 4 * s,
                y: r.y + 4 * s,
                w: hide_w,
                h: 14 * s,
            },
            "X",
            "",
            Action::EndGuidance,
            true,
        ));
        if let Some((label, action)) = find {
            let fw = find_w * s;
            self.buttons.push(button(
                Rect {
                    x: r.x + r.w - fw - 4 * s,
                    y: r.y + r.h - 20 * s,
                    w: fw,
                    h: 16 * s,
                },
                label,
                "",
                action,
                true,
            ));
        }
    }

    /// The pulsing ring on the coach's target, over everything else.
    pub(crate) fn draw_coach_ring(&mut self) {
        let Some(ring) = self.coach_ring else {
            return;
        };
        let s = self.ui_scale();
        let lit = (self.frame / 20).is_multiple_of(2);
        let c = if lit { WHITE } else { JADE };
        match ring {
            CoachRing::Field(x, y) => {
                let grow = if lit { 0 } else { 2 };
                for (rx, ry) in [(22 + grow, 11 + grow / 2), (23 + grow, 11 + grow / 2)] {
                    let (rx, ry) = (rx * s, ry * s);
                    self.canvas.line(x - rx, y, x, y - ry, c);
                    self.canvas.line(x, y - ry, x + rx, y, c);
                    self.canvas.line(x + rx, y, x, y + ry, c);
                    self.canvas.line(x, y + ry, x - rx, y, c);
                }
            }
            CoachRing::Control(r) => {
                let pad = if lit { 2 * s } else { 3 * s };
                for i in 0..s {
                    self.canvas.frame(
                        r.x - pad - i,
                        r.y - pad - i,
                        r.w + 2 * (pad + i),
                        r.h + 2 * (pad + i),
                        c,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};

    fn practice() -> Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.resize_view(1280, 720);
        g.begin_practice();
        g.render();
        g
    }

    fn clear_of(g: &Game) {
        let r = g.coach_rect.expect("the coach is drawn");
        let route = g.route_bounds();
        assert!(
            r.x >= route.x + route.w
                || r.x + r.w <= route.x
                || r.y >= route.y + route.h
                || r.y + r.h <= route.y,
            "the bubble {r:?} covers the sluice card {route:?}"
        );
        match g.coach_ring.expect("the step points at something") {
            super::CoachRing::Field(x, y) => {
                assert!(!r.contains(x, y), "the bubble covers its target");
            }
            super::CoachRing::Control(c) => {
                assert!(r.y + r.h <= c.y, "the bubble stands above the control");
            }
        }
    }

    #[test]
    fn each_step_points_beside_its_target_without_covering_it() {
        let mut g = practice();
        // Step 1 rings a worker on screen.
        assert!(matches!(g.coach_ring, Some(super::CoachRing::Field(..))));
        clear_of(&g);
        // Step 2 rings a wreck once a worker is selected.
        g.action(Action::FocusWorker);
        g.render();
        assert_eq!(g.ux.tutorial.as_ref().unwrap().stage(), 1);
        clear_of(&g);
        // Folded, the coach is one small tab and rings nothing.
        g.action(Action::EndGuidance);
        g.render();
        assert!(g.coach_ring.is_none());
        let tab = g.coach_rect.unwrap();
        assert!(tab.h <= 16 * g.ui_scale());
        assert!(g.buttons.iter().any(|b| b.label == "GUIDE 2/7"));
    }
}
