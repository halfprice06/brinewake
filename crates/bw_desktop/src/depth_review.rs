//! Controlled tactical UI fixtures. These are not human-playtest evidence.
use crate::game::{Action, Game, Screen};
use bw_content::{Doctrine, spec};
use bw_core::{Faction, Kind, Pos};
use bw_sim::Order;
use std::path::{Path, PathBuf};

/// Development field: preserve valid IDs and construct both armies before tick 0.
/// Everything after this fixture setup goes through normal Game actions.
pub fn practice(game: &mut Game) -> Result<(), String> {
    game.start();
    game.world.ai_enabled = false;
    game.ux.practice = true;
    game.ux.tactics_fixture = true;
    game.ux.tutorial = None;
    game.ux.guidance_visible = false;
    for owner in 0..2u8 {
        let kinds = game.world.players[usize::from(owner)].faction.army();
        let ids: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.owner == owner && e.kind.is_worker())
            .take(3)
            .map(|e| e.id)
            .collect();
        for (i, id) in ids.into_iter().enumerate() {
            let e = game
                .world
                .entities
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or("fixture unit missing")?;
            e.kind = kinds[i];
            e.hp = spec(e.kind).health;
            e.max_hp = e.hp;
            e.pos = Pos::cell(if owner == 0 { 30 } else { 37 }, 58 + i as i32 * 3);
            e.facing = if owner == 0 { 2 } else { 6 };
            e.order = Order::Hold;
            e.carried = 0;
            e.carried_kind = None;
            e.gather_ticks = 0;
            e.path.clear();
            e.path_index = 0;
            e.path_target = None;
            e.waypoints.clear();
        }
        let player = &mut game.world.players[usize::from(owner)];
        player.salvage = 1500;
        player.pressure = 300;
        player.crew = game
            .world
            .entities
            .iter()
            .filter(|e| e.owner == owner)
            .map(|e| spec(e.kind).crew)
            .sum();
    }
    game.selected = game
        .world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && !e.kind.is_worker() && !e.kind.is_building())
        .map(|e| e.id)
        .collect();
    game.groups[1] = game.selected.clone();
    game.camera.center(Pos::cell(33, 61));
    game.world.reset_fixture_origin()?;
    game.notify("CONTROLLED TACTICS FIELD / F formation / Z Surge / R face / D deploy");
    Ok(())
}

