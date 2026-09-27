//! Session navigation and user-facing command semantics. No simulation rules.
use crate::canvas::{EDGE, GOLD, INK, JADE, MUTED, RED, WHITE, text_readable_width};
use crate::dock_log::DockState;
use crate::game::{Action, Game, Mode, Screen};
use crate::menus::MenuModel;
use crate::onboarding::{Tutorial, TutorialTarget};
use bw_content::spec;
use bw_core::{EntityId, FP, Faction, Kind, Pos, Terrain};
use bw_sim::{Command, World};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub always_health_bars: bool,
    pub muted: bool,
    pub edge_scroll: bool,
    pub pause_unfocused: bool,
    pub fullscreen: bool,
    pub tutorial_completed: bool,
    /// Interface scale: 0 follows the window and the display, 1 to 4 fix it.
    #[serde(default)]
    pub interface_scale: u8,
    #[serde(
        default = "default_effects_volume",
        deserialize_with = "deserialize_volume"
    )]
    pub effects_volume: u8,
    #[serde(
        default = "default_ambience_volume",
        deserialize_with = "deserialize_volume"
    )]
    pub ambience_volume: u8,
    #[serde(
        default = "default_music_volume",
        deserialize_with = "deserialize_volume"
    )]
    pub music_volume: u8,
    #[serde(
        default = "default_world_zoom",
        deserialize_with = "deserialize_world_zoom"
    )]
    /// The world's scale, 1.0 to 3.0. Older files hold 1, 2, 3, 50 or 100.
    pub world_zoom: f32,
    pub game_speed: crate::tempo::GameSpeed,
    /// The sluice card folded to its status and hold checklist, so the
    /// field shows more ground.
    pub compact_field_cards: bool,
    /// The name shown to other players online; empty for the computer's
    /// user name.
    #[serde(default)]
    pub online_name: String,
    /// The sluice card pinned open under the tide gauge.  Open from the
    /// start (trial 11: both seats found it by accident); stored under a
    /// new name so an older file's unpinned card opens once.
    #[serde(rename = "sluice_card_open", default = "default_sluice_card")]
    pub tide_card: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            always_health_bars: false,
            muted: false,
            edge_scroll: true,
            pause_unfocused: true,
            fullscreen: false,
            tutorial_completed: false,
            interface_scale: 0,
            effects_volume: default_effects_volume(),
            ambience_volume: default_ambience_volume(),
            music_volume: default_music_volume(),
            world_zoom: default_world_zoom(),
            game_speed: Default::default(),
            compact_field_cards: false,
            online_name: String::new(),
            tide_card: default_sluice_card(),
        }
    }
}

const fn default_sluice_card() -> bool {
    true
}

const fn default_effects_volume() -> u8 {
    75
}

const fn default_ambience_volume() -> u8 {
    65
}

const fn default_music_volume() -> u8 {
    60
}
const fn default_world_zoom() -> f32 {
    2.0
}
fn deserialize_world_zoom<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    Ok(crate::zoom::Zoom::from_preference(f64::deserialize(d)? as f32).scale() as f32)
}

fn deserialize_volume<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    // Read a wider unsigned value first so old/future preference files with a
    // value above 100 are accepted and safely clamped instead of making the
    // whole preference file unreadable.
    let value = u64::deserialize(deserializer)?;
    Ok(value.min(100) as u8)
}

impl Preferences {
    pub fn clamp_volumes(&mut self) {
        self.effects_volume = self.effects_volume.min(100);
        self.ambience_volume = self.ambience_volume.min(100);
        self.music_volume = self.music_volume.min(100);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingAction {
    NewSkirmish,
    NewPractice,
    Load,
    Surrender,
    EndPractice,
    Quit,
}

pub struct OrderMarker {
    pub pos: Pos,
    pub tick: u64,
    pub caption: &'static str,
}

pub struct UxState {
    pub roster_page: usize,
    /// The faction shown on the Guide's roster page; the player's until chosen.
    pub roster_faction: Option<Faction>,
    /// The Guide's roster shows the shared buildings.
    pub roster_buildings: bool,
    /// The roster card whose details show, when one was clicked.
    pub roster_unit: Option<Kind>,
    /// The open Settings tab: sound, controls, display, keys.
    pub settings_tab: u8,
    pub alerts: crate::field_alerts::AlertHistory,
    pub minimap_drag: bool,
    /// The volume slider being dragged.
    pub volume_drag: Option<crate::audio::Bus>,
    pub pointer_shift: bool,
    pub last_idle_worker: Option<EntityId>,
    pub paused_unfocused: bool,
    /// Explicit label for the opt-in development field; never inferred in normal play.
    pub tactics_fixture: bool,
    pub preferences: Preferences,
    /// Presentation-only telegraph/history state. It never enters the world
    /// hash or simulation save.
    pub dock: DockState,
    pub traces: crate::tidal_traces::TraceState,
    pub manual: crate::field_manual::ManualState,
    pub chart_memory: crate::chart_memory::MemoryState,
    /// Set only when a completed practice review returns to the menu. Practice
    /// exit does not invent a `World::outcome`.
    pub practice_complete: bool,
    /// Keeps the practice field on the match renderer while the player reviews
    /// its observed dock log. The root tick hook must suppress simulation
    /// stepping while this flag is set.
    pub practice_review: bool,
    /// How far the last ended practice got, for its review panel.
    pub practice_progress: (usize, usize, String),
    pub session_active: bool,
    pub practice: bool,
    pub tutorial: Option<Tutorial>,
    pub guidance_visible: bool,
    pub focused: Option<Action>,
    pub keyboard_navigation: bool,
    pub focus_screen: Screen,
    pub settings_return: Screen,
    pub confirm_return: Screen,
    pub pending: Option<PendingAction>,
    pub help_page: usize,
    pub save_available: bool,
    pub save_description: String,
    pub cached_save_dir: PathBuf,
    pub last_saved_hash: Option<String>,
    /// The field tick the quick save was written at, for the pause menu's
    /// SAVED mark.
    pub saved_at_tick: Option<u64>,
    pub fullscreen: bool,
    pub fullscreen_request: Option<bool>,
    pub quit_requested: bool,
    pub markers: Vec<OrderMarker>,
}
impl UxState {
    pub fn load(data_dir: &Path) -> Self {
        let mut preferences: Preferences = std::fs::read(data_dir.join("ui/preferences.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        preferences.clamp_volumes();
        let requested = preferences.fullscreen.then_some(true);
        let mut state = Self {
            roster_page: 0,
            roster_faction: None,
            roster_buildings: false,
            roster_unit: None,
            settings_tab: 0,
            alerts: Default::default(),
            minimap_drag: false,
            volume_drag: None,
            pointer_shift: false,
            last_idle_worker: None,
            paused_unfocused: false,
            tactics_fixture: false,
            preferences,
            dock: DockState::default(),
            traces: crate::tidal_traces::TraceState::default(),
            manual: crate::field_manual::ManualState::default(),
            chart_memory: crate::chart_memory::MemoryState::default(),
            practice_complete: false,
            practice_review: false,
            practice_progress: (0, 7, String::new()),
            session_active: false,
            practice: false,
            tutorial: None,
            guidance_visible: true,
            focused: None,
            keyboard_navigation: false,
            focus_screen: Screen::Menu,
            settings_return: Screen::Menu,
            confirm_return: Screen::Menu,
            pending: None,
            help_page: 0,
            save_available: false,
            save_description: String::new(),
            cached_save_dir: PathBuf::new(),
            last_saved_hash: None,
            saved_at_tick: None,
            fullscreen: false,
            fullscreen_request: requested,
            quit_requested: false,
            markers: Vec::new(),
        };
        state.refresh_save(data_dir);
        state
    }
    pub fn refresh_save(&mut self, data_dir: &Path) {
        self.cached_save_dir = data_dir.to_path_buf();
        let path = data_dir.join("saves/quick-save.json");
        match World::load(&path) {
            Ok(world) => {
                self.save_available = true;
                // Short enough to sit beside LOAD on a menu row.
                self.save_description = format!(
                    "{} / {} / {}",
                    if world.ai_enabled {
                        "SKIRMISH"
                    } else {
                        "PRACTICE"
                    },
                    match world.players[0].faction {
                        Faction::Union => "UNION",
                        Faction::Assembly => "ASSEMBLY",
                        Faction::Compact => "COMPACT",
                    },
                    clock(world.tick / 30)
                );
            }
            Err(error) => {
                self.save_available = false;
                self.save_description = if error.contains("incompatible")
                    || error.contains("version")
                    || error.contains("rules")
                {
                    "Saved match uses older rules; start a new match"
                } else if path.exists() {
                    "Saved match could not be read"
                } else {
                    "No saved match yet"
                }
                .into();
            }
        }
    }
    pub fn persist(&self, dir: &Path) -> Result<(), String> {
        let mut preferences = self.preferences.clone();
        preferences.clamp_volumes();
        let bytes = serde_json::to_vec_pretty(&preferences).map_err(|e| e.to_string())?;
        atomic_write(&dir.join("ui/preferences.json"), &bytes)
    }

    /// Clear dock feedback/history and the practice return flag for a fresh
    /// skirmish or practice field.
    pub fn reset_dock_for_new_match(&mut self) {
        self.dock.reset_for_new_match();
        self.roster_page = 0;
        self.alerts = Default::default();
        self.traces = crate::tidal_traces::TraceState::default();
        self.chart_memory = crate::chart_memory::MemoryState::default();
        self.practice_complete = false;
        self.practice_review = false;
    }

    /// Mark a guided practice field as completed for its honest return flow.
    /// This is UI state only; callers should invoke it before leaving practice
    /// rather than writing an outcome into `World`.
    pub fn mark_practice_complete(&mut self) {
        self.practice_complete = true;
    }

    /// Enter the post-practice dock review without creating a simulation
    /// outcome. The world remains available for the root's overlaid renderer.
    pub fn begin_practice_review(&mut self) {
        self.practice_review = true;
    }

    /// Leave the post-practice review and return to the home screen. The
    /// completion flag is retained so Home can honestly report the prior
    /// completed review, while a later new match resets it.
    pub fn end_practice_review(&mut self) {
        self.practice_review = false;
        self.practice = false;
        self.session_active = false;
    }

    /// Forward a successful `World::issue` submission to the pending dock
    /// telegraph. Acceptance is deliberately deferred until `after_tick`.
    pub fn on_command_submitted(
        &mut self,
        tick: u64,
        sequence: u64,
        caption: impl Into<String>,
        target: Option<Pos>,
    ) {
        self.dock
            .on_command_submitted(tick, sequence, caption, target);
    }

    /// Forward an immediate authoritative rejection to the dock telegraph.
    pub fn on_command_rejected(
        &mut self,
        tick: u64,
        sequence: Option<u64>,
        caption: impl Into<String>,
        reason: impl Into<String>,
        target: Option<Pos>,
    ) {
        self.dock
            .on_command_rejected(tick, sequence, caption, reason, target);
    }

    /// Reconcile pending commands and collect observed events after one
    /// authoritative simulation tick.
    pub fn after_authoritative_tick(&mut self, world: &World) {
        self.dock.after_authoritative_tick(world);
    }
}

#[derive(Serialize, Deserialize)]
struct SavedGuide {
    version: u32,
    world_hash: String,
    tutorial: Option<Tutorial>,
    guidance_visible: bool,
    #[serde(default)]
    tactics_fixture: bool,
    #[serde(default)]
    dock: DockState,
    #[serde(default)]
    traces: crate::tidal_traces::TraceState,
    #[serde(default)]
    chart_memory: crate::chart_memory::MemoryState,
    #[serde(default)]
    practice_complete: bool,
    #[serde(default)]
    practice_review: bool,
    /// Control groups travel with the match so a loaded field keeps the
    /// player's army and production groups. Restored members are limited to
    /// living local entities.
    #[serde(default)]
    groups: Vec<Vec<EntityId>>,
}

/// Keep only living local members, in stored order and without repeats.
pub(crate) fn sanitize_groups(saved: &[Vec<EntityId>], world: &World) -> [Vec<EntityId>; 10] {
    let mut groups: [Vec<EntityId>; 10] = Default::default();
    for (slot, members) in saved.iter().take(10).enumerate() {
        for &id in members {
            let alive = world
                .entities
                .iter()
                .any(|e| e.id == id && e.owner == 0 && e.hp > 0 && e.aboard.is_none());
            if alive && !groups[slot].contains(&id) {
                groups[slot].push(id);
            }
        }
    }
    groups
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing save folder")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("pending.json");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}

pub fn clock(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
pub fn role_description(kind: Kind) -> &'static str {
    match kind {
        Kind::Hook | Kind::Wick | Kind::Raker => {
            "Gathers salvage, builds, and repairs your machines."
        }
        Kind::Riveter => {
            "A sturdy fighter for close-range battles. Unpicks plating: 1.5x to buildings."
        }
        Kind::Bulwark => "Deploys a shield that halves damage from the front. Flanks stay exposed.",
        Kind::Sounder => {
            "+5 on a deployed Loom or Heliostat; Loom shells do 3/4. SOUND lights the ground."
        }
        Kind::Reedguard => "A durable fighter that holds the front line.",
        Kind::Skipper => "A fast raider that takes flooded lanes well. SOUND lights the ground.",
        Kind::Loom => {
            "Artillery. Deploys to fire ground blasts, 2x to buildings. Blind inside 2 cells."
        }
        Kind::Tidewatch | Kind::Lampwright | Kind::Stilt => {
            "A scout with no gun. Sees far, twice as far on Hold."
        }
        Kind::Caulker | Kind::Tender | Kind::Glazier => {
            "Mends the nearest damaged machine within 3 cells, 5 hull a second, for salvage."
        }
        Kind::Caisson => {
            "A plug. Deploys to block one cell and holds a crossing mouth like a gun machine."
        }
        Kind::Dredger => {
            "Gathers wrecks, wet lanes included, and takes flooded lanes at full speed."
        }
        Kind::Headquarters => {
            "Your base. Trains workers, pays 40 salvage a minute, holds VENT. Losing it ends the match."
        }
        Kind::Works => {
            "Trains the three combat roles and researches Plate and Siege. Adds 12 crew, up to 90."
        }
        Kind::Drydock => {
            "Trains the scout, mender and tide role. Researches Tracks. Adds 10 crew, up to 90."
        }
        Kind::Condenser => "Makes pressure on a well and raises the pressure cap by 100.",
        Kind::Dropoff => {
            "A drop-off near a wreck. Researches Cranes, Sonar and Scrap. Adds 6 crew."
        }
        Kind::Tower => {
            "Long gun. At a mouth: blocks enemy counts (3 seats: halves them), never holds."
        }
        Kind::Palisade => "A wall segment. Blocks movement, no gun. Shapes an approach.",
        Kind::Barge => {
            "Carries four machines over water, never dry ground. Unload (U) sets them on the shore."
        }
        Kind::Lifter => {
            "Flies four machines over anything. Unload (U) sets them down within three cells."
        }
        Kind::Brander => "The fastest line fighter, with the least hull and a short reach.",
        Kind::Heliostat => {
            "Deploys to fire a mirror beam. Each shot on the same target hits harder, up to 10."
        }
        Kind::Glinter => "+5 against a deployed Loom; Loom shells do 3/4. GLINT lights a long ray.",
        Kind::Salter => "Lays a dry salt causeway over tidal water. The next tide change melts it.",
        Kind::Pan => "Deploys on dry ground to boil pressure: 40 a minute, no well needed.",
    }
}

/// One line of flavour per machine and building, for the Guide's roster and
/// the tooltips: the setting lived almost entirely in names before (trial 8).
/// At most 43 glyphs, so it fits one roster line.
pub fn flavour(kind: Kind) -> &'static str {
    match kind {
        Kind::Hook => "Dock crew and a crane hook. Never idle.",
        Kind::Riveter => "Built to take hulls apart, rivet by rivet.",
        Kind::Bulwark => "A breakwater plate on wheels. It plants.",
        Kind::Sounder => "Pings the fog and listens for the answer.",
        Kind::Tidewatch => "A mast, a lamp and sharp eyes. No gun.",
        Kind::Caulker => "Seals a split hull while the fight goes on.",
        Kind::Caisson => "A cofferdam that drives itself into place.",
        Kind::Lifter => "A cargo cage on rotors. Walls mean nothing.",
        Kind::Wick => "Woven reed and a warm boiler. Co-op hands.",
        Kind::Reedguard => "Layered reed armour. It bends, not breaks.",
        Kind::Skipper => "Paddles the shallows ahead of the tide.",
        Kind::Loom => "Weaves its shells in long arcs over water.",
        Kind::Lampwright => "Keeps the lamps that lead the co-op home.",
        Kind::Tender => "Patches woven hulls with reed and resin.",
        Kind::Dredger => "Digs up what the sea took back.",
        Kind::Barge => "A tug hull rebuilt to carry the co-op.",
        Kind::Headquarters => "The harbour office. Lose it, lose all.",
        Kind::Works => "Where machines are riveted or woven.",
        Kind::Drydock => "Hulls go in broken and come out new.",
        Kind::Condenser => "Pulls steam from a deep well for the lines.",
        Kind::Dropoff => "Scrap in, salvage out. It never shuts.",
        Kind::Tower => "A gun nest dug into the bank.",
        Kind::Palisade => "Piles driven into the silt, side by side.",
        Kind::Raker => "Rakes the salt flats. Brine in the joints.",
        Kind::Brander => "A fire-lance on long legs. First to arrive.",
        Kind::Heliostat => "A yoke of mirrors. It burns one thing down.",
        Kind::Glinter => "Flashes a lens down a lane and sees it all.",
        Kind::Stilt => "Tall enough to see over the next tide.",
        Kind::Glazier => "Fuses a cracked plate shut with a lens.",
        Kind::Salter => "Crusts the shallows into road, for a while.",
        Kind::Pan => "Mirrors, a pan, and brine boiled to steam.",
    }
}

/// What a role beats and what beats it, in the C&C style.  The machines
/// named are always another side's (or a role, "NESTS", "RAIDERS"): a
/// Reedguard "lost to Looms", its own side's gun (trial 10).
///
/// Since rules 22 every machine named here is checked by a headless duel
/// for equal salvage (`rules22_words`): in trial 12 the card sent 39
/// Sounders at Heliostats on a bonus that did not exist. The Riveter beat
/// every other side's machine in those duels, so only nests beat it; the
/// Reedguard lost to Riveters and Branders, not the other way round; a
/// deployed Bulwark held only against Skippers; and the nest beat every
/// army of its own cost, falling only to twice its cost.
pub fn matchups(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Hook | Kind::Wick | Kind::Raker => ("", "ANYTHING WITH A GUN"),
        Kind::Riveter => ("BUILDINGS, LIGHT MACHINES", "NESTS"),
        Kind::Bulwark => ("SKIPPERS", "FLANKS, LOOMS, BRANDERS, REEDGUARDS"),
        Kind::Sounder => (
            "DEPLOYED LOOMS, HELIOSTATS, WORKERS",
            "REEDGUARDS, BRANDERS, NESTS",
        ),
        Kind::Reedguard => ("SOUNDERS, GLINTERS, HELIOSTATS", "RIVETERS, BRANDERS"),
        Kind::Skipper => ("WORKERS, SCOUTS, MENDERS", "BULWARKS, BRANDERS, NESTS"),
        Kind::Loom => (
            "BUILDINGS, DEPLOYED LINES",
            "SOUNDERS, GLINTERS, ANY FOE INSIDE 2",
        ),
        Kind::Tidewatch | Kind::Lampwright | Kind::Stilt => ("", "ANYTHING WITH A GUN"),
        Kind::Caulker | Kind::Tender | Kind::Glazier => ("", "FOCUS FIRE"),
        Kind::Brander => ("REEDGUARDS, BULWARKS, RAIDERS", "LOOMS, RIVETERS"),
        Kind::Heliostat => ("HEAVY MACHINES, BUILDINGS", "MANY CHEAP TARGETS, LOOMS"),
        Kind::Glinter => ("LOOM LINES, SCOUTS", "REEDGUARDS, NESTS"),
        Kind::Salter => ("THE TIDE WALL", "ANY GUN, SLOW"),
        Kind::Pan => ("", "RAIDERS, ANY GUN"),
        Kind::Caisson => ("CROSSINGS", "LOOMS, HELIOSTATS"),
        Kind::Dredger => ("WET WRECKS", "NESTS, SOUNDERS ON DRY GROUND"),
        Kind::Tower => ("RAIDERS, SCOUTS", "MASSED LOOMS, HELIOSTATS, RIVETERS"),
        Kind::Palisade => ("", "RIVETERS, LOOMS, HELIOSTATS"),
        Kind::Barge => ("THE TIDE WALL", "NESTS ON THE SHORE, ANY GUN"),
        Kind::Lifter => ("THE TIDE WALL, PALISADES", "ANY GUN, SLOW"),
        Kind::Headquarters | Kind::Works | Kind::Drydock | Kind::Condenser | Kind::Dropoff => {
            ("", "RIVETERS, LOOMS, HELIOSTATS")
        }
    }
}

/// Whether a matchup line names `kind`: "LOOMS" and "LOOM LINES" name the
/// Loom.
fn names(line: &str, kind: Kind) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word == kind.name() || word == format!("{}S", kind.name()))
}

/// Your side's machines that answer an enemy's `kind`: those its WEAK line
/// names, and those whose STRONG line names it (trial 10: "there is no
/// guidance on what of mine beats what I face").
pub(crate) fn answers(kind: Kind, yours: Faction) -> Vec<Kind> {
    let (_, weak) = matchups(kind);
    crate::menus::roster_machines(yours)
        .into_iter()
        .filter(|mine| names(weak, *mine) || names(matchups(*mine).0, kind))
        .collect()
}

/// A machine or building tooltip in one fixed order: cost and time, role,
/// STRONG VS, WEAK VS, the key, then a line of flavour.
pub fn kind_tooltip(kind: Kind, key: &str) -> String {
    let s = spec(kind);
    let mut cost = format!("{} salvage", s.salvage);
    if s.pressure > 0 {
        cost.push_str(&format!(", {} pressure", s.pressure));
    }
    if s.crew > 0 {
        cost.push_str(&format!(", {} crew", s.crew));
    }
    cost.push_str(&format!(", {}s", s.build_ticks.div_ceil(30)));
    let (strong, weak) = matchups(kind);
    let mut lines = vec![cost, role_description(kind).to_string()];
    if !strong.is_empty() {
        lines.push(format!("STRONG VS {strong}"));
    }
    if !weak.is_empty() {
        lines.push(format!("WEAK VS {weak}"));
    }
    if let Some(note) = counter_note(kind) {
        lines.push(note.to_string());
    }
    lines.push(key.to_string());
    // The flavour closes the card, the least urgent line on it.
    lines.push(flavour(kind).to_string());
    lines.join("\n")
}

/// The one fact a Union line needs against the Assembly's artillery, on the
/// machines that meet it.
pub fn counter_note(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Riveter | Kind::Bulwark | Kind::Sounder => {
            Some("LOOMS ARE BLIND INSIDE 2 CELLS: SURGE (Z) TO CLOSE.")
        }
        _ => None,
    }
}

