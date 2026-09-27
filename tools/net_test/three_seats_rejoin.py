#!/usr/bin/env python3
"""Three real games on the Confluence: one crashes, is kept open, rejoins.

    python3 tools/net_test/three_seats_rejoin.py [--bin target/release/brinewake]
        [--ports 4920,4921,4922] [--udp 47920] [--out DIR]

Starts three headless games from the command line (`--host` on the
Confluence, two `--join`), each with its own agent port and data folder,
then:

1. All three play in step for a few seconds, the guests' workers sent out.
2. The third seat (CARA) is killed. The host must name it, offer KEEP SEAT
   OPEN, and the button is pressed.
3. The host and the other guest play on without it for ten seconds; the
   killed seat has not surrendered and the guest sees it AWAY.
4. CARA's game is started again with the same name and `--join` (no token:
   it comes back by its lobby name). It must be sent the match and play on,
   and every seat's state hashes must agree tick for tick.

Pictures of each stage and `report.json` go to the output folder. The exit
status is 0 only if every check passed.
"""

import argparse
import json
import os
import signal
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from two_seats import ROOT, Seat, wait_for  # noqa: E402


class CommandLineSeat(Seat):
    """A seat started straight into a match with `--host` or `--join`."""

    def __init__(self, name, port, extra, out):
        super().__init__(name, port, None, out)
        self.extra = extra

    def start(self, binary, net_sim=None):
        import subprocess

        cmd = [binary, "--headless", "--agent-port", str(self.port), "--size", "1280x720",
               "--data-dir", self.data] + self.extra
        env = dict(os.environ, BRINEWAKE_NAME=self.name)
        self.proc = subprocess.Popen(cmd, stdout=open(self.log, "a"), stderr=subprocess.STDOUT,
                                     env=env, cwd=ROOT)

    def answer(self, within=120):
        wait_for(f"{self.name} to answer", lambda: self.state(), within=within, every=0.5)


def net(state):
    return state.get("net") or {}


def hashes(state):
    return dict((t, h) for t, h in net(state).get("hashes", []))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", default=os.path.join(ROOT, "target", "release", "brinewake"))
    parser.add_argument("--ports", default="4920,4921,4922")
    parser.add_argument("--udp", type=int, default=47920)
    parser.add_argument("--out", default=os.path.join(ROOT, "output", "net-test-rejoin"))
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    pa, pb, pc = (int(p) for p in args.ports.split(","))
    address = f"127.0.0.1:{args.udp}"
    host = CommandLineSeat("ANNE", pa, ["--host", address, "--map", "confluence", "--delay", "3",
                                        "--seed", "7"], args.out)
    bo = CommandLineSeat("BO", pb, ["--join", address], args.out)
    cara = CommandLineSeat("CARA", pc, ["--join", address], args.out)
    checks = []
    report = {"checks": checks}

    def check(name, ok, detail=""):
        checks.append({"check": name, "ok": bool(ok), "detail": detail})
        print(("PASS " if ok else "FAIL ") + name + (f": {detail}" if detail else ""), flush=True)

    seats = [host, bo, cara]
    try:
        host.start(args.bin)
        time.sleep(1.0)
        bo.start(args.bin)
        time.sleep(0.5)
        cara.start(args.bin)
        for seat in seats:
            seat.answer()
        # The clock waits for every seat's first look.
        for seat in seats:
            seat.picture(args.out, "0-start")
        wait_for("the match to run", lambda: all(
            (s.state().get("session_tick") or 0) > 90 for s in seats), within=60)
        for seat in (bo, cara):
            seat.order(640, 300)
        time.sleep(4)
        states = [s.state() for s in seats]
        common = set(hashes(states[0])) & set(hashes(states[1])) & set(hashes(states[2]))
        agree = [t for t in common if len({hashes(s)[t] for s in states}) == 1]
        check("three seats in step", common and len(agree) == len(common),
              f"{len(agree)}/{len(common)} ticks agree")

        # CARA's game crashes.
        cara.proc.send_signal(signal.SIGKILL)
        cara.proc.wait()
        crashed_at = host.state().get("session_tick")
        wait_for("the host to offer KEEP SEAT OPEN",
                 lambda: net(host.state()).get("can_keep_open"), within=20)
        waiting = net(host.state()).get("waiting_for")
        check("the host names the silent seat", waiting == "CARA", f"waiting for {waiting}")
        host.picture(args.out, "1-waiting")
        host.press("KEEP SEAT OPEN")
        wait_for("the seat to be kept open", lambda: net(host.state()).get("away"), within=10)
        before = host.state().get("session_tick")
        time.sleep(10)
        hs, bs = host.state(), bo.state()
        ran = (hs.get("session_tick") or 0) - (before or 0)
        check("the other two play on", ran >= 240, f"{ran} ticks in 10 s")
        check("the guest sees the seat kept open", net(bs).get("away") == "CARA",
              f"away {net(bs).get('away')}, out in {net(bs).get('away_out_seconds')} s")
        players = hs.get("seats") or []
        check("nobody is out: the away seat has not surrendered",
              players and not any(p.get("eliminated") for p in players),
              ", ".join(f"{p.get('title')} out={p.get('eliminated')}" for p in players))
        host.picture(args.out, "2-kept-open")
        bo.picture(args.out, "2-kept-open")
        report["crashed_at"] = crashed_at
        report["players"] = players

        # CARA starts again and joins with the same name: no token.
        cara.start(args.bin)
        cara.answer()
        cara.picture(args.out, "3-back")
        rejoined_at = host.state().get("session_tick")
        wait_for("CARA to play on", lambda: (cara.state().get("session_tick") or 0) > rejoined_at + 150,
                 within=60)
        cara.order(640, 360)
        time.sleep(6)
        states = [s.state() for s in seats]
        common = set(hashes(states[0])) & set(hashes(states[1])) & set(hashes(states[2]))
        agree = sorted(t for t in common if len({hashes(s)[t] for s in states}) == 1)
        check("all three agree after the rejoin", common and len(agree) == len(common),
              f"ticks {agree}")
        check("nobody desynchronised", all(s.get("desync_tick") is None for s in states))
        check("the seat is no longer away", net(states[0]).get("away") is None)
        check("CARA holds its own seat", net(states[2]).get("seat") == 2,
              f"seat {net(states[2]).get('seat')}")
        for seat in seats:
            seat.picture(args.out, "4-after")
        report["ticks"] = [s.get("session_tick") for s in states]
        report["hash_ticks_compared"] = agree
    except Exception as e:  # noqa: BLE001
        check("the run finished", False, repr(e))
    finally:
        for seat in seats:
            seat.stop()
        with open(os.path.join(args.out, "report.json"), "w") as fh:
            json.dump(report, fh, indent=2)
    return 0 if checks and all(c["ok"] for c in checks) else 1


if __name__ == "__main__":
    sys.exit(main())
