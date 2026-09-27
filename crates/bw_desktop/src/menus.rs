//! Menu layouts on an integer-scaled interface grid, independent of the world.
//!
//! The menu renderer owns copy, layout, and the use of existing atlas art. It
//! deliberately does not draw buttons: `game.rs` owns focus, hover, and the
//! final button treatment. Each screen returns the hit rectangles needed by
//! that renderer.
//!
//! Copy rule: a control is its label. A hint appears only when it carries
//! state the player needs before choosing: the session clock, the saved
//! slot, or a setting's current value. Screens carry no kickers, taglines,
//! or repeated key legends.

use crate::audio::Bus;
use crate::canvas::{Atlas, COBALT, Canvas, Color, EDGE, GOLD, INK, JADE, MUTED, RED, WHITE};
use crate::dock_log::DockState;
use crate::field_manual::{self, ManualState, ManualSubject};
use crate::game::{Action, Button};
use bw_core::{Faction, Kind};
use bw_sim::Outcome;

const SCREEN_W: i32 = 640;
const MARGIN: i32 = 24;
/// One readable line, centred by the button renderer. Rows this short show
/// any hint on the same line, right aligned, where it reads as a value.
const ROW_H: i32 = 22;
const ROW_STEP: i32 = 28;

/// Plain presentation state needed by every first-player screen.
///
/// This is intentionally owned data so menu fixtures can be built without a
/// live `Game` or `World`. The game integration layer should refresh it from
/// authoritative state before calling one of the screen functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuModel {
    pub always_health_bars: bool,
    pub selected_faction: Faction,
    pub selected_map: bw_sim::MapId,
    /// The computer seats' faction, `None` for random.
    pub selected_opponent: Option<Faction>,
    pub selected_ai_level: bw_sim::AiLevel,
    pub has_session: bool,
    /// A two-player lockstep match, which keeps running while this seat reads
    /// a menu.
    pub network_session: bool,
    pub practice_session: bool,
    pub practice_complete: bool,
    pub practice_review: bool,
    pub session_seconds: u64,
    pub save_available: bool,
    pub save_description: String,
    pub audio_on: bool,
    pub effects_volume: u8,
    pub ambience_volume: u8,
    pub music_volume: u8,
    pub edge_scroll: bool,
    pub pause_unfocused: bool,
    pub game_speed: crate::tempo::GameSpeed,
    pub paused_unfocused: bool,
    pub fullscreen: bool,
    pub tutorial_completed: bool,
    pub help_page: usize,
    /// The faction whose roster the Guide shows.
    pub roster_faction: Faction,
    /// The roster shows the shared buildings instead of a faction.
    pub roster_buildings: bool,
    /// The roster card whose details show.
    pub roster_unit: Option<Kind>,
    /// Interface scale preference: 0 automatic, 1 to 4 fixed.
    pub interface_scale: u8,
    /// The open Settings tab.
    pub settings_tab: u8,
    /// Menu-only animation clock in render frames; never a simulation tick.
    pub home_tick: u64,
    pub confirmation_title: String,
    pub confirmation_body: String,
    pub confirmation_action: String,
    pub can_save_before_confirm: bool,
    /// The match is still on screen: pause and confirm draw as panels over
    /// the dimmed field instead of a full screen.
    pub overlay: bool,
    /// The quick save holds this exact field.
    pub just_saved: bool,
    /// How far an ended practice got: steps done, steps in all, and the
    /// name of the step that was next.
    pub practice_progress: (usize, usize, String),
    pub surrendered: bool,
    /// Presentation-only field-guide clock. It never mirrors `World::tick`.
    pub manual: ManualState,
    pub dock: DockState,
    /// The match summary the result screen draws.
    pub summary: crate::match_summary::MatchSummary,
    pub result_page: crate::dock_log::ResultPage,
    pub result_log_scroll: usize,
    /// Watching a recording: the result names the winner, not "you".
    pub observing: bool,
    /// The hold length in force in the match on screen, from the rule
    /// (`World::hold_ticks`): the Guide states it instead of the map's
    /// opening value once it differs (120 s after a seat is out).
    pub hold_seconds: Option<u32>,
}

impl Default for MenuModel {
    fn default() -> Self {
        Self {
            always_health_bars: false,
            selected_faction: Faction::Union,
            selected_map: bw_sim::MapId::SplitBasin,
            selected_opponent: Some(Faction::Assembly),
            selected_ai_level: bw_sim::AiLevel::Normal,
            has_session: false,
            network_session: false,
            practice_session: false,
            practice_complete: false,
            practice_review: false,
            session_seconds: 0,
            save_available: false,
            save_description: "No saved match yet".into(),
            audio_on: true,
            effects_volume: 75,
            ambience_volume: 45,
            music_volume: 35,
            edge_scroll: true,
            pause_unfocused: true,
            game_speed: Default::default(),
            paused_unfocused: false,
            fullscreen: false,
            tutorial_completed: false,
            help_page: 0,
            roster_faction: Faction::Union,
            roster_buildings: false,
            roster_unit: None,
            interface_scale: 0,
            settings_tab: 0,
            home_tick: 0,
            confirmation_title: "CONTINUE?".into(),
            confirmation_body: String::new(),
            confirmation_action: "CONTINUE".into(),
            can_save_before_confirm: false,
            overlay: false,
            just_saved: false,
            practice_progress: (0, 7, String::new()),
            surrendered: false,
            manual: ManualState::default(),
            dock: DockState::default(),
            summary: Default::default(),
            result_page: Default::default(),
            result_log_scroll: 0,
            observing: false,
            hold_seconds: None,
        }
    }
}

fn session_label(model: &MenuModel) -> String {
    format!(
        "{} {}",
        if model.practice_session {
            "PRACTICE"
        } else {
            "SKIRMISH"
        },
        format_clock(model.session_seconds)
    )
}

fn save_hint(model: &MenuModel) -> &str {
    if model.save_available {
        &model.save_description
    } else {
        ""
    }
}

/// Draw the home screen and return its navigable controls.  The canvas is
/// left clear around the menu: the renderer draws the field behind it.
pub fn home(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    canvas.clear([0, 0, 0, 0]);
    canvas.rect(0, 0, SCREEN_W, 2, GOLD);
    // The wordmark, large, over a gold rule.
    canvas.text_readable_scaled("BRINEWAKE", 40, 30, WHITE, 3);
    canvas.rect(42, 62, 48, 2, GOLD);
    let playable = atlas.is_some();
    let session = session_label(model);
    let mut rows: Vec<(&str, &str, Action, bool)> = Vec::with_capacity(8);
    if model.has_session {
        rows.push(("CONTINUE", session.as_str(), Action::Resume, true));
    }
    rows.push(("LEARN TO PLAY", "", Action::StartPractice, playable));
    rows.push(("NEW SKIRMISH", "", Action::Setup, playable));
    // A match online is joined from here; one already running is continued
    // above instead.
    if !(model.has_session && model.network_session) {
        rows.push(("PLAY ONLINE", "", Action::Online, playable));
    }
    // LOAD shows once there is something to load.
    if model.save_available {
        rows.push(("LOAD", save_hint(model), Action::Load, true));
    }
    rows.push(("GUIDE", "", Action::Help, true));
    rows.push(("SETTINGS", "", Action::Settings, true));
    rows.push(("QUIT", "", Action::Quit, true));
    let buttons = rows
        .into_iter()
        .enumerate()
        .map(|(index, (label, hint, action, enabled))| {
            menu_button(
                [40, 88 + index as i32 * ROW_STEP, 250, ROW_H],
                label,
                hint,
                action,
                enabled,
            )
        })
        .collect();
    if !playable {
        canvas.text_readable("ART ASSETS UNAVAILABLE", 360, 152, RED);
    }
    // The build, small in the corner.
    let build = concat!("V", env!("CARGO_PKG_VERSION"));
    canvas.text(build, SCREEN_W - 12 - build.len() as i32 * 6, 344, MUTED);
    buttons
}

/// Top of the two faction cards, the row that picks the computer, and the
/// two map cards on the New Skirmish screen.
const SETUP_CARD_Y: i32 = 62;
const SETUP_CARD_W: i32 = 292;
const SETUP_CARD_H: i32 = 70;
const SETUP_ROW_Y: i32 = 158;
const SETUP_MAP_Y: i32 = 204;
const SETUP_MAP_H: i32 = 62;
const SETUP_WIN_Y: i32 = 280;
const SETUP_FOOT_Y: i32 = 316;
const SETUP_THUMB_W: i32 = 104;

fn setup_card_x(i: usize) -> i32 {
    MARGIN + i as i32 * 300
}

/// The three faction cards share the row the two map cards span.
const SETUP_FACTION_W: i32 = 192;

fn faction_card_x(i: usize) -> i32 {
    MARGIN + i as i32 * (SETUP_FACTION_W + 8)
}

/// The computer's faction choices, left to right: each faction, then random.
fn opponent_choices() -> [Option<Faction>; 4] {
    [
        Some(Faction::Union),
        Some(Faction::Assembly),
        Some(Faction::Compact),
        None,
    ]
}

fn opponent_bounds(i: usize) -> [i32; 4] {
    [MARGIN + i as i32 * 78, SETUP_ROW_Y, 74, ROW_H]
}

fn level_bounds(i: usize) -> [i32; 4] {
    [MARGIN + 324 + i as i32 * 136, SETUP_ROW_Y, 132, ROW_H]
}

/// Draw the New Skirmish screen's frame and return its controls: your
/// faction, the computer's faction and level, the map, back and start. The
/// cards' art is drawn over the buttons by [`setup_cards`].
pub fn setup(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    shell(canvas, "NEW SKIRMISH", "");
    canvas.text("YOU", MARGIN, SETUP_CARD_Y - 10, MUTED);
    let computers = model.selected_map.layout().seats.len() - 1;
    canvas.text(
        if computers > 1 {
            "COMPUTERS"
        } else {
            "COMPUTER"
        },
        MARGIN,
        SETUP_ROW_Y - 10,
        MUTED,
    );
    canvas.text("LEVEL", MARGIN + 324, SETUP_ROW_Y - 10, MUTED);
    canvas.text("MAP", MARGIN, SETUP_MAP_Y - 10, MUTED);
    let mut buttons = Vec::new();
    for (i, faction) in Faction::ALL.into_iter().enumerate() {
        buttons.push(menu_button(
            [
                faction_card_x(i),
                SETUP_CARD_Y,
                SETUP_FACTION_W,
                SETUP_CARD_H,
            ],
            "",
            "",
            Action::Faction(faction),
            true,
        ));
    }
    for (i, choice) in opponent_choices().into_iter().enumerate() {
        buttons.push(menu_button(
            opponent_bounds(i),
            choice.map_or("RANDOM", short_faction),
            "",
            Action::Opponent(choice),
            true,
        ));
    }
    for (i, level) in bw_sim::AiLevel::ALL.into_iter().enumerate() {
        buttons.push(menu_button(
            level_bounds(i),
            level.name(),
            "",
            Action::AiLevel(level),
            true,
        ));
    }
    for (i, map) in bw_sim::MapId::ALL.into_iter().enumerate() {
        buttons.push(menu_button(
            [setup_card_x(i), SETUP_MAP_Y, SETUP_CARD_W, SETUP_MAP_H],
            "",
            "",
            Action::Map(map),
            true,
        ));
    }
    buttons.push(menu_button(
        [MARGIN, SETUP_FOOT_Y, 148, ROW_H],
        "BACK",
        "",
        Action::Back,
        true,
    ));
    buttons.push(menu_button(
        [SCREEN_W - MARGIN - 240, SETUP_FOOT_Y, 240, ROW_H],
        "START MATCH",
        if computers > 1 {
            "VS 2 COMPUTERS"
        } else {
            "VS COMPUTER"
        },
        Action::Start,
        atlas.is_some(),
    ));
    buttons
}

