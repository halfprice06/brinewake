-- Root-authored v15 damage overlays in each faction's material. Union steel
-- dents, loses a plate and streaks rust; Assembly weave frays, creases and
-- tears. Every mark is computed on the base drawing's own silhouette and
-- drawn on a transparent overlay, so the base frames stay exact. Marks stay
-- on the building's surfaces above its lowest body row: nothing is ever
-- drawn beneath a building. Two tiers for buildings, one for machines.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/damage';os.execute('mkdir -p '..out)
local A=app.pixelColor.rgbaA
local layers={'dents and tears','streaks and strands','soot'}
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end
local steelish={}
for _,r in ipairs({'steel2','steel3','steel4','ivory2','ivory3','ivory4','rust2','rust3','rust4','glass2','glass3'}) do steelish[C.rgba[r]]=true end
local weave={}
for _,r in ipairs({'jade2','jade3','jade4','reed2','reed3','reed4','ivory2','ivory3'}) do weave[C.rgba[r]]=true end
-- Collect body pixels with their neighbourhood so marks can sit on lit
-- surfaces and strands can hang from lower edges.
local function analyse(im)
 local w,h=im.width,im.height
 local body={};local lowest=-1;local top=h
 for y=0,h-1 do for x=0,w-1 do if A(im:getPixel(x,y))>0 then body[#body+1]={x,y};if y>lowest then lowest=y end;if y<top then top=y end end end end
 local function opaque(x,y) return x>=0 and y>=0 and x<w and y<h and A(im:getPixel(x,y))>0 end
 return body,lowest,top,opaque
end
local function marks(c,im,material,tier,seed)
 local body,lowest,top,opaque=analyse(im)
 if #body==0 then return end
 local w,h=im.width,im.height
 local floor_limit=lowest-math.floor((lowest-top)*0.18) -- keep the foundation courses clean
 local count=(tier==1) and 9 or 16
 if w<100 then count=(tier==1) and 4 or 6 end
 local placed=0;local tries=0
 c.clip=nil
 while placed<count and tries<4000 do
  tries=tries+1
  local p=body[1+hash(tries,seed,tier)%#body]
  local x,y=p[1],p[2]
  local v=im:getPixel(x,y)
  if y<floor_limit and y>top+2 then
   if material=='steel' and steelish[v] then
    -- A dent: a dark pit with a bright lip catching the key light.
    c:layer(1)
    c:raw(x-c.ox,y-c.oy,C.rgba.steel0);if opaque(x+1,y) then c:raw(x+1-c.ox,y-c.oy,C.rgba.ink) end;if opaque(x,y+1) then c:raw(x-c.ox,y+1-c.oy,C.rgba.steel1) end
    if opaque(x-1,y-1) then c:raw(x-1-c.ox,y-1-c.oy,C.rgba.steel4) end
    -- A rust streak runs down the surface from the dent.
    c:layer(2)
    local len=4+hash(x,y,seed)%6
    for d=1,len do if opaque(x,y+1+d) and y+1+d<floor_limit then c:raw(x-c.ox,y+1+d-c.oy,C.rgba[(d%3==0) and 'rust0' or 'rust1']);if d<=2 and opaque(x+1,y+1+d) then c:raw(x+1-c.ox,y+1+d-c.oy,C.rgba.rust0) end end end
    placed=placed+1
   elseif material=='weave' and weave[v] then
    -- A crease in the weave with a frayed strand hanging from it.
    c:layer(1)
    c:raw(x-c.ox,y-c.oy,C.rgba.jade0);if opaque(x+1,y) then c:raw(x+1-c.ox,y-c.oy,C.rgba.jade0) end;if opaque(x+2,y+1) then c:raw(x+2-c.ox,y+1-c.oy,C.rgba.jade1) end
    c:layer(2)
    local len=3+hash(x,y,seed)%5
    for d=1,len do
     local yy=y+1+d
     local xx=x+((d%2==0) and 1 or 0)
     if yy<h and (opaque(xx,yy) or opaque(xx,yy-1)) then c:raw(xx-c.ox,yy-c.oy,C.rgba[(d==len) and 'reed1' or 'reed0']) end
    end
    placed=placed+1
   end
  end
 end
 if tier==2 then
  -- A lost plate or a torn opening, sited on a lit surface near the middle.
  local best;local bt=0
  for i=1,#body,7 do
   local p=body[i];local x,y=p[1],p[2]
   if y<floor_limit-6 and y>top+6 and ((material=='steel' and steelish[im:getPixel(x,y)]) or (material=='weave' and weave[im:getPixel(x,y)])) then
    local ok=true
    for j=0,4 do for i2=0,5 do if not opaque(x+i2,y+j) then ok=false end end end
    if ok then local score=hash(x,y,seed+9)%1000;if score>bt then bt=score;best={x,y} end end
   end
  end
  if best then
   local x,y=best[1],best[2]
   c:layer(1)
   for j=0,4 do for i2=0,5 do c:raw(x+i2-c.ox,y+j-c.oy,C.rgba[(material=='steel') and 'ink' or 'jade0']) end end
   for i2=0,5 do c:raw(x+i2-c.ox,y-1-c.oy,C.rgba[(material=='steel') and 'steel1' or 'jade1']) end
   for j=0,4 do c:raw(x+6-c.ox,y+j-c.oy,C.rgba[(material=='steel') and 'steel0' or 'jade1']) end
   c:raw(x-1-c.ox,y-c.oy,C.rgba[(material=='steel') and 'steel4' or 'jade3'])
   if material=='weave' then c:layer(2);for d=1,3 do if opaque(x+1,y+5+d) then c:raw(x+1-c.ox,y+5+d-c.oy,C.rgba.reed1) end;if opaque(x+3,y+5+d) then c:raw(x+3-c.ox,y+5+d-c.oy,C.rgba.reed0) end end end
  end
  -- Soot above the damage where the pressure vessel has vented.
  c:layer(3)
  local sx,sy=(best and best[1] or body[1][1]),(best and best[2] or body[1][2])
  for d=1,6 do local yy=sy-2-d;local xx=sx+2+(d%2);if opaque(xx,yy) and yy>top then c:raw(xx-c.ox,yy-c.oy,C.rgba.contact) end end
 end
end
local buildings={
 {'union_hq','steel'},{'union_works','steel'},{'assembly_hq','weave'},{'assembly_works','weave'},
 {'condenser','steel'},{'dropoff','steel'},{'tower','steel'},
}
for _,b in ipairs(buildings) do
 local key,material=b[1],b[2]
 local base,bb=C.sprite_image(m,key)
 local frames={}
 for tier=1,2 do
  frames[#frames+1]={key=key..'_damage_'..tier,duration=1,draw=function(c) marks(c,base,material,tier,#key) end}
 end
 C.document(key..'_damage',bb.w,bb.h,bb.anchor_x,bb.anchor_y,layers,frames,'damage_tiers',m,packer,out,entries)
end
local machines={
 {'hook','steel'},{'riveter','steel'},{'bulwark','steel'},{'sounder','steel'},
 {'wick','weave'},{'skipper','weave'},{'reedguard','weave'},{'loom','weave'},
}
for _,u in ipairs(machines) do
 local role,material=u[1],u[2]
 local frames={}
 for face=0,7 do
  local base=C.sprite_image(m,role..'_'..face)
  frames[#frames+1]={key=role..'_'..face..'_damage',duration=1,draw=function(c) marks(c,base,material,1,face+#role*3) end}
 end
 C.document(role..'_damage',64,64,32,50,layers,frames,'damage_facings',m,packer,out,entries)
end
C.write(root..'/tools/art_v15/sources-damage.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' damage overlays; manifest now has '..C.count(m)..' entries.')
