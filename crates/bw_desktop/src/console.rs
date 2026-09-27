//! Dockmaster console composition for the 640×360 desktop view.
//!
//! The renderer consumes a small presentation snapshot rather than a
//! `bw_sim::World`.  This keeps the UI truthful by making the rendering agent
//! choose each value from authoritative state, while keeping layout and text
//! geometry independent of simulation types.  The command buttons remain
//! owned by `game.rs`; this module leaves their existing hit rectangles alone.

use crate::canvas::{
    Atlas, Canvas, Color, EDGE, GOLD, INK, JADE, MUTED, PANEL, WHITE, text_readable_width,
};
use bw_core::Faction;

pub const CONSOLE_Y: i32 = 288;
pub const CONSOLE_H: i32 = 72;
pub const ROUTE_X: i32 = 8;
pub const ROUTE_Y: i32 = 28;
pub const ROUTE_W: i32 = 212;
pub const ROUTE_H: i32 = 24;

/// Values shown in the top resource header.  `pressure_rate_per_minute` and
/// `pressure_cap` are optional because the authoritative adapter may not have
/// a meaningful rate/cap in a fixture or an older save.  The pressure number
/// itself is always rendered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleResources {
    pub salvage: u32,
    pub pressure: u32,
    /// Salvage brought home over the last minute.
    pub salvage_rate_per_minute: Option<u32>,
    pub pressure_rate_per_minute: Option<u32>,
    pub pressure_cap: Option<u32>,
    pub crew: u32,
    pub crew_cap: u32,
}

/// Current and committed future gate lanes.  `warning_ticks_remaining` is a
/// simulation tick count, so the UI can use the same 30 Hz rounding as the
/// game status without inventing a wall-clock timer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleRoute {
    pub tide: bw_sim::Tide,
    pub flood_pending: bool,
    /// The running warning is a DRY's ebb back to neutral (rules 22).
    pub ebb_pending: bool,
    pub flood_ticks_remaining: Option<u64>,
    /// The arm that is dry while the tide is open (`Gate::dry_arm`); on the
    /// Split Basin arm 0 is the north.
    pub dry_arm: bw_sim::Arm,
    /// The arm a committed switch will dry (`Gate::switch_target`).
    pub target_dry_arm: Option<bw_sim::Arm>,
    /// The map: how many arms the tide has and their letters.
    pub map: bw_sim::MapId,
    pub warning_ticks_remaining: Option<u64>,
}

impl ConsoleRoute {
    /// Whether the Split Basin's north arm is the dry one.
    pub fn current_north_dry(&self) -> bool {
        self.dry_arm.is_north()
    }
}

/// One authoritative production queue entry.  `remaining_ticks` and
/// `total_ticks` are the actual queue counters; they are deliberately not a
/// presentation animation timer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProductionTicket {
    pub label: String,
    pub remaining_ticks: u32,
    pub total_ticks: u32,
    pub started: bool,
}

/// Selected-unit/building data needed by the lower console.  `portrait_key`
/// should be an existing atlas key such as `portrait_riveter`; no UI art is
/// generated here.  A building may provide its queue in `queue`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleSelection {
    pub title: String,
    /// One line on what the selected kind does; empty for groups.
    pub role: String,
    /// Damage, reach, speed and sight; empty for groups.
    pub stats: String,
    pub portrait_key: Option<String>,
    pub selected_count: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub status: String,
    pub queue: Vec<ProductionTicket>,
    pub show_queue: bool,
}

/// Complete immutable snapshot consumed by [`draw_console`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsoleState {
    pub faction: Faction,
    pub elapsed_seconds: u64,
    pub resources: ConsoleResources,
    pub route: ConsoleRoute,
    pub selection: Option<ConsoleSelection>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            faction: Faction::Union,
            elapsed_seconds: 0,
            resources: ConsoleResources::default(),
            route: ConsoleRoute::default(),
            selection: None,
        }
    }
}

