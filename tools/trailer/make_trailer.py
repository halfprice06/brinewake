#!/usr/bin/env python3
"""Cut BRINEWAKE's trailer from engine footage.

1. `brinewake --trailer-capture tools/trailer/shots.json output/trailer/clips`
   renders the shots (the practice AI and staged fights in the real engine).
2. This script cuts them to the soundtrack's bar grid, draws the motion
   graphics in the game's own 7x9 console face and palette, renders the
   soundtrack through the game's mixer (`brinewake --trailer-audio`) from the
   game's music and effects, and encodes output/trailer/BRINEWAKE-trailer.mp4.

Usage: tools/trailer/make_trailer.py [--preview] [--start S] [--end S]
  --preview renders at half size, faster, for checking the cut.
"""

import json
import math
import os
import re
import subprocess
import sys

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
CLIPS = os.path.join(ROOT, "output", "trailer", "clips")
OUT = os.path.join(ROOT, "output", "trailer")
GAME = os.path.join(ROOT, "target", "release", "brinewake")
FPS = 30
W, H = 1920, 1080
BPM = 112
BAR = 240 / BPM
BEAT = 60 / BPM
DURATION = 64.5

# The game's palette.
INK = (19, 31, 39)
PANEL = (28, 45, 54)
EDGE = (71, 96, 102)
WHITE = (239, 228, 197)
MUTED = (151, 169, 167)
GOLD = (236, 177, 96)
JADE = (127, 194, 164)
RED = (226, 108, 84)
COBALT = (120, 150, 236)
VIOLET = (178, 138, 236)
WATER = (40, 78, 88)


def bar(n, beats=0.0):
    """The time of bar n (1-based) plus some beats."""
    return (n - 1) * BAR + beats * BEAT


def smooth(t):
    t = min(1.0, max(0.0, t))
    return t * t * (3 - 2 * t)


def ease_out(t):
    t = min(1.0, max(0.0, t))
    return 1 - (1 - t) ** 3


# ---------------------------------------------------------------- the face

def load_glyphs():
    src = open(os.path.join(ROOT, "crates", "bw_desktop", "src", "font7.rs")).read()
    glyphs = {}
    for m in re.finditer(r"'(\\'|[^'])' => \[([0-9, ]+)\]", src):
        glyphs[m.group(1).replace("\\'", "'")] = [int(x) for x in m.group(2).split(",")]
    glyphs['"'] = [54, 54, 108, 0, 0, 0, 0, 0, 0]
    return glyphs


GLYPHS = load_glyphs()


def text_mask(s, scale):
    """The string in the 7x9 face as a boolean array, one font pixel per
    scale x scale block, advance 8."""
    s = s.upper()
    w = max(1, len(s) * 8 - 1)
    mask = np.zeros((9, w), dtype=bool)
    for i, ch in enumerate(s):
        rows = GLYPHS.get(ch, [0] * 9)
        for r, bits in enumerate(rows):
            for c in range(7):
                if bits & (1 << (6 - c)):
                    mask[r, i * 8 + c] = True
    return np.kron(mask, np.ones((scale, scale), dtype=bool))


_cache = {}


def text_image(s, scale, color, shadow=INK, shadow_px=None):
    """RGBA text with a hard drop shadow one font pixel down-right."""
    key = (s, scale, color, shadow, shadow_px)
    if key in _cache:
        return _cache[key]
    mask = text_mask(s, scale)
    d = scale if shadow_px is None else shadow_px
    h, w = mask.shape
    img = np.zeros((h + d, w + d, 4), dtype=np.uint8)
    if shadow is not None:
        img[d:, d:, :3][mask] = shadow
        img[d:, d:, 3][mask] = 220
    img[:h, :w, :3][mask] = color
    img[:h, :w, 3][mask] = 255
    _cache[key] = img
    return img


