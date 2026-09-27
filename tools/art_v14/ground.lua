-- Root-authored v14 ground families: low and high salt tones and a damp
-- waterline tile. Same 40x24 diamond, same anchor as the existing tiles.
-- Quiet interiors: a few connected marks, no enclosed rings, no speckle.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v14/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v14/ground';os.execute('mkdir -p '..out)
local layers={'ground plane','mineral bodies','fracture and wear'}
local function frames(prefix,n,draw)
 local t={};for v=0,n-1 do t[#t+1]={key=prefix..v,duration=1,draw=function(c) draw(c,v) end} end;return t
end
local clip=function(c) c.clip=C.diamond_clip end
-- Low salt: a slightly darker hollow where brine collects and dries last.
C.document('ground_salt_low',40,24,20,12,layers,frames('terrain_salt_low_',4,function(c,v)
 c:layer(1);c:diamond('earth1')
 c:layer(2)
 if v==0 then -- plain fill; the renderer uses variant 0 for most cells
 elseif v==1 then c:path({{-11,0},{-5,-2},{1,-1},{6,1}},'earth0');c:line(-2,3,4,3,'salt0')
 elseif v==2 then c:poly({{-6,-3},{1,-4},{5,-2},{2,-1},{-4,-1}},'earth0');c:line(3,2,9,1,'earth0')
 else c:path({{-8,2},{-2,1},{3,2},{9,0}},'earth0');c:line(-6,-2,-1,-3,'salt0') end
 c:layer(3)
 if v==1 then c:line(-3,-3,0,-3,'salt0') elseif v==3 then c:pixel(5,-1,'salt0');c:pixel(6,-1,'salt0') end
end),'salt_low_variants',m,packer,out,entries,clip)
-- High salt: a paler crust plate catching the light.
C.document('ground_salt_high',40,24,20,12,layers,frames('terrain_salt_high_',4,function(c,v)
 c:layer(1);c:diamond('salt3')
 c:layer(2)
 if v==0 then -- plain fill
 elseif v==1 then c:poly({{-7,1},{-1,-2},{7,-2},{9,0},{3,3},{-3,3}},'ivory2');c:line(-5,-3,0,-4,'ivory2')
 elseif v==2 then c:poly({{-12,0},{-6,-3},{-1,-2},{-4,1}},'ivory2');c:poly({{1,-1},{6,-3},{11,0},{6,2}},'ivory2')
 else c:path({{-9,-1},{-3,-3},{4,-3},{8,-1}},'ivory2');c:path({{-6,2},{0,1},{7,2}},'ivory2') end
 c:layer(3)
 if v==1 then c:line(2,0,6,-1,'salt1') elseif v==2 then c:line(-5,0,-2,1,'salt1') elseif v==3 then c:line(-1,-1,3,-1,'salt1') end
end),'salt_high_variants',m,packer,out,entries,clip)
-- Boundary steps: a low patch meets the mid floor through a 'dim' ring and a
-- high patch through a 'pale' ring, so tonal planes ramp instead of stepping.
C.document('ground_salt_dim',40,24,20,12,layers,frames('terrain_salt_dim_',4,function(c,v)
 c:layer(1);c:diamond('salt0')
 c:layer(2)
 if v==0 then
 elseif v==1 then c:path({{-10,0},{-4,-2},{2,-1},{7,1}},'earth1');c:line(-2,3,3,3,'salt1')
 elseif v==2 then c:poly({{-5,-3},{2,-4},{6,-2},{3,-1},{-3,-1}},'earth1');c:line(4,2,9,1,'earth1')
 else c:path({{-7,2},{-1,1},{4,2},{9,0}},'earth1');c:line(-5,-2,0,-3,'salt1') end
end),'salt_dim_variants',m,packer,out,entries,clip)
C.document('ground_salt_pale',40,24,20,12,layers,frames('terrain_salt_pale_',4,function(c,v)
 c:layer(1);c:diamond('salt2')
 c:layer(2)
 if v==0 then
 elseif v==1 then c:poly({{-6,1},{0,-2},{6,-2},{8,0},{3,3},{-2,3}},'salt3');c:line(-4,-3,1,-4,'salt3')
 elseif v==2 then c:poly({{-11,0},{-5,-3},{0,-2},{-3,1}},'salt3');c:poly({{2,-1},{7,-3},{11,0},{6,2}},'salt3')
 else c:path({{-8,-1},{-2,-3},{5,-3},{9,-1}},'salt3');c:path({{-5,2},{1,1},{8,2}},'salt3') end
 c:layer(3)
 if v==1 then c:line(2,0,6,-1,'salt1') elseif v==2 then c:line(-5,0,-2,1,'salt1') end
end),'salt_pale_variants',m,packer,out,entries,clip)
-- Damp waterline: darker wet sand with a dry crust fleck at the top edge.
C.document('ground_damp',40,24,20,12,layers,frames('terrain_damp_',4,function(c,v)
 c:layer(1);c:diamond('damp')
 c:layer(2)
 if v==0 then c:line(-4,2,1,2,'earth0') -- one quiet wet streak on the common tile
 elseif v==1 then c:path({{-12,0},{-6,-2},{0,-1},{5,0},{10,-1}},'earth0');c:line(-3,3,3,3,'earth0');c:line(2,-4,6,-4,'salt0')
 elseif v==2 then c:poly({{-7,-2},{0,-4},{6,-2},{3,0},{-3,0}},'earth0');c:line(-9,2,-3,3,'earth0');c:line(6,2,10,1,'earth0')
 else c:path({{-9,-1},{-4,1},{2,1},{8,-1}},'earth0');c:poly({{-2,-4},{2,-5},{4,-3},{0,-3}},'salt0');c:line(-5,4,0,4,'earth0') end
 c:layer(3)
 if v%2==0 then c:pixel(-1,1,'earth1');c:pixel(0,1,'earth1') end -- a dry grain catching light
end),'damp_variants',m,packer,out,entries,clip)
C.write(root..'/tools/art_v14/sources-ground.json',json.encode(entries))
C.write(root..'/art/exports/game-assets-v14.json',json.encode(m))
print('Authored '..#entries..' ground frames; manifest now has '..(function() local n=0;for _ in pairs(m.sprites) do n=n+1 end;return n end)()..' entries.')
