-- Original native-pixel console and close-up machine portraits, drawn by root.
local out={}
local function p(c,x,y)return {c.cx+x,c.cy+y}end
local function poly(c,pts,col)local q={};for _,v in ipairs(pts)do q[#q+1]=p(c,v[1],v[2])end;c:poly(q,col)end
local function line(c,x,y,xx,yy,col,w)c:line(p(c,x,y),p(c,xx,yy),col,w)end
local function rect(c,x,y,w,h,col)poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},col)end
local function ellipse(c,x,y,rx,ry,col)c:ellipse(c.cx+x,c.cy+y,rx,ry,col)end
local function frame(c,x,y,w,h,col)
  line(c,x,y,x+w-1,y,col);line(c,x,y+h-1,x+w-1,y+h-1,col)
  line(c,x,y,x,y+h-1,col);line(c,x+w-1,y,x+w-1,y+h-1,col)
end
local function bolt(c,x,y)
  rect(c,x,y,3,3,'ui0');rect(c,x,y,2,1,'ui4');rect(c,x+1,y+2,2,1,'ui2')
end
out.hud_top=function(c)
  c:layer(3);rect(c,0,0,640,24,'ui0');rect(c,0,1,640,3,'ui2');rect(c,0,4,640,17,'ui1')
  rect(c,0,21,640,2,'ui0');line(c,0,23,639,23,'ui3')
  for _,x in ipairs({93,164,290,426,570})do line(c,x,5,x,19,'ui0');line(c,x+1,5,x+1,19,'ui2')end
  for _,x in ipairs({3,631})do bolt(c,x,7)end
  line(c,11,2,78,2,'steel3');line(c,600,2,626,2,'rust2')
end
out.hud_bottom=function(c)
  c:layer(3);rect(c,0,0,640,72,'ui0')
  rect(c,0,0,640,4,'ui2');line(c,0,0,639,0,'ui4');line(c,0,4,639,4,'ink')
  rect(c,0,67,640,5,'ui1');line(c,0,67,639,67,'ui3')
  for _,x in ipairs({4,130,403,631})do rect(c,x,6,5,58,'ui1');line(c,x,7,x,63,'ui3')end
  rect(c,137,7,58,58,'ui0');frame(c,136,6,60,60,'ui3')
  line(c,137,6,178,6,'steel3');line(c,139,65,194,65,'ink')
  frame(c,8,8,120,57,'ui0');line(c,9,8,126,8,'steel2')
  for _,pt in ipairs({{2,1},{634,1},{130,7},{130,59},{403,7},{403,59}})do bolt(c,pt[1],pt[2])end
  -- Grooved lower rail, with functional caution paint only at the panel ends.
  for x=14,73,6 do line(c,x,69,x+2,69,'ui2')end
  for x=580,619,8 do line(c,x,69,x+3,69,'rust2')end
  rect(c,200,7,196,43,'ui1');line(c,200,51,395,51,'ui2')
  line(c,201,8,255,8,'ui2')
end
local function portrait(c,faction)
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
out.portrait_hook=function(c)
  portrait(c,'union')
  poly(c,{{6,35},{10,15},{18,7},{23,11},{16,28},{13,37}},'rust0')
  poly(c,{{9,31},{12,17},{19,10},{21,12},{14,30}},'rust2')
  poly(c,{{17,8},{44,2},{49,6},{45,10},{21,14}},'ivory1')
  line(c,21,9,43,4,'ivory4',2);line(c,20,14,44,9,'rust2',2)
  line(c,46,9,46,22,'ink');line(c,46,22,49,24,'rust3',2);line(c,49,24,51,21,'rust3')
  ellipse(c,16,20,4,4,'steel0');ellipse(c,15,19,2,2,'steel3')
  cab(c,15,22)
  poly(c,{{3,45},{22,43},{50,52},{50,55},{4,55}},'steel0')
  for x=5,41,7 do line(c,x,49,x+3,52,'steel2',2)end
  line(c,5,46,20,45,'rust3')
