-- Root authoring entry point for the Saltglass Compact's buildings
-- (buildings.lua): base, construction, activity, lamps and damage, with the
-- same key sets as the Union's and the Assembly's buildings. New keys only;
-- nothing is drawn beneath a building. Writes editable documents under
-- art/source/v21/, review PNGs under output/art-v21/buildings/ and the
-- registry tools/art_v21/sources-buildings.json.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local D=dofile(root..'/tools/art_v21/damage.lua')
local B=dofile(root..'/tools/art_v21/buildings.lua')
local painter=C.painter
local layers=painter.layers
local out=C.OUT..'buildings';os.execute('mkdir -p '..out..' '..C.SOURCE)
local atlas,m,packer=C.open_atlas()
local ctx={atlas=atlas,packer=packer,entries={},out=out}
local report={}
local total=0
local function paint(W,H,AX,AY,draw)
  local ims={}
  for _,l in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[l]=im end
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=B.BUDGET},1)
  draw(c)
  local list={};for i,l in ipairs(layers) do list[i]=ims[l] end
  return list
end
local function blanks(W,H) local o={};for i=1,#layers do local im=Image(W,H,ColorMode.RGB);im:clear();o[i]=im end;return o end
for _,b in ipairs({
  {name='compact_hq',W=160,H=144,AX=80,AY=128,active=true,lamps=true,build=false},
  {name='compact_works',W=160,H=144,AX=80,AY=128,active=true,lamps=true,build=true},
  {name='compact_drydock',W=160,H=144,AX=80,AY=128,active=true,build=true},
  {name='compact_dropoff',W=160,H=144,AX=80,AY=128,build=true},
  {name='compact_tower',W=160,H=144,AX=80,AY=128,build=true},
  {name='compact_palisade',W=64,H=64,AX=32,AY=50,build=true},
}) do
  local n,W,H,AX,AY=b.name,b.W,b.H,b.AX,b.AY
  if app.params.debug then print(n) end
  local base=paint(W,H,AX,AY,function(c) B[n](c,nil) end)
  local cnt=C.colours({base[2],base[3],base[4],base[5],base[6]},true)
  report[#report+1]=n..' '..cnt..' colours'
  C.document(ctx,n,W,H,AX,AY,layers,{{key=n,duration=1,images=base}},'building')
  total=total+1
  if b.build then
    local frames={}
    for stage=0,2 do frames[#frames+1]={key=n..'_build_'..stage,duration=1,images=paint(W,H,AX,AY,function(c) B.construction(c,n,stage) end)} end
    C.document(ctx,n..'_construction',W,H,AX,AY,layers,frames,{{'foundation',1,1},{'frame',2,2},{'fit_out',3,3}})
    total=total+#frames
  end
  if b.active then
    local act={}
    for phase=0,3 do act[#act+1]={key=n..'_active_'..phase,duration=.2,images=paint(W,H,AX,AY,function(c) B[n](c,phase) end)} end
    C.document(ctx,n..'_activity',W,H,AX,AY,layers,act,'active')
    total=total+#act
  end
  if b.lamps then
    C.document(ctx,n..'_lamps',W,H,AX,AY,layers,{{key=n..'_lamps',duration=1,images=paint(W,H,AX,AY,function(c) B[n..'_lamps'](c) end)}},'lamps')
    total=total+1
  end
  local dmg={}
  local small=W<100
  local body=C.composite(base,2)
  for tier=1,2 do
    local opts=small and {count=(tier==1) and 1 or 2,spacing=8} or {count=(tier==1) and 4 or 6,spacing=11}
    dmg[#dmg+1]={key=n..'_damage_'..tier,duration=1,images=D.marks(C,body,tier,#n,opts)}
  end
  C.document(ctx,n..'_damage',W,H,AX,AY,{'chips','cracks','soot'},dmg,'damage_tiers')
  total=total+#dmg
end
table.sort(ctx.entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
C.write(root..'/tools/art_v21/sources-buildings.json',json.encode(ctx.entries))
C.save_atlas(atlas,m)
print(table.concat(report,'; '))
print('Authored '..total..' building frames.')
