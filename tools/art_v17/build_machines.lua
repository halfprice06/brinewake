-- Root authoring entry point for the v17 machines: the Barge and the
-- Lifter (base and walk facings), then derives the idle acting, the damage overlay and
-- the grounded shadow of every frame by the v15 methods. Writes editable
-- documents under art/source/v17/, review PNGs under output/art-v17/, the
-- registry tools/art_v17/sources-machines.json and the v17 manifest.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v17/common.lua')
local painter=C.painter
local bodies=dofile(root..'/tools/art_v17/machines.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v17/machines';os.execute('mkdir -p '..out)
local layers=painter.layers
local A=app.pixelColor.rgbaA
local W,H,AX,AY=64,64,32,50

-- Paint one machine frame into six layer images.
local function paint(draw,face)
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[layer]=im end
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY},face)
  draw(c)
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  return list
end
-- The oval the body drew on layer 1 becomes the grounded shadow's seed.
local function grounded(list)
  local bodies={};for i=2,#list do bodies[#bodies+1]=list[i] end
  list[1]=C.grounded_shadow(bodies,list[1],W,H)
  return list
end

-- Idle acting (v15): Union machines settle a pixel with a vent puff;
-- Assembly machines flex up a pixel while the chamber brightens.
local function shifted(im,dx,dy)
  local o=Image(im.width,im.height,ColorMode.RGB)
  for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y)
   if A(v)>0 and x+dx>=0 and y+dy>=0 and x+dx<im.width and y+dy<im.height then o:drawPixel(x+dx,y+dy,v) end
  end end
  return o
end
local function recolour(im,map)
  local o=Image(im.width,im.height,ColorMode.RGB)
  for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y);if A(v)>0 then o:drawPixel(x,y,map[v] or v) end end end
  return o
