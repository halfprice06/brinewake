//! Illustrated, deterministic specialist lessons for the field guide.
//!
//! The manual is presentation state only.  It never reads a [`World`], never
//! advances the simulation clock, and never tries to recreate a live combat
//! event. The authored comparisons show complete in-game machines; the
//! tactical examples below are drawn from the same content
//! constants that govern the simulation so the explanation cannot drift from
//! the actual specialist rules.

use crate::canvas::{Atlas, Canvas, EDGE, GOLD, INK, JADE, MUTED, RED, WHITE};
use bw_content::{
    HELIOSTAT_BEAM_MAX, HELIOSTAT_BEAM_STEP, LOOM_BLAST_RADIUS, LOOM_MIN_RANGE, LOOM_WINDUP_TICKS,
    spec,
};
use bw_core::{FP, Faction, Kind};

/// The desktop timer and the simulation both use thirty ticks per second.
/// This clock belongs to the guide only; callers decide when to invoke
/// [`ManualState::advance`].
pub const MANUAL_TICKS_PER_SECOND: u32 = 30;

/// A deployed Bulwark pose remains visible for one complete second.
pub const BULWARK_DEPLOYED_TICKS: u32 = MANUAL_TICKS_PER_SECOND;

/// The short mechanical movement between packed and deployed poses is kept
/// readable without pretending to be the live deploy animation.
pub const BULWARK_TRANSITION_TICKS: u32 = MANUAL_TICKS_PER_SECOND;

/// The packed pose is long enough to identify the safe, mobile state.
pub const BULWARK_PACKED_TICKS: u32 = 24;

/// Loom's wind-up is the real twenty-four-tick warning window.
pub const LOOM_WARNING_TICKS: u32 = LOOM_WINDUP_TICKS;

pub const LOOM_PACKED_TICKS: u32 = 24;
pub const LOOM_RELEASED_TICKS: u32 = 30;

/// The Heliostat's lesson: packed, a second to deploy, then one beam held
/// on one target long enough to climb from its first shot to its cap.
pub const HELIOSTAT_PACKED_TICKS: u32 = 24;
pub const HELIOSTAT_DEPLOY_TICKS: u32 = MANUAL_TICKS_PER_SECOND;
/// Shots from the first to the cap, a cooldown apart, and a beat at the top.
pub fn heliostat_beam_ticks() -> u32 {
    let first = spec(Kind::Heliostat).damage;
    ((HELIOSTAT_BEAM_MAX - first) / HELIOSTAT_BEAM_STEP.max(1) + 2) as u32
        * spec(Kind::Heliostat).cooldown.max(1)
}

/// The beam's damage `tick` guide ticks into the held-beam phase: one step
/// a cooldown, from the first shot to the cap, as the simulation fires it.
pub fn heliostat_beam_at(tick: u32) -> i32 {
    let s = spec(Kind::Heliostat);
    let shots = (tick / s.cooldown.max(1)) as i32;
    (s.damage + shots * HELIOSTAT_BEAM_STEP).min(HELIOSTAT_BEAM_MAX)
}

/// Authored specialist plates are 224×144 with a top-left (0, 0) anchor.
pub const PLATE_WIDTH: i32 = 224;
pub const PLATE_HEIGHT: i32 = 144;

/// The warning and dodge illustration uses twelve pixels per cell so the
/// marked area and the unit moving outside it are visible at native size.
pub const DIAGRAM_CELL_PIXELS: i32 = 12;

/// The fixed ground point used by the Loom lesson.  It is deliberately a
/// diagram coordinate, not a target entity position.
pub const LOOM_MARKER_CELLS: i32 = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ManualSubject {
    #[default]
    Bulwark,
    Loom,
    Heliostat,
}

