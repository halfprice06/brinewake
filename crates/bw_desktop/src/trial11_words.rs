//! The eleventh trial's words (fix stream T): the Compact's GLINT, LAY and
//! Pan explained before the press, the side's own names for its buildings,
//! the pressure-full hint, the capture that stands still without saying
//! why, whose count drains, forward buildings that are not the base, the
//! doctrine that shuts the other, and the enemy under the pointer named
//! before the ground it stands on.  Words only: nothing here changes the
//! simulation.
use crate::canvas::{Color, GOLD, MUTED, RED, WHITE};
use crate::game::{Action, Game};
use crate::hover_card::{Chip, HoverCard, LINE_CHARS, Mark, wrap};
use bw_core::{FP, Faction, Kind, Pos};
use bw_sim::{Order, World};

/// "Heliostats", "Pans": a machine's name in a sentence, plural.
fn plural(kind: Kind) -> String {
    let name = kind.name().to_lowercase();
    let mut chars = name.chars();
    let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or(' ');
    format!("{first}{}s", chars.as_str())
}

/// "Heliostats and Pans", "Bulwarks, Caissons and Looms".
fn listed(kinds: &[Kind]) -> String {
    let words: Vec<String> = kinds.iter().map(|k| plural(*k)).collect();
    match words.len() {
        0 => String::new(),
        1 => words[0].clone(),
        n => format!("{} and {}", words[..n - 1].join(", "), words[n - 1]),
    }
}

/// The machines of `faction` that deploy, in roster order.
pub(crate) fn deployers(faction: Faction) -> Vec<Kind> {
    crate::menus::roster_machines(faction)
        .into_iter()
        .filter(|k| bw_sim::is_specialist(*k))
        .collect()
}

/// What deploying does for one machine, in a card line.
pub(crate) fn deploy_line(kind: Kind) -> String {
    match kind {
        Kind::Bulwark => "Bulwark: shield halves hits.".into(),
        Kind::Loom => "Loom: fires only deployed.".into(),
        Kind::Caisson => "Caisson: plugs a crossing cell.".into(),
        Kind::Heliostat => "Heliostat: beams only deployed.".into(),
        Kind::Pan => format!(
            "Pan: +{} P/min on dry ground.",
            bw_content::PAN_PRESSURE_PER_MINUTE
        ),
        other => format!("{}: deploys.", plural(other)),
    }
}

/// Why D refuses: only the reader's own deploying machines are named
/// (trial 11: the Heliostat's button listed the Union's and the
/// Assembly's too).
pub(crate) fn deploy_refusal(faction: Faction) -> String {
    format!("Only {} deploy.", listed(&deployers(faction)))
}

/// The long words for D: the machines of the selection that deploy, else
/// the side's, and what deploying does for each.
pub(crate) fn deploy_description(game: &Game) -> String {
    let kinds = deploy_kinds(game);
    let lines: Vec<String> = kinds.iter().map(|k| deploy_line(*k)).collect();
    format!(
        "D: deploy {}; P packs. They stand still while changing.\n{}\nA group order packs them; K keeps them deployed.",
        listed(&kinds),
        lines.join(" ")
    )
}

/// The deploying kinds the card speaks of: the selection's, else the side's.
fn deploy_kinds(game: &Game) -> Vec<Kind> {
    let own = deployers(game.faction);
    let selected: Vec<Kind> = own
        .iter()
        .copied()
        .filter(|k| !game.gameplay_ids(|kind| kind == *k).is_empty())
        .collect();
    if selected.is_empty() { own } else { selected }
}

/// GLINT's long words, a price row first.
pub(crate) fn glint_description() -> String {
    format!(
        "{} pressure, {}s\nA Glinter lights a ray {} cells long and {} wide toward a point for {}s, through the fog.\nX, then click a direction. It reaches anywhere; nothing is hurt.",
        bw_content::GLINT_PRESSURE,
        bw_content::GLINT_TICKS / 30,
        bw_content::GLINT_LENGTH / FP,
        bw_content::GLINT_WIDTH / FP,
        bw_content::GLINT_TICKS / 30,
    )
}

/// LAY's long words, a price row first.
pub(crate) fn lay_description() -> String {
    format!(
        "{} pressure\nThe Salter crusts lane or rim water into dry ground, {} cells wide, a row a second, up to {} rows, {} pressure a row.\nCrust matters on deep water: after a DRY or a FLOOD it walks machines across. It melts when the tide next moves.\nL, then click lane or rim water beside a bank.",
        bw_content::LAY_PRESSURE_PER_ROW,
        bw_content::LAY_WIDTH_CELLS,
        bw_content::LAY_MAX_ROWS,
        bw_content::LAY_PRESSURE_PER_ROW,
    )
}

/// The Compact's two orders and D have their own card: the price as
/// marks, what it does and where it works (trial 11: GLINT and LAY had
/// none, and D named every side's machines and not what a Pan does).
/// The doctrine card says what it shuts before it is bought.
pub(crate) fn refine_card(game: &Game, action: &Action, card: &mut HoverCard) {
    let player = &game.world.players[0];
    let pressure = |amount: u32, value: String| Chip {
        mark: Mark::Pressure,
        value,
        short: player.pressure < amount,
    };
    let time = |value: String| Chip {
        mark: Mark::Time,
        value,
        short: false,
    };
    match action {
        Action::Glint => {
            card.costs = vec![
                pressure(
                    bw_content::GLINT_PRESSURE,
                    bw_content::GLINT_PRESSURE.to_string(),
                ),
                time(format!("{}S", bw_content::GLINT_TICKS / 30)),
            ];
            card.body = wrap(
                &format!(
                    "A ray {} long, {} wide lights the fog for {}s.",
                    bw_content::GLINT_LENGTH / FP,
                    bw_content::GLINT_WIDTH / FP,
                    bw_content::GLINT_TICKS / 30
                ),
                LINE_CHARS,
            );
            card.note = wrap("Aim anywhere: X, then a direction.", LINE_CHARS);
        }
        Action::Lay => {
            card.costs = vec![
                pressure(
                    bw_content::LAY_PRESSURE_PER_ROW,
                    format!("{}/ROW", bw_content::LAY_PRESSURE_PER_ROW),
                ),
                time(format!("{}S/ROW", bw_content::LAY_ROW_TICKS / 30)),
            ];
            card.body = wrap(
                &format!(
                    "Crusts lane or rim water dry, {} wide, up to {} rows.",
                    bw_content::LAY_WIDTH_CELLS,
                    bw_content::LAY_MAX_ROWS
                ),
                LINE_CHARS,
            );
            card.note = wrap(
                "Matters on deep water, after DRY or FLOOD. Melts when the tide moves.",
                LINE_CHARS,
            );
        }
        Action::Deploy => {
            card.body = deploy_kinds(game).into_iter().map(deploy_line).collect();
            card.note = wrap("Group orders pack them. K keeps them deployed.", LINE_CHARS);
        }
        Action::Research(doctrine) if player.doctrine.is_none() => {
            // One doctrine a match: say which it shuts before the choice
            // (trial 11 learned it only after buying HAULING).
            if let Some(other) = other_doctrine(*doctrine) {
                let line = format!(
                    "Shuts {} for the match.",
                    crate::tactics::doctrine_name(other)
                );
                // After what it does, before the tier II note.
                let at = card.body.len().min(2);
                for (i, l) in wrap(&line, LINE_CHARS).into_iter().enumerate() {
                    card.body.insert(at + i, l);
                }
            }
        }
        _ => {}
    }
}

