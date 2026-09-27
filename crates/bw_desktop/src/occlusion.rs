//! Presentation geometry for interactive buildings and units.
//!
//! The simulation stores a building at the centre of its top-left cell.  The
//! authored v6 building bases use a different convention: the final point of
//! each `diamond(x, y, w, d)` is the south-east/front corner of the ground
//! foundation.  This module keeps that conversion in one place and provides
//! an ordering relation that treats a building footprint as an interval rather
//! than as one point.  It deliberately contains no simulation or save rules.

use bw_content::spec;
use bw_core::{FP, Faction, Kind, Pos};
use bw_sim::Entity;
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// A rational native-pixel scale used by authored building sprites.
///
/// Most structure sprites are authored for their logical footprint.  The
/// three 2x2 structures have a wider native foundation than a 2x2 isometric
/// cell diamond, so the current evidence-backed render recommendation is 3/4
/// for those entries.  Scaling is presentation-only; it does not change a
/// footprint or a placement rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelScale {
    pub numerator: u16,
    pub denominator: u16,
}

impl PixelScale {
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };
    pub const THREE_QUARTERS: Self = Self {
        numerator: 3,
        denominator: 4,
    };

    /// Apply the scale with symmetric nearest-pixel rounding.
    pub fn apply(self, value: i32) -> i32 {
        assert!(
            self.denominator > 0,
            "pixel scale denominator must be nonzero"
        );
        let numerator = i64::from(value) * i64::from(self.numerator);
        let denominator = i64::from(self.denominator);
        if numerator >= 0 {
            ((numerator + denominator / 2) / denominator) as i32
        } else {
            -(((-numerator + denominator / 2) / denominator) as i32)
        }
    }
}

/// The native authored front point and scale for one building family.
///
/// `front_x` and `front_y` are the source-space offsets from the sprite's
/// atlas anchor.  They come directly from `tools/art_v6/ground_buildings.lua`:
/// the last point in each foundation diamond is `(x, y)`.  The foundation
/// extents are retained here as review evidence and to make the mapping easy
/// to audit when the source art changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingArtProfile {
    pub front_x: i32,
    pub front_y: i32,
    pub foundation_width: i32,
    pub foundation_depth: i32,
    pub scale: PixelScale,
}

/// Return the source-ground profile for a building sprite.
pub fn building_art_profile(kind: Kind, faction: Faction) -> Option<BuildingArtProfile> {
    let profile = match (kind, faction) {
        // union_hq: foundation(-2, 5, 55, 59)
        (Kind::Headquarters, Faction::Union) => BuildingArtProfile {
            front_x: -2,
            front_y: 5,
            foundation_width: 55,
            foundation_depth: 59,
            scale: PixelScale::ONE,
        },
        // assembly_hq: foundation(0, 5, 51, 53)
        (Kind::Headquarters, Faction::Assembly) => BuildingArtProfile {
            front_x: 0,
            front_y: 5,
            foundation_width: 51,
            foundation_depth: 53,
            scale: PixelScale::ONE,
        },
        // union_works: foundation(2, 5, 51, 50)
        (Kind::Works, Faction::Union) => BuildingArtProfile {
            front_x: 2,
            front_y: 5,
            foundation_width: 51,
            foundation_depth: 50,
            scale: PixelScale::ONE,
        },
        // assembly_works: foundation(0, 4, 48, 50)
        (Kind::Works, Faction::Assembly) => BuildingArtProfile {
            front_x: 0,
            front_y: 4,
            foundation_width: 48,
            foundation_depth: 50,
            scale: PixelScale::ONE,
        },
        // dropoff: foundation(0, 5, 43, 48)
        // condenser: foundation(0, 5, 37, 40)
        // tower: foundation(0, 5, 37, 39)
        (Kind::Dropoff, _) => BuildingArtProfile {
            front_x: 0,
            front_y: 5,
            foundation_width: 43,
            foundation_depth: 48,
            scale: PixelScale::THREE_QUARTERS,
        },
        (Kind::Condenser, _) => BuildingArtProfile {
            front_x: 0,
            front_y: 5,
            foundation_width: 37,
            foundation_depth: 40,
            scale: PixelScale::THREE_QUARTERS,
        },
        (Kind::Tower, _) => BuildingArtProfile {
            front_x: 0,
            front_y: 5,
            foundation_width: 37,
            foundation_depth: 39,
            scale: PixelScale::THREE_QUARTERS,
        },
        // The drydocks share their faction's Works foundation; the palisade
        // is authored in a 64x64 cell with its anchor on the cell's front
        // corner, so it needs no offset.
        (Kind::Drydock, Faction::Union) => BuildingArtProfile {
            front_x: 2,
            front_y: 5,
            foundation_width: 51,
            foundation_depth: 50,
            scale: PixelScale::ONE,
        },
        (Kind::Drydock, Faction::Assembly) => BuildingArtProfile {
            front_x: 0,
            front_y: 4,
            foundation_width: 48,
            foundation_depth: 50,
            scale: PixelScale::ONE,
        },
        (Kind::Palisade, _) => BuildingArtProfile {
            front_x: 0,
            front_y: 0,
            foundation_width: 16,
            foundation_depth: 16,
            scale: PixelScale::ONE,
        },
        _ => return None,
    };
    Some(profile)
}