/// Draw the header, gate forecast, and lower console.  The minimap and the
/// command buttons are intentionally outside this function so their existing
/// ownership and input geometry stay in `game.rs`.
#[allow(dead_code)]
pub fn draw_console(canvas: &mut Canvas, atlas: Option<&Atlas>, state: &ConsoleState) {
    draw_top_header(canvas, atlas, state);
    draw_route_indicator(canvas, &state.route);
    draw_bottom_console(canvas, atlas, state);
}

/// Draw only the 24-pixel resource header.
pub fn draw_top_header(canvas: &mut Canvas, atlas: Option<&Atlas>, state: &ConsoleState) {
    let backdrop_drawn = draw_faction_backdrop(
        canvas,
        atlas,
        state.faction,
        ("hud_top_union", "hud_top_assembly", "hud_top_compact"),
        "hud_top",
        BackdropRect {
            x: 0,
            y: 0,
            w: 640,
            h: 24,
            fallback_color: INK,
        },
    );
    if !backdrop_drawn {
        canvas.rect(0, 0, 640, 24, INK);
        canvas.rect(0, 23, 640, 1, EDGE);
    }

    // Keep this header intentionally explicit. The first row names each
    // resource and the second row carries its exact authoritative value;
    // players should not have to decode S/P/C abbreviations while learning.
    // The pressure sight glass remains beside its number and only fills when
    // the adapter supplies a real cap.
    let _ = atlas;
    canvas.text_readable("BRINEWAKE", 8, 2, WHITE);
    canvas.text_readable("SALVAGE", 104, 2, MUTED);
    canvas.text_readable("PRESSURE", 232, 2, MUTED);
    canvas.text_readable("CREW", 432, 2, MUTED);
    canvas.text_readable("TIME", 552, 2, MUTED);

    let salvage = state.resources.salvage.to_string();
    draw_readable_or_legacy(canvas, &salvage, 104, 13, 112, GOLD);

    draw_pressure_sight_glass(
        canvas,
        396,
        4,
        state.resources.pressure,
        state.resources.pressure_cap,
    );
    let pressure = pressure_value(&state.resources);
    draw_readable_or_legacy(canvas, &pressure, 232, 13, 156, JADE);

    let crew = format!("{}/{}", state.resources.crew, state.resources.crew_cap);
    draw_readable_or_legacy(canvas, &crew, 432, 13, 112, WHITE);

    let seconds = state.elapsed_seconds;
    let clock = format!("{:02}:{:02}", seconds / 60, seconds % 60);
    canvas.text_readable(&clock, 552, 13, MUTED);
}