/// Draw the New Skirmish cards over their buttons: portraits and units on
/// the faction cards, a picture of each map, the picks' outlines and the
/// two ways to win.
pub fn setup_cards(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) {
    for (i, faction) in Faction::ALL.into_iter().enumerate() {
        let (x, y) = (faction_card_x(i), SETUP_CARD_Y);
        let accent = faction_accent(faction);
        if model.selected_faction == faction {
            card_edge(canvas, [x, y, SETUP_FACTION_W, SETUP_CARD_H], accent);
        }
        if let Some(atlas) = atlas {
            atlas.draw(canvas, faction_portrait_key(faction), x + 8, y + 7, false);
            atlas.draw(canvas, faction_emblem_key(faction), x + 36, y + 35, false);
            for (n, kind) in faction.army().into_iter().enumerate() {
                let key = format!("ui_unit_{}", kind.asset(faction));
                atlas.draw(canvas, &key, x + 74 + n as i32 * 28, y + 24, false);
            }
        }
        canvas.text_readable(short_faction(faction), x + 74, y + 9, accent);
        canvas.text(faction_tagline(faction), x + 74, y + 56, MUTED);
    }
    for (i, choice) in opponent_choices().into_iter().enumerate() {
        if model.selected_opponent == choice {
            card_edge(canvas, opponent_bounds(i), GOLD);
        }
    }
    for (i, level) in bw_sim::AiLevel::ALL.into_iter().enumerate() {
        if model.selected_ai_level == level {
            card_edge(canvas, level_bounds(i), GOLD);
        }
    }
    for (i, map) in bw_sim::MapId::ALL.into_iter().enumerate() {
        let (x, y) = (setup_card_x(i), SETUP_MAP_Y);
        if model.selected_map == map {
            card_edge(canvas, [x, y, SETUP_CARD_W, SETUP_MAP_H], GOLD);
        }
        map_thumbnail(canvas, map, x + 6, y + 5);
        let layout = map.layout();
        let name = layout.name.strip_prefix("THE ").unwrap_or(layout.name);
        let tx = x + 12 + SETUP_THUMB_W;
        canvas.text_readable(name, tx, y + 8, WHITE);
        canvas.text(
            &format!("{} PLAYERS  {} CELLS", layout.seats.len(), layout.size),
            tx,
            y + 24,
            MUTED,
        );
        canvas.text(map_blurb(map), tx, y + 40, MUTED);
    }
    // The two victories, as the icons the match itself uses.
    let y = SETUP_WIN_Y;
    canvas.text("WIN BY", MARGIN, y + 9, MUTED);
    let three = model.selected_map.layout().seats.len() > 2;
    if let Some(atlas) = atlas {
        let hq = match model.selected_faction {
            Faction::Union => "ui_build_union_hq",
            Faction::Assembly => "ui_build_assembly_hq",
            Faction::Compact => "ui_build_compact_hq",
        };
        atlas.draw(canvas, hq, MARGIN + 44, y, false);
        atlas.draw(canvas, "ui_cmd_capture", MARGIN + 284, y, false);
    }
    canvas.text(
        if three {
            "LAST HEADQUARTERS STANDING"
        } else {
            "DESTROY THE ENEMY HEADQUARTERS"
        },
        MARGIN + 72,
        y + 9,
        WHITE,
    );
    canvas.text("OR", MARGIN + 262, y + 9, MUTED);
    canvas.text(
        if three {
            "HOLD THE SLUICE AND YOUR TWO LANES FOR 75S"
        } else {
            "HOLD THE SLUICE AND BOTH LANES FOR 90S"
        },
        MARGIN + 312,
        y + 9,
        WHITE,
    );
}

/// A 1-pixel outline just inside a picked control.
fn card_edge(canvas: &mut Canvas, [x, y, w, h]: [i32; 4], color: Color) {
    canvas.rect(x, y, w, 1, color);
    canvas.rect(x, y + h - 1, w, 1, color);
    canvas.rect(x, y, 1, h, color);
    canvas.rect(x + w - 1, y, 1, h, color);
}

fn short_faction(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "UNION",
        Faction::Assembly => "ASSEMBLY",
        Faction::Compact => "COMPACT",
    }
}

fn faction_tagline(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "HOLDS A FRONT.",
        Faction::Assembly => "STRIKES FROM RANGE.",
        Faction::Compact => "FAST AND FRAGILE.",
    }
}

fn map_blurb(map: bw_sim::MapId) -> &'static str {
    match map {
        bw_sim::MapId::SplitBasin => "TWO LANES ACROSS A LAKE.",
        bw_sim::MapId::Confluence => "THREE ARMS, ONE SLUICE.",
    }
}

/// A map drawn as a small diamond, the way the field is seen, with each
/// seat's headquarters as a dot in its colour.
pub(crate) fn map_thumbnail(canvas: &mut Canvas, map: bw_sim::MapId, x0: i32, y0: i32) {
    use std::sync::OnceLock;
    static THUMBS: [OnceLock<Vec<Color>>; 2] = [OnceLock::new(), OnceLock::new()];
    let slot = bw_sim::MapId::ALL
        .iter()
        .position(|m| *m == map)
        .unwrap_or(0);
    let (w, h) = (SETUP_THUMB_W, SETUP_THUMB_W / 2);
    let pixels = THUMBS[slot].get_or_init(|| thumbnail_pixels(map, w, h));
    for (i, color) in pixels.iter().enumerate() {
        if color[3] > 0 {
            canvas.pixel(x0 + i as i32 % w, y0 + i as i32 / w, *color);
        }
    }
    let layout = map.layout();
    let n = i32::from(layout.size);
    for (seat, spot) in layout.seats.iter().enumerate() {
        let (u, v) = spot.headquarters;
        let px = x0 + w / 2 + (u - v) * (w / 2) / n;
        let py = y0 + (u + v) * (h / 2) / n;
        canvas.rect(px - 2, py - 2, 5, 5, INK);
        canvas.rect(
            px - 1,
            py - 1,
            3,
            3,
            crate::seats::preview_colour(seat as u8),
        );
    }
}

fn thumbnail_pixels(map: bw_sim::MapId, w: i32, h: i32) -> Vec<Color> {
    use bw_core::Terrain;
    let layout = map.layout();
    let factions: Vec<Faction> = (0..layout.seats.len())
        .map(|seat| Faction::ALL[seat % 2])
        .collect();
    let mut out = vec![[0, 0, 0, 0]; (w * h) as usize];
    let Ok(world) = bw_sim::World::with_map(0, map, &factions) else {
        return out;
    };
    let n = i32::from(layout.size);
    for py in 0..h {
        for px in 0..w {
            // Invert the diamond: across is u - v, down is u + v.
            let across = (2 * px + 1 - w) as f32 / w as f32;
            let down = (2 * py + 1) as f32 / h as f32;
            let u = ((across + down) * n as f32 / 2.0).floor() as i32;
            let v = ((down - across) * n as f32 / 2.0).floor() as i32;
            if u < 0 || v < 0 || u >= n || v >= n {
                continue;
            }
            out[(py * w + px) as usize] = match world.map.terrain(u, v) {
                Terrain::Deep => [34, 60, 78, 255],
                Terrain::Salt => [112, 118, 104, 255],
                Terrain::Silt => [92, 102, 94, 255],
                Terrain::Rock => [62, 68, 72, 255],
                Terrain::Lane0 | Terrain::Lane1 | Terrain::Lane2 => [146, 138, 108, 255],
                Terrain::Rim0 | Terrain::Rim1 | Terrain::Rim2 => [70, 96, 104, 255],
            };
        }
    }
    out
}

/// Draw the pause menu as a panel over the dimmed field, so the player
/// keeps sight of the match they are deciding about. The one control that
/// loses the match sits apart at the foot, in red.
pub fn pause(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    let _ = atlas;
    let (x, y, w, h) = (184, 50, 272, 260);
    // A two-player match cannot be paused: the other seat is still playing.
    // Say so rather than showing a word that is not true.
    if model.network_session {
        modal(canvas, x, y, w, h, "MENU", "THE MATCH RUNS ON");
    } else {
        modal(canvas, x, y, w, h, "PAUSED", &session_label(model));
    }
    // An online field is shared, so it has no quick save of its own.
    let save_hint = if model.network_session {
        "ONLINE"
    } else if model.just_saved {
        "SAVED"
    } else {
        ""
    };
    let rows = [
        ("RESUME", "", Action::Resume, true),
        ("SAVE", save_hint, Action::Save, !model.network_session),
        (
            "LOAD",
            save_hint_for_load(model),
            Action::Load,
            model.save_available && !model.network_session,
        ),
        ("GUIDE", "", Action::Help, true),
        ("SETTINGS", "", Action::Settings, true),
        ("MAIN MENU", "", Action::MainMenu, true),
    ];
    let (bx, bw) = (x + 16, w - 32);
    let mut buttons: Vec<Button> = rows
        .into_iter()
        .enumerate()
        .map(|(index, (label, hint, action, enabled))| {
            menu_button(
                [bx, y + 48 + index as i32 * ROW_STEP, bw, ROW_H],
                label,
                hint,
                action,
                enabled,
            )
        })
        .collect();
    let foot = y + h - 16 - ROW_H;
    canvas.line(bx, foot - 7, bx + bw - 1, foot - 7, EDGE);
    buttons.push(menu_button(
        [bx, foot, bw, ROW_H],
        if model.practice_session {
            "END PRACTICE"
        } else {
            "SURRENDER"
        },
        "",
        Action::Surrender,
        true,
    ));
    buttons
}

fn save_hint_for_load(model: &MenuModel) -> &str {
    if model.network_session {
        ""
    } else {
        save_hint(model)
    }
}

/// Draw sound, scrolling, and display preferences as label/value rows. Every
/// row is a returned button, so mouse and keyboard focus traverse the same
/// complete list. Activating a row cycles its value in the root action layer.
/// The Settings tabs, in order.
pub const SETTINGS_TABS: [&str; 4] = ["SOUND", "CONTROLS", "DISPLAY", "KEYS"];
/// Left edge and width of the Settings rows.
pub(crate) const SETTINGS_X: i32 = 170;
pub(crate) const SETTINGS_W: i32 = 300;

