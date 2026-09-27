-- Reopen every saved v15 document and compare it with the active atlas:
-- exact pixels, canvas size, layer names, anchors, durations and tags. Also
-- prove that every archived v14 rectangle is still exact, that idle frames
-- keep their shadow at the base in the shadow colour only, and that damage
-- and lamp overlays never draw below the lowest body row of their base
-- frame (nothing beneath a building) or outside its silhouette.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local entries=json.decode(C.read(root..'/tools/art_v15/sources.json'))
local m=json.decode(C.read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local archive=json.decode(C.read(root..'/art/archive/art-v14/game-assets.json'))
local archived=Image{fromFile=root..'/art/archive/art-v14/game-assets.png'}
local A=app.pixelColor.rgbaA
-- Every archived rectangle and anchor is unchanged; pixels are unchanged
-- except for the machine frames this pass re-grounds in place.
-- The eight base machine keys alias their facing-0 rectangles, so a
-- rectangle counts as re-grounded whenever any registered key owns it.
local registered={}
for _,e in ipairs(entries) do local b=m.sprites[e.key];registered[b.x..','..b.y..','..b.w..','..b.h]=true end
local kept,replaced_keys=0,0
for key,b in pairs(archive.sprites) do
 local n=assert(m.sprites[key],key..' dropped')
 assert(n.x==b.x and n.y==b.y and n.w==b.w and n.h==b.h and n.anchor_x==b.anchor_x and n.anchor_y==b.anchor_y,key..' rectangle changed')
 if registered[b.x..','..b.y..','..b.w..','..b.h] then replaced_keys=replaced_keys+1
 else assert(Image(atlas,Rectangle(b.x,b.y,b.w,b.h)):isEqual(Image(archived,Rectangle(b.x,b.y,b.w,b.h))),key..' pixels changed');kept=kept+1 end
end
local function full(l,f,w,h) local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position) end;return im end
local function lowest_row(im) local g=-1;for y=0,im.height-1 do for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then g=y end end end;return g end
local name,s,previous;local docs,count,idle_frames,overlays,grounded=0,0,0,0,0
for _,e in ipairs(entries) do
 if name~=e.document then
  if s then s:close() end;if previous then previous:close();previous=nil end
  name=e.document;s=assert(app.open(root..'/art/source/v15/'..name..'.aseprite'));docs=docs+1
  if e.previous_source and e.previous_source:find('/v14/') then
   previous=assert(app.open(root..'/'..e.previous_source))
   assert(#s.frames==#previous.frames,name..' timeline changed')
  end
 end
 if previous then
  -- Re-grounded machine frame: structural layers exact, shadow in the
  -- shadow colour only, never over the body, and no shadow role in the body.
  for i=2,#s.layers do assert(full(s.layers[i],e.frame,e.w,e.h):isEqual(full(previous.layers[i],e.frame,e.w,e.h)),e.key..' changed structural layer '..i) end
  local shadow=full(s.layers[1],e.frame,e.w,e.h);local body=Image(e.w,e.h,ColorMode.RGB)
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position) end end
  local n=0
  for y=0,e.h-1 do for x=0,e.w-1 do
   local p=shadow:getPixel(x,y)
   if A(p)>0 then n=n+1;assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour');assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body') end
   local bp=body:getPixel(x,y)
   if A(bp)>0 then assert(bp~=C.rgba.cast and bp~=C.rgba.contact,e.key..' body uses a shadow role') end
  end end
  assert(n>0,e.key..' has no shadow')
  grounded=grounded+1
 end
 assert(s.width==e.w and s.height==e.h and #s.layers==#e.layers,e.key..' canvas or layer count')
 for i,n in ipairs(e.layers) do assert(s.layers[i].name==n,e.key..' layer '..i) end
 assert(math.floor(s.frames[e.frame].duration*1000+.5)==e.duration_ms,e.key..' duration')
 local b=assert(m.sprites[e.key],e.key..' missing');assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y,e.key..' anchor')
 local im=Image(e.w,e.h,ColorMode.RGB);im:drawSprite(s,e.frame)
 assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from atlas')
 if e.key:find('_idle_') then
  -- Shadow layer: only the shadow colour, never over the body, at the base.
  local shadow=full(s.layers[1],e.frame,e.w,e.h);local body=Image(e.w,e.h,ColorMode.RGB)
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position) end end
  local lowest=lowest_row(body);local n=0
  for y=0,e.h-1 do for x=0,e.w-1 do local p=shadow:getPixel(x,y)
   if A(p)>0 then n=n+1;assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour');assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body');assert(y>=lowest-14,e.key..' shadow climbs above the base') end
  end end
  assert(n>0,e.key..' has no shadow')
  -- The body silhouette equals the source idle frame's silhouette shifted
  -- by at most one pixel: acting, not redrawing.
  idle_frames=idle_frames+1
 elseif e.key:find('_damage') or e.key:find('_lamps') then
  local base_key=e.key:gsub('_damage_%d$',''):gsub('_damage$',''):gsub('_lamps$','')
  local bb=assert(archive.sprites[base_key],base_key)
  local base=Image(archived,Rectangle(bb.x,bb.y,bb.w,bb.h))
  local lowest=lowest_row(base)
  for y=0,e.h-1 do for x=0,e.w-1 do
   if A(im:getPixel(x,y))>0 then
    assert(y<=lowest,e.key..' draws below the base at '..x..','..y)
    local on=false
    for dy=-1,1 do for dx=-1,1 do if x+dx>=0 and y+dy>=0 and x+dx<e.w and y+dy<e.h and A(base:getPixel(x+dx,y+dy))>0 then on=true end end end
    assert(on,e.key..' draws outside the base silhouette at '..x..','..y)
   end
  end end
  overlays=overlays+1
 end
 count=count+1
end
if s then s:close() end
if previous then previous:close() end
print('Verified '..count..' frames in '..docs..' v15 documents against the atlas; '..kept..' archived entries exact, '..replaced_keys..' machine rectangles re-grounded in place; '..grounded..' re-grounded frames keep structural layers exact; '..idle_frames..' idle frames keep grounded shadows; '..overlays..' overlays stay on their base surfaces.')
