-- Trial 8 (ranked item 22): both seats read the Guide's U as V at half
-- scale.  The U gets a flat, full-width floor and stems that run a row
-- lower, so it stays a box at any reduction; the V tapers from the fourth
-- row in runs of 3, 3, 2, 1, so it stays a point.  Only the U and V cells of
-- the glyph layer change.  Run with:
--   aseprite -b --script-param root=$PWD --script tools/art_v8/fix_u_v.lua
local root=app.params.root
local path=root..'/art/source/v8/console_type.aseprite'
local sprite=app.open(path)
local layer
for _,l in ipairs(sprite.layers) do if l.name=='working parts' then layer=l end end
assert(layer,'glyph layer')
local cel=assert(layer:cel(1),'glyph cel')
assert(cel.position.x==0 and cel.position.y==0,'glyph cel at the origin')
local ink=app.pixelColor.rgba(231,217,181,255)
local clear=app.pixelColor.rgba(0,0,0,0)
local glyphs={
  -- Cell origins follow tools/art_v8/font-map.json: ten 8x10 cells a row.
  {x=32,y=40,rows={'1100011','1100011','1100011','1100011','1100011','1100011','1100011','1100011','0111110'}}, -- U
  {x=40,y=40,rows={'1100011','1100011','1100011','0110110','0110110','0110110','0011100','0011100','0001000'}}, -- V
}
local image=cel.image:clone()
for _,g in ipairs(glyphs) do
  for yy=1,9 do for xx=1,7 do
    local on=g.rows[yy]:sub(xx,xx)=='1'
    image:drawPixel(g.x+xx-1,g.y+yy-1,on and ink or clear)
  end end
end
cel.image=image
sprite:saveAs(path)
print('Redrew U and V in '..path)