/// The persistent stats line for the selection panel: damage, reach, speed.
pub fn stats_line(kind: Kind) -> String {
    let s = spec(kind);
    let mut parts = Vec::new();
    if s.damage > 0 {
        parts.push(format!("DMG {}", s.damage));
        parts.push(format!("REACH {}", s.range / FP));
    }
    if s.speed > 0 {
        parts.push(format!("SPEED {}", s.speed / FP));
    }
    parts.push(format!("SIGHT {}", s.sight / FP));
    parts.join(" / ")
}

/// The key that opens a building's placement.
pub(crate) fn build_hotkey(kind: Kind) -> &'static str {
    match kind {
        Kind::Works => "B",
        Kind::Condenser => "C",
        Kind::Dropoff => "Y",
        Kind::Tower => "V",
        Kind::Drydock => "N",
        _ => "P",
    }
}

/// A building's name as its side calls it.  The kinds are shared; the
/// Compact names three of them its own way (design/third-faction-and-
/// confluence.md, "Buildings"): the Kiln, the Glassworks and the Rake shed.
/// The Union and Assembly keep the names the game has always shown.
pub fn building_name(kind: Kind, faction: Faction) -> &'static str {
    match (faction, kind) {
        (Faction::Compact, Kind::Headquarters) => "KILN",
        (Faction::Compact, Kind::Works) => "GLASSWORKS",
        (Faction::Compact, Kind::Dropoff) => "RAKE SHED",
        _ => kind.name(),
    }
}

/// A machine's name in a sentence, plural: "Riveters", "Heliostats".
fn plural_word(kind: Kind) -> String {
    let name = kind.name();
    let mut word = capitalise(&name.to_lowercase());
    word.push('s');
    word
}

/// The machines of `faction` an upgrade changes, as the simulation has it:
/// Plate armours `bw_sim::plated` kinds; Siege works on the Riveter, the
/// Loom and the Heliostat; every other upgrade on the whole side.
pub(crate) fn upgrade_machines(upgrade: bw_content::Upgrade, faction: Faction) -> Vec<Kind> {
    use bw_content::Upgrade;
    let machines = crate::menus::roster_machines(faction);
    match upgrade {
        Upgrade::Plate => machines
            .into_iter()
            .filter(|k| bw_sim::plated(*k))
            .collect(),
        Upgrade::Siege => machines
            .into_iter()
            .filter(|k| matches!(k, Kind::Riveter | Kind::Loom | Kind::Heliostat))
            .collect(),
        _ => machines,
    }
}

