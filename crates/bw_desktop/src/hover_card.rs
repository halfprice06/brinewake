//! The hover card: what a button is, what it costs and what it is for, in
//! that order (playtest, 2026-09-23: the old tooltip was a paragraph with no
//! place for the eye to land).  In the manner of StarCraft II's command
//! card tooltips: the name and key, the price as marks and numbers, one
//! line of what it does, and nothing the Guide already says at length.
//!
//! The card is built from data: a machine's numbers come from its spec,
//! its short role and the one thing to know from the tables below, so a
//! new roster fills them in and gets the same card.
use crate::canvas::{EDGE, GOLD, JADE, MUTED, RED, WHITE};
use crate::game::{Action, Button, Game};
use crate::native_ui::{Rect, card, fit, fit_small, small, text};
use crate::occlusion::PixelScale;
use bw_content::spec;
use bw_core::{FP, Kind};

/// The card's width in interface columns.
const CARD_W: i32 = crate::native_ui::TIP_W;
const PAD: i32 = 7;
/// A row of small text or of marks.
const ROW: i32 = 10;
/// The glyphs a line holds between the pads.
pub(crate) const LINE_CHARS: usize = ((CARD_W - 2 * PAD) / 6) as usize;
/// At most this many lines of prose, the reason included.
const MAX_LINES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Salvage,
    Pressure,
    Crew,
    Time,
    Damage,
    Reach,
    Hull,
    Speed,
    Sight,
}
impl Mark {
    fn sprite(self) -> &'static str {
        match self {
            Mark::Salvage => "tip_salvage",
            Mark::Pressure => "tip_pressure",
            Mark::Crew => "tip_crew",
            Mark::Time => "tip_time",
            Mark::Damage => "tip_damage",
            Mark::Reach => "tip_reach",
            Mark::Hull => "tip_hull",
            Mark::Speed => "tip_speed",
            Mark::Sight => "tip_sight",
        }
    }
}

/// One mark and its number; `short` when the player cannot pay it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Chip {
    pub mark: Mark,
    pub value: String,
    pub short: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HoverCard {
    pub title: String,
    pub key: Option<String>,
    /// A short role under the name: "LINE FIGHTER", "UPGRADE".
    pub tag: Option<String>,
    pub icon: Option<String>,
    /// Why the button refuses, in gold, above everything else.
    pub reason: Option<String>,
    pub costs: Vec<Chip>,
    pub stats: Vec<Chip>,
    /// What it does: one line where the data allows, white.
    pub body: Vec<String>,
    pub good_vs: Option<String>,
    /// The least urgent line, muted.
    pub note: Vec<String>,
    /// The edge colour, when the card names a thing on the field rather
    /// than a button: an enemy's red, a well's jade.
    pub accent: Option<crate::canvas::Color>,
}

/// What a machine or building brings, as marks: hull when asked, then
/// damage and reach for anything armed, speed for anything that moves and
/// sight for machines.
pub(crate) fn stat_chips(kind: Kind, hull: bool) -> Vec<Chip> {
    let s = spec(kind);
    let mut chips = Vec::new();
    let mut stat = |mark, value: String| {
        chips.push(Chip {
            mark,
            value,
            short: false,
        })
    };
    if hull {
        stat(Mark::Hull, s.health.to_string());
    }
    if s.damage > 0 {
        stat(Mark::Damage, s.damage.to_string());
        stat(Mark::Reach, (s.range / FP).to_string());
    }
    if s.speed > 0 {
        stat(Mark::Speed, (s.speed / FP).to_string());
    }
    if !kind.is_building() {
        stat(Mark::Sight, (s.sight / FP).to_string());
    }
    chips
}

