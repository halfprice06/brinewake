//! The lean match HUD (trial-9 feedback: the panels round the field were
//! too wordy and too big).  One short row across the top, a dock of 90
//! interface rows at the bottom, and a command card of icon buttons in the
//! StarCraft manner: the picture and its key on the button, the name, the
//! price and the effect in the tooltip.
use crate::canvas::{EDGE, GOLD, INK, JADE, MUTED, PANEL, RED, WHITE};
use crate::game::{Action, Button, Game};
use crate::native_ui::{Rect, button, fit_small, small, text};
use crate::occlusion::PixelScale;
use bw_core::{Faction, Kind};

/// The top bar's height in interface rows.
pub(crate) const TOP_BAR: i32 = 18;
/// The dock's height in interface rows: minimap, selection and card.
pub(crate) const DOCK: i32 = 90;
/// An icon button's side, one row of frame round a 24-texel icon.
pub(crate) const CELL: i32 = 26;
/// The pitch between icon buttons.
pub(crate) const PITCH: i32 = 28;
/// The command card's columns and rows.
pub(crate) const CARD_COLS: i32 = 5;
pub(crate) const CARD_ROWS: i32 = 3;
/// The card's width: five cells and four gaps.
pub(crate) const CARD_W: i32 = CARD_COLS * PITCH - (PITCH - CELL);
/// The selection panel's left edge: minimap, then the quick-select column.
pub(crate) const PANEL_X: i32 = 182;
/// The quick-select column (idle workers, army, Works).
pub(crate) const QUICK_X: i32 = 150;
const RAISED: [u8; 4] = [34, 53, 60, 255];

/// The icon an action shows on the command card, by atlas key.
pub(crate) fn icon_key(action: &Action, faction: Faction) -> Option<String> {
    Some(match action {
        Action::Train(k) | Action::TrainBatch(k) => format!("ui_unit_{}", k.asset(faction)),
        Action::Build(k) => format!("ui_build_{}", k.asset(faction)),
        Action::Upgrade(u) => format!("ui_up_{}", upgrade_slug(*u)),
        Action::Research(d) => match d {
            bw_content::Doctrine::Hauling => "ui_doc_hauling".into(),
            bw_content::Doctrine::FireControl => "ui_doc_fire_control".into(),
        },
        Action::FilterSelection(k, _) if k.is_building() => {
            format!("ui_build_{}", k.asset(faction))
        }
        Action::FilterSelection(k, _) => format!("ui_unit_{}", k.asset(faction)),
        Action::IdleWorker => format!("ui_unit_{}", faction.worker().asset(faction)),
        Action::SelectArmy => "ui_cmd_army".into(),
        Action::SelectWorks => format!("ui_build_{}", Kind::Works.asset(faction)),
        Action::Attack => "ui_cmd_attack".into(),
        Action::Stop => "ui_cmd_stop".into(),
        Action::Hold => "ui_cmd_hold".into(),
        Action::Capture => "ui_cmd_capture".into(),
        Action::Gather => "ui_cmd_gather".into(),
        // KEEP borrows DEPLOY's picture until it has its own; its chip
        // reads KEPT or OFF.
        Action::Deploy => "ui_cmd_deploy".into(),
        Action::KeepDeployed => "ui_cmd_keep".into(),
        Action::Pack => "ui_cmd_pack".into(),
        Action::Surge => "ui_cmd_surge".into(),
        Action::Unload => "ui_cmd_unload".into(),
        Action::Board => "ui_cmd_board".into(),
        Action::Sound => "ui_cmd_sound".into(),
        Action::Glint => "ui_cmd_glint".into(),
        Action::Lay => "ui_cmd_lay".into(),
        Action::Formation => "ui_cmd_formation_compact".into(),
        Action::Face => "ui_cmd_face".into(),
        Action::Vent => "ui_cmd_vent".into(),
        // RECYCLE has no icon of its own yet: RECLAIM's, turning something
        // into salvage, reads the same.
        Action::Reclaim | Action::Recycle => "ui_cmd_reclaim".into(),
        Action::Cancel | Action::CancelUpgrade | Action::CancelResearch => "ui_cmd_cancel".into(),
        _ => return None,
    })
}

