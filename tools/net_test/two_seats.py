#!/usr/bin/env python3
"""Two real games on this machine playing each other online.

    python3 tools/net_test/two_seats.py [--bin target/release/brinewake]
        [--net-sim latency=60,jitter=20,loss=10] [--seconds 60] [--out DIR]

Starts two headless games, each with its own agent port and data folder,
and drives them the way a person would, through the menus:

1. Lobby. Seat A: PLAY ONLINE, HOST, wait for the code. Seat B: PLAY
   ONLINE, type the code, Enter. B READY, A START.
2. Match. Both seats give orders every few seconds. Every two seconds both
   are sampled: tick, round trip, input delay, stall, and the last state
   hashes, which must agree tick for tick.
3. A freeze. Seat B is stopped (SIGSTOP) for five seconds: A must name B
   and wait, then both carry on in step.
4. A crash. Seat B is killed and started again; it REJOINs from the menu and
   must come back in step with A.
5. A goodbye. Seat B quits; A must win.

`--net-sim` makes each seat's outgoing packets late, reordered and lost.
`--bin-b` runs seat B from another build, so two platforms play each other
(a command with spaces is split, as in `--bin-b "wine BRINEWAKE.exe"`).
Where a seat cannot be paused (Windows, or a seat run under Wine) stage 3
is skipped and reported so.
Pictures of each stage and `report.json` go to the output folder. The exit
status is 0 only if every check passed.
"""

import argparse
import json
import os
import shlex
import signal
import subprocess
import sys
import time
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PREFERENCES = {
    "muted": True,
    "edge_scroll": False,
    "pause_unfocused": False,
    "fullscreen": False,
    "world_zoom": 2,
    "game_speed": "Normal",
}


class Seat:
    def __init__(self, name, port, args, out):
        self.name = name
        self.port = port
        self.args = args
        self.data = os.path.join(out, name)
        self.log = os.path.join(out, f"{name}.log")
        self.proc = None
        os.makedirs(os.path.join(self.data, "ui"), exist_ok=True)
        with open(os.path.join(self.data, "ui", "preferences.json"), "w") as fh:
            json.dump(PREFERENCES, fh)

    def start(self, binary, net_sim):
        cmd = shlex.split(binary) if " " in binary and not os.path.exists(binary) else [binary]
        cmd += ["--headless", "--agent-port", str(self.port), "--size", "1280x720",
               "--data-dir", self.data]
        if net_sim:
            cmd += ["--net-sim", net_sim]
        env = dict(os.environ, BRINEWAKE_NAME=self.name)
        self.proc = subprocess.Popen(cmd, stdout=open(self.log, "a"), stderr=subprocess.STDOUT,
                                     env=env, cwd=ROOT)
        deadline = time.time() + 60
        while time.time() < deadline:
            try:
                self.state()
                return
            except Exception:
                time.sleep(0.3)
        raise RuntimeError(f"{self.name} did not answer on {self.port}")

    def get(self, path):
        with urllib.request.urlopen(f"http://127.0.0.1:{self.port}{path}", timeout=20) as r:
            body = r.read()
        return body if path.startswith("/frame") else json.loads(body)

    def post(self, path, body):
        req = urllib.request.Request(f"http://127.0.0.1:{self.port}{path}",
                                     data=json.dumps(body).encode(), method="POST")
        with urllib.request.urlopen(req, timeout=20) as r:
            return json.loads(r.read() or b"null")

    def state(self):
        return self.get("/state")

    def picture(self, out, label):
        with open(os.path.join(out, f"{self.name}-{label}.png"), "wb") as fh:
            fh.write(self.get("/frame?scale=1"))

    def press(self, label, within=10):
        """Click the enabled button with this label."""
        deadline = time.time() + within
        while time.time() < deadline:
            for b in self.get("/buttons"):
                if b["label"] == label and b["enabled"]:
                    x, y = b["x"] + b["w"] // 2, b["y"] + b["h"] // 2
                    self.post("/input", [{"t": "click", "x": x, "y": y}])
                    return
            time.sleep(0.2)
        raise RuntimeError(f"{self.name}: no enabled {label!r} button")

    def keys(self, *names):
        self.post("/input", [{"t": "key", "key": k} for k in names])

    def order(self, x, y):
        """Select every worker and send them somewhere."""
        self.post("/input", [{"t": "key", "key": "F7"},
                             {"t": "click", "x": x, "y": y, "button": "right"}])

    def stop(self):
        if self.proc and self.proc.poll() is None:
            try:
                self.post("/quit", {})
            except Exception:
                pass
            try:
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.kill()