/// The doctrine a choice of `doctrine` shuts.
pub(crate) fn other_doctrine(doctrine: bw_content::Doctrine) -> Option<bw_content::Doctrine> {
    use bw_content::Doctrine;
    match doctrine {
        Doctrine::Hauling => Some(Doctrine::FireControl),
        Doctrine::FireControl => Some(Doctrine::Hauling),
    }
}

/// The headquarters in a sentence: the Compact's is its Kiln.
fn hq_word(faction: Faction) -> String {
    match faction {
        Faction::Compact => "Kiln".into(),
        _ => "HQ".into(),
    }
}

/// The ability that spends pressure from a machine, by side.
fn spend_word(faction: Faction) -> &'static str {
    match faction {
        Faction::Compact => "Glint",
        Faction::Union | Faction::Assembly => "Sound",
    }
}

/// The note when pressure fills its cap, in the side's own words and
/// short enough for the prompt's three rows (trial 11: the Compact was
/// told to Sound, and the line lost its end at 101 characters).
pub(crate) fn pressure_full_words(faction: Faction) -> String {
    format!(
        "Pressure is full; the rest trickles into salvage. RECLAIM (R at the {}) melts {} into {}. Surge and {} spend it.",
        hq_word(faction),
        bw_content::RECLAIM_PRESSURE,
        bw_content::RECLAIM_SALVAGE,
        spend_word(faction),
    )
}

/// A building's name as the side calls it, in a sentence: "a Glassworks",
/// "a Rake Shed".
fn building_word(kind: Kind, faction: Faction) -> String {
    let name = crate::ux::building_name(kind, faction).to_lowercase();
    name.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why crew is short, naming the side's own buildings (trial 11: "a Works
/// adds 12, a Drydock 10, a Yard 6" on the Compact's seat).
pub(crate) fn crew_ceiling_words(cap: u32, faction: Faction) -> String {
    if cap >= bw_content::CREW_CAP_MAX {
        return crate::production_qol::crew_ceiling_reason(cap);
    }
    format!(
        "Crew capacity is full: a {} adds {}, a {} {}, a {} {}.",
        building_word(Kind::Works, faction),
        bw_content::CREW_CAP_WORKS,
        building_word(Kind::Drydock, faction),
        bw_content::CREW_CAP_DRYDOCK,
        building_word(Kind::Dropoff, faction),
        bw_content::CREW_CAP_YARD
    )
}

/// A log line from the simulation names buildings generically ("WORKS",
/// "SALVAGE YARD"); on the Compact's seat it reads the Compact's names.
pub(crate) fn own_building_text(text: &str, faction: Faction) -> String {
    for kind in [Kind::Headquarters, Kind::Works, Kind::Dropoff] {
        if text == kind.name() {
            return crate::ux::building_name(kind, faction).to_string();
        }
    }
    text.to_string()
}

/// The Guide's ECONOMY page on the Compact's seat, in its own names.
pub(crate) fn compact_economy_page() -> crate::menus::GuideText {
    (
        &[
            "WORKERS HAUL SALVAGE FROM WRECKS. A LANE WRECK WAITS FOR A DRY LANE.",
            "YOUR KILN ADDS 40 SALVAGE A MINUTE; THE SLUICE ADDS 30 MORE.",
            "PRESSURE BUYS SPECIALISTS AND UPGRADES. CAP +100 PER CONDENSER.",
            "CREW: 40, +12 A GLASSWORKS, 6 A RAKE SHED, 10 A DRYDOCK, UP TO 90.",
        ],
        &[
            ("Q", "RAKER AT THE KILN"),
            ("Q/W/E/R", "TRAIN: GLASSWORKS, DOCK"),
            ("SHIFT", "TRAIN FIVE"),
            ("B / N", "RAKER: GLASSWORKS/DOCK"),
            ("C", "RAKER: CONDENSER"),
            ("Y", "RAKER: RAKE SHED"),
            ("V", "RAKER: DEFENSE NEST"),
            ("P", "RAKER: PALISADE"),
        ],
    )
}

/// The player's words for a LAY aimed at `target`: why the simulation
/// would refuse it, naming what lies there.
pub(crate) fn lay_refusal(world: &World, target: Pos, reason: &str) -> String {
    let (x, y) = target.cell_xy();
    match reason {
        "LAY needs tidal water" => match world.map.terrain(x, y) {
            bw_core::Terrain::Deep => {
                "LAY: OPEN WATER NEVER CRUSTS; CHOOSE LANE OR RIM WATER".into()
            }
            bw_core::Terrain::Rock => "LAY: ROCK; CHOOSE LANE OR RIM WATER".into(),
            _ => "LAY: DRY GROUND; CHOOSE LANE OR RIM WATER".into(),
        },
        "LAY needs a bank" => "LAY: NO BANK BEHIND THAT WATER".into(),
        _ => "LAY: CHOOSE WATER AWAY FROM THE SALTER".into(),
    }
}

/// The prompt while LAY aims: what a click would do under the pointer,
/// or why it would be refused (trial 11: a click on the sea by the Kiln
/// showed a red tile and no reason).
pub(crate) fn lay_prompt(game: &Game) -> String {
    let salter = game
        .gameplay_ids(|k| k == Kind::Salter)
        .first()
        .and_then(|id| game.world.entities.iter().find(|e| e.id == *id))
        .map(|e| e.pos);
    let Some(from) = salter else {
        return "LAY: SELECT A SALTER / ESC CANCELS".into();
    };
    if !game.world_pointer_allowed(game.cursor.0, game.cursor.1) {
        // Off the field, the last refusal stays readable.
        if game.message_live() {
            return format!("{} / ESC CANCELS", game.message);
        }
        return "LAY / CLICK LANE OR RIM WATER / ESC CANCELS".into();
    }
    let target = game.unproject(game.cursor.0, game.cursor.1);
    match game.world.lay_plan(from, target) {
        Ok(_) => format!(
            "LAY: CLICK TO CRUST, {}P A ROW / ESC CANCELS",
            bw_content::LAY_PRESSURE_PER_ROW
        ),
        Err(reason) => format!(
            "{} / ESC CANCELS",
            lay_refusal(&game.world, target, &reason)
        ),
    }
}

/// Where the own capture of the sluice stands and, when it stands still,
/// why: the card's line and the gauge's chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureStatus {
    pub words: String,
    /// Three glyphs for the gauge: machines claiming of the four that count.
    pub chip: String,
    pub color: Color,
}

