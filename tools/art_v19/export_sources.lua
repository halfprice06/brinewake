-- Export the editable v19 documents into the runtime atlas. Starts from the
-- archived v18 atlas, then paints every registered v19 frame into the
-- rectangle it already owns in art/exports/game-assets-v19.json. Never
-- reconstructs drawings from geometry.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v19/common.lua')
local manifest=json.decode(C.read(root..'/art/exports/game-assets-v19.json'))
assert(manifest.width==C.ATLAS_W and manifest.height==C.ATLAS_H,'v19 manifest size')
local base=Image{fromFile=C.ARCHIVE..'game-assets.png'}
local atlas=Image(C.ATLAS_W,C.ATLAS_H,ColorMode.RGB);atlas:clear();atlas:drawImage(base,Point(0,0))
local registries={'sources-machines'}
local function plain(e)
 local layers={};for i,n in ipairs(e.layers) do layers[i]=n end
 local o={key=e.key,document=e.document,frame=e.frame,w=e.w,h=e.h,anchor_x=e.anchor_x,anchor_y=e.anchor_y,duration_ms=e.duration_ms,layers=layers}
 if e.muzzle then o.muzzle={e.muzzle[1],e.muzzle[2]} end
 return o
end
local entries={}
for _,r in ipairs(registries) do
 local list=json.decode(C.read(root..'/tools/art_v19/'..r..'.json'))
 for _,e in ipairs(list) do entries[#entries+1]=plain(e) end
end
table.sort(entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
local name,sprite;local count=0
for _,e in ipairs(entries) do
 if name~=e.document then if sprite then sprite:close() end;name=e.document;sprite=assert(app.open(C.SOURCE..name..'.aseprite')) end
 local b=assert(manifest.sprites[e.key],e.key..' missing from manifest');assert(sprite.width==b.w and sprite.height==b.h,e.key..' canvas size')
 local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(sprite,e.frame)
 -- Clear the rectangle first: a re-authored frame may cover fewer pixels.
 for y=0,b.h-1 do for x=0,b.w-1 do atlas:drawPixel(b.x+x,b.y+y,frame:getPixel(x,y)) end end
 count=count+1
end
if sprite then sprite:close() end
local plain_manifest={width=manifest.width,height=manifest.height,sprites={}}
for k,v in pairs(manifest.sprites) do
 local b={x=v.x,y=v.y,w=v.w,h=v.h,anchor_x=v.anchor_x,anchor_y=v.anchor_y}
 if v.muzzle then b.muzzle={v.muzzle[1],v.muzzle[2]} end
 plain_manifest.sprites[k]=b
end
atlas:saveAs(root..'/art/exports/game-assets-v19.png');atlas:saveAs(root..'/art/exports/game-assets.png')
C.write(root..'/art/exports/game-assets.json',json.encode(plain_manifest))
C.write(root..'/tools/art_v19/sources.json',json.encode(entries))
print('Exported '..count..' v19 frames into the atlas.')