/// One line per upgrade for buttons, tooltips and the guide, for the
/// reader's own faction only: trial 10's Compact read Plate as a Bulwark
/// and Reedguard upgrade and Siege as the other two sides' (it helps its
/// Branders, Heliostats and beam).
pub fn upgrade_description(upgrade: bw_content::Upgrade, faction: Faction) -> String {
    use bw_content::Upgrade;
    let machines = upgrade_machines(upgrade, faction);
    if machines.is_empty() {
        return "Does nothing for your machines.".into();
    }
    match upgrade {
        Upgrade::Plate => {
            let names: Vec<String> = machines.iter().map(|k| plural_word(*k)).collect();
            format!(
                "{} carry 25% more hull, new ones too.",
                match names.as_slice() {
                    [one] => one.clone(),
                    [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
                    [] => String::new(),
                }
            )
        }
        Upgrade::Siege => match faction {
            Faction::Union => "Riveters do 2x damage to buildings.".into(),
            Faction::Assembly => "Looms reach one cell farther and deploy in half the time.".into(),
            Faction::Compact => {
                "Heliostats deploy in half the time; the beam builds 2 a shot, not 1.".into()
            }
        },
        other => upgrade_line(other).into(),
    }
}

/// The upgrades every side reads the same way.
fn upgrade_line(upgrade: bw_content::Upgrade) -> &'static str {
    use bw_content::Upgrade;
    match upgrade {
        Upgrade::Plate | Upgrade::Siege => "",
        Upgrade::Tracks => "Flooded lanes slow your machines half as much.",
        Upgrade::Cranes => "Workers carry 2 more salvage.",
        Upgrade::SalvageSonar => "Every wreck shows its salvage on the chart, seen or not.",
        Upgrade::ScrapRecovery => {
            "Kills leave double scrap: a combat machine you destroy leaves a wreck worth its whole cost instead of half."
        }
        Upgrade::Overpressure => "Pressure cap +100.",
        Upgrade::BleedValves => "Pressure income +60 a minute.",
        Upgrade::Temper => "Every combat machine deals 10% more damage. Needs no crew.",
        Upgrade::Refit => "Every combat machine carries 15% more hull. Needs no crew.",
        Upgrade::Overhaul1 | Upgrade::Overhaul2 | Upgrade::Overhaul3 => {
            "Every machine you own, built or not, carries 6% more hull. Salvage only, three levels."
        }
    }
}

/// A phrase with its first letter in capitals: "the east" to "The east".
pub(crate) fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A price in the command card's words: "180S+30P", or "50S" alone.
pub fn price(salvage: u32, pressure: u32) -> String {
    if pressure == 0 {
        format!("{salvage}S")
    } else {
        format!("{salvage}S+{pressure}P")
    }
}

/// An upgrade's name on its command button, short enough for the card; the
/// tooltip gives the rest.
pub fn upgrade_card_name(upgrade: bw_content::Upgrade) -> &'static str {
    match upgrade {
        bw_content::Upgrade::Overpressure => "OVERPRESS",
        bw_content::Upgrade::BleedValves => "BLEEDERS",
        other => other.name(),
    }
}

/// The two-row hint for an upgrade button: cost, then effect.  Every row
/// fits the card's ten glyphs; the research time is in the tooltip.
pub fn upgrade_hint(upgrade: bw_content::Upgrade, faction: Faction) -> String {
    use bw_content::Upgrade;
    let (salvage, pressure, _) = upgrade.cost();
    let effect = match upgrade {
        Upgrade::Plate => "HULL +25%",
        Upgrade::Siege if faction == Faction::Union => "X2 VS BASE",
        Upgrade::Siege if faction == Faction::Assembly => "LOOM REACH",
        Upgrade::Siege => "BEAM X2",
        Upgrade::Tracks => "WADES FAST",
        Upgrade::Cranes => "CARRY +2",
        Upgrade::SalvageSonar => "SEE WRECKS",
        Upgrade::ScrapRecovery => "2X SCRAP",
        Upgrade::Overpressure => "CAP +100",
        Upgrade::BleedValves => "+60/M",
        Upgrade::Temper => "DMG +10%",
        Upgrade::Refit => "HULL +15%",
        Upgrade::Overhaul1 | Upgrade::Overhaul2 | Upgrade::Overhaul3 => "HULL +6%",
    };
    format!("{}\n{effect}", price(salvage, pressure))
}

impl Game {
    pub fn has_session(&self) -> bool {
        self.ux.session_active && self.world.outcome.is_none()
    }

    /// Root action hook for `Action::EndReview`. This returns from the
    /// practice-only dock review without assigning a simulation outcome.
    pub fn end_practice_review(&mut self) {
        self.ux.end_practice_review();
        self.selected.clear();
        self.mode = Mode::Context;
        self.message.clear();
        self.open_screen(Screen::Menu);
    }

    /// Why the tide cannot be set now, or None.
    pub(crate) fn tide_reason(&self, flood: bool) -> Option<String> {
        let gate = &self.world.gate;
        if gate.owner != Some(0) {
            return Some("Capture the sluice with a combat machine first.".into());
        }
        if let Some(until) = gate.warning_until {
            return Some(format!(
                "The tide is changing in {}s.",
                until.saturating_sub(self.world.tick).div_ceil(30)
            ));
        }
        if gate.tide == bw_sim::Tide::Flood {
            return Some(format!(
                "The flood falls in {}s.",
                gate.flood_until
                    .unwrap_or(self.world.tick)
                    .saturating_sub(self.world.tick)
                    .div_ceil(30)
            ));
        }
        if self.world.tick < gate.locked_until {
            return Some(format!(
                "The station can change the tide again in {}s.",
                (gate.locked_until - self.world.tick).div_ceil(30)
            ));
        }
        let price = if flood {
            bw_content::FLOOD_PRESSURE
        } else {
            bw_content::SWITCH_PRESSURE
        };
        if self.world.players[0].pressure < price {
            return Some(format!(
                "Need {} pressure; you have {}.",
                price, self.world.players[0].pressure
            ));
        }
        if self.world.station_contested_for(0) {
            return Some(
                "An enemy stands at the station: clear it first. Nothing is spent.".into(),
            );
        }
        None
    }
    pub fn menu_model(&mut self) -> MenuModel {
        if self.ux.cached_save_dir != self.data_dir {
            self.ux.refresh_save(&self.data_dir);
        }
        // The question is the title; the consequence is one line. Replacing
        // or closing a field loses whatever is not saved, and the Save first
        // control on the same screen is the answer to that.
        let lost = "Unsaved progress will be lost.";
        let (title, body) = match self.ux.pending {
            Some(PendingAction::NewSkirmish) => ("Start a new skirmish?", lost),
            Some(PendingAction::NewPractice) => ("Start practice?", lost),
            Some(PendingAction::Load) => ("Load the saved match?", lost),
            Some(PendingAction::Surrender) => (
                "Surrender?",
                if self.session.is_some() {
                    "Your opponent wins this match."
                } else {
                    "The computer wins this match."
                },
            ),
            Some(PendingAction::EndPractice) => ("End practice?", lost),
            Some(PendingAction::Quit) => ("Quit Brinewake?", lost),
            None => ("Continue?", ""),
        };
        MenuModel {
            selected_faction: self.faction,
            selected_map: self.map,
            selected_opponent: self.opponent,
            selected_ai_level: self.ai_level,
            has_session: self.has_session(),
            network_session: self.session.is_some(),
            practice_session: self.ux.practice,
            practice_complete: self.ux.practice_complete,
            practice_review: self.ux.practice_review,
            session_seconds: self.world.tick / 30,
            save_available: self.ux.save_available,
            save_description: self.ux.save_description.clone(),
            audio_on: !self.muted,
            effects_volume: self.ux.preferences.effects_volume.min(100),
            ambience_volume: self.ux.preferences.ambience_volume.min(100),
            music_volume: self.ux.preferences.music_volume.min(100),
            edge_scroll: self.ux.preferences.edge_scroll,
            pause_unfocused: self.ux.preferences.pause_unfocused,
            game_speed: self.ux.preferences.game_speed,
            always_health_bars: self.ux.preferences.always_health_bars,
            paused_unfocused: self.ux.paused_unfocused,
            fullscreen: self.ux.fullscreen,
            tutorial_completed: self.ux.preferences.tutorial_completed,
            help_page: self.ux.help_page,
            roster_faction: self.ux.roster_faction.unwrap_or(self.faction),
            roster_buildings: self.ux.roster_buildings,
            // The card under focus shows its details; else the one clicked.
            roster_unit: match self.ux.focused {
                Some(Action::RosterUnit(kind)) => Some(kind),
                _ => self.ux.roster_unit,
            },
            interface_scale: self.ux.preferences.interface_scale,
            settings_tab: self.ux.settings_tab,
            home_tick: self.frame,
            confirmation_title: title.into(),
            confirmation_body: body.into(),
            confirmation_action: match self.ux.pending {
                Some(PendingAction::NewSkirmish) => "START MATCH",
                Some(PendingAction::NewPractice) => "START PRACTICE",
                Some(PendingAction::Load) => "LOAD",
                Some(PendingAction::Surrender) => "SURRENDER",
                Some(PendingAction::EndPractice) => "END PRACTICE",
                Some(PendingAction::Quit) => "QUIT",
                None => "CONTINUE",
            }
            .into(),
            can_save_before_confirm: self.has_session()
                && self.session.is_none()
                && self.ux.pending != Some(PendingAction::Load),
            overlay: (self.screen == Screen::Match && self.world.outcome.is_some())
                || self.has_session()
                    && (self.screen == Screen::Pause
                        || (self.screen == Screen::Match && self.ux.practice_review)
                        || (self.screen == Screen::Confirm
                            && matches!(self.ux.confirm_return, Screen::Pause | Screen::Match))),
            just_saved: self.session.is_none() && self.ux.saved_at_tick == Some(self.world.tick),
            practice_progress: self.ux.practice_progress.clone(),
            surrendered: self.world.outcome.is_some()
                && self.world.command_log.iter().any(|r| {
                    r.player == 0
                        && r.applied == Some(true)
                        && matches!(r.command, Command::Surrender)
                }),
            dock: self.ux.dock.clone(),
            manual: self.ux.manual.clone(),
            // A recording's result shows the whole match from the scan, not
            // only the part watched since the last seek.
            summary: self
                .playback
                .as_ref()
                .and_then(|p| p.index_summary())
                .filter(|(_, _, done)| *done)
                .map(|(summary, ..)| summary)
                .unwrap_or_else(|| self.summary.clone()),
            result_page: self.result_page,
            result_log_scroll: self.result_log_scroll,
            observing: self.playback.is_some(),
            // The Guide states the hold length in force (trial 12).
            hold_seconds: (self.has_session() && self.world.map.id == self.map)
                .then(|| self.world.hold_ticks() / bw_core::TICK_HZ as u32),
        }
    }

    pub fn open_screen(&mut self, screen: Screen) {
        let screen_changed = self.screen != screen;
        let leaving_match = self.screen == Screen::Match && screen != Screen::Match;
        if leaving_match && let Some(audio) = &self.audio {
            audio.hush();
        }
        if screen_changed {
            self.message.clear();
            self.message_until = 0;
        }
        self.screen = screen;
        if screen == Screen::Match {
            self.ux.paused_unfocused = false;
        }
        self.ux.focused = None;
        self.drag = None;
        self.ux.minimap_drag = false;
        self.buttons.clear();
        self.ux.keyboard_navigation = false;
        if matches!(screen, Screen::Menu | Screen::Pause) {
            self.ux.refresh_save(&self.data_dir);
        }
    }

    pub fn begin_practice(&mut self) {
        self.start();
        self.world.ai_enabled = false;
        self.ux.reset_dock_for_new_match();
        self.ux.practice = true;
        self.ux.tutorial = Some(Tutorial::new(&self.world));
        self.ux.guidance_visible = true;
        self.selected.clear();
        self.message.clear();
        self.ux.markers.clear();
        self.begin_lesson_intro();
    }

    pub fn request_transition(&mut self, pending: PendingAction) {
        let destructive = self.has_session()
            && (pending == PendingAction::Surrender
                || self.ux.last_saved_hash.as_deref() != Some(self.world.state_hash().as_str()));
        if destructive {
            self.ux.pending = Some(pending);
            self.ux.confirm_return = self.screen;
            self.open_screen(Screen::Confirm);
        } else {
            self.finish_transition(pending);
        }
    }

    pub fn finish_transition(&mut self, pending: PendingAction) {
        self.ux.pending = None;
        match pending {
            PendingAction::NewSkirmish => {
                self.start();
                self.begin_intro();
            }
            PendingAction::NewPractice => self.begin_practice(),
            PendingAction::Load => self.load_match(),
            PendingAction::Quit => {
                // The others hear goodbye rather than silence.
                self.end_network();
                self.ux.quit_requested = true;
            }
            PendingAction::Surrender => {
                self.issue(Command::Surrender);
                self.open_screen(Screen::Match);
            }
            PendingAction::EndPractice => {
                if self.ux.tutorial.as_ref().is_some_and(Tutorial::is_complete) {
                    self.ux.mark_practice_complete();
                }
                self.ux.practice_progress = match self.ux.tutorial.as_ref() {
                    Some(t) => {
                        let view = t.view(&self.world);
                        let done = if view.complete {
                            view.total
                        } else {
                            view.step - 1
                        };
                        (
                            done,
                            view.total,
                            if view.complete {
                                String::new()
                            } else {
                                view.title
                            },
                        )
                    }
                    None => (0, 7, String::new()),
                };
                self.ux.begin_practice_review();
                // Keep the field active for the review overlay so Save and
                // Replay remain reachable. The root tick hook freezes the
                // simulation while `practice_review` is true.
                self.ux.session_active = true;
                self.ux.practice = true;
                self.aftermath_ticks = 0;
                self.ux.tutorial = None;
                self.selected.clear();
                self.mode = Mode::Context;
                self.message.clear();
                self.open_screen(Screen::Match);
            }
        }
    }

