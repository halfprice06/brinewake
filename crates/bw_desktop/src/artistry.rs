//! Tidal-artistry presentation helpers.
//!
//! The simulation remains the source of truth for every decision represented
//! here.  These helpers only inspect a [`bw_sim::World`], choose a deterministic
//! atlas key, and rasterize an already-authored sprite.  They deliberately do
//! not advance clocks, write observations, or alter entities.  The game layer
//! may call them from its existing ground and entity passes.

use crate::canvas::{Atlas, Canvas};
use bw_content::{Doctrine, LOOM_WINDUP_TICKS, SURGE_COOLDOWN_TICKS, SURGE_TICKS};
use bw_core::{Camera, FP, Faction, Kind, Pos, Terrain};
use bw_sim::{Entity, World};

const ARCH_HALF_CELLS: i32 = 2;
const ARCH_FRAME_TICKS: u64 = 12;
const PRESSURE_PHASES: u32 = 4;
const COOL_PHASES: u32 = 3;
const DOCTRINE_FRAME_TICKS: u64 = 6;
const COAST_FRAME_TICKS: u64 = 6;
const COAST_REST_TICKS: u64 = 150;
const COAST_PERFORMANCE_TICKS: u64 = 8 * COAST_FRAME_TICKS;
const COAST_SAFE_RADIUS: i32 = 2;
/// A disturbed coast site stays empty this long after the last machine left.
pub const COAST_RETURN_TICKS: u64 = 600;

#[derive(Clone, Copy)]
struct SpriteContract {
    width: u32,
    height: u32,
    anchor_x: i32,
    anchor_y: i32,
}

const ARCHAEOLOGY_CONTRACT: SpriteContract = SpriteContract {
    width: 160,
    height: 80,
    anchor_x: 80,
    anchor_y: 40,
};
const COAST_CONTRACT: SpriteContract = SpriteContract {
    width: 96,
    height: 64,
    anchor_x: 48,
    anchor_y: 52,
};
const UNIT_CONTRACT: SpriteContract = SpriteContract {
    width: 64,
    height: 64,
    anchor_x: 32,
    anchor_y: 50,
};
const HQ_CONTRACT: SpriteContract = SpriteContract {
    width: 160,
    height: 144,
    anchor_x: 80,
    anchor_y: 128,
};

/// Authored ground patches.  The five-by-five footprints are deliberately
/// listed in map cells instead of inferred from a texture rectangle so a
/// future map edit cannot make an image cross a bank or a lane boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArchaeologyPatch {
    pub center: (i32, i32),
    pub lane: Terrain,
    pub name: &'static str,
}

/// North is the ferry court; south is the old ropewalk.  The patch centres
/// match the owner-authored atlas contract and all use the same 80,40 anchor.
pub const ARCHAEOLOGY_PATCHES: [ArchaeologyPatch; 4] = [
    ArchaeologyPatch {
        center: (58, 49),
        lane: Terrain::Lane0,
        name: "north_a",
    },
    ArchaeologyPatch {
        center: (69, 49),
        lane: Terrain::Lane0,
        name: "north_b",
    },
    ArchaeologyPatch {
        center: (58, 79),
        lane: Terrain::Lane1,
        name: "south_a",
    },
    ArchaeologyPatch {
        center: (69, 79),
        lane: Terrain::Lane1,
        name: "south_b",
    },
];

/// Authored coastal vignettes.  Their positions are on the existing salt
/// approaches, outside both lanes and the central deep cut.  They are static
/// sites whose frames are driven by the public tide state and presentation
/// time; no entity position is used to choose a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoastSite {
    pub name: &'static str,
    pub pos: (i32, i32),
    pub phase: u8,
}

pub const COAST_SITES: [CoastSite; 3] = [
    CoastSite {
        name: "birds",
        pos: (49, 44),
        phase: 0,
    },
    CoastSite {
        name: "crab",
        pos: (49, 88),
        phase: 3,
    },
    CoastSite {
        name: "reeds",
        pos: (79, 88),
        phase: 6,
    },
];

