-- Regenerate the new v8 atlas regions from editable native documents. Existing
-- v7 entries are preserved. Does not reconstruct any artwork from Lua geometry.
local root=assert(app.params.root,'root required')
local function read(path)local f=assert(io.open(path));local s=f:read('*a');f:close();return s end
local function write(path,s)local f=assert(io.open(path,'w'));f:write(s);f:close()end
local painter=dofile(root..'/tools/art_v5/painter.lua')
local metadata=json.decode(read(root..'/tools/art_v8/sources.json'))
local manifest=json.decode(read(root..'/art/exports/game-assets-v8.json'))
local template=app.open(root..'/art/source/game-assets-v8.aseprite')
local atlas_images={}
for i,layer in ipairs(painter.layers) do
  local im=Image(2048,4096,ColorMode.RGB);im:clear()
  local cel=template.layers[i]:cel(1);if cel then im:drawImage(cel.image,cel.position) end
  atlas_images[layer]=im
end
template:close()
local current_name,current
local frames_compared=0
for _,entry in ipairs(metadata) do
  if current_name~=entry.document then
    if current then current:close() end
    current_name=entry.document
    current=assert(app.open(root..'/art/source/v8/'..entry.document..'.aseprite'))
  end
  local b=assert(manifest.sprites[entry.key])
  assert(current.width==b.w and current.height==b.h,entry.key..' changed canvas')
  assert(#current.layers==6,entry.key..' requires six semantic layers')
  for i,name in ipairs(painter.layers) do
    assert(current.layers[i].name==name,entry.key..' changed layer contract')
    local destination=atlas_images[name]
    destination:clear(Rectangle(b.x,b.y,b.w,b.h),0)
    local cel=current.layers[i]:cel(entry.frame)
    if cel then
      assert(cel.position.x>=0 and cel.position.y>=0 and cel.position.x+cel.image.width<=b.w and cel.position.y+cel.image.height<=b.h,'cel outside native canvas')
      destination:drawImage(cel.image,Point(b.x+cel.position.x,b.y+cel.position.y))
    end
  end
  frames_compared=frames_compared+1
end
if current then current:close() end
local atlas=Sprite(2048,4096,ColorMode.RGB);painter.install_palette(atlas)
for i,name in ipairs(painter.layers) do
  local layer=i==1 and atlas.layers[1] or atlas:newLayer();layer.name=name
  atlas:newCel(layer,1,atlas_images[name],Point(0,0))
end
atlas:saveAs(root..'/art/source/game-assets-v8.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets-v8.png')
atlas:saveAs(root..'/art/source/game-assets.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets.png')
write(root..'/art/exports/game-assets.json',json.encode(manifest))

-- The artist-editable font proof and the runtime bitmap share exactly pixels.
local font=app.open(root..'/art/source/v8/console_type.aseprite')
local im=Image(font);im:saveAs(root..'/output/art-v8/console-font.png')
local chars=json.decode(read(root..'/tools/art_v8/font-map.json'))
local rows={};local rust={
  '//! Root-authored 7 by 9 dock-console bitmap face. Editable source: art/source/v8/console_type.aseprite.',
  '// Exported by tools/art_v8/export_sources.lua from the native alpha mask.',
  'pub fn glyph(ch: char) -> [u8; 9] {',
  '    match ch.to_ascii_uppercase() {'}
for i,ch in ipairs(chars) do
  local x=((i-1)%10)*8;local y=math.floor((i-1)/10)*10;local bits={};rows[ch]={}
  for yy=0,8 do
    local row='';local value=0
    for xx=0,6 do
      local set=app.pixelColor.rgbaA(im:getPixel(x+xx,y+yy))>0
      row=row..(set and '1' or '0');value=value*2+(set and 1 or 0)
    end
    bits[#bits+1]=tostring(value);rows[ch][#rows[ch]+1]=row
  end
  rust[#rust+1]="        '"..ch.."' => ["..table.concat(bits,', ')..'], '
end
rust[#rust+1]='        _ => [62, 99, 3, 6, 12, 12, 0, 12, 12],'
rust[#rust+1]='    }';rust[#rust+1]='}'
write(root..'/crates/bw_desktop/src/font7.rs',table.concat(rust,'\n')..'\n')
write(root..'/tools/art_v8/font7.json',json.encode(rows))
print('Exported '..frames_compared..' v8 native source frames and '..#chars..' font glyphs into runtime assets.')
