-- Root-only Aseprite correction: preserve structural art and animation,
-- remove underlying shadow/soil patches, retain actual fitted foundations.
local root=assert(app.params.root)
local painter=dofile(root..'/tools/art_v5/painter.lua')
local repairs=dofile(root..'/tools/art_v13/footings.lua')
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s)local f=assert(io.open(p,'w'));f:write(s);f:close()end
local m=json.decode(read(root..'/art/archive/art-v12/game-assets.json'))
local atlas=Image{fromFile=root..'/art/archive/art-v12/game-assets.png'}
local sources={};local delta_counts={}
local function full(layer,f,w,h)
 local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f)
 if cel then im:drawImage(cel.image,cel.position)end;return im
end
local function emit(s,name,keys,path)
 local layers={};for _,l in ipairs(s.layers)do layers[#layers+1]=l.name end
 for f,key in ipairs(keys)do
  local b=assert(m.sprites[key]);local im=Image(s.width,s.height,ColorMode.RGB);im:drawSprite(s,f)
  im:saveAs(root..'/output/art-v13/'..key..'.png')
  sources[#sources+1]={key=key,document=name,frame=f,w=b.w,h=b.h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers,previous_source=path}
 end
 s:saveAs(root..'/art/source/v13/'..name..'.aseprite');s:close()
end
local function keys(base,suffix,n)local k={};for p=0,n-1 do k[#k+1]=base..suffix..p end;return k end
local activities={union_hq=true,assembly_hq=true,union_works=true,assembly_works=true,condenser=true}
local construction={union_works=true,assembly_works=true,condenser=true,dropoff=true,tower=true}
for _,name in ipairs({'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','well'})do
 local old=assert(app.open(root..'/art/source/v6/'..name..'.aseprite'))
 local original=assert(app.open(root..'/art/archive/art-v5/sources/'..name..'.aseprite'))
 local ims={};for i,l in ipairs(original.layers)do ims[l.name]=i==1 and Image(160,144,ColorMode.RGB)or full(l,1,160,144)end
 local c=painter.new(ims,{key=name,x=0,y=0,w=160,h=144,anchor_x=80,anchor_y=128},0)
 repairs[name](c)
 local deltas={};local changed=0
 for i,l in ipairs(old.layers)do
  local previous=full(l,1,160,144);local replacement=ims[l.name];deltas[i]={}
  for y=0,143 do for x=0,159 do
   local a,b=previous:getPixel(x,y),replacement:getPixel(x,y)
   if a~=b then deltas[i][#deltas[i]+1]={x=x,y=y,a=a,b=b};changed=changed+1 end
  end end
 end
 delta_counts[name]=changed
 local jobs={{doc=name,keys={name}}}
 if activities[name]then jobs[#jobs+1]={doc=name..'_activity',keys=keys(name,'_active_',4)}end
 if construction[name]then jobs[#jobs+1]={doc=name..'_construction',keys=keys(name,'_build_',3)}end
 for _,job in ipairs(jobs)do
  local path='art/source/v7/'..job.doc..'.aseprite';local s=assert(app.open(root..'/'..path))
  assert(#s.frames==#job.keys)
  for f,key in ipairs(job.keys)do
   local current=Image(160,144,ColorMode.RGB);current:drawSprite(s,f);local b=m.sprites[key]
   assert(current:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),key..' source differs from task-entry atlas')
   for i,l in ipairs(s.layers)do
    local im=full(l,f,160,144)
    for _,d in ipairs(deltas[i])do
     -- Only carry the base correction into pixels still owned by the old base.
     -- Frame-specific machinery, flags, scaffolds and their timing stay intact.
     if im:getPixel(d.x,d.y)==d.a then im:drawPixel(d.x,d.y,d.b)end
    end
    local cel=l:cel(f);if cel then s:deleteCel(cel)end;s:newCel(l,f,im,Point(0,0))
   end
  end
  s.layers[1].name='fitted masonry foundation'
  emit(s,job.doc,job.keys,path)
 end
 old:close();original:close()
end
-- These documents contain a dedicated ground-shadow layer below structural
-- masonry. Removing it leaves every pier, step and instrument unchanged.
local registry=json.decode(read(root..'/tools/art_v8/sources.json'))
local grouped={}
for _,e in ipairs(registry)do
 if e.document=='gate_north_dry'or e.document=='gate_south_dry'or e.document=='landmark_tide_gauge'or e.document=='landmark_ferry_stairs'then
  grouped[e.document]=grouped[e.document]or{};grouped[e.document][e.frame]=e.key
 end
end
for _,name in ipairs({'gate_north_dry','gate_south_dry','landmark_tide_gauge','landmark_ferry_stairs'})do
 local path='art/source/v8/'..name..'.aseprite';local s=assert(app.open(root..'/'..path));local ks=assert(grouped[name])
 for f,key in ipairs(ks)do
  local b=m.sprites[key];local im=Image(s.width,s.height,ColorMode.RGB);im:drawSprite(s,f)
  assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),key..' source differs from current atlas')
  local cel=s.layers[1]:cel(f);if cel then s:deleteCel(cel)end
  if name:sub(1,5)=='gate_'then
   -- These detached dark diamonds and strokes sat on the removed shadow,
   -- outside the masonry slab. Keep the actual slab and pier faces intact.
   local finish=s.layers[6]:cel(f);if finish then s:deleteCel(finish)end
  end
 end
 s.layers[1].name='ground shadows forbidden - keep empty'
 emit(s,name,ks,path)
end
write(root..'/tools/art_v13/sources.json',json.encode(sources))
write(root..'/output/art-v13/base-deltas.json',json.encode(delta_counts))
print('Corrected '..#sources..' fixed-structure frames in editable v13 sources.')