/// Draw all valid tidal-archaeology patches beneath actors.
///
/// A patch is drawn only when every one of its 25 cells is the expected lane
/// and every cell has the same visibility state.  This conservative rule keeps
/// a texture rectangle from crossing a bank or making a hidden crossing unit
/// legible at a fog boundary.  Hidden-but-known terrain can still receive the
/// atlas' ordinary dim treatment.
pub fn draw_ground(canvas: &mut Canvas, atlas: &Atlas, world: &World, camera: Camera) -> usize {
    draw_ground_impl(canvas, atlas, world, camera, world.tick)
}

/// Draw archaeology using an explicit presentation tick.  The ordinary
/// in-match path uses `World::tick`; a terminal harbor treatment can pass its
/// own frozen-world cosmetic clock without changing the replay state.
pub fn draw_ground_at_tick(
    canvas: &mut Canvas,
    atlas: &Atlas,
    world: &World,
    camera: Camera,
    cosmetic_tick: u64,
) -> usize {
    if cosmetic_tick == world.tick {
        return draw_ground(canvas, atlas, world, camera);
    }
    draw_ground_impl(canvas, atlas, world, camera, cosmetic_tick)
}

fn draw_ground_impl(
    canvas: &mut Canvas,
    atlas: &Atlas,
    world: &World,
    camera: Camera,
    cosmetic_tick: u64,
) -> usize {
    let mut drawn = 0;
    if !split_basin(world) {
        return drawn;
    }
    for (index, patch) in ARCHAEOLOGY_PATCHES.iter().enumerate() {
        let Some(visible) = archaeology_visibility(world, *patch) else {
            continue;
        };
        // A Salter's crust over the patch hides the old court beneath it.
        let (cx, cy) = patch.center;
        if !world.crust.is_empty()
            && (-3..=3).any(|dx| (-3..=3).any(|dy| world.is_crusted(cx + dx, cy + dy)))
        {
            continue;
        }
        let key = archaeology_key_at_tick(world, index, cosmetic_tick);
        let (x, y) = camera.project(Pos::cell(patch.center.0, patch.center.1));
        if draw_contract(atlas, canvas, &key, x, y, ARCHAEOLOGY_CONTRACT, !visible) {
            drawn += 1;
        }
    }
    drawn
}

/// Draw the three authored coast vignettes beneath actors and taller props.
///
/// The footprint guard keeps the 96-by-64 sprites on quiet, non-lane shore
/// cells.  A nearby observed actor suppresses the vignette for that frame;
/// hidden enemy actors are intentionally not queried, so the cosmetic cannot
/// become a fog side channel.  Since callers place this pass before entities,
/// actors still win depth even when a site is adjacent to a route.
/// Kept for fixtures and review exporters; the game draws coast sites
/// through `draw_coast_flush`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn draw_coast(canvas: &mut Canvas, atlas: &Atlas, world: &World, camera: Camera) -> usize {
    draw_coast_impl(canvas, atlas, world, camera, world.tick)
}

fn draw_coast_impl(
    canvas: &mut Canvas,
    atlas: &Atlas,
    world: &World,
    camera: Camera,
    cosmetic_tick: u64,
) -> usize {
    let mut drawn = 0;
    if !split_basin(world) {
        return drawn;
    }
    for (index, site) in COAST_SITES.iter().enumerate() {
        let pos = Pos::cell(site.pos.0, site.pos.1);
        if !coast_footprint_is_safe(world, *site) || coast_is_occupied(world, pos) {
            continue;
        }
        let key = coast_key_at_tick(world, index, cosmetic_tick);
        let (x, y) = camera.project(pos);
        if draw_contract(
            atlas,
            canvas,
            &key,
            x,
            y,
            COAST_CONTRACT,
            !world.visible(0, pos),
        ) {
            drawn += 1;
        }
    }
    drawn
}

