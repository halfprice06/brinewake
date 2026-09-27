-- Root authoring entry point for the Saltglass Compact's effects and the
-- Salter's crust. New keys only:
--   terrain_crust_{0..3}        32x16, anchor 16,8: a laid causeway cell,
--                               salt plates crusted over tidal water
--   terrain_crust_fresh_{0..2}  the row being laid: wet salt, setting, set
--   fx_beam_flare_{0..2}        32x32, anchor 16,16: the Heliostat's focus
--                               lens as it fires (drawn at the muzzle)
--   fx_beam_hit_{0..3}          32x32, anchor 16,16: the burn where the beam
--                               lands
--   fx_impact_glaze_{0..3}      48x48, anchor 24,32: a shot on a Compact
--                               machine, glaze chips and salt dust
--   fx_wreck_glaze_{0..5}       64x64, anchor 32,50: a Compact machine
--                               breaking, stilts folding, the dish falling
-- The beam line itself is drawn by the game between the flare and the hit:
-- a one-pixel glint core (fffbe6) with a glass3 (91ccd0) edge each side.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local painter=C.painter
local layers=painter.layers
local out=C.OUT..'effects';os.execute('mkdir -p '..out..' '..C.SOURCE)
local atlas,m,packer=C.open_atlas()
local ctx={atlas=atlas,packer=packer,entries={},out=out}
local function new(W,H,ax,ay,face)
  local ims={}
  for _,l in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[l]=im end
  local c=painter.new(ims,{key='fx',x=0,y=0,w=W,h=H,anchor_x=ax,anchor_y=ay,clip=true,remap=M.BUDGET},face or 1)
  local function list() local l={};for i,n in ipairs(layers) do l[i]=ims[n] end;return l end
  return c,list
end
local function px(c,x,y,col) c:pixel(c.cx+x,c.cy+y,col) end
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end

