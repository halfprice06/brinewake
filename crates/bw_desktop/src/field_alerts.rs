//! Recent local field events. Positions are observations, never live enemy tracking.
use bw_core::{EntityId, Kind, Pos};
use bw_sim::{EventKind, World};
use std::collections::BTreeMap;

/// Twenty seconds of field time; long enough to act on, short enough that
/// an opening does not accumulate cards.  Red cards stay twice as long.
const LIFETIME: u64 = 20 * 30;
const URGENT_LIFETIME: u64 = 40 * 30;
/// A hit this close to an own building is an attack on that base.
const BASE_RADIUS: i32 = 10;
/// Base cards anchored this close together are one base.
const BASE_MERGE: i32 = 14;
/// A building this close to a headquarters is part of its base.
const BASE_SPREAD: i32 = 24;
/// Field cards of one kind merge inside this distance.
const FIELD_MERGE: i32 = 6;
/// Enemy-seen cards merge inside this distance.
const REPORT_MERGE: i32 = 16;
/// A site stands this long without a builder before it earns a card: a
/// worker walking over, or an order still on its way, is not a stall.
const STALL_GRACE: u64 = 8 * 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    /// Own machines hit away from any base.
    Attack,
    /// Own machines or buildings hit at a base: one card per base.
    BaseAttack,
    /// A forward building (a nest at a mouth, a wall) or machines beside
    /// it hit away from the base: named by the building, not as the base
    /// (trial 11: "BASE UNDER ATTACK" for a nest forty cells out).
    OutpostAttack(Kind),
    Built(Kind),
    Ready(Kind),
    Lost(Kind),
    /// Own machines destroyed, counted.
    MachinesLost,
    SluiceCaptured,
    /// The enemy took a sluice that was ours.
    SluiceLost,
    /// The enemy took a sluice we never held.
    SluiceTaken,
    SluiceThreatened,
    ExitBlocked(Kind),
    /// Enemy combat machines seen in a quiet region; count on the card.
    EnemySeen,
    /// Enemy combat machines seen in a region holding one of our buildings.
    EnemyAtBase,
    UpgradeDone(bw_content::Upgrade),
    SwitchCancelled,
    /// The lanes changed, by either side's switch.
    LanesSwitched,
    /// Every crossing went deep.
    TideFlood,
    /// The flood fell back.
    TideEbb,
    /// The enemy's hold count started to rise.
    EnemyHold,
    /// An enemy holder stepped onto a lane we hold; the lane index.
    CrossingContested(u8),
    /// A wreck gave its last load; whether the workers stand idle now.
    WreckEmpty(bool),
    /// An enemy Loom fired on our machines; anchored on the Loom, which the
    /// shot lights for us (rules 13).
    LoomFire,
    /// Own workers destroyed, counted apart from combat machines.
    WorkersLost,
    /// Own workers left a wreck under fire (rules 13).
    WorkersFled,
    /// An own construction site has stood without a builder for a while:
    /// Union's Drydock sat unbuilt for six minutes in trial 8.
    NoBuilder(Kind),
    /// The crew reached its cap: nothing more trains until the base grows
    /// (or, at the ceiling, until machines are lost).  Union hit it at
    /// 12:39 in trial 10 and nothing said so.
    CrewFull,
    /// Every worker stands idle and no salvage has come in for a while:
    /// Union sat on 21 idle workers at +0 a minute in trial 10.
    WorkersIdle,
    /// Pressure has stood at its cap: RECLAIM or VENT at the HQ spends it
    /// (trial 12: the Union sat at 250/250 for twenty minutes).
    PressureFull,
    /// The last live wreck near home ran dry: workers walk far or stand
    /// (trial 12: the Union's income fell at 08:25 and nothing said so).
    WrecksOut,
    /// Producers with an empty queue while a machine is affordable; the
    /// count on the card (trial 12: three idle Works at 16:58).
    ProducersIdle,
}
impl AlertKind {
    /// Red accent: something is being taken from the player.
    pub fn urgent(self) -> bool {
        matches!(
            self,
            Self::Attack
                | Self::BaseAttack
                | Self::OutpostAttack(_)
                | Self::Lost(_)
                | Self::MachinesLost
                | Self::SluiceLost
                | Self::SluiceTaken
                | Self::SluiceThreatened
                | Self::ExitBlocked(_)
                | Self::EnemySeen
                | Self::EnemyAtBase
                | Self::SwitchCancelled
                | Self::EnemyHold
                | Self::CrossingContested(_)
                | Self::LoomFire
                | Self::WorkersLost
                | Self::WorkersFled
                | Self::NoBuilder(_)
                | Self::WorkersIdle
        )
    }
    fn merge_radius(self) -> i32 {
        match self {
            // One base, one card: hits anywhere on it, and completions from
            // any of its producers.
            Self::BaseAttack
            | Self::Built(_)
            | Self::Ready(_)
            | Self::MachinesLost
            | Self::WorkersLost
            | Self::WorkersFled => BASE_MERGE,
            _ => FIELD_MERGE,
        }
    }
    fn counted(self) -> bool {
        matches!(
            self,
            Self::Built(_)
                | Self::Ready(_)
                | Self::Lost(_)
                | Self::MachinesLost
                | Self::WorkersLost
        )
    }
    fn merge_radius_cells(self) -> i32 {
        if matches!(self, Self::EnemySeen | Self::EnemyAtBase) {
            REPORT_MERGE
        } else {
            self.merge_radius()
        }
    }
}
/// Short building names keep a card inside the notice band.
fn short_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Headquarters => "HQ",
        Kind::Dropoff => "YARD",
        Kind::Tower => "NEST",
        other => other.name(),
    }
}
/// A place this far from the headquarters is named by the nearest mouth,
/// lane, the sluice or a base.
const HOME_RADIUS: i32 = 12;
/// No salvage for this long with every worker idle raises a card.
const IDLE_GRACE: u64 = 20 * 30;