/// Draw the coast sites with their reactions to machines.  A site the game
/// has marked as flushed plays its take-off or hide strip from the flush
/// tick and then holds its empty frame until the game clears it; undisturbed
/// sites behave as before.  The flush map is presentation state only.
pub fn draw_coast_flush(
    canvas: &mut Canvas,
    atlas: &Atlas,
    world: &World,
    camera: Camera,
    cosmetic_tick: u64,
    flush: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
) -> usize {
    let mut drawn = 0;
    if !split_basin(world) {
        return drawn;
    }
    for (index, site) in COAST_SITES.iter().enumerate() {
        let pos = Pos::cell(site.pos.0, site.pos.1);
        if !coast_footprint_is_safe(world, *site) {
            continue;
        }
        let key = match (site.name, flush.get(&index)) {
            ("birds", Some((flushed, _))) => {
                let age = cosmetic_tick.saturating_sub(*flushed);
                format!("coast_birds_fly_{}", (age / COAST_FRAME_TICKS).min(4))
            }
            ("crab", Some((flushed, _))) => {
                let age = cosmetic_tick.saturating_sub(*flushed);
                format!("coast_crab_hide_{}", (age / COAST_FRAME_TICKS).min(1))
            }
            (_, Some(_)) => continue,
            (_, None) => {
                if coast_is_occupied(world, pos) {
                    continue;
                }
                coast_key_at_tick(world, index, cosmetic_tick)
            }
        };
        let (x, y) = camera.project(pos);
        if draw_contract(
            atlas,
            canvas,
            &key,
            x,
            y,
            COAST_CONTRACT,
            !world.visible(0, pos),
        ) {
            drawn += 1;
        }
    }
    drawn
}

/// Draw transparent pressure and completed-doctrine overlays at an entity's
/// existing ground anchor.  The caller should invoke this immediately after
/// the entity's base sprite.  A missing root-authored key is a no-op, allowing
/// old atlas bundles to remain playable while the new art is authored.
pub fn draw_entity_overlay(
    canvas: &mut Canvas,
    atlas: &Atlas,
    world: &World,
    entity: &Entity,
    x: i32,
    y: i32,
) -> bool {
    // This guard makes the helper safe to call independently of the draw list;
    // it prevents a direct caller from exposing a hidden enemy pressure state.
    if entity.hp <= 0 || !world.entity_visible(0, entity.id) {
        return false;
    }

    let mut drawn = false;
    if let Some(key) = pressure_key(entity) {
        drawn |= draw_contract(atlas, canvas, &key, x, y, UNIT_CONTRACT, false);
    }
    if let Some(key) = doctrine_key(world, entity) {
        drawn |= draw_contract(atlas, canvas, &key, x, y, UNIT_CONTRACT, false);
    }
    if let Some(key) = headquarters_doctrine_key(world, entity) {
        drawn |= draw_contract(atlas, canvas, &key, x, y, HQ_CONTRACT, false);
    }
    drawn
}

/// Return the current full Loom pressure-performance key, if a committed shot
/// is in flight.  The animation is derived from the authoritative 24-tick
/// warning interval, so it cannot promise an earlier shot or extend impact
/// timing.  Hidden Loom sources return no key even if a caller holds a stale
/// entity reference.
pub fn entity_animation_key(world: &World, entity: &Entity) -> Option<String> {
    if entity.kind != Kind::Loom
        || entity.hp <= 0
        || !entity.deployed
        || entity.deploy_remaining > 0
        || entity.surge_remaining > 0
        || !world.entity_visible(0, entity.id)
    {
        return None;
    }
    let shot = world
        .artillery
        .iter()
        .filter(|shot| shot.source == entity.id && shot.owner == entity.owner)
        .min_by_key(|shot| (shot.impact_tick, shot.target.y, shot.target.x))?;
    let windup = u64::from(LOOM_WINDUP_TICKS);
    let start_tick = shot.impact_tick.checked_sub(windup)?;
    let elapsed = world.tick.checked_sub(start_tick)?;
    if elapsed >= windup {
        return None;
    }
    let phase = match elapsed {
        0..4 => 0,
        4..8 => 1,
        8..11 => 2,
        11..15 => 3,
        15..19 => 4,
        19..24 => 5,
        _ => return None,
    };
    Some(format!(
        "loom_{}_pressure_{phase}",
        atlas_face(entity.facing)
    ))
}