def wait_for(what, check, within=30, every=0.25):
    deadline = time.time() + within
    last = None
    while time.time() < deadline:
        try:
            last = check()
            if last:
                return last
        except Exception as e:  # a busy seat can be slow to answer
            last = e
        time.sleep(every)
    raise RuntimeError(f"timed out waiting for {what} (last: {last})")


def agree(a, b):
    """Ticks both seats hashed, and whether each pair matched."""
    ha = dict((t, h) for t, h in (a.get("net") or {}).get("hashes", []))
    hb = dict((t, h) for t, h in (b.get("net") or {}).get("hashes", []))
    common = sorted(set(ha) & set(hb))
    return [(t, ha[t] == hb[t]) for t in common]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", default=os.path.join(ROOT, "target", "release", "brinewake"))
    parser.add_argument("--bin-b", default=None, help="seat B's build (default: --bin)")
    parser.add_argument("--net-sim", default="latency=60,jitter=20,loss=10")
    parser.add_argument("--seconds", type=int, default=60)
    parser.add_argument("--out", default=os.path.join(ROOT, "output", "net-test"))
    parser.add_argument("--ports", default="4811,4812")
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    pa, pb = (int(p) for p in args.ports.split(","))
    a = Seat("ANNE", pa, args, args.out)
    b = Seat("BO", pb, args, args.out)
    checks = []
    samples = []

    def check(name, ok, detail=""):
        checks.append({"check": name, "ok": bool(ok), "detail": detail})
        print(("PASS " if ok else "FAIL ") + name + (f": {detail}" if detail else ""), flush=True)

    try:
        bin_b = args.bin_b or args.bin
        can_pause = hasattr(signal, "SIGSTOP") and "wine" not in bin_b.lower()
        a.start(args.bin, args.net_sim)
        b.start(bin_b, args.net_sim)

        # 1. The lobby, through its buttons.
        a.press("PLAY ONLINE")
        a.press("HOST")
        code = wait_for("the host's code", lambda: a.state()["lobby"]["code"])
        a.picture(args.out, "1-hosting")
        check("host shows a join code", code.startswith("BW-"), code)
        b.press("PLAY ONLINE")
        b.keys(*list(code), "Enter")
        wait_for("the guest to be seated", lambda: (b.state()["lobby"] or {}).get("phase") == "Seated")
        wait_for("both seats in the lobby", lambda: len(a.state()["lobby"]["seats"]) == 2)
        b.picture(args.out, "2-seated")
        b.press("READY")
        wait_for("the host able to start", lambda: a.state()["lobby"]["can_start"] is None)
        a.picture(args.out, "3-ready")
        a.press("START")
        wait_for("both in the match", lambda: a.state()["screen"] == "Match" and b.state()["screen"] == "Match"
                 and (a.state()["session_tick"] or 0) > 30 and (b.state()["session_tick"] or 0) > 30)
        sa, sb = a.state(), b.state()
        check("sides are opposite", sa["faction"] != sb["faction"], f"{sa['faction']} v {sb['faction']}")

        # 2. Play: orders from both seats, sampled.
        started = time.time()
        n = 0
        mismatches = []
        compared = 0
        while time.time() - started < args.seconds:
            n += 1
            if n % 2 == 0:
                a.order(500 + (n * 37) % 300, 300 + (n * 23) % 150)
                b.order(700 - (n * 29) % 300, 320 + (n * 17) % 150)
            sa, sb = a.state(), b.state()
            pairs = agree(sa, sb)
            compared += len(pairs)
            mismatches += [t for t, same in pairs if not same]
            samples.append({
                "t": round(time.time() - started, 1),
                "ticks": [sa["session_tick"], sb["session_tick"]],
                "rtt_ms": [sa["net"]["rtt_ms"], sb["net"]["rtt_ms"]],
                "delay": [sa["net"]["delay"], sb["net"]["delay"]],
                "stalled": [sa["stalled"], sb["stalled"]],
            })
            time.sleep(2)
        wall = time.time() - started
        a.picture(args.out, "4-match")
        b.picture(args.out, "4-match")
        sa, sb = a.state(), b.state()
        ticks = min(sa["session_tick"], sb["session_tick"]) - samples[0]["ticks"][0]
        check("hashes agree tick for tick", compared > 0 and not mismatches,
              f"{compared} compared, mismatched at {mismatches[:5]}")
        check("no desync", sa["desync_tick"] is None and sb["desync_tick"] is None)
        rate = ticks / wall
        check("the match runs near 30 ticks a second", rate > 27, f"{rate:.1f}/s over {wall:.0f} s")
        check("orders landed", sa["own_entities"] > 0, f"message A: {sa['message']!r}")

        # 3. Freeze B for five seconds.
        if can_pause:
            tick_before = a.state()["session_tick"]
            os.kill(b.proc.pid, signal.SIGSTOP)
            waiting = wait_for("A to name B", lambda: a.state()["net"]["waiting_for"], within=15)
            a.picture(args.out, "5-waiting")
            check("a silent seat is named", waiting == "BO", waiting)
            time.sleep(3)
            os.kill(b.proc.pid, signal.SIGCONT)
            wait_for("the match to move on", lambda: a.state()["session_tick"] > tick_before + 90, within=30)
            time.sleep(3)
            pairs = agree(a.state(), b.state())
            check("in step after the freeze", pairs and all(same for _, same in pairs), str(pairs[-3:]))
        else:
            print("SKIP the freeze: seat B cannot be paused here", flush=True)

        # 4. Crash B and bring it back.
        b.proc.kill()
        b.proc.wait()
        time.sleep(1)
        held = a.state()["session_tick"]
        b.start(bin_b, args.net_sim)
        b.press("PLAY ONLINE")
        b.picture(args.out, "6-rejoin-offered")
        b.press("REJOIN")
        wait_for("B back in the match", lambda: b.state()["screen"] == "Match"
                 and (b.state()["session_tick"] or 0) >= held, within=30)
        time.sleep(6)
        sa, sb = a.state(), b.state()
        pairs = agree(sa, sb)
        check("a crashed seat rejoins in step", pairs and all(same for _, same in pairs),
              f"host held at {held}, now {sa['session_tick']}/{sb['session_tick']}, {pairs[-3:]}")
        b.picture(args.out, "7-rejoined")

        # 5. B says goodbye: A wins.
        b.post("/quit", {})
        b.proc.wait(timeout=10)
        outcome = wait_for("A's result", lambda: a.state()["outcome"], within=20)
        a.picture(args.out, "8-result")
        check("the one who stays wins", outcome == "Victory(0)", outcome)
    except Exception as e:
        check("the run finished", False, repr(e))
    finally:
        for seat in (a, b):
            seat.stop()
        report = {"net_sim": args.net_sim, "checks": checks, "samples": samples}
        with open(os.path.join(args.out, "report.json"), "w") as fh:
            json.dump(report, fh, indent=1)
    failed = [c for c in checks if not c["ok"]]
    print(f"{len(checks) - len(failed)}/{len(checks)} checks passed; report in {args.out}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
