-- Root-authored pixel construction for BRINEWAKE. Coordinates terminate on the
-- native pixel grid; no reference bitmap is read or transformed by this code.
local M = {}
M.layers = { 'contact shadows', 'rear mechanisms', 'hulls', 'cabins and shells', 'working parts', 'surface finish' }
M.palette = {
  ink='17232e', contact='33332f', cast='655e50',
  steel0='263844', steel1='405465', steel2='657f89', steel3='9eb4b5', steel4='d8e0cf',
  ivory0='55535c', ivory1='898477', ivory2='beb79b', ivory3='e7d9b5', ivory4='fff0ce',
  rust0='55353a', rust1='874636', rust2='bf673a', rust3='e9994e', rust4='ffd083',
  jade0='253b45', jade1='32615f', jade2='568e7c', jade3='93c4a0', jade4='dbebbc',
  reed0='514346', reed1='7d634e', reed2='af8e5b', reed3='d8bd79', reed4='f7dea1',
  amber0='643d38', amber1='9d5738', amber2='d78e42', amber3='f2c365', amber4='fff0b1',
  glass0='152e42', glass1='24586d', glass2='438e9c', glass3='91ccd0', glass4='e0f2d8',
  earth0='60584e', earth1='827665', earth2='9d8f76', earth3='b6a486', earth4='d4c1a1',
  salt0='928671', salt1='9d8f76', salt2='a5987d', salt3='b0a085',
  ui0='101c29', ui1='1e303f', ui2='314b59', ui3='55727a', ui4='91aaa7',
  foam='c4d4c4', wet0='455853', wet1='667a68', moss0='465d4b', moss1='72815a',
  sea0='213f4b', sea1='305b66', sea2='4d7c82', sea3='80a8a6', sea4='b3c8b7',
}
local pixels = {}
for key, hex in pairs(M.palette) do
  pixels[key] = app.pixelColor.rgba(tonumber(hex:sub(1,2),16), tonumber(hex:sub(3,4),16), tonumber(hex:sub(5,6),16), 255)
