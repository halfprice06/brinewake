-- Root-only visual refinement. Reconstructs v11 sources; preserves v10.
-- Assembled machine comparisons reuse earlier root-authored source layers at
-- exact 2x. Water, emblems, plaques and traces are newly constructed pixels.
local root=assert(app.params.root)
local painter=dofile(root..'/tools/art_v5/painter.lua')
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s)local f=assert(io.open(p,'w'));f:write(s);f:close()end
local source=json.decode(read(root..'/tools/art_v10/sources.json'))
local font=json.decode(read(root..'/tools/art_v8/font7.json'))
local palette=painter.palette
-- Two restrained water-specific midtones replace the harsh paired outlines.
palette.sea_ink='2b515b';palette.sea_mid='416b73'
local rgba={};for k,v in pairs(palette)do rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255)end
local specs={};for _,s in ipairs(source)do specs[s.document]=specs[s.document]or{};table.insert(specs[s.document],s)end
local metadata={}
local function document(name,layers,draw)
 local entries=assert(specs[name]);table.sort(entries,function(a,b)return a.frame<b.frame end)
 local w,h=entries[1].w,entries[1].h
 local sprite=Sprite(w,h,ColorMode.RGB)
 local roles={};for role in pairs(palette)do roles[#roles+1]=role end;table.sort(roles)
 local pal=Palette(#roles+1);pal:setColor(0,0)
 for i,role in ipairs(roles)do pal:setColor(i,rgba[role])end;sprite:setPalette(pal)
 for i,n in ipairs(layers)do local l=i==1 and sprite.layers[1]or sprite:newLayer();l.name=n end
 for phase=0,#entries-1 do
  local f=phase+1;if f>1 then sprite:newEmptyFrame(f)end;sprite.frames[f].duration=entries[f].duration_ms/1000
  local ims={};for i=1,#layers do ims[i]=Image(w,h,ColorMode.RGB);ims[i]:clear()end
  local c={im=ims[1],ims=ims,w=w,h=h}
  function c:layer(i)self.im=self.ims[i]end
  function c:pixel(x,y,col)
   x,y=math.floor(x+.5),math.floor(y+.5)
   if x>=0 and x<self.w and y>=0 and y<self.h then self.im:drawPixel(x,y,type(col)=='number'and col or assert(rgba[col],col))end
  end
  function c:line(x,y,x1,y1,col)
   x,y,x1,y1=math.floor(x+.5),math.floor(y+.5),math.floor(x1+.5),math.floor(y1+.5)
   local dx,dy=math.abs(x1-x),-math.abs(y1-y);local sx,sy=x<x1 and 1 or -1,y<y1 and 1 or -1;local err=dx+dy
   while true do self:pixel(x,y,col);if x==x1 and y==y1 then break end
    local e=2*err;if e>=dy then err=err+dy;x=x+sx end;if e<=dx then err=err+dx;y=y+sy end
   end
  end
  function c:rect(x,y,w,h,col)for yy=y,y+h-1 do self:line(x,yy,x+w-1,yy,col)end end
  function c:path(points,col)for i=2,#points do self:line(points[i-1][1],points[i-1][2],points[i][1],points[i][2],col)end end
  function c:poly(points,col)
   local lo,hi=1e9,-1e9;for _,p in ipairs(points)do lo=math.min(lo,p[2]);hi=math.max(hi,p[2])end
   for y=math.ceil(lo),math.floor(hi)do local cuts={};local j=#points
    for i,a in ipairs(points)do local b=points[j];if(a[2]<=y and b[2]>y)or(b[2]<=y and a[2]>y)then cuts[#cuts+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2])end;j=i end
    table.sort(cuts);for i=1,#cuts-1,2 do self:line(math.ceil(cuts[i]),y,math.ceil(cuts[i+1])-1,y,col)end
   end
  end
  function c:text(s,x,y,col)
   for i=1,#s do for yy,row in ipairs(font[s:sub(i,i)]or font['?'])do for xx=1,7 do
    if row:sub(xx,xx)=='1'then self:pixel(x+(i-1)*8+xx-1,y+yy-1,col)end
   end end end
  end
  draw(c,phase)
  for i,im in ipairs(ims)do sprite:newCel(sprite.layers[i],f,im,Point(0,0))end
  local image=Image(w,h,ColorMode.RGB);image:drawSprite(sprite,f);image:saveAs(root..'/output/art-v11/'..entries[f].key..'.png')
  local e=entries[f]
  metadata[#metadata+1]={key=e.key,document=e.document,frame=e.frame,w=e.w,h=e.h,
   anchor_x=e.anchor_x,anchor_y=e.anchor_y,duration_ms=e.duration_ms,layers=layers}
 end
 sprite:newTag(1,#entries).name=name
 sprite:saveAs(root..'/art/source/v11/'..name..'.aseprite');sprite:close()
end

-- Reuse exact source layers so a guide machine is recognizably the machine
-- the player commands. The source layers remain separately editable.
local mappings={}
for _,v in ipairs({8,9})do for _,s in ipairs(json.decode(read(root..'/tools/art_v'..v..'/sources.json')))do
 mappings[s.key]={path=root..'/art/source/v'..v..'/'..s.document..'.aseprite',frame=s.frame}
end end
for _,unit in ipairs({'bulwark','loom'})do mappings[unit..'_0']={path=root..'/art/source/v7/'..unit..'.aseprite',frame=1}end
local cached={}
local function copy_machine(c,key,ox,oy,first_layer)
 local m=assert(mappings[key],key);local s=cached[m.path]
 if not s then s=assert(app.open(m.path));cached[m.path]=s end
 assert(s.width==64 and s.height==64 and #s.layers==6)
 for i,layer in ipairs(s.layers)do local cel=layer:cel(m.frame)
  if cel then
   c:layer(first_layer+i-1)
   for y=0,cel.image.height-1 do for x=0,cel.image.width-1 do
    local col=cel.image:getPixel(x,y)
    if app.pixelColor.rgbaA(col)>0 then
     local px=ox+(x+cel.position.x)*2;local py=oy+(y+cel.position.y)*2
     c:rect(px,py,2,2,col)
    end
   end end
  end
 end
end
for _,unit in ipairs({'bulwark','loom'})do local u=unit
 local layers={'comparison ground and divider'}
 for _,side in ipairs({'mobile','deployed'})do for _,name in ipairs(painter.layers)do layers[#layers+1]=side..' - '..name end end
 layers[#layers+1]='state headings and highlights'
 document('manual_'..u,layers,function(c,phase)
  c:layer(1)
  c:rect(110,12,1,45,'ui2');c:rect(110,89,1,40,'ui2')
  copy_machine(c,u..'_0',-8,12,2)
  local key=u..'_0_deploy_2'
  if u=='loom'and phase==2 then key='loom_0_pressure_4'end
  copy_machine(c,key,104,12,8)
  c:layer(14)
  c:text('MOBILE',26,3,phase==0 and'ivory3'or'ui4')
  c:text('DEPLOYED',131,3,phase>0 and'ivory3'or'ui4')
  c:text(u=='bulwark'and'MOVE'or'NO FIRE',u=='bulwark'and 42 or 26,134,'ui4')
  c:text(u=='bulwark'and'HOLD FRONT'or'CAN FIRE',u=='bulwark'and 126 or 134,134,u=='bulwark'and'amber3'or'jade3')
  c:rect(phase==0 and 18 or 122,5,3,3,'amber3')
  if phase==1 then
   c:line(101,73,122,73,'amber3');c:line(122,73,117,69,'amber3');c:line(122,73,117,77,'amber3')
  elseif phase==2 then
   c:path({{104,72},{110,78},{120,65}},'jade3')
  end
 end)
end
for _,s in pairs(cached)do s:close()end

-- Surface clusters have a body, a broken light edge and quiet surroundings.
-- Shared edge fragments continue across variants; highlights stay off seams.
local water_paths={
 {{{-3,12},{10,12},{18,10}},{{24,9},{34,10},{43,14},{51,16}},{{60,19},{69,20},{78,18},{89,14}},{{103,12},{113,12},{131,12}},{{10,46},{21,43},{32,44},{39,47}},{{63,52},{71,49},{83,49},{91,51}}},
 {{{-3,12},{10,12},{18,10}},{{29,6},{37,5},{47,8},{57,13}},{{67,18},{75,20},{86,19},{95,15}},{{103,12},{113,12},{131,12}},{{22,41},{33,40},{44,44},{48,48}},{{81,51},{91,48},{104,49}}},
 {{{-3,12},{10,12},{18,10}},{{24,22},{35,20},{46,22},{50,25}},{{57,8},{64,5},{76,4},{87,7},{94,11}},{{103,12},{113,12},{131,12}},{{16,49},{25,45},{37,45}},{{70,44},{77,41},{86,42},{91,45},{87,48},{80,49}}}
}
for variant=0,2 do local v=variant
 document('water_current_'..v,{'quiet transparency','water shadow shapes','surface bodies','broken reflected edges','traveling glints','secondary ripple'},function(c,phase)
  for i,points in ipairs(water_paths[v+1])do
   c:layer(2)
   if i~=1 and i~=4 then
    local shadow={};for _,p in ipairs(points)do shadow[#shadow+1]={p[1]+1,p[2]+3}end
    c:path(shadow,'sea_ink')
   end
   c:layer(3)
   for p=2,#points do
    local a,b=points[p-1],points[p]
    c:line(a[1],a[2]+1,b[1],b[2]+1,'sea1')
   end
   c:layer(4);c:path(points,'sea_mid')
   -- Short glints overlap for two frames and move by small pixel steps.
   if i~=1 and i~=4 then
    local p=points[2];local drift=({-2,-1,0,1,2,1,0,-1})[(phase+i*2)%8+1]
    c:layer(5)
    if(phase+i)%8<5 then c:line(p[1]-2+drift,p[2],p[1]+2+drift,p[2],'sea2')end
   end
  end
  c:layer(6)
  local x=({49,63,112})[v+1];local y=({37,34,34})[v+1]
  c:line(x,y,x+5,y-1,'sea_mid')
 end)
end

local function enamel(c,x,y,w,h,fill)
 c:poly({{x+2,y},{x+w-3,y},{x+w-1,y+2},{x+w-1,y+h-3},{x+w-3,y+h-1},{x+2,y+h-1},{x,y+h-3},{x,y+2}},'steel0')
 c:rect(x+2,y+2,w-4,h-4,fill)
 c:line(x+2,y+1,x+w-4,y+1,'ivory2');c:line(x+1,y+2,x+1,y+h-4,'steel2')
 c:line(x+3,y+h-2,x+w-3,y+h-2,'steel1')
end
local function sign(c,f,x,y,size)
 c:layer(3)
 if size==24 then enamel(c,x,y,24,24,f=='union'and'rust1'or'jade1')
 else enamel(c,x,y,14,12,f=='union'and'rust1'or'jade1')end
 c:layer(5)
 if f=='union'then
  if size==24 then
   c:rect(x+5,y+5,3,12,'ivory3');c:rect(x+15,y+5,3,12,'ivory3')
   c:rect(x+5,y+5,13,3,'ivory3');c:rect(x+10,y+8,2,5,'amber3')
   c:line(x+10,y+13,x+13,y+13,'amber3');c:pixel(x+14,y+12,'amber3')
   c:path({{x+4,y+19},{x+8,y+18},{x+12,y+19},{x+17,y+18},{x+19,y+18}},'ivory2')
  else
   c:rect(x+3,y+3,2,6,'ivory3');c:rect(x+9,y+3,2,6,'ivory3');c:rect(x+3,y+3,8,2,'ivory3');c:rect(x+6,y+5,2,3,'amber3')
  end
 else
  if size==24 then
   c:poly({{x+5,y+17},{x+6,y+9},{x+11,y+4},{x+13,y+4},{x+18,y+9},{x+19,y+17},{x+16,y+17},{x+15,y+10},{x+12,y+7},{x+9,y+10},{x+8,y+17}},'jade3')
   c:poly({{x+10,y+12},{x+12,y+10},{x+14,y+12},{x+14,y+16},{x+12,y+18},{x+10,y+16}},'reed3')
   c:line(x+5,y+19,x+18,y+19,'ivory2')
  else
   c:path({{x+3,y+8},{x+4,y+4},{x+6,y+2},{x+8,y+2},{x+10,y+4},{x+11,y+8}},'jade3')
   c:rect(x+6,y+5,2,4,'reed3')
  end
 end
end
for _,faction in ipairs({'union','assembly'})do local f=faction
 local layers={'contact reference','plate backing','enamel frame','face material','faction symbol','edge finishing'}
 document('faction_'..f..'_emblem',layers,function(c)sign(c,f,0,0,24)end)
 document('faction_'..f..'_badge',layers,function(c)sign(c,f,1,0,14)end)
 document('faction_'..f..'_hq',layers,function(c)
  sign(c,f,5,1,14);c:layer(6);c:pixel(4,6,'steel3');c:pixel(19,6,'steel3')
 end)
 document('faction_'..f..'_plate',layers,function(c)
  c:layer(2);enamel(c,0,0,96,24,'ui1')
  sign(c,f,5,6,14)
  c:layer(6);c:text(f=='union'and'UNION'or'ASSEMBLY',27,8,f=='union'and'ivory3'or'jade3')
 end)
end

for _,kind in ipairs({'jack','rivet','binding'})do local k=kind
 document('trace_'..k,{'ground contact','material shadow','recognizable remains','surface wear','water crossing the mark','settled silt'},function(c,phase)
  if phase==5 then
   c:layer(6)
   if k=='jack'then c:line(7,8,15,11,'wet1');c:line(19,6,24,8,'wet1')
   elseif k=='rivet'then c:rect(15,6,2,1,'wet1')
   else c:path({{10,6},{14,8},{19,8}},'wet1')end
   return
  end
  local flow=({0,0,1,2,3,0})[phase+1]
  local base=phase==0 and'earth0'or phase==5 and'sea1'or'sea_mid'
  c:layer(2)
  if k=='jack'then
   c:poly({{5,7},{13,3},{26,8},{18,12}},base)
   c:layer(3);c:poly({{9,7},{14,5},{22,8},{17,10}},phase==0 and'earth2'or'sea1')
   c:layer(4);if phase<3 then c:line(6,8,16,12,phase==0 and'earth1'or'sea2')end
  elseif k=='rivet'then
   local pieces={{8,8},{14,6},{21,9}}
   for i,p in ipairs(pieces)do if phase<4 or i==2 then
    local x,y=p[1]+flow*(i==1 and 1 or 0),p[2]
    c:layer(2);c:rect(x,y,4,2,base)
    c:layer(3);c:rect(x+1,y-1,2,2,phase==0 and'steel2'or base)
    c:layer(4);if phase<2 then c:pixel(x+1,y-1,phase==0 and'steel3'or'sea2')end
   end end
  else
   c:poly({{5+flow,6},{10+flow,4},{15+flow,7},{23+flow,7},{25+flow,10},{19+flow,11},{13+flow,8},{7+flow,9}},base)
   if phase<4 then
    c:layer(3);c:path({{7+flow,6},{10+flow,5},{15+flow,8},{22+flow,8}},phase==0 and'reed1'or'sea1')
    c:layer(4);c:line(11+flow,6,10+flow,8,phase==0 and'reed2'or'sea2')
   end
  end
  if phase>0 and phase<5 then
   c:layer(5)
   local y=({0,4,6,9,12})[phase+1]
   c:path({{3,y},{10,y-1},{18,y+1},{27,y}},'sea_mid')
   c:line(10,y-1,14,y,'sea2')
  end
 end)
end
assert(#metadata==56)
write(root..'/tools/art_v11/sources.json',json.encode(metadata))
write(root..'/art/exports/game-assets-v11.json',read(root..'/art/archive/art-v10/game-assets.json'))
print('Root refined all 56 frames in 16 v11 documents. Export from the saved sources next.')
