-- Shared v19 helpers. v19 re-authors existing keys in place (same
-- rectangles, same anchors) over the archived v18 atlas; the packer only
-- returns rectangles that already exist and refuses a new key. Adds the
-- engine's walk geometry (how far a machine travels per walk phase on each
-- facing, and how long a phase lasts at full speed) and the role folds that
-- bring the eight original machines inside their colour budget. Root-
-- authored scripted pixel construction: every document it writes stays
-- editable under art/source/v19/.
local M={}
local root=assert(app.params.root)
M.root=root
local painter=dofile(root..'/tools/art_v5/painter.lua')
M.painter=painter
M.palette=painter.palette
M.extra={damp='736c61'}
M.rgba={}
for k,v in pairs(painter.palette) do
 M.rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255)
end
for k,v in pairs(M.extra) do
 M.rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255)
end
M.role_of={}
for k,v in pairs(M.rgba) do M.role_of[v]=k end
function M.read(p) local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
function M.write(p,s) local f=assert(io.open(p,'w'));f:write(s);f:close() end
function M.install_palette(sprite)
 painter.install_palette(sprite)
 local pal=sprite.palettes[1];local n=#pal;pal:resize(n+1);pal:setColor(n,M.rgba.damp)
end
M.ATLAS_W=4096
M.ATLAS_H=8192
M.SOURCE=root..'/art/source/v19/'
M.ARCHIVE=root..'/art/archive/art-v18/'
function M.atlas()
 if not M._atlas then M._atlas=Image{fromFile=M.ARCHIVE..'game-assets.png'} end
 return M._atlas
end
-- The manifest being built: the v19 manifest when a build already wrote
-- one, else the archived v18 manifest. v19 adds no key.
function M.manifest()
 local f=io.open(root..'/art/exports/game-assets-v19.json')
 local text;if f then text=f:read('*a');f:close() else text=M.read(M.ARCHIVE..'game-assets.json') end
 local decoded=json.decode(text)
 local m={width=M.ATLAS_W,height=M.ATLAS_H,sprites={}}
 for k,v in pairs(decoded.sprites) do
  local b={x=v.x,y=v.y,w=v.w,h=v.h,anchor_x=v.anchor_x,anchor_y=v.anchor_y}
  if v.muzzle then b.muzzle={v.muzzle[1],v.muzzle[2]} end
  m.sprites[k]=b
 end
 local packer={}
 function packer:alloc(key,w,h,ax,ay)
  local b=assert(m.sprites[key],key..' is new: v19 only re-authors existing keys')
  assert(b.w==w and b.h==h,key..' size changed')
  assert(b.anchor_x==ax and b.anchor_y==ay,key..' anchor changed')
  return b
 end
 return m,packer
end
function M.sprite_image(m,key)
 local b=assert(m.sprites[key],key..' not in manifest')
 return Image(M.atlas(),Rectangle(b.x,b.y,b.w,b.h)),b
end
function M.count(m) local n=0;for _ in pairs(m.sprites) do n=n+1 end;return n end
function M.save_manifest(m) M.write(root..'/art/exports/game-assets-v19.json',json.encode(m)) end

-- The engine's walk. bw_desktop advances a machine's walk phase once per
-- quarter cell of Manhattan travel (game.rs `walking_phase`); since rules 16
-- a machine walks the same distance per tick in every direction
-- (navigation.rs `move_towards`), and the camera projects a cell
-- to (16,8) screen pixels. On a screen-axis facing (even painter face) a
-- quarter cell of Manhattan travel is 4 screen pixels, 4 painter units; on
-- a screen-diagonal facing (odd face) it is (4,2) pixels, 4*sqrt(2) units.
-- A planted foot must move back exactly this far per phase, or it skates.
function M.travel(face) return face%2==0 and 4 or 4*math.sqrt(2) end
-- How long one phase lasts at full speed on dry ground, in milliseconds:
-- speed is cells per second (bw_content `spec`); on an even face both cell
-- axes move at speed/sqrt(2), so Manhattan travel is sqrt(2) times speed.
-- (Before rules 16 a corner step was Chebyshev: both axes at full speed.)
M.speed={hook=3,riveter=3,bulwark=2,sounder=5,wick=3,skipper=5,reedguard=3,loom=2,
 tidewatch=6,lampwright=6,caulker=3,tender=3,caisson=2,dredger=2,barge=4,lifter=3}
function M.phase_ms(name,face)
 local per_second=assert(M.speed[name],name)*4*(face%2==0 and math.sqrt(2) or 1)
 return math.floor(1000/per_second+.5)
end

-- Colour folds for the eight original machines (role -> role, applied to
-- the finished drawing so the v4 overpaint masks still see the roles they
-- were written for). The Union four drop to 15 colours and ink: the ivory
-- shadow joins the steel shadow, steel's top light joins ivory's, the glass
-- glint joins steel's light. The Assembly four fold to 13-15 and ink: jade
-- carries the darks the steel and reed ramps duplicated, the dull reed and
-- amber lows merge, rust's middle becomes the amber it sat beside.
M.folds={
 union={ivory0='steel1',steel4='ivory4',glass3='steel3'},
 assembly={steel0='jade0',steel1='jade1',steel4='jade4',ivory0='jade1',ivory1='steel2',ivory4='jade4',
  reed0='jade1',reed4='ivory3',amber0='reed1',amber1='reed1',amber4='amber3',glass2='steel2',glass3='steel3',rust2='amber2'},
}
M.faction={hook='union',riveter='union',bulwark='union',sounder='union',
 wick='assembly',skipper='assembly',reedguard='assembly',loom='assembly'}