pub(crate) fn upgrade_slug(upgrade: bw_content::Upgrade) -> &'static str {
    use bw_content::Upgrade;
    match upgrade {
        Upgrade::Plate => "plate",
        Upgrade::Siege => "siege",
        Upgrade::Temper => "temper",
        Upgrade::Tracks => "tracks",
        Upgrade::Refit => "refit",
        Upgrade::Cranes => "cranes",
        Upgrade::SalvageSonar => "salvage_sonar",
        Upgrade::ScrapRecovery => "scrap_recovery",
        Upgrade::Overpressure => "overpressure",
        Upgrade::BleedValves => "bleed_valves",
        // OVERHAUL (rules 21): stacked hull plates; the card marks its
        // level in code.
        Upgrade::Overhaul1 | Upgrade::Overhaul2 | Upgrade::Overhaul3 => "overhaul",
    }
}

/// The key a button answers to, as its corner chip and its hover card
/// show it: the quick column's own keys, else the key that leads the hint
/// ("Q 50S"), or the label when the hint is a word ("F COMPACT" over
/// FORMATION).
pub(crate) fn button_key(b: &Button) -> Option<String> {
    match b.action {
        Action::IdleWorker => return Some("I".into()),
        Action::SelectArmy => return Some("F2".into()),
        Action::SelectWorks => return Some("F4".into()),
        Action::FilterSelection(..) => return None,
        _ => {}
    }
    let (key, _) = crate::native_ui::button_rows(&b.hint, true);
    key.or_else(|| {
        let mut words = b.label.split_whitespace();
        let first = words.next()?;
        (first.chars().count() == 1 && words.next().is_some()).then(|| first.to_string())
    })
}

/// A fixed place on the card for each order, so a key's button never moves
/// with the selection: attack, hold, stop, capture and gather across the
/// top, the specialist orders under them, formation and facing below.
fn machine_slot(action: &Action) -> Option<usize> {
    Some(match action {
        Action::Attack => 0,
        Action::Hold => 1,
        Action::Stop => 2,
        Action::Capture => 3,
        Action::Gather => 4,
        Action::Deploy | Action::Pack => 5,
        Action::Surge => 6,
        // One seat never has both: SOUND is Union and Assembly, GLINT the
        // Compact's; the Compact has no transport to UNLOAD.
        Action::Sound | Action::Glint => 7,
        Action::Unload | Action::Lay => 8,
        Action::Board => 9,
        Action::Formation => 10,
        // KEEP sits on the bottom row under DEPLOY's column (trial 11).
        Action::KeepDeployed => 12,
        Action::Face => 11,
        _ => return None,
    })
}

/// The short state a card button carries under its icon, read from the
/// hint's second row: seconds left, a place in the queue, done.
pub(crate) fn state_chip(hint: &str) -> Option<(String, [u8; 4])> {
    let row = hint.split('\n').nth(1)?.trim();
    if let Some(n) = row.strip_suffix("S LEFT") {
        return Some((format!("{n}S"), GOLD));
    }
    if let Some(n) = row.strip_prefix("QUEUED ") {
        return Some((format!("Q{n}"), MUTED));
    }
    if let Some(n) = row.strip_prefix("OPEN ") {
        return Some((n.to_string(), JADE));
    }
    match row {
        // The KEEP toggle's state (trial 11).
        "KEPT" => Some(("KEPT".into(), JADE)),
        "OFF" => Some(("OFF".into(), MUTED)),
        "DONE" | "COMPLETE" => Some(("DONE".into(), JADE)),
        "TIER II" => Some(("II".into(), GOLD)),
        _ => None,
    }
}

