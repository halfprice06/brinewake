//! Window-sized presentation. Root-authored code-native UI; existing bitmap
//! artwork is reused intact. World and interface have separate integer scales.
use crate::canvas::{Canvas, Color, EDGE, GOLD, INK, JADE, MUTED, PANEL, RED, WHITE};
use crate::game::{Action, Button, Game, Mode, Screen};
use crate::qol::hold_color;
use crate::seats::{arm_letter, seat_name};
use crate::zoom::WorldView;
use bw_core::{Kind, Pos};

const SURFACE: Color = [23, 38, 46, 255];
/// The hover card's width (hover_card.rs): it stands beside what it
/// explains instead of covering the selection panel and the chart.
pub(crate) const TIP_W: i32 = 200;
const RAISED: Color = [34, 53, 60, 255];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl Rect {
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

pub(crate) fn text(c: &mut Canvas, s: &str, x: i32, y: i32, color: Color, scale: i32) {
    c.text_readable_scaled(s, x, y, color, scale);
}
pub(crate) fn small(c: &mut Canvas, s: &str, x: i32, y: i32, color: Color, scale: i32) {
    c.text_scaled(s, x, y, color, scale);
}
/// A small-face line of " / " segments trimmed from the end until it fits:
/// a shorter true line beats a clipped one.
pub(crate) fn fit_segments(s: &str, width: i32, scale: i32) -> String {
    let n = (width / (6 * scale)).max(0) as usize;
    let mut parts: Vec<&str> = s.split(" / ").collect();
    while parts.len() > 1 && parts.join(" / ").chars().count() > n {
        parts.pop();
    }
    let joined = parts.join(" / ");
    if joined.chars().count() <= n {
        joined
    } else {
        joined.chars().take(n).collect()
    }
}
pub(crate) fn fit(s: &str, width: i32, scale: i32) -> String {
    fit_with(s, width, 8 * scale)
}
/// `fit` for the small face: six pixels a glyph.
pub(crate) fn fit_small(s: &str, width: i32, scale: i32) -> String {
    fit_with(s, width, 6 * scale)
}
fn fit_with(s: &str, width: i32, advance: i32) -> String {
    let n = (width / advance.max(1)).max(0) as usize;
    if s.chars().count() <= n {
        s.into()
    } else {
        format!(
            "{}...",
            s.chars().take(n.saturating_sub(3)).collect::<String>()
        )
    }
}
/// A small-face caption wrapped by words into at most `max_lines` rows of
/// `chars` glyphs; the last row is cut with an ellipsis when the words run
/// on.
pub(crate) fn wrap_small(s: &str, chars: usize, max_lines: usize) -> Vec<String> {
    let mut rows: Vec<String> = vec![String::new()];
    for word in s.split_whitespace() {
        let fits = rows.last().is_some_and(|row| {
            row.is_empty() || row.chars().count() + 1 + word.chars().count() <= chars
        });
        if fits {
            let row = rows.last_mut().expect("one row");
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(word);
        } else if rows.len() < max_lines.max(1) {
            rows.push(word.to_string());
        } else {
            let row = rows.last_mut().expect("one row");
            row.push(' ');
            row.push_str(word);
        }
    }
    if let Some(last) = rows.last_mut()
        && last.chars().count() > chars
    {
        *last = format!(
            "{}...",
            last.chars()
                .take(chars.saturating_sub(3))
                .collect::<String>()
        );
    }
    rows
}
/// A button's hint rows, at most two.  On the command card a leading key
/// ("Q", "V 180S+30P") comes off the first row to sit in the corner.
pub(crate) fn button_rows(hint: &str, card: bool) -> (Option<String>, Vec<String>) {
    let mut rows: Vec<String> = hint
        .split('\n')
        .filter(|row| !row.is_empty())
        .map(str::to_string)
        .collect();
    let mut key = None;
    if card && let Some(first) = rows.first().cloned() {
        let mut chars = first.chars();
        let lead = chars.next().filter(|c| c.is_ascii_alphanumeric());
        let rest = chars.as_str();
        if let Some(lead) = lead
            && (rest.is_empty() || rest.starts_with(' '))
        {
            key = Some(lead.to_string());
            let rest = rest.trim_start();
            if rest.is_empty() {
                rows.remove(0);
            } else {
                rows[0] = rest.to_string();
            }
        }
    }
    rows.truncate(2);
    (key, rows)
}
/// The corner key, unless the name would run into it: then the key goes
/// back to the head of the first row ("K 75% BACK" under CANCEL R&D).
pub(crate) fn card_key(
    label: &str,
    (key, mut rows): (Option<String>, Vec<String>),
    w: i32,
    s: i32,
) -> (Option<String>, Vec<String>) {
    let Some(k) = key else {
        return (None, rows);
    };
    let name_w = label.chars().count() as i32 * 6 * s - s;
    if name_w <= w - 16 * s {
        return (Some(k), rows);
    }
    match rows.first_mut() {
        Some(first) => *first = format!("{k} {first}"),
        None => rows.push(k),
    }
    (None, rows)
}
/// A hint row in gold with its pressure figures ("30P") in the pressure
/// jade of the header, so a price reads as two resources, not one word.
fn price_row(c: &mut Canvas, row: &str, x: i32, y: i32, enabled: bool, s: i32) {
    if !enabled {
        small(c, row, x, y, EDGE, s);
        return;
    }
    let chars: Vec<char> = row.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let pressure = i > start && chars.get(i) == Some(&'P');
        let end = if i > start && matches!(chars.get(i), Some('S' | 'P')) {
            i + 1
        } else {
            (start + 1).max(i)
        };
        let piece: String = chars[start..end].iter().collect();
        small(
            c,
            &piece,
            x + start as i32 * 6 * s,
            y,
            if pressure { JADE } else { GOLD },
            s,
        );
        i = end;
    }
}
pub(crate) fn card(c: &mut Canvas, r: Rect, accent: Color) {
    c.rect(r.x, r.y, r.w, r.h, SURFACE);
    c.rect(r.x, r.y, 2, r.h, accent);
    c.line(r.x + 2, r.y, r.x + r.w - 1, r.y, EDGE);
}
pub(crate) fn button(
    bounds: Rect,
    label: &str,
    hint: &str,
    action: Action,
    enabled: bool,
) -> Button {
    Button {
        x: bounds.x,
        y: bounds.y,
        w: bounds.w,
        h: bounds.h,
        label: label.into(),
        hint: hint.into(),
        action,
        enabled,
    }
}