/// The two or three words under a machine's or building's name.
pub(crate) fn role_tag(kind: Kind) -> &'static str {
    match kind {
        Kind::Hook | Kind::Wick | Kind::Raker => "WORKER",
        Kind::Riveter => "CLOSE FIGHTER",
        Kind::Bulwark => "SHIELD",
        Kind::Sounder => "FAST SPOTTER",
        Kind::Reedguard => "LINE FIGHTER",
        Kind::Skipper => "RAIDER",
        Kind::Loom => "ARTILLERY",
        Kind::Tidewatch | Kind::Lampwright | Kind::Stilt => "SCOUT",
        Kind::Caulker | Kind::Tender | Kind::Glazier => "MENDER",
        Kind::Brander => "FAST FIGHTER",
        Kind::Heliostat => "BEAM",
        Kind::Glinter => "HARASSER",
        Kind::Salter => "CAUSEWAY LAYER",
        Kind::Pan => "PRESSURE",
        Kind::Caisson => "CROSSING PLUG",
        Kind::Dredger => "WET SALVAGER",
        Kind::Barge => "WATER TRANSPORT",
        Kind::Lifter => "AIR TRANSPORT",
        Kind::Headquarters => "BASE",
        Kind::Works => "FACTORY",
        Kind::Drydock => "SPECIALIST YARD",
        Kind::Condenser => "PRESSURE",
        Kind::Dropoff => "SALVAGE DROP-OFF",
        Kind::Tower => "DEFENCE",
        Kind::Palisade => "WALL",
    }
}

/// The one thing to know before building it, in a line.
pub(crate) fn key_fact(kind: Kind) -> &'static str {
    match kind {
        Kind::Hook | Kind::Wick | Kind::Raker => "Gathers, builds and repairs.",
        Kind::Riveter => "1.5x damage to buildings.",
        Kind::Bulwark => "D: shield halves front damage.",
        Kind::Sounder => "Sees far. X lights the fog.",
        Kind::Reedguard => "Tough. Holds the front line.",
        Kind::Skipper => "Fast in floods. X lights fog.",
        Kind::Loom => "D to fire. Blind inside 2.",
        Kind::Tidewatch | Kind::Lampwright | Kind::Stilt => "No gun. Hold doubles its sight.",
        Kind::Caulker | Kind::Tender | Kind::Glazier => "Mends machines within 3 cells.",
        Kind::Brander => "Fastest line fighter. Frail.",
        Kind::Heliostat => "D to fire. Beam builds to 10.",
        Kind::Glinter => "X: a long ray lights the fog.",
        Kind::Salter => "L: dry causeway over a lane.",
        Kind::Pan => "D on dry ground: +40 pressure/min.",
        Kind::Caisson => "D plugs a crossing cell.",
        Kind::Dredger => "Digs wrecks, wet lanes too.",
        Kind::Barge => "Carries 4 by water. U unloads.",
        Kind::Lifter => "Flies 4 over walls. U unloads.",
        Kind::Headquarters => "Lose it and the match is lost.",
        Kind::Works => "Trains the line. Adds 12 crew.",
        Kind::Drydock => "Trains specialists. +10 crew.",
        Kind::Condenser => "On a well. Pressure cap +100.",
        Kind::Dropoff => "Salvage drop-off. +6 crew.",
        Kind::Tower => "A fixed gun with a long reach.",
        Kind::Palisade => "Blocks movement. No gun.",
    }
}

