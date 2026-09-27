-- Root-authored v18 portraits and effects. The portrait drawings are the
-- v16/v17 console close-ups saved on two layers named for what they hold
-- (`plate`, `machine`) instead of the six machine layers. The effects are
-- redrawn: the VENT plume is a smoke that forms, splits, thins and breaks up
-- rather than one shape sliding upward; SOUND is a ring of short arcs; the
-- mending sparks are a burst that travels outward and dies, each spark a
-- bright head with a darker tail. Each effect document has one `effect`
-- layer.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v18/common.lua')
local painter=C.painter
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v18/portraits';os.execute('mkdir -p '..out)
local layers=painter.layers

local function p(c,x,y)return {c.cx+x,c.cy+y}end
local function poly(c,pts,col)local q={};for _,v in ipairs(pts)do q[#q+1]=p(c,v[1],v[2])end;c:poly(q,col)end
local function line(c,x,y,xx,yy,col,w)c:line(p(c,x,y),p(c,xx,yy),col,w)end
local function rect(c,x,y,w,h,col)poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},col)end
local function ellipse(c,x,y,rx,ry,col)c:ellipse(c.cx+x,c.cy+y,rx,ry,col)end
local function plate(c,faction)
  c:layer(2);rect(c,0,0,56,56,'ui0')
  poly(c,{{1,1},{47,1},{55,9},{55,55},{1,55}},'ui1')
  poly(c,{{1,1},{29,1},{1,30}},'ui2')
  for y=8,51,12 do line(c,3,y,52,y,'ui1')end
  rect(c,47,4,5,2,faction=='union' and 'rust3' or 'jade3')
  c:layer(4)
end
local function cab(c,x,y)
  poly(c,{{x,y+6},{x+13,y},{x+28,y+6},{x+28,y+24},{x+16,y+32},{x,y+24}},'ivory0')
  poly(c,{{x+1,y+6},{x+13,y+1},{x+27,y+6},{x+16,y+12}},'ivory3')
  poly(c,{{x+1,y+7},{x+16,y+13},{x+16,y+30},{x+1,y+23}},'ivory2')
  poly(c,{{x+17,y+13},{x+27,y+7},{x+27,y+23},{x+17,y+29}},'ivory1')
  poly(c,{{x+3,y+10},{x+14,y+15},{x+14,y+23},{x+3,y+18}},'glass0')
  poly(c,{{x+4,y+11},{x+11,y+14},{x+4,y+16}},'glass2')
  line(c,x+4,y+11,x+12,y+15,'glass3')
  poly(c,{{x+19,y+14},{x+25,y+10},{x+25,y+18},{x+19,y+22}},'glass0')
  line(c,x+20,y+14,x+24,y+11,'glass2')
  line(c,x+3,y+23,x+12,y+27,'rust2',2)
  line(c,x+2,y+7,x+14,y+12,'ivory4')
  rect(c,x+5,y+5,5,2,'steel1');rect(c,x+5,y+5,5,1,'steel3')
  rect(c,x+19,y+25,2,2,'ivory3')
end
local portraits={}
-- Tidewatch: a wheel, the low seat and the tall mast with its glass eye.
portraits.tidewatch=function(c)
  plate(c,'union')
  ellipse(c,16,40,10,10,'steel0');ellipse(c,16,40,8,8,'steel1');ellipse(c,16,40,4,4,'rust2');ellipse(c,15,39,2,2,'steel3')
  poly(c,{{22,30},{44,26},{48,34},{26,40}},'rust1');poly(c,{{24,31},{43,27},{45,32},{27,37}},'rust3')
  poly(c,{{30,18},{42,16},{43,28},{31,30}},'ivory2');line(c,31,18,42,16,'ivory4')
  rect(c,36,4,3,26,'steel1');rect(c,37,4,1,26,'steel3')
  ellipse(c,37,6,5,5,'steel0');ellipse(c,37,6,3,3,'glass1');rect(c,35,4,2,2,'glass4')
  line(c,39,12,46,10,'rust2',2)
