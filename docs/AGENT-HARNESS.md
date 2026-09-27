# Two-player lockstep and agent play

2026-09-14. The owner asked for two AI agents to be able to play each
other, using vision and mouse and keyboard alone, to help improve the game.
This note records what was built, how it fits the plan's staged multiplayer
section, and what it does not claim.

## What exists

**Trusted lockstep session** (`crates/bw_desktop/src/net.rs`). Two copies of
the game run one deterministic simulation. The host binds a TCP port, the
guest connects; they agree the protocol version, the rules digest, the seed,
the host's faction and the input delay. From then on each seat sends one
batch per tick, containing the commands it issued, stamped for the current
tick plus the delay (three ticks). A seat advances tick T only when it holds
the other seat's batch for T. Commands for a tick are applied host first,
then guest, then the tick steps. Every thirty ticks the seats exchange the
canonical state hash; a mismatch is surfaced as a desync and the match stops
rather than diverging. A closed connection ends the match with a notice.
The rules digest check refuses mismatched builds before play starts.

**The seat view** (`World::relabeled_for` in `bw_sim`). The desktop presents
everything from player 0's seat in about 350 places. Rather than thread a
local seat index through all of them, the guest, who is canonically player 1,
draws and reads a relabelled copy of the canonical world in which every
owner, player slot, gate owner, capture, shot, event, outcome and observation
has swapped sides. Ids, positions and the tick are unchanged, so commands
built against the view are valid against the canonical world, and
`World::validate` gives the guest immediate feedback for its canonical seat.
The view is never stepped or hashed; only the canonical world is. This keeps
the simulation's own left and right start asymmetries intact.

**Game integration.** `Game::start_network` takes the seat's faction and view
from the session and centres the camera on that seat's headquarters;
`step_world` steps through the session and refreshes the view; commands route
through the session; pace is fixed at 1x; pause on focus loss is off; Load is
refused; the replay written at the end is the canonical one. A pause by
either seat stalls both, which is acceptable for a trusted match and stated
in the README.

**Agent harness** (`crates/bw_desktop/src/agent.rs`). `--headless` runs the
game loop from wall time without a window or GPU. `--agent-port N` serves a
small local HTTP interface: `GET /frame?scale=k` returns the current picture
as PNG, `POST /input` takes a JSON list of pointer and key events (move,
click, dblclick, drag, key, pan) and applies them through the same entry
points the window uses, and `GET /state` returns a short JSON summary for
referees, including the two alert cards on show and the sluice's owner,
capture progress and contest state. Two headless seats run side by side on
one machine without contending for focus or the mouse.

**Tooling** (`tools/agent_play/`). `match.sh start` launches a host on port
4711 and a guest on 4712 with a loopback link on 4790 and muted, unpaused
profiles under `output/agent-play/`; `play.py` drives a seat from the shell;
`PLAYBOOK.md` is the material an agent needs to play from pictures: how to
see and act, the coordinate scaling, what the interface shows, six rules
and a first minute.

**Recording and observation** (`crates/bw_desktop/src/playback.rs`,
`bw_sim::ReplayPlayer`). Every ten seconds of field time the match so far is
written to `saves/live-match.replay.json` under the data directory, the
canonical world in a session. `--replay FILE` plays a recording tick by tick
with the fog lifted through `World::revealed`, a presentation flag that is
never serialised or hashed; pace and pause work and orders are refused.
`--follow` re-reads a growing file once a second, so a live match, including
one between two agents, can be watched a few seconds behind. The observer
sees from the host's seat: the host's side is jade and the guest's carries
the red edge.

## How a trial runs

Start a fresh match, spawn one agent per seat with the playbook,
the port and an action budget, and ask each for a structured report: actions
with the frame each was based on, anything wrong or unreadable with tick and
frame, whether controls behaved as documented, one suggestion. Agents must
not read source or `/state` to decide moves; the state returned after an
action is confirmation only. A referee (the orchestrating session) checks
both seats' ticks and the session status and collects the reports.

## Limits

- Trusted mode only: each seat holds the whole world, so a modified client
  can see through the fog. Not for public play.
- No lobby, matchmaking or reconnect; a dropped link ends the match.
- The host waits two minutes for a guest and then says no one came.
- Input delay is fixed at three ticks (100 ms), adequate on a LAN.
- Agents see downscaled frames and act through discrete events; they do not
  exercise held keys, edge scrolling or middle-button panning.