/// Short words for the orders whose descriptions run long: what it does,
/// then the one caveat.  Anything not here speaks in its description's
/// first sentence.
fn short_words(action: &Action) -> Option<(&'static str, Option<&'static str>)> {
    Some(match action {
        Action::Attack => ("Move, fighting anything on the way.", None),
        Action::Hold => ("Stay put and fire at anything in range.", None),
        Action::Stop => ("Drop every order.", None),
        Action::Capture => (
            "Claim the sluice: 40s with machines beside it.",
            Some("An enemy beside it halts the claim."),
        ),
        Action::Gather => ("Send workers to a wreck to haul salvage.", None),
        Action::Recycle => (
            "Walks to your nearest HQ, Works or Yard and is broken up.",
            Some("Half its salvage back. Frees its crew."),
        ),
        Action::Surge => ("+50% speed. Weapons off.", Some("12s cooldown.")),
        Action::Vent => ("Combat machines fire 30% faster for 10s.", None),
        Action::Sound => ("Lights the ground around it for 5s.", None),
        Action::Glint => ("Lights a long thin ray toward a point for 5s.", None),
        Action::Lay => (
            "Crusts tidal water into a dry causeway, a row a second, until the tide moves.",
            None,
        ),
        Action::Deploy => (
            "Deploy Bulwarks, Looms, Caissons, Heliostats and Pans.",
            Some("Group orders pack them. K keeps them deployed."),
        ),
        Action::Pack => ("Pack deployed machines so they can move.", None),
        Action::KeepDeployed => ("Keep them deployed on a group move.", None),
        Action::Formation => ("Tight, Line or Loose for the next move.", None),
        Action::Face => ("Click a direction to face.", Some("Clears queued moves.")),
        Action::Unload => ("Set the hold down on free ground nearby.", None),
        Action::Board => ("Climb into the nearest own transport.", None),
        Action::Cancel => (
            "Cancel the first in the queue.",
            Some("Started work refunds 75%."),
        ),
        _ => return None,
    })
}

/// A paragraph's first sentence, its stop kept.
fn first_sentence(paragraph: &str) -> String {
    match paragraph.find(". ") {
        Some(i) => paragraph[..=i].to_string(),
        None => paragraph.to_string(),
    }
}

/// Wrap words to `width` glyphs.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Read a price row ("40 pressure, 10s warning", "120 salvage, 60
/// pressure, 30s") into marks; None when the row is prose.
fn price_chips(row: &str) -> Option<Vec<Chip>> {
    let mut chips = Vec::new();
    for part in row.split(',') {
        let part = part.trim();
        let mut words = part.split_whitespace();
        let first = words.next()?;
        let rest: Vec<&str> = words.collect();
        let chip = |mark, value: &str| Chip {
            mark,
            value: value.to_string(),
            short: false,
        };
        if let Some(n) = first.strip_suffix('s')
            && n.parse::<u32>().is_ok()
        {
            chips.push(chip(Mark::Time, &first.to_uppercase()));
            continue;
        }
        first.parse::<u32>().ok()?;
        match rest.first().copied() {
            Some("salvage") => chips.push(chip(Mark::Salvage, first)),
            Some("pressure") => chips.push(chip(Mark::Pressure, first)),
            Some("crew") => chips.push(chip(Mark::Crew, first)),
            _ => return None,
        }
    }
    (!chips.is_empty()).then_some(chips)
}

impl Game {
    /// The enemy being read in the selection panel: clicked over the
    /// current selection, still alive and in sight.
    pub(crate) fn inspected_enemy(&self) -> Option<&bw_sim::Entity> {
        let (id, over) = self.inspected.as_ref()?;
        if *over != self.selected {
            return None;
        }
        self.world
            .entities
            .iter()
            .find(|e| e.id == *id && e.owner != 0 && e.hp > 0 && self.world.entity_visible(0, e.id))
    }

