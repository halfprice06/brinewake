use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bw_core::{Faction, Kind, Pos};
use bw_sim::Command;

use crate::game::{Action, Game, Mode, Screen};

fn fresh_game(label: &str) -> (Game, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!(
        "brinewake-desktop-audit-{label}-{}-{stamp}",
        std::process::id()
    ));
    let game = Game::new_with_data_dir(base, data_dir.clone());
    (game, data_dir)
}

fn cleanup(path: PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}

fn worker_id(game: &Game) -> u32 {
    // Use the exposed front worker. With the wider v2 hulls, the middle
    // worker visibly covers the rear worker's ground-anchor pixel at (15,67).
    game.world
        .entities
        .iter()
        .find(|entity| {
            entity.owner == 0 && entity.kind.is_worker() && entity.pos.cell_xy() == (17, 67)
        })
        .or_else(|| {
            game.world
                .entities
                .iter()
                .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        })
        .map(|entity| entity.id)
        .expect("starting worker")
}

fn entity(game: &Game, id: u32) -> &bw_sim::Entity {
    game.world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .expect("entity exists")
}

fn point_for(game: &Game, pos: Pos) -> (i32, i32) {
    game.camera.project(pos)
}

fn click_button<F>(game: &mut Game, predicate: F)
where
    F: Fn(&Action) -> bool,
{
    game.render();
    let button = game
        .buttons
        .iter()
        .find(|button| predicate(&button.action))
        .cloned()
        .expect("expected button");
    game.left_down(button.x + button.w / 2, button.y + button.h / 2);
}

#[test]
fn keyboard_navigation_reaches_live_guidance_and_escape_returns_to_the_field() {
    let (mut game, dir) = fresh_game("keyboard-guidance");
    game.key("Enter", false, false);
    assert!(game.ux.practice);
    // The first key closes the how-to-win card and does nothing else.
    assert!(game.intro.is_some_and(|i| i.lesson));
    game.key("Tab", false, false);
    assert!(game.intro.is_none());
    for _ in 0..24 {
        game.key("Tab", false, false);
        game.render();
        assert!(game.ux.keyboard_navigation);
        assert!(
            game.buttons
                .iter()
                .any(|b| b.enabled && Some(&b.action) == game.ux.focused.as_ref())
        );
        if game.ux.focused == Some(Action::FocusWorker) {
            break;
        }
    }
    assert_eq!(game.ux.focused, Some(Action::FocusWorker));
    game.key("Enter", false, false);
    assert_eq!(game.ux.tutorial.as_ref().unwrap().stage(), 1);
    assert!(!game.ux.keyboard_navigation);
    game.key("Tab", false, false);
    game.key("Escape", false, false);
    assert_eq!(game.screen, Screen::Match);
    assert!(!game.ux.keyboard_navigation);
    game.key("Escape", false, false);
    assert_eq!(game.screen, Screen::Pause);
    game.key("Enter", false, false);
    assert_eq!(game.screen, Screen::Match);
    cleanup(dir);
}

#[test]
fn menu_return_preserves_the_field_and_replacement_defaults_to_keep_it() {
    let (mut game, dir) = fresh_game("continue");
    game.begin_practice();
    game.intro = None;
    game.action(Action::FocusWorker);
    game.tick();
    let hash = game.world.state_hash();
    let selected = game.selected.clone();
    game.action(Action::MainMenu);
    game.tick();
    game.action(Action::Setup);
    game.action(Action::Faction(Faction::Assembly));
    game.action(Action::Start);
    assert_eq!(game.screen, Screen::Confirm);
    game.key("Enter", false, false);
    assert_eq!(game.screen, Screen::Setup);
    game.action(Action::Back);
    game.key("Enter", false, false);
    assert_eq!(game.screen, Screen::Match);
    assert_eq!(game.world.state_hash(), hash);
    assert_eq!(game.selected, selected);
    assert_eq!(game.faction, Faction::Union);
    cleanup(dir);
}