/// The Settings keys page: what the keys outside the command card do.
/// The order keys are on the command card and in the Guide.
const SETTINGS_KEYS: [[(&str, &str); 9]; 2] = [
    [
        ("ARROWS / EDGES", "PAN"),
        ("- / +", "ZOOM"),
        ("SPACE", "BACK TO BASE"),
        ("CTRL+1-9", "STORE A GROUP"),
        ("1-9", "GROUP, TWICE CENTRES"),
        ("I / SHIFT+I", "IDLE WORKER / ALL"),
        ("CTRL+I", "CENTRE ON SELECTION"),
        ("F2 F3 F4 F7", "ARMY ALERT WORKS WORKERS"),
        ("CTRL+BUILD KEY", "EVERY ONE OF THAT KIND"),
    ],
    [
        ("ESC", "MENU OR CANCEL"),
        ("F1", "GUIDE"),
        ("F5 / F9", "SAVE / LOAD"),
        ("F6", "SAVE REPLAY"),
        ("M", "MUTE"),
        ("O", "HEALTH BARS"),
        ("T / SHIFT+T", "SWITCH TIDE / FLOOD"),
        ("F  [  ]", "REPLAY: FOLLOW, SKIP 30S"),
        ("CTRL+Q", "QUIT"),
    ],
];

pub fn settings(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    let _ = atlas;
    shell(canvas, "SETTINGS", "");
    let tab = usize::from(model.settings_tab).min(SETTINGS_TABS.len() - 1);
    let mut buttons: Vec<Button> = Vec::with_capacity(12);
    let tab_w = 72;
    let tabs_x = (SCREEN_W - SETTINGS_TABS.len() as i32 * (tab_w + 4) + 4) / 2;
    for (index, title) in SETTINGS_TABS.iter().enumerate() {
        let x = tabs_x + index as i32 * (tab_w + 4);
        buttons.push(menu_button(
            [x, 56, tab_w, 20],
            title,
            "",
            Action::SettingsTab(index as u8),
            true,
        ));
        if index == tab {
            canvas.rect(x, 76, tab_w, 2, GOLD);
        }
    }
    let on_off = |on: bool| if on { "ON" } else { "OFF" };
    let effects = format!("{}%", model.effects_volume.min(100));
    let ambience = format!("{}%", model.ambience_volume.min(100));
    let music = format!("{}%", model.music_volume.min(100));
    let scale = match model.interface_scale {
        0 => "AUTO",
        1 => "1X",
        2 => "2X",
        3 => "3X",
        _ => "4X",
    };
    let rows: Vec<(&str, &str, Action)> = match tab {
        0 => vec![
            ("ALL SOUND", on_off(model.audio_on), Action::Mute),
            ("EFFECTS", &effects, Action::Volume(Bus::Effects)),
            ("AMBIENCE", &ambience, Action::Volume(Bus::Ambience)),
            ("MUSIC", &music, Action::Volume(Bus::Music)),
        ],
        1 => vec![
            (
                "EDGE SCROLL",
                on_off(model.edge_scroll),
                Action::ToggleEdgeScroll,
            ),
            (
                "PAUSE WHEN AWAY",
                on_off(model.pause_unfocused),
                Action::ToggleFocusPause,
            ),
            ("SPEED", model.game_speed.label(), Action::CycleSpeed),
            (
                "HEALTH BARS",
                if model.always_health_bars {
                    "ALL"
                } else {
                    "SELECTED"
                },
                Action::ToggleHealthBars,
            ),
        ],
        2 => vec![
            (
                "DISPLAY",
                if model.fullscreen {
                    "FULLSCREEN"
                } else {
                    "WINDOWED"
                },
                Action::ToggleFullscreen,
            ),
            ("INTERFACE", scale, Action::CycleInterfaceScale),
        ],
        _ => {
            // Keys: two columns, read only.
            for (column, keys) in SETTINGS_KEYS.iter().enumerate() {
                let x = 48 + column as i32 * 284;
                for (row, (key, what)) in keys.iter().enumerate() {
                    let y = 96 + row as i32 * 20;
                    canvas.text(key, x, y, GOLD);
                    canvas.text(what, x + 116, y, WHITE);
                }
            }
            canvas.text(
                "ORDER AND BUILD KEYS SHOW ON THE COMMAND CARD AND IN THE GUIDE.",
                48,
                282,
                MUTED,
            );
            Vec::new()
        }
    };
    buttons.extend(
        rows.into_iter()
            .enumerate()
            .map(|(index, (label, value, action))| {
                menu_button(
                    [SETTINGS_X, 92 + index as i32 * 28, SETTINGS_W, ROW_H],
                    label,
                    value,
                    action,
                    true,
                )
            }),
    );
    buttons.push(menu_button(
        [246, 312, 148, ROW_H],
        "BACK",
        "",
        Action::Back,
        true,
    ));
    buttons
}

/// One line on what a Settings row does, shown for the focused or hovered
/// row, and whether the arrow keys change it.
pub(crate) fn settings_hint(action: &Action) -> Option<&'static str> {
    Some(match action {
        Action::Mute => "EVERY SOUND ON OR OFF AT ONCE. M DOES THE SAME.",
        Action::Volume(Bus::Effects) => "ORDERS, WEAPONS AND ALERTS.",
        Action::Volume(Bus::Ambience) => "WATER, WIND AND THE FIELD.",
        Action::Volume(Bus::Music) => "THE SCORE.",
        Action::ToggleEdgeScroll => "THE POINTER AT A WINDOW EDGE PANS THE CAMERA.",
        Action::ToggleFocusPause => "AN OFFLINE MATCH PAUSES WHEN THE WINDOW LOSES FOCUS.",
        Action::CycleSpeed => "THE PACE OF AN OFFLINE MATCH. ONLINE ALWAYS RUNS AT 1X.",
        Action::ToggleHealthBars => "ALL MACHINES' BARS, OR ONLY THE SELECTED AND DAMAGED.",
        Action::ToggleFullscreen => "A WINDOW OR THE WHOLE SCREEN.",
        Action::CycleInterfaceScale => "AUTO PICKS THE LARGEST SIZE THAT FITS; 1X-4X FIX IT.",
        _ => return None,
    })
}

pub const GUIDE_PAGES: usize = 8;
const GUIDE_TABS: [&str; GUIDE_PAGES] = [
    "BASICS", "ECONOMY", "ORDERS", "SLUICE", "COMBAT", "CONTROL", "TECH", "ROSTER",
];
/// The roster page: every machine and building of both factions with its
/// role line, drawn in two columns instead of facts and keys.
pub const ROSTER_PAGE: usize = 7;
pub(crate) const ROSTER_KINDS: [bw_core::Kind; 31] = [
    bw_core::Kind::Hook,
    bw_core::Kind::Riveter,
    bw_core::Kind::Bulwark,
    bw_core::Kind::Sounder,
    bw_core::Kind::Tidewatch,
    bw_core::Kind::Caulker,
    bw_core::Kind::Caisson,
    bw_core::Kind::Headquarters,
    bw_core::Kind::Works,
    bw_core::Kind::Condenser,
    bw_core::Kind::Dropoff,
    bw_core::Kind::Wick,
    bw_core::Kind::Reedguard,
    bw_core::Kind::Skipper,
    bw_core::Kind::Loom,
    bw_core::Kind::Lampwright,
    bw_core::Kind::Tender,
    bw_core::Kind::Dredger,
    bw_core::Kind::Drydock,
    bw_core::Kind::Tower,
    bw_core::Kind::Palisade,
    bw_core::Kind::Lifter,
    bw_core::Kind::Barge,
    bw_core::Kind::Raker,
    bw_core::Kind::Brander,
    bw_core::Kind::Heliostat,
    bw_core::Kind::Glinter,
    bw_core::Kind::Stilt,
    bw_core::Kind::Glazier,
    bw_core::Kind::Salter,
    bw_core::Kind::Pan,
];

