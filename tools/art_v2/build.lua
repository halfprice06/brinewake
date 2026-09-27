-- Build candidate native art non-destructively; root promotes it after visual review.
local root=assert(app.params.root,'root parameter required')
local core=dofile(root..'/tools/art_v2/core.lua')
local all={};local partial=app.params.partial=='1'
for _,module in ipairs({'union','assembly','structures','environment'})do
 local f=loadfile(root..'/tools/art_v2/'..module..'.lua')
 if f then for name,draw in pairs(f())do assert(not all[name],'duplicate key '..name);all[name]=draw end
 elseif not partial then error('missing module '..module)end
end
local W,H=2048,1024
local sprite=Sprite(W,H,ColorMode.RGB);local images={}
for i,name in ipairs(core.layers)do
 local layer=i==1 and sprite.layers[1] or sprite:newLayer();layer.name=name
 local image=Image(W,H,ColorMode.RGB);image:clear();images[name]=sprite:newCel(layer,1,image,Point(0,0)).image
end
local entries={}
local function draw(key,x,y,w,h,ax,ay,fn,facing,phase)
 local b={key=key,x=x,y=y,w=w,h=h,anchor_x=ax,anchor_y=ay};assert(x+w<=W and y+h<=H,'atlas allocation overflow')
 local ctx=core.context(images,b,facing);ctx.phase=phase;fn(ctx,phase);entries[key]=b
end
local units={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
for i,name in ipairs(units)do if all[name]then
 local bx=((i-1)%4)*512;local by=math.floor((i-1)/4)*320
 for face=0,7 do
  draw(name..'_'..face,bx+face*64,by,64,64,32,50,all[name],face,nil)
  if face==0 then local source=entries[name..'_0'];local alias={};for k,v in pairs(source)do alias[k]=v end;alias.key=name;entries[name]=alias end
  for phase=0,3 do draw(name..'_'..face..'_walk_'..phase,bx+face*64,by+(phase+1)*64,64,64,32,50,all[name],face,phase)end
 end
elseif not partial then error('missing unit '..name)end end
local structures={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','salvage','well'}
for i,name in ipairs(structures)do if all[name]then
 draw(name,(i-1)*160,640,160,144,80,128,all[name],1,nil)
elseif not partial then error('missing structure '..name)end end
local extra={};for name in pairs(all)do if not entries[name]then extra[#extra+1]=name end end;table.sort(extra)
for i,name in ipairs(extra)do
 local tile=name:find('terrain_',1,true)==1
 local x=((i-1)%24)*80;local y=800+math.floor((i-1)/24)*80
 draw(name,x,y,tile and 40 or 64,tile and 24 or 64,tile and 20 or 32,tile and 12 or 50,all[name],0,nil)
end
local filename=partial and 'game-assets-v2-preview' or 'game-assets-v2'
sprite:saveAs(root..'/art/source/'..filename..'.aseprite')
local composite=Image(sprite);composite:saveAs(root..'/art/exports/'..filename..'.png')
local keys={};for name in pairs(entries)do keys[#keys+1]=name end;table.sort(keys)
local file=assert(io.open(root..'/art/exports/'..filename..'.json','w'))
file:write('{"width":',W,',"height":',H,',"sprites":{\n')
for i,name in ipairs(keys)do local b=entries[name];file:write(string.format('%q:{"x":%d,"y":%d,"w":%d,"h":%d,"anchor_x":%d,"anchor_y":%d}',name,b.x,b.y,b.w,b.h,b.anchor_x,b.anchor_y));if i<#keys then file:write(',')end;file:write('\n')end
file:write('}}\n');file:close()
print('Saved '..filename..' with '..#keys..' entries, '..W..'x'..H..', six semantic layers')
