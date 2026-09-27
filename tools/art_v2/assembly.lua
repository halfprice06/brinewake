-- BRINEWAKE Silt Assembly units, art v2.
--
-- This module is deliberately authored in local integer geometry.  It is
-- consumed by the art_v2 runner, which gives each function a fresh 64x64
-- cell and rotates the ground axes through ctx:p().  No generated concept
-- pixels are imported here.  The reference board is used for construction:
-- thick pale-jade shell panels, warm woven ribs, amber pressure hardware,
-- and dark service joints.

local M = {}

-- All names below are roles from tools/art_v2/core.lua.  Keeping the ramps
-- explicit makes the material hierarchy readable at native resolution.
local C = {
  ink='ink', deep='deep', shadow='shadow',
  steel0='steel0', steel1='steel1', steel2='steel2', steel3='steel3',
  cream0='cream0', cream1='cream1', cream2='cream2', cream3='cream3',
  jade0='jade0', jade1='jade1', jade2='jade2', jade3='jade3',
  straw0='straw0', straw1='straw1', straw2='straw2', straw3='straw3',
  amber0='amber0', amber1='amber1', amber2='amber2', amber3='amber3',
  glass0='glass0', glass1='glass1', glass2='glass2', glass3='glass3',
  plum0='plum0', plum1='plum1', plum2='plum2',
  silt0='silt0', silt1='silt1', silt2='silt2', silt3='silt3'
}

local function V(f, s, z)
  return {f, s, z}
end

local function I(v)
  return math.floor(v + 0.5)
end

local function P(ctx, points, color)
  ctx:poly3(points, color)
end

local function L(ctx, a, b, color, width)
  ctx:line3(a, b, color, width or 1)
end

local function B(ctx, f, s, z, hf, hs, h, top, light, dark)
  ctx:box(f, s, z, hf, hs, h, top, light, dark)
end

local function D(ctx, center, axis_a, axis_b, radius, color)
  ctx:disc(center, axis_a, axis_b, radius, color)
end

local function layer(ctx, name)
  ctx:layer(name)
end

local function mark(ctx, f, s, z, color)
  local x, y = ctx:p(f, s, z)
  ctx:pixel(x, y, color)
end

local function phase_index(phase)
  local n = tonumber(phase or 0) or 0
  n = n % 4
  return n + 1
end

local function near_side(ctx)
  local n = ctx:near_side()
  if n == 0 then return 1 end
  return n > 0 and 1 or -1
end

local function socket(ctx, f, s, z, r)
  r = r or 2
  D(ctx, V(f, s, z), V(0, 1, 0), V(0, 0, 1), r + 1, C.ink)
  D(ctx, V(f, s, z), V(0, 1, 0), V(0, 0, 1), r, C.plum0)
  if r > 1 then
    D(ctx, V(f, s, z), V(0, 1, 0), V(0, 0, 1), r - 1, C.plum2)
  end
  L(ctx, V(f, s - r + 1, z), V(f, s + r - 1, z), C.steel0, 1)
end

local function horizontal_cap(ctx, f, s, z, r, color)
  D(ctx, V(f, s, z), V(1, 0, 0), V(0, 1, 0), r, color)
end

local function foot(ctx, f, s, z, hf, hs, top, light, dark)
  -- A low cuboid keeps all contacts on z=0 while making the feet read as
  -- built plates instead of one-pixel stilts.
  B(ctx, f, s, z, hf, hs, 2, top, light, dark)
  L(ctx, V(f - hf + 1, s - hs, z + 2), V(f + hf - 1, s - hs, z + 2), light, 1)
end

local function mech_leg(ctx, hip, knee, ankle, toe, metal, mid, bright, foot_hf, foot_hs)
  L(ctx, hip, knee, C.ink, 4)
  L(ctx, hip, knee, metal, 2)
  L(ctx, knee, ankle, C.ink, 4)
  L(ctx, knee, ankle, mid, 2)
  socket(ctx, knee[1], knee[2], knee[3], 2)
  L(ctx, ankle, toe, C.ink, 3)
  L(ctx, ankle, toe, bright, 1)
  socket(ctx, ankle[1], ankle[2], ankle[3], 2)
  foot(ctx, toe[1], toe[2], toe[3], foot_hf or 3, foot_hs or 2, mid, bright, C.plum0)
end

