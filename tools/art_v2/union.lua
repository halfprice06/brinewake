-- BRINEWAKE Union art v2.
--
-- This module is deliberately geometry-authored.  The generated Union board was
-- used as a construction reference only; no reference pixels are sampled or
-- imported here.  The runner owns the raster target, palette values, and layer
-- compositing.  Keep this file limited to the contract API so that it can be
-- rendered into any 64x64 cell without hidden state.

local M = {}

local function phase4(phase)
  if phase == nil then
    return 0
  end
  return phase % 4
end

local function point(ctx, f, s, z)
  -- Context primitives consume numeric {f,s,z} triples (the same shape used by
  -- line3/disc).  Keep the helper centralized so every face follows that API.
  return { f, s, z }
end

local function pxy(ctx, f, s, z)
  return ctx:p(f, s, z)
end

local function line3(ctx, a, b, color, width)
  ctx:line3(a, b, color, width)
end

local function poly3(ctx, vertices, color)
  ctx:poly3(vertices, color)
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

local SIDE_AXIS = { 0, 1, 0 }
local UP_AXIS = { 0, 0, 1 }
local FWD_AXIS = { 1, 0, 0 }

local function horizontal_disc(ctx, f, s, z, radius, color)
  disc(ctx, point(ctx, f, s, z), FWD_AXIS, SIDE_AXIS, radius, color)
end

local function side_disc(ctx, f, s, z, radius, color)
  disc(ctx, point(ctx, f, s, z), SIDE_AXIS, UP_AXIS, radius, color)
end

local function wheel_disc(ctx, f, s, z, radius, color)
  -- A crawler flywheel's axle runs through local side; its visible face is
  -- therefore the local forward/up plane.  Keep this separate from side_disc,
  -- which is still correct for mast receivers and other side-facing hardware.
  disc(ctx, point(ctx, f, s, z), FWD_AXIS, UP_AXIS, radius, color)
end

local function bolt(ctx, f, s, z, color)
  local x, y = pxy(ctx, f, s, z)
  ctx:pixel(x, y, color)
  ctx:pixel(x + 1, y, color)
end

local function wheel_spokes(ctx, f, s, z, radius, phase, accent)
  -- Four authored orientations keep the wheel's mass stable while the spoke
  -- cluster rotates.  The wheel itself never translates with the body.
  local spoke = {
    { { f, s, z - radius + 1 }, { f, s, z + radius - 1 } },
    { { f - radius + 1, s, z }, { f + radius - 1, s, z } },
    { { f - 3, s, z - 3 }, { f + 3, s, z + 3 } },
    { { f - 3, s, z + 3 }, { f + 3, s, z - 3 } },
  }
  local pair = spoke[(phase % 4) + 1]
  line3(ctx, pair[1], pair[2], accent, 1)
end

local function wheel_teeth(ctx, f, s, z, radius)
  -- Small rim bites are enough to sell a toothed flywheel at native size.  The
  -- teeth share the wheel's forward/z plane so they remain attached when the
  -- local forward axis turns to another screen facing.
  local r = radius + 1
  line3(ctx, { f + r, s, z - 1 }, { f + r + 2, s, z - 1 }, "steel0", 1)
  line3(ctx, { f + r, s, z + 1 }, { f + r + 2, s, z + 1 }, "steel0", 1)
  line3(ctx, { f - r, s, z - 1 }, { f - r - 2, s, z - 1 }, "steel0", 1)
  line3(ctx, { f - r, s, z + 1 }, { f - r - 2, s, z + 1 }, "steel0", 1)
  line3(ctx, { f, s, z + r }, { f, s, z + r + 2 }, "steel0", 1)
  line3(ctx, { f, s, z - r }, { f, s, z - r - 2 }, "steel0", 1)
end

local function gear_wheel(ctx, f, s, z, radius, phase, near, bright)
  -- Deep rim, warm hub, and one clean spoke read as a toothed flywheel at 1x.
  wheel_disc(ctx, f, s, z, radius + 1, "deep")
  wheel_disc(ctx, f, s, z, radius, "steel1")
  wheel_disc(ctx, f, s, z, radius - 2, "orange1")
  wheel_teeth(ctx, f, s, z, radius)
  wheel_spokes(ctx, f, s, z, radius - 2, phase, bright)
  wheel_disc(ctx, f, s, z, 2, "steel3")
  bolt(ctx, f, s + near * 0.15, z, "ink")
