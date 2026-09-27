//! Versioned game definitions. Values are first-playable balance seeds.
use bw_core::{FP, Faction, Kind, Movement};
use serde::{Deserialize, Serialize};
pub const RULES_VERSION: u32 = 22;

/// The source `bw_core`, `bw_content` and `bw_sim` were built from, hashed at
/// build time by `build.rs`.
///
/// [`rules_digest`] covers the content constants, which only change when
/// someone changes them: a behaviour change in `bw_sim` leaves it identical.
/// The lockstep handshake sends this alongside the digest so that two builds
/// of different code are refused before the match instead of desynchronising
/// once it is under way.  It is deliberately not part of the digest, because
/// saves and replays are keyed by the digest and would all become unreadable
/// on the next edit to any source file.
pub fn build_fingerprint() -> &'static str {
    env!("BW_BUILD_FINGERPRINT")
}

/// Mutually exclusive headquarters research choices.  A doctrine is only
/// applied after its bounded research timer reaches zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Doctrine {
    Hauling,
    FireControl,
}

pub const DOCTRINE_SALVAGE: u32 = 120;
pub const DOCTRINE_PRESSURE: u32 = 60;
pub const DOCTRINE_TICKS: u32 = 30 * 30;
/// The second tier of a doctrine, open once the first has completed.
pub const DOCTRINE_TIER2_SALVAGE: u32 = 200;
pub const DOCTRINE_TIER2_PRESSURE: u32 = 120;
pub const DOCTRINE_TIER2_TICKS: u32 = 45 * 30;

