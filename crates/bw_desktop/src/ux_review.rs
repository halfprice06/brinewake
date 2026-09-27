//! Reproducible UI journeys using normal Game actions and authoritative ticks.
//! Captures are controlled scenarios, not recordings of a human playtest.
use crate::game::{Action, Game, Screen};
use bw_core::{Faction, Kind, Pos};
use std::path::{Path, PathBuf};

fn click(game: &mut Game, action: Action) -> Result<(), String> {
    game.render();
    let b = game
        .buttons
        .iter()
        .find(|b| b.action == action && b.enabled)
        .cloned()
        .ok_or_else(|| format!("Expected enabled action {action:?} on {:?}", game.screen))?;
    game.left_down(b.x + b.w / 2, b.y + b.h / 2);
    Ok(())
}
fn capture(game: &mut Game, dir: Option<&Path>, name: &str) -> Result<(), String> {
    if let Some(dir) = dir {
        game.screenshot(&dir.join(format!("{name}.png")))?;
    }
    Ok(())
}
fn stage(game: &Game) -> usize {
    game.ux.tutorial.as_ref().map_or(0, |t| t.stage())
}
fn until_stage(game: &mut Game, wanted: usize, budget: usize) -> Result<(), String> {
    for _ in 0..budget {
        if stage(game) >= wanted {
            return Ok(());
        }
        game.tick();
    }
    Err(format!(
        "Practice stuck at step {} waiting for {}",
        stage(game) + 1,
        wanted + 1
    ))
}

