-- Root-authored coastal machinery, dismantling states, and public route cues.
local root=assert(app.params.root)
local h=dofile(root..'/tools/art_v8/common.lua')
local font_file=assert(io.open(root..'/tools/art_v8/font7.json'))
local font=json.decode(font_file:read('*a'));font_file:close()
local M={}
local function letters(c,text,x,y,color)
  for i=1,#text do
    local rows=font[text:sub(i,i)] or font['?']
    for yy,row in ipairs(rows) do for xx=1,7 do
      if row:sub(xx,xx)=='1' then c:pixel(c.cx+x+(i-1)*8+xx-1,c.cy+y+yy-1,color) end
    end end
  end
end

function M.gate(c,north_dry,warning,phase)
  c:layer(1)
  h.poly(c,{{-59,-8},{-6,-35},{59,-3},{5,14},{-28,7}},'cast')
  c:layer(3)
  h.block(c,0,7,53,53,5,'earth')
  h.block(c,-24,-6,20,16,50,'earth')
  h.block(c,37,-7,18,17,50,'earth')
  -- The paired piers support a continuous bridge and a heavy winch deck.
  h.poly(c,{{-49,-59},{-2,-83},{52,-58},{7,-34}},'earth4')
  h.poly(c,{{-49,-59},{7,-34},{7,-28},{-49,-53}},'earth2')
  h.poly(c,{{7,-34},{52,-58},{52,-52},{7,-28}},'earth1')
  h.line(c,-46,-58,-2,-80,'ivory2')
  h.line(c,8,-35,49,-55,'earth0')
  -- Quiet old watermarks on the concrete distinguish age from new equipment.
  for _,x in ipairs({-42,30}) do
    h.line(c,x,-20,x+10,-15,'wet0')
    h.line(c,x,-18,x+7,-15,'moss1')
    h.line(c,x,-43,x+8,-39,'ivory1')
  end
  c:layer(2)
  h.line(c,-24,-65,-24,-92,'steel0',3)
  h.line(c,24,-65,24,-92,'steel0',3)
  h.line(c,-23,-91,24,-91,'steel2',3)
  h.line(c,-22,-91,22,-91,'steel4')
  c:layer(4)
  for _,x in ipairs({-23,23}) do
    h.rect(c,x-10,-77,21,48,'steel0')
    h.rect(c,x-8,-75,17,44,'steel1')
    h.rect(c,x-5,-71,11,36,'glass0')
    h.rect(c,x-4,-70,2,33,'glass2')
    h.line(c,x-8,-75,x+7,-75,'steel3')
    for y=-66,-39,6 do h.line(c,x+6,y,x+8,y,'ivory2') end
    h.rect(c,x-9,-78,19,3,'ivory2')
  end
  c:layer(5)
  for i,x in ipairs({-23,23}) do
    local dry=i==1 and north_dry or (i==2 and not north_dry)
    local water=dry and -43 or -63
    h.rect(c,x-2,water,6,-34-water,'sea2')
    h.line(c,x-2,water,x+3,water,'sea4')
    h.rect(c,x-6,water-2,13,4,'amber1')
    h.rect(c,x-5,water-2,11,2,'amber3')
    h.line(c,x,water-3,x,-90,'reed1')
    h.oval(c,x,-89,4,3,'steel0');h.oval(c,x,-89,2,2,'rust2')
    letters(c,i==1 and 'N' or 'S',x-3,-104,'ivory3')
    -- Separate external target chevrons never move the current-state float.
    if warning then
      local target=dry and -63 or -43
      local dy=phase%2
      h.poly(c,{{x+14,target-4},{x+10,target},{x+14,target+4}},'amber3')
      h.line(c,x+16,target-6+dy,x+16,target+6-dy,'amber2')
    end
  end
  h.line(c,-22,-88,22,-88,'reed2')
  h.oval(c,0,-18,13,11,'steel0');h.oval(c,0,-19,10,8,'rust1')
  h.oval(c,0,-19,7,5,'steel1')
  local a=warning and phase*math.pi/4 or (north_dry and 0 or math.pi/2)
  for i=0,3 do
    local t=a+i*math.pi/2
    h.line(c,0,-19,math.cos(t)*10,-19+math.sin(t)*8,'rust3',2)
  end
  -- An asymmetric crank handle prevents a four-spoke wheel from repeating
  -- every second key. Each warning phase has a distinct mechanical pose.
  h.line(c,0,-19,math.cos(a)*12,-19+math.sin(a)*9,'ivory2',2)
  h.oval(c,0,-19,3,2,'steel3')
  h.line(c,0,-84,0,-109,'steel2',2)
  local side=north_dry and -1 or 1
  h.poly(c,{{0,-108},{side*19,-114},{side*21,-108},{0,-104}},'rust0')
  h.line(c,side*3,-108,side*17,-111,'ivory3',2)
  h.oval(c,0,-106,3,3,'steel0');h.oval(c,0,-107,1,1,'steel3')
  c:layer(6)
  for _,x in ipairs({-46,43}) do
    h.poly(c,{{x,-4},{x+6,-7},{x+11,-4},{x+5,0}},'steel0')
    h.line(c,x+1,-4,x+6,-6,'steel2')
  end
  h.line(c,-16,-6,-5,0,'steel0');h.line(c,5,0,16,-6,'steel0')