end

local function tread_run(ctx, f0, f1, s, z, phase, light)
  -- A broken belt line and four short shoes give the crawler a clear direction
  -- without drawing a noisy all-over checker pattern.
  line3(ctx, { f0, s, z }, { f1, s, z }, "deep", 3)
  line3(ctx, { f0 + 2, s, z + 2 }, { f1 - 2, s, z + 2 }, light, 1)
  local shoe_f = { f0 + 3, f0 + 8, f0 + 13, f0 + 18 }
  local offset = (phase % 4) - 1
  for i = 1, #shoe_f do
    local f = shoe_f[i] + offset
    if f < f1 - 1 then
      line3(ctx, { f, s, z - 1 }, { f + 2, s, z + 2 }, "steel0", 1)
    end
  end
end

local function rear_rail(ctx, f0, f1, s, z, color)
  line3(ctx, { f0, s, z }, { f1, s, z }, color, 2)
  line3(ctx, { f0, s, z + 3 }, { f1, s, z + 3 }, "steel3", 1)
end

local function piston(ctx, a, b, outer, inner)
  line3(ctx, a, b, "deep", 4)
  line3(ctx, a, b, outer, 3)
  line3(ctx, a, b, inner, 1)
end

local function cab_glass(ctx, f, s, z, half_f, half_s, height, near)
  -- Front glazing is a solid cyan cluster with one offset highlight.  Side
  -- glazing uses the near side so the cab does not turn into a flat rectangle.
  poly3(ctx, {
    point(ctx, f + half_f, -half_s + 1, z + 2),
    point(ctx, f + half_f, half_s - 1, z + 2),
    point(ctx, f + half_f, half_s - 1, z + height - 2),
    point(ctx, f + half_f, -half_s + 1, z + height - 2),
  }, "glass1")
  poly3(ctx, {
    point(ctx, f - half_f + 1, near * half_s, z + 2),
    point(ctx, f + half_f - 1, near * half_s, z + 2),
    point(ctx, f + half_f - 1, near * half_s, z + height - 2),
    point(ctx, f - half_f + 1, near * half_s, z + height - 2),
  }, "glass2")
  line3(ctx, { f + half_f, -half_s + 2, z + height - 3 },
    { f + half_f, half_s - 2, z + height - 3 }, "glass3", 1)
  line3(ctx, { f - half_f + 2, near * half_s, z + height - 3 },
    { f + half_f - 2, near * half_s, z + height - 3 }, "glass3", 1)
end