/// The fixed-point rectangle occupied by an entity's logical building
/// footprint.  `left`/`top` are the outer boundary of the top-left cell and
/// `right`/`bottom` are exclusive outer boundaries of the footprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FootprintBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl FootprintBounds {
    /// Minimum projected ground depth (`x + y`) of the footprint.
    pub fn rear_depth(self) -> i64 {
        i64::from(self.left) + i64::from(self.top)
    }

    /// Maximum projected ground depth (`x + y`) of the footprint.
    pub fn front_depth(self) -> i64 {
        i64::from(self.right) + i64::from(self.bottom)
    }

    pub fn center_depth(self) -> i64 {
        (self.rear_depth() + self.front_depth()) / 2
    }

    pub fn contains(self, pos: Pos) -> bool {
        (self.left..self.right).contains(&pos.x) && (self.top..self.bottom).contains(&pos.y)
    }

    /// The south-east outer corner used to align a source art foundation's
    /// front point with the actual collision footprint.
    pub fn front_corner(self) -> Pos {
        Pos::raw(self.right, self.bottom)
    }
}

/// Compute the authoritative geometric footprint without changing it.
pub fn footprint_bounds(entity: &Entity) -> Option<FootprintBounds> {
    footprint_bounds_at(entity.kind, entity.pos)
}

/// Compute a footprint from a kind and raw position when no live Entity is
/// available, such as a remembered building observation.
pub fn footprint_bounds_at(kind: Kind, pos: Pos) -> Option<FootprintBounds> {
    if !kind.is_building() {
        return None;
    }
    let size = spec(kind).footprint.max(1);
    let half_cell = FP / 2;
    let left = pos.x - half_cell;
    let top = pos.y - half_cell;
    Some(FootprintBounds {
        left,
        top,
        right: left + size * FP,
        bottom: top + size * FP,
    })
}

/// The outer south-east/front point of an entity's logical footprint.
#[cfg(test)]
pub fn building_front_corner(entity: &Entity) -> Option<Pos> {
    footprint_bounds(entity).map(FootprintBounds::front_corner)
}

/// The front corner for a kind/position pair without a live Entity.
/// Presentation transform for a world entity.
///
/// The returned `anchor` is a world-space front corner.  A caller projects it
/// through the camera, applies `render_x_offset`/`render_y_offset`, then uses
/// the returned pixel scale when drawing and hit-testing the atlas sprite.
/// For a unit all offsets are zero and the anchor remains its simulation
/// position.  For a building the offsets move the authored local front point
/// back onto the logical corner.  This makes the same transform suitable for
/// both painting and picking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityRenderTransform {
    pub anchor: Pos,
    pub render_x_offset: i32,
    pub render_y_offset: i32,
    pub scale: PixelScale,
}

/// Return a render transform from only persisted kind/position data.
///
/// This is the path for remembered or fogged building observations; it must
/// stay identical to the live-entity wrapper below.
pub fn render_transform(kind: Kind, pos: Pos, faction: Faction) -> EntityRenderTransform {
    let Some(bounds) = footprint_bounds_at(kind, pos) else {
        return EntityRenderTransform {
            anchor: pos,
            render_x_offset: 0,
            render_y_offset: 0,
            scale: PixelScale::ONE,
        };
    };
    let profile = building_art_profile(kind, faction).unwrap_or(BuildingArtProfile {
        front_x: 0,
        front_y: 0,
        foundation_width: 0,
        foundation_depth: 0,
        scale: PixelScale::ONE,
    });
    EntityRenderTransform {
        anchor: bounds.front_corner(),
        render_x_offset: -profile.scale.apply(profile.front_x),
        render_y_offset: -profile.scale.apply(profile.front_y),
        scale: profile.scale,
    }
}

