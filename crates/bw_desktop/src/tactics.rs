//! Tactical controls and readable, fog-filtered overlays. Rules live in bw_sim.
use crate::canvas::{Color, GOLD, INK, JADE, MUTED, RED};
use crate::game::{Action, Button, Game, Mode, Screen};
use bw_content::{
    DOCTRINE_PRESSURE, DOCTRINE_SALVAGE, Doctrine, LOOM_BLAST_RADIUS, SURGE_PRESSURE, spec,
};
use bw_core::{FP, Kind, Pos};
use bw_sim::{Command, Entity, Formation};

struct TacticalUnit {
    id: u32,
    owner: u8,
    kind: Kind,
    pos: Pos,
    facing: u8,
    deployed: bool,
    surge_remaining: u32,
}

pub fn formation_name(formation: Formation) -> &'static str {
    match formation {
        // Not "COMPACT": a side's name (trial 11).
        Formation::Compact => "TIGHT",
        Formation::Line => "LINE",
        Formation::Loose => "LOOSE",
    }
}
pub fn doctrine_name(doctrine: Doctrine) -> &'static str {
    match doctrine {
        Doctrine::Hauling => "HAULING",
        Doctrine::FireControl => "FIRE CONTROL",
    }
}
pub fn doctrine_benefit(doctrine: Doctrine) -> &'static str {
    match doctrine {
        Doctrine::Hauling => "Workers carry 8 instead of 5.",
        Doctrine::FireControl => "Combat machines fire 15% sooner.",
    }
}
pub fn doctrine_tier2_benefit(doctrine: Doctrine) -> &'static str {
    match doctrine {
        Doctrine::Hauling => "Workers carry 3 more and repair twice as fast.",
        Doctrine::FireControl => "Defence nests reach 50% farther.",
    }
}