#[derive(Clone, Debug)]
pub struct FieldAlert {
    pub id: u64,
    pub kind: AlertKind,
    pub pos: Pos,
    pub tick: u64,
    pub count: usize,
    /// Where it happened, for places away from home: the nearest mouth,
    /// lane, the sluice or a base ("AT N LANE, YOUR BANK", "AT E LANE, RED BANK").
    pub direction: Option<String>,
    /// The headquarters' hull as a percentage while the base is hit.
    pub hq_percent: Option<u32>,
    /// For a loss: the kind of what did it, when the simulation knows.
    pub cause: Option<Kind>,
    /// Who or what a tide card is about, when the map needs it named: the
    /// counting seat ("RED") or the contested arm's letter ("E").  None
    /// keeps the Split Basin's words.
    pub subject: Option<&'static str>,
    /// With three seats: the opponent that did it, in its colour.
    pub by: Option<crate::seats::Hue>,
}
impl FieldAlert {
    pub fn label(&self) -> String {
        let base = self.base_label();
        match &self.direction {
            // A shot comes from a place: "LOOM FIRE FROM E LANE, RED BANK".
            Some(place) if self.kind == AlertKind::LoomFire && named_place(place) => {
                format!("{base} FROM {}", &place[3..])
            }
            Some(direction) => format!("{base} {direction}"),
            None => base,
        }
    }
    /// The second part of a card: what killed the machines (and, with three
    /// seats, whose they were), who is attacking, or what the workers did
    /// when their wreck emptied.
    pub fn detail(&self) -> Option<String> {
        self.detail_in(true)
    }
    /// The detail, with or without the kind of what did it.
    fn detail_in(&self, kinds: bool) -> Option<String> {
        match self.kind {
            AlertKind::WreckEmpty(idle) => Some(if idle {
                "WORKERS IDLE".into()
            } else {
                "WORKERS MOVED ON".into()
            }),
            AlertKind::MachinesLost | AlertKind::Lost(_) | AlertKind::WorkersLost => {
                match (self.by, self.cause) {
                    // Lost with a transport: nobody's doing.
                    (_, Some(kind @ (Kind::Barge | Kind::Lifter))) => Some(cause_words(kind)),
                    (Some(by), Some(Kind::Headquarters)) if kinds => {
                        Some(format!("TO {} HQ", by.owner_word()))
                    }
                    (Some(by), Some(kind)) if kinds => {
                        Some(format!("TO {} {}S", by.word(), short_name(kind)))
                    }
                    (Some(by), _) => Some(format!("TO {}", by.word())),
                    (None, cause) => cause.map(cause_words),
                }
            }
            AlertKind::Attack
            | AlertKind::BaseAttack
            | AlertKind::OutpostAttack(_)
            | AlertKind::CrossingContested(_) => self.by.map(|by| format!("BY {}", by.word())),
            AlertKind::CrewFull => Some(if self.count >= bw_content::CREW_CAP_MAX as usize {
                "THE CEILING".into()
            } else {
                "BUILD FOR MORE".into()
            }),
            AlertKind::WorkersIdle => Some("NO INCOME".into()),
            _ => None,
        }
    }
    /// The words for a card of `chars` a row and `rows` rows: the whole
    /// caption when it fits, else without the kind of what did it, else
    /// without the detail.  Cards wrap rather than cut a word off.
    pub fn card_caption(&self, age_seconds: u64, chars: usize, rows: usize) -> String {
        let fits = |words: &str| {
            crate::native_ui::wrap_small(words, chars, usize::MAX).len() <= rows
                && words.split_whitespace().all(|w| w.chars().count() <= chars)
        };
        let full = self.caption(age_seconds);
        if fits(&full) {
            return full;
        }
        let shorter = self.caption_with(self.detail_in(false), age_seconds);
        if fits(&shorter) {
            return shorter;
        }
        let bare = self.caption_with(None, age_seconds);
        if fits(&bare) || self.direction.is_none() {
            return bare;
        }
        // Last, the place goes too: F3 goes there and says it in full.
        let placeless = FieldAlert {
            direction: None,
            ..self.clone()
        };
        placeless.caption_with(None, age_seconds)
    }
    /// The whole card in words: label, a count for cards whose label has
    /// none, the detail and how many seconds ago it happened.
    pub fn caption(&self, age_seconds: u64) -> String {
        self.caption_with(self.detail(), age_seconds)
    }
    fn caption_with(&self, detail: Option<String>, age_seconds: u64) -> String {
        let mut caption = self.label();
        if self.count > 1
            && matches!(
                self.kind,
                AlertKind::Built(_) | AlertKind::Ready(_) | AlertKind::Lost(_)
            )
        {
            caption.push_str(&format!(" X{}", self.count));
        }
        if let Some(detail) = detail {
            caption.push(' ');
            caption.push_str(&detail);
        }
        // Time since, spelled out: "WICK READY 6S" read as a countdown.
        caption.push_str(&format!(" {age_seconds}S AGO"));
        caption
    }
    /// A building by the side's own name when the card carries one (the
    /// Compact's Glassworks), else its short name.
    fn named(&self, kind: Kind) -> &'static str {
        match self.subject {
            Some(name) if kind.is_building() => name,
            _ => short_name(kind),
        }
    }
    fn base_label(&self) -> String {
        match self.kind {
            AlertKind::Attack => "UNDER ATTACK".into(),
            AlertKind::OutpostAttack(kind) => format!("{} UNDER ATTACK", self.named(kind)),
            AlertKind::BaseAttack => match self.hq_percent {
                Some(percent) if percent < 100 => format!("BASE UNDER ATTACK HQ {percent}%"),
                _ => "BASE UNDER ATTACK".into(),
            },
            AlertKind::Built(kind) => format!("{} BUILT", self.named(kind)),
            AlertKind::Ready(kind) => format!("{} READY", short_name(kind)),
            AlertKind::Lost(kind) => format!("{} DESTROYED", self.named(kind)),
            AlertKind::MachinesLost => {
                let base = if self.count > 1 {
                    format!("{} MACHINES LOST", self.count)
                } else {
                    "MACHINE LOST".into()
                };
                // A loss at home says so; a bare card gave F3 nothing.
                if self.direction.is_none() {
                    format!("{base} AT BASE")
                } else {
                    base
                }
            }
            AlertKind::WorkersLost => {
                let base = if self.count > 1 {
                    format!("{} WORKERS LOST", self.count)
                } else {
                    "WORKER LOST".into()
                };
                if self.direction.is_none() {
                    format!("{base} AT BASE")
                } else {
                    base
                }
            }
            AlertKind::WorkersFled => "WORKERS PULLED BACK".into(),
            AlertKind::LoomFire => match &self.direction {
                None => "LOOM FIRE NEAR BASE".into(),
                Some(place) if named_place(place) => "LOOM FIRE".into(),
                Some(_) => "LOOM FIRE FROM".into(),
            },
            AlertKind::LanesSwitched => "LANES SWITCHED".into(),
            AlertKind::TideFlood => "FLOOD TIDE".into(),
            AlertKind::TideEbb => "TIDE FALLS".into(),
            AlertKind::SluiceCaptured => "SLUICE CAPTURED".into(),
            AlertKind::SluiceLost => "SLUICE LOST".into(),
            AlertKind::SluiceTaken => "ENEMY TOOK SLUICE".into(),
            AlertKind::SluiceThreatened => "ENEMY AT THE SLUICE".into(),
            AlertKind::ExitBlocked(kind) => format!("{} EXIT BLOCKED", short_name(kind)),
            AlertKind::EnemySeen => {
                if self.count > 1 {
                    format!("ENEMY SEEN X{}", self.count)
                } else {
                    "ENEMY SEEN".into()
                }
            }
            AlertKind::EnemyAtBase => {
                if self.count > 1 {
                    format!("ENEMY AT BASE X{}", self.count)
                } else {
                    "ENEMY AT BASE".into()
                }
            }
            AlertKind::UpgradeDone(upgrade) => format!("{} DONE", upgrade.name()),
            AlertKind::SwitchCancelled => "SWITCH CANCELLED, REFUNDED".into(),
            AlertKind::EnemyHold => {
                format!("{} HOLDS BOTH LANES", self.subject.unwrap_or("ENEMY"))
            }
            AlertKind::CrossingContested(lane) => format!(
                "{} LANE CONTESTED",
                self.subject.unwrap_or(if lane == 0 { "N" } else { "S" })
            ),
            AlertKind::WreckEmpty(_) => "WRECK EMPTY".into(),
            AlertKind::NoBuilder(kind) => format!("NO BUILDER: {} SITE", self.named(kind)),
            AlertKind::CrewFull => format!("CREW FULL {}/{}", self.count, self.count),
            AlertKind::WorkersIdle => {
                if self.count > 1 {
                    format!("ALL {} WORKERS IDLE", self.count)
                } else {
                    "WORKER IDLE".into()
                }
            }
            AlertKind::PressureFull | AlertKind::WrecksOut | AlertKind::ProducersIdle => {
                crate::trial12_field::nudge_label(self)
            }
        }
    }
}
/// The side's own name for a building where it differs from the shared
/// one: the Compact's Kiln, Glassworks and Rake Shed (trial 11).
fn own_name(world: &World, kind: Kind) -> Option<&'static str> {
    let faction = world.players.first()?.faction;
    let name = crate::ux::building_name(kind, faction);
    (kind != Kind::Tower && name != kind.name()).then_some(name)
}
/// Whether a card's place is a named one on a map of three arms ("AT E LANE,
/// YOUR BANK", "IN W LANE"), not a compass letter.
fn named_place(place: &str) -> bool {
    place.starts_with("AT ") || place.starts_with("IN ")
}
/// When every own worker stands idle: how many there are and where the
/// first one stands.  None while any works, or without workers.
fn all_workers_idle(world: &World) -> Option<(usize, Pos)> {
    let mut count = 0;
    let mut first = None;
    for e in &world.entities {
        if e.owner != 0 || e.hp <= 0 || !e.kind.is_worker() {
            continue;
        }
        if e.aboard.is_some() || !matches!(e.order, bw_sim::Order::Idle) {
            return None;
        }
        count += 1;
        first.get_or_insert(e.pos);
    }
    first.map(|pos| (count, pos))
}
/// Whether a live worker is on its way to, working on, or has queued this site.
pub(crate) fn site_has_builder(world: &World, site: &bw_sim::Entity) -> bool {
    // A site queued behind the worker's current one still has its builder.
    world.site_builder(site.id) != bw_sim::SiteBuilder::None
}
/// "TO LOOMS", "TO THE HQ", "WITH THE LIFTER": what a loss was lost to.
fn cause_words(kind: Kind) -> String {
    match kind {
        Kind::Headquarters => "TO THE HQ".into(),
        Kind::Barge | Kind::Lifter => format!("WITH THE {}", kind.name()),
        other => format!("TO {}S", short_name(other)),
    }
}
/// The own building nearest a hit, when one stands within `BASE_RADIUS`,
/// and whether it is part of the base: the card is anchored on it, not on
/// each machine that was hit.
fn base_anchor(
    before: &BTreeMap<EntityId, (Kind, u8, Pos)>,
    hit: Pos,
) -> Option<(Kind, Pos, bool)> {
    let (hx, hy) = hit.cell_xy();
    let hqs: Vec<Pos> = before
        .values()
        .filter(|(kind, owner, _)| *owner == 0 && *kind == Kind::Headquarters)
        .map(|(_, _, pos)| *pos)
        .collect();
    before
        .values()
        .filter(|(kind, owner, _)| *owner == 0 && kind.is_building())
        .filter(|(_, _, pos)| {
            let (x, y) = pos.cell_xy();
            (x - hx).abs().max((y - hy).abs()) <= BASE_RADIUS
        })
        .map(|(kind, _, pos)| (*kind, *pos, base_building(*kind, *pos, &hqs)))
        // The base answers first, then the nearest building.
        .min_by_key(|(_, pos, base)| (!*base, pos.distance_sq(hit), pos.x, pos.y))
}
/// Whether an own building is part of the base: anything but a nest or a
/// wall, standing within `BASE_SPREAD` of a headquarters (any, without one).
fn base_building(kind: Kind, pos: Pos, hqs: &[Pos]) -> bool {
    if matches!(kind, Kind::Tower | Kind::Palisade) {
        return false;
    }
    let (x, y) = pos.cell_xy();
    hqs.is_empty()
        || hqs.iter().any(|hq| {
            let (hx, hy) = hq.cell_xy();
            (x - hx).abs().max((y - hy).abs()) <= BASE_SPREAD
        })
}
/// Whether `pos` lies at the own base: within `BASE_RADIUS` of a base
/// building.  The simulation's "at base" counts any region with an own
/// building, a mouth nest's too (trial 11: "ENEMY AT BASE" for the sluice).
fn at_base(world: &World, pos: Pos) -> bool {
    let hqs: Vec<Pos> = world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && e.hp > 0 && e.kind == Kind::Headquarters)
        .map(|e| e.pos)
        .collect();
    let (px, py) = pos.cell_xy();
    world.entities.iter().any(|e| {
        let (x, y) = e.pos.cell_xy();
        e.owner == 0
            && e.hp > 0
            && e.kind.is_building()
            && base_building(e.kind, e.pos, &hqs)
            && (x - px).abs().max((y - py).abs()) <= BASE_RADIUS
    })
}
/// Where `pos` is, for a card: the nearest crossing mouth, lane, the
/// sluice or an opponent's base: "AT E LANE, YOUR BANK", "AT E LANE, RED BANK", "IN W
/// LANE", "AT SLUICE", "AT RED BASE".  A compass letter meant nothing on
/// either map (trial 10 read "BASE UNDER ATTACK W" for the E mouth, trial
/// 11 could not decode "W" at all).  None at home.
fn place(world: &World, pos: Pos) -> Option<String> {
    // The Split Basin too: its compass letter meant nothing to a player
    // (trial 11: "BASE UNDER ATTACK W").
    let home = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .map_or_else(|| world.start_of(0), |e| e.pos);
    let (x, y) = pos.cell_xy();
    let near = |at: Pos| {
        let (ax, ay) = at.cell_xy();
        i64::from(ax - x).pow(2) + i64::from(ay - y).pow(2)
    };
    let (hx, hy) = home.cell_xy();
    if (hx - x).abs().max((hy - y).abs()) <= HOME_RADIUS {
        return None;
    }
    // Candidates: (distance, words); None for the own base.
    let mut spots: Vec<(i64, Option<String>)> = vec![(near(home), None)];
    let layout = world.map.layout();
    for (arm, mouths) in world.crossing_mouths().into_iter().enumerate() {
        let letter = crate::seats::arm_letter(world, arm);
        let banks = world.arm_banks(arm);
        // A lane by its letter, a bank by its owner: "AT RED'S E MOUTH"
        // read as the reader's own E lane (trial 12).
        for (mouth, bank) in mouths.into_iter().zip(banks) {
            let words = format!("AT {}", crate::trial12_hold::bank_words(world, arm, bank));
            spots.push((near(mouth), Some(words)));
        }
        if let Some(info) = layout.arms.get(arm) {
            let centre = Pos::cell(info.centre.0, info.centre.1);
            spots.push((near(centre), Some(format!("IN {letter} LANE"))));
        }
    }
    spots.push((near(world.map.gate_pos), Some("AT SLUICE".into())));
    for seat in world.seats().skip(1) {
        let base = world
            .entities
            .iter()
            .find(|e| e.owner == seat && e.kind == Kind::Headquarters)
            .map_or_else(|| world.start_of(seat), |e| e.pos);
        spots.push((
            near(base),
            Some(format!("AT {} BASE", crate::seats::seat_name(world, seat))),
        ));
    }
    spots
        .into_iter()
        .min_by_key(|(distance, _)| *distance)
        .and_then(|(_, words)| words)
}
/// The hold card's place: none.  A count needs both lanes, so one mouth's
/// letter said nothing (trial 11: "ENEMY HOLDS BOTH CROSSINGS E"); F3 goes
/// to the mouth that stops it.
fn hold_place(_world: &World, _pos: Pos) -> Option<String> {
    None
}
/// The compass direction of `pos` from the own headquarters when it lies
/// beyond `HOME_RADIUS` cells; None at home or without a headquarters.
/// North is the top of the chart.  Cards no longer use it; the tests keep
/// it as a reference for where places lie.
#[cfg(test)]
fn compass(world: &World, pos: Pos) -> Option<&'static str> {
    let home = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)?
        .pos;
    let (hx, hy) = home.cell_xy();
    let (x, y) = pos.cell_xy();
    let (dx, dy) = (x - hx, y - hy);
    if dx.abs().max(dy.abs()) <= HOME_RADIUS {
        return None;
    }
    Some(if dx.abs() > 2 * dy.abs() {
        if dx > 0 { "E" } else { "W" }
    } else if dy.abs() > 2 * dx.abs() {
        if dy > 0 { "S" } else { "N" }
    } else {
        match (dx > 0, dy > 0) {
            (true, true) => "SE",
            (true, false) => "NE",
            (false, true) => "SW",
            (false, false) => "NW",
        }
    })
}
#[derive(Default)]
pub struct AlertHistory {
    pub entries: Vec<FieldAlert>,
    next_id: u64,
    recalled: Option<u64>,
    /// Whether the sluice was ours before this tick's events.
    sluice_ours: bool,
    /// The tick of each card when it was last visited: a refreshed red
    /// card is unvisited again.
    visited: BTreeMap<u64, u64>,
    /// Each seat's hold gauge last tick, and whether an opponent's was
    /// rising.
    enemy_hold: Vec<u32>,
    enemy_rising: bool,
    /// Per arm: whether both sides stood at it last tick.
    contested: Vec<bool>,
    /// Own sites without a builder: since when, where, and whether their
    /// card was raised.
    stalled: BTreeMap<EntityId, (u64, Pos, bool)>,
    /// The cap the crew filled when its card was raised: the card comes
    /// once, and again only for a higher cap or after the crew fell back.
    crew_full: Option<u32>,
    /// The last tick salvage came in.
    last_income: u64,
    /// Whether the idle-workers card was raised for this spell.
    idle_raised: bool,
    /// Attack cards raised so far and where the last one was, for the
    /// alarm.
    pub attacks_raised: u32,
    pub last_attack: Option<Pos>,
}
impl AlertHistory {
    pub fn observe(&mut self, world: &World, before: &BTreeMap<EntityId, (Kind, u8, Pos)>) {
        // The enemy's hold card stays for as long as its count rises: the
        // count runs ninety seconds, longer than any card lives.
        let counting = self.enemy_rising;
        self.track_stalled_sites(world);
        let idle_now = self.idle_raised && all_workers_idle(world).is_some();
        let stalled = &self.stalled;
        // A site's card stays for as long as the site stands idle, and goes
        // the moment a builder returns.
        self.entries.retain(|a| {
            if matches!(a.kind, AlertKind::NoBuilder(_)) {
                return stalled.values().any(|(_, pos, _)| *pos == a.pos);
            }
            // Idle workers stay on the card until one gets a job.
            if a.kind == AlertKind::WorkersIdle {
                return idle_now;
            }
            (counting && a.kind == AlertKind::EnemyHold)
                || world.tick.saturating_sub(a.tick)
                    < if a.kind.urgent() {
                        URGENT_LIFETIME
                    } else {
                        LIFETIME
                    }
        });
        let gate = world.map.gate_pos;
        let hq_percent = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters && e.hp > 0)
            .map(|e| {
                (u64::from(e.hp.max(0) as u32) * 100 / u64::from(e.max_hp.max(1) as u32)) as u32
            });
        let was_ours = self.sluice_ours;
        self.sluice_ours = world.gate.owner == Some(0);
        let three = world.seat_count() > 2;
        // The seat behind an event, as a colour: the attacker of a hit, the
        // killer's owner for a death, the Loom's owner for a shell.
        let by_seat = |event: &bw_sim::Event| -> Option<crate::seats::Hue> {
            if !three {
                return None;
            }
            let seat = match event.kind {
                EventKind::Damage | EventKind::ArtilleryWarning => event.player,
                EventKind::Death => event.other.and_then(|id| {
                    before
                        .get(&id)
                        .map(|(_, owner, _)| *owner)
                        .or_else(|| world.entities.iter().find(|e| e.id == id).map(|e| e.owner))
                }),
                _ => None,
            }?;
            (seat != 0 && usize::from(seat) < world.seat_count())
                .then(|| crate::seats::seat_hue(world, seat))
        };
        for event in &world.events {
            if event.kind == EventKind::Deposit && event.player == Some(0) {
                self.last_income = world.tick;
            }
            let item = match event.kind {
                EventKind::Damage if event.amount > 0 => event
                    .entity
                    .and_then(|id| before.get(&id))
                    .filter(|(_, owner, _)| *owner == 0)
                    .map(|(_, _, pos)| {
                        let hit = event.to.unwrap_or(*pos);
                        match base_anchor(before, hit) {
                            Some((_, anchor, true)) => (AlertKind::BaseAttack, anchor),
                            Some((kind, anchor, false)) => (AlertKind::OutpostAttack(kind), anchor),
                            None => (AlertKind::Attack, hit),
                        }
                    }),
                EventKind::BuildCompleted | EventKind::ProductionCompleted
                    if event.player == Some(0) =>
                {
                    event
                        .entity
                        .and_then(|id| world.entities.iter().find(|e| e.id == id && e.owner == 0))
                        .map(|e| {
                            (
                                if event.kind == EventKind::BuildCompleted {
                                    AlertKind::Built(e.kind)
                                } else {
                                    AlertKind::Ready(e.kind)
                                },
                                e.pos,
                            )
                        })
                }
                EventKind::Death => event
                    .entity
                    .and_then(|id| before.get(&id))
                    .filter(|(_, owner, _)| *owner == 0)
                    .map(|(kind, _, pos)| {
                        if kind.is_building() {
                            (AlertKind::Lost(*kind), *pos)
                        } else if kind.is_worker() {
                            (AlertKind::WorkersLost, *pos)
                        } else {
                            (AlertKind::MachinesLost, *pos)
                        }
                    }),
                // A shell on its way to one of ours: the card is on the
                // Loom, which the shot lights for us.
                EventKind::ArtilleryWarning if event.player != Some(0) => event
                    .other
                    .and_then(|id| before.get(&id))
                    .filter(|(_, owner, _)| *owner == 0)
                    .and(event.from)
                    .map(|loom| (AlertKind::LoomFire, loom)),
                EventKind::WorkersFled if event.player == Some(0) => {
                    event.from.map(|pos| (AlertKind::WorkersFled, pos))
                }
                EventKind::GateCaptured => match event.player {
                    Some(0) => Some((AlertKind::SluiceCaptured, gate)),
                    Some(_) if was_ours => Some((AlertKind::SluiceLost, gate)),
                    Some(_) => Some((AlertKind::SluiceTaken, gate)),
                    None => None,
                },
                EventKind::GateCaptureStarted if event.player.is_some_and(|p| p != 0) => {
                    Some((AlertKind::SluiceThreatened, gate))
                }
                EventKind::EnemySeen if event.player == Some(0) => event.to.map(|pos| {
                    if event.other == Some(1) && at_base(world, pos) {
                        (AlertKind::EnemyAtBase, pos)
                    } else {
                        (AlertKind::EnemySeen, pos)
                    }
                }),
                EventKind::UpgradeCompleted if event.player == Some(0) => event
                    .entity
                    .and_then(|id| world.entities.iter().find(|e| e.id == id))
                    .and_then(|e| {
                        bw_content::Upgrade::ALL
                            .into_iter()
                            .find(|u| u.name() == event.text)
                            .map(|u| (AlertKind::UpgradeDone(u), e.pos))
                    }),
                EventKind::SwitchCancelled if event.player == Some(0) => {
                    Some((AlertKind::SwitchCancelled, gate))
                }
                EventKind::WreckEmptied if event.player == Some(0) => event
                    .from
                    .map(|pos| (AlertKind::WreckEmpty(event.text.contains("idle")), pos)),
                // The lanes changed: a switch strands machines and workers,
                // and the card alone changed silently before.
                EventKind::GateChanged => Some((
                    match event.text.as_str() {
                        "flood" => AlertKind::TideFlood,
                        "tide falls" => AlertKind::TideEbb,
                        _ => AlertKind::LanesSwitched,
                    },
                    gate,
                )),
                _ => None,
            };
            if let Some((kind, pos)) = item {
                let direction = match kind {
                    AlertKind::Attack
                    | AlertKind::BaseAttack
                    | AlertKind::OutpostAttack(_)
                    | AlertKind::Lost(_)
                    | AlertKind::MachinesLost
                    | AlertKind::WorkersLost
                    | AlertKind::WorkersFled
                    | AlertKind::LoomFire
                    | AlertKind::EnemySeen
                    | AlertKind::EnemyAtBase
                    | AlertKind::WreckEmpty(_) => place(world, pos),
                    _ => None,
                };
                self.push_with_direction(kind, pos, world.tick, direction);
                if matches!(
                    kind,
                    AlertKind::Attack | AlertKind::BaseAttack | AlertKind::OutpostAttack(_)
                ) {
                    self.attacks_raised = self.attacks_raised.wrapping_add(1);
                    self.last_attack = Some(pos);
                }
                if let Some(by) = by_seat(event)
                    && matches!(
                        kind,
                        AlertKind::Attack
                            | AlertKind::BaseAttack
                            | AlertKind::OutpostAttack(_)
                            | AlertKind::MachinesLost
                            | AlertKind::Lost(_)
                            | AlertKind::WorkersLost
                            | AlertKind::LoomFire
                    )
                    && let Some(card) = self.entries.first_mut()
                {
                    // The last opponent to hit or kill names the card.
                    card.by = Some(by);
                }
                if matches!(
                    kind,
                    AlertKind::MachinesLost | AlertKind::Lost(_) | AlertKind::WorkersLost
                ) && event.cause.is_some()
                    && let Some(card) = self.entries.first_mut()
                {
                    // The cause on the card: the last shooter names the card.
                    card.cause = event.cause;
                }
                if matches!(kind, AlertKind::EnemySeen | AlertKind::EnemyAtBase)
                    && let Some(card) = self.entries.first_mut()
                {
                    card.count = event.amount.max(1) as usize;
                }
                if let AlertKind::Built(building)
                | AlertKind::Lost(building)
                | AlertKind::OutpostAttack(building) = kind
                    && let Some(card) = self.entries.first_mut()
                {
                    card.subject = own_name(world, building);
                }
                if kind == AlertKind::BaseAttack
                    && let Some(card) = self.entries.first_mut()
                {
                    // The hull the raid is eating: the header has no figure.
                    card.hq_percent = hq_percent;
                }
            }
        }
        // The tide count: a red card the moment the enemy's gauge starts to
        // rise, anchored on the enemy-held mouth nearest home, and one when
        // an enemy holder steps onto a lane we hold.
        // Any opponent's count raises it: with three seats the one whose
        // gauge rises, the highest if two do.
        self.enemy_hold.resize(world.lane_hold.len(), 0);
        let counting_seat = world
            .seats()
            .skip(1)
            .filter(|&seat| world.lane_hold[usize::from(seat)] > self.enemy_hold[usize::from(seat)])
            .max_by_key(|&seat| (world.lane_hold[usize::from(seat)], std::cmp::Reverse(seat)));
        let rising = counting_seat.is_some();
        let subject = counting_seat
            .filter(|_| world.seat_count() > 2)
            .map(|seat| crate::seats::seat_name(world, seat));
        let counter_hue = counting_seat
            .filter(|_| world.seat_count() > 2)
            .map(|seat| crate::seats::seat_hue(world, seat));
        let home = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .unwrap_or(gate);
        let anchor = world
            .arms_of(counting_seat.unwrap_or(1))
            .into_iter()
            .filter_map(|lane| crate::qol::lane_presence(world, lane).1)
            .min_by_key(|mouth| mouth.distance_sq(home))
            .unwrap_or(gate);
        let card_subject = self
            .entries
            .iter()
            .find(|a| a.kind == AlertKind::EnemyHold)
            .map(|a| a.subject);
        if rising && (!self.enemy_rising || card_subject != Some(subject)) {
            // A new count, another seat's, or a card that aged out: a fresh
            // red card for F3 to visit.
            self.entries.retain(|a| a.kind != AlertKind::EnemyHold);
            self.push_with_direction(
                AlertKind::EnemyHold,
                anchor,
                world.tick,
                hold_place(world, anchor),
            );
            if let Some(card) = self.entries.first_mut() {
                card.subject = subject;
                card.by = counter_hue;
            }
        } else if rising
            && let Some(i) = self
                .entries
                .iter()
                .position(|a| a.kind == AlertKind::EnemyHold)
        {
            // The card follows the enemy's nearest holder while it counts,
            // and stays first so later cards cannot push it out of sight.
            let mut card = self.entries.remove(i);
            card.pos = anchor;
            card.direction = hold_place(world, anchor);
            card.subject = subject;
            card.by = counter_hue;
            self.entries.insert(0, card);
        }
        if !rising {
            // The count stopped: its card goes with it (trial 11: it
            // outlived a one-second count by sixteen).
            self.entries.retain(|a| a.kind != AlertKind::EnemyHold);
        }
        self.enemy_rising = rising;
        self.enemy_hold.clone_from(&world.lane_hold);
        let arms = crate::seats::arm_count(world);
        self.contested.resize(arms, false);
        for lane in world.arms_of(0) {
            let (own_present, enemy_mouth) = crate::qol::lane_presence(world, lane);
            let contested = own_present && enemy_mouth.is_some();
            if contested && !self.contested[lane] {
                let mouth = enemy_mouth.unwrap_or(gate);
                // With three arms the lane's letter names the card, and the
                // colour of whoever stands at its mouth says who.
                let rival = crate::qol::mouth_seats(world)
                    .get(lane)
                    .map_or(0, |pair| (pair[0] | pair[1]) & !1)
                    .trailing_zeros();
                self.push_with_direction(
                    AlertKind::CrossingContested(lane as u8),
                    mouth,
                    world.tick,
                    // The lane's letter is in the label; F3 goes to the mouth.
                    None,
                );
                if arms > 2
                    && let Some(card) = self.entries.first_mut()
                {
                    card.subject = Some(crate::seats::arm_letter(world, lane));
                    card.by = (usize::try_from(rival).unwrap_or(usize::MAX) < world.seat_count())
                        .then(|| crate::seats::seat_hue(world, rival as u8));
                }
            }
            self.contested[lane] = contested;
        }
        self.track_crew(world);
        self.track_idle_workers(world);
    }
    /// One card when the crew fills its cap: nothing more trains until the
    /// base grows, and at 90 nothing more at all.
    fn track_crew(&mut self, world: &World) {
        let Some(player) = world.players.first() else {
            return;
        };
        let full = player.cap > 0 && player.crew >= player.cap;
        // Fell back well below, or the base grew: the next fill is news.
        if let Some(cap) = self.crew_full
            && (player.crew + 10 <= cap || player.cap > cap)
        {
            self.crew_full = None;
        }
        if full && self.crew_full.is_none() && !world.is_eliminated(0) {
            self.crew_full = Some(player.cap);
            let home = world
                .entities
                .iter()
                .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
                .map_or_else(|| world.start_of(0), |e| e.pos);
            self.push(AlertKind::CrewFull, home, world.tick);
            if let Some(card) = self.entries.first_mut() {
                card.count = player.cap as usize;
            }
        }
    }
    /// One red card when every worker has stood idle with no salvage
    /// coming in for a while: the economy has stopped.
    fn track_idle_workers(&mut self, world: &World) {
        let Some((count, pos)) = all_workers_idle(world) else {
            self.idle_raised = false;
            return;
        };
        if self.idle_raised
            || world.tick.saturating_sub(self.last_income) < IDLE_GRACE
            || world.is_eliminated(0)
        {
            return;
        }
        self.idle_raised = true;
        self.push_with_direction(AlertKind::WorkersIdle, pos, world.tick, place(world, pos));
        if let Some(card) = self.entries.first_mut() {
            card.count = count;
        }
    }
    /// Follow the own construction sites no worker is building, and raise
    /// one card per site once it has stood idle past the grace.
    fn track_stalled_sites(&mut self, world: &World) {
        let idle_sites: Vec<(EntityId, Kind, Pos)> = world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.hp > 0 && e.kind.is_building() && e.build_remaining > 0)
            .filter(|site| !site_has_builder(world, site))
            .map(|e| (e.id, e.kind, e.pos))
            .collect();
        self.stalled
            .retain(|id, _| idle_sites.iter().any(|(site, _, _)| site == id));
        for (id, kind, pos) in idle_sites {
            let entry = self.stalled.entry(id).or_insert((world.tick, pos, false));
            if !entry.2 && world.tick.saturating_sub(entry.0) >= STALL_GRACE {
                entry.2 = true;
                self.push_with_direction(
                    AlertKind::NoBuilder(kind),
                    pos,
                    world.tick,
                    place(world, pos),
                );
                if let Some(card) = self.entries.first_mut() {
                    card.subject = own_name(world, kind);
                }
            }
        }
    }
    pub(crate) fn push(&mut self, kind: AlertKind, pos: Pos, tick: u64) {
        self.push_with_direction(kind, pos, tick, None);
    }
    fn push_with_direction(
        &mut self,
        kind: AlertKind,
        pos: Pos,
        tick: u64,
        direction: Option<String>,
    ) {
        let (x, y) = pos.cell_xy();
        // A card of the same kind near the same place is refreshed, not
        // repeated: completions count up, an assault stays one card for as
        // long as it lasts.
        let radius = kind.merge_radius_cells();
        // An assault from one direction is one card, however many regions
        // it spans: two UNDER ATTACK E cards side by side said nothing more.
        let existing = self.entries.iter().position(|a| {
            let (ax, ay) = a.pos.cell_xy();
            a.kind == kind
                && ((ax - x).abs().max((ay - y).abs()) <= radius
                    || (matches!(
                        kind,
                        AlertKind::Attack
                            | AlertKind::MachinesLost
                            | AlertKind::WorkersLost
                            | AlertKind::LoomFire
                            | AlertKind::EnemySeen
                    ) && direction.is_some()
                        && a.direction == direction))
        });
        if let Some(i) = existing {
            let mut a = self.entries.remove(i);
            a.tick = tick;
            if kind.counted() {
                a.count += 1;
            }
            self.entries.insert(0, a);
        } else {
            self.next_id += 1;
            self.entries.insert(
                0,
                FieldAlert {
                    id: self.next_id,
                    kind,
                    pos,
                    tick,
                    count: 1,
                    direction,
                    hq_percent: None,
                    cause: None,
                    subject: None,
                    by: None,
                },
            );
            self.entries.truncate(8);
        }
        let live: Vec<u64> = self.entries.iter().map(|a| a.id).collect();
        self.visited.retain(|id, _| live.contains(id));
    }
    /// The card F3 goes to: the newest red card not yet looked at since it
    /// was raised or refreshed, else the next card in turn.
    pub fn next(&mut self) -> Option<u64> {
        if self.entries.is_empty() {
            return None;
        }
        let urgent = self
            .entries
            .iter()
            .find(|a| a.kind.urgent() && self.visited.get(&a.id) != Some(&a.tick))
            .map(|a| a.id);
        let id = urgent.unwrap_or_else(|| {
            let index = self
                .recalled
                .and_then(|id| self.entries.iter().position(|a| a.id == id))
                .map_or(0, |i| (i + 1) % self.entries.len());
            self.entries[index].id
        });
        self.mark_visited(id);
        Some(id)
    }
    pub fn mark_visited(&mut self, id: u64) {
        if let Some(a) = self.entries.iter().find(|a| a.id == id) {
            self.visited.insert(id, a.tick);
            self.recalled = Some(id);
        }
    }
}

