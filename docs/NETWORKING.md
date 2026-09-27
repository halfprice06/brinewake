# Peer-to-peer multiplayer

2026-09-23. The owner asked for peer-to-peer multiplayer: two people on
different machines find each other and play a match reliably. This note
records what was built, how to use it, how it was tested, and what could
not be tested here.

## Playing

**Host.** Main menu, PLAY ONLINE, HOST. A join code appears after a moment
(`BW-` and groups of four letters). COPY it and send it to the other player
in any chat. Pick a side; the other player gets the other one. START lights
when everyone playing is ready.

**Join.** Main menu, PLAY ONLINE, PASTE (or type the code, or an address
such as `203.0.113.7:47800`). A host's code joins at once. Pick a side,
READY.

**If the guest cannot get in** the guest's screen shows a reply code after
three seconds. The guest sends it back; the host presses ADD A REPLY, pastes
it and ADD. Both games then send towards each other, which opens most home
routers.

**If that fails too** one of two things will work: forward UDP port 47800 on
the host's router to the host's machine, or put both machines on the same
virtual network (Tailscale, ZeroTier) and join by that address.

**During the match** the top bar shows signal bars and the round trip in
milliseconds. If a player goes silent, a card names them with a countdown:
the match waits a minute for them, then they surrender. The host's card
has KEEP SEAT OPEN: the others play on at once and the silent seat is kept
for five minutes (see *Rejoining after a drop*). A player whose game
crashed can start it again, PLAY ONLINE, REJOIN (or JOIN with the host's
code under the same name), and carry on from where the host is. A goodbye (MAIN MENU then a new match, or QUIT) ends the match for
the others at once, with the leaver surrendering. If the host leaves a
two-player match the guest wins; the match cannot go on without the host
otherwise.

A third person (and more) can join a lobby. The map holds two sides, so
extra seats watch: they see the whole field and give no orders.

**Command line** (the agent harness uses this): `--host 127.0.0.1:4790`
waits for one guest and starts at once; `--join ADDRESS-OR-CODE` joins and
readies. `--delay N` fixes the input delay; `--net-sim
latency=80,jitter=30,loss=10` makes this seat's outgoing packets late,
reordered and lost, for testing on one machine.

## How it works

Code: `crates/bw_desktop/src/net/` and `net_ui.rs`.

**No servers.** The host is a hub. Guests send to the host only, and the
host passes each guest's orders and hashes on to the other guests. Only the
host has to be reachable.

**Reaching the host** (`lobby.rs`, `upnp.rs`, `stun.rs`, `code.rs`). The
host asks its router to forward UDP 47800 (UPnP), asks a public STUN server
which address its packets come from, and notes its local network address.
The join code carries a random room number and up to three addresses. The
guest tries them all at once. STUN servers only answer "your address is
…"; no game traffic goes near them, and without one the code carries the
local address alone.

**Hole punching.** A router lets a reply in only after its machine has sent
out to that address. The guest's attempts open the guest's router; the
reply code tells the host where to send, which opens the host's. Symmetric
NATs (some mobile and carrier networks) defeat this; forwarding or a VPN is
the answer there.

**The link** (`link.rs`). One UDP socket per game. Messages are numbered
and every packet acknowledges what arrived (a cumulative number and a
32-message window). Each packet also carries the small messages not yet
acknowledged, so a lost packet is repaired by the next one, a tick later,
not after a timeout; larger messages are resent on a timeout measured from
the round trip. A background thread acknowledges, resends and sends a
keepalive every 200 ms, which measures the round trip and keeps routers'
mappings open. A connection is named by a random id rather than an address,
so a player whose address changes mid-match carries on.

**Lockstep** (`session.rs`). As before: a seat's orders from tick `t` run at
`t + delay` on every seat, a tick steps only once every playing seat's batch
for it is in hand, and every thirty ticks each seat sends its state hash. A
mismatch stops the match on every seat with OUT OF SYNC and writes
`saves/desync-TICK.json` and the world as this seat had it. What changed:

- *Input delay follows the connection.* Each seat sizes its own delay to
  the longest path its orders travel (half the round trip plus twice the
  jitter plus 20 ms), measured every second: 2 ticks (67 ms) on a LAN, 4
  at a 150 ms round trip. It rises at once and comes down one tick at a
  time. Raising it fills the skipped ticks with empty batches; no seat has
  to agree.
- *Pace.* A seat that has run ahead of the others holds a tick now and
  then, so the delay's margin absorbs jitter instead of being spent
  waiting. Both seats begin at the same moment, give or take half a round
  trip.
- *N seats.* Nothing counts two. `MatchPlan::world` maps the seats to the
  map (`MAP_SIDES`, one function); a third faction and a three-player map
  change that function and the side rule in `protocol.rs`, not the
  session. Seats beyond the map's sides watch.
- *Dropping and rejoining.* The host decides when a seat is dropped: at
  once on a goodbye, after 60 s of silence otherwise. It names the first
  tick without the seat's orders and every seat has the dropped player
  surrender on that tick, so the worlds stay identical. Within the 60 s a
  restarted guest rejoins with the token it was given in the lobby; the
  host sends it the world as it stands and every batch still to come
  (`Resume`), and it steps on from there.
- *Busy is not gone.* Silence means the connection, not the orders: the
  link's own thread sends keepalives and acknowledgements while the game
  thread is inside a long tick, so a seat whose simulation crawls (trial 10:
  seconds a tick) is waited for ("behind", no drop clock) and never
  dropped. The host tells every guest each seat's connection silence with
  the round trips (`Paths`, protocol 6), so a guest shows the host's real
  drop clock for another guest rather than counting from its last batch.

