-- Export saved editable sources, never rerun construction over an artist edit.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local entries=json.decode(read(root..'/tools/art_v12/sources.json'))
local manifest=json.decode(read(root..'/art/archive/art-v11/game-assets.json'))
local atlas=Image{fromFile=root..'/art/archive/art-v11/game-assets.png'}
local name,sprite
assert(#entries==33)
for _,e in ipairs(entries)do
 if name~=e.document then if sprite then sprite:close()end;name=e.document;sprite=assert(app.open(root..'/art/source/v12/'..name..'.aseprite'))end
 local b=assert(manifest.sprites[e.key]);assert(sprite.width==b.w and sprite.height==b.h)
 local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,e.frame)
 -- Replace transparent pixels as well as opaque pixels, clearing old outlines.
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y))end end
end
if sprite then sprite:close()end
atlas:saveAs(root..'/art/exports/game-assets-v12.png')
atlas:saveAs(root..'/art/exports/game-assets.png')
local f=assert(io.open(root..'/art/exports/game-assets-v12.json','w'));f:write(json.encode(manifest));f:close()
print('Exported 33 coastal frames to the existing atlas slots; manifest unchanged.')