-- The cell diamond (the lane tiles' shape): row y spans |x| < 2*(8-|y-8|).
local function inside(x,y) local d=8-math.abs(y-8);return d>0 and x>=16-2*d and x<16+2*d end

-- A laid cell: salt plates (crust1) with bright crystal tops (crust2), dark
-- joints between the plates (crust0), a rim of wet glass at the diamond's
-- edge where the water still shows, and one glint.
local function crust(c,variant,stage)
  c:layer(2)
  for y=0,15 do for x=0,31 do if inside(x,y) then
    local edge=not (inside(x-2,y) and inside(x+2,y) and inside(x,y-1) and inside(x,y+1))
    local col
    if stage==0 then
      -- Wet salt: mostly brine, the first crystals floating.
      col=(hash(x,y,variant)%5==0) and 'crust1' or ((hash(x,y,variant+9)%3==0) and 'glass2' or 'glass1')
    else
      -- Plates on a 2:1 grid, jittered per variant.
      local u=(x+2*y+variant*3)%10;local v=(x-2*y+40+variant*5)%12
      if u==0 or v==0 then col='crust0' else col=((u+v)%7==1) and 'crust2' or 'crust1' end
      if stage==1 and hash(x,y,variant+3)%3==0 then col='glass2' end
      -- A setting row still shows brine at its rim; a set cell joins its
      -- neighbours with no seam, so a strip reads as one road.
      if edge and stage==1 then col='glass1' end
    end
    c:pixel(x,y,col)
  end end end
  if stage~=0 then
    local g=({{12,6},{19,9},{9,8},{22,6}})[variant+1]
    c:pixel(g[1],g[2],'glint')
  end
end
local frames={}
for v=0,3 do local c,list=new(32,16,16,8);crust(c,v,2);frames[#frames+1]={key='terrain_crust_'..v,duration=1,images=list()} end
C.document(ctx,'terrain_crust',32,16,16,8,layers,frames,'crust')
frames={}
for s=0,2 do local c,list=new(32,16,16,8);crust(c,1,s);frames[#frames+1]={key='terrain_crust_fresh_'..s,duration=1/3,images=list()} end
C.document(ctx,'terrain_crust_fresh',32,16,16,8,layers,frames,'setting')

-- The beam's flare at the focus lens: a white core, glass rays in four
-- directions, shrinking over three frames.
local function star(c,r,core,ray,tip)
  for i=-r,r do px(c,i,0,(math.abs(i)<=1) and core or ray);px(c,0,math.floor(i/2),(math.abs(i)<=1) and core or ray) end
  for i=-math.floor(r/2),math.floor(r/2) do px(c,i,i,tip);px(c,i,-i,tip) end
  px(c,0,0,core)
end
frames={}
for k=0,2 do
  local c,list=new(32,32,16,16)
  c:layer(6)
  local r=({9,6,3})[k+1]
  if k<2 then c:ellipse(16,16,({4,2})[k+1],({3,2})[k+1],'glass3') end
  star(c,r,'glint','glass3','crust1')
  frames[#frames+1]={key='fx_beam_flare_'..k,duration=0.066,images=list()}
end
C.document(ctx,'fx_beam_flare',32,32,16,16,layers,frames,'flare')

-- The burn: a white-hot point, an amber ring that grows and cools to a
-- scorch, sparks leaping up and right.
frames={}
for k=0,3 do
  local c,list=new(32,32,16,16)
  c:layer(5)
  local ring=({2,4,6,7})[k+1]
  c:ellipse(16,17,ring,math.max(1,math.floor(ring/2)),(k<2) and 'amber2' or 'glaze1')
  if k<3 then c:ellipse(16,17,math.max(1,ring-2),math.max(1,math.floor(ring/2)-1),(k<2) and 'amber3' or 'amber2') end
  if k<2 then star(c,({5,3})[k+1],'glint','amber3','amber2') end
  for i,s in ipairs({{3,-4},{5,-7},{-3,-5},{7,-3}}) do
    if (i+k)%2==0 then px(c,s[1]+k,s[2]-k,(k<2) and 'glint' or 'amber3') end
  end
  frames[#frames+1]={key='fx_beam_hit_'..k,duration=0.1,images=list()}
end
C.document(ctx,'fx_beam_hit',32,32,16,16,layers,frames,'hit')

-- A shot on glaze: chips fly off, a puff of salt dust.
frames={}
for k=0,3 do
  local c,list=new(48,48,24,32)
  c:layer(5)
  local spread=({2,5,8,10})[k+1]
  if k==0 then
    star(c,4,'glint','crust2','glaze3')
  end
  for i,d in ipairs({{-1,-1},{1,-1},{1,0},{-1,0},{0,-1},{1,-1.5}}) do
    local x=math.floor(d[1]*spread+.5);local y=math.floor(d[2]*spread*0.6+k*k*0.5+.5)
    if k<3 or i%2==0 then
      px(c,x,y,(i%3==0) and 'glaze3' or 'glaze2');px(c,x+1,y,'glaze1')
      if i%2==0 then px(c,x,y-1,'glaze4') end
    end
  end
  if k>=1 then
    local r=({0,2,3,4})[k+1]
    for i=-r,r do if (i+k)%2==0 then px(c,i,-1-math.floor(k/2),(k<3) and 'crust1' or 'crust0') end end
  end
  frames[#frames+1]={key='fx_impact_glaze_'..k,duration=0.1,images=list()}
end
C.document(ctx,'fx_impact_glaze',48,48,24,32,layers,frames,'impact')

-- A Compact machine breaking: a flash at the lens, the pod dropping on
-- folding stilts, the dish tipping and cracking, salt dust settling.
frames={}
for k=0,5 do
  local c,list=new(64,64,32,50,1)
  local drop=({0,4,9,12,12,12})[k+1]
  c:layer(2)
  if k>=2 then
    c:poly3({{-10,-6,0},{9,-8,0},{12,5,0},{-8,8,0}},(k>=4) and 'crust0' or 'crust1')
  end
  c:layer(3)
  -- Stilts folding under the pod.
  local z=14-drop
  for _,l in ipairs({{-5,-5},{5,-5},{-5,5},{5,5}}) do
    local spread=1+drop/8
    c:rod({l[1]*0.6,l[2]*0.6,z+1},{l[1]*spread,l[2]*spread,0},'glaze1','glaze3')
  end
  c:box(0,0,z,7,5,5,'glaze',2)
  c:box(0,0,z+5,5,4,1,'crust',1)
  c:layer(4)
  -- The dish on its mast, tipping over.
  local tilt=({0,0.3,0.8,1.3,1.5,1.5})[k+1]
  local top={-math.sin(tilt)*10,0,z+6+math.cos(tilt)*10}
  c:line3({0,0,z+6},top,'rust1')
  M.mirror(c,top,{math.sin(tilt)*0.3,0.6,0.75},4,{rim='rust1'})
  if k>=3 then local p=c:point(table.unpack(top));c:line({p[1]-2,p[2]-1},{p[1]+2,p[2]+1},'glaze0') end
  c:layer(6)
  if k==0 then local p=c:point(4,0,z+4);c:ellipse(p[1],p[2],3,2,'glint');c:ellipse(p[1],p[2],2,1,'crust2') end
  if k>=1 and k<=4 then
    -- Salt dust rising and thinning.
    for i=0,5 do
      local x=-8+i*3+k;local y=-2-k*2-(i%2)
      if (i+k)%2==0 or k<3 then px(c,x,y,(k<3) and 'crust2' or 'crust1') end
    end
  end
  frames[#frames+1]={key='fx_wreck_glaze_'..k,duration=0.1,images=list()}
end
C.document(ctx,'fx_wreck_glaze',64,64,32,50,layers,frames,'wreck')

table.sort(ctx.entries,function(a,b) return a.key<b.key end)
C.write(root..'/tools/art_v21/sources-effects.json',json.encode(ctx.entries))
C.save_atlas(atlas,m)
print('Authored '..#ctx.entries..' effect frames.')
