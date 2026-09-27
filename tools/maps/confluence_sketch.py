"""The Confluence: a three-seat map sketch, rasterised on the sim grid and
measured with octile path search (8-neighbour, 141/100 diagonal, no corner
cutting), the same metric bw_sim's navigation uses.

Usage: python3 confluence_sketch.py [D_T D_R SHIFT_R]  (defaults 60 62 0,2)
Writes confluence.json (layout and measurements), confluence_tiles.json,
confluence.png (one pixel a cell) and confluence.map to the current directory.
The sketch was drawn for docs/THIRD-FACTION-AND-CONFLUENCE.md; with its
default arguments, confluence.map is the grid the game embeds as
crates/bw_sim/maps/confluence.map, and confluence.json's "engine" entry holds
the worker and practice-AI cells copied into crates/bw_sim/src/maps.rs."""
import math, heapq, json, zlib, struct, sys

N = 176
C = (88.0, 88.0)
COAST = 84.0         # the ring sea beyond this radius
POOL = 15.0          # the central pool
ARM_HALF = 10.0      # arm half-width (the 2p lake is 24 wide)
LANE_S = (19.0, 24.0)  # lane band along each arm (5 wide, as in 2p)
RIM_R = 77.0         # rims: arm water from here to the coast (7 wide)
ISLAND_CORE = 5.0
ISLAND_HALF = 2.5
ISLAND_S = 19.0
SILT_RINGS = (40.0, 58.0)

def unit(deg):
    r = math.radians(deg)
    return (math.cos(r), math.sin(r))

# seat directions (grid angles, y down).  T sits on the mirror axis x == y.
SEATS = {"T": 225.0, "R": 345.0, "L": 105.0}
# arms bisect the seats: E between T and R, W between T and L, S between R and L
ARMS = {"E": 285.0, "W": 165.0, "S": 45.0}

SEA, DEEP, SALT, SILT, ROCK = "sea", "deep", "salt", "silt", "rock"

def on_island(px, py):
    dx, dy = px - C[0], py - C[1]
    if math.hypot(dx, dy) <= ISLAND_CORE:
        return True
    for name, ang in ARMS.items():
        u = unit(ang); n = (-u[1], u[0])
        s = dx * u[0] + dy * u[1]; t = dx * n[0] + dy * n[1]
        if s >= 0 and abs(t) <= ISLAND_HALF and s < ISLAND_S:
            return True
    return False

def classify(px, py):
    dx, dy = px - C[0], py - C[1]
    r = math.hypot(dx, dy)
    if r > COAST:
        return SEA
    for name, ang in ARMS.items():
        u = unit(ang); n = (-u[1], u[0])
        s = dx * u[0] + dy * u[1]; t = dx * n[0] + dy * n[1]
        if s >= 0 and abs(t) <= ISLAND_HALF and s < ISLAND_S:
            return SALT  # island point
    if r <= ISLAND_CORE:
        return SALT
    for name, ang in ARMS.items():
        u = unit(ang); n = (-u[1], u[0])
        s = dx * u[0] + dy * u[1]; t = dx * n[0] + dy * n[1]
        if s >= 0 and abs(t) <= ARM_HALF:
            if LANE_S[0] <= s < LANE_S[1]:
                return "lane" + name
            if r >= RIM_R:
                return "rim" + name
            return DEEP
    if r <= POOL:
        return DEEP
    for ring in SILT_RINGS:
        if ring <= r < ring + 1.0:
            return SILT
    return SALT

MIRROR = {"laneE": "laneW", "laneW": "laneE", "rimE": "rimW", "rimW": "rimE"}
tiles = [[None] * N for _ in range(N)]
for y in range(N):
    for x in range(N):
        if x <= y:
            tiles[y][x] = classify(x + 0.5, y + 0.5)
for y in range(N):
    for x in range(N):
        if x > y:
            t = tiles[x][y]
            tiles[y][x] = MIRROR.get(t, t)

