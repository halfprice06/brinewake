//! Playing online: the lobby screen, and the network's few marks on the
//! field.
//!
//! The lobby is one screen with four stages. Before anything: host, or
//! paste a code and join. Hosting: the code to share, whether the router
//! opened, the seats, a side and start. Reaching a host: a spinner, and
//! after a few seconds a reply code for the host. Seated: the seats, a side
//! and ready.
//!
//! On the field the network stays out of the way: a signal mark by the
//! clock, a card while the match waits for a silent player (with the time
//! before that player is dropped, and on the host a KEEP SEAT OPEN button),
//! a card while a seat is kept open for its player to come back, and a card
//! when the match cannot go on.

use crate::canvas::{Atlas, Canvas, Color, EDGE, GOLD, INK, JADE, MUTED, RED, WHITE};
use crate::game::{Action, Button, Game, Screen};
use crate::menus::{faction_accent, faction_emblem_key, menu_button, panel, shell};
use crate::native_ui::{Rect, button, card, small, text};
use crate::net::{self, JoinCode, Lobby, LobbyOptions, Phase, protocol::SeatInfo};
use bw_core::Faction;
use bw_sim::MapId;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Everything the lobby's controls do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetAction {
    Host,
    Join,
    Rejoin,
    Paste,
    CopyCode,
    CopyReply,
    Side(Option<Faction>),
    Ready,
    Start,
    Leave,
    /// Edit this seat's name, or keep the edit.
    EditName,
    /// Host: play the next map.
    SwitchMap,
    EditReply,
    AddReply,
    /// Leave a match that cannot go on.
    EndMatch,
    /// Host: keep a silent player's seat open while the others play on.
    KeepSeatOpen(u8),
}

/// The lobby screen's own state, kept by the game.
#[derive(Default)]
pub struct NetUi {
    /// The code or address being typed, or a reply code on the host.
    pub field: String,
    /// The host is typing a guest's reply code.
    pub editing_reply: bool,
    /// This seat's name is being typed, into `name_field`.
    pub editing_name: bool,
    pub name_field: String,
    pub error: Option<String>,
    /// The frame something was copied, for a moment's acknowledgement.
    pub copied_at: Option<u64>,
    clipboard: Option<arboard::Clipboard>,
    /// The code this game last joined with, kept for rejoining.
    pub joined_with: Option<String>,
    /// A rejoin note is on disk.
    note_written: bool,
    /// The stage last drawn: focus starts afresh on a new stage.
    drawn_stage: Option<std::mem::Discriminant<Stage>>,
}

/// Enough to rejoin a match this game left without finishing.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RejoinNote {
    code: String,
    token: u64,
    unix_seconds: u64,
}

const REJOIN_FILE: &str = "online-rejoin.json";
/// A rejoin older than the drop grace and the longest a host keeps a seat
/// open is worthless; a little slack.
const REJOIN_FOR_SECONDS: u64 =
    net::session::DROP_AFTER.as_secs() + net::session::KEEP_OPEN.as_secs() + 30;

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_rejoin(data_dir: &Path) -> Option<RejoinNote> {
    let bytes = std::fs::read(data_dir.join("saves").join(REJOIN_FILE)).ok()?;
    let note: RejoinNote = serde_json::from_slice(&bytes).ok()?;
    (unix_now().saturating_sub(note.unix_seconds) <= REJOIN_FOR_SECONDS).then_some(note)
}

/// Where the lobby stands, as the screen draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    Choose,
    Hosting,
    Reaching,
    Seated,
    Failed(String),
}

/// Plain data for the lobby screen, so it can be drawn without a network.
#[derive(Clone, Debug)]
pub struct LobbyModel {
    pub stage: Stage,
    pub code: Option<String>,
    pub reply: Option<String>,
    pub forwarded: Option<bool>,
    pub port: u16,
    pub seats: Vec<SeatInfo>,
    pub you: Option<u8>,
    pub seed: u64,
    pub map: MapId,
    pub field: String,
    pub editing_reply: bool,
    /// The name this seat goes by, and the name being typed.
    pub name: String,
    pub editing_name: bool,
    pub name_field: String,
    /// The pointer is on the host's map control: the seed shows.
    pub map_focus: bool,
    pub error: Option<String>,
    pub can_start: Result<(), String>,
    pub copied: bool,
    pub frame: u64,
    pub rejoin: bool,
}

impl Default for LobbyModel {
    fn default() -> Self {
        LobbyModel {
            stage: Stage::Choose,
            code: None,
            reply: None,
            forwarded: None,
            port: net::code::DEFAULT_PORT,
            seats: Vec::new(),
            you: None,
            seed: 0,
            map: MapId::default(),
            field: String::new(),
            editing_reply: false,
            name: String::new(),
            editing_name: false,
            name_field: String::new(),
            map_focus: false,
            error: None,
            can_start: Err(String::new()),
            copied: false,
            frame: 0,
            rejoin: false,
        }
    }
}

// ------------------------------------------------------------------ marks

/// Signal bars: four when the round trip is short, one when it is long,
/// grey before it is measured.
pub fn signal_bars(canvas: &mut Canvas, x: i32, y: i32, rtt_ms: Option<u32>, scale: i32) {
    let (lit, color) = match rtt_ms {
        None => (0, EDGE),
        Some(ms) if ms < 80 => (4, JADE),
        Some(ms) if ms < 150 => (3, JADE),
        Some(ms) if ms < 250 => (2, GOLD),
        Some(_) => (1, RED),
    };
    for bar in 0..4 {
        let h = (3 + bar * 2) * scale;
        let bx = x + bar * 3 * scale;
        let by = y + 9 * scale - h;
        canvas.rect(bx, by, 2 * scale, h, if bar < lit { color } else { EDGE });
    }
}

/// A tick mark: ready.
pub fn check(canvas: &mut Canvas, x: i32, y: i32, color: Color, scale: i32) {
    for (dx, dy) in [
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 5),
        (4, 4),
        (5, 3),
        (6, 2),
        (7, 1),
    ] {
        canvas.rect(x + dx * scale, y + dy * scale, scale, 2 * scale, color);
    }
}