/// The own capture, read from the simulation's gate and orders (trial 11:
/// six machines stood by the station for five minutes with "Capture order
/// sent" and nothing else).  None when no own machine has the order.
pub(crate) fn capture_status(world: &World) -> Option<CaptureStatus> {
    let gate = world.map.gate_pos;
    let ring = i64::from(2 * FP).pow(2);
    let contest = i64::from(bw_content::CAPTURE_CONTEST_RADIUS_CELLS * FP).pow(2);
    let near = i64::from((bw_content::CAPTURE_CONTEST_RADIUS_CELLS + 3) * FP).pow(2);
    let fighter = |e: &&bw_sim::Entity| {
        e.hp > 0
            && e.build_remaining == 0
            && e.aboard.is_none()
            && !e.kind.is_worker()
            && !e.kind.is_building()
    };
    let ordered: Vec<&bw_sim::Entity> = world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && matches!(e.order, Order::Capture))
        .filter(fighter)
        .collect();
    if ordered.is_empty() || world.gate.owner == Some(0) {
        return None;
    }
    let in_ring = |e: &&&bw_sim::Entity| e.pos.distance_sq(gate) <= ring;
    let claiming = ordered
        .iter()
        .filter(in_ring)
        .filter(|e| e.surge_remaining == 0)
        .count();
    let surging = ordered
        .iter()
        .filter(|e| e.surge_remaining > 0 && e.pos.distance_sq(gate) <= near)
        .count();
    let close = ordered
        .iter()
        .filter(|e| e.pos.distance_sq(gate) <= near)
        .count();
    let enemy = world
        .entities
        .iter()
        .filter(|e| e.owner != 0)
        .filter(fighter)
        .any(|e| e.pos.distance_sq(gate) <= contest);
    let most = bw_content::CAPTURE_MAX_WORKERS as usize;
    let counting = claiming.min(most);
    let chip = format!("{counting}/{most}");
    let percent = if world.gate.capture_player == Some(0) {
        (world.gate.capture_progress * 100 / world.capture_work().max(1)).min(99)
    } else {
        0
    };
    let (words, color) = if claiming > 0 && enemy {
        ("ENEMY IN RING: CLAIM HALTED".to_string(), RED)
    } else if claiming > 0 {
        (
            format!("CLAIMING {percent}% / {counting} OF {most} IN RANGE"),
            GOLD,
        )
    } else if surging > 0 {
        ("SURGING: THE CLAIM WAITS".to_string(), WHITE)
    } else if close > 0 {
        ("STAND CLOSER TO THE STATION".to_string(), WHITE)
    } else {
        ("CAPTURE: ON THE WAY".to_string(), MUTED)
    };
    Some(CaptureStatus { words, chip, color })
}

/// The sluice chip's words during your capture: where it stands and the
/// rule, so a claim that stands still says why (trial 11: the chip spoke
/// of holds only).
pub(crate) fn capture_hint(world: &World) -> String {
    let words = capture_status(world).map_or_else(String::new, |c| c.words);
    format!(
        "{}.\nMachines with G within 2 cells of the station claim it, {} at most; an enemy machine within {} halts the claim.",
        capitalise_words(&words),
        bw_content::CAPTURE_MAX_WORKERS,
        bw_content::CAPTURE_CONTEST_RADIUS_CELLS
    )
}

/// The banner's headline while the own count drains: whose it is and what
/// it has banked (trial 11: "HELD 79/90 DRAINING" read as either side's).
pub(crate) fn own_draining_headline(banked: u32, full: u32) -> String {
    format!("YOUR {banked}/{full} DRAINS")
}

/// The heard line when a count starts: a count that kept part of its run
/// says it resumes (trial 11: "90s to win" after a break read as a reset).
pub(crate) fn hold_begun_words(
    who: &str,
    own: bool,
    left: u32,
    resumed: bool,
    out: bool,
) -> String {
    match (own, resumed, out) {
        (true, false, _) => format!("you hold both lanes: {left}s to win"),
        (true, true, _) => format!("you hold both again: the count resumes, {left}s to win"),
        (false, false, true) => format!("{who} holds its lanes: wins in {left}s"),
        (false, true, true) => format!("{who} holds its lanes again: resumes, wins in {left}s"),
        (false, false, false) => format!("{who} holds both lanes: you lose in {left}s"),
        (false, true, false) => {
            format!("{who} holds both again: the count resumes, you lose in {left}s")
        }
    }
}

/// How much the side gets a minute without wrecks (rules 20): each
/// standing headquarters and the station while it owns it.  The header's
/// rate left it out and read +0/M with the Kiln standing (trial 11).
pub(crate) fn trickle_per_minute(world: &World, player: u8) -> u32 {
    if world.is_eliminated(player) {
        return 0;
    }
    let hqs = world
        .entities
        .iter()
        .filter(|e| {
            e.owner == player && e.kind == Kind::Headquarters && e.hp > 0 && e.build_remaining == 0
        })
        .count() as u32;
    hqs * bw_content::HQ_SALVAGE_PER_MINUTE
        + if world.gate.owner == Some(player) {
            bw_content::STATION_SALVAGE_PER_MINUTE
        } else {
            0
        }
}

/// A worker's load against what it can carry now: the simulation's
/// `carry_for`, read from the side's doctrine and upgrades.
pub(crate) fn carry_capacity(world: &World, player: u8, kind: Kind) -> u32 {
    let side = &world.players[player as usize];
    let mut carry = if kind == Kind::Dredger { 8 } else { 5 };
    if side.doctrine == Some(bw_content::Doctrine::Hauling) {
        carry += 3;
        if side.doctrine_tier >= 2 {
            carry += 3;
        }
    }
    if side.upgrades.contains(&bw_content::Upgrade::Cranes) {
        carry += 2;
    }
    carry
}

