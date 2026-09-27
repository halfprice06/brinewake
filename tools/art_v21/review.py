#!/usr/bin/env python3
"""Art v21 review: the Saltglass Compact's machines beside the other two sides.

Sheets of every facing, every state and the damage overlays, a lineup
against Union and Assembly machines in colour and greyscale, and walk loops
at engine speed (the art v19 method: the sim's movement at each machine's
spec speed, the desktop's walk phase per quarter cell of Manhattan travel,
the camera's cell projection, one frame per 30 Hz tick, the ground
scrolling under the machine).

    python3 tools/art_v21/review.py [out_dir]   # default output/art-v21/review
"""
import json
import math
import os
import re
import sys

from PIL import Image, ImageDraw, ImageOps

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'output/art-v21/review')
BG = (150, 134, 106, 255)
DARK = (40, 40, 40, 255)
TEXT = (230, 225, 210, 255)
FP, TICK = 256, 30
SPEED = dict(raker=3, brander=4, heliostat=2, glinter=5, stilt=6, glazier=3, salter=2, pan=2)
COMPACT = ['raker', 'brander', 'heliostat', 'glinter', 'stilt', 'glazier', 'salter', 'pan']
# Counterparts by role, for the lineup: Union, Assembly, Compact.
LINEUP = [('hook', 'wick', 'raker'), ('riveter', 'skipper', 'brander'), ('bulwark', 'loom', 'heliostat'),
          ('sounder', 'reedguard', 'glinter'), ('tidewatch', 'lampwright', 'stilt'),
          ('caulker', 'tender', 'glazier'), ('caisson', 'dredger', 'salter')]

PALETTE = {}
for line in open(os.path.join(ROOT, 'tools/art_v21/painter.lua')):
    for k, v in re.findall(r"(\w+)='([0-9a-f]{6})'", line):
        PALETTE[tuple(int(v[i:i + 2], 16) for i in (0, 2, 4))] = k

SPRITES = json.load(open(os.path.join(ROOT, 'art/exports/game-assets.json')))['sprites']
IMAGE = Image.open(os.path.join(ROOT, 'art/exports/game-assets.png')).convert('RGBA')


def get(key):
    s = SPRITES[key]
    return IMAGE.crop((s['x'], s['y'], s['x'] + s['w'], s['y'] + s['h']))


def colours(im):
    roles = {PALETTE.get(p[:3], '?') for _, p in im.getcolors(im.width * im.height) if p[3] > 0}
    return roles - {'cast', 'contact'}


def grid(rows, labels, scale, head=None, lw=90, cw=64, ch=64, gap=2):
    ncol = max(len(r) for r in rows)
    top = 16 if head else 0
    im = Image.new('RGBA', (lw + ncol * (cw * scale + gap), top + len(rows) * (ch * scale + gap)), DARK)
    d = ImageDraw.Draw(im)
    for i, h in enumerate(head or []):
        if h:
            d.text((lw + i * (cw * scale + gap) + 2, 2), h, fill=TEXT)
    for r, row in enumerate(rows):
        d.text((4, top + r * (ch * scale + gap) + 4), labels[r], fill=TEXT)
        for c, s in enumerate(row):
            if s is None:
                continue
            cell = Image.new('RGBA', (cw, ch), BG)
            cell.alpha_composite(s)
            im.paste(cell.resize((cw * scale, ch * scale), Image.NEAREST),
                     (lw + c * (cw * scale + gap), top + r * (ch * scale + gap)))
    return im


def gray(s):
    flat = Image.alpha_composite(Image.new('RGBA', s.size, BG), s).convert('RGB')
    return ImageOps.grayscale(flat).convert('RGBA')


def over(a, b):
    a = a.copy()
    a.alpha_composite(b)
    return a