/// Building upgrades: one at a time per building, completed for the side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Upgrade {
    /// Works: line hull +25%: Bulwark, Riveter and Reedguard (the Riveter
    /// since rules 17).
    Plate,
    /// Works: Union Riveters do 200% to buildings; Assembly Looms reach
    /// 25% farther and deploy in half the time.
    Siege,
    /// Drydock: flooded lanes slow your machines half as much.
    Tracks,
    /// Yard: workers carry two more.
    Cranes,
    /// Yard: every wreck shows its salvage on the chart, seen or not.
    SalvageSonar,
    /// Yard: a combat machine you destroy leaves a wreck worth its full
    /// cost instead of half.
    ScrapRecovery,
    /// Condenser: pressure cap +100.
    Overpressure,
    /// Condenser: pressure income +60 a minute.
    BleedValves,
    /// Works: every combat machine deals 10% more damage (rules 15, a
    /// late salvage sink that needs no crew).
    Temper,
    /// Drydock: every combat machine carries 15% more hull (rules 15, a
    /// late salvage sink that needs no crew).
    Refit,
    /// Headquarters, three levels in a chain (rules 21): each adds
    /// `OVERHAUL_HULL_PERCENT` of base hull to every machine the side owns,
    /// those already built included.  Salvage only: in trial 11 both sides
    /// banked 2,000-7,000 salvage at the crew ceiling with nothing to buy.
    Overhaul1,
    Overhaul2,
    Overhaul3,
}
impl Upgrade {
    pub const ALL: [Upgrade; 13] = [
        Upgrade::Plate,
        Upgrade::Siege,
        Upgrade::Tracks,
        Upgrade::Cranes,
        Upgrade::SalvageSonar,
        Upgrade::ScrapRecovery,
        Upgrade::Overpressure,
        Upgrade::BleedValves,
        Upgrade::Temper,
        Upgrade::Refit,
        Upgrade::Overhaul1,
        Upgrade::Overhaul2,
        Upgrade::Overhaul3,
    ];
    /// OVERHAUL's levels, in the order they are researched.
    pub const OVERHAUL: [Upgrade; 3] = [Upgrade::Overhaul1, Upgrade::Overhaul2, Upgrade::Overhaul3];
    /// The OVERHAUL level this upgrade completes, 1 to 3, or None.
    pub fn overhaul_level(self) -> Option<u8> {
        match self {
            Upgrade::Overhaul1 => Some(1),
            Upgrade::Overhaul2 => Some(2),
            Upgrade::Overhaul3 => Some(3),
            _ => None,
        }
    }
    /// The upgrade that must be complete, running or queued at the same
    /// building before this one may start: the previous OVERHAUL level.
    pub fn requires(self) -> Option<Upgrade> {
        match self {
            Upgrade::Overhaul2 => Some(Upgrade::Overhaul1),
            Upgrade::Overhaul3 => Some(Upgrade::Overhaul2),
            _ => None,
        }
    }
    /// The building kind that researches this upgrade.
    pub fn building(self) -> Kind {
        match self {
            Upgrade::Plate | Upgrade::Siege | Upgrade::Temper => Kind::Works,
            Upgrade::Tracks | Upgrade::Refit => Kind::Drydock,
            Upgrade::Cranes | Upgrade::SalvageSonar | Upgrade::ScrapRecovery => Kind::Dropoff,
            Upgrade::Overpressure | Upgrade::BleedValves => Kind::Condenser,
            Upgrade::Overhaul1 | Upgrade::Overhaul2 | Upgrade::Overhaul3 => Kind::Headquarters,
        }
    }
    /// Salvage, pressure and ticks.
    pub fn cost(self) -> (u32, u32, u32) {
        match self {
            Upgrade::Plate | Upgrade::Siege => (120, 40, 40 * 30),
            Upgrade::Tracks => (150, 50, 40 * 30),
            Upgrade::Cranes => (100, 20, 30 * 30),
            Upgrade::SalvageSonar => (80, 40, 30 * 30),
            Upgrade::ScrapRecovery => (120, 20, 30 * 30),
            Upgrade::Overpressure => (60, 40, 30 * 30),
            Upgrade::BleedValves => (100, 40, 30 * 30),
            Upgrade::Temper | Upgrade::Refit => (300, 40, 60 * 30),
            Upgrade::Overhaul1 => (1_000, 0, OVERHAUL_TICKS),
            Upgrade::Overhaul2 => (1_500, 0, OVERHAUL_TICKS),
            Upgrade::Overhaul3 => (2_000, 0, OVERHAUL_TICKS),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Plate => "PLATE",
            Upgrade::Siege => "SIEGE",
            Upgrade::Tracks => "TRACKS",
            Upgrade::Cranes => "CRANES",
            Upgrade::SalvageSonar => "SONAR",
            Upgrade::ScrapRecovery => "SCRAP",
            Upgrade::Overpressure => "OVERPRESSURE",
            Upgrade::BleedValves => "BLEED VALVES",
            Upgrade::Temper => "TEMPER",
            Upgrade::Refit => "REFIT",
            Upgrade::Overhaul1 => "OVERHAUL I",
            Upgrade::Overhaul2 => "OVERHAUL II",
            Upgrade::Overhaul3 => "OVERHAUL III",
        }
    }
    /// The upgrades a building of this kind offers, in button order.
    pub fn offered_by(kind: Kind) -> &'static [Upgrade] {
        match kind {
            Kind::Works => &[Upgrade::Plate, Upgrade::Siege, Upgrade::Temper],
            Kind::Drydock => &[Upgrade::Tracks, Upgrade::Refit],
            Kind::Dropoff => &[
                Upgrade::Cranes,
                Upgrade::SalvageSonar,
                Upgrade::ScrapRecovery,
            ],
            Kind::Condenser => &[Upgrade::Overpressure, Upgrade::BleedValves],
            Kind::Headquarters => &Upgrade::OVERHAUL,
            _ => &[],
        }
    }
}

/// A building holds its running upgrade and this many more waiting behind
/// it, paid when queued (rules 15).
pub const UPGRADE_QUEUE: usize = 2;
/// TEMPER and REFIT, in percent of base damage and hull.
pub const TEMPER_DAMAGE_PERCENT: i32 = 110;
pub const REFIT_HULL_PERCENT: i32 = 115;
/// OVERHAUL (rules 21): each level adds this share of a machine's base
/// hull, rounded, and takes this long.
pub const OVERHAUL_HULL_PERCENT: i32 = 6;
pub const OVERHAUL_TICKS: u32 = 45 * 30;