end
out.portrait_riveter=function(c)
  portrait(c,'union');cab(c,23,12)
  poly(c,{{4,13},{19,8},{26,13},{24,27},{6,30}},'rust0')
  poly(c,{{6,14},{19,11},{24,14},{21,25},{8,27}},'rust2')
  for _,v in ipairs({{10,15},{16,14},{19,18},{12,21}})do ellipse(c,v[1],v[2],3,2,'steel2')end
  ellipse(c,18,40,15,15,'ink');ellipse(c,17,39,13,14,'steel1');ellipse(c,16,38,10,11,'rust1')
  for _,v in ipairs({{16,29},{24,35},{22,46},{11,46},{7,37}})do ellipse(c,v[1],v[2],2,2,'steel0')end
  line(c,9,31,16,27,'rust3',2);ellipse(c,16,38,4,4,'steel0');ellipse(c,15,37,2,2,'steel3')
  poly(c,{{34,31},{54,33},{54,43},{36,40}},'steel0')
  poly(c,{{35,31},{53,33},{51,36},{36,34}},'steel3')
  rect(c,48,34,5,9,'rust2');rect(c,51,36,3,6,'ink')
end
out.portrait_bulwark=function(c)
  portrait(c,'union');cab(c,18,5)
  poly(c,{{1,26},{23,20},{26,54},{1,54}},'ivory1')
  poly(c,{{3,27},{21,23},{22,43},{4,46}},'ivory3')
  poly(c,{{29,22},{52,29},{53,55},{28,55}},'ivory0')
  poly(c,{{31,25},{50,31},{50,48},{30,43}},'ivory2')
  poly(c,{{2,45},{23,41},{23,55},{2,55}},'rust1')
  poly(c,{{30,43},{52,48},{52,55},{29,55}},'rust2')
  for _,v in ipairs({{6,29},{20,26},{32,28},{48,33},{6,49},{34,49}})do ellipse(c,v[1],v[2],2,2,'steel0');rect(c,v[1]-1,v[2]-1,1,1,'steel3')end
  line(c,12,34,12,41,'ivory1',2);line(c,9,36,16,34,'ivory1')
  line(c,36,36,43,39,'ivory0');line(c,8,47,13,46,'rust3')
end
out.portrait_sounder=function(c)
  portrait(c,'union');cab(c,24,23)
  rect(c,12,13,5,37,'steel0');rect(c,13,13,2,34,'steel3')
  poly(c,{{10,9},{19,4},{26,11},{26,24},{20,32},{12,28},{8,18}},'ivory0')
  poly(c,{{12,10},{19,6},{24,12},{24,23},{19,29},{12,25},{10,18}},'ivory2')
  poly(c,{{13,11},{18,7},{20,13},{15,24},{11,20}},'ivory4')
  line(c,18,18,29,21,'steel0',2);ellipse(c,29,21,2,2,'rust3')
  rect(c,8,39,11,14,'rust1');rect(c,10,40,6,10,'steel0');rect(c,11,42,3,2,'amber3')
  line(c,38,9,38,25,'steel3');rect(c,37,6,3,3,'rust3')
end
out.portrait_wick=function(c)
  portrait(c,'assembly');cab(c,24,9)
  rect(c,7,9,9,22,'amber1');rect(c,8,10,4,20,'amber3');rect(c,6,8,12,4,'steel0');rect(c,6,27,12,4,'steel0')
  poly(c,{{3,30},{9,28},{20,35},{39,28},{48,34},{40,48},{23,54},{8,47}},'jade0')
  poly(c,{{5,30},{11,31},{17,40},{25,44},{36,41},{44,34},{46,38},{38,49},{22,54},{9,46}},'jade3')
  poly(c,{{11,30},{24,35},{39,29},{38,38},{23,43},{13,38}},'reed0')
  for y=33,39,3 do line(c,15,y,25,y+4,'reed3');line(c,25,y+4,36,y-2,'reed2')end
  line(c,9,45,21,51,'jade4')
  line(c,40,27,51,18,'steel2',3);line(c,48,20,52,20,'reed3',2)
end
out.portrait_skipper=function(c)
  portrait(c,'assembly')
  poly(c,{{2,36},{9,22},{25,14},{42,20},{54,34},{47,48},{27,55},{8,47}},'jade0')
  poly(c,{{3,35},{11,23},{26,17},{42,22},{52,34},{27,45}},'jade3')
  poly(c,{{4,37},{27,47},{50,36},{45,48},{27,54},{8,46}},'jade1')
  for _,v in ipairs({{8,34},{15,26},{39,26},{47,34}})do line(c,27,43,v[1],v[2],'jade1');line(c,28,42,v[1]+1,v[2],'jade2')end
  poly(c,{{20,16},{30,13},{38,17},{37,28},{29,33},{19,28}},'ivory1')
  poly(c,{{20,16},{30,14},{37,17},{28,22}},'ivory4')
  poly(c,{{22,22},{28,25},{34,22},{34,27},{28,30},{22,27}},'glass0')
  line(c,23,23,28,26,'glass3')
  ellipse(c,13,37,3,2,'amber3');line(c,8,42,18,47,'jade4')