    /// An enemy's numbers where the selection panel stands, read only: its
    /// picture, name and role in its seat's colour, its hull and what it
    /// brings as marks, and the one thing to know (trial 10: a click only
    /// named it in the prompt).
    pub(crate) fn draw_inspected_enemy(&mut self, x: i32, w: i32, h: i32, s: i32) {
        let Some(e) = self.inspected_enemy().cloned() else {
            return;
        };
        let faction = self.world.players[e.owner as usize].faction;
        let color = crate::seats::seat_colour(&self.world, e.owner);
        let mut title_x = x;
        let (key, times, inset) = if e.kind.is_building() {
            (format!("ui_build_{}", e.kind.asset(faction)), 2, 4)
        } else {
            (format!("portrait_{}", e.kind.asset(faction)), 1, 0)
        };
        if self
            .atlas
            .as_ref()
            .is_some_and(|atlas| atlas.sprites.contains_key(&key))
        {
            self.canvas
                .rect(x, h - 72 * s, 56 * s, 57 * s, crate::canvas::PANEL);
            if let Some(atlas) = &self.atlas {
                atlas.draw_scaled(
                    &mut self.canvas,
                    &key,
                    x + inset * s,
                    h - 72 * s + inset * s,
                    false,
                    PixelScale {
                        numerator: (s * times) as u16,
                        denominator: 1,
                    },
                );
            }
            self.canvas.rect(x, h - 72 * s, 2 * s, 57 * s, color);
            title_x += 62 * s;
        }
        let room = (x + w.min(320 * s)) - title_x;
        text(
            &mut self.canvas,
            &fit(e.kind.name(), room, s),
            title_x,
            h - 72 * s,
            WHITE,
            s,
        );
        small(
            &mut self.canvas,
            &fit_small(&format!("ENEMY / {}", role_tag(e.kind)), room, s),
            title_x,
            h - 60 * s,
            color,
            s,
        );
        let mut chips = vec![Chip {
            mark: Mark::Hull,
            value: format!("{}/{}", e.hp, e.max_hp.max(e.hp)),
            short: false,
        }];
        chips.extend(stat_chips(e.kind, false));
        self.draw_chips(&chips, title_x, h - 47 * s, false);
        let fact = wrap(key_fact(e.kind), (room / (6 * s)).max(1) as usize);
        for (i, line) in fact.iter().take(2).enumerate() {
            small(
                &mut self.canvas,
                line,
                title_x,
                h - (33 - i as i32 * 10) * s,
                MUTED,
                s,
            );
        }
    }