/// Crew and pressure caps grow with the base.
pub const CREW_CAP_BASE: u32 = 40;
pub const CREW_CAP_WORKS: u32 = 12;
pub const CREW_CAP_YARD: u32 = 6;
pub const CREW_CAP_DRYDOCK: u32 = 10;
pub const CREW_CAP_MAX: u32 = 90;
pub const PRESSURE_CAP_BASE: u32 = 150;
pub const PRESSURE_CAP_CONDENSER: u32 = 100;
pub const PRESSURE_CAP_OVERPRESSURE: u32 = 100;
pub const PRESSURE_CAP_MAX: u32 = 450;
/// Pressure income that arrives at a full bar is not lost: every ten
/// pressure over the cap becomes this much salvage (rules 15), half the
/// rate of RECLAIM, so spending pressure still beats letting it overflow.
pub const OVERFLOW_PRESSURE: u32 = 10;
pub const OVERFLOW_SALVAGE: u32 = 3;
/// Pressure a minute for holding the sluice, and the switch's price.
pub const SLUICE_PRESSURE_PER_MINUTE: u32 = 15;
pub const SWITCH_PRESSURE: u32 = 40;
/// FLOOD: every tidal cell deep for a while, at a price and after a warning.
pub const FLOOD_PRESSURE: u32 = 80;
pub const FLOOD_TICKS: u32 = 45 * 30;
pub const FLOOD_WARNING_TICKS: u32 = 5 * 30;
/// When a flood falls, each of the four lane wrecks regains this much
/// salvage, never above its starting 900 (rules 15): FLOOD refills the
/// wrecks at the crossing mouths for whoever holds the lane that dries.
pub const FLOOD_FLOTSAM: u32 = 200;
/// A ground machine caught on a cell that turned deep: slow and exposed.
pub const SWAMPED_SPEED_PERCENT: i32 = 40;
pub const SWAMPED_DAMAGE_PERCENT: i32 = 150;
/// Transports: a hold of four, boarding from within two cells, unloading
/// onto free ground within three.
pub const TRANSPORT_CAPACITY: usize = 4;
pub const BOARD_RADIUS_CELLS: i32 = 2;
pub const UNLOAD_RADIUS_CELLS: i32 = 3;
/// Capture work: forty seconds for one machine, each machine beside the
/// station up to four adds a unit of work per tick.
pub const CAPTURE_WORK: u32 = 40 * 30;
pub const CAPTURE_MAX_WORKERS: u32 = 4;
/// An enemy combat machine this close to the station halts a capture
/// (rules 17; it was two cells, the capture ring, and in the ninth trial
/// two Union machines stood four cells out and never contested).
pub const CAPTURE_CONTEST_RADIUS_CELLS: i32 = 4;
/// The first capture of the neutral station takes this share of the usual
/// work (rules 15): a minute alone, fifteen seconds with four, so the side
/// that arrives second at the opening race still has time to contest.
pub const NEUTRAL_CAPTURE_PERCENT: u32 = 150;
/// Machines standing in a flooded lane take this much more damage.
pub const WADING_DAMAGE_PERCENT: i32 = 125;
/// A headquarters keeps this share of its hull while a finished Works of
/// its owner stands.
pub const KEEL_PERCENT: i32 = 25;
/// Holding both crossing mouths this long, unopposed, wins the match.
/// This is the two-seat length; `World::hold_ticks` gives the length for
/// the match being played.
pub const TIDE_HOLD_TICKS: u32 = 90 * 30;
/// The hold while three seats stand (rules 20): trial 10 ran 39:31 against
/// a 15-25 minute target.
pub const TIDE_HOLD_THREE_SEAT_TICKS: u32 = 75 * 30;
/// The hold once a seat is out, for the rest of the match (rules 20). It
/// then needs only the lanes to seats still standing, so it is longer.
pub const TIDE_HOLD_AFTER_OUT_TICKS: u32 = 120 * 30;
/// Each standing headquarters yields this much salvage a minute (rules
/// 20), whatever the tide: in trial 10 Union earned nothing for fifteen
/// minutes once the sluice holder cut it off from every wreck.
pub const HQ_SALVAGE_PER_MINUTE: u32 = 40;
/// The sluice owner gains this much salvage a minute while it owns the
/// station (rules 20), so the centre pays before the tide is won.
pub const STATION_SALVAGE_PER_MINUTE: u32 = 30;
/// A Sounder's extra damage a shot against a deployed Loom (rules 20; it
/// was 2, and in trial 10 ten Sounders died to deployed Looms in 15 s).
pub const SOUNDER_LOOM_BONUS: i32 = 5;
/// The Glinter's extra damage a shot against a deployed Loom (rules 21):
/// the Sounder's bonus, so the Compact's roster line "Glinter beats Loom
/// lines" is true.  In trial 11 Looms killed 75 of the Compact's 107.
pub const GLINTER_LOOM_BONUS: i32 = SOUNDER_LOOM_BONUS;
/// A Loom shell does this share of its damage to the Loom hunters, the
/// Sounder and the Glinter (rules 22): in trial 12 they died on the
/// approach, 1 Loom killed by 58 hunters built.
pub const HUNTER_LOOM_SHELL_PERCENT: i32 = 75;
/// On a map of three arms a DRY ebbs back to the neutral tide this long
/// after it lands (rules 22): in trial 12 a DRY S left the Union an
/// island for seven and a half minutes.
pub const DRY_EBB_TICKS: u32 = 180 * 30;
pub const CROSSING_HOLD_RADIUS_CELLS: i32 = 4;
/// A hold gauge falls this much a tick while both mouths are not held
/// (rules 21; it was 1, and in trial 11 a five-second break by one
/// machine gave back only five seconds of count).
pub const TIDE_HOLD_DRAIN_PER_TICK: u32 = 3;
/// Abilities.
pub const VENT_PRESSURE: u32 = 100;
/// RECLAIM: the headquarters turns pressure into salvage at once.
pub const RECLAIM_PRESSURE: u32 = 100;
pub const RECLAIM_SALVAGE: u32 = 60;
/// RECYCLE (rules 19): a worker broken up at an own headquarters, Works or
/// Salvage Yard returns this share of its salvage cost and frees its crew.
pub const RECYCLE_REFUND_PERCENT: u32 = 50;
pub const VENT_TICKS: u32 = 10 * 30;
pub const VENT_COOLDOWN_PERCENT: u32 = 70;
pub const SOUND_PRESSURE: u32 = 20;
pub const SOUND_TICKS: u32 = 5 * 30;
pub const SOUND_RADIUS: i32 = 9 * FP;
/// A Loom that fires is lit for the side it fires at, this long and this
/// far around it (rules 13): a shell always shows where it came from.
pub const LOOM_FIRE_REVEAL_TICKS: u32 = 3 * 30;
pub const LOOM_FIRE_REVEAL_RADIUS: i32 = 2 * FP;
/// A worker hit under fire marks its ground as dangerous for its side this
/// long and this far around it (rules 13).  Workers gathering a wreck in a
/// marked area leave it for a safe one, and no worker picks a marked wreck
/// on its own.
pub const WORKER_DANGER_TICKS: u32 = 20 * 30;
pub const WORKER_DANGER_RADIUS_CELLS: i32 = 6;
/// Repair machines: hull a second and salvage per hull.
pub const REPAIR_AURA_RADIUS: i32 = 3 * FP;
pub const REPAIR_AURA_HULL_PER_SECOND: i32 = 5;
pub const REPAIR_AURA_HULL_PER_SALVAGE: i32 = 5;
/// Scouts see twice as far on Hold.
pub const SCOUT_HOLD_SIGHT_PERCENT: i32 = 200;
/// Lane wreck and deep wreck values.
pub const HOME_WRECK: u32 = 1_600;
pub const SIDE_WRECK: u32 = 2_400;
pub const LANE_WRECK: u32 = 900;
pub const DEEP_WRECK: u32 = 1_500;
/// A destroyed combat machine leaves a wreck worth this share of its
/// salvage cost; the killer's Scrap Recovery raises the share.
pub const SCRAP_BASE_PERCENT: u32 = 50;
pub const SCRAP_UPGRADED_PERCENT: u32 = 100;
/// A combat machine that falls within this many cells of a crossing mouth
/// leaves a wreck worth this share of the usual (rules 15): a failed push
/// at a mouth no longer pays the defender for the whole army.
pub const MOUTH_SCRAP_RADIUS_CELLS: i32 = 6;
pub const MOUTH_SCRAP_PERCENT: u32 = 50;
/// Siege adds this many cells to a Loom's reach (rules 15; it was +25%,
/// ten cells, two beyond the sight of Union's line machines).
pub const LOOM_SIEGE_RANGE_CELLS: i32 = 1;
pub const SURGE_PRESSURE: u32 = 10;
pub const SURGE_TICKS: u32 = 3 * 30;
pub const SURGE_COOLDOWN_TICKS: u32 = 12 * 30;
pub const LOOM_WINDUP_TICKS: u32 = 24;
pub const LOOM_MIN_RANGE: i32 = 2 * FP;
pub const LOOM_BLAST_RADIUS: i32 = 5 * FP / 4;
/// Siege roles hit buildings harder: a base falls to a committed push rather
/// than absorbing it.  Riveters unpick plating; Loom shells break it.
pub const RIVETER_BUILDING_DAMAGE_PERCENT: i32 = 150;
pub const LOOM_BUILDING_DAMAGE_PERCENT: i32 = 200;

