local root=assert(app.params.root,'root required')
local atlas=app.open(root..'/art/source/game-assets-v6.aseprite')
local full=Image(atlas)
local names={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
local directions={'e','ne','n','nw','w','sw','s','se'}
local checked=0
for i,name in ipairs(names) do
  local sprite=app.open(root..'/art/source/v6/'..name..'.aseprite')
  assert(sprite.width==64 and sprite.height==64 and #sprite.layers==6 and #sprite.frames==40)
  assert(#sprite.tags==16 and #sprite.palettes[1]==63)
  local tags={};for _,tag in ipairs(sprite.tags) do tags[tag.name]=tag end
  local bx=((i-1)%4)*512;local by=math.floor((i-1)/4)*320
  for face=0,7 do
    local base=face*5+1
    local idle=assert(tags['idle_'..directions[face+1]])
    local walk=assert(tags['walk_'..directions[face+1]])
    assert(idle.fromFrame.frameNumber==base and idle.toFrame.frameNumber==base)
    assert(walk.fromFrame.frameNumber==base+1 and walk.toFrame.frameNumber==base+4)
    for row=0,4 do
      local composite=Image(64,64,ColorMode.RGB)
      composite:clear();composite:drawSprite(sprite,base+row)
      local expected=Image(full,Rectangle(bx+face*64,by+row*64,64,64))
      assert(composite.bytes==expected.bytes,name..' frame differs from atlas '..(base+row))
      checked=checked+1
    end
  end
end
local keys={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','salvage','well'}
for i,name in ipairs(keys) do
  local sprite=app.open(root..'/art/source/v6/'..name..'.aseprite')
  assert(sprite.width==160 and sprite.height==144 and #sprite.layers==6 and #sprite.frames==1)
  assert(Image(sprite).bytes==Image(full,Rectangle((i-1)*160,640,160,144)).bytes,name..' differs')
  checked=checked+1
end
local interface=app.open(root..'/art/source/v6/interface.aseprite')
assert(interface.width==1280 and interface.height==160 and #interface.layers==6)
assert(Image(interface).bytes==Image(full,Rectangle(0,944,1280,160)).bytes,'interface differs')
checked=checked+1
local f=assert(io.open(root..'/art/exports/game-assets-v6.json','r'))
local entries=json.decode(f:read('*a')).sprites;f:close()
local function check(source,frame,key)
  local b=assert(entries[key]);assert(source.width==b.w and source.height==b.h and #source.layers==6)
  local composite=Image(b.w,b.h,ColorMode.RGB);composite:clear();composite:drawSprite(source,frame)
  assert(composite.bytes==Image(full,Rectangle(b.x,b.y,b.w,b.h)).bytes,key..' source differs')
  checked=checked+1
end
for _,key in ipairs({'salvage_turbine','salvage_barge'}) do check(app.open(root..'/art/source/v6/'..key..'.aseprite'),1,key) end
local terrain=app.open(root..'/art/source/v6/terrain.aseprite');local frame=0
assert(#terrain.frames==20 and #terrain.tags==20)
for _,family in ipairs({'salt','silt','causeway','water','shallow'}) do
  for i=0,3 do frame=frame+1;check(terrain,frame,'terrain_'..family..'_'..i) end
end
local props=app.open(root..'/art/source/v6/coastal_props.aseprite')
assert(#props.frames==4 and #props.tags==4)
for i,key in ipairs({'salt_rock','salt_rock_1','salt_rock_2','reed_clump'}) do check(props,i,key) end
local crust=app.open(root..'/art/source/v6/salt_crust.aseprite')
assert(#crust.frames==3 and #crust.tags==3)
for i=0,2 do check(crust,i+1,'salt_crust_'..i) end
local reopened=app.open(root..'/art/source/game-assets.aseprite')
assert(Image(reopened).bytes==full.bytes,'active source differs')
Image(reopened):saveAs(root..'/output/art-v6/reopened.png')
print('Verified '..checked..' saved native frames, 128 directional tag ranges, 27 landscape tags, six layers, palette and atlas equality.')
