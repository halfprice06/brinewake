//! Controlled captures and regressions for the native-resolution presentation.
use crate::game::{Action, Game, Screen};
use crate::zoom::Zoom;
use bw_core::Faction;
#[cfg(test)]
use bw_core::{Kind, Pos};
use std::path::{Path, PathBuf};

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut g = Game::new_with_data_dir(base, dir.join("session"));
    g.resize_view(1920, 1080);
    for (screen, name) in [
        (Screen::Menu, "home"),
        (Screen::Setup, "setup"),
        (Screen::Settings, "settings"),
        (Screen::Help, "guide"),
    ] {
        g.screen = screen;
        g.screenshot(&dir.join(format!("{name}.png")))?;
    }
    g.ux.help_page = 4;
    g.screenshot(&dir.join("guide-combat.png"))?;
    let mut timings = Vec::new();
    for faction in [Faction::Union, Faction::Assembly] {
        g.faction = faction;
        g.start();
        g.world.ai_enabled = false;
        g.message.clear();
        g.cursor = (-999, -999);
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        for z in [Zoom::Overview, Zoom::Wide, Zoom::Detail] {
            g.set_zoom(z, g.world_view().center());
            g.screenshot(&dir.join(format!("{name}-{}x.png", z.factor())))?;
            let mut samples = Vec::new();
            for _ in 0..40 {
                let t = std::time::Instant::now();
                g.render();
                samples.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            samples.sort_by(f64::total_cmp);
            timings.push(serde_json::json!({"faction":name,"world_scale":z.factor(),"size":[1920,1080],"p95_ms":samples[37],"p99_ms":samples[39],"samples":40,"scope":"CPU compositor, ordinary starting field on local hardware"}));
        }
    }
    g.begin_practice();
    g.zoom = Zoom::Wide;
    g.screenshot(&dir.join("practice.png"))?;
    g.action(Action::EndGuidance);
    if let Some(worker) = g
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind.is_worker())
    {
        g.selected = vec![worker.id];
    }
    for (w, h) in [(1280, 720), (1537, 865), (2560, 1440), (960, 540)] {
        g.resize_view(w, h);
        g.screenshot(&dir.join(format!("worker-{w}x{h}.png")))?;
    }
    lean_hud_scenes(&mut g, dir)?;
    g.resize_view(1920, 1080);
    g.screen = Screen::Pause;
    g.screenshot(&dir.join("pause.png"))?;
    std::fs::write(
        dir.join("perf.json"),
        serde_json::to_vec_pretty(&timings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The lean HUD's review scenes: a headquarters with a queue, a mixed army
/// with the pressure bar full, and a Works offering its upgrades.
fn lean_hud_scenes(g: &mut Game, dir: &Path) -> Result<(), String> {
    use bw_core::{Kind, Pos};
    let faction = g.faction;
    let Some((hq, hq_pos)) = g
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .map(|e| (e.id, e.pos))
    else {
        return Ok(());
    };
    let (cx, cy) = hq_pos.cell_xy();
    g.world.players[0].salvage = 2_000;
    g.world.players[0].pressure = g.world.players[0].pressure_cap;
    g.selected = vec![hq];
    for _ in 0..3 {
        g.key("Q", false, false);
    }
    for _ in 0..40 {
        g.tick();
    }
    let army: Vec<Kind> = if faction == Faction::Union {
        vec![
            Kind::Riveter,
            Kind::Riveter,
            Kind::Bulwark,
            Kind::Sounder,
            Kind::Tidewatch,
            Kind::Caulker,
        ]
    } else {
        vec![
            Kind::Reedguard,
            Kind::Reedguard,
            Kind::Loom,
            Kind::Skipper,
            Kind::Lampwright,
            Kind::Tender,
        ]
    };
    let mut ids = Vec::new();
    for (i, k) in army.iter().enumerate() {
        let i = i as i32;
        ids.push(
            g.world
                .spawn_for_tests(0, *k, Pos::cell(cx + 4 + i % 3, cy + 3 + i / 3)),
        );
    }
    let works = g
        .world
        .spawn_for_tests(0, Kind::Works, Pos::cell(cx - 6, cy + 5));
    g.world.players[0].pressure = g.world.players[0].pressure_cap;
    for (w, h) in [(1280, 720), (1920, 1080)] {
        g.resize_view(w, h);
        g.selected = vec![hq];
        g.screenshot(&dir.join(format!("lean-hq-{w}x{h}.png")))?;
        g.selected = ids.clone();
        g.screenshot(&dir.join(format!("lean-army-{w}x{h}.png")))?;
        g.selected = vec![works];
        g.screenshot(&dir.join(format!("lean-works-{w}x{h}.png")))?;
        // The same card with the pointer on its first button: the name,
        // price and effect the button no longer spells out.
        g.render();
        if let Some(b) = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::Train(_)))
            .cloned()
        {
            g.pointer_moved(b.x + b.w / 2, b.y + b.h / 2);
            g.screenshot(&dir.join(format!("lean-tooltip-{w}x{h}.png")))?;
            g.cursor = (-999, -999);
        }
    }
    // One hover card of each shape at 1280x720: a machine, a building, an
    // upgrade, a doctrine, a pressure order, a plain order, a roster icon
    // and a refusal.
    g.resize_view(1280, 720);
    let worker = g
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind.is_worker())
        .map(|e| e.id);
    type Pick = fn(&Action) -> bool;
    let mut shots: Vec<(&str, Vec<u32>, Pick)> = vec![
        ("machine", vec![works], |a| matches!(a, Action::Train(_))),
        ("upgrade", vec![works], |a| matches!(a, Action::Upgrade(_))),
        ("doctrine", vec![hq], |a| matches!(a, Action::Research(_))),
        ("vent", vec![hq], |a| *a == Action::Vent),
        ("capture", ids.clone(), |a| *a == Action::Capture),
        ("surge", ids.clone(), |a| *a == Action::Surge),
        ("roster", ids.clone(), |a| {
            matches!(a, Action::FilterSelection(..))
        }),
    ];
    if let Some(worker) = worker {
        shots.push(("building", vec![worker], |a| {
            *a == Action::Build(bw_core::Kind::Condenser)
        }));
        shots.push(("nest", vec![worker], |a| {
            *a == Action::Build(bw_core::Kind::Tower)
        }));
    }
    for (name, selection, pick) in shots {
        g.selected = selection;
        g.cursor = (-999, -999);
        g.render();
        if let Some(b) = g.buttons.iter().find(|b| pick(&b.action)).cloned() {
            g.pointer_moved(b.x + b.w / 2, b.y + b.h / 2);
            g.screenshot(&dir.join(format!("hover-{name}.png")))?;
        }
    }
    // A refusal: the same machine with the stock spent.
    let salvage = g.world.players[0].salvage;
    g.world.players[0].salvage = 10;
    g.selected = vec![works];
    g.cursor = (-999, -999);
    g.render();
    if let Some(b) = g
        .buttons
        .iter()
        .find(|b| matches!(b.action, Action::Train(_)))
        .cloned()
    {
        g.pointer_moved(b.x + b.w / 2, b.y + b.h / 2);
        g.screenshot(&dir.join("hover-refused.png"))?;
    }
    g.world.players[0].salvage = salvage;
    g.cursor = (-999, -999);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(name: &str) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("brinewake-native-{name}-{}", std::process::id())),
        );
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1920, 1080);
        g.cursor = (-999, -999);
        g.message.clear();
        g
    }
    #[test]
    fn research_keys_start_research_like_the_button() {
        let mut g = game("research-keys");
        g.world.players[0].salvage = 1_000;
        g.world.players[0].pressure = 300;
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![hq];
        g.key("U", false, false);
        g.tick();
        g.tick();
        assert!(
            g.world.players[0].research.is_some(),
            "U with the headquarters selected starts Hauling research"
        );
    }
    #[test]
    fn a_condenser_snaps_to_a_well_and_the_prompt_reads_the_same_site() {
        let mut g = game("condenser-snap");
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        g.world.players[0].salvage = 1_000;
        let well = g.world.map.wells[0];
        let (wx, wy) = well.cell_xy();
        // Pointing two cells off the well still lands the footprint on it.
        assert_eq!(
            g.build_origin(Kind::Condenser, Pos::cell(wx + 2, wy + 1)),
            well
        );
        assert_ne!(g.build_origin(Kind::Condenser, Pos::cell(wx + 6, wy)), well);
        // A Works still centres under the pointer.
        assert_eq!(
            g.build_origin(Kind::Works, Pos::cell(30, 30)),
            Pos::cell(29, 29)
        );
        g.world.revealed = true;
        g.mode = crate::game::Mode::Build(Kind::Condenser);
        g.camera.center(well);
        let (x, y) = g.project(Pos::cell(wx + 1, wy + 1));
        g.pointer_moved(x, y);
        assert!(
            g.world_pointer_allowed(x, y),
            "the well sits in the field at {x} {y}"
        );
        let prompt = g.native_prompt();
        assert!(
            prompt.contains("CLICK TO BUILD"),
            "the prompt judges the snapped site: {prompt}"
        );
        g.left_down(x, y);
        g.tick();
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.owner == 0 && e.kind == Kind::Condenser && e.pos == well),
            "the click placed the condenser on the well"
        );
    }
    #[test]
    fn a_wreck_under_the_fog_takes_a_gather_order_and_the_panel_names_states() {
        let mut g = game("fogged-wreck");
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        let hidden = g
            .world
            .map
            .resources
            .iter()
            .find(|r| !g.world.visible(0, r.pos))
            .cloned()
            .expect("a wreck under the fog");
        g.camera.center(hidden.pos);
        let (x, y) = g.project(hidden.pos);
        assert!(g.world_pointer_allowed(x, y));
        let (command, verb, _) = g
            .context_order(x, y, false)
            .expect("an order for a fogged wreck");
        assert_eq!(verb, "Gather");
        assert!(
            matches!(command, bw_sim::Command::Gather { resource, .. } if resource == hidden.id)
        );
        // The panel says what the machine is doing, not which keys exist.
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == worker)
            .unwrap()
            .order = bw_sim::Order::Idle;
        assert_eq!(g.console_state().selection.unwrap().status, "IDLE");
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == worker)
            .unwrap()
            .order = bw_sim::Order::Gather {
            resource: hidden.id,
        };
        assert_eq!(g.console_state().selection.unwrap().status, "GATHERING");
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![hq];
        g.world.players[0].salvage = 1_000;
        g.key("Q", false, false);
        assert_eq!(g.message, "HOOK queued.");
    }
    #[test]
    fn a_blocked_door_says_so_after_two_seconds_and_raises_one_card() {
        let mut g = game("blocked-exit");
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![hq];
        g.world.players[0].salvage = 1_000;
        g.key("Q", false, false);
        g.tick();
        // Hold the finished worker at the door by keeping its timer at one.
        for _ in 0..90 {
            if let Some(p) = g
                .world
                .entities
                .iter_mut()
                .find(|e| e.id == hq)
                .and_then(|e| e.queue.first_mut())
            {
                p.started = true;
                p.remaining = 2;
            }
            g.tick();
        }
        assert_eq!(
            g.selected_production_info().unwrap().status,
            "EXIT BLOCKED / CLEAR THE DOOR"
        );
        let cards: Vec<String> = g.ux.alerts.entries.iter().map(|a| a.label()).collect();
        assert_eq!(cards, ["HQ EXIT BLOCKED"], "one card, not one per tick");
        // Released: the status and the tracking clear.
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == hq)
            .unwrap()
            .queue
            .clear();
        g.tick();
        assert!(!g.exit_blocked(hq));
    }
    #[test]
    fn the_sluice_card_reads_contest_and_capture_progress() {
        let mut g = game("sluice-card");
        let gate = g.world.map.gate_pos;
        let (gx, gy) = gate.cell_xy();
        g.world.gate.capture_player = Some(0);
        g.world.gate.capture_progress = bw_sim::CAPTURE_TICKS / 2;
        g.resize_view(1280, 720);
        g.render();
        assert!(!g.sluice_contested());
        let template = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .cloned()
            .unwrap();
        for (id, owner, kind, pos) in [
            (900, 0, Kind::Riveter, Pos::cell(gx - 1, gy)),
            (901, 1, Kind::Reedguard, Pos::cell(gx + 1, gy)),
        ] {
            g.world.entities.push(bw_sim::Entity {
                id,
                owner,
                kind,
                pos,
                order: bw_sim::Order::Idle,
                ..template.clone()
            });
        }
        g.world.revealed = true;
        assert!(g.sluice_contested(), "both sides stand at the sluice");
        let state = crate::agent::state_for_tests(&g);
        assert_eq!(state["sluice"]["contested"], true);
        assert_eq!(
            state["sluice"]["capture_progress"],
            bw_sim::CAPTURE_TICKS / 2
        );
    }
    #[test]
    fn the_sluice_card_hides_switch_until_owned_and_dock_tooltips_stand_on_the_dock() {
        let mut g = game("sluice-switch");
        g.ux.preferences.tide_card = true;
        g.resize_view(1280, 720);
        g.render();
        assert!(
            !g.buttons
                .iter()
                .any(|b| matches!(b.action, Action::SetTide(_) | Action::Flood)),
            "no tide buttons while the sluice is neutral"
        );
        let card = g.route_bounds();
        let s = g.ui_scale();
        assert_eq!(card.h, 71 * s);
        g.world.gate.owner = Some(0);
        g.render();
        // The holder is offered both sides and the flood at the neutral tide.
        assert!(
            g.buttons
                .iter()
                .any(|b| b.action == Action::SetTide(bw_sim::Arm::NORTH))
        );
        assert!(
            g.buttons
                .iter()
                .any(|b| b.action == Action::SetTide(bw_sim::Arm::SOUTH))
        );
        assert!(g.buttons.iter().any(|b| b.action == Action::Flood));
        assert_eq!(g.route_bounds().h, 93 * s);

        // A card in the band opens its tooltip downward, never over the field.
        g.ux.alerts.push(
            crate::field_alerts::AlertKind::Attack,
            Pos::cell(40, 40),
            g.world.tick,
        );
        g.render();
        let card = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::FocusAlert(_)))
            .cloned()
            .unwrap();
        g.pointer_moved(card.x + 4, card.y + 4);
        g.render();
        let tip = g.native_tooltip_bounds().expect("a tooltip for the card");
        assert!(
            tip.y >= g.world_view().bottom,
            "tooltip at {} is below the field",
            tip.y
        );
        // The WORKS button lives in the dock's quick column: its tooltip
        // stands on the dock above it, on the screen, not over the button.
        let works = g
            .buttons
            .iter()
            .find(|b| b.action == Action::SelectWorks)
            .cloned()
            .unwrap();
        assert!(works.y >= g.world_view().bottom, "WORKS sits in the dock");
        g.pointer_moved(works.x + 4, works.y + 4);
        g.render();
        let tip = g.native_tooltip_bounds().expect("a tooltip for WORKS");
        assert!(tip.y + tip.h <= works.y, "above the button");
        assert!(tip.y + tip.h <= g.world_view().bottom && tip.x >= 0);
    }
    #[test]
    fn effect_rows_fit_their_buttons_and_the_prompt_points_to_an_unseen_well() {
        let mut g = game("effect-rows");
        g.resize_view(1280, 720);
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.world.players[0].salvage = 1_000;
        g.faction = bw_core::Faction::Assembly;
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        let works = g.world.spawn_for_tests(0, Kind::Works, Pos::cell(100, 60));
        g.selected = vec![works];
        g.render();
        let loom = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Train(Kind::Loom))
            .cloned()
            .expect("a Loom button");
        let rows: Vec<&str> = loom.hint.split('\n').collect();
        assert_eq!(rows.len(), 2, "cost row and effect row: {:?}", loom.hint);
        // The button shows the Loom; the rows are the tooltip's.
        let cap = ((crate::native_ui::TIP_W - 10) / 6) as usize;
        assert!(rows.iter().all(|r| r.len() <= cap), "{rows:?} within {cap}");
        assert_eq!(rows[1], "D TO FIRE");

        let _ = worker;
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        g.mode = crate::game::Mode::Build(Kind::Condenser);
        // Look at the map's corner far from any well: the prompt names a direction.
        g.camera.center(Pos::cell(4, 4));
        g.pointer_moved(640, 300);
        assert!(g.nearest_well_off_screen(Kind::Condenser).is_some());
        let prompt = g.native_prompt();
        assert!(prompt.contains("NEAREST WELL IS"), "{prompt}");
        let well = g.world.map.wells[0];
        g.camera.center(well);
        assert_eq!(g.nearest_well_off_screen(Kind::Condenser), None);
    }
    #[test]
    fn the_command_card_lives_in_the_dock_and_tooltips_follow_one_structure() {
        let mut g = game("command-card");
        g.resize_view(1280, 720);
        let s = g.ui_scale();
        let dock_top = g.world_view().bottom;
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        g.render();
        for kind in [
            Kind::Works,
            Kind::Condenser,
            Kind::Dropoff,
            Kind::Tower,
            Kind::Drydock,
            Kind::Palisade,
        ] {
            let b = g
                .buttons
                .iter()
                .find(|b| b.action == Action::Build(kind))
                .unwrap_or_else(|| panic!("a build button for {kind:?}"));
            assert!(b.y >= dock_top, "{kind:?} sits in the dock");
            assert_eq!(b.h, crate::lean_hud::CELL * s);
        }
        assert!(g.buttons.iter().any(|b| b.action == Action::Gather));
        assert!(g.buttons.iter().any(|b| b.action == Action::Stop));

        // A machine: tactical buttons in the card, SOUND for a Sounder.
        let sounder = g.world.spawn_for_tests(0, Kind::Sounder, Pos::cell(30, 60));
        g.selected = vec![sounder];
        g.render();
        for action in [
            Action::Attack,
            Action::Hold,
            Action::Capture,
            Action::Surge,
            Action::Sound,
            Action::Formation,
            Action::Face,
        ] {
            let b = g
                .buttons
                .iter()
                .find(|b| b.action == action)
                .unwrap_or_else(|| panic!("{action:?} on the card"));
            assert!(b.y >= dock_top);
        }
        // A tooltip for a training button: cost, role, matchups, key; in the dock.
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![hq];
        g.render();
        assert!(g.buttons.iter().any(|b| b.action == Action::Vent));
        let train = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::Train(_)))
            .cloned()
            .unwrap();
        g.pointer_moved(train.x + 4, train.y + 4);
        g.render();
        let tip = g.native_tooltip_bounds().expect("a tooltip");
        assert!(
            tip.y + tip.h <= g.world_view().bottom && tip.y >= g.world_view().top,
            "dock tooltips stand on the dock, over the field's edge: {tip:?} {:?}",
            g.hover_card(&train)
        );
        assert_eq!(tip.x + tip.w, 1280 - 6 * g.ui_scale(), "flush right");
        // The hover card: the name and key, the price as marks, the
        // machine's numbers, one line of what it does.  The flavour and
        // the counters live in the Guide.
        let card = g.hover_card(&train).expect("a hover card");
        assert_eq!(card.title, "HOOK");
        assert_eq!(card.key.as_deref(), Some("Q"));
        assert_eq!(card.tag.as_deref(), Some("WORKER"));
        assert_eq!(card.costs[0].mark, crate::hover_card::Mark::Salvage);
        assert!(
            card.stats
                .iter()
                .any(|c| c.mark == crate::hover_card::Mark::Hull)
        );
        assert_eq!(card.body.len(), 1, "{:?}", card.body);
        assert!(card.note.is_empty(), "{:?}", card.note);
        let _ = s;
        // The panel names the role and the stats without a hover.
        let state = g.console_state();
        let sel = state.selection.unwrap();
        assert!(sel.role.contains("Trains workers"), "{}", sel.role);
        assert!(sel.stats.contains("SIGHT"), "{}", sel.stats);
        // A tooltip no longer closes while the pointer rests.
        for _ in 0..200 {
            g.tick();
        }
        g.render();
        assert!(g.native_tooltip_bounds().is_some());
    }
    #[test]
    fn context_keys_build_tier_two_start_upgrades_and_open_vent() {
        let mut g = game("context-keys");
        g.world.players[0].salvage = 2_000;
        g.world.players[0].pressure = 300;
        g.world.players[0].pressure_cap = 300;
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        g.key("N", false, false);
        assert_eq!(g.mode, crate::game::Mode::Build(Kind::Drydock));
        g.key("Escape", false, false);
        g.key("P", false, false);
        assert_eq!(g.mode, crate::game::Mode::Build(Kind::Palisade));
        g.key("Escape", false, false);
        let works = g.world.spawn_for_tests(0, Kind::Works, Pos::cell(24, 50));
        g.selected = vec![works];
        g.key("P", false, false);
        g.tick();
        g.tick();
        assert!(
            g.world.entities.iter().any(|e| e.id == works
                && e.upgrade
                    .is_some_and(|j| j.upgrade == bw_content::Upgrade::Plate)),
            "P at a Works starts Plate"
        );
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![hq];
        g.key("X", false, false);
        g.tick();
        g.tick();
        assert!(
            g.world.players[0].vent_remaining > 0,
            "X at the headquarters opens VENT"
        );
        // The Drydock trains tier two from the same Q/W/E keys.
        let drydock = g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(28, 50));
        g.selected = vec![drydock];
        // Since trial 11 the scout is W at the Drydock as at headquarters.
        assert_eq!(g.production_hotkey_kind("Q"), Some(Kind::Caulker));
        assert_eq!(g.production_hotkey_kind("W"), Some(Kind::Tidewatch));
        assert_eq!(g.production_hotkey_kind("E"), Some(Kind::Caisson));
    }
    #[test]
    fn idle_key_cycles_without_moving_the_camera_and_control_centres() {
        let mut g = game("idle-tap");
        for e in g
            .world
            .entities
            .iter_mut()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
        {
            e.order = bw_sim::Order::Idle;
        }
        let far = Pos::cell(100, 100);
        g.camera.center(far);
        let before = (g.camera.x, g.camera.y);
        g.key("I", false, false);
        let first = g.selected.clone();
        assert!(!first.is_empty(), "a worker was selected");
        assert_eq!(
            (g.camera.x, g.camera.y),
            before,
            "one press leaves the view alone"
        );
        g.key("I", false, false);
        assert_ne!(
            g.selected, first,
            "a second press cycles to the next idle worker"
        );
        assert_eq!(
            (g.camera.x, g.camera.y),
            before,
            "cycling leaves the view alone"
        );
        g.key("I", false, true);
        assert_ne!(
            (g.camera.x, g.camera.y),
            before,
            "Ctrl+I centres on the selection"
        );
    }
    #[test]
    fn a_machine_behind_a_building_shows_a_ghost_and_stays_targetable() {
        let mut g = game("ghost");
        g.zoom = Zoom::Wide;
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .unwrap();
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        // Park every other machine far away so only the building can cover
        // the worker; put the worker on the headquarters' north face.
        let (hx, hy) = hq.cell_xy();
        for e in g
            .world
            .entities
            .iter_mut()
            .filter(|e| e.owner == 0 && !e.kind.is_building())
        {
            e.pos = if e.id == worker {
                Pos::cell(hx + 1, hy - 1)
            } else {
                Pos::cell(hx - 12, hy + 12)
            };
        }
        g.camera.center(hq);
        g.render();
        let with_worker = g.canvas.pixels.clone();
        let (ux, uy) = g.project(Pos::cell(hx + 1, hy - 1));
        // Somewhere on the worker's body the building is opaque and the
        // picker still returns the worker, not the building.
        let mut ghost_pixels = 0;
        let mut picked = 0;
        for dy in -40..0 {
            for dx in -16..16 {
                let (x, y) = (ux + dx, uy + dy);
                if g.unit_at(x, y) == Some(worker) {
                    picked += 1;
                }
                let i = ((y as u32 * 1920 + x as u32) * 4) as usize;
                if with_worker[i..i + 4] != g.canvas.pixels[i..i + 4] {
                    ghost_pixels += 1;
                }
            }
        }
        assert!(picked > 20, "worker picked through the building: {picked}");
        // Remove the worker: the building alone must render differently
        // where the ghost was.
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == worker) {
            e.pos = Pos::cell(hx - 12, hy + 12);
        }
        g.render();
        let mut changed = 0;
        for dy in -40..0 {
            for dx in -16..16 {
                let (x, y) = (ux + dx, uy + dy);
                let i = ((y as u32 * 1920 + x as u32) * 4) as usize;
                if with_worker[i..i + 4] != g.canvas.pixels[i..i + 4] {
                    changed += 1;
                }
            }
        }
        assert!(
            changed > 20,
            "ghost pixels drawn through the building: {changed}"
        );
        let _ = ghost_pixels;
    }
    #[test]
    fn actual_composited_world_keeps_all_texels_at_every_integer_scale() {
        let mut g = game("pixels");
        let hash = g.world.state_hash();
        for (w, h) in [(1920, 1080), (1537, 865), (1280, 720)] {
            g.resize_view(w, h);
            for z in [Zoom::Overview, Zoom::Wide, Zoom::Detail] {
                g.zoom = z;
                g.render();
                let view = g.world_view();
                let k = z.factor();
                // Compare actual unobscured field pixels. Recovery controls
                // can enter this sample region at narrower window sizes.
                let mut compared = 0;
                for y in (view.top + 250..view.bottom - 40).step_by(5) {
                    for x in (w as i32 / 2 - 120..w as i32 / 2 + 120).step_by(3) {
                        if !g.native_world_pointer_allowed(x, y) {
                            continue;
                        }
                        compared += 1;
                        let src = view.source((x, y));
                        let a = ((y as u32 * w + x as u32) * 4) as usize;
                        let b =
                            ((src.1 as u32 * g.scene_canvas.width() + src.0 as u32) * 4) as usize;
                        assert_eq!(
                            &g.canvas.pixels[a..a + 4],
                            &g.scene_canvas.pixels[b..b + 4],
                            "{w}x{h} {k}x at {x},{y}"
                        );
                    }
                }
                assert!(compared > 500, "retain meaningful world pixel coverage");
                assert_eq!(g.world.state_hash(), hash);
            }
        }
    }
    #[test]
    fn native_world_picking_zoom_anchor_minimap_and_commands_agree() {
        let mut g = game("input");
        let worker = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap();
        worker.pos = Pos::cell(30, 61);
        let id = worker.id;
        g.camera.center(worker.pos);
        for (w, h) in [(1920, 1080), (1537, 865), (1280, 720)] {
            g.resize_view(w, h);
            for z in [Zoom::Overview, Zoom::Wide, Zoom::Detail] {
                let anchor = g.world_view().center();
                let before = g.unproject(anchor.0, anchor.1);
                g.set_zoom(z, anchor);
                assert_eq!(before, g.unproject(anchor.0, anchor.1));
                g.render();
                let (x, y) = g.project(Pos::cell(30, 61));
                let hit = (-60..0)
                    .flat_map(|dy| (-30..30).map(move |dx| (x + dx, y + dy)))
                    .find(|&(x, y)| g.unit_at(x, y) == Some(id))
                    .unwrap();
                g.left_down(hit.0, hit.1);
                g.left_up(hit.0, hit.1, false);
                assert!(g.selected.contains(&id));
                g.render();
                let b = g
                    .buttons
                    .iter()
                    .find(|b| b.action == Action::Build(Kind::Works))
                    .unwrap()
                    .clone();
                assert!(b.enabled);
                g.left_down(b.x + b.w / 2, b.y + b.h / 2);
                assert!(matches!(g.mode, crate::game::Mode::Build(Kind::Works)));
                g.key("Escape", false, false);
                let p = g.project(Pos::cell(32, 63));
                g.right_click(p.0, p.1, false);
                assert!(
                    matches!(&g.world.command_log.last().unwrap().command,bw_sim::Command::Move{target,..} if target.cell_xy()==(32,63))
                );
                let r = g.minimap_bounds();
                g.left_down(r.x + r.w / 2, r.y + r.h / 2);
                let center = g
                    .unproject(g.world_view().center().0, g.world_view().center().1)
                    .cell_xy();
                assert_eq!(
                    center,
                    (
                        i32::from(g.world.map.width) / 2,
                        i32::from(g.world.map.height) / 2
                    )
                );
                g.camera.center(Pos::cell(30, 61));
            }
        }
    }
    #[test]
    fn slow_panning_accumulates_pixels_and_ui_does_not_zoom() {
        let mut g = game("pan");
        g.zoom = Zoom::Detail;
        g.render();
        // The top bar's left half: side, stock, crew and clock.  (Its
        // right end names the zoom, which is the point of zooming.)
        let bar = |g: &Game| -> Vec<u8> {
            (0..crate::lean_hud::TOP_BAR * 2 - 1)
                .flat_map(|y| {
                    let at = (y * 1920 * 4) as usize;
                    g.canvas.pixels[at..at + 960 * 4].to_vec()
                })
                .collect()
        };
        let header = bar(&g);
        let camera = g.camera;
        for _ in 0..3 {
            g.pan(1, 1);
        }
        assert_eq!((g.camera.x, g.camera.y), (camera.x + 1, camera.y + 1));
        for _ in 0..3 {
            g.pan(-1, -1);
        }
        assert_eq!((g.camera.x, g.camera.y), (camera.x, camera.y));
        for zoom in [Zoom::Overview, Zoom::Wide] {
            g.zoom = zoom;
            g.render();
            assert_eq!(bar(&g), header);
        }
    }
    #[test]
    fn menus_and_hud_buttons_stay_on_screen_after_resizing() {
        let mut g = game("layout");
        for (w, h) in [
            (960, 540),
            (1280, 720),
            (1537, 865),
            (1920, 1080),
            (2560, 1440),
        ] {
            g.resize_view(w, h);
            for screen in [
                Screen::Menu,
                Screen::Setup,
                Screen::Settings,
                Screen::Pause,
                Screen::Help,
                Screen::Match,
            ] {
                g.screen = screen;
                g.render();
                for b in &g.buttons {
                    assert!(
                        b.x >= 0 && b.y >= 0 && b.x + b.w <= w as i32 && b.y + b.h <= h as i32,
                        "{:?}: {} at {w}x{h}",
                        screen,
                        b.label
                    );
                }
            }
            g.screen = Screen::Menu;
            g.render();
            let b = g
                .buttons
                .iter()
                .find(|b| b.action == Action::Settings)
                .unwrap()
                .clone();
            g.left_down(b.x + b.w / 2, b.y + b.h / 2);
            assert_eq!(g.screen, Screen::Settings);
            g.render();
            g.key("Escape", false, false);
            assert_eq!(g.screen, Screen::Menu);
        }
    }
}