-- A four-step contact pattern.  The hip remains stationary; only the
-- articulated chain and the planted toe move, so there is no whole-body bob.
local GAIT = {
  { 0,  0, 0,  0,  0, 0}, -- contact
  {-2, -1, 3, -3, -1, 2}, -- lift/back
  { 2,  1, 0,  2,  1, 0}, -- far contact
  { 1,  0, 3,  1,  0, 2}  -- recover
}

local function leg_pose(hip_f, hip_s, leg_no, phase, stride)
  local offsets = stride or {0, 2, 1, 3}
  local k = ((phase + (offsets[leg_no] or 0)) % 4) + 1
  local g = GAIT[k]
  local knee = V(hip_f + g[1], hip_s + g[2], 8 + g[3])
  local ankle = V(hip_f + math.floor(g[1] * 0.5), hip_s + math.floor(g[2] * 0.5), 3 + g[6])
  local toe = V(hip_f + g[4] + (stride and stride[5] or 0), hip_s + g[5], g[6])
  return knee, ankle, toe
end

local function pressure_tank(ctx, f, s, z, h, r)
  r = r or 4
  -- Blocked silhouette first, then three broad value planes and two bands.
  B(ctx, f, s, z, r + 1, r, h, C.ink, C.ink, C.ink)
  B(ctx, f, s, z + 1, r, r - 1, h - 2, C.amber1, C.amber2, C.amber0)
  B(ctx, f, s, z + 4, r + 1, r, 1, C.steel1, C.steel2, C.steel0)
  B(ctx, f, s, z + h - 4, r + 1, r, 1, C.steel1, C.steel2, C.steel0)
  horizontal_cap(ctx, f, s, z + h + 1, r + 1, C.ink)
  horizontal_cap(ctx, f, s, z + h, r, C.amber2)
  L(ctx, V(f - r + 1, s - r, z + 6), V(f - r + 1, s - r, z + h - 5), C.amber3, 1)
  L(ctx, V(f + r - 1, s + r - 1, z + 6), V(f + r - 1, s + r - 1, z + h - 5), C.amber0, 1)
end

local function spool(ctx, f, s, z, length, r)
  r = r or 3
  local half = I(length * 0.5)
  local quarter = I(length * 0.25)
  local fifth = I(length * 0.2)
  B(ctx, f, s, z - r, half + 1, r + 1, r * 2 + 1, C.ink, C.ink, C.ink)
  B(ctx, f, s, z - r + 1, half, r, r * 2 - 1, C.straw1, C.straw2, C.straw0)
  B(ctx, f - quarter, s, z - r - 1, 1, r + 1, r * 2 + 3, C.steel1, C.steel2, C.steel0)
  B(ctx, f + quarter, s, z - r - 1, 1, r + 1, r * 2 + 3, C.steel1, C.steel2, C.steel0)
  D(ctx, V(f - half - 1, s, z), V(0, 1, 0), V(0, 0, 1), r + 1, C.ink)
  D(ctx, V(f - half, s, z), V(0, 1, 0), V(0, 0, 1), r, C.steel2)
  D(ctx, V(f + half + 1, s, z), V(0, 1, 0), V(0, 0, 1), r + 1, C.ink)
  D(ctx, V(f + half, s, z), V(0, 1, 0), V(0, 0, 1), r, C.steel1)
  L(ctx, V(f - fifth, s - r, z + 1), V(f + fifth, s - r, z + 1), C.straw3, 1)
end

local function hose(ctx, points)
  for i = 1, #points - 1 do
    L(ctx, points[i], points[i + 1], C.ink, 3)
    L(ctx, points[i], points[i + 1], C.plum1, 1)
  end
end

local function jade_edge(ctx, a, b)
  L(ctx, a, b, C.jade3, 1)
end

local function rib(ctx, a, b, bright)
  L(ctx, a, b, bright and C.straw2 or C.straw1, 1)
end

local function vent(ctx, f, s, z, w, h)
  B(ctx, f, s, z, w, 1, h, C.ink, C.ink, C.ink)
  for i = -w + 1, w - 1, 3 do
    L(ctx, V(f + i, s - 1, z + 2), V(f + i, s - 1, z + h - 2), C.steel2, 1)
  end
end

-- WICK ---------------------------------------------------------------------