/// Public key helper used by review fixtures and tests.  The key changes on
/// the same authoritative tick as `north_dry`; render frames never advance it.
pub fn archaeology_key(world: &World, patch_index: usize) -> String {
    archaeology_key_impl(world, patch_index, world.tick)
}

/// Return an archaeology key for an explicit presentation tick.
pub fn archaeology_key_at_tick(world: &World, patch_index: usize, cosmetic_tick: u64) -> String {
    if cosmetic_tick == world.tick {
        return archaeology_key(world, patch_index);
    }
    archaeology_key_impl(world, patch_index, cosmetic_tick)
}

fn archaeology_key_impl(world: &World, patch_index: usize, cosmetic_tick: u64) -> String {
    let patch = ARCHAEOLOGY_PATCHES
        .get(patch_index)
        .copied()
        .unwrap_or(ARCHAEOLOGY_PATCHES[0]);
    let wet = crate::presentation::cell_is_wet(world, patch.lane);
    let lane = match patch.lane {
        Terrain::Lane0 => "north",
        Terrain::Lane1 => "south",
        _ => "unknown",
    };
    let state = if wet { "wet" } else { "dry" };
    let phase = ((cosmetic_tick / ARCH_FRAME_TICKS) + patch_index as u64) % 4;
    format!("archaeology_{lane}_{state}_{phase}")
}

/// Return the deterministic frame key for one public coast site.
pub fn coast_key(world: &World, site_index: usize) -> String {
    coast_key_impl(world, site_index, world.tick)
}

/// Return a coast key for an explicit presentation tick.  The first eight
/// frames are a short performance; the following 150 ticks hold on frame 0,
/// leaving the shoreline quiet between public-tide or scheduled moments.
pub fn coast_key_at_tick(world: &World, site_index: usize, cosmetic_tick: u64) -> String {
    if cosmetic_tick == world.tick {
        return coast_key(world, site_index);
    }
    coast_key_impl(world, site_index, cosmetic_tick)
}

fn coast_key_impl(world: &World, site_index: usize, cosmetic_tick: u64) -> String {
    let site = COAST_SITES
        .get(site_index)
        .copied()
        .unwrap_or(COAST_SITES[0]);
    // The tide staggers the performances: 0 while arm 0 (the north) is the
    // dry arm, 29 ticks while arm 1 is.
    let tide_offset = world.gate.dry_arm.index() as u64 * 29;
    let site_offset = u64::from(site.phase) * 37;
    let cycle_seed = cosmetic_tick
        .saturating_add(site_offset)
        .saturating_add(tide_offset)
        .saturating_add(u64::from(world.gate.lane_revision).saturating_mul(47));
    let cycle = cycle_seed % (COAST_PERFORMANCE_TICKS + COAST_REST_TICKS);
    let phase = if cycle < COAST_PERFORMANCE_TICKS {
        (cycle / COAST_FRAME_TICKS).min(7)
    } else {
        0
    };
    format!("coast_{}_{phase}", site.name)
}

/// Return the currently completed doctrine attachment key for a worker.
/// Completed research is the only source; an in-progress research field does
/// not produce a finished kit.  Owner zero is intentionally the only owner.
pub fn doctrine_key(world: &World, entity: &Entity) -> Option<String> {
    if entity.owner != 0 || !entity.kind.is_worker() {
        return None;
    }
    if world.players[0].research.is_some() {
        return None;
    }
    let doctrine = world.players[0].doctrine?;
    // The authored worker attachment is the Hauling cargo cradle. Fire
    // Control has a headquarters plate only, so it must not invent a worker
    // key that the atlas contract does not provide.
    if doctrine != Doctrine::Hauling {
        return None;
    }
    let face = atlas_face(entity.facing);
    let frame = doctrine_frame(world, entity.id);
    Some(format!(
        "doctrine_{}_{}_{}_{frame}",
        entity.kind.asset(world.players[0].faction),
        face,
        doctrine_key_name(doctrine)
    ))
}

