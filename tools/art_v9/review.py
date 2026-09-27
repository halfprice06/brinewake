"""Root's QA compositor; copies authored pixels, never draws runtime art."""
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'output/art-v9'
OUT.mkdir(exist_ok=True)
manifest = json.loads((ROOT/'art/exports/game-assets.json').read_text())
sprites = manifest['sprites']
atlas = Image.open(ROOT/'art/exports/game-assets.png').convert('RGBA')
baseline = Image.open(ROOT/'art/archive/art-v8/game-assets.png').convert('RGBA')
old = json.loads((ROOT/'art/archive/art-v8/game-assets.json').read_text())
assert atlas.crop((0, 0, baseline.width, baseline.height)).tobytes() == baseline.tobytes()
for name, bounds in old['sprites'].items():
    assert sprites[name] == bounds, name

def stamp(image, key, x, y):
    b = sprites[key]
    cut = atlas.crop((b['x'], b['y'], b['x']+b['w'], b['y']+b['h']))
    image.alpha_composite(cut, (x-b['anchor_x'], y-b['anchor_y']))

def ground(size):
    return Image.new('RGBA', size, (157, 143, 118, 255))

names = ['riveter','bulwark','sounder','skipper','reedguard','loom']
for mode, count, delay in [('surge',4,100),('cool',3,200)]:
    frames = []
    for phase in range(count):
        image = ground((768,576)); pen=ImageDraw.Draw(image)
        for row,name in enumerate(names):
            for face in range(8):
                x,y=face*96+48,row*96+76
                stamp(image,f'{name}_{face}',x,y)
                stamp(image,f'pressure_{name}_{face}_{mode}_{phase}',x,y)
                pen.text((face*96+4,row*96+3),f'{name} {face}',fill=(19,31,39))
        frames.append(image.convert('RGB'))
        image.save(OUT/f'{mode}-{phase}.png')
    frames[0].save(OUT/f'{mode}-all-facings.gif',save_all=True,append_images=frames[1:],duration=delay,loop=0)

frames=[]
for phase in range(6):
    image=ground((768,112));pen=ImageDraw.Draw(image)
    for face in range(8):
        stamp(image,f'loom_{face}_pressure_{phase}',face*96+48,85)
        pen.text((face*96+4,4),f'Loom {face}',fill=(19,31,39))
    frames.append(image.convert('RGB'));image.save(OUT/f'loom-{phase}.png')
frames[0].save(OUT/'loom-all-facings.gif',save_all=True,append_images=frames[1:],duration=[130,130,100,130,130,180],loop=0)

image=ground((768,320));pen=ImageDraw.Draw(image)
for row,worker in enumerate(['hook','wick']):
    for face in range(8):
        x,y=face*96+48,row*96+80
        stamp(image,f'{worker}_{face}_loaded',x,y)
        stamp(image,f'doctrine_{worker}_{face}_hauling_0',x,y)
        pen.text((face*96+4,row*96+5),f'{worker} {face}',fill=(19,31,39))
for i,(faction,doctrine) in enumerate([(f,d) for f in ['union','assembly'] for d in ['hauling','fire_control']]):
    x=i*192+95;y=313
    stamp(image,f'{faction}_hq_active_0',x,y)
    stamp(image,f'doctrine_{faction}_{doctrine}_hq_0',x,y)
image.save(OUT/'doctrines.png')
colors=len({p for p in atlas.getdata() if p[3]})
report={'old_entries_preserved':len(old['sprites']),'new_entries':len(sprites)-len(old['sprites']),
        'atlas_dimensions':atlas.size,'opaque_colors':colors,
        'original_atlas_region_sha256':hashlib.sha256(baseline.tobytes()).hexdigest(),
        'old_pixels_rectangles_anchors_exact':True,
        'preview_method':'Existing sprite pixels composed for QA; GIF timing rounded to centiseconds.'}
(OUT/'art-integrity.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
