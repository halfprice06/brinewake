//! Marks painted on the ground under machines: who owns a machine, and
//! whether a line machine is braced.
//!
//! In the eighth trial the Assembly believed it had killed deployed
//! Bulwarks (they were packed), and both seats asked for more than a thin
//! red edge to tell the enemy apart.  An enemy machine now stands on a red
//! plate, and a deployed Bulwark, Loom or Caisson drives braces into the
//! ground; a Bulwark also lays its shield line across its front.
//! Presentation only.

use crate::canvas::{Canvas, Color};
use bw_core::Kind;

/// The enemy plate: a bright front rim, a darker rim behind the machine
/// and a thin wash inside.
pub const ENEMY_RIM: Color = [214, 78, 62, 255];
pub const ENEMY_RIM_BACK: Color = [142, 50, 42, 255];
pub const ENEMY_WASH: Color = [206, 72, 58, 64];
/// The second opponent's plate in a match of three: the same shape in violet.
pub const RIVAL_RIM: Color = [160, 112, 226, 255];
pub const RIVAL_RIM_BACK: Color = [98, 64, 150, 255];
pub const RIVAL_WASH: Color = [150, 104, 220, 64];

/// A third player's plate in a jade seat colour (the Assembly's own).
pub const JADE_RIM: Color = [96, 184, 146, 255];
pub const JADE_RIM_BACK: Color = [52, 112, 88, 255];
pub const JADE_WASH: Color = [96, 184, 146, 64];
/// A mirror match's fourth colour: pink.
pub const PINK_RIM: Color = [226, 104, 164, 255];
pub const PINK_RIM_BACK: Color = [140, 58, 102, 255];
pub const PINK_WASH: Color = [226, 104, 164, 64];

/// An opponent's plate colours in its seat colour: front rim, back rim,
/// wash.  The colour follows the player, the same on every screen.
pub fn plate_tone(hue: crate::seats::Hue) -> (Color, Color, Color) {
    use crate::seats::Hue;
    match hue {
        Hue::Red => (ENEMY_RIM, ENEMY_RIM_BACK, ENEMY_WASH),
        Hue::Violet => (RIVAL_RIM, RIVAL_RIM_BACK, RIVAL_WASH),
        Hue::Jade => (JADE_RIM, JADE_RIM_BACK, JADE_WASH),
        Hue::Pink => (PINK_RIM, PINK_RIM_BACK, PINK_WASH),
    }
}
/// Braces: dark struts with bone caps, the same for both sides.
pub const BRACE: Color = [30, 34, 36, 255];
pub const BRACE_SHADE: Color = [66, 54, 42, 200];
pub const BRACE_CAP: Color = [228, 214, 174, 255];
pub const SHIELD_LINE: Color = [236, 190, 92, 255];

/// The plate's half-width and half-height for a machine: a little wider
/// than the ground shadow baked into its sprite.
pub fn plate_size(kind: Kind) -> (i32, i32) {
    match kind {
        Kind::Bulwark | Kind::Loom | Kind::Caisson | Kind::Barge | Kind::Lifter | Kind::Dredger => {
            (21, 10)
        }
        Kind::Hook | Kind::Wick => (13, 6),
        _ => (17, 8),
    }
}

/// Which part of a ground mark to paint.  The back goes down before the
/// machine's sprite; the front goes over the sprite's baked shadow and its
/// feet, as a mark on the ground in front of it would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Back,
    Front,
}

/// An enemy machine's plate, centred on its ground point.
pub fn enemy_plate(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    kind: Kind,
    hue: crate::seats::Hue,
    part: Part,
) {
    let (rim, rim_back, wash) = plate_tone(hue);
    let (rx, ry) = plate_size(kind);
    let outer = i64::from(rx * rx) * i64::from(ry * ry);
    let (ix, iy) = (rx - 3, ry - 2);
    let inner = i64::from(ix * ix) * i64::from(iy * iy);
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let o =
                i64::from(dx * dx) * i64::from(ry * ry) + i64::from(dy * dy) * i64::from(rx * rx);
            if o > outer {
                continue;
            }
            let i =
                i64::from(dx * dx) * i64::from(iy * iy) + i64::from(dy * dy) * i64::from(ix * ix);
            let color = match (i <= inner, dy < 0, part) {
                (true, _, Part::Back) => wash,
                (false, true, Part::Back) => rim_back,
                (false, false, Part::Front) => rim,
                _ => continue,
            };
            canvas.pixel(x + dx, y + dy, color);
        }
    }
}

