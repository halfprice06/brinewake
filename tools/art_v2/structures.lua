-- BRINEWAKE structures art v2.
--
-- This is Lua-authored native pixel geometry.  The generated faction and
-- battlefield boards, plus the Union/Assembly detail references, were used as
-- construction studies only; no reference pixels are imported, sampled, or
-- downscaled here.  The renderer supplies the palette, layers, bounds, and
-- projection through the contract in tools/art_v2/contract.md.

local M = {}

local function phase4(phase)
  if phase == nil then
    return 0
  end
  return phase % 4
end

local function pt(f, s, z)
  return { f, s, z }
end

local function pxy(ctx, f, s, z)
  return ctx:p(f, s, z)
end

local function line3(ctx, a, b, color, width)
  ctx:line3(a, b, color, width)
end

local function poly3(ctx, points, color)
  ctx:poly3(points, color)
end

local function box(ctx, f, s, z, hf, hs, height, top, light, dark)
  ctx:box(f, s, z, hf, hs, height, top, light, dark)
end

local function extrude(ctx, footprint, z, height, top, light, dark)
  ctx:extrude(footprint, z, height, top, light, dark)
end

local function disc(ctx, center, axis_a, axis_b, radius, color)
  ctx:disc(center, axis_a, axis_b, radius, color)
end

-- Facing 1 is the runner's fixed building presentation: local f=s spans
-- project to a clean 2:1 horizontal screen axis while the two ground faces
-- remain visible.  These axes are only used by disc; all solids stay local.
local SCREEN_AXIS = { 0.707, 0.707, 0 }
local DEPTH_AXIS = { 0.707, -0.707, 0 }
local UP_AXIS = { 0, 0, 1 }
local F_AXIS = { 1, 0, 0 }
local S_AXIS = { 0, 1, 0 }

local function screen_disc(ctx, f, s, z, radius, color)
  disc(ctx, pt(f, s, z), SCREEN_AXIS, UP_AXIS, radius, color)
end

local function flat_disc(ctx, f, s, z, radius, color)
  disc(ctx, pt(f, s, z), F_AXIS, S_AXIS, radius, color)
end

local function bolt(ctx, f, s, z, color)
  local x, y = pxy(ctx, f, s, z)
  ctx:pixel(x, y, color)
  ctx:pixel(x + 1, y, color)
end

local function pipe(ctx, f, s, z0, z1, outer, inner)
  line3(ctx, pt(f, s, z0), pt(f, s, z1), "deep", 4)
  line3(ctx, pt(f, s, z0), pt(f, s, z1), outer, 3)
  if inner then
    line3(ctx, pt(f, s, z0 + 2), pt(f, s, z1 - 2), inner, 1)
  end
end

local function pipe_elbow(ctx, a, b, c, outer, inner)
  pipe(ctx, a[1], a[2], a[3], b[3], outer, inner)
  line3(ctx, pt(a[1], a[2], b[3]), b, outer, 2)
  line3(ctx, b, c, outer, 2)
end

local function screen_panel(ctx, f, s, z, width, height, color, rim)
  -- A plane following f=s reads face-on in the fixed diagonal building view.
  -- f and s are the actual local ground center of the panel.  Keeping those
  -- coordinates explicit avoids confusing a screen-axis center with a local
  -- ground point and accidentally floating a window away from its wall.
  -- width is in screen pixels; local half-width is reduced for the projection.
  local h = width * 0.35
  local f0, s0 = f - h, s - h
  local f1, s1 = f + h, s + h
  poly3(ctx, {
    pt(f0, s0, z), pt(f1, s1, z),
    pt(f1, s1, z + height), pt(f0, s0, z + height),
  }, color)
  if rim then
    line3(ctx, pt(f0, s0, z), pt(f1, s1, z), rim, 1)
    line3(ctx, pt(f0, s0, z + height), pt(f1, s1, z + height), rim, 1)
  end
end

local function screen_panel_side(ctx, f, s, z, width, height, color)
  local h = width * 0.35
  local f0, s0 = f - h, s - h
  local f1, s1 = f + h, s + h
  poly3(ctx, {
    pt(f0, s0, z), pt(f1, s1, z),
    pt(f1, s1, z + height), pt(f0, s0, z + height),
  }, color)
end

local function screen_rail(ctx, t0, t1, depth, z, color, width)
  line3(ctx, pt(t0 - depth, t0 + depth, z),
    pt(t1 - depth, t1 + depth, z), color, width)
end

local function cross_rail(ctx, t, d0, d1, z, color, width)
  line3(ctx, pt(t - d0, t + d0, z), pt(t - d1, t + d1, z), color, width)
end

local function hull_foot(t, length, depth)
  -- A tapered tug hull: q follows the screen-horizontal f=s axis, d is the
  -- shallow ground-depth axis.  The near corner stays behind the 16 px cell
  -- margin when the origin is the requested bottom anchor.
  local a, b = t - length, t + length
  return {
    { a - depth, a + depth }, { a + 4 - depth, a + 4 + depth },
    { b - 4 - depth, b - 4 + depth }, { b + depth, b - depth },
    { b - 4 + depth, b - 4 - depth }, { a + 4 + depth, a + 4 - depth },
    { a + depth, a - depth },
  }
end

local function platform_foot()
  -- Balanced around the screen body, shifted slightly toward the rear local
  -- f axis so the ground contact remains inside a 128x128 cell.
  return {
    { -18, -40 }, { 52, -40 }, { 62, -27 },
    { 55, 8 }, { 30, 20 }, { -15, 20 },
    { -27, 6 }, { -25, -25 },
  }
end

local function arch_point(t, depth, z)
  return pt(t - depth, t + depth, z)
end

local ARCH_T = { -34, -34, -29, -21, -11, 0, 11, 21, 29, 34, 34 }
local ARCH_Z = { 0, 22, 43, 59, 70, 74, 70, 59, 43, 22, 0 }

local function arch_path(depth, lift)
  local path = {}
  for i = 1, #ARCH_T do
    path[i] = arch_point(ARCH_T[i], depth, ARCH_Z[i] + (lift or 0))
  end
  return path
end

local function draw_contact(ctx, rx, ry)
  ctx:layer("shadows")
  -- The shared building anchor leaves sixteen pixels below cy.  Keep the
  -- short shadow inside that margin even when a caller asks for a broad
  -- silhouette shadow.
  ctx:shadow(rx, math.min(ry, 13))
end

local function wear_seams(ctx, t, depth, z, width, color)
  -- Three intentional seams imply repairs without turning the surface into
  -- procedural noise.
  local h = width * 0.35
  line3(ctx, pt(t - h - depth, t - h + depth, z),
    pt(t - h - depth, t - h + depth, z + 5), color, 1)
  line3(ctx, pt(t + h * 0.35 - depth, t + h * 0.35 + depth, z + 2),
    pt(t + h * 0.35 - depth, t + h * 0.35 + depth, z + 6), color, 1)
end

