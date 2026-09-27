-- Root authoring entry point for the v18 machines: the six v16 roles and
-- the two v17 transports rebuilt in place (same keys, same rectangles): base
-- and walk facings, the Caisson deployment, the Dredger's worker set, the
-- idle acting, the damage overlay and the grounded shadow of every frame.
-- Writes editable documents under art/source/v18/, review PNGs under
-- output/art-v18/machines/, the registry tools/art_v18/sources-machines.json
-- and the v18 manifest.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v18/common.lua')
local D=dofile(root..'/tools/art_v18/damage.lua')
local painter=C.painter
local bodies=dofile(root..'/tools/art_v18/machines.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v18/machines';os.execute('mkdir -p '..out)
local layers=painter.layers
local A=app.pixelColor.rgbaA
local W,H,AX,AY=64,64,32,50
local only=app.params.only

local function paint(name,draw,face)
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[layer]=im end
  local b=bodies.budget[name]
  local remap=b and C.budget(b.allowed,b.fallback) or nil
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=remap},face)
  draw(c)
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  return list
end
local function grounded(list)
  local bodies={};for i=2,#list do bodies[#bodies+1]=list[i] end
  list[1]=C.grounded_shadow(bodies,list[1],W,H)
  return list
end
local function composite(list,from)
  local im=Image(W,H,ColorMode.RGB);for i=from or 2,#list do im:drawImage(list[i],Point(0,0)) end;return im
