local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local entries=json.decode(read(root..'/tools/art_v13/sources.json'))
local m=json.decode(read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local function full(l,f,w,h)local im=Image(w,h,ColorMode.RGB);local c=l:cel(f);if c then im:drawImage(c.image,c.position)end;return im end
local name,s,previous;local docs=0;local count=0
local cast=app.pixelColor.rgba(101,94,80,255)
for _,e in ipairs(entries)do
 if name~=e.document then
  if s then s:close();previous:close()end
  name=e.document;s=assert(app.open(root..'/art/source/v13/'..name..'.aseprite'));previous=assert(app.open(root..'/'..e.previous_source));docs=docs+1
  assert(#s.frames==#previous.frames and #s.tags==#previous.tags)
  for i,t in ipairs(previous.tags)do assert(s.tags[i].name==t.name and s.tags[i].fromFrame.frameNumber==t.fromFrame.frameNumber and s.tags[i].toFrame.frameNumber==t.toFrame.frameNumber)end
 end
 assert(s.width==e.w and s.height==e.h and #s.layers==#e.layers)
 for i,n in ipairs(e.layers)do assert(s.layers[i].name==n)end
 assert(math.floor(s.frames[e.frame].duration*1000+.5)==e.duration_ms)
 assert(s.frames[e.frame].duration==previous.frames[e.frame].duration)
 local im=Image(e.w,e.h,ColorMode.RGB);im:drawSprite(s,e.frame)
 local b=assert(m.sprites[e.key]);assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y)
 assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from atlas')
 for pixel in im:pixels()do assert(pixel()~=cast,e.key..' contains cast-shadow paint')end
 for i=2,5 do assert(full(s.layers[i],e.frame,e.w,e.h):isEqual(full(previous.layers[i],e.frame,e.w,e.h)),e.key..' changed structural layer '..i)end
 if name:sub(1,5)=='gate_'or name:sub(1,9)=='landmark_'then
  local layer=full(s.layers[1],e.frame,e.w,e.h)
  for pixel in layer:pixels()do assert(app.pixelColor.rgbaA(pixel())==0,e.key..' shadow layer is not empty')end
 end
 count=count+1
end
if s then s:close();previous:close()end
assert(count==56 and docs==23)
print('Verified 56 frames in 23 saved sources: exact atlas pixels, layers, dimensions, anchors, timings and tags; structural layers 2-5 unchanged; cast paint absent.')
