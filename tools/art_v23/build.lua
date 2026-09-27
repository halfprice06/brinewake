-- Art v23 entry point: the Saltglass Compact's missing pieces.
--
--   aseprite -b --script-param root=$PWD --script-param pieces=PIECES.json \
--     --script tools/art_v23/build.lua
--
-- PIECES.json comes from `python3 tools/art_v23/pieces.py PIECES.json`: the
-- hand-placed pixel rows for the doctrine plates and headquarters annexes,
-- the glaze trace and the GLINT effects. This script builds each as a
-- layered document under art/source/v23/ and adds the Raker's Hauling
-- panniers (cradle.lua, painter construction). Every frame is painted into
-- the game atlas at a free 16-pixel cell by the v21 packer: no earlier
-- rectangle or pixel moves, and a rebuild reuses a key's rectangle.
-- Writes tools/art_v23/sources.json and review PNGs under output/art-v23/.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local cradle=dofile(root..'/tools/art_v23/cradle.lua')
local painter=C.painter
local A=app.pixelColor.rgbaA
local SOURCE=root..'/art/source/v23/'
local OUT=root..'/output/art-v23'
os.execute('mkdir -p '..SOURCE..' '..OUT)
local atlas,m,packer=C.open_atlas()
local entries={}

local function rgba(hex)
  return app.pixelColor.rgba(tonumber(hex:sub(1,2),16),tonumber(hex:sub(3,4),16),tonumber(hex:sub(5,6),16),255)
end

-- The painter's palette plus any colour a document uses that it lacks.
local function palette_for(s,extra)
  C.install_palette(s)
  local pal=s.palettes[1]
  local have={}
  for i=0,#pal-1 do have[pal:getColor(i).rgbaPixel]=true end
  for _,v in ipairs(extra or {}) do
    if not have[v] then pal:resize(#pal+1);pal:setColor(#pal-1,Color(v));have[v]=true end
  end
end

-- Save a layered document and paint its keyed frames into the atlas.
-- `frames` are {key=, ms=, images={one per layer}}; `tags` are
-- {name, first, last}.
local function document(name,w,h,ax,ay,layers,frames,tags,extra)
  local s=Sprite(w,h,ColorMode.RGB);palette_for(s,extra)
  for i,n in ipairs(layers) do local l=i==1 and s.layers[1] or s:newLayer();l.name=n end
  for f,spec in ipairs(frames) do
    if f>1 then s:newEmptyFrame(f) end
    s.frames[f].duration=spec.ms/1000
    for i=1,#layers do s:newCel(s.layers[i],f,assert(spec.images[i],name..' layer '..i),Point(0,0)) end
    local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f)
    local b=packer:alloc(spec.key,w,h,ax,ay)
    C.blit(atlas,b,frame)
    frame:saveAs(OUT..'/'..spec.key..'.png')
    entries[#entries+1]={key=spec.key,document='art/source/v23/'..name..'.aseprite',frame=f,
      w=w,h=h,anchor_x=ax,anchor_y=ay,duration_ms=spec.ms,layers=layers}
  end
  for _,t in ipairs(tags) do s:newTag(t[2],t[3]).name=t[1] end
  s:saveAs(SOURCE..name..'.aseprite');s:close()
end

------------------------------------------------------------------------------
-- The hand-placed pieces.
local P=json.decode(C.read(assert(app.params.pieces,'pieces required')))
local colour={}
for ch,role in pairs(P.roles) do colour[ch]=rgba(assert(P.hex[role],role)) end
local extra={};for _,v in pairs(colour) do extra[#extra+1]=v end
local count=0
for _,d in ipairs(P.documents) do
  local frames={}
  for _,f in ipairs(d.frames) do
    local images={}
    for i,layer in ipairs(d.layers) do
      local im=Image(d.w,d.h,ColorMode.RGB);im:clear()
      for y,row in ipairs(f.cels[layer]) do
        for x=1,#row do
          local ch=row:sub(x,x)
          if ch~='.' then im:drawPixel(x-1,y-1,assert(colour[ch],ch)) end
        end
      end
      images[i]=im
    end
    frames[#frames+1]={key=f.key,ms=f.ms,images=images}
  end
  document(d.name,d.w,d.h,d.anchor[1],d.anchor[2],d.layers,frames,{{d.tag,1,#frames}},extra)
  count=count+#frames
end

------------------------------------------------------------------------------
-- The Raker's Hauling panniers, eight facings of four frames.
local W,H,AX,AY=64,64,32,50
local cast,contact=C.rgba.cast,C.rgba.contact
-- Everything the Raker covers on a facing, in any pose it can be seen in
-- with the kit on (the doctrine key is drawn over every one of them).
local POSES={'','_walk_0','_walk_1','_walk_2','_walk_3','_idle_0','_idle_1','_loaded',
  '_loaded_walk_0','_loaded_walk_1','_loaded_walk_2','_loaded_walk_3',
  '_gather_0','_gather_1','_gather_2','_unload_0','_unload_1','_unload_2'}
local function raker_cover(face)
  local cover={}
  for _,suffix in ipairs(POSES) do
    local b=assert(m.sprites['raker_'..face..suffix],'raker_'..face..suffix)
    assert(b.w==W and b.h==H and b.anchor_x==AX and b.anchor_y==AY)
    for y=0,H-1 do for x=0,W-1 do
      local v=atlas:getPixel(b.x+x,b.y+y)
      if A(v)>0 and v~=cast and v~=contact then cover[y*W+x]=true end
    end end
  end
  return cover
end
local frames,tags={},{}
for face=0,7 do
  local cover=raker_cover(face)
  for k=0,3 do
    local ims={}
    for _,l in ipairs(painter.layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[l]=im end
    local c=painter.new(ims,{key='cradle',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=M.BUDGET},face)
    cradle.paint(c,M.depth,k)
    local far=ims[painter.layers[2]]
    local masked=Image(W,H,ColorMode.RGB);masked:clear()
    for y=0,H-1 do for x=0,W-1 do
      local v=far:getPixel(x,y)
      if A(v)>0 and not cover[y*W+x] then masked:drawPixel(x,y,v) end
    end end
    frames[#frames+1]={key='doctrine_raker_'..face..'_hauling_'..k,ms=200,
      images={masked,ims[painter.layers[5]]}}
  end
  tags[#tags+1]={'face '..face,face*4+1,face*4+4}
end
document('raker_hauling_panniers',W,H,AX,AY,{'far pannier (cut by the raker)','near pannier'},frames,tags)
count=count+#frames

-- The manifest in the layout the earlier passes' Python writes
-- (json.dump(..., sort_keys=True)), so the diff shows only the new keys.
local function encode(v)
  if type(v)=='table' then
    if #v>0 then
      local parts={};for _,x in ipairs(v) do parts[#parts+1]=encode(x) end
      return '['..table.concat(parts,', ')..']'
    end
    local keys={};for k in pairs(v) do keys[#keys+1]=k end;table.sort(keys)
    local parts={};for _,k in ipairs(keys) do parts[#parts+1]='"'..k..'": '..encode(v[k]) end
    return '{'..table.concat(parts,', ')..'}'
  elseif type(v)=='number' then return string.format('%d',v)
  elseif type(v)=='string' then return '"'..v..'"' end
  error('cannot encode '..type(v))
end

table.sort(entries,function(a,b) return a.key<b.key end)
C.write(root..'/tools/art_v23/sources.json',json.encode(entries))
atlas:saveAs(C.ATLAS)
C.write(C.MANIFEST,encode(m))
print('Authored '..count..' frames.')
