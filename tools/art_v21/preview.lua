-- Draft preview: draws one machine's rest pose on all eight facings and its
-- walk on two, into one strip PNG, without touching the atlas. For review
-- while drawing; the build is build_machines.lua.
-- Usage: aseprite -b --script-param root=$PWD --script-param name=stilt
--          --script-param out=<png> [--script-param mode=fire]
--          --script tools/art_v21/preview.lua
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local name=assert(app.params.name)
local W,H,AX,AY=64,64,32,50
local layers=C.painter.layers
local function paint(face,st)
  local ims={}
  for _,l in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[l]=im end
  local c=C.painter.new(ims,{key=name,x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=M.BUDGET},face)
  M[name](c,st)
  c.dz=0
  local list={};for i,l in ipairs(layers) do list[i]=ims[l] end
  local bodies={};for i=2,#list do bodies[#bodies+1]=list[i] end
  list[1]=C.grounded_shadow(bodies,list[1],W,H)
  return C.composite(list)
end
local rows={}
local mode=app.params.mode
local k=tonumber(app.params.k or '0')
-- Row 1: rest (or the named mode) on every facing. Rows 2-3: walk on faces 1 and 6.
local r1={};for f=0,7 do r1[#r1+1]=paint(f,{mode=mode,k=k}) end
rows[1]=r1
for _,f in ipairs({tonumber(app.params.wa or '1'),tonumber(app.params.wb or '6')}) do
  local r={};for p=0,3 do r[#r+1]=paint(f,{phase=p}) end;rows[#rows+1]=r
end
local sheet=Image(W*8,H*#rows,ColorMode.RGB)
local bg=app.pixelColor.rgba(0x9d,0x8f,0x76,255)
for y=0,sheet.height-1 do for x=0,sheet.width-1 do sheet:drawPixel(x,y,bg) end end
for j,r in ipairs(rows) do for i,im in ipairs(r) do sheet:drawImage(im,Point((i-1)*W,(j-1)*H)) end end
sheet:saveAs(assert(app.params.out))