function M.fold_pixels(role_map)
 local map={}
 for from,to in pairs(role_map) do map[assert(M.rgba[from],from)]=assert(M.rgba[to],to) end
 return map
end

-- The v18 colour budget (tools/art_v18/common.lua `budget`, verbatim): the
-- v18 roles still draw through it, so their rest pose stays v18's.
local families={'steel','ivory','rust','jade','reed','amber','glass','earth','sea','ui'}
function M.budget(allowed,fallback)
 fallback=fallback or {}
 local map={}
 for _,fam in ipairs(families) do
  local target=fam
  if not allowed[fam] then target=fallback[fam] end
  for shade=0,4 do
   local role=fam..shade
   if target and allowed[target] then
    local best,bd
    for _,s in ipairs(allowed[target]) do
     local d=math.abs(s-shade)
     if not bd or d<bd or (d==bd and s<best) then best,bd=s,d end
    end
    local to=target..best
    if to~=role then map[role]=to end
   end
  end
 end
 return map
end

-- Count opaque colours in a list of layer images, skipping the shadow roles.
local A=app.pixelColor.rgbaA
function M.colours(images,skip_shadow)
 local seen,n={},0
 for _,im in ipairs(images) do
  for y=0,im.height-1 do for x=0,im.width-1 do
   local v=im:getPixel(x,y)
   if A(v)>0 and not seen[v] then
    local r=M.role_of[v]
    if not (skip_shadow and (r=='cast' or r=='contact')) then seen[v]=true;n=n+1 end
   end
  end end
 end
 return n,seen
end
-- Save a document built from composed layer images and register every frame.
function M.document_images(name,w,h,ax,ay,layers,frames,tag,m,packer,outdir,entries)
 local s=Sprite(w,h,ColorMode.RGB);M.install_palette(s)
 for i,n in ipairs(layers) do local l=i==1 and s.layers[1] or s:newLayer();l.name=n end
 for f,spec in ipairs(frames) do
  if f>1 then s:newEmptyFrame(f) end;s.frames[f].duration=spec.duration or 1
  for i=1,#layers do s:newCel(s.layers[i],f,spec.images[i],Point(0,0)) end
  local b=packer:alloc(spec.key,w,h,ax,ay)
  local e={key=spec.key,document=name,frame=f,w=w,h=h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers}
  if spec.muzzle then e.muzzle=spec.muzzle;b.muzzle=spec.muzzle end
  entries[#entries+1]=e
  if outdir then local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f);frame:saveAs(outdir..'/'..spec.key..'.png') end
 end
 if tag then
  if type(tag)=='table' then for _,t in ipairs(tag) do s:newTag(t[2],t[3]).name=t[1] end
  else s:newTag(1,#frames).name=tag end
 end
 s:saveAs(M.SOURCE..name..'.aseprite');s:close()
end
-- Grounded shadow for a machine frame (the v15 method, unchanged): the
-- authored oval under the footprint, a contact strip under the parts that
-- reach the ground, and a short flat cast leaning right with the key light.
function M.grounded_shadow(bodies,oval,w,h)
 local KX,KY=0.30,0.10
 local body=Image(w,h,ColorMode.RGB)
 for _,im in ipairs(bodies) do body:drawImage(im,Point(0,0)) end
 local g=-1;local bottom={}
 for y=0,h-1 do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then if y>g then g=y end;bottom[x]=y end
 end end
 local outim=Image(w,h,ColorMode.RGB)
 if g<0 then return outim end
 local mask={}
 local function mark(x,y) if x>=0 and y>=0 and x<w and y<h then mask[y*w+x]=true end end
 if oval then for y=0,h-1 do for x=0,w-1 do if A(oval:getPixel(x,y))>0 then mark(x,y) end end end end
 for x,y in pairs(bottom) do if y>=g-3 then for oy=1,2 do mark(x,y+oy);mark(x+1,y+oy) end end end
 for y=0,g do for x=0,w-1 do
  if A(body:getPixel(x,y))>0 then
   local hgt=g-y
   local sx=x+math.floor(hgt*KX+.5);local sy=g+math.floor(hgt*KY+.5)
   mark(sx,sy);mark(sx+1,sy)
  end
 end end
 local keep={}
 for k in pairs(mask) do
  local x,y=k%w,math.floor(k/w);local n=0
  for j=-1,1 do for i=-1,1 do if (i~=0 or j~=0) and mask[(y+j)*w+(x+i)] then n=n+1 end end end
  if n>=2 then keep[k]=true end
 end
 for k in pairs(keep) do local x,y=k%w,math.floor(k/w);if A(body:getPixel(x,y))==0 then outim:drawPixel(x,y,M.rgba.cast) end end
 return outim
end
function M.shifted(im,dx,dy)
 local o=Image(im.width,im.height,ColorMode.RGB)
 for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y)
  if A(v)>0 and x+dx>=0 and y+dy>=0 and x+dx<im.width and y+dy<im.height then o:drawPixel(x+dx,y+dy,v) end
 end end
 return o
end
function M.recolour(im,map)
 local o=Image(im.width,im.height,ColorMode.RGB)
 for y=0,im.height-1 do for x=0,im.width-1 do local v=im:getPixel(x,y);if A(v)>0 then o:drawPixel(x,y,map[v] or v) end end end
 return o
end
-- A layer of a document frame as a full-canvas image.
function M.full(layer,f,w,h)
 local im=Image(w,h,ColorMode.RGB);local cel=layer:cel(f)
 if cel then im:drawImage(cel.image,cel.position) end
 return im
end
return M