// The Saltglass Compact (rules 18, docs/THIRD-FACTION-AND-CONFLUENCE.md).
/// The Heliostat's beam: each shot on the same target adds this much, from
/// the spec's damage up to the maximum; Siege doubles the step. A new
/// target, or this long without a shot, starts again.
pub const HELIOSTAT_BEAM_STEP: i32 = 1;
pub const HELIOSTAT_BEAM_MAX: i32 = 10;
pub const HELIOSTAT_BEAM_RESET_TICKS: u32 = 3 * 30;
pub const HELIOSTAT_BUILDING_DAMAGE_PERCENT: i32 = 150;
/// A deployed Pan boils this much pressure a minute.
pub const PAN_PRESSURE_PER_MINUTE: u32 = 40;
/// GLINT: a ray this long and wide toward a point, lit this long.
pub const GLINT_PRESSURE: u32 = 20;
pub const GLINT_TICKS: u32 = 5 * 30;
pub const GLINT_LENGTH: i32 = 18 * FP;
pub const GLINT_WIDTH: i32 = 3 * FP;
/// LAY: the Salter crusts one row of three cells a second, up to this many
/// rows, at this price a row. The crust is dry until the tide next changes.
pub const LAY_PRESSURE_PER_ROW: u32 = 3;
pub const LAY_ROW_TICKS: u32 = 30;
pub const LAY_MAX_ROWS: u32 = 24;
pub const LAY_WIDTH_CELLS: i32 = 3;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub health: i32,
    pub damage: i32,
    pub range: i32,
    pub speed: i32,
    pub cooldown: u32,
    pub salvage: u32,
    pub pressure: u32,
    pub build_ticks: u32,
    pub crew: u32,
    pub sight: i32,
    pub movement: Movement,
    pub footprint: i32,
}
pub fn spec(kind: Kind) -> Spec {
    let (
        health,
        damage,
        range,
        speed,
        cooldown,
        salvage,
        pressure,
        seconds,
        crew,
        sight,
        movement,
        footprint,
    ) = match kind {
        Kind::Hook | Kind::Wick => (
            65,
            3,
            1,
            3,
            30,
            50,
            0,
            12,
            1,
            7,
            if kind == Kind::Hook {
                Movement::Wheel
            } else {
                Movement::Walker
            },
            1,
        ),
        Kind::Riveter => (125, 12, 4, 3, 24, 70, 0, 15, 2, 8, Movement::Wheel, 1),
        Kind::Bulwark => (300, 10, 3, 2, 32, 140, 30, 24, 4, 8, Movement::Wheel, 1),
        Kind::Sounder => (85, 7, 5, 5, 27, 80, 10, 16, 2, 12, Movement::Walker, 1),
        Kind::Reedguard => (150, 10, 4, 3, 27, 80, 0, 16, 2, 8, Movement::Walker, 1),
        Kind::Skipper => (100, 9, 4, 5, 25, 90, 10, 17, 2, 10, Movement::Paddle, 1),
        // Reach 7 (rules 21; it was 8, and outranged every Compact machine).
        Kind::Loom => (160, 32, 7, 2, 72, 145, 40, 26, 4, 10, Movement::Walker, 1),
        Kind::Headquarters => (2400, 12, 6, 0, 35, 0, 0, 0, 0, 12, Movement::Static, 4),
        Kind::Works => (1000, 0, 0, 0, 0, 180, 0, 20, 0, 7, Movement::Static, 3),
        Kind::Dropoff => (600, 0, 0, 0, 0, 100, 0, 14, 0, 6, Movement::Static, 2),
        Kind::Condenser => (650, 0, 0, 0, 0, 140, 0, 20, 0, 6, Movement::Static, 2),
        Kind::Tower => (1000, 18, 8, 0, 30, 180, 30, 22, 0, 10, Movement::Static, 2),
        Kind::Tidewatch => (60, 0, 0, 6, 0, 40, 0, 10, 1, 16, Movement::Wheel, 1),
        Kind::Lampwright => (60, 0, 0, 6, 0, 40, 0, 10, 1, 16, Movement::Walker, 1),
        Kind::Caulker => (80, 0, 0, 3, 0, 90, 20, 16, 2, 8, Movement::Wheel, 1),
        Kind::Tender => (80, 0, 0, 3, 0, 90, 20, 16, 2, 8, Movement::Walker, 1),
        Kind::Caisson => (500, 0, 0, 2, 0, 200, 60, 24, 3, 6, Movement::Wheel, 1),
        Kind::Dredger => (180, 0, 0, 2, 0, 120, 30, 20, 2, 8, Movement::Paddle, 1),
        Kind::Drydock => (900, 0, 0, 0, 0, 250, 0, 30, 0, 7, Movement::Static, 3),
        Kind::Palisade => (400, 0, 0, 0, 0, 40, 0, 6, 0, 3, Movement::Static, 1),
        Kind::Barge => (400, 0, 0, 4, 0, 220, 60, 40, 3, 8, Movement::Hull, 1),
        Kind::Lifter => (260, 0, 0, 3, 0, 260, 80, 40, 3, 10, Movement::Air, 1),
        // The Saltglass Compact (docs/THIRD-FACTION-AND-CONFLUENCE.md).
        Kind::Raker => (65, 3, 1, 3, 30, 50, 0, 12, 1, 7, Movement::Walker, 1),
        Kind::Brander => (127, 13, 3, 4, 24, 75, 0, 15, 2, 8, Movement::Walker, 1),
        // The beam's damage is its first shot; each shot on the same target
        // adds HELIOSTAT_BEAM_STEP up to HELIOSTAT_BEAM_MAX.
        Kind::Heliostat => (200, 2, 6, 2, 12, 150, 40, 26, 4, 9, Movement::Walker, 1),
        Kind::Glinter => (90, 8, 5, 5, 26, 80, 10, 16, 2, 11, Movement::Walker, 1),
        Kind::Stilt => (60, 0, 0, 6, 0, 40, 0, 10, 1, 16, Movement::Walker, 1),
        Kind::Glazier => (80, 0, 0, 3, 0, 90, 20, 16, 2, 8, Movement::Walker, 1),
        Kind::Salter => (180, 0, 0, 2, 0, 130, 30, 20, 2, 8, Movement::Walker, 1),
        Kind::Pan => (120, 0, 0, 2, 0, 110, 20, 18, 2, 6, Movement::Walker, 1),
    };
    Spec {
        health,
        damage,
        range: range * 256,
        speed: speed * 256,
        cooldown,
        salvage,
        pressure,
        build_ticks: seconds * 30,
        crew,
        sight: sight * 256,
        movement,
        footprint,
    }
}
pub fn faction_allows(faction: Faction, kind: Kind) -> bool {
    kind.is_building()
        || kind == faction.worker()
        || faction.army().contains(&kind)
        || faction.drydock_roles().contains(&kind)
}
/// Which building trains a kind, if any.
/// The headquarters also trains its side's scout, from the first minute
/// (rules 15); the Drydock still does too.
pub fn producer_of(faction: Faction, kind: Kind) -> Option<Kind> {
    if kind == faction.worker() || kind == faction.scout() {
        Some(Kind::Headquarters)
    } else if faction.army().contains(&kind) {
        Some(Kind::Works)
    } else if faction.drydock_roles().contains(&kind) {
        Some(Kind::Drydock)
    } else {
        None
    }
}
/// Whether a building of kind `producer` trains `kind` for this faction.
pub fn trains(faction: Faction, producer: Kind, kind: Kind) -> bool {
    match producer {
        Kind::Headquarters => kind == faction.worker() || kind == faction.scout(),
        Kind::Works => faction.army().contains(&kind),
        Kind::Drydock => faction.drydock_roles().contains(&kind),
        _ => false,
    }
}
pub fn rules_digest() -> String {
    let kinds = [
        Kind::Hook,
        Kind::Riveter,
        Kind::Bulwark,
        Kind::Sounder,
        Kind::Wick,
        Kind::Skipper,
        Kind::Reedguard,
        Kind::Loom,
        Kind::Headquarters,
        Kind::Works,
        Kind::Dropoff,
        Kind::Condenser,
        Kind::Tower,
        Kind::Tidewatch,
        Kind::Caulker,
        Kind::Caisson,
        Kind::Lampwright,
        Kind::Tender,
        Kind::Dredger,
        Kind::Drydock,
        Kind::Palisade,
        Kind::Barge,
        Kind::Lifter,
        Kind::Raker,
        Kind::Brander,
        Kind::Heliostat,
        Kind::Glinter,
        Kind::Stilt,
        Kind::Glazier,
        Kind::Salter,
        Kind::Pan,
    ];
    let data: Vec<_> = kinds.into_iter().map(|k| (k, spec(k))).collect();
    let upgrades: Vec<_> = Upgrade::ALL.into_iter().map(|u| (u, u.cost())).collect();
    let groups = (
        (
            CREW_CAP_BASE,
            CREW_CAP_WORKS,
            CREW_CAP_YARD,
            CREW_CAP_DRYDOCK,
            CREW_CAP_MAX,
        ),
        (
            PRESSURE_CAP_BASE,
            PRESSURE_CAP_CONDENSER,
            PRESSURE_CAP_OVERPRESSURE,
            PRESSURE_CAP_MAX,
        ),
        (
            SLUICE_PRESSURE_PER_MINUTE,
            SWITCH_PRESSURE,
            FLOOD_PRESSURE,
            FLOOD_TICKS,
            FLOOD_WARNING_TICKS,
            SWAMPED_SPEED_PERCENT,
            SWAMPED_DAMAGE_PERCENT,
            TRANSPORT_CAPACITY,
            BOARD_RADIUS_CELLS,
            UNLOAD_RADIUS_CELLS,
            CAPTURE_WORK,
            CAPTURE_MAX_WORKERS,
        ),
        (
            WADING_DAMAGE_PERCENT,
            KEEL_PERCENT,
            TIDE_HOLD_TICKS,
            CROSSING_HOLD_RADIUS_CELLS,
            TIDE_HOLD_DRAIN_PER_TICK,
        ),
        (
            VENT_PRESSURE,
            VENT_TICKS,
            VENT_COOLDOWN_PERCENT,
            SOUND_PRESSURE,
            SOUND_TICKS,
            SOUND_RADIUS,
            RECLAIM_PRESSURE,
            RECLAIM_SALVAGE,
            RECYCLE_REFUND_PERCENT,
        ),
        (
            REPAIR_AURA_RADIUS,
            REPAIR_AURA_HULL_PER_SECOND,
            REPAIR_AURA_HULL_PER_SALVAGE,
            SCOUT_HOLD_SIGHT_PERCENT,
        ),
        (
            HOME_WRECK,
            SIDE_WRECK,
            LANE_WRECK,
            DEEP_WRECK,
            SCRAP_BASE_PERCENT,
            SCRAP_UPGRADED_PERCENT,
        ),
        (
            DOCTRINE_TIER2_SALVAGE,
            DOCTRINE_TIER2_PRESSURE,
            DOCTRINE_TIER2_TICKS,
        ),
        (
            LOOM_FIRE_REVEAL_TICKS,
            LOOM_FIRE_REVEAL_RADIUS,
            WORKER_DANGER_TICKS,
            WORKER_DANGER_RADIUS_CELLS,
        ),
        (
            UPGRADE_QUEUE,
            TEMPER_DAMAGE_PERCENT,
            REFIT_HULL_PERCENT,
            OVERFLOW_PRESSURE,
            OVERFLOW_SALVAGE,
            FLOOD_FLOTSAM,
            NEUTRAL_CAPTURE_PERCENT,
            MOUTH_SCRAP_RADIUS_CELLS,
            MOUTH_SCRAP_PERCENT,
            LOOM_SIEGE_RANGE_CELLS,
        ),
        CAPTURE_CONTEST_RADIUS_CELLS,
        (
            TIDE_HOLD_THREE_SEAT_TICKS,
            TIDE_HOLD_AFTER_OUT_TICKS,
            HQ_SALVAGE_PER_MINUTE,
            STATION_SALVAGE_PER_MINUTE,
            SOUNDER_LOOM_BONUS,
        ),
        (GLINTER_LOOM_BONUS, OVERHAUL_HULL_PERCENT, OVERHAUL_TICKS),
        (HUNTER_LOOM_SHELL_PERCENT, DRY_EBB_TICKS),
    );
    let doctrine = (
        DOCTRINE_SALVAGE,
        DOCTRINE_PRESSURE,
        DOCTRINE_TICKS,
        SURGE_PRESSURE,
        SURGE_TICKS,
        SURGE_COOLDOWN_TICKS,
        LOOM_WINDUP_TICKS,
        LOOM_MIN_RANGE,
        LOOM_BLAST_RADIUS,
        RIVETER_BUILDING_DAMAGE_PERCENT,
        LOOM_BUILDING_DAMAGE_PERCENT,
    );
    let compact = (
        (
            HELIOSTAT_BEAM_STEP,
            HELIOSTAT_BEAM_MAX,
            HELIOSTAT_BEAM_RESET_TICKS,
            HELIOSTAT_BUILDING_DAMAGE_PERCENT,
            PAN_PRESSURE_PER_MINUTE,
        ),
        (GLINT_PRESSURE, GLINT_TICKS, GLINT_LENGTH, GLINT_WIDTH),
        (
            LAY_PRESSURE_PER_ROW,
            LAY_ROW_TICKS,
            LAY_MAX_ROWS,
            LAY_WIDTH_CELLS,
        ),
    );
    let bytes = serde_json::to_vec(&(RULES_VERSION, data, upgrades, groups, doctrine, compact))
        .expect("finite static content serializes");
    blake3::hash(&bytes).to_hex().to_string()
}
