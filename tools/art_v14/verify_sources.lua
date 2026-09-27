-- Reopen every saved v14 document and compare it with the active atlas:
-- exact pixels, canvas size, layer names, anchors, durations and tags. Machine
-- documents must keep their structural layers identical to the previous
-- sources and their shadow layer at or below the anchor line. Buildings are
-- outside this pass and keep the no-shadow rule from v13.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
local entries=json.decode(C.read(root..'/tools/art_v14/sources.json'))
local m=json.decode(C.read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local function full(l,f,w,h) local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position) end;return im end
local A=app.pixelColor.rgbaA
local name,s,previous;local docs,count,machine_frames=0,0,0
for _,e in ipairs(entries) do
 if name~=e.document then
  if s then s:close() end;if previous then previous:close();previous=nil end
  name=e.document;s=assert(app.open(root..'/art/source/v14/'..name..'.aseprite'));docs=docs+1
  if e.previous_source then
   previous=assert(app.open(root..'/'..e.previous_source))
   assert(#s.frames==#previous.frames and #s.tags==#previous.tags,name..' timeline changed')
   for i,t in ipairs(previous.tags) do assert(s.tags[i].name==t.name and s.tags[i].fromFrame.frameNumber==t.fromFrame.frameNumber and s.tags[i].toFrame.frameNumber==t.toFrame.frameNumber,name..' tag '..t.name) end
  end
 end
 assert(s.width==e.w and s.height==e.h and #s.layers==#e.layers,e.key..' canvas or layer count')
 for i,n in ipairs(e.layers) do assert(s.layers[i].name==n,e.key..' layer '..i) end
 assert(math.floor(s.frames[e.frame].duration*1000+.5)==e.duration_ms,e.key..' duration')
 local b=assert(m.sprites[e.key],e.key..' missing');assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y,e.key..' anchor')
 local im=Image(e.w,e.h,ColorMode.RGB);im:drawSprite(s,e.frame)
 assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from atlas')
 if previous then
  assert(s.frames[e.frame].duration==previous.frames[e.frame].duration,e.key..' duration changed')
  for i=2,#s.layers do assert(full(s.layers[i],e.frame,e.w,e.h):isEqual(full(previous.layers[i],e.frame,e.w,e.h)),e.key..' changed structural layer '..i) end
  local shadow=full(s.layers[1],e.frame,e.w,e.h);local body=Image(e.w,e.h,ColorMode.RGB)
  for i=2,#s.layers do local c=s.layers[i]:cel(e.frame);if c then body:drawImage(c.image,c.position) end end
  local lowest=-1
  for y=0,e.h-1 do for x=0,e.w-1 do if A(body:getPixel(x,y))>0 then lowest=y end end end
  local n=0
  for y=0,e.h-1 do for x=0,e.w-1 do
   local p=shadow:getPixel(x,y)
   if A(p)>0 then
    n=n+1
    assert(p==C.rgba.cast,e.key..' shadow uses a foreign colour')
    assert(A(body:getPixel(x,y))==0,e.key..' shadow overlaps the body')
    assert(y>=lowest-14,e.key..' shadow climbs above the machine base')
   end
  end end
  assert(n>0,e.key..' has no shadow')
  machine_frames=machine_frames+1
 end
 count=count+1
end
if s then s:close() end;if previous then previous:close() end
print('Verified '..count..' frames in '..docs..' v14 documents against the atlas; '..machine_frames..' machine frames keep structural layers exact with grounded shadows.')
