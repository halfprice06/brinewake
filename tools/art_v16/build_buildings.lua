-- Root authoring entry point for the v16 buildings: the Drydock of each
-- faction (base, construction, activity, damage) and the Palisade of each
-- (base, construction, damage). Damage marks are computed on the finished
-- drawing's own silhouette above its foundation courses: nothing is drawn
-- beneath a building.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v16/common.lua')
local painter=C.painter
local B=dofile(root..'/tools/art_v16/buildings.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v16/buildings';os.execute('mkdir -p '..out)
local layers=painter.layers
local A=app.pixelColor.rgbaA
local function paint(W,H,AX,AY,draw)
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[layer]=im end
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY},0)
  draw(c)
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  return list
end
-- Damage marks (v15) for buildings: two tiers.
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end
local steelish={};for _,r in ipairs({'steel2','steel3','steel4','ivory2','ivory3','ivory4','rust2','rust3','rust4','glass2','glass3'}) do steelish[C.rgba[r]]=true end
local weave={};for _,r in ipairs({'jade2','jade3','jade4','reed2','reed3','reed4','ivory2','ivory3'}) do weave[C.rgba[r]]=true end
local function damage(base_list,W,H,material,tier,seed)
  local im=Image(W,H,ColorMode.RGB);for i=2,#base_list do im:drawImage(base_list[i],Point(0,0)) end
  local body={};local lowest=-1;local top=H
  for y=0,H-1 do for x=0,W-1 do if A(im:getPixel(x,y))>0 then body[#body+1]={x,y};if y>lowest then lowest=y end;if y<top then top=y end end end end
  local function opaque(x,y) return x>=0 and y>=0 and x<W and y<H and A(im:getPixel(x,y))>0 end
  local dents=Image(W,H,ColorMode.RGB);local streaks=Image(W,H,ColorMode.RGB);local soot=Image(W,H,ColorMode.RGB)
  if #body==0 then return {dents,streaks,soot} end
  local floor_limit=lowest-math.floor((lowest-top)*0.18)
  local count=(tier==1) and 9 or 16
  if W<100 then count=(tier==1) and 4 or 6 end
  local placed,tries=0,0
  while placed<count and tries<4000 do
    tries=tries+1
    local p=body[1+hash(tries,seed,tier)%#body];local x,y=p[1],p[2];local v=im:getPixel(x,y)
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
  if tier==2 then
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
      -- Every mark stays on the base silhouette.
      for j=0,4 do for i2=0,5 do if opaque(x+i2,y+j) then dents:drawPixel(x+i2,y+j,C.rgba[(material=='steel') and 'ink' or 'jade0']) end end end
      for i2=0,5 do if opaque(x+i2,y-1) then dents:drawPixel(x+i2,y-1,C.rgba[(material=='steel') and 'steel1' or 'jade1']) end end
      for j=0,4 do if opaque(x+6,y+j) then dents:drawPixel(x+6,y+j,C.rgba[(material=='steel') and 'steel0' or 'jade1']) end end
      if opaque(x-1,y) then dents:drawPixel(x-1,y,C.rgba[(material=='steel') and 'steel4' or 'jade3']) end
    end
    local sx,sy=(best and best[1] or body[1][1]),(best and best[2] or body[1][2])
    for d=1,6 do local yy=sy-2-d;local xx=sx+2+(d%2);if opaque(xx,yy) and yy>top then soot:drawPixel(xx,yy,C.rgba.contact) end end
  end
  return {dents,streaks,soot}
end
local total=0
for _,b in ipairs({
  {name='union_drydock',W=160,H=144,AX=80,AY=128,material='steel',activity=true},
  {name='assembly_drydock',W=160,H=144,AX=80,AY=128,material='weave',activity=true},
  {name='union_palisade',W=64,H=64,AX=32,AY=50,material='steel',activity=false},
  {name='assembly_palisade',W=64,H=64,AX=32,AY=50,material='weave',activity=false},
}) do
  local n=b.name
  local base=paint(b.W,b.H,b.AX,b.AY,function(c) B[n](c,nil) end)
  C.document_images(n,b.W,b.H,b.AX,b.AY,layers,{{key=n,duration=1,images=base}},'building',m,packer,out,entries)
  local frames={}
  for stage=0,2 do frames[#frames+1]={key=n..'_build_'..stage,duration=1,images=paint(b.W,b.H,b.AX,b.AY,function(c) B.construction(c,n,stage) end)} end
  C.document_images(n..'_construction',b.W,b.H,b.AX,b.AY,layers,frames,{{'foundation',1,1},{'frame',2,2},{'fit_out',3,3}},m,packer,out,entries)
  total=total+1+#frames
  if b.activity then
    local act={}
    for phase=0,3 do act[#act+1]={key=n..'_active_'..phase,duration=.2,images=paint(b.W,b.H,b.AX,b.AY,function(c) B[n](c,phase) end)} end
    C.document_images(n..'_activity',b.W,b.H,b.AX,b.AY,layers,act,'active',m,packer,out,entries)
    total=total+#act
  end
  local dmg={}
  for tier=1,2 do dmg[#dmg+1]={key=n..'_damage_'..tier,duration=1,images=damage(base,b.W,b.H,b.material,tier,#n)} end
  C.document_images(n..'_damage',b.W,b.H,b.AX,b.AY,{'dents and tears','streaks and strands','soot'},dmg,'damage_tiers',m,packer,out,entries)
  total=total+#dmg
end
C.write(root..'/tools/art_v16/sources-buildings.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..total..' building frames; manifest now has '..C.count(m)..' entries.')