def blit(frame, img, x, y, alpha=1.0):
    """Alpha-blend an RGBA image onto the frame at (x, y), clipped."""
    if alpha <= 0:
        return
    h, w = img.shape[:2]
    x0, y0 = max(0, x), max(0, y)
    x1, y1 = min(frame.shape[1], x + w), min(frame.shape[0], y + h)
    if x1 <= x0 or y1 <= y0:
        return
    sub = img[y0 - y : y1 - y, x0 - x : x1 - x]
    a = sub[:, :, 3:4].astype(np.float32) / 255.0 * alpha
    region = frame[y0:y1, x0:x1].astype(np.float32)
    frame[y0:y1, x0:x1] = (region * (1 - a) + sub[:, :, :3].astype(np.float32) * a).astype(np.uint8)


def rect(frame, x, y, w, h, color, alpha=1.0):
    x0, y0 = max(0, int(x)), max(0, int(y))
    x1, y1 = min(frame.shape[1], int(x + w)), min(frame.shape[0], int(y + h))
    if x1 <= x0 or y1 <= y0:
        return
    if alpha >= 1:
        frame[y0:y1, x0:x1] = color
    else:
        region = frame[y0:y1, x0:x1].astype(np.float32)
        frame[y0:y1, x0:x1] = (region * (1 - alpha) + np.array(color, np.float32) * alpha).astype(np.uint8)


def emblem(name, scale):
    atlas = json.load(open(os.path.join(ROOT, "art", "exports", "game-assets.json")))["sprites"]
    s = atlas[f"faction_{name}_emblem"]
    img = Image.open(os.path.join(ROOT, "art", "exports", "game-assets.png")).convert("RGBA")
    e = img.crop((s["x"], s["y"], s["x"] + s["w"], s["y"] + s["h"]))
    e = e.resize((s["w"] * scale, s["h"] * scale), Image.NEAREST)
    return np.array(e)


# ---------------------------------------------------------------- the cut

# (start, end, clip, in-point seconds)
SEGMENTS = [
    # A: the sea went out (bars 1-4), calm.
    (bar(1, 2), bar(2, 4), "a_flat", 0.3),
    (bar(3), bar(5), "a_workers", 0.2),
    # B: build (bars 5-8), working.
    (bar(5), bar(6), "b_build", 0.4),
    (bar(6), bar(7), "b_salvage", 0.4),
    (bar(7), bar(8), "b_train", 0.4),
    (bar(8), bar(9), "b_march", 0.3),
    # C: the tide (bars 9-12), tension.
    (bar(9), bar(10), "c_sluice", 0.2),
    (bar(10), bar(11), "c_tide", 0.33),
    (bar(11), bar(12), "c_cross", 0.3),
    (bar(12), bar(13), "c_drown", 2.2),
    # D: battle (bars 13-20).
    (bar(13), bar(14), "d_clash", 3.0),
    (bar(14), bar(14, 2), "d_loom", 1.0),
    (bar(14, 2), bar(15), "d_assault", 5.0),
    (bar(15), bar(16), "d_union", 1.2),
    (bar(16), bar(17), "d_assembly", 1.2),
    (bar(17), bar(18), "d_compact", 2.6),
] + [
    (bar(18, k), bar(18, k + 1), clip, cin)
    for k, (clip, cin) in enumerate(
        [("d_clash", 5.0), ("d_assault", 7.0), ("d_compact", 5.2), ("d_clash", 6.6),
         ("d_loom", 2.4), ("d_assault", 8.8), ("d_confluence", 5.5), ("d_clash", 8.2)]
    )
] + [
    (bar(20), bar(21), "d_confluence", 3.0),
    # E: hold both crossings (bars 21-24).
    (bar(21), bar(22, 2), "e_hold", 0.5),
    (bar(22, 2), bar(23, 0), "e_hold", 7.8),
    (bar(23, 0), bar(23, 2), "d_clash", 1.0),
    (bar(23, 2), bar(24, 0), "e_hold", 9.6),
    (bar(24, 0), bar(24, 2), "d_union", 2.6),
    (bar(24, 2), bar(25, 0), "d_clash", 9.0),
    # F: the title, over the evening.
    (52.0, DURATION, "f_evening", 0.0),
]