impl ManualSubject {
    /// The three specialists in the order the COMBAT page lists them.
    pub const ALL: [ManualSubject; 3] = [Self::Bulwark, Self::Loom, Self::Heliostat];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Bulwark => "BULWARK",
            Self::Loom => "LOOM",
            Self::Heliostat => "HELIOSTAT",
        }
    }

    /// Whose machine the lesson is about.
    pub const fn faction(self) -> Faction {
        match self {
            Self::Bulwark => Faction::Union,
            Self::Loom => Faction::Assembly,
            Self::Heliostat => Faction::Compact,
        }
    }

    /// The reader's own specialist: the page opens on it (trial 10's
    /// Compact found only the other sides' machines here).
    pub const fn of(faction: Faction) -> ManualSubject {
        match faction {
            Faction::Union => Self::Bulwark,
            Faction::Assembly => Self::Loom,
            Faction::Compact => Self::Heliostat,
        }
    }

    pub const fn plate_key(self, phase: usize) -> &'static str {
        match (self, phase) {
            (Self::Bulwark, 0) => "manual_bulwark_0",
            (Self::Bulwark, 1) => "manual_bulwark_1",
            (Self::Bulwark, _) => "manual_bulwark_2",
            (Self::Loom, 0) => "manual_loom_0",
            (Self::Loom, 1) => "manual_loom_1",
            (Self::Loom, _) => "manual_loom_2",
            // No authored plate: the page draws the engine's own sprites.
            (Self::Heliostat, 0) => "heliostat_0_idle_0",
            (Self::Heliostat, 1) => "heliostat_0_deploy_1",
            (Self::Heliostat, _) => "heliostat_0_deployed_fire_1",
        }
    }
}

/// Presentation state for the looping field-guide plate.
///
/// `phase_tick` is a bounded position in the selected subject's complete
/// three-phase cycle.  It is intentionally separate from `World::tick`: a
/// root timer can advance this clock while the Help page is open and leave a
/// paused match, replay hash, and render-state clock untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualState {
    pub subject: ManualSubject,
    pub playing: bool,
    pub phase_tick: u32,
}

impl Default for ManualState {
    fn default() -> Self {
        Self {
            subject: ManualSubject::Bulwark,
            playing: false,
            phase_tick: 0,
        }
    }
}

impl ManualState {
    /// Advance one guide tick when the plate is playing.  Calling this from a
    /// render function would make screenshots non-deterministic; callers
    /// should own the timer and call it explicitly.
    pub fn advance(&mut self) {
        if self.playing {
            self.advance_one_tick();
        }
    }

    /// Advance to the next named pose and pause. This gives a stable pose
    /// stepping contract for mouse, keyboard, and review fixtures while the
    /// playing timer still exposes the full warning/deploy durations.
    pub fn step(&mut self) {
        self.playing = false;
        let next_phase = (self.phase() + 1) % 3;
        self.phase_tick = phase_start_tick(self.subject, next_phase);
    }

    pub fn toggle_playing(&mut self) {
        self.playing = !self.playing;
    }

    /// Select a plate and restart its three-phase lesson at PACKED.  The
    /// play/pause choice belongs to the reader and is preserved.
    pub fn set_subject(&mut self, subject: ManualSubject) {
        if self.subject != subject {
            self.subject = subject;
            self.phase_tick = 0;
        }
    }

    pub fn phase(&self) -> usize {
        phase_at_tick(self.subject, self.phase_tick).0
    }

    pub fn phase_tick(&self) -> u32 {
        phase_at_tick(self.subject, self.phase_tick).1
    }

    #[allow(dead_code)]
    pub fn phase_duration_ticks(&self) -> u32 {
        phase_duration_ticks(self.subject, self.phase())
    }

    pub fn cycle_ticks(subject: ManualSubject) -> u32 {
        match subject {
            ManualSubject::Bulwark => {
                BULWARK_PACKED_TICKS + BULWARK_TRANSITION_TICKS + BULWARK_DEPLOYED_TICKS
            }
            ManualSubject::Loom => LOOM_PACKED_TICKS + LOOM_WARNING_TICKS + LOOM_RELEASED_TICKS,
            ManualSubject::Heliostat => {
                HELIOSTAT_PACKED_TICKS + HELIOSTAT_DEPLOY_TICKS + heliostat_beam_ticks()
            }
        }
    }

