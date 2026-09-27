#!/usr/bin/env python3
"""Art v19 review: before/after sheets and walk loops at engine speed.

Compares the archived v18 atlas with the active one. The walk loops follow
the engine exactly: the sim's Chebyshev movement at each machine's spec
speed (bw_sim navigation.rs `move_towards`, `speed_per_tick`), the
desktop's walk phase, one per quarter cell of Manhattan travel
(bw_desktop game.rs `walking_phase`), the camera's (16,8) cell projection
(bw_core `Camera::project`), and one frame per 30 Hz tick, with the camera
on the machine so the ground scrolls under it. v18 plays above v19.

    python3 tools/art_v19/review.py [out_dir]   # default output/art-v19/review
"""
import json
import math
import os
import re
import sys

from PIL import Image, ImageDraw, ImageOps

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'output/art-v19/review')
BG = (150, 134, 106, 255)
DARK = (40, 40, 40, 255)
TEXT = (230, 225, 210, 255)
FP, TICK = 256, 30
SPEED = dict(hook=3, riveter=3, bulwark=2, sounder=5, wick=3, skipper=5, reedguard=3, loom=2,
             tidewatch=6, lampwright=6, caulker=3, tender=3, caisson=2, dredger=2, barge=4, lifter=3)
ORIGINALS = ['hook', 'riveter', 'bulwark', 'sounder', 'wick', 'skipper', 'reedguard', 'loom']
ROLES = ['tidewatch', 'caulker', 'caisson', 'lifter', 'lampwright', 'tender', 'dredger', 'barge']

PALETTE = {}
for line in open(os.path.join(ROOT, 'tools/art_v5/painter.lua')):
    for k, v in re.findall(r"(\w+)='([0-9a-f]{6})'", line):
        PALETTE[tuple(int(v[i:i + 2], 16) for i in (0, 2, 4))] = k


def atlas(png, js):
    sprites = json.load(open(os.path.join(ROOT, js)))['sprites']
    image = Image.open(os.path.join(ROOT, png)).convert('RGBA')

    def get(key):
        s = sprites[key]
        return image.crop((s['x'], s['y'], s['x'] + s['w'], s['y'] + s['h']))
    return get


BEFORE = atlas('art/archive/art-v18/game-assets.png', 'art/archive/art-v18/game-assets.json')
AFTER = atlas('art/exports/game-assets.png', 'art/exports/game-assets.json')


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


def sheets():
    rows, labels = [], []
    for n in ORIGINALS:
        rows.append([BEFORE(f'{n}_{f}') for f in (1, 3, 5, 7)] + [None] + [AFTER(f'{n}_{f}') for f in (1, 3, 5, 7)])
        before = set().union(*(colours(BEFORE(f'{n}_{f}')) for f in range(8)))
        after = set().union(*(colours(AFTER(f'{n}_{f}')) for f in range(8)))
        labels.append(f'{n}\n{len(before)} -> {len(after)}')
    grid(rows, labels, 3, ['v18 (face 1)', '3', '5', '7', '', 'v19 (face 1)', '3', '5', '7']).save(
        f'{OUT}/01-original-machines-colour-budget-3x.png')
    grid([[gray(s) if s else None for s in row] for row in rows], [l.split('\n')[0] for l in labels], 2,
         ['v18 gray', '', '', '', '', 'v19 gray']).save(f'{OUT}/02-original-machines-grayscale-2x.png')
    rows, labels = [], []
    for n in ORIGINALS:
        rows += [[BEFORE(f'{n}_{f}') for f in range(8)], [AFTER(f'{n}_{f}') for f in range(8)]]
        labels += [n + ' v18', n + ' v19']
    grid(rows, labels, 1, lw=80).save(f'{OUT}/03-original-machines-all-facings-1x.png')
    for names, name in ((ORIGINALS, '04-walks-original-machines-2x.png'),
                        (ROLES, '05-walks-tier-two-and-transports-2x.png')):
        rows, labels = [], []
        for n in names:
            for f in (0, 1):
                rows.append([BEFORE(f'{n}_{f}_walk_{p}') for p in range(4)] + [None] +
                            [AFTER(f'{n}_{f}_walk_{p}') for p in range(4)])
                labels.append(f'{n} face {f}')
        grid(rows, labels, 2, ['v18 walk 0', '1', '2', '3', '', 'v19 walk 0', '1', '2', '3']).save(f'{OUT}/{name}')
    rows, labels = [], []
    for n in ORIGINALS:
        for f in (1, 5):
            keys = [f'{n}_{f}', f'{n}_{f}_idle_0', f'{n}_{f}_idle_1']
            rows.append([BEFORE(k) for k in keys] + [None] + [AFTER(k) for k in keys])
            labels.append(f'{n} face {f}')
    grid(rows, labels, 2, ['v18 rest', 'idle 0', 'idle 1', '', 'v19 rest', 'idle 0', 'idle 1']).save(
        f'{OUT}/06-idles-original-machines-2x.png')
    states = ['hook_1_gather_1', 'hook_1_loaded', 'hook_1_unload_1', 'riveter_1_fire_0', 'bulwark_1_deploy_2',
              'bulwark_1_deployed_fire_0', 'sounder_1_fire_0', 'wick_1_gather_1', 'wick_1_loaded',
              'skipper_1_fire_0', 'reedguard_1_fire_0', 'loom_1_deploy_2', 'loom_1_pressure_2',
              'loom_1_deployed_fire_0']
    grid([[BEFORE(k) for k in states], [AFTER(k) for k in states]], ['v18', 'v19'], 2,
         [k.split('_', 1)[1] for k in states], lw=40).save(f'{OUT}/07-other-states-recoloured-2x.png')


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
    """(x, y, walk phase) per tick at full speed on dry ground."""
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


def walk_gif(name, ticks=60, scale=2):
    rows = []
    for get in (BEFORE, AFTER):
        row = []
        for face in range(8):
            frames = []
            for x, y, phase in simulate(name, face, ticks):
                sx, sy = project(x, y)
                tile = ground(sx - 32, sy - 50)
                tile.alpha_composite(get(f'{name}_{face}_walk_{phase}'))
                frames.append(tile)
            row.append(frames)
        rows.append(row)
    W, H = 65 * 8 - 1, 14 + 66 * 2
    out = []
    for t in range(ticks):
        im = Image.new('RGBA', (W, H), DARK)
        ImageDraw.Draw(im).text((2, 0), f'{name}  top: v18  bottom: v19  (engine speed, 30 ticks/s, faces 0-7)',
                                fill=TEXT)
        for r, row in enumerate(rows):
            for c, frames in enumerate(row):
                im.paste(frames[t], (c * 65, 14 + r * 66))
        out.append(im.resize((W * scale, H * scale), Image.NEAREST).convert('RGB')
                   .convert('P', palette=Image.ADAPTIVE, colors=128))
    durations = ([33, 33, 34] * (ticks // 3 + 1))[:ticks]
    out[0].save(f'{OUT}/walk-{name}-actual-speed.gif', save_all=True, append_images=out[1:],
                duration=durations, loop=0)


if __name__ == '__main__':
    os.makedirs(OUT, exist_ok=True)
    sheets()
    for n in ORIGINALS + ROLES:
        walk_gif(n)
    print('Review written to', OUT)