end
function M.install_palette(sprite)
  local roles={'ink','contact','cast'}
  for _,family in ipairs({'steel','ivory','rust','jade','reed','amber','glass','earth','sea'}) do
    for shade=0,4 do roles[#roles+1]=family..shade end
  end
  for shade=0,3 do roles[#roles+1]='salt'..shade end
  for shade=0,4 do roles[#roles+1]='ui'..shade end
  for _,role in ipairs({'foam','wet0','wet1','moss0','moss1'}) do roles[#roles+1]=role end
  local palette=Palette(#roles+1)
  palette:setColor(0,0)
  for i,role in ipairs(roles) do palette:setColor(i,pixels[role]) end
  sprite:setPalette(palette)
end
local P = {}; P.__index = P
local function round(n) return math.floor(n + 0.5) end
function M.new(images, bounds, face)
  local angle = -(face or 0) * math.pi / 4
  return setmetatable({ images=images, bounds=bounds, image=images['hulls'],
    cx=bounds.x+bounds.anchor_x, cy=bounds.y+bounds.anchor_y,
    face=face or 0, co=math.cos(angle), si=math.sin(angle) }, P)
end
function P:layer(index) self.image = assert(self.images[M.layers[index]]) end
function P:point(f,s,z)
  return {round(self.cx+f*self.co-s*self.si), round(self.cy+(f*self.si+s*self.co)/2-(z or 0))}
end
function P:pixel(x,y,color)
  x,y=round(x),round(y)
  local b=self.bounds
  assert(x>=b.x and y>=b.y and x<b.x+b.w and y<b.y+b.h, b.key..' pixel outside cell at '..x..','..y)
  if self.mask then
    local visible=0
    for i=#M.layers,1,-1 do
      local value=self.images[M.layers[i]]:getPixel(x,y)
      if app.pixelColor.rgbaA(value)>0 then visible=value;break end
    end
    if not self.mask[visible] then return end
  end
  self.image:drawPixel(x,y,assert(pixels[color],color))
end
function P:overpaint(points,color,allowed)
  self.mask={}
  for _,role in ipairs(allowed) do self.mask[assert(pixels[role])]=true end
  self:poly(points,color)
  self.mask=nil
end
function P:overpaint3(points,color,allowed)
  local projected={};for _,p in ipairs(points) do projected[#projected+1]=self:point(table.unpack(p)) end
  self:overpaint(projected,color,allowed)
end
function P:line(a,b,color,width)
  local x,y,x1,y1=round(a[1]),round(a[2]),round(b[1]),round(b[2])
  local dx,dy=math.abs(x1-x),-math.abs(y1-y)
  local sx,sy=x<x1 and 1 or -1,y<y1 and 1 or -1
  local err=dx+dy
  while true do
    for yy=0,(width or 1)-1 do for xx=0,(width or 1)-1 do self:pixel(x+xx,y+yy,color) end end
    if x==x1 and y==y1 then break end
    local e=err*2
    if e>=dy then err=err+dy;x=x+sx end
    if e<=dx then err=err+dx;y=y+sy end
  end
end
function P:line3(a,b,color,width) self:line(self:point(table.unpack(a)),self:point(table.unpack(b)),color,width) end
function P:poly(points,color)
  local lo,hi=1e9,-1e9
  for _,p in ipairs(points) do lo=math.min(lo,p[2]);hi=math.max(hi,p[2]) end
  for y=math.ceil(lo),math.floor(hi) do
    local cuts={};local j=#points
    for i,a in ipairs(points) do
      local b=points[j]
      if (a[2]<=y and b[2]>y) or (b[2]<=y and a[2]>y) then
        cuts[#cuts+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2])
      end
      j=i
    end
    table.sort(cuts)
    for i=1,#cuts-1,2 do self:line({math.ceil(cuts[i]),y},{math.ceil(cuts[i+1])-1,y},color) end
  end
end
function P:poly3(points,color)
  local q={};for _,p in ipairs(points) do q[#q+1]=self:point(table.unpack(p)) end
  self:poly(q,color)
end
function P:ellipse(x,y,rx,ry,color)
  for dy=-ry,ry do
    local span=math.floor(rx*math.sqrt(math.max(0,1-dy*dy/(ry*ry))))
    self:line({x-span,y+dy},{x+span,y+dy},color)
  end
end
function P:shadow(rx,ry)
  self:layer(1)
  self:ellipse(self.cx+3,self.cy+2,rx,ry,'cast')
end
function P:disc(center,u,v,r,color)
  local q={}
  for i=0,23 do
    local a=i*math.pi/12;local co,si=math.cos(a)*r,math.sin(a)*r
    q[#q+1]={center[1]+u[1]*co+v[1]*si,center[2]+u[2]*co+v[2]*si,center[3]+u[3]*co+v[3]*si}
  end
  self:poly3(q,color)
end
function P:dot(f,s,z,r,material)
  local p=self:point(f,s,z)
  self:ellipse(p[1],p[2],r,r,'ink')
  self:ellipse(p[1],p[2]-1,math.max(1,r-1),math.max(1,r-1),material..'1')
  if r>=3 then self:line({p[1]-r+2,p[2]-r+2},{p[1],p[2]-r+2},material..'3') end
end
function P:beam(a,b,width,material)
  local p,q=self:point(table.unpack(a)),self:point(table.unpack(b))
  local dx,dy=q[1]-p[1],q[2]-p[2];local length=math.sqrt(dx*dx+dy*dy)
  if length<1 then return end
  local nx,ny=-dy/length*width/2,dx/length*width/2
  local function polygon(offset)
    return {{round(p[1]+nx+offset),round(p[2]+ny+offset)},
      {round(q[1]+nx+offset),round(q[2]+ny+offset)},
      {round(q[1]-nx+offset),round(q[2]-ny+offset)},
      {round(p[1]-nx+offset),round(p[2]-ny+offset)}}
  end
  self:poly(polygon(1),'ink');self:poly(polygon(0),material..'2')
  self:line({round(p[1]-nx),round(p[2]-ny)},{round(q[1]-nx),round(q[2]-ny)},material..'3')
  self:line({round(p[1]+nx),round(p[2]+ny)},{round(q[1]+nx),round(q[2]+ny)},material..'0')
end
function M.foot(f,s,hf,hs,cut)
  local b=math.min(cut or 2,hf-1,hs-1)
  return {{f-hf+b,s-hs},{f+hf-b,s-hs},{f+hf,s-hs+b},{f+hf,s+hs-b},
    {f+hf-b,s+hs},{f-hf+b,s+hs},{f-hf,s+hs-b},{f-hf,s-hs+b}}
end
function P:solid(foot,z,height,material)
  local cx,cy=0,0;local top={}
  for _,p in ipairs(foot) do cx=cx+p[1];cy=cy+p[2];top[#top+1]={p[1],p[2],z+height} end
  cx,cy=cx/#foot,cy/#foot
  local middle=self:point(cx,cy,z)
  local walls={}
  for i,a in ipairs(foot) do
    local b=foot[i%#foot+1];local pa,pb=self:point(a[1],a[2],z),self:point(b[1],b[2],z)
    local mx,my=(pa[1]+pb[1])/2,(pa[2]+pb[2])/2
    if my>middle[2]+0.1 then walls[#walls+1]={a=a,b=b,y=my,shade=mx<middle[1] and 2 or 1} end
  end
  table.sort(walls,function(a,b)return a.y<b.y end)
  for _,w in ipairs(walls) do
    local a,b=w.a,w.b
    self:poly3({{a[1],a[2],z},{b[1],b[2],z},{b[1],b[2],z+height},{a[1],a[2],z+height}},material..w.shade)
    self:line3({a[1],a[2],z},{b[1],b[2],z},material..'0')
    if height>5 then
      self:line3({a[1],a[2],z+2},{b[1],b[2],z+2},material..math.max(0,w.shade-1))
    end
  end
  self:poly3(top,material..'3')
  for _,w in ipairs(walls) do
    self:line3({w.a[1],w.a[2],z+height},{w.b[1],w.b[2],z+height},material..(w.shade==2 and '4' or '2'))
  end
end
function P:box(f,s,z,hf,hs,h,material,cut) self:solid(M.foot(f,s,hf,hs,cut),z,h,material) end
function P:cylinder(f,s,z,r,h,material)
  local center=self:point(f,s,z)
  local x,y=center[1],center[2]
  self:ellipse(x,y,r,math.max(1,math.floor(r/2)),material..'0')
  for dx=-r,r do
    local band=dx<-r/2 and 2 or (dx<1 and 3 or (dx<r/2 and 2 or 1))
    self:line({x+dx,y-h},{x+dx,y},material..band)
  end
  self:ellipse(x,y-h,r,math.max(1,math.floor(r/2)),material..'3')
  self:line({x-r+2,y-h-1},{x,y-h-2},material..'4')
end
return M