#[test]
fn loading_cannot_overwrite_its_saved_slot_and_restores_guidance() {
    let (mut game, dir) = fresh_game("load-slot");
    game.begin_practice();
    game.intro = None;
    game.action(Action::FocusWorker);
    assert!(game.save_match());
    let hash = game.world.state_hash();
    let saved_bytes = std::fs::read(dir.join("saves/quick-save.json")).unwrap();
    game.tick();
    game.action(Action::Load);
    game.render();
    assert_eq!(game.screen, Screen::Confirm);
    assert!(
        !game
            .buttons
            .iter()
            .any(|b| b.action == Action::SaveAndConfirm)
    );
    game.action(Action::SaveAndConfirm);
    assert_eq!(game.screen, Screen::Confirm);
    assert_eq!(
        std::fs::read(dir.join("saves/quick-save.json")).unwrap(),
        saved_bytes
    );
    game.action(Action::Confirm);
    assert_eq!(game.world.state_hash(), hash);
    assert_eq!(game.ux.tutorial.as_ref().unwrap().stage(), 1);
    assert!(game.ux.guidance_visible);
    cleanup(dir);
}

#[test]
fn missing_guide_sidecar_keeps_practice_playable_with_visible_recovery() {
    let (mut game, dir) = fresh_game("missing-sidecar");
    game.begin_practice();
    game.intro = None;
    assert!(game.save_match());
    let hash = game.world.state_hash();
    std::fs::remove_file(dir.join("saves/quick-save-ui.json")).unwrap();
    game.action(Action::Load);
    game.render();
    assert_eq!(game.world.state_hash(), hash);
    assert!(game.ux.practice);
    assert!(game.ux.tutorial.is_none());
    assert!(
        game.buttons
            .iter()
            .any(|b| b.enabled && b.action == Action::StartPractice)
    );
    assert!(
        game.buttons
            .iter()
            .any(|b| b.enabled && b.action == Action::Help)
    );
    game.left_down(100, 70);
    assert!(game.drag.is_none());
    cleanup(dir);
}

#[test]
fn failed_load_keeps_the_active_world_and_disables_the_bad_save() {
    let (mut game, dir) = fresh_game("corrupt-save");
    game.begin_practice();
    game.intro = None;
    assert!(game.save_match());
    game.tick();
    let hash = game.world.state_hash();
    std::fs::write(dir.join("saves/quick-save.json"), b"not a valid save").unwrap();
    game.action(Action::Load);
    game.action(Action::Confirm);
    assert_eq!(game.world.state_hash(), hash);
    assert_eq!(game.screen, Screen::Pause);
    assert!(!game.ux.save_available);
    assert!(game.message.contains("unchanged"));
    cleanup(dir);
}

#[test]
fn disabled_mouse_and_keyboard_controls_cannot_start_a_missing_art_game() {
    let (mut game, dir) = fresh_game("missing-art");
    game.atlas = None;
    click_button(&mut game, |a| *a == Action::Setup);
    assert_eq!(game.screen, Screen::Menu);
    for _ in 0..12 {
        game.key("Tab", false, false);
        assert!(!matches!(
            game.ux.focused,
            Some(Action::Setup | Action::StartPractice | Action::Load)
        ));
    }
    game.action(Action::StartPractice);
    assert!(!game.ux.session_active);
    cleanup(dir);
}

#[test]
fn practice_ends_without_a_fake_computer_victory() {
    let (mut game, dir) = fresh_game("end-practice");
    game.begin_practice();
    game.intro = None;
    game.key("Escape", false, false);
    game.render();
    assert!(
        game.buttons
            .iter()
            .any(|b| b.label.eq_ignore_ascii_case("End practice"))
    );
    game.action(Action::Surrender);
    assert_eq!(game.screen, Screen::Confirm);
    game.action(Action::Confirm);
    assert_eq!(game.screen, Screen::Match);
    assert!(game.ux.practice_review);
    let end_hash = game.world.state_hash();
    game.tick();
    assert_eq!(
        game.world.state_hash(),
        end_hash,
        "practice review must freeze rules"
    );
    game.render();
    assert!(
        game.buttons
            .iter()
            .any(|button| button.action == Action::EndReview)
    );
    game.action(Action::EndReview);
    assert_eq!(game.screen, Screen::Menu);
    assert!(!game.has_session());
    assert!(game.world.outcome.is_none());
    assert!(
        !game
            .world
            .command_log
            .iter()
            .any(|r| matches!(r.command, Command::Surrender))
    );
    cleanup(dir);
}