- `/state` is served on the playing seat's own port. The input reply carries
  nothing to play from, but a seat told to read `/state` could; only a
  separate referee port would make the blindness structural.
- The match clock runs behind the wall clock: a backlog over 200 ms is
  discarded by design (`tempo::MAX_CATCH_UP_TICKS`), so game time advances at
  a rate that depends on how often the seats fetch pictures.

## What the harness review changed

2026-09-18, recorded in `design/reviews/harness/root-review.md`. A session now
keeps stepping while a seat reads the Guide or the menu, so one seat can no
longer freeze the match: the pause screen says THE MATCH RUNS ON, because it
does. A stall is visible rather than inferred, in `status()`, on the field
and in `/state` (`stalled`, `peer_quiet_seconds`, `desync_tick`), and
`referee.py` prints it. A seat that is told to quit closes the session and
writes `last-match.replay.json`, and `match.sh stop` asks before it signals,
so an abandoned match is still watchable; every recording is the canonical
world, so both seats write the same file. The handshake carries a build
fingerprint hashed from the simulation's source (protocol 2), so two
different builds are refused before the match rather than desynchronising
during it, while saves and replays stay keyed by the rules digest alone. A
desync leaves `saves/desync-<tick>.json` behind. `relabeled_for` now names
every field of the world, so a new one stops it compiling, and its test gives
the two seats values that can be told apart. `match.sh` verifies both seats
answer before it says the match is up and always builds; `referee.py` names
each seat by the faction it reports.

## Evidence

An in-process two-peer test runs 240 ticks with commands from both seats and
ends with identical canonical worlds; a relabelling test proves the view
swaps every seat and keeps ids and positions; harness tests cover request
parsing, frame encoding and input routing. A live two-seat headless match on
this Mac ran in lockstep with inputs from both sides. The first two-agent
trial ran for 7 minutes 42 seconds of match time without a desync, with
both seats building and fighting; its findings and the fixes they produced
were fixed in the following passes; a local run writes every frame under
`output/agent-play/`.

## What the second trial changed

The second trial's eighteen findings are recorded with the trial under
`output/agent-play/trial-2/notes.md`. The fixes that followed, on top of
the ten made during the trial: alert cards moved into a second row of the
notice band, with one card per base under attack and cards for lost
buildings, sluice changes and a blocked door; new workers gather on their
own and move on when a wreck runs dry; a rally on a wreck sends new workers
to gather and a selected producer draws its rally flag; placement colours
each cell, rings the wells and snaps a condenser to one; wrecks are
labelled and take gather orders under the fog; the selection panel names
states; the result log names the winner; and the rules are version 5, with
Riveters and Looms hitting buildings harder so that a push which reaches a
base can finish it. Recordings made under rules 4 play only in a rules-4
build; `output/agent-play/brinewake-rules-v4` is kept for the trial-2
recording.

## The third trial

Two fresh seats played a rules-5 match with no action cap and each kept a
notes file as it went; the referee's samples, both notes files, the
recording and the findings are under `output/agent-play/trial-3/`. Union
won at 09:47 with a first push that took the sluice and walked on, and
the seats then wrote design proposals from play
(`design-thoughts-union.md`, `design-thoughts-assembly.md`). The fixes
that followed: the sluice card hides its SWITCH button until the station
is yours, F3 visits the newest unvisited red card first and far cards
carry a compass letter, effect words sit on a third button row, the
auto-gather reach is 40 cells (rules 6), enemy sites no longer say NO
BUILDER, wreck labels draw above machines, a refused condenser turns every
cell red, wells get beacon poles and the prompt points to the nearest one
off screen, tooltips for low buttons open downward, and the chart's view
rectangle is two-tone.

## The fourth trial's playbook

From the fourth trial the playbook is controls only: how to see, how to
act, what the screen regions are, and the lockstep lag. What the machines,
buildings, upgrades and the sluice do, a seat learns from the game, as a
new player would: the command card, the selection panel's role and stats
lines, the tooltips with their STRONG VS and WEAK VS rows, the cards and
the Guide. A seat's confusion is then a finding about the game, not about
the playbook.
