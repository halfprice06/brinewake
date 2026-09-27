-- Shared v18 helpers: palette access, per-sprite colour budgets, exact
-- copies of archived sprites, and the packer over the archived v17 manifest.
-- v18 re-authors existing keys in place (same rectangles, same anchors), so
-- the packer only ever returns rectangles that already exist. Root-authored
-- scripted pixel construction; every document it writes stays editable
-- under art/source/v18/.
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
M.RIGHT=2048
M.SOURCE=root..'/art/source/v18/'
function M.atlas()
 if not M._atlas then M._atlas=Image{fromFile=root..'/art/archive/art-v17/game-assets.png'} end
 return M._atlas
end
function M.manifest()
 local f=io.open(root..'/art/exports/game-assets-v18.json')
 local text;if f then text=f:read('*a');f:close() else text=M.read(root..'/art/archive/art-v17/game-assets.json') end
 local decoded=json.decode(text)
 local m={width=M.ATLAS_W,height=M.ATLAS_H,sprites={}}
 for k,v in pairs(decoded.sprites) do
  local b={x=v.x,y=v.y,w=v.w,h=v.h,anchor_x=v.anchor_x,anchor_y=v.anchor_y}
  if v.muzzle then b.muzzle={v.muzzle[1],v.muzzle[2]} end
  m.sprites[k]=b
 end
 local maxy,maxh=0,0
 for _,b in pairs(m.sprites) do
  if b.x>=M.RIGHT then
   if b.y>maxy then maxy=b.y;maxh=b.h elseif b.y==maxy and b.h>maxh then maxh=b.h end
  end
 end
 local packer={x=M.RIGHT,y=0,h=0}
 if maxh>0 then
  local rowx=M.RIGHT
  for _,b in pairs(m.sprites) do if b.x>=M.RIGHT and b.y==maxy and b.x+b.w>rowx then rowx=b.x+b.w end end
  packer.x=rowx;packer.y=maxy;packer.h=maxh
 end
 function packer:alloc(key,w,h,ax,ay)
  if m.sprites[key] then
   local b=m.sprites[key];assert(b.w==w and b.h==h,key..' size changed');return b
  end
  if self.x+w>M.ATLAS_W then self.x=M.RIGHT;self.y=self.y+self.h;self.h=0 end
  assert(self.y+h<=M.ATLAS_H,'v18 atlas overflow at '..key)
  local b={x=self.x,y=self.y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  m.sprites[key]=b;self.x=self.x+w;if h>self.h then self.h=h end
  return b
 end
 return m,packer
end
-- Exact copy of a sprite from the archived v17 atlas.
function M.sprite_image(m,key)
 local b=assert(m.sprites[key],key..' not in manifest')
 return Image(M.atlas(),Rectangle(b.x,b.y,b.w,b.h)),b
end
function M.sprite_image_any(m,key,atlas)
 local b=assert(m.sprites[key],key..' not in manifest')
 return Image(atlas,Rectangle(b.x,b.y,b.w,b.h)),b
end
function M.count(m) local n=0;for _ in pairs(m.sprites) do n=n+1 end;return n end
function M.save_manifest(m) M.write(root..'/art/exports/game-assets-v18.json',json.encode(m)) end

-- Colour budget. `allowed` lists the shades each family may use, for example
-- {rust={0,1,2,3,4},ivory={1,2,3},steel={0,1,2,3},glass={1,4}}; `fallback`
-- names the family a missing family folds into (amber->rust ...). Every other
-- role maps to the nearest allowed shade of its family. `ink`, `contact`,
-- `cast` always pass. Returns the role->role table the painter applies.
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
-- Shift an image by whole pixels, dropping nothing inside the canvas.
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
return M
