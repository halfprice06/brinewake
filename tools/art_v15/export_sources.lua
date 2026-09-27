-- Export the editable v15 documents into the runtime atlas. Starts from the
-- archived v14 atlas, replaces the registered machine rectangles in place
-- (grounded shadows), grows the canvas to 2048x8192 and appends the new
-- rectangles recorded in art/exports/game-assets-v15.json. Never
-- reconstructs drawings from geometry.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local manifest=json.decode(C.read(root..'/art/exports/game-assets-v15.json'))
assert(manifest.height==C.ATLAS_H,'v15 manifest height')
local base=Image{fromFile=root..'/art/archive/art-v14/game-assets.png'}
local atlas=Image(manifest.width,manifest.height,ColorMode.RGB);atlas:clear();atlas:drawImage(base,Point(0,0))
local registries={'sources-plants','sources-submerged','sources-lane','sources-idle','sources-damage','sources-lamps','sources-coast','sources-vignette','sources-shadows'}
-- Decoded JSON values are Aseprite objects, not plain tables; copy the fields
-- so the merged registry re-encodes as JSON.
local function plain(e)
 local layers={};for i,n in ipairs(e.layers) do layers[i]=n end
 return {key=e.key,document=e.document,frame=e.frame,w=e.w,h=e.h,anchor_x=e.anchor_x,anchor_y=e.anchor_y,
  duration_ms=e.duration_ms,layers=layers,previous_source=e.previous_source}
end
local entries={}
for _,r in ipairs(registries) do
 local f=io.open(root..'/tools/art_v15/'..r..'.json')
 if f then local list=json.decode(f:read('*a'));f:close();for _,e in ipairs(list) do entries[#entries+1]=plain(e) end end
end
table.sort(entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
local name,sprite;local count=0
for _,e in ipairs(entries) do
 if name~=e.document then if sprite then sprite:close() end;name=e.document;sprite=assert(app.open(root..'/art/source/v15/'..name..'.aseprite')) end
 local b=assert(manifest.sprites[e.key],e.key..' missing from manifest');assert(sprite.width==b.w and sprite.height==b.h,e.key..' canvas size')
 local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,e.frame)
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y)) end end
 count=count+1
end
if sprite then sprite:close() end
atlas:saveAs(root..'/art/exports/game-assets-v15.png');atlas:saveAs(root..'/art/exports/game-assets.png')
C.write(root..'/art/exports/game-assets.json',json.encode(manifest))
C.write(root..'/tools/art_v15/sources.json',json.encode(entries))
print('Exported '..count..' v15 frames into the atlas.')