/// How far a Bulwark, Loom or Caisson has braced, in twelfths: 0 packed,
/// 12 deployed, in between while its legs go down or come up.
pub fn brace_progress(entity: &bw_sim::Entity) -> u32 {
    if !crate::controls::is_specialist(entity.kind) {
        return 0;
    }
    let total = crate::presentation::DEPLOY_TICKS.max(1);
    if entity.deploy_remaining > 0 {
        let left = entity.deploy_remaining.min(total);
        if entity.deploy_target {
            (total - left) * 12 / total
        } else {
            left * 12 / total
        }
    } else if entity.deployed {
        12
    } else {
        0
    }
}

/// Braces driven into the ground around the machine, clear of its body;
/// `progress` twelfths of their length.  Caps show once they are down.
pub fn braces(canvas: &mut Canvas, x: i32, y: i32, progress: u32, part: Part) {
    if progress == 0 {
        return;
    }
    let p = progress.min(12) as i32;
    let struts: &[(i32, i32, i32, i32)] = match part {
        Part::Back => &[(-8, -5, -14, -10), (8, -5, 14, -10)],
        Part::Front => &[
            (-13, 1, -25, 1),
            (13, 1, 25, 1),
            (-7, 7, -13, 12),
            (7, 7, 13, 12),
        ],
    };
    for &(ox, oy, ex, ey) in struts {
        let tx = ox + (ex - ox) * p / 12;
        let ty = oy + (ey - oy) * p / 12;
        canvas.line(x + ox, y + oy + 1, x + tx, y + ty + 1, BRACE_SHADE);
        canvas.line(x + ox, y + oy, x + tx, y + ty, BRACE);
        if p == 12 {
            // A driven stake: a bone cap with a dark foot.
            canvas.rect(x + tx - 1, y + ty - 1, 3, 2, BRACE_CAP);
            canvas.rect(x + tx - 1, y + ty + 1, 3, 1, BRACE);
        }
    }
}

/// A Bulwark's shield line: the armoured front laid on the ground between
/// two projected points.
pub fn shield_line(canvas: &mut Canvas, a: (i32, i32), b: (i32, i32)) {
    canvas.line(a.0, a.1 + 1, b.0, b.1 + 1, BRACE);
    canvas.line(a.0, a.1, b.0, b.1, SHIELD_LINE);
    canvas.line(a.0, a.1 - 1, b.0, b.1 - 1, SHIELD_LINE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_grow_with_deployment_and_packed_machines_show_none() {
        let world = bw_sim::World::new(3, bw_core::Faction::Union);
        let mut e = world
            .entities
            .iter()
            .find(|e| e.kind == Kind::Hook)
            .cloned()
            .unwrap();
        e.kind = Kind::Bulwark;
        assert_eq!(brace_progress(&e), 0);
        e.deployed = true;
        assert_eq!(brace_progress(&e), 12);
        e.deployed = false;
        e.deploy_target = true;
        e.deploy_remaining = crate::presentation::DEPLOY_TICKS / 2;
        assert_eq!(brace_progress(&e), 6);
        e.deploy_target = false;
        assert_eq!(brace_progress(&e), 6);
        e.kind = Kind::Riveter;
        assert_eq!(brace_progress(&e), 0);
    }

    #[test]
    fn the_enemy_plate_is_red_in_front_and_leaves_the_centre_washed() {
        let mut canvas = Canvas::default();
        canvas.clear([0, 0, 0, 255]);
        enemy_plate(
            &mut canvas,
            100,
            100,
            Kind::Riveter,
            crate::seats::Hue::Red,
            Part::Back,
        );
        assert_eq!(canvas.get(100, 108), Some([0, 0, 0, 255]));
        enemy_plate(
            &mut canvas,
            100,
            100,
            Kind::Riveter,
            crate::seats::Hue::Red,
            Part::Front,
        );
        assert_eq!(canvas.get(100, 108), Some(ENEMY_RIM));
        assert_eq!(canvas.get(100, 92), Some(ENEMY_RIM_BACK));
        let centre = canvas.get(100, 100).unwrap();
        assert!(centre[0] > 0 && centre[0] < 100, "{centre:?}");
    }
}
