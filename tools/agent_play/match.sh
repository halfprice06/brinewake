#!/bin/sh
# Start or stop a lockstep match between headless instances on this
# machine, each with its own agent interface.
#
#   tools/agent_play/match.sh start [--seed N] [--assembly]   host on 4711, guest on 4712
#   tools/agent_play/match.sh start --map confluence [--seed N]
#                                   host on 4711, guest on 4712, third on 4713
#   tools/agent_play/match.sh start ... --resume RECORDING
#                                   carry a stopped match on from where the
#                                   recording ends (every seat replays it)
#   tools/agent_play/match.sh stop
#   tools/agent_play/match.sh status
#
# On the Split Basin the host plays Union unless --assembly (or --faction F)
# is given and the guest takes the other faction. On the Confluence
# (three players, free-for-all) the host plays Union and the guests take the
# sides nobody holds, in joining order: the guest the Silt Assembly, the
# third seat the Saltglass Compact. Frames are 1280x720. Data directories
# and logs live under output/agent-play/. Each seat and the referee get
# their own folder for notes and reports (reports/host, reports/guest,
# reports/third, reports/referee) and the referee a scratch folder of its
# own (referee/scratch); start checks that every one of them can be written
# before the match begins.
#
# Each seat also gets a work folder of its own, $OUT/<seat>/work, holding
# its helper script play-<seat>: play.py with that seat's port built in, so
# a seat never types a port. Give each seat's agent only its own work folder
# as its scratchpad. Trial 10 gave all three seats one scratchpad and one
# helper in /usr/local/bin: a seat's helper was overwritten by another's and
# its first picture came from the wrong seat. A helper refuses any other
# port (BRINEWAKE_SEAT_PORT).
#
# Every seat logs a line to its <seat>.log for any tick over 250 ms, naming
# the simulation system that took longest (BRINEWAKE_WATCHDOG_MS; /state
# carries the count as "watchdog"). A crawling match shows there first.
#
# The match clock starts only when every seat has fetched its first frame,
# so no seat loses time reading this while another plays.
set -eu
cd "$(dirname "$0")/../.."
BIN=target/release/brinewake
# Two matches can run side by side: give the second its own folder, link
# port and agent ports (MATCH_OUT, MATCH_LINK, MATCH_PORT for the host; the
# guest and third take the next two). GUEST_FACTION picks the guest's side
# (union, assembly or compact) instead of the one nobody holds.
OUT="${MATCH_OUT:-output/agent-play}"
LINK="${MATCH_LINK:-127.0.0.1:4790}"
HOST_PORT="${MATCH_PORT:-4711}"
GUEST_PORT=$((HOST_PORT + 1))
THIRD_PORT=$((HOST_PORT + 2))
# Log-only: a line on stderr for a tick that runs long. Play is unchanged.
BRINEWAKE_WATCHDOG_MS="${BRINEWAKE_WATCHDOG_MS:-250}"
export BRINEWAKE_WATCHDOG_MS

# The seats of the running (or last started) match, kept so stop and status
# know whether there is a third.
seats() {
  if [ -f "$OUT/seats" ]; then cat "$OUT/seats"; else echo "host guest"; fi
}

port_of() {
  case "$1" in
    host) echo "$HOST_PORT" ;;
    guest) echo "$GUEST_PORT" ;;
    third) echo "$THIRD_PORT" ;;
  esac
}

seat_answers() {
  python3 tools/agent_play/play.py --port "$1" state >/dev/null 2>&1
}

# Ask each seat to quit before signalling it: a seat that is told to quit
# closes the session and writes last-match.replay.json, so a match that is
# stopped rather than played out is still watchable.
stop_seats() {
  for seat in $(seats); do
    python3 tools/agent_play/play.py --port "$(port_of "$seat")" quit >/dev/null 2>&1 || true
  done
  sleep 2
  for seat in $(seats); do
    if [ -f "$OUT/$seat.pid" ]; then
      kill "$(cat "$OUT/$seat.pid")" 2>/dev/null || true
      rm -f "$OUT/$seat.pid"
    fi
  done
}