/// Entity wrapper for [`render_transform`].
pub fn entity_render_transform(entity: &Entity, faction: Faction) -> EntityRenderTransform {
    render_transform(entity.kind, entity.pos, faction)
}

/// Ground depth used only as the stable fallback key for partial-order ties.
pub fn entity_depth_key(entity: &Entity) -> i64 {
    footprint_bounds(entity)
        .map(FootprintBounds::center_depth)
        .unwrap_or_else(|| ground_depth(entity.pos))
}

fn ground_depth(pos: Pos) -> i64 {
    i64::from(pos.x) + i64::from(pos.y)
}

/// Pairwise relation for a back-to-front scene pass.
///
/// `Before` means the left entity must be painted first, `After` means it
/// must be painted second, and `Tie` leaves the pair to the stable ID/position
/// fallback.  A unit is compared against a building's footprint interval:
/// north/rear units hide behind the structure, south/front units paint over
/// its near face, and opposite side corners at equal depth remain stable
/// ties.  This is intentionally not a single front-anchor sort key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthRelation {
    Before,
    After,
    Tie,
}

fn invert(relation: DepthRelation) -> DepthRelation {
    match relation {
        DepthRelation::Before => DepthRelation::After,
        DepthRelation::After => DepthRelation::Before,
        DepthRelation::Tie => DepthRelation::Tie,
    }
}

fn relation_by_depth(a: i64, b: i64) -> DepthRelation {
    match a.cmp(&b) {
        Ordering::Less => DepthRelation::Before,
        Ordering::Greater => DepthRelation::After,
        Ordering::Equal => DepthRelation::Tie,
    }
}

fn building_vs_unit(building: &Entity, unit: &Entity) -> DepthRelation {
    let bounds = footprint_bounds(building).expect("building_vs_unit requires a building");
    let point = unit.pos;

    // A unit on one of the four cardinal faces has an unambiguous relation
    // even when its x+y depth lies inside the building interval.  These
    // checks keep an east/west neighbour from inheriting one global
    // front-anchor decision.
    let in_x = (bounds.left..bounds.right).contains(&point.x);
    let in_y = (bounds.top..bounds.bottom).contains(&point.y);
    if in_x && point.y < bounds.top {
        return DepthRelation::After;
    }
    if in_x && point.y >= bounds.bottom {
        return DepthRelation::Before;
    }
    if in_y && point.x < bounds.left {
        return DepthRelation::After;
    }
    if in_y && point.x >= bounds.right {
        return DepthRelation::Before;
    }

    let depth = ground_depth(unit.pos);

    // The explicit interval checks document the geometry and keep units that
    // cross the opposite outside corners on the correct side of the building.
    if depth < bounds.rear_depth() {
        return DepthRelation::After;
    }
    if depth > bounds.front_depth() {
        return DepthRelation::Before;
    }

    // This is the relation of the *building* to the unit.  A rear unit has a
    // smaller depth and must be painted first, so the building is After it;
    // the inversion here is deliberate.
    relation_by_depth(bounds.center_depth(), depth)
}

/// Return the required back-to-front relation for two entities.
pub fn entity_relation(a: &Entity, b: &Entity) -> DepthRelation {
    match (a.kind.is_building(), b.kind.is_building()) {
        (false, false) => relation_by_depth(ground_depth(a.pos), ground_depth(b.pos)),
        (true, false) => building_vs_unit(a, b),
        (false, true) => invert(building_vs_unit(b, a)),
        (true, true) => {
            let a_bounds = footprint_bounds(a).expect("building bounds");
            let b_bounds = footprint_bounds(b).expect("building bounds");
            if a_bounds.front_depth() < b_bounds.rear_depth() {
                DepthRelation::Before
            } else if a_bounds.rear_depth() > b_bounds.front_depth() {
                DepthRelation::After
            } else {
                relation_by_depth(a_bounds.center_depth(), b_bounds.center_depth())
            }
        }
    }
}

fn entity_tie_key(entity: &Entity) -> (i64, u8, u32, i32, i32) {
    (
        entity_depth_key(entity),
        u8::from(entity.kind.is_building()),
        entity.id,
        entity.pos.x,
        entity.pos.y,
    )
}