    pub fn save_match(&mut self) -> bool {
        if !self.ux.session_active {
            self.notify("Start a match or practice before saving.");
            return false;
        }
        let dir = self.data_dir.join("saves");
        let temp = dir.join("quick-save.pending.json");
        let result = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|_| self.world.save(&temp))
            .and_then(|_| {
                std::fs::rename(&temp, dir.join("quick-save.json")).map_err(|e| e.to_string())
            });
        match result {
            Ok(()) => {
                let hash = self.world.state_hash();
                self.ux.last_saved_hash = Some(hash.clone());
                self.ux.saved_at_tick = Some(self.world.tick);
                let guide = SavedGuide {
                    version: 1,
                    world_hash: hash,
                    tutorial: self.ux.tutorial.clone(),
                    guidance_visible: self.ux.guidance_visible,
                    tactics_fixture: self.ux.tactics_fixture,
                    dock: self.ux.dock.clone(),
                    traces: self.ux.traces.clone(),
                    chart_memory: self.ux.chart_memory.clone(),
                    practice_complete: self.ux.practice_complete,
                    practice_review: self.ux.practice_review,
                    groups: self.groups.to_vec(),
                };
                let guide_ok = serde_json::to_vec(&guide)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| atomic_write(&dir.join("quick-save-ui.json"), &bytes))
                    .is_ok();
                self.ux.refresh_save(&self.data_dir);
                // The pause menu marks its own SAVE row instead.
                if !guide_ok {
                    self.notify("Saved. Practice progress could not be saved.");
                } else if self.screen != Screen::Pause {
                    self.notify("Saved.");
                }
                true
            }
            Err(error) => {
                eprintln!("Save failed: {error}");
                self.notify("Could not save. Your current field is still open.");
                false
            }
        }
    }

    pub fn load_match(&mut self) {
        let dir = self.data_dir.join("saves");
        match World::load(dir.join("quick-save.json")) {
            Ok(world) => {
                let hash = world.state_hash();
                let guide: Option<SavedGuide> = std::fs::read(dir.join("quick-save-ui.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .filter(|g: &SavedGuide| g.version == 1 && g.world_hash == hash);
                if let Some(audio) = &self.audio {
                    audio.hush();
                }
                self.world = world;
                // The Guide and the setup follow the loaded field's map.
                self.map = self.world.map.id;
                self.aftermath_ticks = 0;
                // The summary starts at the loaded tick and says so.
                self.summary = Default::default();
                self.ux.tactics_fixture = guide.as_ref().is_some_and(|g| g.tactics_fixture);
                let restored_groups = guide
                    .as_ref()
                    .map(|g| sanitize_groups(&g.groups, &self.world))
                    .unwrap_or_default();
                if let Some(guide) = guide.as_ref() {
                    self.ux.dock = guide.dock.clone();
                    self.ux.traces = guide.traces.clone();
                    self.ux.traces.sanitize(&self.world);
                    self.ux.chart_memory = guide.chart_memory.clone();
                    self.ux.chart_memory.sanitize(&self.world);
                    if !self.ux.dock.log.history_available {
                        self.ux.dock.mark_missing_history(self.world.tick);
                    }
                    self.ux.practice_complete = guide.practice_complete;
                    self.ux.practice_review = guide.practice_review;
                } else {
                    // A missing or hash-mismatched sidecar must never invent
                    // a match chronicle. The result renderer shows its safe
                    // "history unavailable" label from this empty state.
                    self.ux.dock.mark_missing_history(self.world.tick);
                    self.ux.traces = crate::tidal_traces::TraceState::default();
                    self.ux.chart_memory = crate::chart_memory::MemoryState::default();
                    self.ux.practice_complete = false;
                    self.ux.practice_review = false;
                }
                self.ux.dock.recover_from_world(&self.world);
                self.faction = self.world.players[0].faction;
                self.ux.session_active = true;
                self.ux.practice = !self.world.ai_enabled;
                self.ux.guidance_visible = guide.as_ref().is_some_and(|g| g.guidance_visible);
                self.ux.tutorial = if self.ux.practice {
                    guide.and_then(|g| g.tutorial)
                } else {
                    None
                };
                self.ux.last_saved_hash = Some(hash);
                self.ux.saved_at_tick = Some(self.world.tick);
                self.ux.pending = None;
                self.ux.markers.clear();
                self.ux.alerts = Default::default();
                self.selected.clear();
                self.replay_saved = false;
                self.last_click = None;
                self.last_group = None;
                self.drag = None;
                self.mode = Mode::Context;
                self.groups = restored_groups;
                self.motion.clear();
                self.effects.clear();
                self.fire_starts.clear();
                self.unload_starts.clear();
                self.resource_stages.clear();
                self.ux_nudges = Default::default();
                self.gate_foam_start = None;
                self.gate_switch = None;
                self.home();
                self.open_screen(Screen::Match);
                self.notify("Save loaded.");
            }
            Err(error) => {
                eprintln!("Load failed: {error}");
                self.ux.refresh_save(&self.data_dir);
                self.open_screen(if self.has_session() {
                    Screen::Pause
                } else {
                    Screen::Menu
                });
                self.notify(
                    if error.contains("incompatible")
                        || error.contains("version")
                        || error.contains("rules")
                    {
                        "This save uses different rules. Start a new match. The saved file is kept."
                    } else {
                        "Could not load the save. Your current field is unchanged."
                    },
                );
            }
        }
    }

    pub fn update_tutorial(&mut self) {
        if let Some(tutorial) = &mut self.ux.tutorial {
            tutorial.update(&self.world, &self.selected);
            if tutorial.is_complete() && !self.ux.preferences.tutorial_completed {
                self.ux.preferences.tutorial_completed = true;
                let _ = self.ux.persist(&self.data_dir);
            }
        }
    }

    pub fn focus_target(&mut self, target: TutorialTarget) {
        if target == TutorialTarget::Gate {
            self.camera.center(self.world.map.gate_pos);
            return;
        }
        let picked = self
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none())
            .filter(|e| match target {
                TutorialTarget::Worker => e.kind.is_worker(),
                TutorialTarget::Works => e.kind == Kind::Works,
                TutorialTarget::Army => !e.kind.is_worker() && !e.kind.is_building(),
                _ => false,
            })
            .min_by_key(|e| (e.build_remaining, e.id))
            .map(|e| (e.id, e.pos));
        if let Some((id, pos)) = picked {
            self.selected = vec![id];
            self.camera.center(pos);
            self.update_tutorial();
        } else {
            self.notify(match target {
                TutorialTarget::Works => "Build a Works with a worker first.",
                TutorialTarget::Army => "Train a combat machine at your Works first.",
                _ => "No worker available. Train one at headquarters.",
            });
        }
    }

    /// The selected own finished headquarters, for RECLAIM and VENT keys.
    pub(crate) fn reclaim_hq(&self) -> Option<EntityId> {
        self.first_building()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
            .filter(|e| e.owner == 0 && e.kind == Kind::Headquarters && e.build_remaining == 0)
            .map(|e| e.id)
    }

    /// The selected machines that could board, and the nearest own
    /// transport with room in the field.
    pub(crate) fn board_target(&self) -> Option<(Vec<EntityId>, EntityId)> {
        let riders: Vec<EntityId> = self.gameplay_ids(|k| !k.is_building() && !k.is_transport());
        let first = self
            .world
            .entities
            .iter()
            .find(|e| Some(&e.id) == riders.first())?
            .pos;
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.build_remaining == 0
                    && e.aboard.is_none()
                    && e.kind.is_transport()
                    && e.cargo.len() < bw_content::TRANSPORT_CAPACITY
            })
            .min_by_key(|e| (e.pos.distance_sq(first), e.id))
            .map(|e| (riders, e.id))
    }

    pub fn gameplay_ids(&self, predicate: impl Fn(Kind) -> bool) -> Vec<EntityId> {
        self.selected
            .iter()
            .filter(|id| {
                self.world.entities.iter().any(|e| {
                    e.id == **id
                        && e.owner == 0
                        && e.hp > 0
                        && e.aboard.is_none()
                        && predicate(e.kind)
                })
            })
            .copied()
            .collect()
    }

    pub fn action_reason(&self, action: &Action) -> Option<String> {
        let gameplay = matches!(
            action,
            Action::FilterSelection(_, _)
                | Action::RosterPage(_)
                | Action::RecallGroup(_)
                | Action::CenterGroup(_)
                | Action::Train(_)
                | Action::TrainBatch(_)
                | Action::SelectWorks
                | Action::FocusAlert(_)
                | Action::FocusHold
                | Action::FocusLane(_)
                | Action::Build(_)
                | Action::Gather
                | Action::Attack
                | Action::Stop
                | Action::Hold
                | Action::Deploy
                | Action::Pack
                | Action::KeepDeployed
                | Action::Capture
                | Action::Reclaim
                | Action::Recycle
                | Action::Board
                | Action::Switch
                | Action::SetTide(_)
                | Action::Flood
                | Action::Cancel
                | Action::Formation
                | Action::Face
                | Action::Surge
                | Action::Research(_)
                | Action::CancelResearch
                | Action::Upgrade(_)
                | Action::CancelUpgrade
                | Action::Vent
                | Action::Unload
                | Action::Sound
                | Action::Glint
                | Action::Lay
                | Action::IdleWorker
                | Action::AllIdleWorkers
                | Action::SelectArmy
        );
        if gameplay
            && (self.screen != Screen::Match
                || self.world.outcome.is_some()
                || self.ux.practice_review)
        {
            return Some("Return to an active field to give orders.".into());
        }
        if let Some(reason) = self.tactical_action_reason(action) {
            return Some(reason);
        }
        match action {
            Action::AllIdleWorkers if self.idle_worker_ids().is_empty() => {
                Some("No idle workers. Your current selection is unchanged.".into())
            }
            Action::IdleWorker
                if !self.world.entities.iter().any(|e| {
                    e.owner == 0 && e.hp > 0 && e.aboard.is_none() && e.kind.is_worker()
                }) =>
            {
                Some("No workers.".into())
            }
            Action::SelectArmy if self.army_ids().is_empty() => {
                Some("No combat machines yet. Train them at Works.".into())
            }
            Action::SelectWorks if self.works_ids().is_empty() => {
                Some("No completed Works. Select a worker and build one with B.".into())
            }
            Action::Start | Action::StartPractice | Action::Setup if self.atlas.is_none() => {
                Some("Game artwork is missing. Open the complete app build.".into())
            }
            Action::Resume if !self.has_session() => Some("No active field to continue.".into()),
            Action::Save | Action::ExportReplay | Action::WatchReplay
                if !self.ux.session_active =>
            {
                Some("Start a match or practice first.".into())
            }
            Action::Load if !self.ux.save_available => Some(self.ux.save_description.clone()),
            Action::SaveAndConfirm if self.ux.pending == Some(PendingAction::Load) => {
                Some("Keep this field to save it. Loading must preserve the saved slot.".into())
            }
            Action::Train(kind) | Action::TrainBatch(kind) => self.training_reason(*kind),
            Action::Build(kind) => {
                if self.first_worker().is_none() {
                    // The key is named: V meant VENT to one seat.
                    Some(format!(
                        "{} builds a {}: select a worker first.",
                        build_hotkey(*kind),
                        building_name(*kind, self.faction)
                    ))
                } else {
                    self.resource_reason(*kind)
                }
            }
            Action::Gather if self.gameplay_ids(|k| k.gathers()).is_empty() => {
                Some("Select a worker or Dredger to gather salvage.".into())
            }
            Action::Capture
                if self
                    .gameplay_ids(|k| !k.is_worker() && !k.is_building())
                    .is_empty() =>
            {
                Some("Select a combat machine. Workers cannot capture the sluice.".into())
            }
            Action::Upgrade(upgrade) => {
                let building = self
                    .first_building()
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == id));
                let Some(building) = building.filter(|e| e.build_remaining == 0) else {
                    return Some("Select the finished building that offers this upgrade.".into());
                };
                if building.kind != upgrade.building() {
                    return Some(format!(
                        "{} is researched at the {}.",
                        upgrade.name(),
                        building_name(upgrade.building(), self.faction)
                    ));
                }
                if self.world.players[0].upgrades.contains(upgrade) {
                    return Some(format!("{} is complete.", upgrade.name()));
                }
                if let Some(before) = upgrade.requires()
                    && !self.world.players[0].upgrades.contains(&before)
                    && building.upgrade.is_none_or(|job| job.upgrade != before)
                    && !building.upgrade_queue.contains(&before)
                {
                    return Some(format!("{} needs {} first.", upgrade.name(), before.name()));
                }
                if self.world.entities.iter().any(|e| {
                    e.owner == 0
                        && (e.upgrade.is_some_and(|job| job.upgrade == *upgrade)
                            || e.upgrade_queue.contains(upgrade))
                }) {
                    return Some(format!(
                        "{} is already researching or queued.",
                        upgrade.name()
                    ));
                }
                if building.upgrade.is_some()
                    && building.upgrade_queue.len() >= bw_content::UPGRADE_QUEUE
                {
                    return Some(format!(
                        "This building's upgrade queue is full: {} waiting.",
                        building.upgrade_queue.len()
                    ));
                }
                crate::trial12_hold::upgrade_shortfall(self, *upgrade)
            }
            Action::CancelUpgrade => {
                let running = self
                    .first_building()
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
                    .is_some_and(|e| e.upgrade.is_some());
                (!running).then(|| "No upgrade is researching here.".into())
            }
            Action::Unload => {
                let transports = self.gameplay_ids(|k| k.is_transport());
                if transports.is_empty() {
                    Some("Select a Barge or Lifter to unload.".into())
                } else if !transports.iter().any(|id| {
                    self.world
                        .entities
                        .iter()
                        .any(|e| e.id == *id && !e.cargo.is_empty())
                }) {
                    Some("The hold is empty. BOARD (E) with machines selected, or right-click the transport.".into())
                } else {
                    None
                }
            }
            Action::Board => {
                if self
                    .gameplay_ids(|k| !k.is_building() && !k.is_transport())
                    .is_empty()
                {
                    Some("Select machines to board.".into())
                } else if self.board_target().is_none() {
                    Some("No own transport with room in the field.".into())
                } else {
                    None
                }
            }
            Action::Recycle => {
                if self.gameplay_ids(|k| k.is_worker()).is_empty() {
                    return Some("Select workers to RECYCLE.".into());
                }
                let yard = self.world.entities.iter().any(|e| {
                    e.owner == 0
                        && e.hp > 0
                        && e.build_remaining == 0
                        && matches!(e.kind, Kind::Headquarters | Kind::Works | Kind::Dropoff)
                });
                (!yard).then(|| "Needs your headquarters, a Works or a Salvage Yard.".into())
            }
            Action::Reclaim => {
                if self.reclaim_hq().is_none() {
                    return Some("Select your finished headquarters to RECLAIM.".into());
                }
                let pressure = self.world.players[0].pressure;
                (pressure < bw_content::RECLAIM_PRESSURE).then(|| {
                    format!(
                        "Need {} more pressure.",
                        bw_content::RECLAIM_PRESSURE - pressure
                    )
                })
            }
            Action::Vent => {
                let hq = self
                    .first_building()
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == id))
                    .filter(|e| e.kind == Kind::Headquarters && e.build_remaining == 0);
                if hq.is_none() {
                    return Some("Select your finished headquarters to open VENT.".into());
                }
                if self.world.players[0].vent_remaining > 0 {
                    return Some(format!(
                        "VENT is open for {}s more.",
                        self.world.players[0].vent_remaining.div_ceil(30)
                    ));
                }
                let p = bw_content::VENT_PRESSURE.saturating_sub(self.world.players[0].pressure);
                (p > 0).then(|| format!("Need {p} more pressure."))
            }
            Action::Sound => {
                if self
                    .gameplay_ids(|k| matches!(k, Kind::Sounder | Kind::Skipper))
                    .is_empty()
                {
                    return Some("Select a Sounder or Skipper to SOUND.".into());
                }
                let p = bw_content::SOUND_PRESSURE.saturating_sub(self.world.players[0].pressure);
                (p > 0).then(|| format!("Need {p} more pressure."))
            }
            Action::Glint => {
                if self.gameplay_ids(|k| k == Kind::Glinter).is_empty() {
                    return Some("Select a Glinter to GLINT.".into());
                }
                let p = bw_content::GLINT_PRESSURE.saturating_sub(self.world.players[0].pressure);
                (p > 0).then(|| format!("Need {p} more pressure."))
            }
            Action::Lay => {
                if self.gameplay_ids(|k| k == Kind::Salter).is_empty() {
                    return Some("Select a Salter to LAY.".into());
                }
                let p =
                    bw_content::LAY_PRESSURE_PER_ROW.saturating_sub(self.world.players[0].pressure);
                (p > 0).then(|| format!("Need {p} more pressure."))
            }
            Action::Deploy | Action::Pack | Action::KeepDeployed => {
                let ids = self.gameplay_ids(crate::controls::is_specialist);
                if ids.is_empty() {
                    Some(crate::trial11_words::deploy_refusal(self.faction))
                } else {
                    None
                }
            }
            Action::SetTide(_) | Action::Flood => self.tide_reason(matches!(action, Action::Flood)),
            Action::Switch => {
                if self.world.gate.owner != Some(0) {
                    Some("Capture the sluice with a combat machine first.".into())
                } else if let Some(until) = self.world.gate.warning_until {
                    Some(format!(
                        "Lanes are switching in {}s.",
                        until.saturating_sub(self.world.tick).div_ceil(30)
                    ))
                } else if self.world.tick < self.world.gate.locked_until {
                    Some(format!(
                        "Sluice can switch again in {}s.",
                        (self.world.gate.locked_until - self.world.tick).div_ceil(30)
                    ))
                } else if self.world.station_contested_for(0) {
                    Some("An enemy stands at the station: clear it first. Nothing is spent.".into())
                } else {
                    None
                }
            }
            Action::Cancel => {
                if self.first_building().is_some_and(|id| {
                    self.world
                        .entities
                        .iter()
                        .any(|e| e.id == id && (e.build_remaining > 0 || !e.queue.is_empty()))
                }) {
                    None
                } else {
                    Some("There is no construction or production to cancel.".into())
                }
            }
            Action::Attack | Action::Stop | Action::Hold
                if self.gameplay_ids(|k| !k.is_building()).is_empty() =>
            {
                Some("Select one or more machines first.".into())
            }
            _ => None,
        }
    }

    fn resource_reason(&self, kind: Kind) -> Option<String> {
        let player = &self.world.players[0];
        let cost = spec(kind);
        let salvage = cost.salvage.saturating_sub(player.salvage);
        let pressure = cost.pressure.saturating_sub(player.pressure);
        match (salvage, pressure) {
            (0, 0) => None,
            (s, 0) => Some(format!("Need {s} more salvage.")),
            (0, p) => Some(format!("Need {p} more pressure.")),
            (s, p) => Some(format!("Need {s} salvage and {p} pressure.")),
        }
    }

    pub fn action_description(&self, action: &Action) -> String {
        match action {
            Action::FilterSelection(kind, remove) => {
                if *remove {
                    format!("Remove {} from the selection.", kind.name())
                } else {
                    format!(
                        "Keep only {}. Shift-click or the minus button removes it.",
                        kind.name()
                    )
                }
            }
            Action::RosterPage(_) => "More selected types.".into(),
            Action::RecallGroup(n) | Action::CenterGroup(n) => self.group_description(*n),
            Action::ToggleHealthBars => {
                "O: health bars on everything visible, or only selected and damaged.".into()
            }
            Action::IdleWorker | Action::AllIdleWorkers => {
                "I: next idle worker. Shift+I: all idle workers.".into()
            }
            Action::SelectArmy => "F2: all combat machines. Shift+F2 also centers on them.".into(),
            Action::ToggleFocusPause => "Pause the field when the window loses focus.".into(),
            Action::SelectWorks => "F4: all completed Works. Shift+Q/W/E trains up to five.".into(),
            Action::FocusAlert(_) => "F3: visit this alert. Selection stays.".into(),
            Action::CompactFieldCards => {
                if self.ux.preferences.compact_field_cards {
                    "Unfold the sluice card.".into()
                } else {
                    "Fold the sluice card to its status and hold checklist, to see more of the field.".into()
                }
            }
            Action::TideCard => {
                if self.ux.preferences.tide_card {
                    "Unpin the sluice card; it opens again on hover.".into()
                } else {
                    "Pin the sluice card open: its checklist and the tide keys.".into()
                }
            }
            Action::FocusHold if crate::trial11_words::capture_status(&self.world).is_some() => {
                crate::trial11_words::capture_hint(&self.world)
            }
            Action::FocusHold => {
                "Look at the lane that decides the count. F3 goes there first while a foe counts."
                    .into()
            }
            Action::FocusLane(_) => "Look at what blocks this lane.".into(),
            Action::CycleSpeed => format!(
                "Game speed, now {}: 0.5x, 1x or 1.5x for both sides.",
                self.shown_pace().label().to_lowercase()
            ),
            Action::ZoomIn | Action::ZoomOut => format!(
                "Zoom {}, now {}. The wheel zooms too.",
                if *action == Action::ZoomIn { "in" } else { "out" },
                self.zoom.label().to_lowercase()
            ),
            Action::Train(k) | Action::TrainBatch(k) => {
                let key = self
                    .production_hotkey_for(*k)
                    .map(|key| format!("{key}. Shift trains up to five."))
                    .unwrap_or_else(|| "Shift trains up to five.".into());
                kind_tooltip(*k, &key)
            }
            Action::Build(k) => {
                let key = build_hotkey(*k);
                kind_tooltip(*k, &format!("{key}, then click a site."))
            }
            Action::Upgrade(upgrade) => {
                let (salvage, pressure, ticks) = upgrade.cost();
                format!(
                    "{} salvage, {} pressure, {}s\n{}\nAt the {}. Paid now; waits behind a running upgrade, up to {} queued.",
                    salvage,
                    pressure,
                    ticks / 30,
                    upgrade_description(*upgrade, self.faction),
                    building_name(upgrade.building(), self.faction),
                    bw_content::UPGRADE_QUEUE
                )
            }
            Action::CancelUpgrade => {
                "K: cancel the last queued upgrade for its whole price, or the running one for 75%.".into()
            }
            Action::Unload => {
                "U: set the hold on free ground within three cells. BOARD (E) sends the selected machines to the nearest own transport with room.".into()
            }
            Action::Board => {
                "E: the selected machines walk to the nearest own transport with room and climb aboard. U unloads them.".into()
            }
            Action::Recycle => format!(
                "+{} salvage a worker\nThe workers walk to the nearest headquarters, Works or Salvage Yard and are broken up: {}% of their salvage back and their crew place free.\nX with workers selected.",
                spec(self.faction.worker()).salvage * bw_content::RECYCLE_REFUND_PERCENT / 100,
                bw_content::RECYCLE_REFUND_PERCENT
            ),
            Action::Reclaim => format!(
                "{} pressure\nMelts {} pressure into {} salvage at once, as often as you like.\nR at the headquarters.",
                bw_content::RECLAIM_PRESSURE,
                bw_content::RECLAIM_PRESSURE,
                bw_content::RECLAIM_SALVAGE
            ),
            Action::Vent => format!(
                "{} pressure\nEvery combat machine fires 30% faster for 10s.\nX at the headquarters.",
                bw_content::VENT_PRESSURE
            ),
            Action::Sound => format!(
                "{} pressure\nLights {} cells around the machine for 5s, fog or not.\nX with a Sounder or Skipper.",
                bw_content::SOUND_PRESSURE,
                bw_content::SOUND_RADIUS / bw_core::FP
            ),
            Action::Glint => crate::trial11_words::glint_description(),
            Action::Lay => crate::trial11_words::lay_description(),
            Action::Gather => "Choose a wreck. The worker hauls salvage home.".into(),
            Action::Attack => "Choose a destination. Machines attack enemies on the way.".into(),
            Action::Hold => "Hold position and fire at enemies in range.".into(),
            Action::Stop => "Stop current orders.".into(),
            Action::Capture => {
                "Claim the sluice with combat machines: 40s of work, each machine beside it counts, four at most; the first claim of the neutral station takes 60s. An enemy beside it halts the work. The owner gains 30 salvage a minute.".into()
            }
            Action::Switch => format!(
                "{} pressure\nT: dry {} after a 10-second warning. The side that goes deep is a wall no walking machine crosses. Refused while an enemy stands at the station; one arriving cancels it and the pressure comes back. Shift+T floods.",
                bw_content::SWITCH_PRESSURE,
                if crate::seats::arm_count(&self.world) > 2 {
                    "the next arm; the other two go deep"
                } else {
                    "the other side"
                }
            ),
            Action::SetTide(arm) => {
                let dry = crate::seats::arm_word(&self.world, arm.index());
                let others = crate::tide_cues::deep_words(&self.world, arm.index());
                let deep = if crate::seats::arm_count(&self.world) > 2 {
                    format!("{} go deep", capitalise(&others))
                } else {
                    format!("{} side goes deep", capitalise(&others))
                };
                // Rules 22: on three arms a DRY ebbs back to shallow.
                let ebb = if self.world.dry_ebbs() {
                    format!(
                        "\nEbbs back to shallow after {}s.",
                        bw_content::DRY_EBB_TICKS / 30
                    )
                } else {
                    String::new()
                };
                format!(
                    "{} pressure, 10s warning\nOpens the {dry} lane and rim to walking machines{}.\n{deep}: a wall. Nothing walks across; only hulls, wings and Dredgers cross.{ebb}\nAn enemy at the station refuses it, or cancels it with a refund.",
                    bw_content::SWITCH_PRESSURE,
                    self.wrecks_opened_words(Some(*arm))
                )
            }
            Action::Flood => format!(
                "{} pressure, 5s warning\nEvery crossing deep for {}s: a wall. Nothing walks across; only hulls, wings and Dredgers cross.\nEvery hold count freezes until it falls.\nCuts off an attack and covers every lane wreck{}. When it falls, back to the open side, each lane wreck regains {} salvage (to {}).",
                bw_content::FLOOD_PRESSURE,
                bw_content::FLOOD_TICKS / 30,
                self.wrecks_opened_words(None),
                bw_content::FLOOD_FLOTSAM,
                bw_content::LANE_WRECK
            ),
            Action::Deploy => crate::trial11_words::deploy_description(self),
            Action::Pack => {
                "P: pack deployed machines so they can move; D deploys. K keeps them deployed when their group moves on.".into()
            }
            Action::KeepDeployed => {
                "K: keep deployed. A move or attack-move for the group leaves them in place until P packs them.".into()
            }
            Action::SelectWorkers => "F7: every worker. Press again to centre them.".into(),
            Action::SelectBuildings(_) => {
                "Ctrl and a build key: every building of that kind. Ctrl+H: headquarters.".into()
            }
            Action::Formation => {
                "F: Tight, Line, or Loose spacing for the next move. Loose dodges blasts."
                    .into()
            }
            Action::Face => "R, then click a direction. Stops and clears queued moves.".into(),
            Action::Surge => format!(
                "{} +50% speed for 3s, weapons off. 12s cooldown; no deployment while active. A capture order waits for the surge to end.",
                self.surge_words()
            ),
            Action::Research(doctrine) => {
                let tier2 = self.world.players[0].doctrine == Some(*doctrine)
                    && self.world.players[0].doctrine_tier == 1;
                if tier2 {
                    format!(
                        "{} salvage, {} pressure, {}s\n{}\nThe second tier of your doctrine. Pauses the headquarters while it runs.",
                        bw_content::DOCTRINE_TIER2_SALVAGE,
                        bw_content::DOCTRINE_TIER2_PRESSURE,
                        bw_content::DOCTRINE_TIER2_TICKS / 30,
                        crate::tactics::doctrine_tier2_benefit(*doctrine)
                    )
                } else {
                    // The second tier is shown before the choice (rules 15):
                    // the eighth trial's seats chose blind.
                    format!(
                        "{} salvage, {} pressure, 30s\n{}\nTier II later ({} salvage, {} pressure): {}\nOne doctrine per match. Pauses the headquarters while it runs.",
                        bw_content::DOCTRINE_SALVAGE,
                        bw_content::DOCTRINE_PRESSURE,
                        crate::tactics::doctrine_benefit(*doctrine),
                        bw_content::DOCTRINE_TIER2_SALVAGE,
                        bw_content::DOCTRINE_TIER2_PRESSURE,
                        crate::tactics::doctrine_tier2_benefit(*doctrine)
                    )
                }
            }
            Action::CancelResearch => "Cancel research for a 75% refund.".into(),
            Action::Cancel => {
                "Cancel construction or the first queued unit. Started work refunds 75%, waiting work 100%."
                    .into()
            }
            Action::Load => self.ux.save_description.clone(),
            Action::Save => "Overwrite the quick save with this field.".into(),
            Action::ToggleEdgeScroll => "Pan by moving the pointer to the screen edge.".into(),
            _ => String::new(),
        }
    }

    /// The lane wrecks a tide opens to workers, in words: "and 2 wrecks
    /// (640 salvage) to workers".  `side` is the side a switch dries; None
    /// is a flood, which opens none and drowns the ones workers can reach.
    pub(crate) fn wrecks_opened_words(&self, side: Option<bw_sim::Arm>) -> String {
        let tidal = |r: &&bw_sim::Resource| {
            let (x, y) = r.pos.cell_xy();
            // A fight's scrap is only known where it was seen.
            r.remaining > 0
                && (!r.scrap || self.world.visible(0, r.pos))
                && self
                    .world
                    .map
                    .terrain(x, y)
                    .tidal_arm()
                    .map(|arm| arm == 0)
                    .is_some()
        };
        let pick: Vec<&bw_sim::Resource> = match side {
            Some(arm) => self
                .world
                .map
                .resources
                .iter()
                .filter(tidal)
                .filter(|r| {
                    let (x, y) = r.pos.cell_xy();
                    self.world.map.terrain(x, y).tidal_arm() == Some(arm.0)
                        && !self.world.gatherable(Kind::Headquarters, r)
                })
                .collect(),
            None => self
                .world
                .map
                .resources
                .iter()
                .filter(tidal)
                .filter(|r| self.world.gatherable(Kind::Headquarters, r))
                .collect(),
        };
        let salvage: u32 = pick.iter().map(|r| r.remaining).sum();
        let count = pick.len();
        let wrecks = if count == 1 { "wreck" } else { "wrecks" };
        match (side, count) {
            (_, 0) => String::new(),
            (Some(_), _) => format!(", and {count} {wrecks} ({salvage} salvage) to workers"),
            (None, _) => format!(", {count} of them ({salvage} salvage) open to workers now"),
        }
    }

    /// Why one cell refuses a building.  `require_explored` is off for the
    /// cells of a condenser footprint other than its well: the footprint is
    /// anchored to a marked well the player already knows.
    pub(crate) fn placement_cell_refusal(
        &self,
        cell: Pos,
        require_explored: bool,
    ) -> Option<Refusal> {
        let (x, y) = cell.cell_xy();
        if !cell.valid(self.world.map.width, self.world.map.height) {
            return Some(Refusal::OffMap);
        }
        if require_explored && !self.world.visible(0, cell) {
            return Some(Refusal::Unexplored);
        }
        if !matches!(self.world.map.terrain(x, y), Terrain::Salt | Terrain::Silt) {
            return Some(Refusal::Tidal);
        }
        if cell.distance_sq(self.world.map.gate_pos) <= i64::from(FP * 4).pow(2) {
            return Some(Refusal::SluiceApproach);
        }
        let occupied = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.hp > 0
                    && e.aboard.is_none()
                    && (e.owner == 0 || self.world.entity_visible(0, e.id))
                    && !matches!(spec(e.kind).movement, bw_core::Movement::Air)
                    // Own machines on their feet step off a new site.
                    && (e.owner != 0
                        || e.kind.is_building()
                        || e.deployed
                        || e.deploy_remaining > 0)
            })
            .any(|e| {
                let (ex, ey) = e.pos.cell_xy();
                let size = spec(e.kind).footprint.max(1);
                (ex..ex + size).contains(&x) && (ey..ey + size).contains(&y)
            });
        if occupied {
            return Some(Refusal::Occupied);
        }
        None
    }

    pub fn placement_reason(&self, kind: Kind, pos: Pos) -> Option<String> {
        self.placement_refusal(kind, pos)
            .map(|refusal| crate::trial11_words::refusal_sentence(self, kind, pos, refusal))
    }

    /// Why a footprint refuses a building: the first cell's refusal, then
    /// the condenser's well.
    pub(crate) fn placement_refusal(&self, kind: Kind, pos: Pos) -> Option<Refusal> {
        let (x, y) = pos.cell_xy();
        let n = spec(kind).footprint;
        // A condenser's first need is its well (trial 11).
        if kind == Kind::Condenser && !self.world.map.wells.iter().any(|p| p.cell_xy() == (x, y)) {
            return Some(Refusal::NoWell);
        }
        for dy in 0..n {
            for dx in 0..n {
                let cell = Pos::cell(x + dx, y + dy);
                // A condenser stands on a charted well, public geography:
                // it needs no sight (ninth trial: a labelled well refused).
                let require_explored = kind != Kind::Condenser;
                if let Some(refusal) = self.placement_cell_refusal(cell, require_explored) {
                    return Some(refusal);
                }
            }
        }
        if kind == Kind::Condenser && !self.world.map.wells.iter().any(|p| p.cell_xy() == (x, y)) {
            return Some(Refusal::NoWell);
        }
        None
    }

    pub fn resource_under_pointer(&self, x: i32, y: i32) -> Option<u32> {
        let pos = self.unproject(x, y);
        let (camera, (x, y)) = self.native_pointer(x, y);
        self.world
            .map
            .resources
            .iter()
            // Wreck positions are chart knowledge: a wreck under the fog
            // takes a gather order, and the field answers if it is spent.
            .filter(|r| r.remaining > 0 || !self.world.visible(0, r.pos))
            .filter(|r| {
                let (rx, ry) = camera.project(r.pos);
                let base = crate::game::salvage_asset(r.id);
                let key = crate::presentation::salvage_stage_asset(
                    base,
                    self.resource_stages.get(&r.id).copied().unwrap_or(0),
                );
                r.pos.distance_sq(pos) < i64::from(FP * 2).pow(2)
                    || self.atlas.as_ref().is_some_and(|a| {
                        a.hit(
                            if a.sprites.contains_key(&key) {
                                &key
                            } else {
                                base
                            },
                            x - rx,
                            y - ry,
                        )
                    })
            })
            .max_by_key(|r| (i64::from(r.pos.x) + i64::from(r.pos.y), r.id))
            .map(|r| r.id)
    }

    /// A visible enemy on the pointed ground: a machine within a cell of
    /// it, or a building whose footprint holds it.
    pub(crate) fn enemy_on_ground(&self, p: Pos) -> Option<EntityId> {
        let (px, py) = p.cell_xy();
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.owner != 0 && e.hp > 0 && e.aboard.is_none() && self.world.entity_visible(0, e.id)
            })
            .filter(|e| {
                if e.kind.is_building() {
                    let (ex, ey) = e.pos.cell_xy();
                    let n = spec(e.kind).footprint.max(1);
                    (ex..ex + n).contains(&px) && (ey..ey + n).contains(&py)
                } else {
                    e.pos.distance_sq(p) <= i64::from(FP).pow(2)
                }
            })
            .min_by_key(|e| (e.pos.distance_sq(p), e.id))
            .map(|e| e.id)
    }

    /// An own building under the pointer that workers could mend: a site
    /// still to build or a damaged building, with where to mark the order.
    pub(crate) fn own_building_to_mend(&self, x: i32, y: i32, p: Pos) -> Option<(EntityId, Pos)> {
        let needs_work = |e: &bw_sim::Entity| {
            e.owner == 0 && e.hp > 0 && (e.build_remaining > 0 || e.hp < e.max_hp)
        };
        if let Some(id) = self.building_at(x, y)
            && let Some(e) = self.world.entities.iter().find(|e| e.id == id)
            && needs_work(e)
        {
            return Some((e.id, e.pos));
        }
        let (px, py) = p.cell_xy();
        self.world
            .entities
            .iter()
            .filter(|e| e.kind.is_building() && needs_work(e))
            .find(|e| {
                let (ex, ey) = e.pos.cell_xy();
                let n = spec(e.kind).footprint.max(1);
                (ex..ex + n).contains(&px) && (ey..ey + n).contains(&py)
            })
            .map(|e| (e.id, e.pos))
    }

    /// DELIVER (rules 22): the selected workers carrying a load, on an own
    /// finished headquarters or Salvage Yard under the pointer.
    pub(crate) fn deliver_order(&self, x: i32, y: i32) -> Option<(Command, &'static str, Pos)> {
        let target = self.building_at(x, y)?;
        let yard = self
            .world
            .entities
            .iter()
            .find(|e| e.id == target && bw_sim::delivers_at(e, 0))?;
        let loaded: Vec<EntityId> = self
            .gameplay_ids(|k| k.gathers())
            .into_iter()
            .filter(|id| {
                self.world
                    .entities
                    .iter()
                    .any(|e| e.id == *id && e.carried > 0)
            })
            .collect();
        if loaded.is_empty() {
            return None;
        }
        Some((
            Command::Deliver {
                units: loaded,
                target,
            },
            "Deliver",
            yard.pos,
        ))
    }

    pub fn context_order(
        &self,
        x: i32,
        y: i32,
        queued: bool,
    ) -> Option<(Command, &'static str, Pos)> {
        if self.selected.is_empty() || !self.world_pointer_allowed(x, y) {
            return None;
        }
        let p = self.unproject(x, y);
        let mobiles = self.gameplay_ids(|k| !k.is_building());
        let workers = self.gameplay_ids(|k| k.is_worker());
        let gatherers = self.gameplay_ids(|k| k.gathers());
        if mobiles.is_empty() {
            let building = self.first_building()?;
            let producer = self
                .world
                .entities
                .iter()
                .any(|e| e.id == building && e.kind.produces());
            return producer.then_some((Command::Rally { building, pos: p }, "Rally", p));
        }
        // An own machine under the pointer takes no order of its own, so a
        // building beneath it (its own builder standing on a site, say) is
        // the intended target.
        let picked = self.unit_at(x, y).and_then(|id| {
            let own_machine = self
                .world
                .entities
                .iter()
                .any(|e| e.id == id && e.owner == 0 && !e.kind.is_building());
            if own_machine {
                self.building_at(x, y).or(Some(id))
            } else {
                Some(id)
            }
        });
        // Nothing drawn under the pointer: an enemy standing on the pointed
        // ground is still the target (trial 10: a right-click on a distant
        // Loom sent a plain move).
        let picked = picked.or_else(|| self.enemy_on_ground(p));
        let enemy_picked = picked.is_some_and(|id| {
            self.world
                .entities
                .iter()
                .any(|e| e.id == id && e.owner != 0)
        });
        // Loaded workers hand their loads in at an own headquarters or
        // Salvage Yard and go back to their wreck (rules 22): in trial 12
        // a right-click on the Kiln moved them and they kept their loads.
        if !enemy_picked && let Some(order) = self.deliver_order(x, y) {
            return Some(order);
        }
        // Workers mend the own building under the pointer, found by its
        // footprint on the ground as well as its picture: in trial 10 a
        // crowd of builders on a NO BUILDER site took the click, and the
        // order was a move.
        if !workers.is_empty()
            && !enemy_picked
            && let Some(site) = self.own_building_to_mend(x, y, p)
        {
            let resume = self
                .world
                .entities
                .iter()
                .any(|e| e.id == site.0 && e.build_remaining > 0);
            return Some((
                Command::Repair {
                    units: workers,
                    target: site.0,
                },
                if resume { "Resume" } else { "Repair" },
                site.1,
            ));
        }
        // Gatherers alone gather a wreck an enemy stands on (trial 11).
        if enemy_picked && let Some(order) = self.gather_under_enemy(x, y, queued) {
            return Some(order);
        }
        if let Some(id) = picked
            && let Some(e) = self.world.entities.iter().find(|e| e.id == id)
        {
            if e.owner != 0 {
                return Some((
                    Command::Attack {
                        units: mobiles,
                        target: id,
                    },
                    "Attack",
                    e.pos,
                ));
            }
            if e.kind.is_transport() {
                // Riders walk to an own transport and climb aboard.
                let riders: Vec<EntityId> = mobiles
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.world
                            .entities
                            .iter()
                            .any(|r| r.id == *id && !r.kind.is_transport() && r.aboard.is_none())
                    })
                    .collect();
                if !riders.is_empty() {
                    return Some((
                        Command::Board {
                            units: riders,
                            transport: id,
                        },
                        "Board",
                        e.pos,
                    ));
                }
            }
            if e.hp < e.max_hp && !workers.is_empty() {
                return Some((
                    Command::Repair {
                        units: workers,
                        target: id,
                    },
                    if e.build_remaining > 0 {
                        "Resume"
                    } else {
                        "Repair"
                    },
                    e.pos,
                ));
            }
        }
        if !gatherers.is_empty()
            && let Some(resource) = self.resource_under_pointer(x, y)
        {
            let pos = self
                .world
                .map
                .resources
                .iter()
                .find(|r| r.id == resource)?
                .pos;
            // Shift keeps a builder on its site: the gather comes after it.
            let command = if queued {
                Command::QueueGather {
                    units: gatherers,
                    resource,
                }
            } else {
                Command::Gather {
                    units: gatherers,
                    resource,
                }
            };
            return Some((command, "Gather", pos));
        }
        let gate = self.world.map.gate_pos;
        let (camera, (nx, ny)) = self.native_pointer(x, y);
        let (gx, gy) = camera.project(gate);
        let key = crate::station3::station_key(&self.world);
        let gate_hit = p.distance_sq(gate) < i64::from(FP * 3).pow(2)
            || self
                .atlas
                .as_ref()
                .is_some_and(|a| a.hit(&key, nx - gx, ny - gy));
        let combat = self.gameplay_ids(|k| !k.is_worker() && !k.is_building());
        if gate_hit && !combat.is_empty() {
            return Some((Command::Capture { units: combat }, "Capture", gate));
        }
        Some((
            Command::Move {
                units: mobiles,
                target: p,
                queued,
            },
            "Move",
            p,
        ))
    }

    /// The Q, W or E that trains a kind at its producer.
    pub fn production_hotkey_for(&self, kind: Kind) -> Option<&'static str> {
        let faction = self.world.players[0].faction;
        if kind == faction.worker() {
            return Some("Q");
        }
        // The scout is W at the headquarters and at the Drydock.
        if kind == faction.scout() {
            return Some("W");
        }
        let keys = ["Q", "W", "E", "R"];
        if let Some(i) = faction.army().iter().position(|k| *k == kind) {
            return Some(keys[i]);
        }
        crate::trial11_controls::drydock_keys(faction)
            .iter()
            .position(|k| *k == kind)
            .map(|i| keys[i])
    }

    pub fn mark_order(&mut self, pos: Pos, caption: &'static str) {
        self.ux.markers.push(OrderMarker {
            pos,
            tick: self.world.tick,
            caption,
        });
        if self.ux.markers.len() > 8 {
            self.ux.markers.remove(0);
        }
    }

    pub fn cancel_mode(&mut self) {
        let words = crate::trial11_words::cancel_words(self);
        self.mode = Mode::Context;
        self.drag = None;
        self.message.clear();
        self.notify(words);
    }

    pub fn world_pointer_allowed(&self, x: i32, y: i32) -> bool {
        if self.drawing_scene {
            return self.scene_pointer_allowed;
        }
        if self.native_ui() {
            return self.native_world_pointer_allowed(x, y);
        }
        if self.ux.practice_review {
            return false;
        }
        if !(0..640).contains(&x) || !(24..264).contains(&y) {
            return false;
        }
        if self.buttons.iter().any(|button| button.contains(x, y))
            || ((228..434).contains(&x)
                && (28..48).contains(&y)
                && !self
                    .gameplay_ids(|k| !k.is_worker() && !k.is_building())
                    .is_empty())
        {
            return false;
        }
        if (8..220).contains(&x) && (28..52).contains(&y) {
            return false;
        }
        if (438..536).contains(&x) && (28..47).contains(&y) {
            return false;
        }
        if (554..633).contains(&x) && (49..67).contains(&y) {
            return false;
        }
        if self.ux.tactics_fixture && (8..258).contains(&x) && (60..90).contains(&y) {
            return false;
        }
        if !self.ux.tactics_fixture
            && self.ux.practice
            && self.ux.tutorial.is_none()
            && (8..240).contains(&x)
            && (60..134).contains(&y)
        {
            return false;
        }
        if !self.ux.tactics_fixture && self.ux.practice && self.ux.tutorial.is_some() {
            if self.ux.guidance_visible && (8..240).contains(&x) && (60..192).contains(&y) {
                return false;
            }
            if !self.ux.guidance_visible && (8..174).contains(&x) && (60..81).contains(&y) {
                return false;
            }
        }
        true
    }

    pub fn back(&mut self) {
        let next = match self.screen {
            Screen::Help => self.paused_from_help,
            Screen::Settings => self.ux.settings_return,
            Screen::Confirm => {
                self.ux.pending = None;
                self.ux.confirm_return
            }
            Screen::Setup => Screen::Menu,
            Screen::Lobby => {
                // Leaving the online screen leaves its lobby.
                if let Some(lobby) = self.lobby.take() {
                    lobby.leave();
                }
                Screen::Menu
            }
            Screen::Pause => Screen::Match,
            Screen::Match => {
                if self.mode != Mode::Context {
                    self.cancel_mode();
                    return;
                }
                if self.world.outcome.is_some() {
                    Screen::Menu
                } else if self.ux.practice_review {
                    return;
                } else {
                    Screen::Pause
                }
            }
            Screen::Menu => return,
        };
        self.open_screen(next);
    }

    pub fn menu_keyboard(&mut self, key: &str, shift: bool) -> bool {
        let live = self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review;
        if live && key != "Tab" && !self.ux.keyboard_navigation {
            return false;
        }
        if key == "Escape" {
            self.back();
            return true;
        }
        if !matches!(
            key,
            "Tab" | "ArrowDown" | "ArrowUp" | "ArrowLeft" | "ArrowRight" | "Enter" | "Space"
        ) {
            return false;
        }
        // In the Guide, left and right on its tabs turn the page.
        if self.screen == Screen::Help
            && matches!(key, "ArrowLeft" | "ArrowRight")
            && self
                .ux
                .focused
                .as_ref()
                .is_none_or(|a| matches!(a, Action::GuidePage(_)))
        {
            let page = self.ux.help_page;
            let page = if key == "ArrowRight" {
                (page + 1).min(crate::menus::GUIDE_PAGES - 1)
            } else {
                page.saturating_sub(1)
            };
            self.action(Action::GuidePage(page));
            self.ux.keyboard_navigation = true;
            self.ux.focused = Some(Action::GuidePage(page));
            return true;
        }
        // In Settings, left and right turn the tabs from the tab row and
        // change the value of the focused row.
        if self.screen == Screen::Settings && matches!(key, "ArrowLeft" | "ArrowRight") {
            let right = key == "ArrowRight";
            let focused = self.ux.focused.clone();
            match focused {
                None | Some(Action::SettingsTab(_)) => {
                    let tabs = crate::menus::SETTINGS_TABS.len() as u8;
                    let tab = self.ux.settings_tab;
                    let tab = if right {
                        (tab + 1).min(tabs - 1)
                    } else {
                        tab.saturating_sub(1)
                    };
                    self.action(Action::SettingsTab(tab));
                    self.ux.keyboard_navigation = true;
                    self.ux.focused = Some(Action::SettingsTab(tab));
                    return true;
                }
                Some(Action::Volume(bus)) => {
                    let step = if shift { 10 } else { 1 };
                    self.action(Action::VolumeStep(bus, if right { step } else { -step }));
                    self.ux.keyboard_navigation = true;
                    return true;
                }
                Some(Action::CycleSpeed) if self.session.is_none() => {
                    // Three paces: back one is forward two.
                    for _ in 0..if right { 1 } else { 2 } {
                        self.action(Action::CycleSpeed);
                    }
                    self.ux.keyboard_navigation = true;
                    return true;
                }
                Some(Action::CycleInterfaceScale) => {
                    for _ in 0..if right { 1 } else { 4 } {
                        self.action(Action::CycleInterfaceScale);
                    }
                    self.ux.keyboard_navigation = true;
                    return true;
                }
                Some(
                    action @ (Action::Mute
                    | Action::ToggleEdgeScroll
                    | Action::ToggleFocusPause
                    | Action::ToggleHealthBars
                    | Action::ToggleFullscreen),
                ) => {
                    self.action(action);
                    self.ux.keyboard_navigation = true;
                    return true;
                }
                _ => {}
            }
        }
        self.render();
        let enabled: Vec<Action> = self
            .buttons
            .iter()
            .filter(|b| b.enabled)
            .map(|b| b.action.clone())
            .collect();
        if enabled.is_empty() {
            return true;
        }
        if live && key == "Tab" && !self.ux.keyboard_navigation {
            self.ux.keyboard_navigation = true;
            self.ux.focused = Some(enabled[if shift { enabled.len() - 1 } else { 0 }].clone());
            return true;
        }
        self.ux.keyboard_navigation = true;
        let current = self
            .ux
            .focused
            .as_ref()
            .and_then(|a| enabled.iter().position(|b| b == a))
            .unwrap_or(0);
        if matches!(key, "Enter" | "Space") {
            // Shift+Enter on a roster icon drops that kind, as a shift-click
            // does: the icon grid has no separate minus buttons.
            self.action(match enabled[current].clone() {
                Action::FilterSelection(kind, false) if shift => {
                    Action::FilterSelection(kind, true)
                }
                action => action,
            });
            if live {
                self.ux.keyboard_navigation = false;
                self.ux.focused = None;
            }
        } else if key.starts_with("Arrow") {
            if let Some(from) = self.buttons.iter().find(|b| b.action == enabled[current]) {
                let horizontal = matches!(key, "ArrowLeft" | "ArrowRight");
                let positive = matches!(key, "ArrowRight" | "ArrowDown");
                let (x, y) = (from.x + from.w / 2, from.y + from.h / 2);
                let candidates: Vec<_> = self
                    .buttons
                    .iter()
                    .filter(|b| b.enabled && b.action != from.action)
                    .filter_map(|b| {
                        let dx = b.x + b.w / 2 - x;
                        let dy = b.y + b.h / 2 - y;
                        let primary = if horizontal { dx } else { dy };
                        if (positive && primary <= 0) || (!positive && primary >= 0) {
                            return None;
                        }
                        let aligned = if horizontal {
                            b.y < from.y + from.h && from.y < b.y + b.h
                        } else {
                            b.x < from.x + from.w && from.x < b.x + b.w
                        };
                        Some((b, aligned, primary.abs(), dx * dx + dy * dy))
                    })
                    .collect();
                let aligned = candidates.iter().any(|c| c.1);
                if let Some(next) = candidates
                    .iter()
                    .filter(|c| !aligned || c.1)
                    .min_by_key(|c| if aligned { c.2 } else { c.3 })
                {
                    self.ux.focused = Some(next.0.action.clone());
                }
            }
        } else {
            let back = key == "ArrowUp" || key == "ArrowLeft" || (key == "Tab" && shift);
            let next = if back {
                (current + enabled.len() - 1) % enabled.len()
            } else {
                (current + 1) % enabled.len()
            };
            self.ux.focused = Some(enabled[next].clone());
        }
        true
    }

    pub fn ensure_menu_focus(&mut self) {
        if self.screen == Screen::Match && self.world.outcome.is_none() && !self.ux.practice_review
        {
            if !self.ux.keyboard_navigation
                || self
                    .ux
                    .focused
                    .as_ref()
                    .is_some_and(|a| !self.buttons.iter().any(|b| b.enabled && &b.action == a))
            {
                self.ux.focused = None;
            }
            return;
        }
        if self.ux.focus_screen != self.screen {
            self.ux.focused = None;
            self.ux.focus_screen = self.screen;
        }
        if self
            .ux
            .focused
            .as_ref()
            .is_some_and(|a| self.buttons.iter().any(|b| b.enabled && &b.action == a))
        {
            return;
        }
        // Focus starts on the screen's one primary action; a screen without
        // one starts on its first control, never on BACK.
        let preferred = match self.screen {
            Screen::Confirm => Some(Action::DismissConfirm),
            Screen::Help => Some(Action::GuidePage(self.ux.help_page)),
            _ => self.primary_action(),
        };
        self.ux.focused = self
            .buttons
            .iter()
            .find(|b| b.enabled && Some(&b.action) == preferred.as_ref())
            .or_else(|| {
                self.buttons
                    .iter()
                    .find(|b| b.enabled && b.action != Action::Back)
            })
            .or_else(|| self.buttons.iter().find(|b| b.enabled))
            .map(|b| b.action.clone());
    }

    /// The one action a menu screen leads with: drawn filled in gold, and
    /// where keyboard focus starts.  None on screens of equal choices.
    pub(crate) fn primary_action(&self) -> Option<Action> {
        use crate::net_ui::NetAction;
        let enabled = |action: &Action| {
            self.buttons
                .iter()
                .any(|b| b.enabled && &b.action == action)
        };
        match self.screen {
            Screen::Menu if self.has_session() => Some(Action::Resume),
            Screen::Menu if !self.ux.preferences.tutorial_completed => Some(Action::StartPractice),
            Screen::Menu => Some(Action::Setup),
            Screen::Setup => Some(Action::Start),
            Screen::Pause => Some(Action::Resume),
            Screen::Match if self.playback.is_some() => Some(Action::ReplayRestart),
            Screen::Match if self.ux.practice_review => {
                let (done, total, _) = &self.ux.practice_progress;
                if self.ux.practice_complete || (*total > 0 && done >= total) {
                    Some(Action::Setup)
                } else {
                    Some(Action::StartPractice)
                }
            }
            Screen::Match => Some(Action::Start),
            Screen::Lobby => [
                NetAction::Start,
                NetAction::Ready,
                NetAction::Join,
                NetAction::AddReply,
                NetAction::CopyCode,
                NetAction::CopyReply,
                NetAction::Host,
            ]
            .into_iter()
            .map(Action::Net)
            .find(|a| enabled(a)),
            _ => None,
        }
    }

    /// How much weight a menu control carries: the primary action, a
    /// routine one, or one that loses progress.
    pub(crate) fn button_weight(&self, b: &crate::game::Button) -> ButtonWeight {
        use crate::net_ui::NetAction;
        let danger = matches!(
            b.action,
            Action::Surrender | Action::Quit | Action::Net(NetAction::EndMatch)
        ) || (self.screen == Screen::Confirm && b.action == Action::Confirm);
        if danger {
            ButtonWeight::Danger
        } else if self.primary_action().as_ref() == Some(&b.action) {
            ButtonWeight::Primary
        } else {
            ButtonWeight::Secondary
        }
    }

    pub fn draw_guidance(&mut self) {
        if self.ux.tactics_fixture {
            self.canvas.rect(8, 60, 250, 30, INK);
            self.canvas.text("CONTROLLED TACTICS FIELD", 15, 65, GOLD);
            self.canvas
                .text("PREPARED ARMIES / AI ORDERS OFF", 15, 78, MUTED);
            return;
        }
        if !self.ux.practice {
            return;
        }
        let Some(view) = self.ux.tutorial.as_ref().map(|t| t.view(&self.world)) else {
            self.canvas.rect(8, 60, 232, 74, INK);
            self.canvas.frame(8, 60, 232, 74, EDGE);
            self.canvas
                .text("PRACTICE / NO ENEMY ATTACKS", 15, 66, JADE);
            self.buttons.push(crate::game::Button {
                x: 16,
                y: 106,
                w: 104,
                h: 20,
                label: "Guide".into(),
                hint: String::new(),
                action: Action::Help,
                enabled: true,
            });
            self.buttons.push(crate::game::Button {
                x: 126,
                y: 106,
                w: 106,
                h: 20,
                label: "New lesson".into(),
                hint: String::new(),
                action: Action::StartPractice,
                enabled: true,
            });
            return;
        };
        if !self.ux.guidance_visible {
            self.buttons.push(crate::game::Button {
                x: 8,
                y: 60,
                w: 166,
                h: 21,
                label: "Show guide".into(),
                hint: String::new(),
                action: Action::EndGuidance,
                enabled: true,
            });
            return;
        }
        self.canvas.rect(8, 60, 232, 132, INK);
        self.canvas.frame(8, 60, 232, 132, EDGE);
        self.canvas.text(
            &format!("STEP {} OF {}", view.step, view.total),
            16,
            68,
            JADE,
        );
        self.canvas.text_readable(&view.title, 16, 82, WHITE);
        crate::menus::draw_wrapped(&mut self.canvas, &view.body, 16, 98, 216, 12, MUTED);
        if view.complete {
            self.buttons.push(crate::game::Button {
                x: 16,
                y: 166,
                w: 128,
                h: 20,
                label: "Try skirmish".into(),
                hint: String::new(),
                action: Action::Setup,
                enabled: true,
            });
            self.buttons.push(crate::game::Button {
                x: 150,
                y: 166,
                w: 82,
                h: 20,
                label: "Keep playing".into(),
                hint: String::new(),
                action: Action::EndGuidance,
                enabled: true,
            });
        } else {
            let (label, action) = match view.target {
                TutorialTarget::Worker => ("Find worker", Action::FocusWorker),
                TutorialTarget::Works => ("Find Works", Action::FocusWorks),
                TutorialTarget::Army => ("Find army", Action::FocusArmy),
                TutorialTarget::Gate => ("Show sluice", Action::FocusGate),
                TutorialTarget::None => ("Guide", Action::Help),
            };
            self.buttons.push(crate::game::Button {
                x: 16,
                y: 166,
                w: 132,
                h: 20,
                label: label.into(),
                hint: String::new(),
                action,
                enabled: true,
            });
            self.buttons.push(crate::game::Button {
                x: 158,
                y: 166,
                w: 74,
                h: 20,
                label: "Hide".into(),
                hint: String::new(),
                action: Action::EndGuidance,
                enabled: true,
            });
        }
    }

    pub fn draw_order_markers(&mut self) {
        self.ux
            .markers
            .retain(|m| self.world.tick.saturating_sub(m.tick) < 24);
        for marker in &self.ux.markers {
            let (x, y) = self.project(marker.pos);
            if !(24..self.canvas.height() as i32).contains(&y) {
                continue;
            }
            let r = 5 + (self.world.tick.saturating_sub(marker.tick) / 5) as i32;
            // The widening diamond is the answer; its colour says the verb
            // (red for a fight, jade for the rest) without a caption.
            let color = if marker.caption.starts_with("Attack") {
                crate::canvas::RED
            } else {
                JADE
            };
            self.canvas.line(x - r, y, x, y - r / 2, color);
            self.canvas.line(x, y - r / 2, x + r, y, color);
            self.canvas.line(x + r, y, x, y + r / 2, color);
            self.canvas.line(x, y + r / 2, x - r, y, color);
        }
    }

    pub fn draw_ux_feedback(&mut self) {
        let live = self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review;
        if live {
            let prompt = match self.mode {
                Mode::Build(kind) => {
                    let pos = self.unproject(self.cursor.0, self.cursor.1);
                    let (x, y) = pos.cell_xy();
                    let reason = if (24..264).contains(&self.cursor.1) {
                        self.placement_reason(kind, Pos::cell(x, y))
                    } else {
                        None
                    };
                    (
                        format!("Place {}", kind.name()),
                        reason.unwrap_or_else(|| {
                            "Left click to build / Esc or right click cancels".into()
                        }),
                    )
                }
                Mode::Attack => (
                    "Attack-move".into(),
                    "Left click a destination / Esc or right click cancels".into(),
                ),
                Mode::Gather => (
                    "Gather salvage".into(),
                    "Left click a visible wreck / Esc or right click cancels".into(),
                ),
                Mode::Glint => (
                    "GLINT".into(),
                    "Left click a direction / Esc or right click cancels".into(),
                ),
                Mode::Lay => (
                    "LAY".into(),
                    "Left click lane or rim water / Esc or right click cancels".into(),
                ),
                Mode::Face => (
                    "Face direction".into(),
                    "Left click a direction / pack deployed specialists first / Esc cancels".into(),
                ),
                Mode::Context => {
                    if self.frame < self.message_until && !self.message.is_empty() {
                        (String::new(), self.message.clone())
                    } else if let Some((_, verb, _)) =
                        self.context_order(self.cursor.0, self.cursor.1, false)
                    {
                        (
                            format!("Right click: {verb}"),
                            "Left click selects / drag selects a group".into(),
                        )
                    } else {
                        (
                            "Give an order".into(),
                            "Select a machine / right click a target or ground".into(),
                        )
                    }
                }
            };
            self.canvas.rect(6, 264, 628, 23, INK);
            if prompt.0.is_empty() {
                if text_readable_width(&prompt.1) <= 610 {
                    self.canvas.text_readable(&prompt.1, 12, 271, GOLD);
                } else {
                    self.canvas.text(
                        &prompt.1.chars().take(101).collect::<String>(),
                        12,
                        272,
                        GOLD,
                    );
                }
            } else {
                self.canvas.text_readable(&prompt.0, 12, 267, WHITE);
                self.canvas.text(&prompt.1, 12, 279, MUTED);
            }
            if let Some(button) = self.buttons.iter().rev().find(|b| {
                if self.ux.keyboard_navigation {
                    self.ux.focused.as_ref() == Some(&b.action)
                } else {
                    b.contains(self.cursor.0, self.cursor.1)
                }
            }) {
                let reason = self.action_reason(&button.action);
                let description = reason
                    .clone()
                    .unwrap_or_else(|| self.action_description(&button.action));
                if !description.is_empty() {
                    let title = if reason.is_some() {
                        format!("{} / unavailable", button.label)
                    } else {
                        button.label.clone()
                    };
                    let x = 382;
                    let y = 165;
                    let w = 252;
                    self.canvas.rect(x, y, w, 91, INK);
                    self.canvas
                        .frame(x, y, w, 91, if reason.is_some() { RED } else { EDGE });
                    crate::menus::draw_wrapped(
                        &mut self.canvas,
                        &title,
                        x + 8,
                        y + 8,
                        w - 16,
                        12,
                        if reason.is_some() { GOLD } else { WHITE },
                    );
                    crate::menus::draw_wrapped(
                        &mut self.canvas,
                        &description,
                        x + 8,
                        y + 34,
                        w - 16,
                        11,
                        MUTED,
                    );
                }
            }
            if self.mode != Mode::Context && (24..264).contains(&self.cursor.1) {
                let (x, y) = self.cursor;
                self.canvas.line(x - 4, y, x + 4, y, GOLD);
                self.canvas.line(x, y - 4, x, y + 4, GOLD);
            }
        } else if self.frame < self.message_until && !self.message.is_empty() {
            // Left of the Guide's BACK, wrapped rather than cut (trial 11).
            if text_readable_width(&self.message) <= 424 {
                self.canvas.rect(24, 338, 436, 18, INK);
                self.canvas.text_readable(&self.message, 30, 343, GOLD);
            } else {
                let rows = crate::trial11_words::menu_message_rows(&self.message);
                let top = 356 - 10 * rows.len() as i32 - 4;
                self.canvas.rect(24, top, 436, 356 - top, INK);
                for (i, row) in rows.iter().enumerate() {
                    self.canvas.text(row, 30, top + 3 + 10 * i as i32, GOLD);
                }
            }
        }
    }
}

