-- Export the editable v9 documents without reconstructing their drawings.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s)local f=assert(io.open(p,'w'));f:write(s);f:close()end
local manifest=json.decode(read(root..'/art/exports/game-assets-v9.json'))
local sources=json.decode(read(root..'/tools/art_v9/sources.json'))
local baseline=Image{fromFile=root..'/art/archive/art-v8/game-assets.png'}
local im=Image(manifest.width,manifest.height,ColorMode.RGB);im:clear();im:drawImage(baseline,Point(0,0))
local name,sprite=nil,nil
for _,entry in ipairs(sources) do
 if entry.document~=name then
  if sprite then sprite:close()end;name=entry.document;sprite=app.open(root..'/art/source/v9/'..name..'.aseprite')
 end
 local b=assert(manifest.sprites[entry.key]);assert(sprite.width==b.w and sprite.height==b.h)
 local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,entry.frame)
 im:drawImage(frame,Point(b.x,b.y))
end
if sprite then sprite:close()end
im:saveAs(root..'/art/exports/game-assets-v9.png');im:saveAs(root..'/art/exports/game-assets.png')
write(root..'/art/exports/game-assets.json',json.encode(manifest))
print('Exported '..#sources..' editable frames; all original v8 pixels/rectangles preserved.')
