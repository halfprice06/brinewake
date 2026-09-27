-- Shared v17 helpers: palette access, exact atlas copies of existing
-- sprites, and an append-only packer that grows the atlas to 4096x8192 and
-- fills its right half. Root-authored scripted pixel construction; every
-- document it writes stays artist-editable under art/source/v17/.
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
function M.read(p) local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
function M.write(p,s) local f=assert(io.open(p,'w'));f:write(s);f:close() end
function M.install_palette(sprite)
 painter.install_palette(sprite)
 local pal=sprite.palettes[1];local n=#pal;pal:resize(n+1);pal:setColor(n,M.rgba.damp)
end
M.ATLAS_W=4096
M.ATLAS_H=8192
M.RIGHT=2048
-- The archived v16 atlas is the source for exact sprite copies.
function M.atlas()
 if not M._atlas then M._atlas=Image{fromFile=root..'/art/archive/art-v16/game-assets.png'} end
 return M._atlas
end
-- Packer over the archived v16 manifest, continued from the v17 manifest
-- when an earlier v17 script already appended rectangles. Existing keys keep
-- their rectangles; new keys pack into the right half from its top.
function M.manifest()
 local f=io.open(root..'/art/exports/game-assets-v17.json')
 local text;if f then text=f:read('*a');f:close() else text=M.read(root..'/art/archive/art-v16/game-assets.json') end
 local decoded=json.decode(text)
 -- Decoded JSON values are Aseprite objects: copy them into plain tables so
 -- iteration and re-encoding behave.
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
 -- Continue on the last used row of the right half.
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
  assert(self.y+h<=M.ATLAS_H,'v17 atlas overflow at '..key)
  local b={x=self.x,y=self.y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  m.sprites[key]=b;self.x=self.x+w;if h>self.h then self.h=h end
  return b
 end
 return m,packer
end
-- Exact copy of an existing sprite from the archived atlas.
function M.sprite_image(m,key)
 local b=assert(m.sprites[key],key..' not in manifest')
 assert(b.x<M.RIGHT,key..' is not an archived sprite')
 return Image(M.atlas(),Rectangle(b.x,b.y,b.w,b.h)),b
end
-- A small painter in anchor-relative coordinates for overlays.
function M.canvas(ims,w,h,ax,ay,key)
 local c={ims=ims,im=ims[1],w=w,h=h,ox=ax,oy=ay,key=key}
 function c:layer(i) self.im=assert(self.ims[i]) end
 function c:pixel(x,y,col)
  x,y=math.floor(x+.5),math.floor(y+.5)
  local px,py=x+self.ox,y+self.oy
  if px<0 or py<0 or px>=self.w or py>=self.h then return end
  self.im:drawPixel(px,py,assert(M.rgba[col],'unknown colour '..tostring(col)))
 end
 function c:raw(x,y,value)
  x,y=math.floor(x+.5),math.floor(y+.5)
  local px,py=x+self.ox,y+self.oy
  if px<0 or py<0 or px>=self.w or py>=self.h then return end
  self.im:drawPixel(px,py,value)
 end
 function c:line(x,y,xx,yy,col)
  x,y,xx,yy=math.floor(x+.5),math.floor(y+.5),math.floor(xx+.5),math.floor(yy+.5)
  local dx,dy=math.abs(xx-x),-math.abs(yy-y);local sx,sy=x<xx and 1 or -1,y<yy and 1 or -1;local err=dx+dy
  while true do self:pixel(x,y,col);if x==xx and y==yy then break end
   local e=2*err;if e>=dy then err=err+dy;x=x+sx end;if e<=dx then err=err+dx;y=y+sy end
  end
 end
 function c:poly(p,col)
  local lo,hi=1e9,-1e9;for _,v in ipairs(p) do lo=math.min(lo,v[2]);hi=math.max(hi,v[2]) end
  for y=math.ceil(lo),math.ceil(hi)-1 do local xs={};local j=#p
   for i,a in ipairs(p) do local b=p[j];if(a[2]<=y and b[2]>y)or(b[2]<=y and a[2]>y)then xs[#xs+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2]) end;j=i end
   table.sort(xs);for i=1,#xs-1,2 do local l,r=math.ceil(xs[i]),math.ceil(xs[i+1])-1;if l<=r then self:line(l,y,r,y,col) end end
  end
 end
 function c:ellipse(x,y,rx,ry,col)
  for dy=-ry,ry do local span=math.floor(rx*math.sqrt(math.max(0,1-dy*dy/(ry*ry))));self:line(x-span,y+dy,x+span,y+dy,col) end
 end
 return c
end
-- Save a document built from already-composed layer images and register
-- every frame; `draw_frames` optionally exports review PNGs.
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
 s:saveAs(root..'/art/source/v17/'..name..'.aseprite');s:close()
end
-- A sprite from either half of the exported atlas.
function M.sprite_image_any(m,key,atlas)
 local b=assert(m.sprites[key],key..' not in manifest')
 return Image(atlas,Rectangle(b.x,b.y,b.w,b.h)),b
end
function M.count(m) local n=0;for _ in pairs(m.sprites) do n=n+1 end;return n end
function M.save_manifest(m) M.write(root..'/art/exports/game-assets-v17.json',json.encode(m)) end
-- Grounded shadow for a machine frame (the v15 method): the authored oval
-- under the footprint, a contact strip under the parts that reach the
-- ground, and a short flat cast leaning right with the key light.
local A=app.pixelColor.rgbaA
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
return M