fn click(game: &mut Game, action: Action) -> Result<(), String> {
    game.render();
    let button = game
        .buttons
        .iter()
        .find(|b| b.action == action && b.enabled)
        .cloned()
        .ok_or_else(|| format!("No enabled {action:?}"))?;
    game.left_down(button.x + button.w / 2, button.y + button.h / 2);
    Ok(())
}
fn capture(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    if game.mode != crate::game::Mode::Face {
        game.cursor = (635, 240);
    }
    game.screenshot(&dir.join(format!("{name}.png")))
}
fn ticks(game: &mut Game, n: usize) {
    for _ in 0..n {
        game.tick();
    }
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for faction in [Faction::Union, Faction::Assembly] {
        let label = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut game =
            Game::new_with_data_dir(base.clone(), dir.join(format!("{label}-user-data")));
        game.faction = faction;
        practice(&mut game)?;
        capture(&mut game, dir, &format!("{label}-01-controls"))?;
        click(&mut game, Action::Formation)?;
        ticks(&mut game, 2);
        click(&mut game, Action::Formation)?;
        ticks(&mut game, 2);
        capture(&mut game, dir, &format!("{label}-02-loose"))?;
        click(&mut game, Action::Face)?;
        let (x, y) = game.camera.project(Pos::cell(35, 61));
        game.cursor = (x, y);
        capture(&mut game, dir, &format!("{label}-03-face"))?;
        game.left_down(x, y);
        ticks(&mut game, 2);
        click(&mut game, Action::Surge)?;
        ticks(&mut game, 2);
        capture(&mut game, dir, &format!("{label}-04-surge"))?;
        let saved = game.world.state_hash();
        if !game.save_match() {
            return Err("depth fixture save failed".into());
        }
        game.tick();
        game.load_match();
        if !game.ux.tactics_fixture {
            return Err("prepared-field label did not survive save/load".into());
        }
        if game.world.state_hash() != saved {
            return Err("depth UI save/load changed active Surge".into());
        }
        game.selected = game
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && !e.kind.is_worker() && !e.kind.is_building())
            .map(|e| e.id)
            .collect();
        game.camera.center(Pos::cell(33, 61));
        ticks(&mut game, 100);
        capture(&mut game, dir, &format!("{label}-05-cooldown"))?;
        let specialist = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && matches!(e.kind, Kind::Loom | Kind::Bulwark))
            .ok_or("No specialist")?
            .id;
        game.selected = vec![specialist];
        click(&mut game, Action::Deploy)?;
        ticks(&mut game, 32);
        capture(&mut game, dir, &format!("{label}-06-deployed"))?;
        if faction == Faction::Assembly {
            for _ in 0..120 {
                if !game.world.artillery.is_empty() {
                    break;
                }
                game.tick();
            }
            if game.world.artillery.is_empty() {
                return Err("deployed Loom produced no warning fixture".into());
            }
            capture(&mut game, dir, "assembly-07-artillery-warning")?;
        }
        // Research uses a fresh safe field so its full duration isn't truncated by combat.
        let mut research =
            Game::new_with_data_dir(base.clone(), dir.join(format!("{label}-research-data")));
        research.faction = faction;
        research.start();
        research.world.ai_enabled = false;
        research.world.players[0].salvage = 1500;
        research.world.players[0].pressure = 300;
        research.world.reset_fixture_origin()?;
        research.home();
        let worker = faction.worker();
        click(&mut research, Action::Train(worker))?;
        ticks(&mut research, 2);
        click(&mut research, Action::Research(Doctrine::Hauling))?;
        ticks(&mut research, 2);
        let remaining = research
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .queue[0]
            .remaining;
        ticks(&mut research, 60);
        if research
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .queue[0]
            .remaining
            != remaining
        {
            return Err("HQ worker queue progressed during research".into());
        }
        capture(&mut research, dir, &format!("{label}-08-research-paused"))?;
        click(&mut research, Action::CancelResearch)?;
        ticks(&mut research, 2);
        click(&mut research, Action::Research(Doctrine::FireControl))?;
        ticks(&mut research, 905);
        if research.world.players[0].doctrine != Some(Doctrine::FireControl) {
            return Err("research did not complete".into());
        }
        capture(&mut research, dir, &format!("{label}-09-doctrine-complete"))?;
        for page in 0..crate::menus::GUIDE_PAGES {
            research.screen = Screen::Help;
            research.action(Action::GuidePage(page));
            capture(&mut research, dir, &format!("{label}-guide-{page}"))?;
        }
    }
    std::fs::write(dir.join("evidence.txt"), "Controlled UI fixtures: both factions, formation controls, facing, Surge and cooldown, specialist deployment, Loom warning, research worker pause/cancellation/completion, active Surge UI save/load, seven guide pages. Actions use normal commands after explicitly prepared fields. These are not human fun/balance evidence.\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Mode;
    use bw_sim::{ArtilleryShot, Formation};
    fn fixture() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!("brinewake-depth-ui-{}", std::process::id()));
        let mut game = Game::new_with_data_dir(base, data);
        practice(&mut game).unwrap();
        game
    }
    #[test]
    fn tactical_mouse_and_keys_submit_commands_and_show_cooldown() {
        let mut game = fixture();
        game.render();
        let count = game.world.command_log.len();
        game.right_click(284, 38, false);
        assert_eq!(
            game.world.command_log.len(),
            count,
            "tactical HUD right click issued a ground order"
        );
        click(&mut game, Action::Formation).unwrap();
        ticks(&mut game, 2);
        assert!(
            game.world
                .entities
                .iter()
                .filter(|e| game.selected.contains(&e.id))
                .all(|e| e.formation == Formation::Line)
        );
        game.key("R", false, false);
        assert_eq!(game.mode, Mode::Face);
        let (x, y) = game.camera.project(Pos::cell(34, 61));
        game.left_down(x, y);
        ticks(&mut game, 2);
        let p = game.world.players[0].pressure;
        game.key("Z", false, false);
        ticks(&mut game, 2);
        assert_eq!(game.world.players[0].pressure, p - 30);
        assert!(
            game.action_reason(&Action::Surge)
                .unwrap()
                .contains("ready in")
        );
        // A capture order chains after the surge since rules 14.
        assert!(game.action_reason(&Action::Capture).is_none());
        assert!(
            game.action_reason(&Action::Deploy)
                .unwrap()
                .contains("Surge")
        );
    }
    #[test]
    fn hidden_artillery_warning_does_not_change_pixels_or_render_advance_time() {
        let mut game = fixture();
        let target = Pos::cell(90, 90);
        assert!(!game.world.visible(0, target));
        game.camera.center(target);
        game.render();
        let before = game.canvas.pixels.clone();
        game.world.artillery.push(ArtilleryShot {
            owner: 1,
            source: 9,
            from: Pos::cell(95, 95),
            target,
            impact_tick: 24,
        });
        let tick = game.world.tick;
        game.render();
        assert_eq!(game.world.tick, tick);
        assert_eq!(
            game.canvas.pixels, before,
            "hidden incoming shot leaked through overlay"
        );
        game.world.artillery[0].owner = 0;
        game.render();
        assert_ne!(
            game.canvas.pixels, before,
            "own committed warning should remain visible"
        );
    }
    #[test]
    fn research_controls_explain_affordability_and_exclusive_choice() {
        let mut game = fixture();
        game.home();
        game.world.players[0].pressure = 0;
        assert!(
            game.action_reason(&Action::Research(Doctrine::Hauling))
                .unwrap()
                .contains("pressure")
        );
        game.world.players[0].pressure = 300;
        click(&mut game, Action::Research(Doctrine::Hauling)).unwrap();
        ticks(&mut game, 2);
        assert!(
            game.action_reason(&Action::Research(Doctrine::FireControl))
                .unwrap()
                .contains("researching")
        );
        assert!(game.action_reason(&Action::CancelResearch).is_none());
        assert!(
            !game
                .action_reason(&Action::Cancel)
                .unwrap()
                .contains("research")
        );
    }

    #[test]
    fn retained_impact_disappears_when_its_ground_point_loses_vision() {
        let mut game = fixture();
        let target = Pos::cell(90, 90);
        let source = game.selected[0];
        let original = game
            .world
            .entities
            .iter()
            .find(|e| e.id == source)
            .unwrap()
            .pos;
        game.world
            .entities
            .iter_mut()
            .find(|e| e.id == source)
            .unwrap()
            .pos = target;
        assert!(game.world.visible(0, target));
        game.camera.center(target);
        game.effects.push(crate::game::Effect {
            from: target,
            to: target,
            death: false,
            material: crate::presentation::material_for_faction(Faction::Union),
            faction: Faction::Union,
            muzzle: None,
            beam: None,
            until: 12,
        });
        game.render();
        game.world
            .entities
            .iter_mut()
            .find(|e| e.id == source)
            .unwrap()
            .pos = original;
        assert!(!game.world.visible(0, target));
        game.render();
        let with_hidden_effect = game.canvas.pixels.clone();
        game.effects.clear();
        game.render();
        assert_eq!(game.canvas.pixels, with_hidden_effect);
    }
}
