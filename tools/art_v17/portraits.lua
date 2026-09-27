-- Root-authored v17 portraits: the Barge and the Lifter as v4 console
-- close-ups, the faction plate behind a machine drawn large.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v17/common.lua')
local painter=C.painter
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v17/portraits';os.execute('mkdir -p '..out)
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
for _,n in ipairs({'barge','lifter'}) do
  local ims={}
  for i,layer in ipairs(layers) do local im=Image(56,56,ColorMode.RGB);im:clear();ims[layer]=im end
  local c=painter.new(ims,{key='portrait',x=0,y=0,w=56,h=56,anchor_x=0,anchor_y=0},0)
  portraits[n](c)
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  C.document_images('portrait_'..n,56,56,0,0,layers,{{key='portrait_'..n,duration=1,images=list}},'portrait',m,packer,out,entries)
end
C.write(root..'/tools/art_v17/sources-portraits.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' portrait frames; manifest now has '..C.count(m)..' entries.')
