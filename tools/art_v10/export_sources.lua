-- Export current editable sources, preserving the v9 atlas byte-for-byte.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local manifest=json.decode(read(root..'/art/exports/game-assets-v10.json'))
local sources=json.decode(read(root..'/tools/art_v10/sources.json'))
local baseline=Image{fromFile=root..'/art/archive/art-v9/game-assets.png'}
local image=Image(manifest.width,manifest.height,ColorMode.RGB);image:clear();image:drawImage(baseline)
local name,sprite
for _,s in ipairs(sources)do
 if name~=s.document then if sprite then sprite:close()end;name=s.document;sprite=assert(app.open(root..'/art/source/v10/'..name..'.aseprite'))end
 local b=assert(manifest.sprites[s.key]);local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,s.frame)
 image:drawImage(frame,Point(b.x,b.y))
end
if sprite then sprite:close()end
image:saveAs(root..'/art/exports/game-assets-v10.png');image:saveAs(root..'/art/exports/game-assets.png')
local f=assert(io.open(root..'/art/exports/game-assets.json','w'));f:write(json.encode(manifest));f:close()
print('Exported '..#sources..' editable v10 frames, v9 region preserved.')
