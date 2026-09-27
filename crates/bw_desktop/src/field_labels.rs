//! Words on the field, asked for rather than shown.
//!
//! The WELL and SALVAGE plates covered the art they named and grew with the
//! world zoom: at 3x one plate was as wide as a building.  Things on the
//! field now carry a small mark at rest (a few gold pips under a salvage
//! heap, a red badge on a stalled site, a count at a held mouth) and say
//! their words only under the pointer, in a tooltip drawn at the interface's
//! own scale.  This module groups the wrecks, sizes their marks and picks the
//! tip under the pointer.  Presentation only.

use crate::canvas::{Canvas, Color, GOLD, MUTED};
use crate::hover_card::{Chip, HoverCard, Mark};

/// What a wreck says about itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WreckClass {
    /// Seen (or sonar-known) salvage.
    Salvage,
    /// A wreck the tide covers: no worker can reach it.
    Drowned,
    /// A spent wreck.
    Empty,
    /// A wreck under the fog: its amount is unknown.
    Unseen,
}

/// One wreck on the field, in the picture's coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WreckMark {
    pub cell: (i32, i32),
    /// The wreck's ground point on the canvas.
    pub at: (i32, i32),
    pub class: WreckClass,
    pub amount: u32,
}

/// A rectangle on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Area {
    /// The rectangle `left`, `up`, `right` and `down` pixels around a point.
    pub fn around((x, y): (i32, i32), left: i32, up: i32, right: i32, down: i32) -> Area {
        Area {
            x: x - left,
            y: y - up,
            w: left + right + 1,
            h: up + down + 1,
        }
    }
    pub fn contains(self, (x, y): (i32, i32)) -> bool {
        (self.x..self.x + self.w).contains(&x) && (self.y..self.y + self.h).contains(&y)
    }
}

/// The words one thing on the field says under the pointer: the command
/// card's hover card, sized to its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldTip {
    /// Where the pointer asks for it, on the scene.
    pub area: Area,
    /// The point the tip stands over: the top of the thing it names.
    pub anchor: (i32, i32),
    pub card: HoverCard,
}

impl FieldTip {
    pub fn new(area: Area, anchor: (i32, i32), title: impl Into<String>, accent: Color) -> Self {
        Self {
            area,
            anchor,
            card: HoverCard {
                title: title.into(),
                accent: Some(accent),
                ..HoverCard::default()
            },
        }
    }
    /// The short role under the name, in jade.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.card.tag = Some(tag.into());
        self
    }
    /// The interface icon beside the name.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.card.icon = Some(icon.into());
        self
    }
    /// A warning line, in gold.
    pub fn warn(mut self, line: impl Into<String>) -> Self {
        self.card.reason = Some(line.into());
        self
    }
    /// A line of what it is for, in white.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.card.body.push(line.into());
        self
    }
    /// A mark and its number, as the card shows a price.
    pub fn chip(mut self, mark: Mark, value: impl Into<String>) -> Self {
        self.card.costs.push(Chip {
            mark,
            value: value.into(),
            short: false,
        });
        self
    }
}

/// The tip under the pointer: the smallest area that holds it, so a mark
/// inside a wider ring (a wreck at a mouth) answers for itself.
pub fn pick(tips: &[FieldTip], pointer: (i32, i32)) -> Option<&FieldTip> {
    tips.iter()
        .filter(|t| t.area.contains(pointer))
        .min_by_key(|t| i64::from(t.area.w) * i64::from(t.area.h))
}

/// Wrecks within this many cells (on both axes) are one heap.
pub const MERGE_CELLS: i32 = 2;
/// How far around a wreck's ground point the pointer names it, in pixels.
pub const HOVER_REACH: i32 = 18;

pub const DROWNED_COLOR: Color = [145, 204, 208, 255];

/// A heap of wrecks: one mark and one tip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WreckHeap {
    pub class: WreckClass,
    pub count: u32,
    pub total: u32,
    /// Below the lowest wreck of the heap, where the wreck's own art ends.
    pub foot: (i32, i32),
    pub area: Area,
}

