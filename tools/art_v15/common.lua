-- Shared v15 helpers: palette access, a diamond-clipped painter, exact atlas
-- copies of existing project sprites, and an append-only packer that grows
-- the atlas to 2048x8192. Root-authored scripted pixel construction; every
-- document it writes stays artist-editable under art/source/v15/.
local M={}
local root=assert(app.params.root)
M.root=root
local painter=dofile(root..'/tools/art_v5/painter.lua')
M.palette=painter.palette
-- v14 added the damp waterline role; the engine's shelf fill is used only
-- inside composed vignettes. No new opaque colour is added in v15.
M.extra={damp='736c61',shelf_far='3d6c75',shelf_near='4d7c82',sea_ink='2b515b',sea_mid='416b73',clear='263d44'}
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
M.ATLAS_H=8192
-- The archived v14 atlas is the source for exact sprite copies.
function M.atlas()
 if not M._atlas then M._atlas=Image{fromFile=root..'/art/archive/art-v14/game-assets.png'} end
 return M._atlas
end
-- Packer over the archived v14 manifest, continued from the v15 manifest
-- when an earlier v15 script already appended rectangles. Existing keys keep
-- their rectangles; new keys are appended below the last used row.
function M.manifest()
 local f=io.open(root..'/art/exports/game-assets-v15.json')
 local text;if f then text=f:read('*a');f:close() else text=M.read(root..'/art/archive/art-v14/game-assets.json') end
 local m=json.decode(text)
 m.height=M.ATLAS_H
 local maxy=0
 for _,b in pairs(m.sprites) do if b.y+b.h>maxy then maxy=b.y+b.h end end
 local packer={x=0,y=maxy,h=0}
 function packer:alloc(key,w,h,ax,ay)
  if m.sprites[key] then
   local b=m.sprites[key];assert(b.w==w and b.h==h,key..' size changed');return b
  end
  if self.x+w>m.width then self.x=0;self.y=self.y+self.h;self.h=0 end
  assert(self.y+h<=m.height,'v15 atlas overflow at '..key)
  local b={x=self.x,y=self.y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  m.sprites[key]=b;self.x=self.x+w;if h>self.h then self.h=h end
  return b
 end
 return m,packer
end
-- Exact copy of an existing sprite from the archived atlas.
function M.sprite_image(m,key)
 local b=assert(m.sprites[key],key..' not in manifest')
 return Image(M.atlas(),Rectangle(b.x,b.y,b.w,b.h)),b
end
-- A small painter in anchor-relative coordinates. `clip` optionally limits
-- painting to the 32x16 ground diamond so tiles never bleed into neighbours.
function M.canvas(ims,w,h,ax,ay,key)
 local c={ims=ims,im=ims[1],w=w,h=h,ox=ax,oy=ay,key=key,clip=nil}
 function c:layer(i) self.im=assert(self.ims[i]) end
 function c:pixel(x,y,col)
  x,y=math.floor(x+.5),math.floor(y+.5)
  if self.clip and not self.clip(x,y) then return end
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
 function c:path(p,col) for i=2,#p do self:line(p[i-1][1],p[i-1][2],p[i][1],p[i][2],col) end end
 function c:poly(p,col)
  local lo,hi=1e9,-1e9;for _,v in ipairs(p) do lo=math.min(lo,v[2]);hi=math.max(hi,v[2]) end
  for y=math.ceil(lo),math.ceil(hi)-1 do local xs={};local j=#p
   for i,a in ipairs(p) do local b=p[j];if(a[2]<=y and b[2]>y)or(b[2]<=y and a[2]>y)then xs[#xs+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2]) end;j=i end
   table.sort(xs);for i=1,#xs-1,2 do local l,r=math.ceil(xs[i]),math.ceil(xs[i+1])-1;if l<=r then self:line(l,y,r,y,col) end end
  end
 end
 -- Stamp an exact atlas sprite at an anchor-relative position.
 function c:stamp(m,key,x,y)
  local im,b=M.sprite_image(m,key)
  self.im:drawImage(im,Point(math.floor(x+.5)+self.ox-b.anchor_x,math.floor(y+.5)+self.oy-b.anchor_y))
 end
 function c:diamond(col) for y=-8,8 do self:line(-16+2*math.abs(y),y,16-2*math.abs(y),y,col) end end
 return c
end
function M.diamond_clip(x,y) return math.abs(x)+2*math.abs(y)<=16 end
function M.lane_clip(x,y) return math.abs(x)+2*math.abs(y)<16 end
-- Build one editable document: `frames` is a list of {key=,draw=,duration=}.
-- Appends the registry entries written to sources JSON.
function M.document(name,w,h,ax,ay,layers,frames,tag,m,packer,outdir,entries,extra)
 local s=Sprite(w,h,ColorMode.RGB);M.install_palette(s)
 for i,n in ipairs(layers) do local l=i==1 and s.layers[1] or s:newLayer();l.name=n end
 for f,spec in ipairs(frames) do
  if f>1 then s:newEmptyFrame(f) end;s.frames[f].duration=spec.duration or 1
  local ims={};for i=1,#layers do ims[i]=Image(w,h,ColorMode.RGB) end
  local c=M.canvas(ims,w,h,ax,ay,spec.key);if extra then extra(c) end
  spec.draw(c)
  for i=1,#layers do s:newCel(s.layers[i],f,ims[i],Point(0,0)) end
  local b=packer:alloc(spec.key,w,h,ax,ay)
  entries[#entries+1]={key=spec.key,document=name,frame=f,w=w,h=h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers}
  local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f);frame:saveAs(outdir..'/'..spec.key..'.png')
 end
 if tag then s:newTag(1,#frames).name=tag end
 s:saveAs(root..'/art/source/v15/'..name..'.aseprite');s:close()
end
-- Save a document built from already-composed layer images (used when frames
-- derive from an opened source document rather than from painter calls).
function M.document_images(name,w,h,ax,ay,layers,frames,tag,m,packer,outdir,entries,previous_source)
 local s=Sprite(w,h,ColorMode.RGB);M.install_palette(s)
 for i,n in ipairs(layers) do local l=i==1 and s.layers[1] or s:newLayer();l.name=n end
 for f,spec in ipairs(frames) do
  if f>1 then s:newEmptyFrame(f) end;s.frames[f].duration=spec.duration or 1
  for i=1,#layers do s:newCel(s.layers[i],f,spec.images[i],Point(0,0)) end
  local b=packer:alloc(spec.key,w,h,ax,ay)
  entries[#entries+1]={key=spec.key,document=name,frame=f,w=w,h=h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers,previous_source=previous_source}
  local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f);frame:saveAs(outdir..'/'..spec.key..'.png')
 end
 if tag then s:newTag(1,#frames).name=tag end
 s:saveAs(root..'/art/source/v15/'..name..'.aseprite');s:close()
end
function M.count(m) local n=0;for _ in pairs(m.sprites) do n=n+1 end;return n end
function M.save_manifest(m) M.write(root..'/art/exports/game-assets-v15.json',json.encode(m)) end
return M