    fn advance_one_tick(&mut self) {
        let cycle = Self::cycle_ticks(self.subject);
        self.phase_tick = if cycle == 0 {
            0
        } else {
            (self.phase_tick + 1) % cycle
        };
    }
}

/// Return the duration of one named phase in guide ticks.
pub fn phase_duration_ticks(subject: ManualSubject, phase: usize) -> u32 {
    match subject {
        ManualSubject::Bulwark => match phase {
            0 => BULWARK_PACKED_TICKS,
            1 => BULWARK_TRANSITION_TICKS,
            _ => BULWARK_DEPLOYED_TICKS,
        },
        ManualSubject::Loom => match phase {
            0 => LOOM_PACKED_TICKS,
            1 => LOOM_WARNING_TICKS,
            _ => LOOM_RELEASED_TICKS,
        },
        ManualSubject::Heliostat => match phase {
            0 => HELIOSTAT_PACKED_TICKS,
            1 => HELIOSTAT_DEPLOY_TICKS,
            _ => heliostat_beam_ticks(),
        },
    }
}

fn phase_at_tick(subject: ManualSubject, tick: u32) -> (usize, u32) {
    let cycle = ManualState::cycle_ticks(subject);
    let mut remaining = if cycle == 0 { 0 } else { tick % cycle };
    for phase in 0..3 {
        let duration = phase_duration_ticks(subject, phase);
        if remaining < duration {
            return (phase, remaining);
        }
        remaining -= duration;
    }
    // The durations above are all positive.  Keep this defensive fallback
    // explicit so a future content edit cannot produce an invalid index.
    (2, 0)
}

fn phase_start_tick(subject: ManualSubject, phase: usize) -> u32 {
    match phase {
        0 => 0,
        1 => phase_duration_ticks(subject, 0),
        _ => phase_duration_ticks(subject, 0) + phase_duration_ticks(subject, 1),
    }
}

/// Convert a diagram origin into the committed Loom ground marker.  The
/// marker is independent of the moving target illustration and of the
/// selected phase.
pub const fn committed_ground_marker(origin: (i32, i32)) -> (i32, i32) {
    (origin.0 + LOOM_MARKER_CELLS * DIAGRAM_CELL_PIXELS, origin.1)
}

/// Small, deterministic target dodge used by the explanatory Loom diagram.
/// It changes the target silhouette's lane while leaving
/// [`committed_ground_marker`] unchanged.
pub const fn loom_target_position(origin: (i32, i32), windup_tick: u32) -> (i32, i32) {
    // The target leaves the committed point by two cells, then three. The
    // final three-cell dodge is outside Loom's actual 1.25-cell blast radius.
    let lane = if windup_tick < LOOM_WARNING_TICKS / 3 {
        0
    } else if windup_tick < (LOOM_WARNING_TICKS * 2) / 3 {
        2
    } else {
        3
    };
    let marker = committed_ground_marker(origin);
    (marker.0 + lane * DIAGRAM_CELL_PIXELS, marker.1)
}

/// Draw the specialist lesson in the Help page's bounded combat work area.
/// The comparison keeps mobile/deployed configurations visible together.
/// The adjacent example explains one tactical consequence at a time.
pub fn draw_combat_page(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    state: &ManualState,
    plate_origin: (i32, i32),
    diagram_origin: (i32, i32),
) {
    let phase = state.phase();
    draw_plate(canvas, atlas, state.subject, phase, plate_origin);
    draw_diagram(canvas, atlas, state, diagram_origin);
}

