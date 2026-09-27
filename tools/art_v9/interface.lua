-- Root-authored instrument faces; existing bitmap text remains engine rendered.
local root=assert(app.params.root)
local h=dofile(root..'/tools/art_v8/common.lua')
local M={}
function M.telegraph(c,state)
 c:layer(3);h.rect(c,0,0,48,24,'ui1')
 h.rect(c,1,1,46,22,'ui2');h.rect(c,3,3,42,18,'ui1')
 h.oval(c,13,12,9,9,'reed1');h.oval(c,13,12,7,7,'ivory2')
 c:layer(5)
 for _,p in ipairs({{-5,-3},{0,-6},{5,-3},{5,3}})do h.rect(c,13+p[1],12+p[2],1,2,'steel1')end
 local x,y=state=='pending' and 13 or state=='accepted' and 18 or state=='rejected' and 9 or 8,
  state=='pending' and 6 or state=='accepted' and 9 or state=='rejected' and 16 or 10
 h.line(c,13,12,x,y,'rust1',2);h.oval(c,13,12,2,2,'steel1')
 local color=state=='pending' and 'amber3' or state=='accepted' and 'jade3' or state=='rejected' and 'rust3' or 'steel2'
 h.rect(c,28,7,12,9,'steel0');h.rect(c,30,9,8,5,color)
 h.line(c,29,19,41,19,'reed1')
 c:layer(6);h.rect(c,2,2,1,1,'steel3');h.rect(c,45,21,1,1,'steel3')
end
function M.doctrine(c,faction,doctrine)
 local steel=faction=='union'
 c:layer(3);h.rect(c,0,0,32,24,'ui1');h.rect(c,1,1,30,22,steel and 'reed1' or 'jade1');h.rect(c,3,3,26,18,'ui1')
 c:layer(5)
 if doctrine=='hauling' then
  h.rect(c,7,10,18,8,steel and 'steel2' or 'reed2');h.line(c,7,8,7,18,'ivory2');h.line(c,24,8,24,18,'ivory2')
  for x=11,22,5 do h.line(c,x,11,x,17,'ui1')end
  h.line(c,10,6,21,6,'amber3',2)
 else
  h.line(c,16,6,16,19,'steel3',2);h.line(c,16,8,8,12,'reed2',2);h.line(c,16,8,24,12,'reed3',2)
  h.oval(c,8,13,3,3,'rust3');h.oval(c,24,13,3,3,'amber3')
 end
 c:layer(6);h.rect(c,2,2,1,1,'steel3');h.rect(c,29,21,1,1,'steel3')
end
function M.signal(c,p)
 c:layer(1);h.oval(c,0,0,11,3,'cast')
 c:layer(3);h.rect(c,-3,-26,6,26,'steel1');h.rect(c,-2,-25,2,23,'steel3')
 h.poly(c,{{-11,-29},{-6,-37},{6,-37},{11,-29},{8,-17},{-8,-17}},'steel0')
 h.rect(c,-7,-30,14,11,'amber1');h.rect(c,-5,-29,10,9,p==1 and 'amber4' or 'amber3')
 h.line(c,0,-32,0,-17,'steel1',2);h.line(c,-8,-24,8,-24,'steel1')
 c:layer(6);h.line(c,-6,-35,4,-35,'steel3');h.line(c,-9,-18,7,-18,'reed2')
 h.rect(c,-9,-3,18,3,'steel1')
end
return M
