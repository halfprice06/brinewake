-- Reopen every saved v18 document and compare it with the active atlas:
-- exact pixels, canvas size, layer names, anchors, durations and tags. Prove
-- that every archived v17 rectangle that v18 did not re-author is still
-- exact and in place and that every re-authored key kept its rectangle and
-- anchor; that every machine frame keeps its shadow in the shadow colour only
-- and never over the body; that damage overlays stay on their base's
-- silhouette above its lowest row and never on glass; and report the colour
-- count of every re-authored base frame against the budget.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v18/common.lua')
local entries=json.decode(C.read(root..'/tools/art_v18/sources.json'))
local m=json.decode(C.read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local archive=json.decode(C.read(root..'/art/archive/art-v17/game-assets.json'))
local archived=Image{fromFile=root..'/art/archive/art-v17/game-assets.png'}
local A=app.pixelColor.rgbaA
assert(atlas.width==C.ATLAS_W and atlas.height==C.ATLAS_H,'atlas size')
-- A key is re-authored when it is registered, or when it is an alias sharing
-- a registered key's rectangle (`barge` for `barge_0`).
local reauthored={};local rects={}
for _,e in ipairs(entries) do reauthored[e.key]=true;local b=m.sprites[e.key];rects[b.x..','..b.y]=true end
for key,b in pairs(m.sprites) do if rects[b.x..','..b.y] then reauthored[key]=true end end
local kept,replaced=0,0
for key,b in pairs(archive.sprites) do
 local n=assert(m.sprites[key],key..' dropped')
 assert(n.x==b.x and n.y==b.y and n.w==b.w and n.h==b.h and n.anchor_x==b.anchor_x and n.anchor_y==b.anchor_y,key..' rectangle changed')
 if reauthored[key] then replaced=replaced+1
 else assert(Image(atlas,Rectangle(b.x,b.y,b.w,b.h)):isEqual(Image(archived,Rectangle(b.x,b.y,b.w,b.h))),key..' pixels changed');kept=kept+1 end
end
for key in pairs(m.sprites) do assert(archive.sprites[key],key..' is new: v18 only re-authors existing keys') end
local glass={};for _,r in ipairs({'glass0','glass1','glass2','glass3','glass4'}) do glass[C.rgba[r]]=true end
local function full(l,f,w,h) local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position) end;return im end
local bases={}
local name,s;local docs,count,shadows,overlays=0,0,0,0
local worst={}
for _,e in ipairs(entries) do
 if name~=e.document then if s then s:close() end;name=e.document;s=assert(app.open(root..'/art/source/v18/'..name..'.aseprite'));docs=docs+1 end
 local b=assert(m.sprites[e.key],e.key)
 assert(s.width==e.w and s.height==e.h and b.w==e.w and b.h==e.h,e.key..' canvas size')
 assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y,e.key..' anchor')
 for i,l in ipairs(s.layers) do assert(l.name==e.layers[i],e.key..' layer '..i..' name') end
 assert(math.floor(s.frames[e.frame].duration*1000+.5)==e.duration_ms,e.key..' duration')
 local frame=Image(e.w,e.h,ColorMode.RGB);frame:drawSprite(s,e.frame)
 assert(frame:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from the atlas')
 count=count+1
 if s.layers[1].name=='contact shadows' then
  local shadow=full(s.layers[1],e.frame,e.w,e.h);local body=Image(e.w,e.h,ColorMode.RGB)
  local imgs={}
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position);imgs[#imgs+1]=c.image end end
  for y=0,e.h-1 do for x=0,e.w-1 do
   local p=shadow:getPixel(x,y)
   if A(p)>0 then assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour');assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body') end
   local bp=body:getPixel(x,y)
   if A(bp)>0 then assert(bp~=C.rgba.cast and bp~=C.rgba.contact,e.key..' body uses a shadow role') end
  end end
  if e.w==160 then
   -- Buildings: the shadow layer must stay empty.
   local n=0;for y=0,e.h-1 do for x=0,e.w-1 do if A(shadow:getPixel(x,y))>0 then n=n+1 end end end
   assert(n==0,e.key..' draws a shadow beneath a building')
  end
  shadows=shadows+1
  if not e.key:find('_damage') then
   bases[e.key]=body
   local doc=e.document
   local n=C.colours({body},true)
   if not worst[doc] or n>worst[doc] then worst[doc]=n end
  end
 elseif s.layers[1].name=='dents and tears' then
  local base_key=e.key:gsub('_damage_%d+$',''):gsub('_damage$','')
  local base=bases[base_key] or (function() local im=C.sprite_image_any(m,base_key,atlas);return im end)()
  local lowest=-1
  for y=0,e.h-1 do for x=0,e.w-1 do if A(base:getPixel(x,y))>0 then lowest=y end end end
  local overlay=Image(e.w,e.h,ColorMode.RGB)
  for i=1,#s.layers do local c=s.layers[i]:cel(e.frame);if c then overlay:drawImage(c.image,c.position) end end
  local n=0
  for y=0,e.h-1 do for x=0,e.w-1 do
   if A(overlay:getPixel(x,y))>0 then
    n=n+1
    assert(y<=lowest,e.key..' draws beneath its base')
    local bp=base:getPixel(x,y)
    assert(A(bp)>0,e.key..' draws outside its base silhouette')
    assert(bp~=C.rgba.cast and bp~=C.rgba.contact,e.key..' draws on the shadow')
    assert(not glass[bp],e.key..' draws on glass')
   end
  end end
  assert(n>0,e.key..' is empty')
  overlays=overlays+1
 end
end
if s then s:close() end
local names={};for k in pairs(worst) do names[#names+1]=k end;table.sort(names)
local parts={};for _,k in ipairs(names) do parts[#parts+1]=k..'='..worst[k] end
print('Colours per re-authored document (max over frames, shadow excluded): '..table.concat(parts,', '))
print('Verified '..count..' v18 frames in '..docs..' documents ('..shadows..' shadowed, '..overlays..' overlays); '..kept..' archived rectangles exact, '..replaced..' re-authored in place.')