/// Return the currently completed headquarters doctrine plate key.
pub fn headquarters_doctrine_key(world: &World, entity: &Entity) -> Option<String> {
    if entity.owner != 0 || entity.kind != Kind::Headquarters {
        return None;
    }
    if world.players[0].research.is_some() {
        return None;
    }
    let doctrine = world.players[0].doctrine?;
    let faction = match world.players[0].faction {
        Faction::Union => "union",
        Faction::Assembly => "assembly",
        Faction::Compact => "compact",
    };
    let frame = doctrine_frame(world, entity.id);
    Some(format!(
        "doctrine_{faction}_{}_hq_{frame}",
        doctrine_key_name(doctrine)
    ))
}

fn doctrine_key_name(doctrine: Doctrine) -> &'static str {
    match doctrine {
        Doctrine::Hauling => "hauling",
        Doctrine::FireControl => "fire_control",
    }
}

fn doctrine_frame(world: &World, id: u32) -> u64 {
    (world.tick / DOCTRINE_FRAME_TICKS + u64::from(id)) % 4
}

fn pressure_key(entity: &Entity) -> Option<String> {
    let role = match entity.kind {
        Kind::Riveter => "riveter",
        Kind::Bulwark => "bulwark",
        Kind::Sounder => "sounder",
        Kind::Skipper => "skipper",
        Kind::Reedguard => "reedguard",
        Kind::Loom => "loom",
        _ => return None,
    };
    let (mode, phase) = if entity.surge_remaining > 0 {
        let elapsed = SURGE_TICKS.saturating_sub(entity.surge_remaining);
        (
            "surge",
            // Each pressure jet frame is a 3-tick (100ms) loop while the
            // authoritative three-second Surge interval remains active.
            (elapsed / 3) % PRESSURE_PHASES,
        )
    } else if entity.surge_cooldown > 0 {
        let elapsed = SURGE_COOLDOWN_TICKS
            .saturating_sub(entity.surge_cooldown)
            .saturating_sub(SURGE_TICKS);
        (
            "cool",
            // Cooling starts when the three-second Surge ends.  The three
            // cooling frames hold for 6 ticks (200ms) and loop while the
            // authoritative cooldown continues.
            (elapsed / 6) % COOL_PHASES,
        )
    } else {
        return None;
    };
    Some(format!(
        "pressure_{role}_{}_{}_{phase}",
        atlas_face(entity.facing),
        mode
    ))
}

fn atlas_face(facing: u8) -> u8 {
    // Simulation octants start at world north and turn clockwise; the
    // authored atlas facings start at screen east and turn counter-clockwise.
    (9 - facing % 8) % 8
}

fn archaeology_visibility(world: &World, patch: ArchaeologyPatch) -> Option<bool> {
    let mut visibility = None;
    for dy in -ARCH_HALF_CELLS..=ARCH_HALF_CELLS {
        for dx in -ARCH_HALF_CELLS..=ARCH_HALF_CELLS {
            let x = patch.center.0 + dx;
            let y = patch.center.1 + dy;
            let pos = Pos::cell(x, y);
            if world.map.terrain(x, y) != patch.lane {
                return None;
            }
            let cell_visible = world.visible(0, pos);
            if let Some(previous) = visibility {
                if previous != cell_visible {
                    return None;
                }
            } else {
                visibility = Some(cell_visible);
            }
        }
    }
    visibility
}

