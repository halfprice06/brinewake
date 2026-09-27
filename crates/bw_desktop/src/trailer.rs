//! Trailer footage. The practice AI plays every seat of a match with the fog
//! lifted, and the capture draws the field full bleed, without the
//! interface, along a scripted camera. Frames go straight to ffmpeg, one
//! clip per shot, and beside each clip a list of the sounds its frames make
//! (time, effect, loudness, pan) for the trailer's mix.
//!
//! `brinewake --trailer-scout SEED MAP [FACTION...]` prints a match's
//! timeline to choose shots from; `brinewake --trailer-capture SHOTS.json
//! DIR [NAME]` renders the shots a JSON file describes (see [`Shot`]).
//!
//! Presentation only: the matches are the practice AI's own, and nothing
//! here changes a rule.

use crate::game::{Game, Screen};
use crate::spectator::Spectator;
use bw_core::{FP, Faction, Kind, Pos, TICK_HZ};
use bw_sim::{MapId, World};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The AI's own pace between turns.
const AI_TURN: u64 = 15;

/// Where the camera looks.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Focus {
    /// The sluice.
    Gate,
    /// A seat's headquarters.
    Hq(u8),
    /// A crossing mouth: arm, then end (0 or 1).
    Mouth(usize, usize),
    /// The middle of a seat's combat machines.
    Army(u8),
    /// The biggest fight, as the spectator camera chooses it.
    Fight,
    /// A cell.
    Cell(i32, i32),
}

/// How the camera moves over a shot.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Move {
    /// Hold on one focus (followed if it moves, eased).
    Hold { at: Focus },
    /// From one focus to another, eased at both ends.
    Pan { from: Focus, to: Focus },
}

/// A scripted tide change: at `tick`, `seat` takes the sluice and sets the
/// tide ("north", "south" or an arm number, or "flood"). For the trailer's
/// footage only: the practice AI rarely moves the tide in a short match.
#[derive(Clone, Debug, Deserialize)]
pub struct TideScript {
    pub tick: u64,
    pub seat: u8,
    pub tide: String,
}

/// A staged army: at `tick`, `seat` gets these machines in ranks around
/// cell `at`, facing `attack`, which they attack-move to. Deploying kinds
/// (Loom, Bulwark, Heliostat) deploy where they stand when `deploy` is set.
#[derive(Clone, Debug, Deserialize)]
pub struct Army {
    pub tick: u64,
    pub seat: u8,
    pub kinds: Vec<(Kind, u32)>,
    pub at: (i32, i32),
    #[serde(default)]
    pub attack: Option<(i32, i32)>,
    /// Machines a rank.
    #[serde(default = "rank")]
    pub rank: u32,
    #[serde(default)]
    pub deploy: bool,
}

/// A finished building placed at `tick`.
#[derive(Clone, Debug, Deserialize)]
pub struct Building {
    pub tick: u64,
    pub seat: u8,
    pub kind: Kind,
    pub at: (i32, i32),
}

fn rank() -> u32 {
    6
}

/// One clip of the trailer.
#[derive(Clone, Debug, Deserialize)]
pub struct Shot {
    pub name: String,
    pub seed: u64,
    /// "split_basin" or "confluence".
    #[serde(default)]
    pub map: Option<String>,
    /// One per seat, "union", "assembly" or "compact".
    pub factions: Vec<String>,
    /// The tick the shot starts on.
    pub start: u64,
    pub frames: u32,
    /// Match ticks per frame (1 is real time at 30 frames a second).
    #[serde(default = "one")]
    pub speed: u32,
    /// World scale at the start and the end, 1 to 3.
    pub zoom: (f64, f64),
    pub camera: Move,
    /// Where the focus sits, in window pixels from the centre.
    #[serde(default)]
    pub offset: (f64, f64),
    /// Ease the follow camera by this share a frame (Hold only).
    #[serde(default = "follow_ease")]
    pub ease: f64,
    #[serde(default)]
    pub tides: Vec<TideScript>,
    /// Whether the practice AI plays (off for staged fights).
    #[serde(default = "yes")]
    pub ai: bool,
    #[serde(default)]
    pub armies: Vec<Army>,
    #[serde(default)]
    pub buildings: Vec<Building>,
}