/// Why a site refuses a building, as a sentence for the prompt and as the
/// two or three words the ghost's plate shows in the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    OffMap,
    Unexplored,
    Tidal,
    SluiceApproach,
    Occupied,
    NoWell,
}
impl Refusal {
    pub fn sentence(self) -> &'static str {
        match self {
            Refusal::OffMap => "Keep the whole building inside the map.",
            Refusal::Unexplored => {
                "Build only on ground your side can see now: send a machine to look."
            }
            Refusal::Tidal => "Build on permanent dry land, away from the crossings.",
            Refusal::SluiceApproach => "Leave the sluice approach clear.",
            Refusal::Occupied => "This space is occupied. Choose clear ground; Esc cancels.",
            Refusal::NoWell => "Place the condenser on a marked pressure well.",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Refusal::OffMap => "OFF THE MAP",
            Refusal::Unexplored => "NOT IN SIGHT",
            Refusal::Tidal => "TIDAL GROUND",
            Refusal::SluiceApproach => "SLUICE APPROACH",
            Refusal::Occupied => "OCCUPIED",
            Refusal::NoWell => "NEEDS A WELL",
        }
    }
}

pub fn friendly_error(message: &str) -> String {
    match message {
        "insufficient resources" => {
            "Not enough resources. Hover the command to see its cost.".into()
        }
        "building placement is blocked" => {
            "That site is blocked. Choose clear, permanent dry ground.".into()
        }
        "crew capacity exceeded" => {
            "Crew capacity is full. Wait for room before training more.".into()
        }
        "resource missing or exhausted" => {
            "That wreck is exhausted. Choose another salvage site.".into()
        }
        "unit has no deploy action" => "Select a Bulwark or Loom to deploy.".into(),
        "target is not visible" => "Scout the target before ordering an attack.".into(),
        "an enemy stands at the station" => {
            "An enemy stands at the station: clear it first. Nothing is spent.".into()
        }
        // Engine wording never reaches the player (ninth trial: "ENTITY 131
        // MISSING" after the target died between the click and the tick).
        other if other.starts_with("entity ") && other.ends_with(" missing") => {
            "That machine is gone: it was destroyed before the order landed.".into()
        }
        other => {
            let mut words = other.to_string();
            if let Some(first) = words.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            if !words.ends_with('.') {
                words.push('.');
            }
            words
        }
    }
}

#[cfg(test)]
mod dock_ux_tests {
    use super::*;

    #[test]
    fn preferences_default_and_deserialize_volume_levels_are_safe() {
        let defaults: Preferences = serde_json::from_str(
            r#"{"muted":false,"edge_scroll":true,"fullscreen":false,"tutorial_completed":false}"#,
        )
        .expect("legacy preferences remain readable");
        assert_eq!(defaults.effects_volume, 75);
        assert_eq!(defaults.ambience_volume, 65);
        assert_eq!(defaults.music_volume, 60);

        let clamped: Preferences = serde_json::from_str(
            r#"{"effects_volume":255,"ambience_volume":101,"music_volume":100}"#,
        )
        .expect("out of range levels remain readable");
        assert_eq!(clamped.effects_volume, 100);
        assert_eq!(clamped.ambience_volume, 100);
        assert_eq!(clamped.music_volume, 100);
    }
}

/// The three kinds of menu control: one filled primary per screen, the
/// quiet routine bar, and red for anything that loses progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonWeight {
    Primary,
    Secondary,
    Danger,
}
