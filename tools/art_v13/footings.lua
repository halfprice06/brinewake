-- Root refinement of the v6 masonry. Shadows and surrounding ground patches
-- have been removed; attached foundation courses and approach surfaces remain.
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
  -- Solid masonry top and side faces only. No underlying earth/shadow patch.
  poly(c,diamond(x,y+3,w+1,d+1),'ivory1')
  -- Only a shallow front course is exposed; this is an embedded foundation.
  poly(c,{{x-w-1,y-(w+1)/2+2},{x,y+3},{x,y+5},{x-w-1,y-(w+1)/2+4}},'earth0')
  poly(c,{{x,y+3},{x+d+1,y-(d+1)/2+2},{x+d+1,y-(d+1)/2+4},{x,y+5}},'earth1')
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
end
local repairs={
  union_hq=function(c)
    foundation(c,-2,5,55,59)
    skirt(c,{{-54,-19},{-22,-3},{-17,-5}},5)
    skirt(c,{{28,-9},{51,-20}},4)
    ramp(c,3,2,19,20)
  end,
  assembly_hq=function(c)
    foundation(c,0,5,51,53)
    skirt(c,{{-49,-8},{-23,5},{-11,-1}},4)
    skirt(c,{{31,-10},{49,-19}},4)
    ramp(c,2,4,20,24)
  end,
  union_works=function(c)
    foundation(c,2,5,51,50)
    skirt(c,{{-41,-21},{2,0},{46,-22}},4)
    ramp(c,-10,6,27,22)
  end,
  assembly_works=function(c)
    foundation(c,0,4,48,50)
    skirt(c,{{-43,-4},{-28,3},{-20,-1}},4)
    skirt(c,{{26,-9},{45,-18}},4)
    ramp(c,0,4,21,25)
  end,
  dropoff=function(c)
    foundation(c,0,5,43,48)
    skirt(c,{{-40,-15},{0,5},{45,-17}},3)
  end,
  condenser=function(c)
    foundation(c,0,5,37,40)
    skirt(c,{{-35,-12},{0,5},{37,-13}},3)
  end,
  tower=function(c)
    foundation(c,0,5,37,39)
    skirt(c,{{-25,-12},{0,0},{27,-13}},5)
  end,
  gate=function(c)
    foundation(c,0,6,53,54)
    skirt(c,{{-45,-6},{-30,1},{-14,-7}},5)
    skirt(c,{{19,-8},{34,-1},{50,-9}},5)
    -- Small abutments meet the bridge deck and the surrounding bank.
    c:layer(6)
    poly(c,{{-34,5},{33,5},{39,9},{33,12},{-36,12},{-41,9}},'earth2')
    line(c,-34,5,32,5,'earth0')
  end,
  well=function(c)
    foundation(c,0,4,27,28)
    skirt(c,{{-25,-8},{0,4},{26,-9}},2)
  end,
}

return repairs