case "${1-start}" in
  start)
    shift
    # Always build. A binary left over from an older rules version starts and
    # plays, and the trial is then run under rules nobody meant to test.
    # bw_tools too: an old `bw_tools analyse` refuses the new recording
    # with "replay rules digest mismatch" (trial 11-a).
    cargo build --release -p bw_desktop -p bw_tools
    # The Confluence holds three players: seat a third instance.
    SEATS="host guest"
    # Every seat carries a resumed match on from the same recording.
    RESUME=""
    prev=""
    for a in "$@"; do
      if [ "$prev" = "--map" ] && [ "$a" = "confluence" ]; then SEATS="host guest third"; fi
      if [ "$prev" = "--resume" ]; then RESUME="$a"; fi
      prev="$a"
    done
    mkdir -p "$OUT"
    echo "$SEATS" > "$OUT/seats"
    for seat in $SEATS; do mkdir -p "$OUT/$seat/ui" "$OUT/$seat/work"; done
    # A seat whose report cannot be written loses its report at the end of
    # the match, when nothing can be done about it. Check now.
    for dir in $(for seat in $SEATS; do echo "reports/$seat $seat/work"; done) reports/referee referee/scratch; do
      mkdir -p "$OUT/$dir"
      probe="$OUT/$dir/.write-check"
      if ! ( : > "$probe" ) 2>/dev/null || ! rm -f "$probe"; then
        echo "cannot write to $OUT/$dir: fix its permissions before the match" >&2
        exit 1
      fi
    done
    # Each seat's own helper, with its port built in and no other allowed.
    ROOT="$(pwd)"
    for seat in $SEATS; do
      helper="$OUT/$seat/work/play-$seat"
      cat > "$helper" <<SH
#!/bin/sh
# The $seat seat's own helper: drives port $(port_of "$seat") and no other.
BRINEWAKE_SEAT_PORT=$(port_of "$seat") exec python3 "$ROOT/tools/agent_play/play.py" --port $(port_of "$seat") "\$@"
SH
      chmod +x "$helper"
    done
    for seat in $SEATS; do
      cat > "$OUT/$seat/ui/preferences.json" <<'JSON'
{"muted": true, "edge_scroll": false, "pause_unfocused": false, "fullscreen": false, "world_zoom": 2, "game_speed": "Normal"}
JSON
    done
    # A recording plays only under the rules it was made with: keep the
    # build beside the match data so the match stays watchable later.
    cp "$BIN" "$OUT/host/brinewake-for-this-match"
    "$BIN" --headless --host "$LINK" --agent-port "$HOST_PORT" --size 1280x720 --data-dir "$OUT/host" "$@" > "$OUT/host.log" 2>&1 &
    echo $! > "$OUT/host.pid"
    sleep 1
    for seat in $SEATS; do
      [ "$seat" = host ] && continue
      # One at a time, so the sides go out in joining order.
      side=""
      [ "$seat" = guest ] && side="${GUEST_FACTION-}"
      "$BIN" --headless --join "$LINK" --agent-port "$(port_of "$seat")" --size 1280x720 --data-dir "$OUT/$seat" ${RESUME:+--resume "$RESUME"} ${side:+--faction "$side"} > "$OUT/$seat.log" 2>&1 &
      echo $! > "$OUT/$seat.pid"
      sleep 1
    done
    # Every seat must actually answer before this says the match is up. A
    # busy port or a refused handshake used to read exactly like a good start.
    ready=0
    # A resumed match first replays its recording on every seat.
    tries=15
    [ -n "$RESUME" ] && tries=900
    for _ in $(seq "$tries"); do
      ready=1
      for seat in $SEATS; do
        seat_answers "$(port_of "$seat")" || ready=0
      done
      [ "$ready" -eq 1 ] && break
      sleep 1
    done
    if [ "$ready" -eq 0 ]; then
      echo "the match did not start; the seats' own words follow" >&2
      for seat in $SEATS; do
        echo "--- $seat.log ---" >&2
        tail -5 "$OUT/$seat.log" >&2 || true
      done
      stop_seats
      exit 1
    fi
    for seat in $SEATS; do
      echo "$seat seat on http://127.0.0.1:$(port_of "$seat")  notes and reports: $OUT/reports/$seat"
      echo "  its work folder (its scratchpad, and no other seat's): $OUT/$seat/work  helper: $OUT/$seat/work/play-$seat"
    done
    echo "referee notes: $OUT/reports/referee  (referee scratch: $OUT/referee/scratch)"
    echo "the clock starts when every seat has fetched its first frame"
    echo "watch it live with the fog lifted: $BIN --replay $OUT/host/saves/live-match.replay.json --follow"
    echo "  or headless for a referee: $BIN --headless --replay $OUT/host/saves/live-match.replay.json --follow --agent-port $((HOST_PORT + 9))"
    echo "after the match: target/release/bw_tools analyse $OUT/host/saves/last-match.replay.json --csv $OUT/reports/referee/deaths.csv"
    echo "after the match, or after 'stop': $OUT/host/brinewake-for-this-match --replay $OUT/host/saves/last-match.replay.json"
    for seat in $SEATS; do
      python3 tools/agent_play/play.py --port "$(port_of "$seat")" state
    done
    ;;
  stop)
    stop_seats
    echo stopped
    ;;
  status)
    for seat in $(seats); do
      python3 tools/agent_play/play.py --port "$(port_of "$seat")" state
    done
    ;;
  *)
    echo "usage: match.sh start|stop|status" >&2
    exit 2
    ;;
esac
