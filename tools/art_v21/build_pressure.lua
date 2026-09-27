-- Root authoring entry point for the Compact's Surge overlays (the Brander,
-- the Glinter and the packed Heliostat): pressure_{n}_{f}_surge_{0..3} and
-- _cool_{0..2}, 64x64 at anchor (32,50), drawn by the game over the machine
-- (artistry.rs `pressure_key`) as for the other sides. A copper nozzle and a
-- glass sight bulb on the machine's back vent brine steam: surging, a white
-- jet streams back and up and the bulb burns to the glint; cooling, the
-- bulb dims and a few wisps drift off.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local painter=C.painter
local layers=painter.layers
local out=C.OUT..'pressure';os.execute('mkdir -p '..out..' '..C.SOURCE)
local atlas,m,packer=C.open_atlas()
local ctx={atlas=atlas,packer=packer,entries={},out=out}
-- The vent on each machine's back, in its local 3D (compact.lua).
local VENT={brander={-8,0,30},glinter={-6,0,21},heliostat={-9,0,26}}
local function frame(n,face,mode,k)
  local ims={}
  for _,l in ipairs(layers) do local im=Image(64,64,ColorMode.RGB);im:clear();ims[l]=im end
  local c=painter.new(ims,{key='pressure',x=0,y=0,w=64,h=64,anchor_x=32,anchor_y=50,clip=true,remap=M.BUDGET},face)
  local v=VENT[n]
  c:layer(5)
  -- The nozzle and the sight bulb.
  c:dot1(v[1],v[2],v[3],'rust2');c:dot1(v[1]-1,v[2],v[3],'rust1')
  local b=c:point(v[1]+1,v[2],v[3]-2)
  local hot=(mode=='surge') and ((k%2==0) and 'glint' or 'glass3') or ({'glass3','glass2','glass2'})[k+1]
  c:pixel(b[1],b[2],hot);c:pixel(b[1]+1,b[2],'glass1');c:pixel(b[1],b[2]+1,'glass1')
  c:layer(6)
  if mode=='surge' then
    -- The jet: back and up from the nozzle, longer and wider each frame,
    -- the head of each frame's puff where the last one ended.
    local len=4+k*3
    for d=1,len do
      local f,s,z=v[1]-d,v[2]+((d+k)%3-1)*0.5,v[3]+d*0.6
      local p=c:point(f,s,z)
      local col=(d<=2) and 'glint' or ((d<len-2) and 'crust2' or 'crust1')
      c:pixel(p[1],p[2],col)
      if d>2 and d%2==0 then c:pixel(p[1],p[2]-1,'crust1') end
      if d>4 and (d+k)%3==0 then c:pixel(p[1]+1,p[2]-1,'crust2') end
    end
  else
    -- Cooling: two wisps drifting up and back.
    for i=0,1 do
      local p=c:point(v[1]-3-k*2-i*3,v[2],v[3]+3+k*2+i*2)
      if k<2 or i==0 then c:pixel(p[1],p[2],(k==0) and 'crust2' or 'crust1');c:pixel(p[1]+1,p[2],'crust1') end
    end
  end
  local l={};for i,nm in ipairs(layers) do l[i]=ims[nm] end
  return l
end
for _,n in ipairs({'brander','glinter','heliostat'}) do
  local frames,tags={},{}
  for face=0,7 do
    local first=#frames+1
    for k=0,3 do frames[#frames+1]={key='pressure_'..n..'_'..face..'_surge_'..k,duration=0.1,images=frame(n,face,'surge',k)} end
    for k=0,2 do frames[#frames+1]={key='pressure_'..n..'_'..face..'_cool_'..k,duration=0.2,images=frame(n,face,'cool',k)} end
    tags[#tags+1]={'face_'..face,first,#frames}
  end
  C.document(ctx,'pressure_'..n,64,64,32,50,layers,frames,tags)
end
table.sort(ctx.entries,function(a,b) return a.key<b.key end)
C.write(root..'/tools/art_v21/sources-pressure.json',json.encode(ctx.entries))
C.save_atlas(atlas,m)
print('Authored '..#ctx.entries..' pressure frames.')
