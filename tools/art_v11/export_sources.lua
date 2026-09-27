local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local metadata=json.decode(read(root..'/tools/art_v11/sources.json'))
assert(#metadata==56,'incomplete v11 source registry')
local m=json.decode(read(root..'/art/exports/game-assets-v11.json'))
local atlas=Image{fromFile=root..'/art/archive/art-v10/game-assets.png'}
local name,sprite;local count=0
for index=1,#metadata do
 local s=assert(metadata[index],'null source record at '..index)
 assert(s.key and s.document and s.frame,'invalid source record')
 if name~=s.document then if sprite then sprite:close()end;name=s.document;sprite=assert(app.open(root..'/art/source/v11/'..name..'.aseprite'))end
 local b=assert(m.sprites[s.key]);local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,s.frame)
 -- Copy exact RGBA, including erased pixels; transparent image compositing
 -- would leave old v10 shapes underneath the redraw.
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y))end end
 count=count+1
end
if sprite then sprite:close()end
atlas:saveAs(root..'/art/exports/game-assets-v11.png');atlas:saveAs(root..'/art/exports/game-assets.png')
local f=assert(io.open(root..'/art/exports/game-assets.json','w'));f:write(json.encode(m));f:close()
assert(count==56)
print('Exported '..count..' refined frames into their existing atlas slots.')
