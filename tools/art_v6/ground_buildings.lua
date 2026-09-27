-- Root-authored Aseprite base repair. Work on preserved native sources; retain
-- the upper architecture and its anchors, replace the floating oval shadow.
local root=assert(app.params.root,'root required')
local painter=dofile(root..'/tools/art_v5/painter.lua')
local function poly(c,p,color)
  local q={};for _,v in ipairs(p) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end
  c:poly(q,color)
end
local function line(c,x,y,xx,yy,color,w)
  c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},color,w)
end
local function diamond(x,y,w,d)
  return {{x-w,y-w/2},{x-w+d,y-(w+d)/2},{x+d,y-d/2},{x,y}}
end
local function foundation(c,x,y,w,d)
  c:layer(1)
  -- A compacted earth perimeter follows the same 2:1 footprint as the walls.
  -- It touches the building all the way around, with no detached front oval.
  poly(c,diamond(x,y+7,w+5,d+5),'salt0')
  poly(c,diamond(x,y+5,w+3,d+3),'earth1')
  poly(c,diamond(x,y+3,w+1,d+1),'ivory1')
  -- Only a shallow front course is exposed; this is an embedded foundation.
  poly(c,{{x-w-1,y-(w+1)/2+2},{x,y+3},{x,y+5},{x-w-1,y-(w+1)/2+4}},'earth0')
  poly(c,{{x,y+3},{x+d+1,y-(d+1)/2+2},{x+d+1,y-(d+1)/2+4},{x,y+5}},'earth1')
  -- Soil laps over short parts of the edge instead of outlining a pristine plinth.
  line(c,x-w+5,y-w/2+6,x-w+14,y-w/2+10,'salt1',2)
  line(c,x+d-17,y-d/2+12,x+d-8,y-d/2+8,'salt1',2)
  line(c,x-8,y+7,x-2,y+8,'salt2')
end
local function skirt(c,p,depth)
  c:layer(6)
  for i=1,#p-1 do
    local a,b=p[i],p[i+1]
    poly(c,{{a[1],a[2]},{b[1],b[2]},{b[1],b[2]+depth},{a[1],a[2]+depth}},i%2==1 and 'ivory1' or 'earth1')
    -- Tight occlusion directly at the wall-to-foundation join.
    line(c,a[1],a[2],b[1],b[2],'earth0')
    local x=(a[1]+b[1])/2;local y=(a[2]+b[2])/2
    line(c,x,y+1,x,y+depth-1,'earth0')
    line(c,a[1]+2,a[2]+depth,b[1]-2,b[2]+depth,'salt0')
  end
end
local function ramp(c,x,y,w,d)
  c:layer(6)
  -- Replace the floating front lip with a poured approach that reaches grade.
  poly(c,{{x-w,y-w/2},{x,y},{x+d,y-d/2},
    {x+d+3,y-d/2+5},{x+1,y+8},{x-w-3,y-w/2+5}},'ivory1')
  poly(c,{{x-w+1,y-w/2},{x,y},{x+d-1,y-d/2},
    {x+d+1,y-d/2+3},{x+1,y+6},{x-w-1,y-w/2+3}},'earth2')
  line(c,x-w+1,y-w/2,x,y,'steel1')
  line(c,x,y,x+d-1,y-d/2,'steel1')
  -- Expansion seams lie in the surface, ending before the broken soil edge.
  line(c,x-w/2,y-w/4+2,x-w/2-1,y-w/4+5,'earth1')
  line(c,x+d/2,y-d/4+2,x+d/2+1,y-d/4+5,'earth1')
  line(c,x-5,y+7,x+1,y+8,'salt0')
  line(c,x+4,y+6,x+9,y+4,'salt1')
end
local function soil(c,x,y,w)
  c:layer(6)
  poly(c,{{x-w,y},{x-2,y-2},{x+4,y-1},{x+w,y-3},{x+w+1,y},{x+2,y+2},{x-6,y+1}},'salt0')
  line(c,x-w+3,y,x-3,y-1,'salt2')
end
local repairs={
  union_hq=function(c)
    foundation(c,-2,5,55,59)
    skirt(c,{{-54,-19},{-22,-3},{-17,-5}},5)
    skirt(c,{{28,-9},{51,-20}},4)
    ramp(c,3,2,19,20)
    soil(c,-40,-6,9);soil(c,39,4,11)
  end,
  assembly_hq=function(c)
    foundation(c,0,5,51,53)
    skirt(c,{{-49,-8},{-23,5},{-11,-1}},4)
    skirt(c,{{31,-10},{49,-19}},4)
    ramp(c,2,4,20,24)
    soil(c,-39,0,9);soil(c,43,-9,8)
  end,
  union_works=function(c)
    foundation(c,2,5,51,50)
    skirt(c,{{-41,-21},{2,0},{46,-22}},4)
    ramp(c,-10,6,27,22)
    soil(c,42,5,10)
  end,
  assembly_works=function(c)
    foundation(c,0,4,48,50)
    skirt(c,{{-43,-4},{-28,3},{-20,-1}},4)
    skirt(c,{{26,-9},{45,-18}},4)
    ramp(c,0,4,21,25)
    soil(c,-34,4,11);soil(c,35,-3,10)
  end,
  dropoff=function(c)
    foundation(c,0,5,43,48)
    skirt(c,{{-40,-15},{0,5},{45,-17}},3)
    soil(c,-31,0,9);soil(c,26,-5,9)
  end,
  condenser=function(c)
    foundation(c,0,5,37,40)
    skirt(c,{{-35,-12},{0,5},{37,-13}},3)
    soil(c,-22,-1,10);soil(c,23,4,12)
  end,
  tower=function(c)
    foundation(c,0,5,37,39)
    skirt(c,{{-25,-12},{0,0},{27,-13}},5)
    soil(c,-24,-2,9);soil(c,24,-3,9)
  end,
  gate=function(c)
    foundation(c,0,6,53,54)
    skirt(c,{{-45,-6},{-30,1},{-14,-7}},5)
    skirt(c,{{19,-8},{34,-1},{50,-9}},5)
    -- Small abutments meet the bridge deck and the surrounding bank.
    c:layer(6)
    poly(c,{{-34,5},{33,5},{39,9},{33,12},{-36,12},{-41,9}},'earth2')
    line(c,-34,5,32,5,'earth0')
    soil(c,-39,4,7);soil(c,42,2,7)
  end,
  well=function(c)
    foundation(c,0,4,27,28)
    skirt(c,{{-25,-8},{0,4},{26,-9}},2)
    soil(c,-18,0,7);soil(c,18,0,7)
  end,
}

local names={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','well'}
for _,name in ipairs(names) do
  local sprite=app.open(root..'/art/archive/art-v5/sources/'..name..'.aseprite')
  assert(sprite.width==160 and sprite.height==144 and #sprite.frames==1 and #sprite.layers==6)
  local images={}
  for i,layer in ipairs(sprite.layers) do
    assert(layer.name==painter.layers[i])
    local im=Image(160,144,ColorMode.RGB);im:clear()
    local cel=layer:cel(1)
    if cel and i~=1 then im:drawImage(cel.image,cel.position) end
    if cel then sprite:deleteCel(cel) end
    images[layer.name]=sprite:newCel(layer,1,im,Point(0,0)).image
  end
  local c=painter.new(images,{key=name,x=0,y=0,w=160,h=144,anchor_x=80,anchor_y=128},0)
  repairs[name](c)
  sprite:saveAs(root..'/art/source/v6/'..name..'.aseprite')
end
print('Root repaired nine fixed structure bases in their editable Aseprite documents.')