def walkable(t, tide):
    if t in (SEA, DEEP, ROCK):
        return False
    if t.startswith("lane") or t.startswith("rim"):
        arm = t[-1]
        if tide == "neutral":
            return True
        if tide == "flood":
            return False
        return tide == arm
    return True

def cost_of(t, tide):
    # dry cost 100; silt 95% speed -> 105; shallow walker 133
    base = 105 if t == SILT else 100
    if (t.startswith("lane") or t.startswith("rim")) and tide == "neutral":
        base = 133
    return base

def dijkstra(src, tide="dryall"):
    INF = 1 << 60
    dist = [[INF] * N for _ in range(N)]
    sx, sy = src
    dist[sy][sx] = 0
    pq = [(0, sx, sy)]
    ok = lambda x, y: 0 <= x < N and 0 <= y < N and (tide == "dryall" and tiles[y][x] not in (SEA, DEEP, ROCK) or tide != "dryall" and walkable(tiles[y][x], tide))
    while pq:
        d, x, y = heapq.heappop(pq)
        if d != dist[y][x]:
            continue
        for ddx in (-1, 0, 1):
            for ddy in (-1, 0, 1):
                if not ddx and not ddy:
                    continue
                nx, ny = x + ddx, y + ddy
                if not ok(nx, ny):
                    continue
                if ddx and ddy and (not ok(x + ddx, y) or not ok(x, y + ddy)):
                    continue
                c = cost_of(tiles[ny][nx], "neutral" if tide == "neutral" else "dry")
                step = c * 141 // 100 if ddx and ddy else c
                nd = d + step
                if nd < dist[ny][nx]:
                    dist[ny][nx] = nd
                    heapq.heappush(pq, (nd, nx, ny))
    return dist

def cell(p):
    return (int(math.floor(p[0])), int(math.floor(p[1])))

def mirror(c):
    return (c[1], c[0])

D_T = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
D_R = float(sys.argv[2]) if len(sys.argv) > 2 else 62.0

def seat_pos(name, dist):
    u = unit(SEATS[name])
    return cell((C[0] + u[0] * dist, C[1] + u[1] * dist))

SHIFT = [int(v) for v in sys.argv[3].split(",")] if len(sys.argv) > 3 else [0, 2]
hq = {"T": seat_pos("T", D_T), "R": seat_pos("R", D_R)}
hq["R"] = (hq["R"][0] + SHIFT[0], hq["R"][1] + SHIFT[1])
hq["L"] = mirror(hq["R"])

# local frame: forward = toward the station; side = forward rotated +90
def local(name, f, s):
    a = SEATS[name] + 180.0
    fu = unit(a); su = (-fu[1], fu[0])
    h = hq[name]
    return cell((h[0] + 0.5 + fu[0] * f + su[0] * s, h[1] + 0.5 + fu[1] * f + su[1] * s))

station = cell(C)
# mouths and lane wrecks per arm
def arm_point(arm, s, t):
    u = unit(ARMS[arm]); n = (-u[1], u[0])
    return cell((C[0] + u[0] * s + n[0] * t, C[1] + u[1] * s + n[1] * t))

mouths = {}
lane_wrecks = []
deep_wrecks = []
for arm in ("E", "S"):
    mid = (LANE_S[0] + LANE_S[1]) / 2
    mouths[arm] = [arm_point(arm, mid, -(ARM_HALF + 2)), arm_point(arm, mid, ARM_HALF + 2)]
    lane_wrecks += [arm_point(arm, mid, -(ARM_HALF - 2)), arm_point(arm, mid, ARM_HALF - 2)]
    deep_wrecks.append(arm_point(arm, 14, 0))
mouths["W"] = [mirror(m) for m in mouths["E"]]
lane_wrecks += [mirror(w) for w in lane_wrecks[:2]]
deep_wrecks.append(mirror(deep_wrecks[0]))
# enforce symmetry of the self-mirrored S arm
mouths["S"] = [mouths["S"][0], mirror(mouths["S"][0])]
lane_wrecks[2:4] = [lane_wrecks[2], mirror(lane_wrecks[2])]