fn yes() -> bool {
    true
}

fn one() -> u32 {
    1
}

fn follow_ease() -> f64 {
    0.06
}

fn faction(name: &str) -> Result<Faction, String> {
    match name {
        "union" => Ok(Faction::Union),
        "assembly" => Ok(Faction::Assembly),
        "compact" => Ok(Faction::Compact),
        other => Err(format!("unknown faction {other}")),
    }
}

fn map_id(name: Option<&str>) -> Result<MapId, String> {
    match name.unwrap_or("split_basin") {
        "split_basin" => Ok(MapId::SplitBasin),
        "confluence" => Ok(MapId::Confluence),
        other => Err(format!("unknown map {other}")),
    }
}

/// A match the AI plays on every seat, driven through the game so the
/// presentation (motion, traces, effects) keeps up.
struct Demo {
    game: Game,
    spectator: Spectator,
    tides: Vec<TideScript>,
    ai: bool,
    armies: Vec<Army>,
    buildings: Vec<Building>,
}

impl Demo {
    fn new(base: &Path, seed: u64, map: MapId, factions: &[Faction]) -> Result<Self, String> {
        let data = std::env::temp_dir().join(format!("brinewake-trailer-{}", std::process::id()));
        std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
        let mut game = Game::new_with_data_dir(base.to_path_buf(), data);
        let mut world = World::with_map(seed, map, factions)?;
        world.revealed = true;
        game.world = world;
        game.faction = factions[0];
        game.screen = Screen::Match;
        game.audio = None;
        game.cinematic = true;
        Ok(Self {
            game,
            spectator: Spectator::default(),
            tides: Vec::new(),
            ai: true,
            armies: Vec::new(),
            buildings: Vec::new(),
        })
    }

    fn step(&mut self) {
        let tick = self.game.world.tick;
        self.game.world.ai_enabled = self.ai;
        if self.ai && tick.is_multiple_of(AI_TURN) {
            self.game.world.ai_turn_for(0);
        }
        for b in self.buildings.iter().filter(|b| b.tick == tick) {
            self.game
                .world
                .spawn_for_tests(b.seat, b.kind, Pos::cell(b.at.0, b.at.1));
        }
        for army in self.armies.iter().filter(|a| a.tick == tick) {
            let world = &mut self.game.world;
            let mut ids = Vec::new();
            let mut deploying = Vec::new();
            let mut i = 0u32;
            let total: u32 = army.kinds.iter().map(|(_, n)| n).sum();
            let rows = total.div_ceil(army.rank.max(1));
            for (kind, count) in &army.kinds {
                for _ in 0..*count {
                    let (col, row) = ((i % army.rank) as i32, (i / army.rank) as i32);
                    let dx = col * 2 - army.rank as i32 + 1;
                    let dy = row * 2 - rows as i32 + 1;
                    let pos = Pos::cell(army.at.0 + dx, army.at.1 + dy);
                    let id = world.spawn_for_tests(army.seat, *kind, pos);
                    if matches!(kind, Kind::Loom | Kind::Bulwark | Kind::Heliostat) && army.deploy {
                        deploying.push(id);
                    } else {
                        ids.push(id);
                    }
                    i += 1;
                }
            }
            if let Some((x, y)) = army.attack {
                let _ = world.issue(
                    army.seat,
                    bw_sim::Command::AttackMove {
                        units: ids,
                        target: Pos::cell(x, y),
                        queued: false,
                    },
                );
            }
            if !deploying.is_empty() {
                let _ = world.issue(
                    army.seat,
                    bw_sim::Command::SetDeployed {
                        units: deploying,
                        deployed: true,
                    },
                );
            }
        }
        for script in self.tides.iter().filter(|s| s.tick == tick) {
            let world = &mut self.game.world;
            world.gate.owner = Some(script.seat);
            if let Some(player) = world.players.get_mut(usize::from(script.seat)) {
                player.pressure = player.pressure.max(player.pressure_cap);
            }
            let command = match script.tide.as_str() {
                "flood" => bw_sim::Command::Flood,
                "north" => bw_sim::Command::SetTide {
                    arm: bw_sim::Arm::NORTH,
                },
                "south" => bw_sim::Command::SetTide {
                    arm: bw_sim::Arm::SOUTH,
                },
                arm => bw_sim::Command::SetTide {
                    arm: bw_sim::Arm(arm.parse().unwrap_or(0)),
                },
            };
            if let Err(e) = world.issue(script.seat, command) {
                eprintln!("tide at {tick}: {e}");
            }
        }
        self.game.tick();
        self.spectator.observe(&self.game.world);
    }