#[test]
fn mixed_attack_selection_excludes_static_buildings_and_cancel_sends_no_order() {
    let (mut game, dir) = fresh_game("mixed-attack");
    game.start();
    game.world.ai_enabled = false;
    let hq = game.selected[0];
    let worker = worker_id(&game);
    game.selected = vec![hq, worker];
    game.action(Action::Attack);
    let before = game.world.command_log.len();
    game.right_click(320, 140, false);
    assert_eq!(game.mode, Mode::Context);
    assert_eq!(game.world.command_log.len(), before);
    game.action(Action::Attack);
    let (x, y) = game.camera.project(Pos::cell(20, 63));
    game.left_down(x, y);
    assert!(matches!(&game.world.command_log.last().unwrap().command,
        Command::AttackMove { units, .. } if units == &vec![worker]));
    cleanup(dir);
}

#[test]
fn rejected_orders_have_no_success_markers_at_submission_or_application() {
    let (mut game, dir) = fresh_game("order-rejected");
    game.start();
    game.world.ai_enabled = false;
    let worker = worker_id(&game);
    game.issue(Command::Move {
        units: vec![worker],
        target: Pos::cell(-1, -1),
        queued: false,
    });
    assert!(game.ux.markers.is_empty());
    game.issue(Command::Build {
        worker,
        kind: Kind::Works,
        pos: Pos::cell(20, 63),
        queued: false,
    });
    assert!(!game.ux.markers.is_empty());
    game.world.players[0].salvage = 0;
    game.tick();
    assert_eq!(game.world.command_log.last().unwrap().applied, Some(false));
    assert!(game.ux.markers.is_empty());
    cleanup(dir);
}

#[test]
fn sound_and_edge_preferences_survive_restart_with_safe_corruption_defaults() {
    let (mut game, dir) = fresh_game("preferences");
    game.action(Action::Mute);
    game.action(Action::ToggleEdgeScroll);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let restored = Game::new_with_data_dir(base.clone(), dir.clone());
    assert!(restored.muted);
    assert!(!restored.ux.preferences.edge_scroll);
    std::fs::write(dir.join("ui/preferences.json"), b"bad settings").unwrap();
    let recovered = Game::new_with_data_dir(base, dir.clone());
    assert!(!recovered.muted);
    assert!(recovered.ux.preferences.edge_scroll);
    cleanup(dir);
}

#[test]
fn faction_button_and_deploy_start_the_selected_faction() {
    let (mut game, data_dir) = fresh_game("faction");
    click_button(&mut game, |action| matches!(action, Action::Setup));
    click_button(&mut game, |action| {
        matches!(action, Action::Faction(Faction::Assembly))
    });
    assert_eq!(game.faction, Faction::Assembly);

    click_button(&mut game, |action| matches!(action, Action::Start));
    assert_eq!(game.screen, Screen::Match);
    assert_eq!(game.world.players[0].faction, Faction::Assembly);
    assert_eq!(game.world.players[1].faction, Faction::Union);
    cleanup(data_dir);
}

#[test]
fn headquarters_train_button_submits_and_applies_a_worker_order() {
    let (mut game, data_dir) = fresh_game("train");
    game.start();
    game.world.ai_enabled = false;
    let headquarters = game.selected[0];
    click_button(&mut game, |action| {
        matches!(action, Action::Train(Kind::Hook))
    });
    let record = game.world.command_log.last().expect("train command log");
    assert!(record.accepted);
    assert!(matches!(
        record.command,
        Command::Train {
            building,
            kind: Kind::Hook
        } if building == headquarters
    ));
    assert_eq!(game.mode, Mode::Context);

    game.tick();
    game.tick();
    assert_eq!(entity(&game, headquarters).queue.len(), 1);
    cleanup(data_dir);
}

