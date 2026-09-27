-- Root-authored v14 banks: the salt cap breaks into runs instead of hugging
-- the whole diamond edge, the eroded face varies in depth, a dark wet foot
-- meets the water and a few foam flecks sit on the shelf. Same 40x32 slots,
-- same anchors and edge geometry as the v12 sources they replace.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v14/banks';os.execute('mkdir -p '..out)
local layers={'wet foot','eroded bank face','salt cap','sediment and foam'}
for _,dir in ipairs({'e','s','w','n'}) do
 local frames={}
 for v=0,2 do
  frames[#frames+1]={key='shore_'..dir..'_'..v,duration=1,draw=function(c)
   local edge=({e={{16,0},{0,8}},s={{-16,0},{0,8}},w={{-16,0},{0,-8}},n={{16,0},{0,-8}}})[dir]
   local a,b=edge[1],edge[2];local front=dir=='e' or dir=='s'
   for i=0,16 do
    local x=a[1]+(b[1]-a[1])*i/16;local y=a[2]+(b[2]-a[2])*i/16
    if front then
     -- Face depth wanders 2..4 in runs; the foot line follows it.
     local depth=2+(((i+v*4)%9)<4 and 1 or 0)+(((i+v*7)%13)<3 and 1 or 0)
     c:layer(1);c:line(x,y+1,x,y+depth+1,'sea0')
     c:layer(2);c:line(x,y,x,y+depth,'earth0')
     if (i+v*2)%5<2 then c:pixel(x,y+1,'earth1') end
     if (i+v*5)%7==0 then c:pixel(x,y+depth,'damp') end
     c:layer(3)
     local cap=((i+v*5)%9)<6
     if cap then c:pixel(x,y,'salt2');c:pixel(x,y-1,'salt1') else c:pixel(x,y,'salt1') end
     if cap and (i+v*4)%13<2 then c:pixel(x,y,'salt3') end
     c:layer(4)
     if (i+v*3)%11<2 then c:pixel(x,y+depth+2,'sea4') end
     if (i+v*6)%17==0 then c:pixel(x+1,y+depth+2,'sea3') end
    else
     -- Back edges: the far lip of the land against the water behind it.
     c:layer(1);c:pixel(x,y-1,'sea0');if (i+v*3)%6<3 then c:pixel(x,y-2,'sea0') end
     c:layer(2);c:pixel(x,y,'earth0')
     c:layer(3)
     if ((i+v*4)%8)<5 then c:pixel(x,y,'salt2') end
     c:pixel(x,y+1,'salt1')
     if (i+v*4)%13<2 then c:pixel(x,y,'salt3') end
     c:layer(4);if (i+v*5)%13<2 then c:pixel(x,y-2,'sea4') end
    end
   end
  end}
 end
 C.document('bank_'..dir,40,32,20,12,layers,frames,'bank_'..dir..'_variants',m,packer,out,entries)
end
C.write(root..'/tools/art_v14/sources-banks.json',json.encode(entries))
print('Authored '..#entries..' bank frames.')
