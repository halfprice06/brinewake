#!/usr/bin/env python3
"""Referee sampler for a two- or three-seat match.

Every INTERVAL seconds fetch /state and a half-size frame from every seat,
save them under DIR as referee-N-<faction>.json / .png, and print one summary
line (stdout, flushed). Exits when any seat reports an outcome (the match is
decided; a seat knocked out of a three-way match has none), or when every
seat stops answering three times running. The sample counter persists in
DIR/.referee-n so a restarted sampler keeps numbering where it left off.

Each seat is named by the faction it reports, not by its port, so a match
hosted by the Assembly (match.sh start --assembly) is labelled the right way
round.

Each sample also prints, per seat, the army by kind, the deaths since the
last sample (kind and what killed it), captures and switches, and the seat's
tempo: pictures fetched and action calls made in the last minute and in
all. Seat speed is a hidden variable in every trial; this makes it visible.
The same goes to DIR/samples.jsonl, one JSON line a sample. The referee's
own pictures are marked as such and do not count as the seat looking.

With --observer-port, a fog-lifted observer (brinewake --headless --replay
... --follow --agent-port N) is sampled too: its whole-map chart as
referee-N-map.png and its entity list as referee-N-entities.json.

Between samples the referee polls both seats' /state every POLL seconds
(default 3; the `hold`, `lane_hold` and `sluice` fields, with the harness log
cut to nothing new) and takes an extra sample, outside the fixed interval,
when a hold count starts (a side's seconds-left leaves the full hold (hold_seconds) or starts falling
again), when a running count breaks (stops falling for two game seconds or
climbs back toward full), and every COUNT_EVERY game seconds (default 15)
while a count runs. Each sample's line and its samples.jsonl record carry
`reason`: "interval", or the triggers, e.g. "union count starts (88s left)",
"assembly count broken at 61s left", "union count running (43s left)". A
count is named by the faction that holds it, read from whichever seat is
furthest ahead, so both seats seeing the same count make one sample.
The fixed interval restarts after every sample.

Every poll and sample also logs each seat's `heard` lines (the tide sounds
a player would hear) to DIR/heard.jsonl, once per distinct line:
{"seat", "tick" (the cue's own tick, from its match clock, to the second),
"clock", "text", "seen_tick" (the seat's tick when the referee first read
it)}. Set against the count samples and the seat's next orders, this gives a
defender's reaction time to a count.

  python3 -u tools/agent_play/referee.py --dir output/agent-play/trial-6 --interval 75
  python3 -u tools/agent_play/referee.py --dir DIR --host-port 4711 --guest-port 4712 --observer-port 4720
  python3 -u tools/agent_play/referee.py --dir DIR --third-port 4713 --observer-port 4720   (the Confluence)
"""
import argparse
import json
import os
import sys
import time
import urllib.request

def fetch(port, path, timeout=90):
    with urllib.request.urlopen(f"http://127.0.0.1:{port}{path}", timeout=timeout) as r:
        return r.read()


# Seconds on a hold count when /state does not say: a side with this many
# left has no count. The rule varies (90 s with two seats, 75 s with three,
# 120 s once one is out), so every reading takes `hold.hold_seconds` from the
# seat's own /state; trial 12's sampler read 15 s high by assuming 90.
FULL_HOLD = 90


def full_hold(state):
    return int((state.get("hold") or {}).get("hold_seconds") or FULL_HOLD)


class HeardLog:
    """Appends each seat's distinct `heard` lines to DIR/heard.jsonl."""

    def __init__(self, directory):
        self.path = os.path.join(directory, "heard.jsonl")
        self.seen = set()
        if os.path.exists(self.path):  # a restarted referee does not repeat lines
            try:
                with open(self.path) as f:
                    for line in f:
                        r = json.loads(line)
                        self.seen.add((r.get("seat"), r.get("clock"), r.get("text")))
            except Exception:  # noqa: BLE001
                pass

    def note(self, tag, s):
        if not s:
            return
        fresh = []
        for line in s.get("heard") or []:
            stamp, _, text = str(line).partition(" ")
            key = (tag, stamp, text)
            if key in self.seen:
                continue
            self.seen.add(key)
            try:
                m, sec = stamp.split(":")
                tick = (int(m) * 60 + int(sec)) * 30
            except ValueError:
                tick, text = None, str(line)
            fresh.append({"seat": tag, "tick": tick, "clock": stamp, "text": text, "seen_tick": s.get("tick")})
        if fresh:
            with open(self.path, "a") as f:
                for r in fresh:
                    f.write(json.dumps(r) + "\n")


