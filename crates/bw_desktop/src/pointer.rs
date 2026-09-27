//! The pointer's shapes.  The pointer is the main feedback of an order: it
//! turns red over an enemy, becomes a crosshair while attack-moving, carries
//! a hammer while placing, and points along the edge the view scrolls.
//!
//! Provenance: authored by hand as the character maps below (one character
//! per pixel) in the root session of the menu review, 2026-09-23.  The maps
//! are the editable source; `image` scales them by whole pixels only.

use crate::canvas::{Canvas, Color};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerKind {
    /// Nothing under the pointer answers a click.
    Arrow,
    /// An own machine or building: a click selects it.
    Select,
    /// An enemy with nothing selected that could attack it.
    Enemy,
    /// Attack-move is armed: the next click picks a destination.
    AttackMove,
    /// A click attacks what is under the pointer.
    Attack,
    /// A click gathers, repairs, boards or faces: any non-combat order.
    Interact,
    /// A site is being placed and this one is good.
    Build,
    /// This click would be refused (a site that cannot be built).
    Refuse,
    /// The view scrolls toward this edge: 0 north, then clockwise to 7.
    Edge(u8),
}

impl PointerKind {
    pub fn name(self) -> String {
        match self {
            PointerKind::Arrow => "arrow".into(),
            PointerKind::Select => "select".into(),
            PointerKind::Enemy => "enemy".into(),
            PointerKind::AttackMove => "attack-move".into(),
            PointerKind::Attack => "attack".into(),
            PointerKind::Interact => "interact".into(),
            PointerKind::Build => "build".into(),
            PointerKind::Refuse => "refuse".into(),
            PointerKind::Edge(d) => format!(
                "edge-{}",
                ["n", "ne", "e", "se", "s", "sw", "w", "nw"][d as usize % 8]
            ),
        }
    }

    /// The edge a scroll delta points to, as the compass index `Edge` takes.
    pub fn edge(dx: i32, dy: i32) -> Option<PointerKind> {
        let d = match (dx.signum(), dy.signum()) {
            (0, -1) => 0,
            (1, -1) => 1,
            (1, 0) => 2,
            (1, 1) => 3,
            (0, 1) => 4,
            (-1, 1) => 5,
            (-1, 0) => 6,
            (-1, -1) => 7,
            _ => return None,
        };
        Some(PointerKind::Edge(d))
    }
}

const ARROW: &str = "
k...........
kk..........
kwk.........
kwwk........
kwwwk.......
kwwwwk......
kwwwwwk.....
kwwwwwwk....
kwwwwwwwk...
kwwwwwwssk..
kwwwwwkkkkk.
kwwkwwk.....
kwk.kwsk....
kk..kwsk....
k....kwsk...
.....kkk....
";

const CROSS: &str = "
......kkk......
......kgk......
......kgk......
......kgk......
......kgk......
......kkk......
kkkkkk...kkkkkk
kgggggk.kgggggk
kkkkkk...kkkkkk
......kkk......
......kgk......
......kgk......
......kgk......
......kgk......
......kkk......
";

/// A ground cell's diamond: the order lands on what is there.
const DIAMOND: &str = "
.......k.......
.....kkjkk.....
...kkjjkjjkk...
.kkjjkk.kkjjkk.
kjjkk.....kkjjk
.kkjjkk.kkjjkk.
...kkjjkjjkk...
.....kkjkk.....
.......k.......
";

/// The arrow with a hammer badge at its heel.
const BUILD: &str = "
k................
kk...............
kwk..............
kwwk.............
kwwwk............
kwwwwk...........
kwwwwwk..........
kwwwwwwk.........
kwwwwwwwk........
kwwwwwwssk.......
kwwwwwkkkkkkkkkkk
kwwkwwk...kgggggk
kwk.kwsk..kGGgGGk
kk..kwsk..kkkwkkk
k....kwsk...kwk..
.....kkk....kwk..
............kkk..
";

const REFUSE: &str = "
....kkkkk....
..kkrrrrrkk..
.krrkkkkkrrk.
.krk....krrk.
krk....krrkrk
krk...krrk.rk
krk..krrk..rk
krk.krrk...rk
krkkrrk....rk
.krrrk....krk
.krrkkkkkrrk.
..kkrrrrrkk..
....kkkkk....
";

const EDGE_N: &str = "
.....k.....
....kgk....
...kgggk...
..kgggggk..
.kgggggggk.
kgGGGGGGGgk
kkkkkkkkkkk
";

const EDGE_NE: &str = "
kkkkkkkkk
kggggggGk
.kgggggGk
..kggggGk
...kgggGk
....kggGk
.....kgGk
......kGk
.......kk
";

fn palette(c: char) -> Color {
    match c {
        'k' => [12, 18, 22, 255],
        'w' => [244, 236, 214, 255],
        's' => [190, 178, 150, 255],
        'g' => [250, 200, 118, 255],
        'G' => [196, 128, 48, 255],
        'r' => [236, 96, 72, 255],
        'R' => [150, 44, 34, 255],
        'j' => [120, 214, 168, 255],
        'J' => [52, 128, 98, 255],
        _ => [0, 0, 0, 0],
    }
}