/// An open eye: watching.
pub fn eye(canvas: &mut Canvas, x: i32, y: i32, color: Color, scale: i32) {
    let rows: [&str; 7] = [
        "...#####...",
        ".##.....##.",
        "#....#....#",
        "#...###...#",
        "#....#....#",
        ".##.....##.",
        "...#####...",
    ];
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if ch == '#' {
                canvas.rect(
                    x + dx as i32 * scale,
                    y + dy as i32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

/// Three dots that take turns: something is on its way.
pub fn spinner(canvas: &mut Canvas, x: i32, y: i32, frame: u64, scale: i32) {
    let on = (frame / 10 % 3) as i32;
    for dot in 0..3 {
        canvas.rect(
            x + dot * 5 * scale,
            y,
            3 * scale,
            3 * scale,
            if dot == on { GOLD } else { EDGE },
        );
    }
}

// ----------------------------------------------------------------- screen

/// A text field. `frame` blinks the caret; `None` is a field not being
/// typed into.
fn field_box(
    canvas: &mut Canvas,
    bounds: [i32; 4],
    text: &str,
    placeholder: &str,
    frame: Option<u64>,
) {
    let [x, y, w, h] = bounds;
    canvas.rect(x, y, w, h, INK);
    canvas.frame(x, y, w, h, GOLD);
    let room = ((w - 12) / 8).max(1) as usize;
    if text.is_empty() {
        canvas.text_readable(placeholder, x + 6, y + (h - 9) / 2, MUTED);
    } else {
        // The end of a long entry, where the typing is.
        let skip = text.chars().count().saturating_sub(room);
        let shown: String = text.chars().skip(skip).collect();
        canvas.text_readable(&shown, x + 6, y + (h - 9) / 2, WHITE);
    }
    if frame.is_some_and(|f| (f / 30).is_multiple_of(2)) {
        let caret = x + 6 + (text.chars().count().min(room) as i32) * 8;
        canvas.rect(caret, y + 4, 1, h - 8, GOLD);
    }
}

/// The side after `side` on a seat's toggle: each faction, then watching.
fn next_side(side: Option<Faction>) -> Option<Faction> {
    match side {
        Some(faction) => {
            let at = Faction::ALL.iter().position(|f| *f == faction).unwrap_or(0);
            Faction::ALL.get(at + 1).copied()
        }
        None => Some(Faction::ALL[0]),
    }
}

fn side_name(side: Option<Faction>) -> &'static str {
    match side {
        Some(Faction::Union) => "UNION",
        Some(Faction::Assembly) => "ASSEMBLY",
        Some(Faction::Compact) => "COMPACT",
        None => "WATCH",
    }
}

/// One row per seat: crest, name, side, signal and ready. Your own row
/// carries the toggle that changes your side.
fn seat_rows(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    model: &LobbyModel,
    buttons: &mut Vec<Button>,
    [x, y, w]: [i32; 3],
) {
    let shown = model.seats.len().max(net::protocol::sides(model.map));
    for row in 0..shown {
        let ry = y + row as i32 * 26;
        let seat = model.seats.get(row);
        let mine = model.you == Some(row as u8);
        panel(canvas, x, ry, w, 24, if mine { GOLD } else { EDGE });
        let Some(seat) = seat else {
            // An open seat breathes while it waits.
            let lit = (model.frame / 40).is_multiple_of(2);
            canvas.text_readable("OPEN", x + 38, ry + 8, if lit { MUTED } else { EDGE });
            continue;
        };
        match seat.faction {
            Some(faction) => {
                let drawn = atlas
                    .is_some_and(|a| a.draw(canvas, faction_emblem_key(faction), x + 6, ry, false));
                if !drawn {
                    canvas.rect(x + 10, ry + 6, 12, 12, faction_accent(faction));
                }
            }
            None => eye(canvas, x + 12, ry + 9, MUTED, 1),
        }
        let color = match seat.faction {
            Some(faction) => faction_accent(faction),
            None => MUTED,
        };
        canvas.text_readable(&seat.name, x + 38, ry + 8, color);
        if mine {
            // A small chevron: this seat is you.
            for i in 0..3 {
                canvas.rect(
                    x + 38 + text_width(&seat.name) + 6 + i,
                    ry + 9 + i,
                    1,
                    7 - 2 * i,
                    GOLD,
                );
            }
            buttons.push(menu_button(
                [x + w - 180, ry + 2, 104, 20],
                side_name(seat.faction),
                "",
                Action::Net(NetAction::Side(next_side(seat.faction))),
                true,
            ));
        } else {
            canvas.text_readable(side_name(seat.faction), x + w - 176, ry + 8, color);
        }
        if row == 0 {
            // The host's own seat has no round trip; it carries a crown.
            for (dx, h) in [(0, 5), (2, 3), (4, 5), (6, 3), (8, 5)] {
                canvas.rect(x + w - 58 + dx, ry + 13 - h, 2, h, GOLD);
            }
            canvas.rect(x + w - 58, ry + 13, 10, 2, GOLD);
        } else {
            signal_bars(canvas, x + w - 58, ry + 7, seat.rtt_ms, 1);
        }
        if seat.faction.is_some() && (seat.ready || row == 0) {
            check(canvas, x + w - 26, ry + 7, JADE, 1);
        }
    }
}

/// The lobby's map: its picture and name, which the host clicks to play the
/// next map. The match's seed shows only while the host points at it.
fn map_panel(canvas: &mut Canvas, model: &LobbyModel, buttons: &mut Vec<Button>, x: i32, y: i32) {
    panel(canvas, x, y, 176, 90, EDGE);
    crate::menus::map_thumbnail(canvas, model.map, x + 36, y + 8);
    let layout = model.map.layout();
    let name = layout.name.strip_prefix("THE ").unwrap_or(layout.name);
    if model.stage == Stage::Hosting {
        buttons.push(menu_button(
            [x, y + 68, 176, 22],
            name,
            "CHANGE",
            Action::Net(NetAction::SwitchMap),
            true,
        ));
        if model.map_focus {
            canvas.text(&format!("SEED {}", model.seed), x, y + 96, MUTED);
        }
    } else {
        canvas.text_readable(name, x + 4, y + 73, WHITE);
    }
}

fn text_width(s: &str) -> i32 {
    crate::canvas::text_readable_width(s)
}

fn wrapped(canvas: &mut Canvas, message: &str, x: i32, y: i32, width: i32, color: Color) -> i32 {
    crate::menus::draw_wrapped(canvas, message, x, y, width, 13, color)
}

/// Draw the lobby and return its controls.
pub fn lobby_screen(canvas: &mut Canvas, atlas: Option<&Atlas>, model: &LobbyModel) -> Vec<Button> {
    let mut buttons = Vec::new();
    let act = |a: NetAction| Action::Net(a);
    let mut error_y = 250;
    match &model.stage {
        Stage::Choose => {
            shell(canvas, "PLAY ONLINE", "");
            // The name the other players see, kept between sessions.
            if model.editing_name {
                field_box(
                    canvas,
                    [24, 56, 284, 22],
                    &model.name_field,
                    "YOUR NAME",
                    Some(model.frame),
                );
                buttons.push(menu_button(
                    [316, 56, 72, 22],
                    "OK",
                    "",
                    act(NetAction::EditName),
                    true,
                ));
            } else {
                buttons.push(menu_button(
                    [24, 56, 284, 22],
                    "NAME",
                    &model.name,
                    act(NetAction::EditName),
                    true,
                ));
            }
            panel(canvas, 24, 90, 284, 150, GOLD);
            canvas.text_readable("HOST", 40, 104, GOLD);
            canvas.text_readable("SHARE A CODE.", 40, 122, MUTED);
            canvas.text("FRIENDS JOIN WITH IT.", 40, 140, MUTED);
            // Both maps, two players or three.
            for (i, map) in MapId::ALL.into_iter().enumerate() {
                crate::menus::map_thumbnail(canvas, map, 40 + i as i32 * 148, 148);
            }
            buttons.push(menu_button(
                [40, 204, 252, 24],
                "HOST",
                "",
                act(NetAction::Host),
                atlas.is_some(),
            ));
            panel(canvas, 332, 90, 284, 150, JADE);
            canvas.text_readable("JOIN", 348, 104, JADE);
            field_box(
                canvas,
                [348, 130, 252, 24],
                &model.field,
                "PASTE A CODE",
                (!model.editing_name).then_some(model.frame),
            );
            buttons.push(menu_button(
                [348, 164, 120, 24],
                "PASTE",
                "",
                act(NetAction::Paste),
                true,
            ));
            buttons.push(menu_button(
                [480, 164, 120, 24],
                "JOIN",
                "",
                act(NetAction::Join),
                !model.field.trim().is_empty() && atlas.is_some(),
            ));
            if model.rejoin {
                buttons.push(menu_button(
                    [348, 204, 252, 24],
                    "REJOIN",
                    "LAST MATCH",
                    act(NetAction::Rejoin),
                    atlas.is_some(),
                ));
            }
            buttons.push(menu_button(
                [24, 316, 120, 24],
                "BACK",
                "",
                Action::Back,
                true,
            ));
        }
        Stage::Hosting => {
            shell(canvas, "HOST", "");
            panel(canvas, 24, 52, 592, 58, GOLD);
            canvas.text("SHARE THIS CODE", 38, 58, MUTED);
            match &model.code {
                Some(code) => {
                    // The code is the whole job here: big, with COPY beside it.
                    if text_width(code) * 2 <= 448 {
                        canvas.text_readable_scaled(code, 38, 70, GOLD, 2);
                    } else {
                        canvas.text_readable(code, 38, 76, GOLD);
                    }
                    buttons.push(menu_button(
                        [500, 66, 104, 26],
                        if model.copied { "COPIED" } else { "COPY" },
                        "",
                        act(NetAction::CopyCode),
                        true,
                    ));
                }
                None => spinner(canvas, 38, 72, model.frame, 2),
            }
            // Whether a guest outside this network can get in.
            match model.forwarded {
                Some(true) => {
                    check(canvas, 38, 94, JADE, 1);
                    canvas.text("ROUTER OPEN", 52, 96, MUTED);
                }
                Some(false) => {
                    canvas.rect(39, 94, 2, 5, GOLD);
                    canvas.rect(39, 100, 2, 2, GOLD);
                    canvas.text("ROUTER CLOSED", 48, 96, MUTED);
                }
                None => spinner(canvas, 38, 96, model.frame, 1),
            }
            seat_rows(canvas, atlas, model, &mut buttons, [24, 122, 392]);
            map_panel(canvas, model, &mut buttons, 440, 122);
            error_y = 208;
            if model.editing_reply {
                // The way in when a friend's game can't reach this one.
                panel(canvas, 24, 226, 592, 82, JADE);
                let mut help = String::from(
                    "THEIR GAME SHOWS A REPLY CODE WHEN IT CAN'T REACH YOU. PASTE IT HERE.",
                );
                if model.forwarded == Some(false) {
                    help.push_str(&format!(
                        " YOU CAN ALSO FORWARD UDP {} ON YOUR ROUTER.",
                        model.port
                    ));
                }
                wrapped(canvas, &help, 38, 234, 564, WHITE);
                field_box(
                    canvas,
                    [38, 276, 372, 24],
                    &model.field,
                    "THEIR REPLY CODE",
                    Some(model.frame),
                );
                buttons.push(menu_button(
                    [418, 276, 88, 24],
                    "ADD",
                    "",
                    act(NetAction::AddReply),
                    !model.field.trim().is_empty(),
                ));
                buttons.push(menu_button(
                    [514, 276, 88, 24],
                    "CLOSE",
                    "",
                    act(NetAction::EditReply),
                    true,
                ));
            } else {
                buttons.push(menu_button(
                    [24, 284, 200, 22],
                    "FRIEND CAN'T CONNECT?",
                    "",
                    act(NetAction::EditReply),
                    true,
                ));
            }
            if let Err(why) = &model.can_start
                && !why.is_empty()
            {
                canvas.text(&why.to_ascii_uppercase(), 160, 324, MUTED);
            }
            buttons.push(menu_button(
                [440, 316, 176, 24],
                "START",
                "",
                act(NetAction::Start),
                model.can_start.is_ok() && atlas.is_some(),
            ));
            buttons.push(menu_button(
                [24, 316, 120, 24],
                "LEAVE",
                "",
                act(NetAction::Leave),
                true,
            ));
        }
        Stage::Reaching => {
            shell(canvas, "JOIN", "");
            spinner(canvas, 24, 70, model.frame, 3);
            canvas.text_readable("REACHING THE HOST", 76, 69, WHITE);
            if let Some(reply) = &model.reply {
                panel(canvas, 24, 100, 592, 78, GOLD);
                canvas.text_readable("NO ANSWER YET. SEND THIS BACK:", 38, 112, MUTED);
                canvas.text_readable(reply, 38, 132, GOLD);
                buttons.push(menu_button(
                    [508, 146, 96, 24],
                    if model.copied { "COPIED" } else { "COPY" },
                    "",
                    act(NetAction::CopyReply),
                    true,
                ));
            }
            buttons.push(menu_button(
                [24, 316, 120, 24],
                "LEAVE",
                "",
                act(NetAction::Leave),
                true,
            ));
        }
        Stage::Seated => {
            shell(canvas, "LOBBY", "");
            seat_rows(canvas, atlas, model, &mut buttons, [24, 60, 392]);
            map_panel(canvas, model, &mut buttons, 440, 60);
            error_y = 206;
            let ready = model
                .you
                .and_then(|y| model.seats.get(y as usize))
                .is_some_and(|s| s.ready);
            let playing = model
                .you
                .and_then(|y| model.seats.get(y as usize))
                .is_some_and(|s| s.faction.is_some());
            if playing {
                buttons.push(menu_button(
                    [440, 316, 176, 24],
                    if ready { "NOT READY" } else { "READY" },
                    "",
                    act(NetAction::Ready),
                    true,
                ));
            }
            canvas.text(
                if ready || !playing {
                    "THE HOST STARTS THE MATCH"
                } else {
                    ""
                },
                160,
                324,
                MUTED,
            );
            buttons.push(menu_button(
                [24, 316, 120, 24],
                "LEAVE",
                "",
                act(NetAction::Leave),
                true,
            ));
        }
        Stage::Failed(reason) => {
            shell(canvas, "PLAY ONLINE", "");
            panel(canvas, 24, 56, 592, 80, RED);
            wrapped(canvas, reason, 38, 70, 560, WHITE);
            buttons.push(menu_button(
                [24, 316, 120, 24],
                "BACK",
                "",
                act(NetAction::Leave),
                true,
            ));
        }
    }
    if let Some(error) = &model.error {
        wrapped(canvas, error, 24, error_y, 592, RED);
    }
    buttons
}

// ------------------------------------------------------------ the game

impl Game {
    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.net_ui.clipboard.is_none() {
            self.net_ui.clipboard = arboard::Clipboard::new().ok();
        }
        self.net_ui.clipboard.as_mut()
    }

    fn copy_text(&mut self, text: String) {
        match self.clipboard().map(|c| c.set_text(text)) {
            Some(Ok(())) => self.net_ui.copied_at = Some(self.frame),
            _ => {
                self.net_ui.error = Some(
                    "Couldn't reach the clipboard. Select the code and copy it by hand.".into(),
                )
            }
        }
    }

    fn paste_text(&mut self) -> Option<String> {
        let text = self.clipboard().and_then(|c| c.get_text().ok());
        if text.is_none() {
            self.net_ui.error = Some("Nothing to paste.".into());
        }
        text.map(|t| {
            t.lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("")
                .to_string()
        })
    }

    /// Open the online screen at its first stage.
    pub fn open_online(&mut self) {
        self.net_ui.error = None;
        self.net_ui.field.clear();
        self.net_ui.editing_reply = false;
        self.net_ui.editing_name = false;
        self.open_screen(Screen::Lobby);
    }

    fn online_options(&self) -> LobbyOptions {
        LobbyOptions {
            name: self.online_name(),
            ..LobbyOptions::default()
        }
    }

    /// The name this seat goes by online: the one typed on the online
    /// screen, else the computer's user name.
    pub(crate) fn online_name(&self) -> String {
        match self.ux.preferences.online_name.as_str() {
            "" => net::protocol::local_name(),
            name => name.to_string(),
        }
    }

    fn join_with(&mut self, text: &str, rejoin: Option<u64>) {
        self.net_ui.error = None;
        let code = match JoinCode::parse(text) {
            Ok(code) => code,
            Err(e) => {
                self.net_ui.error = Some(e);
                return;
            }
        };
        match Lobby::join(&code, rejoin, self.online_options()) {
            Ok(lobby) => {
                self.net_ui.joined_with = Some(text.trim().to_string());
                self.lobby = Some(lobby);
            }
            Err(e) => self.net_ui.error = Some(e),
        }
    }

    pub fn net_action(&mut self, action: NetAction) {
        // Hosting or joining keeps a name still being typed.
        if self.net_ui.editing_name
            && matches!(
                action,
                NetAction::Host | NetAction::Join | NetAction::Rejoin
            )
        {
            self.net_action(NetAction::EditName);
        }
        match action {
            NetAction::Host => {
                self.net_ui.error = None;
                let seed = net::link::random_id() % 100_000;
                let bind = format!("0.0.0.0:{}", net::code::DEFAULT_PORT);
                match Lobby::host(&bind, self.faction, seed, self.online_options()) {
                    Ok(mut lobby) => {
                        lobby.set_map(self.map);
                        self.lobby = Some(lobby);
                    }
                    Err(e) => self.net_ui.error = Some(e),
                }
            }
            NetAction::Join => {
                let text = self.net_ui.field.clone();
                self.join_with(&text, None);
            }
            NetAction::Rejoin => {
                if let Some(note) = read_rejoin(&self.data_dir) {
                    self.join_with(&note.code, Some(note.token));
                } else {
                    self.net_ui.error = Some("That match is over.".into());
                }
            }
            NetAction::Paste if self.net_ui.editing_name => {
                if let Some(text) = self.paste_text() {
                    self.net_ui.name_field = text
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
                        .map(|c| c.to_ascii_uppercase())
                        .take(10)
                        .collect();
                }
            }
            NetAction::Paste => {
                if let Some(text) = self.paste_text() {
                    self.net_ui.field = text.chars().take(80).collect();
                    // A host's code is the whole job: join at once.
                    if !self.net_ui.editing_reply
                        && self.lobby.is_none()
                        && JoinCode::parse(&self.net_ui.field)
                            .is_ok_and(|c| c.kind == net::code::CodeKind::Host)
                    {
                        let text = self.net_ui.field.clone();
                        self.join_with(&text, None);
                    }
                }
            }
            NetAction::CopyCode => {
                if let Some(code) = self
                    .lobby
                    .as_ref()
                    .and_then(|l| l.code())
                    .map(str::to_string)
                {
                    self.copy_text(code);
                }
            }
            NetAction::CopyReply => {
                if let Some(code) = self
                    .lobby
                    .as_ref()
                    .and_then(|l| l.reply_code())
                    .map(str::to_string)
                {
                    self.copy_text(code);
                }
            }
            NetAction::Side(side) => {
                if let Some(lobby) = self.lobby.as_mut() {
                    lobby.choose(side);
                    if let Some(f) = side {
                        self.faction = f;
                    }
                }
            }
            NetAction::Ready => {
                if let Some(lobby) = self.lobby.as_mut() {
                    let ready = lobby.ready();
                    lobby.set_ready(!ready);
                }
            }
            NetAction::Start => {
                if let Some(lobby) = self.lobby.as_mut() {
                    match lobby.start() {
                        Ok(session) => self.begin_online_match(session),
                        Err(e) => self.net_ui.error = Some(e),
                    }
                }
            }
            NetAction::Leave => {
                if let Some(lobby) = self.lobby.take() {
                    lobby.leave();
                    self.open_online();
                } else {
                    self.open_online();
                    self.open_screen(Screen::Menu);
                }
            }
            NetAction::EditName => {
                if self.net_ui.editing_name {
                    let name = net::protocol::clean_name(&self.net_ui.name_field);
                    self.ux.preferences.online_name = name;
                    if self.ux.persist(&self.data_dir).is_err() {
                        self.notify("Name changed for this session; could not save settings.");
                    }
                }
                self.net_ui.editing_name = !self.net_ui.editing_name;
                self.net_ui.name_field = self.online_name();
            }
            NetAction::SwitchMap => {
                if let Some(lobby) = self.lobby.as_mut() {
                    let at = MapId::ALL.iter().position(|m| *m == lobby.map).unwrap_or(0);
                    let next = MapId::ALL[(at + 1) % MapId::ALL.len()];
                    lobby.set_map(next);
                    self.map = next;
                }
            }
            NetAction::EditReply => {
                self.net_ui.editing_reply = !self.net_ui.editing_reply;
                self.net_ui.field.clear();
                self.net_ui.error = None;
            }
            NetAction::KeepSeatOpen(seat) => {
                if let Some(session) = self.session.as_mut()
                    && let Err(why) = session.keep_seat_open(seat)
                {
                    self.notify(&why);
                }
            }
            NetAction::EndMatch => {
                self.end_network();
                self.ux.session_active = false;
                self.open_screen(Screen::Menu);
            }
            NetAction::AddReply => {
                let text = self.net_ui.field.clone();
                if let Some(lobby) = self.lobby.as_mut() {
                    match lobby.add_reply(&text) {
                        Ok(()) => {
                            self.net_ui.editing_reply = false;
                            self.net_ui.field.clear();
                            self.net_ui.error = None;
                            self.notify("Reaching out to your guest.");
                        }
                        Err(e) => self.net_ui.error = Some(e),
                    }
                }
            }
        }
    }

    /// Once a frame: the lobby hears the network, and a match that has
    /// started takes over.
    pub fn frame_update(&mut self) {
        let Some(lobby) = self.lobby.as_mut() else {
            return;
        };
        let started = lobby.poll();
        for notice in lobby.take_notices() {
            self.notify(&notice);
        }
        if let Some(session) = started {
            self.begin_online_match(session);
        }
    }

    fn begin_online_match(&mut self, session: net::Session) {
        let token = self.lobby.as_ref().and_then(|l| l.token);
        self.lobby = None;
        self.net_ui = NetUi {
            clipboard: self.net_ui.clipboard.take(),
            joined_with: self.net_ui.joined_with.take(),
            ..NetUi::default()
        };
        // First the field (which ends any earlier match and its note), then
        // the note for coming back to this one.
        self.start_network(session);
        self.open_screen(Screen::Match);
        if let (Some(token), Some(code)) = (token, self.net_ui.joined_with.clone()) {
            let note = RejoinNote {
                code,
                token,
                unix_seconds: unix_now(),
            };
            let dir = self.data_dir.join("saves");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(
                dir.join(REJOIN_FILE),
                serde_json::to_vec(&note).unwrap_or_default(),
            );
            self.net_ui.note_written = true;
        }
    }

    /// Keep the rejoin note fresh while the match runs, and drop it once
    /// there is nothing to rejoin.
    pub(crate) fn touch_rejoin_note(&mut self) {
        if !self.net_ui.note_written {
            return;
        }
        let path = self.data_dir.join("saves").join(REJOIN_FILE);
        let live = self.session.as_ref().is_some_and(|s| {
            !s.is_host() && s.ending().is_none() && s.canonical().outcome.is_none()
        });
        if !live {
            self.forget_rejoin();
            return;
        }
        if self.world.tick.is_multiple_of(150)
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(mut note) = serde_json::from_slice::<RejoinNote>(&bytes)
        {
            note.unix_seconds = unix_now();
            let _ = std::fs::write(&path, serde_json::to_vec(&note).unwrap_or_default());
        }
    }

    /// Leave any network match, saying goodbye.
    pub fn end_network(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.close();
        }
        self.forget_rejoin();
    }

    /// Nothing to come back to: this seat left on purpose.
    pub fn forget_rejoin(&mut self) {
        if self.net_ui.note_written {
            self.net_ui.note_written = false;
            let _ = std::fs::remove_file(self.data_dir.join("saves").join(REJOIN_FILE));
        }
    }

    fn lobby_model(&mut self) -> LobbyModel {
        let rejoin = read_rejoin(&self.data_dir).is_some();
        let copied = self
            .net_ui
            .copied_at
            .is_some_and(|at| self.frame.saturating_sub(at) < 90);
        let mut model = LobbyModel {
            field: self.net_ui.field.clone(),
            editing_reply: self.net_ui.editing_reply,
            name: self.online_name(),
            editing_name: self.net_ui.editing_name,
            name_field: self.net_ui.name_field.clone(),
            map_focus: self.ux.focused == Some(Action::Net(NetAction::SwitchMap)),
            error: self.net_ui.error.clone(),
            copied,
            frame: self.frame,
            rejoin,
            ..LobbyModel::default()
        };
        let Some(lobby) = self.lobby.as_mut() else {
            return model;
        };
        model.forwarded = lobby.forwarded();
        model.stage = match &lobby.phase {
            Phase::Failed(reason) => Stage::Failed(reason.clone()),
            Phase::Open => Stage::Hosting,
            Phase::Reaching => Stage::Reaching,
            Phase::Seated => Stage::Seated,
        };
        model.code = lobby.code().map(str::to_string);
        model.reply = lobby.reply_code().map(str::to_string);
        model.port = lobby
            .local_addr()
            .map_or(net::code::DEFAULT_PORT, |a| a.port());
        model.seats = lobby.seats.clone();
        model.you = lobby.you;
        model.seed = lobby.seed;
        model.map = lobby.map;
        model.can_start = lobby.can_start();
        model
    }

    pub(crate) fn draw_lobby(&mut self) {
        let model = self.lobby_model();
        let stage = std::mem::discriminant(&model.stage);
        if self.net_ui.drawn_stage != Some(stage) {
            self.net_ui.drawn_stage = Some(stage);
            self.ux.focused = None;
        }
        self.buttons = lobby_screen(&mut self.canvas, self.atlas.as_ref(), &model);
    }

    /// Typing on the lobby screen. True when the key was taken.
    pub(crate) fn online_key(&mut self, key: &str, control: bool) -> bool {
        if self.screen != Screen::Lobby {
            return false;
        }
        let typing = self.lobby.is_none() || self.net_ui.editing_reply;
        if control {
            match key {
                "V" if typing => self.net_action(NetAction::Paste),
                "C" => {
                    if self.lobby.as_ref().is_some_and(|l| l.code().is_some()) {
                        self.net_action(NetAction::CopyCode);
                    } else {
                        self.net_action(NetAction::CopyReply);
                    }
                }
                _ => return false,
            }
            return true;
        }
        if self.net_ui.editing_name && self.lobby.is_none() && !control {
            match key {
                "Escape" => self.net_ui.editing_name = false,
                "Enter" => self.net_action(NetAction::EditName),
                "Backspace" => {
                    self.net_ui.name_field.pop();
                }
                _ => {
                    let mut chars = key.chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None)
                            if (c.is_ascii_alphanumeric() || c == ' ')
                                && self.net_ui.name_field.chars().count() < 10 =>
                        {
                            self.net_ui.name_field.push(c.to_ascii_uppercase());
                        }
                        _ => return false,
                    }
                }
            }
            return true;
        }
        if key == "Escape" {
            if self.net_ui.editing_reply {
                self.net_action(NetAction::EditReply);
            } else {
                self.net_action(NetAction::Leave);
            }
            return true;
        }
        if !typing {
            return false;
        }
        match key {
            "Backspace" => {
                self.net_ui.field.pop();
            }
            "Enter" => self.net_action(if self.net_ui.editing_reply {
                NetAction::AddReply
            } else {
                NetAction::Join
            }),
            _ => {
                let mut chars = key.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None)
                        if (c.is_ascii_alphanumeric()
                            || matches!(c, '-' | '.' | ':' | '[' | ']'))
                            && self.net_ui.field.chars().count() < 80 =>
                    {
                        self.net_ui.field.push(c.to_ascii_uppercase());
                        self.net_ui.error = None;
                    }
                    _ => return false,
                }
            }
        }
        true
    }

    /// The network's marks on the field: the signal by the clock, a card
    /// while waiting for a player, a card when the match cannot go on.
    pub(crate) fn draw_net_marks(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let s = self.ui_scale();
        let view = session.net_view();
        let ending = session.ending().cloned();
        let w = self.canvas.width() as i32;
        let top = self.world_view().top;
        // Signal bars in the top bar where the pace control stands
        // offline, between the zoom buttons and MENU, with the ping.
        let x = w - 78 * s;
        signal_bars(&mut self.canvas, x, 5 * s, view.rtt_ms, s);
        if let Some(ms) = view.rtt_ms {
            small(
                &mut self.canvas,
                &format!("{ms}"),
                x + 14 * s,
                6 * s,
                MUTED,
                s,
            );
        }
        if let Some(ending) = ending {
            if self.world.outcome.is_some() {
                return;
            }
            let (title, line) = match ending {
                net::Ending::Desync { .. } => (
                    "OUT OF SYNC",
                    "THE GAMES NO LONGER AGREE. A REPORT WAS SAVED.",
                ),
                net::Ending::HostLeft => ("HOST LEFT", "THE MATCH CANNOT GO ON WITHOUT THE HOST."),
                net::Ending::Dropped => ("DROPPED", "THE HOST WAITED A MINUTE, THEN PLAYED ON."),
            };
            // Below the corner panels (the sluice card), in open water.
            let r = Rect {
                x: (w - 300 * s) / 2,
                y: top + 96 * s,
                w: 300 * s,
                h: 70 * s,
            };
            card(&mut self.canvas, r, RED);
            text(&mut self.canvas, title, r.x + 12 * s, r.y + 10 * s, RED, s);
            small(&mut self.canvas, line, r.x + 12 * s, r.y + 28 * s, WHITE, s);
            self.buttons.push(button(
                Rect {
                    x: r.x + r.w - 110 * s,
                    y: r.y + r.h - 24 * s,
                    w: 100 * s,
                    h: 18 * s,
                },
                "LEAVE",
                "",
                Action::Net(NetAction::EndMatch),
                true,
            ));
            return;
        }
        if let Some((seat, name, _, left)) = view.waiting {
            let r = self.draw_wait_banner(seat, &name, Some(left));
            if view.can_keep_open {
                // The host's choice: play on without them, their seat kept
                // for five minutes, instead of dropping them.
                let minutes = net::session::KEEP_OPEN.as_secs() / 60;
                self.buttons.push(button(
                    Rect {
                        x: r.x,
                        y: r.y + r.h + 2 * s,
                        w: r.w,
                        h: 18 * s,
                    },
                    "KEEP SEAT OPEN",
                    &format!("{minutes} MIN"),
                    Action::Net(NetAction::KeepSeatOpen(seat)),
                    true,
                ));
            }
        } else if let Some((seat, name, _)) = view.slow {
            // Connected but behind: named, with no drop clock.
            self.draw_wait_banner(seat, &name, None);
        } else if let Some((seat, name, left)) = view.away {
            self.draw_away_banner(seat, &name, left);
        }
    }

    /// A seat kept open while its player is away: "AWAY VIOLET COMPACT /
    /// SEAT OPEN  OUT 4:32". The same quiet card as the wait banner, since
    /// the match is not waiting: its machines hold where they stand.
    pub(crate) fn draw_away_banner(&mut self, match_seat: u8, lobby_name: &str, left: u64) {
        let clock = format!("OUT {}:{:02}", left / 60, left % 60);
        self.draw_seat_card(match_seat, lobby_name, ("AWAY", "SEAT OPEN"), Some(clock));
    }

    /// Who the match waits for, and until when: "WAITING FOR VIOLET
    /// COMPACT / TO RESPOND  DROP 0:52".  Trial 10's "⌛ PLAYER 2 0:52" named
    /// a lobby seat and looked like a hold count, a gold card with a timer,
    /// so the Compact read it as an enemy about to win.  The player is
    /// named by its colour and faction as on the field; the card is a quiet
    /// network card, dim signal bars and no gold; the drop clock is small
    /// and says what it counts to.  `drop_in` is None while a peer is only
    /// slow and no drop clock runs.
    pub(crate) fn draw_wait_banner(
        &mut self,
        match_seat: u8,
        lobby_name: &str,
        drop_in: Option<u64>,
    ) -> Rect {
        let clock = drop_in.map(|left| format!("DROP {}:{:02}", left / 60, left % 60));
        self.draw_seat_card(match_seat, lobby_name, ("WAITING FOR", "TO RESPOND"), clock)
    }

    /// The network card about one player: two short lines around its name,
    /// and a small clock. Returns where it was drawn.
    fn draw_seat_card(
        &mut self,
        match_seat: u8,
        lobby_name: &str,
        (above, below): (&str, &str),
        clock: Option<String>,
    ) -> Rect {
        let s = self.ui_scale();
        let w = self.canvas.width() as i32;
        let top = self.world_view().top;
        let (who, colour) = wait_banner_name(&self.world, match_seat, lobby_name);
        // Top right, clear of the sluice card under the gauge.
        let r = Rect {
            x: w - 232 * s,
            y: top + 12 * s,
            w: 220 * s,
            h: 30 * s,
        };
        card(&mut self.canvas, r, EDGE);
        // No signal from them: four dark bars.
        signal_bars(&mut self.canvas, r.x + 8 * s, r.y + 6 * s, None, s);
        let x = r.x + 26 * s;
        small(&mut self.canvas, above, x, r.y + 5 * s, MUTED, s);
        // The name follows the first line: "WAITING FOR" puts it 72 in.
        let name_x = x + (above.len() as i32 * 6 + 6) * s;
        let room = ((r.x + r.w - 8 * s - name_x) / (6 * s)).max(1) as usize;
        let rows = crate::native_ui::wrap_small(&who, room, 1);
        small(&mut self.canvas, &rows[0], name_x, r.y + 5 * s, colour, s);
        small(&mut self.canvas, below, x, r.y + 17 * s, MUTED, s);
        if let Some(clock) = clock {
            small(
                &mut self.canvas,
                &clock,
                r.x + r.w - 8 * s - clock.len() as i32 * 6 * s,
                r.y + 17 * s,
                WHITE,
                s,
            );
        }
        r
    }
}

