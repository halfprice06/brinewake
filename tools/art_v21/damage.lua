-- v21 damage marks for glazed ceramic, after the v18 method
-- (tools/art_v18/damage.lua): read the base drawing's planes from its
-- shading roles and place a few spaced clusters on wall planes only. Glaze
-- does not dent, it chips and cracks: a chip is a flake knocked off the
-- glaze that shows the pale body underneath, lit on its lower-right inner
-- edge (the pit faces the light), with a dark crack running from it along
-- the plate. Tier two adds a broken plate, an opening with the frame
-- behind it, and one soot smudge. Marks never land on glass, crust or off
-- the body, and never on the lowest rows (the feet and foundations).
local M={}
local A=app.pixelColor.rgbaA
local function hash(x,y,s) local h=(x*73856093)~(y*19349663)~(s*83492791);h=(h~(h>>13))*1274126177;return (h~(h>>16))&0x7fffffff end
function M.marks(C,im,tier,seed,opts)
 opts=opts or {}
 local w,h=im.width,im.height
 local body,lowest,top={},-1,h
 for y=0,h-1 do for x=0,w-1 do if A(im:getPixel(x,y))>0 then body[#body+1]={x,y};if y>lowest then lowest=y end;if y<top then top=y end end end end
 local chips=Image(w,h,ColorMode.RGB);local cracks=Image(w,h,ColorMode.RGB);local soot=Image(w,h,ColorMode.RGB)
 if #body==0 then return {chips,cracks,soot} end
 local floor_limit=lowest-math.floor((lowest-top)*(opts.floor or 0.3))
 local function role(x,y) if x<0 or y<0 or x>=w or y>=h then return nil end local v=im:getPixel(x,y);if A(v)==0 then return nil end return C.role_of[v] end
 -- A wall pixel: glaze in shade 1 or 2 (a painter box's walls).
 local function wall(x,y) local r=role(x,y);return r=='glaze1' or r=='glaze2' end
 local function markable(x,y) local r=role(x,y);return r~=nil and (r:find('^glaze') or r=='ink') end
 local function put(img,x,y,r) if markable(x,y) and y<floor_limit then img:drawPixel(x,y,C.rgba[r]) end end
 local placed={}
 local function clear_of(x,y,d) for _,p in ipairs(placed) do if math.abs(p[1]-x)<d and math.abs(p[2]-y)<d then return false end end return true end
 local function plane(x,y,bw,bh) for j=0,bh-1 do for i=0,bw-1 do if not wall(x+i,y+j) then return false end end end return true end
 local order={}
 for i,p in ipairs(body) do order[i]={p[1],p[2],hash(p[1],p[2],seed+tier*7)} end
 table.sort(order,function(a,b) return a[3]<b[3] end)
 local big=w>=100
 local count=opts.count or ((tier==1) and 2 or 3)
 local spacing=opts.spacing or 9
 local n=0
 for _,o in ipairs(order) do
  if n>=count then break end
  local x,y=o[1],o[2]
  local pw,ph=big and 4 or 3,big and 3 or 2
  if y>top+2 and y+ph<floor_limit and clear_of(x,y,spacing) and plane(x-1,y-1,pw+2,ph+1) then
   placed[#placed+1]={x,y};n=n+1
   -- The chip: pale body showing, darker at its upper-left lip (in the
   -- shadow of the glaze edge), lit at the lower right.
   for j=0,ph-1 do for i=0,pw-1 do put(chips,x+i,y+j,'crust0') end end
   put(chips,x+pw-1,y+ph-1,'crust1');if big then put(chips,x+pw-2,y+ph-1,'crust1') end
   for i=0,pw-1 do put(chips,x+i,y-1,'glaze0') end
   put(chips,x-1,y,'glaze0')
   -- The crack: one pixel wide, stepping down and away from the chip in
   -- runs of two, stopping at the plane's edge.
   local len=(big and 6 or 4)+hash(x,y,seed)%3
   local dir=(hash(y,x,seed)%2==0) and 1 or -1
   local cx,cy=(dir>0) and (x+pw) or (x-1),y+ph-1
   for d=1,len do
    if d%2==1 then cx=cx+dir else cy=cy+1 end
    if not wall(cx,cy) or cy>=floor_limit then break end
    put(cracks,cx,cy,'glaze0')
   end
  end
 end
 if tier==2 then
  local site
  for _,o in ipairs(order) do
   local x,y=o[1],o[2]
   local bw,bh=big and 7 or 5,big and 5 or 4
   if y>top+2 and y+bh<floor_limit and clear_of(x+2,y+2,spacing) and plane(x-1,y-1,bw+2,bh+2) then site={x,y,bw,bh};break end
  end
  if site then
   local x,y,bw,bh=site[1],site[2],site[3],site[4]
   placed[#placed+1]={x+2,y+2}
   -- The opening, with broken corners so it reads as a shattered plate.
   for j=0,bh-1 do for i=0,bw-1 do
    local corner=(i==0 and j==bh-1) or (i==bw-1 and j==0)
    if not corner then put(chips,x+i,y+j,'ink') end
   end end
   -- The frame behind it: a copper rib catching a little light.
   for j=1,bh-1 do put(chips,x+math.floor(bw/2),y+j,'rust1') end
   put(chips,x+math.floor(bw/2),y+1,'rust2')
   -- Pale broken edge along the top and left, where the glaze sheared.
   for i=0,bw-2 do put(chips,x+i,y-1,'crust1') end
   for j=0,bh-2 do put(chips,x-1,y+j,'crust0') end
   -- Soot: one connected smudge drifting up and right.
   local rows={{-2,1,3},{-3,0,4},{-4,1,3},{-5,2,2}}
   for _,r in ipairs(rows) do
    local yy=y+r[1]
    for i=0,r[3]-1 do local xx=x+2+r[2]+i;if markable(xx,yy) and yy>top then soot:drawPixel(xx,yy,C.rgba.contact) end end
   end
  end
 end
 return {chips,cracks,soot}
end
return M