end
-- Caulker: tracks, the spool and the jib ending in the amber torch.
portraits.caulker=function(c)
  plate(c,'union');cab(c,20,16)
  poly(c,{{4,44},{50,38},{52,46},{6,52}},'steel0');for x=6,46,8 do rect(c,x,45,4,4,'steel2') end
  ellipse(c,14,28,7,7,'steel0');ellipse(c,14,28,5,5,'steel2');ellipse(c,14,28,2,2,'steel4')
  line(c,14,22,34,8,'rust2',3);line(c,34,8,50,14,'steel2',2)
  ellipse(c,50,14,3,3,'amber1');ellipse(c,50,14,2,2,'amber3');line(c,51,15,55,19,'amber4')
end
-- Caisson: the plug, strapped, on big wheels.
portraits.caisson=function(c)
  plate(c,'union')
  poly(c,{{10,10},{44,6},{46,40},{12,44}},'ivory0');poly(c,{{11,11},{43,7},{44,38},{12,42}},'ivory2')
  poly(c,{{12,12},{28,10},{28,40},{13,42}},'ivory3')
  for x=14,42,9 do line(c,x,10,x,42,'steel1',2);line(c,x+1,10,x+1,42,'steel3') end
  line(c,11,26,45,22,'rust1',3);line(c,11,25,45,21,'rust3')
  ellipse(c,16,48,7,6,'steel0');ellipse(c,16,48,5,4,'rust2');ellipse(c,40,47,7,6,'steel0');ellipse(c,40,47,5,4,'rust2')
end
-- Lampwright: the lantern on its reed pole above a slim body.
portraits.lampwright=function(c)
  plate(c,'assembly')
  poly(c,{{16,28},{34,24},{36,46},{18,50}},'jade1');poly(c,{{17,29},{33,25},{34,44},{18,48}},'jade2');line(c,17,29,33,25,'jade4')
  rect(c,22,20,10,8,'reed1');rect(c,23,20,8,2,'reed3')
  line(c,26,20,32,8,'reed2',2);line(c,32,8,44,6,'reed2',2)
  rect(c,42,4,10,14,'steel1');rect(c,43,6,8,10,'glass1');rect(c,44,7,3,4,'glass4');rect(c,41,3,12,2,'steel3');rect(c,41,18,12,2,'steel2')
  ellipse(c,13,30,4,4,'amber1');ellipse(c,13,30,2,2,'amber3')
end
-- Tender: the basket of cordage and the bobbin arm.
portraits.tender=function(c)
  plate(c,'assembly');cab(c,22,18)
  poly(c,{{4,34},{24,28},{26,48},{6,53}},'reed1');for y=36,50,4 do line(c,5,y,24,y-6,'reed0') end
  line(c,6,34,24,28,'reed3')
  line(c,12,30,30,10,'jade2',3);line(c,30,10,48,14,'steel2',2)
  ellipse(c,48,14,4,4,'reed0');ellipse(c,48,14,3,3,'reed2');line(c,49,15,53,20,'amber3')
  ellipse(c,40,44,4,4,'amber1');ellipse(c,40,44,2,2,'amber3')
end
-- Dredger: the bucket chain over the bow and the hopper.
portraits.dredger=function(c)
  plate(c,'assembly')
  poly(c,{{4,30},{40,22},{50,32},{46,48},{8,52}},'jade1');poly(c,{{6,31},{39,24},{47,32},{44,46},{9,50}},'jade2');line(c,6,31,39,24,'jade4')
  rect(c,10,18,16,12,'reed1');rect(c,11,18,14,3,'reed3');rect(c,12,14,12,5,'rust2')
  line(c,30,26,52,6,'steel1',3);line(c,30,25,52,5,'steel3')
  for i=0,3 do local x=34+i*5;local y=22-i*5;rect(c,x-2,y-2,5,4,'steel0');rect(c,x-1,y-2,3,2,'steel2') end
  ellipse(c,20,40,4,4,'amber1');ellipse(c,20,40,2,2,'amber3')
  poly(c,{{4,51},{14,49},{16,55},{6,55}},'jade3');poly(c,{{36,49},{48,47},{50,54},{38,55}},'jade3')
end