/// Draw the N/S route truth and, during the ten-second warning, the committed
/// destination plus an exact simulation-tick countdown.  Current states stay
/// on the left of each arrow so the panel never suggests that the switch has
/// already happened.
pub fn draw_route_indicator(canvas: &mut Canvas, route: &ConsoleRoute) {
    canvas.rect(ROUTE_X, ROUTE_Y, ROUTE_W, ROUTE_H, INK);
    canvas.frame(ROUTE_X, ROUTE_Y, ROUTE_W, ROUTE_H, EDGE);
    if route.map.layout().arm_count() > 2 {
        draw_arm_routes(canvas, route);
        return;
    }
    let current_north_dry = route.current_north_dry();

    let now = crate::lane_memory::depths(route.tide, current_north_dry);
    let current_n = depth_word(now[0]);
    let current_s = depth_word(now[1]);
    let warning = route.warning_ticks_remaining.is_some();
    let target = if route.flood_pending {
        Some([bw_sim::Depth::Deep, bw_sim::Depth::Deep])
    } else if route.ebb_pending {
        Some([bw_sim::Depth::Shallow, bw_sim::Depth::Shallow])
    } else {
        route
            .target_dry_arm
            .map(|arm| crate::lane_memory::depths(bw_sim::Tide::Open, arm.is_north()))
    };
    let target_n = target.map(|t| depth_word(t[0])).unwrap_or("?");
    let target_s = target.map(|t| depth_word(t[1])).unwrap_or("?");

    if warning {
        let n_line = format!("N {current_n} > {target_n}");
        let s_line = format!("S {current_s} > {target_s}");
        canvas.text_readable(
            &n_line,
            ROUTE_X + 6,
            ROUTE_Y + 3,
            route_color(current_north_dry),
        );
        canvas.text_readable(
            &s_line,
            ROUTE_X + 6,
            ROUTE_Y + 13,
            route_color(!current_north_dry),
        );
        let seconds = route
            .warning_ticks_remaining
            .unwrap_or_default()
            .div_ceil(30);
        let countdown = format!("CHG {seconds}S");
        let x = ROUTE_X + ROUTE_W - text_readable_width(&countdown) - 6;
        canvas.text_readable(&countdown, x, ROUTE_Y + 8, GOLD);
    } else {
        let n_line = format!("N {current_n}");
        let s_line = format!("S {current_s}");
        canvas.text_readable(
            &n_line,
            ROUTE_X + 6,
            ROUTE_Y + 3,
            route_color(current_north_dry),
        );
        canvas.text_readable(
            &s_line,
            ROUTE_X + 6,
            ROUTE_Y + 13,
            route_color(!current_north_dry),
        );
        canvas.text_readable("STABLE", ROUTE_X + ROUTE_W - 54, ROUTE_Y + 8, MUTED);
    }
}

/// Every arm of a map with three: one column per arm, its letter and the
/// water on it now, and during the warning the water it will have.
fn draw_arm_routes(canvas: &mut Canvas, route: &ConsoleRoute) {
    let arms = route.map.layout().arms;
    let depth = |tide: bw_sim::Tide, dry: bw_sim::Arm, arm: usize| match tide {
        bw_sim::Tide::Neutral => bw_sim::Depth::Shallow,
        bw_sim::Tide::Flood => bw_sim::Depth::Deep,
        bw_sim::Tide::Open if dry.index() == arm => bw_sim::Depth::Dry,
        bw_sim::Tide::Open => bw_sim::Depth::Deep,
    };
    let short = |depth: bw_sim::Depth| match depth {
        bw_sim::Depth::Dry => "DRY",
        bw_sim::Depth::Shallow => "SHAL",
        bw_sim::Depth::Deep => "DEEP",
    };
    let warning = route.warning_ticks_remaining.is_some();
    for (arm, info) in arms.iter().enumerate() {
        let x = ROUTE_X + 6 + arm as i32 * 48;
        let now = depth(route.tide, route.dry_arm, arm);
        canvas.text(
            &format!("{} {}", info.name, short(now)),
            x,
            ROUTE_Y + 3,
            route_color(now == bw_sim::Depth::Dry),
        );
        if !warning {
            continue;
        }
        let next = if route.flood_pending {
            Some(bw_sim::Depth::Deep)
        } else if route.ebb_pending {
            Some(bw_sim::Depth::Shallow)
        } else {
            route
                .target_dry_arm
                .map(|target| depth(bw_sim::Tide::Open, target, arm))
        };
        let word = next.map_or("?", short);
        canvas.text(
            &format!("> {word}"),
            x,
            ROUTE_Y + 13,
            route_color(next == Some(bw_sim::Depth::Dry)),
        );
    }
    if warning {
        let seconds = route
            .warning_ticks_remaining
            .unwrap_or_default()
            .div_ceil(30);
        let countdown = format!("CHG {seconds}S");
        let x = ROUTE_X + ROUTE_W - text_readable_width(&countdown) - 6;
        canvas.text_readable(&countdown, x, ROUTE_Y + 8, GOLD);
    } else {
        canvas.text_readable("STABLE", ROUTE_X + ROUTE_W - 54, ROUTE_Y + 13, MUTED);
    }
}