local function wheel_spokes(ctx, f, s, z, radius, k, color)
  -- screen_disc uses the local screen axis (f=s) and the vertical z axis.
  -- The old helper mixed f/s cross-lines with that plane, which made spokes
  -- drift off the wheel face.  Keep every spoke endpoint on the same two
  -- axes so the hub and face remain concentric at native resolution.
  local spoke = (k % 4)
  local h = (radius - 2) * 0.707
  local v = radius - 2
  if spoke == 0 then
    line3(ctx, pt(f - h, s - h, z), pt(f + h, s + h, z), color, 1)
  elseif spoke == 1 then
    line3(ctx, pt(f, s, z - v), pt(f, s, z + v), color, 1)
  elseif spoke == 2 then
    line3(ctx, pt(f - h * 0.7, s - h * 0.7, z - v * 0.7),
      pt(f + h * 0.7, s + h * 0.7, z + v * 0.7), color, 1)
  else
    line3(ctx, pt(f - h * 0.7, s - h * 0.7, z + v * 0.7),
      pt(f + h * 0.7, s + h * 0.7, z - v * 0.7), color, 1)
  end
end

local function flywheel(ctx, t, depth, z, radius, k, outer, face)
  screen_disc(ctx, t - depth, t + depth, z, radius + 2, "deep")
  screen_disc(ctx, t - depth, t + depth, z, radius, outer)
  screen_disc(ctx, t - depth, t + depth, z, radius - 3, face)
  wheel_spokes(ctx, t - depth, t + depth, z, radius - 3, k, "orange3")
  screen_disc(ctx, t - depth, t + depth, z, 2, "steel0")
end

local function hatch(ctx, f, s, z, width, height, frame, fill)
  screen_panel(ctx, f, s, z, width, height, fill, frame)
  line3(ctx, pt(f - width * 0.3, s - width * 0.3, z + height * 0.45),
    pt(f + width * 0.3, s + width * 0.3, z + height * 0.45), frame, 1)
  bolt(ctx, f - width * 0.23, s - width * 0.23, z + 3, "steel3")
  bolt(ctx, f + width * 0.23, s + width * 0.23, z + height - 3, "steel3")
end

-- BREAKWATER UNION HEADQUARTERS --------------------------------------------

