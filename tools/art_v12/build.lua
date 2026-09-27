-- Root-authored coastal material study, built in Aseprite on the native grid.
-- Reconstructs v12 sources only. Run export_sources.lua after source edits.
local root=assert(app.params.root)
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s)local f=assert(io.open(p,'w'));f:write(s);f:close()end
local m=json.decode(read(root..'/art/archive/art-v11/game-assets.json'))
local palette=dofile(root..'/tools/art_v5/painter.lua').palette
-- Shared warm ground, cooler shadow and lichen. No added source colors.
local rgba={};for k,v in pairs(palette)do rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255)end
local entries={}
local function document(name,keys,layers,draw)
 local b=assert(m.sprites[keys[1]]);local s=Sprite(b.w,b.h,ColorMode.RGB)
 local roles={};for k in pairs(palette)do roles[#roles+1]=k end;table.sort(roles)
 local pal=Palette(#roles+1);pal:setColor(0,0);for i,k in ipairs(roles)do pal:setColor(i,rgba[k])end;s:setPalette(pal)
 for i,n in ipairs(layers)do local l=i==1 and s.layers[1]or s:newLayer();l.name=n end
 for phase,key in ipairs(keys)do
  local b=assert(m.sprites[key]);assert(b.w==s.width and b.h==s.height)
  if phase>1 then s:newEmptyFrame(phase)end;s.frames[phase].duration=1
  local ims={};for i=1,#layers do ims[i]=Image(s.width,s.height,ColorMode.RGB)end
  local c={ims=ims,im=ims[1],w=b.w,h=b.h,ox=b.anchor_x,oy=b.anchor_y}
  function c:layer(i)self.im=self.ims[i]end
  function c:pixel(x,y,col)
   x,y=math.floor(x+self.ox+.5),math.floor(y+self.oy+.5)
   assert(x>=0 and y>=0 and x<self.w and y<self.h,key..' out of bounds '..x..','..y)
   self.im:drawPixel(x,y,assert(rgba[col],col))
  end
  function c:line(x,y,xx,yy,col)
   x,y,xx,yy=math.floor(x+.5),math.floor(y+.5),math.floor(xx+.5),math.floor(yy+.5)
   local dx,dy=math.abs(xx-x),-math.abs(yy-y);local sx,sy=x<xx and 1 or -1,y<yy and 1 or -1;local err=dx+dy
   while true do self:pixel(x,y,col);if x==xx and y==yy then break end
    local e=2*err;if e>=dy then err=err+dy;x=x+sx end;if e<=dx then err=err+dx;y=y+sy end
   end
  end
  function c:path(p,col)for i=2,#p do self:line(p[i-1][1],p[i-1][2],p[i][1],p[i][2],col)end end
  function c:poly(p,col)
   local lo,hi=1e9,-1e9;for _,v in ipairs(p)do lo=math.min(lo,v[2]);hi=math.max(hi,v[2])end
   for y=math.ceil(lo),math.ceil(hi)-1 do local xs={};local j=#p
    for i,a in ipairs(p)do local b=p[j];if(a[2]<=y and b[2]>y)or(b[2]<=y and a[2]>y)then xs[#xs+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2])end;j=i end
    table.sort(xs);for i=1,#xs-1,2 do local l,r=math.ceil(xs[i]),math.ceil(xs[i+1])-1;if l<=r then self:line(l,y,r,y,col)end end
   end
  end
  draw(c,phase-1)
  for i,im in ipairs(ims)do s:newCel(s.layers[i],phase,im,Point(0,0))end
  entries[#entries+1]={key=key,document=name,frame=phase,w=b.w,h=b.h,anchor_x=b.anchor_x,anchor_y=b.anchor_y,duration_ms=1000,layers=layers}
  local frame=Image(b.w,b.h,ColorMode.RGB);frame:drawSprite(s,phase);frame:saveAs(root..'/output/art-v12/'..key..'.png')
 end
 s:newTag(1,#keys).name=name..'_variants'
 s:saveAs(root..'/art/source/v12/'..name..'.aseprite');s:close()
end
local function keys(prefix,n)local t={};for i=0,n-1 do t[#t+1]=prefix..i end;return t end
local groundlayers={'ground plane','mineral bodies','fracture and wear'}
for _,kind in ipairs({'salt','silt','causeway'})do
 document('ground_'..kind,keys('terrain_'..kind..'_',4),groundlayers,function(c,v)
  local base=kind=='salt'and'salt1'or(kind=='silt'and'earth1'or'earth2')
  for y=-8,8 do c:line(-16+2*math.abs(y),y,16-2*math.abs(y),y,base)end
  c:layer(2)
  if kind=='salt'then
   -- Quiet interiors: no repeated enclosed diamond chips.
   if v==1 then c:poly({{-11,0},{-5,-3},{1,-2},{-3,0},{-8,1}},'salt2');c:line(-4,3,2,2,'salt0')
   elseif v==2 then c:poly({{-5,-2},{1,-4},{6,-2},{3,0},{-2,0}},'salt2');c:line(2,3,8,1,'salt0')
   elseif v==3 then c:path({{-8,1},{-3,0},{2,1},{7,-1}},'salt2')end
  elseif kind=='silt'then
   if v==1 then c:poly({{-12,0},{-4,-3},{2,-3},{-4,0}},'salt0')
   elseif v==2 then c:path({{-7,-2},{0,-2},{5,0},{9,0}},'earth0');c:line(0,-3,4,-2,'salt0')
   elseif v==3 then c:poly({{-8,1},{-3,-1},{4,0},{0,2}},'earth0')end
  else
   -- Worn causeway slabs have quiet shared edges and offset inner breaks.
   c:poly({{-13,0},{-2,-5},{10,-1},{12,1},{2,5},{-3,4}},'salt2')
   c:layer(3);c:path({{-12,1},{-3,5},{2,5},{12,1}},'salt0')
   if v==1 then c:path({{-4,-3},{-1,0},{4,1}},'earth1');c:line(3,3,7,2,'salt1')
   elseif v==2 then c:poly({{-7,-1},{-1,-3},{3,-1},{1,1},{-4,1}},'salt1')
   elseif v==3 then c:line(-8,1,-2,3,'salt1')end
  end
 end)
end
-- Large transparent salt veneers: connected facets, quiet gaps and chipped
-- edges. No full enclosing rings. They remain flat walkable surface marks.
local shelves={
 {{-42,1},{-32,-5},{-24,-4},{-13,-11},{-1,-12},{13,-8},{24,-10},{41,-4},{32,0},{18,1},{5,7},{-7,6},{-18,9},{-30,5}},
 {{-39,3},{-29,-1},{-22,-6},{-11,-5},{-2,-11},{11,-9},{20,-5},{33,-4},{38,0},{26,5},{11,6},{0,10},{-9,7},{-20,8}},
 {{-38,0},{-27,-5},{-16,-4},{-4,-10},{9,-8},{17,-3},{34,-4},{41,0},{32,4},{18,3},{9,8},{-4,6},{-16,9},{-28,5}}
}
document('salt_shelves',keys('salt_crust_',3),{'silt recesses','broad mineral plates','sunlit chips','fracture seams'},function(c,v)
 c:layer(2);c:poly(shelves[v+1],'salt2')
 c:layer(3)
 if v==0 then
  c:poly({{-38,0},{-29,-4},{-23,-3},{-14,-8},{-3,-10},{5,-8},{-2,-4},{-13,-3},{-21,2},{-30,3}},'salt3')
  c:poly({{10,-4},{24,-8},{36,-4},{28,-2},{20,-2},{11,2},{3,3}},'salt3')
  c:poly({{-16,5},{-7,3},{1,4},{-5,6},{-13,7}},'salt1')
  c:layer(4);c:path({{-24,-3},{-19,0},{-10,0},{-6,2}},'salt0');c:path({{5,-8},{3,-4},{8,-2}},'salt1')
  c:line(19,0,28,-1,'salt0');c:line(-27,5,-20,6,'salt0')
 elseif v==1 then
  c:poly({{-34,2},{-27,0},{-21,-4},{-12,-3},{-4,-8},{5,-8},{0,-4},{-3,0},{-13,1},{-19,5}},'salt3')
  c:poly({{8,-5},{16,-3},{22,-3},{31,-1},{23,3},{13,3},{5,6},{-2,5}},'salt3')
  c:layer(4);c:path({{-22,-4},{-18,-1},{-11,0},{-8,3}},'salt0');c:path({{8,-6},{7,-1},{12,1}},'salt1')
  c:line(22,5,29,2,'salt0');c:line(-12,7,-5,6,'salt0')
 else
  c:poly({{-34,0},{-26,-3},{-17,-2},{-7,-6},{1,-8},{6,-6},{-2,-2},{-12,1},{-21,4}},'salt3')
  c:poly({{7,-2},{18,-1},{31,-2},{36,0},{29,2},{17,1},{8,4},{0,4}},'salt3')
  c:layer(4);c:path({{-20,-2},{-14,0},{-10,4}},'salt0');c:path({{4,-6},{3,-3},{8,-1}},'salt1')
  c:line(-22,6,-16,7,'salt0');c:line(13,3,20,2,'salt0')
 end
end)
-- Sedimentary outcrops: slanting caps, broad faces, interlocking broken blocks.
-- Same anchor and maximum width as the older rock, no new collider silhouette.
document('coastal_rock',{'salt_rock','salt_rock_1','salt_rock_2'},{'ground contact','weathered stone masses','lit fracture planes','strata and crevices','salt and lichen'},function(c,v)
 c:poly({{-25,2},{-16,-2},{10,-1},{26,3},{23,7},{10,9},{-14,7}},'cast')
 c:layer(2)
 if v==0 then
  c:poly({{-23,-1},{-22,-13},{-16,-19},{-14,-28},{-2,-34},{11,-31},{13,-24},{21,-18},{23,-7},{19,3},{6,7},{-11,5}},'earth0')
  c:poly({{-22,-5},{-21,-13},{-14,-18},{-13,-27},{-2,-31},{9,-29},{5,-23},{-3,-19},{-5,-12},{-14,-8}},'earth3')
  c:poly({{-21,-4},{-14,-8},{-5,-12},{-3,-19},{5,-23},{12,-25},{12,-16},{6,-12},{5,-5},{-3,0},{-13,2}},'earth2')
  c:poly({{6,-12},{13,-16},{20,-16},{21,-7},{17,1},{6,5},{-2,1},{5,-5}},'earth1')
  c:layer(3);c:poly({{-13,-27},{-2,-33},{9,-30},{5,-27},{-3,-25}},'earth4')
  c:poly({{-20,-13},{-15,-18},{-6,-17},{-7,-14},{-14,-10},{-21,-9}},'earth3')
  c:poly({{3,-8},{11,-12},{17,-10},{11,-6},{4,-4},{-3,-1},{-12,-2}},'salt2')
  c:layer(4);c:path({{-13,-20},{-5,-22},{0,-25}},'earth0');c:path({{-18,-7},{-11,-9},{-7,-8}},'earth1')
  c:path({{10,-23},{8,-18},{10,-17}},'earth0');c:path({{0,-15},{-1,-11},{-5,-9}},'earth1')
  c:path({{9,-3},{15,-5},{20,-4}},'earth0');c:line(-13,2,-6,3,'earth1')
 elseif v==1 then
  c:poly({{-24,1},{-24,-8},{-18,-15},{-16,-23},{-9,-28},{-2,-35},{10,-37},{18,-31},{17,-22},{23,-14},{23,-3},{17,5},{1,8},{-12,4}},'earth0')
  c:poly({{-23,-3},{-22,-9},{-16,-15},{-14,-22},{-7,-26},{-1,-33},{9,-35},{12,-31},{8,-25},{1,-22},{-1,-15},{-8,-11},{-10,-5}},'earth3')
  c:poly({{-10,-5},{-8,-11},{-1,-15},{1,-22},{8,-25},{13,-30},{16,-30},{14,-21},{20,-14},{12,-10},{10,-3},{0,3},{-12,3}},'earth2')
  c:poly({{12,-10},{20,-14},{22,-11},{21,-3},{15,4},{1,6},{0,3},{10,-3}},'earth1')
  c:layer(3);c:poly({{-1,-33},{9,-36},{16,-31},{10,-31},{3,-28},{-4,-27}},'earth4')
  c:poly({{-21,-9},{-16,-14},{-7,-16},{-5,-13},{-12,-8},{-21,-6}},'salt2')
  c:poly({{-10,0},{0,-5},{7,-5},{4,-2},{-3,2}},'earth3')
  c:layer(4);c:path({{-11,-21},{-4,-23},{1,-26}},'earth0');c:path({{11,-25},{10,-20},{13,-18}},'earth0')
  c:path({{-7,-11},{-2,-12},{3,-15}},'earth1');c:path({{5,-1},{12,-5},{18,-4}},'earth0')
 else
  c:poly({{-26,1},{-23,-11},{-13,-19},{-3,-20},{2,-17},{11,-25},{18,-26},{23,-21},{25,-10},{26,2},{17,7},{-4,6}},'earth0')
  c:poly({{-24,-2},{-21,-10},{-13,-17},{-3,-18},{1,-15},{-5,-10},{-7,-3},{-14,1}},'earth3')
  c:poly({{-7,-3},{-5,-10},{1,-15},{11,-23},{18,-24},{20,-20},{13,-15},{10,-8},{4,-4},{3,3},{-4,4}},'earth2')
  c:poly({{10,-8},{13,-15},{21,-20},{23,-13},{23,-5},{25,1},{16,5},{3,4},{4,-4}},'earth1')
  c:layer(3);c:poly({{-21,-10},{-13,-18},{-4,-19},{-1,-16},{-10,-14},{-14,-10}},'earth4')
  c:poly({{4,-15},{11,-23},{18,-25},{21,-21},{14,-20},{10,-16}},'earth3')
  c:poly({{-17,-1},{-9,-5},{-3,-4},{-5,-1},{-11,1}},'salt2')
  c:layer(4);c:path({{-14,-12},{-8,-12},{-4,-14}},'earth1');c:path({{2,-13},{3,-9},{0,-6}},'earth0')
  c:path({{14,-14},{19,-12},{22,-13}},'earth0');c:path({{7,-1},{15,-4},{21,-3}},'earth0')
 end
 -- Broken chips and a restrained lichen patch tie the outcrop to the shore.
 c:layer(2);c:poly({{-27,3},{-24,-2},{-19,-2},{-15,3},{-18,7},{-24,6}},'earth1')
 c:layer(3);c:poly({{-26,2},{-23,-1},{-20,-1},{-18,2},{-21,3}},'earth3')
 c:layer(5);c:poly({{-16,-5},{-13,-7},{-8,-7},{-6,-5},{-10,-3},{-15,-3}},'wet1')
 c:line(-14,-6,-10,-7,'moss1');c:line(-22,5,-19,6,'salt2');c:line(14,5,19,3,'salt2')
end)
-- Foliage: broad leaf fans below sparse, asymmetrical seed heads. Grounded
-- by a root mat, with light entering from upper left instead of every stalk.
local function leaf(c,p,light)
 c:poly(p,'moss0')
 local a=p[1];local b=p[2];local tip=p[3]
 c:path({a,b,tip},light or'moss1')
end
document('shore_plants',{'reed_clump','marsh_grass','salt_bush'},{'root contact','back leaves','reed stems','front leaves','sunlit seed heads'},function(c,v)
 c:poly({{-21,1},{-13,-4},{-4,-3},{4,-6},{15,-2},{22,3},{13,7},{-5,7},{-18,4}},'cast')
 c:layer(2)
 c:poly({{-19,1},{-13,-6},{-7,-5},{-3,-10},{3,-8},{9,-10},{14,-4},{20,0},{17,4},{7,6},{-9,4}},'moss0')
 if v<2 then
  local fans={{{-8,2},{-14,-7},{-24,-11},{-19,-5}},{{-6,1},{-8,-12},{-16,-21},{-11,-8}},{{0,0},{1,-18},{-5,-27},{-4,-12}},{{4,1},{10,-12},{16,-18},{10,-5}},{{9,3},{17,-4},{24,-6},{17,1}}}
  for _,p in ipairs(fans)do leaf(c,p)end
  c:layer(3)
  local stems=v==0 and{{-10,0,-15,-28},{-4,0,-5,-37},{4,1,10,-31},{11,2,19,-23}}or{{-7,0,-10,-22},{2,1,5,-29},{10,2,16,-19}}
  for i,p in ipairs(stems)do
   c:path({{p[1],p[2]},{p[1]-1,p[2]-10},{p[3],p[4]}},'reed0')
   c:path({{p[1]-1,p[2]-2},{p[1]-2,p[2]-10},{p[3]-1,p[4]}},i==2 and'reed2'or'reed1')
   c:layer(5);c:poly({{p[3]-2,p[4]},{p[3]-3,p[4]-4},{p[3]-1,p[4]-6},{p[3]+1,p[4]-4},{p[3]+1,p[4]+1}},'reed1')
   c:line(p[3]-2,p[4]-4,p[3]-1,p[4]-2,'reed3');c:layer(3)
  end
  c:layer(4)
  for _,p in ipairs({{{-9,3},{-13,-2},{-20,-4},{-17,1}},{{-3,4},{-6,-5},{-12,-12},{-8,-1}},{{2,4},{5,-7},{8,-16},{7,-2}},{{7,4},{13,-2},{19,-3},{14,2}}})do leaf(c,p,'wet1')end
  c:poly({{-12,3},{-5,0},{1,2},{7,0},{13,3},{6,5},{-4,5}},'moss1')
  c:line(-10,1,-7,0,'wet1');c:line(0,3,3,2,'wet1')
 else
  -- Saltbush has interlocking leaf cushions, not a triangular grass stamp.
  c:poly({{-20,0},{-19,-7},{-14,-9},{-13,-13},{-7,-13},{-4,-19},{2,-21},{7,-17},{9,-12},{15,-13},{20,-8},{20,0},{14,4},{2,6},{-10,4}},'moss0')
  c:layer(4)
  for _,p in ipairs({{{-18,-5},{-14,-10},{-8,-11},{-3,-6},{-7,-3},{-14,-2}},{{-4,-15},{0,-19},{5,-16},{6,-11},{1,-8},{-5,-9}},{{7,-8},{13,-11},{18,-7},{18,-3},{12,-1},{6,-3}},{{-7,0},{-2,-5},{4,-6},{9,-1},{5,3},{-4,4}}})do c:poly(p,'moss1')end
  c:layer(5);c:path({{-16,-6},{-12,-9},{-8,-9}},'wet1');c:path({{-2,-15},{1,-17},{3,-15}},'wet1')
  c:line(11,-8,14,-8,'wet1');c:path({{-3,-1},{0,-3},{3,-3}},'wet1')
 end
end)
-- Shore bevels remain within the old 40x32 slots. Interior cap, irregular
-- wet face and sparse foam connect at exact dimetric corner anchors.
for _,dir in ipairs({'e','s','w','n'})do
 document('bank_'..dir,keys('shore_'..dir..'_',3),{'wet foot','eroded bank face','salt cap','sediment and foam'},function(c,v)
  local edge=({e={{16,0},{0,8}},s={{-16,0},{0,8}},w={{-16,0},{0,-8}},n={{16,0},{0,-8}}})[dir]
  local a,b=edge[1],edge[2];local front=dir=='e'or dir=='s'
  local inside=front and -1 or 1
  for i=0,16 do
   local x=a[1]+(b[1]-a[1])*i/16;local y=a[2]+(b[2]-a[2])*i/16
   local depth=front and (3+((i+v*3)%9<3 and 1 or 0)) or 1
   c:layer(1);c:line(x,y+1,x,y+depth+2,'sea0')
   c:layer(2);c:line(x,y,x,y+depth,front and'earth0'or'wet0')
   if front and i%7<4 then c:pixel(x,y+1,'earth1')end
   c:layer(3);c:pixel(x,y,'salt2');c:pixel(x,y+inside,'salt1')
   if (i+v*4)%13<5 then c:pixel(x,y,'salt3')end
   c:layer(4);if front and (i+v*3)%11<4 then c:pixel(x,y+depth+2,'sea2')end
  end
 end)
end
write(root..'/tools/art_v12/sources.json',json.encode(entries))
print('Root authored '..#entries..' coastal frames in editable Aseprite sources.')
