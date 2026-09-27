//! Agent harness: a headless game loop that serves frames and accepts
//! pointer and key events over a small local HTTP interface.
//!
//! A playing agent sees only pixels (`GET /frame`) and acts only through the
//! same pointer and keyboard entry points the window uses (`POST /input`), so
//! it plays the game the way a person does.  `GET /state` exists for referees
//! and match scripts, not for the players: `POST /input` answers with what it
//! did and the tick it did it on, and nothing a seat could play from, so the
//! blindness holds whatever a seat is asked to do.  No window or GPU is
//! needed, so two instances can run side by side on one machine without
//! fighting for focus or the mouse.

use crate::game::{Game, Screen};
use crate::tempo;
use image::ImageEncoder;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

struct Request {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    body: Vec<u8>,
    reply: Sender<Response>,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(value: serde_json::Value) -> Self {
        Response {
            status: 200,
            content_type: "application/json",
            body: value.to_string().into_bytes(),
        }
    }
    fn error(status: u16, text: &str) -> Self {
        Response {
            status,
            content_type: "application/json",
            body: serde_json::json!({ "error": text })
                .to_string()
                .into_bytes(),
        }
    }
}

/// Method, path, query and body of one request.
type Parsed = (String, String, BTreeMap<String, String>, Vec<u8>);

fn parse_request(stream: &mut TcpStream) -> Result<Parsed, String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header).map_err(|e| e.to_string())?;
        if n == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length.min(1 << 20)];
    if !body.is_empty() {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    let (path, query_text) = target.split_once('?').unwrap_or((&target, ""));
    let mut query = BTreeMap::new();
    for pair in query_text.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        query.insert(k.to_string(), v.to_string());
    }
    Ok((method, path.to_string(), query, body))
}

fn write_response(stream: &mut TcpStream, response: &Response) {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&response.body);
    let _ = stream.flush();
}

/// Start the HTTP listener; requests arrive on the returned channel and are
/// answered through their reply sender by the game loop.
fn serve(port: u16) -> Result<Receiver<Request>, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("agent port {port}: {e}"))?;
    let (tx, rx) = mpsc::channel::<Request>();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let tx = tx.clone();
            std::thread::spawn(move || {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let parsed = parse_request(&mut stream);
                let response = match parsed {
                    Ok((method, path, query, body)) => {
                        let (reply, wait) = mpsc::channel();
                        if tx
                            .send(Request {
                                method,
                                path,
                                query,
                                body,
                                reply,
                            })
                            .is_err()
                        {
                            Response::error(500, "game loop gone")
                        } else {
                            wait.recv_timeout(Duration::from_secs(30))
                                .unwrap_or_else(|_| Response::error(500, "no reply"))
                        }
                    }
                    Err(e) => Response::error(400, &e),
                };
                write_response(&mut stream, &response);
            });
        }
    });
    Ok(rx)
}

/// What a `/frame` request asks for beyond the whole picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FrameRequest {
    /// Keep one pixel in `scale` along each axis (1 to 4).
    scale: u32,
    /// A region of the full frame, `x0,y0,x1,y1` in full-frame pixels.
    crop: Option<(u32, u32, u32, u32)>,
    /// Enlarge the (cropped) picture by whole pixels (1 to 4), so small
    /// text on the panel or the chart reads without a cropper of one's own.
    zoom: u32,
    /// Draw the field without the corner controls (sluice card, guide tab,
    /// alerts) so the ground under them can be read.
    clean: bool,
    /// Draw the pointer the window would show, where it rests.
    pointer: bool,
}

impl FrameRequest {
    fn from_query(query: &BTreeMap<String, String>) -> Result<Self, String> {
        let number = |key: &str| query.get(key).and_then(|s| s.parse::<u32>().ok());
        let crop = match query.get("crop") {
            None => None,
            Some(text) => {
                let v: Vec<u32> = text
                    .split(',')
                    .map(|n| n.trim().parse::<u32>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| format!("crop {text:?}: expected X0,Y0,X1,Y1"))?;
                let [x0, y0, x1, y1] = v[..] else {
                    return Err(format!("crop {text:?}: expected X0,Y0,X1,Y1"));
                };
                Some((x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
            }
        };
        Ok(FrameRequest {
            scale: number("scale").unwrap_or(1).clamp(1, 4),
            crop,
            zoom: number("zoom").unwrap_or(1).clamp(1, 4),
            clean: query.get("clean").is_some_and(|v| v != "0" && v != "false"),
            pointer: query
                .get("pointer")
                .is_some_and(|v| v != "0" && v != "false"),
        })
    }
}

/// Paint the ground back over the field's corner controls: the frame as it
/// was drawn, with the world alone showing through where the sluice card,
/// the guide tab and the alerts sit.  The interface's own layout is
/// left as it was drawn, so clicks read off a clean frame land the same.
fn clean_field(game: &mut Game) {
    let view = game.world_view();
    let mut covered: Vec<crate::native_ui::Rect> = game
        .buttons
        .iter()
        .filter(|b| crate::game::overlay_action(&b.action))
        .map(|b| crate::native_ui::Rect {
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
        })
        .collect();
    covered.push(game.route_bounds());
    let framed = game.canvas.pixels.clone();
    game.draw_zoomed_world();
    let ground = std::mem::replace(&mut game.canvas.pixels, framed);
    let w = game.canvas.width() as i32;
    for r in covered {
        let (x0, x1) = (r.x.max(0), (r.x + r.w).min(w));
        let (y0, y1) = (r.y.max(view.top), (r.y + r.h).min(view.bottom));
        for y in y0..y1 {
            let from = ((y * w + x0) * 4) as usize;
            let to = ((y * w + x1) * 4) as usize;
            if from < to {
                game.canvas.pixels[from..to].copy_from_slice(&ground[from..to]);
            }
        }
    }
}

fn frame_png(game: &Game, request: FrameRequest) -> Result<Vec<u8>, String> {
    let (w, h) = (game.canvas.width(), game.canvas.height());
    let (x0, y0, x1, y1) = match request.crop {
        Some((x0, y0, x1, y1)) => (
            x0.min(w - 1),
            y0.min(h - 1),
            (x1 + 1).min(w),
            (y1 + 1).min(h),
        ),
        None => (0, 0, w, h),
    };
    let scale = request.scale.clamp(1, 4);
    let zoom = request.zoom.clamp(1, 4);
    let (sw, sh) = (((x1 - x0) / scale).max(1), ((y1 - y0) / scale).max(1));
    let (ow, oh) = (sw * zoom, sh * zoom);
    // RGB: the frame is opaque, and a quarter of every picture was alpha.
    let mut out = Vec::with_capacity((ow * oh * 3) as usize);
    for oy in 0..oh {
        let y = y0 + (oy / zoom) * scale;
        for ox in 0..ow {
            let x = x0 + (ox / zoom) * scale;
            let i = ((y * w + x) * 4) as usize;
            out.extend_from_slice(&game.canvas.pixels[i..i + 3]);
        }
    }
    let mut png = Vec::new();
    // Pixel art compresses well with a row filter at the fast level: a full
    // 1280x720 frame goes from 3.7 MB unfiltered to a few hundred KB, and
    // the lockstep, which waits while a picture is served, barely notices.
    image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Sub,
    )
    .write_image(&out, ow, oh, image::ExtendedColorType::Rgb8)
    .map_err(|e| e.to_string())?;
    Ok(png)
}

/// What this interface has seen of its seat and of the match, for the
/// referee's sampler: how often the seat looked and acted, and every death,
/// capture and switch with its tick.
#[derive(Default)]
struct Harness {
    /// Wall-clock instants of each picture the seat fetched and each action
    /// call it made; a referee's own pictures (`referee=1`) are not counted.
    frames: Vec<Instant>,
    inputs: Vec<Instant>,
    events: u64,
    /// The last world tick tallied, so a tick that did not advance is not
    /// read twice.
    tallied_tick: u64,
    log: Vec<serde_json::Value>,
    /// Whether this seat has fetched its first picture. In a session the
    /// clock waits for both seats' first look.
    looked: bool,
    /// Ticks that ran long, for `/state` and stderr.
    watchdog: crate::watchdog::Watchdog,
}

impl Harness {
    /// Record what the tick just stepped left in the world's events.
    fn tally(&mut self, game: &Game) {
        let world = &game.world;
        if world.tick == self.tallied_tick {
            return;
        }
        self.tallied_tick = world.tick;
        for event in &world.events {
            use bw_sim::EventKind as K;
            let side = |p: Option<u8>| match p {
                Some(0) => "own",
                Some(_) => "enemy",
                None => "none",
            };
            let entry = match event.kind {
                K::Death => serde_json::json!({
                    "tick": event.tick, "kind": "death", "side": side(event.player),
                    "unit": event.text, "cause": event.cause.map(|k| k.name()),
                    "cell": event.from.map(|p| p.cell_xy()),
                }),
                K::GateCaptured
                | K::GateCaptureStarted
                | K::GateChanged
                | K::SwitchCancelled
                | K::Victory
                | K::Draw => serde_json::json!({
                    "tick": event.tick, "kind": format!("{:?}", event.kind),
                    "side": side(event.player), "text": event.text,
                }),
                _ => continue,
            };
            self.log.push(entry);
        }
    }