# per-seat economy, offsets copied from the Split Basin (HQ-relative)
ECON = [
    ("home", 6, 7), ("home", 6, -7),
    ("side", 26, 24), ("side", 28, 19), ("side", 26, -24), ("side", 28, -19),
    ("well_home", 9, 5), ("well_side", 34, 22), ("well_side", 34, -22),
]
econ = {}
for name in ("T", "R"):
    econ[name] = [(kind, local(name, f, s)) for kind, f, s in ECON]
# T must be self-mirrored: build its -side items as mirrors of the +side ones
tl = econ["T"]
def fix_pair(i, j):
    tl[j] = (tl[j][0], mirror(tl[i][1]))
fix_pair(0, 1); fix_pair(2, 4); fix_pair(3, 5); fix_pair(7, 8)
econ["L"] = [(k, mirror(c)) for k, c in econ["R"]]

dist = {n: dijkstra(hq[n]) for n in ("T", "R", "L")}
dist_neutral = {n: dijkstra(hq[n], "neutral") for n in ("T", "R")}
# match R's economy path lengths to T's, moving each item at most 2 cells
def refine():
    out = []
    for (k, ct), (_, cr) in zip(econ["T"], econ["R"]):
        want = dist["T"][ct[1]][ct[0]]
        best = None
        for oy in range(-2, 3):
            for ox in range(-2, 3):
                c = (cr[0] + ox, cr[1] + oy)
                if tiles[c[1]][c[0]] not in (SALT, SILT):
                    continue
                score = abs(dist["R"][c[1]][c[0]] - want) * 10 + abs(ox) + abs(oy)
                if best is None or score < best[0]:
                    best = (score, c)
        out.append((k, best[1]))
    econ["R"] = out
    econ["L"] = [(k, mirror(c)) for k, c in econ["R"]]
    dist["L"] = dijkstra(hq["L"])
refine()

def d(n, c, table=None):
    t = (table or dist)[n]
    v = t[c[1]][c[0]]
    return None if v >= 1 << 59 else round(v / 100, 1)

report = {"D_T": D_T, "D_R": D_R, "hq": hq}
rows = []
def own_mouths(n):
    # the two mouths on this seat's bank, nearest by path
    ms = [m for arm in mouths for m in mouths[arm]]
    ms.sort(key=lambda m: dist[n][m[1]][m[0]])
    return ms[:2]
for n in ("T", "R"):
    row = {"seat": n, "hq": hq[n],
           "to_station_dry": d(n, station),
           "to_station_neutral": d(n, station, dist_neutral),
           "own_mouths": [d(n, m) for m in own_mouths(n)],
           "econ": [(k, c, d(n, c)) for k, c in econ[n]]}
    rows.append(row)
report["rows"] = rows
report["hq_to_hq_dry"] = {"T-R": d("T", hq["R"]), "T-L": d("T", hq["L"]), "R-L": d("R", hq["L"])}
report["station"] = station
report["mouths"] = mouths
report["lane_wrecks"] = lane_wrecks
report["deep_wrecks"] = deep_wrecks
report["econ"] = econ

# land area per seat by nearest-HQ path (Voronoi on the dry-all graph)
area = {"T": 0, "R": 0, "L": 0}
for y in range(N):
    for x in range(N):
        if tiles[y][x] in (SALT, SILT):
            best = min(("T", "R", "L"), key=lambda n: dist[n][y][x])
            area[best] += 1
report["land_by_nearest_seat"] = area
counts = {}
for row in tiles:
    for t in row:
        counts[t] = counts.get(t, 0) + 1
report["terrain_counts"] = counts
json.dump(report, open("confluence.json", "w"), indent=1, default=list)
json.dump({"n": N, "tiles": ["".join({SEA: "~", DEEP: "D", SALT: ".", SILT: ",", "laneE": "e", "laneW": "w", "laneS": "s", "rimE": "E", "rimW": "W", "rimS": "S"}[t] for t in row) for row in tiles]}, open("confluence_tiles.json", "w"))

