-- Root-authored v15 wind frames for the three bank plants. Each frame is the
-- v12 shore_plants drawing sheared by height: pixels near the anchor stay,
-- pixels at the crown move downwind by one or two pixels, so a gust reads as
-- a lean rather than a slide. Root contact is never moved. The rest frame is
-- the existing atlas entry; only the two lean frames are new.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/plants';os.execute('mkdir -p '..out)
local src=assert(app.open(root..'/art/source/v12/shore_plants.aseprite'))
local W,H,AX,AY=src.width,src.height,32,50
local layers={};for _,l in ipairs(src.layers) do layers[#layers+1]=l.name end
local plants={'reed_clump','marsh_grass','salt_bush'}
local A=app.pixelColor.rgbaA
local function full(layer,f) local im=Image(W,H,ColorMode.RGB);local cel=layer:cel(f);if cel then im:drawImage(cel.image,cel.position) end;return im end
local function crown(f)
 -- Highest drawn row across the moving layers: the shear scales to it.
 local top=H
 for i=2,#src.layers do local im=full(src.layers[i],f)
  for y=0,H-1 do for x=0,W-1 do if A(im:getPixel(x,y))>0 and y<top then top=y end end end
 end
 return top
end
local frames={}
for p,plant in ipairs(plants) do
 local top=crown(p);local span=math.max(AY-top,1)
 for lean=1,2 do
  local images={}
  for i=1,#src.layers do
   local base=full(src.layers[i],p)
   if i==1 then images[i]=base -- root contact stays planted
   else
    local im=Image(W,H,ColorMode.RGB)
    for y=0,H-1 do
     local h=AY-y
     local dx=0
     if h>0 then dx=math.floor(lean*h/span+.5) end
     -- Marsh grass is the lightest plant: it bends most at the tips.
     if plant=='marsh_grass' and h>span*0.6 then dx=dx+ (lean==2 and 1 or 0) end
     for x=0,W-1 do local v=base:getPixel(x,y);if A(v)>0 and x+dx>=0 and x+dx<W then im:drawPixel(x+dx,y,v) end end
    end
    images[i]=im
   end
  end
  frames[#frames+1]={key=plant..'_sway_'..lean,duration=.2,images=images}
 end
end
src:close()
C.document_images('shore_plants_sway',W,H,AX,AY,layers,frames,'shore_plants_sway',m,packer,out,entries,'art/source/v12/shore_plants.aseprite')
C.write(root..'/tools/art_v15/sources-plants.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' plant sway frames; manifest now has '..C.count(m)..' entries.')
