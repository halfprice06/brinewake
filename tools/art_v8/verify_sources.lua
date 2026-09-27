-- Verify saved native sources against the delivered atlas, including timing,
-- registration, semantic layers, and preservation of every prior atlas entry.
local root=assert(app.params.root,'root required')
local function read(path)local f=assert(io.open(path));local s=f:read('*a');f:close();return s end
local function write(path,s)local f=assert(io.open(path,'w'));f:write(s);f:close()end
local metadata=json.decode(read(root..'/tools/art_v8/sources.json'))
local manifest=json.decode(read(root..'/art/exports/game-assets-v8.json'))
local previous=json.decode(read(root..'/art/archive/art-v7/game-assets.json'))
local image=Image{fromFile=root..'/art/exports/game-assets-v8.png'}
local old=Image{fromFile=root..'/art/archive/art-v7/game-assets.png'}
local painter=dofile(root..'/tools/art_v5/painter.lua')
local count,prior_count,documents=0,0,{}
for key,b in pairs(previous.sprites) do
  local now=assert(manifest.sprites[key],key)
  for _,field in ipairs({'x','y','w','h','anchor_x','anchor_y'}) do assert(now[field]==b[field],key..' moved '..field) end
  local a=Image(image,Rectangle(b.x,b.y,b.w,b.h));local p=Image(old,Rectangle(b.x,b.y,b.w,b.h))
  assert(a:isEqual(p),key..' changed old pixels');prior_count=prior_count+1
end
local name,source
for _,entry in ipairs(metadata) do
  if entry.document~=name then
    if source then source:close() end
    name=entry.document;source=app.open(root..'/art/source/v8/'..name..'.aseprite')
    documents[name]=#source.frames
    assert(source.width==entry.w and source.height==entry.h,name..' native dimensions')
    assert(#source.layers==6,name..' semantic layers')
    for i,l in ipairs(source.layers) do assert(l.name==painter.layers[i],name..' layer order') end
    assert(#source.tags>0,name..' no animation/state tags')
  end
  local b=assert(manifest.sprites[entry.key]);local expected=Image(image,Rectangle(b.x,b.y,b.w,b.h))
  local actual=Image(b.w,b.h,ColorMode.RGB);actual:clear();actual:drawSprite(source,entry.frame)
  assert(actual:isEqual(expected),entry.key..' saved source differs from PNG')
  local ms=math.floor(source.frames[entry.frame].duration*1000+.5)
  assert(ms==entry.duration_ms,entry.key..' timing mismatch')
  assert(b.anchor_x==entry.anchor_x and b.anchor_y==entry.anchor_y,entry.key..' anchor')
  if entry.muzzle then
    assert(b.muzzle and b.muzzle[1]==entry.muzzle[1] and b.muzzle[2]==entry.muzzle[2],entry.key..' muzzle')
    assert(b.muzzle[1]+b.anchor_x>=0 and b.muzzle[1]+b.anchor_x<b.w,entry.key..' muzzle x outside')
    assert(b.muzzle[2]+b.anchor_y>=0 and b.muzzle[2]+b.anchor_y<b.h,entry.key..' muzzle y outside')
  end
  count=count+1
end
if source then source:close() end
local reopened=app.open(root..'/art/source/game-assets-v8.aseprite')
assert(Image(reopened):isEqual(image),'reopened atlas differs from PNG')
Image(reopened):saveAs(root..'/output/art-v8/reopened.png')
local active=Image{fromFile=root..'/art/exports/game-assets.png'}
assert(active:isEqual(image),'active PNG is not v8')
assert(read(root..'/art/exports/game-assets.json')==read(root..'/art/exports/game-assets-v8.json'),'active manifest is not v8')
local report={native_frames_compared=count,old_entries_preserved=prior_count,documents=documents,
  atlas_width=2048,atlas_height=4096,reopened_source_matches_png=true,active_assets_match=true,
  source_timing_and_anchors_match=true,muzzle_metadata_checked=true}
write(root..'/output/art-v8/validation-sources.json',json.encode(report))
print('Verified '..count..' saved native frames and '..prior_count..' unchanged old entries; reopened atlas and active files match.')