# Moments that hit: (time, flash strength, shake pixels)
HITS = [
    (bar(10, 2), 0.55, 6),     # the tide turns
    (bar(12, 1), 0.45, 5),     # the flood
    (bar(13), 0.9, 12),        # the battle
    (bar(15), 0.25, 4),
    (bar(16), 0.25, 4),
    (bar(17), 0.25, 4),
    (bar(25), 0.0, 0),
]

# Tide wipes into these section starts.
WIPES = [bar(5), bar(9), bar(21)]


def letterbox(t):
    """Bar height: widescreen until the battle, then the full frame."""
    full = 132
    if t < bar(13):
        return full
    return int(full * (1 - ease_out((t - bar(13)) / 0.3)))


# ---------------------------------------------------------------- overlays

def caption(frame, t, t0, t1, s, scale=4, color=WHITE):
    """Typed into the lower bar, a gold cursor while typing."""
    if not (t0 <= t < t1):
        return
    n = min(len(s), int((t - t0) * 28))
    fade = min(1.0, (t1 - t) / 0.25)
    shown = s[:n]
    full = text_image(s, scale, color)
    x = (W - full.shape[1]) // 2
    y = H - 132 + (132 - 9 * scale) // 2 - 4
    if shown:
        blit(frame, text_image(shown, scale, color), x, y, fade)
    if n < len(s) and int(t * 8) % 2 == 0:
        cx = x + len(shown) * 8 * scale
        rect(frame, cx, y, 7 * scale, 9 * scale, GOLD, fade)


