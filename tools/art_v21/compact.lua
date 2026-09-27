-- The Saltglass Compact's eight machines, drawn through the v21 painter.
-- Silhouette family: tall and thin. Stilt legs with backward knees, masts,
-- mirror dishes on yokes, lenses, hanging pans. Materials: cobalt-violet
-- glaze plates (the body mass), salt-white crust at the joints and feet,
-- copper collars, clear glass that carries the one bright glint.
--
-- Every machine is a function (c, st): `c` is a painter context on one
-- facing, `st` the state: st.phase is nil at rest or 0..3 on the walk;
-- st.mode names a state pose ('fire', 'deploy', 'gather', ...) and st.k its
-- frame. Walk rule (plan §8, art v19): a planted foot moves back the
-- engine's travel per phase (common.lua `travel`), the body rises on the
-- passing frames, and anything that turns turns a quarter period a frame.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v21/common.lua')
local out={}
local function sign(n) return n>=0 and 1 or -1 end
-- The colour budget: copper keeps two shades (its dark edge is the glaze's
-- darkest violet), so every machine stays within sixteen colours.
out.BUDGET={rust0='glaze0',rust3='rust2'}

-- Screen depth of a local ground point relative to the body centre: >0 is
-- nearer the viewer than the centre.
local function depth(c,f,s) return (f*c.si+s*c.co)/2 end
out.depth=depth

-- The walk gait: gait 0 front contact, 1 planted under the hip (body
-- highest), 2 back contact, 3 swinging forward, lifted.
local function gait_of(phase,offset) return phase and (phase+offset)%4 or nil end
-- Body rise over the planted legs: one unit on the passing frames. The
-- idle's first frame settles the body a unit onto its knees instead.
function out.rise(phase,amount,st)
  if st and st.mode=='idle' and st.k==0 then return -1 end
  if phase==nil then return 0 end
  return (phase%2==1) and amount or 0
end

-- A salt shoe: the flat foot every Compact stilt stands on, crust on top,
-- glaze underneath, long along the machine's heading.
local function shoe(c,f,s,z,len)
  len=len or 2
  c:line3({f-len,s,z},{f+len,s,z},'glaze0')
  c:line3({f-len+1,s,z+1},{f+len-1,s,z+1},'crust1')
  c:dot1(f-len+1,s,z+1,'crust2')
end
out.shoe=shoe
-- A copper collar at a joint.
local function collar(c,f,s,z)
  local p=c:point(f,s,z)
  c:pixel(p[1],p[2],'rust1');c:pixel(p[1]-1,p[2],'rust2');c:pixel(p[1],p[2]-1,'rust2')
end
out.collar=collar
-- A lens: a glass disc seen edge-on or face-on, with the faction's glint.
local function lens(c,f,s,z,r)
  local p=c:point(f,s,z)
  c:ellipse(p[1],p[2],r,r,'glass1')
  if r>=2 then c:ellipse(p[1],p[2],r-1,r-1,'glass2') end
  c:pixel(p[1]-math.max(0,r-1),p[2]-math.max(0,r-1),'glint')
  if r>=2 then c:pixel(p[1]-r+2,p[2]-r+1,'glass3') end
end
out.lens=lens

