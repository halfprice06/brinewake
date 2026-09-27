-- Root-authored home-page vignette: the sluice station on its salt strip
-- between a dry and a flooded crossing, composed on the game's own dimetric
-- grid from the v14 ground families, banks and water modules plus existing
-- project machines and coast life. Static, 256x124, anchor (0,0). The atlas
-- sprites are copied exactly; the ground, shelf and lane floors are painted.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
C.extra.sea_ink='2b515b';C.extra.sea_mid='416b73';C.extra.clear='263d44';C.extra.shelf_far='3d6c75';C.extra.shelf_near='4d7c82'
for k,v in pairs(C.extra) do C.rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255) end
local m,packer=C.manifest()
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local entries={};local out=root..'/output/art-v14/vignette';os.execute('mkdir -p '..out)
local W,H=256,124
local GX,GY=176,118
local layers={'deep water','shallow shelf','ground','banks','crossings','water surface','machines behind','station','machines in front','coast life'}
local function cell(di,dj) return GX+(di-dj)*16, GY+(di+dj)*8 end
local function kind(di,dj)
 if di>=-1 and di<=1 and dj>=-1 and dj<=1 then return 'salt' end
 if (dj==-3 or dj==-2) and di>=-12 and di<=9 then return 'dry' end
 if (dj==2 or dj==3) and di>=-12 and di<=9 then return 'wet' end
 if di<=-7 then return 'salt' end
 return 'water'
end
local function land(k) return k=='salt' or k=='dry' end
local function hash(a,b) return (a*73+b*151+a*b*7)%97 end
local function stamp(c,key,x,y)
 local b=assert(m.sprites[key],key)
 local im=Image(atlas,Rectangle(b.x,b.y,b.w,b.h))
 c.im:drawImage(im,Point(x-b.anchor_x,y-b.anchor_y))
end
local function diamond(c,x,y,col,shrink)
 shrink=shrink or 0
 for yy=-8+shrink,8-shrink do
  local half=16-2*math.abs(yy)-2*shrink
  if half>0 then c:line(x-half,y+yy,x+half-1,y+yy,col) end
 end
end
C.document('home_vignette',W,H,0,0,layers,{{key='home_vignette',duration=1,draw=function(c)
 c.ox,c.oy=0,0
 c:layer(1);for y=0,H-1 do c:line(0,y,W-1,y,'sea1') end
 local cells={}
 for dj=-18,4 do for di=-16,8 do
  local x,y=cell(di,dj)
  if x>-24 and x<W+24 and y>-16 and y<H+16 then cells[#cells+1]={di=di,dj=dj,x=x,y=y,k=kind(di,dj)} end
 end end
 -- Shallow shelf: water beside dry ground steps up in two bands.
 c:layer(2)
 for _,e in ipairs(cells) do
  if e.k=='water' then
   local near=false
   for _,d in ipairs({{1,0},{-1,0},{0,1},{0,-1}}) do if land(kind(e.di+d[1],e.dj+d[2])) then near=true end end
   if near then diamond(c,e.x,e.y,'shelf_near',0);diamond(c,e.x,e.y,'shelf_far',2);diamond(c,e.x,e.y,'sea1',5) end
  end
 end
 -- Ground: v14 tone families with the damp waterline beside water.
 c:layer(3)
 for _,e in ipairs(cells) do
  if e.k=='salt' then
   local h=hash(e.di+20,e.dj+20);local v=(h<70) and 0 or 1+h%3
   local damp=false
   for _,d in ipairs({{1,0},{-1,0},{0,1},{0,-1}}) do if kind(e.di+d[1],e.dj+d[2])=='water' then damp=true end end
   local fam=damp and 'terrain_damp_' or (h%11<3 and 'terrain_salt_high_' or (h%13<3 and 'terrain_salt_low_' or 'terrain_salt_'))
   stamp(c,fam..v,e.x,e.y)
  end
 end
 -- Crossings: the dry lane keeps its causeway floor and crossbars; the
 -- flooded lane shows the field's dark clear water under submerged bars.
 c:layer(5)
 for _,e in ipairs(cells) do
  if e.k=='dry' then stamp(c,'terrain_lane_dry_'..(hash(e.di,e.dj)%4),e.x,e.y)
  elseif e.k=='wet' then diamond(c,e.x,e.y,'clear',0);stamp(c,'terrain_lane_wet_'..(hash(e.di,e.dj)%4),e.x,e.y) end
 end
 -- Banks where land meets water.
 c:layer(4)
 for _,e in ipairs(cells) do
  if land(e.k) then
   local h=hash(e.di+3,e.dj+5)%3
   if kind(e.di+1,e.dj)=='water' then stamp(c,'shore_e_'..h,e.x,e.y) end
   if kind(e.di,e.dj+1)=='water' then stamp(c,'shore_s_'..h,e.x,e.y) end
   if kind(e.di-1,e.dj)=='water' then stamp(c,'shore_w_'..h,e.x,e.y) end
   if kind(e.di,e.dj-1)=='water' then stamp(c,'shore_n_'..h,e.x,e.y) end
  end
 end
 -- Water surface: the authored current modules, sampled on the world grid,
 -- only over open water so the lanes and shelf stay readable.
 c:layer(6)
 local mods={}
 for v=0,2 do local b=m.sprites['water_current_'..v..'_0'];mods[v]=Image(atlas,Rectangle(b.x,b.y,b.w,b.h)) end
 for _,e in ipairs(cells) do
  if e.k=='water' then
   for yy=-8,7 do local half=16-2*math.abs(yy+0.5)
    for xx=math.floor(-half),math.floor(half)-1 do
     local px,py=e.x+xx,e.y+yy
     if px>=0 and py>=0 and px<W and py<H then
      local v=(math.floor(px/128)*13+math.floor(py/64)*17)%3
      local p=mods[v]:getPixel(px%128,py%64)
      if app.pixelColor.rgbaA(p)>0 then c.im:drawPixel(px,py,p) end
     end
    end
   end
  end
 end
 -- Actors in depth order: the crossing party north of the station sits
 -- behind it; the flooded-lane party south of it sits in front.
 c:layer(7)
 local rx,ry=cell(-9,-2);stamp(c,'riveter_0',rx,ry)
 local bx,by=cell(-6,-2);stamp(c,'bulwark_0',bx,by)
 c:layer(8);stamp(c,'gate_north_dry',GX,GY)
 c:layer(9)
 local sx,sy=cell(-7,2);stamp(c,'skipper_1',sx,sy)
 local gx,gy=cell(-4,3);stamp(c,'reedguard_7',gx,gy)
 c:layer(10)
 local px,py=cell(-9,-1);stamp(c,'reed_clump',px,py)
 local qx,qy=cell(-8,1);stamp(c,'salt_bush',qx,qy)
 local cx,cy=cell(-10,-1);stamp(c,'coast_reeds_0',cx,cy)
end}},'home_vignette',m,packer,out,entries)
C.write(root..'/tools/art_v14/sources-vignette.json',json.encode(entries))
C.write(root..'/art/exports/game-assets-v14.json',json.encode(m))
print('Authored the home vignette.')
