-- Root authoring entry point for the v18 damage overlays of the earlier
-- families: the seven v4-v14 buildings and the eight original machines. The
-- base drawings are exact copies from the archived v17 atlas; only the
-- overlay documents are re-authored, with the plane-aware v18 marks.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v18/common.lua')
local D=dofile(root..'/tools/art_v18/damage.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v18/damage';os.execute('mkdir -p '..out)
local layers={'dents and tears','streaks and strands','soot'}
local buildings={
 {'union_hq','steel'},{'union_works','steel'},{'assembly_hq','weave'},{'assembly_works','weave'},
 {'condenser','steel'},{'dropoff','steel'},{'tower','steel'},
}
for _,b in ipairs(buildings) do
 local key,material=b[1],b[2]
 local base,bb=C.sprite_image(m,key)
 local frames={}
 for tier=1,2 do
  frames[#frames+1]={key=key..'_damage_'..tier,duration=1,images=D.marks(C,base,material,tier,#key,{count=(tier==1) and 4 or 6,spacing=11})}
 end
 C.document_images(key..'_damage',bb.w,bb.h,bb.anchor_x,bb.anchor_y,layers,frames,'damage_tiers',m,packer,out,entries)
end
local machines={
 {'hook','steel'},{'riveter','steel'},{'bulwark','steel'},{'sounder','steel'},
 {'wick','weave'},{'skipper','weave'},{'reedguard','weave'},{'loom','weave'},
}
for _,u in ipairs(machines) do
 local role,material=u[1],u[2]
 local frames={}
 for face=0,7 do
  local base=C.sprite_image(m,role..'_'..face)
  -- Strip the shadow roles so marks never sit on the ground shadow.
  local body=Image(base.width,base.height,ColorMode.RGB)
  for y=0,base.height-1 do for x=0,base.width-1 do local v=base:getPixel(x,y);local r=C.role_of[v]
   if app.pixelColor.rgbaA(v)>0 and r~='cast' and r~='contact' then body:drawPixel(x,y,v) end end end
  frames[#frames+1]={key=role..'_'..face..'_damage',duration=1,images=D.marks(C,body,material,1,face+#role*3,{count=2,spacing=9})}
 end
 C.document_images(role..'_damage',64,64,32,50,layers,frames,'damage_facings',m,packer,out,entries)
end
C.write(root..'/tools/art_v18/sources-damage.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' damage overlays; manifest has '..C.count(m)..' entries.')
