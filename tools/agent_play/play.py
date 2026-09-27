#!/usr/bin/env python3
"""Drive one headless BRINEWAKE instance through its agent interface.

Usage:
  play.py --port 4711 frame OUT.png [--scale 2] [--crop X0 Y0 X1 Y1] [--zoom 2] [--clean]
  play.py --port 4711 state
  play.py --port 4711 click X Y [--right] [--shift]
  play.py --port 4711 dblclick X Y
  play.py --port 4711 drag X0 Y0 X1 Y1 [--shift]
  play.py --port 4711 key KEY [--shift] [--control]
  play.py --port 4711 pan DX DY
  play.py --port 4711 events '[{"t":"click","x":100,"y":200}, ...]'
  play.py --port 4711 quit

A seat's own helper (output/agent-play/<seat>/work/play-<seat>, written by
match.sh) runs this with its port built in and BRINEWAKE_SEAT_PORT set; any
other --port is then refused, so a seat cannot drive another seat's game.

Coordinates are pixels of the full frame (1280x720 by default); a frame
fetched with --scale 2 is half size, so double its coordinates.

--crop X0 Y0 X1 Y1 cuts a region out of the full frame (full-frame pixels,
so the numbers are the ones you click with); --zoom 2 to 4 enlarges it by
whole pixels, for the panel, the chart and small text. A cropped picture's
own pixel (px, py) is full-frame (X0 + px * scale / zoom, Y0 + py * scale / zoom).
--clean draws the field without its corner controls (sluice card, zoom,
IDLE/ARMY/WORKS) to read the ground under them; it does not change where
they are.

An action call answers with each event as the game took it (the key name it
matched and the shift and control it carried), what it refused, the tick and
match clock it acted on, the prompt row under the field, and `heard`, the tide
sounds of the last ten seconds as text. `state` is for referees and match scripts; a seat plays from the
picture. `quit` ends a seat's match and leaves saves/last-match.replay.json
behind.
"""
import argparse
import json
import os
import sys
import urllib.request


def call(port, method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=data, method=method)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=30) as response:
        return response.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("command")
    parser.add_argument("args", nargs="*")
    parser.add_argument("--scale", type=int, default=1)
    parser.add_argument("--right", action="store_true")
    parser.add_argument("--shift", action="store_true")
    parser.add_argument("--control", action="store_true")
    parser.add_argument("--crop", type=int, nargs=4, metavar=("X0", "Y0", "X1", "Y1"))
    parser.add_argument("--zoom", type=int, default=1)
    parser.add_argument("--clean", action="store_true")
    a = parser.parse_args()
    seat_port = os.environ.get("BRINEWAKE_SEAT_PORT")
    if seat_port and int(seat_port) != a.port:
        print(f"this helper drives port {seat_port} only, not {a.port}", file=sys.stderr)
        sys.exit(2)
    c, args = a.command, a.args
    if c == "frame":
        out = args[0] if args else "frame.png"
        query = f"scale={a.scale}&zoom={a.zoom}"
        if a.crop:
            query += "&crop=" + ",".join(str(v) for v in a.crop)
        if a.clean:
            query += "&clean=1"
        png = call(a.port, "GET", f"/frame?{query}")
        with open(out, "wb") as f:
            f.write(png)
        width, height = int.from_bytes(png[16:20], "big"), int.from_bytes(png[20:24], "big")
        where = f", crop {a.crop[0]},{a.crop[1]} to {a.crop[2]},{a.crop[3]}" if a.crop else ""
        print(f"wrote {out} ({width}x{height}, {len(png) // 1024} KB, scale {a.scale}, zoom {a.zoom}{where})")
        return
    if c == "state":
        print(call(a.port, "GET", "/state").decode())
        return
    if c == "quit":
        print(call(a.port, "POST", "/quit").decode())
        return
    if c == "events":
        events = json.loads(args[0])
    elif c in ("click", "dblclick"):
        x, y = int(args[0]), int(args[1])
        events = [{"t": c, "x": x, "y": y, "button": "right" if a.right else "left", "shift": a.shift}]
    elif c == "drag":
        x0, y0, x1, y1 = (int(v) for v in args[:4])
        events = [{"t": "drag", "x0": x0, "y0": y0, "x1": x1, "y1": y1, "shift": a.shift}]
    elif c == "key":
        events = [{"t": "key", "key": args[0], "shift": a.shift, "control": a.control}]
    elif c == "pan":
        events = [{"t": "pan", "dx": int(args[0]), "dy": int(args[1])}]
    else:
        print(f"unknown command {c}", file=sys.stderr)
        sys.exit(2)
    print(call(a.port, "POST", "/input", events).decode())


if __name__ == "__main__":
    main()