local function draw_wick(ctx, phase)
  phase = (phase_index(phase) - 1)
  local ns = near_side(ctx)

  layer(ctx, 'shadows')
  ctx:shadow(16, 5)

  local hips = {
    { -8, -7, 16 },
    {  7, -7, 16 },
    {  3,  8, 15 }
  }

  -- Rear three-legged stance, with visible sockets at every hinge.
  layer(ctx, 'far')
  for i = 1, 3 do
    local hip = V(hips[i][1], hips[i][2], hips[i][3])
    local knee, ankle, toe = leg_pose(hips[i][1], hips[i][2], i, phase, {0, 2, 1, 3, i == 1 and -1 or 0})
    mech_leg(ctx, hip, knee, ankle, toe, C.plum0, C.steel1, C.steel2, 3, 2)
  end

  -- Open carrier: the deep basket is an intentional negative space inside a
  -- thick crescent of shell rather than a flat triangle/box.
  layer(ctx, 'body')
  local hull = {
    {-14, -9}, {-11, -12}, {-2, -14}, {9, -11}, {14, -5},
    {13,  7}, {7, 12}, {-5, 13}, {-14, 8}
  }
  ctx:extrude(hull, 11, 9, C.jade2, C.jade1, C.jade0)
  P(ctx, {
    V(-11, -8, 16), V(-7, -10, 16), V(5, -9, 16), V(11, -4, 16),
    V(10,  6, 16), V(4,  9, 16), V(-6, 9, 16), V(-11, 4, 16)
  }, C.deep)
  P(ctx, {
    V(-13, -9, 18), V(-8, -12, 20), V(-2, -13, 20), V(-5, -9, 18),
    V(-8,  6, 18), V(-12,  8, 17)
  }, C.jade2)
  P(ctx, {
    V(7, -10, 18), V(13, -5, 17), V(12, 6, 17), V(7, 10, 18),
    V(4, 8, 18), V(8, 2, 18)
  }, C.jade1)
  jade_edge(ctx, V(-13, -9, 18), V(-8, -12, 20))
  jade_edge(ctx, V(-8, -12, 20), V(-2, -13, 20))
  jade_edge(ctx, V(7, -10, 18), V(13, -5, 17))
  jade_edge(ctx, V(13, -5, 17), V(12, 6, 17))

  -- Basket rim and a suspended cross-lattice establish the work function.
  L(ctx, V(-12, -7, 17), V(9, -7, 17), C.ink, 2)
  L(ctx, V(-12, -7, 18), V(9, -7, 18), C.straw2, 1)
  L(ctx, V(9, -7, 17), V(11, 4, 17), C.ink, 2)
  L(ctx, V(9, -7, 18), V(11, 4, 18), C.straw2, 1)
  for _, f in ipairs({-8, -4, 0, 4, 8}) do
    rib(ctx, V(f, -6, 18), V(f + 3, 4, 18), f == -4 or f == 4)
  end
  for _, s in ipairs({-4, 0, 4}) do
    rib(ctx, V(-9, s, 18), V(8, s + 1, 18), s == 0)
  end

  -- Rear upper pressure tank and a small operatorless instrument cab.
  layer(ctx, 'upper')
  pressure_tank(ctx, -5, -1, 18, 18, 4)
  B(ctx, 3, -1, 21, 5, 4, 7, C.ink, C.ink, C.ink)
  B(ctx, 3, -1, 22, 4, 3, 5, C.cream1, C.cream2, C.cream0)
  P(ctx, {V(0, -4, 26), V(6, -4, 26), V(6, 2, 26), V(1, 3, 26)}, C.cream2)
  L(ctx, V(0, -4, 26), V(6, -4, 26), C.cream3, 1)
  B(ctx, 3, -4, 23, 2, 1, 2, C.glass1, C.glass2, C.glass0)
  socket(ctx, -5, -1, 37, 2)
  L(ctx, V(-5, -1, 37), V(-5, -1, 40), C.steel2, 2)
  horizontal_cap(ctx, -5, -1, 40, 3, C.steel1)

  -- The near side carries Wick's gripper and its counterweight.
  layer(ctx, 'front')
  local near_hip = V(hips[2][1], hips[2][2], hips[2][3])
  local knee, ankle, toe = leg_pose(hips[2][1], hips[2][2], 2, phase, {0, 2, 1, 3, 0})
  mech_leg(ctx, near_hip, knee, ankle, toe, C.plum1, C.jade1, C.jade2, 3, 2)
  socket(ctx, 10, ns * 4, 20, 2)
  L(ctx, V(10, ns * 4, 20), V(14, ns * 7, 26), C.ink, 3)
  L(ctx, V(10, ns * 4, 20), V(14, ns * 7, 26), C.steel2, 1)
  socket(ctx, 14, ns * 7, 26, 2)
  B(ctx, 14, ns * 7, 24, 3, 3, 4, C.steel1, C.steel2, C.steel0)
  -- A three-finger reed gripper, separated with negative gaps.
  for i = -1, 1 do
    L(ctx, V(15, ns * (7 + i), 26), V(20, ns * (8 + i * 2), 27), C.ink, 2)
    L(ctx, V(15, ns * (7 + i), 26), V(20, ns * (8 + i * 2), 27), C.straw2, 1)
  end
  socket(ctx, -9, ns * 8, 19, 2)
  hose(ctx, {V(-5, ns * 3, 27), V(-10, ns * 7, 30), V(-9, ns * 9, 22)})

  layer(ctx, 'details')
  -- Short seams and fasteners break the shell into intentional composite
  -- panels without covering every surface in texture.
  for f = -9, 5, 7 do
    L(ctx, V(f, -11, 19), V(f + 1, -8, 19), C.jade0, 1)
  end
  for s = -8, 8, 8 do
    socket(ctx, -8, s, 18, 1)
  end
  L(ctx, V(-1, -13, 20), V(6, -10, 20), C.jade3, 1)
  L(ctx, V(-2, 11, 19), V(5, 9, 19), C.jade0, 1)
  mark(ctx, 3, ns * 3, 27, C.glass3)
  mark(ctx, 14, ns * 7, 28, C.amber3)