    /// A focus as a point in the scene (projected world pixels).
    fn point(&mut self, focus: &Focus) -> Option<(f64, f64)> {
        let world = &self.game.world;
        let pos = match focus {
            Focus::Gate => world.map.gate_pos,
            Focus::Hq(seat) => {
                world
                    .entities
                    .iter()
                    .find(|e| e.owner == *seat && e.kind == Kind::Headquarters && e.hp > 0)?
                    .pos
            }
            Focus::Mouth(arm, end) => *world.crossing_mouths().get(*arm)?.get(*end)?,
            Focus::Army(seat) => {
                let army: Vec<Pos> = world
                    .entities
                    .iter()
                    .filter(|e| {
                        e.owner == *seat
                            && e.hp > 0
                            && !e.kind.is_building()
                            && !e.kind.is_worker()
                            && e.aboard.is_none()
                    })
                    .map(|e| e.pos)
                    .collect();
                if army.is_empty() {
                    return None;
                }
                let n = army.len() as i64;
                Pos::raw(
                    (army.iter().map(|p| i64::from(p.x)).sum::<i64>() / n) as i32,
                    (army.iter().map(|p| i64::from(p.y)).sum::<i64>() / n) as i32,
                )
            }
            Focus::Fight => self.spectator.choose_focus(world)?,
            Focus::Cell(x, y) => Pos::raw(x * FP + FP / 2, y * FP + FP / 2),
        };
        Some(project(pos))
    }

    /// Point the game's camera so that `point` sits at the window's centre
    /// plus `offset`, with sub-texel precision.
    fn aim(&mut self, point: (f64, f64), zoom: f64, offset: (f64, f64)) {
        self.game.zoom = crate::zoom::Zoom::new(zoom);
        let (sx, sy) = (point.0 - offset.0 / zoom, point.1 - offset.1 / zoom);
        self.game.camera.x = sx.floor() as i32 - 320;
        self.game.camera.y = sy.floor() as i32 - 156;
        self.game.camera_sub = (sx - sx.floor(), sy - sy.floor());
    }

    fn draw(&mut self) -> &[u8] {
        self.game.frame += 1;
        self.game.cursor = (-10_000, -10_000);
        self.game.full_bleed = true;
        self.game.draw_zoomed_world();
        self.game.full_bleed = false;
        &self.game.canvas.pixels
    }
}

