-- Build the lock switch: art/source/v20/gate_switch.aseprite, then paint
-- its frames into the runtime atlas as new keys.
--
-- The document starts from the v13 gate_north_dry document: every layer
-- but "working parts" is the v13 drawing, unchanged on all frames. The
-- working parts of each frame come from gate_switch.py (floats and water,
-- the semaphore arm, the handwheel), run first into `parts`.
-- Tags: to_south_dry (the north shaft fills, the south drains) and
-- to_north_dry. Frames are 166 ms, the engine's 5 ticks, so Aseprite's
-- preview plays at game speed.
--
-- Usage: aseprite --batch --script-param root=$PWD
--          --script-param parts=<dir> --script tools/art_v20/gate_switch.lua
local root=assert(app.params.root)
local parts=assert(app.params.parts)
local FRAMES=12
local W,H,AX,AY=160,144,80,128
-- A band under every v19 rectangle, checked empty when this pass was made.
local BAND_X,BAND_Y=0,7832

local function read(p) local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s) local f=assert(io.open(p,'w'));f:write(s);f:close() end

local sprite=assert(app.open(root..'/art/source/v13/gate_north_dry.aseprite'))
while #sprite.frames>1 do sprite:deleteFrame(sprite.frames[#sprite.frames]) end
-- Every layer but the working parts keeps the rest drawing on every frame.
for _=2,2*FRAMES do sprite:newFrame(#sprite.frames+1) end
for _,l in ipairs(sprite.layers) do
 local rest=l:cel(1)
 if rest and l.name~='working parts' then
  for f=2,2*FRAMES do sprite:newCel(l,f,rest.image,rest.position) end
 end
end
local wp
for _,l in ipairs(sprite.layers) do if l.name=='working parts' then wp=l end end
assert(wp,'working parts layer')
local keys={}
local n=0
for _,side in ipairs({'south','north'}) do
 local first=n+1
 for i=0,FRAMES-1 do
  n=n+1
  local fr=sprite.frames[n]
  fr.duration=0.166
  local img=Image{fromFile=parts..'/wp_to_'..side..'_dry_'..i..'.png'}
  assert(img.width==W and img.height==H,'part size')
  local cel=wp:cel(fr)
  if cel then sprite:deleteCel(cel) end
  sprite:newCel(wp,fr,img,Point(0,0))
  keys[n]='gate_to_'..side..'_dry_'..i
 end
 local tag=sprite:newTag(first,n);tag.name='to_'..side..'_dry'
end
-- The frames must come out in order: check each against its part.
for i=1,n do
 local side=i<=FRAMES and 'south' or 'north'
 local part=Image{fromFile=parts..'/wp_to_'..side..'_dry_'..((i-1)%FRAMES)..'.png'}
 local c=wp:cel(i)
 assert(c and c.image:isEqual(part),'frame '..i..' out of order')
end
sprite:saveAs(root..'/art/source/v20/gate_switch.aseprite')

-- Export into the atlas.
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local manifest=json.decode(read(root..'/art/exports/game-assets.json'))
local sprites={}
for k,v in pairs(manifest.sprites) do
 local b={x=v.x,y=v.y,w=v.w,h=v.h,anchor_x=v.anchor_x,anchor_y=v.anchor_y}
 if v.muzzle then b.muzzle={v.muzzle[1],v.muzzle[2]} end
 sprites[k]=b
end
for i=1,n do
 local x=BAND_X+((i-1)%FRAMES)*W
 local y=BAND_Y+((i-1)//FRAMES)*H
 local frame=Image(W,H,ColorMode.RGB);frame:drawSprite(sprite,i)
 for py=0,H-1 do for px=0,W-1 do atlas:drawPixel(x+px,y+py,frame:getPixel(px,py)) end end
 sprites[keys[i]]={x=x,y=y,w=W,h=H,anchor_x=AX,anchor_y=AY}
end
sprite:close()
atlas:saveAs(root..'/art/exports/game-assets.png')
write(root..'/art/exports/game-assets.json',json.encode({width=manifest.width,height=manifest.height,sprites=sprites}))
local list={}
for i=1,n do list[i]={key=keys[i],document='gate_switch',frame=i,w=W,h=H,anchor_x=AX,anchor_y=AY,duration_ms=166} end
write(root..'/tools/art_v20/sources.json',json.encode(list))
print('Exported '..n..' gate switch frames.')
