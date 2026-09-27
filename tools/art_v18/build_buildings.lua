-- Root authoring entry point for the v18 buildings: the Drydocks and the
-- Palisades of both factions rebuilt in place (base, construction, activity,
-- damage), the Drydocks inside a colour budget and the Palisades as wall
-- segments, with the v18 damage marks. Nothing is drawn beneath a building.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v18/common.lua')
local D=dofile(root..'/tools/art_v18/damage.lua')
local painter=C.painter
local B=dofile(root..'/tools/art_v18/buildings.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v18/buildings';os.execute('mkdir -p '..out)
local layers=painter.layers
local A=app.pixelColor.rgbaA
local budgets={
  union_drydock={allowed={rust={0,1,2,3,4},ivory={1,2,3,4},steel={0,1,2,3},glass={0,1,3},earth={1,2,3}},fallback={amber='rust',reed='ivory',jade='steel'}},
  assembly_drydock={allowed={jade={0,1,2,3,4},reed={0,1,2,3},amber={1,2,3},steel={0,1,2,3},earth={1,2,3}},fallback={rust='reed',ivory='reed',glass='steel'}},
  union_palisade={allowed={steel={0,1,2,3,4},ivory={1,2,4},rust={2,3},earth={1,2,3}},fallback={}},
  assembly_palisade={allowed={reed={0,1,2,3,4},jade={1,2,3,4},earth={1,2,3}},fallback={steel='jade',rust='reed'}},
}
local function paint(name,W,H,AX,AY,draw)
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[layer]=im end
  local b=budgets[name]
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=b and C.budget(b.allowed,b.fallback) or nil},0)
  draw(c)
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  return list
end
local function composite(list,W,H) local im=Image(W,H,ColorMode.RGB);for i=2,#list do im:drawImage(list[i],Point(0,0)) end;return im end
local total=0
local report={}
for _,b in ipairs({
  {name='union_drydock',W=160,H=144,AX=80,AY=128,material='steel',activity=true},
  {name='assembly_drydock',W=160,H=144,AX=80,AY=128,material='weave',activity=true},
  {name='union_palisade',W=64,H=64,AX=32,AY=50,material='steel',activity=false},
  {name='assembly_palisade',W=64,H=64,AX=32,AY=50,material='weave',activity=false},
}) do
  local n=b.name
  local base=paint(n,b.W,b.H,b.AX,b.AY,function(c) B[n](c,nil) end)
  local cnt=C.colours({base[2],base[3],base[4],base[5],base[6]},true);report[#report+1]=n..' colours '..cnt
  C.document_images(n,b.W,b.H,b.AX,b.AY,layers,{{key=n,duration=1,images=base}},'building',m,packer,out,entries)
  local frames={}
  for stage=0,2 do frames[#frames+1]={key=n..'_build_'..stage,duration=1,images=paint(n,b.W,b.H,b.AX,b.AY,function(c) B.construction(c,n,stage) end)} end
  C.document_images(n..'_construction',b.W,b.H,b.AX,b.AY,layers,frames,{{'foundation',1,1},{'frame',2,2},{'fit_out',3,3}},m,packer,out,entries)
  total=total+1+#frames
  if b.activity then
    local act={}
    for phase=0,3 do act[#act+1]={key=n..'_active_'..phase,duration=.2,images=paint(n,b.W,b.H,b.AX,b.AY,function(c) B[n](c,phase) end)} end
    C.document_images(n..'_activity',b.W,b.H,b.AX,b.AY,layers,act,'active',m,packer,out,entries)
    total=total+#act
  end
  local dmg={}
  local small=b.W<100
  for tier=1,2 do
    local opts=small and {count=(tier==1) and 1 or 2,spacing=8} or {count=(tier==1) and 4 or 6,spacing=11}
    dmg[#dmg+1]={key=n..'_damage_'..tier,duration=1,images=D.marks(C,composite(base,b.W,b.H),b.material,tier,#n,opts)}
  end
  C.document_images(n..'_damage',b.W,b.H,b.AX,b.AY,{'dents and tears','streaks and strands','soot'},dmg,'damage_tiers',m,packer,out,entries)
  total=total+#dmg
end
C.write(root..'/tools/art_v18/sources-buildings.json',json.encode(entries))
C.save_manifest(m)
print(table.concat(report,'; '))
print('Authored '..total..' building frames; manifest has '..C.count(m)..' entries.')
