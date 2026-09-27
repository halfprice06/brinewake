local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local entries=json.decode(read(root..'/tools/art_v11/sources.json'))
local manifest=json.decode(read(root..'/art/exports/game-assets.json'))
local atlas=Image{fromFile=root..'/art/exports/game-assets.png'}
assert(#entries==56,'invalid source count')
local name,sprite;local checked,docs=0,0
for i=1,#entries do
 local e=assert(entries[i],'null record '..i);assert(e.key and e.document)
 if name~=e.document then if sprite then sprite:close()end;name=e.document;sprite=assert(app.open(root..'/art/source/v11/'..name..'.aseprite'));docs=docs+1 end
 assert(sprite.width==e.w and sprite.height==e.h)
 assert(#sprite.layers==#e.layers)
 for j=1,#e.layers do assert(sprite.layers[j].name==e.layers[j])end
 assert(math.floor(sprite.frames[e.frame].duration*1000+.5)==e.duration_ms)
 local im=Image(e.w,e.h,ColorMode.RGB);im:drawSprite(sprite,e.frame)
 local b=assert(manifest.sprites[e.key]);assert(b.anchor_x==e.anchor_x and b.anchor_y==e.anchor_y)
 assert(im:isEqual(Image(atlas,Rectangle(b.x,b.y,b.w,b.h))),e.key..' differs from active atlas')
 checked=checked+1
end
if sprite then sprite:close()end
assert(checked==56 and docs==16)
print('Verified '..checked..' refined frames in '..docs..' saved sources against active atlas: layers, pixels, timing, anchors.')