impl WreckHeap {
    /// The heap's name: "WRECK", or "3 WRECKS".
    pub fn words(&self) -> String {
        match self.count {
            1 => "WRECK".into(),
            n => format!("{n} WRECKS"),
        }
    }
    pub fn color(&self) -> Color {
        match self.class {
            WreckClass::Salvage => GOLD,
            WreckClass::Drowned => DROWNED_COLOR,
            WreckClass::Empty | WreckClass::Unseen => MUTED,
        }
    }
    pub fn tip(&self) -> FieldTip {
        let anchor = (self.area.x + self.area.w / 2, self.area.y);
        let tip = FieldTip::new(self.area, anchor, self.words(), self.color());
        match self.class {
            WreckClass::Salvage => tip.chip(Mark::Salvage, self.total.to_string()),
            WreckClass::Drowned => tip
                .chip(Mark::Salvage, self.total.to_string())
                .warn("UNDER WATER: WAIT FOR THE TIDE"),
            WreckClass::Empty => tip.tag("SPENT"),
            WreckClass::Unseen => tip.tag("IN THE FOG"),
        }
    }
}

/// How many pips a salvage heap shows: one, two or three by its size.
pub fn salvage_pips(total: u32) -> i32 {
    match total {
        0 => 0,
        1..200 => 1,
        200..800 => 2,
        _ => 3,
    }
}