end

function M.lane(c,wet,phase)
  c:layer(3)
  -- Rasterized diamond excludes the duplicate bottom edge on 32×16 tiles.
  for y=-8,7 do
    local span=16-2*math.abs(y)
    if span>0 then h.line(c,-span,y,span-1,y,wet and 'sea2' or 'earth2') end
  end
  c:layer(6)
  local base=wet and 'sea1' or 'earth0'
  -- Buried parallel iron crossbars remain legible beneath shallow water.
  h.line(c,-9,0,0,-4,base)
  h.line(c,-5,3,4,-1,base)
  h.line(c,0,-4,6,-1,wet and 'sea3' or 'earth3')
  if wet then
    local p=phase%4
    h.line(c,-10+p,-1,-6+p,-3,'sea3')
    h.line(c,-5+p,-3,-1+p,-3,'sea3')
    h.line(c,4-p,4,8-p,2,'sea4')
  elseif phase%2==0 then
    h.line(c,-3,5,1,3,'wet1')
    h.line(c,3,-3,8,-1,'earth1')
  end
end

function M.sluice(c,phase)
  c:layer(5)
  for _,side in ipairs({-1,1}) do
    local x=side*(8+phase*5);local y=phase*2-7
    local w=phase<3 and (4+phase*2) or 8-phase
    h.line(c,x-w,y,x,y-w/2,'sea3',2)
    h.line(c,x,y-w/2,x+w,y,'sea4')
    if phase<4 then
      h.line(c,x-w+2,y+2,x-2,y-w/2+3,'foam')
      h.line(c,x+2,y-w/2+3,x+w-1,y+1,'sea3')
    else h.line(c,x-2,y+2,x+1,y+1,'sea2') end
  end
end

function M.salvage(c,name,stage)
  if stage==0 then return end -- native original was copied before this call
  if stage==3 then
    for i=1,6 do h.clear(c,i) end
    c:layer(1)
    h.poly(c,{{-32,-2},{-17,-12},{13,-10},{32,0},{20,7},{-13,6}},'salt0')
    c:layer(3)
    h.line(c,-25,-1,-16,-5,'earth1');h.line(c,5,6,16,3,'earth1')
    h.line(c,-10,-8,-5,-9,'wet1');h.line(c,22,-1,25,0,'earth0')
    h.poly(c,{{-9,0},{-4,-3},{0,-1},{-5,1}},'ivory1')
    h.line(c,9,-2,13,-4,'steel1')
    return
  end
  if name=='salvage_turbine' and stage==1 then
    h.clear(c,3);h.clear(c,5);h.clear(c,6)
    c:layer(2)
    -- Removing the rear casing reveals the impeller's mounting cradle.
    for _,x in ipairs({-30,-16}) do
      h.line(c,x,-25,x+12,-34,'steel0',3)
      h.line(c,x+12,-34,x+15,-7,'steel2',3)
      h.line(c,x,-25,x+1,-2,'steel1',3)
    end
    c:layer(3);h.block(c,-13,2,26,13,6,'steel')
    c:layer(5);h.poly(c,{{-26,0},{-18,-5},{-7,0},{-16,5}},'rust2')
  else
    h.clear(c,4);h.clear(c,5);h.clear(c,6)
    if stage==2 then h.clear(c,3) end
    if name=='salvage_turbine' then
      c:layer(3)
      h.line(c,-33,-7,0,10,'steel0',3);h.line(c,0,10,35,-7,'steel0',3)
      h.line(c,-31,-8,0,7,'steel1');h.line(c,0,7,32,-8,'steel2')
      c:layer(4)
      h.oval(c,10,-16,17,14,'steel1');h.oval(c,10,-16,14,11,'ink')
      -- A hollow ring is structural negative space, not a dark filled disk.
      for y=-24,-8 do for x=-1,21 do
        local dx,dy=x-10,y+16
        if dx*dx/144+dy*dy/81<1 then c.image:drawPixel(c.cx+x,c.cy+y,0) end
      end end
      for _,p in ipairs({{-4,-22},{21,-25},{22,-6},{-1,-7}}) do h.line(c,10,-16,p[1],p[2],'steel2',2) end
      h.oval(c,10,-16,4,3,'steel0');h.line(c,8,-18,11,-18,'steel3')
    else
      c:layer(3)
      if stage==2 then
        if name=='salvage' then
          h.poly(c,{{-43,-11},{-27,-22},{20,-17},{42,-5},{30,5},{5,8},{-25,0}},'rust0')
          h.poly(c,{{-37,-11},{-25,-18},{19,-13},{35,-4},{26,1},{5,4},{-23,-4}},'earth1')
          h.line(c,-39,-11,-26,-20,'steel1');h.line(c,-24,-20,18,-15,'steel2')
          h.line(c,19,-15,38,-5,'steel1');h.line(c,8,6,29,3,'rust1')
        else
          h.line(c,-37,-13,2,8,'steel0',3);h.line(c,2,8,38,-8,'steel0',3)
          h.line(c,-36,-14,2,5,'steel1');h.line(c,2,5,37,-10,'steel2')
        end
      end
      c:layer(4)
      local height=stage==1 and 26 or 15
      for i=0,3 do
        local x=-27+i*14;local y=-12+i*2
        h.line(c,x,y,x,y-height,'steel0',3)
        h.line(c,x+1,y-1,x+1,y-height,'rust1')
        h.line(c,x,y-height,x+13,y-height-6,'steel2',2)
        if stage==1 then h.line(c,x+13,y-height-6,x+18,y-6,'steel1',2) end
        if stage==2 then
          h.line(c,x+13,y-height-6,x+17,y-5,'steel1',2)
          h.line(c,x,y,x+17,y-5,'rust1')
        end
      end
      if stage==1 then
        h.poly(c,{{-26,-13},{-8,-5},{-8,-16},{-18,-24},{-26,-25}},'rust1')
        h.line(c,-24,-22,-10,-15,'rust3')
        h.poly(c,{{15,-7},{30,-15},{37,-10},{24,-2}},'steel2')
      end
    end
    c:layer(5)
    h.line(c,-22,-4,-11,2,'rust1',2);h.line(c,18,3,27,-2,'rust2')
  end
