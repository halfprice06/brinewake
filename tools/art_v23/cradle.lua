-- The Raker's Hauling kit: doctrine_raker_{f}_hauling_{0..3} (64x64,
-- anchor 32,50), drawn over the Raker at its anchor like the Hook's cargo
-- cradle and the Wick's panniers (art v9). A Hauling Raker carries 8, not 5,
-- so it wears two salt panniers slung from its basket's rim on copper
-- straps, one each side, clear of the rake's pivot at the front corner.
-- They swing a unit along the heading and back over the four frames.
--
-- The panniers hang below the basket (their lids at z 10, the basket's
-- floor at z 12) so the straps show and the two never merge into one mass.
-- They are a step darker than the body (glaze1/glaze2 walls), with rounded
-- ends and a copper band, and the salt they carry heaps white above the lid.
--
-- Scripted pixel construction through the v21 painter (the Raker's own
-- projection, materials and light), with the far pannier cut away wherever
-- the Raker stands in front of it in any of its poses. No shadow is drawn:
-- the Raker's own contact shadow stays the only one.
local M={}
local SWAY={0,1,0,-1}
-- The pannier's walls a step darker than the Raker's body.
local DARKER={glaze4='glaze3',glaze3='glaze2',glaze2='glaze1'}

-- One pannier on local side `side` (+1 or -1), swung `sway` units along
-- the heading: two copper straps from the rim (s = 6, z = 17) down to the
-- pannier's lid, a rounded glazed basket with a copper band, and the salt
-- it carries heaped proud of the lid.
local function pannier(c,side,sway)
  local s=side*8
  local f=-1+sway
  local saved=c.remap
  local merged={};for a,b in pairs(saved or {}) do merged[a]=b end
  for a,b in pairs(DARKER) do merged[a]=b end
  c.remap=merged
  c:box(f,s,6,3,2,4,'glaze',2)
  c.remap=saved
  c:line3({f-3,s+side*2,8},{f+2,s+side*2,8},'rust1')
  c:poly3({{f-2,s-1,10},{f+2,s-1,10},{f+2,s+1,10},{f-2,s+1,10}},'crust1')
  c:line3({f-1,s,11},{f+1,s,11},'crust2')
  c:line3({-4,side*6,17},{f-2,s,10},'rust1')
  c:line3({2,side*6,17},{f+2,s,10},'rust2')
end

-- Paint one facing and frame. `depth` is compact.lua's screen-depth test;
-- the far pannier goes on painter layer 2, the near on layer 5.
function M.paint(c,depth,frame)
  local sway=SWAY[frame+1]
  for _,side in ipairs({1,-1}) do
    local near=depth(c,0,side*8)>0
    c:layer(near and 5 or 2)
    pannier(c,side,sway)
  end
end
return M