def slam(frame, t, t0, t1, s, scale=12, color=WHITE, x=150, y=360, rule=True, sub=None):
    """A word that lands: two integer steps down in size, a gold rule under
    it that draws out, and it wipes off to the left at the end."""
    if not (t0 <= t < t1):
        return
    k = int((t - t0) * FPS)
    sc = scale + (4 if k < 2 else 2 if k < 4 else 0)
    img = text_image(s, sc, color)
    dx = int((W * 0.7) * smooth((t - (t1 - 0.2)) / 0.2)) if t > t1 - 0.2 else 0
    # A game panel behind the words: ink, with a gold edge on top.
    base = text_image(s, scale, color)
    pw = base.shape[1] + scale * 6
    ph = 9 * scale + scale * (8 if sub else 5)
    grow = ease_out((t - t0) / 0.12)
    rect(frame, x - scale * 3 - dx, y - scale * 2, int(pw * grow), ph, INK, 0.72)
    rect(frame, x - scale * 3 - dx, y - scale * 2, int(pw * grow), max(3, scale // 3), GOLD)
    oy = (9 * sc - 9 * scale) // 2
    blit(frame, img, x - dx - (img.shape[1] - text_image(s, scale, color).shape[1]) // 2, y - oy)
    if rule:
        grow = ease_out((t - t0 - 0.08) / 0.25)
        rect(frame, x + scale - dx, y + 9 * scale + scale * 2, int(16 * scale * grow), max(4, scale // 2), GOLD)
    if sub:
        blit(frame, text_image(sub, 4, MUTED), x + scale - dx, y + 9 * scale + scale * 4, min(1.0, (t - t0 - 0.2) / 0.2))


FACTIONS = {
    "union": ("BREAKWATER UNION", "THE HEAVY SIDE ON DRY GROUND.", (122, 52, 38), RED),
    "assembly": ("SILT ASSEMBLY", "IT WORKS WITH THE WATER.", (36, 84, 70), JADE),
    "compact": ("SALTGLASS COMPACT", "FAST, FRAGILE, BRIGHT.", (58, 48, 110), COBALT),
}
EMBLEMS = {}


def faction_card(frame, t, t0, t1, key):
    if not (t0 <= t < t1):
        return
    name, sub, dark, accent = FACTIONS[key]
    if key not in EMBLEMS:
        EMBLEMS[key] = emblem(key, 8)
    slide_in = ease_out((t - t0) / 0.2)
    slide_out = smooth((t - (t1 - 0.2)) / 0.2)
    off = int(-W * (1 - slide_in) + W * slide_out)
    y, h = 700, 230
    rect(frame, off, y, W, h, dark, 0.88)
    rect(frame, off, y, W, 6, accent)
    rect(frame, off, y + h - 6, W, 6, accent)
    k = int((t - t0) * FPS)
    em = EMBLEMS[key]
    if k >= 3:
        blit(frame, em, off + 150, y + (h - em.shape[0]) // 2)
        blit(frame, text_image(name, 8, WHITE), off + 400, y + 52)
        blit(frame, text_image(sub, 4, accent), off + 408, y + 52 + 9 * 8 + 28)


def countdown(frame, t, t0, n):
    """ENEMY WINS IN, then the second, popping in on the beat."""
    if not (t0 <= t < t0 + BEAT):
        return
    k = int((t - t0) * FPS)
    frame[:] = (frame.astype(np.float32) * 0.62).astype(np.uint8)
    scale = 36 + (8 if k < 2 else 4 if k < 3 else 0)
    color = RED if n <= 3 else GOLD
    img = text_image(str(n), scale, color, shadow_px=6)
    fade = 1.0 - smooth((t - t0 - BEAT * 0.8) / (BEAT * 0.2))
    blit(frame, img, (W - img.shape[1]) // 2, (H - img.shape[0]) // 2 + 40, fade)
    label = text_image("ENEMY WINS IN", 5, RED)
    blit(frame, label, (W - label.shape[1]) // 2, 170)


def hold_banner(frame, t, t0, t1):
    if not (t0 <= t < t1):
        return
    img = text_image("HOLD BOTH CROSSINGS.", 8, WHITE)
    a = min(1.0, (t - t0) / 0.15) * min(1.0, (t1 - t) / 0.15)
    x = (W - img.shape[1]) // 2
    blit(frame, img, x, 200, a)
    gw = img.shape[1]
    rect(frame, x, 320, gw, 16, PANEL, a)
    rect(frame, x, 320, int(gw * smooth((t - t0) / (t1 - t0))), 16, RED, a)


def dissolve_text(frame, t, t0, dur, s, scale, color, cx, y):
    """The wordmark forming out of the ordered-dither pattern the game uses
    for its fog rim."""
    if t < t0:
        return
    p = min(1.0, (t - t0) / dur)
    mask = text_mask(s, 1)
    bayer = np.array([[0, 32, 8, 40, 2, 34, 10, 42], [48, 16, 56, 24, 50, 18, 58, 26],
                      [12, 44, 4, 36, 14, 46, 6, 38], [60, 28, 52, 20, 62, 30, 54, 22],
                      [3, 35, 11, 43, 1, 33, 9, 41], [51, 19, 59, 27, 49, 17, 57, 25],
                      [15, 47, 7, 39, 13, 45, 5, 37], [63, 31, 55, 23, 61, 29, 53, 21]]) / 64.0
    hh, ww = mask.shape
    thr = np.tile(bayer, (hh // 8 + 1, ww // 8 + 1))[:hh, :ww]
    on = mask & (thr < p)
    img = np.zeros((hh * scale + scale, ww * scale + scale, 4), np.uint8)
    big = np.kron(on, np.ones((scale, scale), bool))
    img[scale:, scale:, :3][big] = INK
    img[scale:, scale:, 3][big] = 230
    img[: hh * scale, : ww * scale, :3][big] = color
    img[: hh * scale, : ww * scale, 3][big] = 255
    blit(frame, img, cx - (ww * scale) // 2, y)


def logo(frame, t, darken=True):
    if t < 52.0:
        return
    if darken:
        frame[:] = (frame.astype(np.float32) * 0.45).astype(np.uint8)
    if t < 52.1:
        return
    scale = 16
    y = 300
    dissolve_text(frame, t, 52.1, 1.0, "BRINEWAKE", scale, WHITE, W // 2, y)
    wm_w = (9 * 8 - 1) * scale
    grow = ease_out((t - 53.1) / 0.5)
    rw = int(wm_w * 0.5 * grow)
    rect(frame, W // 2 - rw, y + 9 * scale + 30, rw * 2, 10, GOLD)
    tag = "THE TIDE DECIDES."
    n = max(0, min(len(tag), int((t - 53.7) * 22)))
    if n:
        img = text_image(tag[:n], 5, GOLD)
        full = text_image(tag, 5, GOLD)
        blit(frame, img, (W - full.shape[1]) // 2, y + 9 * scale + 70)
    for i, (s, sc, col, at) in enumerate([
        ("FREE ON MACOS AND WINDOWS", 5, WHITE, 57.6),
        ("DANPRICE.AI/BRINEWAKE", 6, GOLD, 58.2),
        ("PLAY ONLINE WITH FRIENDS. NO ACCOUNT.", 3, MUTED, 58.8),
    ]):
        if t >= at:
            img = text_image(s, sc, col)
            yy = 770 + i * 80 + int(20 * (1 - ease_out((t - at) / 0.3)))
            blit(frame, img, (W - img.shape[1]) // 2, yy, min(1.0, (t - at) / 0.3))


def overlays(frame, t):
    caption(frame, t, bar(1, 3), bar(2, 4) - 0.1, "THE SEA WENT OUT.")
    caption(frame, t, bar(3, 1), bar(5) - 0.15, "THE MACHINES CAME IN.")
    slam(frame, t, bar(5, 0.1), bar(6), "BUILD.")
    slam(frame, t, bar(6, 0.1), bar(7), "SALVAGE.")
    slam(frame, t, bar(7, 0.1), bar(8), "COMMAND.")
    caption(frame, t, bar(8, 0.5), bar(9) - 0.1, "ON THE FLOOR OF A VANISHED SEA.")
    caption(frame, t, bar(9, 0.5), bar(10) - 0.1, "TAKE THE SLUICE.")
    caption(frame, t, bar(10, 2.1), bar(11) - 0.1, "MOVE THE TIDE.")
    caption(frame, t, bar(11, 0.3), bar(12) - 0.1, "DRY A CROSSING FOR YOUR ARMY.")
    caption(frame, t, bar(12, 1.1), bar(13) - 0.05, "DROWN THE ONE THEY NEED.")
    faction_card(frame, t, bar(15, 0.1), bar(16) - 0.05, "union")
    faction_card(frame, t, bar(16, 0.1), bar(17) - 0.05, "assembly")
    faction_card(frame, t, bar(17, 0.1), bar(18) - 0.05, "compact")
    slam(frame, t, bar(20, 0.1), bar(21), "TWO OR THREE PLAYERS.", scale=7, x=140, y=780, sub="ONLINE. ONE JOIN CODE.")
    hold_banner(frame, t, bar(21, 0.05), bar(22, 2))
    for k in range(10):
        countdown(frame, t, bar(22, 2 + k), 10 - k)
    logo(frame, t)


def tide_wipe(frame, prev, t, t0):
    """A wave front sweeping left to right with a foam crest: the incoming
    shot is already under it."""
    dur = 0.33
    if prev is None or not (t0 <= t < t0 + dur):
        return frame
    p = smooth((t - t0) / dur)
    edge = -200 + (W + 400) * p
    ys = np.arange(H)
    xb = (edge + 40 * np.sin(ys / 70.0 + 3 * p) + 18 * np.sin(ys / 23.0 - 5 * p)).astype(int)
    xs = np.arange(W)[None, :]
    behind = xs >= xb[:, None] + 90
    out = frame.copy()
    out[behind] = prev[behind]
    water = (xs >= xb[:, None]) & (xs < xb[:, None] + 90)
    out[water] = WATER
    crest = (xs >= xb[:, None] - 10) & (xs < xb[:, None] + 4)
    out[crest] = WHITE
    return out


# ---------------------------------------------------------------- footage

class Reader:
    def __init__(self, clip, start, size):
        self.size = size
        path = os.path.join(CLIPS, clip + ".mp4")
        self.proc = subprocess.Popen(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-ss", f"{start:.3f}", "-i", path,
             "-vf", f"scale={size[0]}:{size[1]}:flags=neighbor", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self.last = None

    def frame(self):
        n = self.size[0] * self.size[1] * 3
        data = self.proc.stdout.read(n)
        if len(data) == n:
            self.last = np.frombuffer(data, np.uint8).reshape(self.size[1], self.size[0], 3).copy()
        return self.last

    def close(self):
        self.proc.stdout.close()
        self.proc.wait()


def vignette(size):
    w, h = size
    y, x = np.ogrid[:h, :w]
    d = np.sqrt(((x - w / 2) / (w / 2)) ** 2 + ((y - h / 2) / (h / 2)) ** 2)
    return np.clip(1.0 - 0.22 * np.clip(d - 0.55, 0, None) ** 1.4, 0.7, 1.0).astype(np.float32)[..., None]


# ---------------------------------------------------------------- sound

def soundtrack(path):
    steps = [{"t": 0.0, "ambience": [100, 70, 50]}]
    music = lambda t, **m: steps.append({"t": round(t, 3), "music": m})
    cue = lambda t, name, gain=1.0, pan=0.0: steps.append({"t": round(t, 3), "cue": name, "gain": gain, "pan": pan})
    music(0.02, song="Match", intensity=0)
    music(bar(4, 2.5), song="Match", intensity=1)
    music(bar(8, 2.5), song="Match", intensity=2)
    music(bar(12, 2.5), song="Match", intensity=3)
    # The count: the race from thirty seconds, the last ten on the beat.
    for i in range(20):
        music(bar(20, 3.5) + i * (bar(22, 2) - bar(20, 3.5)) / 20, song="Match", intensity=2, hold_theirs=True, hold_left=30 - i)
    for k in range(10):
        music(bar(22, 2 + k) - 0.02, song="Match", intensity=2, hold_theirs=True, hold_left=10 - k)
        cue(bar(22, 2 + k), "HoldTollEnemy", 0.9)
    music(bar(25), song=None, stop="Quick")
    music(52.0, song="Victory", intensity=0)
    music(63.2, song=None, stop="Gentle")
    # Designed hits.
    cue(1.1, "MatchHorn", 0.8)
    cue(bar(5, 0.1), "BuildPlaced", 0.9)
    cue(bar(6, 0.1), "Reclaim", 0.9)
    cue(bar(7, 0.1), "UnitReady", 0.9)
    cue(bar(9, 0.2), "GateWarningBell", 0.9)
    cue(bar(10, 2), "GateChangeRush", 1.0)
    cue(bar(10, 2), "Splash", 0.8)
    cue(bar(12, 1), "GateChangeRush", 1.0)
    cue(bar(13), "ExplosionLarge", 1.0)
    cue(bar(15, 0.1), "SelectUnion", 1.0)
    cue(bar(16, 0.1), "SelectAssembly", 1.0)
    cue(bar(17, 0.1), "SelectCompact", 1.0)
    cue(bar(21, 0.05), "HoldBegunEnemy", 0.9)
    cue(bar(25), "ExplosionLarge", 1.0)
    cue(bar(25) + 0.05, "DebrisMetal", 0.8)
    cue(52.1, "Glint", 0.7)
    cue(53.1, "UiConfirm", 0.6)
    steps.append({"t": bar(13), "ambience": [60, 55, 40]})
    steps.append({"t": bar(25), "ambience": [0, 0, 0]})
    steps.append({"t": 52.0, "ambience": [70, 60, 40]})
    # The footage's own sounds, where they were made.
    for t0, t1, clip, cin in SEGMENTS:
        meta = json.load(open(os.path.join(CLIPS, clip + ".sounds.json")))
        taken = []
        for s in meta["sounds"]:
            if cin <= s["t"] < cin + (t1 - t0):
                at = t0 + s["t"] - cin
                # A volley thickens without piling up: at most six in any
                # tenth of a second.
                if sum(1 for x in taken if abs(x - at) < 0.1) >= 6:
                    continue
                taken.append(at)
                gain = s["gain"] * (0.35 if s["far"] else 0.7)
                cue(at, s["cue"], round(gain, 3), round(s["pan"], 3))
    timeline = {"duration": DURATION, "levels": [90, 70, 100], "steps": steps}
    tl = os.path.join(OUT, "trailer-audio.json")
    json.dump(timeline, open(tl, "w"), indent=1)
    subprocess.run([GAME, "--trailer-audio", tl, path], check=True)


# ---------------------------------------------------------------- render

def main():
    preview = "--preview" in sys.argv
    start = float(sys.argv[sys.argv.index("--start") + 1]) if "--start" in sys.argv else 0.0
    end = float(sys.argv[sys.argv.index("--end") + 1]) if "--end" in sys.argv else DURATION
    wav = os.path.join(OUT, "trailer-audio.wav")
    if "--no-audio" not in sys.argv:
        soundtrack(wav)
    size = (W, H)
    video = os.path.join(OUT, "BRINEWAKE-trailer-preview.mp4" if preview else "BRINEWAKE-trailer.mp4")
    enc = subprocess.Popen(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
         "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
         "-ss", f"{start:.3f}", "-i", wav,
         "-map", "0:v", "-map", "1:a",
         "-vf", "scale=960:540:flags=neighbor" if preview else "null",
         "-c:v", "libx264", "-preset", "fast" if preview else "slow", "-crf", "20" if preview else "14",
         "-pix_fmt", "yuv420p", "-tune", "animation", "-c:a", "aac", "-b:a", "256k",
         "-shortest", "-movflags", "+faststart", video],
        stdin=subprocess.PIPE,
    )
    vig = vignette(size)
    readers = {}
    prev_last = None
    last_seg = None
    for i in range(int(start * FPS), int(end * FPS)):
        t = i / FPS
        seg = next((s for s in SEGMENTS if s[0] <= t < s[1]), None)
        if seg is not last_seg:
            if last_seg is not None and last_seg in readers:
                prev_last = readers[last_seg].last
                readers.pop(last_seg).close()
            if seg is not None:
                readers[seg] = Reader(seg[2], seg[3] + (t - seg[0]), size)
            last_seg = seg
        frame = readers[seg].frame() if seg else None
        if frame is None:
            frame = np.zeros((H, W, 3), np.uint8)
        frame = frame.copy()
        for t0 in WIPES:
            frame = tide_wipe(frame, prev_last, t, t0)
        # Shake and flash on the hits.
        for at, flash, shake in HITS:
            dt = t - at
            if 0 <= dt < 0.3 and shake:
                amp = shake * (1 - dt / 0.3)
                dx, dy = int(amp * math.sin(i * 2.7)), int(amp * math.cos(i * 3.9))
                frame = np.roll(frame, (dy, dx), axis=(0, 1))
        frame = (frame.astype(np.float32) * vig).astype(np.uint8)
        for at, flash, shake in HITS:
            dt = t - at
            if 0 <= dt < 0.25 and flash:
                a = flash * (1 - dt / 0.25) ** 2
                frame = (frame.astype(np.float32) * (1 - a) + np.array(WHITE, np.float32) * a).astype(np.uint8)
        lb = letterbox(t)
        if lb:
            frame[:lb] = 0
            frame[H - lb :] = 0
        overlays(frame, t)
        # In from black, out to black.
        fade = min(1.0, max(0.0, (t - bar(1, 2)) / 0.8)) * min(1.0, max(0.0, (DURATION - 0.3 - t) / 1.2))
        if bar(25) <= t < 52.0:
            fade = 0.0
        elif 52.0 <= t < 52.6:
            fade = min(fade, (t - 52.0) / 0.6)
        if fade < 1:
            frame = (frame.astype(np.float32) * fade).astype(np.uint8)
            # The logo is drawn over the fade-in.
            if 52.0 <= t < DURATION - 1.5:
                logo(frame, t, darken=False)
        enc.stdin.write(frame.tobytes())
        if i % 150 == 0:
            print(f"{t:5.1f}s", flush=True)
    for r in readers.values():
        r.close()
    enc.stdin.close()
    enc.wait()
    print("wrote", video)


if __name__ == "__main__":
    main()