/// Stable topological back-to-front order for a set of entities.
///
/// Strict footprint relations become directed edges.  Ties use fixed-point
/// depth, position, and entity ID, so repeated renders and picking agree.  A
/// malformed future fixture can produce a cycle (for example overlapping
/// static footprints); in that case the smallest remaining tie key is emitted
/// and its outgoing edges are released.  That deterministic cycle break keeps
/// the renderer total without making an art decision authoritative.
pub fn stable_entity_order(entities: &[&Entity]) -> Vec<usize> {
    let count = entities.len();
    let mut outgoing = vec![BTreeSet::new(); count];
    let mut indegree = vec![0usize; count];

    for left in 0..count {
        for right in (left + 1)..count {
            let edge = match entity_relation(entities[left], entities[right]) {
                DepthRelation::Before => Some((left, right)),
                DepthRelation::After => Some((right, left)),
                DepthRelation::Tie => None,
            };
            if let Some((from, to)) = edge
                && outgoing[from].insert(to)
            {
                indegree[to] += 1;
            }
        }
    }

    let mut remaining = vec![true; count];
    let mut ordered = Vec::with_capacity(count);
    while ordered.len() < count {
        let candidate = (0..count)
            .filter(|&index| remaining[index] && indegree[index] == 0)
            .min_by_key(|&index| entity_tie_key(entities[index]))
            .or_else(|| {
                // A cycle is not expected for valid non-overlapping map
                // entities, but a deterministic release is safer than
                // making sort order depend on the allocator or hash state.
                (0..count)
                    .filter(|&index| remaining[index])
                    .min_by_key(|&index| entity_tie_key(entities[index]))
            })
            .expect("remaining entity must have an order candidate");

        remaining[candidate] = false;
        ordered.push(candidate);
        let targets: Vec<_> = outgoing[candidate].iter().copied().collect();
        for target in targets {
            indegree[target] = indegree[target].saturating_sub(1);
        }
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Camera;

    fn fixture(kind: Kind, pos: Pos, id: u32) -> Entity {
        let world = bw_sim::World::new(0x0c01_5ea5, Faction::Union);
        // The fixture only needs a fully initialized Entity.  Reusing the
        // first owned headquarters and changing its kind keeps this test
        // independent of private spawn helpers for later building kinds.
        let mut entity = world
            .entities
            .into_iter()
            .find(|entity| entity.owner == 0)
            .expect("world fixture entity");
        entity.id = id;
        entity.kind = kind;
        entity.pos = pos;
        entity
    }

    #[test]
    fn front_corner_uses_the_actual_outer_footprint_boundary() {
        let entity = fixture(Kind::Headquarters, Pos::cell(10, 20), 7);
        let bounds = footprint_bounds(&entity).expect("building footprint");
        assert_eq!(bounds.left, entity.pos.x - FP / 2);
        assert_eq!(bounds.top, entity.pos.y - FP / 2);
        assert_eq!(bounds.right, entity.pos.x - FP / 2 + 4 * FP);
        assert_eq!(bounds.bottom, entity.pos.y - FP / 2 + 4 * FP);
        assert_eq!(building_front_corner(&entity), Some(bounds.front_corner()));

        let transform = entity_render_transform(&entity, Faction::Union);
        assert_eq!(
            transform,
            render_transform(entity.kind, entity.pos, Faction::Union),
            "remembered and live entities must share one render transform"
        );
        assert_eq!(transform.anchor, bounds.front_corner());
        // union_hq's authored diamond ends at (-2, +5) from the atlas anchor.
        assert_eq!(transform.render_x_offset, 2);
        assert_eq!(transform.render_y_offset, -5);
        assert_eq!(transform.scale, PixelScale::ONE);
    }

    #[test]
    fn two_by_two_profiles_use_the_documented_three_quarter_scale() {
        for kind in [Kind::Dropoff, Kind::Condenser, Kind::Tower] {
            let profile = building_art_profile(kind, Faction::Union).expect("2x2 profile");
            assert_eq!(profile.scale, PixelScale::THREE_QUARTERS);
            assert_eq!(profile.front_x, 0);
            assert_eq!(profile.front_y, 5);
            let entity = fixture(kind, Pos::cell(30, 30), 12);
            let transform = entity_render_transform(&entity, Faction::Union);
            assert_eq!(transform.render_y_offset, -4);
        }
    }

    #[test]
    fn front_anchor_shift_matches_the_footprint_size_in_projected_pixels() {
        let camera = Camera::default();
        for (kind, size) in [
            (Kind::Headquarters, 4),
            (Kind::Works, 3),
            (Kind::Dropoff, 2),
        ] {
            let entity = fixture(kind, Pos::cell(30, 30), 40 + size as u32);
            let transform = entity_render_transform(&entity, Faction::Union);
            let old_center = Pos::raw(
                entity.pos.x + (size - 1) * FP / 2,
                entity.pos.y + (size - 1) * FP / 2,
            );
            let (old_x, old_y) = camera.project(old_center);
            let (front_x, front_y) = camera.project(transform.anchor);
            // The old centre anchor is exactly n/2 cells behind the outer
            // front corner, or 8*n native screen pixels in y.
            assert_eq!(front_x - old_x, 0);
            assert_eq!(front_y - old_y, 8 * size);
        }
    }

    #[test]
    fn rear_and_front_units_split_around_the_building_depth_interval() {
        let building = fixture(Kind::Works, Pos::cell(10, 10), 100);
        let rear = fixture(Kind::Hook, Pos::cell(10, 8), 1);
        let front = fixture(Kind::Hook, Pos::cell(10, 14), 2);
        let west_rear = fixture(Kind::Hook, Pos::cell(8, 11), 3);
        let east_front = fixture(Kind::Hook, Pos::cell(13, 11), 4);

        assert_eq!(entity_relation(&rear, &building), DepthRelation::Before);
        assert_eq!(entity_relation(&building, &rear), DepthRelation::After);
        assert_eq!(entity_relation(&building, &front), DepthRelation::Before);
        assert_eq!(
            entity_relation(&west_rear, &building),
            DepthRelation::Before
        );
        assert_eq!(
            entity_relation(&building, &east_front),
            DepthRelation::Before
        );

        let north_side_a = fixture(Kind::Hook, Pos::cell(10, 8), 5);
        let north_side_b = fixture(Kind::Hook, Pos::cell(12, 8), 6);
        let south_side_a = fixture(Kind::Hook, Pos::cell(10, 14), 7);
        let south_side_b = fixture(Kind::Hook, Pos::cell(12, 14), 8);
        let west_side_a = fixture(Kind::Hook, Pos::cell(8, 10), 9);
        let west_side_b = fixture(Kind::Hook, Pos::cell(8, 12), 10);
        let east_side_a = fixture(Kind::Hook, Pos::cell(13, 10), 11);
        let east_side_b = fixture(Kind::Hook, Pos::cell(13, 12), 12);

        for unit in [&north_side_a, &north_side_b, &west_side_a, &west_side_b] {
            assert_eq!(
                entity_relation(&building, unit),
                DepthRelation::After,
                "rear cardinal face must be covered by the building"
            );
        }
        for unit in [&south_side_a, &south_side_b, &east_side_a, &east_side_b] {
            assert_eq!(
                entity_relation(&building, unit),
                DepthRelation::Before,
                "front cardinal face must cover the building"
            );
        }
    }

    #[test]
    fn opposite_outside_corners_are_equal_depth_and_stably_tied() {
        let building = fixture(Kind::Works, Pos::cell(10, 10), 100);
        // These points sit at opposite outside corners with equal x+y depth.
        let north_east = fixture(Kind::Hook, Pos::cell(13, 9), 21);
        let south_west = fixture(Kind::Hook, Pos::cell(9, 13), 22);
        assert_eq!(ground_depth(north_east.pos), ground_depth(south_west.pos));
        assert_eq!(entity_relation(&north_east, &building), DepthRelation::Tie);
        assert_eq!(entity_relation(&south_west, &building), DepthRelation::Tie);

        let entities = [&south_west, &building, &north_east];
        let order = stable_entity_order(&entities);
        assert_eq!(order.len(), entities.len());
        let ids: Vec<_> = order.into_iter().map(|index| entities[index].id).collect();
        // No allocator/hash order may decide the equal-depth pair.
        assert_eq!(ids, vec![21, 22, 100]);
    }

    #[test]
    fn topological_order_preserves_strict_edges_and_is_repeatable() {
        let building = fixture(Kind::Headquarters, Pos::cell(10, 10), 30);
        let rear = fixture(Kind::Hook, Pos::cell(10, 6), 31);
        let front = fixture(Kind::Hook, Pos::cell(10, 16), 32);
        let entities = [&front, &building, &rear];
        let first = stable_entity_order(&entities);
        let second = stable_entity_order(&entities);
        assert_eq!(first, second);
        let ids: Vec<_> = first.iter().map(|&index| entities[index].id).collect();
        assert_eq!(ids.first(), Some(&31));
        assert_eq!(ids.last(), Some(&32));
        assert!(ids.iter().position(|id| *id == 30).unwrap() > 0);
        assert!(ids.iter().position(|id| *id == 30).unwrap() < ids.len() - 1);
    }
}