impl Game {
    fn selected_combat(&self) -> Vec<&Entity> {
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && !e.kind.is_worker()
                    && !e.kind.is_building()
                    && self.selected.contains(&e.id)
            })
            .collect()
    }
    pub(crate) fn surge_selection_cost(&self) -> u32 {
        self.surge_able_count() as u32 * SURGE_PRESSURE
    }
    /// The selected machines a Surge would move: packed gun machines off
    /// cooldown.  The others are skipped (rules 14).
    pub(crate) fn surge_able_count(&self) -> usize {
        self.selected_combat()
            .iter()
            .filter(|e| {
                spec(e.kind).damage > 0
                    && !e.deployed
                    && e.deploy_remaining == 0
                    && e.surge_remaining == 0
                    && e.surge_cooldown == 0
            })
            .count()
    }
    pub(crate) fn tactical_action_reason(&self, action: &Action) -> Option<String> {
        let player = &self.world.players[0];
        match action {
            Action::Research(_) | Action::CancelResearch => {
                let hq = self
                    .first_building()
                    .and_then(|id| self.world.entities.iter().find(|e| e.id == id));
                let Some(hq) =
                    hq.filter(|e| e.kind == Kind::Headquarters && e.build_remaining == 0)
                else {
                    return Some("Select your completed headquarters for doctrines.".into());
                };
                if matches!(action, Action::CancelResearch) {
                    // K at the headquarters also cancels OVERHAUL (rules 21).
                    let researching = player
                        .research
                        .as_ref()
                        .is_some_and(|r| r.building == hq.id);
                    return (!researching && hq.upgrade.is_none())
                        .then(|| "No headquarters research to cancel.".into());
                }
                if let Some(research) = &player.research {
                    return Some(format!(
                        "{} researching: {}s remaining.",
                        doctrine_name(research.doctrine),
                        research.remaining.div_ceil(30)
                    ));
                }
                let Action::Research(asked) = action else {
                    return None;
                };
                let (salvage, pressure) = match (player.doctrine, player.doctrine_tier) {
                    (None, _) => (DOCTRINE_SALVAGE, DOCTRINE_PRESSURE),
                    (Some(chosen), 1) if chosen == *asked => (
                        bw_content::DOCTRINE_TIER2_SALVAGE,
                        bw_content::DOCTRINE_TIER2_PRESSURE,
                    ),
                    // The other doctrine says why it is shut: the choice,
                    // not the chosen one's progress (trial 10 showed FIRE
                    // CONTROL greyed with "HAULING IS COMPLETE").
                    (Some(chosen), _) if chosen != *asked => {
                        return Some(format!(
                            "{} chosen: the doctrines exclude each other.",
                            doctrine_name(chosen)
                        ));
                    }
                    (Some(chosen), _) => {
                        return Some(format!(
                            "{} is complete at both tiers.",
                            doctrine_name(chosen)
                        ));
                    }
                };
                let s = salvage.saturating_sub(player.salvage);
                let p = pressure.saturating_sub(player.pressure);
                match (s, p) {
                    (0, 0) => {}
                    (s, 0) => return Some(format!("Need {s} more salvage.")),
                    (0, p) => return Some(format!("Need {p} more pressure.")),
                    (s, p) => return Some(format!("Need {s} salvage and {p} pressure.")),
                }
            }
            Action::Formation | Action::Face | Action::Surge => {
                let combat = self.selected_combat();
                if combat.is_empty() {
                    return Some("Select combat machines for this order.".into());
                }
                if matches!(action, Action::Formation) {
                    return None;
                }
                if matches!(action, Action::Face)
                    && combat.iter().any(|e| e.deployed || e.deploy_remaining > 0)
                {
                    return Some("Pack your specialists and wait for packing to finish.".into());
                }
                if matches!(action, Action::Surge) {
                    // The able machines surge and the rest are skipped.
                    let able = self.surge_able_count();
                    if able == 0 {
                        if let Some(e) = combat.iter().find(|e| e.surge_cooldown > 0) {
                            return Some(format!(
                                "{} Surge ready in {}s.",
                                e.kind.name(),
                                e.surge_cooldown.div_ceil(30)
                            ));
                        }
                        return Some(
                            "No machine here can surge: it needs a packed gun machine.".into(),
                        );
                    }
                    let cost = able as u32 * SURGE_PRESSURE;
                    if cost > player.pressure {
                        return Some(format!(
                            "Surge costs {cost} pressure. Need {} more.",
                            cost - player.pressure
                        ));
                    }
                }
            }
            Action::Deploy => {
                let blocked = self
                    .selected_combat()
                    .iter()
                    .any(|e| e.surge_remaining > 0 && matches!(e.kind, Kind::Bulwark | Kind::Loom));
                if blocked {
                    return Some("Surge is active. Wait before deploying.".into());
                }
            }
            _ => {}
        }
        None
    }
    pub(crate) fn tactical_action(&mut self, action: Action) {
        let units = self.gameplay_ids(|k| !k.is_worker() && !k.is_building());
        match action {
            Action::Face => {
                self.mode = Mode::Face;
                self.message.clear();
            }
            Action::Formation => {
                let first = self
                    .selected_combat()
                    .first()
                    .map(|e| e.formation)
                    .unwrap_or_default();
                let formation = match first {
                    Formation::Compact => Formation::Line,
                    Formation::Line => Formation::Loose,
                    Formation::Loose => Formation::Compact,
                };
                self.issue(Command::SetFormation { units, formation });
            }
            Action::Surge => self.issue(Command::Surge { units }),
            Action::Research(doctrine) => {
                if let Some(building) = self.first_building() {
                    self.issue(Command::Research { building, doctrine });
                }
            }
            Action::CancelResearch => {
                if let Some(building) = self.first_building() {
                    let researching = self.world.players[0]
                        .research
                        .as_ref()
                        .is_some_and(|r| r.building == building);
                    let upgrading = self
                        .world
                        .entities
                        .iter()
                        .any(|e| e.id == building && e.upgrade.is_some());
                    if !researching && upgrading {
                        // The headquarters' OVERHAUL, the last level first.
                        self.issue(Command::CancelUpgrade { building });
                    } else {
                        self.issue(Command::CancelResearch { building });
                    }
                }
            }
            _ => {}
        }
    }
    pub(crate) fn draw_research_buttons(&mut self) {
        let player = &self.world.players[0];
        // The button carries its own state: seconds left while it researches,
        // CHOSEN once it is the match's doctrine, LOCKED when the other is.
        let tier = player.doctrine_tier;
        let chosen = player.doctrine;
        let state = |doctrine: Doctrine| -> String {
            if let Some(research) = &player.research {
                if research.doctrine == doctrine {
                    return format!("{}S LEFT", research.remaining.div_ceil(30));
                }
                return "LOCKED".into();
            }
            match (chosen, tier) {
                (Some(c), 1) if c == doctrine => "TIER II".into(),
                (Some(c), _) if c == doctrine => "COMPLETE".into(),
                (Some(_), _) => "LOCKED".into(),
                (None, _) => "PAUSES HQ".into(),
            }
        };
        let rows: Vec<(usize, String, Doctrine, String)> = [
            (2, "U HAULING", Doctrine::Hauling),
            (3, "J FIRE CTRL", Doctrine::FireControl),
        ]
        .into_iter()
        .map(|(index, label, doctrine)| {
            let second = chosen == Some(doctrine) && tier == 1;
            let cost = if second {
                crate::ux::price(
                    bw_content::DOCTRINE_TIER2_SALVAGE,
                    bw_content::DOCTRINE_TIER2_PRESSURE,
                )
            } else {
                crate::ux::price(DOCTRINE_SALVAGE, DOCTRINE_PRESSURE)
            };
            // The state row says TIER II; the name stays whole on the card.
            let label = label.to_string();
            (
                index,
                label,
                doctrine,
                format!("{cost}\n{}", state(doctrine)),
            )
        })
        .collect();
        for (index, label, doctrine, hint) in rows {
            self.command_button(index, &label, &hint, Action::Research(doctrine), true);
        }
        self.command_button(4, "CANCEL R&D", "K 75% BACK", Action::CancelResearch, true);
    }
    pub(crate) fn tactical_selection_status(&self, entities: &[&Entity]) -> Option<String> {
        let entity = entities.first()?;
        if entities.len() == 1 && entity.kind == Kind::Headquarters {
            let player = &self.world.players[0];
            if let Some(research) = &player.research {
                return Some(format!(
                    "{} {}S / HQ TRAINING PAUSED",
                    doctrine_name(research.doctrine),
                    research.remaining.div_ceil(30)
                ));
            }
            return player.doctrine.map(|d| {
                format!(
                    "{}: {}",
                    doctrine_name(d),
                    match d {
                        Doctrine::Hauling => "8 CARGO",
                        Doctrine::FireControl => "85% COOLDOWN",
                    }
                )
            });
        }
        if entities
            .iter()
            .all(|e| !e.kind.is_worker() && !e.kind.is_building())
        {
            let active = entities
                .iter()
                .map(|e| e.surge_remaining)
                .max()
                .unwrap_or(0);
            let cooldown = entities.iter().map(|e| e.surge_cooldown).max().unwrap_or(0);
            if active > 0 {
                return Some(format!("SURGE {}S / WEAPONS OFF", active.div_ceil(30)));
            }
            if entities.len() == 1 {
                if entity.deploy_remaining > 0 {
                    return Some(format!(
                        "{} {}S",
                        if entity.deploy_target {
                            "DEPLOYING"
                        } else {
                            "PACKING"
                        },
                        entity.deploy_remaining.div_ceil(30)
                    ));
                }
                if entity.kind == Kind::Loom {
                    return Some(
                        if entity.deployed {
                            "ARTILLERY 2-8 CELLS / D PACK"
                        } else {
                            "PACKED / D DEPLOY TO FIRE"
                        }
                        .into(),
                    );
                }
                if entity.kind == Kind::Bulwark && entity.deployed {
                    return Some("FRONT SHIELD LOCKED / D PACK".into());
                }
            }
            if cooldown > 0 {
                return Some(format!(
                    "SURGE READY IN {}S / {} UNITS",
                    cooldown.div_ceil(30),
                    entities.len()
                ));
            }
            // Only a gun makes Surge worth naming.
            if entities.iter().any(|e| bw_content::spec(e.kind).damage > 0) {
                return Some(format!(
                    "{} / Z SURGE {}P",
                    self.selection_formation_label(),
                    self.surge_selection_cost()
                ));
            }
            return None;
        }
        None
    }
    pub(crate) fn selection_formation_label(&self) -> &'static str {
        let units = self.selected_combat();
        let Some(first) = units.first() else {
            return formation_name(Formation::Compact);
        };
        if units.iter().any(|e| e.formation != first.formation) {
            "MIXED"
        } else {
            formation_name(first.formation)
        }
    }
    pub(crate) fn draw_tactical_controls(&mut self) {
        if self.screen != Screen::Match {
            return;
        }
        if !self.selected_combat().is_empty() {
            let cooldown = self
                .selected_combat()
                .iter()
                .map(|e| e.surge_cooldown)
                .max()
                .unwrap_or(0);
            let detail = if cooldown > 0 {
                format!(
                    "F {} / SURGE {}S",
                    self.selection_formation_label(),
                    cooldown.div_ceil(30)
                )
            } else {
                format!(
                    "F {} / Z SURGE {}P",
                    self.selection_formation_label(),
                    self.surge_selection_cost()
                )
            };
            self.canvas.text(&detail, 201, 334, MUTED);
            for (x, w, label, action) in [
                (
                    228,
                    112,
                    format!("F {}", self.selection_formation_label()),
                    Action::Formation,
                ),
                (346, 88, "R FACE".into(), Action::Face),
            ] {
                let enabled = self.action_reason(&action).is_none();
                self.buttons.push(Button {
                    x,
                    y: 28,
                    w,
                    h: 20,
                    label,
                    hint: String::new(),
                    action,
                    enabled,
                });
            }
        }
        if let Some(entity) = self
            .selected
            .first()
            .and_then(|id| self.world.entities.iter().find(|e| e.id == *id))
            && self.selected.len() == 1
            && entity.kind == Kind::Headquarters
            && let Some(research) = &self.world.players[0].research
        {
            self.canvas.text(
                &format!("{} WORKERS WAIT / HQ PAUSED", entity.queue.len()),
                201,
                333,
                MUTED,
            );
            self.canvas.rect(201, 343, 203, 3, INK);
            let complete = bw_content::DOCTRINE_TICKS.saturating_sub(research.remaining);
            self.canvas.rect(
                201,
                343,
                (203 * complete / bw_content::DOCTRINE_TICKS) as i32,
                3,
                GOLD,
            );
        }
    }
    pub(crate) fn draw_tactical_overlays(&mut self) {
        // Geometry is a tactical UI overlay. Existing Aseprite artwork is unchanged.
        let entities: Vec<_> = self
            .world
            .entities
            .iter()
            .filter(|e| self.world.entity_visible(0, e.id))
            .map(|e| TacticalUnit {
                id: e.id,
                owner: e.owner,
                kind: e.kind,
                pos: e.pos,
                facing: e.facing,
                deployed: e.deployed,
                surge_remaining: e.surge_remaining,
            })
            .collect();
        for entity in entities {
            if entity.surge_remaining > 0 {
                // The pressure family owns the physical Surge read now. Keep
                // the two jade strokes only for an older atlas bundle so a
                // missing optional asset never makes the state unreadable.
                if !self.pressure_art_present(&entity) {
                    let (x, y) = self.camera.project(entity.pos);
                    self.canvas.line(x - 11, y + 3, x - 6, y - 2, JADE);
                    self.canvas.line(x + 6, y + 3, x + 11, y - 2, JADE);
                }
            }
            if !self.selected.contains(&entity.id) || entity.owner != 0 {
                continue;
            }
            if matches!(entity.kind, Kind::Bulwark | Kind::Loom) {
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
                let (dx, dy) = DIR[usize::from(entity.facing.min(7))];
                let (x, y) = self.camera.project(entity.pos);
                let ahead = Pos {
                    x: entity.pos.x + dx * FP * 2,
                    y: entity.pos.y + dy * FP * 2,
                };
                let (ax, ay) = self.camera.project(ahead);
                self.canvas
                    .line(x, y, ax, ay, if entity.deployed { GOLD } else { MUTED });
                self.canvas.diamond(ax, ay, 3, 2, GOLD);
                if entity.kind == Kind::Bulwark && entity.deployed {
                    for d in [(entity.facing + 2) % 8, (entity.facing + 6) % 8] {
                        let (dx, dy) = DIR[usize::from(d)];
                        let (bx, by) = self.camera.project(Pos {
                            x: entity.pos.x + dx * FP * 2,
                            y: entity.pos.y + dy * FP * 2,
                        });
                        self.canvas.line(bx, by, ax, ay, GOLD);
                    }
                }
                // The Loom's reach and blind zone are drawn with every
                // gun's in `draw_reach_rings` (trial 12): gold, never red.
            }
        }
        // An enemy gun at a mouth while the sluice is ours is what keeps the
        // lane from holding: a red bracket over each one, so the blocker is
        // the thing to hit.
        if self.world.gate.owner == Some(0) {
            let radius = i64::from(FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
            // Only the mouths of our own lanes: on the Confluence a gun at
            // the far arm between the two opponents blocks nothing of ours.
            let layout = self.world.map.layout();
            let own_mouths: Vec<Pos> = self
                .world
                .arms_of(0)
                .into_iter()
                .flat_map(|arm| layout.arm_mouths(arm))
                .collect();
            let blockers: Vec<Pos> = self
                .world
                .entities
                .iter()
                .filter(|e| {
                    e.owner != 0
                        && e.hp > 0
                        && e.build_remaining == 0
                        && e.aboard.is_none()
                        && crate::qol::lane_holder(e.kind)
                        && self.world.entity_visible(0, e.id)
                        && own_mouths
                            .iter()
                            .any(|mouth| e.pos.distance_sq(*mouth) <= radius)
                })
                .map(|e| e.pos)
                .collect();
            let blink = (self.world.tick / 10).is_multiple_of(2);
            for pos in blockers {
                let (x, y) = self.camera.project(pos);
                // A downward chevron clear of the tallest machine, and
                // corner brackets around it; the words under the pointer.
                self.field_tips.push(
                    crate::field_labels::FieldTip::new(
                        crate::field_labels::Area::around((x, y), 16, 44, 16, 6),
                        (x, y - 44),
                        "BLOCKS HOLD",
                        RED,
                    )
                    .line("Kill it and the lane holds."),
                );
                let top = y - 42;
                for dy in 0..2 {
                    self.canvas.line(x - 6, top + dy, x, top + 6 + dy, RED);
                    self.canvas.line(x + 6, top + dy, x, top + 6 + dy, RED);
                }
                if blink {
                    for (sx, sy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let (cx, cy) = (x + sx * 12, y - 14 + sy * 16);
                        self.canvas.line(cx, cy, cx - sx * 4, cy, RED);
                        self.canvas.line(cx, cy, cx, cy - sy * 4, RED);
                    }
                }
            }
        }
        // A capture under way: the ring a defender must stand inside to
        // stop it, in the capturer's colour.
        if self.world.gate.capture_progress > 0
            && let Some(capturer) = self.world.gate.capture_player
        {
            let color = crate::seats::seat_colour(&self.world, capturer);
            let gate = self.world.map.gate_pos;
            self.ground_ring_styled(
                gate,
                bw_content::CAPTURE_CONTEST_RADIUS_CELLS * FP,
                color,
                false,
            );
        }
        // Short on pressure with no condenser: the free wells breathe, so
        // the fix is on the ground rather than in a sentence.
        let player = &self.world.players[0];
        let has_condenser = self
            .world
            .entities
            .iter()
            .any(|e| e.owner == 0 && e.hp > 0 && e.kind == Kind::Condenser);
        // Not in the opening, when no side has one yet.
        if !has_condenser
            && player.pressure < 50
            && self.world.tick >= 3 * 60 * 30
            && (self.world.tick / 15).is_multiple_of(2)
        {
            let wells: Vec<Pos> = self
                .world
                .map
                .wells
                .iter()
                .copied()
                .filter(|w| {
                    !self.world.entities.iter().any(|e| {
                        e.hp > 0
                            && e.kind == Kind::Condenser
                            && e.pos.distance_sq(*w) <= i64::from(FP).pow(2)
                    })
                })
                .collect();
            for well in wells {
                self.ground_ring_styled(well, FP * 2, JADE, false);
            }
        }
        // The crossing mouths of the tide rule stand on the ground: a ring
        // the size of the hold and a name.  Gold while yours, red while an
        // enemy holds one in sight, ink otherwise.
        let mouths = self.world.crossing_mouths();
        for (index, bank, point) in mouths
            .iter()
            .enumerate()
            .flat_map(|(lane, mouths)| mouths.iter().enumerate().map(move |(b, m)| (lane, b, m)))
        {
            // Only the station's owner holds a lane: the ring wears its
            // colour, gold while it is yours.
            let holder = crate::qol::lane_holder_seat(&self.world, index);
            let ours = holder == Some(0);
            let theirs = holder.is_some_and(|seat| seat != 0);
            let (own_present, enemy_mouth) = crate::qol::lane_presence(&self.world, index);
            let contested = own_present && enemy_mouth.is_some();
            let color = if contested {
                crate::canvas::WHITE
            } else if let Some(seat) = holder {
                crate::qol::hold_color(&self.world, seat)
            } else {
                MUTED
            };
            self.ground_ring_styled(
                *point,
                bw_content::CROSSING_HOLD_RADIUS_CELLS * FP,
                color,
                !ours && !theirs && !contested,
            );
            let (x, y) = self.camera.project(*point);
            // The ring says whose mouth it is; a held mouth adds its count,
            // the one number to read at a glance.  The name and the rest
            // are under the pointer.
            let gauge = [
                self.world.lane_hold[0],
                holder
                    .filter(|&seat| seat != 0)
                    .map_or(self.world.hold_pair()[1], |seat| {
                        self.world.lane_hold[usize::from(seat)]
                    }),
            ];
            if let Some(count) = hold_count(ours, theirs, contested, gauge, self.world.hold_ticks())
            {
                crate::field_labels::draw_badge(&mut self.canvas, &count, x, y + 6, color);
            }
            let reach = bw_content::CROSSING_HOLD_RADIUS_CELLS * FP;
            let (rx, _) = self.camera.project(Pos {
                x: point.x + reach,
                y: point.y - reach,
            });
            let (_, ry) = self.camera.project(Pos {
                x: point.x + reach,
                y: point.y + reach,
            });
            let (half_w, half_h) = ((rx - x).abs().max(24), (ry - y).abs().max(12));
            let label = if self.world.seat_count() > 2 {
                seat_crossing_label(&self.world, index, bank, holder, contested)
            } else {
                crossing_label(index, bank, ours, theirs, contested, gauge)
            };
            let mut tip = crate::field_labels::FieldTip::new(
                crate::field_labels::Area::around((x, y), half_w, half_h, half_w, half_h),
                (x, y - half_h / 2),
                label,
                color,
            )
            .icon("ui_cmd_capture")
            .tag("CROSSING MOUTH");
            if crate::lane_wall::lane_closed(&self.world, index) {
                tip = tip.warn(crate::lane_wall::CLOSED_TEXT);
            }
            self.field_tips.push(tip);
        }
        let shots: Vec<_> = self
            .world
            .artillery
            .iter()
            .filter(|s| s.owner == 0 || self.world.visible(0, s.target))
            .cloned()
            .collect();
        for shot in &shots {
            self.draw_shell_flight(shot);
        }
        for shot in shots {
            let color = if shot.owner == 0 {
                GOLD
            } else {
                crate::seats::seat_colour(&self.world, shot.owner)
            };
            if shot.owner != 0 && self.world.visible(0, shot.from) {
                // Where the shell came from: a broken line back to the Loom,
                // which the shot lights for us (rules 13).
                let (fx, fy) = self.camera.project(shot.from);
                let (tx, ty) = self.camera.project(shot.target);
                self.dashed_line(fx, fy - 6, tx, ty, RED);
                self.dashed_line(fx, fy - 5, tx, ty + 1, RED);
                self.field_tips.push(
                    crate::field_labels::FieldTip::new(
                        crate::field_labels::Area::around((fx, fy), 16, 36, 16, 8),
                        (fx, fy - 36),
                        "LOOM FIRE",
                        RED,
                    )
                    .icon("ui_unit_loom")
                    .line("Its shells come from here."),
                );
                // A Loom firing from off the screen: an arrow on the edge
                // points back along the shot to where it stands.
                let (w, h) = (self.canvas.width() as i32, self.canvas.height() as i32);
                let target_on = (0..w).contains(&tx) && (0..h).contains(&ty);
                if let Some((ex, ey)) =
                    screen_edge_point((tx, ty), (fx, fy), w, h).filter(|_| target_on)
                {
                    self.draw_edge_arrow(ex, ey, fx - tx, fy - ty);
                }
            }
            self.danger_ring(shot.target, LOOM_BLAST_RADIUS, color);
            let (x, y) = self.camera.project(shot.target);
            self.canvas.line(x - 4, y, x + 4, y, color);
            self.canvas.line(x, y - 3, x, y + 3, color);
            let tenths = (shot.impact_tick.saturating_sub(self.world.tick) * 10).div_ceil(30);
            // A fuse under the cross burns down to the impact: a pixel for
            // every tenth of a second, three seconds at most.
            let fuse = (tenths as i32).min(30);
            self.canvas.rect(x - 16, y + 9, 33, 4, INK);
            self.canvas
                .rect(x - fuse / 2, y + 10, fuse.max(1), 2, color);
            self.field_tips.push(crate::field_labels::FieldTip::new(
                crate::field_labels::Area::around((x, y), 18, 10, 18, 14),
                (x, y - 10),
                format!("SHELL {}.{}S", tenths / 10, tenths % 10),
                color,
            ));
        }
        if self.mode == Mode::Face && self.world_pointer_allowed(self.cursor.0, self.cursor.1) {
            let origins: Vec<_> = self.selected_combat().iter().map(|e| e.pos).collect();
            for pos in origins {
                let (x, y) = self.camera.project(pos);
                self.canvas.line(x, y, self.cursor.0, self.cursor.1, GOLD);
            }
        }
    }
    /// A filled red triangle whose tip is (x, y), pointing along (dx, dy).
    fn draw_edge_arrow(&mut self, x: i32, y: i32, dx: i32, dy: i32) {
        let len = f64::from(dx).hypot(f64::from(dy)).max(1.0);
        let (ux, uy) = (f64::from(dx) / len, f64::from(dy) / len);
        let (length, half) = (12.0, 6.0);
        for py in y - 13..=y + 13 {
            for px in x - 13..=x + 13 {
                let (rx, ry) = (f64::from(px - x), f64::from(py - y));
                // Distance back from the tip, and across the arrow's axis.
                let back = -(rx * ux + ry * uy);
                let across = (rx * uy - ry * ux).abs();
                if (0.0..=length).contains(&back) && across <= half * back / length + 0.5 {
                    self.canvas.rect(px, py, 1, 1, RED);
                }
            }
        }
    }
    /// A line of four-pixel dashes.
    fn dashed_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs()).max(1);
        let mut i = 0;
        while i < steps {
            let end = (i + 4).min(steps);
            self.canvas.line(
                x0 + dx * i / steps,
                y0 + dy * i / steps,
                x0 + dx * end / steps,
                y0 + dy * end / steps,
                color,
            );
            i += 7;
        }
    }
    /// A Loom shell in the air: it leaves the Loom partway through the
    /// windup and falls on the marked ground at impact, along a lobbed arc.
    /// The trail and the shell are drawn only over ground this seat sees,
    /// so an enemy shell never points back at a Loom under the fog.
    fn draw_shell_flight(&mut self, shot: &bw_sim::ArtilleryShot) {
        let Some(t) = shell_flight_fraction(shot.impact_tick, self.world.tick) else {
            return;
        };
        let color = if shot.owner == 0 {
            GOLD
        } else {
            crate::seats::seat_colour(&self.world, shot.owner)
        };
        let seen = |g: &Self, pos: Pos| shot.owner == 0 || g.world.visible(0, pos);
        let dx = i64::from(shot.target.x - shot.from.x);
        let dy = i64::from(shot.target.y - shot.from.y);
        let cells = ((dx * dx + dy * dy) as f64).sqrt() / f64::from(FP);
        let apex = (28.0 + cells * 4.0).min(80.0);
        let point = |f: f64| {
            let ground = Pos {
                x: shot.from.x + (dx as f64 * f) as i32,
                y: shot.from.y + (dy as f64 * f) as i32,
            };
            let lift = (4.0 * apex * f * (1.0 - f)) as i32;
            (ground, lift)
        };
        // The trail: dots every few pixels behind the shell, thinning out.
        const STEPS: i32 = 24;
        let reached = (t * f64::from(STEPS)) as i32;
        for step in (reached - 10).max(0)..reached {
            let (ground, lift) = point(f64::from(step) / f64::from(STEPS));
            if !seen(self, ground) {
                continue;
            }
            let (x, y) = self.camera.project(ground);
            // Older smoke is fainter.
            let alpha = (60 + (step - reached + 10) * 16).clamp(60, 210) as u8;
            self.canvas.rect(x, y - lift, 2, 2, [214, 206, 188, alpha]);
        }
        let (ground, lift) = point(t);
        if !seen(self, ground) {
            return;
        }
        let (x, y) = self.camera.project(ground);
        // Its shadow on the ground tightens as it falls.
        let spread = 1 + lift / 16;
        self.canvas
            .rect(x - spread, y, 2 * spread + 1, 1, [20, 24, 26, 110]);
        // The shell: a dark round with an ink rim and a lit top in the
        // firing side's colour.
        let sy = y - lift;
        self.canvas.rect(x - 2, sy - 1, 5, 3, INK);
        self.canvas.rect(x - 1, sy - 2, 3, 5, INK);
        self.canvas.rect(x - 1, sy - 1, 2, 2, color);
    }
    fn danger_ring(&mut self, pos: Pos, radius: i32, color: Color) {
        self.ground_ring_styled(pos, radius, color, false);
        // Short cardinal ticks make an actionable blast boundary read as a
        // hard warning at native scale while preserving the exact projected
        // radius used by the simulation's range constants.
        for (dx, dy) in [(radius, 0), (-radius, 0), (0, radius), (0, -radius)] {
            let (x, y) = self.camera.project(Pos {
                x: pos.x + dx,
                y: pos.y + dy,
            });
            self.canvas.line(x - 1, y, x + 1, y, color);
            self.canvas.line(x, y - 1, x, y + 1, color);
        }
    }
    fn ground_ring_styled(&mut self, pos: Pos, radius: i32, color: Color, broken: bool) {
        const CIRCLE: [(i32, i32); 16] = [
            (256, 0),
            (237, 98),
            (181, 181),
            (98, 237),
            (0, 256),
            (-98, 237),
            (-181, 181),
            (-237, 98),
            (-256, 0),
            (-237, -98),
            (-181, -181),
            (-98, -237),
            (0, -256),
            (98, -237),
            (181, -181),
            (237, -98),
        ];
        let mut points = Vec::with_capacity(16);
        for (dx, dy) in CIRCLE {
            points.push(self.camera.project(Pos {
                x: pos.x + radius * dx / 256,
                y: pos.y + radius * dy / 256,
            }));
        }
        for i in 0..16 {
            if broken && i % 2 == 1 {
                continue;
            }
            let (x, y) = points[i];
            let (xx, yy) = points[(i + 1) % 16];
            self.canvas.line(x, y, xx, yy, color);
        }
    }

    fn pressure_art_present(&self, entity: &TacticalUnit) -> bool {
        let role = match entity.kind {
            Kind::Riveter => "riveter",
            Kind::Bulwark => "bulwark",
            Kind::Sounder => "sounder",
            Kind::Skipper => "skipper",
            Kind::Reedguard => "reedguard",
            Kind::Loom => "loom",
            _ => return false,
        };
        let face = (9 - entity.facing % 8) % 8;
        let key = format!("pressure_{role}_{face}_surge_0");
        self.atlas
            .as_ref()
            .is_some_and(|atlas| atlas.sprites.contains_key(&key))
    }
}