end
-- The exhaust point: the top of the cabin layer, biased to its right edge,
-- where a Union machine's vent puff appears.
local function cabin_top(im)
  for y=0,im.height-1 do local xs={}
   for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then xs[#xs+1]=x end end
   if #xs>0 then return xs[math.ceil(#xs*0.7)],y end
  end
end
local glow1={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3}
local glow2={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3,[C.rgba.amber3]=C.rgba.amber4}
-- Idle acting. Union machines settle a pixel on their suspension while a
-- vent puff forms as one cluster, then rises, spreads and drifts right with
-- the breeze as the body comes level. Assembly machines flex up a pixel as
-- the pressure chamber brightens. The Lifter rides the air.
local function idle_frame(base,kind,k)
  local images={}
  images[1]=Image(base[1])
  if kind=='air' then
    local rise=(k==1) and -1 or 0
    for i=2,6 do images[i]=C.shifted(base[i],0,rise) end
    return images
  elseif kind=='union' then
    local dip=(k==0) and 1 or 0
    images[2]=Image(base[2]);images[3]=Image(base[3])
    for i=4,6 do images[i]=C.shifted(base[i],0,dip) end
    local cx,cy=cabin_top(base[4])
    if cx then
      local finish=images[6];local body=composite(images)
      local function puff(x,y,col) if x>=0 and y>=0 and x<W and y<H and A(body:getPixel(x,y))==0 then finish:drawPixel(x,y,C.rgba[col]) end end
      if k==0 then
        -- A fresh puff: dense, five pixels in one cluster, lit on top.
        puff(cx+1,cy-2,'steel3');puff(cx+2,cy-2,'steel3');puff(cx+3,cy-2,'steel3')
        puff(cx+2,cy-1,'steel2');puff(cx+3,cy-1,'steel2')
        puff(cx+2,cy-3,'steel4')
      else
        -- Risen and thinning: wider, paler, leaning right, one piece
        -- breaking away downwind.
        for x=cx+2,cx+6 do puff(x,cy-5,'steel3') end
        for x=cx+3,cx+5 do puff(x,cy-6,'steel3') end
        puff(cx+4,cy-4,'steel2');puff(cx+3,cy-4,'steel2')
        puff(cx+7,cy-7,'steel2');puff(cx+8,cy-7,'steel2')
      end
    end
    return grounded(images)
  else
    images[2]=Image(base[2]);images[3]=Image(base[3])
    images[4]=C.recolour(base[4],(k==0) and glow1 or glow2)
    images[5]=(k==0) and C.shifted(C.recolour(base[5],glow1),0,-1) or C.recolour(base[5],glow2)
    images[6]=Image(base[6])
    return grounded(images)
  end
end
local function damage_frame(base_list,material,seed)
  return D.marks(C,composite(base_list),material,1,seed,{count=2,spacing=9})
end

local machines={
 {name='tidewatch',kind='union',material='steel'},
 {name='caulker',kind='union',material='steel'},
 {name='caisson',kind='union',material='steel'},
 {name='lampwright',kind='assembly',material='weave'},
 {name='tender',kind='assembly',material='weave'},
 {name='dredger',kind='assembly',material='weave'},
 {name='barge',kind='assembly',material='weave'},
 {name='lifter',kind='air',material='steel'},
}
local total=0
local report={}
for _,mc in ipairs(machines) do
  local n=mc.name
  if not only or only==n then
  local body=assert(bodies[n],n)
  local finish=bodies.finish[n]
  local function draw_with(phase) return function(c) body(c,phase);if finish then finish(c) end end end
  local function settle(list) if mc.kind=='air' then return list end return grounded(list) end
  local frames,tags={},{}
  local bases={}
  local maxc=0
  for face=0,7 do
    local base=settle(paint(n,draw_with(nil),face))
    bases[face]=base
    frames[#frames+1]={key=n..'_'..face,duration=1,images=base}
    local cnt=C.colours({base[2],base[3],base[4],base[5],base[6]},true);if cnt>maxc then maxc=cnt end
    for phase=0,3 do
      frames[#frames+1]={key=n..'_'..face..'_walk_'..phase,duration=.14,images=settle(paint(n,draw_with(phase),face))}
    end
    tags[#tags+1]={'face_'..face,face*5+1,face*5+5}
  end
  report[#report+1]=n..' colours '..maxc
  C.document_images(n,W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  m.sprites[n]=m.sprites[n..'_0']
  local idle={}
  for face=0,7 do for k=0,1 do idle[#idle+1]={key=n..'_'..face..'_idle_'..k,duration=.3,images=idle_frame(bases[face],mc.kind,k)} end end
  C.document_images(n..'_idle',W,H,AX,AY,layers,idle,'idle_acting',m,packer,out,entries)
  local dmg={}
  for face=0,7 do dmg[#dmg+1]={key=n..'_'..face..'_damage',duration=1,images=damage_frame(bases[face],mc.material,face+#n*3)} end
  C.document_images(n..'_damage',W,H,AX,AY,{'dents and tears','streaks and strands','soot'},dmg,'damage_facings',m,packer,out,entries)
  total=total+#frames+#idle+#dmg
  end
end
if not only or only=='caisson' then
  local frames,tags={},{}
  for face=0,7 do
    for stage=0,2 do
      frames[#frames+1]={key='caisson_'..face..'_deploy_'..stage,duration=stage==2 and .334 or .333,images=grounded(paint('caisson',function(c) bodies.caisson_deploy(c,stage);bodies.finish.caisson(c) end,face))}
    end
    tags[#tags+1]={'deploy_'..face,face*3+1,face*3+3}
  end
  C.document_images('caisson_deployment',W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  total=total+#frames
end
if not only or only=='dredger' then
for _,mode in ipairs({'loaded','loaded_walk','gather','unload'}) do
  local count=mode=='loaded' and 1 or (mode=='loaded_walk' and 4 or 3)
  local frames,tags={},{}
  for face=0,7 do
    for phase=0,count-1 do
      local key='dredger_'..face..'_'..mode..(count>1 and '_'..phase or '')
      frames[#frames+1]={key=key,duration=mode=='loaded_walk' and .14 or (mode=='gather' and (phase==2 and .334 or .333) or .1),images=grounded(paint('dredger',function(c) bodies.dredger_mode(c,mode,phase);bodies.finish.dredger(c) end,face))}
    end
    tags[#tags+1]={mode..'_'..face,face*count+1,face*count+count}
  end
  C.document_images('dredger_'..mode,W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  total=total+#frames
end
end
if not only then
  C.write(root..'/tools/art_v18/sources-machines.json',json.encode(entries))
  C.save_manifest(m)
end
print(table.concat(report,'; '))
print('Authored '..total..' machine frames; manifest has '..C.count(m)..' entries.')