/// Draw only the lower 72-pixel console.  The minimap is intentionally left
/// to the rendering agent because it needs camera and fog state.
pub fn draw_bottom_console(canvas: &mut Canvas, atlas: Option<&Atlas>, state: &ConsoleState) {
    let has_faction_art = draw_faction_backdrop(
        canvas,
        atlas,
        state.faction,
        (
            "hud_bottom_union",
            "hud_bottom_assembly",
            "hud_bottom_compact",
        ),
        "hud_bottom",
        BackdropRect {
            x: 0,
            y: CONSOLE_Y,
            w: 640,
            h: CONSOLE_H,
            fallback_color: INK,
        },
    );
    if !has_faction_art {
        canvas.rect(0, CONSOLE_Y, 640, CONSOLE_H, INK);
        canvas.rect(0, CONSOLE_Y, 640, 1, EDGE);
    }

    if let Some(atlas) = atlas {
        let portrait = state
            .selection
            .as_ref()
            .and_then(|selection| selection.portrait_key.as_deref())
            .unwrap_or(match state.faction {
                Faction::Union => "portrait_union",
                Faction::Assembly => "portrait_assembly",
                Faction::Compact => "portrait_compact",
            });
        atlas.draw(canvas, portrait, 138, 296, false);
        let badge = match state.faction {
            Faction::Union => "faction_union_badge",
            Faction::Assembly => "faction_assembly_badge",
            Faction::Compact => "faction_compact_badge",
        };
        atlas.draw(canvas, badge, 177, 341, false);
    }

    match &state.selection {
        Some(selection) => draw_selection(canvas, selection),
        None => draw_empty_selection(canvas),
    }
}

/// Draw an amber sight glass next to the exact pressure number.  The gauge
/// fill is only normalized when a cap is supplied; without a cap the glass
/// remains outlined so it cannot imply a fabricated maximum.
pub fn draw_pressure_sight_glass(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    pressure: u32,
    cap: Option<u32>,
) {
    canvas.rect(x, y, 10, 16, INK);
    canvas.frame(x, y, 10, 16, GOLD);
    canvas.rect(x + 3, y + 2, 4, 12, PANEL);
    if let Some(cap) = cap.filter(|cap| *cap > 0) {
        let filled = (u64::from(pressure.min(cap)) * 12 / u64::from(cap)) as i32;
        if filled > 0 {
            canvas.rect(x + 3, y + 14 - filled, 4, filled, GOLD);
        }
    }
    canvas.line(x + 1, y + 1, x + 8, y + 1, WHITE);
}

fn draw_selection(canvas: &mut Canvas, selection: &ConsoleSelection) {
    let title = selection.title.trim();
    if title.is_empty() {
        canvas.text_readable("SELECTION", 201, 296, WHITE);
    } else {
        draw_readable_fit(canvas, title, 201, 296, 203, WHITE);
    }

    let max_hp = selection.max_hp.max(0);
    let hp = selection.hp.max(0);
    let stats = format!(
        "{} SELECTED / HP {}/{}",
        selection.selected_count, hp, max_hp
    );
    draw_legacy_fit(canvas, &stats, 201, 309, 203, MUTED);

    let status = if selection.status.trim().is_empty() {
        "QUEUE"
    } else {
        selection.status.trim()
    };
    draw_legacy_fit(canvas, status, 201, 320, 203, GOLD);
    if selection.show_queue {
        draw_queue(canvas, &selection.queue);
    }
}

fn draw_empty_selection(canvas: &mut Canvas) {
    canvas.text_readable("SELECT A MACHINE", 201, 296, WHITE);
    canvas.text("LEFT CLICK OR DRAG TO SELECT", 201, 313, MUTED);
    canvas.text("RIGHT CLICK ORDERS / ESC CANCELS", 201, 327, MUTED);
}