/// One pointer picture: RGBA rows and the pixel that is the click point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointerImage {
    pub w: u32,
    pub h: u32,
    pub hot_x: u32,
    pub hot_y: u32,
    pub rgba: Vec<u8>,
}

impl PointerImage {
    fn from_map(map: &str, hot: (u32, u32), recolor: &[(char, char)]) -> Self {
        let rows: Vec<&str> = map.lines().filter(|l| !l.is_empty()).collect();
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(1) as u32;
        let h = rows.len() as u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for row in &rows {
            let mut n = 0;
            for c in row.chars() {
                let c = recolor
                    .iter()
                    .find(|(from, _)| *from == c)
                    .map_or(c, |(_, to)| *to);
                rgba.extend_from_slice(&palette(c));
                n += 1;
            }
            for _ in n..w {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
        PointerImage {
            w,
            h,
            hot_x: hot.0,
            hot_y: hot.1,
            rgba,
        }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        self.rgba[i..i + 4].try_into().unwrap()
    }

    /// A quarter turn clockwise; the click point turns with the picture.
    fn turned(&self) -> Self {
        let (w, h) = (self.h, self.w);
        let mut rgba = Vec::with_capacity(self.rgba.len());
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&self.at(y, self.h - 1 - x));
            }
        }
        PointerImage {
            w,
            h,
            hot_x: self.h - 1 - self.hot_y,
            hot_y: self.hot_x,
            rgba,
        }
    }

    /// Whole-pixel enlargement, never a resample.
    fn scaled(&self, scale: u32) -> Self {
        let s = scale.max(1);
        let (w, h) = (self.w * s, self.h * s);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&self.at(x / s, y / s));
            }
        }
        PointerImage {
            w,
            h,
            hot_x: self.hot_x * s + s / 2,
            hot_y: self.hot_y * s + s / 2,
            rgba,
        }
    }
}

/// The picture for `kind` at `scale` whole pixels per art pixel.
pub fn image(kind: PointerKind, scale: u32) -> PointerImage {
    let art = match kind {
        PointerKind::Arrow => PointerImage::from_map(ARROW, (0, 0), &[]),
        PointerKind::Select => PointerImage::from_map(ARROW, (0, 0), &[('w', 'j'), ('s', 'J')]),
        PointerKind::Enemy => PointerImage::from_map(ARROW, (0, 0), &[('w', 'r'), ('s', 'R')]),
        PointerKind::AttackMove => PointerImage::from_map(CROSS, (7, 7), &[]),
        PointerKind::Attack => PointerImage::from_map(CROSS, (7, 7), &[('g', 'r')]),
        PointerKind::Interact => PointerImage::from_map(DIAMOND, (7, 4), &[]),
        PointerKind::Build => PointerImage::from_map(BUILD, (0, 0), &[]),
        PointerKind::Refuse => PointerImage::from_map(REFUSE, (6, 6), &[]),
        PointerKind::Edge(d) => {
            let d = d % 8;
            let mut art = if d % 2 == 0 {
                PointerImage::from_map(EDGE_N, (5, 0), &[])
            } else {
                PointerImage::from_map(EDGE_NE, (8, 0), &[])
            };
            for _ in 0..d / 2 {
                art = art.turned();
            }
            art
        }
    };
    // The arrow's tip is its click point, so its enlargement keeps the
    // point at the tip's corner rather than a pixel's centre.
    let mut out = art.scaled(scale);
    if matches!(
        kind,
        PointerKind::Arrow | PointerKind::Select | PointerKind::Enemy | PointerKind::Build
    ) {
        out.hot_x = 0;
        out.hot_y = 0;
    }
    out
}

/// Draw the pointer into a picture, for captures that must show it.
pub fn draw(canvas: &mut Canvas, kind: PointerKind, x: i32, y: i32, scale: u32) {
    let img = image(kind, scale);
    for yy in 0..img.h {
        for xx in 0..img.w {
            let c = img.at(xx, yy);
            if c[3] != 0 {
                canvas.pixel(
                    x - img.hot_x as i32 + xx as i32,
                    y - img.hot_y as i32 + yy as i32,
                    c,
                );
            }
        }
    }
}