-- Barge: the long hull, the cargo well, the wheelhouse and the stern paddle.
portraits.barge=function(c)
  plate(c,'assembly')
  poly(c,{{4,34},{44,26},{52,34},{48,46},{8,50}},'jade1');poly(c,{{6,35},{43,28},{49,34},{46,44},{9,48}},'jade2');line(c,6,35,43,28,'jade4')
  for i=0,4 do line(c,10+i*7,34+i,14+i*7,44,'jade1') end
  rect(c,20,22,16,10,'reed1');rect(c,21,22,14,3,'reed3');line(c,20,25,36,25,'ivory3')
  cab(c,4,8)
  ellipse(c,46,20,7,7,'steel0');ellipse(c,46,20,5,5,'jade1');line(c,41,20,51,20,'steel3');line(c,46,15,46,25,'steel3')
  line(c,40,30,40,12,'reed2',2);ellipse(c,40,11,3,3,'amber1');ellipse(c,40,11,2,2,'amber3')
  poly(c,{{4,49},{14,47},{16,54},{6,55}},'jade3')
end
-- Lifter: the gasbags above, the gondola and its cabin below, rigging between.
portraits.lifter=function(c)
  plate(c,'union')
  ellipse(c,20,14,16,7,'ivory1');ellipse(c,20,13,15,6,'ivory2');line(c,6,11,34,9,'ivory4');ellipse(c,20,15,15,3,'ivory1')
  ellipse(c,36,18,15,6,'ivory1');ellipse(c,36,17,14,5,'ivory3');line(c,23,15,49,14,'ivory4')
  for _,x in ipairs({10,20,30,42}) do line(c,x,20,x+4,34,'steel1') end
  poly(c,{{8,34},{40,30},{46,38},{42,48},{10,52}},'rust1');poly(c,{{10,35},{39,31},{44,38},{41,46},{11,50}},'rust2');line(c,10,35,39,31,'rust4')
  cab(c,26,19)
  ellipse(c,7,44,5,5,'steel0');line(c,3,44,11,44,'steel3');line(c,7,40,7,48,'steel3')
  ellipse(c,44,49,2,2,'amber3');rect(c,18,50,6,2,'steel1')
end

for _,n in ipairs({'tidewatch','caulker','caisson','lampwright','tender','dredger','barge','lifter'}) do
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(56,56,ColorMode.RGB);im:clear();ims[layer]=im end
  local c=painter.new(ims,{key='portrait',x=0,y=0,w=56,h=56,anchor_x=0,anchor_y=0},0)
  portraits[n](c)
  -- The plate was drawn on layer 2 and the machine on layer 4.
  C.document_images('portrait_'..n,56,56,0,0,{'plate','machine'},{{key='portrait_'..n,duration=1,images={ims[layers[2]],ims[layers[4]]}}},'portrait',m,packer,out,entries)
