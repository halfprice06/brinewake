-- v19 motion parts shared by all sixteen machines: the wheel, the track,
-- the Barge's stern wheel and the Lifter's screw. The rest pose (phase nil)
-- of each is the v4/v18 drawing unchanged; only the moving frames differ.
--
-- The engine picks a walk frame by distance, once per quarter cell, and
-- samples it once per 30 Hz tick; a Tidewatch on a screen-axis facing moves
-- 1.6 phases a tick. A part with rotational symmetry that turns half its
-- symmetry period per phase (v4: four spokes, 45 degrees; v18 screw: three
-- blades, 60 degrees) has no direction at all and strobes, and anything
-- over half reads backwards. Every rotation here turns a quarter of its
-- symmetry period per phase, loops in the four phases, and stays under half
-- a period per tick at every speed in the game.
local M={}
local function sign(n) return n>=0 and 1 or -1 end
M.sign=sign

-- A spoked wheel in the forward-up plane. Rolling forward turns the top
-- toward the front: the spoke angle, measured from forward toward up,
-- decreases 22.5 degrees a phase (a wheel of radius 8 rolling four units
-- turns 29 degrees; the four spokes repeat every 90).
function M.wheel(c,f,s,z,r,phase)
  c:disc({f,s,z},{1,0,0},{0,0,1},r+1,'ink')
  c:disc({f,s,z},{1,0,0},{0,0,1},r,'steel1')
  c:disc({f,s,z},{1,0,0},{0,0,1},r-2,'rust2')
  local p=c:point(f-r/3,s,z+r/3)
  c:line(p,{p[1]+2,p[2]},'rust4')
  local a=-(phase or 0)*math.pi/8
  for k=0,3 do
    local t=a+k*math.pi/2
    c:line3({f,s,z},{f+math.cos(t)*(r-3),s,z+math.sin(t)*(r-3)},'rust0')
  end
  c:disc({f,s,z},{1,0,0},{0,0,1},2,'steel3')
end

-- A track. The tread bars on the top run sit every four units; the top run
-- moves forward over the hull as the machine drives, one unit a phase (a
-- quarter of the bar pitch), wrapping at the ends. v4 shifted them 0,1,0,1:
-- a shimmer, not a drive.
function M.tracks(c,phase,hf,hs)
  local near=sign(c.co)
  c:shadow(hf+2,5)
  local shift=phase or 0
  for _,side in ipairs({-near,near}) do
    c:layer(side==near and 3 or 2)
    c:box(0,side*hs,1,hf,3,6,'steel',3)
    for f=-hf-1,hf-2,4 do
      local at=f+shift
      if at>=-hf+2 and at<=hf-2 then
        c:line3({at,side*hs-2,7},{at,side*hs+2,7},'steel0')
      end
    end
    if side==near then
      for _,f in ipairs({-hf+4,0,hf-4}) do
        c:disc({f,side*(hs+3),4},{1,0,0},{0,0,1},3,'steel0')
        c:disc({f,side*(hs+3),4},{1,0,0},{0,0,1},2,'steel2')
        if phase then
          -- Driving, each road wheel shows a hub bar turning 45 degrees a
          -- phase (a quarter of the bar's 180-degree period), top forward.
          -- Seen side-on the top run is hidden under the hull and this is
          -- the only drive the eye can follow. At rest the hub stays plain.
          local t=-phase*math.pi/4
          for _,k in ipairs({-1,1}) do
            local q=c:point(f+k*math.cos(t)*1.4,side*(hs+3),4+k*math.sin(t)*1.4)
            c:pixel(q[1],q[2],'steel0')
          end
        end
        local p=c:point(f,side*(hs+3),4);c:pixel(p[1],p[2],'rust3')
      end
    end
  end
end

-- The Barge's stern wheel: four paddle arms, turning like a rolling wheel
-- (the bottom pushes water aft), 22.5 degrees a phase.
function M.stern_wheel(c,f,z,phase)
  local a=-(phase or 0)*math.pi/8
  c:disc({f,0,z},{1,0,0},{0,0,1},5,'steel0')
  c:disc({f,0,z},{1,0,0},{0,0,1},4,'jade1')
  for k=0,3 do
    local t=a+k*math.pi/2
    c:line3({f,0,z},{f+math.cos(t)*4,0,z+math.sin(t)*4},'steel3')
  end
  c:dot(f,0,z,1,'steel')
end

-- The Lifter's screw: three blades in the side-up plane, 30 degrees a
-- phase (a quarter of the 120-degree blade period).
function M.screw(c,f,z,phase)
  local a=(phase or 0)*math.pi/6
  c:disc({f,0,z},{0,1,0},{0,0,1},4,'steel0')
  for k=0,2 do
    local t=a+k*2*math.pi/3
    c:line3({f,0,z},{f,math.cos(t)*4,z+math.sin(t)*4},'steel3')
  end
end
return M