impl Game {
    /// The command card's left edge.
    pub(crate) fn lean_card_x(&self) -> i32 {
        self.canvas.width() as i32 - (CARD_W + 6) * self.ui_scale()
    }
    /// Is this one of the dock's icon buttons (the card or the quick column)?
    pub(crate) fn icon_cell(&self, b: &Button) -> bool {
        let s = self.ui_scale();
        b.w == CELL * s && b.h == CELL * s && b.y >= self.world_view().bottom
    }
    /// Lay out the buttons pushed since `first` on the card's grid.  Machine
    /// orders keep fixed places; everything else fills in push order.
    pub(crate) fn place_command_card(&mut self, first: usize) {
        let s = self.ui_scale();
        let x0 = self.lean_card_x();
        let y0 = self.world_view().bottom + 4 * s;
        let fixed = self.buttons[first..]
            .iter()
            .all(|b| machine_slot(&b.action).is_some());
        // Otherwise the card reads by rows, as StarCraft's does: what the
        // selection makes on top, what it researches under that, its other
        // orders on the bottom row and CANCEL in the bottom-right corner.
        let trains = self.buttons[first..]
            .iter()
            .any(|b| matches!(b.action, Action::Train(_) | Action::Build(_)));
        let cells = (CARD_COLS * CARD_ROWS) as usize;
        let mut taken = [false; (CARD_COLS * CARD_ROWS) as usize];
        for b in self.buttons[first..].iter_mut() {
            let free = |from: usize, taken: &[bool]| {
                (from..cells)
                    .chain(0..from)
                    .find(|&i| !taken[i])
                    .unwrap_or(cells - 1)
            };
            let slot = if fixed {
                machine_slot(&b.action).unwrap_or(0)
            } else {
                match b.action {
                    Action::Cancel | Action::CancelUpgrade | Action::CancelResearch => {
                        (0..cells).rev().find(|&i| !taken[i]).unwrap_or(0)
                    }
                    Action::Train(_) | Action::TrainBatch(_) | Action::Build(_) => free(0, &taken),
                    Action::Research(_) | Action::Upgrade(_) => {
                        free(if trains { CARD_COLS as usize } else { 0 }, &taken)
                    }
                    _ => free(2 * CARD_COLS as usize, &taken),
                }
            };
            let slot = slot.min(cells - 1);
            taken[slot] = true;
            let (col, row) = (slot as i32 % CARD_COLS, slot as i32 / CARD_COLS);
            b.x = x0 + col * PITCH * s;
            b.y = y0 + row * PITCH * s;
            b.w = CELL * s;
            b.h = CELL * s;
        }
    }

    /// The one-row top bar: side, salvage, pressure, crew, clock; zoom, pace
    /// and menu at the right.
    pub(crate) fn draw_lean_top_bar(&mut self) {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let top = self.world_view().top;
        let state = self.console_state();
        self.canvas.rect(0, 0, w, TOP_BAR * s, INK);
        self.canvas.line(0, top - 1, w, top - 1, EDGE);
        let scale = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        let badge = match self.faction {
            Faction::Union => "faction_union_badge",
            Faction::Assembly => "faction_assembly_badge",
            Faction::Compact => "faction_compact_badge",
        };
        let drawn = self
            .atlas
            .as_ref()
            .is_some_and(|a| a.draw_scaled(&mut self.canvas, badge, 6 * s, 3 * s, false, scale));
        if !drawn {
            small(
                &mut self.canvas,
                &self.faction.name()[..1],
                8 * s,
                6 * s,
                WHITE,
                s,
            );
        }
        let r = &state.resources;
        let full = r.pressure_cap.is_some_and(|cap| r.pressure >= cap);
        let capped = r.crew >= r.crew_cap;
        let mut x = 30 * s;
        let fields = [
            (
                "icon_salvage",
                r.salvage.to_string(),
                GOLD,
                r.salvage_rate_per_minute.map(|m| format!("+{m}/M")),
            ),
            (
                "icon_pressure",
                r.pressure_cap
                    .map(|cap| format!("{}/{}", r.pressure, cap))
                    .unwrap_or_else(|| r.pressure.to_string()),
                if full { RED } else { JADE },
                if full {
                    None
                } else {
                    r.pressure_rate_per_minute.map(|m| format!("+{m}/M"))
                },
            ),
            (
                "icon_crew",
                format!("{}/{}", r.crew, r.crew_cap),
                if capped { RED } else { WHITE },
                None,
            ),
        ];
        for (icon, value, color, rate) in fields {
            if let Some(atlas) = self.atlas.as_ref() {
                atlas.draw_scaled(&mut self.canvas, icon, x, s, false, scale);
            }
            x += 18 * s;
            text(&mut self.canvas, &value, x, 5 * s, color, s);
            x += value.chars().count() as i32 * 8 * s + 3 * s;
            if let Some(rate) = rate {
                small(&mut self.canvas, &rate, x, 6 * s, MUTED, s);
                x += rate.chars().count() as i32 * 6 * s;
            }
            if icon == "icon_pressure" && full {
                // Full pressure is income thrown away: a pulsing key chip
                // for the sink a seat can use at once (R, Reclaim at the
                // HQ).  The words live in the notice and the Guide.
                let lit = (self.world.tick / 15).is_multiple_of(2);
                let (fill, ink) = if lit { (GOLD, INK) } else { (RED, WHITE) };
                self.canvas.rect(x, 4 * s, 9 * s, 11 * s, fill);
                small(&mut self.canvas, "R", x + 2 * s, 6 * s, ink, s);
                x += 12 * s;
            }
            x += 12 * s;
        }
        let clock = format!(
            "{:02}:{:02}",
            state.elapsed_seconds / 60,
            state.elapsed_seconds % 60
        );
        text(&mut self.canvas, &clock, x, 5 * s, WHITE, s);
        // At the right: zoom out and in, the pace as play arrows, MENU.
        // Online the pace is the shared one, and its place shows the link.
        let online = self.session.is_some();
        let zoom_x = w - if online { 110 } else { 98 } * s;
        self.draw_tide_gauge(x + clock.len() as i32 * 8 * s, zoom_x);
        let zoom_goal = self.zoom_goal.map_or(self.zoom, |g| g.zoom);
        for (x, width, label, action, enabled) in [
            (
                zoom_x,
                14,
                "-".to_string(),
                Action::ZoomOut,
                zoom_goal != crate::zoom::Zoom::Overview,
            ),
            (
                zoom_x + 16 * s,
                14,
                "+".to_string(),
                Action::ZoomIn,
                zoom_goal != crate::zoom::Zoom::Detail,
            ),
            (
                w - 66 * s,
                24,
                self.shown_pace().label().to_string(),
                Action::CycleSpeed,
                !online,
            ),
            (w - 40 * s, 36, "MENU".to_string(), Action::Back, true),
        ] {
            if action == Action::CycleSpeed && online {
                continue;
            }
            self.buttons.push(button(
                Rect {
                    x,
                    y: 2 * s,
                    w: width * s,
                    h: 14 * s,
                },
                &label,
                "",
                action,
                enabled,
            ));
        }
    }