impl Game {
    pub(crate) fn native_ui(&self) -> bool {
        self.canvas.width() >= 960 && self.canvas.height() >= 540
    }
    /// The interface scale: two at 1280x720 and above, doubled again on a
    /// two-to-one display, never more than the window holds of the 640x360
    /// design, and fixed by the INTERFACE setting when it is not AUTO.
    pub(crate) fn ui_scale(&self) -> i32 {
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        if w < 1280 || h < 720 {
            return 1;
        }
        let fit = (w / 640).min(h / 360).clamp(1, 4);
        let wanted = match self.ux.preferences.interface_scale {
            0 => 2 * self.display_scale.max(1),
            n => i32::from(n),
        };
        wanted.clamp(1, fit)
    }
    pub fn resize_view(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if (self.canvas.width(), self.canvas.height()) != (width, height) {
            self.canvas.resize(width, height);
            self.drag = None;
            self.ux.minimap_drag = false;
            self.buttons.clear();
        }
    }
    pub(crate) fn world_view(&self) -> WorldView {
        let s = self.ui_scale();
        if self.full_bleed {
            return WorldView {
                width: self.canvas.width() as i32,
                top: 0,
                bottom: self.canvas.height() as i32,
                zoom: self.zoom,
                sub: self.camera_sub,
            };
        }
        WorldView {
            width: self.canvas.width() as i32,
            top: if self.native_ui() {
                if self.spectator_band() {
                    64 * s
                } else {
                    crate::lean_hud::TOP_BAR * s
                }
            } else {
                24
            },
            // The dock alone: the prompt and the alert cards sit at the
            // field's lower edge and the group tabs in the dock.
            bottom: if self.native_ui() {
                self.canvas.height() as i32 - crate::lean_hud::DOCK * s
            } else {
                self.canvas.height() as i32 - 72
            },
            zoom: self.zoom,
            sub: self.camera_sub,
        }
    }
    /// One crossing held, by either side: the tide count needs both lanes
    /// and the station.  A player standing at a mouth without the station
    /// is told why nothing is counting.
    pub(crate) fn crossing_held(&self) -> Option<String> {
        let world = &self.world;
        let lanes = world.hold_arms(0);
        if world.gate.owner != Some(0)
            && lanes
                .iter()
                .any(|&lane| crate::qol::lane_presence(world, lane).0)
        {
            return Some("TAKE THE SLUICE TO HOLD A LANE".into());
        }
        if let Some(&held) = lanes.iter().find(|&&lane| world.holds_lane(0, lane)) {
            let rest: Vec<&str> = lanes
                .iter()
                .filter(|&&lane| lane != held)
                .map(|&lane| arm_letter(world, lane))
                .collect();
            // Once a seat is out one lane may be all a hold needs.
            return Some(if rest.is_empty() {
                format!("YOU HOLD {} LANE", arm_letter(world, held))
            } else {
                format!(
                    "YOU HOLD {} LANE / TAKE {}",
                    arm_letter(world, held),
                    rest.join(" + ")
                )
            });
        }
        for seat in world.seats().skip(1) {
            for lane in world.hold_arms(seat) {
                if world.holds_lane(seat, lane) {
                    return Some(format!(
                        "{} HOLDS {} LANE",
                        seat_name(world, seat),
                        arm_letter(world, lane)
                    ));
                }
            }
        }
        None
    }
    /// The left edge of the command card in the dock.
    pub(crate) fn command_card_x(&self) -> i32 {
        self.lean_card_x()
    }
    pub(crate) fn minimap_bounds(&self) -> Rect {
        if !self.native_ui() {
            return Rect {
                x: 11,
                y: 299,
                w: 114,
                h: 53,
            };
        }
        let s = self.ui_scale();
        Rect {
            x: 6 * s,
            y: self.canvas.height() as i32 - 82 * s,
            w: 140 * s,
            h: 74 * s,
        }
    }
    /// The practice coach's bubble (or its folded tab) as last drawn.
    fn guide_bounds(&self) -> Rect {
        self.coach_rect.unwrap_or(Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        })
    }
    /// The sluice card while it shows under the tide gauge; an empty
    /// rectangle while it is closed, so nothing on the field is hidden.
    pub(crate) fn route_bounds(&self) -> Rect {
        let r = self.tide_card_rect();
        if self.tide_card_shown() {
            r
        } else {
            Rect { w: 0, h: 0, ..r }
        }
    }
    /// The hold banner across the top of the field: clear of the sluice
    /// card, the practice guide and the zoom buttons.
    pub(crate) fn hold_banner_bounds(&self) -> Rect {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let bw = 232 * s;
        // Under the sluice card while it shows.
        let card = self.route_bounds();
        let y = if card.h > 0 {
            card.y + card.h + 6 * s
        } else {
            self.world_view().top + 8 * s
        };
        Rect {
            x: (w - bw) / 2,
            y,
            w: bw,
            h: 34 * s,
        }
    }
    pub(crate) fn native_world_pointer_allowed(&self, x: i32, y: i32) -> bool {
        if self.ux.practice_review
            || self.world.outcome.is_some()
            || !self.world_view().contains(x, y)
        {
            return false;
        }
        // A right-click on the card's body is meant for the ground under it.
        if (self.route_bounds().contains(x, y) && !self.pointer_through_card)
            || (self.ux.practice && self.guide_bounds().contains(x, y))
        {
            return false;
        }
        !self.buttons.iter().any(|b| {
            b.contains(x, y)
                && !(crate::game::passes_pending_click(&b.action)
                    && self.mode != crate::game::Mode::Context)
        }) && !self
            .native_tooltip_bounds()
            .is_some_and(|r| r.contains(x, y))
    }
    pub(crate) fn render_native(&mut self) {
        self.frame += 1;
        self.buttons.clear();
        let live = self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review;
        if !live {
            self.render_native_menu();
            return;
        }
        self.refresh_hover();
        self.draw_zoomed_world();
        if let Some((x, y)) = self.drag {
            self.canvas.frame(
                x.min(self.cursor.0),
                y.min(self.cursor.1),
                (self.cursor.0 - x).abs() + 1,
                (self.cursor.1 - y).abs() + 1,
                JADE,
            );
        }
        self.draw_native_hud();
        self.draw_native_guidance();
        self.draw_field_alerts();
        self.draw_net_marks();
        self.ensure_menu_focus();
        self.draw_native_buttons();
        self.draw_nudge_hud();
        self.draw_coach_ring();
        self.draw_native_feedback();
        self.draw_matchup();
    }
    /// The menus' own scale: the largest whole multiple of the 640 by 360
    /// menu that fits the window (3x at 1080p), unless Settings fixes a
    /// smaller interface.  The field's HUD keeps `ui_scale`.
    pub(crate) fn menu_scale(&self) -> i32 {
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        let fit = (w / 640).min(h / 360).max(1);
        match self.ux.preferences.interface_scale {
            0 => fit,
            n => i32::from(n).clamp(1, fit),
        }
    }
    fn render_native_menu(&mut self) {
        let cursor = self.cursor;
        let scale = self.menu_scale();
        let x = (self.canvas.width() as i32 - 640 * scale) / 2;
        let y = (self.canvas.height() as i32 - 360 * scale) / 2;
        let overlay = self.menu_model().overlay;
        std::mem::swap(&mut self.canvas, &mut self.ui_canvas);
        self.cursor = (
            (cursor.0 - x).div_euclid(scale),
            (cursor.1 - y).div_euclid(scale),
        );
        match self.screen {
            Screen::Menu => self.draw_menu(),
            Screen::Setup => self.draw_setup(),
            Screen::Settings => self.draw_settings(),
            Screen::Confirm => self.draw_confirm(),
            Screen::Help => self.draw_help(),
            Screen::Pause => self.draw_pause(),
            Screen::Lobby => self.draw_lobby(),
            Screen::Match => {
                // The result stands on its own: the last menu drawn here
                // (a Guide page) showed through its edges before.
                let model = self.menu_model();
                self.buttons = if self.ux.practice_review {
                    crate::menus::practice_review(&mut self.canvas, self.atlas.as_ref(), &model)
                } else {
                    crate::menus::result(
                        &mut self.canvas,
                        self.atlas.as_ref(),
                        &model,
                        self.world.outcome.clone().unwrap(),
                    )
                };
            }
        }
        self.ensure_menu_focus();
        self.draw_buttons();
        self.draw_setup_cards();
        self.draw_ux_feedback();
        self.cursor = cursor;
        std::mem::swap(&mut self.canvas, &mut self.ui_canvas);
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        if overlay {
            // Pause and its confirmations sit over the match they are about:
            // the field and HUD stay in sight, dimmed and inert. An online
            // field keeps moving, so it is dimmed less.
            let menu_buttons = std::mem::take(&mut self.buttons);
            self.cursor = (-10_000, -10_000);
            self.draw_zoomed_world();
            if self.screen == Screen::Match && !self.ux.practice_review {
                // The result shows the field it ended on, without the HUD.
                let v = self.world_view();
                self.canvas.rect(0, 0, w, v.top, INK);
                self.canvas.rect(0, v.bottom, w, h - v.bottom, INK);
            } else {
                self.draw_native_hud();
            }
            self.cursor = cursor;
            self.buttons = menu_buttons;
            let dim = if self.session.is_some() { 110 } else { 160 };
            self.canvas.rect(0, 0, w, h, [8, 14, 18, dim]);
        } else if self.screen == Screen::Menu {
            // Home stands on the field itself, darkened under the menu
            // column and fading out toward the right.
            self.draw_home_backdrop();
            self.canvas.rect(0, 0, w, h, [8, 14, 18, 70]);
            let solid = x + 300 * scale;
            let fade = 120 * scale;
            self.canvas.rect(0, 0, solid.max(0), h, [8, 14, 18, 190]);
            for i in 0..fade {
                let alpha = 190 - 190 * i / fade;
                self.canvas
                    .rect(solid + i, 0, 1, h, [8, 14, 18, alpha as u8]);
            }
        } else {
            self.canvas.clear(INK);
            // A quiet chart margin extends to any window aspect ratio.
            for yy in (0..h).step_by(48) {
                self.canvas.line(0, yy, w, yy, [26, 41, 48, 255]);
            }
            for xx in (0..w).step_by(48) {
                self.canvas.line(xx, 0, xx, h, [26, 41, 48, 255]);
            }
        }
        self.canvas.blit_integer(&self.ui_canvas, x, y, scale);
        for b in &mut self.buttons {
            b.x = x + b.x * scale;
            b.y = y + b.y * scale;
            b.w *= scale;
            b.h *= scale;
        }
    }
    fn draw_native_hud(&mut self) {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        let v = self.world_view();
        let state = self.console_state();
        self.draw_lean_top_bar();
        if self.tide_card_shown() {
            self.draw_tide_card(&state);
        }
        self.draw_hold_banner();
        // The dock: minimap, the quick-select column, the selection panel
        // under its group tabs, and the command card.
        self.canvas.rect(0, v.bottom, w, h - v.bottom, INK);
        self.canvas.line(0, v.bottom, w, v.bottom, EDGE);
        self.draw_native_minimap();
        if self.observing() {
            // Watching: a row per side instead of one seat's controls. A
            // seat knocked out of a match of three watches the same way.
            self.draw_observer_panel(v.bottom, h);
            self.draw_kill_feed();
            self.draw_spectator_band(v.top);
            self.draw_out_banner();
            return;
        }
        self.push_quick_select();
        let left = crate::lean_hud::PANEL_X * s;
        let command_x = self.command_card_x();
        let selection_w = command_x - left - 8 * s;
        self.push_group_tabs(left + selection_w);
        if self.inspected_enemy().is_some() {
            // An enemy clicked to read it stands in the panel, read only.
            self.draw_inspected_enemy(left, selection_w, h, s);
        } else if let Some(sel) = state.selection {
            if sel.selected_count > 1 {
                self.draw_selection_roster(left, selection_w, h, s);
            } else if let Some(info) = self.selected_production_info() {
                self.draw_production_panel(&sel, &info, left, selection_w, h, s);
            } else {
                // A wide dock keeps the panel's content together rather
                // than strung across the screen.
                let detail_w = selection_w.min(300 * s);
                let mut title_x = left;
                let one: Option<(Kind, bw_core::Faction)> = self
                    .selected
                    .first()
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
                    .map(|e| (e.kind, self.world.players[e.owner as usize].faction));
                // Machines show their portrait; a building its icon, drawn
                // at twice the size in the same frame.
                let portrait = sel.portrait_key.clone().map(|key| (key, 1, 0)).or_else(|| {
                    one.filter(|(kind, _)| kind.is_building())
                        .map(|(kind, faction)| (format!("ui_build_{}", kind.asset(faction)), 2, 4))
                });
                if let Some((key, times, inset)) = portrait {
                    self.canvas.rect(left, h - 72 * s, 56 * s, 57 * s, PANEL);
                    if let Some(atlas) = &self.atlas {
                        atlas.draw_scaled(
                            &mut self.canvas,
                            &key,
                            left + inset * s,
                            h - 72 * s + inset * s,
                            false,
                            crate::occlusion::PixelScale {
                                numerator: (s * times) as u16,
                                denominator: 1,
                            },
                        );
                    }
                    title_x += 62 * s;
                }
                text(
                    &mut self.canvas,
                    &fit(&sel.title, detail_w - (title_x - left), s),
                    title_x,
                    h - 72 * s,
                    WHITE,
                    s,
                );
                if sel.selected_count > 1 {
                    small(
                        &mut self.canvas,
                        &format!("{} MACHINES", sel.selected_count),
                        title_x,
                        h - 55 * s,
                        JADE,
                        s,
                    );
                } else {
                    let bw = (detail_w - (title_x - left)).min(170 * s);
                    self.canvas.rect(title_x, h - 60 * s, bw, 3 * s, RAISED);
                    self.canvas.rect(
                        title_x,
                        h - 60 * s,
                        bw * sel.hp.max(0) / sel.max_hp.max(1),
                        3 * s,
                        JADE,
                    );
                    // Its hull, then what it brings, as the same marks the
                    // hover cards use.
                    let mut chips = vec![crate::hover_card::Chip {
                        mark: crate::hover_card::Mark::Hull,
                        value: format!("{}/{}", sel.hp, sel.max_hp),
                        short: false,
                    }];
                    if let Some((kind, _)) = one {
                        chips.extend(crate::hover_card::stat_chips(kind, false));
                    }
                    // A transport's riders, aboard and room (trial 12).
                    if let Some(rider) = self
                        .selected
                        .first()
                        .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
                        .and_then(|e| self.rider_chip(e))
                    {
                        chips.insert(1, rider);
                    }
                    self.draw_chips(&chips, title_x, h - 54 * s, true);
                }
                // The role sentence lives in the Guide and the tooltips now
                // (the dock was too wordy): the panel keeps the numbers and
                // what the machine is doing.
                let role_w = detail_w - (title_x - left);
                small(
                    &mut self.canvas,
                    &fit(&sel.status, role_w, s),
                    title_x,
                    h - 41 * s,
                    WHITE,
                    s,
                );
            }
        } else {
            small(&mut self.canvas, "NO SELECTION", left, h - 70 * s, MUTED, s);
        }
        // The command card: a five-by-three grid of icons in the dock, in
        // the StarCraft manner.  Every order the selection can take lives
        // here, including formation and facing; nothing sits over the field.
        let first = self.buttons.len();
        self.collect_command_buttons();
        self.place_command_card(first);
        if self.spectator_band() {
            self.draw_spectator_band(v.top);
        }
    }
    /// The card's hold row: a chip for each lane coloured by who stands at
    /// its mouths (gold yours, red the enemy's, a red frame with a gold
    /// letter when both do), and between them the gauge of whichever hold
    /// count is running with the seconds it has to go.  Drawn in every
    /// tide state, so a flood never hides the hold.
    /// The sluice card under the tide gauge: status, water, the hold row
    /// and checklist, and the tide keys while the sluice is yours.
    fn draw_tide_card(&mut self, state: &crate::console::ConsoleState) {
        let s = self.ui_scale();
        let route = self.route_bounds();
        let compact = self.ux.preferences.compact_field_cards;
        card(&mut self.canvas, route, GOLD);
        self.buttons.push(button(
            Rect {
                x: route.x + route.w - 21 * s,
                y: route.y + 3 * s,
                w: 18 * s,
                h: 12 * s,
            },
            if compact { "+" } else { "-" },
            "",
            Action::CompactFieldCards,
            true,
        ));
        let lanes = if crate::seats::arm_count(&self.world) <= 2 {
            match self.world.gate.tide {
                bw_sim::Tide::Neutral => "TIDE NEUTRAL / BOTH SHALLOW".to_string(),
                bw_sim::Tide::Flood => "FLOOD / EVERY LANE DEEP".to_string(),
                bw_sim::Tide::Open if self.world.gate.north_dry() => {
                    "NORTH DRY / SOUTH DEEP".to_string()
                }
                bw_sim::Tide::Open => "SOUTH DRY / NORTH DEEP".to_string(),
            }
        } else {
            // Three arms: the dry one by its letter, the deep ones after.
            let dry = self.world.gate.dry_arm.index();
            match self.world.gate.tide {
                bw_sim::Tide::Neutral => "TIDE NEUTRAL / ALL SHALLOW".to_string(),
                bw_sim::Tide::Flood => "FLOOD / EVERY LANE DEEP".to_string(),
                bw_sim::Tide::Open => {
                    let deep: Vec<&str> = (0..crate::seats::arm_count(&self.world))
                        .filter(|&arm| arm != dry)
                        .map(|arm| arm_letter(&self.world, arm))
                        .collect();
                    format!(
                        "{} DRY / {} DEEP",
                        arm_letter(&self.world, dry),
                        deep.join(" + ")
                    )
                }
            }
        };
        if !compact {
            small(
                &mut self.canvas,
                &lanes,
                route.x + 9 * s,
                route.y + 21 * s,
                WHITE,
                s,
            );
        }
        let gate = &self.world.gate;
        let owner = match gate.owner {
            Some(0) => "YOUR SLUICE".to_string(),
            Some(seat) => format!(
                "{} SLUICE",
                crate::seats::seat_owner_word(&self.world, seat)
            ),
            None => "NEUTRAL SLUICE".to_string(),
        };
        // The line is gold, the card's colour; with three seats a line about
        // one opponent wears that opponent's colour.
        let mut status_color = GOLD;
        let capture = gate
            .capture_player
            .filter(|_| gate.capture_progress > 0)
            .map(|player| {
                (
                    player,
                    (gate.capture_progress * 100 / self.world.capture_work()).min(99),
                )
            });
        let status = if let Some(t) = state.route.warning_ticks_remaining {
            if state.route.flood_pending {
                format!("FLOOD IN {}S", t.div_ceil(30))
            } else if gate.ebb_pending {
                format!(
                    "{} EBBS IN {}S",
                    crate::seats::arm_word(&self.world, gate.dry_arm.index()).to_uppercase(),
                    t.div_ceil(30)
                )
            } else {
                let target = gate.switch_target.unwrap_or(bw_sim::Arm::SOUTH);
                format!(
                    "{} DRIES IN {}S",
                    crate::seats::arm_word(&self.world, target.index()).to_uppercase(),
                    t.div_ceil(30)
                )
            }
        } else if let Some(t) = state.route.flood_ticks_remaining {
            self.flood_status(t)
        } else if let Some(capture) = crate::trial11_words::capture_status(&self.world) {
            // Your claim and, when it stands still, why (trial 11); the
            // banner keeps any count in sight meanwhile.
            status_color = capture.color;
            capture.words
        } else if let Some((player, ticks)) = self.hold_gauge() {
            // The gauge fills while both lanes are held and drains while
            // they are not; the line says which is happening.
            let holding = self.world.holds_every_lane(player);
            let total = self.world.hold_ticks();
            let left = total.saturating_sub(ticks).div_ceil(30);
            let who = seat_name(&self.world, player);
            if player != 0 && self.world.seat_count() > 2 {
                status_color = hold_color(&self.world, player);
            }
            // A broken count drains three ticks a tick (rules 21): it
            // neither pauses nor resets.
            // A colour's name is longer than ENEMY: with three seats the
            // line drops a word to stay clear of the fold button.
            let three = self.world.seat_count() > 2;
            // A draining count names what is banked, a number that falls
            // as it drains; "DRAINS / 24S TO WIN" rose (trial 10).
            let banked = ticks.min(total) / 30;
            let full = total / 30;
            match (player, holding) {
                (0, true) => format!("YOU HOLD BOTH / WIN IN {left}S"),
                (0, false) => format!("YOU HELD {banked}/{full} / DRAINING"),
                (_, true) if three => format!("{who} HOLDS / WINS IN {left}S"),
                (_, true) => format!("{who} HOLDS BOTH / WINS IN {left}S"),
                (_, false) => format!("{who} HELD {banked}/{full} / DRAINING"),
            }
        } else if self.sluice_contested() {
            "SLUICE CONTESTED".into()
        } else if let Some((player, percent)) = capture {
            if player == 0 {
                format!("CAPTURING {percent}%")
            } else {
                if self.world.seat_count() > 2 {
                    status_color = hold_color(&self.world, player);
                }
                format!("{} CAPTURING {percent}%", seat_name(&self.world, player))
            }
        } else if let Some(held) = self.crossing_held() {
            // An opponent's lane: only the station's owner holds one.
            if let Some(seat) = gate.owner.filter(|&seat| seat != 0)
                && self.world.seat_count() > 2
                && held.starts_with(seat_name(&self.world, seat))
            {
                status_color = hold_color(&self.world, seat);
            }
            held
        } else {
            if let Some(seat) = gate.owner.filter(|&seat| seat != 0)
                && self.world.seat_count() > 2
            {
                status_color = hold_color(&self.world, seat);
            }
            owner
        };
        small(
            &mut self.canvas,
            &status,
            route.x + 9 * s,
            route.y + 7 * s,
            status_color,
            s,
        );
        let owner_is_us = gate.owner == Some(0);
        let tide = gate.tide;
        let north_dry = gate.north_dry();
        let dry_arm = gate.dry_arm.index();
        if compact {
            // Folded, the checklist stays: it is the part that says why a
            // count runs or not.
            self.draw_hold_checklist(route.y + 21 * s, route, s);
        } else {
            self.draw_hold_row(route, s);
            self.draw_hold_checklist(route.y + 49 * s, route, s);
        }
        if owner_is_us {
            // The holder chooses the tide: dry a side, or flood every
            // crossing for a while.  Nothing is offered during a flood.
            // A folded card keeps these: they are the only way to turn it.
            let mut choices: Vec<(&str, &str, Action)> = Vec::new();
            let arms = crate::seats::arm_count(&self.world);
            match tide {
                bw_sim::Tide::Flood => {}
                // Three arms: every arm not dry already, then FLOOD.
                _ if arms > 2 => {
                    for arm in 0..arms {
                        if tide == bw_sim::Tide::Open && arm == dry_arm {
                            continue;
                        }
                        choices.push((
                            crate::seats::dry_label(&self.world, arm),
                            "",
                            Action::SetTide(bw_sim::Arm(arm as u8)),
                        ));
                    }
                    choices.push(("FLOOD", "", Action::Flood));
                }
                bw_sim::Tide::Neutral => {
                    choices.push(("DRY N", "", Action::SetTide(bw_sim::Arm::NORTH)));
                    choices.push(("DRY S", "", Action::SetTide(bw_sim::Arm::SOUTH)));
                    choices.push(("FLOOD", "", Action::Flood));
                }
                bw_sim::Tide::Open => {
                    if north_dry {
                        choices.push(("DRY S", "", Action::SetTide(bw_sim::Arm::SOUTH)));
                    } else {
                        choices.push(("DRY N", "", Action::SetTide(bw_sim::Arm::NORTH)));
                    }
                    choices.push(("FLOOD", "", Action::Flood));
                }
            }
            let count = choices.len() as i32;
            // Four choices (three arms and FLOOD) share the card's width.
            let (step, width) = if count > 3 { (50, 49) } else { (66, 64) };
            for (i, (label, hint, action)) in choices.into_iter().enumerate() {
                let x = route.x + (209 - step * (count - i as i32)) * s;
                let enabled = self.action_reason(&action).is_none();
                self.buttons.push(button(
                    Rect {
                        x,
                        y: route.y + (if compact { 36 } else { 64 }) * s,
                        w: width * s,
                        h: 22 * s,
                    },
                    label,
                    hint,
                    action,
                    enabled,
                ));
            }
        }
    }
    fn draw_hold_row(&mut self, route: Rect, s: i32) {
        let y = route.y + 35 * s;
        let gauge = self.hold_gauge();
        // Your own two lanes: both arms on the Split Basin, the two that
        // touch your land on the Confluence.  Each chip wears the colour of
        // the seat that holds it.
        let lanes = self.world.arms_of(0);
        let seats_at = crate::qol::mouth_seats(&self.world);
        for (lane, x) in lanes
            .iter()
            .copied()
            .zip([route.x + 9 * s, route.x + 189 * s])
        {
            let holder = crate::qol::lane_holder_seat(&self.world, lane);
            let (own_present, enemy_mouth) = crate::qol::lane_presence(&self.world, lane);
            let contested = own_present && enemy_mouth.is_some();
            // The opponent standing there, for the contested frame.
            let rival = seats_at
                .get(lane)
                .map_or(0, |pair| (pair[0] | pair[1]) & !1)
                .trailing_zeros()
                .min(7) as u8;
            let letter = arm_letter(&self.world, lane);
            let (fill, frame, ink) = if contested {
                (PANEL, hold_color(&self.world, rival.max(1)), GOLD)
            } else if let Some(seat) = holder {
                (
                    hold_color(&self.world, seat),
                    hold_color(&self.world, seat),
                    INK,
                )
            } else {
                (PANEL, MUTED, MUTED)
            };
            self.canvas.rect(x, y, 20 * s, 11 * s, fill);
            self.canvas.frame(x, y, 20 * s, 11 * s, frame);
            small(&mut self.canvas, letter, x + 7 * s, y + 2 * s, ink, s);
        }
        let track_x = route.x + 35 * s;
        let track_w = 108 * s;
        self.canvas.rect(track_x, y, track_w, 11 * s, PANEL);
        self.canvas.frame(track_x, y, track_w, 11 * s, EDGE);
        let total = self.world.hold_ticks();
        match gauge {
            Some((player, ticks)) => {
                let color = hold_color(&self.world, player);
                let fill =
                    (i64::from(track_w) * i64::from(ticks.min(total)) / i64::from(total)) as i32;
                if fill > 0 {
                    self.canvas.rect(track_x, y, fill, 11 * s, color);
                }
                let left = total.saturating_sub(ticks).div_ceil(30);
                small(
                    &mut self.canvas,
                    &format!("{left}S"),
                    route.x + 149 * s,
                    y + 2 * s,
                    color,
                    s,
                );
            }
            None => small(
                &mut self.canvas,
                &format!("{}S", total / 30),
                route.x + 149 * s,
                y + 2 * s,
                MUTED,
                s,
            ),
        }
    }
    /// The hold rule as a checklist under the gauge: whose sluice, and per
    /// lane whether an own gun and an enemy one stand at a mouth.  A hold
    /// needs the sluice, a gun at a mouth of each lane and no enemy at
    /// either mouth; the chips say which part is missing.  A click shows
    /// the mouth that decides the hold.
    fn draw_hold_checklist(&mut self, y: i32, route: Rect, s: i32) {
        let h = 11 * s;
        let holders = crate::qol::mouth_holders(&self.world);
        let chip = |canvas: &mut Canvas, x: i32, w: i32, label: &str, on: bool, color: Color| {
            if on {
                canvas.rect(x, y, w, h, color);
            } else {
                canvas.rect(x, y, w, h, PANEL);
                canvas.frame(x, y, w, h, EDGE);
            }
            small(
                canvas,
                label,
                x + (w - label.len() as i32 * 6 * s) / 2 + s,
                y + 2 * s,
                if on { INK } else { MUTED },
                s,
            );
        };
        let x0 = route.x + 9 * s;
        let (sluice_on, sluice_color, sluice_label) = match self.world.gate.owner {
            Some(0) => (true, GOLD, "SLUICE"),
            Some(seat) => (true, hold_color(&self.world, seat), "SLUICE"),
            None => (false, MUTED, "SLUICE"),
        };
        chip(
            &mut self.canvas,
            x0,
            40 * s,
            sluice_label,
            sluice_on,
            sluice_color,
        );
        let three = self.world.seat_count() > 2;
        let seats_at = crate::qol::mouth_seats(&self.world);
        let nests_at = crate::qol::mouth_nests(&self.world);
        let mut lane_chips = Vec::new();
        for (lane, x) in self
            .world
            .arms_of(0)
            .into_iter()
            .zip([x0 + 48 * s, x0 + 116 * s])
        {
            let own = holders[lane].iter().any(|(own, _)| *own);
            // With three seats an enemy nest alone does not block (rules 22)
            // but slows the count: the chip lights and reads SLOW.
            let enemy = holders[lane].iter().any(|(_, enemy)| *enemy)
                || crate::trial12_hold::lane_slowed(&self.world, 0, lane);
            small(
                &mut self.canvas,
                arm_letter(&self.world, lane),
                x,
                y + 2 * s,
                WHITE,
                s,
            );
            // The FOE chip: an enemy gun at either mouth of the lane blocks
            // the hold.  Trial 10 could not read "FOE R", "FOE Y" (a bank's
            // initial) or "FOE 2", so the chip says FOE and wears whose it
            // is: the foe's colour, split when two foes stand there.  On
            // the Split Basin it still names the bank (W or E), and FOES
            // means both banks.  The on-field mouth tags name the bank.
            let foe = match (three, holders[lane][0].1, holders[lane][1].1) {
                (false, true, true) => "FOES",
                (false, true, false) => "FOE W",
                (false, false, true) => "FOE E",
                _ => "FOE",
            };
            let foes =
                seats_at[lane][0] | seats_at[lane][1] | nests_at[lane][0] | nests_at[lane][1];
            let foe_seats: Vec<u8> = (1..self.world.seat_count().min(8) as u8)
                .filter(|&seat| foes & (1 << seat) != 0)
                .collect();
            let foe_seat = foe_seats.first().copied().unwrap_or(1);
            chip(&mut self.canvas, x + 8 * s, 24 * s, "GUN", own, GOLD);
            chip(
                &mut self.canvas,
                x + 34 * s,
                32 * s,
                if enemy { "" } else { foe },
                enemy,
                hold_color(&self.world, foe_seat),
            );
            if enemy && let Some(&second) = foe_seats.get(1) {
                // Two foes: the chip's right half in the second's colour.
                let half = 16 * s;
                self.canvas.rect(
                    x + 34 * s + half,
                    y,
                    half,
                    11 * s,
                    hold_color(&self.world, second),
                );
            }
            if enemy {
                // What blocks it, as a mark before the word: a nest's
                // turret or a machine (trial 12: the Assembly spent fifteen
                // minutes not knowing two nests blocked its E lane).  With
                // two seats the word is the bank.  An enemy nest that only
                // slows the count reads SLOW (rules 22, W1's flag).
                let nest = crate::trial12_hold::lane_blockers(&self.world, lane)
                    .iter()
                    .any(|b| b.nest);
                let word = if crate::trial12_hold::lane_slowed(&self.world, 0, lane) {
                    "SLOW"
                } else {
                    match foe {
                        "FOE W" => "W",
                        "FOE E" => "E",
                        "FOES" => "W+E",
                        _ => "FOE",
                    }
                };
                let width = 7 * s + word.len() as i32 * 6 * s - s;
                let left = x + 34 * s + (32 * s - width) / 2;
                crate::trial12_hold::blocker_mark(&mut self.canvas, left, y + 3 * s, nest, INK, s);
                small(&mut self.canvas, word, left + 7 * s, y + 2 * s, INK, s);
            }
            lane_chips.push((lane, Rect { x, y, w: 66 * s, h }));
        }
        self.buttons.push(button(
            Rect {
                x: x0,
                y,
                w: 178 * s,
                h,
            },
            "HOLD CHECKLIST",
            "",
            Action::FocusHold,
            true,
        ));
        // Each lane's chips are a button of their own, over the row: a
        // click looks at what blocks the lane (trial 12).
        for (lane, r) in lane_chips {
            let label = format!("{} LANE", arm_letter(&self.world, lane));
            self.buttons
                .push(button(r, &label, "", Action::FocusLane(lane), true));
        }
    }
    /// The sluice card's line while a flood runs.  A flood freezes every
    /// count (rules 20): with one banked the line says so, and when the
    /// count takes up again.
    pub(crate) fn flood_status(&self, ticks_left: u64) -> String {
        if self.hold_gauge().is_some() {
            format!("FROZEN: FLOOD {}S", ticks_left.div_ceil(30))
        } else {
            format!("FLOOD {}S", ticks_left.div_ceil(30))
        }
    }
    /// The running tide count across the top of the field, for either
    /// side: a large timer, whose count it is and what stops or keeps it.
    /// Red while the enemy counts, blinking in its last thirty seconds;
    /// gold while we count; muted while a broken count drains.  A click,
    /// or F3 during an enemy count, shows the mouth that decides it.
    fn draw_hold_banner(&mut self) {
        // A recording speaks for neither side: its bar shows both counts.
        if self.spectator_band() {
            return;
        }
        let Some((player, ticks)) = self.hold_gauge() else {
            return;
        };
        let s = self.ui_scale();
        let r = self.hold_banner_bounds();
        let holding = self.world.holds_every_lane(player);
        let total = self.world.hold_ticks();
        let left = total.saturating_sub(ticks).div_ceil(30);
        // While a count drains, the big number is what it has banked, so it
        // falls as the words say (trial 10: "YOUR COUNT DRAINS" beside a
        // number that rose 66, 75, 89).
        let banked = ticks.min(total) / 30;
        let full = total / 30;
        let missing = || {
            if self.world.gate.owner != Some(0) {
                "RETAKE THE SLUICE".to_string()
            } else {
                let lanes: Vec<&str> = self
                    .world
                    .hold_arms(0)
                    .into_iter()
                    .filter(|lane| !self.world.holds_lane(0, *lane))
                    .map(|lane| arm_letter(&self.world, lane))
                    .collect();
                format!("{} LANE NOT HELD", lanes.join(" + "))
            }
        };
        // The counting seat names the banner and gives it its colour: RED
        // or VIOLET with three seats, ENEMY with one opponent.
        let who = seat_name(&self.world, player);
        let (accent, headline, hint) = match (player, holding) {
            // A flood freezes every count where it stands (rules 20): the
            // banner says so, what is banked, and when the flood falls.
            _ if self.world.hold_frozen() => (
                WHITE,
                "FROZEN: FLOOD".to_string(),
                format!(
                    "{} {}/{full}S, FLOOD {}S",
                    if player == 0 { "YOURS" } else { who },
                    ticks.min(total) / 30,
                    self.world
                        .gate
                        .flood_until
                        .map_or(0, |until| until.saturating_sub(self.world.tick))
                        .div_ceil(30)
                ),
            ),
            // An enemy nest at a bank halves the count with three seats
            // (rules 22): the banner says so rather than looking stuck.
            (0, true) if self.world.hold_slowed(0) => (
                GOLD,
                "YOU WIN IN".to_string(),
                "HALF SPEED: A NEST AT A BANK".to_string(),
            ),
            (0, true) => (
                GOLD,
                "YOU WIN IN".to_string(),
                "KEEP BOTH LANES CLEAR".to_string(),
            ),
            // Whose count it is (trial 11: "HELD 79/90 DRAINING" read as
            // either side's).
            (0, false) => (
                MUTED,
                crate::trial11_words::own_draining_headline(banked, full),
                missing(),
            ),
            // Only a gun at a bank of the counting seat's own lanes stops
            // it (trial 12: both defenders sent guns to another lane).
            (_, true) => (
                hold_color(&self.world, player),
                format!("{who} WINS IN"),
                if self.world.hold_slowed(player) {
                    format!(
                        "HALF SPEED: {}",
                        crate::trial12_hold::stop_hint(&self.world, player)
                    )
                } else {
                    crate::trial12_hold::stop_hint(&self.world, player)
                },
            ),
            // A broken count is not a reset: it keeps what it banked and
            // drains three seconds a second.  In the ninth trial Union read each
            // "hold broken" as safety while the gauge kept most of its run.
            (_, false) => (
                MUTED,
                format!("{who} DRAINING"),
                format!("HELD {banked}/{full}S, NOT RESET"),
            ),
        };
        let blink = player != 0
            && holding
            && !self.world.hold_frozen()
            && left <= 30
            && (self.world.tick / 15).is_multiple_of(2);
        let hover = r.contains(self.cursor.0, self.cursor.1);
        self.canvas.rect(
            r.x,
            r.y,
            r.w,
            r.h,
            if !blink {
                SURFACE
            } else if self.world.seat_count() > 2 {
                crate::seats::shade(crate::seats::seat_colour(&self.world, player))
            } else {
                [72, 22, 26, 255]
            },
        );
        for i in 0..s {
            self.canvas.frame(
                r.x + i,
                r.y + i,
                r.w - 2 * i,
                r.h - 2 * i,
                if hover { WHITE } else { accent },
            );
        }
        self.canvas.rect(r.x, r.y, 4 * s, r.h, accent);
        let timer = if holding {
            format!("{left}S")
        } else {
            format!("{banked}S")
        };
        let timer_color = if player != 0 && holding {
            WHITE
        } else {
            accent
        };
        text(
            &mut self.canvas,
            &timer,
            r.x + 10 * s,
            r.y + 8 * s,
            timer_color,
            2 * s,
        );
        let x = r.x + 64 * s;
        text(&mut self.canvas, &headline, x, r.y + 6 * s, WHITE, s);
        small(
            &mut self.canvas,
            &fit_small(&hint, r.x + r.w - x - 16 * s, s),
            x,
            r.y + 21 * s,
            if player != 0 && holding {
                hold_color(&self.world, player)
            } else {
                MUTED
            },
            s,
        );
        small(
            &mut self.canvas,
            "F3",
            r.x + r.w - 15 * s,
            r.y + 21 * s,
            MUTED,
            s,
        );
        // The gauge itself along the foot of the banner: how much of the
        // ninety seconds is banked, so a drain reads as a bar that empties.
        let full = r.w - 8 * s;
        let banked = full * ticks.min(total) as i32 / total.max(1) as i32;
        self.canvas
            .rect(r.x + 4 * s, r.y + r.h - 3 * s, full, 2 * s, INK);
        self.canvas
            .rect(r.x + 4 * s, r.y + r.h - 3 * s, banked, 2 * s, accent);
        self.buttons
            .push(button(r, &headline, "", Action::FocusHold, true));
    }
    fn draw_native_minimap(&mut self) {
        let r = self.minimap_bounds();
        let s = self.ui_scale();
        // The basin chart: paper, ink coastline, hatched deep water, the lane
        // marks by public state, wells, wreck beds and the sluice symbol.
        let chart = self.chart();
        let bounds = crate::minimap_chart::ChartRect::axis(r.x, r.y, r.w, r.h);
        crate::minimap_chart::draw_margin(&mut self.canvas, &self.world, bounds, chart, s);
        crate::minimap_chart::draw_field(&mut self.canvas, &self.world, chart, s);
        // Echo remote orders on the chart without revealing any world entity.
        for alert in self.ux.alerts.entries.iter().take(3) {
            let (x, y) = chart.plot(&self.world, alert.pos);
            let x = x.clamp(r.x + 3 * s, r.x + r.w - 3 * s - 1);
            let y = y.clamp(r.y + 3 * s, r.y + r.h - 3 * s - 1);
            let c = if alert.kind == crate::field_alerts::AlertKind::Attack {
                RED
            } else {
                JADE
            };
            self.canvas.frame(x - 3 * s, y - 3 * s, 7 * s, 7 * s, c);
        }
        // The rally of a selected producer: a small gold flag on the chart.
        let rallies: Vec<Pos> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0 && e.hp > 0 && e.aboard.is_none() && self.selected.contains(&e.id)
            })
            .filter_map(|e| e.rally)
            .collect();
        for rally in rallies {
            let (x, y) = chart.plot(&self.world, rally);
            let x = x.clamp(r.x + 3 * s, r.x + r.w - 3 * s - 1);
            let y = y.clamp(r.y + 3 * s, r.y + r.h - 3 * s - 1);
            self.canvas.line(x, y, x, y - 4 * s, WHITE);
            self.canvas.rect(x + 1, y - 4 * s, 3 * s, 2 * s, GOLD);
        }
        // The crossing mouths, and every enemy building seen: the chart
        // keeps what the scout paid for.
        let mouths = self.world.crossing_mouths();
        for (lane, pair) in mouths.iter().enumerate() {
            let c = match crate::qol::lane_holder_seat(&self.world, lane) {
                Some(seat) => hold_color(&self.world, seat),
                None => crate::minimap_chart::INK,
            };
            for point in pair {
                let (x, y) = chart.plot(&self.world, *point);
                self.canvas.line(x - 2 * s, y, x, y - 2 * s, c);
                self.canvas.line(x, y - 2 * s, x + 2 * s, y, c);
                self.canvas.line(x + 2 * s, y, x, y + 2 * s, c);
                self.canvas.line(x, y + 2 * s, x - 2 * s, y, c);
            }
        }
        // While a count rises, the held mouths flash on the chart: a
        // square in the holder's colour, on and off twice a second.
        if let Some((player, _)) = self.hold_gauge()
            && self.world.holds_every_lane(player)
            && (self.world.tick / 8).is_multiple_of(2)
        {
            let seats_at = crate::qol::mouth_seats(&self.world);
            let c = hold_color(&self.world, player);
            for lane in self.world.hold_arms(player) {
                for (index, point) in mouths[lane].iter().enumerate() {
                    if seats_at[lane][index] & (1 << player) == 0 {
                        continue;
                    }
                    let (x, y) = chart.plot(&self.world, *point);
                    for i in 0..s {
                        self.canvas.frame(
                            x - 4 * s + i,
                            y - 4 * s + i,
                            9 * s - 2 * i,
                            9 * s - 2 * i,
                            c,
                        );
                    }
                }
            }
        }
        // The banks where a gun of ours stops an enemy count (trial 12).
        self.draw_stop_marks_on_chart(chart);
        // Each enemy building in its owner's colour: with three seats, red
        // for every foe named no one.
        let mut enemy_buildings: Vec<(Pos, u8)> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner != 0
                    && e.hp > 0
                    && e.kind.is_building()
                    && self.world.entity_visible(0, e.id)
            })
            .map(|e| (e.pos, e.owner))
            .collect();
        enemy_buildings.extend(
            self.world
                .knowledge(0)
                .stale
                .iter()
                .filter(|o| {
                    o.owner != 0
                        && o.kind.is_building()
                        && self
                            .ux
                            .chart_memory
                            .cleared
                            .get(&o.id)
                            .is_none_or(|t| *t < o.last_seen)
                })
                .map(|o| (o.pos, o.owner)),
        );
        for (pos, owner) in enemy_buildings {
            let (x, y) = chart.plot(&self.world, pos);
            let x = x.clamp(r.x + s, r.x + r.w - 2 * s);
            let y = y.clamp(r.y + s, r.y + r.h - 2 * s);
            let colour = crate::seats::seat_colour(&self.world, owner);
            self.canvas.rect(x - s, y - s, 3 * s, 3 * s, colour);
        }
        for marker in &self.ux.markers {
            if self.world.tick.saturating_sub(marker.tick) >= 24 {
                continue;
            }
            let (x, y) = chart.plot(&self.world, marker.pos);
            let x = x.clamp(r.x + 3 * s, r.x + r.w - 3 * s - 1);
            let y = y.clamp(r.y + 3 * s, r.y + r.h - 3 * s - 1);
            self.canvas.line(x - 3 * s, y, x, y - 3 * s, GOLD);
            self.canvas.line(x, y - 3 * s, x + 3 * s, y, GOLD);
            self.canvas.line(x + 3 * s, y, x, y + 3 * s, GOLD);
            self.canvas.line(x, y + 3 * s, x - 3 * s, y, GOLD);
        }
        let view = self.world_view();
        let corners = [
            (0, view.top),
            (view.width - 1, view.top),
            (view.width - 1, view.bottom - 1),
            (0, view.bottom - 1),
        ]
        .map(|(x, y)| {
            let (x, y) = chart.plot(&self.world, self.unproject(x, y));
            (x.clamp(r.x, r.x + r.w - 1), y.clamp(r.y, r.y + r.h - 1))
        });
        // Ink with a paper-light line inside it: the view reads on hatched
        // water and on dry paper alike.
        for i in 0..4 {
            let (x0, y0) = corners[i];
            let (x1, y1) = corners[(i + 1) % 4];
            self.canvas.line(x0, y0, x1, y1, crate::minimap_chart::INK);
            self.canvas
                .line(x0 + 1, y0 + 1, x1 + 1, y1 + 1, [236, 226, 196, 255]);
        }
        self.canvas.frame(r.x - 1, r.y - 1, r.w + 2, r.h + 2, EDGE);
    }
    fn draw_field_alerts(&mut self) {
        // A recording's alerts are one seat's; the observer has the band.
        if self.world.outcome.is_some() || self.ux.practice_review || self.playback.is_some() {
            return;
        }
        // Cards stand over the field's lower left corner, above the chart
        // they point into, clear of the prompt and the selection.  Two at
        // most, newest nearest the dock.
        let s = self.ui_scale();
        let v = self.world_view();
        let card_w = 140 * s;
        // The words wrap into the card's two rows; a long place and cause
        // drop the cause's kind first, never a cut word.
        let chars = ((card_w - 10 * s) / (6 * s)).max(1) as usize;
        for (i, a) in self.ux.alerts.entries.iter().take(2).enumerate() {
            let i = i as i32;
            let r = Rect {
                x: 6 * s,
                y: v.bottom - (i + 1) * 22 * s,
                w: card_w,
                h: 20 * s,
            };
            let caption = a.card_caption(self.world.tick.saturating_sub(a.tick) / 30, chars, 2);
            self.buttons.push(button(
                r,
                &caption,
                "",
                Action::FocusAlert(Some(a.id)),
                true,
            ));
        }
    }
    fn draw_native_guidance(&mut self) {
        if !self.ux.practice {
            self.coach_rect = None;
            self.coach_ring = None;
            return;
        }
        self.draw_coach();
    }
    fn draw_native_buttons(&mut self) {
        let s = self.ui_scale();
        let dock_top = self.world_view().bottom;
        let card_x = self.command_card_x();
        for i in 0..self.buttons.len() {
            let b = &self.buttons[i].clone();
            // The hold banner, the checklist and the tide gauge draw
            // themselves.
            if matches!(
                b.action,
                Action::FocusHold | Action::FocusLane(_) | Action::TideCard
            ) {
                continue;
            }
            let hover = b.contains(self.cursor.0, self.cursor.1);
            let focus = self.ux.focused.as_ref() == Some(&b.action);
            if self.icon_cell(b) {
                self.draw_icon_cell(b, hover, focus);
                continue;
            }
            if matches!(b.action, Action::FilterSelection(_, true)) {
                self.draw_remove_control(b, hover);
                continue;
            }
            self.canvas.rect(
                b.x,
                b.y,
                b.w,
                b.h,
                if b.enabled && (hover || focus) {
                    RAISED
                } else {
                    PANEL
                },
            );
            self.canvas.line(
                b.x,
                b.y + b.h - 1,
                b.x + b.w - 1,
                b.y + b.h - 1,
                if b.enabled && (hover || focus) {
                    GOLD
                } else {
                    EDGE
                },
            );
            if focus && b.enabled {
                self.canvas.rect(b.x, b.y, 2 * s, b.h, GOLD);
            }
            if let Action::RecallGroup(n) = b.action
                && b.h < 16 * s
            {
                self.draw_group_tab(b, n);
                continue;
            }
            if let Action::RecallGroup(n) = b.action {
                let ids = self.live_group(n);
                let mut selected = self.gameplay_ids(|_| true);
                selected.sort_unstable();
                selected.dedup();
                let active = !ids.is_empty() && ids == selected;
                small(
                    &mut self.canvas,
                    &n.to_string(),
                    b.x + 5 * s,
                    b.y + 2 * s,
                    if active { GOLD } else { WHITE },
                    s,
                );
                small(
                    &mut self.canvas,
                    &format!("{}", ids.len()),
                    b.x + 5 * s,
                    b.y + 11 * s,
                    if ids.is_empty() { MUTED } else { JADE },
                    s,
                );
                if active {
                    self.canvas.rect(b.x, b.y, 2 * s, b.h, GOLD);
                }
                continue;
            }
            if let Action::FocusAlert(Some(id)) = b.action {
                let card = self.ux.alerts.entries.iter().find(|a| a.id == id);
                let urgent = card.is_some_and(|a| a.kind.urgent());
                self.canvas
                    .rect(b.x, b.y, 2 * s, b.h, if urgent { RED } else { JADE });
                // The opponent behind it, as a swatch of its colour in the
                // corner: shape and colour before words.
                if let Some(by) = card.and_then(|a| a.by) {
                    self.canvas
                        .rect(b.x + b.w - 5 * s, b.y + s, 4 * s, 4 * s, by.colour());
                }
                // Two small rows when the words need them: a count, a
                // direction, a cause and an age all fit on one card.
                let cap = ((b.w - 10 * s) / (6 * s)).max(1) as usize;
                let rows = wrap_small(&b.label, cap, 2);
                if rows.len() == 1 {
                    small(
                        &mut self.canvas,
                        &rows[0],
                        b.x + 6 * s,
                        b.y + (b.h - 7 * s) / 2,
                        WHITE,
                        s,
                    );
                } else {
                    small(
                        &mut self.canvas,
                        &rows[0],
                        b.x + 6 * s,
                        b.y + 2 * s,
                        WHITE,
                        s,
                    );
                    small(
                        &mut self.canvas,
                        &rows[1],
                        b.x + 6 * s,
                        b.y + 11 * s,
                        WHITE,
                        s,
                    );
                }
                continue;
            }
            if b.action == Action::CycleSpeed && b.h < 16 * s {
                // Pace as play arrows, one per step, then the multiplier:
                // no word, and room for "1.5X" inside the button.
                let color = if b.enabled { WHITE } else { MUTED };
                let arrows = match self.shown_pace() {
                    crate::tempo::GameSpeed::Slow => 1,
                    crate::tempo::GameSpeed::Normal => 2,
                    crate::tempo::GameSpeed::Fast => 3,
                };
                // The arrows alone, centred: their count is the pace, and
                // the hover card names it.
                let aw = (arrows * 4 + 1) * s;
                for i in 0..arrows {
                    play_arrow(
                        &mut self.canvas,
                        b.x + (b.w - aw) / 2 + i * 4 * s,
                        b.y + 3 * s,
                        color,
                        s,
                    );
                }
                continue;
            }
            if b.h < 16 * s && b.y < self.world_view().top {
                // The top bar's short buttons: the word centred, small.
                let shown = fit_small(&b.label, b.w - 2 * s, s);
                let tw = shown.chars().count() as i32 * 6 * s - s;
                small(
                    &mut self.canvas,
                    &shown,
                    b.x + (b.w - tw) / 2,
                    b.y + 4 * s,
                    if b.enabled { WHITE } else { MUTED },
                    s,
                );
                continue;
            }
            if b.action == Action::CycleSpeed && b.w <= 46 * s {
                small(&mut self.canvas, "PACE", b.x + 5 * s, b.y + 3 * s, MUTED, s);
                small(
                    &mut self.canvas,
                    &b.label,
                    b.x + 5 * s,
                    b.y + 13 * s,
                    WHITE,
                    s,
                );
                continue;
            }
            // A command-card button carries its key in its top right
            // corner, as StarCraft's card does, so the rows under the name
            // keep all ten glyphs for the price and the effect.
            let card_button = b.y >= dock_top && b.x >= card_x;
            let (key, rows) = card_key(&b.label, button_rows(&b.hint, card_button), b.w, s);
            let room = b.w - if key.is_some() { 14 } else { 10 } * s;
            let y = if rows.len() == 2 {
                b.y + 2 * s
            } else if (b.h >= 28 * s && b.h < 30 * s && !rows.is_empty())
                || (key.is_some() && !rows.is_empty())
            {
                b.y + 3 * s
            } else if b.h >= 30 * s && !rows.is_empty() {
                b.y + 5 * s
            } else {
                b.y + (b.h - 9 * s) / 2
            };
            let color = if b.enabled { WHITE } else { MUTED };
            let glyphs = b.label.chars().count() as i32;
            // A short button (the folded field row) reads in small type.
            if glyphs * 8 * s - s <= room && b.h >= 16 * s {
                text(&mut self.canvas, &b.label, b.x + 5 * s, y, color, s);
            } else if glyphs * 6 * s - s <= room {
                small(&mut self.canvas, &b.label, b.x + 5 * s, y + s, color, s);
            } else {
                small(
                    &mut self.canvas,
                    &fit_small(&b.label, room + s, s),
                    b.x + 5 * s,
                    y + s,
                    color,
                    s,
                );
            }
            if let Some(key) = &key {
                small(
                    &mut self.canvas,
                    key,
                    b.x + b.w - 8 * s,
                    y + s,
                    if b.enabled { MUTED } else { EDGE },
                    s,
                );
            }
            if !rows.is_empty() && b.h >= 28 * s {
                // The cost row, and below it the effect row when the
                // button carries one: each cut to the width it has.
                let cap = ((b.w - 10 * s) / (6 * s)).max(1) as usize;
                let row_y: &[i32] = if rows.len() == 2 { &[12, 20] } else { &[18] };
                for (row, dy) in rows.iter().zip(row_y) {
                    let shown: String = row.chars().take(cap).collect();
                    price_row(
                        &mut self.canvas,
                        &shown,
                        b.x + 5 * s,
                        b.y + dy * s,
                        b.enabled,
                        s,
                    );
                }
            }
            // With three seats a DRY button carries the arm's two banks as a
            // strip of their colours under its letter: whom the lane joins.
            if let Action::SetTide(arm) = b.action
                && self.world.seat_count() > 2
                && arm.index() < crate::seats::arm_count(&self.world)
            {
                let banks = self.world.arm_banks(arm.index());
                let w = (b.w - 10 * s) / 2;
                for (i, seat) in banks.into_iter().enumerate() {
                    self.canvas.rect(
                        b.x + 5 * s + i as i32 * w,
                        b.y + b.h - 5 * s,
                        w - s,
                        2 * s,
                        if b.enabled {
                            hold_color(&self.world, seat)
                        } else {
                            EDGE
                        },
                    );
                }
            }
        }
    }
    pub(crate) fn native_tooltip_bounds(&self) -> Option<Rect> {
        let s = self.ui_scale();
        // A key or a click dismisses the tooltip until the pointer moves
        // again: hovering asks, acting has been answered.
        if self.tooltip_dismissed_at == Some(self.cursor) && !self.ux.keyboard_navigation {
            return None;
        }
        self.buttons
            .iter()
            .rev()
            .find(|b| {
                if self.ux.keyboard_navigation {
                    self.ux.focused.as_ref() == Some(&b.action)
                } else {
                    b.contains(self.cursor.0, self.cursor.1)
                }
            })
            .and_then(|b| self.hover_card(b).map(|card| (b, card)))
            .map(|(b, card)| {
                let h = Game::hover_card_rows(&card) * s;
                let view = self.world_view();
                let canvas_w = self.canvas.width() as i32;
                let canvas_h = self.canvas.height() as i32;
                let dock_top = view.bottom;
                if b.y >= dock_top {
                    // A dock button's tooltip stands on the dock's top edge,
                    // as StarCraft keeps its tooltips above the card: the
                    // command card's flush with the screen's right edge, any
                    // other over its button.  The selection stays in sight.
                    let x = if b.x >= self.command_card_x() {
                        canvas_w - (TIP_W + 6) * s
                    } else {
                        b.x.clamp(6 * s, canvas_w - (TIP_W + 6) * s)
                    };
                    let y = (dock_top - h - 4 * s).max(view.top + 6 * s);
                    return Rect {
                        x,
                        y,
                        w: TIP_W * s,
                        h,
                    };
                }
                let above = b.y - h - 6 * s;
                let below = b.y + b.h + 6 * s;
                // A button in the field's bottom row, the band or the dock
                // opens its tooltip downward when there is room, so the
                // tooltip never covers the ground the player is watching.
                let low = b.y >= view.bottom - 40 * s;
                let y = if low && below + h <= canvas_h - 6 * s {
                    below
                } else if above >= view.top + 6 * s {
                    above
                } else {
                    below
                }
                .clamp(
                    view.top + 6 * s,
                    (canvas_h - h - 6 * s).max(view.top + 6 * s),
                );
                Rect {
                    x: b.x.clamp(6 * s, canvas_w - (TIP_W + 6) * s),
                    y,
                    w: TIP_W * s,
                    h,
                }
            })
    }
    pub(crate) fn native_prompt(&self) -> String {
        match self.mode {
            Mode::Build(k) => {
                let reason = if self.world_pointer_allowed(self.cursor.0, self.cursor.1) {
                    // The same site the click would use: centred under the
                    // pointer, or snapped to a well for a condenser.
                    let origin = self.build_origin(k, self.unproject(self.cursor.0, self.cursor.1));
                    match self.nearest_well_off_screen(k) {
                        Some(direction) => format!("NEAREST WELL IS {direction}"),
                        None => self
                            .placement_reason(k, origin)
                            .unwrap_or_else(|| "CLICK TO BUILD".into()),
                    }
                } else {
                    "CHOOSE A SITE".into()
                };
                // A refusal that already says Esc cancels is not told twice.
                let esc = if reason.to_lowercase().contains("esc cancels") {
                    ""
                } else {
                    " / ESC CANCELS"
                };
                format!(
                    "{}: {reason}{esc}",
                    crate::ux::building_name(k, self.faction)
                )
            }
            Mode::Attack => self.attack_mode_prompt(),
            Mode::Gather => "GATHER / CLICK A WRECK / ESC CANCELS".into(),
            Mode::Face => "FACE / CLICK A DIRECTION / ESC CANCELS".into(),
            Mode::Glint => "GLINT / CLICK A DIRECTION / ESC CANCELS".into(),
            Mode::Lay => crate::trial11_words::lay_prompt(self),
            Mode::Context if self.message_live() => self.message.clone(),
            Mode::Context if self.minimap_bounds().contains(self.cursor.0, self.cursor.1) => {
                "DRAG PANS / RIGHT CLICK MOVES OR SETS RALLY".into()
            }
            Mode::Context => self
                .context_order(self.cursor.0, self.cursor.1, self.ux.pointer_shift)
                .map(|(command, verb, _)| self.order_hint(&command, verb))
                .unwrap_or_default(),
        }
    }
    /// When a condenser is being placed and no well is on screen, the
    /// compass direction of the nearest one from the view's centre.
    pub(crate) fn nearest_well_off_screen(&self, kind: Kind) -> Option<&'static str> {
        if kind != Kind::Condenser {
            return None;
        }
        let view = self.world_view();
        let (cx, cy) = (view.width / 2, (view.top + view.bottom) / 2);
        let wells = &self.world.map.wells;
        if wells.iter().any(|well| {
            let (x, y) = self.project(*well);
            (0..view.width).contains(&x) && (view.top..view.bottom).contains(&y)
        }) {
            return None;
        }
        let (dx, dy) = wells
            .iter()
            .map(|well| {
                let (x, y) = self.project(*well);
                (x - cx, y - cy)
            })
            .min_by_key(|(dx, dy)| i64::from(*dx).pow(2) + i64::from(*dy).pow(2))?;
        Some(if dx.abs() > 2 * dy.abs() {
            if dx > 0 { "EAST" } else { "WEST" }
        } else if dy.abs() > 2 * dx.abs() {
            if dy > 0 { "SOUTH" } else { "NORTH" }
        } else {
            match (dx > 0, dy > 0) {
                (true, true) => "SOUTH-EAST",
                (true, false) => "NORTH-EAST",
                (false, true) => "SOUTH-WEST",
                (false, false) => "NORTH-WEST",
            }
        })
    }
    fn draw_native_feedback(&mut self) {
        self.draw_lean_prompt();
        if self.native_tooltip_bounds().is_none() {
            self.draw_field_tip();
        }
        if let Some(r) = self.native_tooltip_bounds() {
            let b = self
                .buttons
                .iter()
                .rev()
                .find(|b| {
                    if self.ux.keyboard_navigation {
                        self.ux.focused.as_ref() == Some(&b.action)
                    } else {
                        b.contains(self.cursor.0, self.cursor.1)
                    }
                })
                .unwrap();
            if let Some(hover) = self.hover_card(&b.clone()) {
                self.draw_hover_card(r, &hover);
            }
        }
        // The window's pointer shows the armed order (pointer.rs); a
        // crosshair drawn here trailed it by a frame.
    }
}