class CountWatch:
    """Follows every hold count, named by the faction holding it, and says
    when one starts, breaks, or has run COUNT_EVERY game seconds unsampled."""

    def __init__(self, seat_tags, every_seconds):
        self.tags = seat_tags
        self.every = every_seconds * 30
        self.last = {}  # holder -> (tick, seconds_left)
        self.running = {}  # holder -> bool
        self.last_sample_tick = 0

    def readings(self, states):
        """holder -> (tick, seconds_left, full), from the seat furthest ahead."""
        out = {}
        for tag, s in states.items():
            if not s or not s.get("hold"):
                continue
            full = full_hold(s)
            if s.get("seats"):
                # Every seat's gauge, named by its faction: right for any
                # number of seats.
                for seat in s["seats"]:
                    holder = str(seat.get("faction", "?")).lower()
                    v = -(-(full * 30 - int(seat.get("lane_hold") or 0)) // 30)
                    if holder not in out or s["tick"] > out[holder][0]:
                        out[holder] = (s["tick"], max(v, 0), full)
                continue
            hold = s["hold"]
            other = next((t for t in self.tags if t != tag), "enemy")
            for holder, key in ((tag, "own_seconds_left"), (other, "enemy_seconds_left")):
                v = hold.get(key)
                if v is None:
                    continue
                if holder not in out or s["tick"] > out[holder][0]:
                    out[holder] = (s["tick"], v, full)
        return out

    def reasons(self, states):
        why = []
        for holder, (tick, v, full) in sorted(self.readings(states).items()):
            prev = self.last.get(holder)
            running = self.running.get(holder, False)
            if prev is None:
                if v < full:
                    running = True
                    why.append(f"{holder} count running ({v}s left)")
            elif tick > prev[0]:
                p = prev[1]
                if not running and v < p:
                    running = True
                    why.append(f"{holder} count starts ({v}s left)")
                elif running and (v > p or (v == p and tick - prev[0] >= 60)):
                    running = False
                    why.append(f"{holder} count broken at {p}s left (now {v}s)")
                elif running and tick - self.last_sample_tick >= self.every:
                    why.append(f"{holder} count running ({v}s left)")
                elif v == p and tick - prev[0] < 60:
                    continue  # keep the older reading until two seconds pass
            self.running[holder] = running
            self.last[holder] = (tick, v)
        return why

    def sampled(self, tick):
        self.last_sample_tick = tick


def seats(host_port, guest_port, third_port=None):
    """Name each seat by the faction it reports, falling back to its role."""
    named = []
    roles = [("host", host_port), ("guest", guest_port)]
    if third_port:
        roles.append(("third", third_port))
    for role, port in roles:
        tag = role
        try:
            faction = json.loads(fetch(port, "/state")).get("faction")
            if faction:
                tag = str(faction).lower()
        except Exception:  # noqa: BLE001
            pass
        named.append((tag, port))
    if len({tag for tag, _ in named}) < len(named):  # a mirror: name by role
        named = roles
    return named


def clock(tick):
    s = tick // 30
    return f"{s // 60:02d}:{s % 60:02d}"


def owner_word(v):
    return {None: "none", 0: "own"}.get(v, "enemy" if isinstance(v, int) else str(v))


def summary(tag, s):
    if s is None:
        return f"{tag}: no answer"
    alerts = ",".join(s.get("alerts") or [])
    msg = (s.get("message") or "").replace("\n", " ")[:70]
    g = s["sluice"]
    cap = ""
    if g.get("capture_player") is not None:
        cap = f" capturing {owner_word(g['capture_player'])} {g['capture_percent']}%"
    tide = s.get("tide") or {}
    hold = s.get("hold") or {}
    stall = ""
    if s.get("stalled"):
        stall = f" STALLED {s.get('peer_quiet_seconds')}s"
    net = s.get("net") or {}
    if s.get("behind") and net.get("behind"):
        # Connected but late (a busy game): no drop clock runs.
        stall += f" BEHIND {net['behind']} {net.get('behind_seconds')}s"
    dog = s.get("watchdog") or {}
    if dog.get("slow_ticks"):
        stall += f" SLOW-TICKS {dog['slow_ticks']} worst {dog.get('worst_ms')}ms"
    if s.get("desync_tick") is not None:
        stall += f" DESYNC at {s['desync_tick']}"
    tide_words = ""
    if tide:
        # dry_arm is null while no arm is dry (neutral tide or a flood).
        side = f"{s['dry_arm']}-dry" if s.get("dry_arm") else "none-dry" if "dry_arm" in s else "N-dry" if tide.get("north_dry") else "S-dry"
        tide_words = f" tide {tide.get('mode')} {side}"
        if tide.get("warning_seconds") is not None:
            tide_words += f" warn {tide['warning_seconds']}s"
        if tide.get("flood_seconds") is not None:
            tide_words += f" flood {tide['flood_seconds']}s"
    hold_words = ""
    if hold:
        def lane(l):
            return "both" if l.get("own") and l.get("enemy") else "own" if l.get("own") else "enemy" if l.get("enemy") else "-"
        lanes = hold.get("lanes") or [{}, {}]
        letters = [l.get("arm") or "NS"[i % 2] for i, l in enumerate(lanes)]
        hold_words = (
            f" hold own {hold.get('own_seconds_left')}s enemy {hold.get('enemy_seconds_left')}s "
            + " ".join(f"{c}:{lane(l)}" for c, l in zip(letters, lanes))
        )
        out = [str(x.get("faction", "?")).lower() for x in s.get("seats") or [] if x.get("eliminated")]
        if out:
            hold_words += f" out[{','.join(out)}]"
    return (
        f"{tag}: crew {s['crew']}/{s['crew_cap']} salv {s['salvage']} pres {s['pressure']}/{s['pressure_cap']} "
        f"own {s['own_entities']} seen {s['enemies_visible']} sel {s['selected']} tier {s['doctrine_tier']} "
        f"upg {len(s['upgrades'])} gauge {s['lane_hold']} sluice {owner_word(g['owner'])}{cap}"
        f"{tide_words}{hold_words}{stall} alerts[{alerts}] msg \"{msg}\""
    )


def army(s):
    counts = s.get("own_by_kind") or {}
    return " ".join(f"{n} {k.lower()}" for k, n in sorted(counts.items(), key=lambda kv: -kv[1]))


def happenings(s):
    """Deaths, captures and switches the seat's interface logged since the last sample."""
    log = (s.get("harness") or {}).get("log") or []
    lost = {}
    lines = []
    for e in log:
        if e.get("kind") == "death":
            if e.get("side") == "own":
                key = f"{e.get('unit', '?').lower()} (by {str(e.get('cause') or '?').lower()})"
                lost[key] = lost.get(key, 0) + 1
        else:
            lines.append(f"{clock(e.get('tick', 0))} {e.get('side')} {e.get('text')}")
    return lost, lines


def tempo(s):
    h = s.get("harness") or {}
    return (
        f"looked {h.get('frames_last_minute', 0)}/min ({h.get('frames', 0)}) "
        f"acted {h.get('inputs_last_minute', 0)}/min ({h.get('inputs', 0)})"
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--interval", type=float, default=75)
    ap.add_argument("--host-port", type=int, default=4711)
    ap.add_argument("--guest-port", type=int, default=4712)
    ap.add_argument("--third-port", type=int, help="the third seat on the Confluence (4713)")
    ap.add_argument("--observer-port", type=int)
    ap.add_argument("--poll", type=float, default=3, help="seconds between cheap hold-count polls")
    ap.add_argument("--count-every", type=float, default=15, help="game seconds between samples while a count runs")
    a = ap.parse_args()
    SEATS = seats(a.host_port, a.guest_port, a.third_port)
    print(f"seats: {', '.join(f'{tag} on {port}' for tag, port in SEATS)}", flush=True)
    os.makedirs(a.dir, exist_ok=True)
    counter_path = os.path.join(a.dir, ".referee-n")
    n = 1
    if os.path.exists(counter_path):
        n = int(open(counter_path).read().strip() or "1")
    failures = 0
    since = {tag: 0 for tag, _ in SEATS}
    heard = HeardLog(a.dir)
    counts = CountWatch([tag for tag, _ in SEATS], a.count_every)
    reason = "interval"
    while True:
        states = {}
        for tag, port in SEATS:
            try:
                s = json.loads(fetch(port, f"/state?since={since[tag]}"))
                with open(os.path.join(a.dir, f"referee-{n}-{tag}.json"), "w") as f:
                    json.dump(s, f, indent=1)
                png = fetch(port, "/frame?scale=2&referee=1")
                with open(os.path.join(a.dir, f"referee-{n}-{tag}.png"), "wb") as f:
                    f.write(png)
                states[tag] = s
                heard.note(tag, s)
            except Exception as e:  # noqa: BLE001
                states[tag] = None
                print(f"[{n}] {tag} seat error: {e}", flush=True)
        live = [s for s in states.values() if s]
        if not live:
            failures += 1
            if failures >= 3:
                print(f"[{n}] every seat down three times running; referee exits", flush=True)
                sys.exit(1)
        else:
            failures = 0
            tick = max(s["tick"] for s in live)
            if reason == "interval":  # a count that moved since the last poll is still news
                reason = "; ".join(counts.reasons(states)) or "interval"
            else:
                counts.reasons(states)
            counts.sampled(tick)
            line = f"[{n}] tick {tick} ({clock(tick)}) [{reason}] | " + " | ".join(
                summary(tag.upper()[0], states[tag]) for tag, _ in SEATS
            )
            print(line, flush=True)
            record = {"n": n, "tick": tick, "reason": reason, "seats": {}}
            for tag, _ in SEATS:
                s = states[tag]
                if not s:
                    continue
                lost, lines = happenings(s)
                lost_words = ", ".join(f"{v} {k}" for k, v in lost.items()) or "none"
                print(f"    {tag}: army {army(s) or '-'} | lost since last: {lost_words} | {tempo(s)}", flush=True)
                for l in lines:
                    print(f"    {tag}: {l}", flush=True)
                h = s.get("harness") or {}
                record["seats"][tag] = {
                    "hold": s.get("hold"),
                    "lane_hold": s.get("lane_hold"),
                    "army": s.get("own_by_kind"),
                    "lost": lost,
                    "events": lines,
                    "frames": h.get("frames"),
                    "frames_last_minute": h.get("frames_last_minute"),
                    "inputs": h.get("inputs"),
                    "inputs_last_minute": h.get("inputs_last_minute"),
                }
                since[tag] = s["tick"]
            with open(os.path.join(a.dir, "samples.jsonl"), "a") as f:
                f.write(json.dumps(record) + "\n")
            if a.observer_port:
                try:
                    with open(os.path.join(a.dir, f"referee-{n}-map.png"), "wb") as f:
                        f.write(fetch(a.observer_port, "/map?cell=4"))
                    ents = json.loads(fetch(a.observer_port, "/entities"))
                    with open(os.path.join(a.dir, f"referee-{n}-entities.json"), "w") as f:
                        json.dump(ents, f, indent=1)
                    print(f"    observer at {ents['clock']}: {ents['counts']}", flush=True)
                except Exception as e:  # noqa: BLE001
                    print(f"    observer error: {e}", flush=True)
            for tag, _ in SEATS:
                s = states[tag]
                if s and s.get("outcome"):
                    print(f"[{n}] RESULT at tick {tick} ({clock(tick)}): {tag} seat reports outcome {s['outcome']}", flush=True)
                    with open(counter_path, "w") as f:
                        f.write(str(n + 1))
                    sys.exit(0)
        n += 1
        with open(counter_path, "w") as f:
            f.write(str(n))
        reason = wait(a, SEATS, heard, counts)


def wait(a, seats_, heard, counts):
    """Poll every seat's /state cheaply until the interval is up or a hold
    count starts, breaks or is due its running sample; say which."""
    deadline = time.monotonic() + a.interval
    while True:
        left = deadline - time.monotonic()
        if left <= 0:
            return "interval"
        time.sleep(min(a.poll, left))
        states = {}
        for tag, port in seats_:
            try:
                # `since` far ahead keeps the harness log out of the reply.
                s = json.loads(fetch(port, f"/state?since={1 << 40}", timeout=10))
                states[tag] = s
                heard.note(tag, s)
            except Exception:  # noqa: BLE001
                states[tag] = None
        if any(s and s.get("outcome") for s in states.values()):
            return "outcome"
        why = counts.reasons(states)
        if why:
            return "; ".join(why)


if __name__ == "__main__":
    main()