-- A stilt leg: a glazed thigh plate from the hip to a backward knee (a
-- wader's leg), a thin glaze shin, a copper collar at the knee and a salt
-- shoe on the ground. Far legs are drawn a step darker than near ones.
local FAR={glaze3='glaze2',glaze2='glaze1',glaze4='glaze3',crust2='crust1',crust1='crust0',rust2='rust1'}
local function stilt_leg(c,hip,foot,gait,opts,far)
  opts=opts or {}
  local step=C.travel(c.face)*(opts.stride or 1)
  local travel=gait and ({step,0,-step,0})[gait+1] or 0
  local lift=(gait==3) and (opts.lift or 3) or 0
  local toe={foot[1]+travel,foot[2],lift+1}
  local mf,ms,mz=(hip[1]+toe[1])/2,(hip[2]+toe[2])/2,(hip[3]+toe[3])/2
  local back=opts.back or 3
  local knee
  if gait==3 then knee={mf+1,ms,mz+2}
  elseif gait==1 then knee={mf-back+1,ms,mz+1}
  else knee={mf-back,ms,mz} end
  local saved=c.remap
  if far then
    local merged={};for a,b in pairs(saved or {}) do merged[a]=b end
    for a,b in pairs(FAR) do merged[a]=(saved and saved[b]) or b end
    c.remap=merged
  end
  c:beam(hip,knee,opts.thigh or 3,'glaze')
  c:rod(knee,toe,'glaze1','glaze3')
  collar(c,knee[1],knee[2],knee[3])
  shoe(c,toe[1],toe[2],lift,opts.shoe)
  c.remap=saved
end
-- A set of legs: far legs on layer 2, near legs on layer 5. `positions`
-- are {f,s,offset}; hips hang under the body at `height`, pulled in.
local function legs(c,positions,height,phase,opts,rise)
  opts=opts or {}
  table.sort(positions,function(a,b) return depth(c,a[1],a[2])<depth(c,b[1],b[2]) end)
  for i,p in ipairs(positions) do
    local near=depth(c,p[1],p[2])>0
    c:layer(near and 5 or 2)
    local pull=opts.pull or 0.6
    -- The farthest leg of a set is shaded down, whatever its side.
    stilt_leg(c,{p[1]*pull,p[2]*pull,height+(rise or 0)},p,gait_of(phase,p[3]),opts,i==1 and #positions>1)
  end
end
out.legs=legs

-- A glazed plate: a painter box in glaze with a salt-crust seam where its
-- walls meet the deck below (the white at the joints).
local function plate(c,f,s,z,hf,hs,h,cut,opts)
  opts=opts or {}
  cut=cut or 2
  c:box(f,s,z,hf,hs,h,'glaze',cut)
  local near=sign(c.co);local front=sign(c.si)
  c:line3({f-hf+cut,s+near*hs,z+1},{f+hf-cut,s+near*hs,z+1},'crust1')
  c:line3({f+front*hf,s-hs+cut,z+1},{f+front*hf,s+hs-cut,z+1},'crust0')
  if h>=5 and not opts.plain then
    -- Plate seams on the two visible walls, and copper rivets at the top.
    local sf=f+math.floor(hf/3)
    c:line3({sf,s+near*hs,z+2},{sf,s+near*hs,z+h-1},'glaze1')
    c:dot1(sf-1,s+near*hs,z+h-1,'rust2')
    local ss=s+math.floor(hs/2)*near
    if hs>=4 then c:line3({f+front*hf,ss,z+2},{f+front*hf,ss,z+h-1},'glaze1') end
  end
end
out.plate=plate

------------------------------------------------------------------------------
-- Shared parts.
local function cross(a,b) return {a[2]*b[3]-a[3]*b[2],a[3]*b[1]-a[1]*b[3],a[1]*b[2]-a[2]*b[1]} end
local function norm(a) local l=math.sqrt(a[1]^2+a[2]^2+a[3]^2);return {a[1]/l,a[2]/l,a[3]/l} end
local function basis(n)
  n=norm(n)
  local up=(math.abs(n[3])>0.9) and {0,1,0} or {0,0,1}
  local u=norm(cross(up,n));local v=cross(n,u)
  return n,u,v
end
local function add(p,a,k) return {p[1]+a[1]*k,p[2]+a[2]*k,p[3]+a[3]*k} end
out.basis=basis
-- A mirror or dish: a round plate facing along n. The face is glass in a
-- glazed rim with the glint on its upper edge; seen from behind it is a
-- glazed back with copper struts. `concave` shades a dish: the inner side
-- away from the light is lit, as the inside of a bowl is.
local function mirror(c,cen,n,r,opts)
  opts=opts or {}
  local nn,u,v=basis(n)
  local vis=c:facing(nn)
  if vis>0.04 then
    c:disc(cen,u,v,r,opts.rim or 'glaze1')
    c:disc(cen,u,v,r-1,'glass1')
    local lit=opts.concave and 0.35 or -0.35
    -- Shift the inner disc toward the lit side in screen space.
    local p0=c:point(table.unpack(cen))
    local best,bk=nil,-1e9
    for _,cand in ipairs({u,v,{-u[1],-u[2],-u[3]},{-v[1],-v[2],-v[3]}}) do
      local q=c:point(table.unpack(add(cen,cand,1)))
      local score=(q[1]-p0[1])*(lit>0 and 1 or -1)+(q[2]-p0[2])*(lit>0 and 1 or -1)
      if score>bk then best,bk=cand,score end
    end
    c:disc(add(cen,best,math.max(0,r*0.25)),u,v,math.max(1,r-2),'glass2')
    if r>=5 then c:disc(add(cen,best,r*0.45),u,v,math.max(1,r*0.35),'glass3') end
    -- The glint: the brightest pixel on the field, on the upper-left edge.
    local top,tv=nil,1e9
    for i=0,15 do
      local a=i*math.pi/8
      local q=c:point(table.unpack(add(add(cen,u,math.cos(a)*(r-1.5)),v,math.sin(a)*(r-1.5))))
      local score=q[1]+q[2]
      if score<tv then top,tv=q,score end
    end
    c:pixel(top[1],top[2],'glint')
    if r>=4 then c:pixel(top[1]+1,top[2],'glass3');c:pixel(top[1],top[2]+1,'glass3') end
    return true
  else
    c:disc(cen,u,v,r,'glaze1')
    c:disc(cen,u,v,math.max(1,r-1),'glaze2')
    c:line3(add(cen,u,-(r-1)),add(cen,u,r-1),'rust1')
    c:line3(add(cen,v,-(r-1)),add(cen,v,r-1),'rust1')
    return false
  end
end
out.mirror=mirror
local function quad(fx,sx) return {{fx,-sx,0},{fx,sx,2},{-fx,-sx,2},{-fx,sx,0}} end

------------------------------------------------------------------------------
-- STILT: the scout. Two very long legs under a pod, a neck that holds a
-- spyglass lens forward, and a salt vane behind for balance. A wader: the
-- tallest, thinnest thing on the field.
out.stilt=function(c,st)
  local phase=st.phase
  c:shadow(9,3)
  local rise=out.rise(phase,1,st)
  legs(c,{{0,-5,0},{0,5,2}},27,phase,{pull=0.5,back=5,lift=4},rise)
  c.dz=rise
  local back_view=c.si<-0.75
  c:layer(3)
  plate(c,-1,0,24,7,4,6,2)
  c:box(-2,0,30,4,3,1,'crust',1)
  c:layer(back_view and 2 or 4)
  -- The neck and the spyglass head: a glazed barrel, the lens at its front.
  c:rod({5,0,28},{10,0,36},'glaze1','glaze3')
  collar(c,5,0,29)
  c:box(11,0,35,4,3,5,'glaze',1)
  c:line3({8,0,40},{13,0,40},'crust2')
  if not back_view then lens(c,15,0,37,2) end
  c:layer(back_view and 5 or 2)
  -- The vane: a salt fin on a rod, leaning back.
  c:rod({-7,0,28},{-13,0,34},'glaze1','glaze3')
  c:poly3({{-12,0,33},{-15,0,34},{-17,0,41},{-13,0,37}},'crust1')
  c:line3({-13,0,34},{-16,0,40},'crust2')
  c:layer(6)
  c:line3({-6,-4,29},{2,-4,29},'glaze4')
  c.dz=0
end

------------------------------------------------------------------------------
-- BRANDER: the fast line machine. A biped with a long stride, a lean
-- glazed body under a salt-white cowl with a glass visor, a brine tank on
-- its back and a fire-lance held level forward: a copper tube with a brass
-- band ending in a glass burner.
out.brander=function(c,st)
  local phase=st.phase
  local fire=st.mode=='fire' and st.k or nil
  local recoil=(fire==0) and 2 or ((fire==1) and 1 or 0)
  local near=sign(c.co)
  c:shadow(12,4)
  local rise=out.rise(phase,1,st)
  legs(c,{{1,-6,0},{1,6,2}},18,phase,{pull=0.55,back=5,lift=4},rise)
  c.dz=rise
  local ls=near*7
  local tip={19-recoil,ls,25}
  local function lance()
    c:beam({-6-recoil,ls,25},{16-recoil,ls,25},3,'rust')
    c:line3({5-recoil,ls,27},{8-recoil,ls,27},'reed3')
    c:line3({5-recoil,ls,26},{8-recoil,ls,26},'reed3')
    -- A hose from the tank to the lance's breech.
    c:line3({-6,ls*0.6,22},{-6-recoil,ls,24},'glaze0')
    -- The burner: a glass bulb in a copper cup at the tip.
    collar(c,17-recoil,ls,25)
    lens(c,tip[1],tip[2],tip[3],2)
    if fire==0 then
      local p=c:point(tip[1]+3,ls,25)
      c:ellipse(p[1],p[2],3,2,'amber2');c:ellipse(p[1],p[2],2,1,'amber3')
      c:line({p[1]-1,p[2]},{p[1]+2,p[2]},'glint')
    elseif fire==1 then
      local p=c:point(tip[1]+3,ls,25);c:pixel(p[1],p[2],'amber3');c:pixel(p[1]+1,p[2]-1,'amber2')
    end
  end
  -- The tank on the back, behind the hull from most facings.
  c:layer(depth(c,-7,0)>2 and 4 or 2)
  c:cylinder(-7,0,19,4,10,'glaze')
  c:line3({-10,0,22},{-4,0,22},'crust1')
  c:line3({-9,0,29},{-6,0,29},'crust2')
  c:layer(3)
  plate(c,0,0,15,8,5,7,2)
  c:layer(4)
  c:box(3,0,22,5,4,5,'crust',2)
  -- The visor: a glass slit across the front of the cowl.
  if c.si>-0.75 then
    c:line3({8,-3,24},{8,3,24},'glass1')
    c:line3({8,-3,25},{8,2,25},'glass2')
    c:dot1(8,-3,25,'glint')
  end
  c:layer(depth(c,0,ls)>0 and 5 or 2)
  lance()
  c:layer(6)
  c:line3({-5,-4,22},{3,-4,22},'glaze4')
  c.dz=0
end
out.muzzle={brander={19+3,7,25}}

------------------------------------------------------------------------------
-- GLINTER: the harasser. A light biped under a tall mast that carries a
-- heliograph mirror on a gimbal: the mirror is its weapon and its eye
-- (GLINT). The mast makes it the tall mark in a Compact line.
out.glinter=function(c,st)
  local phase=st.phase
  local fire=st.mode=='fire' and st.k or nil
  c:shadow(10,3)
  local rise=out.rise(phase,1,st)
  legs(c,{{1,-5,0},{1,5,2}},16,phase,{pull=0.55,back=4,lift=4},rise)
  c.dz=rise
  c:layer(3)
  plate(c,0,0,14,7,5,5,2)
  c:box(-1,0,19,4,4,2,'crust',1)
  -- A small glass port on the nose.
  if c.si>-0.75 then lens(c,7,0,17,1) end
  c:layer(4)
  c:rod({-1,0,21},{-1,0,31},'glaze1','glaze3')
  collar(c,-1,0,26)
  -- The mirror turns half toward its right side on the mast, so every
  -- facing sees its face or its back, never only its edge.
  local tilt=(fire==0) and 0.2 or 0.45
  local n={math.cos(tilt)*0.75,-math.cos(tilt)*0.66,math.sin(tilt)}
  local cen={0,0,37}
  c:line3({-1,0,31},{-1,0,33},'rust1')
  mirror(c,cen,n,6,{rim='rust1'})
  collar(c,-1,0,32)
  if fire==0 then
    local p=c:point(5,0,40)
    c:pixel(p[1],p[2],'glint')
    for _,o in ipairs({{-1,0},{1,0},{0,-1},{0,1},{-2,0},{2,0},{0,-2},{0,2}}) do c:pixel(p[1]+o[1],p[2]+o[2],'glint') end
    for _,o in ipairs({{-3,0},{3,0},{0,-3},{0,3},{-2,-2},{2,-2},{-2,2},{2,2}}) do c:pixel(p[1]+o[1],p[2]+o[2],'glass3') end
  elseif fire==1 then
    local p=c:point(5,0,40);c:pixel(p[1],p[2],'glint');c:pixel(p[1]+1,p[2]-1,'glass3');c:pixel(p[1]-1,p[2]+1,'glass3')
  end
  c:layer(6)
  c:line3({-5,-4,19},{3,-4,19},'glaze4')
  c.dz=0
end
out.muzzle.glinter={5,0,40}

------------------------------------------------------------------------------
-- RAKER: the worker. Four stilts under a shallow glazed basket; a long
-- rake on a copper pivot at its front corner, carried upright like a mast
-- and swung down to the ground to rake salvage in.
local function rake(c,pivot,tip)
  c:rod(pivot,tip,'rust1','rust2')
  local d={tip[1]-pivot[1],tip[2]-pivot[2],tip[3]-pivot[3]}
  local l=math.sqrt(d[1]^2+d[2]^2+d[3]^2);d={d[1]/l,d[2]/l,d[3]/l}
  -- The head: a glazed bar across the pole's end with four tines.
  local a,b={tip[1],tip[2]-4,tip[3]},{tip[1],tip[2]+4,tip[3]}
  c:line3(a,b,'glaze0')
  c:line3({a[1],a[2],a[3]+1},{b[1],b[2],b[3]+1},'crust1')
  for i=-4,4,2 do
    local e={tip[1]+d[1]*4,tip[2]+i,tip[3]+d[3]*4}
    c:line3({tip[1],tip[2]+i,tip[3]},e,'glaze1')
  end
end
out.raker=function(c,st)
  local phase=st.phase
  local mode,k=st.mode,st.k
  local near=sign(c.co)
  c:shadow(12,4)
  local rise=out.rise(phase,1,st)
  legs(c,quad(6,6),12,phase,{pull=0.7,back=2,lift=3,thigh=2},rise)
  c.dz=rise
  c:layer(3)
  -- The basket: a shallow glazed tray, salt-crusted inside, on a copper rim.
  plate(c,0,0,12,9,6,5,2,{plain=true})
  c:poly3({{-7,-4,17},{7,-4,17},{7,4,17},{-7,4,17}},'crust0')
  c:line3({-7,-4,17},{7,-4,17},'glaze0')
  c:line3({-6,near*6,13},{7,near*6,13},'rust1')
  local loaded=(mode=='loaded' or mode=='unload' or mode=='loaded_walk' or (mode=='gather' and k==2))
  if loaded then
    local heap=(mode=='unload') and ({3,1,0})[k+1] or 3
    if heap>0 then
      c:poly3({{-5,-3,17},{5,-3,17},{4,3,17},{-4,3,17}},'crust1')
      c:box(-1,0,17,3,2,heap,'rust',1)
      c:dot1(3,-2,17+heap,'reed3');c:dot1(-4,1,18,'glass2');c:dot1(2,2,18,'crust2');c:dot1(-2,-2,18+heap-1,'crust2')
    end
    if mode=='unload' and k<2 then
      -- Scrap falling off the side of the tipped basket.
      local sd=-near*9
      c:layer(depth(c,0,sd)>0 and 5 or 2)
      for i=0,2-k do c:dot1(-3+i*3,sd-i,9-k*4-i*2,(i%2==0) and 'rust2' or 'reed3') end
    end
  end
  c:layer(4)
  -- A small cab at the back corner with a glass eye.
  c:box(-5,-near*3,17,4,3,6,'glaze',1)
  c:box(-5,-near*3,23,3,2,1,'crust',1)
  if c.si>-0.75 then lens(c,-1,-near*3,20,1) end
  local pivot={6,near*6,15}
  local tip
  if mode=='gather' then tip=({{18,near*6,1},{12,near*6,1},{8,near*6,9}})[k+1]
  else tip={-1,near*6,35} end
  c:layer(depth(c,pivot[1],pivot[2])>0 and 5 or 2)
  rake(c,pivot,tip)
  collar(c,pivot[1],pivot[2],pivot[3])
  c:layer(6)
  c:line3({-7,-6,17},{5,-6,17},'glaze4')
  c.dz=0
end

------------------------------------------------------------------------------
-- GLAZIER: the mender. Four stilts under a round kiln pot whose mouth
-- glows; a copper blowpipe reaches forward with a gather of hot glass on
-- its end, which it lays on a hurt machine like solder.
out.glazier=function(c,st)
  local phase=st.phase
  local k=st.k or 0
  c:shadow(12,4)
  local rise=out.rise(phase,1,st)
  legs(c,quad(6,6),13,phase,{pull=0.65,back=2,lift=3,thigh=2},rise)
  c.dz=rise
  c:layer(3)
  plate(c,0,0,12,8,6,4,2,{plain=true})
  c:layer(4)
  c:cylinder(-1,0,16,6,10,'glaze')
  -- Salt-crusted shoulder bands and the glowing mouth.
  c:line3({-7,0,19},{5,0,19},'crust0')
  local top=c:point(-1,0,26)
  c:ellipse(top[1],top[2],6,3,'crust1')
  c:ellipse(top[1],top[2],4,2,'rust1')
  c:ellipse(top[1],top[2],3,1,(st.mode=='idle' and k==1) and 'amber3' or 'amber2')
  c:pixel(top[1]-1,top[2],'amber3');c:pixel(top[1],top[2],'amber3')
  -- A stub chimney at the pot's back.
  c:cylinder(-6,-2,24,2,5,'glaze')
  local near=sign(c.co)
  c:layer(depth(c,8,near*4)>0 and 5 or 2)
  c:beam({3,near*4,22},{14,near*4,17},2,'rust')
  collar(c,3,near*4,22)
  local g=c:point(15,near*4,17)
  c:ellipse(g[1],g[2],2,1,'amber2');c:pixel(g[1]-1,g[2]-1,'amber3');c:pixel(g[1],g[2]-1,'glint');c:pixel(g[1]+1,g[2],'amber3')
  c:layer(6)
  c:line3({-6,-5,13},{4,-5,13},'glaze4')
  c.dz=0
end

------------------------------------------------------------------------------
-- SALTER: lays the causeway. A long glazed hopper heaped with white salt on
-- four tall stilts, a spreading chute sloping from its front to the ground
-- and a tall screed arm that combs the crust flat.
out.salter=function(c,st)
  local phase=st.phase
  local mode,k=st.mode,st.k or 0
  local near=sign(c.co)
  c:shadow(17,6)
  local rise=out.rise(phase,1,st)
  legs(c,quad(10,8),16,phase,{pull=0.7,back=3,lift=3},rise)
  c.dz=rise
  c:layer(3)
  -- The hopper: wider at the top than the bottom.
  c:solid({{-11,-6},{10,-6},{12,-4},{12,4},{10,6},{-11,6},{-13,4},{-13,-4}},15,7,'glaze')
  c:line3({-3,near*6,17},{-3,near*6,21},'glaze1');c:line3({5,near*6,17},{5,near*6,21},'glaze1')
  c:solid({{-12,-7},{11,-7},{13,-5},{13,5},{11,7},{-12,7},{-14,5},{-14,-5}},22,2,'glaze')
  c:line3({-12,near*7,23},{11,near*7,23},'crust1')
  c:layer(4)
  -- The salt heap: a white mound in the hopper.
  c:poly3({{-11,-5,24},{10,-5,24},{10,5,24},{-11,5,24}},'crust1')
  c:poly3({{-9,-4,26},{7,-4,26},{8,0,27},{7,4,26},{-9,4,26},{-10,0,27}},'crust2')
  c:line3({-7,-2,27},{3,-2,27},'glint')
  c:line3({-10,near*5,24},{10,near*5,24},'crust0')
  -- The chute.
  c:layer(depth(c,14,0)>0 and 5 or 2)
  c:poly3({{12,-3,19},{12,3,19},{17,3,10},{17,-3,10}},'glaze1')
  c:line3({12,-3,19},{17,-3,10},'rust2')
  c:line3({12,3,19},{17,3,10},'rust1')
  c:line3({17,-3,10},{17,3,10},'crust1')
  if mode=='lay' then
    -- Salt pouring from the chute and spreading on the water ahead.
    local g=c:point(18,0,9)
    for i=0,8 do c:pixel(g[1]+((i+k)%2),g[2]+i,(i%2==0) and 'crust2' or 'crust1') end
    local spread={{-4,0},{-2,1},{0,0},{2,1},{4,0},{-3,2},{1,2},{3,2}}
    for i,o in ipairs(spread) do if (i+k)%3~=0 then local q=c:point(20+o[2],o[1],0);c:pixel(q[1],q[2],'crust2') end end
  end
  -- The screed arm, a tall mast leaning back over the hopper.
  c:layer(depth(c,-11,-near*6)>0 and 5 or 2)
  c:rod({-11,-near*6,22},{-14,-near*6,40},'glaze1','glaze3')
  c:line3({-14,-near*6-4,40},{-14,-near*6+4,40},'rust1')
  c:line3({-14,-near*6-4,41},{-14,-near*6+4,41},'crust1')
  collar(c,-11,-near*6,23)
  c:layer(6)
  c:line3({-12,-7,23},{8,-7,23},'glaze4')
  c.dz=0
end

------------------------------------------------------------------------------
-- PAN: the mobile still. A wide shallow brine pan on four stilts, ringed
-- by four mirror petals on copper arms. Walking, the petals stand closed
-- around the pan like a bud; deployed, the legs fold, the machine sits on
-- its pan and the petals open to throw the sun into the brine.
out.pan=function(c,st)
  local phase=st.phase
  local mode,k=st.mode,st.k or 0
  local open=0
  if mode=='deploy' then open=k+1 end
  if mode=='deployed' then open=3 end
  c:shadow(14,5)
  local lower=({0,3,7,9})[open+1]
  local rise=out.rise(phase,1,st)
  if open<3 then
    legs(c,quad(7,7),14-lower,phase,{pull=0.75,back=2+open,lift=3,thigh=2},rise)
  else
    -- Folded: the knees splay out to the sides, the shoes flat beside the pan.
    local q4=quad(10,10)
    table.sort(q4,function(a,b) return depth(c,a[1],a[2])<depth(c,b[1],b[2]) end)
    for _,q in ipairs(q4) do
      c:layer(depth(c,q[1],q[2])>0 and 5 or 2)
      c:beam({q[1]*0.5,q[2]*0.5,5},{q[1],q[2],3},2,'glaze')
      collar(c,q[1],q[2],3);shoe(c,q[1],q[2],0)
    end
  end
  c.dz=rise
  local z=14-lower
  c:layer(3)
  c:box(0,0,z,5,5,2,'glaze',1)
  -- The pan: a wide flat dish of glaze, brine inside, salt drying at its rim.
  local cen=c:point(0,0,z+3)
  c:ellipse(cen[1],cen[2]+1,12,6,'glaze0')
  c:ellipse(cen[1],cen[2],12,6,'glaze2')
  c:ellipse(cen[1],cen[2],11,5,'crust1')
  c:ellipse(cen[1],cen[2],10,4,'glass1')
  c:ellipse(cen[1]-1,cen[2],7,3,'glass2')
  c:line({cen[1]-11,cen[2]+1},{cen[1]-6,cen[2]+4},'glaze3')
  if open==3 or (mode=='deploy' and k==2) then
    c:line({cen[1]-4,cen[2]-1},{cen[1],cen[2]-1},'glass3')
    c:pixel(cen[1]+2+(k%2),cen[2]-2,'glint')
  end
  -- The petals on their arms, one at each corner.
  local petals={{8,-8},{8,8},{-8,-8},{-8,8}}
  table.sort(petals,function(a,b) return depth(c,a[1],a[2])<depth(c,b[1],b[2]) end)
  for _,pt in ipairs(petals) do
    c:layer(depth(c,pt[1],pt[2])>0 and 5 or 2)
    local ang=({0.15,0.5,0.85,1.1})[open+1]
    local dir=norm({-pt[1],-pt[2],0})
    local ca,sa=math.cos(ang),math.sin(ang)
    local root={pt[1]*0.8,pt[2]*0.8,z+2}
    local cen2={pt[1]*(0.8+0.35*sa),pt[2]*(0.8+0.35*sa),z+4+9*ca}
    c:line3(root,cen2,'rust1')
    mirror(c,cen2,{dir[1]*ca,dir[2]*ca,sa},5,{rim='rust1'})
  end
  if mode=='deployed' then
    -- Steam off the brine: one cluster that rises and thins.
    c:layer(6)
    local s0=c:point(0,0,z+7+k*3)
    local rows=({{{0,3},{-1,4}},{{-1,5},{0,3},{1,2}},{{0,4},{2,3},{2,1}},{{1,3},{3,1}}})[k+1]
    for j,r in ipairs(rows) do for i=0,r[2]-1 do c:pixel(s0[1]+r[1]+i-1,s0[2]-j,(j==1) and 'crust1' or 'crust2') end end
  end
  c.dz=0
end

------------------------------------------------------------------------------
-- HELIOSTAT: the heavy. A tall four-legged walker carrying a great mirror
-- dish on a copper yoke. Walking, the dish rides face-up like a bowl.
-- Deployed, the legs brace wide and low, the dish swings to face its
-- target and a lens on a boom sits at its focus, where the beam leaves.
out.heliostat=function(c,st)
  local phase=st.phase
  local mode,k=st.mode,st.k or 0
  local stage=0
  if mode=='deploy' then stage=k+1 elseif mode=='deployed_fire' then stage=3 end
  local near=sign(c.co)
  c:shadow(16,6)
  local drop=({0,2,4,5})[stage+1]
  local spread=({8,9,10,11})[stage+1]
  local rise=out.rise(phase,1,st)
  legs(c,quad(spread,spread),19-drop,phase,{pull=0.55,back=3+math.floor(stage/2),lift=3},rise)
  c.dz=rise
  local z=19-drop
  c:layer(3)
  plate(c,0,0,z,9,6,6,2)
  c:box(-1,0,z+6,6,5,2,'crust',1)
  local tilt=({1.35,1.1,0.8,0.55})[stage+1]
  local n={math.cos(tilt),0,math.sin(tilt)}
  local cen={0,0,z+19}
  local r=12
  local function yoke(side)
    c:beam({0,side*6,z+7},{0,side*9,z+18},2,'rust')
    collar(c,0,side*9,z+18)
  end
  c:layer(2);yoke(-near)
  c:layer(4)
  mirror(c,cen,n,r,{concave=true,rim='glaze1'})
  local nn,u,v=basis(n)
  c:layer(5);yoke(near)
  if stage>=2 then
    -- The focus boom and lens, out along the dish's axis.
    local focus=add(cen,nn,9)
    c:layer(c:facing(nn)>0 and 6 or 3)
    c:line3(add(cen,v,-r+1),focus,'rust1')
    lens(c,focus[1],focus[2],focus[3],2)
    if mode=='deployed_fire' and k<2 then
      local p=c:point(table.unpack(focus))
      c:pixel(p[1],p[2],'glint')
      if k==0 then
        for _,o in ipairs({{-1,0},{1,0},{0,-1},{0,1},{-2,0},{2,0}}) do c:pixel(p[1]+o[1],p[2]+o[2],'glint') end
        for _,o in ipairs({{-3,0},{3,0},{0,-2},{0,2},{-1,-1},{1,1}}) do c:pixel(p[1]+o[1],p[2]+o[2],'amber3') end
      end
    end
  end
  c:layer(6)
  c:line3({-7,-6,z+6},{6,-6,z+6},'glaze4')
  c.dz=0
end
-- The beam leaves the deployed dish's focus.
function out.heliostat_focus(z_drop)
  local z=19-5
  local tilt=0.55
  return {math.cos(tilt)*9,0,z+19+math.sin(tilt)*9}
end

return out