/// The words on a crossing mouth's plate: who holds the lane, or that both
/// sides stand at it, and the seconds the running count has to go.
/// Ticks a Loom shell spends in the air: it leaves when the Loom's windup
/// animation fires (its fourth frame) and lands at the shot's impact tick.
pub(crate) const SHELL_FLIGHT_TICKS: u64 = 13;

/// How far along its flight a shell is, from 0 at launch to 1 at impact, or
/// `None` while it is still in the Loom or after it has landed.
pub(crate) fn shell_flight_fraction(impact_tick: u64, tick: u64) -> Option<f64> {
    let launch = impact_tick.checked_sub(SHELL_FLIGHT_TICKS)?;
    if tick < launch || tick >= impact_tick {
        return None;
    }
    Some((tick - launch) as f64 / SHELL_FLIGHT_TICKS as f64)
}

/// Where the line from `inside` toward an off-screen `outside` leaves the
/// picture, pulled 8 px in; None when `outside` is on the screen.
pub(crate) fn screen_edge_point(
    inside: (i32, i32),
    outside: (i32, i32),
    w: i32,
    h: i32,
) -> Option<(i32, i32)> {
    let margin = 8;
    let on = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h;
    if on(outside.0, outside.1) {
        return None;
    }
    let (x0, y0) = (f64::from(inside.0), f64::from(inside.1));
    let (dx, dy) = (
        f64::from(outside.0 - inside.0),
        f64::from(outside.1 - inside.1),
    );
    let (lo_x, hi_x) = (f64::from(margin), f64::from(w - margin));
    let (lo_y, hi_y) = (f64::from(margin), f64::from(h - margin));
    let mut t: f64 = 1.0;
    if dx > 0.0 {
        t = t.min((hi_x - x0) / dx);
    } else if dx < 0.0 {
        t = t.min((lo_x - x0) / dx);
    }
    if dy > 0.0 {
        t = t.min((hi_y - y0) / dy);
    } else if dy < 0.0 {
        t = t.min((lo_y - y0) / dy);
    }
    let t = t.max(0.0);
    Some(((x0 + dx * t).round() as i32, (y0 + dy * t).round() as i32))
}