#[test]
fn worker_build_button_and_world_click_submit_a_build() {
    let (mut game, data_dir) = fresh_game("build");
    game.start();
    game.world.ai_enabled = false;
    let worker = worker_id(&game);
    game.selected = vec![worker];
    click_button(&mut game, |action| {
        matches!(action, Action::Build(Kind::Dropoff))
    });
    assert_eq!(game.mode, Mode::Build(Kind::Dropoff));

    // The footprint is centred on the pointed cell, so pointing one cell
    // south-east of the origin places a two-cell yard at the origin.
    let build_pos = Pos::cell(20, 63);
    let (x, y) = point_for(&game, Pos::cell(21, 64));
    game.left_down(x, y);
    assert_eq!(game.mode, Mode::Context);
    let record = game.world.command_log.last().expect("build command log");
    assert!(record.accepted);
    assert!(matches!(
        record.command,
        Command::Build {
            worker: id,
            kind: Kind::Dropoff,
            ..
        } if id == worker
    ));

    game.tick();
    game.tick();
    assert!(game.world.entities.iter().any(|entity| {
        entity.owner == 0 && entity.kind == Kind::Dropoff && entity.pos == build_pos
    }));
    cleanup(data_dir);
}

#[test]
fn click_release_selects_worker_and_control_group_recalls_it() {
    let (mut game, data_dir) = fresh_game("selection-group");
    game.start();
    game.world.ai_enabled = false;
    let worker = worker_id(&game);
    let pos = entity(&game, worker).pos;
    let (x, y) = point_for(&game, pos);
    game.left_down(x, y);
    game.left_up(x, y, false);
    assert_eq!(game.selected, vec![worker]);

    game.key("1", false, true);
    game.selected.clear();
    game.key("1", false, false);
    assert_eq!(game.selected, vec![worker]);
    cleanup(data_dir);
}

#[test]
fn hidden_enemy_click_cannot_select_or_reveal_a_unit() {
    let (mut game, data_dir) = fresh_game("hidden-selection");
    game.start();
    game.world.ai_enabled = false;
    let enemy = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 1 && !entity.kind.is_building())
        .expect("hidden enemy")
        .clone();
    assert!(!game.world.entity_visible(0, enemy.id));
    game.camera.center(enemy.pos);
    let (x, y) = point_for(&game, enemy.pos);
    game.left_down(x, y);
    game.left_up(x, y, false);
    assert!(game.selected.is_empty());
    assert!(!game.world.entity_visible(0, enemy.id));
    cleanup(data_dir);
}

#[test]
fn save_load_restores_hash_and_future_game_continuity() {
    let (mut game, data_dir) = fresh_game("save-load");
    game.start();
    game.world.ai_enabled = false;
    let worker = worker_id(&game);
    game.selected = vec![worker];
    let resource = game.world.map.resources[0].clone();
    let (x, y) = point_for(&game, resource.pos);
    game.right_click(x, y, false);
    assert!(matches!(
        game.world.command_log.last().map(|record| &record.command),
        Some(Command::Gather { units, resource: id }) if units == &vec![worker] && *id == resource.id
    ));
    game.tick();
    for _ in 0..80 {
        game.tick();
    }
    let expected = game.world.clone();
    let expected_hash = expected.state_hash();

    game.action(Action::Save);
    assert!(data_dir.join("saves/quick-save.json").exists());
    game.tick();
    assert_ne!(game.world.state_hash(), expected_hash);

    game.action(Action::Load);
    assert_eq!(game.screen, Screen::Confirm);
    game.action(Action::Confirm);
    assert_eq!(game.screen, Screen::Match);
    assert_eq!(game.world.state_hash(), expected_hash);
    let mut expected_next = expected.clone();
    for _ in 0..20 {
        game.tick();
        expected_next.step();
        assert_eq!(game.world.state_hash(), expected_next.state_hash());
    }
    cleanup(data_dir);
}