local function union_hq(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  draw_contact(ctx, 51, 18)

  ctx:layer("far")
  extrude(ctx, platform_foot(), 0, 4, "steel1", "steel0", "deep")
  -- Rear tug hulls and their exposed undercarriages.
  extrude(ctx, hull_foot(-24, 17, 7), 3, 23, "cream1", "orange1", "orange0")
  extrude(ctx, hull_foot(24, 17, 7), 3, 23, "cream1", "orange1", "orange0")
  box(ctx, -24, -24, 3, 13, 4, 5, "steel1", "steel0", "deep")
  box(ctx, 24, 24, 3, 13, 4, 5, "steel1", "steel0", "deep")
  -- Court floor is deliberately quiet so the three hulls and gantry read as
  -- one functioning maintenance yard.
  extrude(ctx, {
    { -20, -22 }, { 30, -22 }, { 38, -8 },
    { 29, 9 }, { -13, 9 }, { -28, -7 },
  }, 4, 4, "steel2", "steel1", "deep")

  ctx:layer("body")
  -- Front portions of the tug hulls get warmer planes and a hard lower seam.
  extrude(ctx, hull_foot(-24, 16, 7), 9, 21, "cream2", "orange2", "orange0")
  extrude(ctx, hull_foot(24, 16, 7), 9, 21, "cream2", "orange2", "orange0")
  box(ctx, -24, -24, 8, 12, 3, 4, "steel0", "steel1", "deep")
  box(ctx, 24, 24, 8, 12, 3, 4, "steel0", "steel1", "deep")
  -- Central repurposed tug cabin bridges the rear of the open court.
  box(ctx, 3, -10, 5, 18, 12, 28, "steel1", "steel2", "steel0")
  box(ctx, 1, -17, 28, 12, 3, 8, "cream1", "cream2", "orange0")
  hatch(ctx, 1, -20, 31, 18, 7, "orange1", "cream2")
  -- Side winches make the lower hulls functional silhouettes.
  flywheel(ctx, -24, -31, 16, 8, (k + 1) % 4, "steel1", "orange1")
  flywheel(ctx, 24, 11, 16, 8, k, "steel1", "orange1")

  ctx:layer("upper")
  -- Two offset wheelhouse cabs: broad glazing clusters, not flat windows.
  box(ctx, -24, -24, 24, 10, 8, 18, "cream2", "cream1", "orange0")
  box(ctx, 24, 24, 24, 10, 8, 18, "cream2", "cream1", "orange0")
  screen_panel(ctx, -24, -31, 27, 17, 9, "glass1", "steel0")
  screen_panel(ctx, 24, 11, 27, 17, 9, "glass1", "steel0")
  screen_panel_side(ctx, -25, -16, 27, 12, 9, "glass2")
  screen_panel_side(ctx, 23, 26, 27, 12, 9, "glass2")
  -- Cab roofs have a warm cap and one raised exhaust each.
  box(ctx, -24, -24, 42, 11, 8, 3, "orange2", "orange1", "orange0")
  box(ctx, 24, 24, 42, 11, 8, 3, "orange2", "orange1", "orange0")
  pipe(ctx, -18, -23, 45, 57, "steel2", "cream2")
  pipe(ctx, 30, 25, 45, 57, "steel2", "cream2")
  flat_disc(ctx, -18, -23, 57, 4, "steel0")
  flat_disc(ctx, 30, 25, 57, 4, "steel0")

  ctx:layer("front")
  -- Open maintenance court gantry: heavy twin posts, offset crossbeam, and
  -- two-tier railings.  The beam stays screen-up because its local span uses
  -- f=s; no raster rotation is applied.
  pipe(ctx, -30, -26, 4, 66, "orange1", "cream2")
  pipe(ctx, 30, 26, 4, 66, "orange1", "cream2")
  line3(ctx, pt(-30, -26, 66), pt(30, 26, 66), "deep", 5)
  line3(ctx, pt(-30, -26, 66), pt(30, 26, 66), "orange2", 3)
  line3(ctx, pt(-28, -24, 70), pt(28, 24, 70), "cream2", 2)
  -- Front rail follows the court edge and leaves the center opening visible.
  screen_rail(ctx, -30, -5, near * 9, 39, "deep", 3)
  screen_rail(ctx, -30, -5, near * 9, 42, "orange2", 1)
  screen_rail(ctx, 5, 30, near * 9, 39, "deep", 3)
  screen_rail(ctx, 5, 30, near * 9, 42, "orange2", 1)
  for _, t in ipairs({ -30, -18, -6, 6, 18, 30 }) do
    line3(ctx, pt(t - near * 9, t + near * 9, 39),
      pt(t - near * 9, t + near * 9, 45), "orange1", 2)
  end
  -- A front service hatch and two braced tow plates anchor the court.
  hatch(ctx, -8, near * 8, 10, 16, 13, "cream2", "steel0")
  hatch(ctx, 14, near * 8, 10, 14, 13, "orange1", "cream1")

  ctx:layer("details")
  -- Glazing highlights and deliberately sparse repair marks.
  line3(ctx, pt(-29, -31, 34), pt(-20, -31, 34), "glass3", 1)
  line3(ctx, pt(19, 11, 34), pt(28, 11, 34), "glass3", 1)
  wear_seams(ctx, -24, -31, 12, 16, "orange0")
  wear_seams(ctx, 24, 11, 12, 16, "orange0")
  bolt(ctx, -32, -24, 22, "cream3")
  bolt(ctx, 29, 27, 22, "cream3")
  bolt(ctx, -8, near * 8, 18, "steel3")
  bolt(ctx, 14, near * 8, 18, "cream3")
  -- Orange hazard bars are grouped at the work opening, not tiled everywhere.
  for i = 0, 2 do
    line3(ctx, pt(-16 + i * 5, near * 9, 13),
      pt(-13 + i * 5, near * 9, 18), "orange3", 1)
  end
  -- The hoist block and hanging hook sell the maintenance court at native 1x.
  line3(ctx, pt(4, 2, 66), pt(4, 2, 47), "ink", 2)
  box(ctx, 4, 2, 43, 3, 3, 4, "orange1", "steel2", "deep")
  line3(ctx, pt(4, 2, 43), pt(7, 2, 39), "deep", 2)
  line3(ctx, pt(7, 2, 39), pt(9, 2, 42), "orange2", 2)
  line3(ctx, pt(9, 2, 42), pt(8, 2, 44), "deep", 1)
end

-- SILT ASSEMBLY HEADQUARTERS -----------------------------------------------

local function woven_arch(ctx, depth, base_color, edge_color, lift)
  local path = arch_path(depth, lift)
  for i = 1, #path - 1 do
    line3(ctx, path[i], path[i + 1], "deep", 6)
    line3(ctx, path[i], path[i + 1], base_color, 4)
  end
  -- Interleaving bands follow the curve at a few joints; no checker noise.
  for _, i in ipairs({ 2, 4, 6, 8, 10 }) do
    local a = path[i]
    local b = arch_point(ARCH_T[i] + (i < 6 and 3 or -3), depth + 3, ARCH_Z[i] + (lift or 0) - 3)
    line3(ctx, a, b, edge_color, 2)
  end
end

local function assembly_hq(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  draw_contact(ctx, 50, 18)

  ctx:layer("far")
  extrude(ctx, {
    { -22, -39 }, { 48, -39 }, { 59, -25 },
    { 51, 7 }, { 28, 19 }, { -16, 19 },
    { -28, 5 }, { -27, -24 },
  }, 0, 4, "jade1", "jade0", "plum0")
  -- Open floor and stilt feet establish the woven workshop's ground contact.
  extrude(ctx, {
    { -23, -25 }, { 39, -25 }, { 47, -10 },
    { 35, 10 }, { -16, 10 }, { -31, -8 },
  }, 4, 5, "straw1", "jade0", "plum0")
  for _, t in ipairs({ -26, 0, 27 }) do
    box(ctx, t, t - near * 7, 0, 4, 4, 10, "plum1", "plum0", "deep")
    flat_disc(ctx, t, t - near * 7, 0, 5, "steel0")
    flat_disc(ctx, t, t - near * 7, 2, 3, "steel2")
  end

  ctx:layer("body")
  -- Two deep arch ribs, kept parallel in the local depth axis, create the
  -- thick woven shell and preserve a large readable maintenance opening.
  woven_arch(ctx, -5, "jade1", "straw2", 0)
  woven_arch(ctx, 5, "jade2", "straw3", 0)
  -- Low side baskets and a dark floor well give the arch a real interior.
  extrude(ctx, {
    { -29, -19 }, { -12, -22 }, { -7, -10 },
    { -12, 4 }, { -27, 1 }, { -35, -9 },
  }, 9, 16, "jade1", "jade2", "jade0")
  extrude(ctx, {
    { 16, -20 }, { 36, -18 }, { 42, -6 },
    { 34, 6 }, { 18, 3 }, { 12, -9 },
  }, 9, 16, "jade1", "jade2", "jade0")
  extrude(ctx, {
    { -20, -14 }, { 25, -14 }, { 31, -2 },
    { 22, 8 }, { -15, 8 }, { -27, -2 },
  }, 10, 2, "plum0", "plum1", "deep")

  ctx:layer("upper")
  -- Pressure heart: an amber canister hangs in the open court, suspended by
  -- a dark collar and fed by a short woven pipe.
  screen_disc(ctx, 0, near * 4, 28, 14, "plum0")
  screen_disc(ctx, 0, near * 4, 29, 11, "amber1")
  screen_disc(ctx, 0, near * 4, 30, 8, "amber2")
  box(ctx, 0, near * 4, 39, 4, 4, 6, "plum1", "plum2", "plum0")
  box(ctx, 0, near * 4, 24, 4, 4, 4, "plum1", "plum2", "plum0")
  pipe(ctx, 0, near * 4, 45, 60, "steel2", "straw2")
  flat_disc(ctx, 0, near * 4, 60, 4, "steel0")
  -- Woven floor beams and arch ties make the load path legible.
  for _, t in ipairs({ -27, -14, 14, 27 }) do
    line3(ctx, pt(t - near * 5, t + near * 5, 14),
      pt(t - near * 5, t + near * 5, 37), "straw1", 2)
  end
  for _, t in ipairs({ -21, 0, 21 }) do
    cross_rail(ctx, t, -5, 5, 58, "straw2", 2)
  end

  ctx:layer("front")
  -- Front hem, slatted walkway, and hanging canvas create the open-floor read.
  screen_rail(ctx, -28, 30, near * 10, 17, "plum0", 4)
  screen_rail(ctx, -27, 29, near * 10, 20, "jade3", 2)
  for _, t in ipairs({ -24, -12, 0, 12, 24 }) do
    line3(ctx, pt(t - near * 10, t + near * 10, 17),
      pt(t - near * 10, t + near * 10, 24), "straw1", 2)
  end
  -- Side pressure capsules and black collars echo the reference material.
  screen_disc(ctx, -22, near * 8, 25, 7, "plum0")
  screen_disc(ctx, -22, near * 8, 25, 5, "amber2")
  screen_disc(ctx, 22, near * 8, 25, 7, "plum0")
  screen_disc(ctx, 22, near * 8, 25, 5, "amber1")
  line3(ctx, pt(-22, near * 8, 18), pt(-22, near * 8, 32), "straw2", 2)
  line3(ctx, pt(22, near * 8, 18), pt(22, near * 8, 32), "straw2", 2)
  -- A low entry ramp remains clear of the pressure heart.
  extrude(ctx, {
    { -16, 11 }, { 16, 11 }, { 21, 17 }, { -20, 17 },
  }, 4, 4, "straw2", "straw1", "plum0")

  ctx:layer("details")
  -- Bright upper-left edges are selective, while dark weave seams establish
  -- thickness and avoid pillow-shading the entire arch.
  -- These accents follow real rib segments; isolated vertical strokes would
  -- read as floating debris beside the arch at 1x.
  local highlight_arch = arch_path(-5, 0)
  for _, seg in ipairs({ { 2, 3 }, { 3, 4 }, { 4, 5 }, { 7, 8 }, { 8, 9 }, { 9, 10 } }) do
    line3(ctx, highlight_arch[seg[1]], highlight_arch[seg[2]], "straw3", 1)
  end
  -- Pressure highlight and two gauge ticks are the only strongest accents.
  if k == 1 or k == 3 then
    line3(ctx, pt(-4, near * 4, 31), pt(0, near * 4, 34), "amber3", 2)
  else
    line3(ctx, pt(0, near * 4, 34), pt(4, near * 4, 31), "amber3", 2)
  end
  bolt(ctx, -22, near * 8, 25, "straw3")
  bolt(ctx, 22, near * 8, 25, "straw3")
  bolt(ctx, -27, near * 10, 20, "jade3")
  bolt(ctx, 26, near * 10, 20, "jade3")
end

-- UNION WORKS ---------------------------------------------------------------

local function union_works(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  draw_contact(ctx, 48, 17)

  ctx:layer("far")
  extrude(ctx, platform_foot(), 0, 4, "steel0", "deep", "shadow")
  -- The factory mass is squat and rectangular, unlike the three-hull HQ.
  box(ctx, 20, -10, 4, 36, 28, 30, "steel1", "steel2", "steel0")
  box(ctx, 20, -10, 34, 34, 26, 4, "orange1", "orange2", "orange0")
  -- Rear pipe forest is sparse and staggered.
  pipe(ctx, 43, -13, 4, 55, "steel1", "steel3")
  pipe(ctx, 45, -6, 4, 47, "orange1", "cream2")

  ctx:layer("body")
  -- Front buttress columns and a deep roller bay distinguish production from
  -- the HQ's open court.
  box(ctx, 2, near * 17, 7, 8, 5, 25, "orange1", "orange2", "orange0")
  box(ctx, 38, near * 17, 7, 8, 5, 25, "orange1", "orange2", "orange0")
  hatch(ctx, 20, near * 19, 8, 25, 18, "cream2", "steel0")
  screen_disc(ctx, 20, near * 20, 25, 12, "deep")
  screen_disc(ctx, 20, near * 20, 25, 9, "steel1")
  screen_disc(ctx, 20, near * 20, 25, 6, "steel2")
  screen_disc(ctx, 20, near * 20, 25, 2, "orange2")
  wheel_spokes(ctx, 20, near * 20, 25, 7, k, "cream3")
  -- Side service housings step outward from the factory body.
  box(ctx, -8, near * 22, 12, 7, 4, 18, "cream1", "orange1", "orange0")
  box(ctx, 48, near * 4, 12, 7, 4, 18, "cream1", "orange1", "orange0")

  ctx:layer("upper")
  -- Upper press cage and a conveyor bridge read as factory machinery at 1x.
  box(ctx, 20, -5, 38, 12, 12, 16, "steel2", "steel1", "deep")
  screen_panel(ctx, 20, -17, 42, 18, 8, "glass1", "steel0")
  line3(ctx, pt(9, -17, 51), pt(31, -17, 51), "glass3", 1)
  pipe(ctx, 20, -5, 54, 69, "steel2", "cream2")
  flat_disc(ctx, 20, -5, 69, 5, "steel0")
  -- Roof ribs and two vent housings break the broad sheet with load-bearing
  -- clusters that stay attached to the actual roof plane.
  for _, s in ipairs({ -30, -18, -6, 6 }) do
    line3(ctx, pt(-7, s, 38), pt(45, s, 38), "orange0", 2)
    line3(ctx, pt(-5, s + 1, 39), pt(43, s + 1, 39), "orange2", 1)
  end
  box(ctx, 8, -25, 38, 4, 4, 3, "steel0", "steel1", "deep")
  box(ctx, 33, -20, 38, 4, 4, 3, "steel0", "steel1", "deep")
  line3(ctx, pt(5, -28, 41), pt(11, -28, 41), "steel3", 1)
  line3(ctx, pt(30, -23, 41), pt(36, -23, 41), "steel3", 1)
  -- Orange side brackets and dark belt under the press.
  box(ctx, 8, -15, 37, 3, 3, 10, "orange2", "orange1", "orange0")
  box(ctx, 32, -15, 37, 3, 3, 10, "orange2", "orange1", "orange0")
  line3(ctx, pt(5, near * 16, 15), pt(35, near * 16, 15), "deep", 5)
  line3(ctx, pt(5, near * 16, 17), pt(35, near * 16, 17), "steel2", 2)

  ctx:layer("front")
  -- Chimney top and warning collar make the Works silhouette factory-like.
  pipe(ctx, 46, 8, 32, 58, "steel0", "steel3")
  box(ctx, 46, 8, 56, 6, 6, 4, "orange1", "orange2", "orange0")
  flat_disc(ctx, 46, 8, 60, 4, "steel0")
  -- Crane trolley over the roller bay.
  line3(ctx, pt(4, 6, 62), pt(43, 6, 62), "deep", 4)
  line3(ctx, pt(4, 6, 62), pt(43, 6, 62), "orange2", 2)
  box(ctx, 26, 6, 55, 3, 3, 5, "steel1", "steel2", "deep")
  line3(ctx, pt(26, 6, 55), pt(26, 6, 39), "ink", 1)
  box(ctx, 26, 6, 35, 2, 2, 4, "orange1", "steel2", "deep")
  -- A framed roller bay and two inspection windows belong to the front wall;
  -- all panels use explicit local f/s centers so they cannot float free.
  screen_panel(ctx, 20, near * 27, 9, 38, 22, "steel0", "steel2")
  line3(ctx, pt(1, near * 27, 9), pt(39, near * 27, 9), "orange1", 2)
  line3(ctx, pt(1, near * 27, 31), pt(39, near * 27, 31), "orange2", 2)
  screen_panel(ctx, 7, near * 25, 17, 12, 8, "glass1", "orange1")
  screen_panel(ctx, 35, near * 25, 17, 12, 8, "glass1", "orange1")
  line3(ctx, pt(2, near * 25, 21), pt(12, near * 25, 21), "glass3", 1)
  line3(ctx, pt(30, near * 25, 21), pt(40, near * 25, 21), "glass3", 1)
  -- The camera-left factory wall is the actual f=-16 extrusion plane.
  -- Its glazing and mullions follow s/z rather than a camera-facing billboard.
  for _, span in ipairs({ { -30, -15 }, { -9, 7 } }) do
    local a, b = span[1], span[2]
    poly3(ctx, { pt(-16, a, 16), pt(-16, b, 16),
      pt(-16, b, 28), pt(-16, a, 28) }, "steel0")
    poly3(ctx, { pt(-16, a + 2, 19), pt(-16, b - 2, 19),
      pt(-16, b - 2, 26), pt(-16, a + 2, 26) }, "glass1")
    line3(ctx, pt(-16, a + 2, 26), pt(-16, b - 2, 26), "glass3", 1)
    line3(ctx, pt(-16, (a + b) / 2, 18), pt(-16, (a + b) / 2, 27), "steel1", 1)
  end
  for _, s in ipairs({ -35, -12, 13 }) do
    line3(ctx, pt(-16, s, 5), pt(-16, s, 33), "steel1", 2)
    bolt(ctx, -16, s, 9, "steel3")
  end
  line3(ctx, pt(-16, -37, 12), pt(-16, 17, 12), "steel1", 1)
  -- Edge rails are physically attached to the factory's s=18 wall lip.
  line3(ctx, pt(-16, 18, 34), pt(55, 18, 34), "orange0", 3)
  line3(ctx, pt(-16, 18, 36), pt(55, 18, 36), "orange2", 1)

  ctx:layer("details")
  wear_seams(ctx, 20, near * 20, 10, 24, "orange0")
  for _, t in ipairs({ 2, 38 }) do
    bolt(ctx, t, near * 22, 16, "cream3")
  end
  line3(ctx, pt(8, near * 19, 21), pt(32, near * 19, 21), "orange3", 1)
  line3(ctx, pt(13, near * 17, 47), pt(27, near * 17, 47), "glass3", 1)
  -- Moving press light changes once per phase while the factory mass stays put.
  local light_t = (k % 2 == 0) and 14 or 26
  bolt(ctx, light_t, near * 18, 55, "amber3")
end

-- ASSEMBLY WORKS ------------------------------------------------------------

local function assembly_works(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  draw_contact(ctx, 47, 17)

  ctx:layer("far")
  extrude(ctx, {
    { -17, -37 }, { 46, -37 }, { 56, -22 },
    { 48, 9 }, { 25, 19 }, { -13, 19 },
    { -25, 4 }, { -24, -22 },
  }, 0, 4, "jade0", "plum0", "deep")
  extrude(ctx, {
    { -23, -22 }, { 43, -22 }, { 49, -9 },
    { 35, 9 }, { -17, 9 }, { -29, -6 },
  }, 4, 4, "straw1", "jade0", "plum0")
  for _, t in ipairs({ -25, 25 }) do
    box(ctx, t, t - near * 7, 0, 4, 4, 9, "plum1", "plum0", "deep")
    flat_disc(ctx, t, t - near * 7, 0, 5, "steel0")
  end

  ctx:layer("body")
  -- Lower canopy is deliberately flatter than HQ's high arch and wraps a
  -- central loom machine.
  local canopy = {
    { -31, -5, 10 }, { -24, -5, 34 }, { -12, -5, 48 },
    { 0, -5, 51 }, { 12, -5, 48 }, { 24, -5, 34 }, { 31, -5, 10 },
  }
  for i = 1, #canopy - 1 do
    line3(ctx, canopy[i], canopy[i + 1], "deep", 6)
    line3(ctx, canopy[i], canopy[i + 1], "jade1", 4)
  end
  -- Central production block and its open lower loom.
  box(ctx, 0, -9, 7, 16, 10, 18, "jade2", "jade1", "jade0")
  hatch(ctx, 0, near * 17, 9, 18, 12, "straw2", "plum0")
  box(ctx, 0, -9, 25, 12, 8, 5, "straw1", "straw2", "plum0")
  -- Cross braces form the woven structural rhythm.
  line3(ctx, pt(-28, -1, 15), pt(-7, -1, 42), "straw2", 2)
  line3(ctx, pt(28, -1, 15), pt(7, -1, 42), "straw2", 2)
  line3(ctx, pt(-22, -12, 16), pt(0, -12, 44), "jade3", 1)
  line3(ctx, pt(22, -12, 16), pt(0, -12, 44), "jade3", 1)

  ctx:layer("upper")
  -- Two side spools and a suspended amber seed chamber distinguish the works.
  screen_disc(ctx, -21, near * 9, 29, 8, "plum0")
  screen_disc(ctx, -21, near * 9, 29, 6, "straw1")
  screen_disc(ctx, -21, near * 9, 29, 3, "straw3")
  screen_disc(ctx, 21, near * 9, 29, 8, "plum0")
  screen_disc(ctx, 21, near * 9, 29, 6, "straw1")
  screen_disc(ctx, 21, near * 9, 29, 3, "straw3")
  screen_disc(ctx, 0, near * 4, 33, 10, "plum0")
  screen_disc(ctx, 0, near * 4, 34, 8, "amber1")
  screen_disc(ctx, 0, near * 4, 35, 5, "amber2")
  pipe(ctx, 0, near * 4, 43, 53, "steel1", "straw2")
  flat_disc(ctx, 0, near * 4, 53, 3, "steel0")

  ctx:layer("front")
  -- Hanging reed curtains are made from three connected cloth strips.
  for i = -1, 1 do
    local t = i * 10
    line3(ctx, pt(t - near * 14, t + near * 14, 16),
      pt(t + i * 2 - near * 14, t + i * 2 + near * 14, 31), "straw1", 3)
  end
  screen_rail(ctx, -28, 27, near * 13, 15, "plum0", 3)
  screen_rail(ctx, -27, 26, near * 13, 18, "jade3", 1)
  -- Pressure hoses bridge the amber core to each spool.
  pipe_elbow(ctx, pt(-8, near * 8, 30), pt(-14, near * 8, 36), pt(-21, near * 9, 36), "plum1", "straw2")
  pipe_elbow(ctx, pt(8, near * 8, 30), pt(14, near * 8, 36), pt(21, near * 9, 36), "plum1", "straw2")

  ctx:layer("details")
  -- Highlight only the upper-left sides of the actual canopy ribs.  Free
  -- standing straw strokes were visually detached from the structure.
  for _, seg in ipairs({ { 2, 3 }, { 3, 4 }, { 4, 5 }, { 5, 6 } }) do
    line3(ctx, canopy[seg[1]], canopy[seg[2]], "straw3", 1)
  end
  bolt(ctx, -21, near * 9, 29, "straw3")
  bolt(ctx, 21, near * 9, 29, "straw3")
  if k == 0 or k == 2 then
    line3(ctx, pt(-3, near * 4, 35), pt(0, near * 4, 38), "amber3", 2)
  else
    line3(ctx, pt(0, near * 4, 38), pt(3, near * 4, 35), "amber3", 2)
  end
end

-- DROP-OFF YARD -------------------------------------------------------------

local function crate(ctx, t, depth, z, w, h, top, side)
  box(ctx, t - depth, t + depth, z, w, 5, h, top, side, "silt0")
  -- A dark front frame separates each cargo face from the quiet yard pad.
  local panel_f, panel_s = t - depth - 5, t + depth + 5
  screen_panel(ctx, panel_f, panel_s, z + 2, w * 1.7, h - 4, side, "deep")
  line3(ctx, pt(t - w * 0.35 - depth - 5, t - w * 0.35 + depth + 5, z + 2),
    pt(t + w * 0.35 - depth - 5, t + w * 0.35 + depth + 5, z + h - 2), "silt3", 1)
  line3(ctx, pt(t + w * 0.35 - depth - 5, t + w * 0.35 + depth + 5, z + 2),
    pt(t - w * 0.35 - depth - 5, t - w * 0.35 + depth + 5, z + h - 2), "silt1", 1)
  line3(ctx, pt(t - w * 0.52 - depth - 5, t - w * 0.52 + depth + 5, z + 2),
    pt(t - w * 0.52 - depth - 5, t - w * 0.52 + depth + 5, z + h - 2), "silt3", 1)
end

local function dropoff(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 42, 15)

  ctx:layer("far")
  extrude(ctx, {
    { -21, -28 }, { 40, -28 }, { 47, -12 },
    { 36, 10 }, { -15, 10 }, { -28, -8 },
  }, 0, 4, "silt2", "silt1", "silt0")
  box(ctx, 27, -15, 4, 14, 10, 10, "steel1", "steel2", "deep")
  pipe(ctx, 41, -5, 4, 36, "steel1", "steel3")

  ctx:layer("body")
  -- Stacked crates are the primary drop-off cue; each has a frame and cross
  -- brace so they read as cargo instead of three random boxes.
  crate(ctx, -18, near * 9, 4, 8, 14, "orange1", "orange2")
  crate(ctx, 0, near * 8, 4, 9, 16, "straw1", "straw2")
  crate(ctx, 18, near * 7, 4, 8, 12, "orange2", "cream2")
  crate(ctx, 4, near * 7, 20, 8, 11, "silt2", "silt3")
  -- A low pallet and a narrow tow bar ground the yard.
  box(ctx, 0, near * 12, 4, 28, 4, 3, "steel0", "steel1", "deep")
  line3(ctx, pt(-29, near * 13, 8), pt(28, near * 13, 8), "straw2", 2)

  ctx:layer("upper")
  -- Yard hoist frame, smaller than the HQ gantry but mechanically distinct.
  pipe(ctx, -28, -16, 4, 40, "steel1", "cream2")
  pipe(ctx, 27, 11, 4, 40, "steel1", "cream2")
  line3(ctx, pt(-28, -16, 40), pt(27, 11, 40), "deep", 4)
  line3(ctx, pt(-28, -16, 40), pt(27, 11, 40), "orange2", 2)
  line3(ctx, pt(8, near * 3, 40), pt(8, near * 3, 27), "ink", 1)
  box(ctx, 8, near * 3, 24, 3, 3, 4, "orange1", "steel2", "deep")

  ctx:layer("front")
  screen_rail(ctx, -28, -8, near * 14, 16, "deep", 3)
  screen_rail(ctx, 8, 28, near * 14, 16, "deep", 3)
  for _, t in ipairs({ -28, -18, -8, 8, 18, 28 }) do
    line3(ctx, pt(t - near * 14, t + near * 14, 16),
      pt(t - near * 14, t + near * 14, 22), "steel2", 2)
  end
  -- One open crate exposes a dark interior and keeps the pile legible.
  hatch(ctx, 4, near * 14, 23, 13, 9, "cream2", "deep")

  ctx:layer("details")
  for _, t in ipairs({ -18, 0, 18, 4 }) do
    bolt(ctx, t, near * 15, 10, "cream3")
  end
  line3(ctx, pt(-22, near * 15, 13), pt(-14, near * 15, 16), "orange3", 1)
  line3(ctx, pt(15, near * 13, 12), pt(22, near * 13, 15), "cream3", 1)
  if k == 2 then
    bolt(ctx, 8, near * 3, 28, "amber3")
  else
    bolt(ctx, 8, near * 3, 31, "amber2")
  end
end

-- CONDENSER / PUMP ----------------------------------------------------------

local function gauge(ctx, t, depth, z, radius, needle_tilt)
  screen_disc(ctx, t - depth, t + depth, z, radius + 2, "deep")
  screen_disc(ctx, t - depth, t + depth, z, radius, "glass1")
  screen_disc(ctx, t - depth, t + depth, z, radius - 2, "glass2")
  line3(ctx, pt(t - depth, t + depth, z),
    pt(t + needle_tilt - depth, t + needle_tilt + depth, z + radius - 2), "amber3", 1)
  screen_disc(ctx, t - depth, t + depth, z, 1, "cream3")
end

local function condenser(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 38, 15)

  ctx:layer("far")
  extrude(ctx, {
    { -18, -26 }, { 35, -26 }, { 42, -11 },
    { 32, 9 }, { -12, 9 }, { -25, -7 },
  }, 0, 4, "water1", "water0", "deep")
  box(ctx, 4, -8, 4, 25, 15, 5, "steel1", "steel0", "deep")
  -- Back intake pipe bends to the tank.
  pipe_elbow(ctx, pt(30, -12, 8), pt(30, -12, 30), pt(14, -12, 30), "steel1", "water3")

  ctx:layer("body")
  -- Main tank is a pressure cylinder with an amber liquid core.
  box(ctx, 8, -5, 6, 15, 11, 33, "steel1", "steel2", "steel0")
  screen_disc(ctx, 8, near * 8, 24, 14, "deep")
  screen_disc(ctx, 8, near * 8, 24, 11, "steel2")
  screen_disc(ctx, 8, near * 8, 24, 8, "amber1")
  screen_disc(ctx, 8, near * 8, 24, 5, "amber2")
  flat_disc(ctx, 8, -5, 39, 10, "steel0")
  flat_disc(ctx, 8, -5, 40, 8, "steel2")
  -- Squat pump and valve on the near side.
  box(ctx, -16, near * 12, 6, 8, 7, 12, "steel1", "steel2", "deep")
  flat_disc(ctx, -16, near * 12, 19, 7, "steel0")
  flat_disc(ctx, -16, near * 12, 20, 5, "steel3")

  ctx:layer("upper")
  pipe(ctx, 8, -5, 40, 57, "steel2", "cream2")
  box(ctx, 8, -5, 55, 5, 5, 5, "steel1", "steel3", "deep")
  flat_disc(ctx, 8, -5, 60, 4, "steel0")
  -- Feed and return pipes are separate, with elbows that expose the route.
  pipe_elbow(ctx, pt(-4, near * 9, 14), pt(-4, near * 9, 38), pt(8, near * 8, 38), "water2", "water3")
  pipe_elbow(ctx, pt(21, near * 8, 14), pt(21, near * 8, 43), pt(8, near * 8, 43), "steel1", "amber2")

  ctx:layer("front")
  gauge(ctx, -16, near * 19, 22, 7, k == 0 and 2 or (k == 1 and 4 or 1))
  gauge(ctx, 8, near * 20, 39, 6, k == 3 and -3 or 2)
  -- Three visible pressure couplers sit on the tank's lower lip.
  for _, t in ipairs({ -1, 8, 17 }) do
    screen_disc(ctx, t, near * 15, 11, 3, "deep")
    screen_disc(ctx, t, near * 15, 11, 1, "steel3")
  end

  ctx:layer("details")
  line3(ctx, pt(2, near * 15, 29), pt(14, near * 15, 29), "glass3", 1)
  line3(ctx, pt(4, near * 8, 19), pt(12, near * 8, 19), "amber3", 1)
  bolt(ctx, -16, near * 19, 22, "cream3")
  bolt(ctx, 8, near * 20, 39, "cream3")
  for _, t in ipairs({ -6, 22 }) do
    bolt(ctx, t, near * 13, 14, "steel3")
  end
end

-- DEFENSE TOWER -------------------------------------------------------------

local function tower(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 40, 16)

  ctx:layer("far")
  extrude(ctx, {
    { -21, -29 }, { 37, -29 }, { 46, -13 },
    { 35, 9 }, { -14, 9 }, { -28, -8 },
  }, 0, 4, "steel0", "deep", "shadow")
  -- Rear brace supports the tall defense machine.
  line3(ctx, pt(34, -7, 4), pt(30, -7, 54), "deep", 4)
  line3(ctx, pt(34, -7, 4), pt(30, -7, 54), "steel1", 2)

  ctx:layer("body")
  box(ctx, 7, -8, 4, 24, 17, 16, "steel1", "steel2", "steel0")
  flat_disc(ctx, 7, -8, 20, 17, "deep")
  flat_disc(ctx, 7, -8, 21, 13, "steel1")
  -- Armored turret block and a broad gun socket.
  box(ctx, 7, -8, 19, 16, 12, 17, "steel2", "steel1", "deep")
  screen_disc(ctx, 7, near * 10, 29, 10, "deep")
  screen_disc(ctx, 7, near * 10, 29, 7, "orange1")
  screen_disc(ctx, 7, near * 10, 29, 3, "steel3")

  ctx:layer("upper")
  -- Fixed tall spine and sensor head establish defensive reach.
  box(ctx, 7, -8, 35, 7, 7, 31, "steel1", "steel2", "steel0")
  box(ctx, 7, -8, 61, 11, 8, 7, "orange1", "orange2", "orange0")
  screen_panel(ctx, 7, near * 1, 62, 13, 5, "glass1", "cream2")
  pipe(ctx, 7, -8, 68, 78, "steel2", "cream2")
  flat_disc(ctx, 7, -8, 79, 4, "steel0")
  -- Sensor dish is vertical and broad; center remains screen-up.
  screen_disc(ctx, -6, near * 4, 67, 10, "steel0")
  screen_disc(ctx, -6, near * 4, 67, 7, "steel2")
  screen_disc(ctx, -6, near * 4, 67, 4, "glass2")

  ctx:layer("front")
  -- Long gun barrel and muzzle ring are the defense-machine role cue.
  line3(ctx, pt(7, near * 12, 31), pt(34, near * 12, 40), "deep", 6)
  line3(ctx, pt(7, near * 12, 31), pt(34, near * 12, 40), "steel3", 3)
  line3(ctx, pt(28, near * 12, 39), pt(38, near * 12, 42), "orange2", 3)
  screen_disc(ctx, 38, near * 12, 42, 5, "deep")
  screen_disc(ctx, 38, near * 12, 42, 2, "orange3")
  -- Two side jacks make the platform heavy and grounded.
  for _, t in ipairs({ -12, 26 }) do
    box(ctx, t, near * 17, 5, 4, 3, 9, "steel1", "steel2", "deep")
    flat_disc(ctx, t, near * 17, 4, 5, "steel0")
    flat_disc(ctx, t, near * 17, 5, 3, "steel3")
  end

  ctx:layer("details")
  -- Hazard banding is a compact structural accent at the lower turret.
  for i = 0, 2 do
    line3(ctx, pt(-3 + i * 4, near * 20, 22),
      pt(0 + i * 4, near * 20, 27), "orange3", 1)
  end
  line3(ctx, pt(3, near * 2, 68), pt(12, near * 2, 68), "glass3", 1)
  bolt(ctx, 7, near * 10, 29, "cream3")
  bolt(ctx, 7, near * 11, 47, "steel3")
  -- A single phase-controlled threat lamp gives the tower life.
  bolt(ctx, 7, -8, (k == 1 or k == 2) and 75 or 72, "amber3")
end

-- GATE ----------------------------------------------------------------------

local function gate(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 46, 16)

  ctx:layer("far")
  extrude(ctx, {
    { -31, -20 }, { 31, -20 }, { 40, -6 },
    { 32, 12 }, { -30, 12 }, { -40, -5 },
  }, 0, 4, "silt1", "silt0", "deep")
  -- Water shows under the raised mechanical span.
  extrude(ctx, {
    { -34, 4 }, { 34, 4 }, { 38, 11 }, { -37, 11 },
  }, 4, 2, "water1", "water0", "deep")

  ctx:layer("body")
  -- Concrete/silt pylons and a central steel sill make the gate legible before
  -- the wheel details are added.
  box(ctx, -25, -25, 4, 8, 8, 38, "silt2", "silt1", "silt0")
  box(ctx, 25, 25, 4, 8, 8, 38, "silt2", "silt1", "silt0")
  box(ctx, 0, 0, 5, 26, 9, 10, "steel1", "steel2", "deep")
  -- Large operating wheel: rim, face, hub, and four sparse spokes.
  screen_disc(ctx, 0, near * 9, 38, 20, "deep")
  screen_disc(ctx, 0, near * 9, 38, 17, "steel1")
  screen_disc(ctx, 0, near * 9, 38, 13, "orange1")
  wheel_spokes(ctx, 0, near * 9, 38, 11, k, "cream2")
  screen_disc(ctx, 0, near * 9, 38, 4, "steel0")
  screen_disc(ctx, 0, near * 9, 38, 2, "orange3")

  ctx:layer("upper")
  -- Gate beam and two concrete cap courses frame the wheel.
  box(ctx, 0, -3, 42, 27, 7, 8, "silt2", "silt1", "silt0")
  box(ctx, -25, -25, 42, 10, 9, 5, "silt3", "silt2", "silt0")
  box(ctx, 25, 25, 42, 10, 9, 5, "silt3", "silt2", "silt0")
  -- Level post is kept outside the wheel so the gauge remains readable.
  pipe(ctx, 28, near * 8, 43, 70, "steel1", "steel3")
  box(ctx, 28, near * 8, 57, 6, 3, 14, "cream1", "steel1", "deep")
  screen_panel(ctx, 28, near * 12, 60, 7, 8, "glass1", "cream2")

  ctx:layer("front")
  -- Level gauge and a compact pointer provide the gate-state read.
  gauge(ctx, 28, near * 16, 64, 4, k == 0 and -2 or (k == 1 and 2 or 0))
  line3(ctx, pt(-23, near * 8, 21), pt(23, near * 8, 21), "steel3", 2)
  for _, t in ipairs({ -19, -9, 0, 9, 19 }) do
    line3(ctx, pt(t, near * 8, 19), pt(t, near * 8, 24), "steel0", 1)
  end
  -- The threshold lip leaves an honest dark channel below the wheel.
  box(ctx, 0, near * 12, 5, 25, 3, 5, "deep", "steel0", "water0")

  ctx:layer("details")
  -- Concrete courses: restrained, offset seams rather than a noisy brick wall.
  for _, t in ipairs({ -28, -22, 22, 28 }) do
    line3(ctx, pt(t, t, 10), pt(t + (t < 0 and 2 or -2), t + (t < 0 and 2 or -2), 33), "silt0", 1)
  end
  line3(ctx, pt(-8, near * 10, 38), pt(8, near * 10, 38), "cream3", 1)
  bolt(ctx, -25, -25, 40, "silt3")
  bolt(ctx, 25, 25, 40, "silt3")
  bolt(ctx, 28, near * 16, 64, "cream3")
  line3(ctx, pt(25, near * 12, 67), pt(31, near * 12, 67), "glass3", 1)
end

-- SALVAGE WRECK PILE --------------------------------------------------------

local function salvage(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 42, 15)

  ctx:layer("far")
  extrude(ctx, {
    { -29, -23 }, { 35, -23 }, { 43, -8 },
    { 33, 10 }, { -17, 10 }, { -38, -5 },
  }, 0, 3, "silt1", "silt0", "deep")
  -- Broken stern and a collapsed beam establish a pile silhouette.
  extrude(ctx, {
    { -30, -6 }, { -20, -16 }, { -6, -10 },
    { 2, -4 }, { -4, 8 }, { -20, 10 },
  }, 3, 13, "orange0", "orange1", "deep")
  line3(ctx, pt(-32, -14, 12), pt(23, 9, 39), "deep", 5)
  line3(ctx, pt(-32, -14, 12), pt(23, 9, 39), "steel1", 3)

  ctx:layer("body")
  -- Three distinct wreck masses: split hull, wheel, and an overturned crate.
  extrude(ctx, {
    { -12, -16 }, { 12, -17 }, { 21, -6 },
    { 17, 6 }, { 2, 9 }, { -16, 3 }, { -19, -8 },
  }, 5, 16, "cream1", "orange1", "orange0")
  box(ctx, 17, near * 8, 5, 9, 7, 13, "steel1", "steel2", "deep")
  flywheel(ctx, -4, near * 17, 18, 10, k, "orange1", "orange2")
  -- Exposed ribbed engine block on the broken side.
  box(ctx, -22, near * 13, 6, 6, 5, 12, "steel0", "steel1", "deep")
  for i = 0, 2 do
    line3(ctx, pt(-25 + i * 4, near * 18, 9),
      pt(-25 + i * 4, near * 18, 17), "steel3", 1)
  end

  ctx:layer("upper")
  -- Jagged fin and a broken cab window explain the wreck's previous function.
  poly3(ctx, {
    pt(0, -14, 18), pt(16, -14, 18), pt(22, -7, 33),
    pt(12, -7, 28), pt(2, -7, 36), pt(-5, -7, 25),
  }, "orange1")
  screen_panel(ctx, 7, near * 3, 22, 13, 8, "glass0", "steel2")
  -- A second fallen plate overlaps the wheel without hiding it.
  box(ctx, 25, near * 15, 5, 7, 3, 9, "orange1", "orange2", "orange0")
  line3(ctx, pt(21, near * 18, 7), pt(30, near * 18, 14), "cream2", 1)

  ctx:layer("front")
  -- Hanging cable and a bent mast complete the wreck narrative.
  line3(ctx, pt(-28, near * 11, 19), pt(-28, near * 11, 34), "ink", 2)
  line3(ctx, pt(-28, near * 11, 34), pt(-17, near * 11, 39), "steel2", 2)
  screen_disc(ctx, -17, near * 11, 39, 3, "orange2")
  line3(ctx, pt(11, near * 4, 23), pt(20, near * 4, 13), "deep", 3)
  box(ctx, 20, near * 4, 10, 4, 3, 5, "steel1", "steel2", "deep")

  ctx:layer("details")
  line3(ctx, pt(-14, near * 16, 12), pt(-1, near * 16, 12), "orange3", 1)
  line3(ctx, pt(8, near * 11, 17), pt(16, near * 11, 19), "cream3", 1)
  bolt(ctx, -22, near * 18, 13, "steel3")
  bolt(ctx, 17, near * 9, 11, "steel3")
  if k == 1 or k == 3 then
    bolt(ctx, 25, near * 18, 10, "amber2")
  end
end

-- WELL ----------------------------------------------------------------------

local function well(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()
  draw_contact(ctx, 32, 13)

  ctx:layer("far")
  extrude(ctx, {
    { -19, -19 }, { 26, -19 }, { 33, -7 },
    { 25, 9 }, { -15, 9 }, { -29, -6 },
  }, 0, 3, "silt1", "silt0", "deep")

  ctx:layer("body")
  -- Stone ring, dark opening, and short pump body.
  flat_disc(ctx, 0, -3, 5, 18, "silt0")
  flat_disc(ctx, 0, -3, 7, 15, "silt2")
  flat_disc(ctx, 0, -3, 9, 10, "water0")
  box(ctx, 0, -3, 8, 9, 8, 13, "silt1", "silt2", "silt0")
  screen_disc(ctx, 0, near * 10, 15, 7, "deep")
  screen_disc(ctx, 0, near * 10, 15, 5, "water2")
  screen_disc(ctx, 0, near * 10, 15, 2, "water3")

  ctx:layer("upper")
  pipe(ctx, 0, -3, 21, 52, "steel1", "steel3")
  flat_disc(ctx, 0, -3, 53, 4, "steel0")
  -- Hand pump and long handle, offset so the circle opening stays visible.
  box(ctx, -8, near * 10, 19, 5, 4, 14, "steel1", "steel2", "deep")
  line3(ctx, pt(-8, near * 12, 45), pt(16, near * 12, 51), "deep", 3)
  line3(ctx, pt(-8, near * 12, 45), pt(16, near * 12, 51), "steel3", 1)
  screen_disc(ctx, 16, near * 12, 51, 3, "orange2")

  ctx:layer("front")
  -- Small level gauge on the pump body.
  gauge(ctx, -8, near * 15, 29, 4, k == 0 and -1 or 2)
  line3(ctx, pt(-14, near * 12, 12), pt(13, near * 12, 12), "silt3", 1)

  ctx:layer("details")
  for _, t in ipairs({ -11, 0, 11 }) do
    bolt(ctx, t, near * 12, 9, "silt3")
  end
  line3(ctx, pt(-3, near * 10, 15), pt(4, near * 10, 15), "glass3", 1)
  bolt(ctx, -8, near * 15, 29, "cream3")
end

M.union_hq = union_hq
M.assembly_hq = assembly_hq
M.union_works = union_works
M.assembly_works = assembly_works
M.dropoff = dropoff
M.condenser = condenser
M.tower = tower
M.gate = gate
M.salvage = salvage
M.well = well

return M
