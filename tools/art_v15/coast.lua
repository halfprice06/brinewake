-- Root-authored v15 living coast: the bird site's take-off strip and the crab
-- site's hide strip. Each frame keeps the site's bank, stems and accents from
-- the v9 documents and replaces only the animal layer: gulls lift off and
-- drift downwind over four frames; the crab scuttles under the stones and
-- leaves the site still.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/coast';os.execute('mkdir -p '..out)
local A=app.pixelColor.rgbaA
local function full(layer,f,w,h) local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f);if cel then im:drawImage(cel.image,cel.position) end;return im end
local animal_colours={}
for _,r in ipairs({'ivory2','ivory3','ivory4','steel1','steel2','steel3','steel4','rust0','rust1','rust2','rust3','ink','contact'}) do animal_colours[C.rgba[r]]=true end
local function without_animals(im)
 local out=Image(im.width,im.height,ColorMode.RGB)
 for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y)
  if A(v)>0 and not animal_colours[v] then out:drawPixel(x,y,v) end
 end end
 return out
end
local function bounds(im)
 local x0,y0,x1,y1=im.width,im.height,-1,-1
 for y=0,im.height-1 do for x=0,im.width-1 do if A(im:getPixel(x,y))>0 and animal_colours[im:getPixel(x,y)] then
  if x<x0 then x0=x end;if y<y0 then y0=y end;if x>x1 then x1=x end;if y>y1 then y1=y end end end end
 return x0,y0,x1,y1
end
-- Birds --------------------------------------------------------------------
local src=assert(app.open(root..'/art/source/v9/coast_birds.aseprite'))
local W,H=src.width,src.height
local layers={};for _,l in ipairs(src.layers) do layers[#layers+1]=l.name end
local base={};for i=1,#src.layers do base[i]=full(src.layers[i],1,W,H) end
local bx0,by0,bx1,by1=bounds(base[5])
src:close()
local function gull(im,x,y,wings,tone)
 local function p(dx,dy,col) if x+dx>=0 and y+dy>=0 and x+dx<W and y+dy<H then im:drawPixel(x+dx,y+dy,C.rgba[col]) end end
 p(0,0,tone);p(1,0,tone)
 if wings=='up' then p(-1,-1,'steel2');p(-2,-2,'steel3');p(2,-1,'steel2');p(3,-2,'steel3')
 elseif wings=='level' then p(-1,0,'steel3');p(-2,0,'steel2');p(2,0,'steel3');p(3,0,'steel2')
 else p(-1,1,'steel2');p(-2,1,'steel3');p(2,1,'steel2');p(3,1,'steel3') end
end
local frames={}
local function gulls(im,k)
 -- Two gulls rise from where they stood and drift right with the wind.
 local ox,oy=(bx0+bx1)//2,by0
 local lift=k*6;local drift=k*5
 gull(im,ox-6+drift,oy-2-lift,({'up','level','down','level'})[k+1],'ivory3')
 gull(im,ox+8+drift+k,oy-lift+2-(k*2),({'level','down','up','level'})[k+1],'ivory4')
 if k>=2 then gull(im,ox+2+drift*2,oy-lift-6,(k==2) and 'up' or 'level','ivory3') end
end
for k=0,4 do
 local images={}
 for i=1,#layers do images[i]=(i==5) and without_animals(base[5]) or Image(base[i]) end
 -- Frame 4 is the site after the birds have gone.
 if k<4 then gulls(images[5],k) end
 frames[#frames+1]={key='coast_birds_fly_'..k,duration=.2,images=images}
end
C.document_images('coast_birds_fly',W,H,48,52,layers,frames,'fly',m,packer,out,entries,'art/source/v9/coast_birds.aseprite')
-- Gulls alone, for the home page flight over the vignette.
local home={}
for k=0,3 do
 local im=Image(W,H,ColorMode.RGB);gulls(im,k)
 home[#home+1]={key='home_birds_'..k,duration=.2,images={im}}
end
C.document_images('home_birds',W,H,48,52,{'gulls'},home,'flight',m,packer,out,entries)
-- Crab ---------------------------------------------------------------------
src=assert(app.open(root..'/art/source/v9/coast_crab.aseprite'))
layers={};for _,l in ipairs(src.layers) do layers[#layers+1]=l.name end
base={};for i=1,#src.layers do base[i]=full(src.layers[i],1,W,H) end
local cx0,cy0,cx1,cy1=bounds(base[5])
src:close()
frames={}
for k=0,1 do
 local images={}
 for i=1,#layers do images[i]=(i==5) and without_animals(base[5]) or Image(base[i]) end
 local im=images[5]
 if k==0 then
  -- The crab has run half under the stones: only its rear legs and claws show.
  for y=cy0,cy1 do for x=cx0,cx1 do local v=base[5]:getPixel(x,y)
   if A(v)>0 and animal_colours[v] and x+7<W and (x-cx0)>=(cx1-cx0)*0.5 then im:drawPixel(x+7,y+1,v) end
  end end
 else
  -- Gone. Two claw tips remain at the stone edge.
  im:drawPixel(math.min(cx1+8,W-1),cy1,C.rgba.rust2);im:drawPixel(math.min(cx1+10,W-1),cy1-1,C.rgba.rust1)
 end
 frames[#frames+1]={key='coast_crab_hide_'..k,duration=.2,images=images}
end
C.document_images('coast_crab_hide',W,H,48,52,layers,frames,'hide',m,packer,out,entries,'art/source/v9/coast_crab.aseprite')
C.write(root..'/tools/art_v15/sources-coast.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' coast frames; manifest now has '..C.count(m)..' entries.')
