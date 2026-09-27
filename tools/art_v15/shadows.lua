-- Root-authored v15 correction: ground the machines. The v14 shadow was a
-- projection that started at the base and spilled down and to the right,
-- clipped at the sprite edge: an offset drop shadow, which reads as a body
-- lifted above the ground. This pass puts each machine back in its own
-- contact shadow: the authored oval from the original document sits under
-- the footprint, a two-row contact strip meets the parts that touch the
-- ground, and a short flattened cast leans right with the top-left key
-- light without leaving the base. Structural layers 2-6 are untouched.
-- With `proto=1` only two machines are exported as PNGs for review.
local root=assert(app.params.root)
local proto=app.params.proto
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local CAST=C.rgba.cast
local A=app.pixelColor.rgbaA
-- Cast: 0.30 px right and 0.10 px down per pixel of height, no offset.
local KX,KY=0.30,0.10
local out=root..'/output/art-v15/shadows';os.execute('mkdir -p '..out)
local function full(layer,f,w,h) local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f);if cel then im:drawImage(cel.image,cel.position) end;return im end
local function shadow_for(bodies,oval,w,h)
 local body=Image(w,h,ColorMode.RGB)
 for _,im in ipairs(bodies) do body:drawImage(im,Point(0,0)) end
 local g=-1;local bottom={}
 for y=0,h-1 do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then if y>g then g=y end;bottom[x]=y end
 end end
 local outim=Image(w,h,ColorMode.RGB)
 if g<0 then return outim end
 local mask={}
 local function mark(x,y) if x>=0 and y>=0 and x<w and y<h then mask[y*w+x]=true end end
 -- The authored contact oval under the footprint, in the shadow colour.
 for y=0,h-1 do for x=0,w-1 do if A(oval:getPixel(x,y))>0 then mark(x,y) end end end
 -- Contact strip under the parts that reach the ground.
 for x,y in pairs(bottom) do if y>=g-3 then for oy=1,2 do mark(x,y+oy);mark(x+1,y+oy) end end end
 -- A short, flat cast: tall parts lean their shadow to the right, never far
 -- below the base, so the machine is never lifted off it.
 for y=0,g do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then
   local hgt=g-y
   local sx=x+math.floor(hgt*KX+.5);local sy=g+math.floor(hgt*KY+.5)
   mark(sx,sy);mark(sx+1,sy)
  end
 end end
 local keep={}
 for k in pairs(mask) do
  local x,y=k%w,math.floor(k/w);local n=0
  for j=-1,1 do for i=-1,1 do if (i~=0 or j~=0) and mask[(y+j)*w+(x+i)] then n=n+1 end end end
  if n>=2 then keep[k]=true end
 end
 for k in pairs(keep) do local x,y=k%w,math.floor(k/w);if A(body:getPixel(x,y))==0 then outim:drawPixel(x,y,CAST) end end
 return outim
end
local entries={};local frames_done=0
-- doc_path: the document whose structural layers are kept (v14 machine
-- documents, or the v15 idle documents). orig_path: the original document
-- carrying the authored oval on layer 1. keymap: frame -> key. orig_frame:
-- frame -> frame in the original document. save_as: v15 document name.
local function process(doc_path,orig_path,save_as,keymap,orig_frame,previous_source)
 local s=assert(app.open(root..'/'..doc_path))
 local o=assert(app.open(root..'/'..orig_path))
 assert(s.layers[1].name=='contact shadows',save_as..' layer contract')
 assert(o.layers[1].name=='contact shadows',orig_path..' layer contract')
 local w,h=s.width,s.height
 for f=1,#s.frames do
  local key=keymap[f]
  if key then
   local b=assert(m.sprites[key],key)
   local bodies={};for i=2,#s.layers do bodies[#bodies+1]=full(s.layers[i],f,w,h) end
   local oval=full(o.layers[1],orig_frame(f),w,h)
   local shadow=shadow_for(bodies,oval,w,h)
   local cel=s.layers[1]:cel(f);if cel then s:deleteCel(cel) end
   s:newCel(s.layers[1],f,shadow,Point(0,0))
   local layers={};for _,l in ipairs(s.layers) do layers[#layers+1]=l.name end
   entries[#entries+1]={key=key,document=save_as,frame=f,w=b.w,h=b.h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers,previous_source=previous_source}
   frames_done=frames_done+1
   if proto then local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f);frame:saveAs(out..'/'..key..'.png') end
  end
 end
 o:close()
 if proto then s:close() else s:saveAs(root..'/art/source/v15/'..save_as..'.aseprite');s:close() end
end
local units={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
if proto then
 for _,unit in ipairs({'hook','riveter'}) do
  local map={};for _,face in ipairs({0,3}) do map[face*5+1]=unit..'_'..face;map[face*5+2]=unit..'_'..face..'_walk_0' end
  process('art/source/v14/'..unit..'.aseprite','art/source/v7/'..unit..'.aseprite',unit,map,function(f) return f end,'art/source/v14/'..unit..'.aseprite')
  local imap={};for _,face in ipairs({0,3}) do imap[face*2+1]=unit..'_'..face..'_idle_0' end
  process('art/source/v15/'..unit..'_idle.aseprite','art/source/v7/'..unit..'.aseprite',unit..'_idle',imap,function(f) return math.floor((f-1)/2)*5+1 end,nil)
 end
 print('Prototype exported '..frames_done..' frames to '..out)
 return
end
-- Every v14 machine document: same frames and keys, oval from its original.
local v14=json.decode(C.read(root..'/tools/art_v14/sources.json'))
local docs={}
for _,e in ipairs(v14) do
 if e.previous_source then
  local d=docs[e.document] or {orig=e.previous_source,map={}};d.map[e.frame]=e.key;docs[e.document]=d
 end
end
local names={};for n in pairs(docs) do names[#names+1]=n end;table.sort(names)
for _,n in ipairs(names) do
 local d=docs[n]
 process('art/source/v14/'..n..'.aseprite',d.orig,n,d.map,function(f) return f end,'art/source/v14/'..n..'.aseprite')
end
-- The v15 idle documents: their acted bodies, the oval of the v7 idle frame.
for _,unit in ipairs(units) do
 local map={};for face=0,7 do for k=0,1 do map[face*2+k+1]=unit..'_'..face..'_idle_'..k end end
 process('art/source/v15/'..unit..'_idle.aseprite','art/source/v7/'..unit..'.aseprite',unit..'_idle',map,function(f) return math.floor((f-1)/2)*5+1 end,nil)
end
table.sort(entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
-- Idle entries stay in the idle registry; the replaced v14 documents get
-- their own registry so the exporter writes them in place.
local replaced={}
for _,e in ipairs(entries) do if e.previous_source then replaced[#replaced+1]=e end end
C.write(root..'/tools/art_v15/sources-shadows.json',json.encode(replaced))
print('Grounded '..frames_done..' machine frames; '..#replaced..' replaced v14 rectangles registered.')
