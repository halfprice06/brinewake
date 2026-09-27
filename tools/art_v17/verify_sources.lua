-- Reopen every saved v17 document and compare it with the active atlas:
-- exact pixels, canvas size, layer names, anchors, durations and tags. Prove
-- that every archived v16 rectangle is still exact and in place, that every
-- machine frame keeps its shadow in the shadow colour only and never over
-- the body, and that damage overlays stay on their base's silhouette above
-- its lowest body row (nothing beneath a building).
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v17/common.lua')
local entries=json.decode(C.read(root..'/tools/art_v17/sources.json'))
local m=json.decode(C.read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local archive=json.decode(C.read(root..'/art/archive/art-v16/game-assets.json'))
local archived=Image{fromFile=root..'/art/archive/art-v16/game-assets.png'}
local A=app.pixelColor.rgbaA
assert(atlas.width==C.ATLAS_W and atlas.height==C.ATLAS_H,'atlas size')
local kept=0
for key,b in pairs(archive.sprites) do
 local n=assert(m.sprites[key],key..' dropped')
 assert(n.x==b.x and n.y==b.y and n.w==b.w and n.h==b.h and n.anchor_x==b.anchor_x and n.anchor_y==b.anchor_y,key..' rectangle changed')
 assert(Image(atlas,Rectangle(b.x,b.y,b.w,b.h)):isEqual(Image(archived,Rectangle(b.x,b.y,b.w,b.h))),key..' pixels changed');kept=kept+1
end
local function full(l,f,w,h) local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position) end;return im end
local bases={}
local name,s;local docs,count,shadows,overlays=0,0,0,0
for _,e in ipairs(entries) do
 if name~=e.document then if s then s:close() end;name=e.document;s=assert(app.open(root..'/art/source/v17/'..name..'.aseprite'));docs=docs+1 end
 local b=assert(m.sprites[e.key],e.key)
 assert(b.x>=C.RIGHT,e.key..' packed into the archived half')
 assert(s.width==e.w and s.height==e.h and b.w==e.w and b.h==e.h,e.key..' canvas size')
 assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y,e.key..' anchor')
 for i,l in ipairs(s.layers) do assert(l.name==e.layers[i],e.key..' layer '..i..' name') end
 assert(math.floor(s.frames[e.frame].duration*1000+.5)==e.duration_ms,e.key..' duration')
 local frame=Image(e.w,e.h,ColorMode.RGB);frame:drawSprite(s,e.frame)
 assert(frame:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from the atlas')
 count=count+1
 if s.layers[1].name=='contact shadows' then
  -- Machine and building frames: the shadow layer holds only the shadow
  -- colour and never overlaps the body; no body pixel uses a shadow role.
  local shadow=full(s.layers[1],e.frame,e.w,e.h);local body=Image(e.w,e.h,ColorMode.RGB)
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position) end end
  for y=0,e.h-1 do for x=0,e.w-1 do
   local p=shadow:getPixel(x,y)
   if A(p)>0 then assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour');assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body') end
   local bp=body:getPixel(x,y)
   if A(bp)>0 then assert(bp~=C.rgba.cast and bp~=C.rgba.contact,e.key..' body uses a shadow role') end
  end end
  shadows=shadows+1
  if not e.key:find('_damage') then bases[e.key]=body end
 elseif s.layers[1].name=='dents and tears' then
  -- Damage overlays sit on the base silhouette above its foundation rows.
  local base_key=e.key:gsub('_damage_%d+$',''):gsub('_damage$','')
  local base=bases[base_key] or (function() local im,bb=C.sprite_image_any(m,base_key,atlas);return im end)()
  local lowest=-1
  for y=0,e.h-1 do for x=0,e.w-1 do if A(base:getPixel(x,y))>0 then lowest=y end end end
  local overlay=Image(e.w,e.h,ColorMode.RGB)
  for i=1,#s.layers do local c=s.layers[i]:cel(e.frame);if c then overlay:drawImage(c.image,c.position) end end
  for y=0,e.h-1 do for x=0,e.w-1 do
   if A(overlay:getPixel(x,y))>0 then
    assert(y<=lowest,e.key..' draws beneath its base');
    assert(A(base:getPixel(x,y))>0,e.key..' draws outside its base silhouette')
   end
  end end
  overlays=overlays+1
 end
end
if s then s:close() end
print('Verified '..count..' v17 frames in '..docs..' documents ('..shadows..' shadowed, '..overlays..' overlays); '..kept..' archived rectangles exact.')
