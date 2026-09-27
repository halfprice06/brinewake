-- Root-authored v14 water: broad quiet pools of deeper shade, two or three
-- long current ribbons with a shadow edge, and sparse crests that travel
-- along the ribbons over the eight 200 ms phases. The shared ribbon at y=12
-- still enters and leaves every module at the same points so neighbouring
-- modules of different variants join. Transparent texels show the live fill.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
C.extra.sea_ink='2b515b';C.extra.sea_mid='416b73'
for k,v in pairs(C.extra) do C.rgba[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255) end
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v14/water';os.execute('mkdir -p '..out)
local layers={'quiet transparency','deep shade pools','current ribbons','ribbon shadow','surface sheen','travelling crests'}
local shared={{-3,12},{10,12},{18,10}}
local shared_out={{103,12},{113,12},{131,12}}
local ribbons={
 {{{18,10},{30,8},{44,10},{58,16},{72,20},{86,18},{103,12}},{{12,44},{26,40},{40,42},{52,48},{66,52},{80,50},{96,46},{110,48}}},
 {{{18,10},{34,6},{50,8},{62,14},{78,18},{92,16},{103,12}},{{22,40},{36,36},{50,40},{60,46},{74,50},{90,48},{104,44}}},
 {{{18,10},{28,14},{42,22},{56,24},{70,20},{84,14},{103,12}},{{14,50},{28,46},{40,44},{54,46},{68,52},{82,54},{98,50},{112,44}}},
}
local pools={
 {{{30,22},{58,20},{80,26},{74,36},{48,38},{28,32}},{{84,40},{110,38},{118,46},{100,54},{82,50}}},
 {{{20,20},{44,18},{60,24},{52,34},{30,34},{16,28}},{{72,30},{104,26},{118,34},{108,42},{82,42},{68,38}}},
 {{{40,28},{70,26},{86,32},{76,40},{50,42},{34,36}},{{16,40},{34,38},{40,46},{24,52},{12,48}}},
}
local function length(path) local l=0;for i=2,#path do l=l+math.abs(path[i][1]-path[i-1][1])+math.abs(path[i][2]-path[i-1][2]) end;return l end
local function at(path,t)
 -- Walk the polyline by manhattan length to a point at parameter t in 0..1.
 local total=length(path);local want=t*total;local acc=0
 for i=2,#path do
  local a,b=path[i-1],path[i];local seg=math.abs(b[1]-a[1])+math.abs(b[2]-a[2])
  if acc+seg>=want then local f=(want-acc)/math.max(seg,1);return {a[1]+(b[1]-a[1])*f,a[2]+(b[2]-a[2])*f} end
  acc=acc+seg
 end
 return path[#path]
end
for v=0,2 do
 local frames={}
 for phase=0,7 do
  frames[#frames+1]={key='water_current_'..v..'_'..phase,duration=.2,draw=function(c)
   c:layer(2)
   for k,pool in ipairs(pools[v+1]) do
    c:poly(pool,'sea_ink')
    -- Break the pool's outline so it reads as a soft deeper area, not a
    -- cut-out shape: erase short runs along the polygon edge.
    local n=#pool
    for i=1,n do
     local a,b=pool[i],pool[i%n+1]
     local steps=math.max(math.abs(b[1]-a[1]),math.abs(b[2]-a[2]))
     for t=0,steps do
      if (t+i*3+k*5)%7<3 then
       local x=a[1]+(b[1]-a[1])*t/steps;local y=a[2]+(b[2]-a[2])*t/steps
       c.im:drawPixel(math.floor(x+.5),math.floor(y+.5),0)
      end
     end
    end
    -- A few deeper strokes inside instead of a hard darker core.
    local cx,cy=0,0;for _,p in ipairs(pool) do cx=cx+p[1];cy=cy+p[2] end;cx,cy=cx/n,cy/n
    c:line(cx-9,cy-1,cx+4,cy-2,'sea0');c:line(cx-3,cy+3,cx+8,cy+2,'sea0');c:line(cx-11,cy+2,cx-6,cy+3,'sea0')
   end
   c:layer(3);c:path(shared,'sea_mid');c:path(shared_out,'sea_mid')
   for _,r in ipairs(ribbons[v+1]) do
    c:path(r,'sea_mid')
    local upper={};for _,p in ipairs(r) do upper[#upper+1]={p[1],p[2]-1} end;c:path(upper,'sea_mid')
    c:layer(4);local shadow={};for _,p in ipairs(r) do shadow[#shadow+1]={p[1]+1,p[2]+2} end;c:path(shadow,'sea_ink');c:layer(3)
   end
   c:layer(5)
   local s=({{26,30,34,29},{96,22,102,21},{60,58,70,57}})[v+1]
   c:line(s[1],s[2],s[3],s[4],'sea2')
   c:layer(6)
   for i,r in ipairs(ribbons[v+1]) do
    for k=0,1 do
     local t=((phase/8)+k*0.5+i*0.23)%1
     local p=at(r,t);local x,y=math.floor(p[1]+.5),math.floor(p[2]+.5)-1
     c:line(x-1,y,x+2,y,'sea2');c:pixel(x+1,y-1,'sea3')
    end
   end
   -- One crest also travels the shared ribbon so module joins share motion.
   local x=-3+math.floor(((phase*2)%16)+.5);c:line(x,12,x+2,12,'sea2')
   local x2=103+((phase*2)%16);c:line(x2,12,x2+2,12,'sea2')
  end}
 end
 C.document('water_current_'..v,128,64,0,0,layers,frames,'water_current_'..v,m,packer,out,entries)
end
C.write(root..'/tools/art_v14/sources-water.json',json.encode(entries))
print('Authored '..#entries..' water module frames.')