end

-- SKIPPER ------------------------------------------------------------------

local function paddle(ctx, f, s, phase_shift)
  local sweep = ({-2, -1, 2, 1})[(phase_shift % 4) + 1]
  P(ctx, {
    V(f - 8 + sweep, s - 3, 1), V(f + 5 + sweep, s - 3, 1),
    V(f + 8 + sweep, s, 1), V(f + 3 + sweep, s + 3, 1),
    V(f - 8 + sweep, s + 3, 1), V(f - 10 + sweep, s, 1)
  }, C.ink)
  P(ctx, {
    V(f - 7 + sweep, s - 2, 2), V(f + 4 + sweep, s - 2, 2),
    V(f + 6 + sweep, s, 2), V(f + 2 + sweep, s + 2, 2),
    V(f - 7 + sweep, s + 2, 2), V(f - 8 + sweep, s, 2)
  }, C.jade1)
  L(ctx, V(f - 5 + sweep, s - 1, 2), V(f + 4 + sweep, s - 1, 2), C.jade3, 1)
  L(ctx, V(f + 2 + sweep, s - 2, 2), V(f + 5 + sweep, s, 2), C.straw1, 1)
end

local function draw_skipper(ctx, phase)
  phase = phase_index(phase) - 1
  local ns = near_side(ctx)
  layer(ctx, 'shadows')
  ctx:shadow(18, 5)

  -- Long folding paddle legs leave two clean negative gaps under the hull.
  layer(ctx, 'far')
  local hips = {{7, -9, 19}, {6, 9, 19}}
  for i = 1, 2 do
    local hip = V(hips[i][1], hips[i][2], hips[i][3])
    local knee, ankle, toe = leg_pose(hips[i][1], hips[i][2], i, phase, {0, 2, 1, 3, i == 1 and -1 or 1})
    mech_leg(ctx, hip, knee, ankle, toe, C.plum0, C.steel1, C.steel2, 3, 2)
    paddle(ctx, toe[1] + 3, toe[2], phase + i)
  end

  layer(ctx, 'body')
  local hull = {
    {-17, -10}, {-10, -14}, {3, -13}, {14, -8}, {18, -1},
    {14, 7}, {5, 11}, {-10, 10}, {-18, 3}
  }
  ctx:extrude(hull, 17, 9, C.jade2, C.jade1, C.jade0)
  P(ctx, {
    V(-14, -9, 26), V(-8, -12, 26), V(3, -11, 26), V(13, -7, 26),
    V(16, -1, 26), V(12, 5, 26), V(5, 8, 26), V(-9, 7, 26), V(-15, 2, 26)
  }, C.jade1)
  P(ctx, {V(-11, -10, 27), V(-5, -12, 27), V(3, -11, 27), V(8, -8, 27), V(2, -7, 27), V(-8, -8, 27)}, C.jade2)
  jade_edge(ctx, V(-14, -9, 26), V(-8, -12, 26))
  jade_edge(ctx, V(-8, -12, 26), V(3, -13, 26))
  jade_edge(ctx, V(3, -13, 26), V(14, -8, 26))
  -- Underside intakes reinforce the hydrofoil reading.
  vent(ctx, 10, -8, 19, 3, 5)
  vent(ctx, 10,  8, 19, 3, 5)

  layer(ctx, 'upper')
  -- Woven roof ribs follow the fan's own planes and leave quiet jade fields.
  for s = -9, 9, 3 do
    rib(ctx, V(-12, s, 27), V(10, s * 0.55, 28), s == -6 or s == 3)
  end
  P(ctx, {V(-3, -4, 27), V(4, -4, 27), V(6, 1, 27), V(1, 4, 27), V(-4, 2, 27)}, C.ink)
  P(ctx, {V(-2, -3, 28), V(3, -3, 28), V(4, 0, 28), V(0, 2, 28), V(-3, 1, 28)}, C.cream2)
  B(ctx, 1, 0, 28, 3, 3, 6, C.ink, C.ink, C.ink)
  B(ctx, 1, 0, 29, 2, 2, 4, C.cream1, C.cream2, C.cream0)
  B(ctx, 2, -2, 30, 1, 1, 2, C.glass1, C.glass2, C.glass0)
  pressure_tank(ctx, -2, ns * 10, 19, 12, 3)

  layer(ctx, 'front')
  -- Repaint the near paddle assembly after the hull so its sockets read.
  local near_i = ns > 0 and 2 or 1
  local hip = V(hips[near_i][1], hips[near_i][2], hips[near_i][3])
  local knee, ankle, toe = leg_pose(hips[near_i][1], hips[near_i][2], near_i, phase, {0, 2, 1, 3, 0})
  mech_leg(ctx, hip, knee, ankle, toe, C.plum1, C.jade1, C.jade2, 3, 2)
  paddle(ctx, toe[1] + 3, toe[2], phase + near_i)
  socket(ctx, 11, ns * 8, 23, 2)
  L(ctx, V(11, ns * 8, 23), V(14, ns * 11, 24), C.ink, 2)
  L(ctx, V(11, ns * 8, 23), V(14, ns * 11, 24), C.steel2, 1)
  B(ctx, 14, ns * 11, 21, 2, 2, 5, C.steel1, C.steel2, C.steel0)

  layer(ctx, 'details')
  for s = -8, 0, 8 do
    L(ctx, V(-13, s, 24), V(11, s * 0.55, 25), C.jade0, 1)
  end
  for f = -8, -1, 6 do
    socket(ctx, f, ns * 11, 25, 1)
  end
  L(ctx, V(-10, -12, 24), V(-4, -11, 25), C.straw1, 1)
  L(ctx, V(5, 9, 25), V(11, 5, 24), C.jade3, 1)
  mark(ctx, 13, ns * 11, 25, C.glass3)
