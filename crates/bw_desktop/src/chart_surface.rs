//! Charted terrain and composed water surface.
//!
//! This module is deliberately a semantic terrain pass.  It is intended to
//! run after the ordinary terrain and salt-patch pass, and before shores,
//! decorations, landmarks, and actors.  It never reads entities, events, or
//! screenshot colours.  A small cell cache is built once per call; all later
//! per-pixel decisions come from camera ground projection plus that cache.

use crate::canvas::{Atlas, Canvas, Color, Sprite};
use bw_core::{Camera, FP, Pos, TICK_HZ, Terrain};
use bw_sim::World;

const TILE_HALF_W: i32 = 16;
const TILE_HALF_H: i32 = 8;
const WATER_MODULE_W: usize = 128;
const WATER_MODULE_H: usize = 64;
const WATER_PHASES: usize = 8;
const WATER_VARIANTS: usize = 3;
const WATER_PHASE_TICKS: u64 = TICK_HZ / 5; // 200ms at the 30Hz visual clock.
const MAX_CACHE_CELLS: usize = 16_384;

/// The palette used for live water's quiet base.  The authored current
/// modules are transparent overlays, so this fill remains visible through
/// every module revision and through the fallback path.
pub const LIVE_WATER: Color = [48, 91, 102, 255];
pub const CURRENT_RIBBON: Color = [70, 113, 123, 120];
pub const CURRENT_HIGHLIGHT: Color = [112, 150, 153, 165];

const CHART_SALT: Color = [64, 82, 91, 235];
const CHART_SILT: Color = [71, 93, 97, 235];
const CHART_DEEP: Color = [40, 62, 75, 255];
const CHART_DRY_LANE: Color = [84, 104, 106, 205];
const CHART_WET_LANE: Color = [48, 82, 95, 215];
const CHART_ROCK: Color = [74, 87, 92, 235];
const CHART_CAUSEWAY: Color = [82, 98, 101, 225];
const CHART_CONTOUR: Color = [130, 151, 151, 120];
const CHART_SURVEY: Color = [157, 173, 165, 135];
/// Tone contours: the ground-light planes drawn as chart ink.
const CHART_TONE_LINE: Color = [130, 151, 151, 90];
/// Soundings hatch on hidden deep water.
const CHART_HATCH: Color = [49, 73, 87, 255];
/// Foam crest of a flood front and the sheen of a draining lane.
const FRONT_FOAM: Color = [196, 212, 196, 210];
const FRONT_CREST: Color = [128, 168, 166, 150];
const DRAIN_SHEEN: Color = [128, 168, 166, 110];
/// Ground never in sight: a darker, neutral sheet with a survey grid and no
/// soundings, so it reads apart from both charted ground and hatched water.
const UNSURVEYED_GROUND: Color = [36, 42, 46, 255];
const UNSURVEYED_WATER: Color = [31, 37, 42, 255];
const UNSURVEYED_GRID: Color = [58, 66, 70, 255];
/// The survey grid's pitch in cells.
const UNSURVEYED_PITCH: i32 = 4;

/// Convert an existing scene colour into a restrained blue-gray chart ink.
///
/// This is a presentation helper for surrounding scenery and does not
/// inspect the canvas.  Alpha is preserved, while luminance is kept only as a
/// small value variation so chart marks do not become a second faction colour.
pub fn chart_color(c: Color) -> Color {
    let luma = (u16::from(c[0]) * 3 + u16::from(c[1]) * 5 + u16::from(c[2]) * 2) / 10;
    [
        (44 + luma / 7).min(93) as u8,
        (60 + luma / 5).min(122) as u8,
        (70 + luma / 4).min(134) as u8,
        c[3],
    ]
}

