-- Original native pixel construction primitives. No generated-image pixels enter this renderer.
local M = {}
M.palette = {
 ink='14232d', deep='233842', shadow='39434a',
 steel0='293e49',steel1='47636b',steel2='789095',steel3='b1c2bd',
 cream0='796c56',cream1='b5a381',cream2='ded0a6',cream3='fff0c8',
 orange0='633f35',orange1='a25a39',orange2='d88a47',orange3='f4bf72',
 jade0='294f4e',jade1='497e72',jade2='7db89b',jade3='c1dfbc',
 straw0='69563b',straw1='a08857',straw2='d1b77c',straw3='f3dca0',
 amber0='6e442f',amber1='ae713a',amber2='dda353',amber3='ffe3a1',
 glass0='122f3c',glass1='235767',glass2='51939d',glass3='a3d0cb',
 plum0='292b3e',plum1='4b475e',plum2='797080',
 silt0='695f4e',silt1='928269',silt2='b5a17c',silt3='d8c6a0',
 ground0='877961',ground1='928269',ground2='9e8d70',ground3='aa9879',
 water0='203f4d',water1='2c5965',water2='46757a',water3='719394',foam='b1c1af'
}
M.layers={'shadows','far','body','upper','front','details'}
local rgb={}
for k,v in pairs(M.palette)do rgb[k]=app.pixelColor.rgba(tonumber(v:sub(1,2),16),tonumber(v:sub(3,4),16),tonumber(v:sub(5,6),16),255)end
local function round(v) return math.floor(v+0.5) end
local Context={};Context.__index=Context
function M.context(images,bounds,facing)
 return setmetatable({images=images,bounds=bounds,cx=bounds.x+bounds.anchor_x,cy=bounds.y+bounds.anchor_y,facing=facing or 0,current=images.body},Context)
end
function Context:layer(name) assert(self.images[name],'unknown layer '..name);self.current=self.images[name] end
function Context:pixel(x,y,color)
 x,y=round(x),round(y);assert(rgb[color],'unknown palette role '..tostring(color))
 local b=self.bounds
 assert(x>=b.x and y>=b.y and x<b.x+b.w and y<b.y+b.h, string.format('asset %s escaped bounds at %d,%d (%d,%d %dx%d)',b.key,x,y,b.x,b.y,b.w,b.h))
 self.current:drawPixel(x,y,rgb[color])
end
function Context:p(f,s,z)
 local a=-self.facing*math.pi/4;local cs,sn=math.cos(a),math.sin(a)
 return round(self.cx+f*cs-s*sn),round(self.cy+(f*sn+s*cs)*0.5-(z or 0))
end
function Context:line(x,y,x1,y1,c,width)
 x,y,x1,y1=round(x),round(y),round(x1),round(y1);local dx,dy=math.abs(x1-x),-math.abs(y1-y);local sx=x<x1 and 1 or -1;local sy=y<y1 and 1 or -1;local e=dx+dy;local w=width or 1
 while true do
  for yy=0,w-1 do for xx=0,w-1 do self:pixel(x+xx,y+yy,c)end end
  if x==x1 and y==y1 then break end;local e2=e*2;if e2>=dy then e=e+dy;x=x+sx end;if e2<=dx then e=e+dx;y=y+sy end
 end
end
function Context:line3(a,b,c,w)local x,y=self:p(a[1],a[2],a[3]);local xx,yy=self:p(b[1],b[2],b[3]);self:line(x,y,xx,yy,c,w)end
function Context:poly(points,c)
 local lo,hi=1e9,-1e9;local q={}
 for i,p in ipairs(points)do q[i]={round(p[1]),round(p[2])};lo=math.min(lo,q[i][2]);hi=math.max(hi,q[i][2])end
 for y=lo,hi do local cuts={};local j=#q
  for i=1,#q do local a,b=q[i],q[j];if (a[2]<=y and b[2]>y)or(b[2]<=y and a[2]>y)then cuts[#cuts+1]=a[1]+(y-a[2])*(b[1]-a[1])/(b[2]-a[2])end;j=i end
  table.sort(cuts);for i=1,#cuts-1,2 do for x=math.ceil(cuts[i]),math.ceil(cuts[i+1])-1 do self:pixel(x,y,c)end end
 end
end
function Context:poly3(points,c)local q={};for i,p in ipairs(points)do local x,y=self:p(p[1],p[2],p[3]);q[i]={x,y}end;self:poly(q,c)end
function Context:ellipse(x,y,rx,ry,c)
 rx,ry=round(rx),round(ry);if rx<=0 or ry<=0 then return end
 for dy=-ry,ry do local span=math.floor(rx*math.sqrt(math.max(0,1-dy*dy/(ry*ry))));self:line(x-span,y+dy,x+span,y+dy,c)end
end
function Context:disc(center,a,b,r,c)
 local points={};for i=0,15 do local t=i*math.pi/8;local co,si=math.cos(t)*r,math.sin(t)*r;points[#points+1]={center[1]+a[1]*co+b[1]*si,center[2]+a[2]*co+b[2]*si,center[3]+a[3]*co+b[3]*si}end;self:poly3(points,c)
end
function Context:extrude(foot,z,height,top,light,dark)
 -- Footprints should be supplied in clockwise screen order at facing0.
 local roof={};local projected={}
 for i,p in ipairs(foot)do roof[i]={p[1],p[2],z+height};local x,y=self:p(p[1],p[2],z);projected[i]={x,y}end
 local cx,cy=0,0;for _,p in ipairs(projected)do cx=cx+p[1];cy=cy+p[2]end;cx=cx/#projected;cy=cy/#projected
 self:poly3(roof,top)
 local faces={}
 for i,a in ipairs(foot)do local j=i%#foot+1;local b=foot[j];local pa,pb=projected[i],projected[j]
  local midx,midy=(pa[1]+pb[1])*0.5,(pa[2]+pb[2])*0.5
  if midy>cy+0.01 then faces[#faces+1]={y=midy,c=midx<cx and light or dark,points={{a[1],a[2],z+height},{b[1],b[2],z+height},{b[1],b[2],z},{a[1],a[2],z}}}end
 end
 table.sort(faces,function(a,b)return a.y<b.y end);for _,f in ipairs(faces)do self:poly3(f.points,f.c)end
 -- A selective upper-left rim reads the thickness without tracing every contour.
 for i,a in ipairs(foot)do local b=foot[i%#foot+1];local x,y=self:p(a[1],a[2],z+height);local xx,yy=self:p(b[1],b[2],z+height);if (x+xx)*0.5<cx then self:line(x,y,xx,yy,top)end end
end
function Context:box(f,s,z,hf,hs,h,top,light,dark)
 self:extrude({{f-hf,s-hs},{f+hf,s-hs},{f+hf,s+hs},{f-hf,s+hs}},z,h,top,light,dark)
end
function Context:shadow(rx,ry)self:layer('shadows');self:ellipse(self.cx+1,self.cy+1,rx,ry,'shadow')end
function Context:near_side()local a=-self.facing*math.pi/4;return math.cos(a)>=0 and 1 or -1 end
return M