end

function M.landmark(c,name)
  c:layer(1)
  h.poly(c,{{-37,-4},{-3,-19},{37,-2},{9,9},{-20,5}},'salt0')
  if name=='tide_gauge' then
    c:layer(3)
    h.block(c,3,3,20,20,9,'earth')
    h.block(c,2,-4,12,10,70,'earth')
    h.block(c,4,-71,15,13,5,'earth')
    c:layer(4)
    h.poly(c,{{-9,-11},{-1,-7},{-1,-69},{-9,-73}},'ivory1')
    h.line(c,-8,-69,-8,-13,'ivory2')
    for y=-63,-16,6 do h.line(c,-7,y,-3,y+2,'earth0') end
    -- The obsolete high-water stripe is far above the present dry footing.
    h.line(c,-10,-52,0,-47,'wet0',3)
    h.line(c,0,-47,10,-52,'wet0',3)
    h.line(c,-9,-53,0,-48,'salt3')
    c:layer(5)
    h.line(c,8,-40,14,-43,'steel1',2);h.line(c,14,-43,14,-27,'steel1',2)
    h.oval(c,14,-25,5,4,'steel1');h.oval(c,14,-25,3,2,'earth0')
    c:layer(6)
    h.line(c,-13,-3,-21,-1,'moss0',2);h.line(c,-15,-5,-18,-12,'moss1')
  elseif name=='ferry_stairs' then
    c:layer(3)
    for i=0,5 do
      h.block(c,-17+i*7,3-i*3,20,7,3+i*4,'earth')
      h.line(c,-35+i*7,-10-i*7,-22+i*7,-16-i*7,'ivory1')
    end
    c:layer(5)
    h.line(c,-35,-13,0,-56,'steel1',2)
    h.line(c,-34,-13,-34,-1,'steel1',2)
    h.line(c,-20,-32,-20,-13,'steel1',2)
    h.line(c,-6,-49,-6,-25,'steel1',2)
    h.line(c,0,-56,5,-58,'steel2')
    h.oval(c,21,-35,5,3,'steel0');h.oval(c,21,-36,3,2,'steel2')
    c:layer(6)
    h.line(c,-23,-7,-19,-6,'moss0',2);h.line(c,4,-25,11,-29,'wet0')
  else
    c:layer(3)
    h.poly(c,{{-46,-7},{-30,-20},{25,-16},{47,-2},{23,8},{-19,5}},'earth1')
    h.line(c,-42,-8,-20,4,'steel0',3);h.line(c,-20,4,31,-2,'steel0',3)
    for i=0,4 do
      local x=-34+i*15;local y=-6-i%2*2;local z=16+(i==2 and 8 or 0)
      c:layer(4)
      h.line(c,x,y,x-5,y-z,'steel0',3)
      h.line(c,x-5,y-z,x+4,y-z-7,'steel1',3)
      h.line(c,x+4,y-z-7,x+11,y-z+1,'steel2',2)
      h.line(c,x+11,y-z+1,x+12,y-4,'steel1',3)
      h.line(c,x-3,y-z,x+3,y-z-5,'ivory1')
    end
    c:layer(6)
    h.line(c,-20,4,-5,1,'salt2',2);h.line(c,7,4,20,1,'salt3')
    h.line(c,-38,-2,-30,0,'moss0');h.line(c,25,-8,29,-14,'moss1')
  end
end
return M