fn project(pos: Pos) -> (f64, f64) {
    (
        f64::from(pos.x - pos.y) * 16.0 / f64::from(FP),
        f64::from(pos.x + pos.y) * 8.0 / f64::from(FP),
    )
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Render one shot to `dir/NAME.mp4`, with `dir/NAME.sounds.json`.
fn capture(base: &Path, shot: &Shot, dir: &Path) -> Result<(), String> {
    let factions = shot
        .factions
        .iter()
        .map(|f| faction(f))
        .collect::<Result<Vec<_>, _>>()?;
    let mut demo = Demo::new(base, shot.seed, map_id(shot.map.as_deref())?, &factions)?;
    demo.tides = shot.tides.clone();
    demo.ai = shot.ai;
    demo.armies = shot.armies.clone();
    demo.buildings = shot.buildings.clone();
    let (w, h) = (1920u32, 1080u32);
    demo.game.resize_view(w, h);
    while demo.game.world.tick < shot.start && demo.game.world.outcome.is_none() {
        demo.step();
    }
    let out = dir.join(format!("{}.mp4", shot.name));
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
        ])
        .arg(format!("{w}x{h}"))
        .args([
            "-r", "30", "-i", "-", "-c:v", "libx264", "-preset", "medium", "-crf", "10",
            "-pix_fmt", "yuv444p",
        ])
        .arg(&out)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start ffmpeg: {e}"))?;
    let mut stdin = ffmpeg.stdin.take().ok_or("no ffmpeg input")?;
    let mut sounds = Vec::new();
    let mut eased: Option<(f64, f64)> = None;
    for frame in 0..shot.frames {
        let t = if shot.frames > 1 {
            f64::from(frame) / f64::from(shot.frames - 1)
        } else {
            0.0
        };
        let zoom = shot.zoom.0 + (shot.zoom.1 - shot.zoom.0) * smooth(t);
        let point = match &shot.camera {
            Move::Hold { at } => {
                let target = demo.point(at).or(eased).unwrap_or((0.0, 0.0));
                let next = match eased {
                    Some((x, y)) => (
                        x + (target.0 - x) * shot.ease,
                        y + (target.1 - y) * shot.ease,
                    ),
                    None => target,
                };
                eased = Some(next);
                next
            }
            Move::Pan { from, to } => {
                let a = demo.point(from).unwrap_or((0.0, 0.0));
                let b = demo.point(to).unwrap_or(a);
                let e = smooth(t);
                (a.0 + (b.0 - a.0) * e, a.1 + (b.1 - a.1) * e)
            }
        };
        demo.aim(point, zoom, shot.offset);
        let pixels = demo.draw();
        stdin
            .write_all(pixels)
            .map_err(|e| format!("ffmpeg stopped: {e}"))?;
        // The sounds this frame's ticks make, heard from this camera.
        for _ in 0..shot.speed {
            let before: BTreeMap<u32, (Kind, u8, Pos)> = demo
                .game
                .world
                .entities
                .iter()
                .map(|e| (e.id, (e.kind, e.owner, e.pos)))
                .collect();
            demo.step();
            let world = &demo.game.world;
            for event in &world.events {
                let Some((cue, loud, place)) =
                    crate::sound_director::event_cue(world, event, &before, true)
                else {
                    continue;
                };
                let (pan, gain, far) = match place {
                    crate::sound_director::Place::At(pos) => {
                        match crate::sound_director::hear(demo.game.camera, pos) {
                            Some(heard) => heard,
                            None => continue,
                        }
                    }
                    crate::sound_director::Place::Global => (0.0, 1.0, false),
                };
                sounds.push(serde_json::json!({
                    "t": f64::from(frame) / 30.0,
                    "cue": format!("{cue:?}"),
                    "gain": loud * gain,
                    "pan": pan,
                    "far": far,
                }));
            }
        }
    }
    drop(stdin);
    let status = ffmpeg.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("ffmpeg failed on {}", shot.name));
    }
    let meta = serde_json::json!({
        "name": shot.name,
        "frames": shot.frames,
        "start_tick": shot.start,
        "end_tick": demo.game.world.tick,
        "sounds": sounds,
    });
    std::fs::write(
        dir.join(format!("{}.sounds.json", shot.name)),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}: {} frames, {} sounds",
        shot.name,
        shot.frames,
        meta["sounds"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}