impl crate::game::Game {
    /// Whether the camera already looks at `pos`: the view a focus on it
    /// would give, within a few cells.
    fn camera_on(&self, pos: Pos) -> bool {
        let mut there = self.camera;
        there.center(pos);
        (there.x - self.camera.x).abs() <= 64 && (there.y - self.camera.y).abs() <= 32
    }

    /// Centre on the mouth that decides the running hold: the banner's
    /// button, and F3's first stop while the enemy counts.
    pub(crate) fn focus_hold(&mut self) {
        let Some(mouth) = self.hold_focus_mouth() else {
            return;
        };
        self.camera.center(mouth);
        self.camera_sub = (0.0, 0.0);
        self.drag = None;
        self.ux.minimap_drag = false;
        self.mode = crate::game::Mode::Context;
        self.last_click = None;
        if let Some(card) = self
            .ux
            .alerts
            .entries
            .iter()
            .find(|a| a.kind == AlertKind::EnemyHold)
            .map(|a| a.id)
        {
            self.ux.alerts.mark_visited(card);
        }
        let message = self.hold_brief();
        self.notify(&message);
    }

    /// The running hold in one sentence: whose, how long left, and what
    /// stops or keeps it.
    pub(crate) fn hold_brief(&self) -> String {
        let Some((player, ticks)) = self.hold_gauge() else {
            return "No count yet. A hold needs the sluice and a gun in each lane.".into();
        };
        let holding = self.world.holds_every_lane(player);
        let left = self.world.hold_ticks().saturating_sub(ticks).div_ceil(30);
        // With three seats the counting opponent is named by its colour.
        let who = if self.world.seat_count() > 2 {
            let name = crate::seats::seat_name(&self.world, player);
            let mut chars = name.chars();
            chars.next().map_or(String::new(), |first| {
                first.to_string() + &chars.as_str().to_lowercase()
            })
        } else {
            "Enemy".to_string()
        };
        match (player, holding) {
            (0, true) => {
                format!("You win in {left}s. Keep a gun in each lane and foes off its banks.")
            }
            (0, false) => {
                format!(
                    "Your count drains 3s a second. {left}s to win once both lanes are held again."
                )
            }
            // Only the counting seat's own lanes (trial 12: a gun at
            // another lane's bank lit its chip and stopped nothing).
            (_, true) => {
                let lanes = crate::trial12_hold::stop_lanes(&self.world, player);
                format!(
                    "{who} wins in {left}s. A gun of yours at a bank of lane {} stops it.",
                    crate::trial12_hold::lane_letters(&self.world, &lanes).replace("OR", "or")
                )
            }
            (_, false) if self.world.seat_count() > 2 => {
                format!("{who}'s count drains 3s a second. {left}s left if it retakes both lanes.")
            }
            (_, false) => {
                format!(
                    "The enemy count drains 3s a second. {left}s left if they retake both lanes."
                )
            }
        }
    }

