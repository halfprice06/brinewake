-- Reopen every saved v19 document and compare it with the active atlas:
-- exact pixels, canvas size, layer names, anchors and durations. Prove that
-- every archived v18 rectangle v19 did not re-author is still exact and in
-- place, that every re-authored key kept its rectangle and anchor and no
-- key was added; that every state v19 only recoloured is the v18 pixel
-- through its machine's fold and nothing else (same silhouette, same
-- shading structure), and that the v18 roles' rest frames are unchanged;
-- that every machine frame keeps its shadow in the shadow colour only and
-- never over the body; that damage overlays stay on their base's silhouette
-- above its lowest row and never on glass; and report the colour count of
-- every re-authored document against the budget.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v19/common.lua')
local entries={}
for _,e in ipairs(json.decode(C.read(root..'/tools/art_v19/sources.json'))) do
 local layers={};for i,n in ipairs(e.layers) do layers[i]=n end
 entries[#entries+1]={key=e.key,document=e.document,frame=e.frame,w=e.w,h=e.h,anchor_x=e.anchor_x,anchor_y=e.anchor_y,duration_ms=e.duration_ms,layers=layers}
end
local m=json.decode(C.read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local archive=json.decode(C.read(C.ARCHIVE..'game-assets.json'))
local archived=Image{fromFile=C.ARCHIVE..'game-assets.png'}
local A=app.pixelColor.rgbaA
assert(atlas.width==C.ATLAS_W and atlas.height==C.ATLAS_H,'atlas size')
local reauthored={};local rects={}
for _,e in ipairs(entries) do reauthored[e.key]=true;local b=m.sprites[e.key];rects[b.x..','..b.y]=true end
for key,b in pairs(m.sprites) do if rects[b.x..','..b.y] then reauthored[key]=true end end
local kept,replaced=0,0
for key,b in pairs(archive.sprites) do
 local n=assert(m.sprites[key],key..' dropped')
 assert(n.x==b.x and n.y==b.y and n.w==b.w and n.h==b.h and n.anchor_x==b.anchor_x and n.anchor_y==b.anchor_y,key..' rectangle changed')
 if (n.muzzle or b.muzzle) then assert(n.muzzle and b.muzzle and n.muzzle[1]==b.muzzle[1] and n.muzzle[2]==b.muzzle[2],key..' muzzle changed') end
 if reauthored[key] then replaced=replaced+1
 else assert(Image(atlas,Rectangle(b.x,b.y,b.w,b.h)):isEqual(Image(archived,Rectangle(b.x,b.y,b.w,b.h))),key..' pixels changed');kept=kept+1 end
end
for key in pairs(m.sprites) do assert(archive.sprites[key],key..' is new: v19 only re-authors existing keys') end

-- Which machine a key belongs to, and whether v19 redrew it or only
-- recoloured it.
local originals={hook=true,riveter=true,bulwark=true,sounder=true,wick=true,skipper=true,reedguard=true,loom=true}
local function machine_of(key)
 local n=key:match('^pressure_(%a+)_') or key:match('^doctrine_(%a+)_') or key:match('^(%a+)_')
 return n
end
local function redrawn(key) return key:find('_walk_') or key:find('_idle_') end
local function is_rest(key) return key:match('^%a+_%d$')~=nil end
local folds={}
for n in pairs(originals) do folds[n]=C.fold_pixels(C.folds[C.faction[n]]) end
local recoloured,unchanged_rest=0,0
for _,e in ipairs(entries) do
 local n=machine_of(e.key);local b=m.sprites[e.key]
 local now=Image(atlas,Rectangle(b.x,b.y,b.w,b.h));local was=Image(archived,Rectangle(b.x,b.y,b.w,b.h))
 if originals[n] and not redrawn(e.key) then
  local fold=folds[n]
  for y=0,b.h-1 do for x=0,b.w-1 do
   local v=was:getPixel(x,y);local expect=A(v)>0 and (fold[v] or v) or v
   if A(v)==0 then assert(A(now:getPixel(x,y))==0,e.key..' silhouette changed')
   else assert(now:getPixel(x,y)==expect,e.key..' is not its v18 pixels through the fold') end
  end end
  recoloured=recoloured+1
 elseif not originals[n] and is_rest(e.key) then
  assert(now:isEqual(was),e.key..' rest frame changed');unchanged_rest=unchanged_rest+1
 end
end

local glass={};for _,r in ipairs({'glass0','glass1','glass2','glass3','glass4'}) do glass[C.rgba[r]]=true end
local function full(l,f,w,h) local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position) end;return im end
local bases={}
local name,s;local docs,count,shadows,overlays=0,0,0,0
local worst={}
-- Base frames first, so overlays check against the v19 base.
table.sort(entries,function(a,b)
 local da,db=a.document:find('_damage') and 1 or 0,b.document:find('_damage') and 1 or 0
 if da~=db then return da<db end
 if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
for _,e in ipairs(entries) do
 if name~=e.document then if s then s:close() end;name=e.document;s=assert(app.open(C.SOURCE..name..'.aseprite'));docs=docs+1 end
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
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position) end end
  for y=0,e.h-1 do for x=0,e.w-1 do
   local p=shadow:getPixel(x,y)
   if A(p)>0 then assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour');assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body') end
   local bp=body:getPixel(x,y)
   if A(bp)>0 then assert(bp~=C.rgba.cast and bp~=C.rgba.contact,e.key..' body uses a shadow role') end
  end end
  if e.w==160 then
   local n=0;for y=0,e.h-1 do for x=0,e.w-1 do if A(shadow:getPixel(x,y))>0 then n=n+1 end end end
   assert(n==0,e.key..' draws a shadow beneath a building')
  end
  shadows=shadows+1
  bases[e.key]=body
  local n=C.colours({body},true)
  if not worst[e.document] or n>worst[e.document] then worst[e.document]=n end
 elseif s.layers[1].name=='dents and tears' then
  local base_key=e.key:gsub('_damage$','')
  local base=assert(bases[base_key],base_key..' base not verified before its overlay')
  local lowest=-1
  for y=0,e.h-1 do for x=0,e.w-1 do if A(base:getPixel(x,y))>0 then lowest=y end end end
  local n=0
  for y=0,e.h-1 do for x=0,e.w-1 do
   if A(frame:getPixel(x,y))>0 then
    n=n+1
    assert(y<=lowest,e.key..' draws beneath its base')
    local bp=base:getPixel(x,y)
    assert(A(bp)>0,e.key..' draws outside its base silhouette')
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
print('Colours per machine document (max over frames, shadow excluded): '..table.concat(parts,', '))
print('Verified '..count..' v19 frames in '..docs..' documents ('..shadows..' shadowed, '..overlays..' damage overlays); '..recoloured..' original-machine frames are their v18 pixels through the fold; '..unchanged_rest..' v18-role rest frames unchanged; '..kept..' archived rectangles exact, '..replaced..' re-authored in place.')