/// Tint an observed building or other remembered presentation mark as a
/// last-seen chart observation.  The function does not create or locate the
/// remembered object; callers must obtain that object from the simulation's
/// observation data and keep it separate from live entities.
pub fn memory_color(c: Color) -> Color {
    let luma = (u16::from(c[0]) * 3 + u16::from(c[1]) * 5 + u16::from(c[2]) * 2) / 10;
    [
        (54 + luma / 5).min(112) as u8,
        (79 + luma / 4).min(142) as u8,
        (91 + luma / 3).min(158) as u8,
        c[3].min(205),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CachedCell {
    terrain: Terrain,
    visible: bool,
    /// In sight now or at some time before.
    explored: bool,
}

struct SurfaceCache {
    min_x: i32,
    min_y: i32,
    width: usize,
    height: usize,
    view_width: i32,
    view_height: i32,
    cells: Vec<CachedCell>,
}

impl SurfaceCache {
    fn get(&self, x: i32, y: i32) -> Option<CachedCell> {
        if x < self.min_x || y < self.min_y {
            return None;
        }
        let ix = usize::try_from(x - self.min_x).ok()?;
        let iy = usize::try_from(y - self.min_y).ok()?;
        if ix >= self.width || iy >= self.height {
            return None;
        }
        self.cells.get(iy * self.width + ix).copied()
    }
}

/// Integer equivalent of [`Camera::unproject`] followed by [`Pos::cell_xy`].
///
/// `Camera::unproject` intentionally truncates the fixed-point numerator
/// toward zero before `cell_xy` applies Euclidean division.  Keep that order
/// here, but perform the intermediate arithmetic in i64 so an offscreen or
/// malformed camera cannot overflow the surface pass.
fn ground_cell(camera: Camera, screen_x: i32, screen_y: i32) -> (i64, i64) {
    let sx = i64::from(screen_x) + i64::from(camera.x);
    let sy = i64::from(screen_y) + i64::from(camera.y);
    let world_x = (sx + 2 * sy) * i64::from(FP) / 32;
    let world_y = (2 * sy - sx) * i64::from(FP) / 32;
    (
        world_x.div_euclid(i64::from(FP)),
        world_y.div_euclid(i64::from(FP)),
    )
}

/// Project a map cell using the same integer geometry as `Camera::project`.
fn project_cell(camera: Camera, x: i32, y: i32) -> Option<(i32, i32)> {
    let sx = (i64::from(x) - i64::from(y)) * 16 - i64::from(camera.x);
    let sy = (i64::from(x) + i64::from(y) + 1) * 8 - i64::from(camera.y);
    Some((i32::try_from(sx).ok()?, i32::try_from(sy).ok()?))
}

fn map_shape_is_safe(world: &World) -> Option<(i32, i32)> {
    let width = i32::from(world.map.width);
    let height = i32::from(world.map.height);
    if width <= 0 || height <= 0 || world.map.width > 512 || world.map.height > 512 {
        return None;
    }
    let expected = usize::from(world.map.width).checked_mul(usize::from(world.map.height))?;
    (world.map.tiles.len() == expected).then_some((width, height))
}

fn cache_bounds(
    camera: Camera,
    view_width: i32,
    view_height: i32,
    map_width: i32,
    map_height: i32,
) -> Option<(i32, i32, i32, i32)> {
    if view_width <= 0 || view_height <= 0 || map_width <= 0 || map_height <= 0 {
        return None;
    }
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for (sx, sy) in [
        (0, 0),
        (view_width - 1, 0),
        (0, view_height - 1),
        (view_width - 1, view_height - 1),
    ] {
        let (x, y) = ground_cell(camera, sx, sy);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    // A camera looking wholly outside the map must not clamp to the entire
    // map.  That would turn an offscreen draw into an unbounded visibility
    // query despite the supplied viewport bounds.
    if max_x < 0 || max_y < 0 || min_x >= i64::from(map_width) || min_y >= i64::from(map_height) {
        return None;
    }
    let min_x = (min_x - 2).max(0).min(i64::from(map_width - 1)) as i32;
    let min_y = (min_y - 2).max(0).min(i64::from(map_height - 1)) as i32;
    let max_x = (max_x + 2).max(0).min(i64::from(map_width - 1)) as i32;
    let max_y = (max_y + 2).max(0).min(i64::from(map_height - 1)) as i32;
    let area = usize::try_from(max_x - min_x + 1)
        .ok()?
        .checked_mul(usize::try_from(max_y - min_y + 1).ok()?)?;
    (area <= MAX_CACHE_CELLS).then_some((min_x, min_y, max_x, max_y))
}

fn build_cache(
    world: &World,
    camera: Camera,
    view_width: i32,
    view_height: i32,
    explored: Option<&crate::chart_memory::Explored>,
) -> Option<SurfaceCache> {
    let (map_width, map_height) = map_shape_is_safe(world)?;
    let (min_x, min_y, max_x, max_y) =
        cache_bounds(camera, view_width, view_height, map_width, map_height)?;
    let width = usize::try_from(max_x - min_x + 1).ok()?;
    let height = usize::try_from(max_y - min_y + 1).ok()?;
    let mut cells = Vec::with_capacity(width.checked_mul(height)?);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let index = usize::try_from(y).ok()? * usize::from(world.map.width)
                + usize::try_from(x).ok()?;
            let terrain = *world.map.tiles.get(index)?;
            // This is the only visibility query in the pass.  Per-pixel work
            // below consults this result through `SurfaceCache::get`.
            let visible = world.visible(0, Pos::cell(x, y));
            cells.push(CachedCell {
                terrain,
                visible,
                explored: visible || explored.is_none_or(|e| e.get(x, y)),
            });
        }
    }
    Some(SurfaceCache {
        min_x,
        min_y,
        width,
        height,
        view_width,
        view_height,
        cells,
    })
}

fn terrain_is_lane(terrain: Terrain) -> bool {
    terrain.is_tidal()
}

fn chart_terrain(terrain: Terrain, wet: bool) -> Color {
    match terrain {
        Terrain::Salt => CHART_SALT,
        Terrain::Silt => CHART_SILT,
        Terrain::Deep => CHART_DEEP,
        Terrain::Lane0 | Terrain::Lane1 | Terrain::Lane2 => {
            if wet {
                CHART_WET_LANE
            } else {
                CHART_DRY_LANE
            }
        }
        Terrain::Rock => CHART_ROCK,
        Terrain::Rim1 | Terrain::Rim0 | Terrain::Rim2 => {
            if wet {
                CHART_WET_LANE
            } else {
                CHART_CAUSEWAY
            }
        }
    }
}

fn cell_projection_pixels<F: FnMut(i32, i32)>(
    camera: Camera,
    x: i32,
    y: i32,
    view_width: i32,
    view_height: i32,
    mut paint: F,
) {
    if view_width <= 0 || view_height <= 0 {
        return;
    }
    let Some((sx, sy)) = project_cell(camera, x, y) else {
        return;
    };
    let min_x = (sx - TILE_HALF_W).max(0);
    let max_x = (sx + TILE_HALF_W).min(view_width - 1);
    let min_y = (sy - TILE_HALF_H).max(0);
    let max_y = (sy + TILE_HALF_H).min(view_height - 1);
    if min_x > max_x || min_y > max_y {
        return;
    }
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            // Ground projection, rather than the colour already in the
            // framebuffer, determines tile ownership at this pixel.  The
            // cache supplies the semantic state for the resulting cell.
            let (gx, gy) = ground_cell(camera, px, py);
            if gx == i64::from(x) && gy == i64::from(y) {
                paint(px, py)
            }
        }
    }
}