/// The archaeology patches and the coast sites stand at the Split Basin's
/// cells: its lane courts and its salt approaches.  On another map those
/// cells are other ground (on the Confluence, seat T's headquarters and the
/// station island), so they are not drawn there.
fn split_basin(world: &World) -> bool {
    world.map.id.is_split_basin()
}

fn coast_footprint_is_safe(world: &World, site: CoastSite) -> bool {
    let (cx, cy) = site.pos;
    if !coast_cell_safe(world, cx, cy) {
        return false;
    }
    for dy in -COAST_SAFE_RADIUS..=COAST_SAFE_RADIUS {
        for dx in -COAST_SAFE_RADIUS..=COAST_SAFE_RADIUS {
            if !coast_cell_safe(world, cx + dx, cy + dy) {
                return false;
            }
        }
    }
    true
}

fn coast_cell_safe(world: &World, x: i32, y: i32) -> bool {
    matches!(world.map.terrain(x, y), Terrain::Salt | Terrain::Silt)
}

pub(crate) fn coast_is_occupied(world: &World, pos: Pos) -> bool {
    let radius = i64::from(FP * COAST_SAFE_RADIUS);
    world.entities.iter().any(|entity| {
        entity.hp > 0
            && (entity.owner == 0 || world.entity_visible(0, entity.id))
            && entity.pos.distance_sq(pos) <= radius.saturating_mul(radius)
    })
}