fn draw_plate(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    subject: ManualSubject,
    phase: usize,
    origin: (i32, i32),
) {
    let (x, y) = origin;
    canvas.rect(x, y, PLATE_WIDTH, PLATE_HEIGHT, [35, 49, 55, 255]);
    if subject == ManualSubject::Heliostat {
        // The engine's own sprite at twice its size, anchored at its feet,
        // with the pose named under it.
        let drawn = atlas.is_some_and(|atlas| {
            atlas.draw_scaled(
                canvas,
                subject.plate_key(phase),
                x + PLATE_WIDTH / 2,
                y + 112,
                false,
                crate::occlusion::PixelScale {
                    numerator: 2,
                    denominator: 1,
                },
            )
        });
        if !drawn {
            canvas.diamond(x + PLATE_WIDTH / 2, y + 80, 20, 24, EDGE);
        }
        let pose = ["PACKED: MOVES", "DEPLOYING: 1S", "DEPLOYED: FIRES"][phase.min(2)];
        canvas.text_readable(
            pose,
            x + (PLATE_WIDTH - crate::canvas::text_readable_width(pose)) / 2,
            y + 130,
            if phase == 2 { JADE } else { MUTED },
        );
        return;
    }
    let drawn =
        atlas.is_some_and(|atlas| atlas.draw(canvas, subject.plate_key(phase), x, y, false));
    if !drawn {
        canvas.text_readable("IMAGE UNAVAILABLE", x + 36, y + 61, MUTED);
        canvas.text_readable(subject.label(), x + 82, y + 78, WHITE);
    }
}

fn arrow(canvas: &mut Canvas, from: (i32, i32), to: (i32, i32), color: crate::canvas::Color) {
    canvas.line(from.0, from.1, to.0, to.1, color);
    let side = if to.0 >= from.0 { -1 } else { 1 };
    canvas.line(to.0, to.1, to.0 + side * 5, to.1 - 3, color);
    canvas.line(to.0, to.1, to.0 + side * 5, to.1 + 3, color);
}