    fn per_minute(stamps: &[Instant]) -> usize {
        stamps
            .iter()
            .rev()
            .take_while(|t| t.elapsed() < Duration::from_secs(60))
            .count()
    }

    fn json(&self, since: u64) -> serde_json::Value {
        serde_json::json!({
            "looked": self.looked,
            "frames": self.frames.len(),
            "frames_last_minute": Self::per_minute(&self.frames),
            "inputs": self.inputs.len(),
            "inputs_last_minute": Self::per_minute(&self.inputs),
            "events": self.events,
            "log": self.log.iter().filter(|e| e["tick"].as_u64().unwrap_or(0) > since).collect::<Vec<_>>(),
        })
    }
}

fn clock(tick: u64) -> String {
    let s = tick / 30;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Every entity with the fog lifted.  Served by an observer only: a seat
/// plays from the picture, and this would be a map hack.
fn entities_json(game: &Game) -> serde_json::Value {
    let world = &game.world;
    let mut by_side: Vec<BTreeMap<&str, u32>> = vec![BTreeMap::new(); world.seat_count().max(2)];
    let list: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.hp > 0)
        .map(|e| {
            if let Some(counts) = by_side.get_mut(e.owner as usize) {
                *counts.entry(e.kind.name()).or_default() += 1;
            }
            serde_json::json!({
                "id": e.id,
                "owner": e.owner,
                "faction": world.players.get(e.owner as usize).map(|p| format!("{:?}", p.faction)),
                "kind": e.kind.name(),
                "hp": e.hp,
                "cell": e.pos.cell_xy(),
                "order": format!("{:?}", e.order).split(['{', '(', ' ']).next().unwrap_or("").to_string(),
                "deployed": e.deployed,
                "aboard": e.aboard,
                "building": e.build_remaining > 0,
            })
        })
        .collect();
    serde_json::json!({
        "tick": world.tick,
        "clock": clock(world.tick),
        "factions": world.players.iter().map(|p| format!("{:?}", p.faction)).collect::<Vec<_>>(),
        "counts": by_side,
        "lane_hold": world.lane_hold,
        "holds": world.seats().map(|p| (0..crate::seats::arm_count(world)).map(|lane| world.holds_lane(p, lane)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "sluice_owner": world.gate.owner,
        "entities": list,
    })
}

/// The tide count as seat 0 reads it.  `lanes` are seat 0's own lanes
/// (both on the Split Basin, the two beside its land on the Confluence), in
/// arm order: whether you hold it, whether an opponent does, and which seat.
/// The enemy count is the leading opponent's gauge.
fn hold_json(world: &bw_sim::World) -> serde_json::Value {
    let enemy = world.leading_enemy_gauge().map_or(0, |(_, ticks)| ticks);
    serde_json::json!({
        "own_seconds_left": world.hold_ticks().saturating_sub(world.lane_hold[0]).div_ceil(30),
        "enemy_seconds_left": world.hold_ticks().saturating_sub(enemy).div_ceil(30),
        "hold_seconds": world.hold_ticks() / 30,
        "frozen": world.hold_frozen(),
        // Rules 22: an enemy nest at one of the counting seat's mouths
        // halves its count (three seats). Your count, and the leading
        // enemy's.
        "slowed": world.hold_slowed(0),
        "enemy_slowed": world.leading_enemy_gauge().is_some_and(|(seat, _)| world.hold_slowed(seat)),
        "enemy_seat": world.leading_enemy_gauge().map(|(seat, _)| seat),
        "lanes": world.arms_of(0).into_iter().map(|lane| {
            let holder = crate::qol::lane_holder_seat(world, lane);
            serde_json::json!({
                "arm": crate::seats::arm_letter(world, lane),
                "own": holder == Some(0),
                "enemy": holder.is_some_and(|seat| seat != 0),
                "holder": holder,
            })
        }).collect::<Vec<_>>(),
    })
}

/// The map and every seat: `map` ("SplitBasin" or "Confluence"), the dry
/// arm's letter (null while no arm is dry: the neutral tide or a flood),
/// and per seat its name, faction, hold gauge, whether it is out,
/// whether it holds every lane its count needs and whether an enemy nest
/// halves that count (rules 22).
fn seats_json(world: &bw_sim::World, state: &mut serde_json::Value) {
    state["map"] = serde_json::json!(format!("{:?}", world.map.id));
    state["dry_arm"] = serde_json::json!(
        (world.gate.tide == bw_sim::Tide::Open)
            .then(|| crate::seats::arm_letter(world, world.gate.dry_arm.index()))
    );
    // Rules 22: on the Confluence a DRY ebbs back to neutral; the seconds
    // until it does, null while none is due.
    state["dry_ebbs_in"] = serde_json::json!(
        world
            .gate
            .ebb_at
            .map(|at| at.saturating_sub(world.tick).div_ceil(30))
    );
    state["seats"] = world
        .seats()
        .map(|seat| {
            serde_json::json!({
                "seat": seat,
                "name": crate::seats::seat_name(world, seat),
                // The player's colour and faction, the same on every screen.
                "title": crate::seats::seat_title(world, seat),
                "faction": format!("{:?}", world.players[usize::from(seat)].faction),
                "lane_hold": world.lane_hold.get(usize::from(seat)).copied().unwrap_or(0),
                "eliminated": world.is_eliminated(seat),
                "holds_every_lane": world.holds_every_lane(seat),
                "hold_slowed": world.hold_slowed(seat),
            })
        })
        .collect::<Vec<_>>()
        .into();
}

/// The whole basin as a chart, the fog lifted: an observer's map view.
fn map_png(game: &Game, cell_px: u32) -> Result<Vec<u8>, String> {
    let map = &game.world.map;
    let k = cell_px.clamp(2, 8) as i32;
    let (w, h) = (i32::from(map.width) * k, i32::from(map.height) * k);
    let mut canvas = crate::canvas::Canvas::new(w as u32, h as u32);
    crate::minimap_chart::draw_field(
        &mut canvas,
        &game.world,
        crate::minimap_chart::ChartRect::axis(0, 0, w, h),
        (k / 2).max(1),
    );
    let mut out = Vec::with_capacity((w * h * 3) as usize);
    for px in canvas.pixels.chunks_exact(4) {
        out.extend_from_slice(&px[..3]);
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Sub,
    )
    .write_image(&out, w as u32, h as u32, image::ExtendedColorType::Rgb8)
    .map_err(|e| e.to_string())?;
    Ok(png)
}

#[cfg(test)]
pub(crate) fn state_for_tests(game: &Game) -> serde_json::Value {
    state_json(game)
}

fn state_json(game: &Game) -> serde_json::Value {
    let mut own_by_kind: BTreeMap<&str, u32> = BTreeMap::new();
    for e in game
        .world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && e.hp > 0)
    {
        *own_by_kind.entry(e.kind.name()).or_default() += 1;
    }
    let player = &game.world.players[0];
    let own = game
        .world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && e.hp > 0)
        .count();
    let enemies_seen = game
        .world
        .entities
        .iter()
        .filter(|e| e.owner != 0 && e.hp > 0 && game.world.entity_visible(0, e.id))
        .count();
    let mut state = serde_json::json!({
        "tick": game.world.tick,
        "seconds": game.world.tick / 30,
        "clock": clock(game.world.tick),
        "screen": format!("{:?}", game.screen),
        "pointer": game.pointer_kind((0, 0)).name(),
        "outcome": game.world.outcome.as_ref().map(|o| format!("{o:?}")),
        "faction": format!("{:?}", game.faction),
        "seat": game.session.as_ref().map_or(0, |s| s.local),
        "salvage": player.salvage,
        "pressure": player.pressure,
        "pressure_cap": player.pressure_cap,
        "crew": player.crew,
        "crew_cap": player.cap,
        "upgrades": player.upgrades.iter().map(|u| u.name()).collect::<Vec<_>>(),
        "doctrine": player.doctrine.map(|d| format!("{d:?}")),
        "doctrine_tier": player.doctrine_tier,
        "lane_hold": game.world.lane_hold,
        "tide": serde_json::json!({
            "mode": format!("{:?}", game.world.gate.tide),
            "north_dry": game.world.gate.north_dry(),
            "warning_seconds": game.world.gate.warning_until.map(|until| until.saturating_sub(game.world.tick).div_ceil(30)),
            "flood_seconds": game.world.gate.flood_until.filter(|_| game.world.gate.tide == bw_sim::Tide::Flood).map(|until| until.saturating_sub(game.world.tick).div_ceil(30)),
            "lock_seconds": game.world.gate.locked_until.saturating_sub(game.world.tick).div_ceil(30),
        }),
        "hold": hold_json(&game.world),
        "own_entities": own,
        "own_by_kind": own_by_kind,
        "enemies_visible": enemies_seen,
        "selected": game.selected.len(),
        "message": game.message,
        "prompt": game.native_prompt(),
        "alerts": game.ux.alerts.entries.iter().take(2).map(|a| a.label()).collect::<Vec<_>>(),
        "heard": game.tide_cues.recent().map(|h| format!("{} {}", clock(h.tick), h.text)).collect::<Vec<_>>(),
        "sluice": serde_json::json!({
            "owner": game.world.gate.owner,
            "capture_player": game.world.gate.capture_player,
            "capture_progress": game.world.gate.capture_progress,
            "capture_percent": game.world.gate.capture_progress * 100 / game.world.capture_work(),
            "contested": game.sluice_contested(),
        }),
        "session": game.session.as_ref().map(|s| s.status()),
        "session_seed": game.session.as_ref().map(|s| s.seed),
        "session_tick": game.session.as_ref().map(|s| s.tick()),
        // A stalled match and a slow one look the same from the tick alone:
        // `stalled` is a silent peer (the drop clock runs), `behind` (set
        // below) a connected peer whose orders are late (a busy game).
        "stalled": game.session.as_ref().is_some_and(|s| s.waiting_on_peer()),
        "peer_quiet_seconds": game.session.as_ref().map(|s| s.peer_quiet_seconds()),
        "desync_tick": game.session.as_ref().and_then(|s| s.desync),
        "observer": game.playback.as_ref().map(|p| serde_json::json!({"tick": p.tick(), "end_tick": p.end_tick(), "follow": p.follow, "camera_follows": game.spectator.follow, "can_seek": p.can_seek()})),
        "size": [game.canvas.width(), game.canvas.height()],
    });
    // The network as this seat sees it and its last state hashes, so two
    // seats can be compared tick for tick; the lobby, if one is open.
    seats_json(&game.world, &mut state);
    state["behind"] = serde_json::json!(
        game.session
            .as_ref()
            .is_some_and(|s| s.waiting_on_slow_peer())
    );
    state["net"] = net_json(game);
    state["lobby"] = lobby_json(game);
    state["online_error"] = serde_json::json!(game.net_ui.error);
    state
}

fn net_json(game: &Game) -> serde_json::Value {
    serde_json::json!(game.session.as_ref().map(|s| {
        let view = s.net_view();
        let history = s.hash_history();
        let last = history.last().cloned();
        serde_json::json!({
            "seat": s.local_seat,
            "host": s.is_host(),
            "rtt_ms": view.rtt_ms,
            "delay": view.delay,
            "watching": view.watching,
            "waiting_for": view.waiting.as_ref().map(|w| w.1.clone()),
            "drop_in_seconds": view.waiting.as_ref().map(|w| w.3),
            "behind": view.slow.as_ref().map(|w| w.1.clone()),
            "behind_seconds": view.slow.as_ref().map(|w| w.2),
            // A seat kept open while its player is away, and seconds before
            // it is dropped; whether this seat (the host) may keep the
            // waited-for seat open (the KEEP SEAT OPEN button).
            "away": view.away.as_ref().map(|w| w.1.clone()),
            "away_out_seconds": view.away.as_ref().map(|w| w.2),
            "can_keep_open": view.can_keep_open,
            "ending": s.ending().map(|e| format!("{e:?}")),
            "hash_tick": last.as_ref().map(|h| h.0),
            "hash": last.as_ref().map(|h| h.1.clone()),
            // The last few, for comparing seats sampled a moment apart.
            "hashes": history.iter().rev().take(8).map(|h| (h.0, h.1.clone())).collect::<Vec<_>>(),
        })
    }))
}

fn lobby_json(game: &Game) -> serde_json::Value {
    serde_json::json!(game.lobby.as_ref().map(|l| serde_json::json!({
        "phase": format!("{:?}", l.phase),
        "code": l.code(),
        "reply": l.reply_code(),
        "you": l.you,
        "seed": l.seed,
        "seats": l.seats.iter().map(|s| serde_json::json!({
            "name": s.name,
            "faction": s.faction.map(|f| format!("{f:?}")),
            "ready": s.ready,
            "rtt_ms": s.rtt_ms,
        })).collect::<Vec<_>>(),
        "can_start": l.can_start().err(),
    })))
}

/// The key name the game expects, with any modifier prefix split off.
fn normalise_key(raw: &str, mut shift: bool, mut control: bool) -> (String, bool, bool) {
    let mut key = raw.trim().to_string();
    loop {
        let lower = key.to_ascii_lowercase();
        let prefix = ["ctrl+", "control+", "shift+"]
            .into_iter()
            .find(|p| lower.starts_with(p) && key.len() > p.len());
        match prefix {
            Some(p) => {
                if p == "shift+" {
                    shift = true;
                } else {
                    control = true;
                }
                key = key[p.len()..].to_string();
            }
            None => break,
        }
    }
    let named = [
        "Space",
        "Escape",
        "Enter",
        "Tab",
        "Home",
        "End",
        "Backspace",
        "Delete",
        "Up",
        "Down",
        "Left",
        "Right",
        "PageUp",
        "PageDown",
    ];
    let lower = key.to_ascii_lowercase();
    let function_key = lower.starts_with('f') && lower[1..].chars().all(|c| c.is_ascii_digit());
    let key = if let Some(name) = named.iter().find(|n| n.to_ascii_lowercase() == lower) {
        (*name).to_string()
    } else if key.chars().count() == 1 || function_key {
        key.to_ascii_uppercase()
    } else {
        key
    };
    (key, shift, control)
}

/// Refuse an event the game would misread: an unknown type, a key the type
/// does not take (trial 10: `"right": true` was applied as a left click), or
/// a value of the wrong kind. A refused event is not applied at all.
fn check_event(event: &serde_json::Value) -> Result<(), String> {
    let Some(fields) = event.as_object() else {
        return Err(format!("an event is a JSON object, not {event}"));
    };
    let kind = match fields.get("t") {
        Some(serde_json::Value::String(kind)) => kind.as_str(),
        Some(other) => return Err(format!("\"t\" must be a string, not {other}")),
        None => return Err("an event needs \"t\" (move, click, dblclick, drag, key, pan)".into()),
    };
    let allowed: &[&str] = match kind {
        "move" => &["x", "y"],
        "click" | "dblclick" => &["x", "y", "button", "shift"],
        "drag" => &["x0", "y0", "x1", "y1", "shift"],
        "key" => &["key", "shift", "control", "ctrl"],
        "pan" => &["dx", "dy"],
        other => return Err(format!("unknown event type {other:?}")),
    };
    for (name, value) in fields {
        if name == "t" {
            continue;
        }
        if !allowed.contains(&name.as_str()) {
            let hint = if name == "right" {
                " (a right click is \"button\": \"right\")"
            } else {
                ""
            };
            return Err(format!(
                "unknown key {name:?} for a {kind} event{hint}; it takes {}",
                allowed.join(", ")
            ));
        }
        let ok = match name.as_str() {
            "shift" | "control" | "ctrl" => value.is_boolean(),
            "button" => matches!(value.as_str(), Some("left" | "right")),
            "key" => value.is_string(),
            _ => value.is_i64(),
        };
        if !ok {
            let want = match name.as_str() {
                "shift" | "control" | "ctrl" => "true or false",
                "button" => "\"left\" or \"right\"",
                "key" => "a key name",
                _ => "a whole number",
            };
            return Err(format!(
                "{name:?} must be {want}, not {value} ({kind} event)"
            ));
        }
    }
    Ok(())
}

/// Apply one JSON input event through the same entry points the window uses,
/// and say what was applied: the event as the game took it, with the key
/// name it matched and the modifiers that reached it.
fn apply_event(game: &mut Game, event: &serde_json::Value) -> Result<serde_json::Value, String> {
    check_event(event)?;
    let kind = event.get("t").and_then(|v| v.as_str()).unwrap_or("");
    let num = |key: &str| event.get(key).and_then(|v| v.as_i64()).map(|v| v as i32);
    let flag = |key: &str| event.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
    let shift = flag("shift");
    let inside = |x: i32, y: i32| {
        x >= 0 && y >= 0 && x < game.canvas.width() as i32 && y < game.canvas.height() as i32
    };
    match kind {
        "move" => {
            let (x, y) = (num("x").ok_or("x")?, num("y").ok_or("y")?);
            if inside(x, y) {
                game.pointer_moved(x, y);
            } else {
                game.pointer_left();
            }
            Ok(serde_json::json!({"t": "move", "x": x, "y": y}))
        }
        "click" | "dblclick" => {
            let (x, y) = (num("x").ok_or("x")?, num("y").ok_or("y")?);
            if !inside(x, y) {
                return Err("click outside the frame".into());
            }
            let right = event.get("button").and_then(|v| v.as_str()) == Some("right");
            let times = if kind == "dblclick" { 2 } else { 1 };
            for _ in 0..times {
                game.pointer_moved(x, y);
                if right {
                    game.right_click(x, y, shift);
                } else {
                    game.ux.pointer_shift = shift;
                    game.left_down(x, y);
                    game.left_up(x, y, shift);
                }
                game.render();
            }
            Ok(serde_json::json!({
                "t": kind, "x": x, "y": y,
                "button": if right { "right" } else { "left" }, "shift": shift,
            }))
        }
        "drag" => {
            let (x0, y0) = (num("x0").ok_or("x0")?, num("y0").ok_or("y0")?);
            let (x1, y1) = (num("x1").ok_or("x1")?, num("y1").ok_or("y1")?);
            if !inside(x0, y0) || !inside(x1, y1) {
                return Err("drag outside the frame".into());
            }
            game.pointer_moved(x0, y0);
            game.ux.pointer_shift = shift;
            game.left_down(x0, y0);
            for step in 1..=4 {
                game.pointer_moved(x0 + (x1 - x0) * step / 4, y0 + (y1 - y0) * step / 4);
                game.render();
            }
            game.left_up(x1, y1, shift);
            Ok(
                serde_json::json!({"t": "drag", "x0": x0, "y0": y0, "x1": x1, "y1": y1, "shift": shift}),
            )
        }
        "key" => {
            let key = event.get("key").and_then(|v| v.as_str()).ok_or("key")?;
            // A chord may be spelled in the key: "Ctrl+1", "Shift+I".
            // Names are case-insensitive: "SPACE" is Space, "g" is G.
            let (key, shift, control) = normalise_key(key, shift, flag("control") || flag("ctrl"));
            if key == "F11" || key == "F12" {
                return Err(format!("{key} is a window key and does nothing here"));
            }
            game.key(&key, shift, control);
            Ok(serde_json::json!({"t": "key", "key": key, "shift": shift, "control": control}))
        }
        "pan" => {
            let (dx, dy) = (num("dx").unwrap_or(0), num("dy").unwrap_or(0));
            game.pan(dx, dy);
            Ok(serde_json::json!({"t": "pan", "dx": dx, "dy": dy}))
        }
        other => Err(format!("unknown event type {other:?}")),
    }
}

fn png(body: Vec<u8>) -> Response {
    Response {
        status: 200,
        content_type: "image/png",
        body,
    }
}

fn handle(game: &mut Game, harness: &mut Harness, request: Request) -> bool {
    let observer = game.playback.is_some();
    let response = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/frame") => match FrameRequest::from_query(&request.query) {
            Ok(frame) => {
                game.render();
                if frame.clean && game.screen == Screen::Match && game.world.outcome.is_none() {
                    clean_field(game);
                }
                if frame.pointer {
                    let kind = game.pointer_kind((0, 0));
                    let scale = game.pointer_scale();
                    let (x, y) = game.cursor;
                    crate::pointer::draw(&mut game.canvas, kind, x, y, scale);
                }
                let result = frame_png(game, frame);
                if frame.clean || frame.pointer {
                    game.render();
                }
                // A referee's pictures are not the seat looking.
                if !request.query.contains_key("referee") {
                    // A click read off this picture finds a machine that
                    // walks on before it lands (trial 11).
                    game.note_seen();
                    harness.frames.push(Instant::now());
                    harness.looked = true;
                }
                match result {
                    Ok(body) => png(body),
                    Err(e) => Response::error(500, &e),
                }
            }
            Err(e) => Response::error(400, &e),
        },
        ("GET", "/state") => {
            let since = request
                .query
                .get("since")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let mut state = state_json(game);
            state["harness"] = harness.json(since);
            // Ticks over the watchdog's limit: a crawling match says so.
            state["watchdog"] = harness.watchdog.json();
            Response::json(state)
        }
        // Every control on screen by label and place: for test scripts that
        // drive the menus, not for seats, which play from the picture.
        ("GET", "/buttons") => {
            game.render();
            Response::json(serde_json::json!(
                game.buttons
                    .iter()
                    .map(|b| serde_json::json!({
                        "label": b.label,
                        "hint": b.hint,
                        "action": format!("{:?}", b.action),
                        "x": b.x, "y": b.y, "w": b.w, "h": b.h,
                        "enabled": b.enabled,
                    }))
                    .collect::<Vec<_>>()
            ))
        }
        ("GET", "/entities") if observer => Response::json(entities_json(game)),
        ("GET", "/map") if observer => {
            let cell = request
                .query
                .get("cell")
                .and_then(|s| s.parse().ok())
                .unwrap_or(4);
            match map_png(game, cell) {
                Ok(body) => png(body),
                Err(e) => Response::error(500, &e),
            }
        }
        ("GET", "/entities" | "/map") => {
            Response::error(403, "only an observer (--replay) serves the whole map")
        }
        ("POST", "/input") => match serde_json::from_slice::<serde_json::Value>(&request.body) {
            Ok(serde_json::Value::Array(events)) => {
                harness.inputs.push(Instant::now());
                // The interface lays out its controls during render, so a
                // click sees the same layout a person would.
                game.render();
                let mut applied = Vec::new();
                let mut errors = Vec::new();
                // Every event is read off the same picture: a group key's
                // recentre waits until the batch is applied (trial 12).
                game.begin_input_batch();
                for event in &events {
                    match apply_event(game, event) {
                        Ok(done) => applied.push(done),
                        Err(e) => errors.push(e),
                    }
                }
                game.end_input_batch();
                harness.events += applied.len() as u64;
                // An agent sees the view it asked for, not a zoom mid-ease.
                game.settle_zoom();
                game.render();
                // Confirmation, not a state feed: what was applied, with the
                // modifiers that reached the game, the clock and the prompt
                // row, all of which the picture shows too.  Referees and
                // match scripts read `/state`.
                Response::json(serde_json::json!({
                    "applied": applied.len(),
                    "as_applied": applied,
                    "errors": errors,
                    "tick": game.world.tick,
                    "clock": clock(game.world.tick),
                    "prompt": game.native_prompt(),
                    // A seat has no speakers: the tide cues of the last ten
                    // seconds, as a player with sound would have heard them.
                    "heard": game.tide_cues.recent().map(|h| format!("{} {}", clock(h.tick), h.text)).collect::<Vec<_>>(),
                }))
            }
            Ok(_) => Response::error(400, "expected a JSON array of events"),
            Err(e) => Response::error(400, &e.to_string()),
        },
        ("POST", "/quit") => {
            let _ = request
                .reply
                .send(Response::json(serde_json::json!({ "quit": true })));
            return true;
        }
        _ => Response::error(404, "unknown endpoint"),
    };
    let _ = request.reply.send(response);
    false
}

