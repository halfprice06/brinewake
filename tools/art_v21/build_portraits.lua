-- Root authoring entry point for the Saltglass Compact's console art: the
-- eight machine portraits and the faction portrait (56x56), the faction
-- badge, emblem, HQ mark and plate, the HUD bars and the wreck. A machine
-- portrait is the console close-up of the v4 vocabulary: the faction plate
-- behind the machine built again at a larger scale (painter `scale`; the
-- drawing is constructed at that size, nothing is resampled), cropped to
-- its working end.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local painter=C.painter
local layers=painter.layers
local out=C.OUT..'console';os.execute('mkdir -p '..out..' '..C.SOURCE)
local atlas,m,packer=C.open_atlas()
local ctx={atlas=atlas,packer=packer,entries={},out=out}
local function new(W,H,opts)
  local ims={}
  for _,l in ipairs(layers) do local im=Image(W,H,ColorMode.RGB);im:clear();ims[l]=im end
  local b={key='console',x=0,y=0,w=W,h=H,anchor_x=opts and opts.ax or 0,anchor_y=opts and opts.ay or 0,clip=true,
    scale=opts and opts.scale,remap=opts and opts.remap}
  local c=painter.new(ims,b,opts and opts.face or 0)
  local function list() local l={};for i,n in ipairs(layers) do l[i]=ims[n] end;return l end
  return c,list
end
local function P(c,x,y) return {x,y} end
local function poly(c,pts,col) c:poly(pts,col) end
local function line(c,x,y,xx,yy,col,w) c:line({x,y},{xx,yy},col,w) end
local function rect(c,x,y,w,h,col) c:poly({{x,y},{x+w,y},{x+w,y+h},{x,y+h}},col) end
local function px(c,x,y,col) c:pixel(x,y,col) end
local font=json.decode(C.read(root..'/tools/art_v8/font7.json'))
local function text(c,s,x,y,col)
  for i=1,#s do for yy,row in ipairs(font[s:sub(i,i)] or font['?']) do for xx=1,7 do
    if row:sub(xx,xx)=='1' then c:pixel(x+(i-1)*8+xx-1,y+yy-1,col) end
  end end end
end

-- The console plate (v17 portraits.lua), with the Compact's violet mark.
local function plate(c)
  c:layer(2);rect(c,0,0,56,56,'ui0')
  poly(c,{{1,1},{47,1},{55,9},{55,55},{1,55}},'ui1')
  poly(c,{{1,1},{29,1},{1,30}},'ui2')
  for y=8,51,12 do line(c,3,y,52,y,'ui1') end
  rect(c,47,4,5,2,'glaze4')
end

-- Portrait framing: the machine on its three-quarter facing, built at the
-- largest scale that fits its width in the plate (at most 2), placed so its
-- top clears the plate by four pixels. Tall machines run off the bottom of
-- the plate: the portrait is of the working end.
local FACE={raker=7,brander=7,heliostat=6,glinter=5,stilt=1,glazier=7,salter=7,pan=7}
local A=app.pixelColor.rgbaA
local function bbox(l)
  local x0,y0,x1,y1=1e9,1e9,-1,-1
  for i=2,#l do local im=l[i]
    for y=0,im.height-1 do for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then
      if x<x0 then x0=x end;if x>x1 then x1=x end;if y<y0 then y0=y end;if y>y1 then y1=y end
    end end end
  end
  return x0,y0,x1,y1
end
for n,face in pairs(FACE) do
  local c0,l0=new(240,240,{face=face,scale=1,ax=120,ay=180,remap=M.BUDGET})
  M[n](c0,{})
  local x0,y0,x1=bbox(l0())
  local scale=math.min(2,44/(x1-x0+1))
  local ax=math.floor(28-((x0+x1)/2-120)*scale+.5)
  local ay=math.floor(6+(180-y0)*scale+.5)
  local c,list=new(56,56)
  plate(c)
  local c2,list2=new(56,56,{face=face,scale=scale,ax=ax,ay=ay,remap=M.BUDGET})
  M[n](c2,{})
  local l1,l2=list(),list2()
  for i=2,#l1 do l1[i]:drawImage(l2[i],Point(0,0)) end
  C.document(ctx,'portrait_'..n,56,56,0,0,layers,{{key='portrait_'..n,duration=1,images=l1}},'portrait')