    pub(crate) fn focus_alert(&mut self, id: Option<u64>) {
        // While the enemy counts, F3 goes to the mouth that stops the count
        // first, unless the camera is already there.
        // The banner's F3 goes where the banner speaks of while it stands,
        // either side's count (trial 11: the banner said "S LANE NOT HELD"
        // and F3 went to "CROSSING N CONTESTED").
        if id.is_none()
            && self.hold_gauge().is_some()
            && !self.spectator_band()
            && let Some(mouth) = self.hold_focus_mouth()
            && !self.camera_on(mouth)
        {
            self.focus_hold();
            return;
        }
        let id = match id {
            Some(id) => {
                self.ux.alerts.mark_visited(id);
                Some(id)
            }
            None => self.ux.alerts.next(),
        };
        // The economy nudges act as well as look: the HQ, a live wreck, the
        // idle producers (trial 12).
        if let Some(kind) = self
            .ux
            .alerts
            .entries
            .iter()
            .find(|a| Some(a.id) == id)
            .map(|a| a.kind)
            && self.focus_nudge(kind)
        {
            return;
        }
        if let Some(a) = self.ux.alerts.entries.iter().find(|a| Some(a.id) == id) {
            self.camera.center(a.pos);
            self.camera_sub = (0.0, 0.0);
            self.drag = None;
            self.ux.minimap_drag = false;
            self.mode = crate::game::Mode::Context;
            self.last_click = None;
            let message = format!(
                "{}.",
                a.caption(self.world.tick.saturating_sub(a.tick) / 30)
            );
            self.notify(&message);
        } else {
            self.notify("No recent alerts.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;
    #[test]
    fn recall_clicks_preserve_selection_and_alerts_keep_to_their_corner() {
        use crate::game::{Action, Game, Mode, Screen};
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-qol2-alert-ui-{}", std::process::id())),
        );
        g.begin_practice();
        g.home();
        let selection = g.selected.clone();
        for practice in [false, true] {
            g.ux.practice = practice;
            for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
                g.resize_view(w, h);
                g.ux.alerts = Default::default();
                for x in [20, 40, 60] {
                    g.ux.alerts
                        .push(AlertKind::Attack, Pos::cell(x, 60), g.world.tick);
                }
                g.ux.alerts.push(
                    AlertKind::Lost(Kind::Dropoff),
                    Pos::cell(70, 60),
                    g.world.tick,
                );
                g.render();
                let view = g.world_view();
                let cards: Vec<_> = g
                    .buttons
                    .iter()
                    .filter(|b| matches!(b.action, Action::FocusAlert(_)))
                    .cloned()
                    .collect();
                assert_eq!(cards.len(), 2, "{w} {h}: two cards at most");
                for a in &cards {
                    // The lean HUD has no notice band: the cards stand in
                    // the field's lower-left corner, over the minimap.
                    let s = g.ui_scale();
                    assert!(
                        a.y + a.h <= view.bottom
                            && a.y >= view.bottom - 46 * s
                            && a.x + a.w <= 150 * s,
                        "{w} {h}: the card sits in the field's lower-left corner"
                    );
                    assert!(!g.native_world_pointer_allowed(a.x + 2, a.y + 2));
                    for b in &g.buttons {
                        if a.action == b.action {
                            continue;
                        }
                        assert!(
                            a.x + a.w <= b.x
                                || b.x + b.w <= a.x
                                || a.y + a.h <= b.y
                                || b.y + b.h <= a.y,
                            "{w} {h}: alert overlaps {:?}",
                            b.action
                        );
                    }
                }
                let b = cards[0].clone();
                let hash = g.world.state_hash();
                g.left_down(b.x + 2, b.y + 2);
                g.left_up(b.x + 2, b.y + 2, false);
                assert_eq!(g.world.state_hash(), hash);
                assert_eq!(g.selected, selection);
                assert_eq!(g.mode, Mode::Context);
                // Trial 10: while a placement waits, a card lets its click
                // through to the ground rather than jumping the camera.
                g.mode = Mode::Build(Kind::Works);
                assert!(g.native_world_pointer_allowed(b.x + 2, b.y + 2));
                g.mode = Mode::Context;
            }
        }
        let age = g.ux.alerts.entries[0].tick;
        g.screen = Screen::Pause;
        for _ in 0..1000 {
            g.tick();
        }
        assert_eq!(g.ux.alerts.entries[0].tick, age);
        g.start();
        assert!(g.ux.alerts.entries.is_empty());
    }
    #[test]
    fn only_local_damage_and_completions_reveal_recorded_target_positions() {
        let mut w = World::new(1, Faction::Union);
        let own = w.entities.iter().find(|e| e.owner == 0).unwrap().clone();
        let enemy = w.entities.iter().find(|e| e.owner == 1).unwrap().clone();
        let before = w
            .entities
            .iter()
            .map(|e| (e.id, (e.kind, e.owner, e.pos)))
            .collect();
        w.events.clear();
        for target in [&own, &enemy] {
            w.events.push(bw_sim::Event {
                tick: 0,
                kind: EventKind::Damage,
                player: Some(1 - target.owner),
                entity: Some(target.id),
                other: None,
                from: Some(enemy.pos),
                to: Some(target.pos),
                amount: 1,
                text: String::new(),
                cause: None,
            });
        }
        let mut alerts = AlertHistory::default();
        alerts.observe(&w, &before);
        assert_eq!(alerts.entries.len(), 1);
        // The anchor is the hit machine or its own base, never the enemy.
        let (ax, ay) = alerts.entries[0].pos.cell_xy();
        let (ox, oy) = own.pos.cell_xy();
        assert!((ax - ox).abs().max((ay - oy).abs()) <= BASE_RADIUS);
        assert_ne!(alerts.entries[0].pos, enemy.pos);
        w.events.clear();
        // Enemy completion without position must not be resolved through world state.
        w.events.push(bw_sim::Event {
            tick: 0,
            kind: EventKind::ProductionCompleted,
            player: Some(1),
            entity: Some(enemy.id),
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        });
        alerts.observe(&w, &before);
        assert_eq!(alerts.entries.len(), 1);
        w.events.clear();
        w.tick = LIFETIME;
        alerts.observe(&w, &before);
        assert_eq!(alerts.entries.len(), 1, "a red card outlives a plain one");
        w.tick = URGENT_LIFETIME;
        alerts.observe(&w, &before);
        assert!(alerts.entries.is_empty());
    }
    #[test]
    fn hits_at_a_base_make_one_card_and_losses_and_sluice_changes_get_cards() {
        let mut w = World::new(2, Faction::Union);
        w.ai_enabled = false;
        let hq = w
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .clone();
        let workers: Vec<_> = w
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .cloned()
            .collect();
        let before: BTreeMap<_, _> = w
            .entities
            .iter()
            .map(|e| (e.id, (e.kind, e.owner, e.pos)))
            .collect();
        let hit = |entity: &bw_sim::Entity, tick: u64| bw_sim::Event {
            tick,
            kind: EventKind::Damage,
            player: Some(1),
            entity: Some(entity.id),
            other: None,
            from: None,
            to: Some(entity.pos),
            amount: 5,
            text: String::new(),
            cause: None,
        };
        w.events.clear();
        for worker in &workers {
            w.events.push(hit(worker, 0));
        }
        w.events.push(hit(&hq, 0));
        let mut alerts = AlertHistory::default();
        alerts.observe(&w, &before);
        assert_eq!(alerts.entries.len(), 1, "one card for the whole base");
        assert_eq!(alerts.entries[0].kind, AlertKind::BaseAttack);
        assert_eq!(alerts.entries[0].pos, hq.pos);
        assert_eq!(alerts.entries[0].label(), "BASE UNDER ATTACK");

        // A machine hit far from any building is a field card.
        let far = Pos::cell(60, 20);
        w.events.clear();
        w.events.push(bw_sim::Event {
            to: Some(far),
            ..hit(&workers[0], 0)
        });
        alerts.observe(&w, &before);
        assert_eq!(alerts.entries.len(), 2);
        assert_eq!(alerts.entries[0].kind, AlertKind::Attack);

        // A destroyed own building, a captured and a lost sluice all get cards.
        w.events.clear();
        w.events.push(bw_sim::Event {
            tick: 0,
            kind: EventKind::Death,
            player: Some(0),
            entity: Some(hq.id),
            other: None,
            from: Some(hq.pos),
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        });
        for player in [Some(0u8), Some(1)] {
            w.events.push(bw_sim::Event {
                tick: 0,
                kind: EventKind::GateCaptured,
                player,
                entity: None,
                other: None,
                from: None,
                to: None,
                amount: 0,
                text: String::new(),
                cause: None,
            });
        }
        w.events.push(bw_sim::Event {
            tick: 0,
            kind: EventKind::GateCaptureStarted,
            player: Some(1),
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        });
        // The sluice was ours before this tick, so the enemy's capture is a loss.
        alerts.sluice_ours = true;
        alerts.observe(&w, &before);
        let labels: Vec<String> = alerts.entries.iter().map(|a| a.label()).collect();
        assert_eq!(
            labels,
            [
                "ENEMY AT THE SLUICE",
                "SLUICE LOST",
                "SLUICE CAPTURED",
                "HQ DESTROYED",
                "UNDER ATTACK IN N LANE",
                "BASE UNDER ATTACK"
            ]
        );
        assert!(alerts.entries[1].kind.urgent() && !alerts.entries[2].kind.urgent());
        assert_eq!(alerts.entries[0].pos, w.map.gate_pos);
    }
    #[test]
    fn f3_goes_to_the_newest_unvisited_red_card_first_and_far_cards_name_a_direction() {
        let mut a = AlertHistory::default();
        a.push(AlertKind::Ready(Kind::Hook), Pos::cell(4, 4), 1);
        let ready = a.entries[0].id;
        a.push(AlertKind::Attack, Pos::cell(40, 40), 2);
        let attack = a.entries[0].id;
        a.push(AlertKind::Ready(Kind::Riveter), Pos::cell(4, 6), 3);
        let newest = a.entries[0].id;
        assert_eq!(a.next(), Some(attack), "the red card comes first");
        assert_eq!(a.next(), Some(ready), "then the cycle continues from it");
        assert_eq!(a.next(), Some(newest));
        // A refreshed red card is unvisited again.
        a.push(AlertKind::Attack, Pos::cell(41, 40), 9);
        assert_eq!(a.next(), Some(attack));

        let mut w = World::new(3, Faction::Union);
        w.ai_enabled = false;
        let hq = w
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .cloned()
            .unwrap();
        let (hx, hy) = hq.pos.cell_xy();
        assert_eq!(compass(&w, hq.pos), None);
        assert_eq!(compass(&w, Pos::cell(hx + 30, hy)), Some("E"));
        assert_eq!(compass(&w, Pos::cell(hx, hy - 30)), Some("N"));
        assert_eq!(compass(&w, Pos::cell(hx + 20, hy + 20)), Some("SE"));
        let mut alerts = AlertHistory::default();
        alerts.push_with_direction(
            AlertKind::Attack,
            Pos::cell(hx + 30, hy),
            1,
            Some("E".into()),
        );
        assert_eq!(alerts.entries[0].label(), "UNDER ATTACK E");
    }
    #[test]
    fn nearby_events_coalesce_history_is_bounded_and_recall_wraps() {
        let mut a = AlertHistory::default();
        a.push(AlertKind::Attack, Pos::cell(4, 4), 1);
        let id = a.entries[0].id;
        for t in 2..100 {
            a.push(AlertKind::Attack, Pos::cell(5, 4), t);
        }
        assert_eq!(a.entries.len(), 1);
        assert_eq!(a.entries[0].id, id);
        assert_eq!(a.entries[0].pos, Pos::cell(4, 4));
        a.push(AlertKind::Ready(Kind::Hook), Pos::cell(4, 4), 100);
        a.push(AlertKind::Ready(Kind::Hook), Pos::cell(5, 4), 101);
        assert_eq!(a.entries[0].count, 2);
        // The unvisited red card first, then the cycle from it.
        assert_eq!(a.next(), Some(id));
        assert_eq!(a.next(), Some(a.entries[0].id));
        assert_eq!(a.next(), Some(id));
        for t in 1..20 {
            a.push(AlertKind::Attack, Pos::cell(t * 8, 50), t as u64);
        }
        assert_eq!(a.entries.len(), 8);
    }

    #[test]
    fn attacks_from_one_direction_are_one_card_across_regions() {
        let mut a = AlertHistory::default();
        a.push_with_direction(AlertKind::Attack, Pos::cell(40, 40), 1, Some("E".into()));
        a.push_with_direction(AlertKind::Attack, Pos::cell(80, 40), 2, Some("E".into()));
        assert_eq!(a.entries.len(), 1, "same kind and direction merge");
        a.push_with_direction(AlertKind::Attack, Pos::cell(40, 90), 3, Some("S".into()));
        assert_eq!(a.entries.len(), 2, "another direction is another card");
    }

    #[test]
    fn any_counting_opponent_raises_the_hold_card_under_its_colour() {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .expect("world");
        world.ai_enabled = false;
        let before = BTreeMap::new();
        let mut a = AlertHistory::default();
        a.observe(&world, &before);
        assert!(a.entries.is_empty());
        // The third seat starts to count: the card names it.
        world.lane_hold[2] = 30;
        a.observe(&world, &before);
        let card = a
            .entries
            .iter()
            .find(|c| c.kind == AlertKind::EnemyHold)
            .expect("hold card");
        // Union, Assembly, Union: the second Union wears pink, the one
        // colour neither the Assembly nor the other Union wears.
        assert_eq!(card.label().split(' ').next(), Some("PINK"));
        assert!(card.label().starts_with("PINK HOLDS BOTH LANES"));
        // While it keeps rising the card stays, whatever its age.
        world.tick += URGENT_LIFETIME + 1;
        world.lane_hold[2] = 60;
        a.observe(&world, &before);
        assert!(a.entries.iter().any(|c| c.kind == AlertKind::EnemyHold));
    }

    #[test]
    fn a_contested_confluence_lane_is_named_by_its_arm_letter() {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .expect("world");
        world.ai_enabled = false;
        // You at your mouth of the west arm, violet at its own.
        let ours = world.own_mouth(0, 1).expect("mouth");
        let theirs = world.own_mouth(2, 1).expect("mouth");
        world.spawn_for_tests(0, Kind::Bulwark, ours);
        world.spawn_for_tests(2, Kind::Bulwark, theirs);
        let mut a = AlertHistory::default();
        a.observe(&world, &BTreeMap::new());
        let card = a
            .entries
            .iter()
            .find(|c| c.kind == AlertKind::CrossingContested(1))
            .expect("contested card");
        assert!(card.label().starts_with("W LANE CONTESTED"));
        // Whose machine contests it, in words and as the card's swatch.
        assert_eq!(card.by, Some(crate::seats::seat_hue(&world, 2)));
        assert_eq!(card.detail().as_deref(), Some("BY PINK"));
    }

    fn three_factions() -> World {
        let mut world = World::with_map(
            1,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("world");
        world.ai_enabled = false;
        world.events.clear();
        world
    }

    fn snapshot(world: &World) -> BTreeMap<EntityId, (Kind, u8, Pos)> {
        world
            .entities
            .iter()
            .map(|e| (e.id, (e.kind, e.owner, e.pos)))
            .collect()
    }

    /// Trial 10: "12 MACHINES LOST W TO NESTS" came from the E lane at
    /// violet's mouth, and the Compact read N or NE on every card. On the
    /// Confluence a card names the mouth, the lane or the base, and the
    /// opponent that did it.
    #[test]
    fn confluence_cards_name_the_mouth_and_the_opponent() {
        let mut world = three_factions();
        // The arm between you (the Union) and the Compact, at the
        // Compact's mouth.
        let arm = world
            .arms_of(0)
            .into_iter()
            .find(|&arm| world.arm_banks(arm).contains(&2))
            .expect("a shared arm");
        let letter = crate::seats::arm_letter(&world, arm);
        let theirs = world.own_mouth(2, arm).expect("mouth");
        let ours = world.own_mouth(0, arm).expect("mouth");
        let victim = world.spawn_for_tests(0, Kind::Riveter, theirs);
        let nest = world.spawn_for_tests(2, Kind::Tower, theirs);
        let before = snapshot(&world);
        world.events.clear();
        world.events.push(bw_sim::Event {
            tick: world.tick,
            kind: EventKind::Death,
            player: Some(0),
            entity: Some(victim),
            other: Some(nest),
            from: Some(theirs),
            to: None,
            amount: 0,
            text: String::new(),
            cause: Some(Kind::Tower),
        });
        let mut alerts = AlertHistory::default();
        alerts.observe(&world, &before);
        // Since rules 20 the Compact's nest also blocks the lane, so a
        // contested-crossing card may come first.
        let card = alerts
            .entries
            .iter()
            .find(|card| card.kind == AlertKind::MachinesLost)
            .expect("a machines-lost card");
        assert_eq!(
            card.label(),
            format!("MACHINE LOST AT {letter} LANE, VIOLET BANK")
        );
        assert_eq!(card.detail().as_deref(), Some("TO VIOLET NESTS"));
        assert_eq!(card.by, Some(crate::seats::Hue::Violet));
        // A card's words always wrap into its two rows, never cut: the
        // kind of what did it goes first.
        let short = card.card_caption(5, 21, 2);
        assert!(
            crate::native_ui::wrap_small(&short, 21, usize::MAX).len() <= 2,
            "{short}"
        );
        // The card's corner swatch says whose; the F3 line says it all.
        assert!(short.starts_with("MACHINE LOST"), "{short}");
        assert!(card.caption(5).contains("TO VIOLET NESTS"));

        // A raid on buildings at your own mouth: the mouth, not "W".
        let mut world = three_factions();
        let ours_nest = world.spawn_for_tests(0, Kind::Tower, ours);
        let raider = world.spawn_for_tests(1, Kind::Reedguard, ours);
        let before = snapshot(&world);
        world.events.clear();
        world.events.push(bw_sim::Event {
            tick: world.tick,
            kind: EventKind::Damage,
            player: Some(1),
            entity: Some(ours_nest),
            other: Some(raider),
            from: Some(ours),
            to: Some(ours),
            amount: 5,
            text: String::new(),
            cause: None,
        });
        let mut alerts = AlertHistory::default();
        alerts.observe(&world, &before);
        let card = &alerts.entries[0];
        // A nest at a mouth is not the base (trial 11): it is named.
        assert_eq!(card.kind, AlertKind::OutpostAttack(Kind::Tower));
        assert_eq!(
            card.label(),
            format!("NEST UNDER ATTACK AT {letter} LANE, YOUR BANK")
        );
        assert_eq!(card.detail().as_deref(), Some("BY JADE"));

        // Places for the other landmarks.
        let world = three_factions();
        assert_eq!(
            place(&world, world.map.gate_pos).as_deref(),
            Some("AT SLUICE")
        );
        assert_eq!(
            place(&world, world.start_of(1)).as_deref(),
            Some("AT JADE BASE")
        );
        assert_eq!(place(&world, world.start_of(0)), None, "home");
        let centre = world.map.layout().arms[arm].centre;
        assert_eq!(
            place(&world, Pos::cell(centre.0, centre.1)).as_deref(),
            Some(format!("IN {letter} LANE").as_str())
        );
    }

    /// The Split Basin names places too: its compass letter meant nothing
    /// to a player (trial 11).
    #[test]
    fn split_basin_cards_name_the_place() {
        let mut world = World::new(1, Faction::Union);
        world.ai_enabled = false;
        let hq = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        let (hx, hy) = hq.cell_xy();
        assert_eq!(
            place(&world, Pos::cell(hx + 30, hy)).as_deref(),
            Some("AT N LANE, YOUR BANK")
        );
        let victim = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(hx + 30, hy));
        let killer = world.spawn_for_tests(1, Kind::Riveter, Pos::cell(hx + 31, hy));
        let before = snapshot(&world);
        world.events.clear();
        world.events.push(bw_sim::Event {
            tick: world.tick,
            kind: EventKind::Death,
            player: Some(0),
            entity: Some(victim),
            other: Some(killer),
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: Some(Kind::Riveter),
        });
        let mut alerts = AlertHistory::default();
        alerts.observe(&world, &before);
        let card = &alerts.entries[0];
        assert_eq!(card.label(), "MACHINE LOST AT N LANE, YOUR BANK");
        assert_eq!(card.detail().as_deref(), Some("TO RIVETERS"));
        assert_eq!(card.by, None, "one opponent needs no name");
    }

    /// Trial 10: Union hit the crew cap at 12:39 and sat on 21 idle
    /// workers at +0 a minute, and no card said either.
    #[test]
    fn a_full_crew_and_an_idle_economy_raise_one_card_each() {
        let mut world = World::new(1, Faction::Union);
        world.ai_enabled = false;
        world.events.clear();
        let before = snapshot(&world);
        let mut alerts = AlertHistory::default();
        let cap = world.players[0].cap;
        world.players[0].crew = cap;
        for id in world.entities.iter().map(|e| e.id).collect::<Vec<_>>() {
            if let Some(e) = world.entities.iter_mut().find(|e| e.id == id)
                && e.owner == 0
                && e.kind.is_worker()
            {
                e.order = bw_sim::Order::Idle;
            }
        }
        alerts.observe(&world, &before);
        let crew = alerts
            .entries
            .iter()
            .filter(|a| a.kind == AlertKind::CrewFull)
            .count();
        assert_eq!(crew, 1);
        let card = alerts
            .entries
            .iter()
            .find(|a| a.kind == AlertKind::CrewFull)
            .unwrap();
        assert_eq!(card.label(), format!("CREW FULL {cap}/{cap}"));
        assert!(!card.kind.urgent());
        // No idle card yet: the grace runs from the last salvage.
        assert!(
            alerts
                .entries
                .iter()
                .all(|a| a.kind != AlertKind::WorkersIdle)
        );
        // A while later with no salvage in: one red idle card, and the
        // crew card is not raised again.
        world.tick += IDLE_GRACE;
        alerts.observe(&world, &before);
        world.tick += 1;
        alerts.observe(&world, &before);
        let idle: Vec<_> = alerts
            .entries
            .iter()
            .filter(|a| a.kind == AlertKind::WorkersIdle)
            .collect();
        assert_eq!(idle.len(), 1);
        let workers = world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .count();
        assert_eq!(
            idle[0].caption(0),
            format!("ALL {workers} WORKERS IDLE NO INCOME 0S AGO")
        );
        assert!(idle[0].kind.urgent());
        // The crew card has aged out and is not raised again.
        assert!(
            alerts
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::CrewFull)
                .count()
                <= 1
        );
        assert_eq!(alerts.crew_full, Some(cap));
        // A worker put to work clears it.
        let hq = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        if let Some(e) = world
            .entities
            .iter_mut()
            .find(|e| e.owner == 0 && e.kind.is_worker())
        {
            e.order = bw_sim::Order::Move { target: hq };
        }
        world.tick += 1;
        alerts.observe(&world, &before);
        assert!(
            alerts
                .entries
                .iter()
                .all(|a| a.kind != AlertKind::WorkersIdle)
        );
    }