/// Close the session and keep the match: a trial that is stopped rather than
/// played out still leaves `last-match.replay.json` to watch, which is the
/// file the harness points at.
fn finish(game: &mut Game) {
    if let Some(session) = &mut game.session {
        session.close();
    }
    game.forget_rejoin();
    if game.world.tick > 0
        && let Err(e) = game.export_final_replay()
    {
        eprintln!("Replay save: {e}");
    }
}

/// Answer every request waiting on the agent port; true when one says quit.
fn serve_pending(
    game: &mut Game,
    harness: &mut Harness,
    requests: Option<&Receiver<Request>>,
) -> bool {
    let Some(rx) = requests else { return false };
    while let Ok(request) = rx.try_recv() {
        if handle(game, harness, request) {
            return true;
        }
    }
    false
}

/// Run the game without a window: fixed-rate ticks from wall time, frames on
/// demand, and inputs from the agent interface.  Returns when `/quit` is
/// posted or the optional time limit passes.
pub fn run_headless(
    mut game: Game,
    size: (u32, u32),
    port: Option<u16>,
    quit_after: Option<Duration>,
) -> Result<(), String> {
    game.zoom = crate::zoom::Zoom::from_preference(game.ux.preferences.world_zoom);
    game.resize_view(size.0, size.1);
    let requests = match port {
        Some(port) => Some(serve(port)?),
        None => None,
    };
    let started = Instant::now();
    let mut harness = Harness {
        watchdog: crate::watchdog::Watchdog::from_env(),
        ..Harness::default()
    };
    // A seat's clock waits for its first picture, and the lockstep holds the
    // other seat with it: the match starts when both seats have looked, not
    // while one is still reading its instructions.
    let waits_for_first_look = requests.is_some() && game.session.is_some();
    if waits_for_first_look {
        game.notify("THE CLOCK STARTS WHEN EVERY SEAT HAS LOOKED");
    }
    let mut last = Instant::now();
    let mut accumulator = 0.0;
    let mut last_screen = game.screen;
    game.render();
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last).as_secs_f64();
        last = now;
        game.frame_update();
        // Once the clock runs the note that it waits is wrong: drop it
        // rather than let it stand for its full five seconds.
        if waits_for_first_look
            && game.world.tick > 0
            && game.message == "THE CLOCK STARTS WHEN EVERY SEAT HAS LOOKED"
        {
            game.message.clear();
        }
        if game.screen != last_screen {
            tempo::reset_accumulator(&mut accumulator);
            last_screen = game.screen;
        }
        // A session keeps stepping while this seat reads the Guide or opens
        // the menu: a pause by one seat stalls both, and the other seat is
        // not the one reading.
        let stepping = (game.screen == Screen::Match || game.session_steps_on())
            && (harness.looked || !waits_for_first_look);
        if stepping {
            let due = tempo::schedule_ticks(&mut accumulator, elapsed, tempo::GameSpeed::Normal);
            // An observer that has fallen behind a live match catches up.
            let pace = game.playback.as_ref().map_or(1, |p| p.catch_up_factor());
            let mut burst = Instant::now();
            for _ in 0..due * pace {
                harness.watchdog.tick(&mut game);
                harness.tally(&game);
                // A crawling match (trial 10: seconds a tick) still answers
                // the agent port between ticks, not after the whole burst.
                if burst.elapsed() >= crate::watchdog::BURST_BUDGET {
                    if serve_pending(&mut game, &mut harness, requests.as_ref()) {
                        finish(&mut game);
                        return Ok(());
                    }
                    burst = Instant::now();
                }
            }
        } else {
            tempo::reset_accumulator(&mut accumulator);
        }
        if serve_pending(&mut game, &mut harness, requests.as_ref()) {
            finish(&mut game);
            return Ok(());
        }
        if quit_after.is_some_and(|limit| started.elapsed() >= limit) {
            finish(&mut game);
            println!(
                "Headless session ended at simulation tick {}",
                game.world.tick
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_parse_paths_queries_and_bodies() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            let mut s = TcpStream::connect(addr).unwrap();
            s.write_all(
                b"POST /input?scale=2&x=1 HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\nhello",
            )
            .unwrap();
            s
        });
        let (mut server, _) = listener.accept().unwrap();
        let (method, path, query, body) = parse_request(&mut server).unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/input");
        assert_eq!(query.get("scale").map(String::as_str), Some("2"));
        assert_eq!(body, b"hello");
        drop(client.join().unwrap());
    }

    #[test]
    fn frames_encode_and_inputs_reach_the_game() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("brinewake-agent-{}", std::process::id())),
        );
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.render();
        let png = frame_png(
            &game,
            FrameRequest {
                scale: 2,
                zoom: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        let events: serde_json::Value = serde_json::json!([
            {"t": "move", "x": 640, "y": 360},
            {"t": "key", "key": "Space"},
        ]);
        for event in events.as_array().unwrap() {
            apply_event(&mut game, event).unwrap();
        }
        assert!(!game.selected.is_empty(), "Space selects the headquarters");
        let state = state_json(&game);
        assert_eq!(state["seat"], 0);
        assert!(state["own_entities"].as_u64().unwrap() > 0);
    }

    fn game_for_test(tag: &str) -> Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("brinewake-{tag}-{}", std::process::id())),
        )
    }

    #[test]
    fn a_session_keeps_stepping_while_a_seat_reads_the_menu() {
        let (host, guest) = crate::net::local_pair(5, bw_core::Faction::Union, 3);
        let mut a = game_for_test("pause-host");
        let mut b = game_for_test("pause-guest");
        a.resize_view(1280, 720);
        b.resize_view(1280, 720);
        a.start_network(host);
        b.start_network(guest);
        // One seat opens the menu. The other is still playing, so the match
        // must go on; before this it stalled both seats with no way out.
        a.screen = Screen::Pause;
        assert!(a.session_steps_on());
        for _ in 0..2000 {
            a.tick();
            b.tick();
            if a.world.tick > 50 && b.world.tick > 50 {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            a.world.tick > 50 && b.world.tick > 50,
            "both seats advanced: {} and {}",
            a.world.tick,
            b.world.tick
        );
        a.session.as_mut().expect("session").close();
        b.session.as_mut().expect("session").close();
    }

    #[test]
    fn a_lone_match_still_pauses() {
        let mut game = game_for_test("lone-pause");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.screen = Screen::Pause;
        assert!(!game.session_steps_on());
        let held = game.world.tick;
        for _ in 0..30 {
            game.tick();
        }
        assert_eq!(game.world.tick, held, "one player's pause still pauses");
    }

    #[test]
    fn an_action_is_confirmed_without_handing_back_the_match() {
        let mut game = game_for_test("input-reply");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.render();
        let (reply, wait) = mpsc::channel();
        let request = Request {
            method: "POST".into(),
            path: "/input".into(),
            query: BTreeMap::new(),
            body: serde_json::json!([{"t": "key", "key": "Space"}])
                .to_string()
                .into_bytes(),
            reply,
        };
        assert!(!handle(&mut game, &mut Harness::default(), request));
        let response = wait.recv().expect("a reply");
        let body: serde_json::Value = serde_json::from_slice(&response.body).expect("json");
        assert_eq!(body["applied"], 1);
        assert!(body.get("tick").is_some(), "the tick confirms the action");
        // A seat plays from the picture: the reply carries nothing to play
        // from. Referees read /state.
        for leaked in [
            "state",
            "salvage",
            "own_entities",
            "enemies_visible",
            "sluice",
        ] {
            assert!(
                body.get(leaked).is_none(),
                "the input reply does not carry {leaked}"
            );
        }
    }

    #[test]
    fn the_live_recording_is_written_at_the_result() {
        let mut game = game_for_test("live-result");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        for _ in 0..40 {
            game.tick();
        }
        game.world
            .issue(0, bw_sim::Command::Surrender)
            .expect("surrender is accepted");
        game.tick();
        assert!(game.world.outcome.is_some());
        let path = game.data_dir.join("saves").join("live-match.replay.json");
        // The recorder writes on its own thread.
        let deadline = Instant::now() + Duration::from_secs(10);
        let finished = loop {
            if let Ok(mut player) = bw_sim::ReplayPlayer::open(&path) {
                while player.step().expect("replays") {}
                if player.world().outcome.is_some() {
                    break true;
                }
            }
            if Instant::now() > deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(finished, "a live observer sees how the match ended");
    }

    #[test]
    fn frames_crop_zoom_and_clean() {
        let mut game = game_for_test("frame-options");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.render();
        let query: BTreeMap<String, String> = [("crop", "100,200,199,249"), ("zoom", "2")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let request = FrameRequest::from_query(&query).expect("parses");
        let png = frame_png(&game, request).expect("encodes");
        let size = (
            u32::from_be_bytes(png[16..20].try_into().unwrap()),
            u32::from_be_bytes(png[20..24].try_into().unwrap()),
        );
        assert_eq!(size, (200, 100), "a 100x50 region at 2X");
        let bad: BTreeMap<String, String> = [("crop".to_string(), "1,2,3".to_string())].into();
        assert!(FrameRequest::from_query(&bad).is_err());
        let whole = frame_png(
            &game,
            FrameRequest {
                scale: 1,
                zoom: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            whole.len() < 1_500_000,
            "a full frame is compressed: {} bytes",
            whole.len()
        );
        // A clean frame leaves the controls where they were for clicking.
        let before: Vec<_> = game.buttons.iter().map(|b| (b.x, b.y, b.w, b.h)).collect();
        clean_field(&mut game);
        let after: Vec<_> = game.buttons.iter().map(|b| (b.x, b.y, b.w, b.h)).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn a_seat_does_not_serve_the_whole_map() {
        let mut game = game_for_test("no-map-hack");
        game.resize_view(1280, 720);
        game.start();
        for path in ["/entities", "/map"] {
            let (reply, wait) = mpsc::channel();
            let request = Request {
                method: "GET".into(),
                path: path.into(),
                query: BTreeMap::new(),
                body: Vec::new(),
                reply,
            };
            handle(&mut game, &mut Harness::default(), request);
            assert_eq!(wait.recv().unwrap().status, 403, "{path} is for observers");
        }
    }

    #[test]
    fn an_event_with_an_unknown_key_is_refused_and_not_applied() {
        // Trial 10: {"right": true} was taken as a left click, and the reply
        // listed no errors.
        let mut game = game_for_test("unknown-key");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.render();
        let (reply, wait) = mpsc::channel();
        let request = Request {
            method: "POST".into(),
            path: "/input".into(),
            query: BTreeMap::new(),
            body: serde_json::json!([
                {"t": "click", "x": 640, "y": 360, "right": true},
                {"t": "key", "key": "Space"},
            ])
            .to_string()
            .into_bytes(),
            reply,
        };
        assert!(!handle(&mut game, &mut Harness::default(), request));
        let body: serde_json::Value =
            serde_json::from_slice(&wait.recv().expect("a reply").body).expect("json");
        assert_eq!(body["applied"], 1, "only the good event: {body}");
        let errors = body["errors"].as_array().expect("errors");
        assert_eq!(errors.len(), 1, "{body}");
        let error = errors[0].as_str().unwrap();
        assert!(error.contains("\"right\""), "{error}");
        assert!(error.contains("button"), "the error says how: {error}");
        for bad in [
            serde_json::json!({"t": "click", "x": 1, "y": 1, "button": "middle"}),
            serde_json::json!({"t": "click", "x": 1, "y": 1, "shift": "yes"}),
            serde_json::json!({"t": "key", "key": "G", "alt": true}),
            serde_json::json!({"t": "move", "x": 1.5, "y": 1}),
            serde_json::json!({"t": "jump"}),
            serde_json::json!({"x": 1, "y": 1}),
            serde_json::json!("click"),
        ] {
            assert!(apply_event(&mut game, &bad).is_err(), "refused: {bad}");
        }
        // What the harness and its scripts send is still taken.
        for good in [
            serde_json::json!({"t": "click", "x": 640, "y": 360, "button": "right", "shift": false}),
            serde_json::json!({"t": "dblclick", "x": 640, "y": 360, "button": "left", "shift": true}),
            serde_json::json!({"t": "drag", "x0": 600, "y0": 300, "x1": 700, "y1": 400, "shift": false}),
            serde_json::json!({"t": "key", "key": "Escape", "shift": false, "control": false}),
            serde_json::json!({"t": "key", "key": "Escape", "ctrl": false}),
            serde_json::json!({"t": "pan", "dx": 4, "dy": -4}),
            serde_json::json!({"t": "move", "x": 10, "y": 10}),
        ] {
            assert!(apply_event(&mut game, &good).is_ok(), "taken: {good}");
        }
    }

    #[test]
    fn an_action_reply_says_which_modifiers_reached_the_game() {
        let mut game = game_for_test("modifiers");
        game.resize_view(1280, 720);
        game.start();
        game.world.ai_enabled = false;
        game.render();
        let done = apply_event(
            &mut game,
            &serde_json::json!({"t": "key", "key": "Shift+q"}),
        )
        .unwrap();
        assert_eq!(done["key"], "Q");
        assert_eq!(done["shift"], true);
        assert_eq!(done["control"], false);
    }

    #[test]
    fn the_sampler_log_carries_deaths_once_with_their_cause() {
        let mut game = game_for_test("tally");
        game.start();
        game.world.tick = 90;
        game.world.events.push(bw_sim::Event {
            tick: 90,
            kind: bw_sim::EventKind::Death,
            player: Some(1),
            entity: Some(7),
            other: None,
            from: Some(bw_core::Pos::cell(40, 50)),
            to: None,
            amount: 0,
            text: "LOOM".into(),
            cause: Some(bw_core::Kind::Bulwark),
        });
        let mut harness = Harness::default();
        harness.tally(&game);
        harness.tally(&game);
        let json = harness.json(0);
        let log = json["log"].as_array().unwrap();
        assert_eq!(
            log.len(),
            1,
            "a tick that did not advance is not read twice"
        );
        assert_eq!(log[0]["side"], "enemy");
        assert_eq!(log[0]["unit"], "LOOM");
        assert_eq!(log[0]["cause"], bw_core::Kind::Bulwark.name());
        assert!(harness.json(90)["log"].as_array().unwrap().is_empty());
    }

    #[test]
    fn key_names_accept_chords_and_any_case() {
        assert_eq!(
            normalise_key("Ctrl+1", false, false),
            ("1".into(), false, true)
        );
        assert_eq!(
            normalise_key("shift+i", false, false),
            ("I".into(), true, false)
        );
        assert_eq!(
            normalise_key("SPACE", false, false),
            ("Space".into(), false, false)
        );
        assert_eq!(normalise_key("g", false, false), ("G".into(), false, false));
        assert_eq!(
            normalise_key("f1", false, false),
            ("F1".into(), false, false)
        );
        assert_eq!(
            normalise_key("Escape", true, false),
            ("Escape".into(), true, false)
        );
    }

    #[test]
    fn state_names_the_map_every_seat_and_your_own_two_lanes() {
        let mut game = game_for_test("three-seats");
        game.map = bw_sim::MapId::Confluence;
        game.start();
        game.world.ai_enabled = false;
        // Violet owns the station and holds its lanes; its count leads.
        game.world.gate.owner = Some(2);
        for arm in game.world.arms_of(2) {
            let mouth = game.world.own_mouth(2, arm).expect("mouth");
            game.world.spawn_for_tests(2, bw_core::Kind::Bulwark, mouth);
        }
        game.world.lane_hold[2] = 30 * 30;
        let state = state_json(&game);
        assert_eq!(state["map"], "Confluence");
        // The neutral tide dries no arm (trial 10 read "E" here).
        assert!(state["dry_arm"].is_null(), "{}", state["dry_arm"]);
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm(1);
        assert_eq!(state_json(&game)["dry_arm"], "W");
        game.world.gate.tide = bw_sim::Tide::Flood;
        assert!(state_json(&game)["dry_arm"].is_null());
        game.world.gate.tide = bw_sim::Tide::Neutral;
        let seats = state["seats"].as_array().expect("seats");
        assert_eq!(seats.len(), 3);
        // Union, Assembly, Assembly: the second Assembly wears pink, never
        // the jade of its own machines, on every screen.
        assert_eq!(seats[2]["name"], "PINK");
        assert_eq!(seats[2]["title"], "PINK ASSEMBLY");
        assert_eq!(seats[2]["holds_every_lane"], true);
        assert_eq!(seats[2]["lane_hold"], 900);
        assert_eq!(seats[1]["eliminated"], false);
        let hold = &state["hold"];
        // Three seats hold for 75 s (rules 20): 45 s left after 30.
        assert_eq!(hold["enemy_seconds_left"], 45);
        assert_eq!(hold["hold_seconds"], 75);
        assert_eq!(hold["frozen"], false);
        assert_eq!(hold["enemy_seat"], 2);
        // Your lanes are the east and the west; violet holds the west.
        let lanes = hold["lanes"].as_array().expect("lanes");
        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0]["arm"], "E");
        assert_eq!(lanes[1]["arm"], "W");
        assert_eq!(lanes[1]["holder"], 2);
        assert_eq!(lanes[1]["enemy"], true);
        assert_eq!(lanes[0]["enemy"], false);
        // The observer's counts keep a row for the third seat.
        let entities = entities_json(&game);
        assert_eq!(entities["counts"].as_array().expect("counts").len(), 3);
        assert_eq!(entities["holds"][2].as_array().expect("holds").len(), 3);
    }

    #[test]
    fn state_keeps_the_split_basin_keys() {
        let mut game = game_for_test("two-seats");
        game.start();
        let state = state_json(&game);
        assert_eq!(state["map"], "SplitBasin");
        assert!(state["dry_arm"].is_null());
        game.world.gate.tide = bw_sim::Tide::Open;
        assert_eq!(state_json(&game)["dry_arm"], "N");
        assert_eq!(state["seats"][1]["name"], "ENEMY");
        assert_eq!(state["hold"]["lanes"].as_array().expect("lanes").len(), 2);
        assert!(state["hold"]["own_seconds_left"].is_u64());
        assert!(state["tide"]["north_dry"].is_boolean());
        assert_eq!(state["lane_hold"].as_array().expect("gauges").len(), 2);
    }
}