end
local function cabin_top(im)
  for y=0,im.height-1 do local xs={}
   for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then xs[#xs+1]=x end end
   if #xs>0 then return xs[math.ceil(#xs/2)],y end
  end
end
local glow1={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3}
local glow2={[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3,[C.rgba.amber3]=C.rgba.amber4}
local function idle_frame(base,union,k,airborne)
  local images={}
  if airborne then
    -- A lifter rides the air: the whole body lifts a pixel and settles.
    local rise=(k==1) and -1 or 0
    for i=2,6 do images[i]=shifted(base[i],0,rise) end
    images[1]=Image(base[1])
    return images
  elseif union then
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
    images[2]=Image(base[2]);images[3]=Image(base[3])
    images[4]=recolour(base[4],(k==0) and glow1 or glow2)
    images[5]=(k==0) and shifted(recolour(base[5],glow1),0,-1) or recolour(base[5],glow2)
    images[6]=Image(base[6])
  end
  images[1]=Image(base[1])
  return grounded(images)
end

-- Damage marks (v15): dents and rust streaks on steel, creases and frayed
-- strands on the weave, on the base silhouette only.
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end
local steelish={};for _,r in ipairs({'steel2','steel3','steel4','ivory2','ivory3','ivory4','rust2','rust3','rust4','glass2','glass3'}) do steelish[C.rgba[r]]=true end
local weave={};for _,r in ipairs({'jade2','jade3','jade4','reed2','reed3','reed4','ivory2','ivory3'}) do weave[C.rgba[r]]=true end
local function damage_frame(base_list,material,seed)
  local im=Image(W,H,ColorMode.RGB);for i=2,#base_list do im:drawImage(base_list[i],Point(0,0)) end
  local body={};local lowest=-1;local top=H
  for y=0,H-1 do for x=0,W-1 do if A(im:getPixel(x,y))>0 then body[#body+1]={x,y};if y>lowest then lowest=y end;if y<top then top=y end end end end
  local function opaque(x,y) return x>=0 and y>=0 and x<W and y<H and A(im:getPixel(x,y))>0 end
  local dents=Image(W,H,ColorMode.RGB);local streaks=Image(W,H,ColorMode.RGB);local soot=Image(W,H,ColorMode.RGB)
  if #body==0 then return {dents,streaks,soot} end
  local floor_limit=lowest-math.floor((lowest-top)*0.18)
  local placed,tries=0,0
  while placed<4 and tries<4000 do
    tries=tries+1
    local p=body[1+hash(tries,seed,1)%#body];local x,y=p[1],p[2];local v=im:getPixel(x,y)
    if y<floor_limit and y>top+2 then
      if material=='steel' and steelish[v] then
        dents:drawPixel(x,y,C.rgba.steel0);if opaque(x+1,y) then dents:drawPixel(x+1,y,C.rgba.ink) end;if opaque(x,y+1) then dents:drawPixel(x,y+1,C.rgba.steel1) end
        if opaque(x-1,y-1) then dents:drawPixel(x-1,y-1,C.rgba.steel4) end
        local len=4+hash(x,y,seed)%6
        for d=1,len do if opaque(x,y+1+d) and y+1+d<floor_limit then streaks:drawPixel(x,y+1+d,C.rgba[(d%3==0) and 'rust0' or 'rust1']);if d<=2 and opaque(x+1,y+1+d) then streaks:drawPixel(x+1,y+1+d,C.rgba.rust0) end end end
        placed=placed+1
      elseif material=='weave' and weave[v] then
        dents:drawPixel(x,y,C.rgba.jade0);if opaque(x+1,y) then dents:drawPixel(x+1,y,C.rgba.jade0) end;if opaque(x+2,y+1) then dents:drawPixel(x+2,y+1,C.rgba.jade1) end
        local len=3+hash(x,y,seed)%5
        for d=1,len do local yy=y+1+d;local xx=x+((d%2==0) and 1 or 0)
          if opaque(xx,yy) then streaks:drawPixel(xx,yy,C.rgba[(d==len) and 'reed1' or 'reed0']) end
        end
        placed=placed+1
      end
    end
  end
  return {dents,streaks,soot}
end

local machines={
 {name='barge',union=false,material='weave'},
 {name='lifter',union=true,material='steel',airborne=true},
}
local total=0
for _,mc in ipairs(machines) do
  local n=mc.name
  local body=assert(bodies[n],n)
  local finish=bodies.finish[n]
  local function draw_with(phase) return function(c) body(c,phase);if finish then finish(c) end end end
  -- A lifter keeps the plain oval on the ground; the rest ground their shadow.
  local function settle(list) if mc.airborne then return list end return grounded(list) end
  -- Base and walk: eight facings, five frames each.
  local frames,tags={},{}
  local bases={}
  for face=0,7 do
    local base=settle(paint(draw_with(nil),face))
    bases[face]=base
    frames[#frames+1]={key=n..'_'..face,duration=1,images=base}
    for phase=0,3 do
      frames[#frames+1]={key=n..'_'..face..'_walk_'..phase,duration=.14,images=settle(paint(draw_with(phase),face))}
    end
    tags[#tags+1]={'face_'..face,face*5+1,face*5+5}
  end
  C.document_images(n,W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  m.sprites[n]=m.sprites[n..'_0']
  -- Idle acting.
  local idle={}
  for face=0,7 do for k=0,1 do idle[#idle+1]={key=n..'_'..face..'_idle_'..k,duration=.2,images=idle_frame(bases[face],mc.union,k,mc.airborne)} end end
  C.document_images(n..'_idle',W,H,AX,AY,layers,idle,'idle_acting',m,packer,out,entries)
  -- Damage overlays.
  local dmg={}
  for face=0,7 do dmg[#dmg+1]={key=n..'_'..face..'_damage',duration=1,images=damage_frame(bases[face],mc.material,face+#n*3)} end
  C.document_images(n..'_damage',W,H,AX,AY,{'dents and tears','streaks and strands','soot'},dmg,'damage_facings',m,packer,out,entries)
  total=total+#frames+#idle+#dmg
end
C.write(root..'/tools/art_v17/sources-machines.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..total..' machine frames; manifest now has '..C.count(m)..' entries.')