end
out.portrait_reedguard=function(c)
  portrait(c,'assembly');cab(c,19,7)
  poly(c,{{2,28},{11,24},{18,28},{17,46},{7,51},{1,43}},'jade2')
  poly(c,{{19,24},{43,29},{52,39},{45,54},{21,54},{16,45}},'reed0')
  poly(c,{{21,26},{42,32},{48,39},{43,51},{23,52},{19,44}},'reed2')
  for y=31,47,5 do line(c,22,y,44,y+4,'reed0',2);line(c,23,y-1,42,y+3,'reed4')end
  line(c,26,30,27,51,'reed1',2);line(c,37,32,35,51,'reed1',2)
  poly(c,{{17,24},{22,21},{43,27},{45,32},{39,33},{20,28}},'jade3')
  rect(c,7,28,5,14,'amber2');line(c,8,29,8,38,'amber4')
end
out.portrait_loom=function(c)
  portrait(c,'assembly')
  poly(c,{{4,52},{5,30},{14,12},{27,3},{42,14},{51,32},{53,54},{45,54},{43,33},{35,17},{27,12},{18,20},{12,36},{12,54}},'jade0')
  poly(c,{{5,52},{7,29},{16,12},{27,4},{32,7},{20,18},{13,35},{12,52}},'jade3')
  poly(c,{{31,8},{42,16},{49,32},{51,52},{46,51},{41,31},{34,19},{27,13}},'jade2')
  line(c,9,29,16,15,'jade4',2);line(c,17,13,27,6,'jade4',2)
  rect(c,24,14,4,11,'steel2')
  poly(c,{{19,27},{25,23},{35,27},{36,45},{30,51},{19,46}},'amber0')
  poly(c,{{20,27},{26,25},{32,28},{32,46},{26,49},{20,45}},'amber2')
  rect(c,21,28,3,15,'amber4');rect(c,18,26,18,4,'steel1');rect(c,18,43,18,4,'steel0')
  line(c,12,31,17,33,'reed3',2);line(c,38,26,44,25,'reed2',2)
end
out.portrait_union=function(c)
  portrait(c,'union')
  ellipse(c,28,28,21,21,'steel0');ellipse(c,28,27,18,18,'ui2')
  for _,x in ipairs({16,25,34})do poly(c,{{x,17},{x+6,17},{x+6,34},{x+3,39},{x,34}},'rust2');line(c,x,17,x+5,17,'rust4')end
  line(c,14,43,41,43,'steel3');line(c,22,11,34,11,'ivory3')
end
out.portrait_assembly=function(c)
  portrait(c,'assembly')
  poly(c,{{8,43},{13,25},{28,9},{43,25},{48,43},{39,43},{35,27},{28,20},{21,27},{17,43}},'jade2')
  line(c,13,26,27,12,'jade4',2);line(c,30,15,42,29,'jade3',2)
  ellipse(c,28,35,6,8,'amber1');ellipse(c,26,33,3,5,'amber3')
  line(c,18,47,37,47,'reed3')
end
out.icon_salvage=function(c)
  c:layer(3);poly(c,{{1,7},{7,2},{14,5},{14,10},{8,15},{1,11}},'rust0')
  poly(c,{{1,7},{7,3},{14,5},{8,10}},'rust3');poly(c,{{2,8},{7,11},{7,14},{2,11}},'rust2')
  line(c,4,6,10,9,'steel3');line(c,9,5,4,9,'steel1')
end
out.icon_pressure=function(c)
  c:layer(3);rect(c,4,2,8,12,'steel0');rect(c,5,3,6,10,'glass1');rect(c,6,4,2,7,'glass3')
  rect(c,3,1,10,3,'steel2');rect(c,3,12,10,3,'steel2');line(c,4,1,10,1,'steel4')
end
out.icon_crew=function(c)
  c:layer(3)
  ellipse(c,4,6,3,4,'steel1');ellipse(c,11,6,3,4,'steel1')
  ellipse(c,8,7,4,5,'ivory2');rect(c,5,6,6,3,'glass0');line(c,6,6,9,6,'glass2')
  poly(c,{{2,15},{3,12},{8,10},{13,12},{15,15}},'steel2');line(c,4,12,8,11,'ivory3')
end
return out