def sheets():
    rows, labels = [], []
    for n in COMPACT:
        rows.append([get(f'{n}_{f}') for f in range(8)])
        used = set().union(*(colours(get(f'{n}_{f}')) for f in range(8)))
        labels.append(f'{n}\n{len(used)} colours')
    grid(rows, labels, 3, [f'face {f}' for f in range(8)]).save(f'{OUT}/01-compact-all-facings-3x.png')
    grid(rows, [l.split('\n')[0] for l in labels], 1, lw=70).save(f'{OUT}/02-compact-all-facings-1x.png')
    rows, labels = [], []
    for trio in LINEUP:
        for f in (1, 6):
            rows.append([get(f'{n}_{f}') for n in trio])
            labels.append(f'{trio[2]} face {f}')
    grid(rows, labels, 2, ['Union', 'Assembly', 'Compact']).save(f'{OUT}/03-lineup-by-role-2x.png')
    grid([[gray(s) for s in row] for row in rows], labels, 2, ['Union', 'Assembly', 'Compact']).save(
        f'{OUT}/04-lineup-by-role-grayscale-2x.png')
    rows, labels = [], []
    for n in COMPACT:
        for f in (0, 1, 6):
            rows.append([get(f'{n}_{f}_walk_{p}') for p in range(4)] + [None] +
                        [get(f'{n}_{f}'), get(f'{n}_{f}_idle_0'), get(f'{n}_{f}_idle_1')])
            labels.append(f'{n} face {f}')
    grid(rows, labels, 2, ['walk 0', '1', '2', '3', '', 'rest', 'idle 0', 'idle 1']).save(
        f'{OUT}/05-walks-and-idles-2x.png')
    states = [
        ('brander', ['fire_0', 'fire_1', 'fire_2']),
        ('glinter', ['fire_0', 'fire_1', 'fire_2']),
        ('heliostat', ['deploy_0', 'deploy_1', 'deploy_2', 'deployed_fire_0', 'deployed_fire_1', 'deployed_fire_2']),
        ('pan', ['deploy_0', 'deploy_1', 'deploy_2', 'deployed_0', 'deployed_1', 'deployed_2', 'deployed_3']),
        ('salter', ['lay_0', 'lay_1', 'lay_2']),
        ('raker', ['gather_0', 'gather_1', 'gather_2', 'loaded', 'unload_0', 'unload_1', 'unload_2']),
    ]
    rows, labels, heads = [], [], []
    for n, ks in states:
        for f in (1, 6):
            row = []
            for k in ks:
                key = f'{n}_{f}_{k}'
                s = get(key)
                mz = SPRITES[key].get('muzzle')
                if mz:
                    s = s.copy()
                    x, y = 32 + mz[0], 50 + mz[1]
                    for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                        if 0 <= x + dx < 64 and 0 <= y + dy < 64:
                            s.putpixel((x + dx, y + dy), (255, 0, 255, 255))
                row.append(s)
            rows.append(row)
            labels.append(f'{n} {f}\n' + ' '.join(k.replace('deployed_', 'd').replace('deploy_', 'dep')
                                                  for k in ks)[:40])
    grid(rows, labels, 2, None, lw=150).save(f'{OUT}/06-states-2x-muzzles-magenta.png')
    rows, labels = [], []
    for n in COMPACT:
        rows.append([over(get(f'{n}_{f}'), get(f'{n}_{f}_damage')) for f in range(8)])
        labels.append(n)
    grid(rows, labels, 2, [f'damaged {f}' for f in range(8)]).save(f'{OUT}/07-damage-2x.png')


