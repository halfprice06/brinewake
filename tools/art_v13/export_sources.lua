-- Export saved editable sources, never rerun construction over an artist edit.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local entries=json.decode(read(root..'/tools/art_v13/sources.json'))
local manifest=json.decode(read(root..'/art/archive/art-v12/game-assets.json'))
local atlas=Image{fromFile=root..'/art/archive/art-v12/game-assets.png'}
local name,sprite
assert(#entries==56)
for _,e in ipairs(entries)do
 if name~=e.document then if sprite then sprite:close()end;name=e.document;sprite=assert(app.open(root..'/art/source/v13/'..name..'.aseprite'))end
 local b=assert(manifest.sprites[e.key]);assert(sprite.width==b.w and sprite.height==b.h)
 local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,e.frame)
 -- Replace transparent pixels as well as opaque pixels, clearing old outlines.
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y))end end
end
if sprite then sprite:close()end
atlas:saveAs(root..'/art/exports/game-assets-v13.png')
atlas:saveAs(root..'/art/exports/game-assets.png')
local f=assert(io.open(root..'/art/exports/game-assets-v13.json','w'));f:write(json.encode(manifest));f:close()
print('Exported 56 fixed-structure frames to the existing atlas slots; manifest unchanged.')