/// The seconds left on a held lane's count, for the badge at its mouth.
pub(crate) fn hold_count(
    ours: bool,
    theirs: bool,
    contested: bool,
    gauge: [u32; 2],
    total: u32,
) -> Option<String> {
    let left = |ticks: u32| total.saturating_sub(ticks).div_ceil(30);
    if contested {
        None
    } else if ours {
        Some(left(gauge[0]).to_string())
    } else if theirs {
        Some(left(gauge[1]).to_string())
    } else {
        None
    }
}

/// A crossing mouth's name with three seats: its arm's letter and whose
/// bank it stands on ("E YOUR BANK", "S RED'S BANK"), then who holds the
/// lane and the seconds left on that seat's count.
pub(crate) fn seat_crossing_label(
    world: &bw_sim::World,
    lane: usize,
    mouth: usize,
    holder: Option<u8>,
    contested: bool,
) -> String {
    let bank = world.arm_banks(lane)[mouth.min(1)];
    let name = format!(
        "{} {} BANK",
        crate::seats::arm_letter(world, lane),
        crate::seats::seat_owner_word(world, bank)
    );
    let left = |ticks: u32| world.hold_ticks().saturating_sub(ticks).div_ceil(30);
    match holder {
        _ if contested => format!("{name}: CONTESTED"),
        Some(0) => format!("{name}: HELD {}S", left(world.lane_hold[0])),
        Some(seat) => format!(
            "{name}: {} {}S",
            crate::seats::seat_name(world, seat),
            left(world.lane_hold.get(usize::from(seat)).copied().unwrap_or(0))
        ),
        None => name,
    }
}

pub(crate) fn crossing_label(
    lane: usize,
    mouth: usize,
    ours: bool,
    theirs: bool,
    contested: bool,
    gauge: [u32; 2],
) -> String {
    // Each mouth is named by its lane and its bank: NW, NE, SW, SE.
    let name = format!(
        "{}{}",
        if lane == 0 { "N" } else { "S" },
        if mouth == 0 { "W" } else { "E" }
    );
    let left = |ticks: u32| {
        bw_content::TIDE_HOLD_TICKS
            .saturating_sub(ticks)
            .div_ceil(30)
    };
    if contested {
        format!("{name} CONTESTED")
    } else if ours {
        format!("{name} HELD {}S", left(gauge[0]))
    } else if theirs {
        format!("{name} ENEMY {}S", left(gauge[1]))
    } else {
        name
    }
}