fn draw_diagram(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    state: &ManualState,
    origin: (i32, i32),
) {
    let (x, y) = origin;
    let paper = [35, 49, 55, 255];
    canvas.rect(x, y, 306, 144, paper);
    let (title, first, second) = match state.subject {
        ManualSubject::Bulwark => (
            "HOLD A FRONT",
            "Deploy to resist frontal hits.",
            "Pack before turning or moving.",
        ),
        ManualSubject::Loom => (
            "DODGE THE MARK",
            "The shot hits the marked ground.",
            "Move your units out in 0.8s.",
        ),
        ManualSubject::Heliostat => (
            "HOLD THE BEAM",
            "Each shot on one target hits harder.",
            "A new target starts it over.",
        ),
    };
    canvas.text_readable(title, x + 12, y + 8, GOLD);
    canvas.text_readable(first, x + 12, y + 27, WHITE);
    canvas.text_readable(second, x + 12, y + 43, MUTED);
    match state.subject {
        ManualSubject::Bulwark => {
            // This whole diagram is explicitly the deployed case, while the
            // paired art beside it keeps both configurations visible.
            canvas.text("WHEN DEPLOYED", x + 107, y + 65, MUTED);
            if let Some(atlas) = atlas {
                atlas.draw(canvas, "bulwark_0_deploy_2", x + 148, y + 117, false);
            }
            canvas.text_readable("EXPOSED", x + 20, y + 78, RED);
            canvas.text_readable("PROTECTED", x + 207, y + 78, JADE);
            arrow(canvas, (x + 26, y + 102), (x + 125, y + 102), RED);
            arrow(canvas, (x + 278, y + 102), (x + 178, y + 102), MUTED);
            canvas.rect(x + 174, y + 93, 2, 18, GOLD);
            canvas.text("REAR", x + 45, y + 113, MUTED);
            canvas.text("FRONT", x + 229, y + 113, MUTED);
            canvas.text_readable("Flank a deployed Bulwark.", x + 12, y + 130, WHITE);
        }
        ManualSubject::Loom => {
            let origin = (x + 66, y + 102);
            let marker = committed_ground_marker(origin);
            let radius = (LOOM_BLAST_RADIUS * DIAGRAM_CELL_PIXELS / FP).max(1);
            if let Some(atlas) = atlas {
                atlas.draw(canvas, "loom_0_deploy_2", x + 40, y + 121, false);
            }
            for xx in (x + 65..marker.0 - radius - 4).step_by(7) {
                canvas.line(xx, y + 101, xx + 3, y + 101, EDGE);
            }
            canvas.ellipse(marker.0, marker.1, radius, radius, RED);
            canvas.ellipse(marker.0, marker.1, radius - 2, radius - 2, paper);
            canvas.line(marker.0 - 4, marker.1, marker.0 + 4, marker.1, RED);
            canvas.line(marker.0, marker.1 - 4, marker.0, marker.1 + 4, RED);
            canvas.text("FIXED MARK", marker.0 - 27, y + 75, MUTED);
            let tick = match state.phase() {
                0 => 0,
                1 => state.phase_tick(),
                _ => LOOM_WARNING_TICKS - 1,
            };
            let target = loom_target_position(origin, tick);
            canvas.rect(target.0 - 4, target.1 - 4, 9, 9, INK);
            canvas.frame(target.0 - 4, target.1 - 4, 9, 9, WHITE);
            canvas.rect(target.0 - 1, target.1 - 1, 3, 3, JADE);
            arrow(
                canvas,
                (marker.0 + radius + 6, y + 116),
                (x + 184, y + 116),
                JADE,
            );
            canvas.text_readable("MOVE CLEAR", x + 204, y + 97, JADE);
            if state.phase() == 2 {
                canvas.diamond(marker.0, marker.1, 5, 5, GOLD);
            }
            let min_cells = LOOM_MIN_RANGE / FP;
            let max_cells = spec(Kind::Loom).range / FP;
            canvas.text(
                &format!("RANGE {min_cells}-{max_cells} CELLS / DEPLOY TO FIRE"),
                x + 12,
                y + 133,
                MUTED,
            );
        }
        ManualSubject::Heliostat => {
            // The beam from the mirror to one target, and the damage of each
            // shot as a bar that climbs while it stays on that target.
            if let Some(atlas) = atlas {
                atlas.draw(canvas, "heliostat_0_deploy_2", x + 40, y + 121, false);
            }
            let damage = match state.phase() {
                2 => heliostat_beam_at(state.phase_tick()),
                _ => 0,
            };
            let (tx, ty) = (x + 250, y + 104);
            canvas.rect(tx - 4, ty - 4, 9, 9, INK);
            canvas.frame(tx - 4, ty - 4, 9, 9, WHITE);
            canvas.rect(tx - 1, ty - 1, 3, 3, RED);
            if damage > 0 {
                canvas.line(x + 52, y + 98, tx - 5, ty, GOLD);
                canvas.line(x + 52, y + 99, tx - 5, ty + 1, GOLD);
            }
            // One bar per shot from the first to the cap.
            let first = spec(Kind::Heliostat).damage;
            let shots = (HELIOSTAT_BEAM_MAX - first) / HELIOSTAT_BEAM_STEP.max(1) + 1;
            for i in 0..shots {
                let value = first + i * HELIOSTAT_BEAM_STEP;
                let bx = x + 92 + i * 14;
                let h = 3 + value * 2;
                let lit = damage >= value;
                canvas.rect(bx, y + 92 - h, 10, h, if lit { GOLD } else { EDGE });
            }
            canvas.text(
                &format!("SHOT {first} TO {HELIOSTAT_BEAM_MAX}"),
                x + 12,
                y + 66,
                MUTED,
            );
            if damage > 0 {
                canvas.text_readable(&format!("HIT {damage}"), x + 232, y + 80, GOLD);
            }
            let reach = spec(Kind::Heliostat).range / FP;
            canvas.text(
                &format!("RANGE {reach} CELLS / DEPLOY TO FIRE / HEAVY TARGETS"),
                x + 12,
                y + 133,
                MUTED,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    #[test]
    fn default_manual_starts_packed_and_paused() {
        let state = ManualState::default();
        assert_eq!(state.subject, ManualSubject::Bulwark);
        assert!(!state.playing);
        assert_eq!(state.phase(), 0);
        assert_eq!(state.phase_tick(), 0);
    }

    #[test]
    fn pause_freezes_and_step_advances_next_pose() {
        let mut state = ManualState::default();
        state.advance();
        assert_eq!(state.phase_tick, 0);
        state.step();
        assert!(!state.playing);
        assert_eq!(state.phase(), 1);
        assert_eq!(state.phase_tick(), 0);
        assert_eq!(state.phase_tick, BULWARK_PACKED_TICKS);
        state.advance();
        assert_eq!(state.phase_tick, BULWARK_PACKED_TICKS);
    }

    #[test]
    fn bulwark_deployed_pose_lasts_one_second() {
        let mut state = ManualState {
            subject: ManualSubject::Bulwark,
            playing: false,
            phase_tick: BULWARK_PACKED_TICKS + BULWARK_TRANSITION_TICKS,
        };
        assert_eq!(state.phase(), 2);
        assert_eq!(state.phase_duration_ticks(), MANUAL_TICKS_PER_SECOND);
        state.playing = true;
        for _ in 0..BULWARK_DEPLOYED_TICKS - 1 {
            state.advance();
            assert_eq!(state.phase(), 2);
        }
        state.advance();
        assert_eq!(state.phase(), 0);
    }

    #[test]
    fn loom_windup_is_the_twenty_four_tick_warning() {
        let mut state = ManualState {
            subject: ManualSubject::Loom,
            playing: false,
            phase_tick: LOOM_PACKED_TICKS,
        };
        assert_eq!(state.phase(), 1);
        assert_eq!(state.phase_duration_ticks(), LOOM_WINDUP_TICKS);
        state.playing = true;
        for _ in 0..LOOM_WINDUP_TICKS - 1 {
            state.advance();
            assert_eq!(state.phase(), 1);
        }
        state.advance();
        assert_eq!(state.phase(), 2);
    }

    #[test]
    fn committed_marker_does_not_follow_the_dodging_target() {
        let origin = (100, 120);
        let marker = committed_ground_marker(origin);
        assert_eq!(marker, committed_ground_marker(origin));
        assert_ne!(
            loom_target_position(origin, 0),
            loom_target_position(origin, 20)
        );
        assert_eq!(marker.1, loom_target_position(origin, 0).1);
        assert_eq!(marker.1, loom_target_position(origin, 20).1);
        assert_eq!(marker.1, origin.1);
        let final_target = loom_target_position(origin, LOOM_WARNING_TICKS - 1);
        let dx = i64::from(final_target.0 - marker.0);
        let dy = i64::from(final_target.1 - marker.1);
        assert!(
            (dx * dx + dy * dy) * i64::from(FP).pow(2)
                > i64::from(LOOM_BLAST_RADIUS).pow(2) * i64::from(DIAGRAM_CELL_PIXELS).pow(2)
        );
    }

    #[test]
    fn each_side_opens_on_its_own_specialist_and_the_beam_climbs_to_its_cap() {
        for faction in Faction::ALL {
            assert_eq!(ManualSubject::of(faction).faction(), faction);
        }
        assert_eq!(heliostat_beam_at(0), spec(Kind::Heliostat).damage);
        assert_eq!(
            heliostat_beam_at(heliostat_beam_ticks() - 1),
            HELIOSTAT_BEAM_MAX
        );
        let mut state = ManualState::default();
        state.set_subject(ManualSubject::Heliostat);
        state.step();
        state.step();
        assert_eq!(state.phase(), 2);
        let mut canvas = Canvas::default();
        draw_combat_page(&mut canvas, None, &state, (40, 104), (286, 104));
    }

    #[test]
    fn diagram_is_bounded_at_native_canvas_size_without_atlas() {
        let mut canvas = Canvas::default();
        let state = ManualState::default();
        draw_combat_page(&mut canvas, None, &state, (40, 104), (286, 104));
        let nonzero = canvas
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[3] != 0)
            .count();
        assert!(nonzero > 0);
        assert!(nonzero < 640 * 360);
    }
}