    /// The card for a button, or None when it has nothing to say.
    pub(crate) fn hover_card(&self, b: &Button) -> Option<HoverCard> {
        // The gauge opens the sluice card itself on hover; a tooltip would
        // cover it.
        if b.action == crate::game::Action::TideCard {
            return None;
        }
        let reason = self.action_reason(&b.action);
        let description = self.action_description(&b.action);
        if reason.is_none() && description.is_empty() {
            return None;
        }
        let mut card = HoverCard {
            title: b.label.clone(),
            key: crate::lean_hud::button_key(b),
            icon: crate::lean_hud::icon_key(&b.action, self.faction).filter(|key| {
                self.atlas
                    .as_ref()
                    .is_some_and(|atlas| atlas.sprites.contains_key(key))
            }),
            // "Need 70 more salvage: 10 on hand, 0 pending." reads as the
            // shortfall alone; the price row already shows it in red.
            reason: reason.map(|r| match r.split_once(": ") {
                Some((need, _)) if need.starts_with("Need") => format!("{need}."),
                _ => r,
            }),
            ..Default::default()
        };
        let player = &self.world.players[0];
        let lacks = |mark: Mark, amount: u32| match mark {
            Mark::Salvage => player.salvage < amount,
            Mark::Pressure => player.pressure < amount,
            _ => false,
        };
        match &b.action {
            Action::Train(k) | Action::TrainBatch(k) | Action::Build(k) => {
                let k = *k;
                let s = spec(k);
                card.title = crate::ux::building_name(k, self.faction).to_string();
                card.tag = Some(role_tag(k).into());
                card.costs.push(Chip {
                    mark: Mark::Salvage,
                    value: s.salvage.to_string(),
                    short: lacks(Mark::Salvage, s.salvage),
                });
                if s.pressure > 0 {
                    card.costs.push(Chip {
                        mark: Mark::Pressure,
                        value: s.pressure.to_string(),
                        short: lacks(Mark::Pressure, s.pressure),
                    });
                }
                if s.crew > 0 {
                    card.costs.push(Chip {
                        mark: Mark::Crew,
                        value: s.crew.to_string(),
                        short: player.crew + s.crew > self.world.crew_cap_for(0),
                    });
                }
                card.costs.push(Chip {
                    mark: Mark::Time,
                    value: format!("{}S", s.build_ticks.div_ceil(30)),
                    short: false,
                });
                card.stats = stat_chips(k, true);
                card.body = wrap(key_fact(k), LINE_CHARS);
                let (strong, _) = crate::ux::matchups(k);
                if !strong.is_empty() {
                    card.good_vs = Some(strong.to_string());
                }
            }
            Action::FilterSelection(k, remove) => {
                card.title = k.name().to_string();
                card.tag = Some(role_tag(*k).into());
                card.body = vec![if *remove {
                    "Click removes them from the selection.".into()
                } else {
                    "Click keeps only these.".into()
                }];
                if !*remove {
                    card.note = vec!["Shift-click removes them.".into()];
                }
            }
            Action::Upgrade(u) => {
                let (salvage, pressure, ticks) = u.cost();
                card.title = u.name().to_string();
                card.tag = Some(format!(
                    "UPGRADE / {}",
                    crate::ux::building_name(u.building(), self.faction)
                ));
                card.costs = vec![
                    Chip {
                        mark: Mark::Salvage,
                        value: salvage.to_string(),
                        short: lacks(Mark::Salvage, salvage),
                    },
                    Chip {
                        mark: Mark::Pressure,
                        value: pressure.to_string(),
                        short: lacks(Mark::Pressure, pressure),
                    },
                    Chip {
                        mark: Mark::Time,
                        value: format!("{}S", ticks / 30),
                        short: false,
                    },
                ];
                // OVERHAUL costs salvage only (rules 21): no empty chip.
                card.costs
                    .retain(|c| !(c.mark == Mark::Pressure && c.value == "0"));
                card.body = wrap(
                    &crate::ux::upgrade_description(*u, self.faction),
                    LINE_CHARS,
                );
            }
            _ => {
                // Everything else speaks in paragraphs: a price row becomes
                // marks, the first paragraph is the line that matters, the
                // rest is small print.
                let mut paragraphs: Vec<&str> =
                    description.split('\n').filter(|p| !p.is_empty()).collect();
                if let Some(first) = paragraphs.first()
                    && let Some(chips) = price_chips(first)
                {
                    card.costs = chips
                        .into_iter()
                        .map(|mut c| {
                            if let Ok(n) = c.value.parse::<u32>() {
                                c.short = lacks(c.mark, n);
                            }
                            c
                        })
                        .collect();
                    paragraphs.remove(0);
                }
                if let Action::Research(d) = &b.action {
                    let tier2 = self.world.players[0].doctrine == Some(*d)
                        && self.world.players[0].doctrine_tier >= 1;
                    card.tag = Some("DOCTRINE".into());
                    card.title = format!(
                        "{}{}",
                        crate::tactics::doctrine_name(*d),
                        if tier2 { " II" } else { "" }
                    );
                }
                // One sentence of what it does, one of small print: the
                // first sentence of each of the first two paragraphs,
                // unless the order has its own short words.
                let (body, note) = match short_words(&b.action) {
                    Some((body, note)) => (Some(body.to_string()), note.map(str::to_string)),
                    None => (
                        paragraphs.first().map(|p| first_sentence(p)),
                        paragraphs.get(1).map(|p| first_sentence(p)),
                    ),
                };
                if let Action::Research(d) = &b.action {
                    let tier2 = self.world.players[0].doctrine == Some(*d)
                        && self.world.players[0].doctrine_tier == 1;
                    // The pause is part of the price: trial 10's Compact
                    // read it only in the prompt after pressing.
                    let (benefit, ticks) = if tier2 {
                        (
                            crate::tactics::doctrine_tier2_benefit(*d),
                            bw_content::DOCTRINE_TIER2_TICKS,
                        )
                    } else {
                        (
                            crate::tactics::doctrine_benefit(*d),
                            bw_content::DOCTRINE_TICKS,
                        )
                    };
                    card.body = wrap(benefit, LINE_CHARS);
                    card.body.extend(wrap(
                        &format!("Pauses HQ worker training {}s.", ticks / 30),
                        LINE_CHARS,
                    ));
                    if !tier2 {
                        card.note = wrap(
                            &format!("Tier II: {}", crate::tactics::doctrine_tier2_benefit(*d)),
                            LINE_CHARS,
                        );
                    }
                } else if b.action == Action::Surge {
                    // The price before the press: 10 a machine, and how
                    // many that is now (trial 10: one press emptied the bar).
                    card.body = wrap(&self.surge_words(), LINE_CHARS);
                    card.note = wrap("+50% speed for 3s, weapons off. 12s cooldown.", LINE_CHARS);
                } else {
                    card.body = body.map(|b| wrap(&b, LINE_CHARS)).unwrap_or_default();
                    card.note = note.map(|n| wrap(&n, LINE_CHARS)).unwrap_or_default();
                }
                if b.action == Action::Surge {
                    card.costs = vec![
                        Chip {
                            mark: Mark::Pressure,
                            value: self.surge_selection_cost().to_string(),
                            short: lacks(Mark::Pressure, self.surge_selection_cost()),
                        },
                        Chip {
                            mark: Mark::Time,
                            value: "3S".into(),
                            short: false,
                        },
                    ];
                }
            }
        }
        crate::trial11_words::refine_card(self, &b.action, &mut card);
        // The sluice card's hold row and lane chips say what they show and
        // that a click looks (trial 12).
        if let Some((title, body)) = crate::trial12_hold::hold_card_words(self, b) {
            card.title = title;
            card.body = wrap(&body, LINE_CHARS);
            card.note.clear();
        }
        // A reason leads; the prose keeps what room is left.
        let reason_lines = card
            .reason
            .as_deref()
            .map_or(0, |r| wrap(r, LINE_CHARS).len());
        let room = MAX_LINES.saturating_sub(reason_lines);
        card.body.truncate(room);
        let room = room.saturating_sub(card.body.len());
        let good = usize::from(card.good_vs.is_some());
        card.note.truncate(room.saturating_sub(good));
        Some(card)
    }

