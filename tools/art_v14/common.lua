-- Shared v14 helpers: palette access, a diamond-clipped tile painter, and an
-- append-only shelf packer for new atlas rectangles. Root-authored scripted
-- pixel construction; every document it writes stays artist-editable.
local M={}
local root=assert(app.params.root)
M.root=root
local painter=dofile(root..'/tools/art_v5/painter.lua')
M.palette=painter.palette
-- One new palette role for this pass: cool damp sand at the waterline, sitting
-- between earth1 and earth0 in value and slightly cooler than both.
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
-- Packer over the archived v13 manifest. New keys are appended below the last
-- used row; existing keys keep their rectangles.
function M.manifest()
 -- Continue from the v14 manifest when an earlier v14 script already
 -- appended rectangles; otherwise start from the archived v13 manifest.
 local f=io.open(root..'/art/exports/game-assets-v14.json')
 local text;if f then text=f:read('*a');f:close() else text=M.read(root..'/art/archive/art-v13/game-assets.json') end
 local m=json.decode(text)
 local maxy=0
 for _,b in pairs(m.sprites) do if b.y+b.h>maxy then maxy=b.y+b.h end end
 local packer={x=0,y=maxy,h=0}
 function packer:alloc(key,w,h,ax,ay)
  if m.sprites[key] then
   local b=m.sprites[key];assert(b.w==w and b.h==h,key..' size changed');return b
  end
  if self.x+w>m.width then self.x=0;self.y=self.y+self.h;self.h=0 end
  assert(self.y+h<=m.height,'v14 atlas overflow at '..key)
  local b={x=self.x,y=self.y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  m.sprites[key]=b;self.x=self.x+w;if h>self.h then self.h=h end
  return b
 end
 return m,packer
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
 function c:diamond(col) for y=-8,8 do self:line(-16+2*math.abs(y),y,16-2*math.abs(y),y,col) end end
 return c
end
function M.diamond_clip(x,y) return math.abs(x)+2*math.abs(y)<=16 end
-- Build one editable document: `frames` is a list of {key=,draw=,duration=}.
-- Returns the registry entries written to sources JSON.
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
 s:saveAs(root..'/art/source/v14/'..name..'.aseprite');s:close()
end
return M