/// A worker's panel line while it carries salvage: what it does and its
/// load of what it can carry (trial 11: an idle Raker read "LOADED 5
/// SALVAGE" long after Hauling raised the load to 8).
pub(crate) fn worker_load_words(world: &World, worker: &bw_sim::Entity) -> String {
    let doing = match worker.order {
        Order::Gather { .. } => "GATHERING",
        Order::Build { .. } => "BUILDING",
        Order::Repair { .. } => "REPAIRING",
        Order::Move { .. } => "MOVING",
        Order::Deliver { .. } => "DELIVERING",
        Order::Idle => "IDLE",
        _ => "WORKING",
    };
    format!(
        "{doing} / CARRIES {} OF {}",
        worker.carried,
        carry_capacity(world, worker.owner, worker.kind)
    )
}

/// What stands on a site, in the side's own names, and whether it is one
/// of the player's own sites (a site just queued with shift).
pub(crate) fn occupant(game: &Game, kind: Kind, origin: Pos) -> Option<(String, bool)> {
    let (x, y) = origin.cell_xy();
    let n = bw_content::spec(kind).footprint.max(1);
    let world = &game.world;
    world
        .entities
        .iter()
        .filter(|e| {
            e.hp > 0
                && e.aboard.is_none()
                && (e.owner == 0 || world.entity_visible(0, e.id))
                && !matches!(bw_content::spec(e.kind).movement, bw_core::Movement::Air)
                && (e.owner != 0 || e.kind.is_building() || e.deployed || e.deploy_remaining > 0)
        })
        .find(|e| {
            let (ex, ey) = e.pos.cell_xy();
            let size = bw_content::spec(e.kind).footprint.max(1);
            ex < x + n && x < ex + size && ey < y + n && y < ey + size
        })
        .map(|e| {
            let own_site = e.owner == 0 && e.kind.is_building() && e.build_remaining > 0;
            let whose = if e.owner == 0 { "YOUR" } else { "ENEMY" };
            let name = match e.kind {
                Kind::Tower => "NEST",
                k => crate::ux::building_name(k, world.players[e.owner as usize].faction),
            };
            let state = if e.build_remaining > 0 {
                " SITE"
            } else if !e.kind.is_building() {
                " (DEPLOYED)"
            } else {
                ""
            };
            (format!("{whose} {name}{state}"), own_site)
        })
}

/// The ghost's plate for an occupied site: an own queued site reads as
/// queued, not as an error (trial 11: "OCCUPIED: YOUR CONDENSER SITE"
/// for the site just placed).
pub(crate) fn occupied_plate(game: &Game, kind: Kind, origin: Pos) -> String {
    match occupant(game, kind, origin) {
        Some((what, true)) => format!("{what}: QUEUED"),
        _ => game
            .placement_occupant(kind, origin)
            .map_or_else(|| "OCCUPIED".into(), |o| format!("OCCUPIED: {o}")),
    }
}

/// The prompt's sentence for a refused site.
pub(crate) fn refusal_sentence(
    game: &Game,
    kind: Kind,
    origin: Pos,
    refusal: crate::ux::Refusal,
) -> String {
    use crate::ux::Refusal;
    match refusal {
        Refusal::Occupied => match occupant(game, kind, origin) {
            Some((what, true)) => format!(
                "{} is queued here. Click clear ground for another; Esc stops placing.",
                capitalise_words(&what)
            ),
            _ => "This space is occupied. Choose clear ground; Esc cancels.".into(),
        },
        // A condenser's one need (trial 11: off a well it said "permanent
        // dry land").
        Refusal::NoWell | Refusal::Tidal if kind == Kind::Condenser => {
            "A condenser needs a well: place it on a marked pressure well.".into()
        }
        other => other.sentence().into(),
    }
}

fn capitalise_words(words: &str) -> String {
    let lower = words.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|f| f.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

/// Escape while placing: sites already queued stay (trial 11: "Order
/// cancelled." read as if the queue was lost).
pub(crate) fn cancel_words(game: &Game) -> &'static str {
    let crate::game::Mode::Build(kind) = game.mode else {
        return "Order cancelled.";
    };
    let queued = game
        .world
        .entities
        .iter()
        .any(|e| e.owner == 0 && e.kind == kind && e.hp > 0 && e.build_remaining > 0)
        || game.pending_sites().iter().any(|(k, _)| *k == kind);
    if queued {
        "Placing ended. Sites already placed stay queued."
    } else {
        "Order cancelled."
    }
}

/// The tip for a machine under the pointer, on the scene: an enemy is
/// named; an own machine says nothing, and the ground under either stays
/// quiet.
pub(crate) fn machine_tip(
    game: &Game,
    id: bw_core::EntityId,
) -> Option<crate::field_labels::FieldTip> {
    let e = game.world.entities.iter().find(|e| e.id == id)?;
    let (x, y) = game.camera.project(e.pos);
    enemy_tip(game, id, (x, y - 26))
}

/// The enemy machine under the pointer, as a tip: its name, role and hull,
/// the one thing to know, and what of yours answers it (trial 11: the
/// Union hovered a violet walker and read the mouth ring instead).
pub(crate) fn enemy_tip(
    game: &Game,
    id: bw_core::EntityId,
    anchor: (i32, i32),
) -> Option<crate::field_labels::FieldTip> {
    let e = game.world.entities.iter().find(|e| e.id == id)?;
    if e.owner == 0 || e.kind.is_building() {
        return None;
    }
    let color = crate::seats::seat_colour(&game.world, e.owner);
    let mut tip = crate::field_labels::FieldTip::new(
        crate::field_labels::Area::around(anchor, 0, 0, 0, 0),
        anchor,
        e.kind.name(),
        color,
    )
    .tag(format!(
        "{} / {}",
        crate::seats::seat_name(&game.world, e.owner),
        crate::hover_card::role_tag(e.kind)
    ))
    .icon(format!(
        "ui_unit_{}",
        e.kind.asset(game.world.players[e.owner as usize].faction)
    ))
    .chip(Mark::Hull, format!("{}/{}", e.hp, e.max_hp.max(e.hp)))
    .line(crate::hover_card::key_fact(e.kind));
    let answers = crate::ux::answers(e.kind, game.faction);
    if !answers.is_empty() {
        let names: Vec<&str> = answers.iter().take(2).map(|k| k.name()).collect();
        tip.card.note = wrap(&format!("YOUR ANSWER: {}", names.join(", ")), LINE_CHARS);
    }
    Some(tip)
}

