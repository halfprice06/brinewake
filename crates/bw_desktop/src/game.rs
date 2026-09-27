//! Game-facing scene, HUD and command translation. The simulation stays authoritative.
use crate::audio::{Audio, Cue};
use crate::canvas::*;
use crate::console::{
    ConsoleResources, ConsoleRoute, ConsoleSelection, ConsoleState, ProductionTicket,
};
use crate::controls::building_hotkey_kind;
use crate::presentation::{
    self, ImpactMaterial, Landmark, gate_asset, gate_warning_phase, material_for_faction,
    resource_stage, salvage_stage_asset, wreck_asset,
};
use crate::ux::UxState;
use bw_content::spec;
use bw_core::{Camera, EntityId, FP, Faction, Kind, Pos, Terrain};
use bw_sim::{Command, EventKind, Order, Outcome, World};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Two seconds without a free cell at the door.
const EXIT_BLOCKED_TICKS: u64 = 60;

/// A new order is written to the live recording within this many ticks.
const ORDER_RECORD_TICKS: u64 = 30;
/// The control groups beside the live recording, in the saves folder.
const LIVE_GROUPS_FILE: &str = "live-match-groups.json";

/// A background writer for live recordings.
///
/// A recording grows with the match, and writing one on the loop thread stops
/// this seat for as long as it takes, which in a lockstep session stops the
/// other seat too.  One thread, one pending world: a write still in flight
/// when the next is due drops the older one, because only the newest is worth
/// having.
struct Recorder {
    tx: std::sync::mpsc::Sender<(PathBuf, World)>,
    /// The command log's length and the tick at the last write: a new order
    /// is written within a second, so a match carried on from the recording
    /// keeps the production it queued (trial 10 resumed 120 ticks short).
    commands: usize,
    written: u64,
    /// The match (seed and seat) and control groups last written beside
    /// the recording.
    groups: Option<(u64, u8, Vec<Vec<EntityId>>)>,
}

impl Recorder {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, World)>();
        std::thread::spawn(move || {
            while let Ok((path, world)) = rx.recv() {
                // Only the newest queued world matters; skip past any that
                // piled up while the last write ran.
                let (path, world) = rx.try_iter().last().unwrap_or((path, world));
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = world.export_replay(&path);
            }
        });
        Recorder {
            tx,
            commands: 0,
            written: 0,
            groups: None,
        }
    }

    fn write(&self, path: PathBuf, world: World) {
        let _ = self.tx.send((path, world));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Setup,
    Settings,
    Confirm,
    Match,
    Pause,
    Help,
    /// Playing online: host, join, pick sides.
    Lobby,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Context,
    Attack,
    Face,
    Gather,
    Build(Kind),
    /// GLINT: the next click picks the ray's direction (rules 18).
    Glint,
    /// LAY: the next click picks the tidal cell to crust toward.
    Lay,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the online screen.
    Online,
    /// Pin the sluice card open under the tide gauge, or unpin it.
    TideCard,
    /// A control on the online screen.
    Net(crate::net_ui::NetAction),
    FilterSelection(Kind, bool),
    RosterPage(bool),
    RecallGroup(usize),
    CenterGroup(usize),
    ToggleHealthBars,
    CycleInterfaceScale,
    RosterFaction(Faction),
    /// The Guide roster's shared-buildings view.
    RosterBuildings,
    /// Show one roster card's details.
    RosterUnit(Kind),
    /// Dry an arm of the tide (deep on the others).
    SetTide(bw_sim::Arm),
    /// Every tidal cell deep for a while.
    Flood,
    Start,
    Setup,
    StartPractice,
    Settings,
    Back,
    ToggleEdgeScroll,
    ToggleFocusPause,
    CycleSpeed,
    FocusAlert(Option<u64>),
    /// Centre on the crossing mouth that decides the running tide hold.
    FocusHold,
    /// A lane chip on the sluice card: look at what blocks the lane, else
    /// at our bank of it (trial 12).
    FocusLane(usize),
    /// Fold or unfold the sluice card and the field buttons.
    CompactFieldCards,
    SelectWorks,
    TrainBatch(Kind),
    ToggleFullscreen,
    GuidePage(usize),
    ManualSubject(crate::field_manual::ManualSubject),
    ManualPlay,
    ManualStep,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    FocusWorker,
    IdleWorker,
    AllIdleWorkers,
    SelectArmy,
    FocusWorks,
    FocusArmy,
    FocusGate,
    Gather,
    EndGuidance,
    Confirm,
    DismissConfirm,
    SaveAndConfirm,
    Quit,
    Faction(Faction),
    /// Pick the map for a new skirmish.
    Map(bw_sim::MapId),
    /// The computer seats' faction for a new skirmish, `None` for random.
    Opponent(Option<Faction>),
    /// How hard the computer plays.
    AiLevel(bw_sim::AiLevel),
    Resume,
    Help,
    MainMenu,
    Save,
    Load,
    ExportReplay,
    /// Watch the match that just ended from its start, as an observer.
    WatchReplay,
    /// The result screen's summary or its dock log.
    ResultPage(crate::dock_log::ResultPage),
    /// Page the result log toward the start of the match (true) or back.
    ResultLogScroll(bool),
    /// Watch a recording again from its start.
    ReplayRestart,
    Mute,
    Volume(crate::audio::Bus),
    /// Nudge a volume by this many points (the arrow keys on its slider:
    /// 1, or 10 with Shift).
    VolumeStep(crate::audio::Bus, i8),
    /// Set a volume, 0-100 (a click or drag on its slider). Saved when the
    /// drag ends.
    VolumeSet(crate::audio::Bus, u8),
    /// Open a Settings tab: sound, controls, display, keys.
    SettingsTab(u8),
    EndReview,
    Train(Kind),
    Build(Kind),
    Attack,
    Stop,
    Hold,
    /// Deploy the selected specialists; never packs (rules 14).
    Deploy,
    /// Pack the selected specialists; never deploys.
    Pack,
    /// Keep the selected specialists deployed through group moves, or not.
    KeepDeployed,
    /// Every worker (F7).
    SelectWorkers,
    /// Every own building of a kind (Ctrl and its build key).
    SelectBuildings(Kind),
    Formation,
    Face,
    Surge,
    Research(bw_content::Doctrine),
    CancelResearch,
    Upgrade(bw_content::Upgrade),
    CancelUpgrade,
    Vent,
    Sound,
    /// A Glinter's GLINT: arms the next click as the ray's direction.
    Glint,
    /// A Salter's LAY: arms the next click as the water to crust.
    Lay,
    /// A transport sets its hold down beside it.
    Unload,
    /// The selected machines board the nearest own transport with room.
    Board,
    /// The headquarters melts pressure into salvage.
    Reclaim,
    /// The selected workers walk to the nearest own headquarters, Works
    /// or Salvage Yard and are broken up for half their salvage, freeing
    /// their crew (rules 19).
    Recycle,
    Capture,
    Switch,
    Cancel,
    Surrender,
}

/// A volume slider's track on its Settings row: left edge and width.
pub(crate) fn volume_track(b: &Button) -> (i32, i32) {
    (b.x + 104, b.w - 104 - 48)
}

/// The level, 0-100, under canvas column `x` on a volume row's slider.
pub(crate) fn volume_at(b: &Button, x: i32) -> u8 {
    let (tx, w) = volume_track(b);
    ((x - tx) * 100 + w / 2).div_euclid(w.max(1)).clamp(0, 100) as u8
}

#[derive(Clone)]
pub struct Button {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub label: String,
    pub hint: String,
    pub action: Action,
    pub enabled: bool,
}
impl Button {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// The controls drawn over the field and the group bar: they fire on
/// release, a press that becomes a drag boxes the ground under them, and
/// they yield to a pending placement or order.
pub(crate) fn overlay_action(action: &Action) -> bool {
    // The quick-select column, the group tabs and the zoom buttons moved
    // into the dock and the top bar (the lean HUD): only the sluice card's
    // controls still stand on the field.
    matches!(
        action,
        Action::SetTide(_)
            | Action::Flood
            | Action::FocusHold
            | Action::FocusLane(_)
            | Action::CompactFieldCards
    )
}

/// A card on the field that lets a click through to the ground while an
/// order or a placement waits for it: the sluice card's controls and the
/// alert cards (trial 10: an ENEMY SEEN card took a Drydock's placement).
pub(crate) fn passes_pending_click(action: &Action) -> bool {
    overlay_action(action) || matches!(action, Action::FocusAlert(_))
}

/// Where the enemy edge is drawn around a sprite: one outline, or four
/// offsets at interface scale two so the edge is two pixels in both axes
/// and survives a halved picture.
pub(crate) fn enemy_edge_offsets(ui_scale: i32) -> &'static [(i32, i32)] {
    if ui_scale >= 2 {
        &[(0, 0), (1, 0), (0, 1), (1, 1)]
    } else {
        &[(0, 0)]
    }
}

pub struct Game {
    pub world: World,
    pub camera: Camera,
    /// The camera's part of a texel beyond `camera`, for smooth panning and
    /// zooming. Presentation only; saves and the simulation never see it.
    pub(crate) camera_sub: (f64, f64),
    pub zoom: crate::zoom::Zoom,
    /// Where an eased zoom is heading, while it moves.
    pub(crate) zoom_goal: Option<crate::zoom::ZoomGoal>,
    pub(crate) scene_canvas: Canvas,
    pub(crate) drawing_scene: bool,
    pub(crate) scene_pointer_allowed: bool,
    /// What each named thing on the field says under the pointer, in the
    /// scene's own pixels, gathered while the scene is drawn.
    pub(crate) field_tips: Vec<crate::field_labels::FieldTip>,
    /// A placement's refusal, which stands at the site whatever the
    /// pointer is over.
    pub(crate) placement_tip: Option<crate::field_labels::FieldTip>,
    /// The tip to draw this frame, anchored in window pixels.
    pub(crate) hover_tip: Option<crate::field_labels::FieldTip>,
    /// The machine or building under the pointer this frame.
    pub(crate) hovered: Option<EntityId>,
    /// The practice coach's bubble and what it points at, as last drawn.
    pub(crate) coach_rect: Option<crate::native_ui::Rect>,
    pub(crate) coach_ring: Option<crate::coach::CoachRing>,
    /// The tide gauge in the top bar, as last drawn.
    pub(crate) gauge_rect: Option<crate::native_ui::Rect>,
    /// The sluice card was opened by the pointer on the gauge and the
    /// pointer has stayed on the gauge or the card since.  Passing over the
    /// field where the card stands never opens it (trial 10: it swallowed
    /// clicks meant for the ground).
    pub(crate) tide_card_hover: bool,
    /// An enemy clicked to read its numbers, and the own selection it was
    /// clicked over: that selection keeps its orders, and a new selection
    /// ends the reading (trial 10: a click only named the enemy).
    pub(crate) inspected: Option<(EntityId, Vec<EntityId>)>,
    /// Until this tick a lone Escape does not open the menu: it follows a
    /// placement that was refused or has just ended (trial 10).
    pub(crate) escape_guard_until: u64,
    /// Set for the moment a right-click is read: the sluice card's body
    /// lets it through to the ground.
    pub(crate) pointer_through_card: bool,
    pub selected: Vec<EntityId>,
    pub screen: Screen,
    pub mode: Mode,
    pub canvas: Canvas,
    pub(crate) ui_canvas: Canvas,
    pub atlas: Option<Atlas>,
    pub buttons: Vec<Button>,
    pub cursor: (i32, i32),
    /// The selection current when the prompt row's message was set: the
    /// message belongs to it and hides once the selection changes.
    pub message_selection: Vec<EntityId>,
    /// The field that plays behind the home menu, made when first shown.
    pub(crate) backdrop: Option<Box<crate::home_backdrop::Backdrop>>,
    /// The world is drawn over the whole window, not between the top bar
    /// and the dock: the home backdrop.
    pub(crate) full_bleed: bool,
    /// Trailer capture: the field without order overlays.
    pub(crate) cinematic: bool,
    pub drag: Option<(i32, i32)>,
    pub faction: Faction,
    /// The map a new skirmish is played on.
    pub map: bw_sim::MapId,
    /// The computer seats' faction for a new skirmish, `None` for a random
    /// one per seat.
    pub opponent: Option<Faction>,
    /// How hard the computer plays a new skirmish.
    pub ai_level: bw_sim::AiLevel,
    pub message: String,
    pub message_until: u64,
    pub frame: u64,
    pub paused_from_help: Screen,
    pub groups: [Vec<EntityId>; 10],
    pub base: PathBuf,
    pub last_click: Option<(EntityId, u64)>,
    pub data_dir: PathBuf,
    pub replay_saved: bool,
    pub audio: Option<Audio>,
    pub muted: bool,
    pub effects: Vec<Effect>,
    pub last_group: Option<(usize, u64)>,
    /// Visual tick of the last pointer movement: tooltips close once the
    /// pointer has rested for a while, so a parked pointer never leaves a
    /// panel over the field.
    pub pointer_moved_tick: u64,
    pub motion: BTreeMap<EntityId, (Pos, u64, u64)>,
    /// Presentation-only event clocks. These are intentionally excluded from
    /// the authoritative world, saves, and replay hashes.
    pub fire_starts: BTreeMap<EntityId, u64>,
    pub unload_starts: BTreeMap<EntityId, u64>,
    pub resource_stages: BTreeMap<u32, u8>,
    pub gate_foam_start: Option<u64>,
    /// The lock switching: (change tick, the side now dry is north), while
    /// the gate plays its switch frames. Presentation only.
    pub gate_switch: Option<(u64, bool)>,
    /// The three-arm station's swing (maps with three arms): the pose it
    /// last rested in and when it began moving to the gate's current one.
    /// Presentation only.
    pub station3: Option<crate::station3::Motion>,
    /// Tick of the last public gate change: the drained lane's drying minute
    /// and the flood front are derived from it. Presentation only.
    pub tide_change: Option<crate::lane_memory::TideChange>,
    /// Crust rows a Salter just laid: (cell, tick), drawn fresh for a
    /// second. Presentation only.
    pub crust_laid: Vec<((i32, i32), u64)>,
    /// Coast sites a machine has disturbed: (flushed tick, cleared tick).
    pub coast_flush: BTreeMap<usize, (u64, Option<u64>)>,
    /// Patch marks left by observed repairs, per building.
    pub repair_marks: crate::damage::RepairMarks,
    /// Producers whose finished machine has found no free cell: the first
    /// tick seen and whether the card has been raised.
    pub exit_blocked: BTreeMap<EntityId, (u64, bool)>,
    /// The pointer position at which a key or click dismissed the tooltip.
    pub tooltip_dismissed_at: Option<(i32, i32)>,
    /// SOUND rings on the ground: where, and until which tick.
    pub pings: Vec<(Pos, u64)>,
    /// VENT plumes at headquarters: the building and until which tick.
    pub vents: BTreeMap<EntityId, u64>,
    /// A trusted lockstep session with another player. When present, the
    /// world is stepped by the session and `world` is this seat's view.
    pub session: Option<crate::net::Session>,
    /// An online lobby being set up; it becomes `session` when the match
    /// starts.
    pub lobby: Option<crate::net::Lobby>,
    /// The online screen's field, errors and clipboard.
    pub net_ui: crate::net_ui::NetUi,
    /// Writes live recordings away from the loop thread.
    recorder: Option<Recorder>,
    /// A desync leaves its post-mortem once, not once a frame.
    desync_recorded: bool,
    /// A recorded match being observed with the fog lifted; no orders.
    pub playback: Option<crate::playback::Playback>,
    /// Army, income, sluice and hold history for the result screen.
    pub summary: crate::match_summary::MatchSummary,
    /// Captures, cancelled switches and hold counts, as sound and as text.
    pub tide_cues: crate::tide_cues::TideCues,
    /// Spectator presentation: the follow camera and the replay timeline.
    pub spectator: crate::spectator::Spectator,
    pub result_page: crate::dock_log::ResultPage,
    pub result_log_scroll: usize,
    pub ux: UxState,
    /// Terminal presentation clock: advances at 30Hz, never from rendering.
    pub aftermath_ticks: u64,
    /// The matchup card at the start of a match, while it shows.
    pub(crate) intro: Option<crate::matchup::Intro>,
    /// What is heard: the fight's heat, rate limits, the last selection.
    pub(crate) sound: crate::sound_director::SoundState,
    /// Workers given a site in the last three seconds, so a second site
    /// from the same selection goes to another worker.
    pub(crate) recent_builders: Vec<(EntityId, u64)>,
    /// Sites this side ordered lately, until they stand on the field.
    pub(crate) placed_sites: Vec<(Kind, Pos, u64)>,
    /// A second site right beside one just placed, waiting for a second
    /// click.
    pub(crate) double_site: Option<(Kind, Pos)>,
    /// Trial 11 control state: the last picture's positions, shift placing.
    pub(crate) controls11: crate::trial11_controls::ControlsState,
    /// Trial 12 input state: a recentre waiting for a batch of input.
    pub(crate) orders12: crate::trial12_orders::OrdersState,
    /// Salvage deposits of the last minute, for the header's rate.
    pub(crate) deposits: std::collections::VecDeque<(u64, u32)>,
    /// The one-time note that pressure at its cap is income lost.
    pub(crate) pressure_full_noted: bool,
    /// Since when pressure has stood below its cap, to note a new fill.
    pub(crate) pressure_below_since: Option<u64>,
    /// The Loom note has been given once this match.
    pub(crate) loom_noted: bool,
    /// Trial 12's field nudges: stranded guns, full pressure, spent home
    /// wrecks and idle producers.
    pub(crate) ux_nudges: crate::trial12_field::FieldNudges,
    /// A field control pressed and not yet released, with the press point.
    pub(crate) pressed_overlay: Option<(Action, i32, i32)>,
    /// The display's pixel ratio (2 on a Retina screen), for the interface scale.
    pub display_scale: i32,
}

pub struct Effect {
    pub from: Pos,
    pub to: Pos,
    pub death: bool,
    pub material: ImpactMaterial,
    pub faction: Faction,
    /// Optional authored muzzle offset from the firing sprite's ground
    /// anchor. It is recorded only when the source is visible, preserving fog
    /// boundaries while keeping directional flashes attached to the weapon.
    pub muzzle: Option<[i32; 2]>,
    /// A Heliostat's beam: the shot's damage, 2 to 10, which sets how hot
    /// the light is drawn.
    pub beam: Option<i32>,
    /// Simulation tick at which this effect expires. Render frames never
    /// advance this clock, so pause and repeated screenshots are stable.
    pub until: u64,
}

#[derive(Clone, Copy)]
enum WorldDrawItem<'a> {
    Resource(&'a bw_sim::Resource),
    Well(Pos),
    Gate(Pos),
    Entity(&'a bw_sim::Entity),
}

struct EntityOverlay {
    owner: u8,
    /// An unfinished site whose builder is absent, dead or busy elsewhere.
    stalled: bool,
    /// An unfinished site whose worker comes once its current site is done.
    queued_site: bool,
    x: i32,
    y: i32,
    selected: bool,
    kind: Kind,
    asset_key: &'static str,
    hp: i32,
    /// The hull the sim gives it with its side's upgrades (Plate, REFIT),
    /// not the spec's: trial 10 showed a refitted Loom at 184/160.
    max_hp: i32,
    build_remaining: u32,
    directional: String,
    animation: Option<String>,
    scale: crate::occlusion::PixelScale,
}

fn ground_depth(pos: Pos) -> i64 {
    i64::from(pos.x) + i64::from(pos.y)
}

fn world_draw_key(item: &WorldDrawItem<'_>) -> (i64, u8, u32) {
    match item {
        WorldDrawItem::Resource(resource) => (ground_depth(resource.pos), 0, resource.id),
        WorldDrawItem::Well(pos) => (ground_depth(*pos), 1, 0),
        WorldDrawItem::Gate(pos) => (ground_depth(*pos), 2, 0),
        WorldDrawItem::Entity(entity) => (crate::occlusion::entity_depth_key(entity), 3, entity.id),
    }
}

fn sort_world_draw_items(items: &mut [WorldDrawItem<'_>]) {
    items.sort_by_key(world_draw_key);
    let slots: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| matches!(item, WorldDrawItem::Entity(_)).then_some(i))
        .collect();
    let entities: Vec<_> = slots
        .iter()
        .map(|&i| match items[i] {
            WorldDrawItem::Entity(e) => e,
            _ => unreachable!(),
        })
        .collect();
    let order = crate::occlusion::stable_entity_order(&entities);
    for (slot, index) in slots.into_iter().zip(order) {
        items[slot] = WorldDrawItem::Entity(entities[index]);
    }
}

pub(crate) fn salvage_asset(id: u32) -> &'static str {
    match id % 3 {
        1 => "salvage_turbine",
        2 => "salvage_barge",
        _ => "salvage",
    }
}

/// Presentation only: existing construction and production state chooses art.
/// Simulation ticks freeze these loops with the match; no visual timer is saved.
fn building_state_key(
    entity: &bw_sim::Entity,
    faction: Faction,
    tick: u64,
    gust: u8,
) -> Option<String> {
    if !entity.kind.is_building() {
        return None;
    }
    let base = entity.kind.asset(faction);
    let total = spec(entity.kind).build_ticks;
    if entity.build_remaining > 0 && total > 0 {
        let done = total.saturating_sub(entity.build_remaining);
        let stage = (done.saturating_mul(3) / total).min(2);
        return Some(format!("{base}_build_{stage}"));
    }
    let active = match entity.kind {
        Kind::Headquarters | Kind::Condenser => entity.build_remaining == 0,
        Kind::Works | Kind::Drydock => {
            entity.build_remaining == 0
                && entity
                    .queue
                    .first()
                    .is_some_and(|p| p.started && p.remaining > 1)
        }
        _ => false,
    };
    // A gust hurries the pennants and steam: the loop steps twice as fast.
    let step = if gust > 0 { 3 } else { 6 };
    active.then(|| format!("{base}_active_{}", (tick / step + u64::from(entity.id)) % 4))
}