pub fn first_session(game: &mut Game, dir: Option<&Path>) -> Result<(), String> {
    game.action(Action::MainMenu);
    capture(game, dir, "01-home")?;
    click(game, Action::Settings)?;
    capture(game, dir, "02-settings")?;
    click(game, Action::Back)?;
    click(game, Action::Setup)?;
    click(game, Action::Faction(Faction::Assembly))?;
    capture(game, dir, "03-skirmish-setup")?;
    click(game, Action::Back)?;
    game.faction = Faction::Union;
    for page in 0..crate::menus::GUIDE_PAGES {
        if page == 0 {
            click(game, Action::Help)?;
        }
        game.action(Action::GuidePage(page));
        capture(game, dir, &format!("04-guide-{page}"))?;
    }
    click(game, Action::Back)?;
    // Fresh Home's primary Enter action is safe practice.
    game.key("Enter", false, false);
    if game.screen != Screen::Match || game.world.ai_enabled {
        return Err("Home did not start safe practice".into());
    }
    capture(game, dir, "05-how-to-win")?;
    game.key("Space", false, false);
    if game.intro.is_some() {
        return Err("A key did not close the how-to-win card".into());
    }
    capture(game, dir, "05-practice-select")?;
    click(game, Action::FocusWorker)?;
    if stage(game) != 1 {
        return Err("Selecting a worker did not advance guidance".into());
    }
    capture(game, dir, "06-practice-gather")?;
    let wreck = game
        .world
        .map
        .resources
        .iter()
        .find(|r| game.world.visible(0, r.pos))
        .ok_or("No visible wreck")?
        .pos;
    let (x, y) = game.camera.project(wreck);
    game.right_click(x, y, false);
    until_stage(game, 2, 10)?;
    capture(game, dir, "07-practice-build")?;
    click(game, Action::Build(Kind::Works))?;
    let site = Pos::cell(20, 63);
    let (x, y) = game.camera.project(site);
    game.cursor = (x, y);
    capture(game, dir, "08-placement")?;
    game.left_down(x, y);
    game.tick();
    capture(game, dir, "09-construction")?;
    until_stage(game, 3, 2400)?;
    click(game, Action::FocusWorks)?;
    capture(game, dir, "10-practice-train")?;
    game.key("Q", false, false);
    game.tick();
    capture(game, dir, "11-production")?;
    until_stage(game, 4, 2400)?;
    click(game, Action::FocusArmy)?;
    capture(game, dir, "12-practice-move")?;
    let army = game.selected[0];
    let from = game
        .world
        .entities
        .iter()
        .find(|e| e.id == army)
        .ok_or("No army")?
        .pos;
    let (cx, cy) = from.cell_xy();
    let (x, y) = game.camera.project(Pos::cell(cx + 3, cy - 3));
    game.right_click(x, y, false);
    until_stage(game, 5, 300)?;
    capture(game, dir, "13-practice-capture")?;
    click(game, Action::Capture)?;
    until_stage(game, 6, 5000)?;
    click(game, Action::FocusGate)?;
    capture(game, dir, "14-practice-switch")?;
    click(game, Action::Switch)?;
    game.tick();
    capture(game, dir, "15-switch-warning")?;
    until_stage(game, 7, 400)?;
    capture(game, dir, "16-practice-complete")?;
    if !game.ux.preferences.tutorial_completed {
        return Err("Practice completion not persisted".into());
    }
    Ok(())
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let data = dir.join("fixture-user-data");
    // A unique subfolder prevents an old fixture save changing fresh Home.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let mut game = Game::new_with_data_dir(base.clone(), data.join(stamp.to_string()));
    first_session(&mut game, Some(dir))?;
    game.key("Escape", false, false);
    capture(&mut game, Some(dir), "17-pause")?;
    let saved = game.world.state_hash();
    click(&mut game, Action::Save)?;
    click(&mut game, Action::MainMenu)?;
    capture(&mut game, Some(dir), "18-continue-home")?;
    click(&mut game, Action::Resume)?;
    if game.world.state_hash() != saved {
        return Err("Continue changed the paused field".into());
    }
    game.tick();
    game.action(Action::Load);
    if game.screen != Screen::Confirm {
        return Err("Load did not protect current progress".into());
    }
    capture(&mut game, Some(dir), "19-load-confirmation")?;
    click(&mut game, Action::Confirm)?;
    if game.world.state_hash() != saved {
        return Err("Saved state did not restore".into());
    }
    capture(&mut game, Some(dir), "20-restored-practice")?;
    std::fs::remove_file(game.data_dir.join("saves/quick-save-ui.json"))
        .map_err(|e| e.to_string())?;
    game.action(Action::Load);
    capture(&mut game, Some(dir), "20b-practice-recovery")?;
    game.tick();
    game.key("Escape", false, false);
    game.action(Action::Surrender);
    if game.screen != Screen::Confirm {
        return Err("Ending unsaved practice did not offer to keep the field".into());
    }
    capture(&mut game, Some(dir), "20c-end-practice-confirmation")?;
    game.action(Action::DismissConfirm);
    game.action(Action::MainMenu);
    game.action(Action::Setup);
    game.tick(); // Menus must not advance the saved field.
    game.action(Action::Start);
    if game.screen == Screen::Confirm {
        game.action(Action::Confirm);
    }
    if !game.world.ai_enabled {
        return Err("Skirmish did not enable the computer".into());
    }
    game.action(Action::FocusWorker);
    game.world.players[0].salvage = 0;
    game.render();
    if let Some(b) = game
        .buttons
        .iter()
        .find(|b| b.action == Action::Build(Kind::Works))
    {
        game.cursor = (b.x + b.w / 2, b.y + b.h / 2);
    }
    capture(&mut game, Some(dir), "21-unavailable-command")?;
    game.action(Action::Surrender);
    capture(&mut game, Some(dir), "22-surrender-confirmation")?;
    game.action(Action::Confirm);
    game.tick();
    capture(&mut game, Some(dir), "23-result")?;
    let mut missing = Game::new_with_data_dir(base, game.data_dir.join("missing-save"));
    missing.action(Action::Load);
    capture(&mut missing, Some(dir), "24-missing-save")?;
    std::fs::write(dir.join("journey.txt"),"Controlled UI journey completed: fresh Home, settings/help/setup, seven practice steps using real commands and ticks, save/continue/load, skirmish, unavailable command, surrender/result, missing save. These are not human-playtest results.\n").map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_fresh_player_can_finish_all_seven_steps_through_game_controls() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = std::env::temp_dir().join(format!("brinewake-ux-journey-{}", std::process::id()));
        let mut game = Game::new_with_data_dir(root, dir.clone());
        first_session(&mut game, None).expect("complete first-player journey");
        assert!(!game.world.ai_enabled);
        assert!(game.ux.tutorial.as_ref().unwrap().is_complete());
        let _ = std::fs::remove_dir_all(dir);
    }
}
