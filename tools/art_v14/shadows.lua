-- Root-authored v14 correction: replace every machine's flat oval contact
-- shadow with a footprint projection of its own silhouette toward the lower
-- right (top-left key light), plus a short contact strip under its lowest
-- pixels. Structural layers 2-6 are untouched; only 'contact shadows' changes.
-- Editable documents are saved under art/source/v14/. Export separately.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
local m=json.decode(C.read(root..'/art/archive/art-v13/game-assets.json'))
local atlas=Image{fromFile=root..'/art/archive/art-v13/game-assets.png'}
local CAST=C.rgba.cast
local KX,KY,DX,DY=0.36,0.20,1,1
local entries={};local frames_done=0
local function full(layer,f,w,h) local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f);if cel then im:drawImage(cel.image,cel.position) end;return im end
local function shadow_for(s,f)
 local w,h=s.width,s.height
 local body=Image(w,h,ColorMode.RGB)
 for i=2,#s.layers do local cel=s.layers[i]:cel(f);if cel then body:drawImage(cel.image,cel.position) end end
 local A=app.pixelColor.rgbaA
 local g=-1;local bottom={}
 for y=0,h-1 do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then if y>g then g=y end;bottom[x]=y end
 end end
 local out=Image(w,h,ColorMode.RGB)
 if g<0 then return out end
 local mask={}
 local function mark(x,y) if x>=0 and y>=0 and x<w and y<h then mask[y*w+x]=true end end
 for y=0,g do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then
   local hgt=g-y
   local sx=x+DX+math.floor(hgt*KX+.5);local sy=g+DY+math.floor(hgt*KY+.5)
   mark(sx,sy);mark(sx+1,sy)
  end
 end end
 -- The contact strip belongs only to parts that actually reach the ground;
 -- an overhanging crane or shield must not carry a floating strip.
 for x,y in pairs(bottom) do if y>=g-3 then for oy=1,2 do mark(x,y+oy);mark(x+1,y+oy) end end end
 -- Orphan cleanup: a shadow pixel needs two neighbours in the shadow.
 local keep={}
 for k in pairs(mask) do
  local x,y=k%w,math.floor(k/w);local n=0
  for j=-1,1 do for i=-1,1 do if (i~=0 or j~=0) and mask[(y+j)*w+(x+i)] then n=n+1 end end end
  if n>=2 then keep[k]=true end
 end
 for k in pairs(keep) do local x,y=k%w,math.floor(k/w);if A(body:getPixel(x,y))==0 then out:drawPixel(x,y,CAST) end end
 return out
end
local function process(doc_path,name,keymap)
 local s=assert(app.open(root..'/'..doc_path))
 assert(s.layers[1].name=='contact shadows',name..' layer contract')
 for f=1,#s.frames do
  local key=keymap[f]
  if key then
   local b=assert(m.sprites[key],key)
   local before=Image(s.width,s.height,ColorMode.RGB);before:drawSprite(s,f)
   assert(before:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),key..' source differs from the v13 atlas')
   local shadow=shadow_for(s,f)
   local cel=s.layers[1]:cel(f);if cel then s:deleteCel(cel) end
   s:newCel(s.layers[1],f,shadow,Point(0,0))
   local layers={};for _,l in ipairs(s.layers) do layers[#layers+1]=l.name end
   entries[#entries+1]={key=key,document=name,frame=f,w=b.w,h=b.h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers,previous_source=doc_path,muzzle=b.muzzle}
   frames_done=frames_done+1
  end
 end
 s:saveAs(root..'/art/source/v14/'..name..'.aseprite');s:close()
end
-- Idle and walk frames live in the v7 unit documents: frame = face*5 + row + 1.
for _,unit in ipairs({'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}) do
 local map={}
 for face=0,7 do map[face*5+1]=unit..'_'..face;for row=1,4 do map[face*5+row+1]=unit..'_'..face..'_walk_'..(row-1) end end
 process('art/source/v7/'..unit..'.aseprite',unit,map)
end
-- State frames live in the v8 documents listed in its registry.
local v8=json.decode(C.read(root..'/tools/art_v8/sources.json'))
local wanted={}
for _,d in ipairs({'bulwark_deployment','loom_deployment','bulwark_deployed_firing','loom_deployed_firing','riveter_firing','bulwark_firing','sounder_firing','skipper_firing','reedguard_firing','loom_firing','hook_gather','hook_loaded','hook_loaded_walk','hook_unload','wick_gather','wick_loaded','wick_loaded_walk','wick_unload'}) do wanted[d]={} end
for _,e in ipairs(v8) do if wanted[e.document] then wanted[e.document][e.frame]=e.key end end
for doc,map in pairs(wanted) do process('art/source/v8/'..doc..'.aseprite',doc,map) end
-- The Loom's committed-shot performance lives in the v9 registry.
local v9=json.decode(C.read(root..'/tools/art_v9/sources.json'))
local loom={}
for _,e in ipairs(v9) do if e.document=='loom_pressure' then loom[e.frame]=e.key end end
process('art/source/v9/loom_pressure.aseprite','loom_pressure',loom)
table.sort(entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
C.write(root..'/tools/art_v14/sources-machines.json',json.encode(entries))
print('Rebuilt contact shadows for '..frames_done..' machine frames in '..#entries..' registry entries.')