/// A roster role wrapped into lines of at most 46 glyphs.
pub(crate) fn role_lines(kind: Kind) -> Vec<String> {
    let role = crate::ux::role_description(kind);
    let mut lines: Vec<String> = vec![String::new()];
    for word in role.split_whitespace() {
        if lines
            .last()
            .is_some_and(|l| !l.is_empty() && l.len() + word.len() + 1 > 46)
        {
            lines.push(String::new());
        }
        let line = lines.last_mut().expect("one line");
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines
}

/// A Guide page's facts (at most four lines) and its key table.
pub(crate) type GuideText = (
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
);

/// Three or four facts and a short key table per page, on the Split Basin.
/// Facts state rules; the table lists keys. Nothing is repeated between the
/// two.
#[cfg(test)]
pub(crate) fn guide_page(page: usize, faction: Faction) -> GuideText {
    guide_page_on(page, faction, bw_sim::MapId::SplitBasin)
}

/// A Guide page for a map: the Confluence's three arms, three seats and
/// two lanes a seat change the words of the hold (trial 10: the Guide
/// showed only DRY N / DRY S).
pub(crate) fn guide_page_on(page: usize, faction: Faction, map: bw_sim::MapId) -> GuideText {
    let three = map.layout().arm_count() > 2;
    match page {
        0 if three => (
            &[
                "CLICK OR DRAG TO SELECT. SHIFT ADDS.",
                "RIGHT CLICK TO MOVE, GATHER, ATTACK, OR REPAIR.",
                "TWO WAYS TO WIN: DESTROY THE ENEMY HQS, OR HOLD YOUR TWO LANES 75S.",
                "ONCE A SEAT IS OUT: 120S, AND ONLY LANES TO SEATS STILL STANDING.",
            ],
            BASICS_KEYS,
        ),
        3 if three => (
            &[
                "HOLD A LANE: OWN THE SLUICE, A GUN OR CAISSON AT EITHER BANK,",
                "NO ENEMY GUN AT EITHER. NESTS SLOW A COUNT TO HALF, NEVER HOLD.",
                "YOUR TWO LANES FOR 75S WINS. A BROKEN COUNT DRAINS 3S A SECOND.",
                "DRY E / W / S 40P: 180S DRY. FLOOD 80P: ALL DEEP, COUNTS FREEZE.",
            ],
            &[
                ("G", "CAPTURE"),
                ("T", "DRY THE NEXT LANE"),
                ("SHIFT+T", "FLOOD"),
                ("H", "HOLD IN PLACE"),
                ("F2", "ARMY, NOT BANK GUARDS"),
            ],
        ),
        _ => guide_page_basin(page, faction),
    }
}

const BASICS_KEYS: &[(&str, &str)] = &[
    ("SPACE", "HEADQUARTERS"),
    ("I/SHIFT+I", "IDLE WORKER / ALL IDLE"),
    ("F7", "EVERY WORKER"),
    ("F2", "ARMY, NOT BANK GUARDS"),
    ("F3", "ALERT OR HOLD BANK"),
    ("CTRL+B/N", "ALL WORKS / DRYDOCKS"),
    ("CTRL+1-9", "STORE GROUP"),
    ("1-9", "RECALL, TWICE CENTRES"),
];

fn guide_page_basin(page: usize, faction: Faction) -> GuideText {
    match page {
        // The Compact's own names for its buildings (trial 11).
        1 if faction == Faction::Compact => crate::trial11_words::compact_economy_page(),
        0 => (
            &[
                "CLICK OR DRAG TO SELECT. SHIFT ADDS.",
                "RIGHT CLICK TO MOVE, GATHER, ATTACK, OR REPAIR.",
                "TWO WAYS TO WIN: DESTROY THE ENEMY HQ, OR HOLD BOTH LANES FOR 90S.",
            ],
            BASICS_KEYS,
        ),
        1 => (
            &[
                "WORKERS HAUL SALVAGE FROM WRECKS. A LANE WRECK WAITS FOR A DRY LANE.",
                "YOUR HQ ADDS 40 SALVAGE A MINUTE; THE SLUICE ADDS 30 MORE.",
                "PRESSURE BUYS SPECIALISTS AND UPGRADES. CAP +100 PER CONDENSER.",
                "CREW: 40, PLUS 12 PER WORKS, 6 PER YARD, 10 PER DRYDOCK, UP TO 90.",
            ],
            &[
                ("Q", "WORKER AT HEADQUARTERS"),
                ("Q/W/E/R", "TRAIN AT WORKS/DRYDOCK"),
                ("SHIFT", "TRAIN FIVE"),
                ("B / N", "WORKER: WORKS/DRYDOCK"),
                ("C", "WORKER: CONDENSER"),
                ("Y", "WORKER: SALVAGE YARD"),
                ("V", "WORKER: DEFENSE NEST"),
                ("P", "WORKER: PALISADE"),
            ],
        ),
        2 => (
            &[
                "SELECT, THEN PICK AN ORDER FROM THE DOCK.",
                "SHIFT QUEUES MOVES, WAYPOINTS AND A WORKER'S NEXT SITE.",
                "ON THE CHART, DRAG PANS. RIGHT CLICK MOVES OR SETS A RALLY.",
                "KEYS FOLLOW THE SELECTION: THE COMMAND CARD SHOWS WHAT EACH DOES.",
            ],
            &[
                ("A", "ATTACK-MOVE"),
                ("S", "STOP"),
                ("H", "HOLD IN PLACE"),
                ("D / P", "MACHINE: DEPLOY/PACK"),
                ("K", "STAY DEPLOYED ON MOVES"),
                ("G", "GATHER OR CAPTURE"),
                ("BACKSPACE", "CANCEL QUEUE"),
                ("ESC", "CANCEL ORDER MODE"),
            ],
        ),
        3 => (
            &[
                "HOLD A LANE: OWN THE SLUICE, A GUN OR CAISSON AT EITHER BANK,",
                "NO ENEMY GUN OR NEST AT EITHER. NESTS BLOCK, BUT DON'T HOLD.",
                "BOTH LANES FOR 90S WINS. A BROKEN COUNT DRAINS 3S A SECOND.",
                "DRY N / DRY S 40P: ONE DRIES. FLOOD 80P: ALL DEEP, COUNTS FREEZE.",
            ],
            &[
                ("G", "CAPTURE"),
                ("T", "DRY THE OTHER SIDE"),
                ("SHIFT+T", "FLOOD"),
                ("H", "HOLD IN PLACE"),
                ("F2", "ARMY, NOT BANK GUARDS"),
            ],
        ),
        5 => (
            &[
                "TIGHT HOLDS TOGETHER. LINE WIDENS THE FRONT. LOOSE SPREADS OUT.",
                "SURGE: 10 PRESSURE EACH FOR 3S OF SPEED WITH WEAPONS OFF.",
                "SURGE COOLS DOWN FOR 12S. G WHILE SURGING CAPTURES ON ARRIVAL.",
            ],
            &[
                ("F", "CYCLE FORMATION"),
                ("Z", "SURGE"),
                ("R", "FACE A DIRECTION"),
                ("E / U", "BOARD / UNLOAD"),
                ("O", "HEALTH BARS"),
            ],
        ),
        // The page speaks to the reader's own faction: the TECH page told
        // the Assembly about the Union's Caisson (trial 8).
        _ => match faction {
            Faction::Union => (
                &[
                    "ONE DOCTRINE PER MATCH: HAULING (U) OR FIRE CONTROL (J), NEVER BOTH.",
                    "UPGRADES QUEUE UP TO THREE AT THE WORKS, DRYDOCK, YARD, CONDENSER.",
                    "VENT: 100P, FIRE 30% FASTER FOR 10S. SOUND: 20P, SIGHT FOR 5S.",
                    "OVERHAUL AT THE HQ: 1000/1500/2000S, EACH +6% HULL ON EVERY MACHINE.",
                ],
                &[
                    ("U", "HAULING, THEN TIER II"),
                    ("J", "FIRE CONTROL, THEN II"),
                    ("K", "CANCEL RESEARCH"),
                    ("P / L / N", "BUILDING: UPGRADE 1-3"),
                    ("P", "HQ: NEXT OVERHAUL"),
                    ("X", "VENT OR SOUND"),
                    ("D", "CAISSON BLOCKS A CELL"),
                    ("H", "SCOUT SEES TWICE AS FAR"),
                ],
            ),
            Faction::Assembly => (
                &[
                    "ONE DOCTRINE PER MATCH: HAULING (U) OR FIRE CONTROL (J), NEVER BOTH.",
                    "UPGRADES QUEUE UP TO THREE AT THE WORKS, DRYDOCK, YARD, CONDENSER.",
                    "VENT: 100P, FIRE 30% FASTER FOR 10S. SOUND: 20P, SIGHT FOR 5S.",
                    "OVERHAUL AT THE HQ: 1000/1500/2000S, EACH +6% HULL ON EVERY MACHINE.",
                ],
                &[
                    ("U", "HAULING, THEN TIER II"),
                    ("J", "FIRE CONTROL, THEN II"),
                    ("K", "CANCEL RESEARCH"),
                    ("P / L / N", "BUILDING: UPGRADE 1-3"),
                    ("P", "HQ: NEXT OVERHAUL"),
                    ("X", "VENT OR SOUND"),
                    ("D", "LOOM DEPLOYS TO FIRE"),
                    ("H", "SCOUT SEES TWICE AS FAR"),
                ],
            ),
            Faction::Compact => (
                &[
                    "ONE DOCTRINE PER MATCH: HAULING (U) OR FIRE CONTROL (J), NEVER BOTH.",
                    "GLINT: 20P, A RAY OF SIGHT 18 LONG. LAY: 3P A ROW OF CRUST.",
                    "CRUST IS DRY GROUND FOR EVERYONE UNTIL THE TIDE NEXT MOVES.",
                    "OVERHAUL AT THE HQ: 1000/1500/2000S, EACH +6% HULL ON EVERY MACHINE.",
                ],
                &[
                    ("U", "HAULING, THEN TIER II"),
                    ("J", "FIRE CONTROL, THEN II"),
                    ("P / L / N", "BUILDING: UPGRADE 1-3"),
                    ("P", "HQ: NEXT OVERHAUL"),
                    ("X", "VENT OR GLINT"),
                    ("L", "SALTER LAYS A CAUSEWAY"),
                    ("D", "HELIOSTAT FIRES DEPLOYED"),
                    ("D", "PAN DEPLOYS TO BOIL"),
                ],
            ),
        },
    }
}

pub(crate) const SLUICE_PAGE: usize = 3;

/// The hold rule drawn: the two banks, the lake with the station on its
/// island, the north and south lanes and their four mouths.  A schematic in
/// the map's own layout, 264 by 128 pixels from its top-left corner.
fn sluice_diagram(canvas: &mut Canvas, x0: i32, y0: i32) {
    const LAND: Color = [58, 72, 62, 255];
    const WATER: Color = [30, 62, 82, 255];
    const LANE: Color = [124, 112, 82, 255];
    const ISLAND: Color = [96, 98, 84, 255];
    // Map cells 40..88 both ways onto the box: the lanes cross at rows 49
    // and 79, the mouths stand at columns 50 and 77.
    let x = |cell: i32| x0 + (cell - 40) * 11 / 2;
    let y = |cell: i32| y0 + (cell - 40) * 8 / 3;
    canvas.rect(x0, y0, 264, 128, WATER);
    canvas.rect(x0, y0, x(50) - x0, 128, LAND);
    canvas.rect(x(77), y0, x0 + 264 - x(77), 128, LAND);
    for row in [49, 79] {
        canvas.rect(x(50), y(row) - 5, x(77) - x(50), 11, LANE);
    }
    canvas.rect(x(62), y(52), x(66) - x(62), y(76) - y(52), ISLAND);
    // The station.
    let (gx, gy) = (x(64), y(64));
    canvas.rect(gx - 5, gy - 5, 11, 11, INK);
    canvas.frame(gx - 5, gy - 5, 11, 11, GOLD);
    canvas.rect(gx - 1, gy - 3, 3, 7, GOLD);
    canvas.text("STATION", gx + 9, gy - 3, WHITE);
    canvas.text("OWN IT", gx + 9, gy + 6, MUTED);
    // The lanes and their mouths.
    canvas.text("N LANE", x(60), y(49) - 16, WHITE);
    canvas.text("S LANE", x(60), y(79) + 10, WHITE);
    for row in [49, 79] {
        for column in [50, 77] {
            let (mx, my) = (x(column), y(row));
            canvas.ellipse(mx, my, 12, 6, GOLD);
            canvas.ellipse(mx, my, 10, 4, INK);
            canvas.diamond(mx, my, 3, 2, GOLD);
        }
    }
    canvas.text("BANK", x(50) - 30, y(49) - 16, GOLD);
    canvas.text("BANK", x(77) + 2, y(79) + 10, GOLD);
    canvas.text("WEST", x0 + 4, y0 + 110, MUTED);
    canvas.text("BANK", x0 + 4, y0 + 118, MUTED);
    canvas.text("EAST", x(77) + 14, y0 + 4, MUTED);
    canvas.text("BANK", x(77) + 14, y0 + 12, MUTED);
    canvas.frame(x0 - 1, y0 - 1, 266, 130, EDGE);
}

/// The hold rule on the Confluence: three lands around a Y of water, the
/// station where the arms meet, and each arm's lane with a mouth at each
/// end.  Each seat holds the two lanes beside its land.  A schematic in the
/// same 264 by 128 box as the Split Basin's.
fn confluence_diagram(canvas: &mut Canvas, map: bw_sim::MapId, x0: i32, y0: i32) {
    const LAND: Color = [58, 72, 62, 255];
    const WATER: Color = [30, 62, 82, 255];
    const LANE: Color = [124, 112, 82, 255];
    const ISLAND: Color = [96, 98, 84, 255];
    let (w, h) = (264, 128);
    let (cx, cy) = (x0 + w / 2, y0 + 58);
    // The three arms as the chart draws them: up-left, up-right and down.
    let layout = map.layout();
    let dirs: [(i32, i32); 3] = [(10, -6), (-10, -6), (0, 10)];
    let letter = |arm: usize| layout.arms.get(arm).map_or("?", |a| a.name);
    canvas.rect(x0, y0, w, h, LAND);
    // Water: every pixel within reach of an arm's centre line.
    for py in 0..h {
        for px in 0..w {
            let (dx, dy) = (x0 + px - cx, y0 + py - cy);
            let wet = dirs.iter().any(|&(ax, ay)| {
                let len2 = ax * ax + ay * ay;
                let along = dx * ax + dy * ay;
                if along < 0 {
                    return dx * dx + dy * dy < 14 * 14;
                }
                let cross = dx * ay - dy * ax;
                cross * cross < 12 * 12 * len2
            });
            if wet {
                canvas.pixel(x0 + px, y0 + py, WATER);
            }
        }
    }
    // The station's island where the arms meet.
    canvas.diamond(cx, cy, 12, 9, ISLAND);
    canvas.rect(cx - 5, cy - 5, 11, 11, INK);
    canvas.frame(cx - 5, cy - 5, 11, 11, GOLD);
    canvas.rect(cx - 1, cy - 3, 3, 7, GOLD);
    // Each arm's lane crosses it a little way out, a mouth on either bank,
    // and its letter beside the outer mouth.
    for (arm, &(ax, ay)) in dirs.iter().enumerate() {
        let len = ((ax * ax + ay * ay) as f32).sqrt();
        let (ux, uy) = (ax as f32 / len, ay as f32 / len);
        let (mx, my) = (cx as f32 + ux * 34.0, cy as f32 + uy * 34.0);
        let (nx, ny) = (-uy * 15.0, ux * 15.0);
        let a = ((mx + nx) as i32, (my + ny) as i32);
        let b = ((mx - nx) as i32, (my - ny) as i32);
        for t in -1..=1 {
            canvas.line(a.0, a.1 + t, b.0, b.1 + t, LANE);
        }
        for (px, py) in [a, b] {
            canvas.ellipse(px, py, 7, 4, GOLD);
            canvas.ellipse(px, py, 5, 2, INK);
            canvas.diamond(px, py, 2, 1, GOLD);
        }
        let label = format!("{} LANE", letter(arm));
        let (lx, ly) = match arm {
            0 => (a.0.max(b.0) + 10, a.1.max(b.1) - 3),
            1 => (a.0.min(b.0) - 10 - 36, a.1.max(b.1) - 3),
            _ => (a.0.max(b.0) + 10, a.1 - 3),
        };
        canvas.text(&label, lx, ly, WHITE);
        if arm == 2 {
            canvas.text("BANK", a.0.min(b.0) - 10 - 30, a.1 - 3, GOLD);
        }
    }
    canvas.text("STATION", cx - 62, cy + 12, WHITE);
    canvas.text("OWN IT", cx - 62, cy + 21, MUTED);
    // The lands: each holds the two lanes beside it.
    canvas.text("BASE", cx - 12, y0 + 4, MUTED);
    canvas.text("BASE", x0 + 6, y0 + h - 12, MUTED);
    canvas.text("BASE", x0 + w - 30, y0 + h - 12, MUTED);
    canvas.text(
        "EACH BASE HOLDS THE TWO LANES BESIDE IT",
        x0,
        y0 + h + 6,
        MUTED,
    );
    canvas.frame(x0 - 1, y0 - 1, w + 2, h + 2, EDGE);
}

/// The Guide's navigation row, under a panel tall enough for the roster.
const GUIDE_NAV_Y: i32 = 330;
/// Flavour lines read in a warm parchment tone, apart from the rules.
const FLAVOUR: Color = [196, 170, 128, 255];

/// Where and on which key a faction's machine is made, as the command card
/// has it: the worker at the HQ, three roles at the Works, four at the
/// Drydock, and the scout at both, on W at each since trial 11 (trial 10
/// read only "W AT HQ" and pressed W at the Drydock).  One line a place.
pub(crate) fn roster_train(faction: Faction, kind: Kind) -> String {
    const KEYS: [&str; 4] = ["Q", "W", "E", "R"];
    if kind == faction.worker() {
        return "Q AT HQ".into();
    }
    let dock_keys = crate::trial11_controls::drydock_keys(faction);
    if kind == faction.scout() {
        let dock = dock_keys
            .iter()
            .position(|k| *k == kind)
            .map_or(String::new(), |i| format!("\n{} AT DRYDOCK", KEYS[i]));
        return format!("W AT HQ{dock}");
    }
    if let Some(i) = faction.army().iter().position(|k| *k == kind) {
        return format!("{} AT WORKS", KEYS[i]);
    }
    if let Some(i) = dock_keys.iter().position(|k| *k == kind) {
        return format!("{} AT DRYDOCK", KEYS[i]);
    }
    String::new()
}

/// A faction's machines in command-card order.
pub(crate) fn roster_machines(faction: Faction) -> Vec<Kind> {
    let mut kinds = vec![faction.worker()];
    kinds.extend(faction.army());
    kinds.extend(faction.drydock_roles());
    kinds
}

/// The roster page: one faction's machines, or the buildings both sides
/// share, as a grid of cards (portrait, name, costs as marks, where it is
/// made), and under it the details of the card in focus: its role, what it
/// beats, what beats it, and a line of flavour.
fn roster(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    model: &MenuModel,
    buttons: &mut Vec<Button>,
) {
    let buildings = model.roster_buildings;
    let faction = model.roster_faction;
    for (x, label, action, on) in [
        (
            196,
            "UNION",
            Action::RosterFaction(Faction::Union),
            !buildings && faction == Faction::Union,
        ),
        (
            300,
            "ASSEMBLY",
            Action::RosterFaction(Faction::Assembly),
            !buildings && faction == Faction::Assembly,
        ),
        (
            404,
            "COMPACT",
            Action::RosterFaction(Faction::Compact),
            !buildings && faction == Faction::Compact,
        ),
        (508, "BUILDINGS", Action::RosterBuildings, buildings),
    ] {
        buttons.push(menu_button([x, 82, 100, ROW_H], label, "", action, true));
        if on {
            canvas.rect(x, 104, 100, 2, GOLD);
        }
    }
    let title = if buildings {
        "BUILDINGS"
    } else {
        match faction {
            Faction::Union => "UNION MACHINES",
            Faction::Assembly => "ASSEMBLY MACHINES",
            Faction::Compact => "COMPACT MACHINES",
        }
    };
    canvas.text_readable(title, 36, 88, GOLD);
    let kinds: Vec<Kind> = if buildings {
        ROSTER_KINDS
            .iter()
            .copied()
            .filter(|k| k.is_building())
            .collect()
    } else {
        roster_machines(faction)
    };
    let shown = model
        .roster_unit
        .filter(|k| kinds.contains(k))
        .unwrap_or(kinds[0]);
    let (cw, ch, gap) = (138, 62, 6);
    for (i, &kind) in kinds.iter().enumerate() {
        let x = 32 + (i as i32 % 4) * (cw + gap);
        let y = 112 + (i as i32 / 4) * (ch + gap);
        roster_card(canvas, atlas, faction, kind, [x, y, cw, ch], kind == shown);
        buttons.push(menu_button(
            [x, y, cw, ch],
            kind.name(),
            "",
            Action::RosterUnit(kind),
            true,
        ));
    }
    // The details of the card in focus.
    let dy = 112 + 2 * (ch + gap) + 2;
    canvas.line(32, dy - 3, 607, dy - 3, EDGE);
    let shown_name = crate::ux::building_name(shown, faction);
    canvas.text_readable(shown_name, 36, dy, WHITE);
    let name_w = crate::canvas::text_readable_width(shown_name) + 12;
    canvas.text(crate::ux::flavour(shown), 36 + name_w, dy + 1, FLAVOUR);
    let mut ly = dy + 13;
    for line in role_lines(shown).iter().take(2) {
        canvas.text(line, 36, ly, MUTED);
        ly += 10;
    }
    let (strong, weak) = crate::ux::matchups(shown);
    if strong.is_empty() {
        canvas.text("NO GUN", 330, dy + 13, MUTED);
    } else {
        canvas.text(&format!("BEATS {strong}"), 330, dy + 13, JADE);
    }
    canvas.text(&format!("LOSES TO {weak}"), 330, dy + 23, RED);
    // Another side's machine: what of yours answers it.
    let yours = model.selected_faction;
    if !buildings && faction != yours {
        let answers: Vec<&str> = crate::ux::answers(shown, yours)
            .into_iter()
            .map(|k| k.name())
            .collect();
        if !answers.is_empty() {
            canvas.text(
                &format!("YOUR ANSWER: {}", answers.join(", ")),
                330,
                dy + 33,
                GOLD,
            );
        }
    }
}

/// One roster card: the portrait (a machine's, or a building's icon), its
/// name, its costs as marks, and the key and place it is made.
fn roster_card(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    faction: Faction,
    kind: Kind,
    [x, y, w, h]: [i32; 4],
    shown: bool,
) {
    canvas.rect(
        x,
        y,
        w,
        h,
        if shown {
            [34, 53, 60, 255]
        } else {
            [23, 38, 46, 255]
        },
    );
    canvas.frame(x, y, w, h, if shown { GOLD } else { EDGE });
    let asset = kind.asset(faction);
    let portrait = format!("portrait_{asset}");
    let drawn = atlas.is_some_and(|atlas| {
        atlas.draw(canvas, &portrait, x + 3, y + 3, false)
            || atlas.draw_scaled(
                canvas,
                &format!("ui_build_{asset}"),
                x + 7,
                y + 7,
                false,
                crate::occlusion::PixelScale {
                    numerator: 2,
                    denominator: 1,
                },
            )
    });
    if !drawn {
        canvas.rect(x + 3, y + 3, 56, 56, INK);
        canvas.diamond(x + 31, y + 31, 12, 14, EDGE);
    }
    let tx = x + 64;
    let name = crate::ux::building_name(kind, faction);
    let room = ((w - 66) / 6) as usize;
    canvas.text(
        &name.chars().take(room).collect::<String>(),
        tx,
        y + 5,
        WHITE,
    );
    // Costs as the hover card has them: a mark and a number.
    let spec = bw_content::spec(kind);
    let mut cy = y + 17;
    if kind == Kind::Headquarters {
        canvas.text("GIVEN", tx, cy + 1, MUTED);
    } else {
        for (mark, value) in [
            ("tip_salvage", spec.salvage),
            ("tip_pressure", spec.pressure),
        ] {
            if value == 0 {
                continue;
            }
            let marked = atlas.is_some_and(|atlas| atlas.draw(canvas, mark, tx, cy, false));
            if !marked {
                canvas.rect(tx + 1, cy + 1, 6, 6, GOLD);
            }
            canvas.text(&value.to_string(), tx + 11, cy + 1, GOLD);
            cy += 11;
        }
    }
    let made = if kind.is_building() {
        if kind == Kind::Headquarters {
            String::new()
        } else {
            format!("{} BY WORKER", crate::ux::build_hotkey(kind))
        }
    } else {
        roster_train(faction, kind)
    };
    // Where it is made, one line a place, from the card's foot up.
    let places: Vec<&str> = made.lines().collect();
    for (i, place) in places.iter().enumerate() {
        canvas.text(
            &place.chars().take(room).collect::<String>(),
            tx,
            y + h - 11 - (places.len() - 1 - i) as i32 * 10,
            MUTED,
        );
    }
}

/// Draw a short, task-based guide page, including advanced counterplay.
pub fn help(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    shell(canvas, "GUIDE", "");
    let page = model.help_page.min(GUIDE_PAGES - 1);
    let mut buttons = Vec::with_capacity(12);
    for (index, title) in GUIDE_TABS.iter().enumerate() {
        let x = 24 + index as i32 * 74;
        buttons.push(menu_button(
            [x, 48, 72, 20],
            title,
            "",
            Action::GuidePage(index),
            true,
        ));
        if index == page {
            // The open tab is marked in paint, just below its button, so it
            // stays visible whatever the button renderer paints inside.
            canvas.rect(x, 68, 72, 2, GOLD);
        }
    }

    panel(canvas, 24, 78, 592, 244, EDGE);
    if page == ROSTER_PAGE {
        roster(canvas, atlas, model, &mut buttons);
    } else if page == 4 {
        // Combat pairs complete mobile/deployed silhouettes with one tactical
        // example. The example controls are returned as ordinary Buttons so
        // mouse, Tab, and arrow focus stay on one navigation path.
        field_manual::draw_combat_page(canvas, atlas, &model.manual, (40, 88), (286, 88));
        canvas.text("D DEPLOY  P PACK  K KEEP  R FACE", 40, 240, MUTED);
        // Every side's specialist, the reader's own marked: trial 10's
        // Compact found only the Bulwark and the Loom here.
        for (i, subject) in ManualSubject::ALL.into_iter().enumerate() {
            let x = 40 + i as i32 * 82;
            buttons.push(menu_button(
                [x, 252, 80, ROW_H],
                subject.label(),
                "",
                Action::ManualSubject(subject),
                true,
            ));
            if model.manual.subject == subject {
                canvas.rect(x, 274, 80, 2, GOLD);
            }
            if subject.faction() == model.selected_faction {
                canvas.text("YOURS", x + 25, 278, JADE);
            }
        }
        buttons.push(menu_button(
            [292, 252, 106, ROW_H],
            if model.manual.playing {
                "PAUSE"
            } else {
                "PLAY"
            },
            "",
            Action::ManualPlay,
            true,
        ));
        buttons.push(menu_button(
            [404, 252, 112, ROW_H],
            "NEXT STEP",
            "",
            Action::ManualStep,
            true,
        ));
    } else {
        let keys = guide_page_on(page, model.selected_faction, model.selected_map).1;
        let lines = crate::trial12_hold::guide_lines(
            page,
            model.selected_faction,
            model.selected_map,
            model.hold_seconds,
        );
        let mut y = 92;
        for (index, line) in lines.iter().enumerate() {
            y += draw_wrapped(
                canvas,
                line,
                44,
                y,
                544,
                14,
                if index == 0 { WHITE } else { MUTED },
            );
            y += 4;
        }
        canvas.text_readable("KEYS", 44, 172, GOLD);
        // The sluice page's diagram fills the right half, so its keys stay
        // in one column rather than running under the picture.
        let per_column = if page == SLUICE_PAGE {
            keys.len().max(1)
        } else {
            4
        };
        for (index, (key, action)) in keys.iter().enumerate() {
            let x = 44 + (index / per_column) as i32 * 276;
            let row_y = 190 + (index % per_column) as i32 * 15;
            canvas.text_readable(key, x, row_y, WHITE);
            canvas.text_readable(action, x + 80, row_y, MUTED);
        }
        if page == SLUICE_PAGE {
            if model.selected_map.layout().arm_count() > 2 {
                confluence_diagram(canvas, model.selected_map, 336, 160);
            } else {
                sluice_diagram(canvas, 336, 160);
            }
        }
    }

    // The tabs are the navigation; left and right on them turn the page.
    canvas.text("< > TURN PAGES ON THE TABS", 24, GUIDE_NAV_Y + 8, MUTED);
    buttons.push(menu_button(
        [468, GUIDE_NAV_Y, 148, ROW_H],
        "BACK",
        "",
        Action::Back,
        true,
    ));
    buttons
}

/// Draw a concrete leave/replace confirmation: the question is the title,
/// the consequence is one line, and each button names what it does. Over a
/// match it is a panel on the dimmed field; the answers sit right aligned
/// with the one that loses progress last, in red.
pub fn confirm(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &MenuModel) -> Vec<Button> {
    let _ = atlas;
    let (x, w) = (136, 368);
    let body_h = if model.confirmation_body.is_empty() {
        0
    } else {
        wrap_lines(&model.confirmation_body, w - 32).len() as i32 * 14
    };
    let h = 48 + body_h + 16 + ROW_H + 16;
    let y = (360 - h) / 2;
    let title = model.confirmation_title.to_ascii_uppercase();
    if model.overlay {
        modal(canvas, x, y, w, h, &title, "");
    } else {
        canvas.clear(INK);
        canvas.rect(0, 0, SCREEN_W, 2, GOLD);
        modal_panel(canvas, x, y, w, h, &title, "");
    }
    if !model.confirmation_body.is_empty() {
        draw_wrapped(
            canvas,
            &model.confirmation_body,
            x + 16,
            y + 48,
            w - 32,
            14,
            MUTED,
        );
    }
    let by = y + h - 16 - ROW_H;
    let mut answers: Vec<(String, Action, i32)> =
        vec![("CANCEL".into(), Action::DismissConfirm, 92)];
    if model.can_save_before_confirm {
        answers.push(("SAVE FIRST".into(), Action::SaveAndConfirm, 104));
    }
    answers.push((model.confirmation_action.clone(), Action::Confirm, 120));
    let mut right = x + w - 16;
    let mut buttons: Vec<Button> = answers
        .into_iter()
        .rev()
        .map(|(label, action, bw)| {
            right -= bw;
            let b = menu_button([right, by, bw, ROW_H], &label, "", action, true);
            right -= 8;
            b
        })
        .collect();
    // Keyboard order follows reading order: left to right.
    buttons.reverse();
    buttons
}

/// Draw the finished-match screen. The outcome is authoritative and is
/// passed separately so the shared menu model stays useful for fixtures.
pub fn result(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    model: &MenuModel,
    outcome: Outcome,
) -> Vec<Button> {
    use crate::dock_log::ResultPage;
    // The result stands over the finished field, dimmed behind it: a
    // banner with the outcome, then the summary or the log.
    canvas.clear([0, 0, 0, 0]);
    let (x, y, w, h) = (16, 10, 608, 288);
    crate::dock_log::draw_result_overlay(
        canvas,
        atlas,
        &model.dock,
        &crate::dock_log::ResultOverlayOptions {
            outcome: Some(&outcome),
            surrendered: model.surrendered,
            practice_complete: false,
            x,
            y,
            w,
            h,
            summary: Some(&model.summary),
            page: model.result_page,
            log_scroll: model.result_log_scroll,
            you: (!model.observing).then_some(0),
        },
    );
    let [summary_tab, log_tab] = crate::dock_log::result_tab_rects(x, y + 74, w);
    let mut buttons = vec![
        // The page on show is pointed at.
        menu_button(
            summary_tab,
            if model.result_page == ResultPage::Summary {
                "> SUMMARY"
            } else {
                "SUMMARY"
            },
            "",
            Action::ResultPage(ResultPage::Summary),
            true,
        ),
        menu_button(
            log_tab,
            if model.result_page == ResultPage::Log {
                "> DOCK LOG"
            } else {
                "DOCK LOG"
            },
            "",
            Action::ResultPage(ResultPage::Log),
            true,
        ),
    ];
    if model.result_page == ResultPage::Log {
        let rows = crate::dock_log::log_row_count(&model.dock.log);
        // At the right end of the log's own title row, under the tabs: on
        // the tabs' row they covered the placings ("2ND VIOL" in trial 10).
        let right = x + w - 16;
        let row_y = summary_tab[1] + 26;
        buttons.push(menu_button(
            [right - 140, row_y, 68, 18],
            "NEWER",
            "",
            Action::ResultLogScroll(false),
            model.result_log_scroll > 0,
        ));
        buttons.push(menu_button(
            [right - 68, row_y, 68, 18],
            "OLDER",
            "",
            Action::ResultLogScroll(true),
            model.result_log_scroll + 1 < rows,
        ));
    }
    // The actions sit at the bottom right, the primary one rightmost.
    let row = 308;
    let actions: Vec<(&str, Action)> = if model.observing {
        vec![
            ("MAIN MENU", Action::MainMenu),
            ("WATCH AGAIN", Action::ReplayRestart),
        ]
    } else {
        vec![
            ("MAIN MENU", Action::MainMenu),
            ("SAVE REPLAY", Action::ExportReplay),
            ("WATCH REPLAY", Action::WatchReplay),
            ("PLAY AGAIN", Action::Start),
        ]
    };
    let (bw, gap) = (132, 10);
    let mut bx = x + w - (bw + gap) * actions.len() as i32 + gap;
    for (label, action) in actions {
        buttons.push(menu_button([bx, row, bw, ROW_H], label, "", action, true));
        bx += bw + gap;
    }
    buttons
}

/// Draw the honest end of a guided practice as a panel over the frozen
/// field: how far the lesson got, what came next, and where to go now.  No
/// `Outcome` is manufactured in `World`.
pub fn practice_review(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    model: &MenuModel,
) -> Vec<Button> {
    let _ = atlas;
    let (x, y, w, h) = (170, 85, 300, 190);
    let (done, total, next) = &model.practice_progress;
    let complete = model.practice_complete || (*total > 0 && done >= total);
    modal(
        canvas,
        x,
        y,
        w,
        h,
        if complete {
            "PRACTICE COMPLETE"
        } else {
            "PRACTICE ENDED"
        },
        "",
    );
    // The lesson's steps as dots, then the count in words.
    let total = (*total).max(1);
    for i in 0..total {
        let c = if i < *done { JADE } else { EDGE };
        canvas.rect(x + 16 + i as i32 * 10, y + 50, 7, 7, c);
    }
    canvas.text_readable(
        &format!("{} OF {} STEPS", (*done).min(total), total),
        x + 26 + total as i32 * 10,
        y + 49,
        WHITE,
    );
    let line = if complete {
        "EVERY STEP DONE. TRY A SKIRMISH NEXT.".to_string()
    } else if next.is_empty() {
        String::new()
    } else {
        format!("NEXT WAS: {}", next.to_ascii_uppercase())
    };
    canvas.text(&line, x + 16, y + 68, MUTED);
    let (bx, bw) = (x + 16, (w - 40) / 2);
    let row = |i: i32| y + 88 + i * ROW_STEP;
    let first = if complete {
        (
            "NEW SKIRMISH",
            Action::Setup,
            "PRACTICE AGAIN",
            Action::StartPractice,
        )
    } else {
        (
            "PRACTICE AGAIN",
            Action::StartPractice,
            "NEW SKIRMISH",
            Action::Setup,
        )
    };
    vec![
        menu_button([bx, row(0), bw, ROW_H], first.0, "", first.1, true),
        menu_button([bx + bw + 8, row(0), bw, ROW_H], first.2, "", first.3, true),
        menu_button([bx, row(1), bw, ROW_H], "SAVE", "", Action::Save, true),
        menu_button(
            [bx + bw + 8, row(1), bw, ROW_H],
            "SAVE REPLAY",
            "",
            Action::ExportReplay,
            true,
        ),
        menu_button(
            [bx, row(2) + 8, w - 32, ROW_H],
            "MAIN MENU",
            "",
            Action::EndReview,
            true,
        ),
    ]
}

pub(crate) fn shell(canvas: &mut Canvas, title: &str, subtitle: &str) {
    canvas.clear(INK);
    canvas.rect(0, 0, SCREEN_W, 2, GOLD);
    canvas.text_scaled(&title.to_ascii_uppercase(), MARGIN, 16, WHITE, 2);
    canvas.rect(MARGIN, 40, 32, 2, GOLD);
    if !subtitle.is_empty() {
        canvas.text(&subtitle.to_ascii_uppercase(), MARGIN + 40, 37, MUTED);
    }
}

/// A centred menu panel on a transparent menu canvas: the renderer draws
/// the dimmed field behind it.
pub(crate) fn modal(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    title: &str,
    subtitle: &str,
) {
    canvas.clear([0, 0, 0, 0]);
    modal_panel(canvas, x, y, w, h, title, subtitle);
}

fn modal_panel(canvas: &mut Canvas, x: i32, y: i32, w: i32, h: i32, title: &str, subtitle: &str) {
    canvas.rect(x, y, w, h, INK);
    canvas.frame(x, y, w, h, EDGE);
    canvas.rect(x, y, w, 2, GOLD);
    canvas.text_scaled(&title.to_ascii_uppercase(), x + 16, y + 14, WHITE, 2);
    canvas.rect(x + 16, y + 36, 32, 2, GOLD);
    if !subtitle.is_empty() {
        canvas.text(&subtitle.to_ascii_uppercase(), x + 56, y + 33, MUTED);
    }
}

pub(crate) fn panel(canvas: &mut Canvas, x: i32, y: i32, w: i32, h: i32, edge: Color) {
    canvas.rect(x, y, w, h, [23, 38, 46, 255]);
    canvas.line(x, y, x + w - 1, y, edge);
    canvas.rect(x, y, 2, 12, edge);
}

pub(crate) fn menu_button(
    bounds: [i32; 4],
    label: &str,
    hint: &str,
    action: Action,
    enabled: bool,
) -> Button {
    let [x, y, w, h] = bounds;
    Button {
        x,
        y,
        w,
        h,
        label: label.into(),
        hint: hint.into(),
        action,
        enabled,
    }
}

pub(crate) fn faction_accent(faction: Faction) -> Color {
    match faction {
        Faction::Union => GOLD,
        Faction::Assembly => JADE,
        Faction::Compact => COBALT,
    }
}

fn faction_portrait_key(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "portrait_union",
        Faction::Assembly => "portrait_assembly",
        Faction::Compact => "portrait_compact",
    }
}