    /// Trial 10: Union found red's count by chance; newer cards had pushed
    /// the hold card out of the two shown.  It stays first while it counts.
    #[test]
    fn the_hold_card_stays_in_sight_while_the_count_rises() {
        let mut world = three_factions();
        let before = BTreeMap::new();
        let mut alerts = AlertHistory::default();
        alerts.observe(&world, &before);
        world.lane_hold[1] = 30;
        alerts.observe(&world, &before);
        assert_eq!(alerts.entries[0].kind, AlertKind::EnemyHold);
        assert!(alerts.entries[0].label().starts_with("JADE HOLDS"));
        assert_eq!(
            alerts.entries[0].by,
            Some(crate::seats::Hue::Jade),
            "its swatch"
        );
        for x in [20, 40, 60] {
            alerts.push(AlertKind::Attack, Pos::cell(x, 60), world.tick);
        }
        assert_ne!(alerts.entries[0].kind, AlertKind::EnemyHold);
        world.tick += 1;
        world.lane_hold[1] = 31;
        alerts.observe(&world, &before);
        assert_eq!(alerts.entries[0].kind, AlertKind::EnemyHold, "pinned first");
        // F3 visits it first: a red card not yet looked at.
        let first = alerts.entries[0].id;
        assert_eq!(alerts.next(), Some(first));
    }
}