fn draw_queue(canvas: &mut Canvas, queue: &[ProductionTicket]) {
    if queue.is_empty() {
        return;
    }

    // Keep the strip inside the information column (x=201..403).  Three
    // wider tickets leave a dedicated, exact-count overflow marker before
    // the command buttons begin at x=414.
    const CARD_W: i32 = 58;
    const CARD_STRIDE: i32 = 60;
    const MORE_X: i32 = 383;
    let visible = queue.len().min(3);
    for (index, ticket) in queue.iter().take(visible).enumerate() {
        let x = 201 + index as i32 * CARD_STRIDE;
        let active = index == 0 && ticket.started;
        let border = if active { GOLD } else { EDGE };
        canvas.rect(x, 329, CARD_W, 17, PANEL);
        canvas.frame(x, 329, CARD_W, 17, border);
        let code = ticket_code(&ticket.label);
        canvas.text(&code, x + 2, 331, if active { WHITE } else { MUTED });
        let remaining = ticket_time(ticket);
        canvas.text(&remaining, x + 39, 331, if active { GOLD } else { EDGE });

        let progress = ticket_progress(ticket);
        let width = (54 * progress / 100) as i32;
        canvas.rect(x + 2, 342, 54, 2, INK);
        if width > 0 {
            canvas.rect(x + 2, 342, width, 2, if active { GOLD } else { JADE });
        }
    }
    if queue.len() > visible {
        let more = format!("+{}", queue.len() - visible);
        canvas.text(&more, MORE_X, 334, MUTED);
    }
}

#[derive(Clone, Copy)]
struct BackdropRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fallback_color: Color,
}

fn draw_faction_backdrop(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    faction: Faction,
    faction_keys: (&str, &str, &str),
    fallback_key: &str,
    rect: BackdropRect,
) -> bool {
    if let Some(atlas) = atlas {
        let faction_key = match faction {
            Faction::Union => faction_keys.0,
            Faction::Assembly => faction_keys.1,
            Faction::Compact => faction_keys.2,
        };
        if atlas.draw(canvas, faction_key, rect.x, rect.y, false) {
            return true;
        }
        if atlas.draw(canvas, fallback_key, rect.x, rect.y, false) {
            return true;
        }
    }
    canvas.rect(rect.x, rect.y, rect.w, rect.h, rect.fallback_color);
    false
}

fn draw_readable_fit(canvas: &mut Canvas, text: &str, x: i32, y: i32, width: i32, color: Color) {
    if text_readable_width(text) <= width {
        canvas.text_readable(text, x, y, color);
        return;
    }
    // A visible period marks the deliberate abbreviation; a raw crop would
    // make a long title look like a different value.
    let max_chars = (width / 8).max(4) as usize;
    let take = max_chars.saturating_sub(3);
    let mut short = text
        .to_ascii_uppercase()
        .chars()
        .take(take)
        .collect::<String>();
    short.push_str("...");
    canvas.text_readable(&short, x, y, color);
}

fn draw_readable_or_legacy(
    canvas: &mut Canvas,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    color: Color,
) {
    if text_readable_width(text) <= width {
        canvas.text_readable(text, x, y, color);
    } else {
        // The small face preserves exact large resource numbers when a test
        // fixture intentionally exceeds the normal in-game range.
        canvas.text(text, x, y + 1, color);
    }
}

fn draw_legacy_fit(canvas: &mut Canvas, text: &str, x: i32, y: i32, width: i32, color: Color) {
    let max_chars = (width / 6).max(1) as usize;
    let upper = text.to_ascii_uppercase();
    if upper.chars().count() <= max_chars {
        canvas.text(&upper, x, y, color);
        return;
    }
    let take = max_chars.saturating_sub(3);
    let mut short = upper.chars().take(take).collect::<String>();
    short.push_str("...");
    canvas.text(&short, x, y, color);
}