end

-- REEDGUARD ---------------------------------------------------------------

local function shield_ribs(ctx, ns)
  -- The ribs interlock with alternating starts; they do not become a flat
  -- repeated hatch over the entire front panel.
  for z = 15, 29, 3 do
    local offset = (z % 2 == 0) and 1 or 0
    rib(ctx, V(13, ns * (-9 + offset), z), V(13, ns * (8 + offset), z + 1), z == 21 or z == 27)
  end
  for s = -7, 7, 4 do
    rib(ctx, V(13, ns * s, 14), V(13, ns * (s + 2), 30), s == -3 or s == 5)
  end
end

local function draw_reedguard(ctx, phase)
  phase = phase_index(phase) - 1
  local ns = near_side(ctx)
  layer(ctx, 'shadows')
  ctx:shadow(19, 5)

  local hips = {
    {-9, -7, 17}, {8, -7, 17}, {-8, 7, 17}, {8, 7, 17}
  }
  layer(ctx, 'far')
  for i = 1, 4 do
    local h = hips[i]
    local hip = V(h[1], h[2], h[3])
    local knee, ankle, toe = leg_pose(h[1], h[2], i, phase, {0, 2, 1, 3})
    mech_leg(ctx, hip, knee, ankle, toe, C.plum0, C.steel1, C.steel2, 3, 2)
  end

  layer(ctx, 'body')
  -- A broad, low chamber gives a heavier mass than Skipper.
  B(ctx, 0, 0, 13, 13, 9, 13, C.ink, C.ink, C.ink)
  B(ctx, 0, 0, 14, 12, 8, 11, C.jade1, C.jade2, C.jade0)
  P(ctx, {V(-10, -8, 25), V(5, -9, 25), V(12, -4, 25), V(10, 6, 25), V(2, 9, 25), V(-10, 7, 25)}, C.jade2)
  jade_edge(ctx, V(-10, -8, 25), V(5, -9, 25))
  jade_edge(ctx, V(5, -9, 25), V(12, -4, 25))
  B(ctx, -2, 2, 17, 6, 3, 7, C.steel0, C.steel1, C.plum0)
  pressure_tank(ctx, -3, -5, 16, 10, 3)
  vent(ctx, -2, -9, 17, 3, 5)

  layer(ctx, 'upper')
  -- Ivory crest and shoulder hardware break the mass at the top.
  P(ctx, {V(-6, -5, 25), V(-1, -5, 25), V(2, -3, 33), V(-2, -2, 35), V(-7, -3, 29)}, C.ink)
  P(ctx, {V(-5, -4, 26), V(-1, -4, 26), V(1, -3, 32), V(-2, -2, 33), V(-6, -3, 29)}, C.cream2)
  L(ctx, V(-5, -4, 27), V(-2, -3, 32), C.cream3, 1)
  spool(ctx, -1, ns * 11, 28, 8, 3)
  socket(ctx, -1, ns * 9, 28, 2)

  layer(ctx, 'front')
  -- Thick woven shield: silhouette, jade backing, then warm structural ribs.
  P(ctx, {
    V(13, ns * -11, 12), V(13, ns * 10, 12), V(13, ns * 11, 28),
    V(13, ns * 6, 33), V(13, ns * -5, 33), V(13, ns * -11, 28)
  }, C.ink)
  P(ctx, {
    V(14, ns * -9, 14), V(14, ns * 8, 14), V(14, ns * 9, 27),
    V(14, ns * 5, 31), V(14, ns * -4, 31), V(14, ns * -9, 27)
  }, C.jade1)
  shield_ribs(ctx, ns)
  -- Near legs and their broad, square feet are redrawn for depth.
  for _, i in ipairs({2, 4}) do
    local h = hips[i]
    local hip = V(h[1], h[2], h[3])
    local knee, ankle, toe = leg_pose(h[1], h[2], i, phase, {0, 2, 1, 3})
    mech_leg(ctx, hip, knee, ankle, toe, C.plum1, C.jade1, C.jade2, 4, 2)
  end

  layer(ctx, 'details')
  for z = 16, 22, 6 do
    L(ctx, V(-11, -8, z), V(9, -8, z + 1), C.jade0, 1)
  end
  socket(ctx, -10, -8, 21, 1)
  socket(ctx, 8, 7, 21, 1)
  L(ctx, V(-7, 8, 24), V(4, 7, 24), C.jade3, 1)
  L(ctx, V(-8, -8, 20), V(-2, -8, 20), C.steel3, 1)
  mark(ctx, -1, ns * 12, 31, C.glass3)
  mark(ctx, -3, -5, 24, C.amber3)
