-- Root's material-specific contact and breakdown drawings. Individual fragments
-- keep causal trajectories; large readable lobes precede small remnants.
local h=dofile(assert(app.params.root)..'/tools/art_v8/common.lua')
local M={}
function M.impact(c,material,p)
  c:layer(5)
  local reed=material=='reed'
  local main=reed and 'reed3' or 'amber3'
  local edge=reed and 'reed0' or 'steel0'
  if p==0 then
    h.poly(c,{{-6,-13},{-2,-13},{0,-19},{3,-12},{8,-13},{4,-8},{6,-4},{0,-7},{-5,-3},{-4,-9}},edge)
    h.poly(c,{{-3,-12},{0,-16},{2,-11},{5,-11},{2,-8},{0,-9},{-3,-6},{-2,-10}},main)
    h.line(c,-1,-12,1,-10,reed and 'ivory3' or 'ivory4',2)
  elseif p==1 then
    for _,a in ipairs({{-10,-14,-6,-11},{6,-19,3,-14},{11,-7,6,-9},{-7,-1,-4,-5}}) do
      h.line(c,a[1],a[2],a[3],a[4],edge,2)
      h.line(c,a[1],a[2]-1,a[3],a[4]-1,main)
    end
    if reed then h.oval(c,0,-11,4,3,'steel2');h.oval(c,-1,-12,2,2,'steel3') end
  else
    local d=p-2
    for _,a in ipairs({{-11-d,-10+d*3,-8-d,-12+d*3},{9+d,-15+d*3,11+d,-12+d*3},{5+d,0+d*2,8+d,-1+d*2}}) do
      h.line(c,a[1],a[2],a[3],a[4],reed and 'reed2' or (p==2 and 'rust3' or 'steel2'))
    end
    if reed and p==2 then h.line(c,-2,-14,2,-14,'steel2') end
  end
end

function M.wreck(c,material,p)
  c:layer(5)
  local reed=material=='reed'
  if p<2 then
    local r=4+p*4
    h.poly(c,{{-r,-9},{-r+2,-18},{-3,-15},{1,-25},{5,-16},{r,-18},{r+2,-9},{4,-4},{-2,-1}},reed and 'reed0' or 'rust0')
    h.poly(c,{{-r+2,-9},{-3,-16},{0,-18-p*3},{3,-13},{r,-10},{3,-5},{-2,-5}},reed and 'amber2' or 'amber3')
    h.oval(c,-1,-11,3+p,3,reed and 'steel4' or 'ivory4')
  end
  if p>=1 then
    local age=p-1
    -- Two smoke lobes separate and cool; they do not share one expanding disc.
    if p<5 then
      h.oval(c,-3-age,-17-age*3,6,4,'steel0')
      h.oval(c,-5-age,-20-age*3,4,4,'steel1')
      h.oval(c,5+age,-13-age*2,4,3,'steel1')
      h.line(c,-7-age,-23-age*3,-3-age,-23-age*3,'steel2')
    else
      h.line(c,-12,-33,-7,-35,'steel1');h.line(c,10,-26,14,-27,'steel1')
    end
    local x=7+age*4;local y=-8-age*3+age*age
    if reed then
      h.line(c,-x,y,-x+5,y-5,'reed0',2);h.line(c,-x+1,y,-x+5,y-4,'reed2')
      h.line(c,x-4,y-3,x,y+2,'jade1',3);h.line(c,x-3,y-4,x,y,'jade3')
    else
      h.poly(c,{{-x-3,y},{-x+1,y-4},{-x+5,y-1},{-x+2,y+3}},'steel0')
      h.line(c,-x-2,y,-x+1,y-3,'steel3')
      h.oval(c,x,y+1,3,3,'steel0');h.oval(c,x,y+1,1,1,'rust2')
    end
  end
end

function M.residue(c,faction)
  c:layer(1)
  h.poly(c,{{-18,-1},{-6,-7},{12,-4},{20,2},{7,7},{-11,5}},'salt0')
  c:layer(3)
  if faction=='union' then
    h.poly(c,{{-13,-2},{-3,-8},{7,-2},{0,3},{-10,2}},'steel1')
    h.line(c,-12,-3,-4,-7,'steel2')
    h.line(c,-4,-6,4,-2,'rust1')
    h.oval(c,12,0,5,3,'steel0');h.oval(c,12,-1,3,2,'steel1')
    h.line(c,10,-3,13,-3,'ivory1')
  else
    h.line(c,-15,0,-9,-9,'jade0',3);h.line(c,-9,-9,1,-4,'jade1',3)
    h.line(c,1,-4,14,1,'jade0',3);h.line(c,-13,-1,-9,-7,'jade2')
    h.line(c,-6,1,4,5,'reed1',2);h.line(c,3,2,12,-2,'reed2')
    h.line(c,6,-1,9,-4,'steel1')
  end
end
return M