#[test]
fn new_match_first_click_is_single_selection() {
    let (mut game, data_dir) = fresh_game("click-reset");
    game.start();
    let worker = worker_id(&game);
    let pos = entity(&game, worker).pos;
    let (x, y) = point_for(&game, pos);
    game.left_down(x, y);
    game.left_up(x, y, false);
    assert_eq!(game.selected, vec![worker]);

    game.start();
    let replacement = worker_id(&game);
    let replacement_pos = entity(&game, replacement).pos;
    let (x, y) = point_for(&game, replacement_pos);
    game.left_down(x, y);
    game.left_up(x, y, false);
    assert_eq!(
        game.selected,
        vec![replacement],
        "starting a match must clear the prior double-click history"
    );
    cleanup(data_dir);
}

#[test]
fn minimap_edge_clicks_keep_camera_center_inside_map() {
    let (mut game, data_dir) = fresh_game("minimap-edge");
    game.start();
    for (x, y) in [(10, 299), (125, 352)] {
        game.left_down(x, y);
        let (cell_x, cell_y) = game.camera.unproject(320, 156).cell_xy();
        assert!((0..i32::from(game.world.map.width)).contains(&cell_x));
        assert!((0..i32::from(game.world.map.height)).contains(&cell_y));
    }
    cleanup(data_dir);
}

#[test]
fn gate_warning_keeps_current_state_and_foam_starts_on_change_tick() {
    let (mut game, data_dir) = fresh_game("gate-v8");
    game.start();
    game.world.ai_enabled = false;
    game.world.gate.owner = Some(0);
    game.action(Action::Switch);
    game.tick();
    assert!(game.world.gate.warning_until.is_some());
    assert_eq!(game.gate_foam_start, None);
    let current = game.world.gate.north_dry();

    for _ in 0..(bw_sim::GATE_WARNING_TICKS + 2) {
        if game.world.gate.warning_until.is_none() {
            break;
        }
        game.tick();
    }
    assert!(game.world.gate.warning_until.is_none());
    assert_eq!(game.world.gate.tide, bw_sim::Tide::Open);
    assert_eq!(game.gate_foam_start, Some(game.world.tick));
    // The lock is seen to turn when the dry side moves.
    if game.world.gate.north_dry() != current {
        assert_eq!(
            game.gate_switch,
            Some((game.world.tick, game.world.gate.north_dry()))
        );
    } else {
        assert_eq!(game.gate_switch, None);
    }

    // Render-only frames do not age a simulation-tick effect. The paused
    // match therefore keeps the same foam phase and event clocks.
    game.message.clear();
    let tick = game.world.tick;
    let effects = game.effects.len();
    game.screen = Screen::Pause;
    game.tick();
    game.render();
    game.render();
    assert_eq!(game.world.tick, tick);
    assert_eq!(game.effects.len(), effects);
    cleanup(data_dir);
}

#[test]
fn resource_stage_memory_does_not_follow_unseen_depletion() {
    let (mut game, data_dir) = fresh_game("resource-memory-v8");
    game.start();
    game.world.ai_enabled = false;
    let resource = game
        .world
        .map
        .resources
        .iter()
        .find(|resource| resource.pos.cell_xy().0 > 80)
        .expect("remote resource")
        .clone();
    for entity in game
        .world
        .entities
        .iter_mut()
        .filter(|entity| entity.owner == 0)
    {
        entity.pos = Pos::cell(10, 110);
    }
    game.render();
    assert!(!game.resource_stages.contains_key(&resource.id));

    let mut worker = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .expect("worker");
    worker.pos = resource.pos;
    game.render();
    assert_eq!(game.resource_stages.get(&resource.id), Some(&0));

    game.world
        .map
        .resources
        .iter_mut()
        .find(|candidate| candidate.id == resource.id)
        .expect("resource")
        .remaining = 0;
    worker = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .expect("worker");
    worker.pos = Pos::cell(10, 110);
    game.render();
    assert_eq!(game.resource_stages.get(&resource.id), Some(&0));

    worker = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .expect("worker");
    worker.pos = resource.pos;
    game.render();
    assert_eq!(game.resource_stages.get(&resource.id), Some(&3));
    cleanup(data_dir);
}