impl crate::game::Game {
    /// The pointer for where it rests now.  `edge` is the view's current
    /// edge-scroll direction, which wins over everything on the field.
    pub(crate) fn pointer_kind(&self, edge: (i32, i32)) -> PointerKind {
        use crate::game::{Mode, Screen};
        let live = self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review;
        if !live {
            return PointerKind::Arrow;
        }
        if let Some(kind) = PointerKind::edge(edge.0, edge.1) {
            return kind;
        }
        let (x, y) = self.cursor;
        if !self.world_pointer_allowed(x, y) {
            return PointerKind::Arrow;
        }
        let hovered = self
            .hovered
            .and_then(|id| self.world.entities.iter().find(|e| e.id == id));
        let enemy = hovered.is_some_and(|e| e.owner != 0);
        match self.mode {
            Mode::Build(kind) => {
                let origin = self.build_origin(kind, self.unproject(x, y));
                if self.placement_refusal(kind, origin).is_some() {
                    PointerKind::Refuse
                } else {
                    PointerKind::Build
                }
            }
            Mode::Attack if enemy => PointerKind::Attack,
            Mode::Attack => PointerKind::AttackMove,
            Mode::Gather | Mode::Face | Mode::Glint => PointerKind::Interact,
            Mode::Lay => {
                let target = self.unproject(x, y);
                let (cx, cy) = target.cell_xy();
                if self.world.map.terrain(cx, cy).tidal_arm().is_some() {
                    PointerKind::Interact
                } else {
                    PointerKind::Refuse
                }
            }
            Mode::Context => {
                if !self.observing()
                    && let Some((_, verb, _)) = self.context_order(x, y, false)
                {
                    match verb {
                        "Attack" => return PointerKind::Attack,
                        "Move" | "Rally" => {}
                        _ => return PointerKind::Interact,
                    }
                }
                match hovered {
                    Some(_) if enemy => PointerKind::Enemy,
                    Some(_) => PointerKind::Select,
                    None => PointerKind::Arrow,
                }
            }
        }
    }

    /// Whole pixels per pointer art pixel: the interface's own scale.
    pub(crate) fn pointer_scale(&self) -> u32 {
        self.ui_scale().clamp(1, 4) as u32
    }

    /// What the pointer rests on, for the hover ring and the pointer.
    pub(crate) fn refresh_hover(&mut self) {
        use crate::game::{Mode, Screen};
        let live = self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review;
        let (x, y) = self.cursor;
        self.follow_tide_card_hover();
        self.hovered = if live
            && matches!(self.mode, Mode::Context | Mode::Attack)
            && self.drag.is_none()
            && self.world_pointer_allowed(x, y)
        {
            self.unit_at(x, y)
        } else {
            None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pointer_is_whole_and_clicks_on_its_own_pixels() {
        let mut kinds = vec![
            PointerKind::Arrow,
            PointerKind::Select,
            PointerKind::Enemy,
            PointerKind::AttackMove,
            PointerKind::Attack,
            PointerKind::Interact,
            PointerKind::Build,
            PointerKind::Refuse,
        ];
        kinds.extend((0..8).map(PointerKind::Edge));
        for kind in kinds {
            for scale in 1..=4 {
                let img = image(kind, scale);
                assert_eq!(img.rgba.len(), (img.w * img.h * 4) as usize, "{kind:?}");
                assert!(img.hot_x < img.w && img.hot_y < img.h, "{kind:?}");
                // Small enough for every platform's cursor size limit.
                assert!(img.w <= 128 && img.h <= 128);
            }
        }
    }

    #[test]
    fn the_pointer_says_what_a_click_would_do() {
        use crate::game::{Game, Mode};
        use bw_core::{Faction, Kind, Pos};
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.faction = Faction::Union;
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        g.selected.clear();
        let own = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(30, 60));
        let foe = g.world.spawn_for_tests(1, Kind::Riveter, Pos::cell(32, 60));
        g.tick();
        g.tick();
        g.camera.center(Pos::cell(31, 60));
        g.render();
        let at = |g: &Game, id: u32| {
            let e = g.world.entities.iter().find(|e| e.id == id).unwrap();
            let (x, y) = g.project(e.pos);
            (x, y - 6)
        };
        g.cursor = at(&g, foe);
        g.render();
        assert_eq!(g.hovered, Some(foe));
        assert_eq!(g.pointer_kind((0, 0)), PointerKind::Enemy);
        g.selected = vec![own];
        g.render();
        assert_eq!(g.pointer_kind((0, 0)), PointerKind::Attack);
        g.cursor = at(&g, own);
        g.render();
        assert_eq!(g.hovered, Some(own));
        assert_eq!(g.pointer_kind((0, 0)), PointerKind::Select);
        g.mode = Mode::Attack;
        let (x, y) = g.project(Pos::cell(31, 64));
        g.cursor = (x, y);
        g.render();
        assert_eq!(g.pointer_kind((0, 0)), PointerKind::AttackMove);
        assert_eq!(g.pointer_kind((1, 0)), PointerKind::Edge(2));
    }

    #[test]
    fn edge_arrows_point_where_the_view_scrolls() {
        // North's tip is at the top; east's at the right; south's at the
        // bottom; west's at the left.
        let n = image(PointerKind::Edge(0), 1);
        assert_eq!(n.hot_y, 0);
        let e = image(PointerKind::Edge(2), 1);
        assert_eq!(e.hot_x, e.w - 1);
        let s = image(PointerKind::Edge(4), 1);
        assert_eq!(s.hot_y, s.h - 1);
        let w = image(PointerKind::Edge(6), 1);
        assert_eq!(w.hot_x, 0);
        assert_eq!(PointerKind::edge(1, 1), Some(PointerKind::Edge(3)));
        assert_eq!(PointerKind::edge(0, 0), None);
    }
}