/// A nest's valid sites inside the mouth ring its ghost stands at, as
/// screen diamonds (trial 11: the Compact gave up after 45 s of refusals,
/// with nothing to show which cells of the ring would take it).
pub(crate) fn ring_sites(
    game: &Game,
    camera: bw_core::Camera,
    kind: Kind,
    origin: Pos,
) -> Vec<(i32, i32, Color)> {
    if kind != Kind::Tower {
        return Vec::new();
    }
    let Some((_, mouth, _)) = game.placement_mouth(kind, origin) else {
        return Vec::new();
    };
    let reach = bw_content::CROSSING_HOLD_RADIUS_CELLS;
    let (mx, my) = mouth.cell_xy();
    let mut out = Vec::new();
    for dy in -reach - 1..=reach {
        for dx in -reach - 1..=reach {
            let cell = Pos::cell(mx + dx, my + dy);
            if game.placement_refusal(kind, cell).is_none()
                && game
                    .placement_mouth(kind, cell)
                    .is_some_and(|(_, m, inside)| inside && m == mouth)
            {
                let (px, py) = camera.project(cell);
                out.push((px, py, [GOLD[0], GOLD[1], GOLD[2], 110]));
            }
        }
    }
    out
}

/// The menu's message row: wrapped left of BACK, up to three rows (trial
/// 11: the Guide's pressure note ran under the BACK button).
pub(crate) fn menu_message_rows(message: &str) -> Vec<String> {
    crate::lean_hud::prompt_lines(message, 70)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Button, Screen};
    use bw_sim::MapId;
    use std::path::PathBuf;

    fn game_on(faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-t11words-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.map = MapId::SplitBasin;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn button(action: Action) -> Button {
        Button {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
            label: String::new(),
            hint: String::new(),
            action,
            enabled: true,
        }
    }

    fn spawn(g: &mut Game, owner: u8, kind: Kind, cell: (i32, i32)) -> u32 {
        let id = g
            .world
            .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
        }
        id
    }

    fn hq(g: &Game) -> Pos {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos
    }

    #[test]
    fn glint_and_lay_have_cards_that_say_what_where_and_the_price() {
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        let glinter = spawn(&mut g, 0, Kind::Glinter, (x + 3, y + 3));
        g.selected = vec![glinter];
        let card = g.hover_card(&button(Action::Glint)).expect("a GLINT card");
        let words = format!("{} {}", card.body.join(" "), card.note.join(" "));
        assert!(words.contains("18 long"), "{words}");
        assert!(words.contains("5s"), "{words}");
        assert!(
            card.costs
                .iter()
                .any(|c| c.mark == Mark::Pressure && c.value == "20")
        );
        let salter = spawn(&mut g, 0, Kind::Salter, (x + 3, y + 4));
        g.selected = vec![salter];
        let card = g.hover_card(&button(Action::Lay)).expect("a LAY card");
        let words = format!("{} {}", card.body.join(" "), card.note.join(" "));
        assert!(words.contains("DRY or FLOOD"), "{words}");
        assert!(words.contains("lane or rim water"), "{words}");
        assert!(card.costs.iter().any(|c| c.value == "3/ROW"));
        for line in card.body.iter().chain(&card.note) {
            assert!(line.chars().count() <= LINE_CHARS, "{line}");
        }
        assert!(!g.action_description(&Action::Glint).is_empty());
        assert!(!g.action_description(&Action::Lay).is_empty());
    }

    #[test]
    fn lay_aims_with_a_live_reason_under_the_pointer() {
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        let salter = spawn(&mut g, 0, Kind::Salter, (x + 3, y + 3));
        g.selected = vec![salter];
        g.world.players[0].pressure = 100;
        g.action(Action::Lay);
        assert_eq!(g.mode, crate::game::Mode::Lay);
        // Point at the Salter's own dry ground: the prompt says why not.
        let at = g.project(Pos::cell(x + 3, y + 3));
        g.cursor = at;
        let prompt = g.native_prompt();
        assert!(prompt.starts_with("LAY: "), "{prompt}");
        assert!(prompt.contains("LANE OR RIM"), "{prompt}");
        // A refused click on it says the same, and keeps aiming.
        g.left_down(at.0, at.1);
        assert_eq!(g.mode, crate::game::Mode::Lay);
        assert!(g.message.contains("LANE OR RIM"), "{}", g.message);
        // The sea by the base says it is open water.
        let deep = (0..g.world.map.width as i32)
            .flat_map(|cx| (0..g.world.map.height as i32).map(move |cy| (cx, cy)))
            .find(|&(cx, cy)| g.world.map.terrain(cx, cy) == bw_core::Terrain::Deep)
            .expect("open water");
        let words = lay_refusal(&g.world, Pos::cell(deep.0, deep.1), "LAY needs tidal water");
        assert!(words.contains("OPEN WATER"), "{words}");
    }

    #[test]
    fn deploy_names_only_the_readers_own_machines_and_what_a_pan_does() {
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        let pan = spawn(&mut g, 0, Kind::Pan, (x + 3, y + 3));
        g.selected = vec![pan];
        let card = g.hover_card(&button(Action::Deploy)).expect("card");
        let body = card.body.join(" ");
        assert!(body.contains("+40 P/min"), "{body}");
        assert!(
            !body.contains("Bulwark") && !body.contains("Loom"),
            "{body}"
        );
        let long = g.action_description(&Action::Deploy);
        assert!(
            !long.contains("Bulwark") && !long.contains("Caisson"),
            "{long}"
        );
        assert_eq!(
            deploy_refusal(Faction::Compact),
            "Only Heliostats and Pans deploy."
        );
        assert_eq!(
            deploy_refusal(Faction::Union),
            "Only Bulwarks and Caissons deploy."
        );
        assert_eq!(deploy_refusal(Faction::Assembly), "Only Looms deploy.");
        for kind in [
            Kind::Bulwark,
            Kind::Loom,
            Kind::Caisson,
            Kind::Heliostat,
            Kind::Pan,
        ] {
            assert!(deploy_line(kind).chars().count() <= LINE_CHARS, "{kind:?}");
        }
    }

    #[test]
    fn the_ui_font_has_a_semicolon() {
        assert_ne!(crate::canvas::glyph(';'), [0; 7]);
        assert_ne!(crate::canvas::glyph(';'), crate::canvas::glyph(','));
        // Every glyph the game's prose uses is drawn.
        let prose = format!(
            "{} {} {}",
            pressure_full_words(Faction::Compact),
            crate::ux::upgrade_description(bw_content::Upgrade::Siege, Faction::Compact),
            "Choose clear ground; Esc cancels."
        );
        for c in prose.to_uppercase().chars().filter(|c| *c != ' ') {
            assert_ne!(crate::canvas::glyph(c), [0; 7], "{c:?} has no glyph");
        }
    }

    #[test]
    fn the_pressure_note_speaks_the_sides_words_and_fits_the_prompt() {
        let compact = pressure_full_words(Faction::Compact);
        assert!(
            compact.contains("Glint") && compact.contains("Kiln"),
            "{compact}"
        );
        assert!(!compact.contains("Sound"), "{compact}");
        assert!(pressure_full_words(Faction::Union).contains("Sound"));
        // The prompt row's room at the smallest window: three rows, no cut.
        let room = 640 - crate::lean_hud::PANEL_X - (crate::native_ui::TIP_W + 12);
        let chars = (room / 6) as usize;
        for faction in Faction::ALL {
            let lines = crate::lean_hud::prompt_lines(&pressure_full_words(faction), chars);
            assert!(lines.len() <= 3, "{lines:?}");
            assert!(
                lines.iter().all(|l| l.chars().count() <= chars),
                "{lines:?}"
            );
        }
        // The Guide's row stays left of BACK.
        for row in menu_message_rows(&compact) {
            assert!(row.chars().count() * 6 <= 436, "{row}");
        }
        assert!(menu_message_rows(&compact).len() <= 3);
    }

    #[test]
    fn the_compact_reads_its_own_building_names() {
        let words = crew_ceiling_words(50, Faction::Compact);
        assert!(
            words.contains("Glassworks") && words.contains("Rake Shed"),
            "{words}"
        );
        assert!(!words.contains("a Works"), "{words}");
        assert!(crew_ceiling_words(50, Faction::Union).contains("a Works adds 12"));
        assert_eq!(
            own_building_text("SALVAGE YARD", Faction::Compact),
            "RAKE SHED"
        );
        assert_eq!(own_building_text("WORKS", Faction::Compact), "GLASSWORKS");
        assert_eq!(own_building_text("WORKS", Faction::Union), "WORKS");
        let (facts, keys) = crate::menus::guide_page_on(1, Faction::Compact, MapId::SplitBasin);
        let page = format!("{facts:?} {keys:?}");
        assert!(
            page.contains("GLASSWORKS") && page.contains("RAKE SHED"),
            "{page}"
        );
        assert!(
            !page.contains("SALVAGE YARD") && !page.contains(" WORKS/"),
            "{page}"
        );
        // Y's refusal names the Rake Shed on the Compact's seat.
        let g = game_on(Faction::Compact);
        let reason = g
            .action_reason(&Action::Build(Kind::Dropoff))
            .unwrap_or_default();
        assert!(reason.contains("RAKE SHED"), "{reason}");
    }

    #[test]
    fn the_capture_says_why_it_stands_still() {
        let mut g = game_on(Faction::Compact);
        let (gx, gy) = g.world.map.gate_pos.cell_xy();
        assert_eq!(capture_status(&g.world), None);
        let far = spawn(&mut g, 0, Kind::Brander, (gx + 3, gy));
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == far)
            .unwrap()
            .order = Order::Capture;
        let status = capture_status(&g.world).expect("status");
        assert!(status.words.starts_with("STAND CLOSER"), "{status:?}");
        // Every status line fits the card beside its fold button.
        for words in [
            "CLAIMING 99% / 4 OF 4 IN RANGE",
            "ENEMY IN RING: CLAIM HALTED",
            "SURGING: THE CLAIM WAITS",
            status.words.as_str(),
            "CAPTURE: ON THE WAY",
        ] {
            assert!(words.chars().count() <= 31, "{words}");
        }
        assert_eq!(status.chip, "0/4");
        let near = spawn(&mut g, 0, Kind::Brander, (gx, gy + 1));
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == near)
            .unwrap()
            .order = Order::Capture;
        let status = capture_status(&g.world).expect("status");
        assert!(status.words.contains("1 OF 4 IN RANGE"), "{status:?}");
        assert_eq!(status.chip, "1/4");
        spawn(&mut g, 1, Kind::Riveter, (gx + 2, gy + 2));
        let status = capture_status(&g.world).expect("status");
        assert!(status.words.starts_with("ENEMY IN RING"), "{status:?}");
        g.world.entities.retain(|e| e.owner == 0);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == near)
            .unwrap()
            .surge_remaining = 30;
        let status = capture_status(&g.world).expect("status");
        assert!(status.words.starts_with("SURGING"), "{status:?}");
    }

    #[test]
    fn the_sluice_card_starts_open() {
        assert!(crate::ux::Preferences::default().tide_card);
        // An older settings file, written before the card opened itself,
        // does not fold it again.
        let old: crate::ux::Preferences =
            serde_json::from_str(r#"{"tide_card": false}"#).expect("prefs");
        assert!(old.tide_card);
    }

    #[test]
    fn whose_count_drains_and_a_resumed_count_says_so() {
        assert_eq!(own_draining_headline(79, 90), "YOUR 79/90 DRAINS");
        // The banner's headline room: twenty glyphs.
        assert!(own_draining_headline(120, 120).chars().count() <= 20);
        assert_eq!(
            hold_begun_words("enemy", false, 21, true, false),
            "enemy holds both again: the count resumes, you lose in 21s"
        );
        assert_eq!(
            hold_begun_words("you", true, 90, false, false),
            "you hold both lanes: 90s to win"
        );
        assert!(hold_begun_words("you", true, 31, true, false).contains("resumes"));
    }

    #[test]
    fn the_header_rate_counts_the_trickle() {
        let mut g = game_on(Faction::Compact);
        assert_eq!(
            trickle_per_minute(&g.world, 0),
            bw_content::HQ_SALVAGE_PER_MINUTE
        );
        g.world.gate.owner = Some(0);
        assert_eq!(
            trickle_per_minute(&g.world, 0),
            bw_content::HQ_SALVAGE_PER_MINUTE + bw_content::STATION_SALVAGE_PER_MINUTE
        );
        g.deposits.clear();
        let rate = g
            .console_state()
            .resources
            .salvage_rate_per_minute
            .unwrap_or(0);
        assert!(rate >= 70, "{rate}");
    }

    #[test]
    fn a_workers_load_is_read_against_what_it_can_carry() {
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        let raker = spawn(&mut g, 0, Kind::Raker, (x + 3, y + 3));
        g.world.players[0].doctrine = Some(bw_content::Doctrine::Hauling);
        let e = g.world.entities.iter_mut().find(|e| e.id == raker).unwrap();
        e.carried = 5;
        e.order = Order::Idle;
        let e = g
            .world
            .entities
            .iter()
            .find(|e| e.id == raker)
            .unwrap()
            .clone();
        assert_eq!(worker_load_words(&g.world, &e), "IDLE / CARRIES 5 OF 8");
    }

    #[test]
    fn a_queued_site_reads_as_queued_and_escape_keeps_it() {
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        let site = g
            .world
            .spawn_for_tests(0, Kind::Tower, Pos::cell(x + 6, y + 6));
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == site)
            .unwrap()
            .build_remaining = 100;
        let origin = Pos::cell(x + 6, y + 6);
        let plate = occupied_plate(&g, Kind::Tower, origin);
        assert_eq!(plate, "YOUR NEST SITE: QUEUED");
        let sentence = refusal_sentence(&g, Kind::Tower, origin, crate::ux::Refusal::Occupied);
        assert!(sentence.contains("queued"), "{sentence}");
        g.mode = crate::game::Mode::Build(Kind::Tower);
        assert!(cancel_words(&g).contains("stay queued"));
        g.mode = crate::game::Mode::Attack;
        assert_eq!(cancel_words(&g), "Order cancelled.");
        // A condenser off a well asks for a well.
        let reason = g
            .placement_reason(Kind::Condenser, Pos::cell(x + 5, y - 5))
            .unwrap_or_default();
        assert!(reason.contains("well"), "{reason}");
    }

    #[test]
    fn the_doctrine_card_says_what_it_shuts_before_the_choice() {
        let mut g = game_on(Faction::Union);
        let hq_id = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .id;
        g.selected = vec![hq_id];
        let card = g
            .hover_card(&button(Action::Research(bw_content::Doctrine::Hauling)))
            .expect("card");
        let body = card.body.join(" ");
        assert!(body.contains("Shuts FIRE CONTROL"), "{body}");
    }

    #[test]
    fn an_enemy_under_the_pointer_is_named_with_its_answer() {
        let mut g = game_on(Faction::Union);
        let (x, y) = hq(&g).cell_xy();
        let brander = spawn(&mut g, 1, Kind::Brander, (x + 4, y + 4));
        let tip = enemy_tip(&g, brander, (10, 10)).expect("tip");
        assert_eq!(tip.card.title, "BRANDER");
        assert!(tip.card.tag.as_deref().unwrap_or("").starts_with("ENEMY"));
        let own = spawn(&mut g, 0, Kind::Riveter, (x + 4, y + 5));
        assert!(enemy_tip(&g, own, (10, 10)).is_none());
    }

    #[test]
    fn the_roster_opens_on_the_readers_own_buildings() {
        let mut g = game_on(Faction::Union);
        g.ux.help_page = crate::menus::ROSTER_PAGE;
        g.open_screen(Screen::Help);
        g.action(Action::RosterFaction(Faction::Compact));
        g.action(Action::RosterBuildings);
        g.render();
        assert!(g.ux.roster_faction.is_none_or(|f| f == Faction::Union));
    }

    #[test]
    fn the_formation_word_is_not_a_sides_name() {
        for formation in [
            bw_sim::Formation::Compact,
            bw_sim::Formation::Line,
            bw_sim::Formation::Loose,
        ] {
            let word = crate::tactics::formation_name(formation);
            for faction in Faction::ALL {
                assert!(!faction.name().contains(word), "{word}");
            }
        }
    }

    fn snapshot(world: &World) -> std::collections::BTreeMap<u32, (Kind, u8, Pos)> {
        world
            .entities
            .iter()
            .map(|e| (e.id, (e.kind, e.owner, e.pos)))
            .collect()
    }

    fn event(
        kind: bw_sim::EventKind,
        player: u8,
        entity: Option<u32>,
        to: Option<Pos>,
    ) -> bw_sim::Event {
        bw_sim::Event {
            tick: 0,
            kind,
            player: Some(player),
            entity,
            other: Some(1),
            from: None,
            to,
            amount: 5,
            text: String::new(),
            cause: None,
        }
    }

    #[test]
    fn a_forward_nest_is_named_and_the_sluice_is_not_the_base() {
        use crate::field_alerts::{AlertHistory, AlertKind};
        let mut g = game_on(Faction::Compact);
        let mouth = g.world.crossing_mouths()[0][0];
        let (mx, my) = mouth.cell_xy();
        let nest = g.world.spawn_for_tests(0, Kind::Tower, Pos::cell(mx, my));
        let before = snapshot(&g.world);
        g.world.events.clear();
        g.world
            .events
            .push(event(bw_sim::EventKind::Damage, 1, Some(nest), None));
        let mut alerts = AlertHistory::default();
        alerts.observe(&g.world, &before);
        let card = &alerts.entries[0];
        assert_eq!(card.kind, AlertKind::OutpostAttack(Kind::Tower));
        let label = card.label();
        assert!(label.starts_with("NEST UNDER ATTACK"), "{label}");
        assert!(label.contains("BANK"), "{label}");
        // Enemies at the sluice, reported "at base" by the simulation's
        // region test because a nest stands near: not the base.
        g.world.events.clear();
        g.world.events.push(event(
            bw_sim::EventKind::EnemySeen,
            0,
            None,
            Some(g.world.map.gate_pos),
        ));
        let mut alerts = AlertHistory::default();
        alerts.observe(&g.world, &before);
        assert_eq!(alerts.entries[0].kind, AlertKind::EnemySeen);
        assert!(!alerts.entries[0].label().contains("BASE"));
        // At the Kiln it is the base.
        g.world.events.clear();
        g.world
            .events
            .push(event(bw_sim::EventKind::EnemySeen, 0, None, Some(hq(&g))));
        let mut alerts = AlertHistory::default();
        alerts.observe(&g.world, &before);
        assert_eq!(alerts.entries[0].kind, AlertKind::EnemyAtBase);
    }

    #[test]
    fn a_split_basin_card_names_the_place_not_a_compass_letter() {
        use crate::field_alerts::AlertHistory;
        let mut g = game_on(Faction::Union);
        let (hx, hy) = hq(&g).cell_xy();
        let far = Pos::cell(hx + 40, hy);
        let victim = g.world.spawn_for_tests(0, Kind::Riveter, far);
        let before = snapshot(&g.world);
        g.world.events.clear();
        let mut death = event(bw_sim::EventKind::Death, 0, Some(victim), None);
        death.other = None;
        g.world.events.push(death);
        let mut alerts = AlertHistory::default();
        alerts.observe(&g.world, &before);
        let label = alerts.entries[0].label();
        assert!(label.contains(" AT ") || label.contains(" IN "), "{label}");
        for letter in [" E", " W", " N", " S", " NE", " SE", " NW", " SW"] {
            assert!(!label.ends_with(letter), "{label}");
        }
    }

    #[test]
    fn the_enemy_hold_card_goes_when_the_count_stops_and_names_no_mouth() {
        use crate::field_alerts::{AlertHistory, AlertKind};
        let mut g = game_on(Faction::Compact);
        let before = std::collections::BTreeMap::new();
        let mut alerts = AlertHistory::default();
        g.world.lane_hold[1] = 30;
        alerts.observe(&g.world, &before);
        let card = alerts
            .entries
            .iter()
            .find(|a| a.kind == AlertKind::EnemyHold)
            .expect("hold card");
        assert_eq!(card.label(), "ENEMY HOLDS BOTH LANES");
        // The count stops: the card goes at once.
        alerts.observe(&g.world, &before);
        assert!(
            !alerts
                .entries
                .iter()
                .any(|a| a.kind == AlertKind::EnemyHold)
        );
    }

    #[test]
    fn f3_goes_where_the_banner_speaks_of_first() {
        let mut g = game_on(Faction::Assembly);
        // Your count drains with a lane not held: the banner stands.
        g.world.lane_hold[0] = 30 * 20;
        assert!(g.hold_gauge().is_some());
        let mouth = g.hold_focus_mouth().expect("a mouth");
        g.ux.alerts.push(
            crate::field_alerts::AlertKind::CrossingContested(0),
            Pos::cell(5, 5),
            g.world.tick,
        );
        g.focus_alert(None);
        let mut there = g.camera;
        there.center(mouth);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
    }

    /// Pictures of the new words for review, saved when
    /// BW_TRIAL11_WORDS_FRAMES names a folder; the scenes render either way.
    #[test]
    fn frames_for_review() {
        fn keep(g: &Game, name: &str) {
            if let Ok(dir) = std::env::var("BW_TRIAL11_WORDS_FRAMES") {
                std::fs::create_dir_all(&dir).expect("frame folder");
                g.canvas
                    .save(&PathBuf::from(dir).join(format!("{name}.png")))
                    .expect("frame");
            }
        }
        fn hover(g: &mut Game, action: Action) {
            g.render();
            let b = g
                .buttons
                .iter()
                .find(|b| b.action == action)
                .cloned()
                .unwrap_or_else(|| panic!("{action:?} button"));
            g.cursor = (b.x + b.w / 2, b.y + b.h / 2);
            g.render();
        }
        let mut g = game_on(Faction::Compact);
        let (x, y) = hq(&g).cell_xy();
        g.world.players[0].pressure = 100;
        let glinter = spawn(&mut g, 0, Kind::Glinter, (x + 3, y + 3));
        g.selected = vec![glinter];
        hover(&mut g, Action::Glint);
        keep(&g, "glint-card");
        let salter = spawn(&mut g, 0, Kind::Salter, (x + 3, y + 4));
        g.selected = vec![salter];
        hover(&mut g, Action::Lay);
        keep(&g, "lay-card");
        g.action(Action::Lay);
        let deep = (0..g.world.map.width as i32)
            .flat_map(|cx| (0..g.world.map.height as i32).map(move |cy| (cx, cy)))
            .filter(|&(cx, cy)| g.world.map.terrain(cx, cy) == bw_core::Terrain::Deep)
            .min_by_key(|&(cx, cy)| (cx - x).pow(2) + (cy - y).pow(2))
            .expect("open water");
        g.camera.center(Pos::cell(deep.0, deep.1));
        g.cursor = g.project(Pos::cell(deep.0, deep.1));
        g.render();
        assert!(
            g.native_prompt().contains("OPEN WATER"),
            "{}",
            g.native_prompt()
        );
        keep(&g, "lay-open-water");
        g.cancel_mode();
        let pan = spawn(&mut g, 0, Kind::Pan, (x + 4, y + 3));
        g.selected = vec![pan];
        g.camera.center(hq(&g));
        hover(&mut g, Action::Deploy);
        keep(&g, "deploy-card");
        // A capture that stands two cells short.
        let gate = g.world.map.gate_pos;
        let (gx, gy) = gate.cell_xy();
        let brander = spawn(&mut g, 0, Kind::Brander, (gx + 3, gy));
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == brander)
            .unwrap()
            .order = Order::Capture;
        g.selected.clear();
        g.camera.center(gate);
        g.cursor = (10, 400);
        g.render();
        keep(&g, "capture-stand-closer");
        // Your count drains.
        g.world.entities.retain(|e| e.id != brander);
        g.world.gate.owner = Some(0);
        g.world.lane_hold[0] = 50 * 30;
        g.tick();
        g.render();
        keep(&g, "own-draining");
        // An enemy under the pointer at a mouth.
        g.world.gate.owner = None;
        g.world.lane_hold[0] = 0;
        let mouth = g.world.crossing_mouths()[0][0];
        let (mx, my) = mouth.cell_xy();
        let enemy = spawn(&mut g, 1, Kind::Brander, (mx, my));
        spawn(&mut g, 0, Kind::Stilt, (mx - 2, my));
        for _ in 0..4 {
            g.tick();
        }
        g.camera.center(mouth);
        g.render();
        let (px, py) = g.project(Pos::cell(mx, my));
        let spot = (-30..10)
            .flat_map(|dy| (-10..10).map(move |dx| (px + dx, py + dy)))
            .find(|&(sx, sy)| g.unit_at(sx, sy) == Some(enemy));
        if let Some(spot) = spot {
            g.cursor = spot;
            g.render();
            let tip = g.hover_tip.clone().expect("a tip for the enemy");
            assert_eq!(tip.card.title, "BRANDER");
            keep(&g, "enemy-tip");
        }
        // A nest's valid sites in the ring.
        g.world.entities.retain(|e| e.id != enemy);
        let raker = spawn(&mut g, 0, Kind::Raker, (mx - 1, my));
        g.selected = vec![raker];
        g.world.players[0].salvage = 1000;
        g.action(Action::Build(Kind::Tower));
        g.cursor = g.project(Pos::cell(mx, my));
        g.render();
        keep(&g, "nest-ring");
    }

    #[test]
    fn the_compact_hears_its_own_building_names_on_alerts() {
        use crate::field_alerts::AlertHistory;
        let mut g = game_on(Faction::Compact);
        let (hx, hy) = hq(&g).cell_xy();
        let works = g
            .world
            .spawn_for_tests(0, Kind::Works, Pos::cell(hx + 5, hy + 5));
        let before = snapshot(&g.world);
        g.world.events.clear();
        g.world.events.push(event(
            bw_sim::EventKind::BuildCompleted,
            0,
            Some(works),
            None,
        ));
        let mut alerts = AlertHistory::default();
        alerts.observe(&g.world, &before);
        assert!(alerts.entries[0].label().starts_with("GLASSWORKS BUILT"));
    }
}
