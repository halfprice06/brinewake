-- v18 damage marks. The v15 marks were hash-scattered four-pixel specks that
-- landed on glass and on roof planes with screen-vertical streaks. These
-- read the base drawing's planes from its shading roles (a painter box paints
-- its lit wall in shade 2, its shaded wall in shade 1 and its top in shade 3)
-- and place a few larger, spaced clusters on wall planes only: a dent is a
-- 3x2 pit with a lit lip and a rust run that narrows as it falls; a torn
-- plate is an opening with a rib inside and a lit rim; soot is one connected
-- smudge. Every pixel stays on the base silhouette above its foundation rows.
local M={}
local A=app.pixelColor.rgbaA
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end
-- material -> the families whose walls may be marked
local walls={steel={steel=true,ivory=true,rust=true},weave={jade=true,reed=true,ivory=true}}
function M.marks(C,im,material,tier,seed,opts)
 opts=opts or {}
 local w,h=im.width,im.height
 local body,lowest,top={},-1,h
 for y=0,h-1 do for x=0,w-1 do if A(im:getPixel(x,y))>0 then body[#body+1]={x,y};if y>lowest then lowest=y end;if y<top then top=y end end end end
 local dents=Image(w,h,ColorMode.RGB);local streaks=Image(w,h,ColorMode.RGB);local soot=Image(w,h,ColorMode.RGB)
 if #body==0 then return {dents,streaks,soot} end
 local floor_limit=lowest-math.floor((lowest-top)*0.18)
 local fam_ok=walls[material]
 local function role(x,y) if x<0 or y<0 or x>=w or y>=h then return nil end local v=im:getPixel(x,y);if A(v)==0 then return nil end return C.role_of[v] end
 local function split(r) if not r then return nil end local fam,sh=r:match('^(%a+)(%d)$');if not fam then return nil end return fam,tonumber(sh) end
 -- A wall pixel: a marked family in shade 1 or 2. Returns family, shade.
 local function wall(x,y) local fam,sh=split(role(x,y));if fam and fam_ok[fam] and (sh==1 or sh==2) then return fam,sh end end
 local function opaque(x,y) return role(x,y)~=nil end
 local function inside(y) return y<floor_limit and y>top+2 end
 local placed={}
 local function clear_of(x,y,d) for _,p in ipairs(placed) do if math.abs(p[1]-x)<d and math.abs(p[2]-y)<d then return false end end return true end
 local function same_plane(x,y,bw,bh)
  local fam,sh=wall(x,y);if not fam then return nil end
  for j=0,bh-1 do for i=0,bw-1 do local f2,s2=wall(x+i,y+j);if f2~=fam or s2~=sh then return nil end end end
  return fam,sh
 end
 local order={}
 for i,p in ipairs(body) do order[i]={p[1],p[2],hash(p[1],p[2],seed+tier*7)} end
 table.sort(order,function(a,b) return a[3]<b[3] end)
 local count=opts.count or ((tier==1) and 3 or 5)
 local spacing=opts.spacing or 10
 -- Marks never land on glass or off the body.
 local function markable(x,y) local r=role(x,y);return r~=nil and not r:find('^glass') end
 local function put(img,x,y,r) if markable(x,y) and y<=lowest then img:drawPixel(x,y,C.rgba[r]) end end
 -- Dents (steel) or creases (weave).
 local n=0
 for _,o in ipairs(order) do
  if n>=count then break end
  local x,y=o[1],o[2]
  if inside(y) and inside(y+1) and clear_of(x,y,spacing) then
   local big=w>=100
   local pw,ph=big and 4 or 3,big and 3 or 2
   local fam,sh=same_plane(x,y,pw,ph)
   if fam then
    placed[#placed+1]={x,y};n=n+1
    if material=='steel' then
     -- Pit: 3x2 (4x3 on a building) in the family's darkest shade, deepest
     -- at the lower right.
     for j=0,ph-1 do for i=0,pw-1 do put(dents,x+i,y+j,fam..'0') end end
     put(dents,x+pw-1,y+ph-1,'ink');put(dents,x+pw-2,y+ph-1,'ink')
     if big then put(dents,x+pw-1,y+ph-2,'ink') end
     -- Lit lip: the light comes from the upper left, so the rim above and
     -- to the left catches it; keep it inside the same plane.
     if wall(x-1,y) then put(dents,x-1,y,fam..(sh==2 and '4' or '3')) end
     if wall(x,y-1) and wall(x+1,y-1) then put(dents,x,y-1,fam..(sh==2 and '4' or '3'));put(dents,x+1,y-1,fam..(sh==2 and '4' or '3')) end
     -- Rust run: two wide at the pit, one below, stopping at the plane's edge.
     local len=(big and 5 or 3)+hash(x,y,seed)%4
     for d=1,len do
      local yy=y+ph-1+d
      if not wall(x+1,yy) or yy>=floor_limit then break end
      put(streaks,x+1,yy,(d%3==0) and 'rust0' or 'rust1')
      if d<=(big and 3 or 2) and wall(x+2,yy) then put(streaks,x+2,yy,'rust0') end
     end
    else
     -- Crease: a dark fold with a frayed strand hanging from it.
     for j=0,ph-1 do for i=0,pw-1 do put(dents,x+i,y+j,'jade0') end end
     put(dents,x,y,fam..'1');put(dents,x+pw-1,y+ph-1,'ink')
     if wall(x-1,y-1) then put(dents,x-1,y-1,fam..'3') end
     local len=(big and 4 or 3)+hash(x,y,seed)%3
     for d=1,len do
      local yy=y+ph-1+d;local xx=x+1+((d>=2) and 1 or 0)
      if not markable(xx,yy) or yy>=floor_limit then break end
      put(streaks,xx,yy,(d==len) and 'reed1' or 'reed0')
      if d==1 then put(streaks,xx+1,yy,'reed0') end
     end
    end
   end
  end
 end
 if tier==2 then
  -- A torn opening on a lit wall, spaced from the dents.
  local site
  for _,o in ipairs(order) do
   local x,y=o[1],o[2]
   if inside(y) and inside(y+4) and clear_of(x+3,y+2,spacing+2) then
    local fam,sh=same_plane(x,y,7,5)
    if fam then site={x,y,fam,sh};break end
   end
  end
  if site then
   local x,y,fam,sh=site[1],site[2],site[3],site[4]
   placed[#placed+1]={x+3,y+2}
   local hole=(material=='steel') and 'ink' or 'jade0'
   for j=0,4 do for i=0,6 do put(dents,x+i,y+j,hole) end end
   -- Structure inside the hole: a rib or a bent stay catching a little light.
   if material=='steel' then
    for j=0,4 do put(dents,x+3,y+j,'steel0') end
    put(dents,x+3,y+1,'steel1');put(dents,x+3,y+2,'steel1')
   else
    for j=0,4 do put(dents,x+2+math.floor(j/2),y+j,'reed0') end
    put(dents,x+2,y,'reed1')
   end
   -- Lit rim above and left, dark edge below and right, torn corner.
   for i=-1,6 do if wall(x+i,y-1) then put(dents,x+i,y-1,fam..(sh==2 and '4' or '3')) end end
   for j=0,4 do if wall(x-1,y+j) then put(dents,x-1,y+j,fam..'3') end end
   for j=0,4 do if wall(x+7,y+j) then put(dents,x+7,y+j,fam..'0') end end
   -- A plate edge or reed ends hanging under the opening.
   if material=='steel' then
    put(dents,x+4,y+5,'steel1');put(dents,x+5,y+5,'steel1');put(dents,x+5,y+6,'steel0')
   else
    for d=1,3 do put(streaks,x+2,y+4+d,'reed0');put(streaks,x+4,y+4+d,(d==3) and 'reed1' or 'reed0') end
   end
   -- Soot: one connected smudge drifting up and right from the opening.
   local rows={{-2,0,3},{-3,-1,5},{-4,0,4},{-5,1,2}}
   for _,r in ipairs(rows) do
    local yy=y+r[1]
    for i=0,r[3]-1 do local xx=x+2+r[2]+i;if markable(xx,yy) and yy>top then soot:drawPixel(xx,yy,C.rgba.contact) end end
   end
  end
 end
 return {dents,streaks,soot}
end
return M
