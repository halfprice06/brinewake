-- Root-authored v15 evening lamps: the glass of each headquarters and works
-- relit in amber for the last phase of the match day. The overlay holds only
-- the relit window pixels found on the base drawing, so it fits the base
-- frame exactly and draws nothing anywhere else.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/lamps';os.execute('mkdir -p '..out)
local A=app.pixelColor.rgbaA
local layers={'lit glass','lamp bloom'}
local union_map={[C.rgba.glass1]=C.rgba.amber2,[C.rgba.glass2]=C.rgba.amber3,[C.rgba.glass3]=C.rgba.amber4}
local assembly_map={[C.rgba.glass1]=C.rgba.amber2,[C.rgba.glass2]=C.rgba.amber3,[C.rgba.glass3]=C.rgba.amber4,[C.rgba.amber1]=C.rgba.amber2,[C.rgba.amber2]=C.rgba.amber3}
for _,b in ipairs({{'union_hq',union_map},{'union_works',union_map},{'assembly_hq',assembly_map},{'assembly_works',assembly_map}}) do
 local key,map=b[1],b[2]
 local base,bb=C.sprite_image(m,key)
 C.document(key..'_lamps',bb.w,bb.h,bb.anchor_x,bb.anchor_y,layers,{{key=key..'_lamps',duration=1,draw=function(c)
  c:layer(1)
  local lit=0
  for y=0,bb.h-1 do for x=0,bb.w-1 do local v=base:getPixel(x,y);local r=map[v]
   if r then c:raw(x-c.ox,y-c.oy,r);lit=lit+1 end
  end end
  -- A one-pixel warm bloom beside the brightest panes, on the lit side.
  c:layer(2)
  for y=1,bb.h-2 do for x=1,bb.w-2 do
   if map[base:getPixel(x,y)]==C.rgba.amber4 and A(base:getPixel(x-1,y))>0 and not map[base:getPixel(x-1,y)] and (x+y)%3==0 then c:raw(x-1-c.ox,y-c.oy,C.rgba.amber3) end
  end end
  assert(lit>0,key..' has no glass to relight')
 end}},nil,m,packer,out,entries)
end
C.write(root..'/tools/art_v15/sources-lamps.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' lamp overlays; manifest now has '..C.count(m)..' entries.')
