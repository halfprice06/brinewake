-- Root-authored Aseprite helpers. All drawing is on integer native pixels.
local M={}
function M.poly(c,points,color)
  local p={};for _,v in ipairs(points) do p[#p+1]={c.cx+v[1],c.cy+v[2]} end
  c:poly(p,color)
end
function M.line(c,x,y,x1,y1,color,width)
  c:line({c.cx+x,c.cy+y},{c.cx+x1,c.cy+y1},color,width)
end
function M.rect(c,x,y,w,h,color)
  if w<=0 or h<=0 then return end
  for yy=y,y+h-1 do M.line(c,x,yy,x+w-1,yy,color) end
end
function M.oval(c,x,y,rx,ry,color)
  c:ellipse(c.cx+x,c.cy+y,rx,ry,color)
end
function M.block(c,x,y,w,d,h,mat)
  M.poly(c,{{x-w,y-w/2},{x,y},{x,y-h},{x-w,y-w/2-h}},mat..'2')
  M.poly(c,{{x,y},{x+d,y-d/2},{x+d,y-d/2-h},{x,y-h}},mat..'1')
  M.poly(c,{{x-w,y-w/2-h},{x-w+d,y-(w+d)/2-h},{x+d,y-d/2-h},{x,y-h}},mat..'3')
  M.line(c,x-w,y-w/2-h,x-w+d,y-(w+d)/2-h,mat..'4')
end
function M.clear(c,index)
  c:layer(index);c.image:clear()
end
return M