fn world_pixel(camera: Camera, px: i32, py: i32) -> (i64, i64) {
    (
        i64::from(px) + i64::from(camera.x),
        i64::from(py) + i64::from(camera.y),
    )
}

fn water_variant_block(block_x: i64, block_y: i64) -> usize {
    usize::try_from(
        (block_x * 13 + block_y * 17 + block_x * block_y * 3).rem_euclid(WATER_VARIANTS as i64),
    )
    .unwrap_or(0)
}

fn water_variant_at_world(absolute_x: i64, absolute_y: i64) -> usize {
    water_variant_block(
        absolute_x.div_euclid(WATER_MODULE_W as i64),
        absolute_y.div_euclid(WATER_MODULE_H as i64),
    )
}

fn water_phase(tick: u64, north_dry: bool) -> usize {
    let base = (tick / WATER_PHASE_TICKS.max(1)) % WATER_PHASES as u64;
    if north_dry {
        base as usize
    } else {
        // The public lane state reverses the decorative crest travel instead
        // of translating the whole water field by a phase offset.
        ((WATER_PHASES as u64 - 1 - base) % WATER_PHASES as u64) as usize
    }
}

fn load_water_module(atlas: &Atlas, variant: usize, phase: usize) -> Option<Sprite> {
    let key = format!("water_current_{variant}_{phase}");
    let sprite = atlas.sprites.get(&key)?;
    if sprite.w as usize != WATER_MODULE_W
        || sprite.h as usize != WATER_MODULE_H
        || sprite.anchor_x != 0
        || sprite.anchor_y != 0
    {
        return None;
    }
    Some(sprite.clone())
}

fn module_pixel(atlas: &Atlas, sprite: &Sprite, x: i64, y: i64) -> Color {
    let x = x.rem_euclid(WATER_MODULE_W as i64) as usize;
    let y = y.rem_euclid(WATER_MODULE_H as i64) as usize;
    atlas
        .sample(sprite, x as u32, y as u32)
        .unwrap_or([0, 0, 0, 0])
}

fn fallback_current_pixel(
    absolute_x: i64,
    absolute_y: i64,
    variant: usize,
    phase: usize,
    north_dry: bool,
) -> Option<Color> {
    // Static broad ribbons are anchored in world projection.  Only a small
    // crest moves through each ribbon, so a camera pan cannot translate the
    // entire water texture as one sheet.
    let direction = if north_dry { 1i64 } else { -1i64 };
    let along = absolute_x + direction * absolute_y + variant as i64 * 29;
    let cross = absolute_y - direction * absolute_x;
    let ribbon = along.rem_euclid(61);
    let crest = (along + phase as i64 * 2 + cross.div_euclid(7)).rem_euclid(19);
    if ribbon <= 1 && cross.rem_euclid(17) < 13 {
        if crest <= 1 {
            Some(CURRENT_HIGHLIGHT)
        } else {
            Some(CURRENT_RIBBON)
        }
    } else {
        None
    }
}

/// Shallow shelf colours from the bank inward: two steps, then the quiet fill.
pub const SHELF_NEAR: Color = [77, 124, 130, 255];
pub const SHELF_FAR: Color = [61, 108, 117, 255];
const SHELF_NEAR_DEPTH: i32 = 2;
const SHELF_FAR_DEPTH: i32 = 5;

/// The sides of a deep cell that meet dry ground (dry lanes count as ground,
/// flooded lanes as water).  Only known semantic cells count.
fn shelf_banks(cache: &SurfaceCache, x: i32, y: i32, world: &World) -> Vec<(i32, i32)> {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .filter(|(dx, dy)| {
            cache.get(x + dx, y + dy).is_some_and(|n| {
                n.terrain != Terrain::Deep
                    && !crate::presentation::cell_is_wet_at(world, x + dx, y + dy)
            })
        })
        .collect()
}

fn shelf_color(banks: &[(i32, i32)], sx: i32, sy: i32, px: i32, py: i32) -> Color {
    let nearest = banks
        .iter()
        .map(|&direction| edge_distance(direction, sx, sy, px, py))
        .min();
    let Some(d) = nearest else {
        return LIVE_WATER;
    };
    // The shelf edge wanders by a pixel or two along the bank so the two
    // bands do not trace the diamond grid; the wander is a fixed function of
    // the pixel's position along the edge, so it is stable under panning.
    let along = (px - sx) + 2 * (py - sy);
    let wander = i32::from(along.rem_euclid(7) < 3) + i32::from(along.rem_euclid(11) < 4);
    if d < SHELF_NEAR_DEPTH + wander {
        SHELF_NEAR
    } else if d <= SHELF_FAR_DEPTH + wander {
        SHELF_FAR
    } else {
        LIVE_WATER
    }
}

fn edge_distance(direction: (i32, i32), sx: i32, sy: i32, px: i32, py: i32) -> i32 {
    let u = px - sx;
    let v = py - sy;
    match direction {
        (1, 0) => 16 - u - 2 * v,
        (-1, 0) => 16 + u + 2 * v,
        (0, 1) => 16 + u - 2 * v,
        (0, -1) => 16 - u + 2 * v,
        _ => 0,
    }
}