## Rejoining after a drop (protocol 7)

2026-09-23, trial 10 round 2 (suggestion 10). Before this a seat that went
silent for a minute was dropped: it surrendered and could not come back,
and until then the whole match waited for it.

- **Keep the seat open.** While the match waits for a silent player the
  host's waiting card carries KEEP SEAT OPEN (5 MIN). Pressed, the others
  play on at once. The seat is not surrendered and not out; its machines,
  workers included, hold where they stand until its player is back
  (buildings finish what they were making). A card on every screen reads
  AWAY, the player's colour and faction, SEAT OPEN and OUT m:ss. After
  five minutes (`KEEP_OPEN`) the seat is dropped and surrenders as before.
  Without the press nothing changes: a minute of silence drops the seat.
- **How it stays in step.** The host stands in for the kept seat: it sends
  that seat's batches as it sends its own, empty but for one Hold order in
  the first. They travel and are relayed like any batch, so every seat
  applies the same orders on the same ticks and nothing else needs
  agreeing; no seat ever steps a tick of that seat without a batch. The old
  connection is cut, so a game that wakes cannot land orders on ticks the
  host has filled.
- **Coming back.** The returning game joins with the host's code: with the
  token from its rejoin note (REJOIN, now kept for 6.5 minutes), or with
  no token under its lobby name ("ANNE" also finds "ANNE 2" when only that
  seat is open; only a seat whose player is gone can be taken by name). The
  host stops standing in and sends `Resume`: the world as it stands, which
  carries the whole command log, so the returning seat's recording is the
  whole match, and every batch still to come. The seat's own batches follow
  on from the last one the host sent for it. This reuses the in-match
  rejoin path rather than `--resume`: sending the world is one message and
  needs no replay on the returning machine.
- **Refused.** A seat already dropped cannot come back ("Your seat was
  given up"). The host itself cannot be kept open: it is the hub.
- **Not done.** The returning seat's result screen and dock log start at
  its return (a `--resume` rebuilds them from the recording; this does
  not). A guest cut off by its own network rather than a crash sees the
  host fall silent and ends its session after a minute; it can come back
  the same way.
- **Tests.** `net::tests::a_crashed_guest_of_three_is_kept_open_while_the_others_play_on_then_rejoins_in_step`
  (three seats on the Confluence: a guest crashes, the host keeps it open,
  the other two play five seconds, the seat holds and has not surrendered,
  the guest comes back by name with no token, all three worlds and every
  exchanged hash agree, and the seat gives orders again);
  `a_seat_kept_open_is_dropped_when_its_time_is_up_and_cannot_come_back`;
  `net_ui::tests::the_host_keeps_a_silent_seat_open_from_the_waiting_card`.
  Live: `tools/net_test/three_seats_rejoin.py`, three headless release
  games on the Confluence (`--host`, two `--join`). Last run 9/9: the third
  seat killed at tick 220; the host named it and KEEP SEAT OPEN was
  pressed; the other two ran 298 ticks in 10 s while the guest's card read
  AWAY VIOLET COMPACT, OUT 4:49 and nobody was out; the game restarted
  with the same name and `--join` (no token) took seat 2 back; the hashes
  of all three agreed at every compared tick (630 to 840), no desync.

## Tested here

- `cargo test -p bw_desktop net` — 34 tests, most over real sockets: messages
  arrive whole and in order through 20 % loss and 30 ms of jitter; a
  punch opens the way for a guest; a changed address carries on; codes
  round-trip and typos are refused; a real-time match at 60 ms each way,
  up to 25 ms jitter and 10 % loss both ways stays in step with its worst
  lag behind the wall clock at one tick, the delay settling at 4; a third
  seat watches in step; a goodbye surrenders; a host leaving hands the
  guest the win; a silent guest is named and waited for, then carries on;
  a crashed guest rejoins with its token and plays on in step; a forced
  difference is caught as a desync on both seats; a world sent to a
  rejoining guest steps identically for 600 ticks.
- `tools/net_test/two_seats.py` — two real processes, headless, with
  `--net-sim` on both: orders from both seats, compared hashes, a cut and
  restored connection, one seat quitting. And the lobby driven through
  its buttons: host, copy the code from the state, type it into the other
  game, pick sides, ready, start. Last run (release build, 60 ms each way,
  20 ms jitter, 10 % loss on both seats, 40 s of play): 10/10 checks; 144
  hash pairs compared, none differed; 29.9 ticks a second; round trip 150
  ms and input delay 4 ticks on both seats; a five-second freeze named the
  frozen seat and both carried on in step; a killed seat restarted,
  pressed REJOIN and caught up (host at 1444, both at 1627, hashes equal);
  the seat that quit lost and the other won.

## Not tested here

- **Real routers.** This environment has no NAT. UPnP, STUN and hole
  punching are written to their specifications and tested against
  themselves on one machine (the STUN parser against a built answer, the
  punch between two local sockets), not against home routers. What to try:
  host on one home connection, join from another (a phone's hotspot is a
  different network); if the code does not get the guest in, try the reply
  code; if that fails, forward UDP 47800.
- **IPv6.** Codes carry IPv4 addresses only.
- **Long matches over the internet**, macOS and Windows clipboards, and
  the host itself crashing (the match ends; only guests can rejoin).
