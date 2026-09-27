-- Root's frame-only console art. Typography and instruments are rendered from
-- real data; quiet interiors leave room for the engineering-owned layout.
local h=dofile(assert(app.params.root)..'/tools/art_v8/common.lua')
local M={}
local function fastener(c,x,y,faction)
  if faction=='union' then
    h.rect(c,x,y,5,4,'ui0');h.rect(c,x+1,y,3,1,'steel3')
    h.rect(c,x+1,y+2,3,1,'steel1');h.line(c,x+2,y+1,x+2,y+2,'ui3')
  else
    h.line(c,x,y+1,x+4,y+3,'reed1',2)
    h.line(c,x,y+3,x+4,y+1,'reed3')
  end
end
function M.bottom(c,faction)
  c:layer(3)
  h.rect(c,0,0,640,72,'ui0');h.rect(c,0,1,640,3,'ui2')
  h.line(c,0,0,639,0,'ui4');h.line(c,0,4,639,4,'ink')
  h.rect(c,200,7,196,49,'ui1')
  for _,x in ipairs({4,130,403,633}) do
    h.rect(c,x,7,3,57,'ui1');h.line(c,x,8,x,63,'ui3')
  end
  h.rect(c,137,7,58,58,'ui0')
  h.line(c,136,6,195,6,'steel2');h.line(c,136,6,136,65,'ui3')
  h.line(c,136,65,195,65,'ui2');h.line(c,195,6,195,65,'ui3')
  h.line(c,9,8,126,8,'steel2')
  h.rect(c,0,67,640,5,'ui1');h.line(c,0,67,639,67,'ui3')
  c:layer(6)
  for _,p in ipairs({{1,1},{634,1},{129,7},{129,59},{400,7},{400,59}}) do fastener(c,p[1],p[2],faction) end
  if faction=='union' then
    for _,x in ipairs({14,22,30,594,602,610}) do h.line(c,x,69,x+4,69,'rust2') end
    h.line(c,139,6,173,6,'ivory2')
  else
    for _,x in ipairs({14,21,28,595,602,609}) do h.line(c,x,69,x+4,70,'reed2') end
    h.line(c,139,6,173,6,'jade3')
  end
end
function M.top(c,faction)
  c:layer(3)
  h.rect(c,0,0,640,24,'ui0');h.rect(c,0,3,640,18,'ui1')
  h.line(c,0,1,639,1,'ui2');h.line(c,0,23,639,23,'ui3')
  c:layer(6)
  for _,x in ipairs({2,633}) do fastener(c,x,6,faction) end
  h.line(c,12,1,80,1,faction=='union' and 'rust2' or 'jade2')
  h.line(c,583,1,625,1,faction=='union' and 'rust2' or 'reed2')
end
return M