end

-- LOOM ---------------------------------------------------------------------

local function arch(ctx, f)
  -- Segmented thick beams make an imperfect civil-engineering arch.  The
  -- two arches are parallel in f and surround the suspended machine.
  local a = V(f, -12, 8)
  local b = V(f, -12, 25)
  local c = V(f, -10, 37)
  local d = V(f, -5, 41)
  local e = V(f,  0, 42)
  local g = V(f,  5, 41)
  local h = V(f, 10, 36)
  local i = V(f, 12, 25)
  local j = V(f, 12, 8)
  local segments = {{a,b},{b,c},{c,d},{d,e},{e,g},{g,h},{h,i},{i,j}}
  for _, pair in ipairs(segments) do
    L(ctx, pair[1], pair[2], C.ink, 5)
    L(ctx, pair[1], pair[2], C.jade1, 3)
    L(ctx, pair[1], pair[2], C.jade3, 1)
  end
  -- A few warm ribs sit on the inner edge and establish the composite weave.
  for _, z in ipairs({13, 19, 26, 33, 39}) do
    local s = z < 33 and (z < 22 and 11 or 10) or (z < 39 and 8 or 4)
    rib(ctx, V(f, -s, z), V(f, -s + 2, z + 2), z == 19 or z == 33)
    rib(ctx, V(f, s, z), V(f, s - 2, z + 2), z == 26 or z == 39)
  end