end

-- The faction portrait: the Kiln's mouth under a mirror, on the plate.
do
  local c,list=new(56,56)
  plate(c)
  c:layer(4)
  -- Two mirrors on the plate, the sun between them.
  c:ellipse(28,24,17,17,'glaze1');c:ellipse(28,24,15,15,'glaze2')
  c:ellipse(27,23,12,12,'glass1');c:ellipse(25,21,8,8,'glass2');c:ellipse(23,19,4,4,'glass3')
  line(c,17,32,36,14,'glass3');line(c,19,33,37,16,'glass2')
  px(c,19,13,'glint');px(c,20,12,'glint');px(c,18,14,'glass3')
  c:ellipse(28,24,17,17,'glaze1')
  -- Redraw the dish over the ring: the ring is the rim.
  c:ellipse(28,24,15,15,'rust1');c:ellipse(28,24,14,14,'glass1');c:ellipse(26,22,10,10,'glass2');c:ellipse(24,20,5,5,'glass3')
  line(c,16,30,36,14,'glass3');px(c,21,15,'glint');px(c,22,14,'glint');px(c,20,16,'glint')
  -- The yoke and the salt crust at its foot.
  line(c,28,40,28,48,'glaze3',2);line(c,14,26,20,44,'rust2',2);line(c,42,26,36,44,'rust2',2)
  rect(c,10,46,36,5,'crust1');rect(c,10,46,36,1,'crust2');rect(c,10,50,36,1,'crust0')
  C.document(ctx,'portrait_compact',56,56,0,0,layers,{{key='portrait_compact',duration=1,images=list()}},'portrait')
end

-- The faction mark: a mirror dish on a stilt, drawn at three sizes.
local function badge(c)
  -- 16x12: the dish and its stilt in a violet tab.
  c:layer(3);rect(c,0,0,16,12,'glaze1');rect(c,1,1,14,10,'glaze2')
  c:ellipse(8,5,4,3,'crust1');c:ellipse(8,5,3,2,'glass2');px(c,6,4,'glint');px(c,7,4,'glass3')
  line(c,8,8,8,10,'crust2');line(c,5,10,11,10,'crust1')
end
local function emblem(c)
  -- 24x24: a salt-white disc, the dish on its fork and a lens glint.
  c:layer(3);c:ellipse(12,12,11,11,'glaze0');c:ellipse(12,12,10,10,'glaze2');c:ellipse(11,11,8,8,'glaze3')
  c:ellipse(12,10,7,5,'crust1');c:ellipse(12,10,6,4,'glass1');c:ellipse(11,9,4,3,'glass2');line(c,8,11,14,7,'glass3')
  px(c,9,7,'glint');px(c,10,7,'glint')
  line(c,6,11,11,17,'rust2');line(c,18,11,13,17,'rust2')
  line(c,12,16,12,20,'crust2');line(c,8,20,16,20,'crust1')
end
-- The sign at 14 pixels (the others' HQ mark and plate use their sign at
-- this size): the dish on its stilt in a violet ring.
local function sign14(c,x,y)
  c:ellipse(x+7,y+7,7,7,'glaze0');c:ellipse(x+7,y+7,6,6,'glaze2');c:ellipse(x+6,y+6,5,5,'glaze3')
  c:ellipse(x+7,y+5,4,3,'crust1');c:ellipse(x+7,y+5,3,2,'glass1');px(c,x+6,y+4,'glass3');px(c,x+5,y+4,'glint')
  line(c,x+7,y+8,x+7,y+11,'crust2');line(c,x+5,y+11,x+9,y+11,'crust1')
end
local function hq(c)
  -- 24x16, anchor (12,14).
  c:layer(3);sign14(c,5,1)
  c:layer(6);px(c,4,6,'crust2');px(c,19,6,'crust2')
