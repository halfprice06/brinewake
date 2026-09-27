-- Export artist-editable v3 Aseprite documents into the active runtime atlas.
-- Unlike build.lua, this does not reconstruct drawings from Lua geometry.
local root=assert(app.params.root,'root required')
local template=app.open(root..'/art/source/game-assets-v4.aseprite')
local atlas=Sprite(template)
local function copy_region(source,frame,dx,dy,w,h)
  assert(source.width==w and source.height==h,'source canvas dimensions changed')
  assert(#source.layers==#atlas.layers,'source layer contract changed')
  for i,layer in ipairs(source.layers) do
    assert(layer.name==atlas.layers[i].name,'source layer names changed')
    local destination=atlas.layers[i]:cel(1).image
    for y=0,h-1 do for x=0,w-1 do destination:drawPixel(dx+x,dy+y,0) end end
    local cel=layer:cel(frame)
    if cel then
      for y=0,cel.image.height-1 do
        for x=0,cel.image.width-1 do
          local px,py=x+cel.position.x,y+cel.position.y
          assert(px>=0 and py>=0 and px<w and py<h,'source cel exceeds canvas')
          destination:drawPixel(dx+px,dy+py,cel.image:getPixel(x,y))
        end
      end
    end
  end
end
local names={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
for i,name in ipairs(names) do
  local source=app.open(root..'/art/source/v4/'..name..'.aseprite')
  assert(#source.frames==40,'unit requires eight idle and four-phase direction sets')
  local bx=((i-1)%4)*512;local by=math.floor((i-1)/4)*320
  for face=0,7 do
    for row=0,4 do copy_region(source,face*5+row+1,bx+face*64,by+row*64,64,64) end
  end
end
local structures={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','salvage','well'}
for i,name in ipairs(structures) do
  local source=app.open(root..'/art/source/v4/'..name..'.aseprite')
  copy_region(source,1,(i-1)*160,640,160,144)
end
local interface=app.open(root..'/art/source/v4/interface.aseprite')
copy_region(interface,1,0,944,1280,160)
atlas:saveAs(root..'/art/source/game-assets.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets.png')
local manifest=assert(io.open(root..'/art/exports/game-assets-v4.json','rb'))
local bytes=manifest:read('*a');manifest:close()
local target=assert(io.open(root..'/art/exports/game-assets.json','wb'))
target:write(bytes);target:close()
print('Exported active runtime atlas from editable native unit and structure sources.')
