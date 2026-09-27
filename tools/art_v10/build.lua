-- Original root-authored scripted Aseprite artwork for chart and craft.
-- Run only to reconstruct v10 sources; use export_sources.lua after artist edits.
local root=assert(app.params.root)
local p=dofile(root..'/tools/art_v5/painter.lua')
local h=dofile(root..'/tools/art_v8/common.lua')
local function read(path)local f=assert(io.open(path));local s=f:read('*a');f:close();return s end
local function write(path,s)local f=assert(io.open(path,'w'));f:write(s);f:close()end
local font=json.decode(read(root..'/tools/art_v8/font7.json'))
local manifest=json.decode(read(root..'/art/archive/art-v9/game-assets.json'))
manifest.height=7168
local sources={};local sx,sy,row=0,6144,0
local function text(c,s,x,y,col)
 for i=1,#s do for yy,line in ipairs(font[s:sub(i,i)]or font['?'])do
  for xx=1,7 do if line:sub(xx,xx)=='1'then c:pixel(x+(i-1)*8+xx-1,y+yy-1,col)end end
 end end
end
local function document(name,w,hh,ax,ay,count,dt,draw,layers)
 local sprite=Sprite(w,hh,ColorMode.RGB);p.install_palette(sprite)
 for i,n in ipairs(layers or p.layers)do local l=i==1 and sprite.layers[1]or sprite:newLayer();l.name=n end
 for phase=0,count-1 do
  local f=phase+1;if f>1 then sprite:newEmptyFrame(f)end
  local duration=dt
  if name=='manual_bulwark'then duration=phase==0 and .8 or 1 end
  if name=='manual_loom'then duration=phase==2 and 1 or .8 end
  sprite.frames[f].duration=duration
  local ims={};for _,n in ipairs(p.layers)do ims[n]=Image(w,hh,ColorMode.RGB);ims[n]:clear()end
  local key=count==1 and name or name..'_'..phase
  local c=p.new(ims,{key=key,x=0,y=0,w=w,h=hh,anchor_x=ax,anchor_y=ay},0)
  draw(c,phase)
  for i,n in ipairs(p.layers)do sprite:newCel(sprite.layers[i],f,ims[n],Point(0,0))end
  if sx+w>2048 then sx=0;sy=sy+row;row=0 end
  assert(not manifest.sprites[key]);assert(sy+hh<=7168)
  local ui=name:match('^manual_')or name:match('_emblem$')or name:match('_plate$')or name:match('_badge$')
  local ex,ey=ui and 0 or ax,ui and 0 or ay
  manifest.sprites[key]={x=sx,y=sy,w=w,h=hh,anchor_x=ex,anchor_y=ey}
  sources[#sources+1]={key=key,document=name,frame=f,w=w,h=hh,anchor_x=ex,anchor_y=ey,duration_ms=math.floor(duration*1000+.5)}
  sx=sx+w;row=math.max(row,hh)
  local flat=Image(w,hh,ColorMode.RGB);flat:drawSprite(sprite,f)
  flat:saveAs(root..'/output/art-v10/'..key..'.png')
 end
 sprite:newTag(1,count).name=name
 sprite:saveAs(root..'/art/source/v10/'..name..'.aseprite');sprite:close()
end

-- Seam-compatible current ribbons. End runs agree for all three variants.
-- The body of a ribbon holds; a short, broken light crest moves along it.
for variant=0,2 do local v=variant
 document('water_current_'..v,128,64,0,0,8,.2,function(c,phase)
  local original=c.pixel
  c.pixel=function(self,x,y,col)
   x,y=math.floor(x+.5),math.floor(y+.5)
   if x>=0 and x<128 and y>=0 and y<64 then original(self,x,y,col)end
  end
  for ribbon=0,1 do
   local yy=ribbon*32
   local points={{-16,14},{0,12},{16,14},{32,21-v*2},{48,25-v},{64,23+v},{80,16+v*2},{96,9+v},{112,9},{128,12},{144,14}}
   local function wave(offset,col,accent)
    local pix=c.pixel
    c.pixel=function(self,x,y,color)
     local xx=math.floor(x)
     local runs=ribbon==0 and ({ {{-16,24},{48,100},{122,144}},{{-16,20},{64,78},{108,144}},{{-16,22},{37,57},{96,144}} })[v+1]
       or ({ {{12,56},{89,106}},{{24,39},{60,101}},{{8,65},{82,92}} })[v+1]
     local show=false;for _,run in ipairs(runs)do if xx>=run[1]and xx<=run[2]then show=true end end
     local phase_x=(xx+phase*16+ribbon*37)%128
     if show and (not accent or(xx>10 and xx<117 and(phase_x<10 or(phase_x>17 and phase_x<21))))then pix(self,x,y,color)end
    end
    for i=2,#points do c:line({points[i-1][1],points[i-1][2]+yy+offset},{points[i][1],points[i][2]+yy+offset},col)end
    c.pixel=pix
   end
   c:layer(2);wave(2,'sea1',false)
   c:layer(4);wave(0,'sea2',false)
   c:layer(5);wave(0,'sea3',true)
  end
  c:layer(6)
  for _,a in ipairs({{43,5},{83,37}})do
   local x=a[1]+(phase%4==2 and 1 or 0)
   c:line({x,a[2]},{x+4,a[2]-1},'sea2')
  end
  if v==2 then
   c:layer(4)
   c:line({80,43},{89,40},'sea2');c:line({89,40},{98,43},'sea2')
   c:line({101,47},{96,51},'sea2');c:line({96,51},{87,52},'sea2')
   c:layer(5);if phase==2 or phase==3 then c:line({87,41},{92,41},'sea3')end
  end
 end,{'quiet transparency','ribbon undersides','spare water plane','connected current forms','traveling crests','small secondary ripples'})
end

local function wheel(c,x,y)
 h.oval(c,x,y,12,8,'ink');h.oval(c,x,y-1,10,6,'steel1');h.oval(c,x,y-1,5,4,'steel2')
 h.oval(c,x,y-1,2,2,'amber2');h.line(c,x-7,y-4,x-3,y-6,'steel3')
end
local function manual_base(c)
 c:layer(1)
 h.line(c,-91,6,79,6,'ui2')
 for x=-88,72,16 do h.line(c,x,6,x,9,'ui2')end
 h.line(c,-96,-92,-96,-80,'ui3');h.line(c,-96,-92,-84,-92,'ui3')
 h.line(c,96,12,96,0,'ui3');h.line(c,96,12,84,12,'ui3')
end
document('manual_bulwark',224,144,110,116,3,1,function(c,phase)
 manual_base(c)
 local lift=({-10,-4,0})[phase+1];local reach=({36,43,49})[phase+1]
 c:layer(2)
 h.poly(c,{{-62,-20},{-28,-38},{37,-14},{6,5}},'steel0')
 h.poly(c,{{-60,-27},{-27,-44},{36,-23},{6,-6}},'steel2')
 h.line(c,-58,-28,-28,-43,'steel3')
 c:layer(3)
 wheel(c,-46,-4);wheel(c,-13,4);wheel(c,19,-9)
 h.poly(c,{{-60,-14},{-18,5},{-4,0},{-46,-20}},'steel1')
 h.line(c,-60,-15,-18,4,'steel3')
 for _,x in ipairs({-29,24})do
  h.rect(c,x,-22,5,21+lift,'steel3');h.rect(c,x+1,-18,2,15+lift,'steel1')
  h.poly(c,{{x-6,lift},{x+2,lift-4},{x+12,lift},{x+4,lift+4}},'steel1')
  h.line(c,x-5,lift,x+3,lift+3,'ivory2')
 end
 c:layer(4)
 h.poly(c,{{-51,-44},{-28,-56},{1,-43},{-20,-31}},'ivory3')
 h.poly(c,{{-51,-43},{-20,-30},{-20,-14},{-51,-27}},'rust2')
 h.poly(c,{{-20,-30},{1,-42},{1,-25},{-20,-14}},'rust1')
 h.poly(c,{{-47,-39},{-27,-30},{-27,-21},{-47,-30}},'steel0')
 h.line(c,-46,-38,-28,-30,'glass2')
 -- The opened side housing deliberately exposes one connected pressure ram.
 h.poly(c,{{-16,-36},{12,-45},{38,-32},{12,-19},{-16,-27}},'ivory2')
 h.poly(c,{{-11,-32},{13,-40},{32,-31},{11,-22}},'steel0')
 h.poly(c,{{-10,-30},{1,-35},{20,-28},{10,-23}},'amber1')
 h.line(c,-7,-31,10,-25,'amber3',3)
 h.line(c,9,-25,reach-3,-13,'steel3',3)
 h.line(c,9,-22,reach-3,-10,'steel0')
 h.poly(c,{{-12,-37},{-5,-55},{18,-47},{13,-29}},'ivory3')
 h.line(c,-11,-38,-5,-54,'ivory4')
 c:layer(5)
 local top=phase==0 and -52 or -38
 h.poly(c,{{reach-5,top-13},{reach+19,top},{reach+19,-3+lift},{reach-5,-15+lift}},'rust1')
 h.poly(c,{{reach+19,top},{reach+31,top-7},{reach+31,-10+lift},{reach+19,-3+lift}},'ivory2')
 h.poly(c,{{reach-3,top-10},{reach+15,top-1},{reach+15,-10+lift},{reach-3,-19+lift}},'ivory3')
 h.line(c,reach-4,top-12,reach+19,top,'ivory4')
 h.line(c,reach+1,top+1,reach+12,top+6,'rust2',3)
 h.line(c,reach+20,top+3,reach+20,-5+lift,'steel0')
 -- The near outrigger is in front of the chassis so its load path is clear.
 local foot=({-8,-2,4})[phase+1]
 h.line(c,17,-11,35,-5,'steel1',5)
 h.rect(c,34,-8,5,foot+9,'steel3');h.rect(c,36,-5,2,foot+6,'steel1')
 h.poly(c,{{28,foot+1},{35,foot-3},{47,foot+2},{40,foot+6}},'steel1')
 h.line(c,29,foot+1,40,foot+5,'ivory2')
 c:layer(6)
 for _,x in ipairs({-47,-24,reach+1,reach+12})do h.oval(c,x,x<0 and -26 or -24,1,1,'amber3')end
 h.line(c,-3,-51,5,-73,'ui3');h.line(c,5,-73,36,-73,'ui3')
 text(c,'RAM',c.cx+39,c.cy-78,'ui4')
 h.line(c,35,foot+3,8,22,'ui3');h.line(c,8,22,-20,22,'ui3')
 text(c,'JACK',c.cx-56,c.cy+17,'ui4')
end,{'drawing registration','rear chassis','wheels and load-bearing jacks','opened housing and ram','moving shield','mechanical annotations'})

document('manual_loom',224,144,110,116,3,.8,function(c,phase)
 manual_base(c)
 local spread=phase==0 and 26 or 44
 local tension=phase==1 and -7 or phase==2 and 4 or 0
 c:layer(2)
 h.line(c,-spread,-2,-30,-66,'jade0',6);h.line(c,-30,-65,2,-91,'jade0',6)
 h.line(c,2,-91,38,-64,'jade0',6);h.line(c,38,-64,spread+11,-3,'jade0',6)
 c:layer(3)
 for _,x in ipairs({-spread,spread+7})do
  h.poly(c,{{x-10,1},{x-2,-4},{x+10,0},{x+2,6}},'steel1')
  h.line(c,x-7,0,x+2,4,'reed3')
 end
 h.line(c,-spread,0,spread+7,0,'reed1',3)
 c:layer(4)
 h.poly(c,{{-36,-70},{-4,-96},{5,-93},{-27,-64}},'jade2')
 h.poly(c,{{5,-93},{41,-70},{40,-61},{-4,-88}},'jade1')
 h.line(c,-35,-71,-3,-95,'jade3',2)
 h.line(c,-spread,-1,-28,-65,'jade2',5);h.line(c,38,-64,spread+9,-2,'jade2',5)
 h.line(c,-spread-1,-2,-29,-66,'jade3')
 -- Open woven canopy: large, quiet planes and a few tensioned bindings.
 h.poly(c,{{-22,-76},{-2,-91},{25,-72},{19,-67},{-2,-79},{-16,-68}},'reed2')
 h.line(c,-19,-75,-1,-87,'reed3')
 c:layer(5)
 local chamber=-39+tension
 h.line(c,-20,-73,-14,chamber-5,'reed3');h.line(c,22,-72,19,chamber-4,'reed3')
 h.oval(c,3,chamber,22,16,'amber0');h.oval(c,1,chamber-2,20,14,'amber1')
 h.oval(c,-5,chamber-5,12,9,'amber2');h.oval(c,-8,chamber-9,6,3,'amber3')
 h.line(c,-13,chamber-13,15,chamber-1,'reed1',3)
 h.line(c,-15,chamber+5,14,chamber+12,'reed3',2)
 h.line(c,19,chamber-2,42,chamber-14,'steel1',5)
 h.line(c,19,chamber-4,41,chamber-16,'steel3',2)
 h.poly(c,{{37,chamber-21},{49,chamber-15},{48,chamber-7},{39,chamber-12}},'jade1')
 if phase==2 then
  h.poly(c,{{51,chamber-19},{66,chamber-26},{75,chamber-21},{62,chamber-14}},'sea2')
  h.line(c,59,chamber-21,71,chamber-24,'sea3')
 end
 c:layer(6)
 for _,a in ipairs({{-28,-65},{36,-60},{-spread,-14},{spread+4,-18}})do
  h.line(c,a[1]-3,a[2],a[1]+4,a[2]+3,'reed3',2)
  h.line(c,a[1]-3,a[2]+4,a[1]+4,a[2]+7,'reed1')
 end
 h.line(c,-18,chamber-3,-58,-57,'ui3');h.line(c,-58,-57,-86,-57,'ui3')
 text(c,'VESSEL',c.cx-94,c.cy-70,'ui4')
 h.line(c,23,-73,57,-86,'ui3');text(c,'RIG',c.cx+53,c.cy-98,'ui4')
end,{'drawing registration','rear arch','ground feet','woven structural frame','suspended vessel and release','bindings and annotations'})

local function emblem(c,f,small)
 local x,y=c.cx,c.cy;c:layer(3)
 if small then
  if f=='union'then
   h.rect(c,-6,-5,13,10,'ivory2');h.rect(c,-5,-4,11,8,'rust1')
   h.rect(c,-3,-3,2,6,'ivory3');h.rect(c,1,-3,2,6,'ivory3');h.rect(c,-3,1,6,2,'ivory3')
  else
   h.poly(c,{{-6,0},{-2,-5},{3,-5},{6,0},{2,5},{-3,5}},'jade1')
   h.line(c,-3,3,3,-3,'reed3');h.line(c,-3,-2,3,3,'jade3');h.rect(c,-1,-4,2,8,'jade3')
  end
  return
 end
 if f=='union'then
  h.poly(c,{{-10,-11},{9,-11},{11,-8},{11,9},{8,11},{-10,11},{-12,8},{-12,-8}},'steel0')
  h.rect(c,-10,-9,19,18,'ivory2');h.rect(c,-8,-7,15,14,'rust1')
  h.rect(c,-6,-5,3,10,'ivory3');h.rect(c,2,-5,3,10,'ivory3');h.rect(c,-6,2,11,3,'ivory3')
  h.line(c,-5,-5,-4,-5,'rust1');h.line(c,3,0,4,0,'rust1')
  c:layer(6);for _,a in ipairs({{-9,-8},{8,-8},{-9,8},{8,8}})do h.rect(c,a[1],a[2],1,1,'steel3')end
 else
  h.poly(c,{{-5,-11},{4,-11},{11,-4},{11,4},{4,11},{-5,11},{-12,4},{-12,-4}},'jade0')
  h.poly(c,{{-4,-9},{3,-9},{9,-3},{9,3},{3,9},{-4,9},{-10,3},{-10,-3}},'jade1')
  h.line(c,-5,7,5,-7,'reed3',2)
  h.poly(c,{{-3,2},{-8,0},{-8,-4},{-5,-3},{-1,0}},'jade3')
  h.poly(c,{{0,-2},{1,-7},{5,-8},{5,-5},{2,-1}},'jade3')
  h.poly(c,{{0,3},{5,-1},{8,-1},{8,2},{3,5}},'reed3')
  c:layer(6);h.line(c,-7,7,5,7,'jade2');h.line(c,7,-5,7,5,'jade2')
 end
end
for _,faction in ipairs({'union','assembly'})do local f=faction
 document('faction_'..f..'_emblem',24,24,12,12,1,1,function(c)emblem(c,f,false)end)
 document('faction_'..f..'_badge',16,12,8,6,1,1,function(c)emblem(c,f,true)end)
 document('faction_'..f..'_hq',24,16,12,14,1,1,function(c)
  c.cy=8;emblem(c,f,true)
 end)
 document('faction_'..f..'_plate',96,24,12,12,1,1,function(c)
  c:layer(2);h.rect(c,-12,-12,96,24,f=='union'and'ui1'or'jade0')
  emblem(c,f,false);c:layer(6)
  text(c,f=='union'and'UNION'or'ASSEMBLY',28,8,f=='union'and'ivory3'or'jade3')
 end)
end

for _,kind in ipairs({'jack','rivet','binding'})do local k=kind
 document('trace_'..k,32,16,16,8,6,.2,function(c,phase)
  local shift=phase>0 and phase<5 and phase or 0
  local mat=phase==0 and 'earth1'or phase==5 and'sea1'or'sea2'
  c:layer(3)
  if k=='jack'then
   local w=phase==5 and 6 or 10
   h.poly(c,{{-w,-1},{-2,-5},{w,0},{2,4}},mat)
   if phase<4 then h.poly(c,{{-w+3,-1},{-2,-3},{w-3,0},{2,2}},phase==0 and'salt1'or'sea1')end
  elseif k=='rivet'then
   for i,a in ipairs({{-8,1},{-3,-2},{4,0},{7,3}})do
    if phase<4 or i<3 then h.line(c,a[1]+shift,a[2],a[1]+shift+2,a[2]-1,mat)end
   end
  else
   h.poly(c,{{-9+shift,-2},{-3+shift,-4},{4+shift,0},{8+shift,1},{5+shift,3},{-3+shift,-1},{-8+shift,0}},mat)
   if phase<3 then h.line(c,-4+shift,-2,3+shift,1,phase==0 and'reed1'or'sea3')end
  end
  if phase>0 and phase<5 then
   c:layer(5);h.line(c,-12+shift,3,-4+shift,2,'sea3')
   h.line(c,1+shift,-3,7+shift,-4,'sea2')
  end
 end,{'ground reference','empty rear layer','material traces','fine fragments','washing crests','silt residue'})
end
write(root..'/tools/art_v10/sources.json',json.encode(sources))
write(root..'/art/exports/game-assets-v10.json',json.encode(manifest))
print('Root authored '..#sources..' new v10 frames; used through row '..(sy+row)..'. Export next.')