pub(crate) fn faction_emblem_key(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "faction_union_emblem",
        Faction::Assembly => "faction_assembly_emblem",
        Faction::Compact => "faction_compact_emblem",
    }
}

fn format_clock(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// Draw words on readable rows without silently dropping any instruction.
/// Split text into the lines `draw_wrapped` draws at `width`.
pub(crate) fn wrap_lines(text: &str, width: i32) -> Vec<String> {
    let max_chars = (width / 8).max(1) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if candidate.chars().count() <= max_chars {
            current = candidate;
        } else if current.is_empty() {
            // A single unusually long model-provided word is split into
            // visible chunks so it remains reviewable and never overflows.
            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() == max_chars {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(ch);
            }
            current = chunk;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(crate) fn draw_wrapped(
    canvas: &mut Canvas,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    line_height: i32,
    color: Color,
) -> i32 {
    let lines = wrap_lines(text, width);
    for (index, line) in lines.iter().enumerate() {
        canvas.text_readable(line, x, y + index as i32 * line_height, color);
    }
    lines.len().max(1) as i32 * line_height
}

#[cfg(test)]
mod tests {
    #[test]
    fn home_leaves_the_field_clear_beside_the_menu() {
        let mut canvas = crate::canvas::Canvas::default();
        super::home(&mut canvas, None, &super::MenuModel::default());
        // Right of the menu column the canvas is clear, so the field the
        // renderer draws behind it shows.
        assert_eq!(canvas.get(500, 200), Some([0, 0, 0, 0]));
        assert_ne!(canvas.get(41, 31), Some([0, 0, 0, 0]), "the wordmark");
    }

    use super::*;
    use crate::canvas::{Canvas, text_readable_width};

    fn on_canvas(button: &Button) -> bool {
        button.x >= 0 && button.y >= 0 && button.x + button.w <= 640 && button.y + button.h <= 360
    }

    #[test]
    fn default_home_is_safe_practice_first_and_save_is_disabled() {
        let mut canvas = Canvas::default();
        let model = MenuModel::default();
        let buttons = home(&mut canvas, None, &model);
        assert!(matches!(buttons[0].action, Action::StartPractice));
        assert!(
            !buttons
                .iter()
                .any(|button| matches!(button.action, Action::Load)),
            "LOAD waits for a save"
        );
        assert_eq!(canvas.pixels.len(), 640 * 360 * 4);
    }

    #[test]
    fn active_home_puts_continue_before_practice() {
        let mut canvas = Canvas::default();
        let model = MenuModel {
            has_session: true,
            save_available: true,
            ..MenuModel::default()
        };
        let buttons = home(&mut canvas, None, &model);
        assert!(matches!(buttons[0].action, Action::Resume));
        assert!(matches!(buttons[1].action, Action::StartPractice));
    }

    #[test]
    fn home_and_pause_hints_carry_only_state() {
        let mut canvas = Canvas::default();
        let fresh = home(&mut canvas, None, &MenuModel::default());
        assert!(fresh.iter().all(|button| button.hint.is_empty()));
        let model = MenuModel {
            has_session: true,
            practice_session: true,
            session_seconds: 75,
            save_available: true,
            save_description: "SKIRMISH / UNION / 04:12".into(),
            ..MenuModel::default()
        };
        type Screen = fn(&mut Canvas, Option<&Atlas>, &MenuModel) -> Vec<Button>;
        for (screen, has_continue) in [(home as Screen, true), (pause as Screen, false)] {
            let buttons = screen(&mut canvas, None, &model);
            for button in &buttons {
                match button.action {
                    Action::Resume if has_continue => assert_eq!(button.hint, "PRACTICE 01:15"),
                    Action::Load => assert_eq!(button.hint, model.save_description),
                    _ => assert!(button.hint.is_empty(), "{}", button.label),
                }
            }
        }
    }

    #[test]
    fn setup_offers_every_choice_and_names_both_victories() {
        let mut canvas = Canvas::default();
        let buttons = setup(&mut canvas, None, &MenuModel::default());
        let has = |action: Action| buttons.iter().any(|button| button.action == action);
        for faction in Faction::ALL {
            assert!(has(Action::Faction(faction)));
            assert!(has(Action::Opponent(Some(faction))));
        }
        assert!(has(Action::Opponent(None)), "a random computer");
        for level in bw_sim::AiLevel::ALL {
            assert!(has(Action::AiLevel(level)));
        }
        for map in bw_sim::MapId::ALL {
            assert!(has(Action::Map(map)));
        }
        assert!(has(Action::Back) && has(Action::Start));
        assert!(buttons.iter().all(on_canvas));
        for (i, a) in buttons.iter().enumerate() {
            for b in &buttons[i + 1..] {
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "{:?} overlaps {:?}", a.action, b.action);
            }
        }
        let start = |map| {
            let model = MenuModel {
                selected_map: map,
                ..MenuModel::default()
            };
            let buttons = setup(&mut Canvas::default(), None, &model);
            let start = buttons.iter().find(|b| b.action == Action::Start).unwrap();
            start.hint.clone()
        };
        assert_eq!(start(bw_sim::MapId::SplitBasin), "VS COMPUTER");
        assert_eq!(start(bw_sim::MapId::Confluence), "VS 2 COMPUTERS");
        // Both victories are written out on the screen itself: the cards'
        // text fits the canvas at every pick.
        for map in bw_sim::MapId::ALL {
            let model = MenuModel {
                selected_map: map,
                ..MenuModel::default()
            };
            let mut canvas = Canvas::default();
            setup(&mut canvas, None, &model);
            setup_cards(&mut canvas, None, &model);
        }
    }

    #[test]
    fn the_skirmish_starts_with_the_picked_computer_and_level() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = crate::game::Game::new(base);
        game.action(Action::Faction(Faction::Assembly));
        assert_eq!(
            game.opponent,
            Some(Faction::Union),
            "a computer on the other side follows your swap"
        );
        game.action(Action::Opponent(Some(Faction::Assembly)));
        game.action(Action::AiLevel(bw_sim::AiLevel::Easy));
        game.start();
        let factions: Vec<_> = game.world.players.iter().map(|p| p.faction).collect();
        assert_eq!(factions, [Faction::Assembly, Faction::Assembly]);
        assert_eq!(game.world.ai_level, bw_sim::AiLevel::Easy);
        // A mirror match plays: both seats build and nothing breaks.
        game.world.ai_level = bw_sim::AiLevel::Normal;
        for _ in 0..3000 {
            game.world.step();
        }
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.owner == 1 && e.kind == Kind::Works)
        );
        game.action(Action::Map(bw_sim::MapId::Confluence));
        game.action(Action::Opponent(None));
        game.start();
        assert_eq!(game.world.players.len(), 3);
        assert_eq!(game.world.players[0].faction, Faction::Assembly);
    }

    #[test]
    fn guide_clamps_page_and_keeps_all_navigation_within_canvas() {
        let mut canvas = Canvas::default();
        let model = MenuModel {
            help_page: 99,
            ..MenuModel::default()
        };
        let buttons = help(&mut canvas, None, &model);
        assert!(buttons.iter().any(|button| button.label == "BACK"));
        // The tabs are the one way between pages.
        assert!(
            buttons
                .iter()
                .any(|button| button.action == Action::GuidePage(GUIDE_PAGES - 1))
        );
        assert!(
            !buttons
                .iter()
                .any(|button| button.label == "PREVIOUS" || button.label == "NEXT")
        );
        assert!(buttons.iter().all(on_canvas));
        assert!(buttons.iter().all(|button| button.hint.is_empty()));
    }

    #[test]
    fn guide_text_fits_its_columns() {
        for page in 0..GUIDE_PAGES {
            if page == 4 || page == ROSTER_PAGE {
                continue;
            }
            let (lines, keys) = guide_page(page, Faction::Union);
            assert!((3..=4).contains(&lines.len()));
            assert!(keys.len() <= 8);
            for line in lines {
                assert!(text_readable_width(line) <= 544, "{line}");
            }
            for (key, action) in keys {
                assert!(text_readable_width(key) <= 72, "{key}");
                assert!(text_readable_width(action) <= 196, "{action}");
            }
        }
    }

    #[test]
    fn confirmation_save_first_is_optional_and_explicit() {
        let mut canvas = Canvas::default();
        let model = MenuModel {
            can_save_before_confirm: true,
            ..MenuModel::default()
        };
        let buttons = confirm(&mut canvas, None, &model);
        assert!(
            buttons
                .iter()
                .any(|button| matches!(button.action, Action::SaveAndConfirm))
        );
        assert!(
            buttons
                .iter()
                .any(|button| matches!(button.action, Action::DismissConfirm))
        );
        assert!(buttons.iter().all(on_canvas));
    }

    #[test]
    fn confirmation_puts_the_losing_answer_last_and_cancel_first() {
        let mut canvas = Canvas::default();
        let model = MenuModel {
            can_save_before_confirm: true,
            overlay: true,
            confirmation_body: "Your opponent wins this match.".into(),
            ..MenuModel::default()
        };
        let buttons = confirm(&mut canvas, None, &model);
        let actions: Vec<_> = buttons.iter().map(|b| b.action.clone()).collect();
        assert_eq!(
            actions,
            [
                Action::DismissConfirm,
                Action::SaveAndConfirm,
                Action::Confirm
            ]
        );
        assert!(buttons.windows(2).all(|p| p[0].x + p[0].w < p[1].x));
        // Over a match the canvas stays clear outside the panel.
        assert_eq!(canvas.get(4, 4), Some([0, 0, 0, 0]));
    }

    #[test]
    fn pause_sets_surrender_apart_and_has_no_save_online() {
        let mut canvas = Canvas::default();
        let offline = pause(&mut canvas, None, &MenuModel::default());
        let last = offline.last().unwrap();
        assert_eq!(last.action, Action::Surrender);
        let before = &offline[offline.len() - 2];
        assert!(last.y - (before.y + before.h) > ROW_STEP - ROW_H);
        assert!(offline.iter().all(on_canvas));
        let online = MenuModel {
            network_session: true,
            save_available: true,
            ..MenuModel::default()
        };
        let buttons = pause(&mut canvas, None, &online);
        for action in [Action::Save, Action::Load] {
            let b = buttons.iter().find(|b| b.action == action).unwrap();
            assert!(!b.enabled);
        }
    }

    #[test]
    fn readable_wrap_keeps_rows_inside_requested_width() {
        let mut canvas = Canvas::default();
        let height = draw_wrapped(
            &mut canvas,
            "A LONG SENTENCE THAT MUST WRAP INSTEAD OF CLIPPING.",
            10,
            10,
            160,
            14,
            WHITE,
        );
        assert!(height > 14);
        assert!(text_readable_width("A LONG SENTENCE") <= 160);
    }

    #[test]
    fn settings_rows_pair_each_label_with_its_value_in_focus_order() {
        let mut canvas = Canvas::default();
        let sound = settings(&mut canvas, None, &MenuModel::default());
        // The four tabs lead, then the open tab's rows, then BACK.
        assert!(matches!(sound[0].action, Action::SettingsTab(0)));
        assert!(matches!(sound[3].action, Action::SettingsTab(3)));
        let rows = &sound[4..];
        assert!(matches!(rows[0].action, Action::Mute));
        assert!(matches!(rows[1].action, Action::Volume(Bus::Effects)));
        assert!(matches!(rows[2].action, Action::Volume(Bus::Ambience)));
        assert!(matches!(rows[3].action, Action::Volume(Bus::Music)));
        assert_eq!(rows[0].hint, "ON");
        assert_eq!(rows[1].hint, "75%");
        assert_eq!(rows[2].hint, "45%");
        assert_eq!(rows[3].hint, "35%");
        assert!(matches!(sound.last().unwrap().action, Action::Back));
        let tab = |settings_tab| {
            let model = MenuModel {
                settings_tab,
                ..MenuModel::default()
            };
            settings(&mut Canvas::default(), None, &model)
        };
        let controls = tab(1);
        assert_eq!(controls[4].hint, "ON");
        assert_eq!(controls[7].hint, "SELECTED");
        assert_eq!(tab(2)[4].hint, "WINDOWED");
        assert_eq!(tab(3).len(), 5, "the keys page is read only");
        for buttons in [sound, controls, tab(2), tab(3)] {
            assert!(
                buttons
                    .iter()
                    .all(|button| button.enabled && on_canvas(button) && button.h < 24)
            );
            for row in buttons.iter().filter(|b| !b.hint.is_empty()) {
                assert!(settings_hint(&row.action).is_some(), "{:?}", row.action);
            }
        }
    }

    #[test]
    fn arrows_change_the_focused_setting_and_turn_the_tabs() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = crate::game::Game::new(base);
        g.resize_view(1280, 720);
        g.action(Action::Settings);
        g.render();
        g.ux.keyboard_navigation = true;
        g.ux.focused = Some(Action::Volume(Bus::Music));
        let music = g.ux.preferences.music_volume;
        g.menu_keyboard("ArrowRight", false);
        assert_eq!(g.ux.preferences.music_volume, music + 1);
        g.menu_keyboard("ArrowLeft", false);
        g.menu_keyboard("ArrowLeft", false);
        assert_eq!(g.ux.preferences.music_volume, music - 1);
        g.menu_keyboard("ArrowRight", true);
        assert_eq!(g.ux.preferences.music_volume, music + 9);
        g.ux.focused = Some(Action::SettingsTab(0));
        g.menu_keyboard("ArrowRight", false);
        assert_eq!(g.ux.settings_tab, 1);
        assert_eq!(g.ux.focused, Some(Action::SettingsTab(1)));
    }

    /// A press on a volume slider sets the level under it, and dragging
    /// moves it a point at a time.
    #[test]
    fn volume_sliders_follow_the_pointer() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = crate::game::Game::new(base);
        g.resize_view(1280, 720);
        g.action(Action::Settings);
        g.render();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Volume(Bus::Music))
            .cloned()
            .expect("music row");
        let (tx, w) = crate::game::volume_track(&b);
        let y = b.y + b.h / 2;
        g.left_down(tx + w * 37 / 100, y);
        assert!(
            (36..=38).contains(&g.ux.preferences.music_volume),
            "{}",
            g.ux.preferences.music_volume
        );
        g.pointer_moved(tx + w, y);
        assert_eq!(g.ux.preferences.music_volume, 100);
        g.pointer_moved(tx - 40, y);
        assert_eq!(g.ux.preferences.music_volume, 0);
        g.left_up(tx - 40, y, false);
        assert_eq!(g.ux.volume_drag, None);
        g.pointer_moved(tx + w / 2, y);
        assert_eq!(g.ux.preferences.music_volume, 0, "no drag after release");
    }

    #[test]
    fn result_leaves_the_field_clear_and_offers_the_replay() {
        let mut canvas = Canvas::default();
        let buttons = result(
            &mut canvas,
            None,
            &MenuModel::default(),
            Outcome::Victory(0),
        );
        // Below the actions the field shows through.
        let pixel = ((350 * 640 + 4) * 4) as usize;
        assert_eq!(&canvas.pixels[pixel..pixel + 4], &[0, 0, 0, 0]);
        let actions: Vec<Action> = buttons
            .iter()
            .filter(|b| !matches!(b.action, Action::ResultPage(_)))
            .map(|b| b.action.clone())
            .collect();
        assert_eq!(
            actions,
            [
                Action::MainMenu,
                Action::ExportReplay,
                Action::WatchReplay,
                Action::Start
            ]
        );
    }

    /// A finished tide match of two seats, won by `winner`, with a count
    /// broken six times.
    fn finished(winner: u8) -> MenuModel {
        use crate::match_summary::{HoldRun, Loss, MatchSummary, Sample};
        let mut summary = MatchSummary::default();
        summary.factions = vec![Some(Faction::Union), Some(Faction::Assembly)];
        for i in 0..40u32 {
            summary.samples.push(Sample {
                tick: u64::from(i) * 900,
                army: vec![i * 150 % 2600, i * 170 % 3100],
                income: vec![300 + i * 9, 280 + i * 11],
                crew: vec![6 + i, 6 + i],
                sluice: (i > 8).then_some(if i < 20 { 0 } else { winner }),
                hold: vec![0, 0],
            });
        }
        for i in 0..9u64 {
            summary.losses.push(Loss {
                tick: 12_000 + i * 60,
                owner: u8::from(winner == 0),
                kind: Kind::Riveter,
                cause: Some(Kind::Loom),
                pos: None,
                killer: Some(winner),
            });
        }
        summary.holds.push(HoldRun {
            player: winner,
            start: 30_000,
            end: Some(35_100),
            peak: bw_content::TIDE_HOLD_TICKS,
            won: true,
            breaks: 6,
            holding: true,
        });
        summary.outcome = Some((35_100, Outcome::Victory(winner)));
        summary.tide = true;
        MenuModel {
            summary,
            ..MenuModel::default()
        }
    }

    #[test]
    fn the_result_leads_with_the_outcome_in_the_winners_colour() {
        for (winner, colour) in [(0, crate::canvas::JADE), (1, crate::canvas::RED)] {
            let mut canvas = Canvas::default();
            let model = finished(winner);
            result(&mut canvas, None, &model, Outcome::Victory(winner));
            if let Ok(dir) = std::env::var("BW_RESULT_FRAMES") {
                std::fs::create_dir_all(&dir).expect("frame folder");
                canvas
                    .save(&std::path::PathBuf::from(dir).join(format!("result-{winner}.png")))
                    .expect("frame");
            }
            // The big word is drawn in the winner's colour in the banner.
            let lit = (18..56).any(|y| {
                (200..440).any(|x| {
                    let at = ((y * 640 + x) * 4) as usize;
                    canvas.pixels[at..at + 4] == colour
                })
            });
            assert!(lit, "seat {winner} wins in its colour");
            // The long tide line is one line, whole: it fits the banner.
            let reason = model.summary.decided_line_for(Some(0));
            assert!(
                reason.contains("BROKEN 6 TIMES BUT NEVER EMPTIED."),
                "{reason}"
            );
            assert!(reason.len() as i32 * 6 + 14 <= 640 - 32, "{reason}");
        }
    }

    #[test]
    fn a_three_seat_result_names_the_winner_in_its_colour() {
        let mut model = finished(2);
        let summary = &mut model.summary;
        summary.factions.push(Some(Faction::Union));
        for sample in &mut summary.samples {
            sample.army.push(sample.army[0] / 2);
            sample.income.push(sample.income[1] + 20);
            sample.crew.push(10);
            sample.hold.push(0);
        }
        summary.eliminated = vec![(1, 20_000), (0, 30_000)];
        // The winner's own colour, the same on every screen.
        let winner = model.summary.colour(2);
        let mut canvas = Canvas::default();
        result(&mut canvas, None, &model, Outcome::Victory(2));
        if let Ok(dir) = std::env::var("BW_RESULT_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            canvas
                .save(&std::path::PathBuf::from(dir).join("result-three.png"))
                .expect("frame");
        }
        let lit = (18..56).any(|y| {
            (200..440).any(|x| {
                let at = ((y * 640 + x) * 4) as usize;
                canvas.pixels[at..at + 4] == winner
            })
        });
        assert!(lit, "the winner is named in its colour");
    }

    #[test]
    fn practice_review_has_end_review_and_replay_controls() {
        let mut canvas = Canvas::default();
        let buttons = practice_review(&mut canvas, None, &MenuModel::default());
        assert!(
            buttons
                .iter()
                .any(|button| button.action == Action::EndReview)
        );
        assert!(buttons.iter().any(|button| button.action == Action::Save));
        assert!(
            buttons
                .iter()
                .any(|button| button.action == Action::ExportReplay)
        );
    }
}
