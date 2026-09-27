local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local sources=json.decode(read(root..'/tools/art_v10/sources.json'))
local manifest=json.decode(read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local baseline=Image{fromFile=root..'/art/archive/art-v9/game-assets.png'}
assert(baseline:isEqual(Image(atlas,Rectangle(0,0,baseline.width,baseline.height))))
local old=json.decode(read(root..'/art/archive/art-v9/game-assets.json'))
for key,b in pairs(old.sprites)do local a=assert(manifest.sprites[key]);for k,v in pairs(b)do
 if type(v)~='table'then assert(a[k]==v,key..' old metadata changed')end
end end
local name,sprite;local count=0
for _,s in ipairs(sources)do
 if name~=s.document then if sprite then sprite:close()end;name=s.document;sprite=assert(app.open(root..'/art/source/v10/'..name..'.aseprite'));count=count+1 end
 assert(#sprite.layers==6 and sprite.width==s.w and sprite.height==s.h)
 assert(math.floor(sprite.frames[s.frame].duration*1000+.5)==s.duration_ms)
 local actual=Image(s.w,s.h,ColorMode.RGB);actual:drawSprite(sprite,s.frame)
 local b=assert(manifest.sprites[s.key]);assert(actual:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),s.key)
 assert(b.anchor_x==s.anchor_x and b.anchor_y==s.anchor_y)
end
if sprite then sprite:close()end
print('Verified '..#sources..' v10 frames in '..count..' documents, pixels/timing/anchors and prior atlas region exact.')
