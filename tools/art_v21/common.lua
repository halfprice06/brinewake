-- Shared v21 helpers: the Saltglass Compact pass. v21 adds new keys to the
-- runtime atlas; it never moves or repaints a rectangle an earlier pass
-- owns. A key v21 already placed keeps its rectangle on a rebuild, so the
-- pass can be rerun after a drawing change. Root-authored scripted pixel
-- construction: every document it writes stays editable under
-- art/source/v21/.
local M={}
local root=assert(app.params.root)
M.root=root
local painter=dofile(root..'/tools/art_v21/painter.lua')
M.painter=painter
M.palette=painter.palette
M.rgba=painter.pixels
M.role_of={}
for k,v in pairs(M.rgba) do M.role_of[v]=k end
M.install_palette=painter.install_palette
M.SOURCE=root..'/art/source/v21/'
M.OUT=root..'/output/art-v21/'
local A=app.pixelColor.rgbaA
function M.read(p) local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
function M.write(p,s) local f=assert(io.open(p,'w'));f:write(s);f:close() end

-- The engine's walk (tools/art_v19/common.lua, unchanged): a walk phase is
-- a quarter cell of Manhattan travel, 4 painter units on a screen-axis
-- facing and 4*sqrt(2) on a diagonal.
function M.travel(face) return face%2==0 and 4 or 4*math.sqrt(2) end
-- Seed speeds from docs/THIRD-FACTION-AND-CONFLUENCE.md.
M.speed={raker=3,brander=4,heliostat=2,glinter=5,stilt=6,glazier=3,salter=2,pan=2}
function M.phase_ms(name,face)
 local per_second=assert(M.speed[name],name)*4*(face%2==0 and math.sqrt(2) or 1)
 return math.floor(1000/per_second+.5)
end

-- Grounded shadow for a machine frame (the v15 method, as v19 uses it): an
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
function M.pixmap(t) local o={};for a,b in pairs(t) do o[assert(M.rgba[a],a)]=assert(M.rgba[b],b) end;return o end
function M.composite(list,from)
 local im=Image(list[1].width,list[1].height,ColorMode.RGB)
 for i=from or 1,#list do im:drawImage(list[i],Point(0,0)) end
 return im
end
-- Count opaque colours in a list of layer images, skipping the shadow roles.
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

-- The atlas. New keys go into free space: a 16-pixel occupancy grid built
-- from every rectangle in the manifest, first fit, scanning rows from the
-- top of the right half, then the left. A key v21 placed before keeps its
-- rectangle; its size and anchor may not change.
M.ATLAS=root..'/art/exports/game-assets.png'
M.MANIFEST=root..'/art/exports/game-assets.json'
function M.open_atlas()
 local atlas=Image{fromFile=M.ATLAS}
 local decoded=json.decode(M.read(M.MANIFEST))
 local m={width=decoded.width,height=decoded.height,sprites={}}
 for k,v in pairs(decoded.sprites) do
  local b={x=v.x,y=v.y,w=v.w,h=v.h,anchor_x=v.anchor_x,anchor_y=v.anchor_y}
  if v.muzzle then b.muzzle={v.muzzle[1],v.muzzle[2]} end
  m.sprites[k]=b
 end
 local G=16
 local gw,gh=math.floor(m.width/G),math.floor(m.height/G)
 local used={}
 local function mark(b)
  for gy=math.floor(b.y/G),math.floor((b.y+b.h-1)/G) do
   for gx=math.floor(b.x/G),math.floor((b.x+b.w-1)/G) do used[gy*gw+gx]=true end
  end
 end
 for _,b in pairs(m.sprites) do mark(b) end
 local packer={placed={},cursor={}}
 local function fits(gx,gy,cw,ch)
  if gx+cw>gw or gy+ch>gh then return false end
  for y=gy,gy+ch-1 do for x=gx,gx+cw-1 do if used[y*gw+x] then return false end end end
  return true
 end
 function packer:alloc(key,w,h,ax,ay)
  local b=m.sprites[key]
  if b then
   assert(b.w==w and b.h==h,key..' size changed')
   assert(b.anchor_x==ax and b.anchor_y==ay,key..' anchor changed')
   self.placed[key]=true
   return b
  end
  local cw,ch=math.ceil(w/G),math.ceil(h/G)
  -- Allocation is sequential, so a size resumes its scan where it last fit.
  local size=cw..'x'..ch
  local start=self.cursor[size] or {1,0}
  local halves={{math.floor(gw/2),gw-1},{0,math.floor(gw/2)-1}}
  for hi=start[1],2 do local half=halves[hi]
   for gy=(hi==start[1] and start[2] or 0),gh-ch do for gx=half[1],half[2]-cw+1 do
    if fits(gx,gy,cw,ch) then
     b={x=gx*G,y=gy*G,w=w,h=h,anchor_x=ax,anchor_y=ay}
     m.sprites[key]=b;mark(b);self.placed[key]=true
     self.cursor[size]={hi,gy}
     return b
    end
   end end
  end
  error('atlas full for '..key)
 end
 return atlas,m,packer
end
function M.blit(atlas,b,frame)
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y)) end end
end
function M.save_atlas(atlas,m)
 atlas:saveAs(M.ATLAS)
 M.write(M.MANIFEST,json.encode(m))
end

-- Save a document built from composed layer images, register every frame
-- and paint it into the atlas. `frames` are {key=,duration=,images={..}};
-- `tags` are {name,first,last} or one name for the whole document.
function M.document(ctx,name,w,h,ax,ay,layers,frames,tags)
 local s=Sprite(w,h,ColorMode.RGB);M.install_palette(s)
 for i,n in ipairs(layers) do local l=i==1 and s.layers[1] or s:newLayer();l.name=n end
 for f,spec in ipairs(frames) do
  if f>1 then s:newEmptyFrame(f) end;s.frames[f].duration=spec.duration or 1
  for i=1,#layers do s:newCel(s.layers[i],f,assert(spec.images[i],name..' layer '..i),Point(0,0)) end
  local frame=Image(w,h,ColorMode.RGB);frame:drawSprite(s,f)
  if spec.key then
   local b=ctx.packer:alloc(spec.key,w,h,ax,ay)
   if spec.muzzle then b.muzzle=spec.muzzle end
   M.blit(ctx.atlas,b,frame)
   local e={key=spec.key,document=name,frame=f,w=w,h=h,anchor_x=ax,anchor_y=ay,
    duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=layers}
   if spec.muzzle then e.muzzle=spec.muzzle end
   ctx.entries[#ctx.entries+1]=e
   if ctx.out then frame:saveAs(ctx.out..'/'..spec.key..'.png') end
  end
 end
 if tags then
  if type(tags)=='table' then for _,t in ipairs(tags) do s:newTag(t[2],t[3]).name=t[1] end
  else s:newTag(1,#frames).name=tags end
 end
 s:saveAs(M.SOURCE..name..'.aseprite');s:close()
end
return M
