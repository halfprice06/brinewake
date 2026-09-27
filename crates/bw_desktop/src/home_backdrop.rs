//! The home screen's backdrop: the field itself, full bleed behind the
//! menu.  With a match in progress it is that match, paused where it was
//! left; otherwise a field of its own plays on under the menu, the fog
//! lifted, while the camera drifts between a headquarters and the sluice.
//!
//! Presentation only.  The backdrop's world never touches the player's
//! match: it is swapped in for the one frame it is drawn and swapped out.

use crate::game::Game;
use bw_core::{Camera, Faction, Kind, Pos};
use bw_sim::World;

/// How many render frames one leg of the camera's drift takes.
const DRIFT_FRAMES: u64 = 60 * 40;

pub(crate) struct Backdrop {
    world: World,
    frames: u64,
}

impl Backdrop {
    fn new() -> Self {
        let mut world = World::new(5, Faction::Union);
        world.revealed = true;
        Self { world, frames: 0 }
    }

    /// Where the camera looks this frame: back and forth between the first
    /// headquarters and the sluice, eased at both ends.
    fn focus(&self) -> Pos {
        let hq = self
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map_or(self.world.map.gate_pos, |e| e.pos);
        let gate = self.world.map.gate_pos;
        let leg = self.frames % (2 * DRIFT_FRAMES);
        let t = if leg < DRIFT_FRAMES {
            leg
        } else {
            2 * DRIFT_FRAMES - leg
        } as f64
            / DRIFT_FRAMES as f64;
        let eased = t * t * (3.0 - 2.0 * t);
        let lerp = |a: i32, b: i32| a + ((f64::from(b - a)) * eased) as i32;
        Pos::raw(lerp(hq.x, gate.x), lerp(hq.y, gate.y))
    }
}

impl Game {
    /// Draw the field behind the home menu, full bleed.
    pub(crate) fn draw_home_backdrop(&mut self) {
        let cursor = self.cursor;
        let buttons = std::mem::take(&mut self.buttons);
        self.cursor = (-10_000, -10_000);
        self.full_bleed = true;
        if self.has_session() {
            // The match waiting to be continued, where it was left.
            self.draw_zoomed_world();
        } else {
            let mut backdrop = self
                .backdrop
                .take()
                .unwrap_or_else(|| Box::new(Backdrop::new()));
            backdrop.frames += 1;
            // The field plays on at half pace: busy enough to be alive,
            // calm enough to sit behind words.
            if backdrop.frames.is_multiple_of(2) {
                backdrop.world.step();
            }
            // The focus stands right of centre, clear of the menu column.
            let mut camera = Camera::default();
            camera.center(backdrop.focus());
            camera.x -= (f64::from(self.canvas.width()) * 0.18 / self.zoom.scale()) as i32;
            let camera = std::mem::replace(&mut self.camera, camera);
            let sub = std::mem::replace(&mut self.camera_sub, (0.0, 0.0));
            std::mem::swap(&mut self.world, &mut backdrop.world);
            self.draw_zoomed_world();
            std::mem::swap(&mut self.world, &mut backdrop.world);
            self.camera = camera;
            self.camera_sub = sub;
            self.backdrop = Some(backdrop);
        }
        self.full_bleed = false;
        self.cursor = cursor;
        self.buttons = buttons;
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Game, Screen};

    #[test]
    fn the_home_backdrop_plays_on_without_touching_the_match() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.resize_view(1280, 720);
        g.screen = Screen::Menu;
        let hash = g.world.state_hash();
        let camera = g.camera;
        for _ in 0..4 {
            g.render();
        }
        assert_eq!(
            g.world.state_hash(),
            hash,
            "the player's world is untouched"
        );
        assert_eq!((g.camera.x, g.camera.y), (camera.x, camera.y));
        let backdrop = g.backdrop.as_ref().expect("a backdrop world");
        assert!(backdrop.world.tick >= 2, "the backdrop plays on");
        // The field shows through: the frame is not the flat menu ink.
        assert_ne!(g.canvas.get(1100, 500), Some(crate::canvas::INK));
    }
}