# PNG, one pixel per cell
COL = {SEA: (32, 63, 77), DEEP: (44, 89, 101), SALT: (216, 198, 160), SILT: (181, 161, 124),
       "lane": (113, 147, 148), "rim": (113, 147, 148), ROCK: (57, 67, 74)}
def color(t):
    if t.startswith("lane"): return COL["lane"]
    if t.startswith("rim"): return COL["rim"]
    return COL[t]
raw = b""
for y in range(N):
    raw += b"\x00" + bytes(v for x in range(N) for v in color(tiles[y][x]))
def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xffffffff)
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
open("confluence.png", "wb").write(png)

# The engine's grid: one character a cell, the island's salt marked apart
# from the mainland's so the game knows the sluice approach.
island = [[x <= y and on_island(x + 0.5, y + 0.5) for x in range(N)] for y in range(N)]
for y in range(N):
    for x in range(N):
        if x > y:
            island[y][x] = island[x][y]
CHAR = {SEA: "~", DEEP: "D", SALT: ".", SILT: ",", "laneE": "e", "laneW": "w", "laneS": "s",
        "rimE": "E", "rimW": "W", "rimS": "S"}
with open("confluence.map", "w") as f:
    for y in range(N):
        f.write("".join("i" if island[y][x] and tiles[y][x] == SALT else CHAR[tiles[y][x]]
                        for x in range(N)) + "\n")

# Starting workers, six a seat, three either side of the path to the station
# as on the Split Basin (5-7 cells ahead, 3 to the side); T's are mirrored
# pairs and L's mirror R's.
def land(c):
    return tiles[c[1]][c[0]] in (SALT, SILT)
workers = {}
for name in ("T", "R"):
    ws = []
    for side in (-3, 3):
        for f in (5, 6, 7):
            ws.append(local(name, f, side))
    workers[name] = ws
workers["T"] = workers["T"][:3] + [mirror(c) for c in workers["T"][:3]]
workers["L"] = [mirror(c) for c in workers["R"]]
# Practice-AI sites in the seat's own frame, taken from the Split Basin's east
# seat: Works 12 ahead and 8 aside, salvage yard 13 ahead, condenser on the
# home well, and Drydocks on the coast behind the base, nearest the ring sea.
def sea_reach(c):
    best = 99
    for oy in range(-12, 13):
        for ox in range(-12, 13):
            x, y = c[0] + ox, c[1] + oy
            if 0 <= x < N and 0 <= y < N and tiles[y][x] == SEA:
                best = min(best, max(abs(ox), abs(oy)))
    return best
def clear(c, r):
    return all(0 <= c[0] + ox < N and 0 <= c[1] + oy < N and land((c[0] + ox, c[1] + oy))
               for oy in range(-r, r + 1) for ox in range(-r, r + 1))
ai = {}
for name in ("T", "R"):
    econ_cells = [c for _, c in econ[name]] + [hq[name]]
    docks = []
    for f, s in ((-14, 0), (-12, 9), (-12, -9), (-6, 15), (-6, -15)):
        c = local(name, f, s)
        docks.append((c, clear(c, 2), sea_reach(c)))
    ai[name] = {"works": local(name, 12, 8), "dropoff": local(name, 13, 0),
                "condenser": econ[name][6][1], "drydocks": docks}
ai["L"] = {"works": mirror(ai["R"]["works"]), "dropoff": mirror(ai["R"]["dropoff"]),
           "condenser": mirror(ai["R"]["condenser"]),
           "drydocks": [(mirror(c), ok, r) for c, ok, r in ai["R"]["drydocks"]]}
report["engine"] = {"workers": workers, "ai": ai,
                    "workers_on_land": all(land(c) for w in workers.values() for c in w)}
json.dump(report, open("confluence.json", "w"), indent=1, default=list)
print(json.dumps({k: report[k] for k in ("D_T", "D_R", "hq", "hq_to_hq_dry", "land_by_nearest_seat")}, default=list))
for r in rows:
    print(r["seat"], r["hq"], "station dry", r["to_station_dry"], "neutral", r["to_station_neutral"], "mouths", r["own_mouths"])
    print("   ", [(k, dd) for k, c, dd in r["econ"]])
