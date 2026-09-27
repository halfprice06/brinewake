-- BRINEWAKE original pixel study, authored with Aseprite Lua drawing commands.
-- Presentation art only. This is not game or engine implementation.
local root = app.params['root']
assert(root and #root > 0, 'Pass --script-param root=<project directory>')
local W,H = 640,360
local s = Sprite(W,H,ColorMode.RGB)
local colors = {
 ink='17242e', dark='243640', slate='36505a', steel='54717a', mist='89a0a1',
 water='244b59', sea='326470', ripple='53858c', foam='90b4af',
 mud='675c50', soil='8d7a60', sand='b3a080', salt='d1c3a1', cream='eee0b4',
 rust='714838', red='a55e3e', orange='d38c4d', gold='f0ba68',
 plum='443d52', vine='3c6964', jade='619b87', mint='a0c7a0',
 amber='dd9b53', glow='f7d891', white='f2edd5', hp='88bc87', alert='da7760'
}
local C={}
for name,h in pairs(colors) do C[name]=app.pixelColor.rgba(tonumber(h:sub(1,2),16),tonumber(h:sub(3,4),16),tonumber(h:sub(5,6),16),255) end
local im
local function layer(name)
 local l
 if s.layers[1].name=='Layer 1' then l=s.layers[1] else l=s:newLayer() end
 l.name=name; im=Image(W,H,ColorMode.RGB); im:clear()
 local cel=s:newCel(l,1,im,Point(0,0)); im=cel.image
 return l
end
local function px(x,y,c) x=math.floor(x);y=math.floor(y);if x>=0 and y>=0 and x<W and y<H then im:drawPixel(x,y,C[c]) end end
local function rect(x,y,w,h,c) for yy=y,y+h-1 do for xx=x,x+w-1 do px(xx,yy,c) end end end
local function line(x0,y0,x1,y1,c)
 local dx,dy=math.abs(x1-x0),-math.abs(y1-y0); local sx=x0<x1 and 1 or -1; local sy=y0<y1 and 1 or -1;local e=dx+dy
 while true do px(x0,y0,c);if x0==x1 and y0==y1 then break end;local e2=e*2;if e2>=dy then e=e+dy;x0=x0+sx end;if e2<=dx then e=e+dx;y0=y0+sy end end
end
local function poly(points,c)
 local ymin,ymax=H,0
 for _,p in ipairs(points) do ymin=math.min(ymin,p[2]);ymax=math.max(ymax,p[2]) end
 for y=math.max(0,ymin),math.min(H-1,ymax) do
  local nodes={};local j=#points
  for i=1,#points do local a,b=points[i],points[j];if (a[2]<=y and b[2]>y) or (b[2]<=y and a[2]>y) then nodes[#nodes+1]=a[1]+(y-a[2])/(b[2]-a[2])*(b[1]-a[1]) end;j=i end
  table.sort(nodes);for i=1,#nodes-1,2 do for x=math.ceil(nodes[i]),math.ceil(nodes[i+1])-1 do px(x,y,c) end end
 end
end
local function oval(cx,cy,rx,ry,c)
 for yy=-ry,ry do local span=math.floor(rx*math.sqrt(math.max(0,1-yy*yy/(ry*ry))));rect(cx-span,cy+yy,span*2+1,1,c) end
end
local function box(x,y,w,d,h,top,left,right)
 poly({{x,y-h},{x+w,y+d-h},{x,y+2*d-h},{x-w,y+d-h}},top)
 poly({{x-w,y+d-h},{x,y+2*d-h},{x,y+2*d},{x-w,y+d}},left)
 poly({{x,y+2*d-h},{x+w,y+d-h},{x+w,y+d},{x,y+2*d}},right)
 line(x-w,y+d-h,x,y-h,'cream');line(x,y-h,x+w,y+d-h,top)
end
local glyph={
 A={'01110','10001','10001','11111','10001','10001','10001'}, B={'11110','10001','10001','11110','10001','10001','11110'},
 C={'01111','10000','10000','10000','10000','10000','01111'}, D={'11110','10001','10001','10001','10001','10001','11110'},
 E={'11111','10000','10000','11110','10000','10000','11111'}, F={'11111','10000','10000','11110','10000','10000','10000'},
 G={'01111','10000','10000','10111','10001','10001','01111'}, H={'10001','10001','10001','11111','10001','10001','10001'},
 I={'11111','00100','00100','00100','00100','00100','11111'}, J={'00111','00010','00010','00010','10010','10010','01100'},
 K={'10001','10010','10100','11000','10100','10010','10001'}, L={'10000','10000','10000','10000','10000','10000','11111'},
 M={'10001','11011','10101','10101','10001','10001','10001'}, N={'10001','11001','10101','10011','10001','10001','10001'},
 O={'01110','10001','10001','10001','10001','10001','01110'}, P={'11110','10001','10001','11110','10000','10000','10000'},
 Q={'01110','10001','10001','10001','10101','10010','01101'}, R={'11110','10001','10001','11110','10100','10010','10001'},
 S={'01111','10000','10000','01110','00001','00001','11110'}, T={'11111','00100','00100','00100','00100','00100','00100'},
 U={'10001','10001','10001','10001','10001','10001','01110'}, V={'10001','10001','10001','10001','10001','01010','00100'},
 W={'10001','10001','10001','10101','10101','10101','01010'}, X={'10001','10001','01010','00100','01010','10001','10001'},
 Y={'10001','10001','01010','00100','00100','00100','00100'}, Z={'11111','00001','00010','00100','01000','10000','11111'},
 ['0']={'01110','10001','10011','10101','11001','10001','01110'}, ['1']={'00100','01100','00100','00100','00100','00100','01110'},
 ['2']={'01110','10001','00001','00010','00100','01000','11111'}, ['3']={'11110','00001','00001','01110','00001','00001','11110'},
 ['4']={'00010','00110','01010','10010','11111','00010','00010'}, ['5']={'11111','10000','10000','11110','00001','00001','11110'},
 ['6']={'01110','10000','10000','11110','10001','10001','01110'}, ['7']={'11111','00001','00010','00100','01000','01000','01000'},
 ['8']={'01110','10001','10001','01110','10001','10001','01110'}, ['9']={'01110','10001','10001','01111','00001','00001','01110'},
 [':']={'00000','00100','00100','00000','00100','00100','00000'}, ['/']={'00001','00001','00010','00100','01000','10000','10000'},
 ['-']={'00000','00000','00000','11111','00000','00000','00000'}, ['.']={'00000','00000','00000','00000','00000','00100','00100'}
}
local function text(t,x,y,c)
 for i=1,#t do local g=glyph[t:sub(i,i)];if g then for row=1,7 do for col=1,5 do if g[row]:sub(col,col)=='1' then px(x+(i-1)*6+col-1,y+row-1,c) end end end end end
end

layer('01 salt flat / quiet terrain');rect(0,0,W,H,'sand')
poly({{0,0},{276,0},{330,62},{264,98},{290,132},{249,170},{294,215},{248,288},{0,288}},'soil')
poly({{0,26},{165,26},{210,65},{160,91},{175,116},{97,155},{0,131}},'sand')
poly({{390,26},{640,26},{640,170},{580,155},{530,133},{464,126}},'salt')
poly({{430,227},{512,194},{640,207},{640,288},{375,288}},'soil')
-- Intentional sparse surface clusters follow shallow contours; combat corridors stay quiet.
for i=0,26 do local x=18+(i*83)%620;local y=35+(i*47)%239
 line(x,y,x+10,y,'soil');line(x+10,y,x+14,y-2,'soil');line(x+20,y+6,x+27,y+6,'salt') end
layer('02 tidal channel / shore depth')
local channel={{269,20},{366,20},{347,55},{396,77},{394,106},{352,129},{402,155},{404,185},{353,206},{372,228},{345,288},{242,288},{283,232},{269,209},{316,178},{314,160},{283,143},{300,106},{324,87},{280,60}}
-- Set explicit closing bank point; no simulation or procedural water solver.
channel[#channel]={280,60}
poly(channel,'mud');for _,p in ipairs(channel) do p[1]=p[1]+7 end;poly(channel,'water')
poly({{310,24},{350,24},{332,57},{377,83},{367,110},{328,129},{386,165},{379,184},{329,206},{350,233},{321,286},{288,286},{312,232},{304,209},{347,177},{341,156},{307,141},{324,104},{347,87},{305,61}},'sea')
for i=0,25 do local y=34+i*9;local x=328+math.floor(math.sin(i*0.72)*27);line(x-5,y,x+7,y,'ripple');line(x+16,y+3,x+21,y+3,'ripple') end
layer('03 dry ford / gate platform')
poly({{256,168},{279,153},{431,180},{410,196}},'mud')
poly({{256,164},{279,153},{431,174},{410,188}},'salt')
for i=0,9 do line(275+i*13,163+i,282+i*13,159+i,'sand') end
-- Sluice wall deliberately split into structural supports and deck.
box(330,118,72,36,9,'steel','dark','slate')
for i=0,4 do box(279+i*25,137+i*12,6,3,22,'mist','slate','dark') end
line(263,127,336,163,'mist');line(336,163,403,130,'steel')
for i=0,3 do rect(298+i*4,137+i*2,2,8,'rust') end
box(334,135,10,5,35,'cream','steel','slate');rect(329,110,5,7,'gold')
rect(341,98,2,12,'ink');line(342,99,352,104,'alert');line(342,103,350,107,'alert')
layer('04 environment / salvage and reed silhouettes')
for _,p in ipairs({{39,207},{58,218},{83,226},{455,235},{569,162},{601,171},{192,66},{440,64}}) do
 local x,y=p[1],p[2];poly({{x-7,y},{x-3,y-6},{x+5,y-7},{x+10,y-2},{x+6,y+3},{x-5,y+3}},'mud');line(x-4,y-4,x+4,y-5,'sand') end
for _,p in ipairs({{289,57},{398,102},{379,223},{257,256},{495,47},{597,227},{424,157}}) do
 local x,y=p[1],p[2];line(x,y,x-3,y-12,'vine');line(x+3,y,x+5,y-15,'vine');line(x-2,y-7,x-7,y-11,'jade');line(x+4,y-9,x+9,y-12,'jade') end

local function hook(x,y)
 oval(x,y+2,13,5,'dark');box(x,y-1,10,5,6,'orange','rust','red');box(x-3,y-6,6,3,8,'cream','orange','red')
 line(x-4,y-12,x,y-10,'water');rect(x+6,y-24,3,17,'ink');line(x+7,y-24,x+21,y-17,'gold');line(x+6,y-25,x+20,y-18,'orange')
 line(x+20,y-17,x+20,y-7,'ink');line(x+20,y-7,x+17,y-6,'steel');line(x+17,y-6,x+15,y-8,'steel')
 rect(x-10,y+1,3,4,'ink');rect(x+5,y+4,5,3,'ink');line(x-4,y-2,x+1,y,'gold')
end
local function riveter(x,y)
 oval(x,y+3,14,6,'dark');box(x,y,10,5,7,'orange','red','rust')
 oval(x-9,y+4,5,4,'ink');oval(x-9,y+3,3,2,'steel');oval(x+8,y+6,5,4,'ink');oval(x+8,y+5,3,2,'steel')
 box(x-1,y-6,7,4,7,'cream','orange','red');line(x+4,y-8,x+17,y-3,'ink');line(x+4,y-9,x+17,y-4,'steel');rect(x+15,y-6,5,5,'ink')
 line(x-6,y-12,x-2,y-10,'water');line(x-2,y-11,x+2,y-9,'foam');rect(x-6,y-5,2,2,'gold')
end
local function bulwark(x,y)
 oval(x,y+5,21,8,'dark');box(x,y+1,16,8,10,'orange','red','rust');box(x-6,y-7,8,4,10,'cream','orange','rust')
 for i=-1,1,2 do rect(x+i*14-2,y+4,6,7,'ink');rect(x+i*14-1,y+4,4,2,'steel') end
 poly({{x+4,y-15},{x+24,y-5},{x+24,y+11},{x+4,y+1}},'dark');poly({{x+6,y-13},{x+22,y-5},{x+22,y+7},{x+6,y-1}},'steel')
 line(x+7,y-11,x+20,y-5,'cream');line(x+14,y-8,x+14,y+2,'ink');rect(x+16,y-1,3,3,'gold');line(x-11,y-13,x-6,y-11,'water')
end
local function wick(x,y)
 oval(x,y+5,13,5,'dark');for _,p in ipairs({{-11,2,-5,-8},{10,5,7,-6},{0,9,1,-3}}) do line(x+p[1],y+p[2],x+p[3],y+p[4],'plum');line(x+p[1]+1,y+p[2],x+p[3]+1,y+p[4],'steel') end
 oval(x,y-7,9,5,'vine');oval(x,y-9,8,4,'jade');oval(x+2,y-11,5,3,'mint');oval(x+3,y-12,3,2,'ink')
 line(x-7,y-10,x-10,y-17,'mint');line(x-10,y-17,x-16,y-14,'jade');rect(x-17,y-14,2,7,'amber');rect(x-4,y-9,2,3,'gold')
end
local function skipper(x,y)
 oval(x,y+4,17,5,'dark');line(x-11,y-3,x-17,y+6,'plum');line(x+8,y,x+17,y+7,'plum');line(x-17,y+6,x-12,y+7,'mint');line(x+17,y+7,x+21,y+5,'mint')
 poly({{x-15,y-9},{x-5,y-15},{x+8,y-11},{x+17,y-1},{x+3,y+3},{x-11,y-2}},'plum')
 poly({{x-13,y-9},{x-4,y-13},{x+7,y-9},{x+14,y-2},{x+3,y},{x-10,y-3}},'jade')
 poly({{x-12,y-9},{x-4,y-13},{x+7,y-9},{x+3,y-6}},'mint');line(x-7,y-10,x+5,y-4,'vine');line(x-2,y-12,x+10,y-5,'vine');box(x+1,y-8,4,2,5,'cream','amber','rust')
end
local function loom(x,y)
 oval(x,y+6,20,7,'dark');for _,p in ipairs({{-17,5,-10,-15},{14,7,10,-15},{-1,13,0,-7}}) do
 line(x+p[1],y+p[2],x+p[3],y+p[4],'plum');line(x+p[1]+1,y+p[2],x+p[3]+1,y+p[4],'steel');rect(x+p[1]-2,y+p[2],6,2,'mint') end
 poly({{x-15,y-11},{x-10,y-27},{x-1,y-33},{x+12,y-23},{x+17,y-8},{x+12,y-7},{x+7,y-22},{x-1,y-27},{x-7,y-22},{x-10,y-9}},'jade')
 line(x-10,y-28,x-1,y-33,'mint');line(x-1,y-33,x+11,y-27,'mint');line(x,y-27,x,y-18,'plum')
 oval(x,y-11,7,9,'rust');oval(x-1,y-13,5,7,'amber');rect(x-3,y-18,3,5,'glow');line(x+3,y-18,x+5,y-13,'gold');line(x+5,y-12,x+15,y-9,'steel')
end

layer('05 Breakwater / dockhouse and gantry')
oval(115,121,67,24,'mud')
box(105,101,50,25,13,'steel','dark','slate');box(84,91,25,12,24,'cream','orange','rust');box(126,107,29,14,19,'sand','orange','red')
box(72,112,15,8,11,'orange','rust','red');box(137,94,18,9,14,'cream','red','rust')
for i=0,4 do line(107+i*7,101+i*3,115+i*7,97+i*3,'slate') end
line(64,92,94,107,'gold');line(65,94,94,109,'rust');rect(79,78,11,5,'water');rect(81,78,6,2,'foam')
rect(103,49,4,46,'rust');rect(106,49,2,46,'orange');line(105,49,151,71,'ink');line(105,46,152,69,'gold');line(106,50,151,72,'orange')
line(108,48,122,69,'red');line(122,55,123,70,'red');line(123,70,137,63,'red');line(138,64,150,81,'red');line(151,70,151,91,'ink');line(151,91,145,94,'steel')
rect(62,78,5,24,'dark');rect(62,77,6,3,'steel');rect(126,72,4,21,'dark');rect(125,71,6,3,'mist')
layer('06 Silt / woven workshop arches')
oval(525,100,60,19,'soil')
box(526,83,43,22,4,'slate','vine','dark')
for k=0,1 do local x=497+k*42;local y=77+k*7
 poly({{x-20,y+12},{x-17,y-18},{x-3,y-34},{x+7,y-30},{x+23,y-7},{x+26,y+25},{x+20,y+24},{x+16,y-4},{x+4,y-22},{x-2,y-24},{x-10,y-12},{x-13,y+15}},'vine')
 poly({{x-20,y+8},{x-17,y-18},{x-3,y-34},{x+7,y-30},{x+23,y-7},{x+22,y+1},{x+4,y-22},{x-2,y-25},{x-11,y-13},{x-14,y+10}},'jade')
 line(x-17,y-19,x-3,y-34,'mint');line(x-3,y-34,x+7,y-29,'cream');line(x-11,y-14,x-2,y-26,'mint')
 for j=0,3 do line(x-17+j*2,y-15-j*4,x-11+j*2,y-12-j*4,'vine') end
 rect(x-15,y+13,3,8,'plum');rect(x+20,y+20,3,7,'plum')
end
oval(523,94,8,11,'rust');oval(522,91,7,9,'amber');rect(519,85,3,6,'glow');box(551,103,12,6,7,'mint','jade','vine')
layer('07 Union / Hook worker');hook(177,130)
layer('08 Union / Riveter group');riveter(231,185);riveter(257,202);riveter(219,214);riveter(285,215)
layer('09 Union / Bulwark barrier');bulwark(293,168)
layer('10 Silt / Wick worker');wick(469,106)
layer('11 Silt / Skipper flank');skipper(437,156);skipper(461,174);skipper(407,204)
layer('12 Silt / Loom support');loom(492,202)

layer('13 interaction / selection and orders')
for _,p in ipairs({{231,193},{257,210},{219,222},{285,223}}) do
 local x,y=p[1],p[2];line(x-17,y,x-8,y+5,'hp');line(x-8,y+5,x+8,y+5,'hp');line(x+8,y+5,x+17,y,'hp');rect(x-10,y-39,21,3,'ink');rect(x-9,y-38,17,1,'hp') end
for i=0,7 do line(310+i*7,210-i*3,313+i*7,209-i*3,'gold') end
line(364,184,374,189,'gold');line(374,189,364,194,'gold');line(364,194,354,189,'gold');line(354,189,364,184,'gold')

layer('14 HUD / editable layout and pixel type')
rect(0,0,640,24,'ink');rect(0,23,640,1,'steel');text('BRINEWAKE',12,8,'cream');text('SALVAGE 420',200,8,'gold');text('PRESSURE 80',302,8,'foam');text('CREW 24/60',414,8,'mint');text('06:42',582,8,'white')
rect(258,28,145,19,'ink');text('SLUICE: DRAINED',271,34,'cream')
rect(0,288,640,72,'ink');rect(0,288,640,1,'steel');rect(9,297,112,55,'slate');rect(12,300,106,49,'soil')
poly({{55,300},{75,300},{71,314},{81,322},{68,333},{76,349},{54,349},{59,337},{50,328},{62,315}},'sea')
rect(23,311,7,5,'orange');rect(96,306,7,5,'mint');rect(45,322,33,18,'cream');rect(46,323,31,16,'soil');line(61,323,61,338,'sea')
text('4 RIVETERS',135,300,'cream');text('MOBILE LINE FIGHTERS',135,313,'mist');text('FOCUS FIRE / HOLD FORD',135,337,'gold')
for i=0,3 do rect(338+i*24,299,20,21,'slate');rect(341+i*24,304,14,9,'orange');rect(344+i*24,302,8,6,'cream');rect(341+i*24,316,14,1,'hp') end
text('MOVE',449,301,'white');text('ATTACK',507,301,'white');text('STOP',582,301,'white');text('M',459,315,'gold');text('A',522,315,'gold');text('S',591,315,'gold')
text('ASEPRITE STUDY - STATIC MOCKUP',448,344,'mist')

s:saveAs(root..'/art/source/battlefield-pixel-study.aseprite')
local composite=Image(s);composite:saveAs(root..'/art/exports/battlefield-native.png')
-- Actual used-color audit, including flattened output.
local used={};for it in composite:pixels() do used[it()]=true end;local count=0;for _ in pairs(used) do count=count+1 end
print('Saved '..W..'x'..H..' source; layers='..#s.layers..'; used colors='..count)