end
local function faction_plate(c)
  -- 96x24: the badge's dish and the side's name block in violet and salt.
  c:layer(2);rect(c,0,0,96,24,'ui0');rect(c,1,1,94,22,'glaze1');rect(c,2,2,92,20,'ui1')
  line(c,2,2,93,2,'glaze3');line(c,2,21,93,21,'glaze0')
  c:layer(3);sign14(c,5,6)
  c:layer(6);text(c,'COMPACT',27,8,'crust1')
end
for _,s in ipairs({
  {'faction_compact_badge',16,12,0,0,badge},
  {'faction_compact_emblem',24,24,0,0,emblem},
  {'faction_compact_hq',24,16,12,14,hq},
  {'faction_compact_plate',96,24,0,0,faction_plate},
}) do
  local c,list=new(s[2],s[3])
  s[6](c)
  C.document(ctx,s[1],s[2],s[3],s[4],s[5],layers,{{key=s[1],duration=1,images=list()}},'mark')
end

-- The HUD bars: the Union's and the Assembly's bars are the same frame with
-- the side's trim; the Compact takes the Union bar and swaps its trim for
-- violet and salt, pixel for pixel (copper accents to glaze, rivets to crust).
local function trim_copy(src,dst,map)
  local b=m.sprites[src]
  local im=Image(b.w,b.h,ColorMode.RGB)
  for y=0,b.h-1 do for x=0,b.w-1 do
    local v=atlas:getPixel(b.x+x,b.y+y)
    im:drawPixel(x,y,map[v] or v)
  end end
  local list={im};for i=2,#layers do local e=Image(b.w,b.h,ColorMode.RGB);e:clear();list[i]=e end
  C.document(ctx,dst,b.w,b.h,b.anchor_x,b.anchor_y,layers,{{key=dst,duration=1,images=list}},'bar')
end
local trim=C.pixmap{rust0='glaze0',rust1='glaze1',rust2='glaze3',rust3='glaze4',rust4='crust2',
  ivory0='glaze1',ivory1='crust0',ivory2='crust1',ivory3='crust1',ivory4='crust2',
  amber1='glaze2',amber2='glaze3',amber3='glaze4',amber4='crust2'}
trim_copy('hud_top_union','hud_top_compact',trim)
trim_copy('hud_bottom_union','hud_bottom_compact',trim)

-- The wreck: a Compact machine fallen, stilts broken, a cracked dish and
-- white salt spilled round it (64x64, anchor 32,50 as the others).
do
  local c,list=new(64,64,{ax=32,ay=50,face=1,remap=M.BUDGET})
  c:layer(2)
  c:poly3({{-14,-6,0},{12,-9,0},{16,6,0},{-10,10,0}},'crust1')
  c:poly3({{-10,-3,0},{8,-6,0},{11,4,0},{-7,7,0}},'crust0')
  c:layer(3)
  c:box(0,0,0,8,5,4,'glaze',2)
  c:line3({-8,5,4},{8,5,4},'glaze0')
  c:layer(4)
  -- Broken stilts splayed on the ground.
  c:rod({6,6,1},{18,12,0},'glaze1','glaze3');c:rod({-6,-6,1},{-18,-10,0},'glaze1','glaze3')
  c:rod({-4,6,1},{-12,16,0},'glaze1','glaze3')
  c:dot1(18,12,0,'crust2');c:dot1(-18,-10,0,'crust2')
  -- The dish, cracked, tipped against the hull.
  M.mirror(c,{-2,-10,6},{0.1,0.5,0.85},6,{rim='rust1'})
  local p=c:point(-2,-10,6);c:line({p[1]-3,p[2]-1},{p[1]+2,p[2]+2},'glaze0')
  c:dot1(4,-2,6,'glint')
  C.document(ctx,'wreck_compact',64,64,32,50,layers,{{key='wreck_compact',duration=1,images=list()}},'wreck')
end

table.sort(ctx.entries,function(a,b) return a.key<b.key end)
C.write(root..'/tools/art_v21/sources-console.json',json.encode(ctx.entries))
C.save_atlas(atlas,m)
print('Authored '..#ctx.entries..' console frames.')
