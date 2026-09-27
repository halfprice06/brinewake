//! Controlled engine captures for the art pass v16: the tier-two machines,
//! the Drydock and the Palisade of each faction, and the new effects, each
//! seen on the field at 1x, 2x and 3x from both seats.  Nothing here changes
//! rules, saves or replays.
use crate::game::{Game, Screen};
use crate::zoom::Zoom;
use bw_core::{Faction, Kind, Pos};
use bw_sim::{Command, EventKind};
use std::path::{Path, PathBuf};

fn fresh(base: &Path, dir: &Path, faction: Faction) -> Game {
    let mut g = Game::new_with_data_dir(base.to_path_buf(), dir.join("session"));
    g.resize_view(1920, 1080);
    g.faction = faction;
    g.start();
    g.world.ai_enabled = false;
    g.world.revealed = true;
    g.message.clear();
    g.cursor = (-999, -999);
    g.screen = Screen::Match;
    g
}

fn shoot(g: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    g.screenshot(&dir.join(format!("{name}.png")))
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut g = fresh(&base, dir, faction);
        // Idle every starting worker so the field is calm.
        for id in g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect::<Vec<_>>()
        {
            let _ = g.world.issue(0, Command::Stop { units: vec![id] });
        }
        // A row of the six roles, each facing a different octant, with a
        // deployed Caisson and a loaded Dredger beside them.
        let roles = faction.drydock_roles();
        let mut ids = Vec::new();
        for (i, kind) in roles.into_iter().enumerate() {
            let id = g
                .world
                .spawn_for_tests(0, kind, Pos::cell(30 + i as i32 * 3, 60));
            if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
                e.facing = (i as u8 * 3) % 8;
            }
            ids.push(id);
        }
        for (i, kind) in faction.drydock_roles().into_iter().enumerate() {
            let id = g
                .world
                .spawn_for_tests(0, kind, Pos::cell(30 + i as i32 * 3, 64));
            if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
                e.facing = (i as u8 * 3 + 4) % 8;
                e.hp = e.max_hp / 3;
            }
        }
        if faction == Faction::Union {
            let caisson = g.world.spawn_for_tests(0, Kind::Caisson, Pos::cell(40, 62));
            if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == caisson) {
                e.deployed = true;
                e.facing = 2;
            }
        } else {
            let dredger = g.world.spawn_for_tests(0, Kind::Dredger, Pos::cell(40, 62));
            if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == dredger) {
                e.carried = 8;
                e.carried_kind = Some(bw_sim::ResourceKind::Salvage);
                e.facing = 6;
            }
        }
        // The Drydock finished, the Drydock under construction, a Palisade
        // line and a damaged Palisade.
        g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(28, 50));
        let site = g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(36, 50));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == site) {
            e.build_remaining = bw_content::spec(Kind::Drydock).build_ticks / 2;
            e.hp = e.max_hp / 2;
        }
        for i in 0..4 {
            let wall = g
                .world
                .spawn_for_tests(0, Kind::Palisade, Pos::cell(44, 54 + i));
            if i == 3
                && let Some(e) = g.world.entities.iter_mut().find(|e| e.id == wall)
            {
                e.hp = e.max_hp / 5;
            }
        }
        // Enemy copies for the red edge.
        for (i, kind) in faction.drydock_roles().into_iter().enumerate() {
            g.world
                .spawn_for_tests(1, kind, Pos::cell(30 + i as i32 * 3, 68));
        }
        g.world.step();
        let look = Pos::cell(36, 60);
        for (zoom, tag) in [
            (Zoom::Overview, "1x"),
            (Zoom::Wide, "2x"),
            (Zoom::Detail, "3x"),
        ] {
            g.set_zoom(zoom, g.world_view().center());
            g.camera.center(look);
            shoot(&mut g, dir, &format!("{name}-roles-{tag}"))?;
        }
        // A worker selected shows the command card; a role selected shows its
        // portrait, role line and stats.
        g.selected = vec![ids[0]];
        g.set_zoom(Zoom::Wide, g.world_view().center());
        g.camera.center(look);
        shoot(&mut g, dir, &format!("{name}-panel-scout"))?;
        g.selected = vec![ids[2]];
        shoot(&mut g, dir, &format!("{name}-panel-tide"))?;
        // Effects: SOUND from a Sounder or Skipper, VENT at the headquarters,
        // mending sparks from the mender.
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .unwrap();
        let sounder = g.world.spawn_for_tests(
            0,
            if faction == Faction::Union {
                Kind::Sounder
            } else {
                Kind::Skipper
            },
            Pos::cell(24, 60),
        );
        g.world.players[0].pressure = 300;
        g.world.players[0].pressure_cap = 300;
        let _ = g.world.issue(0, Command::Sound { unit: sounder });
        let _ = g.world.issue(0, Command::Vent { building: hq });
        for _ in 0..3 {
            g.tick();
        }
        assert!(
            g.world
                .events
                .iter()
                .all(|e| e.kind != EventKind::CommandRejected),
            "effects issued"
        );
        g.camera.center(Pos::cell(26, 60));
        shoot(&mut g, dir, &format!("{name}-effects-2x"))?;
        for _ in 0..6 {
            g.tick();
        }
        shoot(&mut g, dir, &format!("{name}-effects-later-2x"))?;
    }
    Ok(())
}