#[test]
fn never_observed_wreck_keeps_the_same_fogged_silhouette_after_hidden_depletion() {
    let (mut game, data_dir) = fresh_game("unknown-wreck-v8");
    game.start();
    game.world.ai_enabled = false;
    game.message.clear();
    let resource = game
        .world
        .map
        .resources
        .iter()
        .find(|r| r.pos.cell_xy().0 > 80)
        .unwrap()
        .clone();
    for entity in game.world.entities.iter_mut().filter(|e| e.owner == 0) {
        entity.pos = Pos::cell(10, 110);
    }
    game.camera.center(resource.pos);
    assert!(!game.world.visible(0, resource.pos));
    game.render();
    let before = game.canvas.pixels.clone();
    assert!(!game.resource_stages.contains_key(&resource.id));
    game.world
        .map
        .resources
        .iter_mut()
        .find(|r| r.id == resource.id)
        .unwrap()
        .remaining = 0;
    game.render();
    assert_eq!(
        game.canvas.pixels, before,
        "unseen depletion must not alter any fogged resource pixel"
    );
    assert!(!game.resource_stages.contains_key(&resource.id));
    cleanup(data_dir);
}

#[test]
fn an_actual_unseen_enemy_deposit_does_not_create_an_unload_clock() {
    let (mut game, data_dir) = fresh_game("hidden-deposit-v8");
    game.start();
    game.world.ai_enabled = false;
    let hq = game
        .world
        .entities
        .iter()
        .find(|e| e.owner == 1 && e.kind == Kind::Headquarters)
        .unwrap()
        .pos;
    let resource = game.world.map.resources[0].id;
    let worker = game
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 1 && e.kind.is_worker())
        .unwrap();
    let id = worker.id;
    worker.pos = Pos::raw(hq.x - 256, hq.y + 256);
    worker.path.clear();
    worker.path_index = 0;
    worker.carried = 5;
    worker.carried_kind = Some(bw_sim::ResourceKind::Salvage);
    worker.order = bw_sim::Order::Gather { resource };
    assert!(!game.world.entity_visible(0, id));
    let mut deposited = false;
    for _ in 0..180 {
        game.tick();
        if game
            .world
            .events
            .iter()
            .any(|e| e.kind == bw_sim::EventKind::Deposit && e.entity == Some(id))
        {
            deposited = true;
            break;
        }
    }
    assert!(
        deposited,
        "worker must reach its actual assigned dropoff slot"
    );
    assert!(!game.unload_starts.contains_key(&id));
    cleanup(data_dir);
}

#[test]
fn an_unseen_loom_that_fires_is_lit_and_its_shell_shows_where_it_came_from() {
    // Rules 13: a firing Loom is lit for the side it fires at, so the shot
    // animates at its source and the impact is drawn from it.  Before, the
    // source stayed hidden and Union never saw the Looms that killed it.
    let (mut game, data_dir) = fresh_game("hidden-shot-v8");
    game.start();
    game.world.ai_enabled = false;
    // Rules 21: the Loom reaches seven cells, eight with SIEGE, and the
    // target stands seven and a half away, past a worker's sight.
    game.world.players[1].upgrades = vec![bw_content::Upgrade::Siege];
    let target = worker_id(&game);
    let attacker = game
        .world
        .entities
        .iter()
        .find(|e| e.owner == 1 && e.kind.is_worker())
        .unwrap()
        .id;
    for entity in &mut game.world.entities {
        entity.order = bw_sim::Order::Idle;
        entity.path.clear();
        entity.path_index = 0;
        entity.path_target = None;
        if entity.owner == 0 {
            entity.pos = Pos::cell(10, 110);
        }
        if entity.id == target {
            entity.pos = Pos::raw(47 * 256 + 128, 40 * 256);
        }
        if entity.id == attacker {
            entity.pos = Pos::cell(40, 40);
            entity.kind = Kind::Loom;
            entity.hp = 160;
            entity.max_hp = 160;
            entity.deployed = true;
            entity.deploy_target = true;
            entity.order = bw_sim::Order::Attack { target };
        }
    }
    assert!(!game.world.entity_visible(0, attacker));
    game.tick();
    assert!(
        game.world
            .events
            .iter()
            .any(|e| e.kind == bw_sim::EventKind::ArtilleryWarning && e.entity == Some(attacker))
    );
    assert!(
        game.world.entity_visible(0, attacker),
        "the shot lights the Loom"
    );
    assert!(game.fire_starts.contains_key(&attacker));
    for _ in 0..bw_content::LOOM_WINDUP_TICKS {
        game.tick();
    }
    assert!(
        game.world
            .events
            .iter()
            .any(|e| e.kind == bw_sim::EventKind::Shot && e.entity == Some(attacker))
    );
    let hit = game
        .effects
        .iter()
        .find(|effect| !effect.death)
        .expect("visible target impact");
    assert_ne!(hit.from, hit.to, "the impact is drawn from the lit Loom");
    cleanup(data_dir);
}

