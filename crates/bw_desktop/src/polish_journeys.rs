//! Controlled UI journeys for root review; no human-playtest claim.
use crate::audio::Bus;
use crate::dock_log::TelegraphStatus;
use crate::game::{Action, Game, Screen};
use bw_core::{Faction, Pos};
use bw_sim::{Command, Event, EventKind, Outcome};
use std::path::{Path, PathBuf};

fn capture(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    game.cursor = (638, 242);
    game.render();
    image::save_buffer(
        dir.join(format!("{name}.png")),
        &game.canvas.pixels,
        640,
        360,
        image::ColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut game = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-profile")));
        game.faction = faction;
        game.begin_practice();
        game.ux.guidance_visible = false;
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .ok_or("worker absent")?
            .id;
        game.selected = vec![id];
        let pos = game.world.entities.iter().find(|e| e.id == id).unwrap().pos;
        let (cx, cy) = pos.cell_xy();
        game.issue(Command::Move {
            units: vec![id],
            target: Pos::cell(cx + 2, cy),
            queued: false,
        });
        if game.ux.dock.telegraph.status != TelegraphStatus::Pending {
            return Err("submission falsely acknowledged".into());
        }
        capture(&mut game, dir, &format!("{name}-01-pending"))?;
        if !game.save_match() {
            return Err("pending command save failed".into());
        }
        game.load_match();
        game.tick();
        if game.ux.dock.telegraph.status != TelegraphStatus::Accepted {
            return Err("saved pending command did not acknowledge".into());
        }
        capture(&mut game, dir, &format!("{name}-02-accepted"))?;
        game.issue(Command::SwitchGate);
        if game.ux.dock.telegraph.status != TelegraphStatus::Rejected {
            return Err("unowned gate was not rejected".into());
        }
        capture(&mut game, dir, &format!("{name}-03-rejected"))?;
        game.action(Action::Settings);
        capture(&mut game, dir, &format!("{name}-04-settings"))?;
        for bus in [Bus::Effects, Bus::Ambience, Bus::Music] {
            game.action(Action::Volume(bus));
        }
        let prefs = [
            game.ux.preferences.effects_volume,
            game.ux.preferences.ambience_volume,
            game.ux.preferences.music_volume,
        ];
        let reloaded = crate::ux::UxState::load(&game.data_dir);
        if prefs
            != [
                reloaded.preferences.effects_volume,
                reloaded.preferences.ambience_volume,
                reloaded.preferences.music_volume,
            ]
        {
            return Err("volume persistence failed".into());
        }
        game.action(Action::Back);
        // Log fixtures explicitly represent observed public events; no match
        // win or gate control is claimed by these outcome layout examples.
        for (tick, kind, text) in [
            (30, EventKind::GateWarning, "north lane change announced"),
            (330, EventKind::GateChanged, "north flooded / south dry"),
            (420, EventKind::ResearchCompleted, "Fire Control complete"),
        ] {
            game.world.tick = tick;
            game.world.events = vec![Event {
                tick,
                kind,
                player: Some(0),
                entity: None,
                other: None,
                from: Some(game.world.map.gate_pos),
                to: None,
                amount: 0,
                text: text.into(),
                cause: None,
            }];
            game.ux.after_authoritative_tick(&game.world);
        }
        for (label, outcome) in [
            ("victory", Outcome::Victory(0)),
            ("defeat", Outcome::Victory(1)),
            ("draw", Outcome::Draw),
        ] {
            game.world.outcome = Some(outcome);
            game.ux.practice_review = false;
            game.aftermath_ticks = 0;
            let before = game.world.state_hash();
            for _ in 0..31 {
                game.tick();
            }
            if before != game.world.state_hash() {
                return Err("aftermath advanced the simulation".into());
            }
            capture(&mut game, dir, &format!("{name}-05-{label}-controlled"))?;
            for action in [
                Action::Start,
                Action::WatchReplay,
                Action::ExportReplay,
                Action::MainMenu,
            ] {
                if !game.buttons.iter().any(|b| b.action == action && b.enabled) {
                    return Err(format!("missing immediate result control {action:?}"));
                }
            }
        }
        game.world.outcome = None;
        game.ux.practice_review = true;
        game.ux.practice_complete = false;
        game.aftermath_ticks = 0;
        let hash = game.world.state_hash();
        for _ in 0..30 {
            game.tick();
        }
        game.key("A", false, false);
        if game.world.state_hash() != hash || game.mode != crate::game::Mode::Context {
            return Err("practice review accepts field input".into());
        }
        capture(&mut game, dir, &format!("{name}-06-practice-ended"))?;
        game.ux.practice_complete = true;
        capture(
            &mut game,
            dir,
            &format!("{name}-07-practice-complete-controlled"),
        )?;
        game.action(Action::EndReview);
        if game.screen != Screen::Menu || game.has_session() {
            return Err("End review did not close the field".into());
        }
        // A real command supplies the surrender result variant.
        game.start();
        game.world.ai_enabled = false;
        game.issue(Command::Surrender);
        game.tick();
        game.tick();
        if game.world.outcome != Some(Outcome::Victory(1)) {
            return Err("surrender result incorrect".into());
        }
        capture(&mut game, dir, &format!("{name}-08-surrender"))?;
    }
    std::fs::write(dir.join("evidence.txt"),"Both factions: pending/save/load/accept, immediate rejection, independent volume persistence, frozen aftermath with immediate controls, honest practice review, actual surrender. Victory/defeat/draw and history layouts use explicitly controlled states; they are not played match outcomes.\n").map_err(|e|e.to_string())
}