#[cfg(test)]
fn pressure_readout(resources: &ConsoleResources) -> String {
    let mut value = match resources.pressure_cap {
        Some(cap) => format!("P {}/{}", resources.pressure, cap),
        None => format!("P {}", resources.pressure),
    };
    if resources
        .pressure_cap
        .is_some_and(|cap| resources.pressure >= cap)
    {
        value.push_str(" FULL");
    } else if let Some(rate) = resources.pressure_rate_per_minute {
        value.push_str(&format!(" +{rate}/M"));
    }
    value
}

fn pressure_value(resources: &ConsoleResources) -> String {
    let mut value = match resources.pressure_cap {
        Some(cap) => format!("{}/{}", resources.pressure, cap),
        None => resources.pressure.to_string(),
    };
    if resources
        .pressure_cap
        .is_some_and(|cap| resources.pressure >= cap)
    {
        value.push_str(" FULL");
    } else if let Some(rate) = resources.pressure_rate_per_minute {
        value.push_str(&format!(" +{rate}/M"));
    }
    value
}

fn depth_word(depth: bw_sim::Depth) -> &'static str {
    match depth {
        bw_sim::Depth::Dry => "DRY",
        bw_sim::Depth::Shallow => "SHALLOW",
        bw_sim::Depth::Deep => "DEEP",
    }
}

#[allow(dead_code)]
fn lane_word(north_dry: bool) -> &'static str {
    if north_dry { "DRY" } else { "FLOOD" }
}

fn route_color(dry: bool) -> Color {
    if dry { JADE } else { MUTED }
}

fn div_ceil_30(ticks: u32) -> u32 {
    ticks.saturating_add(29) / 30
}

fn ticket_time(ticket: &ProductionTicket) -> String {
    if !ticket.started {
        "--".into()
    } else if ticket.remaining_ticks <= 1 {
        "RDY".into()
    } else {
        format!("{}S", div_ceil_30(ticket.remaining_ticks))
    }
}

fn ticket_progress(ticket: &ProductionTicket) -> u32 {
    if !ticket.started {
        return 0;
    }
    if ticket.remaining_ticks <= 1 {
        return 100;
    }
    if ticket.total_ticks == 0 {
        return u32::from(ticket.remaining_ticks == 0) * 100;
    }
    let remaining = ticket.remaining_ticks.min(ticket.total_ticks);
    ((u64::from(ticket.total_ticks - remaining) * 100) / u64::from(ticket.total_ticks)) as u32
}