fn draw_contract(
    atlas: &Atlas,
    canvas: &mut Canvas,
    key: &str,
    x: i32,
    y: i32,
    contract: SpriteContract,
    dim: bool,
) -> bool {
    let Some(sprite) = atlas.sprites.get(key) else {
        return false;
    };
    if sprite.w != contract.width
        || sprite.h != contract.height
        || sprite.anchor_x != contract.anchor_x
        || sprite.anchor_y != contract.anchor_y
    {
        return false;
    }
    atlas.draw(canvas, key, x, y, dim)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coastal_vignettes_do_not_share_interactive_or_landmark_anchors() {
        let world = World::new(1, Faction::Union);
        for site in COAST_SITES {
            let pos = Pos::cell(site.pos.0, site.pos.1);
            for other in world
                .map
                .wells
                .iter()
                .copied()
                .chain(crate::presentation::LANDMARKS.iter().map(|p| p.pos))
            {
                assert!(
                    pos.distance_sq(other) >= i64::from(FP * 4).pow(2),
                    "{} is hidden by an existing prop",
                    site.name
                );
            }
        }
    }
    use bw_sim::ArtilleryShot;
    use std::path::PathBuf;

    fn base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn atlas() -> Atlas {
        Atlas::load(&base()).expect("current atlas remains loadable")
    }

    #[test]
    fn ground_and_coast_are_deterministic_read_only_and_pause_stable() {
        let atlas = atlas();
        let world = World::new(901, Faction::Union);
        let before = world.state_hash();
        let camera = Camera::default();
        let mut first = Canvas::default();
        let mut second = Canvas::default();
        first.clear([38, 61, 68, 255]);
        second.clear([38, 61, 68, 255]);
        let drawn_ground = draw_ground(&mut first, &atlas, &world, camera);
        let drawn_coast = draw_coast(&mut first, &atlas, &world, camera);
        draw_ground(&mut second, &atlas, &world, camera);
        draw_coast(&mut second, &atlas, &world, camera);
        assert_eq!(world.state_hash(), before);
        assert_eq!(world.tick, 0);
        assert_eq!(first.pixels, second.pixels);
        // Every contracted key is present in the current root-authored atlas;
        // if a future bundle omits one, the helper remains a safe no-op for it.
        assert_eq!(drawn_ground, ARCHAEOLOGY_PATCHES.len());
        assert_eq!(drawn_coast, COAST_SITES.len());
    }

    #[test]
    fn ground_pass_can_render_past_the_design_canvas_edge() {
        let atlas = atlas();
        let world = World::new(906, Faction::Union);
        let mut camera = Camera::default();
        camera.center(Pos::cell(58, 49));
        camera.x -= 520;
        let mut canvas = Canvas::new(1280, 552);

        assert_eq!(
            draw_ground(&mut canvas, &atlas, &world, camera),
            ARCHAEOLOGY_PATCHES.len()
        );
        let old_edge_has_pixels = (640..canvas.width()).any(|x| {
            (0..canvas.height()).any(|y| {
                let index = ((y * canvas.width() + x) * 4) as usize;
                canvas.pixels[index + 3] != 0
            })
        });
        assert!(
            old_edge_has_pixels,
            "wide artistry pass was clipped at x=640"
        );
    }

    #[test]
    fn tide_switch_changes_archaeology_state_on_the_same_tick() {
        let mut world = World::new(902, Faction::Union);
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(true);
        assert_eq!(archaeology_key(&world, 0), "archaeology_north_dry_0");
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(false);
        assert_eq!(archaeology_key(&world, 0), "archaeology_north_wet_0");
        world.tick = ARCH_FRAME_TICKS;
        assert_eq!(archaeology_key(&world, 0), "archaeology_north_wet_1");
    }

    #[test]
    fn mixed_fog_or_lane_cells_suppress_a_whole_archaeology_patch() {
        let mut world = World::new(903, Faction::Union);
        let patch = ARCHAEOLOGY_PATCHES[0];
        assert!(archaeology_visibility(&world, patch).is_some());
        world.map.tiles
            [patch.center.1 as usize * world.map.width as usize + patch.center.0 as usize] =
            Terrain::Salt;
        assert!(archaeology_visibility(&world, patch).is_none());
    }

    #[test]
    fn hidden_loom_never_gets_a_pressure_animation_key() {
        let mut world = World::new(904, Faction::Union);
        let enemy = world
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 1 && entity.kind.is_worker())
            .expect("opponent worker");
        enemy.kind = Kind::Loom;
        enemy.deployed = true;
        enemy.pos = Pos::cell(90, 90);
        let id = enemy.id;
        world.artillery.push(ArtilleryShot {
            owner: 1,
            source: id,
            from: Pos::cell(90, 90),
            target: Pos::cell(95, 90),
            impact_tick: u64::from(LOOM_WINDUP_TICKS),
        });
        let entity = world
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert!(!world.entity_visible(0, id));
        assert_eq!(entity_animation_key(&world, entity), None);
    }

    #[test]
    fn visible_loom_animation_follows_committed_twenty_four_tick_shot() {
        let mut world = World::new(905, Faction::Union);
        let loom = world
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .expect("own worker");
        loom.kind = Kind::Loom;
        loom.deployed = true;
        loom.facing = 0;
        loom.pos = Pos::cell(40, 64);
        let id = loom.id;
        world.artillery.push(ArtilleryShot {
            owner: 0,
            source: id,
            from: Pos::cell(40, 64),
            target: Pos::cell(45, 64),
            impact_tick: u64::from(LOOM_WINDUP_TICKS),
        });
        let entity = world
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(
            entity_animation_key(&world, entity).as_deref(),
            Some("loom_1_pressure_0")
        );
        world.tick = 20;
        assert_eq!(
            entity_animation_key(&world, entity).as_deref(),
            Some("loom_1_pressure_5")
        );
        world.tick = 24;
        assert_eq!(entity_animation_key(&world, entity), None);
    }

    #[test]
    fn pressure_frames_use_short_surges_and_two_hundred_ms_cooling_loops() {
        let world = World::new(907, Faction::Union);
        let mut entity = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .cloned()
            .expect("own worker");
        entity.kind = Kind::Riveter;
        entity.facing = 0;
        for (remaining, phase) in [(90, 0), (87, 1), (84, 2), (81, 3), (78, 0)] {
            entity.surge_remaining = remaining;
            entity.surge_cooldown = SURGE_COOLDOWN_TICKS;
            let expected = format!("pressure_riveter_1_surge_{phase}");
            assert_eq!(pressure_key(&entity).as_deref(), Some(expected.as_str()));
        }
        for (cooldown, phase) in [(270, 0), (264, 1), (258, 2), (252, 0)] {
            entity.surge_remaining = 0;
            entity.surge_cooldown = cooldown;
            let expected = format!("pressure_riveter_1_cool_{phase}");
            assert_eq!(pressure_key(&entity).as_deref(), Some(expected.as_str()));
        }
    }

    #[test]
    fn doctrine_attachments_are_owner_only_and_completed_only() {
        let mut world = World::new(906, Faction::Union);
        let own_worker = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .cloned()
            .expect("own worker");
        let enemy_worker = world
            .entities
            .iter()
            .find(|entity| entity.owner == 1 && entity.kind.is_worker())
            .cloned()
            .expect("enemy worker");
        assert_eq!(doctrine_key(&world, &own_worker), None);
        world.players[0].doctrine = Some(Doctrine::Hauling);
        assert!(doctrine_key(&world, &own_worker).is_some());
        assert_eq!(doctrine_key(&world, &enemy_worker), None);
        world.players[0].doctrine = Some(Doctrine::FireControl);
        assert_eq!(doctrine_key(&world, &own_worker), None);
        world.players[0].doctrine = None;
        world.players[0].research = Some(bw_sim::Research {
            building: world
                .entities
                .iter()
                .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
                .unwrap()
                .id,
            doctrine: Doctrine::FireControl,
            remaining: bw_content::DOCTRINE_TICKS,
            tier: 1,
        });
        assert_eq!(doctrine_key(&world, &own_worker), None);
        world.players[0].doctrine = Some(Doctrine::Hauling);
        assert_eq!(doctrine_key(&world, &own_worker), None);
    }

    /// Art v23: every key the Compact's doctrine, trace and GLINT passes ask
    /// for is in the atlas at its contracted size and anchor.
    #[test]
    fn compact_v23_keys_are_in_the_atlas_at_their_contract() {
        let atlas = atlas();
        let check = |key: &str, size: (u32, u32), anchor: (i32, i32)| {
            let sprite = atlas
                .sprites
                .get(key)
                .unwrap_or_else(|| panic!("{key} missing"));
            assert_eq!((sprite.w, sprite.h), size, "{key}");
            assert_eq!((sprite.anchor_x, sprite.anchor_y), anchor, "{key}");
        };
        let mut world = World::new(907, Faction::Compact);
        let mut worker = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .cloned()
            .expect("own worker");
        let hq = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
            .cloned()
            .expect("own headquarters");
        for doctrine in [Doctrine::Hauling, Doctrine::FireControl] {
            world.players[0].doctrine = Some(doctrine);
            check(
                crate::dock_log::doctrine_plate_key(Faction::Compact, doctrine),
                (32, 24),
                (0, 0),
            );
            for frame in 0..4 {
                world.tick = frame * DOCTRINE_FRAME_TICKS;
                let key = headquarters_doctrine_key(&world, &hq).expect("annex key");
                check(&key, (160, 144), (80, 128));
                for facing in 0..8 {
                    worker.facing = facing;
                    if let Some(key) = doctrine_key(&world, &worker) {
                        assert_eq!(doctrine, Doctrine::Hauling);
                        check(&key, (64, 64), (32, 50));
                    }
                }
            }
        }
        for phase in 0..6 {
            check(&format!("trace_glaze_{phase}"), (32, 16), (16, 8));
        }
        for age in 0..crate::glint_fx::FLASH_TICKS {
            let key = crate::glint_fx::flash_key(age).unwrap();
            check(&key, (32, 32), (16, 16));
        }
        for age in 0..16 {
            check(&crate::glint_fx::spot_key(age), (32, 16), (16, 8));
        }
        let total = u64::from(bw_content::GLINT_TICKS);
        for age in 0..total {
            for (key, _) in crate::glint_fx::motes(age, total - age) {
                check(&key, (16, 16), (8, 8));
            }
        }
    }
}
