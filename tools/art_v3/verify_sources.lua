local root=assert(app.params.root,'root required')
local atlas=app.open(root..'/art/source/game-assets-v3.aseprite')
local full=Image(atlas)
local names={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
local directions={'e','ne','n','nw','w','sw','s','se'}
local checked=0
for i,name in ipairs(names) do
  local sprite=app.open(root..'/art/source/v3/'..name..'.aseprite')
  assert(sprite.width==64 and sprite.height==64 and #sprite.layers==6 and #sprite.frames==40)
  assert(#sprite.tags==16 and #sprite.palettes[1]==53)
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
  local sprite=app.open(root..'/art/source/v3/'..name..'.aseprite')
  assert(sprite.width==160 and sprite.height==144 and #sprite.layers==6 and #sprite.frames==1)
  assert(Image(sprite).bytes==Image(full,Rectangle((i-1)*160,640,160,144)).bytes,name..' differs')
  checked=checked+1
end
print('Verified '..checked..' saved native frames, 128 tag ranges, six layers, palette and atlas equality.')