/// Merge the wrecks into heaps.  Single-link groups of one class: a chain
/// of wrecks two cells apart is one heap on the ground.
pub fn heaps(marks: &[WreckMark]) -> Vec<WreckHeap> {
    let mut parent: Vec<usize> = (0..marks.len()).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        let mut i = i;
        while parent[i] != root {
            let next = parent[i];
            parent[i] = root;
            i = next;
        }
        root
    }
    for i in 0..marks.len() {
        for j in i + 1..marks.len() {
            let (a, b) = (marks[i], marks[j]);
            if a.class == b.class
                && (a.cell.0 - b.cell.0).abs() <= MERGE_CELLS
                && (a.cell.1 - b.cell.1).abs() <= MERGE_CELLS
            {
                let (ra, rb) = (find(&mut parent, i), find(&mut parent, j));
                if ra != rb {
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut groups: Vec<Vec<WreckMark>> = Vec::new();
    let mut root_group: Vec<Option<usize>> = vec![None; marks.len()];
    for (i, mark) in marks.iter().enumerate() {
        let root = find(&mut parent, i);
        match root_group[root] {
            Some(g) => groups[g].push(*mark),
            None => {
                root_group[root] = Some(groups.len());
                groups.push(vec![*mark]);
            }
        }
    }
    groups
        .into_iter()
        .map(|members| {
            let count = members.len() as i32;
            let cx = members.iter().map(|m| m.at.0).sum::<i32>() / count;
            let bottom = members.iter().map(|m| m.at.1).max().unwrap_or(0);
            let left = members.iter().map(|m| m.at.0).min().unwrap_or(0);
            let right = members.iter().map(|m| m.at.0).max().unwrap_or(0);
            let top = members.iter().map(|m| m.at.1).min().unwrap_or(0);
            WreckHeap {
                class: members[0].class,
                count: count as u32,
                total: members.iter().map(|m| m.amount).sum(),
                foot: (cx, bottom + 6),
                area: Area {
                    x: left - HOVER_REACH,
                    y: top - HOVER_REACH - 8,
                    w: right - left + HOVER_REACH * 2 + 1,
                    h: bottom - top + HOVER_REACH * 2 + 9,
                },
            }
        })
        .collect()
}

const MARK_INK: Color = [17, 35, 46, 230];

/// The heap's mark at rest: gold pips for salvage, a small wave for a
/// drowned heap, nothing for a spent or unseen one (the wreck's art is
/// its own mark).
pub fn draw_heap_mark(canvas: &mut Canvas, heap: &WreckHeap) {
    let (cx, y) = heap.foot;
    match heap.class {
        WreckClass::Salvage => {
            let n = salvage_pips(heap.total);
            if n == 0 {
                return;
            }
            let w = n * 4 + 1;
            let x = cx - w / 2;
            canvas.rect(x, y, w, 4, MARK_INK);
            for i in 0..n {
                canvas.rect(x + 1 + i * 4, y + 1, 3, 2, GOLD);
            }
        }
        WreckClass::Drowned => {
            canvas.rect(cx - 5, y, 11, 5, MARK_INK);
            for (dx, dy) in [
                (-4, 2),
                (-3, 1),
                (-2, 1),
                (-1, 2),
                (0, 3),
                (1, 2),
                (2, 1),
                (3, 1),
                (4, 2),
            ] {
                canvas.pixel(cx + dx, y + dy, DROWNED_COLOR);
            }
        }
        WreckClass::Empty | WreckClass::Unseen => {}
    }
}

/// A compact count on the field (a hold's seconds), in the small face on
/// a dark plate: a third of a readable plate's width.
pub fn draw_badge(canvas: &mut Canvas, text: &str, cx: i32, y: i32, color: Color) {
    let w = text.chars().count() as i32 * 6 + 3;
    let height = canvas.height() as i32;
    if y - 2 < 0 || y + 7 > height {
        return;
    }
    let x = (cx - w / 2).clamp(0, (canvas.width() as i32 - w).max(0));
    canvas.rect(x, y - 2, w, 11, MARK_INK);
    canvas.text(text, x + 2, y, color);
}

/// The card's width in interface columns: as wide as its longest row.
pub(crate) fn card_width(card: &HoverCard, has_icon: bool) -> i32 {
    use crate::hover_card::{LINE_CHARS, wrap};
    let chars = |s: &str| s.chars().count() as i32;
    let head = if has_icon { 28 } else { 0 }
        + (chars(&card.title) * 8).max(card.tag.as_deref().map_or(0, |t| chars(t) * 6))
        + 2;
    let marks = card
        .costs
        .iter()
        .chain(&card.stats)
        .map(|c| 10 + chars(&c.value) * 6 + 7)
        .sum::<i32>()
        - 7;
    let prose = card
        .reason
        .iter()
        .flat_map(|r| wrap(r, LINE_CHARS))
        .chain(card.body.iter().cloned())
        .chain(card.note.iter().cloned())
        .map(|l| chars(&l) * 6)
        .max()
        .unwrap_or(0);
    head.max(marks).max(prose) + 2 * 7 + 2
}

impl crate::game::Game {
    /// The words of the thing under the pointer, in the command card's
    /// hover card at the interface's scale, so they read the same at every
    /// world zoom.  It stands over the thing and never under the pointer.
    pub(crate) fn draw_field_tip(&mut self) {
        let Some(mut tip) = self.hover_tip.clone() else {
            return;
        };
        let has_icon = tip.card.icon.as_ref().is_some_and(|k| {
            self.atlas
                .as_ref()
                .is_some_and(|a| a.sprites.contains_key(k))
        });
        if !has_icon {
            tip.card.icon = None;
        }
        let s = self.ui_scale().max(1);
        let view = self.world_view();
        let w = card_width(&tip.card, has_icon) * s;
        let h = Self::hover_card_rows(&tip.card) * s;
        let (ax, ay) = tip.anchor;
        let (px, py) = self.cursor;
        let clamp_x = |x: i32| x.clamp(4 * s, (view.width - w - 4 * s).max(4 * s));
        let clamp_y = |y: i32| y.clamp(view.top + 4 * s, (view.bottom - h - 4 * s).max(view.top));
        let covers = |x: i32, y: i32| (x..x + w).contains(&px) && (y..y + h).contains(&py);
        // Over the thing when there is room; otherwise beside the pointer,
        // on whichever side has room, so the card never hides what it names
        // or the pointer asking.
        let over = (clamp_x(ax - w / 2), ay - h - 6 * s);
        let (x, y) = if over.1 >= view.top + 4 * s && !covers(over.0, over.1) {
            over
        } else {
            let right = px + 18 * s;
            let x = if right + w <= view.width - 4 * s {
                right
            } else {
                px - 18 * s - w
            };
            (clamp_x(x), clamp_y(py - h / 2))
        };
        let y = clamp_y(y);
        self.draw_hover_card(crate::native_ui::Rect { x, y, w, h }, &tip.card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn salvage(cell: (i32, i32), amount: u32) -> WreckMark {
        WreckMark {
            cell,
            at: ((cell.0 - cell.1) * 16 + 320, (cell.0 + cell.1) * 8 - 500),
            class: WreckClass::Salvage,
            amount,
        }
    }

    #[test]
    fn a_heap_of_machine_wrecks_is_one_heap_with_its_total() {
        // Eight Bulwark wrecks at a mouth, as at 17:13 in the eighth trial.
        let marks: Vec<_> = (0..8)
            .map(|i| salvage((50 + i % 3, 49 + i / 3), 70))
            .collect();
        let heaps = heaps(&marks);
        assert_eq!(heaps.len(), 1, "{heaps:?}");
        assert_eq!(heaps[0].words(), "8 WRECKS");
        assert_eq!(heaps[0].tip().card.costs[0].value, "560");
        assert_eq!(salvage_pips(heaps[0].total), 2);
    }

    #[test]
    fn wrecks_three_cells_apart_are_their_own_heaps() {
        let marks = [salvage((40, 40), 900), salvage((43, 40), 900)];
        assert_eq!(heaps(&marks).len(), 2);
    }

    #[test]
    fn the_pointer_on_a_wreck_asks_for_its_words() {
        let marks = [salvage((40, 40), 35), salvage((46, 40), 2400)];
        let tips: Vec<_> = heaps(&marks).iter().map(WreckHeap::tip).collect();
        let small = pick(&tips, marks[0].at).unwrap();
        assert_eq!(small.card.costs[0].value, "35");
        assert_eq!(
            pick(&tips, marks[1].at).unwrap().card.costs[0].value,
            "2400"
        );
        assert!(pick(&tips, (marks[0].at.0, marks[0].at.1 + 60)).is_none());
    }

    #[test]
    fn the_smallest_area_under_the_pointer_answers() {
        let ring = FieldTip::new(
            Area::around((100, 100), 60, 30, 60, 30),
            (100, 70),
            "NW",
            GOLD,
        );
        let wreck = FieldTip::new(
            Area::around((110, 100), 10, 10, 10, 10),
            (110, 90),
            "W",
            GOLD,
        );
        let tips = [ring, wreck];
        assert_eq!(pick(&tips, (110, 100)).unwrap().card.title, "W");
        assert_eq!(pick(&tips, (60, 100)).unwrap().card.title, "NW");
    }

    #[test]
    fn spent_and_fogged_heaps_draw_no_mark() {
        let mut canvas = Canvas::new(64, 64);
        let before = canvas.pixels.clone();
        for class in [WreckClass::Empty, WreckClass::Unseen] {
            let heap = WreckHeap {
                class,
                count: 1,
                total: 0,
                foot: (32, 32),
                area: Area::around((32, 32), 8, 8, 8, 8),
            };
            draw_heap_mark(&mut canvas, &heap);
        }
        assert_eq!(canvas.pixels, before);
    }

    #[test]
    fn a_salvage_mark_stays_under_its_heap() {
        // Three pips on a plate thirteen pixels wide: the old plate for
        // SALVAGE 2400 was ninety-nine.
        let heap = &heaps(&[salvage((40, 30), 2400)])[0];
        let mut canvas = Canvas::new(640, 360);
        canvas.clear([0, 0, 0, 255]);
        draw_heap_mark(&mut canvas, heap);
        let lit: Vec<(i32, i32)> = (0..360)
            .flat_map(|y| (0..640).map(move |x| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y) != Some([0, 0, 0, 255]))
            .collect();
        let xs = lit.iter().map(|p| p.0);
        let width = xs.clone().max().unwrap() - xs.min().unwrap() + 1;
        assert!(width <= 13, "{width}");
        assert!(lit.iter().all(|p| p.1 >= heap.foot.1), "below the heap");
    }
}
