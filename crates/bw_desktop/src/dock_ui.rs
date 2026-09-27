//! Code-native command dock; uses existing fonts and integer UI scaling.
use crate::canvas::{EDGE, GOLD, JADE, MUTED, PANEL, WHITE};
use crate::console::ConsoleSelection;
use crate::dock_qol::{ProductionInfo, short_name};
use crate::game::{Action, Game};
use crate::lean_hud::{CELL, PITCH};
use crate::native_ui::{Rect, button, fit, fit_small, small, text};

impl Game {
    /// The selection as StarCraft shows it: a grid of icons, one per kind,
    /// each with its count.  A click keeps only that kind; a shift-click
    /// drops it.
    pub(crate) fn draw_selection_roster(&mut self, x: i32, w: i32, h: i32, s: i32) {
        let rows = self.selection_roster();
        let cols = (w / (PITCH * s)).clamp(1, 12) as usize;
        let per_page = cols * 2;
        let pages = rows.len().div_ceil(per_page).max(1);
        self.ux.roster_page = self.ux.roster_page.min(pages - 1);
        let page = self.ux.roster_page;
        let count: usize = rows.iter().map(|r| r.count).sum();
        let summary = self.selected_queue_summary();
        let header = if summary.is_empty() {
            format!("{count} SELECTED")
        } else {
            format!("{count} SELECTED / {summary}")
        };
        let room = if pages > 1 { w - 40 * s } else { w };
        small(
            &mut self.canvas,
            &fit_small(&header, room, s),
            x,
            h - 72 * s,
            JADE,
            s,
        );
        if pages > 1 {
            for (next, offset, label, enabled) in [
                (false, 34, "<", page > 0),
                (true, 17, ">", page + 1 < pages),
            ] {
                self.buttons.push(button(
                    Rect {
                        x: x + w - offset * s,
                        y: h - 75 * s,
                        w: 15 * s,
                        h: 12 * s,
                    },
                    label,
                    "",
                    Action::RosterPage(next),
                    enabled,
                ));
            }
        }
        let mixed = rows.len() > 1;
        for (i, row) in rows.iter().skip(page * per_page).take(per_page).enumerate() {
            let (cx, cy) = (
                x + (i % cols) as i32 * PITCH * s,
                h - (62 - (i / cols) as i32 * PITCH) * s,
            );
            self.buttons.push(button(
                Rect {
                    x: cx,
                    y: cy,
                    w: CELL * s,
                    h: CELL * s,
                },
                &format!("{} {}", short_name(row.kind), row.count),
                "",
                Action::FilterSelection(row.kind, false),
                true,
            ));
            // The "-" that removes the type, in the cell's corner.
            if mixed {
                self.push_remove_control(row.kind, cx, cy, CELL * s);
            }
        }
    }

    /// A producer's panel: its name and hull, then the queue as icons with
    /// the running one first and framed in gold, its progress under the
    /// row, and one line of state.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_production_panel(
        &mut self,
        sel: &ConsoleSelection,
        info: &ProductionInfo,
        x: i32,
        w: i32,
        h: i32,
        s: i32,
    ) {
        // The building's icon at twice its size, as a machine shows its
        // portrait; a wide dock keeps the rest together beside it.
        let mut x = x;
        let mut w = w.min(360 * s);
        let building = self
            .selected
            .first()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
            .map(|e| (e.kind, self.world.players[e.owner as usize].faction));
        if let Some((kind, faction)) = building
            && w > 200 * s
        {
            self.canvas.rect(x, h - 72 * s, 56 * s, 57 * s, PANEL);
            if let Some(atlas) = &self.atlas {
                atlas.draw_scaled(
                    &mut self.canvas,
                    &format!("ui_build_{}", kind.asset(faction)),
                    x + 4 * s,
                    h - 68 * s,
                    false,
                    crate::occlusion::PixelScale {
                        numerator: (2 * s) as u16,
                        denominator: 1,
                    },
                );
            }
            x += 62 * s;
            w -= 62 * s;
        }
        let hull = format!("{}/{}", sel.hp, sel.max_hp);
        let hull_w = (hull.len() as i32 * 6 + 10) * s;
        text(
            &mut self.canvas,
            &fit(&sel.title, w - hull_w - 6 * s, s),
            x,
            h - 73 * s,
            WHITE,
            s,
        );
        let hull = [crate::hover_card::Chip {
            mark: crate::hover_card::Mark::Hull,
            value: hull,
            short: false,
        }];
        self.draw_chips(&hull, x + w - hull_w, h - 72 * s, true);
        let slots = ((w / (PITCH * s)) as usize).clamp(1, 6);
        if info.queued.is_empty() {
            small(&mut self.canvas, &info.heading, x, h - 58 * s, MUTED, s);
        }
        for (i, kind) in info.queued.iter().take(slots).enumerate() {
            let cx = x + i as i32 * PITCH * s;
            let cy = h - 60 * s;
            self.canvas.rect(cx, cy, CELL * s, CELL * s, PANEL);
            if !self.draw_kind_icon(*kind, cx + s, cy + s) {
                small(
                    &mut self.canvas,
                    &fit_small(short_name(*kind), CELL * s, s),
                    cx + s,
                    cy + 9 * s,
                    WHITE,
                    s,
                );
            }
            self.canvas
                .frame(cx, cy, CELL * s, CELL * s, if i == 0 { GOLD } else { EDGE });
        }
        if info.queued.len() > slots {
            small(
                &mut self.canvas,
                &format!("+{}", info.queued.len() - slots),
                x + slots as i32 * PITCH * s,
                h - 51 * s,
                MUTED,
                s,
            );
        }
        self.canvas.rect(x, h - 30 * s, w, 3 * s, PANEL);
        self.canvas.rect(
            x,
            h - 30 * s,
            (i64::from(w) * i64::from(info.fraction.0) / i64::from(info.fraction.1)) as i32,
            3 * s,
            GOLD,
        );
        small(
            &mut self.canvas,
            &fit_small(&info.status, w, s),
            x,
            h - 23 * s,
            GOLD,
            s,
        );
    }
}