    /// The quick-select column beside the minimap: idle workers, the army,
    /// the Works.  Icons with their counts.
    pub(crate) fn push_quick_select(&mut self) {
        let s = self.ui_scale();
        let y0 = self.world_view().bottom + 4 * s;
        let workers = self
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && e.kind.is_worker())
            .count();
        for (i, (label, action, count)) in [
            ("IDLE", Action::IdleWorker, self.idle_worker_ids().len()),
            ("ARMY", Action::SelectArmy, self.army_ids().len()),
            ("WORKS", Action::SelectWorks, self.works_ids().len()),
        ]
        .into_iter()
        .enumerate()
        {
            // The IDLE button carries the worker count too: the only count
            // of an economy whose workers never read as idle.
            let text = if label == "IDLE" {
                format!("{label} {count}/{workers}")
            } else {
                format!("{label} {count}")
            };
            self.buttons.push(button(
                Rect {
                    x: QUICK_X * s,
                    y: y0 + i as i32 * PITCH * s,
                    w: CELL * s,
                    h: CELL * s,
                },
                &text,
                "",
                action,
                count > 0 || (label == "IDLE" && workers > 0),
            ));
        }
    }

    /// The control-group tabs along the top of the selection panel: only
    /// the groups that hold something, as StarCraft shows them.
    pub(crate) fn push_group_tabs(&mut self, right: i32) {
        let s = self.ui_scale();
        let y = self.world_view().bottom + 3 * s;
        let mut x = PANEL_X * s;
        for n in (1..10).chain(std::iter::once(0)) {
            let size = self.live_group(n).len();
            // The number, a rule, and the size: wider for two digits.
            let tw = if size >= 10 { 30 } else { 24 } * s;
            if size == 0 || x + tw > right {
                continue;
            }
            self.buttons.push(button(
                Rect {
                    x,
                    y,
                    w: tw,
                    h: 11 * s,
                },
                &format!("GROUP {n}"),
                "",
                Action::RecallGroup(n),
                true,
            ));
            x += tw + 2 * s;
        }
    }

    /// Draw a group tab: its number (gold while it is the selection), a
    /// rule, and its size, muted, so "1" and "3" never read as thirteen.
    pub(crate) fn draw_group_tab(&mut self, b: &Button, n: usize) {
        let s = self.ui_scale();
        let ids = self.live_group(n);
        let mut selected = self.gameplay_ids(|_| true);
        selected.sort_unstable();
        selected.dedup();
        let active = !ids.is_empty() && ids == selected;
        let hover = b.contains(self.cursor.0, self.cursor.1);
        self.canvas.rect(
            b.x,
            b.y,
            b.w,
            b.h,
            if active || hover { RAISED } else { PANEL },
        );
        small(
            &mut self.canvas,
            &n.to_string(),
            b.x + 3 * s,
            b.y + 2 * s,
            if active { GOLD } else { WHITE },
            s,
        );
        self.canvas
            .rect(b.x + 10 * s, b.y + 2 * s, s, b.h - 4 * s, EDGE);
        small(
            &mut self.canvas,
            &ids.len().to_string(),
            b.x + 13 * s,
            b.y + 2 * s,
            if active { WHITE } else { MUTED },
            s,
        );
        if active {
            self.canvas.rect(b.x, b.y + b.h - s, b.w, s, GOLD);
        }
    }

    /// Draw an icon cell: the frame, the icon (dimmed when refused), the key
    /// in the top right corner, and a state chip along the bottom.
    pub(crate) fn draw_icon_cell(&mut self, b: &Button, hover: bool, focus: bool) {
        let s = self.ui_scale();
        let lit = b.enabled && (hover || focus);
        self.canvas
            .rect(b.x, b.y, b.w, b.h, if lit { RAISED } else { PANEL });
        let key = if b.action == Action::Formation {
            // One picture per formation: the button shows the one set.
            let name = b.label.split_whitespace().last().unwrap_or("COMPACT");
            Some(match name {
                "LINE" => "ui_cmd_formation_line".to_string(),
                "LOOSE" => "ui_cmd_formation_loose".to_string(),
                _ => "ui_cmd_formation_compact".to_string(),
            })
        } else {
            icon_key(&b.action, self.faction)
        };
        let scale = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        let drawn = key.as_deref().is_some_and(|key| {
            self.atlas.as_ref().is_some_and(|atlas| {
                atlas.draw_scaled(&mut self.canvas, key, b.x + s, b.y + s, !b.enabled, scale)
            })
        });
        if !drawn {
            // No picture yet: the first word, small.
            let word = b.label.split_whitespace().next().unwrap_or("");
            small(
                &mut self.canvas,
                &fit_small(word, b.w - 2 * s, s),
                b.x + 2 * s,
                b.y + 9 * s,
                if b.enabled { WHITE } else { MUTED },
                s,
            );
        }
        let frame = if lit {
            GOLD
        } else if b.enabled {
            EDGE
        } else {
            PANEL
        };
        self.canvas.frame(b.x, b.y, b.w, b.h, frame);
        if focus && b.enabled {
            self.canvas.rect(b.x, b.y, 2 * s, b.h, GOLD);
        }
        // The quick column and the roster show counts, the card its key
        // and state.
        match b.action {
            Action::FilterSelection(..) => {
                let count = b.label.split_whitespace().last().unwrap_or("");
                self.chip(b, count, WHITE, false);
            }
            Action::IdleWorker | Action::SelectArmy | Action::SelectWorks => {
                let count = b.label.split_whitespace().nth(1).unwrap_or("0");
                let count = count.split('/').next().unwrap_or(count);
                self.chip(b, count, if b.enabled { WHITE } else { MUTED }, false);
                if let Some(key) = button_key(b) {
                    self.corner_key(b, &key);
                }
            }
            _ => {
                if let Some(key) = button_key(b) {
                    self.corner_key(b, &key);
                }
                if let Action::Upgrade(upgrade) = b.action
                    && let Some(level) = upgrade.overhaul_level()
                {
                    self.level_mark(b, level);
                }
                if let Some((chip, color)) = state_chip(&b.hint) {
                    self.chip(b, &chip, color, true);
                }
            }
        }
    }
    /// OVERHAUL's level in the top left corner, as the card's other marks
    /// are drawn: I, II or III in gold on ink.
    fn level_mark(&mut self, b: &Button, level: u8) {
        let s = self.ui_scale();
        let mark = ["I", "II", "III"][usize::from(level.clamp(1, 3)) - 1];
        let w = mark.len() as i32 * 6 * s + s;
        self.canvas.rect(b.x + s, b.y + s, w, 9 * s, INK);
        small(
            &mut self.canvas,
            mark,
            b.x + 2 * s,
            b.y + 2 * s,
            if b.enabled { GOLD } else { EDGE },
            s,
        );
    }
    fn corner_key(&mut self, b: &Button, key: &str) {
        let s = self.ui_scale();
        let w = key.chars().count() as i32 * 6 * s + s;
        let x = b.x + b.w - w - s;
        self.canvas.rect(x, b.y + s, w, 9 * s, INK);
        small(
            &mut self.canvas,
            key,
            x + s,
            b.y + 2 * s,
            if b.enabled { GOLD } else { EDGE },
            s,
        );
    }
    fn chip(&mut self, b: &Button, label: &str, color: [u8; 4], left: bool) {
        let s = self.ui_scale();
        let w = label.chars().count() as i32 * 6 * s + s;
        let x = if left { b.x + s } else { b.x + b.w - w - s };
        let y = b.y + b.h - 10 * s;
        self.canvas.rect(x, y, w, 9 * s, INK);
        small(&mut self.canvas, label, x + s, y + s, color, s);
    }

    /// The field prompt: a single small line just above the dock, over a
    /// strip of ink as wide as its words.  Hover hints read muted; a mode
    /// or a message reads white.
    pub(crate) fn draw_lean_prompt(&mut self) {
        let prompt = self.native_prompt();
        if prompt.is_empty() {
            return;
        }
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let v = self.world_view();
        let x = PANEL_X * s;
        let room = w - x - (CARD_W + 12) * s;
        // A long prompt wraps upward onto a second or third row rather than
        // losing its end to "..." (trial 10: "SURGE (Z) C...", "CHOOSE
        // CLE...").
        let lines = prompt_lines(&prompt, (room / (6 * s)).max(1) as usize);
        let quiet = self.mode == crate::game::Mode::Context && !self.message_live();
        let bottom = v.bottom - 12 * s;
        let count = lines.len() as i32;
        for (i, line) in lines.iter().enumerate() {
            let shown = fit_small(line, room, s);
            let width = shown.chars().count() as i32 * 6 * s + 5 * s;
            let y = bottom - (count - 1 - i as i32) * 10 * s;
            self.canvas.rect(x - 3 * s, y - 2 * s, width, 11 * s, INK);
            small(
                &mut self.canvas,
                &shown,
                x,
                y,
                if quiet { MUTED } else { WHITE },
                s,
            );
        }
    }

    /// The icon of a unit kind at the interface scale, for the roster and
    /// the production queue.  False when the atlas has no such icon.
    pub(crate) fn draw_kind_icon(&mut self, kind: Kind, x: i32, y: i32) -> bool {
        let s = self.ui_scale();
        let key = if kind.is_building() {
            format!("ui_build_{}", kind.asset(self.faction))
        } else {
            format!("ui_unit_{}", kind.asset(self.faction))
        };
        let scale = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        self.atlas
            .as_ref()
            .is_some_and(|atlas| atlas.draw_scaled(&mut self.canvas, &key, x, y, false, scale))
    }
}