fn ticket_code(label: &str) -> String {
    let upper = label.to_ascii_uppercase();
    if upper.chars().count() <= 5 {
        return upper;
    }
    let mut code = upper.chars().take(4).collect::<String>();
    code.push('.');
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    #[test]
    fn completed_waiting_ticket_and_capped_pressure_make_no_time_or_gain_promise() {
        let ticket = ProductionTicket {
            remaining_ticks: 1,
            total_ticks: 300,
            started: true,
            label: "LOOM".into(),
        };
        assert_eq!(ticket_time(&ticket), "RDY");
        assert_eq!(ticket_progress(&ticket), 100);
        let resources = ConsoleResources {
            pressure: 300,
            pressure_cap: Some(300),
            salvage_rate_per_minute: Some(120),
            pressure_rate_per_minute: Some(60),
            ..Default::default()
        };
        assert_eq!(pressure_readout(&resources), "P 300/300 FULL");
        assert_eq!(pressure_value(&resources), "300/300 FULL");
    }

    #[test]
    fn warning_route_contains_current_and_future_lines() {
        let render = |route: ConsoleRoute| {
            let mut canvas = Canvas::default();
            canvas.clear(INK);
            draw_route_indicator(&mut canvas, &route);
            canvas
        };
        let warning = render(ConsoleRoute {
            dry_arm: bw_sim::Arm::NORTH,
            target_dry_arm: Some(bw_sim::Arm::SOUTH),
            warning_ticks_remaining: Some(31),
            ..Default::default()
        });
        let stable = render(ConsoleRoute {
            dry_arm: bw_sim::Arm::NORTH,
            target_dry_arm: None,
            warning_ticks_remaining: None,
            ..Default::default()
        });
        // Compare the text interior instead of the shared frame pixels.  A
        // future target and countdown must change the warning composition.
        assert!(route_band_differs(&warning, &stable));

        let opposite_target = render(ConsoleRoute {
            dry_arm: bw_sim::Arm::NORTH,
            target_dry_arm: Some(bw_sim::Arm::NORTH),
            warning_ticks_remaining: Some(31),
            ..Default::default()
        });
        assert!(route_band_differs(&warning, &opposite_target));
    }

    #[test]
    fn three_arm_routes_show_the_warning_and_its_target() {
        let render = |route: ConsoleRoute| {
            let mut canvas = Canvas::default();
            canvas.clear(INK);
            draw_route_indicator(&mut canvas, &route);
            canvas
        };
        let open = |target: Option<bw_sim::Arm>, warning: Option<u64>| ConsoleRoute {
            tide: bw_sim::Tide::Open,
            dry_arm: bw_sim::Arm(0),
            target_dry_arm: target,
            map: bw_sim::MapId::Confluence,
            warning_ticks_remaining: warning,
            ..Default::default()
        };
        let stable = render(open(None, None));
        let to_w = render(open(Some(bw_sim::Arm(1)), Some(31)));
        let to_s = render(open(Some(bw_sim::Arm(2)), Some(31)));
        assert!(route_band_differs(&stable, &to_w));
        assert!(route_band_differs(&to_w, &to_s));
        // The two-arm drawing is not the one used for three arms.
        let basin = render(ConsoleRoute {
            tide: bw_sim::Tide::Open,
            ..Default::default()
        });
        assert!(route_band_differs(&stable, &basin));
    }

    fn route_band_differs(a: &Canvas, b: &Canvas) -> bool {
        (ROUTE_Y + 1..ROUTE_Y + ROUTE_H - 1).any(|y| {
            (ROUTE_X + 1..ROUTE_X + ROUTE_W - 1).any(|x| {
                let i = (y as usize * 640 + x as usize) * 4;
                a.pixels[i..i + 4] != b.pixels[i..i + 4]
            })
        })
    }

    #[test]
    fn ticket_progress_uses_authoritative_remaining_ticks() {
        assert_eq!(ticket_progress(&ProductionTicket::default()), 0);
        assert_eq!(
            ticket_progress(&ProductionTicket {
                label: "RIVETER".into(),
                remaining_ticks: 50,
                total_ticks: 100,
                started: true,
            }),
            50
        );
        assert_eq!(
            ticket_progress(&ProductionTicket {
                label: "RIVETER".into(),
                remaining_ticks: 0,
                total_ticks: 0,
                started: true,
            }),
            100
        );
    }

    #[test]
    fn ticket_labels_mark_abbreviation_instead_of_raw_cropping() {
        assert_eq!(ticket_code("HOOK"), "HOOK");
        assert_eq!(ticket_code("RIVETER"), "RIVE.");
        assert_eq!(ticket_code("HEADQUARTERS"), "HEAD.");
    }

    #[test]
    fn queue_strip_stays_inside_info_column_for_full_queue() {
        let mut canvas = Canvas::default();
        canvas.clear(INK);
        let queue = (0..16)
            .map(|index| ProductionTicket {
                label: format!("HEADQUARTERS-{index}"),
                remaining_ticks: 90,
                total_ticks: 120,
                started: index == 0,
            })
            .collect::<Vec<_>>();
        draw_queue(&mut canvas, &queue);

        for y in 329..346 {
            for x in 404..640 {
                let i = (y as usize * 640 + x as usize) * 4;
                assert_eq!(
                    &canvas.pixels[i..i + 4],
                    &INK,
                    "queue pixel crossed the info-column boundary at ({x},{y})"
                );
            }
        }
    }
}