def other_sheets():
    def canvas(keys, cols, cw, ch, scale, bg=BG):
        rows = (len(keys) + cols - 1) // cols
        im = Image.new('RGBA', (cols * (cw + 4), rows * (ch + 4)), bg)
        for i, k in enumerate(keys):
            s = get(k)
            if k.endswith('_lamps') or '_damage_' in k:
                base = get(k.rsplit('_', 1)[0] if k.endswith('_lamps') else k.rsplit('_damage_', 1)[0])
                s = over(base, s)
            im.alpha_composite(s, ((i % cols) * (cw + 4), (i // cols) * (ch + 4)))
        return im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    big = ['compact_hq', 'compact_works', 'compact_drydock', 'compact_dropoff', 'compact_tower']
    rows = []
    for b in big:
        extra = [k for k in (f'{b}_build_0', f'{b}_build_1', f'{b}_active_0', f'{b}_active_2', f'{b}_lamps',
                             f'{b}_damage_1', f'{b}_damage_2') if k in SPRITES]
        rows += [b] + extra + [None] * (7 - len(extra))
    keys = [k for k in rows]
    im = Image.new('RGBA', (8 * 164, 5 * 148), BG)
    for i, k in enumerate(keys):
        if k:
            s = get(k)
            if k.endswith('_lamps') or '_damage_' in k:
                s = over(get(k.rsplit('_', 1)[0] if k.endswith('_lamps') else k.rsplit('_damage_', 1)[0]), s)
            im.alpha_composite(s, ((i % 8) * 164, (i // 8) * 148))
    im.resize((im.width * 2, im.height * 2), Image.NEAREST).save(f'{OUT}/08-buildings-and-states-2x.png')
    lineup = ['union_hq', 'assembly_hq', 'compact_hq', 'union_works', 'assembly_works', 'compact_works',
              'union_drydock', 'assembly_drydock', 'compact_drydock', 'dropoff', 'tower', 'compact_dropoff',
              'compact_tower']
    canvas(lineup, 3, 160, 144, 1).save(f'{OUT}/09-buildings-beside-the-others-1x.png')
    canvas(['compact_palisade', 'compact_palisade_build_1', 'compact_palisade_damage_2', 'union_palisade',
            'assembly_palisade', 'wreck_compact', 'wreck_union'], 7, 64, 64, 3).save(f'{OUT}/10-palisade-and-wreck-3x.png')
    canvas(['portrait_' + n for n in COMPACT + ['compact', 'union', 'assembly']], 11, 56, 56, 3).save(
        f'{OUT}/11-portraits-3x.png')
    canvas(['faction_compact_badge', 'faction_compact_emblem', 'faction_compact_hq', 'faction_union_badge',
            'faction_union_emblem', 'faction_union_hq'], 6, 24, 24, 4).save(f'{OUT}/12-marks-4x.png')
    canvas(['faction_compact_plate', 'faction_union_plate', 'faction_assembly_plate'], 3, 96, 24, 3).save(
        f'{OUT}/13-plates-3x.png')
    canvas(['hud_top_compact', 'hud_top_union', 'hud_bottom_compact', 'hud_bottom_union'], 1, 640, 72, 1).save(
        f'{OUT}/14-hud-bars-1x.png')
    # The causeway: a strip three cells wide over tidal water, its head row
    # still setting; the beam's flare and hit; a shot on glaze; a wreck.
    scene = Image.new('RGBA', (360, 200), BG)
    for r in range(-5, 6):
        for q in range(-5, 6):
            x, y = 164 + (q - r) * 16, 90 + (q + r) * 8
            if abs(q) <= 1 and r > -5:
                k = 'terrain_crust_%d' % ((r * 7 + q * 3) % 4)
            elif abs(q) <= 1:
                k = 'terrain_crust_fresh_1'
            else:
                k = 'terrain_lane_wet_%d' % ((r + q) % 4)
            scene.alpha_composite(get(k), (x, y))
    salter = get('salter_7_lay_1')
    scene.alpha_composite(salter, (164 + 80 - 32 + 16, 90 - 40 - 50 + 8))
    scene.resize((scene.width * 3, scene.height * 3), Image.NEAREST).save(f'{OUT}/15-causeway-3x.png')
    fx = [f'fx_beam_flare_{k}' for k in range(3)] + [f'fx_beam_hit_{k}' for k in range(4)]
    canvas(fx, 7, 32, 32, 4).save(f'{OUT}/16-beam-flare-and-hit-4x.png')
    canvas([f'fx_impact_glaze_{k}' for k in range(4)], 4, 48, 48, 3).save(f'{OUT}/17-impact-glaze-3x.png')
    canvas([f'fx_wreck_glaze_{k}' for k in range(6)], 6, 64, 64, 3).save(f'{OUT}/18-wreck-glaze-3x.png')


def cell_direction(face):
    co, si = math.cos(-face * math.pi / 4), math.sin(-face * math.pi / 4)
    a, b = co + si, si - co
    m = max(abs(a), abs(b))
    return round(a / m), round(b / m)


def trunc(a, b):
    return int(a / b) if a * b < 0 else a // b


def project(x, y):
    return trunc((x - y) * 16, FP), trunc((x + y) * 8, FP)


def simulate(name, face, ticks):
    a, b = cell_direction(face)
    numerator, denominator = SPEED[name] * FP * 100, TICK * 100
    x = y = FP * 200
    distance, out = 0, []
    for t in range(ticks):
        phase = t % denominator
        step = (phase + 1) * numerator // denominator - phase * numerator // denominator
        nx, ny = x + a * step, y + b * step
        distance += abs(nx - x) + abs(ny - y)
        x, y = nx, ny
        out.append((x, y, (distance // (FP // 4)) % 4))
    return out


def ground(ox, oy, w=64, h=64):
    im = Image.new('RGBA', (w, h), BG)
    px = im.load()
    for sy in range(h):
        for sx in range(w):
            X, Y = sx + ox, sy + oy
            if (X % 16, Y % 8) in ((0, 0), (8, 4)):
                px[sx, sy] = (128, 114, 90, 255)
            elif (X % 16, Y % 8) in ((1, 0), (9, 4)):
                px[sx, sy] = (165, 149, 120, 255)
    return im


def walk_gif(name, suffix='walk', ticks=60, scale=2):
    row = []
    for face in range(8):
        frames = []
        for x, y, phase in simulate(name, face, ticks):
            sx, sy = project(x, y)
            tile = ground(sx - 32, sy - 50)
            tile.alpha_composite(get(f'{name}_{face}_{suffix}_{phase}'))
            frames.append(tile)
        row.append(frames)
    W, H = 65 * 8 - 1, 14 + 64
    out = []
    for t in range(ticks):
        im = Image.new('RGBA', (W, H), DARK)
        ImageDraw.Draw(im).text((2, 0), f'{name} {suffix}  (engine speed, 30 ticks/s, faces 0-7)', fill=TEXT)
        for c, frames in enumerate(row):
            im.paste(frames[t], (c * 65, 14))
        out.append(im.resize((W * scale, H * scale), Image.NEAREST).convert('RGB')
                   .convert('P', palette=Image.ADAPTIVE, colors=128))
    durations = ([33, 33, 34] * (ticks // 3 + 1))[:ticks]
    out[0].save(f'{OUT}/walk-{name}{"" if suffix == "walk" else "-loaded"}-actual-speed.gif', save_all=True,
                append_images=out[1:], duration=durations, loop=0)


if __name__ == '__main__':
    os.makedirs(OUT, exist_ok=True)
    sheets()
    other_sheets()
    for n in COMPACT:
        walk_gif(n)
    walk_gif('raker', 'loaded_walk')
    print('Review written to', OUT)