/// The prompt row's words in rows of at most `chars` glyphs, three rows at
/// most; the third keeps whatever is left, for the caller to fit.
pub(crate) fn prompt_lines(prompt: &str, chars: usize) -> Vec<String> {
    let mut lines = crate::hover_card::wrap(prompt, chars);
    if lines.len() > 3 {
        let rest = lines.split_off(2).join(" ");
        lines.push(rest);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_prompt_wraps_instead_of_losing_its_end() {
        let prompt = "Enemy Looms: blind inside two cells. Surge (Z) closes the distance.";
        let lines = prompt_lines(prompt, 40);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines.iter().all(|l| l.chars().count() <= 40));
        assert_eq!(lines.join(" "), prompt);
        assert_eq!(prompt_lines("SHORT", 40), vec!["SHORT".to_string()]);
        assert!(prompt_lines(&"word ".repeat(60), 20).len() == 3);
    }

    #[test]
    fn state_chips_read_the_hint() {
        assert_eq!(state_chip("P\n12S LEFT").unwrap().0, "12S");
        assert_eq!(state_chip("P\nQUEUED 2").unwrap().0, "Q2");
        assert_eq!(state_chip("P\nDONE").unwrap().0, "DONE");
        assert_eq!(state_chip("X\nOPEN 9S").unwrap().0, "9S");
        assert!(state_chip("V 180S+30P").is_none());
    }
}