/// The waited-for player's name and colour: its colour and faction as the
/// field names it ("VIOLET COMPACT"), or its lobby name when the world has
/// no such seat.
pub(crate) fn wait_banner_name(
    world: &bw_sim::World,
    match_seat: u8,
    lobby_name: &str,
) -> (String, Color) {
    if usize::from(match_seat) >= world.seat_count() {
        return (lobby_name.to_uppercase(), WHITE);
    }
    let seat = crate::seats::seat_of_match_seat(world, match_seat);
    (
        crate::seats::seat_title(world, seat),
        crate::seats::seat_colour(world, seat),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(name: &str, faction: Option<Faction>, ready: bool) -> SeatInfo {
        SeatInfo {
            name: name.into(),
            faction,
            ready,
            rtt_ms: Some(40),
        }
    }

    fn labels(buttons: &[Button]) -> Vec<String> {
        buttons.iter().map(|b| b.label.clone()).collect()
    }

    #[test]
    fn every_stage_draws_on_the_canvas_with_a_way_out() {
        let seats = vec![
            seat("MARIN", Some(Faction::Union), false),
            seat("ANNE", Some(Faction::Assembly), true),
            seat("BO", None, false),
        ];
        let stages = [
            Stage::Choose,
            Stage::Hosting,
            Stage::Reaching,
            Stage::Seated,
            Stage::Failed("Couldn't reach the host.".into()),
        ];
        for (stage, open) in stages.iter().flat_map(|s| [(s, false), (s, true)]) {
            let model = LobbyModel {
                stage: stage.clone(),
                editing_reply: open,
                editing_name: open,
                map: if open {
                    MapId::Confluence
                } else {
                    MapId::SplitBasin
                },
                code: Some("BW-1234-5678-9ABC-DEFG-HJKM-NPQR".into()),
                reply: Some("BW-ZZZZ-YYYY".into()),
                forwarded: Some(false),
                seats: seats.clone(),
                you: Some(1),
                seed: 4821,
                can_start: Ok(()),
                ..LobbyModel::default()
            };
            let mut canvas = Canvas::default();
            let buttons = lobby_screen(&mut canvas, None, &model);
            assert!(
                buttons
                    .iter()
                    .any(|b| matches!(b.action, Action::Back | Action::Net(NetAction::Leave))),
                "{stage:?} has a way out"
            );
            for b in &buttons {
                assert!(
                    b.x >= 0 && b.y >= 0 && b.x + b.w <= 640 && b.y + b.h <= 360,
                    "{stage:?}: {} off the screen",
                    b.label
                );
            }
            for (i, a) in buttons.iter().enumerate() {
                for b in &buttons[i + 1..] {
                    let overlap =
                        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
                    assert!(!overlap, "{stage:?}: {} overlaps {}", a.label, b.label);
                }
            }
        }
    }

    #[test]
    fn the_host_starts_and_a_guest_readies() {
        let mut model = LobbyModel {
            stage: Stage::Hosting,
            seats: vec![seat("A", Some(Faction::Union), false)],
            you: Some(0),
            can_start: Err("Waiting for another player.".into()),
            ..LobbyModel::default()
        };
        let mut canvas = Canvas::default();
        let host = lobby_screen(&mut canvas, None, &model);
        let start = host.iter().find(|b| b.label == "START").unwrap();
        assert!(!start.enabled, "nobody to play yet");
        assert!(labels(&host).contains(&"FRIEND CAN'T CONNECT?".to_string()));
        assert!(
            !host.iter().any(|b| b.label.contains("MAP ")),
            "the map goes by its name, not its seed"
        );
        model.stage = Stage::Seated;
        model.seats.push(seat("B", Some(Faction::Assembly), false));
        model.you = Some(1);
        let guest = lobby_screen(&mut canvas, None, &model);
        assert!(labels(&guest).contains(&"READY".to_string()));
        assert!(
            !labels(&guest).contains(&"START".to_string()),
            "only the host starts"
        );
        // The side toggle sits on your own row only, names your side and
        // steps to the next one.
        let toggles: Vec<_> = guest
            .iter()
            .filter(|b| matches!(b.action, Action::Net(NetAction::Side(_))))
            .collect();
        assert_eq!(toggles.len(), 1);
        assert_eq!(toggles[0].label, "ASSEMBLY");
        assert_eq!(
            toggles[0].action,
            Action::Net(NetAction::Side(Some(Faction::Compact)))
        );
        assert!((86..86 + 24).contains(&toggles[0].y), "on the second row");
    }

    #[test]
    fn a_seat_toggles_through_each_side_and_watching() {
        let mut side = Some(Faction::Union);
        let mut seen = vec![side];
        for _ in 0..4 {
            side = next_side(side);
            seen.push(side);
        }
        assert_eq!(
            seen,
            [
                Some(Faction::Union),
                Some(Faction::Assembly),
                Some(Faction::Compact),
                None,
                Some(Faction::Union)
            ]
        );
    }

    #[test]
    fn a_typed_name_is_kept_and_hosted_under() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = std::env::temp_dir().join(format!("bw-net-name-{}", std::process::id()));
        let mut game = Game::new_with_data_dir(base.clone(), dir.clone());
        game.open_online();
        game.net_action(NetAction::EditName);
        assert!(game.net_ui.editing_name);
        for _ in 0..12 {
            game.online_key("Backspace", false);
        }
        for key in ["a", "n", "n", "e", "1"] {
            assert!(game.online_key(key, false));
        }
        assert!(game.net_ui.field.is_empty(), "the code field is left alone");
        assert!(game.online_key("Enter", false));
        assert!(!game.net_ui.editing_name);
        assert_eq!(game.online_name(), "ANNE1");
        assert_eq!(game.online_options().name, "ANNE1");
        // Escape drops an edit.
        game.net_action(NetAction::EditName);
        game.online_key("x", false);
        game.online_key("Escape", false);
        assert_eq!(game.online_name(), "ANNE1");
        // The name is kept for the next session.
        let again = Game::new_with_data_dir(base, dir.clone());
        assert_eq!(again.online_name(), "ANNE1");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn typing_a_code_edits_the_field_and_enter_joins() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-net-ui-{}", std::process::id())),
        );
        game.open_online();
        for key in ["1", "2", "7", ".", "0", ".", "0", ".", "1", ":", "9"] {
            assert!(game.online_key(key, false));
        }
        assert!(game.online_key("Backspace", false));
        assert_eq!(game.net_ui.field, "127.0.0.1:");
        assert!(!game.online_key("F1", false), "other keys pass through");
        game.net_ui.field = "BW-NOPE".into();
        assert!(game.online_key("Enter", false));
        assert!(game.net_ui.error.is_some(), "a bad code says so");
        assert!(game.lobby.is_none());
    }

    /// Trial 10: "PLAYER 2 0:52" named a lobby seat. Every screen names
    /// the waited-for player by its colour and faction, as on the field.
    #[test]
    fn the_wait_banner_names_the_player_by_colour_and_faction() {
        let host = bw_sim::World::with_map(
            1,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("world");
        for local in 0..3u8 {
            let view = host.relabeled_for(local);
            let (name, colour) = wait_banner_name(&view, 2, "Player 3");
            assert_eq!(name, "VIOLET COMPACT", "seen from seat {local}");
            assert_eq!(colour, crate::canvas::VIOLET);
        }
        let basin = bw_sim::World::new(1, Faction::Union);
        assert_eq!(wait_banner_name(&basin, 1, "Guest").0, "SILT ASSEMBLY");
        assert_eq!(wait_banner_name(&basin, 5, "Guest").0, "GUEST");
    }

    #[test]
    fn the_wait_banner_is_a_quiet_network_card_not_a_hold_count() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-net-wait-{}", std::process::id())),
        );
        game.map = MapId::Confluence;
        game.start();
        game.resize_view(1280, 720);
        game.render();
        game.draw_wait_banner(1, "Player 2", Some(52));
        if let Ok(dir) = std::env::var("BW_THREE_SEAT_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            game.canvas
                .save(&std::path::PathBuf::from(dir).join("wait-banner.png"))
                .expect("frame");
        }
        let s = game.ui_scale();
        let w = game.canvas.width() as i32;
        let (x, y) = (w - 232 * s, game.world_view().top + 12 * s);
        // The frame is the quiet edge colour, never the hold banner's gold.
        assert_eq!(game.canvas.get(x, y), Some(EDGE));
        let (name, colour) = wait_banner_name(&game.world, 1, "Player 2");
        assert!(name.ends_with("ASSEMBLY") || name.ends_with("UNION") || name.ends_with("COMPACT"));
        let lit = (0..220 * s)
            .flat_map(|dx| (0..30 * s).map(move |dy| (dx, dy)))
            .any(|(dx, dy)| game.canvas.get(x + dx, y + dy) == Some(colour));
        assert!(lit, "the name is drawn in the player's colour");
    }

    /// The host's way to keep a silent player's seat open is a button on the
    /// waiting card; once pressed, the card says the seat is away and when
    /// it will be given up, and the button is gone.
    #[test]
    fn the_host_keeps_a_silent_seat_open_from_the_waiting_card() {
        let (host, guest) = crate::net::local_pair(9, Faction::Union, 3);
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-net-keep-{}", std::process::id())),
        );
        game.resize_view(1280, 720);
        game.start_network(host);
        // Past the matchup card, as a minute into a match.
        game.intro = None;
        guest.crash();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !game.session.as_ref().unwrap().net_view().can_keep_open {
            assert!(std::time::Instant::now() < deadline, "never offered");
            let _ = game.session.as_mut().unwrap().try_step();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        game.render();
        if let Ok(dir) = std::env::var("BW_THREE_SEAT_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            game.canvas
                .save(&std::path::PathBuf::from(dir).join("seat-waiting-keep.png"))
                .expect("frame");
        }
        let keep = game
            .buttons
            .iter()
            .find(|b| b.label == "KEEP SEAT OPEN")
            .expect("the host is offered to keep the seat")
            .clone();
        assert_eq!(keep.action, Action::Net(NetAction::KeepSeatOpen(1)));
        assert_eq!(keep.hint, "5 MIN");
        let s = game.ui_scale();
        let w = game.canvas.width() as i32;
        let top = game.world_view().top;
        assert_eq!(keep.x, w - 232 * s, "under the waiting card");
        assert!(keep.y >= top + 42 * s);
        game.net_action(NetAction::KeepSeatOpen(1));
        for _ in 0..10 {
            let _ = game.session.as_mut().unwrap().try_step();
        }
        let view = game.session.as_ref().unwrap().net_view();
        let (seat, _, left) = view.away.clone().expect("the seat is kept open");
        assert_eq!(seat, 1);
        assert!(left > 290, "{left}");
        assert!(view.waiting.is_none(), "the match no longer waits");
        game.render();
        if let Ok(dir) = std::env::var("BW_THREE_SEAT_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            game.canvas
                .save(&std::path::PathBuf::from(dir).join("seat-kept-open.png"))
                .expect("frame");
        }
        assert!(!game.buttons.iter().any(|b| b.label == "KEEP SEAT OPEN"));
        // The away card: the quiet network card, the player in its colour.
        let (x, y) = (w - 232 * s, top + 12 * s);
        assert_eq!(game.canvas.get(x, y), Some(EDGE));
        let (_, colour) = wait_banner_name(&game.world, 1, "Guest");
        let lit = (0..220 * s)
            .flat_map(|dx| (0..30 * s).map(move |dy| (dx, dy)))
            .any(|(dx, dy)| game.canvas.get(x + dx, y + dy) == Some(colour));
        assert!(lit, "the away player is named in its colour");
        game.end_network();
    }
}
