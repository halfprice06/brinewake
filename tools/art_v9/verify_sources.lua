-- Reopen every editable source and verify the exact exported pixels/timing.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local sources=json.decode(read(root..'/tools/art_v9/sources.json'))
local manifest=json.decode(read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local name,sprite=nil,nil;local documents=0
for _,s in ipairs(sources)do
 if name~=s.document then
  if sprite then sprite:close()end;name=s.document;sprite=assert(app.open(root..'/art/source/v9/'..name..'.aseprite'));documents=documents+1
 end
 assert(#sprite.layers==6,s.key..' semantic layers lost')
 assert(sprite.width==s.w and sprite.height==s.h,s.key..' canvas changed')
 assert(math.floor(sprite.frames[s.frame].duration*1000+.5)==s.duration_ms,s.key..' timing mismatch')
 local actual=Image(s.w,s.h,ColorMode.RGB);actual:drawSprite(sprite,s.frame)
 local b=assert(manifest.sprites[s.key]);local expected=Image(atlas,Rectangle(b.x,b.y,b.w,b.h))
 assert(actual:isEqual(expected),s.key..' saved source/export mismatch')
 assert(b.anchor_x==s.anchor_x and b.anchor_y==s.anchor_y,s.key..' anchor mismatch')
end
if sprite then sprite:close()end
print('Verified '..#sources..' frames in '..documents..' editable sources: exact pixels, timing, anchors, six semantic layers.')