    /// The card's height in interface rows.
    pub(crate) fn hover_card_rows(card: &HoverCard) -> i32 {
        let head = if card.icon.is_some() { 30 } else { 20 };
        let reason = card
            .reason
            .as_deref()
            .map_or(0, |r| wrap(r, LINE_CHARS).len()) as i32;
        let marks = [!card.costs.is_empty(), !card.stats.is_empty()]
            .iter()
            .filter(|b| **b)
            .count() as i32;
        let good =
            card.good_vs
                .as_deref()
                .map_or(0, |g| wrap(&format!("GOOD VS {g}"), LINE_CHARS).len()) as i32;
        let prose = reason + card.body.len() as i32 + good + card.note.len() as i32;
        head + 4 + marks * (ROW + 2) + prose * ROW + if prose > 0 { 3 } else { 0 } + PAD - 2
    }

    /// A row of marks and their numbers from `x`, faint for stats and
    /// red where the player is short. Returns where the row ends.
    pub(crate) fn draw_chips(&mut self, row: &[Chip], x: i32, y: i32, faint: bool) -> i32 {
        let s = self.ui_scale();
        let scale = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        let mut cx = x;
        for chip in row {
            if let Some(atlas) = &self.atlas {
                atlas.draw_scaled(&mut self.canvas, chip.mark.sprite(), cx, y, faint, scale);
            }
            let color = if chip.short {
                RED
            } else if faint {
                MUTED
            } else {
                WHITE
            };
            small(&mut self.canvas, &chip.value, cx + 10 * s, y + s, color, s);
            cx += (10 + chip.value.chars().count() as i32 * 6 + 7) * s;
        }
        cx
    }