#[test]
fn shot_uses_target_material_and_tick_stable_recoil() {
    let (mut game, data_dir) = fresh_game("combat-v8");
    game.start();
    game.world.ai_enabled = false;
    let attacker = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .map(|entity| entity.id)
        .expect("attacker");
    let target = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 1 && entity.kind.is_worker())
        .map(|entity| entity.id)
        .expect("target");
    for entity in &mut game.world.entities {
        if entity.id == attacker || entity.id == target {
            entity.pos = Pos::cell(48, 40);
        }
    }
    game.world
        .issue(
            0,
            Command::Attack {
                units: vec![attacker],
                target,
            },
        )
        .expect("attack command");
    game.tick();
    assert_eq!(game.fire_starts.get(&attacker), Some(&game.world.tick));
    let effect = game
        .effects
        .iter()
        .find(|effect| !effect.death)
        .expect("shot impact effect");
    assert_eq!(format!("{:?}", effect.material), "Reed");
    game.message.clear();
    game.render();
    let pixels = game.canvas.pixels.clone();
    game.render();
    assert_eq!(
        game.canvas.pixels, pixels,
        "render frames cannot age recoil"
    );
    for _ in 0..20 {
        game.tick();
    }
    assert!(game.effects.is_empty());
    assert!(!game.fire_starts.contains_key(&attacker));
    cleanup(data_dir);
}

#[test]
fn x_recycles_the_selected_workers() {
    // Trial 10: with every wreck gone, idle workers sat on crew with no
    // way back (rules 19).
    let (mut game, dir) = fresh_game("recycle");
    game.start();
    let worker = worker_id(&game);
    game.selected = vec![worker];
    game.key("X", false, false);
    assert!(
        game.message.starts_with("RECYCLE: 1 worker"),
        "{}",
        game.message
    );
    // The order reaches the world on its next tick.
    game.world.step();
    let order = game
        .world
        .entities
        .iter()
        .find(|e| e.id == worker)
        .map(|e| e.order.clone());
    assert!(
        matches!(order, Some(bw_sim::Order::Recycle { .. }) | None),
        "{order:?}"
    );
    cleanup(dir);
}

#[test]
fn a_resumed_seat_gets_its_control_groups_back() {
    // Control groups live only on a seat's screen; beside the live
    // recording they survive a restart (trial 10: "Group 1 is empty").
    let (mut game, dir) = fresh_game("groups");
    game.start();
    let worker = worker_id(&game);
    let saves = dir.join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    let write = |seed: u64, seat: u8| {
        let record = serde_json::json!({
            "seed": seed,
            "seat": seat,
            "tick": 39_300,
            "groups": [[], [worker, 999_999], [], [], [], [], [], [], [], []],
        });
        std::fs::write(saves.join("live-match-groups.json"), record.to_string()).unwrap();
    };
    let seed = game.world.seed;
    write(seed + 1, 0);
    game.restore_live_groups(seed, 0);
    assert!(game.groups[1].is_empty(), "another match's groups stay out");
    write(seed, 1);
    game.restore_live_groups(seed, 0);
    assert!(game.groups[1].is_empty(), "another seat's groups stay out");
    write(seed, 0);
    game.restore_live_groups(seed, 0);
    assert_eq!(game.groups[1], vec![worker], "only members still standing");
    cleanup(dir);
}