local function hook_unit(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  ctx:layer("shadows")
  ctx:shadow(17, 9)

  -- Rear crawler and far ballast.  The octagonal footprints and exposed top
  -- lips are the main form pass; they remain fixed through the walk.
  ctx:layer("far")
  extrude(ctx, {
    { -15, -9 }, { 9, -9 }, { 13, -6 },
    { 13, 6 }, { 9, 9 }, { -15, 9 },
    { -16, 5 }, { -16, -5 },
  }, 0, 7, "steel1", "steel2", "deep")
  tread_run(ctx, -14, 12, -near * 10, 3, k, "steel2")
  rear_rail(ctx, -12, 9, -near * 8, 8, "steel0")
  box(ctx, -10, -near * 6, 5, 3, 3, 8, "orange1", "orange2", "deep")

  -- Main chassis has a sloped nose and a service block on the near side.
  ctx:layer("body")
  extrude(ctx, {
    { -13, -8 }, { 7, -8 }, { 11, -5 },
    { 11, 5 }, { 7, 8 }, { -13, 8 },
    { -15, 4 }, { -15, -4 },
  }, 6, 7, "cream1", "cream2", "deep")
  box(ctx, -9, near * 7, 7, 3, 2, 6, "orange1", "orange2", "deep")
  box(ctx, 6, near * 6, 7, 3, 2, 7, "cream2", "cream1", "deep")
  tread_run(ctx, -14, 12, near * 10, 3, k, "steel3")

  -- Cab: broad cream shell, cyan glazing, and a dark sill that separates it
  -- from the crawler.  The split planes keep the worker readable at 1x.
  ctx:layer("upper")
  box(ctx, -2, 0, 12, 6, 6, 10, "cream2", "cream1", "deep")
  cab_glass(ctx, -2, 0, 12, 6, 6, 10, near)
  box(ctx, -2, near * 6, 11, 5, 1, 2, "steel0", "steel1", "deep")
  box(ctx, -2, -near * 6, 11, 5, 1, 2, "steel0", "steel1", "deep")
  box(ctx, -7, 0, 17, 3, 5, 3, "orange1", "orange2", "deep")
  horizontal_disc(ctx, -7, near * 2, 21, 2, "steel3")

  -- Crane pedestal, pivot, and boom.  The boom is a thick structural member,
  -- not a one-pixel diagonal, and the piston remains visible on the near side.
  ctx:layer("front")
  box(ctx, -10, 0, 11, 4, 4, 8, "steel2", "steel1", "deep")
  side_disc(ctx, -9, near * 4, 22, 4, "orange2")
  side_disc(ctx, -9, near * 4, 22, 2, "steel3")
  line3(ctx, { -9, -3, 25 }, { 10, -3, 33 }, "deep", 5)
  line3(ctx, { -9, -3, 25 }, { 10, -3, 33 }, "orange2", 3)
  line3(ctx, { -6, 3, 27 }, { 10, 3, 33 }, "cream1", 3)
  piston(ctx, { -8, near * 5, 13 }, { 0, near * 5, 26 }, "steel2", "cream2")
  side_disc(ctx, 0, near * 5, 26, 2, "orange2")
  side_disc(ctx, 10, near * 3, 33, 3, "steel1")

  -- Hoist cable, block, and a hooked tip extend past the cab as the role cue.
  ctx:layer("details")
  line3(ctx, { 10, near * 3, 33 }, { 10, near * 3, 21 }, "ink", 1)
  box(ctx, 10, near * 3, 19, 2, 2, 4, "orange1", "steel2", "deep")
  line3(ctx, { 10, near * 3, 19 }, { 13, near * 3, 16 }, "deep", 2)
  line3(ctx, { 13, near * 3, 16 }, { 15, near * 3, 18 }, "orange2", 2)
  line3(ctx, { 15, near * 3, 18 }, { 14, near * 3, 20 }, "deep", 1)
  -- A second, warm inner stroke makes the suspended J profile survive at 1x.
  line3(ctx, { 13, near * 3, 16 }, { 15, near * 3, 17 }, "orange1", 1)
  line3(ctx, { 15, near * 3, 17 }, { 16, near * 3, 19 }, "orange2", 1)
  -- Wear is structural: two dark seams and a restrained bright catchlight.
  line3(ctx, { -12, near * 8, 9 }, { 5, near * 8, 9 }, "orange0", 1)
  line3(ctx, { -8, near * 8, 12 }, { -3, near * 8, 12 }, "cream3", 1)
  bolt(ctx, -10, near * 8, 10, "cream3")
  bolt(ctx, 5, near * 7, 12, "steel3")
  bolt(ctx, -9, near * 4, 22, "cream3")
end

local function riveter_unit(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  ctx:layer("shadows")
  ctx:shadow(16, 8)

  ctx:layer("far")
  extrude(ctx, {
    { -13, -9 }, { 9, -9 }, { 13, -5 },
    { 13, 5 }, { 9, 9 }, { -13, 9 },
    { -15, 5 }, { -15, -5 },
  }, 0, 7, "steel1", "steel2", "deep")
  tread_run(ctx, -13, 12, -near * 10, 3, k, "steel2")
  rear_rail(ctx, -11, 10, -near * 8, 8, "orange0")
  gear_wheel(ctx, -5, -near * 9, 8, 6, (k + 1) % 4, -near, "steel2")

  ctx:layer("body")
  extrude(ctx, {
    { -12, -8 }, { 8, -8 }, { 11, -5 },
    { 11, 5 }, { 8, 8 }, { -12, 8 },
    { -14, 4 }, { -14, -4 },
  }, 6, 9, "cream1", "cream2", "deep")
  tread_run(ctx, -13, 12, near * 10, 3, k, "steel3")
  gear_wheel(ctx, -5, near * 9, 8, 7, k, near, "orange3")
  gear_wheel(ctx, 4, near * 9, 8, 6, (k + 2) % 4, near, "steel3")
  box(ctx, -7, near * 7, 9, 3, 2, 6, "orange1", "orange2", "deep")
  box(ctx, 4, -near * 6, 8, 3, 2, 5, "steel2", "steel1", "deep")

  -- Hammerhead is high and square, with a clear tapered front block.
  ctx:layer("upper")
  box(ctx, 0, 0, 15, 7, 7, 8, "cream2", "cream1", "deep")
  box(ctx, 6, 0, 15, 3, 6, 7, "orange1", "orange2", "deep")
  box(ctx, 10, 0, 16, 4, 3, 4, "steel2", "steel3", "deep")
  box(ctx, 13, 0, 15, 2, 2, 6, "steel0", "steel2", "deep")
  side_disc(ctx, 0, near * 7, 18, 3, "steel1")
  side_disc(ctx, 0, near * 7, 18, 1, "orange3")
  box(ctx, -2, 0, 23, 3, 4, 3, "orange2", "orange1", "deep")
  horizontal_disc(ctx, -2, 0, 27, 2, "steel3")

  ctx:layer("front")
  -- Short nozzle and riveter face; the front plane is intentionally brighter
  -- than the body so line-fighter identity survives a crowded battle.
  poly3(ctx, {
    point(ctx, 12, -3, 15), point(ctx, 12, 3, 15),
    point(ctx, 12, 3, 20), point(ctx, 12, -3, 20),
  }, "steel3")
  box(ctx, 14, 0, 16, 2, 3, 3, "steel3", "steel2", "deep")
  side_disc(ctx, 13, near * 3, 17, 2, "orange2")
  side_disc(ctx, 13, near * 3, 17, 1, "steel0")
  piston(ctx, { 6, near * 6, 12 }, { 10, near * 5, 16 }, "steel3", "cream2")

  ctx:layer("details")
  -- Wheel teeth and fasteners are sparse, repeated structural motifs.
  for i = 0, 3 do
    local f = -9 + i * 5 + ((k + i) % 2)
    line3(ctx, { f, near * 10, 2 }, { f + 2, near * 10, 4 }, "steel0", 1)
  end
  bolt(ctx, -5, near * 10, 8, "cream3")
  bolt(ctx, 4, near * 10, 8, "steel3")
  bolt(ctx, 0, near * 7, 18, "cream3")
  line3(ctx, { -5, near * 8, 12 }, { 5, near * 8, 12 }, "orange0", 1)
  line3(ctx, { -3, near * 8, 19 }, { 5, near * 8, 19 }, "cream3", 1)
end

local function shield_plate(ctx, f, s, near)
  -- A chamfered shield slab.  Two slabs leave a narrow honest center seam.
  poly3(ctx, {
    point(ctx, f, s - 5, 3), point(ctx, f, s + 5, 3),
    point(ctx, f, s + 5, 20), point(ctx, f, s + 3, 23),
    point(ctx, f, s - 4, 23), point(ctx, f, s - 5, 20),
  }, "cream1")
  line3(ctx, { f, s - 4, 7 }, { f, s + 4, 7 }, "orange1", 2)
  line3(ctx, { f, s - 4, 21 }, { f, s + 3, 21 }, "cream3", 1)
  bolt(ctx, f, s - 3, 11, "steel3")
  bolt(ctx, f, s + 3, 17, "steel0")
end

local function jack_leg(ctx, root_f, root_s, foot_f, foot_s, foot_z, near, front)
  local dark = front and "deep" or "shadow"
  line3(ctx, { root_f, root_s, 9 }, { foot_f, foot_s, foot_z + 2 }, dark, 4)
  line3(ctx, { root_f, root_s, 9 }, { foot_f, foot_s, foot_z + 2 }, "steel2", 2)
  horizontal_disc(ctx, foot_f, foot_s, foot_z, 3, "steel0")
  horizontal_disc(ctx, foot_f, foot_s, foot_z + 1, 2, "steel3")
end

local function bulwark_unit(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  ctx:layer("shadows")
  ctx:shadow(18, 10)

  ctx:layer("far")
  extrude(ctx, {
    { -14, -10 }, { 10, -10 }, { 14, -6 },
    { 14, 6 }, { 10, 10 }, { -14, 10 },
    { -16, 5 }, { -16, -5 },
  }, 0, 8, "steel1", "steel2", "deep")
  tread_run(ctx, -15, 13, -near * 11, 3, k, "steel2")
  rear_rail(ctx, -12, 10, -near * 9, 9, "steel0")
  jack_leg(ctx, -8, -near * 8, -10, -near * 10, 0, near, false)
  jack_leg(ctx, 8, -near * 8, 10, -near * 10, 0, near, false)

  ctx:layer("body")
  extrude(ctx, {
    { -13, -9 }, { 8, -9 }, { 11, -5 },
    { 11, 5 }, { 8, 9 }, { -13, 9 },
    { -15, 4 }, { -15, -4 },
  }, 7, 10, "cream1", "cream2", "deep")
  tread_run(ctx, -15, 13, near * 11, 3, k, "steel3")
  box(ctx, -5, near * 7, 11, 5, 2, 7, "orange1", "orange2", "deep")
  box(ctx, 4, -near * 6, 11, 4, 3, 9, "cream2", "cream1", "deep")
  side_disc(ctx, -6, near * 10, 8, 5, "steel1")
  side_disc(ctx, -6, near * 10, 8, 2, "orange2")
  side_disc(ctx, 5, near * 10, 8, 4, "steel0")
  side_disc(ctx, 5, near * 10, 8, 2, "steel3")

  ctx:layer("upper")
  -- Brace feet are visible against the body, then the barrier rises as two
  -- angled panels.  The center gap is a functional split, not a decorative
  -- stripe pasted over one flat rectangle.
  box(ctx, 4, -near * 5, 16, 4, 2, 6, "steel2", "steel1", "deep")
  box(ctx, 4, near * 5, 16, 4, 2, 6, "steel2", "steel1", "deep")
  shield_plate(ctx, 10, -near * 5, near)
  shield_plate(ctx, 10, near * 5, near)
  piston(ctx, { 4, near * 7, 13 }, { 9, near * 7, 22 }, "steel2", "cream2")
  side_disc(ctx, 9, near * 8, 21, 3, "orange2")
  side_disc(ctx, 9, near * 8, 21, 1, "steel3")

  ctx:layer("front")
  -- Front jacks take the stance cue.  One foot rises through the step while
  -- the other three keep ground contact, so the unit does not bob as a blob.
  local front_lift = (k == 1) and 1 or 0
  local near_lift = (k == 2) and 1 or 0
  jack_leg(ctx, 9, -near * 8, 12, -near * 10, front_lift, near, true)
  jack_leg(ctx, 9, near * 8, 12, near * 10, near_lift, near, true)
  box(ctx, 12, -near * 5, 5, 2, 3, 15, "cream2", "cream1", "deep")
  box(ctx, 12, near * 5, 5, 2, 3, 15, "cream2", "cream1", "deep")
  line3(ctx, { 14, -near * 5, 8 }, { 14, -near * 5, 19 }, "orange1", 2)
  line3(ctx, { 14, near * 5, 8 }, { 14, near * 5, 19 }, "orange1", 2)

  ctx:layer("details")
  -- Hazard stripes and large corner bolts are sparse enough to preserve the
  -- cream/orange/steel value grouping at native size.
  line3(ctx, { 14, -near * 5, 11 }, { 14, -near * 5, 14 }, "orange0", 1)
  line3(ctx, { 14, near * 5, 7 }, { 14, near * 5, 10 }, "orange0", 1)
  bolt(ctx, 14, -near * 7, 20, "steel3")
  bolt(ctx, 14, near * 7, 20, "steel0")
  line3(ctx, { -10, near * 9, 12 }, { 1, near * 9, 12 }, "orange0", 1)
  line3(ctx, { -8, near * 9, 15 }, { -3, near * 9, 15 }, "cream3", 1)
end

local function tripod_leg(ctx, hub, foot, lifted, near, front)
  local foot_z = lifted and 2 or 0
  local dark = front and "deep" or "shadow"
  line3(ctx, hub, { foot[1], foot[2], foot_z + 2 }, dark, 4)
  line3(ctx, hub, { foot[1], foot[2], foot_z + 2 }, "steel2", 2)
  line3(ctx, { foot[1], foot[2], foot_z + 2 },
    { foot[1] + (foot[1] > 0 and -1 or 1), foot[2], foot_z + 4 }, "cream2", 1)
  horizontal_disc(ctx, foot[1], foot[2], foot_z, 3, "steel0")
  horizontal_disc(ctx, foot[1], foot[2], foot_z + 1, 2, "steel3")
end

local function sounder_unit(ctx, phase)
  local k = phase4(phase)
  local near = ctx:near_side()

  ctx:layer("shadows")
  ctx:shadow(13, 7)

  local hub = { 0, 0, 11 }
  local feet = {
    { 7, near * 6 },
    { -7, near * 6 },
    { -6, -near * 5 },
  }

  ctx:layer("far")
  tripod_leg(ctx, hub, feet[3], k == 3, near, false)
  box(ctx, -2, -near * 5, 4, 4, 2, 7, "steel1", "steel2", "deep")
  line3(ctx, { -3, -near * 5, 8 }, { 3, -near * 5, 8 }, "steel0", 2)

  ctx:layer("body")
  extrude(ctx, {
    { -9, -6 }, { 7, -6 }, { 9, -3 },
    { 9, 3 }, { 7, 6 }, { -9, 6 },
    { -10, 3 }, { -10, -3 },
  }, 4, 7, "cream1", "cream2", "deep")
  box(ctx, 3, near * 4, 6, 3, 2, 6, "orange1", "orange2", "deep")
  poly3(ctx, {
    point(ctx, 7, -3, 7), point(ctx, 7, 3, 7),
    point(ctx, 7, 3, 11), point(ctx, 7, -3, 11),
  }, "glass1")
  line3(ctx, { 7, -2, 10 }, { 7, 2, 10 }, "glass3", 1)

  ctx:layer("upper")
  tripod_leg(ctx, hub, feet[1], k == 1, near, true)
  tripod_leg(ctx, hub, feet[2], k == 2, near, true)
  -- Asymmetric mast: the offset receiver and counter-rod keep the scout from
  -- reading as a generic antenna tower.
  box(ctx, -3, near * 2, 10, 2, 2, 22, "steel2", "cream2", "deep")
  line3(ctx, { -3, near * 4, 12 }, { -3, near * 4, 35 }, "deep", 3)
  line3(ctx, { -3, near * 4, 12 }, { -3, near * 4, 35 }, "steel3", 1)
  line3(ctx, { -2, near * 1, 28 }, { 4, near * 1, 33 }, "orange1", 2)
  side_disc(ctx, -3, near * 4, 31, 5, "steel1")
  side_disc(ctx, -3, near * 4, 31, 3, "steel3")
  side_disc(ctx, -3, near * 4, 31, 1, "glass2")
  box(ctx, -3, near * 4, 35, 1, 1, 3, "orange2", "orange3", "deep")

  ctx:layer("front")
  -- Two antenna whiskers and a survey receiver dish.  Every tall part is
  -- defined in z, so facing changes never rotate the mast onto its side.
  line3(ctx, { -3, near * 2, 32 }, { -3, near * 2, 40 }, "deep", 2)
  line3(ctx, { -3, near * 2, 32 }, { -3, near * 2, 40 }, "steel3", 1)
  line3(ctx, { -1, near * 5, 31 }, { 4, near * 5, 38 }, "deep", 2)
  line3(ctx, { -1, near * 5, 31 }, { 4, near * 5, 38 }, "orange2", 1)
  side_disc(ctx, 4, near * 5, 38, 2, "orange3")
  side_disc(ctx, -3, near * 5, 31, 6, "steel0")
  side_disc(ctx, -3, near * 5, 31, 5, "steel2")
  side_disc(ctx, -3, near * 5, 31, 3, "steel3")
  line3(ctx, { -3, near * 5, 26 }, { -3, near * 5, 36 }, "cream2", 1)

  ctx:layer("details")
  bolt(ctx, 3, near * 6, 8, "cream3")
  bolt(ctx, -6, near * 6, 7, "steel3")
  line3(ctx, { -7, near * 6, 6 }, { 2, near * 6, 6 }, "orange0", 1)
  line3(ctx, { -2, near * 3, 14 }, { -2, near * 3, 24 }, "cream3", 1)
  -- A restrained receiver glint is the only animated detail on the mast.
  if k == 1 or k == 3 then
    bolt(ctx, -3, near * 5, 31, "glass3")
  end
end

M.hook = hook_unit
M.riveter = riveter_unit
M.bulwark = bulwark_unit
M.sounder = sounder_unit

return M