    pub(crate) fn draw_hover_card(&mut self, r: Rect, hover: &HoverCard) {
        let s = self.ui_scale();
        card(
            &mut self.canvas,
            r,
            hover
                .accent
                .unwrap_or(if hover.reason.is_some() { GOLD } else { JADE }),
        );
        let scale = PixelScale {
            numerator: s as u16,
            denominator: 1,
        };
        let x = r.x + PAD * s;
        let mut title_x = x;
        let mut y = r.y + PAD * s;
        // The head: the button's own picture, the name, the key, the role.
        if let Some(icon) = &hover.icon
            && let Some(atlas) = &self.atlas
            && atlas.draw_scaled(&mut self.canvas, icon, x, r.y + 5 * s, false, scale)
        {
            title_x = x + 28 * s;
        }
        let key_w = hover
            .key
            .as_deref()
            .map_or(0, |k| k.chars().count() as i32 * 6 + 5);
        let title_room = r.x + r.w - PAD * s - key_w * s - title_x - 2 * s;
        // Without a role under it, the name sits in the middle of the head.
        let title_dy = match (hover.icon.is_some(), hover.tag.is_some()) {
            (true, false) => 7,
            (true, true) => 1,
            _ => 0,
        };
        text(
            &mut self.canvas,
            &fit(&hover.title, title_room, s),
            title_x,
            y + title_dy * s,
            WHITE,
            s,
        );
        if let Some(key) = &hover.key {
            let kx = r.x + r.w - PAD * s - key_w * s;
            self.canvas.rect(kx, y - s, key_w * s, 11 * s, EDGE);
            small(&mut self.canvas, key, kx + 3 * s, y + s, GOLD, s);
        }
        if let Some(tag) = &hover.tag {
            small(
                &mut self.canvas,
                &fit_small(tag, r.x + r.w - PAD * s - title_x, s),
                title_x,
                y + 13 * s,
                JADE,
                s,
            );
        }
        y = r.y + if hover.icon.is_some() { 32 } else { 22 } * s;
        self.canvas.rect(x, y - 3 * s, r.w - 2 * PAD * s, s, EDGE);
        // The numbers: price first, then what the machine brings.
        for (row, faint) in [(&hover.costs, false), (&hover.stats, true)] {
            if row.is_empty() {
                continue;
            }
            self.draw_chips(row, x, y, faint);
            y += (ROW + 2) * s;
        }
        y += 2 * s;
        if let Some(reason) = &hover.reason {
            for line in wrap(reason, LINE_CHARS) {
                small(&mut self.canvas, &line, x, y, GOLD, s);
                y += ROW * s;
            }
        }
        for line in &hover.body {
            small(&mut self.canvas, line, x, y, WHITE, s);
            y += ROW * s;
        }
        if let Some(good) = &hover.good_vs {
            for (i, line) in wrap(&format!("GOOD VS {good}"), LINE_CHARS)
                .iter()
                .enumerate()
            {
                if i == 0 {
                    small(&mut self.canvas, "GOOD VS", x, y, JADE, s);
                    let rest = line.strip_prefix("GOOD VS").unwrap_or(line);
                    small(&mut self.canvas, rest, x + 42 * s, y, WHITE, s);
                } else {
                    small(&mut self.canvas, line, x, y, WHITE, s);
                }
                y += ROW * s;
            }
        }
        for line in &hover.note {
            small(&mut self.canvas, line, x, y, MUTED, s);
            y += ROW * s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_rows_become_marks_and_prose_stays_prose() {
        let chips = price_chips("120 salvage, 60 pressure, 30s").unwrap();
        let marks: Vec<Mark> = chips.iter().map(|c| c.mark).collect();
        assert_eq!(marks, [Mark::Salvage, Mark::Pressure, Mark::Time]);
        assert_eq!(chips[2].value, "30S");
        assert!(price_chips("40 pressure, 10s warning").is_some());
        assert!(price_chips("Hold position and fire at enemies in range.").is_none());
        assert!(price_chips("Spend 40 pressure: +50% speed").is_none());
    }

    #[test]
    fn every_role_line_fits_one_card_line() {
        for kind in [
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
        ] {
            assert!(
                key_fact(kind).len() <= LINE_CHARS,
                "{kind:?}: {}",
                key_fact(kind)
            );
            assert!(role_tag(kind).len() <= 20, "{kind:?}");
        }
    }
}
