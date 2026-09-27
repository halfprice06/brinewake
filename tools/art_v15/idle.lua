-- Root-authored v15 idle acting: two acted frames per machine facing, built
-- from the v7 idle drawings. Union machines settle: the cabin and working
-- parts dip one pixel on their suspension and a vent puff appears above the
-- cabin, then the body returns while the puff rises and thins. Assembly
-- machines breathe: the working parts flex up one pixel while the pressure
-- chamber brightens one step, then settle while the glow peaks. Layers stay
-- named and separate; shadows are reprojected by the v14 method.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/idle';os.execute('mkdir -p '..out)
local A=app.pixelColor.rgbaA
local CAST=C.rgba.cast
local KX,KY,DX,DY=0.36,0.20,1,1
local function full(layer,f,w,h) local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f);if cel then im:drawImage(cel.image,cel.position) end;return im end
local function shifted(im,dx,dy)
 local out=Image(im.width,im.height,ColorMode.RGB)
 for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y)
  if A(v)>0 and x+dx>=0 and y+dy>=0 and x+dx<im.width and y+dy<im.height then out:drawPixel(x+dx,y+dy,v) end
 end end
 return out
end
local function recolour(im,map)
 local out=Image(im.width,im.height,ColorMode.RGB)
 for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y)
  if A(v)>0 then out:drawPixel(x,y,map[v] or v) end
 end end
 return out
end
-- The v14 projection: silhouette of layers 2-6 cast toward the lower right
-- plus a contact strip under parts that reach the ground.
local function shadow_for(images,w,h)
 local body=Image(w,h,ColorMode.RGB)
 for i=2,#images do body:drawImage(images[i],Point(0,0)) end
 local g=-1;local bottom={}
 for y=0,h-1 do for x=0,w-1 do if A(body:getPixel(x,y))>0 then if y>g then g=y end;bottom[x]=y end end end
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
 for x,y in pairs(bottom) do if y>=g-3 then for oy=1,2 do mark(x,y+oy);mark(x+1,y+oy) end end end
 local keep={}
 for k in pairs(mask) do
  local x,y=k%w,math.floor(k/w);local n=0
  for j=-1,1 do for i=-1,1 do if (i~=0 or j~=0) and mask[(y+j)*w+(x+i)] then n=n+1 end end end
  if n>=2 then keep[k]=true end
 end
 for k in pairs(keep) do local x,y=k%w,math.floor(k/w);if A(body:getPixel(x,y))==0 then out:drawPixel(x,y,CAST) end end
 return out
end
local function cabin_top(im)
 for y=0,im.height-1 do
  local xs={}
  for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then xs[#xs+1]=x end end
  if #xs>0 then return xs[math.ceil(#xs/2)],y end
 end
 return nil
end
local union={hook=true,riveter=true,bulwark=true,sounder=true}
local glow1={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3}
local glow2={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3,[C.rgba.amber3]=C.rgba.amber4}
for _,unit in ipairs({'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}) do
 local src=assert(app.open(root..'/art/source/v7/'..unit..'.aseprite'))
 local W,H=src.width,src.height
 local layers={};for _,l in ipairs(src.layers) do layers[#layers+1]=l.name end
 local frames={}
 for face=0,7 do
  local f=face*5+1
  local base={};for i=1,#src.layers do base[i]=full(src.layers[i],f,W,H) end
  for k=0,1 do
   local images={}
   if union[unit] then
    -- Frame 0: cabin, working parts and finish dip one pixel; small puff.
    -- Frame 1: body level again; the puff has risen and thinned.
    local dip=(k==0) and 1 or 0
    images[2]=Image(base[2]);images[3]=Image(base[3])
    for i=4,6 do images[i]=shifted(base[i],0,dip) end
    local cx,cy=cabin_top(base[4])
    if cx then
     local finish=images[6]
     local function puff(x,y,col) if x>=0 and y>=0 and x<W and y<H and A(finish:getPixel(x,y))==0 then finish:drawPixel(x,y,C.rgba[col]) end end
     if k==0 then puff(cx+2,cy-2,'steel3');puff(cx+3,cy-2,'steel3');puff(cx+3,cy-3,'steel4')
     else puff(cx+3,cy-5,'steel3');puff(cx+5,cy-6,'steel2');puff(cx+4,cy-7,'steel3') end
    end
   else
    -- Frame 0: working parts flex up one pixel, chamber brightens a step.
    -- Frame 1: parts settle, the glow peaks.
    images[2]=Image(base[2]);images[3]=Image(base[3])
    images[4]=recolour(base[4],(k==0) and glow1 or glow2)
    images[5]=(k==0) and shifted(recolour(base[5],glow1),0,-1) or recolour(base[5],glow2)
    images[6]=Image(base[6])
   end
   images[1]=shadow_for(images,W,H)
   frames[#frames+1]={key=unit..'_'..face..'_idle_'..k,duration=.2,images=images}
  end
 end
 src:close()
 C.document_images(unit..'_idle',W,H,32,50,layers,frames,'idle_acting',m,packer,out,entries)
end
C.write(root..'/tools/art_v15/sources-idle.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' idle acting frames; manifest now has '..C.count(m)..' entries.')
