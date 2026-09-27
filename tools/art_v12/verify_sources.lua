local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local entries=json.decode(read(root..'/tools/art_v12/sources.json'))
local m=json.decode(read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
local name,sprite;local docs=0;local checked=0
assert(#entries==33)
for _,e in ipairs(entries)do
 if name~=e.document then if sprite then sprite:close()end;name=e.document;sprite=assert(app.open(root..'/art/source/v12/'..name..'.aseprite'));docs=docs+1 end
 assert(sprite.width==e.w and sprite.height==e.h)
 assert(#sprite.layers==#e.layers)
 for j,n in ipairs(e.layers)do assert(sprite.layers[j].name==n)end
 assert(math.floor(sprite.frames[e.frame].duration*1000+.5)==e.duration_ms)
 local im=Image(e.w,e.h,ColorMode.RGB);im:drawSprite(sprite,e.frame)
 local b=assert(m.sprites[e.key]);assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y)
 assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' mismatched export')
 checked=checked+1
end
if sprite then sprite:close()end
assert(checked==33 and docs==10)
print('Verified 33 frames in 10 saved sources: exact atlas RGBA, layers, anchors, native dimensions, durations.')