impl Game {
    pub fn new(base: PathBuf) -> Self {
        let data_dir = std::env::var_os("BRINEWAKE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // Each test keeps its own data folder, so a preference one
                // test saves cannot reach another.
                if cfg!(test) {
                    std::env::temp_dir().join(format!(
                        "brinewake-test-{}-{:?}",
                        std::process::id(),
                        std::thread::current().id()
                    ))
                } else if base.ends_with("Contents/Resources") {
                    std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| base.clone())
                        .join("Library/Application Support/Brinewake")
                } else if cfg!(windows)
                    && !base.join("Cargo.toml").exists()
                    && let Some(app_data) = std::env::var_os("APPDATA")
                {
                    // A download unzipped anywhere: preferences, the player's
                    // name and saves outlive the folder and the next version.
                    PathBuf::from(app_data).join("Brinewake")
                } else {
                    base.clone()
                }
            });
        Self::new_with_data_dir(base, data_dir)
    }
    pub fn new_with_data_dir(base: PathBuf, data_dir: PathBuf) -> Self {
        let ux = UxState::load(&data_dir);
        let muted = ux.preferences.muted;
        let world = World::new(1, Faction::Union);
        let mut camera = Camera::default();
        if let Some(hq) = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        {
            camera.center(hq.pos)
        }
        let atlas = match Atlas::load(&base) {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("Asset load: {e}");
                None
            }
        };
        Self {
            world,
            camera,
            camera_sub: (0.0, 0.0),
            zoom: crate::zoom::Zoom::Overview,
            zoom_goal: None,
            scene_canvas: Canvas::default(),
            drawing_scene: false,
            scene_pointer_allowed: false,
            field_tips: Vec::new(),
            placement_tip: None,
            hover_tip: None,
            hovered: None,
            coach_rect: None,
            coach_ring: None,
            gauge_rect: None,
            tide_card_hover: false,
            inspected: None,
            escape_guard_until: 0,
            pointer_through_card: false,
            selected: Vec::new(),
            screen: Screen::Menu,
            mode: Mode::Context,
            canvas: Canvas::default(),
            ui_canvas: Canvas::default(),
            atlas,
            buttons: Vec::new(),
            cursor: (0, 0),
            drag: None,
            faction: Faction::Union,
            map: bw_sim::MapId::SplitBasin,
            opponent: Some(Faction::Assembly),
            ai_level: bw_sim::AiLevel::Normal,
            message: String::new(),
            message_until: 0,
            message_selection: Vec::new(),
            backdrop: None,
            full_bleed: false,
            cinematic: false,
            frame: 0,
            paused_from_help: Screen::Match,
            groups: Default::default(),
            data_dir,
            base,
            last_click: None,
            effects: Vec::new(),
            last_group: None,
            pointer_moved_tick: 0,
            motion: BTreeMap::new(),
            replay_saved: false,
            audio: None,
            muted,
            fire_starts: BTreeMap::new(),
            unload_starts: BTreeMap::new(),
            resource_stages: BTreeMap::new(),
            gate_foam_start: None,
            gate_switch: None,
            station3: None,
            tide_change: None,
            crust_laid: Vec::new(),
            coast_flush: BTreeMap::new(),
            repair_marks: crate::damage::RepairMarks::default(),
            exit_blocked: BTreeMap::new(),
            tooltip_dismissed_at: None,
            pings: Vec::new(),
            vents: BTreeMap::new(),
            session: None,
            lobby: None,
            net_ui: Default::default(),
            recorder: None,
            desync_recorded: false,
            playback: None,
            summary: Default::default(),
            tide_cues: Default::default(),
            spectator: Default::default(),
            result_page: Default::default(),
            result_log_scroll: 0,
            ux,
            aftermath_ticks: 0,
            intro: None,
            sound: Default::default(),
            recent_builders: Vec::new(),
            placed_sites: Vec::new(),
            double_site: None,
            controls11: Default::default(),
            orders12: Default::default(),
            deposits: std::collections::VecDeque::new(),
            pressure_full_noted: false,
            ux_nudges: Default::default(),
            pressure_below_since: None,
            loom_noted: false,
            pressed_overlay: None,
            display_scale: 1,
        }
    }
    /// Begin a two-player lockstep match. The local seat's faction and view
    /// come from the session; pause on focus loss and loading are off.
    pub fn start_network(&mut self, session: crate::net::Session) {
        self.faction = session.local_faction();
        self.start();
        self.world = session.view();
        // A joined seat was launched without the host's map: the Guide,
        // its diagram and the hold words follow the session's field
        // (trial 12: the guest and the third read the Split Basin's).
        self.map = self.world.map.id;
        // A match carried on from a recording keeps its history: the
        // summary and the dock log from the first tick, and the control
        // groups this seat last had.
        if let Some((summary, dock)) =
            crate::net::session::resume_history(session.canonical(), session.local)
        {
            self.summary = summary;
            self.ux.dock = dock;
        }
        if self.world.tick > 0 {
            self.restore_live_groups(session.seed, session.local);
        }
        // The view places this seat's headquarters where the canonical world
        // has it, not where the throwaway local world did.
        self.home();
        self.ux.preferences.pause_unfocused = false;
        self.session = Some(session);
        self.begin_intro();
    }
    /// True while a lockstep session must keep stepping although this seat is
    /// not looking at the field. A pause by one seat stalls both, and the
    /// other seat is not the one reading a menu; the pause screen says so.
    /// The pace the field actually runs at: a lockstep match always runs
    /// at the shared normal pace, whatever this seat's preference says.
    pub fn shown_pace(&self) -> crate::tempo::GameSpeed {
        if self.session.is_some() {
            crate::tempo::GameSpeed::Normal
        } else {
            self.ux.preferences.game_speed
        }
    }
    pub fn session_steps_on(&self) -> bool {
        self.session.is_some()
            && matches!(
                self.screen,
                Screen::Help | Screen::Pause | Screen::Settings | Screen::Confirm | Screen::Menu
            )
    }
    /// Observe a recorded match: the fog is lifted, pace and pause work,
    /// orders are off. With `follow` the file is re-read as it grows.
    pub fn start_playback(&mut self, path: &Path, follow: bool) -> Result<(), String> {
        let playback = crate::playback::Playback::open(path, follow)?;
        let view = playback.view();
        self.faction = view.players[0].faction;
        self.start();
        self.world = view;
        self.map = self.world.map.id;
        self.home();
        self.ux.preferences.pause_unfocused = false;
        self.playback = Some(playback);
        // An observer starts on the action; any pan hands the camera back.
        self.spectator.follow = true;
        self.notify("Watching with the fog lifted. F follows the action.");
        Ok(())
    }
    /// The world a recording should hold: the canonical one in a session, not
    /// this seat's relabelled view, which would come back with the sides
    /// swapped.
    fn world_to_record(&self) -> &World {
        match &self.session {
            Some(session) => session.canonical(),
            None => &self.world,
        }
    }
    /// Every ten seconds of field time the match so far is written as a
    /// replay, so an observer can follow it live and nothing is lost if the
    /// match is abandoned. The canonical world is written in a session.
    ///
    /// The write happens on a writer thread: it grows with the match, and on
    /// the loop thread it would stall this seat, and with it the other one.
    ///
    /// The result, a capture and a tide switch are written at once as well:
    /// a live observer otherwise never sees how the match ended.
    fn record_live(&mut self) {
        let tick = self.world.tick;
        let notable = self.world.outcome.is_some()
            || self.world.events.iter().any(|e| {
                matches!(
                    e.kind,
                    bw_sim::EventKind::GateCaptured | bw_sim::EventKind::GateChanged
                )
            });
        let commands = self.world_to_record().command_log.len();
        let ordered = self.recorder.as_ref().is_none_or(|recorder| {
            // A tick before the last write is a new match.
            recorder.commands != commands
                && (tick < recorder.written || tick >= recorder.written + ORDER_RECORD_TICKS)
        });
        self.record_live_groups();
        if tick == 0 || !(tick.is_multiple_of(300) || notable || ordered) {
            return;
        }
        let path = self.data_dir.join("saves").join("live-match.replay.json");
        let world = self.world_to_record().clone();
        let recorder = self.recorder.get_or_insert_with(Recorder::new);
        recorder.commands = commands;
        recorder.written = tick;
        recorder.write(path, world);
    }
    /// A seat's control groups live only on its screen. Beside the live
    /// recording they are written when they change, so a seat carried on
    /// from that recording (`--resume`) has its groups back: in trial 10
    /// "Group 1 is empty" sent three workers on an attack-move.
    fn record_live_groups(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        let seat = session.local;
        let seed = session.seed;
        let groups = self.groups.to_vec();
        let recorder = self.recorder.get_or_insert_with(Recorder::new);
        if recorder
            .groups
            .as_ref()
            .is_some_and(|(match_seed, at, last)| {
                *match_seed == seed && *at == seat && *last == groups
            })
        {
            return;
        }
        let record = serde_json::json!({
            "seed": seed,
            "seat": seat,
            "tick": self.world.tick,
            "groups": groups,
        });
        let dir = self.data_dir.join("saves");
        let path = dir.join(LIVE_GROUPS_FILE);
        let temp = dir.join("live-match-groups.pending.json");
        let written = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&temp, record.to_string()))
            .and_then(|_| std::fs::rename(&temp, &path));
        if written.is_ok() {
            recorder.groups = Some((seed, seat, groups));
        }
    }
    /// The control groups written beside the live recording, when they are
    /// this match's and this seat's: members that no longer stand are left
    /// out.
    pub(crate) fn restore_live_groups(&mut self, seed: u64, seat: u8) {
        let path = self.data_dir.join("saves").join(LIVE_GROUPS_FILE);
        let Some(record) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return;
        };
        if record["seed"].as_u64() != Some(seed) || record["seat"].as_u64() != Some(u64::from(seat))
        {
            return;
        }
        let saved: Vec<Vec<EntityId>> =
            serde_json::from_value(record["groups"].clone()).unwrap_or_default();
        self.groups = crate::ux::sanitize_groups(&saved, &self.world);
        // What was just read is what is on disk: no need to write it back.
        let recorder = self.recorder.get_or_insert_with(Recorder::new);
        recorder.groups = Some((seed, seat, self.groups.to_vec()));
    }
    /// Write the match as `last-match.replay.json`, the file the harness and
    /// the observer look for. Called when a match is decided and when a
    /// headless seat is asked to quit, so an abandoned match is still
    /// watchable.
    pub fn export_final_replay(&mut self) -> Result<(), String> {
        let dir = self.data_dir.join("saves");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        self.world_to_record()
            .export_replay(dir.join("last-match.replay.json"))
    }
    /// Keep what a desync leaves behind: both sides' hashes and the world as
    /// this seat had it. Without this the match ends with a tick number and
    /// nothing to look at.
    fn record_desync(&mut self, tick: u64) {
        let dir = self.data_dir.join("saves");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let Some(session) = &self.session else { return };
        let hashes: Vec<_> = session
            .hash_history()
            .into_iter()
            .map(
                |(at, local, peer)| serde_json::json!({ "tick": at, "local": local, "peer": peer }),
            )
            .collect();
        let report = serde_json::json!({
            "desync_tick": tick,
            "seat": session.local,
            "seed": session.seed,
            "delay": session.delay,
            "rules_digest": bw_content::rules_digest(),
            "build": bw_content::build_fingerprint(),
            "hashes": hashes,
        });
        let _ = std::fs::write(
            dir.join(format!("desync-{tick}.json")),
            serde_json::to_vec_pretty(&report).unwrap_or_default(),
        );
        let _ = session
            .canonical()
            .export_replay(dir.join(format!("desync-{tick}.replay.json")));
    }
    /// Advance the world one tick: locally, through the lockstep session (the
    /// presented world is refreshed from the canonical one), or from a
    /// recording. Returns false when nothing advanced.
    fn step_world(&mut self) -> bool {
        if self.playback.is_some() {
            let (stepped, notice, view) = {
                let playback = self.playback.as_mut().expect("playback present");
                let stepped = playback.step();
                (
                    stepped,
                    playback.notice.take(),
                    stepped.then(|| playback.view()),
                )
            };
            if let Some(notice) = notice {
                self.notify(&notice);
            }
            if let Some(view) = view {
                self.world = view;
            }
            return stepped;
        }
        let Some(session) = self.session.as_mut() else {
            self.world.step();
            self.record_live();
            return true;
        };
        match session.try_step() {
            crate::net::StepOutcome::Stepped => {
                let rejection = session.take_rejection();
                let notices = session.take_notices();
                self.world = session.view();
                for notice in notices {
                    self.notify(&notice);
                }
                self.touch_rejoin_note();
                // The order was confirmed three ticks ago and no longer
                // stands; saying nothing leaves it looking as if it worked.
                if let Some(refusal) = rejection {
                    let words = crate::trial12_hold::late_refusal_words(self, &refusal);
                    self.notify(&words);
                }
                self.record_live();
                true
            }
            crate::net::StepOutcome::Stalled => {
                // A silent player has its own card on the field; the notices
                // (someone left, someone is back) still say so once.
                for notice in session.take_notices() {
                    self.notify(&notice);
                }
                false
            }
            crate::net::StepOutcome::Ended => {
                let desync = session.desync;
                self.world = session.view();
                self.touch_rejoin_note();
                let status = self.session.as_ref().map_or(String::new(), |s| s.status());
                if !status.is_empty() && self.message != status {
                    self.notify(&status);
                }
                if let Some(tick) = desync
                    && !self.desync_recorded
                {
                    self.desync_recorded = true;
                    self.record_desync(tick);
                }
                // A decided match still needs its aftermath bookkeeping.
                self.world.outcome.is_some()
            }
        }
    }
    pub fn start(&mut self) {
        // A new field ends any network match: the others hear goodbye.
        self.end_network();
        if let Some(audio) = &self.audio {
            audio.hush();
        }
        self.aftermath_ticks = 0;
        self.intro = None;
        self.unfold_sluice_card();
        // Each match's sound starts fresh: its heat, threat and rests.
        self.sound = self.sound.fresh();
        self.summary = Default::default();
        self.tide_cues = Default::default();
        self.spectator = Default::default();
        self.result_page = Default::default();
        self.result_log_scroll = 0;
        self.loom_noted = false;
        self.ux_nudges = Default::default();
        self.pressed_overlay = None;
        self.ux.reset_dock_for_new_match();
        self.ux.tactics_fixture = false;
        self.world = World::with_map(1, self.map, &self.match_factions())
            .unwrap_or_else(|_| World::new(1, self.faction));
        self.world.ai_level = self.ai_level;
        self.selected.clear();
        self.mode = Mode::Context;
        self.screen = Screen::Match;
        self.groups = Default::default();
        self.ux.last_idle_worker = None;
        self.ux.paused_unfocused = false;
        self.ux.minimap_drag = false;
        self.effects.clear();
        self.crust_laid.clear();
        self.replay_saved = false;
        self.last_click = None;
        self.last_group = None;
        self.drag = None;
        self.motion.clear();
        self.fire_starts.clear();
        self.unload_starts.clear();
        self.resource_stages.clear();
        self.gate_foam_start = None;
        self.gate_switch = None;
        self.station3 = None;
        self.ux.session_active = true;
        self.ux.practice = false;
        self.ux.tutorial = None;
        self.ux.focused = None;
        self.ux.pending = None;
        self.ux.last_saved_hash = None;
        self.ux.saved_at_tick = None;
        self.ux.markers.clear();
        self.home();
        let works = crate::ux::capitalise(
            &crate::ux::building_name(Kind::Works, self.faction).to_lowercase(),
        );
        self.notify(&format!("Workers are gathering. Build a {works} next."));
    }
    /// With three seats a match goes on after yours is knocked out: lift
    /// the fog so the rest can be watched, and say once where you placed.
    fn note_local_elimination(&mut self) {
        if self.world.revealed || self.world.outcome.is_some() || !self.world.is_eliminated(0) {
            return;
        }
        self.world.revealed = true;
        // One plus the seats that outlasted this one: seats that fall
        // together share a place, as the result screen counts.
        let out_at = self
            .world
            .eliminated
            .iter()
            .find(|&&(seat, _)| seat == 0)
            .map_or(0, |&(_, tick)| tick);
        let outlasted = self
            .world
            .seats()
            .filter(|&seat| {
                self.world
                    .eliminated
                    .iter()
                    .find(|&&(s, _)| s == seat)
                    .is_none_or(|&(_, tick)| tick > out_at)
            })
            .count();
        let ordinal = crate::match_summary::ordinal(outlasted as u8 + 1);
        // Nothing of the seat's is left to select, group, train or warn
        // about: its header, groups and card give way to the watcher's band
        // and dock, and the follow camera takes the biggest fight.
        self.selected.clear();
        self.groups = Default::default();
        self.ux.alerts.entries.clear();
        self.message.clear();
        self.spectator.follow = true;
        self.spectator.focus = None;
        self.mode = Mode::Context;
        if let Some(audio) = &self.audio {
            audio.play(Cue::Eliminated);
        }
        self.notify(&format!("YOU ARE OUT: {ordinal}. WATCHING THE REST."));
    }

    /// The factions of a new skirmish, seat by seat: yours, then the other
    /// faction, then (on the Confluence) your own again as a mirror seat.
    pub fn match_factions(&self) -> Vec<Faction> {
        let seats = self.map.layout().seat_count();
        // A random opponent is drawn per seat from the wall clock: the
        // factions it picks are part of the world from then on.
        let mut roll = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(self.frame, |d| d.as_nanos() as u64)
            | 1;
        std::iter::once(self.faction)
            .chain((1..seats).map(|_| {
                self.opponent.unwrap_or_else(|| {
                    roll ^= roll << 13;
                    roll ^= roll >> 7;
                    roll ^= roll << 17;
                    Faction::ALL[(roll % Faction::ALL.len() as u64) as usize]
                })
            }))
            .collect()
    }
    pub fn notify(&mut self, s: &str) {
        self.message = s.to_string();
        self.message_until = self.frame + 300;
        self.message_selection = self.selected.clone();
    }
    /// Whether the prompt row's message still shows: in time, and about
    /// the selection that was current when it was set.
    pub(crate) fn message_live(&self) -> bool {
        self.frame < self.message_until
            && !self.message.is_empty()
            && self.message_selection == self.selected
    }
    /// A note for a move order that leaves deployed machines behind.
    fn deployed_stay_note(&self, units: &[EntityId]) -> &'static str {
        let deployed =
            |e: &&bw_sim::Entity| units.contains(&e.id) && (e.deployed || e.deploy_remaining > 0);
        let kept = self
            .world
            .entities
            .iter()
            .filter(deployed)
            .any(|e| e.keep_deployed);
        let packing = self
            .world
            .entities
            .iter()
            .filter(deployed)
            .any(|e| !e.keep_deployed);
        match (kept, packing) {
            (true, true) => " KEPT ONES STAY; REST PACKED.",
            (true, false) => " Kept-deployed machines stay.",
            (false, true) => crate::trial12_orders::PACKED_NOTE,
            (false, false) => "",
        }
    }
    /// A producer whose finished machine waits for a free cell is blocked
    /// after two seconds: the panel says so and one card is raised.
    fn track_blocked_exits(&mut self) {
        let tick = self.world.tick;
        let player = &self.world.players[0];
        let stuck: Vec<(EntityId, Kind, Pos)> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.aboard.is_none()
                    && e.kind.is_building()
                    && e.build_remaining == 0
            })
            .filter(|e| {
                e.queue.first().is_some_and(|p| {
                    p.started
                        && p.remaining <= 1
                        && player.crew.saturating_add(spec(p.kind).crew) <= player.cap
                })
            })
            .map(|e| (e.id, e.kind, e.pos))
            .collect();
        self.exit_blocked
            .retain(|id, _| stuck.iter().any(|(s, _, _)| s == id));
        for (id, kind, pos) in stuck {
            let entry = self.exit_blocked.entry(id).or_insert((tick, false));
            if !entry.1 && tick.saturating_sub(entry.0) >= EXIT_BLOCKED_TICKS {
                entry.1 = true;
                self.ux
                    .alerts
                    .push(crate::field_alerts::AlertKind::ExitBlocked(kind), pos, tick);
            }
        }
    }
    pub(crate) fn exit_blocked(&self, id: EntityId) -> bool {
        self.exit_blocked
            .get(&id)
            .is_some_and(|(since, _)| self.world.tick.saturating_sub(*since) >= EXIT_BLOCKED_TICKS)
    }
    /// The player's words for a refused command; the crew ceiling names
    /// what would raise it.
    pub(crate) fn rejection_reason(&self, error: &str) -> String {
        if error == "crew capacity exceeded" {
            crate::trial11_words::crew_ceiling_words(self.world.players[0].cap, self.faction)
        } else {
            crate::ux::friendly_error(error)
        }
    }
    pub fn home(&mut self) {
        if let Some(e) = self
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        {
            self.camera.center(e.pos);
            // A pending placement or order keeps its selection: Space only
            // looks home then.
            if self.mode == Mode::Context {
                self.selected = vec![e.id];
            }
        }
    }
    pub fn tick(&mut self) {
        self.prune_placed_sites();
        if self.screen == Screen::Match && (self.world.outcome.is_some() || self.ux.practice_review)
        {
            self.aftermath_ticks = self.aftermath_ticks.saturating_add(1);
            // The final blow's effects ring out; the score takes a breath
            // and then plays the result (sound_director).
            self.play_harbor(true);
            return;
        }
        if self.screen == Screen::Match && self.intro_holds() {
            return;
        }
        let stepping = self.screen == Screen::Match || self.session_steps_on();
        if stepping && self.world.outcome.is_none() {
            let before_entities: BTreeMap<EntityId, (Kind, u8, Pos)> = self
                .world
                .entities
                .iter()
                .map(|entity| (entity.id, (entity.kind, entity.owner, entity.pos)))
                .collect();
            // Which shaft was dry before the step: an open that keeps the
            // same side dry turns nothing. The sluice art has a north and a
            // south shaft; arm 0 shows the north one and every other arm the
            // south. The tide before the step says which arms change.
            let rest_north_dry = self.world.gate.north_dry();
            let rest_tide = (self.world.gate.tide, self.world.gate.dry_arm);
            if !self.step_world() {
                return;
            }
            // A seat that is out has nothing left to warn about: the band
            // shows every seat's count.
            if !self.world.is_eliminated(0) {
                self.ux.alerts.observe(&self.world, &before_entities);
                self.ux_nudges
                    .observe(&self.world, &mut self.ux.alerts, &self.resource_stages);
            }
            self.note_local_elimination();
            self.track_blocked_exits();
            for event in &self.world.events {
                if event.kind == bw_sim::EventKind::Deposit && event.player == Some(0) {
                    self.deposits
                        .push_back((self.world.tick, event.amount.max(0) as u32));
                }
            }
            while self
                .deposits
                .front()
                .is_some_and(|(t, _)| self.world.tick.saturating_sub(*t) > 1800)
            {
                self.deposits.pop_front();
            }
            self.recent_builders
                .retain(|(_, at)| self.world.tick.saturating_sub(*at) < 90);
            let player = &self.world.players[0];
            let full = player.pressure >= player.pressure_cap;
            // The note names the sink a seat can use at once: Union found
            // RECLAIM at 19:24 with its pressure full for half the match.
            // It comes again when pressure fills after a minute below cap.
            if full {
                self.pressure_below_since = None;
            } else if self.pressure_below_since.is_none() {
                self.pressure_below_since = Some(self.world.tick);
            }
            if self.pressure_full_noted
                && self
                    .pressure_below_since
                    .is_some_and(|since| self.world.tick.saturating_sub(since) >= 60 * 30)
            {
                self.pressure_full_noted = false;
            }
            if !self.pressure_full_noted
                && self.world.tick > 60
                && full
                && self.mode == Mode::Context
                && !self.observing()
            {
                self.pressure_full_noted = true;
                let words = crate::trial11_words::pressure_full_words(self.faction);
                self.notify(&words);
            }
            if !self.loom_noted
                && !self.observing()
                && self.world.entities.iter().any(|e| {
                    e.owner != 0
                        && e.hp > 0
                        && e.kind == Kind::Loom
                        && e.aboard.is_none()
                        && self.world.entity_visible(0, e.id)
                })
            {
                // The one fact the Union line needs, the first time it sees
                // the artillery.
                self.loom_noted = true;
                self.notify("Enemy Looms: blind inside two cells. Surge (Z) closes the distance.");
            }
            if self.world.outcome.is_some() {
                self.mode = Mode::Context;
                self.drag = None;
                self.message.clear();
                // The result is drawn on the field, never over the Guide.
                if self.screen == Screen::Help {
                    self.screen = Screen::Match;
                }
            }
            self.ux.after_authoritative_tick(&self.world);
            self.summary.observe(&self.world);
            if self.observing() {
                self.spectator.observe(&self.world);
                self.follow_camera();
            }
            self.ux.chart_memory.observe(&self.world);
            self.ux.traces.observe_deployments(&self.world);
            for event in &self.world.events {
                self.ux.traces.record_event(&self.world, event);
            }
            self.ux.traces.sanitize(&self.world);
            self.update_tutorial();
            for entity in &self.world.entities {
                let state = self.motion.entry(entity.id).or_insert((entity.pos, 0, 0));
                let distance = (i64::from(entity.pos.x) - i64::from(state.0.x)).unsigned_abs()
                    + (i64::from(entity.pos.y) - i64::from(state.0.y)).unsigned_abs();
                if distance > 0 {
                    state.1 = state.1.wrapping_add(distance);
                    state.2 = self.world.tick;
                }
                state.0 = entity.pos;
            }
            self.motion
                .retain(|id, _| self.world.entities.iter().any(|e| e.id == *id));
            self.play_events(&before_entities);
            let mut notice = None;
            for event in &self.world.events {
                if event.kind == EventKind::CommandRejected && event.player == Some(0) {
                    notice = Some(crate::ux::friendly_error(&event.text));
                }
                if event.kind == EventKind::PathBlocked
                    && event.player == Some(0)
                    && event.entity.is_some_and(|id| self.selected.contains(&id))
                {
                    notice = Some("No way through to that point. The machine stopped.".into());
                }
                if event.kind == EventKind::ResearchCompleted
                    && event.player == Some(0)
                    && let Some(doctrine) = self.world.players[0].doctrine
                {
                    let tier = self.world.players[0].doctrine_tier;
                    let benefit = if tier >= 2 {
                        crate::tactics::doctrine_tier2_benefit(doctrine)
                    } else {
                        crate::tactics::doctrine_benefit(doctrine)
                    };
                    notice = Some(format!(
                        "{}{} complete: {}",
                        crate::tactics::doctrine_name(doctrine),
                        if tier >= 2 { " II" } else { "" },
                        benefit
                    ));
                }
                let source_observed = event.player == Some(0)
                    || event
                        .entity
                        .is_some_and(|id| self.world.entity_visible(0, id))
                    || event.from.is_some_and(|pos| self.world.visible(0, pos));
                let starts_fire = event.kind == EventKind::ArtilleryWarning
                    || event.kind == EventKind::Shot
                        && event
                            .entity
                            .and_then(|id| before_entities.get(&id))
                            .is_none_or(|(kind, _, _)| *kind != Kind::Loom);
                if starts_fire
                    && source_observed
                    && let Some(entity) = event.entity
                {
                    self.fire_starts.insert(entity, self.world.tick);
                }
                if event.kind == EventKind::Deposit
                    && source_observed
                    && let Some(entity) = event.entity
                {
                    self.unload_starts.insert(entity, self.world.tick);
                }
                if event.kind == EventKind::GateChanged {
                    // The foam begins on the authoritative change tick, never
                    // at warning start. Rendering uses world.tick below, so a
                    // paused match holds this phase exactly.
                    self.gate_foam_start = Some(self.world.tick);
                    // An open that moves the dry side turns the lock: the
                    // arm swings over, the wheel turns, one shaft fills and
                    // the other drains.
                    let now = self.world.gate.north_dry();
                    let opened = self
                        .world
                        .map
                        .layout()
                        .arms
                        .iter()
                        .any(|arm| arm.opened == event.text);
                    if opened && rest_north_dry != now {
                        self.gate_switch = Some((self.world.tick, now));
                    }
                    self.tide_change = Some(crate::lane_memory::TideChange::observed(
                        rest_tide.0,
                        rest_tide.1,
                        &self.world.gate,
                        self.world.tick,
                    ));
                }
                if event.kind == EventKind::Laid
                    && let (Some(from), Some(to)) = (event.from, event.to)
                {
                    let ((fx, fy), (tx, ty)) = (from.cell_xy(), to.cell_xy());
                    let (dx, dy) = ((tx - fx).signum(), (ty - fy).signum());
                    for offset in -1..=1 {
                        let cell = (tx + dy.abs() * offset, ty + dx.abs() * offset);
                        self.crust_laid.push((cell, self.world.tick));
                    }
                }
                if event.kind == EventKind::Repair
                    && let Some(target) = event.other
                    && event.to.is_some_and(|p| self.world.visible(0, p))
                {
                    self.repair_marks.record_limited(target, self.world.tick);
                }
                if event.kind == EventKind::Sound
                    && let Some(pos) = event.to
                    && self.world.visible(0, pos)
                {
                    self.pings
                        .push((pos, self.world.tick + u64::from(bw_content::SOUND_TICKS)));
                    if self.pings.len() > 32 {
                        self.pings.remove(0);
                    }
                }
                if event.kind == EventKind::Vent
                    && let Some(id) = event.entity
                    && self.world.entity_visible(0, id)
                {
                    self.vents
                        .insert(id, self.world.tick + u64::from(bw_content::VENT_TICKS));
                }
                if matches!(event.kind, EventKind::Shot | EventKind::Death)
                    && let Some(to) = event.to.or(event.from)
                    && self.world.visible(0, to)
                {
                    let from = event
                        .from
                        .filter(|p| self.world.visible(0, *p))
                        .unwrap_or(to);
                    let target_id = match event.kind {
                        EventKind::Shot => event.other,
                        EventKind::Death => event.entity,
                        _ => None,
                    };
                    let target_owner = target_id
                        .and_then(|id| before_entities.get(&id).copied())
                        .map(|(_, owner, _)| owner)
                        .unwrap_or_else(|| {
                            // Unknown target: with two seats it was the
                            // other one; with more, any seat will do for
                            // the effect's material.
                            event.player.map_or(1, |owner| {
                                if self.world.players.len() == 2 {
                                    1 - owner
                                } else {
                                    owner
                                }
                            })
                        });
                    let target_faction = self.world.players[target_owner as usize].faction;
                    let source_visible = event
                        .from
                        .is_some_and(|source| self.world.visible(0, source));
                    let muzzle = if event.kind == EventKind::Shot && source_visible {
                        event.entity.and_then(|id| {
                            let attacker =
                                self.world.entities.iter().find(|entity| entity.id == id)?;
                            let key = self
                                .entity_animation_key(attacker)
                                .unwrap_or_else(|| self.directional_key(attacker));
                            self.atlas
                                .as_ref()
                                .and_then(|atlas| atlas.sprites.get(&key))
                                .and_then(|sprite| sprite.muzzle)
                        })
                    } else {
                        None
                    };
                    let beam = (event.kind == EventKind::Shot
                        && event
                            .entity
                            .and_then(|id| before_entities.get(&id))
                            .is_some_and(|(kind, _, _)| *kind == Kind::Heliostat))
                    .then_some(event.amount);
                    self.effects.push(Effect {
                        from,
                        to,
                        death: event.kind == EventKind::Death,
                        material: material_for_faction(target_faction),
                        faction: target_faction,
                        muzzle,
                        beam,
                        until: self.world.tick
                            + if event.kind == EventKind::Death {
                                presentation::WRECK_TICKS
                            } else {
                                presentation::IMPACT_TICKS
                            },
                    });
                }
            }
            // The tide is public, so its cues play wherever the camera is.
            for cue in self.tide_cues.observe(&self.world) {
                if let Some(audio) = &self.audio {
                    audio.play(cue);
                }
            }
            // Machines that sank with their transport were never on the
            // field: the prompt counts them (trial 12).
            if let Some(words) = crate::trial12_orders::riders_lost(&self.world.events) {
                notice = Some(words);
            }
            // A watcher is no seat: a neutral line, not the first seat's.
            if self.playback.is_some() {
                notice = crate::observer::neutral_notice(&self.world);
            }
            if let Some(text) = notice
                && self.world.outcome.is_none()
            {
                // A submitted order can still fail on the authoritative tick.
                // Do not leave a positive field marker next to its error.
                self.ux.markers.clear();
                self.notify(&text);
            }
            // One response for the last applied local command in this tick.
            // A rejection remains audible even after a rapid submission.
            if let Some(event) = self.world.events.iter().rev().find(|event| {
                event.player == Some(0)
                    && matches!(
                        event.kind,
                        EventKind::CommandAccepted | EventKind::CommandRejected
                    )
            }) && let Some(audio) = &self.audio
            {
                // An acceptance right after our own click would be a
                // second beep for one order.
                let just_clicked = self
                    .sound
                    .last_submit
                    .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(200));
                if event.kind == EventKind::CommandRejected {
                    audio.play(Cue::OrderRejected);
                } else if !just_clicked {
                    audio.play(Cue::OrderAccepted);
                }
            }
            if self.world.gate.warning_until.is_some_and(|until| {
                until > self.world.tick && (until - self.world.tick).is_multiple_of(60)
            }) && !self
                .world
                .events
                .iter()
                .any(|event| event.kind == EventKind::GateWarning)
                && let Some(audio) = &self.audio
            {
                audio.play(Cue::GateWarningBell);
            }
            if self.world.outcome.is_none() {
                self.play_harbor(false);
            }
            let tick = self.world.tick;
            self.effects.retain(|effect| {
                effect.until > tick
                    || effect.death
                        && effect.until.saturating_add(presentation::RESIDUE_TICKS) > tick
            });
            self.fire_starts
                .retain(|_, start| tick.saturating_sub(*start) < presentation::FIRE_TICKS);
            self.unload_starts
                .retain(|_, start| tick.saturating_sub(*start) < presentation::UNLOAD_TICKS);
            self.repair_marks.expire(tick);
            self.pings.retain(|(_, until)| *until > tick);
            self.vents.retain(|_, until| *until > tick);
            self.repair_marks
                .retain_ids(|id| self.world.entities.iter().any(|e| e.id == id && e.hp > 0));
            self.update_coast_flush();
            let now = self.world.tick;
            self.crust_laid
                .retain(|(_, tick)| now < tick + u64::from(bw_content::LAY_ROW_TICKS));
            if self.world.crust.is_empty() {
                self.crust_laid.clear();
            }
            if self.effects.len() > 256 {
                self.effects.drain(..self.effects.len() - 256);
            }
            self.selected
                .retain(|id| self.world.entities.iter().any(|e| e.id == *id && e.hp > 0));
            if self.world.outcome.is_some() && !self.replay_saved {
                match self.export_final_replay() {
                    Ok(()) => self.replay_saved = true,
                    Err(e) => self.notify(&format!("REPLAY SAVE FAILED: {e}")),
                }
            }
        }
    }
    /// Coast sites react to machines: the first arrival flushes the birds or
    /// hides the crab; the site stays empty until it has been clear for a
    /// while. Presentation state only, never saved.
    fn update_coast_flush(&mut self) {
        let tick = self.world.tick;
        for (index, site) in crate::artistry::COAST_SITES.iter().enumerate() {
            let occupied =
                crate::artistry::coast_is_occupied(&self.world, Pos::cell(site.pos.0, site.pos.1));
            match self.coast_flush.get(&index).copied() {
                None if occupied => {
                    self.coast_flush.insert(index, (tick, None));
                }
                Some((flushed, None)) if !occupied => {
                    self.coast_flush.insert(index, (flushed, Some(tick)));
                }
                Some((flushed, Some(_))) if occupied => {
                    self.coast_flush.insert(index, (flushed, None));
                }
                Some((_, Some(cleared)))
                    if tick.saturating_sub(cleared) >= crate::artistry::COAST_RETURN_TICKS =>
                {
                    self.coast_flush.remove(&index);
                }
                _ => {}
            }
        }
    }
    fn play_harbor(&self, aftermath: bool) {
        let Some(audio) = &self.audio else { return };
        for voice in
            crate::audio_scene::scheduled(&self.world, self.camera, self.visual_tick(), aftermath)
        {
            let cue = crate::audio_scene::harbor_cue(voice.sound);
            audio.play_spatial(cue, voice.pan, voice.gain);
        }
    }
    fn visual_tick(&self) -> u64 {
        self.world.tick.saturating_add(self.aftermath_ticks)
    }
    pub(crate) fn issue(&mut self, c: Command) {
        self.issue_accepted(c);
    }
    /// Send an order for the local side; true when it was accepted, which
    /// in a two-player match means validated and queued for the network.
    pub(crate) fn issue_accepted(&mut self, c: Command) -> bool {
        if self.world.outcome.is_some() || self.ux.practice_review {
            return false;
        }
        // A queued order leaves capturers to their claim (trial 12).
        let Some((c, capture_kept)) = self.keep_capture_on_queue(c) else {
            self.notify(crate::trial12_orders::CAPTURE_KEPT.trim_start());
            return false;
        };
        let capture_dropped = self.drops_capture(&c);
        let (message, marker) = match &c {
            Command::Move {
                target,
                queued,
                units,
            } => (
                format!(
                    "{}{}",
                    if *queued {
                        "Move waypoint queued."
                    } else {
                        "Move order sent."
                    },
                    self.deployed_stay_note(units)
                ),
                Some((*target, "Move")),
            ),
            Command::AttackMove {
                target,
                queued,
                units,
            } => (
                format!(
                    "{}{}",
                    if *queued {
                        "Attack waypoint queued."
                    } else {
                        "Attack-move order sent."
                    },
                    self.deployed_stay_note(units)
                ),
                Some((*target, "Attack")),
            ),
            Command::Attack { target, units } => (
                format!(
                    "Attack order sent.{}",
                    self.attack_pack_note(units, *target)
                ),
                self.world
                    .entities
                    .iter()
                    .find(|e| e.id == *target)
                    .map(|e| (e.pos, "Attack")),
            ),
            Command::Gather { resource, .. } => (
                self.gather_words(*resource),
                self.world
                    .map
                    .resources
                    .iter()
                    .find(|r| r.id == *resource)
                    .map(|r| (r.pos, "Gather")),
            ),
            Command::QueueGather { resource, units } => (
                if units.iter().any(|id| self.building_now(*id)) {
                    "Gather queued: after the site.".into()
                } else {
                    self.gather_words(*resource)
                },
                self.world
                    .map
                    .resources
                    .iter()
                    .find(|r| r.id == *resource)
                    .map(|r| (r.pos, "Gather")),
            ),
            Command::Build { kind, pos, .. } => (
                format!(
                    "{} construction ordered.",
                    crate::ux::building_name(*kind, self.faction)
                ),
                Some((*pos, "Build")),
            ),
            Command::Train { kind, .. } => (format!("{} queued.", kind.name()), None),
            Command::Capture { units } => (
                format!(
                    "Capture order sent: to the sluice.{}",
                    self.capture_pack_note(units)
                ),
                Some((self.world.map.gate_pos, "Capture")),
            ),
            Command::SwitchGate => (
                format!(
                    "Sluice switch ordered for {} pressure. Ten seconds to the change.",
                    bw_content::SWITCH_PRESSURE
                ),
                None,
            ),
            Command::SetTide { arm } => (
                crate::tide_cues::set_tide_sentence(&self.world, *arm, bw_content::SWITCH_PRESSURE),
                None,
            ),
            Command::Board { transport, .. } => (
                "Board order sent: they walk to the transport and climb aboard.".into(),
                self.world
                    .entities
                    .iter()
                    .find(|e| e.id == *transport)
                    .map(|e| (e.pos, "Board")),
            ),
            Command::Unload { .. } => ("Unload order sent.".into(), None),
            Command::Flood => (
                format!(
                    "FLOOD in five seconds for {} pressure: every crossing deep for {}s.",
                    bw_content::FLOOD_PRESSURE,
                    bw_content::FLOOD_TICKS / 30
                ),
                None,
            ),
            // Ordered, not started: the world takes it up a tick later and
            // may still refuse it (trial 12: "OVERHAUL II started." then a
            // refusal).
            Command::Upgrade { upgrade, .. } => (format!("{} ordered.", upgrade.name()), None),
            Command::CancelUpgrade { .. } => ("Upgrade cancelled: 75% refund.".into(), None),
            Command::Vent { .. } => (
                "VENT opened: combat machines fire faster for 10s.".into(),
                None,
            ),
            Command::Recycle { units } => (
                format!(
                    "RECYCLE: {} worker{} to the nearest yard, {}% of the salvage back and the crew freed.",
                    units.len(),
                    if units.len() == 1 { "" } else { "s" },
                    bw_content::RECYCLE_REFUND_PERCENT
                ),
                None,
            ),
            Command::Deliver { target, units } => (
                format!(
                    "DELIVER: {} load{} in, then back to the wreck.",
                    units.len(),
                    if units.len() == 1 { "" } else { "s" },
                ),
                self.world
                    .entities
                    .iter()
                    .find(|e| e.id == *target)
                    .map(|e| (e.pos, "Deliver")),
            ),
            Command::Reclaim { .. } => (
                format!(
                    "RECLAIM: {} pressure melted into {} salvage.",
                    bw_content::RECLAIM_PRESSURE,
                    bw_content::RECLAIM_SALVAGE
                ),
                None,
            ),
            Command::Glint { target, .. } => (
                "GLINT: a ray of sight lit for 5s.".into(),
                Some((*target, "Glint")),
            ),
            Command::Lay { target, .. } => (
                "LAY: the Salter crusts a causeway until the tide moves.".into(),
                Some((*target, "Lay")),
            ),
            Command::Sound { unit } => (
                "SOUND: the ground around it is lit for 5s.".into(),
                self.world
                    .entities
                    .iter()
                    .find(|e| e.id == *unit)
                    .map(|e| (e.pos, "Sound")),
            ),
            Command::Rally { building, pos } => {
                (self.rally_words(*building, *pos), Some((*pos, "Rally")))
            }
            Command::Deploy { units } => (
                if self.deploy_packs_units(units) {
                    "Pack order sent.".into()
                } else {
                    "Deploy order sent.".into()
                },
                None,
            ),
            Command::Face { target, .. } => ("Facing order sent.".into(), Some((*target, "Face"))),
            Command::SetFormation { formation, .. } => (
                format!(
                    "{} formation: applies to your next move.",
                    crate::tactics::formation_name(*formation)
                ),
                None,
            ),
            Command::Surge { .. } => (
                "Surge ordered: faster movement, weapons off for 3s.".into(),
                None,
            ),
            Command::Research { doctrine, .. } => (
                format!(
                    "{} research ordered. HQ worker training pauses for {}s.",
                    crate::tactics::doctrine_name(*doctrine),
                    if self.world.players[0].doctrine_tier >= 1 {
                        bw_content::DOCTRINE_TIER2_TICKS / 30
                    } else {
                        bw_content::DOCTRINE_TICKS / 30
                    }
                ),
                None,
            ),
            Command::CancelResearch { .. } => {
                ("Research cancellation ordered: 75% refund.".into(), None)
            }
            Command::SetDeployed { deployed, .. } => (
                if *deployed {
                    "Deploy order sent.".into()
                } else {
                    "Pack order sent.".into()
                },
                None,
            ),
            Command::KeepDeployed { keep, .. } => (
                if *keep {
                    "Kept deployed: group moves leave them in place. P packs; K lets them go."
                        .into()
                } else {
                    "No longer kept deployed: a move packs them first.".into()
                },
                None,
            ),
            Command::Stop { .. } => ("Machines stopped.".into(), None),
            Command::Hold { .. } => (
                "Holding position. A move under way is cancelled.".into(),
                None,
            ),
            Command::Cancel { .. } => ("Cancellation ordered.".into(), None),
            Command::Repair { target, .. } => (
                if self
                    .world
                    .entities
                    .iter()
                    .any(|e| e.id == *target && e.build_remaining > 0)
                {
                    "Construction resumed.".into()
                } else {
                    "Repair order sent.".into()
                },
                None,
            ),
            _ => (String::new(), None),
        };
        if self.playback.is_some() {
            self.notify("Observing: orders are off.");
            return false;
        }
        if self.knocked_out() {
            self.notify("You are out: watching the rest.");
            return false;
        }
        // An order no selected machine could carry out is refused now and
        // says which lane blocks it, not accepted to fail later (trial 12).
        if let Some(why) = self.unreachable_order(&c) {
            self.notify(&why);
            return false;
        }
        let message = if capture_dropped && !message.is_empty() {
            format!("{message}{}", crate::trial12_orders::CAPTURE_DROPPED)
        } else if capture_kept {
            format!("{message}{}", crate::trial12_orders::CAPTURE_KEPT)
        } else {
            message
        };
        let issued = match self.session.as_mut() {
            Some(session) => session.issue(c),
            None => self.world.issue(0, c),
        };
        if let Err(e) = issued {
            let reason = self.rejection_reason(&e);
            // The dock line names the refusal, not the order it refused: it
            // once read "REJECTED / ATTACK-MOVE ORDER SENT".
            self.ux.on_command_rejected(
                self.world.tick,
                None,
                reason.clone(),
                reason.clone(),
                marker.map(|(pos, _)| pos),
            );
            self.notify(&reason);
            if let Some(audio) = &self.audio {
                audio.play(Cue::OrderRejected);
            }
            false
        } else {
            if let Some(sequence) = self.world.command_log.last().map(|record| record.sequence) {
                self.ux.on_command_submitted(
                    self.world.tick,
                    sequence,
                    message.clone(),
                    marker.map(|(pos, _)| pos),
                );
            }
            self.mode = Mode::Context;
            self.message.clear();
            if !message.is_empty() {
                self.notify(&message);
            }
            if let Some((pos, label)) = marker {
                self.mark_order(pos, label);
            }
            if let Some(audio) = &self.audio {
                audio.play(Cue::OrderSubmit);
            }
            self.sound.last_submit = Some(std::time::Instant::now());
            true
        }
    }
    pub(crate) fn first_worker(&self) -> Option<EntityId> {
        self.selected
            .iter()
            .find(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == **id && e.owner == 0 && e.kind.is_worker())
            })
            .copied()
    }
    /// The selected worker for a new site: one without a build of its own,
    /// given or pending, nearest to the site; two sites from one selection
    /// get two builders.
    pub(crate) fn builder_for(&self, site: Pos) -> Option<EntityId> {
        let tick = self.world.tick;
        let workers: Vec<&bw_sim::Entity> = self
            .selected
            .iter()
            .filter_map(|id| self.world.entities.iter().find(|e| e.id == *id))
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .collect();
        let free = |e: &bw_sim::Entity| {
            !matches!(e.order, Order::Build { .. })
                && !self
                    .recent_builders
                    .iter()
                    .any(|(id, at)| *id == e.id && tick.saturating_sub(*at) < 90)
        };
        let pick = |candidates: Vec<&bw_sim::Entity>| {
            candidates
                .into_iter()
                .min_by_key(|e| e.pos.distance_sq(site))
                .map(|e| e.id)
        };
        pick(workers.iter().copied().filter(|e| free(e)).collect())
            .or_else(|| pick(workers.clone()))
            .or_else(|| self.first_worker())
    }
    /// A wreck with salvage within `cells` of `pos`: a rally there gathers.
    /// What a rally does, said before it costs anything: a headquarters
    /// sends new workers, and a rally on a wreck in the lake, on the sluice
    /// island or across it walks them where the enemy fights.  A Works or
    /// Drydock sends new machines.
    pub(crate) fn rally_words(&self, building: EntityId, pos: Pos) -> String {
        let producer = self
            .world
            .entities
            .iter()
            .find(|e| e.id == building)
            .map(|e| e.kind);
        if producer != Some(Kind::Headquarters) {
            return "Rally point set. New machines move here.".into();
        }
        if !self.wreck_within(pos, 2) {
            return "Rally point set. New workers move here.".into();
        }
        let (x, y) = pos.cell_xy();
        let gate = self.world.map.gate_pos.cell_xy().0;
        let home = self
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map_or(gate, |e| e.pos.cell_xy().0);
        let across = (x - gate).signum() != (home - gate).signum() && (x - gate).abs() > 3;
        if self.world.in_worker_danger(0, pos) {
            "Rally on a wreck under fire: new workers take the nearest safe wreck until it is quiet."
                .into()
        } else if self.world.map.island_cell(x, y) {
            "Warning: rally on the sluice island. New workers walk out into the lake to gather there."
                .into()
        } else if self.world.map.terrain(x, y).is_tidal() {
            "Warning: rally on a lane wreck. New workers walk into the crossing to gather there."
                .into()
        } else if across {
            "Warning: rally across the lake. New workers cross to the enemy's bank to gather there."
                .into()
        } else {
            "Rally point set on a wreck: new workers gather there.".into()
        }
    }
    /// Where a gather order's loads go: the finished headquarters or yard
    /// nearest the wreck, as the simulation picks it.  The words once
    /// always said headquarters, beside a yard built for that wreck.
    pub(crate) fn gather_words(&self, resource: u32) -> String {
        let Some(wreck) = self
            .world
            .map
            .resources
            .iter()
            .find(|r| r.id == resource)
            .map(|r| r.pos)
        else {
            return "Gather order sent.".into();
        };
        let drop = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.build_remaining == 0
                    && matches!(e.kind, Kind::Headquarters | Kind::Dropoff)
            })
            .min_by_key(|e| (e.pos.distance_sq(wreck), e.id))
            .map(|e| e.kind);
        match drop {
            Some(Kind::Dropoff) => {
                "Gather order sent. Loads go to the nearest salvage yard.".into()
            }
            Some(_) => "Gather order sent. Loads return to headquarters.".into(),
            None => "Gather order sent. No headquarters or yard takes the loads.".into(),
        }
    }
    fn wreck_within(&self, pos: Pos, cells: i32) -> bool {
        let limit = i64::from(bw_core::FP) * i64::from(cells);
        self.world.map.resources.iter().any(|r| {
            r.remaining > 0
                && (i64::from(r.pos.x) - i64::from(pos.x)).abs() <= limit
                && (i64::from(r.pos.y) - i64::from(pos.y)).abs() <= limit
        })
    }
    /// Whether a Deploy order for `units` packs them: all are deployed.
    fn deploy_packs_units(&self, units: &[EntityId]) -> bool {
        !units.is_empty()
            && units.iter().all(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && (e.deployed || e.deploy_remaining > 0))
            })
    }
    /// Whether D packs: every selected specialist is deployed.
    pub(crate) fn deploy_packs(&self) -> bool {
        let all = self.gameplay_ids(crate::controls::is_specialist);
        !all.is_empty()
            && all.iter().all(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && (e.deployed || e.deploy_remaining > 0))
            })
    }
    pub(crate) fn first_building(&self) -> Option<EntityId> {
        self.selected
            .iter()
            .find(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == **id && e.owner == 0 && e.kind.is_building())
            })
            .copied()
    }
    fn directional_key(&self, entity: &bw_sim::Entity) -> String {
        let key = entity
            .kind
            .asset(self.world.players[entity.owner as usize].faction);
        // Simulation octants start at world north and turn clockwise; atlas
        // facings start at screen east and turn counter-clockwise.
        format!("{key}_{}", (9 - entity.facing % 8) % 8)
    }
    fn walking_phase(&self, entity: &bw_sim::Entity) -> Option<u8> {
        self.motion
            .get(&entity.id)
            .filter(|(_, distance, tick)| {
                *distance > 0 && self.world.tick.saturating_sub(*tick) <= 2
            })
            .map(|(_, distance, _)| ((distance / (FP as u64 / 4)) % 4) as u8)
    }
    fn entity_animation_key(&self, entity: &bw_sim::Entity) -> Option<String> {
        if entity.kind.is_building() {
            let (sx, sy) = self.camera.project(entity.pos);
            let gust = crate::wind::strength(
                i64::from(sx) + i64::from(self.camera.x),
                i64::from(sy) + i64::from(self.camera.y),
                self.visual_tick(),
            );
            return building_state_key(
                entity,
                self.world.players[entity.owner as usize].faction,
                self.world.tick,
                gust,
            );
        }
        let directional = self.directional_key(entity);
        let role = entity
            .kind
            .asset(self.world.players[entity.owner as usize].faction);
        let face = (9 - entity.facing % 8) % 8;
        let face_key = |suffix: &str| format!("{role}_{face}{suffix}");

        // A transition pose is authoritative for its whole 30-tick window.
        // It must win over a stale Shot event so deployment/packing remains
        // physically legible when both events land close together.
        if let Some(stage) = presentation::deployment_stage(entity)
            && crate::controls::is_specialist(entity.kind)
        {
            return Some(face_key(&format!("_deploy_{stage}")));
        }
        if let Some(key) = crate::artistry::entity_animation_key(&self.world, entity)
            && self
                .atlas
                .as_ref()
                .is_some_and(|atlas| atlas.sprites.contains_key(&key))
        {
            return Some(key);
        }

        // A shot event is the only source for recoil. The simulation's
        // cooldown remains authoritative; this short visual phase never adds a
        // pre-shot promise or changes attack timing.
        if matches!(
            entity.kind,
            Kind::Riveter
                | Kind::Bulwark
                | Kind::Sounder
                | Kind::Skipper
                | Kind::Reedguard
                | Kind::Loom
                | Kind::Brander
                | Kind::Glinter
                | Kind::Heliostat
        ) && let Some(start) = self.fire_starts.get(&entity.id)
            && let Some(phase) = presentation::fire_phase(*start, self.world.tick)
        {
            let suffix = if matches!(entity.kind, Kind::Bulwark | Kind::Loom | Kind::Heliostat)
                && entity.deployed
            {
                format!("_deployed_fire_{phase}")
            } else {
                format!("_fire_{phase}")
            };
            return Some(face_key(&suffix));
        }

        // A deployed Pan boils: four frames on a slow loop.
        if entity.deployed && entity.kind == Kind::Pan {
            let phase = (self.world.tick / 8 + u64::from(entity.id)) % 4;
            return Some(face_key(&format!("_deployed_{phase}")));
        }
        if entity.deployed && crate::controls::is_specialist(entity.kind) {
            return Some(face_key("_deploy_2"));
        }
        // A Salter at the head of its causeway works the salt.
        if let Order::Lay { head, progress, .. } = entity.order
            && entity.pos.cell_xy() == head.cell_xy()
            && entity.path.is_empty()
        {
            return Some(face_key(&format!("_lay_{}", (progress / 10) % 3)));
        }

        if entity.kind.gathers() {
            if let Some(start) = self.unload_starts.get(&entity.id)
                && let Some(phase) = presentation::unload_phase(*start, self.world.tick)
            {
                return Some(face_key(&format!("_unload_{phase}")));
            }
            if entity.carried > 0 {
                if let Some(phase) = self.walking_phase(entity) {
                    return Some(face_key(&format!("_loaded_walk_{phase}")));
                }
                return Some(face_key("_loaded"));
            }
            if matches!(entity.order, Order::Gather { .. }) && entity.gather_ticks > 0 {
                return Some(face_key(&format!(
                    "_gather_{}",
                    (entity.gather_ticks / 10) % 3
                )));
            }
        }

        if let Some(phase) = self.walking_phase(entity) {
            return Some(format!("{directional}_walk_{phase}"));
        }
        // Idle acting means idle: only a machine with no order and nothing in
        // hand plays its strip, on a period keyed by its id so a base does
        // not fidget in unison. The IDLE readout counts the same state.
        let resting = matches!(entity.order, Order::Idle)
            || (matches!(entity.order, Order::Hold) && !entity.kind.is_worker());
        if resting && !entity.deployed && entity.carried == 0 {
            let period = 72 + u64::from(entity.id % 5) * 12;
            let phase = (self.world.tick + u64::from(entity.id) * 17) % period;
            if phase < 6 {
                return Some(face_key("_idle_0"));
            } else if phase < 12 {
                return Some(face_key("_idle_1"));
            }
        }
        None
    }
    pub(crate) fn unit_at(&self, x: i32, y: i32) -> Option<EntityId> {
        self.pick_at(x, y, false)
    }
    /// The building under the pointer, ignoring machines standing on it.
    pub(crate) fn building_at(&self, x: i32, y: i32) -> Option<EntityId> {
        self.pick_at(x, y, true)
    }
    fn pick_at(&self, x: i32, y: i32, buildings_only: bool) -> Option<EntityId> {
        let screen = (x, y);
        let (camera, (x, y)) = self.native_pointer(x, y);
        let entities: Vec<_> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.hp > 0
                    && e.aboard.is_none()
                    && (e.owner == 0 || self.world.entity_visible(0, e.id))
            })
            .collect();
        // Topmost first. Where a machine and a building both lie under the
        // cursor, the machine wins: a machine behind a building shows its
        // ghost there and must stay targetable. A miss retries a few pixels
        // around the point for machines only, so a moving machine that has
        // stepped since the picture was read is still caught.
        let order = crate::occlusion::stable_entity_order(&entities);
        // The rings widen to eleven pixels: a worker read off a picture a
        // beat ago has walked that far by the time the click lands.
        for (dx, dy) in [
            (0, 0),
            (-3, 0),
            (3, 0),
            (0, -3),
            (0, 3),
            (-3, -3),
            (3, 3),
            (-3, 3),
            (3, -3),
            (-7, 0),
            (7, 0),
            (0, -7),
            (0, 7),
            (-6, -6),
            (6, 6),
            (-6, 6),
            (6, -6),
            (-11, 0),
            (11, 0),
            (0, -11),
            (0, 11),
        ] {
            let (x, y) = (x + dx, y + dy);
            let exact = dx == 0 && dy == 0;
            // Each hit carries whether it claims the point outright: a
            // building's own site label, or the core of a machine's body.
            let hits: Vec<(&bw_sim::Entity, bool)> = order
                .iter()
                .rev()
                .filter_map(|&i| {
                    let e = entities[i];
                    if !exact && e.kind.is_building() {
                        return None;
                    }
                    if buildings_only && !e.kind.is_building() {
                        return None;
                    }
                    let (sx, sy, scale) = entity_render_position(
                        camera,
                        e,
                        self.world.players[e.owner as usize].faction,
                    );
                    let building = e.kind.is_building();
                    let rx = if building { 36 } else { 17 };
                    let ht = if building { 56 } else { 34 };
                    let key = e.kind.asset(self.world.players[e.owner as usize].faction);
                    let directional = self.directional_key(e);
                    let state_key = self.entity_animation_key(e);
                    // A machine without a drawing yet is picked by its box,
                    // as it is drawn by one.
                    let hit = match &self.atlas {
                        Some(atlas) => {
                            let chosen = if let Some(state) = state_key.as_deref()
                                && atlas.sprites.contains_key(state)
                            {
                                state
                            } else if atlas.sprites.contains_key(&directional) {
                                &directional
                            } else {
                                key
                            };
                            if atlas.sprites.contains_key(chosen) {
                                atlas.hit_scaled(chosen, x - sx, y - sy, scale)
                            } else {
                                (x - sx).abs() <= rx && y >= sy - ht && y <= sy + 8
                            }
                        }
                        None => (x - sx).abs() <= rx && y >= sy - ht && y <= sy + 8,
                    };
                    // The band over an own site (its bar and its NO
                    // BUILDER badge) is part of it, so a right-click there
                    // resumes the site.
                    let label_top = sy - scale.apply(63) - 18;
                    let on_label = building
                        && e.owner == 0
                        && e.build_remaining > 0
                        && (x - sx).abs() <= 28
                        && y >= label_top
                        && y <= label_top + 22;
                    let core = !building
                        && (x - sx).abs() <= scale.apply(7)
                        && y >= sy - scale.apply(26)
                        && y <= sy - scale.apply(4);
                    if hit || on_label {
                        Some((e, on_label || core))
                    } else {
                        None
                    }
                })
                .collect();
            // A machine drawn in front of a building wins.  A machine behind
            // one keeps the click only on the core of its ghost, so the rest
            // of the building's face picks the building: in the eighth trial
            // clicks on a Works picked machines parked behind it.
            let found = match hits.first() {
                Some((top, _)) if !top.kind.is_building() => Some(*top),
                Some((top, true)) => Some(*top),
                // A second click on a building that is already the whole
                // selection takes a machine under it (ninth trial: workers
                // stacked on the headquarters could not be clicked).
                Some((top, false)) if self.selected == [top.id] => hits
                    .iter()
                    .find(|(e, _)| !e.kind.is_building())
                    .map(|(e, _)| *e)
                    .or(Some(*top)),
                Some((top, false)) => hits
                    .iter()
                    .find(|(e, core)| !e.kind.is_building() && *core)
                    .map(|(e, _)| *e)
                    .or(Some(*top)),
                None => None,
            };
            if let Some(found) = found {
                return Some(found.id);
            }
        }
        // Trial 11: a moving machine, where the last picture drew it.
        (!buildings_only)
            .then(|| self.pick_moving(screen.0, screen.1))
            .flatten()
    }
    /// The footprint origin for a building placed under the pointer: the
    /// footprint is centred on the pointed cell, so the ghost sits under the
    /// cursor rather than hanging below and to the right of it.
    pub(crate) fn build_origin(&self, kind: Kind, pointed: Pos) -> Pos {
        let n = spec(kind).footprint;
        let (cx, cy) = pointed.cell_xy();
        if kind == Kind::Condenser {
            // A click on the pump's picture means its well (trial 10: the
            // pump art stands a cell from the site, and a click on it did
            // nothing).
            if let Some(well) = self.well_art_under_pointer() {
                return well;
            }
            // A condenser only stands on a well: the footprint snaps to one
            // within two cells of the pointer.  Wells are chart knowledge.
            if let Some(well) = self
                .world
                .map
                .wells
                .iter()
                .filter(|well| {
                    let (wx, wy) = well.cell_xy();
                    (wx - cx).abs().max((wy - cy).abs()) <= 2
                })
                .min_by_key(|well| (well.distance_sq(pointed), well.x, well.y))
            {
                return *well;
            }
        }
        Pos::cell(cx - n / 2, cy - n / 2)
    }
    /// The well whose pump the pointer is over, by its picture (or, with
    /// no pictures loaded, the box it would stand in).
    pub(crate) fn well_art_under_pointer(&self) -> Option<Pos> {
        let (camera, (x, y)) = self.native_pointer(self.cursor.0, self.cursor.1);
        self.world.map.wells.iter().copied().find(|well| {
            let (px, py) = camera.project(*well);
            let (dx, dy) = (x - px, y - py);
            self.atlas
                .as_ref()
                .filter(|atlas| atlas.sprites.contains_key("well"))
                .map_or(
                    (-14..=14).contains(&dx) && (-44..=8).contains(&dy),
                    |atlas| atlas.hit("well", dx, dy),
                )
        })
    }
    pub fn pan(&mut self, x: i32, y: i32) {
        if self.observing() && (x, y) != (0, 0) {
            self.spectator.follow = false;
        }
        let s = self.zoom.scale();
        self.move_camera(f64::from(x) / s, f64::from(y) / s);
    }
    /// Move the camera by a distance in texels, keeping the part of a texel
    /// for the next frame.
    pub(crate) fn move_camera(&mut self, dx: f64, dy: f64) {
        let axis = |whole: i32, sub: f64, d: f64, lo: i32, hi: i32| {
            let v = (f64::from(whole) + sub + d).clamp(f64::from(lo), f64::from(hi));
            // Snap float noise so a whole step keeps its exact blocks.
            let v = if (v - v.round()).abs() < 1e-7 {
                v.round()
            } else {
                v
            };
            (v.floor() as i32, v - v.floor())
        };
        let [left, right, top, bottom] = camera_limits(self.world.map.width);
        let (x, sx) = axis(self.camera.x, self.camera_sub.0, dx, left, right);
        let (y, sy) = axis(self.camera.y, self.camera_sub.1, dy, top, bottom);
        self.camera = Camera { x, y };
        self.camera_sub = (sx, sy);
    }
    pub fn project(&self, pos: Pos) -> (i32, i32) {
        if self.drawing_scene {
            self.camera.project(pos)
        } else {
            self.world_view().project(self.camera, pos)
        }
    }
    pub fn unproject(&self, x: i32, y: i32) -> Pos {
        if self.drawing_scene {
            self.camera.unproject(x, y)
        } else {
            self.world_view().unproject(self.camera, (x, y))
        }
    }
    pub(crate) fn native_pointer(&self, x: i32, y: i32) -> (Camera, (i32, i32)) {
        if self.drawing_scene {
            (self.camera, (x, y))
        } else {
            (
                self.world_view().scene_camera(self.camera),
                self.world_view().source((x, y)),
            )
        }
    }
    /// Change the scale at once, keeping the world under `anchor` where it
    /// is. At a whole step the texel under the anchor stays under it.
    pub fn set_zoom(&mut self, zoom: crate::zoom::Zoom, anchor: (i32, i32)) {
        if zoom == self.zoom {
            return;
        }
        let view = self.world_view();
        let (cx, cy) = view.center();
        let (old, new) = (self.zoom.scale(), zoom.scale());
        let ax = f64::from(anchor.0) + 0.5 - f64::from(cx);
        let ay = f64::from(anchor.1) + 0.5 - f64::from(cy);
        // The world texel coordinate under the anchor, before and after.
        let wx = f64::from(self.camera.x) + self.camera_sub.0 + 320.0 + ax / old;
        let wy = f64::from(self.camera.y) + self.camera_sub.1 + 156.0 + ay / old;
        let mut x = wx - 320.0 - ax / new;
        let mut y = wy - 156.0 - ay / new;
        if let Some(k) = zoom.whole() {
            // Put texel edges on window pixels, choosing among the k
            // placements the one nearest that keeps the anchored texel.
            let k = f64::from(k);
            let fit = |c: f64, w: f64, a: f64, mid: f64| {
                let lo = (k * (w.floor() - mid) - a).ceil();
                ((c * k).round().clamp(lo, lo + k - 1.0)) / k
            };
            x = fit(x, wx, ax, 320.0);
            y = fit(y, wy, ay, 156.0);
        }
        self.zoom = zoom;
        self.camera_sub = (0.0, 0.0);
        self.camera = Camera { x: 0, y: 0 };
        self.move_camera(x, y);
    }
    /// Step to the next whole scale, eased. The +/- keys and buttons use it.
    pub fn change_zoom(&mut self, closer: bool, anchor: (i32, i32)) {
        if self.screen != Screen::Match || self.world.outcome.is_some() || self.ux.practice_review {
            return;
        }
        let from = self.zoom_goal.map_or(self.zoom, |g| g.zoom);
        let next = if closer { from.closer() } else { from.out() };
        if next == from {
            self.notify(if closer {
                "Nearest view already."
            } else {
                "Widest view already."
            });
            return;
        }
        self.aim_zoom(next, anchor, false);
    }
    /// Scroll the scale continuously; `notches` of 1.0 is one wheel click.
    pub fn wheel_zoom(&mut self, notches: f64, anchor: (i32, i32)) {
        if self.screen != Screen::Match
            || self.world.outcome.is_some()
            || self.ux.practice_review
            || !notches.is_finite()
        {
            return;
        }
        let from = self.zoom_goal.map_or(self.zoom, |g| g.zoom);
        let next = crate::zoom::Zoom::new(from.scale() * crate::zoom::WHEEL_RATIO.powf(notches));
        if next != from {
            self.aim_zoom(next, anchor, true);
        }
    }
    fn aim_zoom(&mut self, zoom: crate::zoom::Zoom, anchor: (i32, i32), wheel: bool) {
        let now = std::time::Instant::now();
        self.zoom_goal = Some(crate::zoom::ZoomGoal {
            zoom,
            anchor,
            wheel,
            since: now,
            last_frame: self.zoom_goal.map_or(now, |g| g.last_frame),
        });
    }
    /// Move the eased zoom on by the time since the last frame. A frame
    /// drawn long after the input (a capture or an agent's frame) lands.
    pub(crate) fn advance_zoom(&mut self) {
        let Some(goal) = self.zoom_goal.as_mut() else {
            return;
        };
        let now = std::time::Instant::now();
        let dt = now.duration_since(goal.last_frame).as_secs_f64();
        goal.last_frame = now;
        if goal.wheel && now.duration_since(goal.since).as_secs_f64() > 0.18 {
            // A wheel that has come to rest settles on a whole step if one
            // is close, so the resting picture keeps its exact blocks.
            if let Some(step) = goal.zoom.nearby_step(0.08) {
                goal.zoom = step;
            }
            goal.wheel = false;
        }
        self.advance_zoom_by(dt);
    }
    fn advance_zoom_by(&mut self, dt: f64) {
        let Some(goal) = self.zoom_goal else {
            return;
        };
        let (from, to) = (self.zoom.scale().ln(), goal.zoom.scale().ln());
        if dt <= 0.25 && (to - from).abs() > 0.002 {
            let eased = from + (to - from) * (1.0 - (-dt * 14.0).exp());
            self.set_zoom(crate::zoom::Zoom::new(eased.exp()), goal.anchor);
            return;
        }
        self.set_zoom(goal.zoom, goal.anchor);
        if goal.wheel {
            // Landed while the wheel may still turn; the rest check above
            // may yet move it to a whole step.
            return;
        }
        self.zoom_goal = None;
        self.ux.preferences.world_zoom = self.zoom.scale() as f32;
        let _ = self.ux.persist(&self.data_dir);
    }
    /// Finish any eased zoom now.
    pub fn settle_zoom(&mut self) {
        if let Some(goal) = self.zoom_goal.as_mut() {
            goal.wheel = false;
            self.advance_zoom_by(1.0);
        }
    }
    fn draw_zoom_controls(&mut self) {
        if self.world.outcome.is_some() || self.ux.practice_review {
            return;
        }
        for (x, w, label, action, enabled) in [
            (
                438,
                24,
                "-".to_string(),
                Action::ZoomOut,
                self.zoom_goal.map_or(self.zoom, |g| g.zoom) != crate::zoom::Zoom::Overview,
            ),
            (
                464,
                46,
                format!("{}%", self.zoom.percent()),
                Action::ZoomReset,
                true,
            ),
            (
                512,
                24,
                "+".to_string(),
                Action::ZoomIn,
                self.zoom_goal.map_or(self.zoom, |g| g.zoom) != crate::zoom::Zoom::Detail,
            ),
        ] {
            self.buttons.push(Button {
                x,
                y: 28,
                w,
                h: 19,
                label,
                hint: String::new(),
                action,
                enabled,
            });
        }
    }
    pub(crate) fn draw_zoomed_world(&mut self) {
        let camera = self.camera;
        let cursor = self.cursor;
        let view = self.world_view();
        self.scene_pointer_allowed = self.world_pointer_allowed(cursor.0, cursor.1);
        // A machine under the pointer answers before the ground it stands
        // on (trial 11: a violet walker showed the mouth ring's card).
        let machine = (self.scene_pointer_allowed && self.mode == Mode::Context)
            .then(|| self.unit_at(cursor.0, cursor.1))
            .flatten()
            .filter(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && !e.kind.is_building())
            });
        self.field_tips.clear();
        self.placement_tip = None;
        let (w, h) = view.size();
        if (self.scene_canvas.width(), self.scene_canvas.height()) != (w, h) {
            self.scene_canvas.resize(w, h);
        }
        std::mem::swap(&mut self.canvas, &mut self.scene_canvas);
        self.camera = view.scene_camera(camera);
        self.cursor = view.source(cursor);
        self.drawing_scene = true;
        self.draw_world();
        // The trailer's footage shows the field without the orders drawn
        // on it.
        if !self.cinematic {
            self.draw_tactical_overlays();
            self.draw_reach_rings();
            self.draw_reach_misses();
            self.draw_stop_rings();
            self.draw_order_markers();
            self.draw_cargo_pips();
        }
        // Words on the field are asked for: the tip of the thing under the
        // pointer, or a placement's refusal at its site.
        let asked = if self.scene_pointer_allowed && self.drag.is_none() {
            match machine {
                Some(id) => crate::trial11_words::machine_tip(self, id),
                None => crate::field_labels::pick(&self.field_tips, self.cursor).cloned(),
            }
        } else {
            None
        };
        self.hover_tip = self.placement_tip.take().or(asked).map(|mut tip| {
            tip.anchor = view.screen(tip.anchor);
            tip
        });
        self.drawing_scene = false;
        self.camera = camera;
        self.cursor = cursor;
        std::mem::swap(&mut self.canvas, &mut self.scene_canvas);
        self.enlarge_scene(view);
    }
    /// Enlarge the scene into the window. At a whole step every texel is a
    /// block and nothing is blended; between steps the one window pixel a
    /// texel edge crosses is mixed from its two texels, so every texel
    /// keeps the same size and its edges stay one pixel sharp.
    fn enlarge_scene(&mut self, view: crate::zoom::WorldView) {
        let src_w = self.scene_canvas.width() as usize;
        let src_h = self.scene_canvas.height() as usize;
        let stride = src_w * 4;
        let dest_stride = self.canvas.width() as usize * 4;
        let columns = view.taps(0, view.width, false, src_w);
        let rows = view.taps(view.top, view.bottom, true, src_h);
        let whole = view.zoom.whole() == Some(1) && columns.first().is_some_and(|c| c.0 == 0);
        let mut mixed = vec![0u8; stride];
        let src = &self.scene_canvas.pixels;
        let lerp = |a: u8, b: u8, w: u16| -> u8 {
            ((u32::from(a) * (256 - u32::from(w)) + u32::from(b) * u32::from(w) + 128) >> 8) as u8
        };
        for (i, &(sy, wy)) in rows.iter().enumerate() {
            let y = view.top as usize + i;
            let line: &[u8] = if wy == 0 {
                &src[sy * stride..(sy + 1) * stride]
            } else {
                let below = (sy + 1).min(src_h - 1);
                let (a, b) = (
                    &src[sy * stride..(sy + 1) * stride],
                    &src[below * stride..(below + 1) * stride],
                );
                for ((m, &a), &b) in mixed.iter_mut().zip(a).zip(b) {
                    *m = lerp(a, b, wy);
                }
                &mixed
            };
            let row = &mut self.canvas.pixels[y * dest_stride..(y + 1) * dest_stride];
            if whole {
                row.copy_from_slice(&line[..dest_stride]);
                continue;
            }
            for (pixel, &(sx, wx)) in row.chunks_exact_mut(4).zip(&columns) {
                let from = sx * 4;
                if wx == 0 {
                    pixel.copy_from_slice(&line[from..from + 4]);
                } else {
                    let next = (sx + 1).min(src_w - 1) * 4;
                    for c in 0..4 {
                        pixel[c] = lerp(line[from + c], line[next + c], wx);
                    }
                }
            }
        }
    }
    pub fn left_down(&mut self, x: i32, y: i32) {
        self.cursor = (x, y);
        if self.screen == Screen::Match && self.skip_intro() {
            return;
        }
        self.ux.keyboard_navigation = false;
        self.tooltip_dismissed_at = Some((x, y));
        if self.screen == Screen::Match
            && self.world.outcome.is_none()
            && self.spectator_click(x, y)
        {
            return;
        }
        if self.screen != Screen::Match && self.buttons.is_empty() {
            // A menu just opened has no buttons until it is drawn; lay it
            // out now so a quick click lands on what it will show.
            self.render();
        }
        if let Some(b) = self
            .buttons
            .iter()
            .rev()
            .find(|b| {
                b.contains(x, y) && !(passes_pending_click(&b.action) && self.mode != Mode::Context)
            })
            .cloned()
        {
            self.ux.focused = Some(b.action.clone());
            if overlay_action(&b.action) {
                // Field controls fire on release: a press that turns into a
                // drag is a box over the ground they cover.
                let v = self.world_view();
                self.drag = Some((x, y.clamp(v.top, v.bottom - 1)));
                self.pressed_overlay = Some((b.action.clone(), x, y));
                return;
            }
            // A volume row: a press on its slider sets the level there and
            // starts a drag.
            if let Action::Volume(bus) = b.action
                && b.enabled
            {
                let (tx, w) = volume_track(&b);
                if (tx - 6..=tx + w + 6).contains(&x) {
                    self.ux.volume_drag = Some(bus);
                    self.action(Action::VolumeSet(bus, volume_at(&b, x)));
                }
                return;
            }
            if b.enabled {
                self.action(match b.action {
                    Action::IdleWorker if self.ux.pointer_shift => Action::AllIdleWorkers,
                    Action::RecallGroup(n) if self.ux.pointer_shift => Action::CenterGroup(n),
                    Action::Train(kind) if self.ux.pointer_shift => Action::TrainBatch(kind),
                    Action::FilterSelection(kind, false) if self.ux.pointer_shift => {
                        Action::FilterSelection(kind, true)
                    }
                    action => action,
                });
            } else {
                let reason = self.action_reason(&b.action).unwrap_or_else(|| {
                    if b.hint.is_empty() {
                        "This option is unavailable.".into()
                    } else {
                        b.hint.replace('\n', " ")
                    }
                });
                self.notify(&reason);
            }
            return;
        }
        if self.screen != Screen::Match || self.world.outcome.is_some() || self.ux.practice_review {
            return;
        }
        // While an order waits the card stands aside (`tide_card_shown`),
        // so the ground under it takes the click.
        if self.native_ui() && self.mode == Mode::Context && self.route_bounds().contains(x, y) {
            // The sluice card is a place: a click on it looks at the station.
            // It acts on release, so a press that becomes a drag boxes the
            // ground under the card instead of jumping the camera.
            let v = self.world_view();
            self.drag = Some((x, y.clamp(v.top, v.bottom - 1)));
            self.pressed_overlay = Some((Action::FocusGate, x, y));
            return;
        }
        if self.minimap_bounds().contains(x, y) {
            match self.mode {
                Mode::Context => {
                    self.ux.minimap_drag = true;
                    self.minimap_click(x, y);
                }
                Mode::Attack => self.attack_destination(self.minimap_pos(x, y)),
                _ => self.chart_click_during_order(x, y),
            }
            return;
        }
        if !self.world_pointer_allowed(x, y) {
            // A box may start on the header or the band: its corner is
            // the nearest point of the field.
            if self.mode == Mode::Context && self.native_ui() {
                let v = self.world_view();
                if x >= 0 && x < v.width {
                    self.drag = Some((x, y.clamp(v.top, v.bottom - 1)));
                }
            }
            return;
        }
        match self.mode {
            Mode::Build(kind) => {
                if self.plain_click_ends_shift_placing() {
                    return;
                }
                let target = self.build_origin(kind, self.unproject(x, y));
                if let Some((worker, queued)) = self.builder_and_queue(target) {
                    if let Some(reason) = self.placement_reason(kind, target) {
                        self.notify(&reason);
                        return;
                    }
                    if self.confirm_double_site(kind, target) {
                        return;
                    }
                    self.recent_builders.push((worker, self.world.tick));
                    if self.issue_accepted(Command::Build {
                        worker,
                        kind,
                        pos: target,
                        queued,
                    }) {
                        self.remember_placed_site(kind, target);
                        if queued {
                            self.notify(&format!(
                                "{} queued: the worker builds it after its current site.",
                                kind.name()
                            ));
                        }
                        self.after_placement(kind);
                        self.note_placed(kind);
                    }
                } else {
                    self.notify("SELECT A WORKER FIRST")
                }
            }
            Mode::Attack => {
                if !self.attack_click(x, y) {
                    self.attack_destination(self.unproject(x, y))
                }
            }
            Mode::Face => self.issue(Command::Face {
                units: self.gameplay_ids(|k| !k.is_building() && !k.is_worker()),
                target: self.unproject(x, y),
            }),
            Mode::Glint => {
                if let Some(unit) = self.gameplay_ids(|k| k == Kind::Glinter).first().copied() {
                    self.issue(Command::Glint {
                        unit,
                        target: self.unproject(x, y),
                    });
                }
            }
            Mode::Lay => {
                let target = self.unproject(x, y);
                let salter = self.gameplay_ids(|k| k == Kind::Salter).first().copied();
                let plan = salter
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
                    .map(|e| self.world.lay_plan(e.pos, target));
                match (salter, plan) {
                    (Some(unit), Some(Ok(_))) => self.issue(Command::Lay { unit, target }),
                    (_, Some(Err(reason))) => {
                        // Keep aiming: the next click may find water.
                        let words = crate::trial11_words::lay_refusal(&self.world, target, &reason);
                        self.notify(&words);
                    }
                    _ => {}
                }
            }
            Mode::Gather => {
                if let Some(resource) = self.resource_under_pointer(x, y) {
                    let units = self.gameplay_ids(|k| k.gathers());
                    // Shift keeps a builder on its site (trial 10): the
                    // gather comes after it.
                    self.issue(if self.ux.pointer_shift {
                        Command::QueueGather { units, resource }
                    } else {
                        Command::Gather { units, resource }
                    });
                } else {
                    self.notify("Choose a salvage wreck to gather.");
                }
            }
            Mode::Context => self.drag = Some((x, y)),
        }
    }
    pub fn left_up(&mut self, x: i32, y: i32, shift: bool) {
        if self.ux.volume_drag.is_some() {
            self.end_volume_drag();
            return;
        }
        if self.ux.minimap_drag {
            self.ux.minimap_drag = false;
            return;
        }
        let Some((sx, sy)) = self.drag.take() else {
            return;
        };
        if self.screen != Screen::Match {
            return;
        }
        if let Some((action, px, py)) = self.pressed_overlay.take()
            && (x - px).abs() + (y - py).abs() <= 6
        {
            if action == Action::FocusGate {
                if self.route_bounds().contains(x, y) {
                    self.action(Action::FocusGate);
                }
                return;
            }
            let button = self
                .buttons
                .iter()
                .rev()
                .find(|b| b.action == action && b.contains(x, y))
                .cloned();
            match button {
                Some(b) if b.enabled => {
                    let shift = shift || self.ux.pointer_shift;
                    self.action(match b.action {
                        Action::IdleWorker if shift => Action::AllIdleWorkers,
                        Action::RecallGroup(n) if shift => Action::CenterGroup(n),
                        action => action,
                    });
                }
                Some(b) => {
                    let reason = self.action_reason(&b.action).unwrap_or_else(|| {
                        if b.hint.is_empty() {
                            "This option is unavailable.".into()
                        } else {
                            b.hint.replace('\n', " ")
                        }
                    });
                    self.notify(&reason);
                }
                None => {}
            }
            return;
        }
        if (x - sx).abs() + (y - sy).abs() > 6 {
            self.last_click = None;
            let mut ids: Vec<_> = self
                .world
                .entities
                .iter()
                .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && !e.kind.is_building())
                .filter(|e| self.box_takes(e.pos, (sx, sy), (x, y)))
                .map(|e| e.id)
                .collect();
            if !shift
                && ids.iter().any(|id| {
                    self.world
                        .entities
                        .iter()
                        .any(|e| e.id == *id && !e.kind.is_worker())
                })
            {
                ids.retain(|id| {
                    self.world
                        .entities
                        .iter()
                        .any(|e| e.id == *id && !e.kind.is_worker())
                })
            }
            if !shift {
                self.selected.clear()
            }
            self.selected.extend(ids);
        } else if let Some(id) = self.unit_at(x, y) {
            let e = self
                .world
                .entities
                .iter()
                .find(|e| e.id == id)
                .expect("hit entity exists");
            if e.owner != 0 {
                // Its numbers stand in the selection panel, read only.
                self.inspected = Some((id, self.selected.clone()));
                self.notify(&format!("ENEMY {}", e.kind.name()));
                return;
            }
            let kind = e.kind;
            if self
                .last_click
                .is_some_and(|(last, at)| last == id && self.frame.saturating_sub(at) < 20)
            {
                let matching: Vec<_> = self
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && e.kind == kind)
                    .filter(|e| {
                        let (px, py) = self.project(e.pos);
                        self.world_view().contains(px, py)
                    })
                    .map(|e| e.id)
                    .collect();
                if !shift {
                    self.selected.clear();
                }
                self.selected.extend(matching);
            } else if shift {
                if self.selected.contains(&id) {
                    self.selected.retain(|v| *v != id)
                } else {
                    self.selected.push(id)
                }
            } else {
                self.selected = vec![id]
            }
            self.last_click = Some((id, self.frame));
        } else if !shift {
            self.selected.clear()
        }
        self.selected.sort_unstable();
        self.selected.dedup();
        self.update_tutorial();
    }
    pub fn right_click(&mut self, x: i32, y: i32, shift: bool) {
        if self.screen == Screen::Match && self.skip_intro() {
            return;
        }
        self.cursor = (x, y);
        self.ux.keyboard_navigation = false;
        if self.screen != Screen::Match || self.world.outcome.is_some() || self.ux.practice_review {
            return;
        }
        if self.mode != Mode::Context {
            self.cancel_mode();
            return;
        }
        if self.minimap_bounds().contains(x, y) {
            self.minimap_order(x, y, shift);
            return;
        }
        // The sluice card takes left clicks; a right-click on its body is
        // an order for the ground under it (trial 10).
        self.pointer_through_card = true;
        let order = self.context_order(x, y, shift);
        self.pointer_through_card = false;
        if let Some((command, _, _)) = order {
            // An order nobody could carry out is refused in `issue`.
            if let Command::Rally { pos, .. } = command {
                // Every selected building that trains takes the rally.
                self.rally_selected(pos);
            } else {
                self.issue(command);
            }
        }
    }
    pub(crate) fn minimap_click(&mut self, x: i32, y: i32) {
        self.camera.center(self.minimap_pos(x, y));
    }
    pub fn action(&mut self, action: Action) {
        if self.ux.cached_save_dir != self.data_dir {
            self.ux.refresh_save(&self.data_dir);
        }
        // Menus answer a press: going back falls, starting rises, the
        // rest click.
        if self.screen != Screen::Match
            && let Some(audio) = &self.audio
        {
            audio.play(match action {
                Action::Back | Action::DismissConfirm => Cue::UiBack,
                Action::Start | Action::StartPractice | Action::Confirm => Cue::UiConfirm,
                _ => Cue::UiClick,
            });
        }
        if let Some(reason) = self.action_reason(&action) {
            // A refused placement leaves no placement to cancel: the Escape
            // that follows must not open the menu (trial 10).
            if matches!(action, Action::Build(_)) && self.screen == Screen::Match {
                self.guard_escape();
            }
            self.notify(&reason);
            return;
        }
        use crate::onboarding::TutorialTarget;
        use crate::ux::PendingAction;
        match action {
            Action::FilterSelection(kind, remove) => self.filter_selection(kind, remove),
            Action::RosterPage(next) => {
                self.ux.roster_page = if next {
                    self.ux.roster_page.saturating_add(1)
                } else {
                    self.ux.roster_page.saturating_sub(1)
                };
            }
            Action::RecallGroup(n) => self.control_group(n, false, false),
            Action::CenterGroup(n) => self.control_group(n, true, false),
            Action::ToggleHealthBars => {
                self.ux.preferences.always_health_bars = !self.ux.preferences.always_health_bars;
                let label = if self.ux.preferences.always_health_bars {
                    "Health bars: all visible machines."
                } else {
                    "Health bars: selected and damaged machines."
                };
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Health bars changed for this session; could not save settings.");
                } else {
                    self.notify(label);
                }
            }
            Action::Start => self.request_transition(PendingAction::NewSkirmish),
            Action::StartPractice => self.request_transition(PendingAction::NewPractice),
            Action::Setup => self.open_screen(Screen::Setup),
            Action::Online => self.open_online(),
            Action::Net(action) => self.net_action(action),
            Action::Faction(f) => {
                // An opponent left on "the other side" follows the swap.
                if self.opponent == Some(other_faction(self.faction)) {
                    self.opponent = Some(other_faction(f));
                }
                self.faction = f;
            }
            Action::Opponent(opponent) => self.opponent = opponent,
            Action::AiLevel(level) => self.ai_level = level,
            Action::Map(map) => self.map = map,
            Action::Resume => {
                self.faction = self.world.players[0].faction;
                self.map = self.world.map.id;
                self.open_screen(Screen::Match);
            }
            Action::MainMenu => {
                self.mode = Mode::Context;
                self.open_screen(Screen::Menu);
            }
            Action::Back | Action::DismissConfirm => self.back(),
            Action::Settings => {
                self.ux.settings_return = self.screen;
                self.open_screen(Screen::Settings);
            }
            Action::Help => {
                if self.screen == Screen::Help {
                    self.back();
                } else {
                    // The Guide opens on its first page, never where it was
                    // last left: a reader clicking a tab found a stale page.
                    self.paused_from_help = self.screen;
                    self.ux.help_page = 0;
                    // COMBAT opens on the reader's own specialist.
                    self.ux
                        .manual
                        .set_subject(crate::field_manual::ManualSubject::of(self.faction));
                    self.open_screen(Screen::Help);
                }
            }
            Action::GuidePage(page) => {
                self.ux.help_page = page.min(crate::menus::GUIDE_PAGES - 1);
                self.ux.focused = Some(Action::GuidePage(self.ux.help_page));
            }
            Action::ManualSubject(subject) => self.ux.manual.set_subject(subject),
            Action::ManualPlay => self.ux.manual.toggle_playing(),
            Action::ManualStep => self.ux.manual.step(),
            Action::ZoomIn => self.change_zoom(true, self.world_view().center()),
            Action::ZoomOut => self.change_zoom(false, self.world_view().center()),
            Action::ZoomReset => {
                if self.zoom_goal.map_or(self.zoom, |g| g.zoom) != crate::zoom::Zoom::Wide {
                    self.aim_zoom(crate::zoom::Zoom::Wide, self.world_view().center(), false);
                }
            }
            Action::Mute => {
                self.muted = !self.muted;
                self.ux.preferences.muted = self.muted;
                if let Some(audio) = &self.audio {
                    audio.set_muted(self.muted);
                }
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Sound changed for this session; settings could not be saved.");
                }
            }
            Action::SettingsTab(tab) => {
                self.ux.settings_tab = tab.min(crate::menus::SETTINGS_TABS.len() as u8 - 1);
            }
            Action::Volume(_) | Action::VolumeStep(..) | Action::VolumeSet(..) => {
                let (bus, step, set) = match action {
                    Action::VolumeStep(bus, step) => (bus, Some(step), None),
                    Action::VolumeSet(bus, to) => (bus, None, Some(to)),
                    Action::Volume(bus) => (bus, None, None),
                    _ => unreachable!(),
                };
                let level = match bus {
                    crate::audio::Bus::Effects => &mut self.ux.preferences.effects_volume,
                    crate::audio::Bus::Ambience => &mut self.ux.preferences.ambience_volume,
                    crate::audio::Bus::Music => &mut self.ux.preferences.music_volume,
                };
                *level = match (step, set) {
                    (Some(step), _) => (i16::from(*level) + i16::from(step)).clamp(0, 100) as u8,
                    (_, Some(to)) => to.min(100),
                    // Enter on the row walks the slider in quarters and wraps.
                    _ => match *level {
                        0..25 => 25,
                        25..50 => 50,
                        50..75 => 75,
                        75..100 => 100,
                        _ => 0,
                    },
                };
                if let Some(audio) = &self.audio {
                    audio.set_levels(
                        self.ux.preferences.effects_volume,
                        self.ux.preferences.ambience_volume,
                        self.ux.preferences.music_volume,
                    );
                }
                if set.is_none() && self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Volume changed for this session; could not save settings.");
                }
            }
            Action::EndReview => {
                self.end_practice_review();
                if let Some(audio) = &self.audio {
                    audio.hush();
                }
            }
            Action::ToggleEdgeScroll => {
                self.ux.preferences.edge_scroll = !self.ux.preferences.edge_scroll;
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify(
                        "Camera setting changed for this session; could not save settings.",
                    );
                }
            }
            Action::ToggleFocusPause => {
                self.ux.preferences.pause_unfocused = !self.ux.preferences.pause_unfocused;
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Pause setting changed for this session; could not save settings.");
                }
            }
            Action::CycleSpeed if self.session.is_some() => {
                self.notify("An online match runs at 1X for every seat.");
            }
            Action::CycleSpeed => {
                self.ux.preferences.game_speed = self.ux.preferences.game_speed.cycle();
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Pace changed for this session; settings could not be saved.");
                }
            }
            Action::CycleInterfaceScale => {
                self.ux.preferences.interface_scale = (self.ux.preferences.interface_scale + 1) % 5;
                if self.ux.persist(&self.data_dir).is_err() {
                    self.notify("Scale changed for this session; settings could not be saved.");
                }
            }
            Action::RosterFaction(faction) => {
                self.ux.roster_faction = Some(faction);
                self.ux.roster_buildings = false;
                self.ux.roster_unit = None;
            }
            Action::RosterBuildings => {
                // The reader's own buildings, whichever side was last read
                // (trial 11: the Union seat saw the Kiln).
                self.ux.roster_faction = None;
                self.ux.roster_buildings = true;
                self.ux.roster_unit = None;
            }
            Action::RosterUnit(kind) => self.ux.roster_unit = Some(kind),
            Action::FocusAlert(id) => self.focus_alert(id),
            Action::FocusHold => self.focus_hold(),
            Action::FocusLane(lane) => self.focus_lane(lane),
            Action::SelectWorks => self.select_works(),
            Action::TrainBatch(kind) => self.train_selection(kind, 5),
            Action::ToggleFullscreen => self.ux.fullscreen_request = Some(!self.ux.fullscreen),
            Action::Save => {
                self.save_match();
            }
            Action::Load if self.session.is_some() || self.playback.is_some() => {
                self.notify("Loading is not available here.")
            }
            Action::Load => self.request_transition(PendingAction::Load),
            Action::Confirm => {
                if let Some(pending) = self.ux.pending {
                    self.finish_transition(pending);
                }
            }
            Action::SaveAndConfirm => {
                if let Some(pending) = self.ux.pending
                    && self.save_match()
                {
                    self.finish_transition(pending);
                }
            }
            Action::Quit => self.request_transition(PendingAction::Quit),
            Action::Surrender => self.request_transition(if self.ux.practice {
                PendingAction::EndPractice
            } else {
                PendingAction::Surrender
            }),
            Action::FocusWorker => self.focus_target(TutorialTarget::Worker),
            Action::IdleWorker => self.select_idle_workers(false),
            Action::AllIdleWorkers => self.select_idle_workers(true),
            Action::SelectArmy => self.select_army(false),
            Action::FocusWorks => self.focus_target(TutorialTarget::Works),
            Action::FocusArmy => self.focus_target(TutorialTarget::Army),
            Action::FocusGate => self.focus_target(TutorialTarget::Gate),
            Action::EndGuidance => {
                self.ux.guidance_visible = !self.ux.guidance_visible;
                self.message.clear();
            }
            Action::TideCard => {
                self.ux.preferences.tide_card = !self.ux.preferences.tide_card;
                let _ = self.ux.persist(&self.data_dir);
            }
            Action::CompactFieldCards => {
                let compact = !self.ux.preferences.compact_field_cards;
                self.ux.preferences.compact_field_cards = compact;
                // Saved, but the fold works for the session either way.
                let _ = self.ux.persist(&self.data_dir);
            }
            Action::ResultPage(page) => {
                self.result_page = page;
                self.result_log_scroll = 0;
            }
            Action::ResultLogScroll(older) => {
                let rows = crate::dock_log::log_row_count(&self.ux.dock.log);
                self.result_log_scroll = if older {
                    (self.result_log_scroll + 8).min(rows.saturating_sub(1))
                } else {
                    self.result_log_scroll.saturating_sub(8)
                };
            }
            Action::ReplayRestart => self.seek_replay(0),
            Action::WatchReplay => {
                let path = self.data_dir.join("saves/last-match.replay.json");
                let result = self
                    .export_final_replay()
                    .and_then(|()| self.start_playback(&path, false));
                if let Err(e) = result {
                    eprintln!("Watch replay: {e}");
                    self.notify("Could not open the replay. Your field is unchanged.");
                }
            }
            Action::ExportReplay => {
                let result = self.export_final_replay();
                match result {
                    Ok(()) => self.notify("Replay saved."),
                    Err(e) => {
                        eprintln!("Replay save: {e}");
                        self.notify("Could not save the replay. Your field is unchanged.");
                    }
                }
            }
            Action::Train(kind) => self.train_selection(kind, 1),
            Action::Build(kind) => {
                self.mode = Mode::Build(kind);
                self.start_placing();
                self.message.clear();
            }
            Action::Gather => {
                self.mode = Mode::Gather;
                self.message.clear();
            }
            Action::Attack => {
                self.mode = Mode::Attack;
                self.message.clear();
            }
            Action::Stop => self.issue(Command::Stop {
                units: self.gameplay_ids(|k| !k.is_building()),
            }),
            Action::Hold => self.issue(Command::Hold {
                units: self.gameplay_ids(|k| !k.is_building()),
            }),
            Action::Deploy => self.issue(Command::SetDeployed {
                units: self.gameplay_ids(crate::controls::is_specialist),
                deployed: true,
            }),
            Action::Pack => self.issue(Command::SetDeployed {
                units: self.gameplay_ids(crate::controls::is_specialist),
                deployed: false,
            }),
            Action::KeepDeployed => {
                let units = self.gameplay_ids(crate::controls::is_specialist);
                let keep = !self.keeps_deployed();
                self.issue(Command::KeepDeployed { units, keep });
            }
            Action::SelectWorkers => self.select_workers(),
            Action::SelectBuildings(kind) => self.select_buildings(kind),
            Action::Upgrade(upgrade) => {
                if let Some(building) = self.first_building() {
                    self.issue(Command::Upgrade { building, upgrade });
                }
            }
            Action::CancelUpgrade => {
                if let Some(building) = self.first_building() {
                    self.issue(Command::CancelUpgrade { building });
                }
            }
            Action::Reclaim => {
                if let Some(hq) = self.reclaim_hq() {
                    self.issue(Command::Reclaim { building: hq });
                } else {
                    self.notify("Select your finished headquarters to RECLAIM.");
                }
            }
            Action::Recycle => {
                let units = self.gameplay_ids(|k| k.is_worker());
                if units.is_empty() {
                    self.notify("Select workers to RECYCLE.");
                } else {
                    self.issue(Command::Recycle { units });
                }
            }
            Action::Board => {
                if let Some((units, transport)) = self.board_target() {
                    self.issue(Command::Board { units, transport });
                } else {
                    self.notify("Select machines with an own transport in the field to board.");
                }
            }
            Action::Vent => {
                if let Some(building) = self.first_building() {
                    self.issue(Command::Vent { building });
                }
            }
            Action::Unload => {
                let loaded: Vec<EntityId> = self
                    .gameplay_ids(|k| k.is_transport())
                    .into_iter()
                    .filter(|id| {
                        self.world
                            .entities
                            .iter()
                            .any(|e| e.id == *id && !e.cargo.is_empty())
                    })
                    .collect();
                for transport in loaded {
                    self.issue(Command::Unload { transport });
                }
            }
            Action::Sound => {
                if let Some(unit) = self
                    .gameplay_ids(|k| matches!(k, Kind::Sounder | Kind::Skipper))
                    .first()
                    .copied()
                {
                    self.issue(Command::Sound { unit });
                }
            }
            Action::Glint => {
                if self.gameplay_ids(|k| k == Kind::Glinter).is_empty() {
                    self.notify("SELECT A GLINTER FIRST");
                } else {
                    self.mode = Mode::Glint;
                    self.message.clear();
                }
            }
            Action::Lay => {
                if self.gameplay_ids(|k| k == Kind::Salter).is_empty() {
                    self.notify("SELECT A SALTER FIRST");
                } else {
                    self.mode = Mode::Lay;
                    self.message.clear();
                }
            }
            Action::Formation
            | Action::Face
            | Action::Surge
            | Action::Research(_)
            | Action::CancelResearch => self.tactical_action(action),
            Action::Capture => self.issue(Command::Capture {
                units: self.gameplay_ids(|k| !k.is_worker() && !k.is_building()),
            }),
            Action::Switch => self.issue(Command::SwitchGate),
            Action::SetTide(arm) => self.issue(Command::SetTide { arm }),
            Action::Flood => self.issue(Command::Flood),
            Action::Cancel => {
                if let Some(building) = self.first_building() {
                    self.issue(Command::Cancel { building });
                }
            }
        }
    }
    pub fn key(&mut self, key: &str, shift: bool, control: bool) {
        self.tooltip_dismissed_at = Some(self.cursor);
        if self.screen == Screen::Match && self.skip_intro() {
            return;
        }
        if self.screen == Screen::Match && !control && matches!(key, "-" | "_" | "=" | "+") {
            self.change_zoom(matches!(key, "=" | "+"), self.world_view().center());
            return;
        }
        if self.screen == Screen::Confirm {
            self.menu_keyboard(key, shift);
            return;
        }
        if self.online_key(key, control) {
            return;
        }
        if self.observing() && self.screen == Screen::Match && !control {
            let tick = self.world.tick;
            match key {
                "F" => return self.toggle_follow(),
                "[" => return self.seek_replay(tick.saturating_sub(crate::spectator::SKIP_TICKS)),
                "]" => return self.seek_replay(tick + crate::spectator::SKIP_TICKS),
                _ => {}
            }
        }
        if control && key == "Q" {
            self.action(Action::Quit);
            return;
        }
        if key == "Escape" {
            if self.screen == Screen::Match && self.ux.keyboard_navigation {
                self.ux.keyboard_navigation = false;
                self.ux.focused = None;
                return;
            }
            if self.escape_guarded() {
                self.escape_guard_until = 0;
                self.notify("Nothing to cancel. Esc again opens the menu.");
                return;
            }
            // Esc puts the own selection back in the panel.
            if self.screen == Screen::Match
                && self.mode == Mode::Context
                && self.inspected_enemy().is_some()
            {
                self.inspected = None;
                self.message.clear();
                return;
            }
            self.back();
            return;
        }
        if self.screen == Screen::Help && self.ux.help_page == 4 && !control {
            let action = match key {
                "P" => Some(Action::ManualPlay),
                "N" => Some(Action::ManualStep),
                "B" => Some(Action::ManualSubject(
                    crate::field_manual::ManualSubject::Bulwark,
                )),
                "L" => Some(Action::ManualSubject(
                    crate::field_manual::ManualSubject::Loom,
                )),
                _ => None,
            };
            if let Some(action) = action {
                self.action(action);
                return;
            }
        }
        if self.menu_keyboard(key, shift) {
            return;
        }
        if key == "F1" {
            self.action(Action::Help);
            return;
        }
        if key == "F5" {
            self.action(Action::Save);
            return;
        }
        if key == "F9" {
            self.action(Action::Load);
            return;
        }
        if key == "F6" {
            self.action(Action::ExportReplay);
            return;
        }
        if key == "F8" {
            match Atlas::load(&self.base) {
                Ok(atlas) => {
                    self.atlas = Some(atlas);
                    self.notify("ART ASSETS RELOADED")
                }
                Err(e) => self.notify(&e),
            }
            return;
        }
        if key == "M" {
            self.action(Action::Mute);
            return;
        }
        if self.screen == Screen::Setup {
            match key {
                "1" => self.action(Action::Faction(Faction::Union)),
                "2" => self.action(Action::Faction(Faction::Assembly)),
                _ => (),
            }
            return;
        }
        if self.screen != Screen::Match || self.world.outcome.is_some() || self.ux.practice_review {
            return;
        }
        if let Ok(n) = key.parse::<usize>()
            && n < 10
        {
            self.control_group(n, shift, control);
            return;
        }
        if control && let Some(kind) = building_hotkey_kind(key) {
            self.action(Action::SelectBuildings(kind));
            return;
        }
        match key {
            "I" if control => self.center_on_selection(),
            "F2" if control => self.select_army(true),
            "I" => self.action(if shift {
                Action::AllIdleWorkers
            } else {
                Action::IdleWorker
            }),
            "F2" => self.select_army(shift),
            "F3" => self.action(Action::FocusAlert(None)),
            "F4" => self.action(Action::SelectWorks),
            "F7" => self.action(Action::SelectWorkers),
            "O" => self.action(Action::ToggleHealthBars),
            "A" => self.action(Action::Attack),
            "S" => self.action(Action::Stop),
            "H" => self.action(Action::Hold),
            "D" => self.action(Action::Deploy),
            "F" => self.action(Action::Formation),
            // R faces a direction; at a Drydock it trains the transport.
            "R" => {
                if let Some(kind) = self.production_hotkey_kind(key) {
                    self.action(if shift {
                        Action::TrainBatch(kind)
                    } else {
                        Action::Train(kind)
                    });
                } else if self.reclaim_hq().is_some() {
                    self.action(Action::Reclaim);
                } else {
                    self.action(Action::Face);
                }
            }
            "Z" => self.action(Action::Surge),
            "U" => self.action(if self.gameplay_ids(|k| k.is_transport()).is_empty() {
                Action::Research(bw_content::Doctrine::Hauling)
            } else {
                Action::Unload
            }),
            "J" => self.action(Action::Research(bw_content::Doctrine::FireControl)),
            "K" => self.action(if self.selected_specialists() {
                Action::KeepDeployed
            } else {
                Action::CancelResearch
            }),
            "G" => self.action(
                if self.first_worker().is_some()
                    && self
                        .gameplay_ids(|k| !k.is_worker() && !k.is_building())
                        .is_empty()
                {
                    Action::Gather
                } else {
                    Action::Capture
                },
            ),
            "T" => self.action(if shift { Action::Flood } else { Action::Switch }),
            "Home" | "Space" => self.home(),
            // B is BUILD only; E boards (trial 12: B with Sounders
            // selected, meant as build, put nine of them in a Lifter).
            "B" if self.first_worker().is_none() && self.board_target().is_some() => {
                self.notify("B builds with workers. E boards.")
            }
            "B" => self.action(Action::Build(Kind::Works)),
            "E" if self.production_hotkey_kind(key).is_none()
                && !self
                    .gameplay_ids(|k| !k.is_building() && !k.is_transport())
                    .is_empty() =>
            {
                self.action(Action::Board)
            }
            "C" => self.action(Action::Build(Kind::Condenser)),
            "Y" => self.action(Action::Build(Kind::Dropoff)),
            "V" => self.action(Action::Build(Kind::Tower)),
            "N" | "P" | "L" => {
                if let Some(action) = self.context_hotkey(key) {
                    self.action(action);
                }
            }
            // X at workers recycles them; RECYCLE is the worker card's
            // special, as VENT is the headquarters' and SOUND a scout's.
            "X" if self.first_worker().is_some() && self.first_building().is_none() => {
                self.action(Action::Recycle)
            }
            "X" => self.action(
                if self.first_worker().is_none()
                    && !self
                        .gameplay_ids(|k| matches!(k, Kind::Sounder | Kind::Skipper))
                        .is_empty()
                {
                    Action::Sound
                } else if self.first_worker().is_none()
                    && !self.gameplay_ids(|k| k == Kind::Glinter).is_empty()
                {
                    Action::Glint
                } else {
                    Action::Vent
                },
            ),
            "Backspace" => self.action(Action::Cancel),
            "Q" | "W" | "E" => {
                if let Some(kind) = self.production_hotkey_kind(key) {
                    self.action(if shift {
                        Action::TrainBatch(kind)
                    } else {
                        Action::Train(kind)
                    });
                } else {
                    self.notify("Select headquarters, a Works or a Drydock to train machines.");
                }
            }
            _ => (),
        }
    }
    /// N, P and L by selection: a worker builds a Drydock or Palisade; a
    /// building starts its first, second or third upgrade.
    fn context_hotkey(&self, key: &str) -> Option<Action> {
        if key == "P" && self.first_worker().is_none() && self.selected_specialists() {
            return Some(Action::Pack);
        }
        if key == "L"
            && self.first_worker().is_none()
            && !self.gameplay_ids(|k| k == Kind::Salter).is_empty()
        {
            return Some(Action::Lay);
        }
        if self.first_worker().is_some() {
            return match key {
                "N" => Some(Action::Build(Kind::Drydock)),
                "P" => Some(Action::Build(Kind::Palisade)),
                _ => None,
            };
        }
        let kind = self
            .first_building()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
            .map(|e| e.kind)?;
        if kind == Kind::Headquarters {
            // One OVERHAUL button, the next level (rules 21).
            return (key == "P").then(|| Action::Upgrade(self.next_overhaul()));
        }
        let index = match key {
            "P" => 0,
            "L" => 1,
            _ => 2,
        };
        bw_content::Upgrade::offered_by(kind)
            .get(index)
            .map(|upgrade| Action::Upgrade(*upgrade))
    }

    pub fn render(&mut self) {
        if self.screen == Screen::Match {
            self.advance_zoom();
        }
        if self.native_ui() {
            self.render_native();
            return;
        }
        self.frame += 1;
        self.buttons.clear();
        match self.screen {
            Screen::Menu => self.draw_menu(),
            Screen::Setup => self.draw_setup(),
            Screen::Lobby => self.draw_lobby(),
            Screen::Settings => self.draw_settings(),
            Screen::Confirm => self.draw_confirm(),
            Screen::Help => self.draw_help(),
            Screen::Pause => self.draw_pause(),
            Screen::Match => {
                self.draw_zoomed_world();
                self.draw_hud();
                self.draw_zoom_controls();
                if self.ux.practice_review {
                    let model = self.menu_model();
                    self.buttons = crate::menus::practice_review(
                        &mut self.canvas,
                        self.atlas.as_ref(),
                        &model,
                    );
                } else if let Some(outcome) = self.world.outcome.clone() {
                    self.draw_result(outcome);
                } else {
                    if let Some((x, y)) = self.drag {
                        self.canvas.frame(
                            x.min(self.cursor.0),
                            y.min(self.cursor.1),
                            (self.cursor.0 - x).abs() + 1,
                            (self.cursor.1 - y).abs() + 1,
                            JADE,
                        );
                    }
                    self.draw_guidance();
                }
            }
        }
        self.ensure_menu_focus();
        self.draw_buttons();
        self.draw_setup_cards();
        if self.screen == Screen::Match {
            self.draw_field_tip();
        }
        self.draw_ux_feedback();
    }
    fn update_resource_stages(&mut self) {
        let observed: Vec<(u32, u8)> = self
            .world
            .map
            .resources
            .iter()
            .filter(|resource| self.world.visible(0, resource.pos))
            .map(|resource| (resource.id, resource_stage(resource.remaining)))
            .collect();
        for (id, stage) in observed {
            self.resource_stages.insert(id, stage);
        }
    }
    fn resource_asset_key(&self, resource: &bw_sim::Resource) -> String {
        let base = salvage_asset(resource.id);
        let stage = self.resource_stages.get(&resource.id).copied().unwrap_or(0);
        salvage_stage_asset(base, stage)
    }
    /// Where LAY under the pointer would crust: the strip from the bank
    /// across the water, up to LAY_MAX_ROWS rows, as screen diamonds.
    fn lay_preview(&self, camera: Camera) -> Vec<(i32, i32, Color)> {
        let Some(salter) = self
            .gameplay_ids(|k| k == Kind::Salter)
            .first()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
        else {
            return Vec::new();
        };
        let target = camera.unproject(self.cursor.0, self.cursor.1);
        let Ok((bank, dx, dy)) = self.world.lay_plan(salter.pos, target) else {
            let (x, y) = camera.project(Pos::cell(target.cell_xy().0, target.cell_xy().1));
            return vec![(x, y, [206, 72, 58, 120])];
        };
        let (dx, dy) = (i32::from(dx), i32::from(dy));
        let (mut x, mut y) = bank.cell_xy();
        let map = &self.world.map;
        let mut out = Vec::new();
        for _ in 0..bw_content::LAY_MAX_ROWS {
            x += dx;
            y += dy;
            if map.terrain(x, y).tidal_arm().is_none() {
                break;
            }
            for offset in -1..=1 {
                let cell = (x + dy.abs() * offset, y + dx.abs() * offset);
                if map.terrain(cell.0, cell.1).tidal_arm().is_some() {
                    let (px, py) = camera.project(Pos::cell(cell.0, cell.1));
                    out.push((px, py, [COBALT[0], COBALT[1], COBALT[2], 90]));
                }
            }
        }
        out
    }
    /// A Heliostat's beam: a line of light from the mirror to the target,
    /// wider and whiter as the beam builds on one target, with its flare at
    /// the mirror and its hit on the target.
    fn draw_beam(
        canvas: &mut Canvas,
        atlas: Option<&Atlas>,
        (sx, sy): (i32, i32),
        (tx, ty): (i32, i32),
        damage: i32,
        age: i32,
        source_visible: bool,
    ) {
        let heat = (damage - 2).clamp(0, 8);
        if age < 6 && source_visible {
            // COBALT warming to white over eight steps.
            let core: Color = std::array::from_fn(|i| {
                let (a, b) = (i32::from(COBALT[i]), i32::from(WHITE[i]));
                (a + (b - a) * heat / 8) as u8
            });
            let width = 1 + heat / 3;
            for offset in 0..width {
                let dy = offset - width / 2;
                canvas.line(sx, sy + dy, tx, ty + dy, core);
            }
            canvas.line(sx, sy, tx, ty, WHITE);
        }
        if let Some(atlas) = atlas {
            if age < 6 && source_visible {
                atlas.draw(
                    canvas,
                    &format!("fx_beam_flare_{}", (age / 2).min(2)),
                    sx,
                    sy,
                    false,
                );
            }
            atlas.draw(
                canvas,
                &format!("fx_beam_hit_{}", (age / 3).min(3)),
                tx,
                ty,
                false,
            );
        }
    }
    fn draw_landmarks(canvas: &mut Canvas, atlas: Option<&Atlas>, world: &World, camera: Camera) {
        let Some(atlas) = atlas else {
            return;
        };
        let mut visible: Vec<Landmark> = crate::presentation::landmarks(world.map.id)
            .iter()
            .copied()
            .filter(|landmark| {
                world.visible(0, landmark.pos) && atlas.sprites.contains_key(landmark.key)
            })
            .collect();
        visible.sort_by_key(|landmark| (ground_depth(landmark.pos), landmark.key));
        for landmark in visible {
            let (x, y) = camera.project(landmark.pos);
            atlas.draw(canvas, landmark.key, x, y, false);
        }
    }
    fn draw_world(&mut self) {
        self.canvas.clear([38, 61, 68, 255]);
        self.update_resource_stages();
        let explored = self.ux.chart_memory.explored.known().cloned();
        let station3_pieces = self.station3_pieces();
        let map = &self.world.map;
        let camera = self.camera;
        let visual_tick = self.visual_tick();
        let mut shores = Vec::new();
        let mut decorations = Vec::new();
        let mut ground_patches = Vec::new();
        for sum in 0..i32::from(map.width + map.height) {
            for x in 0..i32::from(map.width) {
                let y = sum - x;
                if y < 0 || y >= i32::from(map.height) {
                    continue;
                }
                let pos = Pos::cell(x, y);
                let (sx, sy) = camera.project(pos);
                if !(-160..self.canvas.width() as i32 + 160).contains(&sx)
                    || !(-160..self.canvas.height() as i32 + 160).contains(&sy)
                {
                    continue;
                }
                let t = map.terrain(x, y);
                let depth = self.world.depth(t);
                let visible = self.world.visible(0, pos);
                let hash = (x * 13 + y * 17 + x * y * 3).rem_euclid(11);
                let variant = if hash < 8 { 0 } else { 1 + hash % 3 };
                let water_phase = if hash < 8 {
                    0
                } else {
                    1 + (visual_tick / 24 + x as u64 * 5 + y as u64 * 3) % 3
                };
                let (key, fallback) = match t {
                    Terrain::Deep => (
                        format!("terrain_water_{water_phase}"),
                        format!("terrain_water_{water_phase}"),
                    ),
                    t if t.is_tidal() && self.world.is_crusted(x, y) => {
                        // A Salter's crust: dry white salt over the water
                        // until the tide moves, bright for its first second.
                        let fresh = self
                            .crust_laid
                            .iter()
                            .find(|(cell, _)| *cell == (x, y))
                            .map(|(_, tick)| visual_tick.saturating_sub(*tick));
                        let key = match fresh {
                            Some(age) => format!("terrain_crust_fresh_{}", (age / 10).min(2)),
                            None => format!("terrain_crust_{}", hash % 4),
                        };
                        (key, format!("terrain_causeway_{variant}"))
                    }
                    t if t.is_tidal() => {
                        // Deep tidal water is the lake's water; shallow is
                        // the wet lane; dry is the lane road or the rim's
                        // causeway.  A side that just dried shows its
                        // drying stages first.
                        let rim = t.is_rim();
                        let arm = t.tidal_arm().map_or(0, usize::from);
                        match depth {
                            Some(bw_sim::Depth::Deep) => (
                                format!("terrain_water_{water_phase}"),
                                format!("terrain_water_{water_phase}"),
                            ),
                            Some(bw_sim::Depth::Shallow) => {
                                let lane_phase = ((visual_tick / 24) + hash as u64) % 4;
                                (
                                    format!("terrain_lane_wet_{lane_phase}"),
                                    format!("terrain_shallow_{water_phase}"),
                                )
                            }
                            _ => {
                                let lane_phase = (hash as u64) % 4;
                                let dry_key = if rim {
                                    format!("terrain_causeway_{variant}")
                                } else {
                                    format!("terrain_lane_dry_{lane_phase}")
                                };
                                let drying = self
                                    .tide_change
                                    .and_then(|change| {
                                        change.dried(arm).then(|| {
                                            crate::lane_memory::drying_stage(
                                                Some(change.tick),
                                                visual_tick,
                                            )
                                        })
                                    })
                                    .flatten();
                                match drying {
                                    Some(stage) => (
                                        crate::lane_memory::drying_key(stage, hash as u64),
                                        dry_key,
                                    ),
                                    None => (dry_key, format!("terrain_causeway_{variant}")),
                                }
                            }
                        }
                    }
                    Terrain::Silt => {
                        // Ground light: a damp waterline beside deep water.
                        let base = format!("terrain_silt_{variant}");
                        let lit = crate::ground_light::floor_key(map, x, y, variant)
                            .unwrap_or_else(|| base.clone());
                        (lit, base)
                    }
                    _ => {
                        // Ground light: broad low/high tonal planes and the
                        // damp waterline, chosen from map data only.
                        let base = format!("terrain_salt_{variant}");
                        let lit = crate::ground_light::floor_key(map, x, y, variant)
                            .unwrap_or_else(|| base.clone());
                        (lit, base)
                    }
                };
                let drawn = self.atlas.as_ref().is_some_and(|atlas| {
                    atlas.draw(&mut self.canvas, &key, sx, sy, !visible)
                        || atlas.draw(&mut self.canvas, &fallback, sx, sy, !visible)
                });
                if !drawn {
                    let color = if t == Terrain::Deep {
                        [44, 89, 101, 255]
                    } else {
                        [146, 130, 105, 255]
                    };
                    self.canvas.diamond(
                        sx,
                        sy,
                        16,
                        8,
                        if visible { color } else { shade(color, 40) },
                    );
                }
                // Ground never in sight is a blank survey sheet: no shore,
                // plant or stone is drawn on it.
                let known = |x: i32, y: i32| explored.as_ref().is_none_or(|e| e.get(x, y));
                if !visible && !known(x, y) {
                    continue;
                }
                if t != Terrain::Deep {
                    let edges = [
                        (1, 0, (sx + 16, sy, sx, sy + 8), true, "e"),
                        (-1, 0, (sx - 16, sy, sx, sy - 8), false, "w"),
                        (0, 1, (sx, sy + 8, sx - 16, sy), true, "s"),
                        (0, -1, (sx, sy - 8, sx + 16, sy), false, "n"),
                    ];
                    for (dx, dy, edge, front, direction) in edges {
                        if map.terrain(x + dx, y + dy) == Terrain::Deep {
                            shores.push((edge, front, visible, sx, sy, direction, hash % 3));
                        }
                    }
                    if let Some(key) = bank_plant(map, x, y) {
                        // One wind over the basin: the plant leans with the
                        // gust field at its own world position.
                        let gust = crate::wind::strength_with_jitter(
                            i64::from(sx) + i64::from(camera.x),
                            i64::from(sy) + i64::from(camera.y),
                            visual_tick,
                            crate::wind::cell_jitter(x, y),
                        );
                        decorations.push((sx, sy, visible, crate::wind::plant_key(key, gust)));
                    }
                }
                if t == Terrain::Rock
                    && let Some(atlas) = &self.atlas
                {
                    let key = match (x * 7 + y * 11).rem_euclid(3) {
                        1 => "salt_rock_1",
                        2 => "salt_rock_2",
                        _ => "salt_rock",
                    };
                    decorations.push((
                        sx,
                        sy,
                        visible,
                        if atlas.sprites.contains_key(key) {
                            key
                        } else {
                            "salt_rock"
                        }
                        .to_string(),
                    ));
                }
                // These flat mineral patches span several tiles. Keep the whole
                // footprint on salt and within one visibility state, so a patch
                // cannot paint across a bank, road, or fog boundary.
                if t == Terrain::Salt
                    && (x + (y / 5) * 2).rem_euclid(5) == 0
                    && y.rem_euclid(5) == 2
                    && (-2..=2).all(|dx| {
                        (-2..=2).all(|dy| {
                            map.terrain(x + dx, y + dy) == Terrain::Salt
                                && self.world.visible(0, Pos::cell(x + dx, y + dy)) == visible
                                && known(x + dx, y + dy)
                        })
                    })
                {
                    ground_patches.push((
                        sx,
                        sy,
                        visible,
                        (x * 13 + y * 7 + (x * y).rem_euclid(11)).rem_euclid(3),
                    ));
                }
                if matches!(t, Terrain::Salt | Terrain::Silt)
                    && (x * 19 + y * 23).rem_euclid(97) == 0
                {
                    decorations.push((
                        sx,
                        sy,
                        visible,
                        if hash % 3 == 0 {
                            "drift_scrap"
                        } else {
                            "pebbles"
                        }
                        .to_string(),
                    ));
                }
            }
        }
        for (x, y, visible, variant) in ground_patches {
            if let Some(atlas) = &self.atlas {
                atlas.draw(
                    &mut self.canvas,
                    &format!("salt_crust_{variant}"),
                    x,
                    y,
                    !visible,
                );
            }
        }
        crate::chart_surface::draw(
            &mut self.canvas,
            self.atlas.as_ref(),
            &self.world,
            camera,
            visual_tick,
            self.tide_change,
            self.ux.chart_memory.explored.known(),
        );
        // Paint shore depth after the floor pass so water tiles cannot erase it.
        for ((x0, y0, x1, y1), front, visible, sx, sy, direction, variant) in shores {
            if self.atlas.as_ref().is_some_and(|atlas| {
                atlas.draw(
                    &mut self.canvas,
                    &format!("shore_{direction}_{variant}"),
                    sx,
                    sy,
                    !visible,
                )
            }) {
                continue;
            }
            let shadow = if visible {
                [92, 83, 66, 255]
            } else {
                [37, 37, 35, 255]
            };
            let rim = if visible {
                [190, 169, 132, 255]
            } else {
                [76, 68, 53, 255]
            };
            if front {
                for depth in 1..=3 {
                    self.canvas.line(x0, y0 + depth, x1, y1 + depth, shadow);
                }
            }
            self.canvas.line(x0, y0, x1, y1, rim);
        }
        for (x, y, visible, key) in decorations {
            if let Some(atlas) = &self.atlas {
                atlas.draw(&mut self.canvas, &key, x, y, !visible);
            }
        }
        crate::lane_wall::draw(&mut self.canvas, &self.world, camera);
        if let Some(atlas) = &self.atlas {
            crate::artistry::draw_ground_at_tick(
                &mut self.canvas,
                atlas,
                &self.world,
                camera,
                visual_tick,
            );
            crate::artistry::draw_coast_flush(
                &mut self.canvas,
                atlas,
                &self.world,
                camera,
                visual_tick,
                &self.coast_flush,
            );
        }
        // Landmarks are sorted among themselves and painted before interactive
        // world items, so unit silhouettes, selection masks, and target paths
        // remain the top read even when a large landmark sprite extends upward.
        Self::draw_landmarks(&mut self.canvas, self.atlas.as_ref(), &self.world, camera);
        self.ux.traces.draw(
            &mut self.canvas,
            self.atlas.as_ref(),
            &self.world,
            camera,
            visual_tick,
        );
        // Observed fragments settle onto the ground, beneath surviving units.
        // Their fixed cosmetic expiry reveals no further enemy activity.
        if let Some(atlas) = &self.atlas {
            for effect in self.effects.iter().filter(|effect| {
                effect.death
                    && self.world.visible(0, effect.to)
                    && visual_tick >= effect.until.saturating_sub(presentation::WRECK_TICKS / 2)
                    && visual_tick < effect.until.saturating_add(presentation::RESIDUE_TICKS)
            }) {
                let (x, y) = camera.project(effect.to);
                atlas.draw(&mut self.canvas, wreck_asset(effect.faction), x, y, false);
            }
        }
        if let Some(e) = self
            .world
            .entities
            .iter()
            .find(|e| self.selected.first() == Some(&e.id) && e.owner == 0)
        {
            let mut previous = camera.project(e.pos);
            for p in e.path.iter().skip(e.path_index).take(80) {
                let next = camera.project(*p);
                self.canvas
                    .line(previous.0, previous.1, next.0, next.1, [170, 173, 121, 150]);
                self.canvas.diamond(next.0, next.1, 2, 1, GOLD);
                previous = next;
            }
        }
        crate::chart_memory::draw(
            &mut self.canvas,
            self.atlas.as_ref(),
            &self.world,
            camera,
            &self.ux.chart_memory,
        );
        // The match clock as daylight: key the ground, water and chart now,
        // before machines and buildings are drawn, so they keep exact colours.
        let day_bottom = self.canvas.height() as i32;
        crate::daylight::apply(
            &mut self.canvas,
            crate::daylight::key(visual_tick),
            0,
            day_bottom,
        );
        if matches!(self.mode, Mode::Build(_)) {
            // Ground that refuses every building is hatched over the whole
            // view before the pointer gets there: tidal ground and the
            // sluice's approach (trial 12).  On the ground, under the
            // machines and the station.
            crate::trial12_field::hatch_refused_ground(
                &mut self.canvas,
                &self.world,
                explored.as_ref(),
                camera,
            );
        }
        let mut draw_items = Vec::with_capacity(
            map.resources.len() + map.wells.len() + self.world.entities.len() + 1,
        );
        // Map resource locations are public; an unknown wreck keeps its initial
        // appearance until observed, even if its hidden amount has reached zero.
        draw_items.extend(map.resources.iter().map(WorldDrawItem::Resource));
        draw_items.extend(map.wells.iter().copied().map(WorldDrawItem::Well));
        draw_items.push(WorldDrawItem::Gate(map.gate_pos));
        draw_items.extend(
            self.world
                .entities
                .iter()
                .filter(|e| {
                    e.hp > 0
                        && e.aboard.is_none()
                        && (e.owner == 0 || self.world.entity_visible(0, e.id))
                })
                .map(WorldDrawItem::Entity),
        );
        sort_world_draw_items(&mut draw_items);
        let mut entity_overlays = Vec::with_capacity(self.world.entities.len());
        // Wreck labels are drawn after the machines so a worker in front of
        // a wreck never hides its words.
        let mut wreck_marks: Vec<crate::field_labels::WreckMark> = Vec::new();
        let venting = self.world.players[0].vent_remaining > 0;
        // The station as drawn, and how many machines were drawn before it:
        // those behind its rails keep a ghost (trial 12).
        let mut station_parts: Vec<(String, i32, i32)> = Vec::new();
        let mut drawn_before_station: Option<usize> = None;

        // Ground marks go down before any sprite, so a machine or building
        // in front covers them as it covers the ground.
        for item in &draw_items {
            let WorldDrawItem::Entity(e) = item else {
                continue;
            };
            if e.kind.is_building() {
                continue;
            }
            let (x, y) = camera.project(e.pos);
            if !(-40..self.canvas.width() as i32 + 40).contains(&x)
                || !(-40..self.canvas.height() as i32 + 40).contains(&y)
            {
                continue;
            }
            use crate::ground_marks::Part;
            if e.owner != 0 {
                crate::ground_marks::enemy_plate(
                    &mut self.canvas,
                    x,
                    y,
                    e.kind,
                    crate::seats::seat_hue(&self.world, e.owner),
                    Part::Back,
                );
            }
            let progress = crate::ground_marks::brace_progress(e);
            crate::ground_marks::braces(&mut self.canvas, x, y, progress, Part::Back);
            if e.kind == Kind::Bulwark && progress == 12 {
                const DIR: [(i32, i32); 8] = [
                    (0, -1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                    (0, 1),
                    (-1, 1),
                    (-1, 0),
                    (-1, -1),
                ];
                let (dx, dy) = DIR[usize::from(e.facing.min(7))];
                // Diagonal facings are longer vectors; keep the line at the
                // same distance and width on every face.
                let (ahead, half) = if dx != 0 && dy != 0 {
                    (FP * 9 / 10, FP * 7 / 10)
                } else {
                    (FP * 13 / 10, FP)
                };
                let front = Pos {
                    x: e.pos.x + dx * ahead,
                    y: e.pos.y + dy * ahead,
                };
                let a = camera.project(Pos {
                    x: front.x - dy * half,
                    y: front.y + dx * half,
                });
                let b = camera.project(Pos {
                    x: front.x + dy * half,
                    y: front.y - dx * half,
                });
                crate::ground_marks::shield_line(&mut self.canvas, a, b);
            }
        }

        for item in draw_items {
            match item {
                WorldDrawItem::Resource(r) => {
                    let (x, y) = camera.project(r.pos);
                    if !(-160..self.canvas.width() as i32 + 160).contains(&x)
                        || !(-160..self.canvas.height() as i32 + 160).contains(&y)
                    {
                        continue;
                    }
                    let dim = !self.world.visible(0, r.pos);
                    let staged = self.resource_asset_key(r);
                    // A wreck seen spent lies flat as pale ash (trial 12).
                    let spent = self.resource_stages.get(&r.id) == Some(&3);
                    let drawn = self.atlas.as_ref().is_some_and(|a| {
                        if spent {
                            let key = if a.sprites.contains_key(&staged) {
                                staged.as_str()
                            } else {
                                salvage_asset(r.id)
                            };
                            return crate::trial12_field::draw_spent(
                                a,
                                &mut self.canvas,
                                key,
                                x,
                                y,
                                dim,
                            );
                        }
                        a.draw(&mut self.canvas, &staged, x, y, dim)
                            || a.draw(&mut self.canvas, salvage_asset(r.id), x, y, dim)
                            || a.draw(&mut self.canvas, "salvage", x, y, dim)
                    });
                    if !drawn {
                        self.canvas.diamond(
                            x,
                            y,
                            14,
                            7,
                            if dim {
                                [70, 63, 54, 255]
                            } else {
                                [118, 77, 47, 255]
                            },
                        );
                        self.canvas.rect(
                            x - 8,
                            y - 12,
                            11,
                            11,
                            if dim {
                                [70, 63, 54, 255]
                            } else {
                                [222, 159, 87, 255]
                            },
                        );
                        self.canvas.line(
                            x - 8,
                            y - 12,
                            x + 2,
                            y - 12,
                            if dim { [70, 63, 54, 255] } else { WHITE },
                        );
                    }
                    // The wreck says what it is: the salvage left when seen,
                    // EMPTY once spent, and a plain WRECK under the fog.
                    let seen = self.world.visible(0, r.pos);
                    let drowned = !self.world.gatherable(Kind::Hook, r);
                    // Every wreck in sight names its salvage, whatever is
                    // selected: one seat spent twenty minutes not knowing
                    // where the salvage lay because the plates hid.
                    use crate::field_labels::WreckClass;
                    let class = if seen && r.remaining == 0 {
                        WreckClass::Empty
                    } else if drowned {
                        WreckClass::Drowned
                    } else if seen
                        || self.world.players[0]
                            .upgrades
                            .contains(&bw_content::Upgrade::SalvageSonar)
                    {
                        WreckClass::Salvage
                    } else {
                        WreckClass::Unseen
                    };
                    wreck_marks.push(crate::field_labels::WreckMark {
                        cell: r.pos.cell_xy(),
                        at: (x, y),
                        class,
                        amount: r.remaining,
                    });
                }
                WorldDrawItem::Well(pos) => {
                    let (x, y) = camera.project(pos);
                    let dim = !self.world.visible(0, pos);
                    if !self
                        .atlas
                        .as_ref()
                        .is_some_and(|a| a.draw(&mut self.canvas, "well", x, y, dim))
                    {
                        self.canvas.ellipse(x, y, 14, 7, EDGE);
                        self.canvas.ellipse(x, y - 3, 8, 4, JADE);
                    }
                }
                WorldDrawItem::Gate(pos) => {
                    let (x, y) = camera.project(pos);
                    let warning = self.world.gate.warning_until.is_some();
                    let phase = gate_warning_phase(self.world.gate.warning_until, self.world.tick)
                        .unwrap_or(0);
                    let rest = gate_asset(self.world.gate.north_dry(), warning, phase);
                    let key = self
                        .gate_switch
                        .and_then(|(start, north_dry)| {
                            presentation::gate_switch_asset(north_dry, start, self.world.tick)
                        })
                        .unwrap_or(rest);
                    // Three arms: the station with a shaft per arm and no
                    // N/S letters (art v22).
                    let station3 = station3_pieces.as_ref().zip(self.atlas.as_ref());
                    if let Some((pieces, atlas)) = station3 {
                        for (key, dx, dy) in pieces {
                            atlas.draw(&mut self.canvas, key, x + dx, y + dy, false);
                        }
                    }
                    let drawn = station3.is_some()
                        || self.atlas.as_ref().is_some_and(|a| {
                            a.draw(&mut self.canvas, &key, x, y, false)
                                || a.draw(&mut self.canvas, "gate", x, y, false)
                        });
                    drawn_before_station = Some(entity_overlays.len());
                    station_parts = match station3 {
                        Some((pieces, _)) => pieces
                            .iter()
                            .map(|(key, dx, dy)| (key.clone(), x + dx, y + dy))
                            .collect(),
                        None => vec![(key.clone(), x, y), ("gate".to_string(), x, y)],
                    };
                    if !drawn {
                        self.canvas.diamond(x, y, 35, 17, EDGE);
                        self.canvas.rect(x - 13, y - 34, 26, 31, PANEL);
                        self.canvas.rect(x - 10, y - 30, 20, 4, WHITE);
                        self.canvas.ellipse(x, y - 17, 7, 7, GOLD);
                    }
                    if let Some(start) = self.gate_foam_start
                        && let Some(age) = visual_tick.checked_sub(start)
                        && age < presentation::SLUICE_FX_TICKS
                    {
                        let fx = format!("fx_sluice_{}", (age / 6).min(5));
                        let _ = self
                            .atlas
                            .as_ref()
                            .is_some_and(|a| a.draw(&mut self.canvas, &fx, x, y, false));
                    }
                }
                WorldDrawItem::Entity(e) => {
                    let (x, y, scale) = entity_render_position(
                        camera,
                        e,
                        self.world.players[e.owner as usize].faction,
                    );
                    if !(-160..self.canvas.width() as i32 + 160).contains(&x)
                        || !(-160..self.canvas.height() as i32 + 160).contains(&y)
                    {
                        continue;
                    }
                    let selected = self.selected.contains(&e.id);
                    let color = crate::seats::seat_colour(&self.world, e.owner);
                    if selected {
                        let radius = if e.kind.is_building() { 25 } else { 14 };
                        self.canvas.line(x - radius, y, x, y + radius / 2, color);
                        self.canvas.line(x, y + radius / 2, x + radius, y, color);
                        self.canvas.line(x - radius, y, x, y - radius / 2, color);
                        self.canvas.line(x, y - radius / 2, x + radius, y, color);
                    } else if self.hovered == Some(e.id) {
                        // What a click would hit: a broken ring in its
                        // owner's colour, quieter than a selection's.  Its
                        // outline follows once the body is drawn.
                        let radius = if e.kind.is_building() { 25 } else { 14 };
                        dashed_diamond(&mut self.canvas, x, y, radius, color);
                    }
                    let key = e.kind.asset(self.world.players[e.owner as usize].faction);
                    let directional = self.directional_key(e);
                    let animation = self.entity_animation_key(e);
                    // Wading: a machine on a flooded cell drops its baked ground
                    // shadow and shows a broken reflection where water lies.
                    let (ecx, ecy) = e.pos.cell_xy();
                    let wading = !e.kind.is_building()
                        && crate::presentation::cell_is_wet_at(&self.world, ecx, ecy);
                    let is_water = |px: i32, py: i32| {
                        let (cx, cy) = camera.unproject(px, py).cell_xy();
                        crate::presentation::cell_is_wet_at(&self.world, cx, cy)
                    };
                    let drawn = if let Some(a) = &self.atlas {
                        let draw_key = |canvas: &mut crate::canvas::Canvas, k: &str| {
                            if wading {
                                a.draw_wading(canvas, k, x, y, &is_water)
                            } else {
                                a.draw_scaled(canvas, k, x, y, false, scale)
                            }
                        };
                        animation
                            .as_ref()
                            .is_some_and(|key| draw_key(&mut self.canvas, key))
                            || draw_key(&mut self.canvas, &directional)
                            || draw_key(&mut self.canvas, key)
                    } else {
                        false
                    };
                    if wading
                        && drawn
                        && let Some(phase) = self.walking_phase(e)
                    {
                        draw_wake(&mut self.canvas, x, y, phase, visual_tick, &is_water);
                    }
                    let hover_edge = drawn && !selected && self.hovered == Some(e.id);
                    // Every enemy machine and building carries a red edge, so
                    // ownership reads at a glance whatever the faction material.
                    // The one under the pointer is edged too, lighter, in its
                    // owner's colour.
                    if (e.owner != 0 || hover_edge)
                        && drawn
                        && let Some(a) = &self.atlas
                    {
                        let edge_key = animation
                            .as_deref()
                            .filter(|k| a.sprites.contains_key(*k))
                            .or_else(|| {
                                a.sprites
                                    .contains_key(&directional)
                                    .then_some(directional.as_str())
                            })
                            .unwrap_or(key);
                        // One pixel vanishes on a small moving machine in a
                        // halved picture; two in both axes hold.
                        let edge = if hover_edge {
                            let c = color;
                            let lift = |v: u8| ((u16::from(v) + 255) / 2) as u8;
                            [lift(c[0]), lift(c[1]), lift(c[2]), 255]
                        } else {
                            enemy_edge(e.owner)
                        };
                        for (dx, dy) in enemy_edge_offsets(self.ui_scale()) {
                            a.draw_outline(&mut self.canvas, edge_key, x + dx, y + dy, scale, edge);
                        }
                    }
                    if !e.kind.is_building() {
                        use crate::ground_marks::Part;
                        let (gx, gy) = camera.project(e.pos);
                        if e.owner != 0 {
                            crate::ground_marks::enemy_plate(
                                &mut self.canvas,
                                gx,
                                gy,
                                e.kind,
                                crate::seats::seat_hue(&self.world, e.owner),
                                Part::Front,
                            );
                        }
                        crate::ground_marks::braces(
                            &mut self.canvas,
                            gx,
                            gy,
                            crate::ground_marks::brace_progress(e),
                            Part::Front,
                        );
                    }
                    if !drawn {
                        let body = if e.owner == 0 { GOLD } else { JADE };
                        if e.kind.is_building() {
                            // Buildings meet the terrain directly, including
                            // missing-sprite fallbacks. Never add a ground shadow.
                            self.canvas.rect(x - 20, y - 34, 40, 34, body);
                            self.canvas.frame(x - 20, y - 34, 40, 34, WHITE);
                            self.canvas.rect(x - 16, y - 38, 32, 6, WHITE);
                            self.canvas.rect(x - 6, y - 13, 12, 13, PANEL);
                        } else {
                            self.canvas.ellipse(x, y, 14, 6, INK);
                            self.canvas.rect(x - 10, y - 22, 20, 20, body);
                            self.canvas.rect(x - 8, y - 24, 16, 6, WHITE);
                            self.canvas.rect(x - 4, y - 16, 7, 4, PANEL);
                        }
                    }
                    if let Some(atlas) = &self.atlas {
                        crate::artistry::draw_entity_overlay(
                            &mut self.canvas,
                            atlas,
                            &self.world,
                            e,
                            x,
                            y,
                        );
                        if e.kind == Kind::Headquarters && e.build_remaining == 0 {
                            let mark = match self.world.players[e.owner as usize].faction {
                                Faction::Union => "faction_union_hq",
                                Faction::Assembly => "faction_assembly_hq",
                                Faction::Compact => "faction_compact_hq",
                            };
                            atlas.draw(&mut self.canvas, mark, x - 17, y - 26, false);
                        }
                        // Damage in the faction's material, evening lamps, a
                        // vapour wisp on a failing hull, and observed repairs.
                        let faction = self.world.players[e.owner as usize].faction;
                        let (tier, overlay) = if e.kind.is_building() {
                            let tier = crate::damage::building_tier(e.hp, e.max_hp);
                            (
                                tier,
                                (e.build_remaining == 0)
                                    .then(|| crate::damage::building_overlay_key(key, tier))
                                    .flatten(),
                            )
                        } else {
                            let tier = crate::damage::machine_tier(e.hp, e.max_hp);
                            (
                                tier,
                                crate::damage::machine_overlay_key(
                                    key,
                                    (9 - e.facing % 8) % 8,
                                    tier,
                                ),
                            )
                        };
                        if let Some(overlay) = overlay {
                            atlas.draw_scaled(&mut self.canvas, &overlay, x, y, false, scale);
                        }
                        if e.kind.is_building()
                            && e.build_remaining == 0
                            && crate::daylight::lamps(visual_tick) > 0
                        {
                            atlas.draw_scaled(
                                &mut self.canvas,
                                &format!("{key}_lamps"),
                                x,
                                y,
                                false,
                                scale,
                            );
                        }
                        let severe = if e.kind.is_building() {
                            tier >= 2
                        } else {
                            tier >= 1 && i64::from(e.hp) * 100 < i64::from(e.max_hp.max(1)) * 30
                        };
                        if severe {
                            let top = animation
                                .as_ref()
                                .and_then(|k| atlas.sprites.get(k))
                                .or_else(|| atlas.sprites.get(&directional))
                                .or_else(|| atlas.sprites.get(key))
                                .map_or(30, |s| scale.apply(s.anchor_y - s.opaque_top as i32));
                            crate::damage::draw_wisp(
                                &mut self.canvas,
                                x,
                                y - top,
                                visual_tick,
                                e.id,
                                faction,
                                crate::wind::LEAN_X,
                            );
                        }
                        for (mark, (dx, dy)) in self.repair_marks.active(e.id, visual_tick, faction)
                        {
                            atlas.draw(&mut self.canvas, &mark, x + dx, y + dy, false);
                        }
                    }
                    self.canvas.rect(x - 2, y + 3, 4, 2, color);
                    let stalled = e.kind.is_building()
                        && e.build_remaining > 0
                        && self.world.site_builder(e.id) == bw_sim::SiteBuilder::None;
                    let queued_site = e.kind.is_building()
                        && e.build_remaining > 0
                        && self.world.site_builder(e.id) == bw_sim::SiteBuilder::Queued;
                    entity_overlays.push(EntityOverlay {
                        owner: e.owner,
                        stalled,
                        queued_site,
                        x,
                        y,
                        selected,
                        kind: e.kind,
                        asset_key: key,
                        hp: e.hp,
                        max_hp: e.max_hp.max(1),
                        build_remaining: e.build_remaining,
                        directional,
                        animation,
                        scale,
                    });
                }
            }
        }
        // A machine hidden behind a building keeps a ghost of itself through
        // the building's opaque pixels, tinted by ownership, so it stays
        // visible and can be targeted. The ghost is drawn only where a
        // building painted later covers it.
        if let Some(atlas) = &self.atlas {
            for (i, unit) in entity_overlays.iter().enumerate() {
                if unit.kind.is_building() {
                    continue;
                }
                let Some(unit_key) = drawn_key(atlas, unit) else {
                    continue;
                };
                let tint = crate::seats::seat_colour(&self.world, unit.owner);
                for building in entity_overlays[i + 1..]
                    .iter()
                    .filter(|b| b.kind.is_building())
                {
                    let Some(building_key) = drawn_key(atlas, building) else {
                        continue;
                    };
                    if !sprites_overlap(atlas, unit, unit_key, building, building_key) {
                        continue;
                    }
                    atlas.draw_ghost_through(
                        &mut self.canvas,
                        unit_key,
                        unit.x,
                        unit.y,
                        unit.scale,
                        building_key,
                        building.x,
                        building.y,
                        building.scale,
                        tint,
                    );
                }
            }
        }
        if let (Some(atlas), Some(before)) = (&self.atlas, drawn_before_station) {
            let units: Vec<crate::trial12_field::GhostUnit> = entity_overlays[..before]
                .iter()
                .filter(|unit| !unit.kind.is_building())
                .filter_map(|unit| {
                    drawn_key(atlas, unit).map(|key| crate::trial12_field::GhostUnit {
                        key: key.to_string(),
                        x: unit.x,
                        y: unit.y,
                        scale: unit.scale,
                        tint: crate::seats::seat_colour(&self.world, unit.owner),
                    })
                })
                .collect();
            crate::trial12_field::ghost_behind_station(
                atlas,
                &mut self.canvas,
                &units,
                &station_parts,
            );
        }
        for overlay in entity_overlays {
            let EntityOverlay {
                owner,
                stalled,
                queued_site,
                x,
                y,
                selected,
                kind,
                asset_key,
                hp,
                max_hp,
                build_remaining,
                directional,
                animation,
                scale,
            } = overlay;
            if self.ux.preferences.always_health_bars
                || selected
                || hp < max_hp
                || build_remaining > 0
            {
                let height = self.atlas.as_ref().and_then(|a| {
                    let current = animation
                        .as_ref()
                        .and_then(|key| a.sprites.get(key))
                        .or_else(|| a.sprites.get(&directional))
                        .or_else(|| a.sprites.get(asset_key))
                        .map(|s| s.anchor_y - s.opaque_top as i32 + 6);
                    if kind.is_building() && build_remaining == 0 {
                        // Moving pennants and steam must not make the health
                        // bar bob. Reserve the tallest extent of the whole loop.
                        (0..4)
                            .filter_map(|phase| {
                                a.sprites.get(&format!("{asset_key}_active_{phase}"))
                            })
                            .map(|s| s.anchor_y - s.opaque_top as i32 + 6)
                            .chain(current)
                            .max()
                    } else {
                        current
                    }
                });
                let by =
                    y - scale.apply(height.unwrap_or(if kind.is_building() { 63 } else { 38 }));
                self.canvas.rect(x - 16, by, 32, 4, INK);
                let frac = (hp.max(0) * 30 / max_hp).clamp(0, 30);
                // An enemy's bar is the enemy's colour: a jade bar over a
                // red-edged machine read as one of ours.
                let bar = if owner != 0 {
                    crate::ground_marks::plate_tone(crate::seats::seat_hue(&self.world, owner)).0
                } else if frac < 9 {
                    RED
                } else {
                    JADE
                };
                self.canvas.rect(x - 15, by + 1, frac, 2, bar);
                if build_remaining > 0 && owner == 0 {
                    // A site without a builder says so with a red badge at
                    // the end of its bar: right-click it with a worker to
                    // resume.  The words are under the pointer.  An
                    // attacker reads nothing off an enemy site.
                    if stalled {
                        crate::field_labels::draw_badge(&mut self.canvas, "!", x + 21, by - 2, RED);
                    }
                    let area = crate::field_labels::Area::around(
                        (x, y),
                        scale.apply(26),
                        y - by + 6,
                        scale.apply(26),
                        scale.apply(8),
                    );
                    let tip = crate::field_labels::FieldTip::new(
                        area,
                        (x, by - 4),
                        kind.name(),
                        if stalled { RED } else { GOLD },
                    )
                    .icon(format!("ui_build_{}", kind.asset(self.faction)));
                    self.field_tips.push(if stalled {
                        tip.tag("SITE")
                            .warn("No builder: right-click with a worker.")
                    } else if queued_site {
                        tip.tag("SITE / BUILDER ON THE WAY")
                    } else {
                        tip.tag("SITE / BUILDING")
                    });
                }
            }
            if owner != 0 && kind.is_building() && build_remaining == 0 {
                // An enemy building in sight is named under the pointer:
                // the headquarters was the biggest orange building until a
                // seat guessed.
                let top = y - scale.apply(63);
                self.field_tips.push(
                    crate::field_labels::FieldTip::new(
                        crate::field_labels::Area::around(
                            (x, y),
                            scale.apply(28),
                            y - top,
                            scale.apply(28),
                            scale.apply(8),
                        ),
                        (x, top),
                        kind.name(),
                        enemy_edge(owner),
                    )
                    .icon(format!(
                        "ui_build_{}",
                        kind.asset(self.world.players[usize::from(owner)].faction)
                    ))
                    .tag(format!("ENEMY {}", crate::hover_card::role_tag(kind))),
                );
            }
            if venting && owner == 0 && !kind.is_building() && spec(kind).damage > 0 {
                // VENT shows on the machines it speeds: a gold ring at the feet.
                for (dx, dy) in [(0, 0), (1, 0)] {
                    let (cx, cy) = (x + dx, y + dy);
                    self.canvas.line(cx - 16, cy, cx, cy - 8, GOLD);
                    self.canvas.line(cx, cy - 8, cx + 16, cy, GOLD);
                    self.canvas.line(cx + 16, cy, cx, cy + 8, GOLD);
                    self.canvas.line(cx, cy + 8, cx - 16, cy, GOLD);
                }
            }
        }
        // A well names itself under the pointer; its pump is its mark.
        for well in &self.world.map.wells {
            let (px, py) = camera.project(*well);
            let taken = self.world.entities.iter().any(|e| {
                e.hp > 0
                    && e.kind == Kind::Condenser
                    && e.pos.distance_sq(*well) <= i64::from(FP).pow(2)
            });
            let tip = crate::field_labels::FieldTip::new(
                crate::field_labels::Area::around((px, py), 16, 30, 16, 8),
                (px, py - 30),
                "WELL",
                JADE,
            )
            .icon("ui_build_condenser")
            .tag("PRESSURE");
            self.field_tips.push(if taken {
                tip.line("A condenser stands on it.")
            } else {
                tip.line("Build a condenser here.")
            });
        }
        // The wrecks in sight: a small mark under each heap, its salvage
        // under the pointer.  One seat spent twenty minutes not knowing
        // where the salvage lay, so salvage always keeps its pips.
        // A heap's pips step aside while a fight is on beside it (trial
        // 12: salvage plates over the station fight).
        let shots: Vec<Pos> = self
            .effects
            .iter()
            .filter(|effect| !effect.death && effect.until > visual_tick)
            .map(|effect| effect.to)
            .collect();
        for heap in crate::field_labels::heaps(&wreck_marks) {
            let at = camera.unproject(heap.foot.0, heap.foot.1 - 6);
            if !crate::trial12_field::fight_near(&self.world, &shots, at) {
                crate::field_labels::draw_heap_mark(&mut self.canvas, &heap);
            }
            self.field_tips.push(heap.tip());
        }
        // GLINT: the heliograph flash at the Glinter's mirror for the first
        // moments, the ray's spine on the ground fading over its five
        // seconds, motes of light running out along it and a ring where its
        // reach ends (glint_fx, art v23). Only your own rays: the light is
        // yours.
        let total = u64::from(bw_content::GLINT_TICKS);
        let rays: Vec<_> = self
            .world
            .beacons
            .iter()
            .filter(|b| b.owner == 0)
            .filter_map(|beacon| {
                let end = beacon.ray_to?;
                let left = beacon.until.saturating_sub(visual_tick);
                let age = total.saturating_sub(left);
                let (ax, ay) = camera.project(beacon.pos);
                // The mirror of the Glinter that cast it, while it still
                // stands there: the muzzle authored on its firing pose.
                let mirror = self
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.owner == 0 && e.hp > 0 && e.kind == Kind::Glinter)
                    .filter(|e| e.pos.distance_sq(beacon.pos) <= 4 * i64::from(FP).pow(2))
                    .min_by_key(|e| e.pos.distance_sq(beacon.pos))
                    .and_then(|e| {
                        let key = format!("{}_fire_0", self.directional_key(e));
                        let [dx, dy] = self.atlas.as_ref()?.sprites.get(&key)?.muzzle?;
                        let (x, y) = camera.project(e.pos);
                        Some((x + dx, y + dy))
                    })
                    .unwrap_or((ax, ay - 40));
                Some(((ax, ay), camera.project(end), mirror, age, left))
            })
            .collect();
        for (a, b, mirror, age, left) in rays {
            let (core, edge) = crate::glint_fx::spine_colours(left, total);
            self.canvas.line(a.0, a.1 - 1, b.0, b.1 - 1, edge);
            self.canvas.line(a.0, a.1 + 1, b.0, b.1 + 1, edge);
            self.canvas.line(a.0, a.1, b.0, b.1, core);
            let Some(atlas) = &self.atlas else {
                continue;
            };
            for (key, at) in crate::glint_fx::motes(age, left) {
                let (x, y) = crate::glint_fx::along(a, b, at);
                atlas.draw(&mut self.canvas, &key, x, y, false);
            }
            let spot = crate::glint_fx::spot_key(age);
            atlas.draw(&mut self.canvas, &spot, b.0, b.1, false);
            if let Some(key) = crate::glint_fx::flash_key(age) {
                atlas.draw(&mut self.canvas, &key, mirror.0, mirror.1, false);
            }
        }
        // SOUND rings widen on the ground; VENT plumes rise from the stack.
        if let Some(atlas) = &self.atlas {
            for (pos, until) in &self.pings {
                let age = (u64::from(bw_content::SOUND_TICKS))
                    .saturating_sub(until - visual_tick.min(*until));
                let phase = ((age / 5) % 4) as usize;
                let (px, py) = camera.project(*pos);
                atlas.draw(
                    &mut self.canvas,
                    &format!("fx_sound_{phase}"),
                    px,
                    py,
                    false,
                );
            }
            for id in self.vents.keys() {
                if let Some(e) = self.world.entities.iter().find(|e| e.id == *id) {
                    let (px, py) = camera.project(e.pos);
                    let phase = ((visual_tick / 4 + u64::from(*id)) % 4) as usize;
                    atlas.draw(
                        &mut self.canvas,
                        &format!("fx_vent_{phase}"),
                        px,
                        py - 70,
                        false,
                    );
                }
            }
        }
        // The rally of a selected producer: a line from the door to a
        // pennant on the ground, so new machines are never sent blind.
        let rallies: Vec<(Pos, Pos)> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0 && e.hp > 0 && e.aboard.is_none() && self.selected.contains(&e.id)
            })
            .filter_map(|e| e.rally.map(|rally| (e.pos, rally)))
            .collect();
        for (from, to) in rallies {
            let (fx, fy) = camera.project(from);
            let (tx, ty) = camera.project(to);
            self.canvas.line(fx, fy, tx, ty, [222, 190, 90, 110]);
            self.canvas.diamond(tx, ty, 7, 3, [222, 190, 90, 120]);
            self.canvas.line(tx, ty, tx, ty - 17, WHITE);
            for dy in 0..7 {
                let width = if dy <= 3 {
                    dy * 2 + 1
                } else {
                    (6 - dy) * 2 + 1
                };
                self.canvas.rect(tx + 1, ty - 17 + dy, width, 1, GOLD);
            }
            self.field_tips.push(
                crate::field_labels::FieldTip::new(
                    crate::field_labels::Area::around((tx, ty), 8, 20, 12, 5),
                    (tx, ty - 20),
                    "RALLY",
                    GOLD,
                )
                .line("New machines walk here."),
            );
        }
        let (gx, gy) = camera.project(map.gate_pos);
        let gate_color = match self.world.gate.owner {
            Some(seat) => crate::seats::seat_colour(&self.world, seat),
            None => GOLD,
        };
        self.canvas.diamond(gx, gy + 3, 5, 2, gate_color);
        // The capture bar is in the capturer's colour, not the holder's.
        let capture_color = match self.world.gate.capture_player {
            Some(seat) => crate::seats::seat_colour(&self.world, seat),
            None => gate_color,
        };
        if self.world.gate.capture_progress > 0 {
            let gate_key = crate::station3::station_key(&self.world);
            let height = self
                .atlas
                .as_ref()
                .and_then(|atlas| {
                    atlas
                        .sprites
                        .get(&gate_key)
                        .or_else(|| atlas.sprites.get("gate"))
                })
                .map_or(43, |s| s.anchor_y - s.opaque_top as i32 + 6);
            self.canvas.rect(gx - 23, gy - height, 46, 4, INK);
            self.canvas.rect(
                gx - 22,
                gy - height + 1,
                (44 * self.world.gate.capture_progress / self.world.capture_work()).min(44) as i32,
                2,
                capture_color,
            );
        }
        for effect in &self.effects {
            if effect.until <= visual_tick || !self.world.visible(0, effect.to) {
                continue;
            }
            let (x, y) = camera.project(effect.to);
            let source_visible = self.world.visible(0, effect.from);
            let (sx, sy) = camera.project(if source_visible {
                effect.from
            } else {
                effect.to
            });
            if effect.death {
                let age = (presentation::WRECK_TICKS
                    - effect
                        .until
                        .saturating_sub(visual_tick)
                        .min(presentation::WRECK_TICKS)) as i32;
                let phase = (age / 4).min(5);
                let drawn = self.atlas.as_ref().is_some_and(|atlas| {
                    atlas.draw(
                        &mut self.canvas,
                        &format!("fx_wreck_{}_{}", effect.material.key(), phase),
                        x,
                        y,
                        false,
                    ) || atlas.draw(&mut self.canvas, &format!("fx_wreck_{phase}"), x, y, false)
                });
                if drawn {
                    continue;
                }
                let r = 3 + age / 3;
                self.canvas.ellipse(x, y - 8, r, r / 2, [153, 98, 57, 190]);
                self.canvas.line(x - r, y - 8, x - 2, y - 11, GOLD);
                self.canvas.line(x + 2, y - 5, x + r, y - 2, RED);
            } else {
                let age = (presentation::IMPACT_TICKS
                    - effect
                        .until
                        .saturating_sub(visual_tick)
                        .min(presentation::IMPACT_TICKS)) as i32;
                let (source_x, source_y) = effect
                    .muzzle
                    .filter(|_| source_visible)
                    .map(|[dx, dy]| (sx + dx, sy + dy))
                    .unwrap_or((sx, sy - 12));
                if let Some(damage) = effect.beam {
                    Self::draw_beam(
                        &mut self.canvas,
                        self.atlas.as_ref(),
                        (source_x, source_y),
                        (x, y - 12),
                        damage,
                        age,
                        source_visible,
                    );
                    continue;
                }
                let drawn = self.atlas.as_ref().is_some_and(|atlas| {
                    if age < 6 && source_visible && effect.from != effect.to {
                        atlas.draw(
                            &mut self.canvas,
                            &format!("fx_muzzle_{}", age / 2),
                            source_x,
                            source_y,
                            false,
                        );
                    }
                    atlas.draw(
                        &mut self.canvas,
                        &format!("fx_impact_{}_{}", effect.material.key(), (age / 3).min(3)),
                        x,
                        y - 12,
                        false,
                    ) || atlas.draw(
                        &mut self.canvas,
                        &format!("fx_impact_{}", (age / 3).min(3)),
                        x,
                        y - 12,
                        false,
                    )
                });
                if age < 3 || !drawn {
                    self.canvas.line(source_x, source_y, x, y - 12, GOLD);
                }
                if !drawn {
                    self.canvas.line(x - 3, y - 12, x + 3, y - 12, WHITE);
                    self.canvas.line(x, y - 15, x, y - 9, GOLD);
                }
            }
        }
        if self.mode == Mode::Lay {
            for (px, py, color) in self.lay_preview(camera) {
                self.canvas.diamond(px, py, 15, 7, color);
            }
        }
        if let Mode::Build(kind) = self.mode {
            let origin = self.build_origin(kind, camera.unproject(self.cursor.0, self.cursor.1));
            // Sites already placed this run are shaded gold; the shade is
            // their mark.
            for (px, py, color) in self.placement_help(camera, origin) {
                self.canvas.diamond(px, py, 15, 7, color);
            }
            // A nest's valid sites in the mouth ring it stands at (trial 11).
            for (px, py, color) in crate::trial11_words::ring_sites(self, camera, kind, origin) {
                self.canvas.diamond(px, py, 15, 7, color);
            }
            let (cx, cy) = origin.cell_xy();
            let (x, y) = camera.project(origin);
            let valid = self.placement_reason(kind, origin).is_none();
            let n = spec(kind).footprint;
            if kind == Kind::Condenser {
                // Every free well is a candidate site: ring each one on the
                // field, fill its footprint and point at it (trial 12: the
                // wells were still hard to find).
                let pulse = if (visual_tick / 15).is_multiple_of(2) {
                    GOLD
                } else {
                    WHITE
                };
                let wells = crate::trial12_field::free_wells(&self.world);
                self.point_to_wells(camera, &wells, visual_tick);
                for well in &wells {
                    let (px, py) = camera.project(*well);
                    if !(-40..self.canvas.width() as i32 + 40).contains(&px)
                        || !(-40..self.canvas.height() as i32 + 40).contains(&py)
                    {
                        continue;
                    }
                    for (rx, ry) in [(24, 12), (27, 13)] {
                        self.canvas.line(px - rx, py, px, py - ry, pulse);
                        self.canvas.line(px, py - ry, px + rx, py, pulse);
                        self.canvas.line(px + rx, py, px, py + ry, pulse);
                        self.canvas.line(px, py + ry, px - rx, py, pulse);
                    }
                    // A beacon pole reads from across the base.
                    self.canvas.line(px, py - 14, px, py - 44, pulse);
                    self.canvas.line(px + 1, py - 14, px + 1, py - 44, INK);
                }
            }
            // Each cell answers for itself, so a refused site shows which
            // cells are wrong and how far to move.  A refusal of the whole
            // footprint (a condenser off its well) turns every cell red.
            let cells: Vec<(Pos, bool)> = (0..n)
                .flat_map(|a| (0..n).map(move |b| Pos::cell(cx + a, cy + b)))
                .map(|cell| {
                    let ok = self
                        .placement_cell_refusal(cell, kind != Kind::Condenser)
                        .is_none();
                    (cell, ok)
                })
                .collect();
            let cells_ok = cells.iter().all(|(_, ok)| *ok);
            for (cell, ok) in cells {
                let (px, py) = camera.project(cell);
                let cell_col = if valid || (!cells_ok && ok) {
                    JADE
                } else {
                    RED
                };
                self.canvas
                    .diamond(px, py, 15, 7, [cell_col[0], cell_col[1], cell_col[2], 90]);
            }
            // The building itself stands on the site, half there: the
            // player sees its size and face before paying.  A refused site
            // shows it reddened.  No shadow is drawn beneath it.
            if let Some(atlas) = &self.atlas {
                let faction = self.world.players[0].faction;
                let t = crate::occlusion::render_transform(kind, origin, faction);
                let (gx, gy) = camera.project(t.anchor);
                atlas.draw_ghost(
                    &mut self.canvas,
                    kind.asset(faction),
                    gx + t.render_x_offset,
                    gy + t.render_y_offset,
                    t.scale,
                    if valid { None } else { Some(RED) },
                );
            }
            // Near a crossing mouth the ghost shows the hold's ring, gold
            // while the site stands inside it (trial 10: three nests were
            // built just outside a ring nobody could see while placing).
            let mouth = self.placement_mouth(kind, origin);
            if let Some((_, point, inside)) = mouth {
                let ring_color = if inside { GOLD } else { WHITE };
                let reach = bw_content::CROSSING_HOLD_RADIUS_CELLS * FP;
                let points: Vec<(i32, i32)> = (0..32)
                    .map(|i| {
                        let a = f64::from(i) * std::f64::consts::TAU / 32.0;
                        camera.project(Pos {
                            x: point.x + (f64::from(reach) * a.cos()) as i32,
                            y: point.y + (f64::from(reach) * a.sin()) as i32,
                        })
                    })
                    .collect();
                for i in 0..points.len() {
                    let (ax, ay) = points[i];
                    let (bx, by) = points[(i + 1) % points.len()];
                    self.canvas.line(ax, ay, bx, by, ring_color);
                    self.canvas.line(ax, ay + 1, bx, by + 1, ring_color);
                }
            }
            // A refused site says in two words why, at the site; a good
            // one near a mouth says whether it stands in the ring and that
            // a building never holds it (nests do not count: only machines
            // do); elsewhere a good site needs no words (its cells are
            // jade).
            self.placement_tip = self
                .placement_refusal(kind, origin)
                .map(|refusal| {
                    crate::field_labels::FieldTip::new(
                        crate::field_labels::Area::around((x, y), 0, 0, 0, 0),
                        (x, y - 30),
                        crate::ux::building_name(kind, self.faction),
                        RED,
                    )
                    .warn(match refusal {
                        crate::ux::Refusal::Occupied => {
                            crate::trial11_words::occupied_plate(self, kind, origin)
                        }
                        _ => refusal.short().to_string(),
                    })
                })
                .or_else(|| {
                    mouth.map(|(lane, _, inside)| {
                        crate::field_labels::FieldTip::new(
                            crate::field_labels::Area::around((x, y), 0, 0, 0, 0),
                            (x, y - 30),
                            format!(
                                "{} {} MOUTH RING",
                                if inside { "IN" } else { "OUTSIDE" },
                                crate::seats::arm_letter(&self.world, lane)
                            ),
                            if inside { GOLD } else { WHITE },
                        )
                        .warn(match kind {
                            // Rules 22: with three seats a nest only slows.
                            Kind::Tower if self.world.nests_slow_counts() => {
                                "Nests slow a count, never hold"
                            }
                            Kind::Tower => "Nests block, but don't hold",
                            _ => "Buildings don't hold it",
                        })
                        .line(
                            if kind == Kind::Tower && self.world.nests_slow_counts() {
                                "It halves an enemy count here. Only machines hold."
                            } else if kind == Kind::Tower {
                                "It stops an enemy count here. Only machines hold."
                            } else {
                                "Only machines hold a mouth."
                            },
                        )
                    })
                });
        }
    }
    pub(crate) fn console_state(&self) -> ConsoleState {
        let player = &self.world.players[0];
        let condenser_count = self
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.owner == 0
                    && entity.kind == Kind::Condenser
                    && entity.hp > 0
                    && entity.build_remaining == 0
            })
            .count() as u32;
        let selected_entities: Vec<&bw_sim::Entity> = self
            .selected
            .iter()
            .filter_map(|id| self.world.entities.iter().find(|entity| entity.id == *id))
            .collect();
        let selection = selected_entities.first().copied();
        let selection = selection.map(|entity| {
            let producers = selected_entities.len() > 1
                && selected_entities
                    .iter()
                    .all(|e| e.owner == 0 && e.kind == Kind::Works);
            let status = if producers {
                format!(
                    "{} WORKS / {} QUEUED",
                    selected_entities.len(),
                    selected_entities
                        .iter()
                        .map(|e| e.queue.len())
                        .sum::<usize>()
                )
            } else if let Some(status) = self.tactical_selection_status(&selected_entities) {
                status
            } else if selected_entities.len() > 1 {
                format!(
                    "{} MACHINES / {} CARRYING",
                    selected_entities.len(),
                    selected_entities
                        .iter()
                        .filter(|entity| entity.carried > 0)
                        .count()
                )
            } else if entity.kind.is_building() && entity.build_remaining > 0 {
                format!(
                    "BUILDING / {}S REMAINING",
                    entity.build_remaining.div_ceil(30)
                )
            } else if entity.kind.is_building() {
                entity
                    .queue
                    .first()
                    .map(|production| {
                        if production.started && production.remaining <= 1 {
                            "READY / AWAITING RELEASE".to_string()
                        } else {
                            format!(
                                "QUEUE {} / NEXT {}S",
                                entity.queue.len(),
                                production.remaining.div_ceil(30)
                            )
                        }
                    })
                    .unwrap_or_else(|| "QUEUE EMPTY".to_string())
            } else if entity.kind.is_transport() {
                let hold: Vec<Kind> = entity
                    .cargo
                    .iter()
                    .filter_map(|id| self.world.entities.iter().find(|e| e.id == *id))
                    .map(|e| e.kind)
                    .collect();
                if hold.is_empty() {
                    format!("EMPTY HOLD 0/{}", bw_content::TRANSPORT_CAPACITY)
                } else {
                    format!(
                        "HOLD {}/{}: {}",
                        hold.len(),
                        bw_content::TRANSPORT_CAPACITY,
                        crate::dock_qol::composition_label(&crate::dock_qol::composition(hold))
                    )
                }
            } else if entity.kind.is_worker() && entity.carried > 0 {
                crate::trial11_words::worker_load_words(&self.world, entity)
            } else if entity.kind.is_worker() {
                match entity.order {
                    Order::Gather { .. } => "GATHERING",
                    Order::Build { .. } => "BUILDING",
                    Order::Repair { .. } => "REPAIRING",
                    Order::Move { .. } => "MOVING",
                    // Pulled back from a wreck under fire (rules 22): it
                    // goes back when the danger passes.
                    Order::Deliver { .. } => "WAITING",
                    Order::Idle => "IDLE",
                    _ => "READY",
                }
                .to_string()
            } else if entity.deploy_remaining > 0 {
                if entity.deploy_target {
                    "DEPLOYING"
                } else {
                    "PACKING"
                }
                .to_string()
            } else if entity.deployed {
                "DEPLOYED".to_string()
            } else {
                match entity.order {
                    Order::Capture => "CAPTURING",
                    Order::Hold => "HOLDING",
                    Order::Attack { .. } | Order::AttackMove { .. } => "ATTACKING",
                    Order::Move { .. } => "MOVING",
                    Order::Lay { .. } => "LAYING CRUST",
                    Order::Idle => "IDLE",
                    _ => "READY",
                }
                .to_string()
            };
            let queue = entity
                .queue
                .iter()
                .map(|production| ProductionTicket {
                    label: production.kind.name().to_string(),
                    remaining_ticks: production.remaining,
                    total_ticks: spec(production.kind).build_ticks,
                    started: production.started,
                })
                .collect();
            ConsoleSelection {
                title: if selected_entities.len() == 1 {
                    crate::ux::building_name(
                        entity.kind,
                        self.world.players[entity.owner as usize].faction,
                    )
                    .to_string()
                } else if producers {
                    "PRODUCTION GROUP".to_string()
                } else {
                    "TASK FORCE".to_string()
                },
                role: if selected_entities.len() == 1 {
                    crate::ux::role_description(entity.kind).to_string()
                } else {
                    String::new()
                },
                stats: if selected_entities.len() == 1 {
                    crate::ux::stats_line(entity.kind)
                } else {
                    String::new()
                },
                portrait_key: (!entity.kind.is_building())
                    .then(|| {
                        format!(
                            "portrait_{}",
                            entity
                                .kind
                                .asset(self.world.players[entity.owner as usize].faction)
                        )
                    })
                    .filter(|_| selected_entities.len() == 1),
                selected_count: selected_entities.len(),
                hp: selected_entities.iter().fold(0i32, |total, selected| {
                    total.saturating_add(selected.hp.max(0))
                }),
                max_hp: selected_entities.iter().fold(0i32, |total, selected| {
                    total.saturating_add(selected.max_hp.max(0))
                }),
                status,
                queue,
                show_queue: entity.kind.is_building()
                    && entity.build_remaining == 0
                    && selected_entities.len() == 1
                    && player
                        .research
                        .as_ref()
                        .is_none_or(|r| r.building != entity.id),
            }
        });
        // Deployed Pans boil PAN_PRESSURE_PER_MINUTE each (rules 18).
        let boiling_pans = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.kind == Kind::Pan
                    && e.hp > 0
                    && e.deployed
                    && e.deploy_remaining == 0
            })
            .count() as u32;
        ConsoleState {
            faction: player.faction,
            elapsed_seconds: self.world.tick / 30,
            resources: ConsoleResources {
                salvage: player.salvage,
                pressure: player.pressure,
                salvage_rate_per_minute: {
                    let gathered: u32 = self.deposits.iter().map(|(_, a)| *a).sum();
                    let window = self.world.tick.clamp(1, 1800);
                    // The HQ's and the station's trickle is income too.
                    Some(
                        (u64::from(gathered) * 1800 / window) as u32
                            + crate::trial11_words::trickle_per_minute(&self.world, 0),
                    )
                },
                pressure_rate_per_minute: Some(
                    (1 + condenser_count * 2) * 60
                        + if self.world.gate.owner == Some(0) {
                            bw_content::SLUICE_PRESSURE_PER_MINUTE
                        } else {
                            0
                        }
                        + if player.upgrades.contains(&bw_content::Upgrade::BleedValves) {
                            60
                        } else {
                            0
                        }
                        + bw_content::PAN_PRESSURE_PER_MINUTE * boiling_pans,
                ),
                pressure_cap: Some(player.pressure_cap),
                crew: player.crew,
                crew_cap: player.cap,
            },
            route: ConsoleRoute {
                tide: self.world.gate.tide,
                flood_pending: self.world.gate.flood_pending,
                ebb_pending: self.world.gate.ebb_pending,
                flood_ticks_remaining: self
                    .world
                    .gate
                    .flood_until
                    .map(|until| until.saturating_sub(self.world.tick)),
                dry_arm: self.world.gate.dry_arm,
                target_dry_arm: self.world.gate.switch_target,
                map: self.world.map.id,
                warning_ticks_remaining: self
                    .world
                    .gate
                    .warning_until
                    .map(|until| until.saturating_sub(self.world.tick)),
            },
            selection,
        }
    }
    fn draw_hud(&mut self) {
        let console_state = self.console_state();
        crate::console::draw_top_header(&mut self.canvas, self.atlas.as_ref(), &console_state);
        crate::console::draw_bottom_console(&mut self.canvas, self.atlas.as_ref(), &console_state);
        crate::dock_log::draw_telegraph(
            &mut self.canvas,
            self.atlas.as_ref(),
            &self.ux.dock.telegraph,
            141,
            332,
        );
        if self
            .selected
            .first()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
            .is_some_and(|e| e.owner == 0 && e.kind == Kind::Headquarters && e.build_remaining == 0)
            && let Some(doctrine) = self.world.players[0].doctrine
        {
            crate::dock_log::draw_doctrine_plate(
                &mut self.canvas,
                self.atlas.as_ref(),
                self.faction,
                doctrine,
                149,
                300,
            );
        }
        if self.screen == Screen::Match {
            crate::console::draw_route_indicator(&mut self.canvas, &console_state.route);
        }
        self.draw_minimap();
        self.collect_command_buttons();
        self.draw_tactical_controls();
        self.buttons.push(Button {
            x: 554,
            y: 49,
            w: 79,
            h: 18,
            label: "SWITCH".into(),
            hint: String::new(),
            action: Action::Switch,
            enabled: self.world.gate.owner == Some(0)
                && self.world.gate.warning_until.is_none()
                && self.world.tick >= self.world.gate.locked_until,
        });
        self.canvas
            .text("ESC PAUSE / F1 GUIDE / F5 SAVE", 201, 348, MUTED);
        self.buttons.push(Button {
            x: 596,
            y: 2,
            w: 38,
            h: 20,
            label: "Menu".into(),
            hint: String::new(),
            action: Action::Back,
            enabled: true,
        });
    }
    pub(crate) fn collect_command_buttons(&mut self) {
        let Some(e) = self
            .world
            .entities
            .iter()
            .find(|e| self.selected.first() == Some(&e.id))
            .cloned()
        else {
            return;
        };
        let kind = e.kind;
        let queue = e.queue.len();
        let faction = self.faction;
        if kind.is_building() && e.build_remaining > 0 {
            self.command_button(0, "CANCEL", "75% REFUND", Action::Cancel, true);
            return;
        }
        if kind == Kind::Headquarters {
            // The worker's name is the side's own word (WICK, HOOK): the
            // button says what it is.
            self.command_button(
                0,
                &format!("Q {}", faction.worker().name()),
                &format!("{}\nWORKER", cost(faction.worker())),
                Action::Train(faction.worker()),
                true,
            );
            self.command_button(1, "CANCEL", "BACKSPACE", Action::Cancel, queue > 0);
            // The scout from the first minute (rules 15), not only from
            // the Drydock: both seats of the eighth trial asked for one.
            self.command_button(
                7,
                &format!("W {}", faction.scout().name()),
                &format!("{}\nSCOUT", cost(faction.scout())),
                Action::Train(faction.scout()),
                true,
            );
            self.draw_research_buttons();
            let vent = self.world.players[0].vent_remaining;
            self.command_button(
                5,
                "VENT",
                &if vent > 0 {
                    format!("X\nOPEN {}S", vent.div_ceil(30))
                } else {
                    format!("X {}P\nFIRE +30%", bw_content::VENT_PRESSURE)
                },
                Action::Vent,
                true,
            );
            self.command_button(
                6,
                "RECLAIM",
                &format!(
                    "R {}P\n{} SALVAGE",
                    bw_content::RECLAIM_PRESSURE,
                    bw_content::RECLAIM_SALVAGE
                ),
                Action::Reclaim,
                true,
            );
            self.overhaul_button(8);
            return;
        }
        if kind.produces() {
            let roles: Vec<Kind> = if kind == Kind::Works {
                faction.army().to_vec()
            } else {
                crate::trial11_controls::drydock_keys(faction).to_vec()
            };
            for (i, k) in roles.into_iter().enumerate() {
                // A role whose core mechanic is not obvious says so on the
                // button; four Looms once marched to their deaths undeployed.
                let effect = match k {
                    Kind::Loom => "D TO FIRE",
                    Kind::Bulwark => "D SHIELDS",
                    Kind::Caisson => "D BLOCKS",
                    Kind::Caulker | Kind::Tender => "MENDS",
                    Kind::Tidewatch | Kind::Lampwright => "SCOUT",
                    Kind::Dredger => "WET WRECKS",
                    Kind::Sounder | Kind::Skipper => "X SOUNDS",
                    Kind::Barge => "4 BY WATER",
                    Kind::Lifter => "4 BY AIR",
                    _ => "",
                };
                let hint = if effect.is_empty() {
                    cost(k)
                } else {
                    format!("{}\n{effect}", cost(k))
                };
                self.command_button(
                    i,
                    &format!("{} {}", ["Q", "W", "E", "R"][i], k.name()),
                    &hint,
                    Action::Train(k),
                    true,
                );
            }
            self.command_button(3, "CANCEL", "BACKSPACE", Action::Cancel, queue > 0);
            self.upgrade_buttons(kind, 4);
            return;
        }
        if kind.is_building() {
            self.upgrade_buttons(kind, 0);
            return;
        }
        if kind.is_worker() {
            for (i, (key, k)) in [
                ("B", Kind::Works),
                ("C", Kind::Condenser),
                ("Y", Kind::Dropoff),
                ("V", Kind::Tower),
                ("N", Kind::Drydock),
                ("P", Kind::Palisade),
            ]
            .into_iter()
            .enumerate()
            {
                let label = match k {
                    Kind::Condenser => "CONDENSER".to_string(),
                    Kind::Dropoff if self.faction == Faction::Compact => "SHED".to_string(),
                    Kind::Dropoff => "YARD".to_string(),
                    Kind::Tower => "NEST".to_string(),
                    other => crate::ux::building_name(other, self.faction).to_string(),
                };
                self.command_button(
                    i,
                    &format!("{key} {label}"),
                    &cost(k),
                    Action::Build(k),
                    true,
                );
            }
            self.command_button(6, "GATHER", "G", Action::Gather, true);
            self.command_button(7, "STOP", "S", Action::Stop, true);
            // With every wreck gone a worker only sits on crew: RECYCLE
            // gives the place back with half its salvage (trial 10).
            let refund = spec(faction.worker()).salvage * bw_content::RECYCLE_REFUND_PERCENT / 100;
            self.command_button(
                8,
                "RECYCLE",
                &format!("X +{refund}\nFREES CREW"),
                Action::Recycle,
                true,
            );
            return;
        }
        // Machines.  The card answers for the whole selection: a gun in it
        // makes it a fighting group, a Sounder or Skipper in it can SOUND,
        // a specialist in it can deploy, a gatherer in it can gather.
        let selected_kinds: Vec<Kind> = self
            .selected
            .iter()
            .filter_map(|id| self.world.entities.iter().find(|e| e.id == *id))
            .filter(|e| e.owner == 0 && !e.kind.is_building())
            .map(|e| e.kind)
            .collect();
        let any = |test: fn(Kind) -> bool| selected_kinds.iter().copied().any(test);
        let armed = any(|k| spec(k).damage > 0);
        let can_sound = any(|k| matches!(k, Kind::Sounder | Kind::Skipper));
        let can_glint = any(|k| k == Kind::Glinter);
        let can_lay = any(|k| k == Kind::Salter);
        let can_deploy = any(crate::controls::is_specialist);
        let can_gather = any(|k| k == Kind::Dredger);
        let can_unload = any(|k| k.is_transport());
        let mut slot = 0;
        let mut put = |game: &mut Game, label: &str, hint: &str, action: Action, enabled: bool| {
            game.command_button(slot, label, hint, action, enabled);
            slot += 1;
        };
        if armed {
            put(self, "ATTACK", "A", Action::Attack, true);
        }
        put(self, "HOLD", "H", Action::Hold, true);
        put(self, "STOP", "S", Action::Stop, true);
        if armed {
            put(self, "CAPTURE", "G", Action::Capture, true);
        }
        if can_gather {
            put(self, "GATHER", "G", Action::Gather, true);
        }
        if can_deploy {
            // One slot, two orders: D deploys whatever is packed, P packs a
            // line that is all deployed.
            if self.deploy_packs() {
                put(self, "PACK", "P", Action::Pack, true);
            } else {
                put(self, "DEPLOY", "D", Action::Deploy, true);
            }
            // KEEP has its own button, lit when on (trial 11).
            let hint = self.keep_hint();
            put(self, "KEEP", hint, Action::KeepDeployed, true);
        }
        if armed {
            // The effect belongs on the button: one seat fired Surge in
            // three defensive fights believing it was a weapon buff.
            let surge = format!("Z {}P\nNO GUNS 3S", self.surge_selection_cost());
            put(self, "SURGE", &surge, Action::Surge, true);
        }
        if can_unload {
            put(self, "UNLOAD", "U\nDROP CARGO", Action::Unload, true);
        }
        if let Some((_, transport)) = self.board_target() {
            let carrier = self
                .world
                .entities
                .iter()
                .find(|e| e.id == transport)
                .map_or("TRANSPORT", |e| e.kind.name());
            let hint = format!("E\n{carrier}");
            put(self, "BOARD", &hint, Action::Board, true);
        }
        if can_sound {
            let sound = format!(
                "X {}P\nREVEAL {}",
                bw_content::SOUND_PRESSURE,
                bw_content::SOUND_RADIUS / bw_core::FP
            );
            put(self, "SOUND", &sound, Action::Sound, true);
        }
        if can_glint {
            let glint = format!(
                "X {}P\nRAY {}",
                bw_content::GLINT_PRESSURE,
                bw_content::GLINT_LENGTH / bw_core::FP
            );
            put(self, "GLINT", &glint, Action::Glint, true);
        }
        if can_lay {
            let lay = format!("L {}P\nA ROW", bw_content::LAY_PRESSURE_PER_ROW);
            put(self, "LAY", &lay, Action::Lay, true);
        }
        let formation = format!("F {}", self.selection_formation_label());
        put(self, &formation, "FORMATION", Action::Formation, true);
        put(self, "R FACE", "TURN", Action::Face, true);
    }
    /// The OVERHAUL level the headquarters card offers next (rules 21): the
    /// first neither complete, running nor queued, else the last.
    pub(crate) fn next_overhaul(&self) -> bw_content::Upgrade {
        use bw_content::Upgrade;
        let pending = |level: &Upgrade| {
            self.world.players[0].upgrades.contains(level)
                || self.world.entities.iter().any(|e| {
                    e.owner == 0
                        && (e.upgrade.is_some_and(|job| job.upgrade == *level)
                            || e.upgrade_queue.contains(level))
                })
        };
        Upgrade::OVERHAUL
            .into_iter()
            .find(|level| !pending(level))
            .unwrap_or(Upgrade::Overhaul3)
    }
    /// The headquarters' one OVERHAUL button: the next level's price and
    /// effect, the seconds left while a level runs, DONE after the third.
    fn overhaul_button(&mut self, index: usize) {
        let next = self.next_overhaul();
        let running = self
            .first_building()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
            .and_then(|e| e.upgrade)
            .filter(|job| job.upgrade.overhaul_level().is_some());
        let hint = if let Some(job) = running {
            format!("P\n{}S LEFT", job.remaining.div_ceil(30))
        } else if self.world.players[0].upgrades.contains(&next) {
            "P\nDONE".to_string()
        } else {
            format!("P {}", crate::ux::upgrade_hint(next, self.faction))
        };
        self.command_button(index, "P OVERHAUL", &hint, Action::Upgrade(next), true);
    }
    /// The upgrade buttons a building offers, from `first` slot on: cost and
    /// effect rows, seconds left while one runs, DONE when it is complete.
    fn upgrade_buttons(&mut self, kind: Kind, first: usize) {
        let building = self
            .first_building()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == id));
        let running = building.and_then(|e| e.upgrade);
        let queued = building
            .map(|e| e.upgrade_queue.clone())
            .unwrap_or_default();
        let done = self.world.players[0].upgrades.clone();
        for (i, upgrade) in bw_content::Upgrade::offered_by(kind).iter().enumerate() {
            let key = ["P", "L", "N"][i.min(2)];
            let hint = if let Some(job) = running.filter(|job| job.upgrade == *upgrade) {
                format!("{key}\n{}S LEFT", job.remaining.div_ceil(30))
            } else if let Some(place) = queued.iter().position(|q| q == upgrade) {
                format!("{key}\nQUEUED {}", place + 1)
            } else if done.contains(upgrade) {
                format!("{key}\nDONE")
            } else {
                format!("{key} {}", crate::ux::upgrade_hint(*upgrade, self.faction))
            };
            self.command_button(
                first + i,
                &format!("{key} {}", crate::ux::upgrade_card_name(*upgrade)),
                &hint,
                Action::Upgrade(*upgrade),
                true,
            );
        }
        if running.is_some() {
            self.command_button(
                first + bw_content::Upgrade::offered_by(kind).len(),
                "CANCEL R&D",
                if queued.is_empty() {
                    "K 75% BACK"
                } else {
                    "K ALL BACK"
                },
                Action::CancelUpgrade,
                true,
            );
        }
    }
    pub(crate) fn command_button(
        &mut self,
        index: usize,
        label: &str,
        hint: &str,
        action: Action,
        enabled: bool,
    ) {
        let available = enabled && self.action_reason(&action).is_none();
        let mut title = label.to_string();
        let mut detail = hint.to_string();
        if matches!(
            action,
            Action::Train(_) | Action::Build(_) | Action::Research(_) | Action::Upgrade(_)
        ) && label.as_bytes().get(1) == Some(&b' ')
        {
            // The cost row reads "KEY 180S+30P"; an effect row after a
            // newline is kept as written so its words stay readable.  A row
            // that already leads with the key keeps one copy of it: an
            // upgrade once read "P P150S50P".
            let (cost, effect) = hint
                .split_once('\n')
                .map_or((hint, None), |(cost, effect)| (cost, Some(effect)));
            let key = &label[..1];
            let cost = cost
                .strip_prefix(key)
                .filter(|rest| rest.is_empty() || rest.starts_with(' '))
                .unwrap_or(cost)
                .replace(' ', "");
            title = label[2..].to_string();
            detail = if cost.is_empty() {
                key.to_string()
            } else {
                format!("{key} {cost}")
            };
            if let Some(effect) = effect {
                detail.push('\n');
                detail.push_str(effect);
            }
        }
        if matches!(action, Action::Build(Kind::Condenser)) {
            title = "CONDENSER".into();
        }
        self.buttons.push(Button {
            x: 414 + (index % 3) as i32 * 74,
            y: 297 + (index / 3) as i32 * 29,
            w: 70,
            h: 26,
            label: title,
            hint: detail,
            action,
            enabled: available,
        });
    }
    pub(crate) fn draw_minimap(&mut self) {
        self.canvas.rect(9, 297, 118, 57, EDGE);
        let bounds = crate::minimap_chart::ChartRect::axis(11, 299, 114, 53);
        let r = self.chart();
        crate::minimap_chart::draw_margin(&mut self.canvas, &self.world, bounds, r, 1);
        crate::minimap_chart::draw_field(&mut self.canvas, &self.world, r, 1);
        let corners = [
            self.unproject(0, self.world_view().top),
            self.unproject(self.world_view().width - 1, self.world_view().top),
            self.unproject(self.world_view().width - 1, self.world_view().bottom - 1),
            self.unproject(0, self.world_view().bottom - 1),
        ];
        for i in 0..4 {
            let (x0, y0) = r.plot(&self.world, corners[i]);
            let (x1, y1) = r.plot(&self.world, corners[(i + 1) % 4]);
            let (x0, y0) = bounds.clamp(x0, y0, 0);
            let (x1, y1) = bounds.clamp(x1, y1, 0);
            self.canvas.line(x0, y0, x1, y1, crate::minimap_chart::INK)
        }
    }
    pub(crate) fn draw_menu(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::home(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    pub(crate) fn draw_setup(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::setup(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    /// The New Skirmish cards' art goes over their buttons.
    pub(crate) fn draw_setup_cards(&mut self) {
        if self.screen == Screen::Setup {
            let model = self.menu_model();
            crate::menus::setup_cards(&mut self.canvas, self.atlas.as_ref(), &model);
        }
        if self.screen == Screen::Settings {
            self.draw_settings_extras();
        }
    }
    /// A volume drag has ended: save the level it left.
    pub(crate) fn end_volume_drag(&mut self) {
        if self.ux.volume_drag.take().is_some() && self.ux.persist(&self.data_dir).is_err() {
            self.notify("Volume changed for this session; could not save settings.");
        }
    }
    /// Over the drawn Settings rows: each volume's slider, and one line on
    /// what the focused or hovered row does.
    fn draw_settings_extras(&mut self) {
        use crate::canvas::{EDGE, GOLD, MUTED, WHITE};
        for b in &self.buttons {
            let Action::Volume(bus) = b.action else {
                continue;
            };
            let level = i32::from(match bus {
                crate::audio::Bus::Effects => self.ux.preferences.effects_volume,
                crate::audio::Bus::Ambience => self.ux.preferences.ambience_volume,
                crate::audio::Bus::Music => self.ux.preferences.music_volume,
            })
            .min(100);
            let (x, w) = volume_track(b);
            let y = b.y + b.h / 2 - 1;
            self.canvas.rect(x, y, w, 3, EDGE);
            self.canvas.rect(x, y, w * level / 100, 3, GOLD);
            self.canvas
                .rect(x + w * level / 100 - 1, y - 3, 3, 9, WHITE);
        }
        let target = if self.ux.keyboard_navigation {
            self.ux.focused.clone()
        } else {
            self.buttons
                .iter()
                .find(|b| b.contains(self.cursor.0, self.cursor.1))
                .map(|b| b.action.clone())
        };
        if let Some(hint) = target.as_ref().and_then(crate::menus::settings_hint) {
            let line = format!("{hint}  < > CHANGES IT");
            let x = (640 - line.len() as i32 * 6) / 2;
            self.canvas.text(&line, x.max(8), 292, MUTED);
        }
    }
    pub(crate) fn draw_pause(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::pause(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    pub(crate) fn draw_help(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::help(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    pub(crate) fn draw_settings(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::settings(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    pub(crate) fn draw_confirm(&mut self) {
        let model = self.menu_model();
        self.buttons = crate::menus::confirm(&mut self.canvas, self.atlas.as_ref(), &model);
    }
    pub(crate) fn draw_result(&mut self, outcome: Outcome) {
        let model = self.menu_model();
        self.buttons = crate::menus::result(&mut self.canvas, self.atlas.as_ref(), &model, outcome);
    }
    pub(crate) fn draw_buttons(&mut self) {
        use crate::ux::ButtonWeight;
        let weights: Vec<ButtonWeight> =
            self.buttons.iter().map(|b| self.button_weight(b)).collect();
        for (b, weight) in self.buttons.iter().zip(weights) {
            // One highlight: the pointer moves focus, so the lit control is
            // the one Enter presses.
            let lit = b.enabled && self.ux.focused.as_ref() == Some(&b.action);
            // A roster card is drawn by its page; focus only frames it.
            if matches!(b.action, Action::RosterUnit(_)) {
                if lit {
                    self.canvas.frame(b.x, b.y, b.w, b.h, WHITE);
                }
                continue;
            }
            let focused = lit;
            let (fill, edge, ink) = match (weight, b.enabled, lit) {
                (_, false, _) => (PANEL, EDGE, EDGE),
                (ButtonWeight::Primary, _, false) => (MENU_PRIMARY, MENU_PRIMARY_EDGE, INK),
                (ButtonWeight::Primary, _, true) => (MENU_PRIMARY_LIT, WHITE, INK),
                (ButtonWeight::Danger, _, false) => (PANEL, MENU_DANGER, MENU_DANGER),
                (ButtonWeight::Danger, _, true) => (MENU_DANGER_LIT, MENU_DANGER, WHITE),
                (ButtonWeight::Secondary, _, false) => (PANEL, EDGE, WHITE),
                (ButtonWeight::Secondary, _, true) => ([47, 67, 73, 255], GOLD, WHITE),
            };
            self.canvas.rect(b.x, b.y, b.w, b.h, fill);
            self.canvas
                .line(b.x, b.y + b.h - 1, b.x + b.w - 1, b.y + b.h - 1, edge);
            if weight == ButtonWeight::Danger && b.enabled {
                // A red rule down the left marks a control that loses
                // progress, lit or not.
                self.canvas.rect(b.x, b.y, 2, b.h, MENU_DANGER);
            }
            let color = ink;
            // Rows too short for a second line show their hint on the same
            // line, right aligned, where it reads as the row's value.
            let mut label_room = b.w - 8;
            if !b.hint.is_empty() && b.h < 24 {
                let label_w = text_readable_width(&b.label).min(label_room / 2);
                let room = label_room - label_w - 8;
                let readable_w = text_readable_width(&b.hint);
                let (shown, w, readable) = if readable_w <= room {
                    (b.hint.clone(), readable_w, true)
                } else {
                    let shown: String = b.hint.chars().take((room / 6).max(1) as usize).collect();
                    let w = shown.chars().count() as i32 * 6;
                    (shown, w, false)
                };
                let hx = b.x + b.w - 4 - w;
                // A row's value reads quieter than its name.
                let value = if weight == ButtonWeight::Secondary && b.enabled {
                    MUTED
                } else {
                    color
                };
                if readable {
                    self.canvas
                        .text_readable(&shown, hx, b.y + (b.h - 9) / 2, value);
                } else {
                    self.canvas.text(&shown, hx, b.y + (b.h - 7) / 2, value);
                }
                label_room = hx - b.x - 12;
            }
            let label = b
                .label
                .chars()
                .take((label_room / 6).max(1) as usize)
                .collect::<String>();
            let label_y = if b.h >= 24 && !b.hint.is_empty() {
                b.y + 3
            } else {
                b.y + (b.h - 9) / 2
            };
            if b.h >= 16 && text_readable_width(&label) <= label_room {
                self.canvas.text_readable(&label, b.x + 4, label_y, color)
            } else {
                self.canvas.text(
                    &label,
                    b.x + 4,
                    if b.h < 16 { b.y + 1 } else { label_y + 1 },
                    color,
                )
            }
            if focused && weight == ButtonWeight::Secondary {
                self.canvas
                    .line(b.x + 2, b.y + 2, b.x + 2, b.y + b.h - 3, GOLD);
                self.canvas.line(b.x + 3, b.y + 2, b.x + 6, b.y + 2, GOLD);
                self.canvas
                    .line(b.x + 3, b.y + b.h - 3, b.x + 6, b.y + b.h - 3, GOLD);
            }
            if !b.hint.is_empty() && b.h >= 24 {
                self.canvas.text(
                    &b.hint
                        .chars()
                        .take((b.w / 6 - 1) as usize)
                        .collect::<String>(),
                    b.x + 4,
                    b.y + 16,
                    if !b.enabled {
                        EDGE
                    } else if weight == ButtonWeight::Primary {
                        INK
                    } else {
                        MUTED
                    },
                );
            }
        }
    }
    pub fn screenshot(&mut self, path: &Path) -> Result<(), String> {
        self.render();
        self.canvas.save(path)
    }
}
/// Menu control fills: the one primary action in gold, a darker gold rule
/// under it, and red for controls that lose progress.
const MENU_PRIMARY: Color = [226, 164, 72, 255];
const MENU_PRIMARY_LIT: Color = [250, 200, 118, 255];
const MENU_PRIMARY_EDGE: Color = [150, 100, 40, 255];
const MENU_DANGER: Color = [226, 102, 80, 255];
const MENU_DANGER_LIT: Color = [110, 42, 36, 255];
/// The one-pixel edge drawn around every enemy machine and building.
const ENEMY_EDGE: Color = [206, 72, 58, 225];
const RIVAL_EDGE: Color = [160, 112, 226, 225];

/// The edge drawn round an opponent's machine and its building names.
fn enemy_edge(owner: u8) -> Color {
    if owner == 2 { RIVAL_EDGE } else { ENEMY_EDGE }
}
/// The atlas key an entity was actually drawn with: its state frame, its
/// facing, or its base key, whichever exists.
fn drawn_key<'a>(atlas: &'a Atlas, overlay: &'a EntityOverlay) -> Option<&'a str> {
    overlay
        .animation
        .as_deref()
        .filter(|k| atlas.sprites.contains_key(*k))
        .or_else(|| {
            atlas
                .sprites
                .contains_key(&overlay.directional)
                .then_some(overlay.directional.as_str())
        })
        .or_else(|| {
            atlas
                .sprites
                .contains_key(overlay.asset_key)
                .then_some(overlay.asset_key)
        })
}
/// Whether two drawn sprites' screen rectangles overlap at all.
fn sprites_overlap(
    atlas: &Atlas,
    a: &EntityOverlay,
    a_key: &str,
    b: &EntityOverlay,
    b_key: &str,
) -> bool {
    let rect = |o: &EntityOverlay, key: &str| {
        let s = &atlas.sprites[key];
        let left = o.x - o.scale.apply(s.anchor_x);
        let top = o.y - o.scale.apply(s.anchor_y);
        (
            left,
            top,
            left + o.scale.apply(s.w as i32),
            top + o.scale.apply(s.h as i32),
        )
    };
    let (al, at, ar, ab) = rect(a, a_key);
    let (bl, bt, br, bb) = rect(b, b_key);
    al < br && bl < ar && at < bb && bt < ab
}
/// The bank plant planted at a cell, if any.  One jittered planting per
/// seven-cell bank segment keeps calm gaps without leaving a whole starting
/// shore bare; the species hash is decorrelated from the tile hash so a
/// straight bank does not repeat one species at one interval.
pub(crate) fn bank_plant(map: &bw_sim::Map, x: i32, y: i32) -> Option<&'static str> {
    let t = map.terrain(x, y);
    if !matches!(t, Terrain::Salt | Terrain::Silt) {
        return None;
    }
    let segment_seed =
        ((x / 7) as u32).wrapping_mul(73_856_093) ^ ((y / 7) as u32).wrapping_mul(19_349_663);
    let plant_offset = (segment_seed ^ (segment_seed >> 13)) % 7;
    let mut shore_growth = false;
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if map.terrain(x + dx, y + dy) == Terrain::Deep {
            let along_bank = if dx == 0 { x } else { y };
            shore_growth |= along_bank as u32 % 7 == plant_offset;
        }
    }
    if !shore_growth {
        return None;
    }
    let coast_seed = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
    let coast_hash = (coast_seed ^ (coast_seed >> 13)).wrapping_mul(1_274_126_177);
    Some(match (coast_hash >> 16) % 3 {
        0 => "reed_clump",
        1 => "marsh_grass",
        _ => "salt_bush",
    })
}
/// A broken ripple ring around a machine wading through a flooded lane.
fn draw_wake(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    phase: u8,
    tick: u64,
    is_water: &dyn Fn(i32, i32) -> bool,
) {
    let spread = i32::from(phase % 2) + ((tick / 4) % 2) as i32;
    let (rx, ry) = (11 + spread * 2, 5 + spread);
    let ring: Color = [128, 168, 166, 140];
    for step in 0..24 {
        if (step + tick / 3).is_multiple_of(3) {
            continue;
        }
        let angle = step as f64 * std::f64::consts::TAU / 24.0;
        let px = x + (angle.cos() * f64::from(rx)).round() as i32;
        let py = y + 2 + (angle.sin() * f64::from(ry)).round() as i32;
        if is_water(px, py) {
            canvas.pixel(px, py, ring);
        }
    }
}
fn shade(mut c: Color, pct: u16) -> Color {
    for ch in c.iter_mut().take(3) {
        *ch = (u16::from(*ch) * pct / 100).min(255) as u8;
    }
    c
}
fn entity_render_position(
    camera: Camera,
    entity: &bw_sim::Entity,
    faction: Faction,
) -> (i32, i32, crate::occlusion::PixelScale) {
    let t = crate::occlusion::entity_render_transform(entity, faction);
    let (x, y) = camera.project(t.anchor);
    (x + t.render_x_offset, y + t.render_y_offset, t.scale)
}
/// A diamond ring in two-pixel dashes, the hover ring's shape.
fn dashed_diamond(canvas: &mut crate::canvas::Canvas, x: i32, y: i32, radius: i32, color: Color) {
    for i in 0..=radius {
        if (i / 2) % 2 == 1 {
            continue;
        }
        let dy = (radius - i) / 2;
        for (px, py) in [
            (x - i, y - dy),
            (x + i, y - dy),
            (x - i, y + dy),
            (x + i, y + dy),
        ] {
            canvas.pixel(px, py, color);
        }
    }
}
/// A price on the command card: "180S+30P", or "50S" without pressure.
pub(crate) fn cost(k: Kind) -> String {
    let s = spec(k);
    crate::ux::price(s.salvage, s.pressure)
}

pub(crate) fn other_faction(faction: Faction) -> Faction {
    match faction {
        Faction::Union => Faction::Assembly,
        Faction::Assembly | Faction::Compact => Faction::Union,
    }
}

/// How far the camera may pan, in texels: `[left, right, top, bottom]`.
/// The map's diamond is `16 * size` texels either side of its middle and
/// `16 * size` deep; the margin keeps its corners reachable at the view's
/// edge. On the Split Basin (128 cells) this is the old ±2400, -200..2400.
pub(crate) fn camera_limits(size: u16) -> [i32; 4] {
    let reach = i32::from(size) * 16 + 352;
    [-reach, reach, -200, reach]
}

#[cfg(test)]
mod draw_order_tests {
    use super::*;
    use bw_core::{CANVAS_W, WORLD_TOP};

    #[test]
    fn building_faces_occlude_real_unit_pixels_and_picking_agrees() {
        for faction in [Faction::Union, Faction::Assembly] {
            for kind in [
                Kind::Headquarters,
                Kind::Works,
                Kind::Dropoff,
                Kind::Condenser,
                Kind::Tower,
            ] {
                let mut game = building_fixture(kind);
                game.world.players[0].faction = faction;
                game.faction = faction;
                game.selected.clear();
                let building = game.world.entities[0].clone();
                let bounds = crate::occlusion::footprint_bounds(&building).unwrap();
                let mut unit = bw_sim::World::new(1, faction)
                    .entities
                    .into_iter()
                    .find(|e| e.owner == 0 && e.kind.is_worker())
                    .unwrap();
                unit.id = 900;
                unit.order = Order::Idle;
                unit.path.clear();
                unit.carried = 0;
                unit.carried_kind = None;
                game.world.entities.push(unit.clone());
                let midx = (bounds.left + bounds.right) / 2;
                let midy = (bounds.top + bounds.bottom) / 2;
                for (pos, unit_in_front) in [
                    (Pos::raw(midx, bounds.top - FP / 2), false),
                    (Pos::raw(bounds.left - FP / 2, midy), false),
                    (Pos::raw(midx, bounds.bottom + FP / 2), true),
                    (Pos::raw(bounds.right + FP / 2, midy), true),
                ] {
                    assert!(!bounds.contains(pos));
                    game.world.entities[1].pos = pos;
                    game.draw_world();
                    let e = &game.world.entities[1];
                    let (ux, uy, us) = entity_render_position(game.camera, e, faction);
                    let uk = game
                        .entity_animation_key(e)
                        .unwrap_or_else(|| game.directional_key(e));
                    let (bx, by, bs) = entity_render_position(game.camera, &building, faction);
                    let bk = game
                        .entity_animation_key(&building)
                        .unwrap_or_else(|| kind.asset(faction).into());
                    let atlas = game.atlas.as_ref().unwrap();
                    let mut unit_pixels = Canvas::default();
                    let mut building_pixels = Canvas::default();
                    atlas.draw_scaled(&mut unit_pixels, &uk, ux, uy, false, us);
                    atlas.draw_scaled(&mut building_pixels, &bk, bx, by, false, bs);
                    if kind == Kind::Headquarters {
                        atlas.draw(
                            &mut building_pixels,
                            if faction == Faction::Union {
                                "faction_union_hq"
                            } else {
                                "faction_assembly_hq"
                            },
                            bx - 17,
                            by - 26,
                            false,
                        );
                    }
                    let mut overlaps = 0;
                    for y in WORLD_TOP..uy - 8 {
                        for x in 0..640 {
                            let at = (y as usize * 640 + x as usize) * 4;
                            let u = &unit_pixels.pixels[at..at + 4];
                            let b = &building_pixels.pixels[at..at + 4];
                            if u[3] == 255 && b[3] == 255 && u != b {
                                if unit_in_front {
                                    assert_eq!(
                                        &game.canvas.pixels[at..at + 4],
                                        u,
                                        "{faction:?} {kind:?} wrong front at {pos:?}, pixel{x},{y}"
                                    );
                                } else {
                                    // Behind: the building covers the machine's
                                    // own pixels and a tinted ghost shows
                                    // through, so neither plain pixel appears.
                                    assert_ne!(
                                        &game.canvas.pixels[at..at + 4],
                                        u,
                                        "{faction:?} {kind:?} machine painted over the front at {pos:?}, pixel{x},{y}"
                                    );
                                    assert_ne!(
                                        &game.canvas.pixels[at..at + 4],
                                        b,
                                        "{faction:?} {kind:?} no ghost through the building at {pos:?}, pixel{x},{y}"
                                    );
                                }
                                // The machine wins the pick in front; behind,
                                // it keeps the core of its ghost and the rest
                                // of the face picks the building (rules 14).
                                let core = (x - ux).abs() <= us.apply(7)
                                    && y >= uy - us.apply(26)
                                    && y <= uy - us.apply(4);
                                if unit_in_front || core {
                                    assert_eq!(game.unit_at(x, y), Some(unit.id));
                                }
                                overlaps += 1;
                            }
                        }
                    }
                    assert!(
                        overlaps > 2,
                        "{faction:?} {kind:?} must exercise overlapping body pixels on face {pos:?}"
                    );
                }
            }
        }
    }

    fn building_fixture(kind: Kind) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.message.clear();
        game.world
            .entities
            .retain(|e| e.owner == 0 && e.kind == Kind::Headquarters);
        let e = &mut game.world.entities[0];
        e.kind = kind;
        e.hp = spec(kind).health;
        e.max_hp = e.hp;
        e.queue.clear();
        game.camera.center(e.pos);
        game.world.map.resources.clear();
        game.world.map.wells.clear();
        game
    }

    #[test]
    fn missing_building_sprites_leave_the_ground_below_them_untouched() {
        for faction in [Faction::Union, Faction::Assembly] {
            for kind in [
                Kind::Headquarters,
                Kind::Works,
                Kind::Dropoff,
                Kind::Condenser,
                Kind::Tower,
            ] {
                let mut game = building_fixture(kind);
                game.atlas = None;
                game.selected.clear();
                game.faction = faction;
                game.world.players[0].faction = faction;
                // Public sluice visibility keeps the background identical
                // when the subject is removed for the comparison render.
                game.world.entities[0].pos = Pos::raw(
                    game.world.map.gate_pos.x,
                    game.world.map.gate_pos.y - 3 * FP,
                );
                game.camera.center(game.world.entities[0].pos);
                let building = game.world.entities.pop().unwrap();
                let (x, y, _) = entity_render_position(game.camera, &building, faction);
                game.draw_world();
                let ground = game.canvas.pixels.clone();
                game.world.entities.push(building);
                game.draw_world();
                for dy in 0..=6 {
                    for dx in -14..=14 {
                        if (-2..2).contains(&dx) && (3..5).contains(&dy) {
                            continue; // Ownership marker, not a shadow.
                        }
                        let at = ((y + dy) as usize * CANVAS_W as usize + (x + dx) as usize) * 4;
                        assert_eq!(
                            &game.canvas.pixels[at..at + 4],
                            &ground[at..at + 4],
                            "{faction:?} {kind:?} painted below its foundation at {dx},{dy}"
                        );
                    }
                }
                let foot = ((y - 1) as usize * CANVAS_W as usize + (x + 10) as usize) * 4;
                assert_ne!(
                    &game.canvas.pixels[foot..foot + 4],
                    &ground[foot..foot + 4],
                    "fallback wall must reach the ground"
                );
            }
        }
    }

    #[test]
    fn every_lock_switch_frame_is_in_the_atlas_and_ends_on_the_rest_drawing() {
        let game = building_fixture(Kind::Headquarters);
        let atlas = game.atlas.as_ref().unwrap();
        for north_dry in [true, false] {
            for tick in 0..presentation::GATE_SWITCH_FRAMES * presentation::GATE_SWITCH_FRAME_TICKS
            {
                let key = presentation::gate_switch_asset(north_dry, 0, tick).unwrap();
                assert!(atlas.hit(&key, 0, 4), "masonry slab missing in {key}");
                assert!(!atlas.hit(&key, 0, 12), "shadow returned in {key}");
            }
        }
    }

    #[test]
    fn every_sluice_state_leaves_the_former_cast_shadow_transparent() {
        let game = building_fixture(Kind::Headquarters);
        let atlas = game.atlas.as_ref().unwrap();
        for north_dry in [true, false] {
            for warning in [false, true] {
                for phase in 0..4 {
                    let key = gate_asset(north_dry, warning, phase);
                    assert!(!atlas.hit(&key, 0, 12), "shadow returned in {key}");
                    assert!(atlas.hit(&key, 0, 4), "masonry slab missing in {key}");
                }
            }
        }
    }

    #[test]
    fn construction_stage_boundaries_and_hit_masks_match_visible_art() {
        let mut game = building_fixture(Kind::Works);
        game.selected.clear();
        let total = spec(Kind::Works).build_ticks;
        for (remaining, stage) in [(total, 0), (401, 0), (400, 1), (201, 1), (200, 2), (1, 2)] {
            game.world.entities[0].build_remaining = remaining;
            let e = &game.world.entities[0];
            let key = building_state_key(e, Faction::Union, 0, 0).expect("construction key");
            assert_eq!(key, format!("union_works_build_{stage}"));
            let (x, y, scale) = entity_render_position(game.camera, e, Faction::Union);
            let id = e.id;
            let atlas = game.atlas.as_ref().expect("atlas");
            let mut hits = Vec::new();
            let mut gaps = Vec::new();
            for dy in -90..-20 {
                for dx in -60..60 {
                    if atlas.hit_scaled(&key, dx, dy, scale) {
                        hits.push((x + dx, y + dy));
                    } else if atlas.hit_scaled("union_works", dx, dy, scale) {
                        gaps.push((x + dx, y + dy));
                    }
                }
            }
            assert!(
                !hits.is_empty(),
                "stage must have actual visible target pixels"
            );
            for (px, py) in hits.iter().step_by(11) {
                assert_eq!(game.unit_at(*px, *py), Some(id));
            }
            // The site's BUILD label counts as the site (rules 14).
            let label_top = y - scale.apply(63) - 18;
            for (px, py) in gaps.iter().step_by(11) {
                let on_label = (px - x).abs() <= 28 && *py >= label_top && *py <= label_top + 22;
                assert_eq!(
                    game.unit_at(*px, *py),
                    if on_label { Some(id) } else { None },
                    "unfinished roof cannot be clicked"
                );
            }
        }
        game.world.entities[0].build_remaining = 0;
        assert!(building_state_key(&game.world.entities[0], Faction::Union, 0, 0).is_none());
    }

    #[test]
    fn works_activity_requires_progressing_production() {
        let mut game = building_fixture(Kind::Works);
        for (started, remaining, active) in
            [(false, 100, false), (true, 100, true), (true, 1, false)]
        {
            game.world.entities[0].queue = vec![bw_sim::Production {
                kind: Kind::Riveter,
                started,
                remaining,
                cost_salvage: 70,
                cost_pressure: 0,
            }];
            let key = building_state_key(&game.world.entities[0], Faction::Union, 0, 0);
            assert_eq!(
                key.is_some(),
                active,
                "unstarted and completion-stalled queues must stay idle"
            );
        }
    }

    #[test]
    fn active_building_hit_pixels_and_health_bar_remain_registered() {
        let mut game = building_fixture(Kind::Condenser);
        let e = &game.world.entities[0];
        let (x, y, scale) = entity_render_position(game.camera, e, Faction::Union);
        let id = e.id;
        let atlas = game.atlas.as_ref().expect("atlas");
        let max_height = (0..4)
            .map(|p| {
                let s = &atlas.sprites[&format!("condenser_active_{p}")];
                s.anchor_y - s.opaque_top as i32 + 6
            })
            .max()
            .expect("active frames");
        for phase in 0..4 {
            game.world.tick = phase * 6;
            // The wind may hurry the loop, so take the frame the renderer and
            // the picker actually agree on.
            let key = game.entity_animation_key(&game.world.entities[0]).unwrap();
            let atlas = game.atlas.as_ref().expect("atlas");
            let (dx, dy) = (-100..-25)
                .flat_map(|dy| (-40..40).map(move |dx| (dx, dy)))
                .find(|&(dx, dy)| {
                    atlas.hit_scaled(&key, dx, dy, scale)
                        && !atlas.hit_scaled("condenser", dx, dy, scale)
                })
                .expect("active steam pixels beyond the idle silhouette");
            assert_eq!(game.unit_at(x + dx, y + dy), Some(id));
            game.render();
            let by = y - scale.apply(max_height);
            let pixel = |px, py| ((py * CANVAS_W as i32 + px) * 4) as usize;
            let top = pixel(x - 16, by);
            let hp = pixel(x, by + 1);
            assert_eq!(&game.canvas.pixels[top..top + 4], &INK);
            assert_eq!(&game.canvas.pixels[hp..hp + 4], &JADE);
            let first = game.canvas.pixels.clone();
            game.render();
            assert_eq!(
                game.canvas.pixels, first,
                "render calls alone must not advance the animation"
            );
            assert_eq!(game.world.tick, phase * 6);
        }
    }

    #[test]
    fn clicking_the_upper_wreck_silhouette_orders_gathering() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        let worker = game
            .first_worker()
            .or_else(|| {
                game.world
                    .entities
                    .iter()
                    .find(|e| e.owner == 0 && e.kind.is_worker())
                    .map(|e| e.id)
            })
            .expect("starting worker");
        game.selected = vec![worker];
        game.world.entities.retain(|e| e.id == worker);
        let pos = Pos::cell(24, 61);
        game.world.entities[0].pos = Pos::cell(25, 63);
        game.camera.center(pos);
        let (x, y) = game.camera.project(pos);
        for id in 0..3 {
            game.world.map.resources = vec![bw_sim::Resource {
                scrap: false,
                id,
                pos,
                remaining: 2400,
                kind: bw_sim::ResourceKind::Salvage,
            }];
            let atlas = game.atlas.as_ref().expect("runtime atlas");
            let (dx, dy) = (-70..-35)
                .flat_map(|dy| (-40..40).map(move |dx| (dx, dy)))
                .find(|&(dx, dy)| {
                    atlas.hit(salvage_asset(id), dx, dy)
                        && pos.distance_sq(game.camera.unproject(x + dx, y + dy))
                            >= i64::from(FP * 2).pow(2)
                })
                .expect("wreck extends beyond the old ground-radius hit target");
            game.right_click(x + dx, y + dy, false);
            assert!(
                matches!(game.world.command_log.last().expect("command").command,
                Command::Gather { resource, .. } if resource == id),
                "upper silhouette of {} must be a gather target",
                salvage_asset(id)
            );
        }
    }

    #[test]
    fn tall_props_occlude_units_by_ground_depth_in_the_rendered_canvas() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.selected.clear();
        let unit = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .expect("starting worker")
            .clone();
        game.world.entities.retain(|e| e.id == unit.id);
        game.world.map.wells.clear();
        game.camera.center(unit.pos);
        let (ux, uy) = game.camera.project(unit.pos);
        let key = format!("hook_{}", (9 - unit.facing % 8) % 8);

        for (resource_id, offset) in [10_000, 10_001, 10_002]
            .into_iter()
            .flat_map(|id| [-FP, FP].map(|offset| (id, offset)))
        {
            let pos = Pos::raw(unit.pos.x + offset, unit.pos.y + offset);
            game.world.map.resources = vec![bw_sim::Resource {
                scrap: false,
                id: resource_id,
                pos,
                remaining: 2_400,
                kind: bw_sim::ResourceKind::Salvage,
            }];
            game.draw_world();
            let (px, py) = game.camera.project(pos);
            let atlas = game.atlas.as_ref().expect("runtime atlas");
            let mut unit_pixels = Canvas::default();
            let mut prop_pixels = Canvas::default();
            assert!(atlas.draw(&mut unit_pixels, &key, ux, uy, false));
            assert!(atlas.draw(&mut prop_pixels, salvage_asset(resource_id), px, py, false));
            let mut overlap = 0;
            // The wreck's salvage plate sits over both surfaces by design.
            let plate_w = crate::canvas::text_readable_width("SALVAGE 2400") + 6;
            let plate = crate::native_ui::Rect {
                x: px - plate_w / 2,
                y: py + 6,
                w: plate_w,
                h: 13,
            };
            for y in WORLD_TOP..uy - 8 {
                for x in 0..CANVAS_W as i32 {
                    if plate.contains(x, y) {
                        continue;
                    }
                    let i = ((y as u32 * CANVAS_W + x as u32) * 4) as usize;
                    let unit_pixel = &unit_pixels.pixels[i..i + 4];
                    let prop_pixel = &prop_pixels.pixels[i..i + 4];
                    if unit_pixel[3] == 255 && prop_pixel[3] == 255 && unit_pixel != prop_pixel {
                        let front = if offset > 0 { prop_pixel } else { unit_pixel };
                        assert_eq!(
                            &game.canvas.pixels[i..i + 4],
                            front,
                            "wrong front surface at {x},{y}, depth offset {offset}"
                        );
                        overlap += 1;
                    }
                }
            }
            assert!(
                overlap > 10,
                "fixture must contain visible overlapping surfaces"
            );
        }
    }
}