/// Render the shots in `shots` (a JSON list of [`Shot`]), or only `only`.
pub fn export(base: &Path, shots: &Path, dir: &Path, only: Option<&str>) -> Result<(), String> {
    let text = std::fs::read_to_string(shots).map_err(|e| format!("{}: {e}", shots.display()))?;
    let shots: Vec<Shot> =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", shots.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for shot in shots
        .iter()
        .filter(|s| only.is_none_or(|name| s.name == name))
    {
        capture(base, shot, dir)?;
    }
    Ok(())
}

/// Print a match's timeline: every ten seconds the tide, each seat's army
/// and buildings, the biggest fight, the counts, and the deaths since.
pub fn scout(
    base: &Path,
    seed: u64,
    map: &str,
    factions: &[String],
    minutes: u64,
) -> Result<(), String> {
    let factions = factions
        .iter()
        .map(|f| faction(f))
        .collect::<Result<Vec<_>, _>>()?;
    let mut demo = Demo::new(base, seed, map_id(Some(map))?, &factions)?;
    {
        let world = &demo.game.world;
        let cell = |p: Pos| format!("({},{})", p.x / FP, p.y / FP);
        println!("gate {}", cell(world.map.gate_pos));
        for (arm, mouths) in world.crossing_mouths().iter().enumerate() {
            println!("arm {arm} mouths {} {}", cell(mouths[0]), cell(mouths[1]));
        }
        for e in world
            .entities
            .iter()
            .filter(|e| e.kind == Kind::Headquarters)
        {
            println!("seat {} headquarters {}", e.owner, cell(e.pos));
        }
    }
    let mut deaths = 0usize;
    let mut last_tide = format!("{:?}", demo.game.world.gate.tide);
    println!(
        "tick   time  tide        armies (combat/buildings)        fight              counts  deaths"
    );
    while demo.game.world.tick < minutes * 60 * TICK_HZ && demo.game.world.outcome.is_none() {
        demo.step();
        let world = &demo.game.world;
        deaths += world
            .events
            .iter()
            .filter(|e| e.kind == bw_sim::EventKind::Death)
            .count();
        let tide = format!("{:?}", world.gate.tide);
        if tide != last_tide {
            println!(
                "{:>6} {:>5}  TIDE {} -> {}",
                world.tick,
                clock(world.tick),
                last_tide,
                tide
            );
            last_tide = tide;
        }
        if world.tick.is_multiple_of(10 * TICK_HZ) {
            let armies: Vec<String> = world
                .seats()
                .map(|s| {
                    let combat = world
                        .entities
                        .iter()
                        .filter(|e| {
                            e.owner == s && e.hp > 0 && !e.kind.is_building() && !e.kind.is_worker()
                        })
                        .count();
                    let buildings = world
                        .entities
                        .iter()
                        .filter(|e| e.owner == s && e.hp > 0 && e.kind.is_building())
                        .count();
                    format!("{combat:>2}/{buildings:<2}")
                })
                .collect();
            let fight = demo
                .spectator
                .choose_focus(world)
                .map(|p| format!("({},{})", p.x / FP, p.y / FP))
                .unwrap_or_else(|| "-".into());
            let counts: Vec<String> = world
                .seats()
                .filter(|s| world.holds_every_lane(*s))
                .map(|s| s.to_string())
                .collect();
            println!(
                "{:>6} {:>5}  {:<10}  {:<32} {:<18} {:<7} {}",
                world.tick,
                clock(world.tick),
                last_tide,
                armies.join("  "),
                fight,
                counts.join(","),
                deaths
            );
            deaths = 0;
        }
    }
    if let Some(outcome) = &demo.game.world.outcome {
        println!(
            "{:>6} {:>5}  OUTCOME {outcome:?}",
            demo.game.world.tick,
            clock(demo.game.world.tick)
        );
    }
    Ok(())
}

fn clock(tick: u64) -> String {
    let s = tick / TICK_HZ;
    format!("{}:{:02}", s / 60, s % 60)
}