fn draw_chart_accents(canvas: &mut Canvas, cache: &SurfaceCache, world: &World, camera: Camera) {
    for y in cache.min_y..cache.min_y + cache.height as i32 {
        for x in cache.min_x..cache.min_x + cache.width as i32 {
            let Some(cell) = cache.get(x, y) else {
                continue;
            };
            if cell.visible || !cell.explored || cell.terrain == Terrain::Deep {
                continue;
            }
            let survey = !terrain_is_lane(cell.terrain) && (x * 31 + y * 17).rem_euclid(29) == 0;
            let mut contour_edges =
                [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .into_iter()
                    .filter(|&(dx, dy)| {
                        world.map.terrain(x + dx, y + dy) != cell.terrain
                            && world.map.terrain(x + dx, y + dy) != Terrain::Rock
                    });
            let contour_edge = contour_edges.next();
            // Tone contours: where the ground-light plane steps down to a
            // neighbouring cell, draw the boundary as chart ink.  The same
            // field lights the visible floor, so the chart shows the same
            // ground the light shows.
            let mut tone_edges: [Option<(i32, i32)>; 4] = [None; 4];
            if matches!(cell.terrain, Terrain::Salt | Terrain::Silt) {
                let tone = crate::ground_light::tone(x, y);
                for (slot, (dx, dy)) in [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().enumerate() {
                    if matches!(
                        world.map.terrain(x + dx, y + dy),
                        Terrain::Salt | Terrain::Silt
                    ) && crate::ground_light::tone(x + dx, y + dy) < tone
                    {
                        tone_edges[slot] = Some((dx, dy));
                    }
                }
            }
            let has_tone_edge = tone_edges.iter().any(|e| e.is_some());
            if !survey && contour_edge.is_none() && !has_tone_edge {
                continue;
            }
            let Some((sx, sy)) = project_cell(camera, x, y) else {
                continue;
            };
            cell_projection_pixels(
                camera,
                x,
                y,
                cache.view_width,
                cache.view_height,
                |px, py| {
                    if survey {
                        let lx = px - sx;
                        let ly = py - sy;
                        if (lx.abs() <= 3 && ly == -3) || (ly.abs() <= 1 && lx == -4) {
                            canvas.pixel(px, py, CHART_SURVEY);
                        }
                    }
                    if let Some(direction) = contour_edge
                        && edge_distance(direction, sx, sy, px, py) <= 1
                    {
                        canvas.pixel(px, py, CHART_CONTOUR);
                    }
                    for direction in tone_edges.iter().flatten() {
                        if edge_distance(*direction, sx, sy, px, py) == 0 {
                            canvas.pixel(px, py, CHART_TONE_LINE);
                        }
                    }
                },
            );
        }
    }
}

fn draw_visible_transitions(canvas: &mut Canvas, cache: &SurfaceCache, camera: Camera) {
    for y in cache.min_y..cache.min_y + cache.height as i32 {
        for x in cache.min_x..cache.min_x + cache.width as i32 {
            let Some(cell) = cache.get(x, y) else {
                continue;
            };
            if !cell.visible {
                continue;
            }
            let Some((sx, sy)) = project_cell(camera, x, y) else {
                continue;
            };
            for (direction, dx, dy) in [
                ((1, 0), 1, 0),
                ((-1, 0), -1, 0),
                ((0, 1), 0, 1),
                ((0, -1), 0, -1),
            ] {
                // A transition is drawn only when the adjacent semantic cell
                // is known and hidden.  Unknown/off-map space stays untouched.
                if cache
                    .get(x + dx, y + dy)
                    .is_none_or(|neighbor| neighbor.visible)
                {
                    continue;
                }
                cell_projection_pixels(
                    camera,
                    x,
                    y,
                    cache.view_width,
                    cache.view_height,
                    |px, py| {
                        let distance = edge_distance(direction, sx, sy, px, py);
                        // An ordered dither charts three quarters of the first
                        // row, half of the second and a quarter of the third,
                        // so the sight boundary breaks up instead of stepping.
                        let threshold = match distance {
                            0 => 3,
                            1 => 2,
                            2 => 1,
                            _ => 0,
                        };
                        if threshold == 0 {
                            return;
                        }
                        let bayer =
                            [[0, 2], [3, 1]][py.rem_euclid(2) as usize][px.rem_euclid(2) as usize];
                        if bayer < threshold
                            && let Some(c) = canvas.get(px, py)
                        {
                            canvas.pixel(px, py, chart_color(c));
                        }
                    },
                );
            }
        }
    }
}

/// Draw charted fog and composed water into the semantic terrain layer.
///
/// The caller should invoke this after the ordinary terrain/salt-patch pass
/// and before shores, decorations, landmarks, or actors.  The pass never
/// renders entities and never calls `World::visible` per pixel.
pub fn draw(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    world: &World,
    camera: Camera,
    tick: u64,
    lane_changed: Option<crate::lane_memory::TideChange>,
    explored: Option<&crate::chart_memory::Explored>,
) {
    let Some(cache) = build_cache(
        world,
        camera,
        i32::try_from(canvas.width()).expect("Canvas width fits i32"),
        i32::try_from(canvas.height()).expect("Canvas height fits i32"),
        explored,
    ) else {
        return;
    };
    // The crests travel one way while arm 0 is the dry arm (the Split
    // Basin's north, the Confluence's E) and the other way while any other
    // arm is: the authored current modules have two travel directions only.
    let north_dry = world.gate.north_dry();

    // First replace the repetitive visible-water tile field with a quiet
    // semantic fill, and turn hidden terrain into chart ink.  Lanes receive
    // an alpha blend so their existing crossbars and dry/wet structure remain
    // visible through the chart tint.
    for y in cache.min_y..cache.min_y + cache.height as i32 {
        for x in cache.min_x..cache.min_x + cache.width as i32 {
            let Some(cell) = cache.get(x, y) else {
                continue;
            };
            if cell.visible && cell.terrain != Terrain::Deep {
                continue;
            }
            if !cell.explored {
                draw_unsurveyed(canvas, &cache, world, camera, x, y, cell);
                continue;
            }
            let wet = crate::presentation::cell_is_wet_at(world, x, y);
            if cell.visible && cell.terrain == Terrain::Deep {
                // Live deep water: a quiet fill with a short shallow shelf
                // along any side that meets dry ground.  Flooded lanes count
                // as water, so the ramp never outlines a route.
                let banks = shelf_banks(&cache, x, y, world);
                let Some((sx, sy)) = project_cell(camera, x, y) else {
                    continue;
                };
                cell_projection_pixels(
                    camera,
                    x,
                    y,
                    cache.view_width,
                    cache.view_height,
                    |px, py| {
                        canvas.pixel(px, py, shelf_color(&banks, sx, sy, px, py));
                    },
                );
                continue;
            }
            let color = chart_terrain(cell.terrain, wet);
            let deep = cell.terrain == Terrain::Deep;
            cell_projection_pixels(
                camera,
                x,
                y,
                cache.view_width,
                cache.view_height,
                |px, py| {
                    canvas.pixel(px, py, color);
                    // Soundings: a sparse world-anchored hatch on hidden deep
                    // water, so the chart reads as surveyed water, not night.
                    if deep {
                        let (ax, ay) = world_pixel(camera, px, py);
                        if (ax + ay).rem_euclid(8) == 0 {
                            canvas.pixel(px, py, CHART_HATCH);
                        }
                    }
                },
            );
        }
    }

    // Resolve each authored 128x64 module once per variant/phase.  Atlas::sample
    // reads transparent texels directly, avoiding a per-frame scratch canvas
    // while still sampling at world-projected coordinates.
    draw_submerged(canvas, atlas, &cache, camera);
    let phase = water_phase(tick, north_dry);
    // Every phase of every variant is resolved so a gust can push the crests
    // of one world region a phase or two ahead of the calm water around it.
    let mut modules: [[Option<Sprite>; WATER_PHASES]; WATER_VARIANTS] =
        std::array::from_fn(|_| std::array::from_fn(|_| None));
    for (variant, phases) in modules.iter_mut().enumerate() {
        for (p, module) in phases.iter_mut().enumerate() {
            *module = atlas.and_then(|a| load_water_module(a, variant, p));
        }
    }
    for y in cache.min_y..cache.min_y + cache.height as i32 {
        for x in cache.min_x..cache.min_x + cache.width as i32 {
            let Some(cell) = cache.get(x, y) else {
                continue;
            };
            if !cell.visible || cell.terrain != Terrain::Deep {
                continue;
            }
            cell_projection_pixels(
                camera,
                x,
                y,
                cache.view_width,
                cache.view_height,
                |px, py| {
                    let (absolute_x, absolute_y) = world_pixel(camera, px, py);
                    // A 128x64 world-projected module owns the variant for all of
                    // its pixels.  Choosing by cell would tear a module at each
                    // 32x16 terrain diamond even when adjacent module art joins.
                    let variant = water_variant_at_world(absolute_x, absolute_y);
                    let gust = usize::from(crate::wind::strength(absolute_x, absolute_y, tick));
                    let phase = if north_dry {
                        (phase + gust) % WATER_PHASES
                    } else {
                        (phase + WATER_PHASES - gust) % WATER_PHASES
                    };
                    let module = modules[variant][phase].as_ref();
                    // A present authored module owns its transparent texels too:
                    // they intentionally reveal the quiet fill.  Use the
                    // procedural path only when the requested module is absent.
                    let color = match (atlas, module) {
                        (Some(atlas), Some(sprite)) => {
                            let sample = module_pixel(atlas, sprite, absolute_x, absolute_y);
                            (sample[3] != 0).then_some(sample)
                        }
                        _ => fallback_current_pixel(
                            absolute_x, absolute_y, variant, phase, north_dry,
                        ),
                    };
                    if let Some(color) = color {
                        canvas.pixel(px, py, color);
                    }
                },
            );
        }
    }

    draw_lane_fronts(canvas, &cache, world, camera, tick, lane_changed);
    draw_chart_accents(canvas, &cache, world, camera);
    draw_visible_transitions(canvas, &cache, camera);
}

/// Ground never in sight: a flat sheet that keeps only the coast, crossed by
/// a dashed survey grid along every fourth cell edge.
fn draw_unsurveyed(
    canvas: &mut Canvas,
    cache: &SurfaceCache,
    world: &World,
    camera: Camera,
    x: i32,
    y: i32,
    cell: CachedCell,
) {
    let water = cell.terrain == Terrain::Deep
        || (cell.terrain.is_tidal() && matches!(world.depth_at(x, y), Some(bw_sim::Depth::Deep)));
    let fill = if water {
        UNSURVEYED_WATER
    } else {
        UNSURVEYED_GROUND
    };
    let Some((sx, sy)) = project_cell(camera, x, y) else {
        return;
    };
    let west = x.rem_euclid(UNSURVEYED_PITCH) == 0;
    let north = y.rem_euclid(UNSURVEYED_PITCH) == 0;
    cell_projection_pixels(
        camera,
        x,
        y,
        cache.view_width,
        cache.view_height,
        |px, py| {
            let (ax, _) = world_pixel(camera, px, py);
            let dash = ax.div_euclid(2).rem_euclid(2) == 0;
            let on_grid = dash
                && ((west && edge_distance((-1, 0), sx, sy, px, py) == 0)
                    || (north && edge_distance((0, -1), sx, sy, px, py) == 0));
            canvas.pixel(px, py, if on_grid { UNSURVEYED_GRID } else { fill });
        },
    );
}

/// The drowned town: fixed submerged silhouettes under the deep water, drawn
/// over the fill and under the crests, only where every cell under the
/// sprite is deep water.  Hidden cells draw them as chart ink.
fn draw_submerged(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    cache: &SurfaceCache,
    camera: Camera,
) {
    let Some(atlas) = atlas else {
        return;
    };
    for site in crate::presentation::SUBMERGED {
        let (cx, cy) = site.cell;
        let mut all_deep = true;
        let mut visible = false;
        for dy in -1..=1 {
            for dx in -2..=2 {
                match cache.get(cx + dx, cy + dy) {
                    Some(cell) if cell.terrain == Terrain::Deep && cell.explored => {
                        visible |= cell.visible
                    }
                    _ => all_deep = false,
                }
            }
        }
        if !all_deep {
            continue;
        }
        let Some((sx, sy)) = project_cell(camera, cx, cy) else {
            continue;
        };
        atlas.draw(canvas, site.key, sx, sy, !visible);
    }
}

/// Lane memory in motion: after a public gate change a foam crest rolls out
/// from the spine along the lane that flooded, and a sheen recedes toward
/// the spine along the lane that drained.  Both ride on tiles that already
/// show the authoritative state.
fn draw_lane_fronts(
    canvas: &mut Canvas,
    cache: &SurfaceCache,
    world: &World,
    camera: Camera,
    tick: u64,
    changed: Option<crate::lane_memory::TideChange>,
) {
    let Some(change) = changed else {
        return;
    };
    if crate::lane_memory::flood_front(Some(change.tick), tick).is_none() {
        return;
    }
    // Each arm's fronts cross it from its centre line to its farthest cell
    // in the same two seconds, however wide the arm is.
    let layout = world.map.layout();
    let arms = layout.arm_count().min(crate::lane_memory::MAX_ARMS);
    let fronts: Vec<(Option<i32>, Option<i32>)> = (0..arms)
        .map(|arm| {
            let reach = if layout.id.is_split_basin() {
                crate::lane_memory::LANE_REACH
            } else {
                crate::lane_memory::arm_reach(world, arm)
            };
            (
                crate::lane_memory::flood_front_within(Some(change.tick), tick, reach),
                crate::lane_memory::drain_front_within(Some(change.tick), tick, reach),
            )
        })
        .collect();
    for y in cache.min_y..cache.min_y + cache.height as i32 {
        for x in cache.min_x..cache.min_x + cache.width as i32 {
            let Some(cell) = cache.get(x, y) else {
                continue;
            };
            if !cell.visible || !terrain_is_lane(cell.terrain) {
                continue;
            }
            let Some(arm) = cell.terrain.tidal_arm().map(usize::from) else {
                continue;
            };
            let Some(&(flood, drain)) = fronts.get(arm) else {
                continue;
            };
            let Some(d) = crate::lane_memory::crossing_distance(layout, arm, x, y) else {
                continue;
            };
            if d < 0 {
                continue;
            }
            let (color, density) =
                if change.flooded(arm) && flood.is_some_and(|f| d == f || d == f - 1) {
                    if Some(d) == flood {
                        (FRONT_FOAM, 2)
                    } else {
                        (FRONT_CREST, 4)
                    }
                } else if change.dried(arm) && drain.is_some_and(|f| d == f || d == f + 1) {
                    (DRAIN_SHEEN, 5)
                } else {
                    continue;
                };
            cell_projection_pixels(
                camera,
                x,
                y,
                cache.view_width,
                cache.view_height,
                |px, py| {
                    let (ax, ay) = world_pixel(camera, px, py);
                    if (ax + 2 * ay + (tick / 2) as i64).rem_euclid(density) == 0
                        && (ax * 3 + ay).rem_euclid(7) < 4
                    {
                        canvas.pixel(px, py, color);
                    }
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{CANVAS_H, CANVAS_W, Faction, Terrain};

    #[test]
    fn ground_projection_keeps_camera_inverse_order() {
        let camera = Camera { x: -221, y: 495 };
        for px in [0, 17, 319, 639] {
            for py in [0, 24, 180, 359] {
                let expected = camera.unproject(px, py).cell_xy();
                assert_eq!(
                    ground_cell(camera, px, py),
                    (i64::from(expected.0), i64::from(expected.1))
                );
            }
        }
    }

    #[test]
    fn water_phase_loops_at_200ms_and_changes_with_lane_direction() {
        assert_eq!(water_phase(0, true), 0);
        assert_eq!(water_phase(5, true), 0);
        assert_eq!(water_phase(6, true), 1);
        assert_eq!(water_phase(6 * 8, true), 0);
        assert_eq!(water_phase(0, false), WATER_PHASES - 1);
        assert_eq!(water_phase(6, false), WATER_PHASES - 2);
        assert_ne!(water_phase(6, true), water_phase(6, false));
    }

    #[test]
    fn module_variant_is_stable_inside_a_world_projected_block() {
        assert_eq!(
            water_variant_at_world(3, 5),
            water_variant_at_world((WATER_MODULE_W - 1) as i64, (WATER_MODULE_H - 1) as i64)
        );
        assert_eq!(
            water_variant_at_world(-1, -1),
            water_variant_at_world(-127, -63)
        );
    }

    #[test]
    fn absolute_world_projection_survives_camera_pan() {
        let first = Camera { x: 130, y: -41 };
        let second = Camera { x: 151, y: -29 };
        // A camera shift of (+21,+12) moves a world point by (-21,-12).
        let screen = (284, 117);
        assert_eq!(
            world_pixel(first, screen.0, screen.1),
            world_pixel(second, screen.0 - 21, screen.1 - 12)
        );
    }

    #[test]
    fn expanded_view_projection_reaches_past_the_design_canvas_edge() {
        let mut camera = Camera::default();
        camera.center(Pos::cell(58, 49));
        camera.x -= 520;

        let mut pixels = Vec::new();
        cell_projection_pixels(camera, 58, 49, 1280, 552, |px, py| {
            pixels.push((px, py));
        });

        assert!(!pixels.is_empty());
        assert!(pixels.iter().any(|(px, _)| *px > CANVAS_W as i32));
        assert!(
            pixels
                .iter()
                .all(|(px, py)| (0..1280).contains(px) && (0..552).contains(py))
        );
    }

    #[test]
    fn fallback_surface_pixels_follow_world_when_camera_pans() {
        let mut world = World::new(13, Faction::Union);
        let deep_anchor = Pos::cell(53, 64);
        let hq = world
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 0 && entity.kind == bw_core::Kind::Headquarters)
            .expect("starting headquarters");
        hq.pos = deep_anchor;
        let first_camera = {
            let mut camera = Camera::default();
            camera.center(deep_anchor);
            camera
        };
        let second_camera = Camera {
            x: first_camera.x + 16,
            y: first_camera.y,
        };
        let mut first = Canvas::default();
        let mut second = Canvas::default();
        draw(&mut first, None, &world, first_camera, 18, None, None);
        draw(&mut second, None, &world, second_camera, 18, None, None);
        for y in 0..CANVAS_H as i32 {
            for x in 16..CANVAS_W as i32 {
                let first_index = ((y as u32 * CANVAS_W + x as u32) * 4) as usize;
                let second_index = ((y as u32 * CANVAS_W + (x - 16) as u32) * 4) as usize;
                assert_eq!(
                    &first.pixels[first_index..first_index + 4],
                    &second.pixels[second_index..second_index + 4],
                    "world-anchored chart pixel changed at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn invalid_or_offscreen_world_is_a_bounded_noop() {
        let mut world = World::new(7, Faction::Union);
        world.map.width = 0;
        world.map.height = 0;
        world.map.tiles.clear();
        let before = Canvas::default().pixels;
        let mut canvas = Canvas::default();
        draw(&mut canvas, None, &world, Camera::default(), 0, None, None);
        assert_eq!(canvas.pixels, before);

        let world = World::new(7, Faction::Union);
        let before = Canvas::default().pixels;
        let mut canvas = Canvas::default();
        draw(
            &mut canvas,
            None,
            &world,
            Camera {
                x: i32::MAX,
                y: i32::MIN,
            },
            u64::MAX,
            None,
            None,
        );
        assert_eq!(canvas.pixels, before);
    }

    #[test]
    fn repeated_render_is_stable_and_hidden_enemy_state_is_irrelevant() {
        let world = World::new(11, Faction::Union);
        let mut first = Canvas::default();
        let mut second = Canvas::default();
        draw(&mut first, None, &world, Camera::default(), 42, None, None);
        draw(&mut second, None, &world, Camera::default(), 42, None, None);
        assert_eq!(first.pixels, second.pixels);

        let mut paired = world.clone();
        if let Some(enemy) = paired
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 1 && !entity.kind.is_building())
        {
            enemy.pos = Pos::cell(100, 100);
            enemy.hp = 1;
            enemy.facing = 7;
        }
        let mut changed = Canvas::default();
        draw(
            &mut changed,
            None,
            &paired,
            Camera::default(),
            42,
            None,
            None,
        );
        assert_eq!(first.pixels, changed.pixels);
    }

    #[test]
    fn chart_and_memory_helpers_are_blue_gray_and_alpha_preserving() {
        let source = [240, 80, 40, 177];
        let chart = chart_color(source);
        let memory = memory_color(source);
        assert_eq!(chart[3], source[3]);
        assert_eq!(memory[3], source[3]);
        assert!(chart[2] >= chart[1] - 10);
        assert!(memory[2] >= memory[0]);
        assert_ne!(chart, source);
        assert_ne!(memory, source);
    }

    #[test]
    fn terrain_cache_is_semantic_and_contains_no_entity_state() {
        let world = World::new(17, Faction::Union);
        let cache = build_cache(
            &world,
            Camera::default(),
            CANVAS_W as i32,
            CANVAS_H as i32,
            None,
        )
        .expect("default camera sees map");
        assert!(cache.cells.iter().any(|cell| cell.terrain == Terrain::Deep));
        assert!(cache.cells.iter().all(|cell| matches!(
            cell.terrain,
            Terrain::Salt
                | Terrain::Silt
                | Terrain::Deep
                | Terrain::Lane0
                | Terrain::Lane1
                | Terrain::Rock
                | Terrain::Rim1
                | Terrain::Rim0
        )));
    }

    #[test]
    fn wide_surface_cache_uses_the_supplied_view_dimensions() {
        let world = World::new(18, Faction::Union);
        let mut camera = Camera::default();
        camera.center(Pos::cell(58, 49));
        camera.x -= 520;
        let cache = build_cache(&world, camera, 1280, 552, None).expect("wide view intersects map");
        assert_eq!((cache.view_width, cache.view_height), (1280, 552));
        assert!(cache.cells.len() <= MAX_CACHE_CELLS);
    }
}

/// Engine captures of the Confluence at every tide, for review by eye:
/// `BW_REVIEW_DIR=/some/dir cargo test -p bw_desktop --release -- --ignored
/// confluence_tide_review`.  Only public world state and presentation clocks
/// are set; nothing is stepped.
#[cfg(test)]
mod confluence_review {
    use crate::game::Game;
    use crate::lane_memory::TideChange;
    use crate::zoom::Zoom;
    use bw_core::{Faction, Pos};
    use bw_sim::{Arm, MapId, Tide};
    use std::path::{Path, PathBuf};

    fn game(dir: &Path, map: MapId, width: u32, height: u32) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(base, dir.join("session"));
        g.resize_view(width, height);
        g.faction = Faction::Union;
        g.map = map;
        g.start();
        g.world.ai_enabled = false;
        g.world.revealed = true;
        g.message.clear();
        g.cursor = (-999, -999);
        g
    }

    fn set_tide(g: &mut Game, tide: Tide, arm: u8) {
        g.world.gate.tide = tide;
        g.world.gate.dry_arm = Arm(arm);
        g.world.gate.opened = tide != Tide::Neutral;
        g.tide_change = None;
        g.gate_switch = None;
    }

    fn shoot(g: &mut Game, dir: &Path, name: &str) {
        g.screenshot(&dir.join(format!("{name}.png")))
            .expect("capture");
    }

    #[test]
    #[ignore]
    fn confluence_tide_review() {
        let Some(dir) = std::env::var_os("BW_REVIEW_DIR").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&dir).unwrap();
        let tides = [
            ("neutral", Tide::Neutral, 0),
            ("dry-e", Tide::Open, 0),
            ("dry-w", Tide::Open, 1),
            ("dry-s", Tide::Open, 2),
            ("flood", Tide::Flood, 0),
        ];
        // The whole map at 1x on one very wide view.
        let mut g = game(&dir, MapId::Confluence, 5800, 3100);
        g.world.tick = 4000;
        for (name, tide, arm) in tides {
            set_tide(&mut g, tide, arm);
            g.set_zoom(Zoom::Overview, g.world_view().center());
            g.camera.center(Pos::cell(88, 88));
            shoot(&mut g, &dir, &format!("whole-{name}"));
        }
        // The station and each arm's lane at 2x, at every tide.
        let mut g = game(&dir, MapId::Confluence, 1920, 1080);
        g.world.tick = 4000;
        let layout = g.world.map.layout();
        let mut spots = vec![("station", Pos::cell(88, 88))];
        for (arm, name) in layout.arms.iter().zip(["e", "w", "s"]) {
            spots.push((name, Pos::cell(arm.centre.0, arm.centre.1)));
        }
        for (name, tide, arm) in tides {
            set_tide(&mut g, tide, arm);
            for (spot, at) in &spots {
                g.set_zoom(Zoom::Wide, g.world_view().center());
                g.camera.center(*at);
                shoot(&mut g, &dir, &format!("{spot}-{name}"));
            }
        }
        // A switch from E dry to W dry, one second in: the E lane's foam
        // front, the W lane's receding sheen and its first drying stage,
        // and the S lane (deep before and after) quiet.
        set_tide(&mut g, Tide::Open, 1);
        g.world.tick = 4000;
        g.tide_change = Some(TideChange::observed(
            Tide::Open,
            Arm(0),
            &g.world.gate,
            4000 - 30,
        ));
        g.gate_switch = Some((4000 - 30, false));
        g.set_zoom(Zoom::Overview, g.world_view().center());
        g.camera.center(Pos::cell(80, 80));
        shoot(&mut g, &dir, "switch-e-to-w-1x");
        for (spot, at) in &spots {
            g.set_zoom(Zoom::Wide, g.world_view().center());
            g.camera.center(*at);
            shoot(&mut g, &dir, &format!("switch-e-to-w-{spot}"));
        }
        // The same switch two drying stages later: only W dries.
        g.gate_switch = None;
        g.world.tick = 4000 + crate::lane_memory::STAGE_TICKS * 2;
        g.camera.center(spots[2].1);
        shoot(&mut g, &dir, "switch-e-to-w-w-drying-2");
        // The far edges of the round map and the three homes at 1x.
        g.world.tick = 4000;
        set_tide(&mut g, Tide::Neutral, 0);
        g.set_zoom(Zoom::Overview, g.world_view().center());
        for (name, at) in [
            ("north", Pos::cell(10, 10)),
            ("east", Pos::cell(165, 10)),
            ("west", Pos::cell(10, 165)),
            ("south", Pos::cell(165, 165)),
            ("hq-t", Pos::cell(45, 45)),
            ("hq-r", Pos::cell(147, 73)),
            ("hq-l", Pos::cell(73, 147)),
            ("landmark-gauge", Pos::cell(87, 37)),
            ("landmark-ribs", Pos::cell(135, 113)),
            ("landmark-stairs", Pos::cell(43, 113)),
        ] {
            g.camera.center(at);
            shoot(&mut g, &dir, &format!("edge-{name}"));
        }
        // The Split Basin, for comparison with earlier captures.
        let mut g = game(&dir, MapId::SplitBasin, 1920, 1080);
        g.world.tick = 4000;
        for (name, tide, arm) in [
            ("neutral", Tide::Neutral, 0),
            ("dry-n", Tide::Open, 0),
            ("dry-s", Tide::Open, 1),
            ("flood", Tide::Flood, 0),
        ] {
            set_tide(&mut g, tide, arm);
            g.set_zoom(Zoom::Overview, g.world_view().center());
            g.camera.center(Pos::cell(64, 64));
            shoot(&mut g, &dir, &format!("basin-{name}"));
        }
    }
}