/// A small play arrow, 4 by 7 logical pixels, for the pace button.
fn play_arrow(canvas: &mut Canvas, x: i32, y: i32, color: Color, s: i32) {
    for (col, half) in [(0, 3), (1, 2), (2, 1), (3, 0)] {
        canvas.rect(
            x + col * s,
            y + (3 - half) * s,
            s,
            (2 * half + 1) * s,
            color,
        );
    }
}

/// The sluice card, the hold banner and the chart on the Confluence: three
/// seats, three arms.  Set `BW_THREE_SEAT_FRAMES` to a folder to keep the
/// frames for review.
#[cfg(test)]
mod three_seat_tests {
    use crate::canvas::Color;
    use crate::game::{Action, Game};
    use bw_core::Kind;
    use bw_sim::{Arm, MapId, Order};
    use std::path::PathBuf;

    /// A Confluence of the Union, the Assembly and the Assembly again:
    /// the Union alone wears its red, the two Assemblies violet and pink
    /// (never the jade their own machines wear).
    fn confluence() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.map = MapId::Confluence;
        game.start();
        assert_eq!(game.world.seat_count(), 3);
        game.world.ai_enabled = false;
        game.resize_view(1920, 1080);
        game.selected.clear();
        game
    }

    /// The whole frame, and the sluice card, the banner and the chart cut
    /// out at their own size.
    fn keep_frame(g: &Game, name: &str) {
        let Ok(dir) = std::env::var("BW_THREE_SEAT_FRAMES") else {
            return;
        };
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("frame folder");
        g.canvas
            .save(&dir.join(format!("{name}.png")))
            .expect("frame");
        let card = g.route_bounds();
        let banner = g.hold_banner_bounds();
        let chart = g.minimap_bounds();
        for (part, r) in [("card", card), ("banner", banner), ("chart", chart)] {
            let mut crop = crate::canvas::Canvas::new(r.w as u32 + 8, r.h as u32 + 8);
            for y in 0..r.h + 8 {
                for x in 0..r.w + 8 {
                    if let Some(c) = g.canvas.get(r.x - 4 + x, r.y - 4 + y) {
                        crop.pixel(x, y, c);
                    }
                }
            }
            crop.save(&dir.join(format!("{name}-{part}.png")))
                .expect("crop");
        }
    }

    /// A gun of `seat` on HOLD at its own mouth of every lane it touches.
    fn hold_every_lane(g: &mut Game, seat: u8) {
        g.world.gate.owner = Some(seat);
        for arm in g.world.arms_of(seat) {
            let mouth = g.world.own_mouth(seat, arm).expect("mouth");
            let id = g.world.spawn_for_tests(seat, Kind::Bulwark, mouth);
            if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
                e.order = Order::Hold;
            }
        }
        assert!(g.world.holds_every_lane(seat));
    }

    /// Trial 10's cryptic chips: "FOE R", "FOE Y" (a bank's initial, Y for
    /// YOU) and "FOE 2". The chip says FOE and wears each foe's colour, the
    /// right half the second's when two stand at the lane.
    #[test]
    fn a_foe_chip_wears_its_foes_colours_not_initials() {
        let mut g = confluence();
        g.ux.preferences.tide_card = true;
        g.world.gate.owner = Some(0);
        let arm = g.world.arms_of(0)[0];
        let mouth = g.world.own_mouth(0, arm).expect("mouth");
        g.world.spawn_for_tests(1, Kind::Bulwark, mouth);
        g.world.spawn_for_tests(2, Kind::Bulwark, mouth);
        // A tide switch on its way, for the gauge's arrow.
        g.world.gate.warning_until = Some(g.world.tick + 200);
        g.world.gate.switch_target = Some(Arm(2));
        // A loss card with the opponent's swatch.
        g.ux.alerts.entries.clear();
        g.ux.alerts.push(
            crate::field_alerts::AlertKind::MachinesLost,
            mouth,
            g.world.tick,
        );
        if let Some(card) = g.ux.alerts.entries.first_mut() {
            card.direction = Some(format!(
                "AT {} MOUTH",
                crate::seats::arm_letter(&g.world, arm)
            ));
            card.cause = Some(Kind::Riveter);
            card.by = Some(crate::seats::seat_hue(&g.world, 1));
        }
        g.render();
        keep_frame(&g, "s1-foe-chip-and-swatch");
        let s = g.ui_scale();
        let card = g.route_bounds();
        let one = crate::seats::seat_colour(&g.world, 1);
        let two = crate::seats::seat_colour(&g.world, 2);
        let find = |colour: Color| {
            (card.x..card.x + card.w)
                .any(|x| (card.y..card.y + card.h).any(|y| g.canvas.get(x, y) == Some(colour)))
        };
        assert!(find(one) && find(two), "both foes' colours on the card");
        assert!(
            !labels(&g).iter().any(|l| l.starts_with("FOE ")),
            "no initials"
        );
        // The card's swatch in the corner.
        let alert = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::FocusAlert(Some(_))))
            .expect("alert card");
        assert_eq!(
            g.canvas.get(alert.x + alert.w - 3 * s, alert.y + 2 * s),
            Some(one)
        );
    }

    fn labels(g: &Game) -> Vec<String> {
        g.buttons.iter().map(|b| b.label.clone()).collect()
    }

    fn tide_labels(g: &Game) -> Vec<String> {
        g.buttons
            .iter()
            .filter(|b| matches!(b.action, Action::SetTide(_) | Action::Flood))
            .map(|b| b.label.clone())
            .collect()
    }

    #[test]
    fn the_station_owner_offers_every_arm_not_dry_and_a_flood() {
        let mut g = confluence();
        g.ux.preferences.tide_card = true;
        g.world.gate.owner = Some(0);
        g.render();
        keep_frame(&g, "tide-neutral-yours");
        assert_eq!(tide_labels(&g), ["DRY E", "DRY W", "DRY S", "FLOOD"]);
        // Four buttons stay inside the card.
        let card = g.route_bounds();
        for b in g
            .buttons
            .iter()
            .filter(|b| matches!(b.action, Action::SetTide(_) | Action::Flood))
        {
            assert!(b.x >= card.x && b.x + b.w <= card.x + card.w, "{}", b.label);
        }
        g.world.gate.tide = bw_sim::Tide::Open;
        g.world.gate.dry_arm = Arm(1);
        g.render();
        keep_frame(&g, "tide-w-dry-yours");
        assert_eq!(tide_labels(&g), ["DRY E", "DRY S", "FLOOD"]);
        assert!(
            g.buttons
                .iter()
                .any(|b| b.action == Action::SetTide(Arm(2)))
        );
    }

    #[test]
    fn a_violet_count_is_named_and_coloured_by_violet() {
        let mut g = confluence();
        hold_every_lane(&mut g, 2);
        let before = std::collections::BTreeMap::new();
        g.ux.alerts.observe(&g.world, &before);
        g.world.lane_hold[2] = 16 * 30;
        g.ux.alerts.observe(&g.world, &before);
        assert_eq!(g.hold_gauge(), Some((2, 16 * 30)));
        g.render();
        assert!(
            labels(&g)
                .iter()
                .any(|l| l.starts_with("PINK HOLDS BOTH LANES")),
            "{:?}",
            labels(&g)
        );
        keep_frame(&g, "violet-counts");
        assert!(
            labels(&g).iter().any(|l| l == "PINK WINS IN"),
            "{:?}",
            labels(&g)
        );
        // The focus goes to a mouth violet stands at, on one of its lanes.
        let focus = g.hold_focus_mouth().expect("focus");
        assert!(
            g.world
                .arms_of(2)
                .into_iter()
                .any(|arm| g.world.own_mouth(2, arm) == Some(focus))
        );
        // A broken count drains under the same name.
        g.world.gate.owner = Some(1);
        assert!(!g.world.holds_every_lane(2));
        g.render();
        keep_frame(&g, "violet-drains");
        assert!(
            labels(&g).iter().any(|l| l == "PINK DRAINING"),
            "{:?}",
            labels(&g)
        );
    }

    #[test]
    fn your_own_count_and_single_lane_read_with_your_arm_letters() {
        let mut g = confluence();
        g.world.gate.owner = Some(0);
        let arm = g.world.arms_of(0)[0];
        let mouth = g.world.own_mouth(0, arm).expect("mouth");
        g.world.spawn_for_tests(0, Kind::Bulwark, mouth);
        assert_eq!(
            g.crossing_held().as_deref(),
            Some("YOU HOLD E LANE / TAKE W")
        );
        g.camera.center(mouth);
        g.render();
        keep_frame(&g, "you-hold-e");
        hold_every_lane(&mut g, 0);
        g.world.lane_hold[0] = 30 * 30;
        g.render();
        keep_frame(&g, "you-count");
        assert!(labels(&g).iter().any(|l| l == "YOU WIN IN"));
    }

    #[test]
    fn a_red_lane_hold_is_named_for_red() {
        let mut g = confluence();
        g.world.gate.owner = Some(1);
        // Red at its mouth of the east arm, which it shares with you.
        let mouth = g.world.own_mouth(1, 0).expect("mouth");
        g.world.spawn_for_tests(1, Kind::Bulwark, mouth);
        assert_eq!(g.crossing_held().as_deref(), Some("VIOLET HOLDS E LANE"));
        g.camera.center(mouth);
        g.render();
        keep_frame(&g, "red-holds-e");
    }

    #[test]
    fn confluence_mouths_are_named_by_arm_and_bank() {
        let mut g = confluence();
        g.world.lane_hold[2] = 30 * 30;
        let label = |g: &Game, lane, mouth, holder, contested| {
            crate::tactics::seat_crossing_label(&g.world, lane, mouth, holder, contested)
        };
        assert_eq!(label(&g, 0, 0, None, false), "E YOUR BANK");
        assert_eq!(label(&g, 2, 0, None, false), "S VIOLET'S BANK");
        assert_eq!(label(&g, 1, 1, Some(2), false), "W PINK'S BANK: PINK 45S");
        assert_eq!(label(&g, 0, 1, Some(1), true), "E VIOLET'S BANK: CONTESTED");
        g.world.lane_hold[0] = 0;
        assert_eq!(label(&g, 0, 0, Some(0), false), "E YOUR BANK: HELD 75S");
    }

    #[test]
    fn the_small_window_route_reads_every_arm() {
        let mut g = confluence();
        g.resize_view(640, 360);
        g.world.gate.tide = bw_sim::Tide::Open;
        g.world.gate.dry_arm = Arm(0);
        g.world.gate.switch_target = Some(Arm(2));
        g.world.gate.warning_until = Some(g.world.tick + 200);
        let route = g.console_state().route;
        assert_eq!(route.dry_arm, Arm(0));
        assert_eq!(route.target_dry_arm, Some(Arm(2)));
        assert_eq!(route.map, MapId::Confluence);
        g.render();
        if let Ok(dir) = std::env::var("BW_THREE_SEAT_FRAMES") {
            g.canvas
                .save(&PathBuf::from(dir).join("small-window-route.png"))
                .expect("frame");
        }
    }

    #[test]
    fn the_split_basin_keeps_its_two_tide_buttons() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new(base);
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1920, 1080);
        g.ux.preferences.tide_card = true;
        g.world.gate.owner = Some(0);
        g.render();
        keep_frame(&g, "basin-neutral-yours");
        assert_eq!(tide_labels(&g), ["DRY N", "DRY S", "FLOOD"]);
    }
}