end

local function draw_loom(ctx, phase)
  phase = phase_index(phase) - 1
  local ns = near_side(ctx)
  layer(ctx, 'shadows')
  ctx:shadow(19, 5)

  local hips = {{-7, -9, 10}, {-7, 9, 10}, {8, 0, 11}}
  layer(ctx, 'far')
  for i = 1, 3 do
    local h = hips[i]
    local hip = V(h[1], h[2], h[3])
    local knee, ankle, toe = leg_pose(h[1], h[2], i, phase, {0, 2, 1, 3, i == 3 and 1 or 0})
    mech_leg(ctx, hip, knee, ankle, toe, C.plum0, C.steel1, C.steel2, 3, 2)
  end

  layer(ctx, 'body')
  -- Cross braces and the low carrier keep the open frame grounded.
  B(ctx, 0, 0, 9, 8, 6, 6, C.ink, C.ink, C.ink)
  B(ctx, 0, 0, 10, 7, 5, 4, C.jade1, C.jade2, C.jade0)
  L(ctx, V(-7, -8, 12), V(7, 8, 12), C.ink, 3)
  L(ctx, V(-7, 8, 12), V(7, -8, 12), C.ink, 3)
  L(ctx, V(-7, -8, 12), V(7, 8, 12), C.steel1, 1)

  layer(ctx, 'upper')
  arch(ctx, -5)
  arch(ctx, 5)
  -- Crossbars give the pair a shared engineered frame without closing the
  -- central negative space.
  for _, z in ipairs({18, 34}) do
    L(ctx, V(-5, -10, z), V(5, -10, z), C.ink, 3)
    L(ctx, V(-5, -10, z), V(5, -10, z), C.jade2, 1)
  end
  for _, s in ipairs({-10, 10}) do
    socket(ctx, -5, s, 18, 2)
    socket(ctx, 5, s, 34, 2)
  end

  -- The amber drum hangs in the aperture; bands and valves establish weight.
  pressure_tank(ctx, 0, 0, 18, 17, 4)
  socket(ctx, 0, 0, 36, 2)
  B(ctx, 0, 0, 36, 2, 2, 2, C.steel1, C.steel2, C.steel0)
  hose(ctx, {V(0, 0, 18), V(-4, ns * 8, 16), V(-5, ns * 10, 11)})

  layer(ctx, 'front')
  -- Offset horizontal spool launcher: side-mounted and visibly functional.
  socket(ctx, 4, ns * 12, 25, 2)
  spool(ctx, 6, ns * 13, 25, 9, 3)
  B(ctx, 2, ns * 12, 22, 2, 2, 5, C.steel1, C.steel2, C.steel0)
  L(ctx, V(10, ns * 13, 25), V(14, ns * 13, 25), C.plum1, 2)
  L(ctx, V(10, ns * 13, 25), V(17, ns * 13, 25), C.straw2, 1)
  -- Three spread feet retain the open icon in motion.
  local near_i = ns > 0 and 2 or 1
  local h = hips[near_i]
  local hip = V(h[1], h[2], h[3])
  local knee, ankle, toe = leg_pose(h[1], h[2], near_i, phase, {0, 2, 1, 3, 0})
  mech_leg(ctx, hip, knee, ankle, toe, C.plum1, C.jade1, C.jade2, 3, 2)

  layer(ctx, 'details')
  -- Tension stays subordinate to the arch silhouette and terminates at real
  -- sockets rather than floating in the aperture.
  L(ctx, V(-5, -10, 34), V(0, -2, 30), C.steel2, 1)
  L(ctx, V(5, 10, 34), V(0, 2, 30), C.steel2, 1)
  L(ctx, V(-5, 10, 20), V(0, 3, 18), C.straw1, 1)
  for s = -8, 8, 8 do
    L(ctx, V(-4, s, 13), V(4, s + 1, 13), C.jade3, 1)
  end
  mark(ctx, 0, 0, 28, C.amber3)
  mark(ctx, 6, ns * 13, 27, C.glass3)
end

M.wick = draw_wick
M.skipper = draw_skipper
M.reedguard = draw_reedguard
M.loom = draw_loom

return M