end
-- Effects.
local fx=root..'/output/art-v18/effects';os.execute('mkdir -p '..fx)
local function frames_of(prefix,count,w,h,ax,ay,draw,durations)
  local list={}
  for phase=0,count-1 do
    local ims={}
    for i,layer in ipairs(layers) do local im=Image(w,h,ColorMode.RGB);im:clear();ims[layer]=im end
    local c=painter.new(ims,{key=prefix,x=0,y=0,w=w,h=h,anchor_x=ax,anchor_y=ay},0)
    c:layer(5)
    draw(c,phase)
    list[#list+1]={key=prefix..'_'..phase,duration=durations and durations[phase+1] or .1,images={ims[layers[5]]}}
  end
  return list
end
-- A short arc of a ground ellipse, as a run of connected pixels.
local function arc(c,rx,ry,a0,a1,col,thick)
  local steps=math.max(2,math.floor((a1-a0)*rx/2))
  local px,py
  for i=0,steps do
    local t=a0+(a1-a0)*i/steps
    local x=math.floor(math.cos(t)*rx+.5);local y=math.floor(math.sin(t)*ry+.5)
    if px then c:line({c.cx+px,c.cy+py},{c.cx+x,c.cy+y},col) else c:pixel(c.cx+x,c.cy+y,col) end
    if thick and thick>1 then c:pixel(c.cx+x,c.cy+y+1,col) end
    px,py=x,y
  end
end
-- SOUND: a ring of light on the ground that widens and thins. Eight arcs
-- on the first frame, thinning to four faint ones as it spreads.
local sound=frames_of('fx_sound',4,64,32,32,16,function(c,phase)
  local r=6+phase*7
  local cols={'glass4','glass3','glass2','glass1'}
  local n=({8,8,6,4})[phase+1]
  local span=({0.55,0.45,0.35,0.3})[phase+1]
  for k=0,n-1 do
    local a=k*2*math.pi/n+phase*0.3
    arc(c,r,r/2,a,a+span,cols[phase+1],phase<2 and 2 or 1)
  end
  if phase==0 then ellipse(c,0,0,3,1,'glass4') end
end,{.15,.15,.15,.15})
-- VENT: smoke from the stack. Frame 0 is the dense contrast frame at the
-- mouth with the hot core; then the plume rises as one lobed mass, splits
-- into two, and breaks into pale fragments downwind.
local vent=frames_of('fx_vent',4,48,48,24,40,function(c,phase)
  if phase==0 then
    ellipse(c,0,-5,4,3,'steel0');ellipse(c,-1,-6,2,2,'steel1');ellipse(c,1,-8,2,1,'steel1')
    ellipse(c,0,-2,2,1,'amber3');c:pixel(c.cx,c.cy-2,'amber4')
  elseif phase==1 then
    ellipse(c,1,-10,6,4,'steel1');ellipse(c,-3,-12,4,3,'steel2');ellipse(c,3,-14,3,2,'steel2')
    ellipse(c,-2,-13,2,1,'steel3');ellipse(c,0,-6,3,2,'steel0')
    ellipse(c,0,-3,1,1,'amber2')
  elseif phase==2 then
    ellipse(c,-3,-16,5,3,'steel2');ellipse(c,5,-18,5,3,'steel2')
    ellipse(c,-4,-18,3,2,'steel3');ellipse(c,6,-20,3,2,'steel3')
    ellipse(c,1,-13,3,2,'steel1');ellipse(c,0,-9,2,1,'steel1')
  else
    ellipse(c,-4,-22,3,2,'steel3');ellipse(c,6,-24,4,2,'steel3');ellipse(c,2,-18,2,1,'steel2')
    ellipse(c,8,-27,2,1,'steel3');ellipse(c,-6,-19,2,1,'steel2')
  end
end,{.12,.12,.12,.12})
-- MEND: rivet sparks. A bright burst with a dark contrast core, then
-- sparks flying out as short streaks (bright head, darker tail), then the
-- last embers far out, two pixels each.
local mend=frames_of('fx_mend',3,24,24,12,12,function(c,phase)
  local dirs={{-1,-1},{1,-1},{-1,1},{1,1},{0,-1},{1,0}}
  if phase==0 then
    ellipse(c,0,0,2,2,'ivory4');ellipse(c,0,0,1,1,'amber3')
    for _,d in ipairs(dirs) do line(c,d[1]*2,d[2]*2,d[1]*3,d[2]*3,'amber3') end
  elseif phase==1 then
    c:pixel(c.cx,c.cy,'amber2')
    for i,d in ipairs(dirs) do
      local r=4+(i%2)*2
      line(c,d[1]*(r-2),d[2]*(r-2),d[1]*r,d[2]*r,'amber2');c:pixel(c.cx+d[1]*r,c.cy+d[2]*r,'amber4')
    end
  else
    for i,d in ipairs(dirs) do
      if i%3~=0 then local r=7+(i%2)*2;line(c,d[1]*(r-1),d[2]*(r-1),d[1]*r,d[2]*r,'amber2') end
    end
  end
end,{.08,.08,.08})
C.document_images('fx_sound',64,32,32,16,{'effect'},sound,'sound_ring',m,packer,fx,entries)
C.document_images('fx_vent',48,48,24,40,{'effect'},vent,'vent_plume',m,packer,fx,entries)
C.document_images('fx_mend',24,24,12,12,{'effect'},mend,'mend_sparks',m,packer,fx,entries)
C.write(root..'/tools/art_v18/sources-portraits.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' portrait and effect frames; manifest has '..C.count(m)..' entries.')
