# BRINEWAKE — design authority for the game as it stands

**Rules version 17 · simulation schema 2 · art pass v19 · revised 2026-09-23**

This document describes the game as it stood at rules version 17
(2026-09-23): the rules, the map, the interface and the engine, with the
intent behind them and the questions still open. Where a number appears it is
the number in `crates/bw_content/src/lib.rs` or `crates/bw_sim/src/lib.rs` at
that version. Rules changes since then (the Saltglass Compact, the Confluence
and rules 18–22) are summarised in §19 and §20 and live in the code; where
this document and the code disagree, the code is right.

Balance is still a hypothesis. Eight two-agent matches have been played and
recorded; no human has played a full match. Nothing below is a claim that the
game is fun or fair.

## 1. The game worth building

**Command rival reclamation fleets across the exposed floor of a vanished sea.
Take the sluice that moves the tide, dry a crossing for your own army or drown
the one the enemy needs, and turn that change into a production timing, a
raid, or a decisive fight.**

The important experience is deciding where to commit an army when both players
can see the next physical change but cannot see each other's intentions. A
good match creates moments like: "You dried the north crossing for your
armored push, so I flooded everything for forty-five seconds and landed five
Looms behind your line from a barge."

The setting is coastal industrial science fiction: salt terraces, drowned
civic infrastructure, pressure engines, repurposed tug hulls, and engineered
reed composites. The two societies disagree over how to inhabit the reclaimed
land. Neither is the monster faction or the objectively correct faction.
Machines look built and maintained, with readable joints and working tools,
rather than like existing franchise units.

The StarCraft inspiration is the quality of army control, information,
production timing, asymmetry, and comeback through better decisions. The
familiar worker/production/control-group vocabulary helps players learn; the
game explains its own machines the way StarCraft and Command & Conquer do,
through a command card, a selection panel and structured tooltips, because two
blind agent seats proved it could be learned that way. The tide and the
reclamation machinery carry this game's own identity. The title remains
provisional, with naming research before commercial release.

### Design pillars

| Pillar | Player consequence | What we remove if it conflicts |
| --- | --- | --- |
| A readable map that players can change | Predict a route before committing an army | Random lethal weather, hidden water rules, constant terrain churn |
| Commit, then exploit | Heavy positions and mobile flanks both have opportunity costs | An all-purpose unit or an always-correct tide setting |
| Economy with visible tradeoffs | Expansion, reinforcements, upgrades and pressure spending compete | Duplicate worker chores, upkeep taxes, arbitrary extra resources |
| Small pixels, clear battles | Recognize faction, role, ownership, and threat at ordinary desk distance | Decorative noise that hides silhouettes or selection feedback |
| Precise and trustworthy controls | Orders work, fights are explainable, replays reproduce failures | Teleport fixes, AI omniscience, invisible command rejection |
| The game teaches itself | A new player learns a machine's job, cost and counters from the interface | Rules that exist only in a manual, a playbook or the source |

### Target experience and scope

- Desktop mouse and keyboard. macOS on Metal is the implemented and verified
  target; Windows and Linux are planned and untested.
- The core is 1v1 skirmish: two factions, mirrored starts, one map, and a
  practice opponent. A trusted two-peer lockstep session exists and is what
  the agent trials play over.
- Delivered: an offline match against the built-in opponent, a seven-step
  practice path, a trusted-LAN lockstep match, a headless harness two agents
  play through pictures and input, live recording, and a fog-lifted observer.
- A full first release candidate still wants three maps, a stronger opponent,
  human playtesting, and platform hardening.
- Campaign, heroes, air combat as a layer, procedural worlds, destructible
  terrain, matchmaking, ranked service, mod distribution, and a universal
  engine editor remain out of scope. None is required to prove this game.

## 2. Match loop and victory

1. Assign workers and choose the first production investment.
2. Scout the opponent's composition, expansion, tier-two building and
   route commitment.
3. Decide between more economy, an army timing, a contest for the sluice, or
   a defensive position.
4. Take the sluice and set the tide for the plan you intend to run.
5. Fight, disengage, reinforce, upgrade, or trade an exposed expansion for
   another advantage.
6. Win by breaking the enemy's base, or by holding both crossings.

### The two victories

**The base.** Destroying the enemy's headquarters wins. It is a siege, not a
snipe: a headquarters cannot fall below a quarter of its hull while a finished
Works of its owner stands (`KEEL_PERCENT`, 25). A push must take the Works
first, which gives the defender somewhere to fight and something to see
coming. Headquarters cannot move or be rebuilt after destruction. A
simultaneous destruction is a draw; surrender is available.

**The tide.** A player who owns the sluice and holds both crossings for
ninety seconds of gauge wins: "{FACTION} HOLDS THE TIDE"
(`TIDE_HOLD_TICKS`). A lane is held when the sluice is yours and an own gun
machine or Caisson stands within four cells of either of that lane's two bank
mouths with no enemy one at either mouth (`holds_lane`,
`CROSSING_HOLD_RADIUS_CELLS`). The gauge rises a tick for every tick both
lanes are held and falls three ticks for every tick they are not
(`TIDE_HOLD_DRAIN_PER_TICK`, one before rules 21), so a hold that is broken is
a setback and not a reset, and a break held for a while takes back real
time. All four mouths are lit for both players
at all times, so a count never starts out of the defender's sight.

With three seats (the Confluence, rules 22, from trial 12) an enemy Defense
Nest in a mouth ring of a counting lane no longer stops the count: it halves
its rate, and only an enemy gun machine stops it (`hold_slowed`). A DRY on the
Confluence ebbs back to neutral 180 s after it lands (`DRY_EBB_TICKS`), with
the usual warning. The Sounder's +5 applies to a deployed Heliostat as well as
a deployed Loom, and Loom shells do 75% to Sounders and Glinters
(`HUNTER_LOOM_SHELL_PERCENT`).

A running count is announced to both sides, not left on a card (rules 13,
from the eighth trial). A banner across the top of the field carries the
seconds left in large figures: red with ENEMY WINS IN while the enemy counts,
blinking in the last thirty seconds; gold with YOU WIN IN for your own; muted
while a broken count drains. Clicking it, or F3 during an enemy count, centres
on the mouth that stops the count, and the chart flashes the held mouths. The
route card lists the rule as a checklist (whose sluice; per lane, an own gun
and an enemy at a mouth), and an enemy gun blocking a lane you could hold is
marked BLOCKS HOLD on the field. A recording shows no banner, as it speaks
for neither side; its spectator bar carries both counts.

The 2026-09-12 plan said there would be no timer or control-point victory, and
that turtling should be tested before an additional victory system was
invented. The test happened. In the second trial a decision needed a
crew-capped army thirty-four minutes in because both early pushes died on
defended bases; the third trial's two seats, writing design proposals from
play, independently asked for a headquarters kill that is a siege rather than
a snipe. The keel rule and the tide victory are that answer, and the tide is
not a timer: it is a position that must be taken and held against an opponent
who can see it. The sixth trial was decided by it at 73:21 after four counts
had been broken by fighting. The seventh then spent forty minutes unable to
reach it, because the rule did not include the station and both sides simply
stood on their own bank denying the other; rules 12 put the station into the
condition so that the lanes and the island are one objective and the fight has
a place. The eighth trial (2026-09-22, AI seats and an AI referee) was
decided by the tide at 20:58 with neither headquarters attacked: the rule
worked, but both seats misread it for most of the match and the Assembly
found the enemy's 78-second count by luck. The referee argued against
changing the rule until the defender could see it, so rules 13 keeps the
ninety seconds and the drain as they were and makes the count loud instead.

### Intended match arc — hypotheses, not measured pacing

| Time | Meaningful decision | Readable evidence |
| --- | --- | --- |
| 0–2 min | Worker allocation, first production, scouting direction | Opponent production or expansion can be discovered |
| 2–6 min | First skirmish, safe versus exposed expansion, first upgrade | Small armies can retreat; one mistake is recoverable |
| 6–12 min | The contest for the station, the Drydock, and the second army composition | Both tide settings can serve a sensible plan |
| 12–25 min | Coordinated pressure on two crossings, or a transport raid | Economy, upgrades and scouting matter as much as raw army size |
| 25 min+ | Breach, counterattack, a tide count, or a last successful expansion | Players can explain what created the decisive advantage |

The 15–25 minute target is not met. Observed agent matches: 09:47, 14:41,
34:17, 61:47, 73:21, and one stopped at 88 minutes with no result. The short
ones were routs and the long ones were stalemates broken by the tide rule.
Pacing is the open balance question, not a solved one.

## 3. The signature system: the sluice and the tide

### What the tide is

One capturable control station sits on a salt island in the middle of a lake
that splits the map. Four strips cross the lake and are tidal: the north lane,
the south lane, the north rim (the path around the top) and the south rim (the
path around the bottom). Each strip belongs to a side — north lane with north
rim, south lane with south rim — and the station's holder decides what state
that side is in.

| Tide | North side | South side |
| --- | --- | --- |
| Neutral (every match starts here) | shallow | shallow |
| Open north | dry | deep |
| Open south | deep | dry |
| Flood | deep | deep |

- **Dry** is a road at full speed.
- **Shallow** is wading: slow by movement class, 125% damage taken
  (`WADING_DAMAGE_PERCENT`). Tracks halves the slow; a Dredger wades at full
  speed.
- **Deep** cannot be entered by a ground machine at all. This is the wall: at
  full flood there is no way across the basin without a hull or wings.

A ground machine standing on a cell that turns deep is **swamped**: 40% speed,
150% damage taken, and the tide immediately orders it to the nearest ground it
can stand on (`EventKind::Swamped`). The Dredger treats deep tidal water as
shallow. Wrecks on shallow or deep cells can be gathered only by a Dredger;
on a dry cell anyone may work them.

The neutral tide returns only with a new match.

### The station

| State | Rule |
| --- | --- |
| Neutral | Either player can claim the station; the tide is public from the first tick |
| Capturing | Own machines that are neither workers nor buildings and are within two cells of the station contribute work; up to four count, so four machines take ten seconds where one takes forty (`CAPTURE_WORK`, `CAPTURE_MAX_WORKERS`). The first capture of the neutral station takes half as long again, sixty seconds alone or fifteen with four (`NEUTRAL_CAPTURE_PERCENT`, rules 15), so the side that loses the opening race still has time to contest it |
| Contested | An enemy machine that is likewise neither worker nor building, within four cells of the station (`CAPTURE_CONTEST_RADIUS_CELLS`, rules 17; two before), halts the work; the four-cell ring is drawn on the ground in the capturer's colour while a capture runs; neither side gains progress and ownership never flips on its own |
| Owned | The holder sees every tidal cell, earns 15 pressure a minute, and may set the tide when the station is not locked |
| Warning | DRY N or DRY S announces the new state to both players for ten seconds; FLOOD announces for five. A switch is refused, with the reason and at no cost, while an enemy gun machine stands within two cells of the station; one that arrives during the warning cancels it and the price comes back (rules 13) |
| Transition | Every tidal cell changes together on the announced tick; water art animates around it, but path cost and the overlay use the authoritative state |
| Locked | Forty-five seconds before the tide can be set again. Taking the station during the lock does not cancel or reset it |

Leaving the area for two seconds resets incomplete capture progress. A machine
that is surging cannot channel. Drying a side costs 40 pressure
(`SWITCH_PRESSURE`); a flood costs 80 (`FLOOD_PRESSURE`) and lasts forty-five
seconds, after which the tide falls back to the side that was last open, or to
neutral if none ever was, and the lock runs from there.

### Movement classes in shallow water

| Movement class | Dry lane / land | Shallow | Deep, if the tide catches you there | Example |
| --- | ---: | ---: | --- | --- |
| Wheel | 100% | 55% | swamped (40%) | Riveter, Bulwark, Caisson, Tidewatch |
| Walker | 100% | 75% | swamped (40%) | Reedguard, Loom, Sounder, Lampwright |
| Paddle | 100% | 90% | swamped (40%); the Dredger takes it at 90% by design | Skipper, Dredger |
| Hull | — | 100% | 100%; water is the only ground it has | Barge |
| Air | 100% | 100% | 100% over everything but rock | Lifter |

Tracks (Drydock) halves the shallow-water slow for your side. Silt costs a
little: 95%.

### Orders during a change

The path preview shows the selected group's slowest movement class; the route
card names the current tide, counts the warning and the running flood, and
carries the hold row for both sides. Units continue their current movement
segment and affected routes are repriced on the scheduled tick; replanning is
bounded and deterministic. A destination in deep water or rock resolves to the
nearest ground within twenty-four cells rather than being refused; a machine
with no path stops after ten seconds of blocked traffic and the band says "No
way through to that point" instead of leaving the order silently unexecuted.

Redundant cues: dry is tan with exposed crossbars, shallow is teal with ripple
bands, deep is the lake's own water, a warning has a chevron border and a
countdown, a drained lane dries through four stages over a minute and a flood
rolls a foam front out from the spine. The station and the tide are public map
infrastructure from the first tick. Scouting conceals armies, production and
intentions, not a physical rule.

### Falsification test

The original test — compare the same scenarios with the sluice fixed and
controllable, and abandon it if every player leaves it alone or one setting is
always correct — has partly run itself in the trials. Blind seats found the
tide controls within minutes of taking the station, the station changed hands
five times in one match, and one fight was decided by opening a lane under the
enemy's army. What has not been shown is that the settings are balanced
against each other, or that a player who ignores the station can still play.
Keep watching for: a side that always wants the same setting, and a match in
which nobody contests the island.

## 4. Economy, production, and growth

### Two resources

| Resource | Acquisition | Spending | Strategic vulnerability |
| --- | --- | --- | --- |
| Salvage | Workers gather finite wreck beds and return loads to a headquarters or salvage yard; Dredgers work drowned wrecks | Workers, buildings, machines, upgrades, repair | Hauling distance and raids; a spent bed forces an expansion or a fight for the lane wrecks |
| Pressure | A headquarters trickle, condensers on fixed wells, and the sluice | Specialists, upgrades, doctrines, the tide, VENT, SOUND, Surge | Exposed condensers and a station that pays its holder |

Pressure is stored and spent, not a continuous power requirement. Crew is an
army limit rather than a gathered resource. There is no third resource.

### The numbers, at rules 15

- **Start:** a headquarters, six workers, 360 salvage, 40 pressure, crew cap
  40. Starts are mirrored at cells (10,64) and (117,64).
- **Gathering:** two seconds at the wreck, then the trip. A worker carries 5,
  a Dredger 8; Cranes adds 2, the Hauling doctrine 3, and its second tier 3
  more.
- **Wreck beds:** home beds 1,600 each, side beds 2,400, four lane wrecks of
  900 at the crossing mouths, two deep wrecks of 1,500 on the sluice island.
  A destroyed combat machine leaves a wreck worth half its salvage cost for
  whoever holds the ground, or all of it when the killer's side has Scrap
  Recovery. One that falls within six cells of a crossing mouth leaves half
  of that (`MOUTH_SCRAP_RADIUS_CELLS`, `MOUTH_SCRAP_PERCENT`, rules 15): in
  the eighth trial seven Bulwarks died at the Assembly's mouth and their
  wrecks took its income from about 470 to 1,454 a minute, the swing that
  decided the match. A failed push at a mouth still pays the defender, but
  a quarter of the army's cost rather than half.
- **Flotsam (rules 15):** when a flood falls, each of the four lane wrecks
  regains 200 salvage, never above its starting 900 (`FLOOD_FLOTSAM`). The
  flood falls back to the side that was last open, so the station's holder
  decides which pair of refilled wrecks dries first.
- **Crew cap:** 40, plus 12 per finished Works, 6 per salvage yard and 10 per
  Drydock, to a ceiling of 90. The refusal message names the ceiling and the
  buildings that raise it.
- **Pressure cap:** 150, plus 100 per condenser, to 450; Overpressure adds
  100 on top of the base.
- **Pressure income:** 60 a minute from the headquarters, 120 per condenser,
  15 for holding the sluice, and 60 more with Bleed Valves.
- **Overflow (rules 15):** income that arrives at a full bar becomes salvage,
  three for every ten pressure (`OVERFLOW_PRESSURE`, `OVERFLOW_SALVAGE`),
  half RECLAIM's rate. Pressure sat at its cap in 12 of 22 samples for Union
  in the eighth trial; a full bar is now slow salvage rather than a dead
  number, and spending it or RECLAIM still pays better.
- **Pressure sinks:** the tide (40, or 80 to flood), Surge (10 a machine),
  SOUND (20), VENT (100), RECLAIM (100 pressure for 60 salvage, at a finished
  headquarters, no cooldown), specialists, upgrades and doctrines.
- **Workers:** 50 salvage, 12 seconds, one crew. A new worker with no rally
  joins the nearest wreck with salvage within forty cells, a rally within two
  cells of a wreck sends new workers to gather there, and a worker whose wreck
  runs dry moves on. Workers never pick a tidal or island wreck on their own.
  A rally on the island, on a lane wreck or across the lake is set with a
  warning saying where the workers will walk.
- **Workers under fire (rules 13):** a worker that is hit marks the ground for
  its side for twenty seconds, six cells around (`WORKER_DANGER_TICKS`,
  `WORKER_DANGER_RADIUS_CELLS`). The hit worker and every worker of its side
  gathering a wreck inside the mark go to the nearest safe wreck from their
  drop-off, or back to the drop-off when there is none, and a WORKERS PULLED
  BACK card says so. No worker picks a marked wreck on its own and a rally on
  one sends new workers to a safe wreck instead; an order still sends a worker
  in. Worker deaths get their own WORKERS LOST card with the cause.
- **Construction and refunds:** cost is paid when queued; cancelling before
  work begins returns 100%, and 75% once it is under way. One worker builds a
  building and extra workers do not speed it up. Destroyed investments refund
  nothing. Repair costs salvage and is bounded; a Caulker or Tender repairs
  the nearest damaged own machine within three cells at 5 hull a second for 1
  salvage per 5 hull.

### What makes this economy strategic

The home beds are deliberately small, so the mid-game glut the third trial
found is now a reason to leave the base: the side beds are twenty-eight cells
out, the lane wrecks sit at the crossing mouths where armies meet and wait for
a dry tide unless a Dredger comes, and the deep wrecks are on the island
itself, which means crossing a lane and giving the order yourself — a worker
never picks a tidal or island wreck on its own. Pressure has somewhere
to go after the first condenser — upgrades, the tide, VENT, RECLAIM — so a
full pressure bar is a decision not to spend rather than a dead number.
RECLAIM lets a side whose salvage is gone rebuild from pressure, which the
seventh trial's Union seat lived on for forty minutes.

## 5. Factions, roles, counters, and tech

Both sides have a worker, three roles from the Works, and four from the
Drydock. Every number below is `bw_content::spec`.

### Breakwater Union

Dock crews turn heavy reclamation equipment into a coordinated front. Broad
bases, open gantries, off-center cranes, large flywheels, replaceable armor
plates. Cream and orange materials; machinery is grounded and visibly bears
weight. On dry ground the Union is the heavier side: the Caisson plugs a
crossing, Tracks keeps its wheels moving in the wet, and the Riveter unpicks
buildings.

| Machine | Hull | Damage | Range | Speed | Cooldown | Cost | Build | Crew | Sight | Move | Role |
| --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | --- |
| Hook | 65 | 3 | 1 | 3 | 30 | 50S | 12s | 1 | 7 | Wheel | Worker |
| Riveter | 125 | 12 | 4 | 3 | 24 | 70S | 15s | 2 | 8 | Wheel | Line fighter; 150% to buildings, 200% with Siege |
| Bulwark | 300 | 10 | 3 | 2 | 32 | 140S 30P | 24s | 4 | 8 | Wheel | Deploys to a fixed forward shield and arc |
| Sounder | 85 | 7 | 5 | 5 | 27 | 80S 10P | 16s | 2 | 12 | Walker | Fast harasser; carries SOUND |
| Tidewatch | 60 | — | — | 6 | — | 40S | 10s | 1 | 16 | Wheel | Scout; sight doubled on Hold; trained at the HQ (W) or the Drydock |
| Caulker | 80 | — | — | 3 | — | 90S 20P | 16s | 2 | 8 | Wheel | Mender |
| Caisson | 500 | — | — | 2 | — | 200S 60P | 24s | 3 | 6 | Wheel | Deploys to block a cell; counts at a crossing mouth |
| Lifter | 260 | — | — | 3 | — | 260S 80P | 40s | 3 | 10 | Air | Transport; flies over everything but rock |

### Silt Assembly

Reclamation cooperatives build light articulated machinery from woven
composites, dark flexible joints, and warm pressure chambers. Open arches and
paddles provide clear negative spaces. The Assembly is the tide-and-siege
side: the Dredger works drowned wrecks, the Loom reaches, and the Barge
crosses water the Union has to fly over.

| Machine | Hull | Damage | Range | Speed | Cooldown | Cost | Build | Crew | Sight | Move | Role |
| --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | --- |
| Wick | 65 | 3 | 1 | 3 | 30 | 50S | 12s | 1 | 7 | Walker | Worker |
| Reedguard | 150 | 10 | 4 | 3 | 27 | 80S | 16s | 2 | 8 | Walker | Durable line unit |
| Skipper | 100 | 9 | 4 | 5 | 25 | 90S 10P | 17s | 2 | 10 | Paddle | Scout and flanker; carries SOUND |
| Loom | 160 | 32 | 7 | 2 | 72 | 145S 40P | 26s | 4 | 10 | Walker | Artillery; must deploy to fire (reach 8 before rules 21) |
| Lampwright | 60 | — | — | 6 | — | 40S | 10s | 1 | 16 | Walker | Scout; sight doubled on Hold; trained at the HQ (W) or the Drydock |
| Tender | 80 | — | — | 3 | — | 90S 20P | 16s | 2 | 8 | Walker | Mender |
| Dredger | 180 | — | — | 2 | — | 120S 30P | 20s | 2 | 8 | Paddle | Gathers wet wrecks at full speed; carries 8 |
| Barge | 400 | — | — | 4 | — | 220S 60P | 40s | 3 | 8 | Hull | Transport; water only |

### How the machines argue

- Riveters beat Skippers in a committed open fight; Skippers and Sounders
  punish isolated workers, menders and exposed specialists.
- A deployed Bulwark holds one angle: its shield and its fire are locked to
  the front while deployed, so a player must pack, face and redeploy to cover
  another. Flank it.
- A Loom must be deployed to fire, has a two-cell dead zone, and commits its
  shot to a ground point with a 0.8-second warning; the point does not follow
  the target. Spread, dodge, or close inside two cells. Looms are blind inside
  two cells and the tooltips say so: SURGE (Z) to close.
- Reedguards buy the time a Loom line needs without winning a fight alone.
- A Caisson deployed in a crossing is a plug that also counts as a holder of
  the mouth.
- A mender behind a line changes what a trade costs; focus it.
- A transport answers the wall: a barge landed five Looms behind the Union
  line in the seventh trial, the first transport raid of any match.

A counter should confer an advantage, not make the opposing player's control
irrelevant. No global hitstop or camera shake. Weapon animation follows
authoritative firing events.

### Abilities

| Ability | Who | Cost | Effect |
| --- | --- | --- | --- |
| Deploy (D) / pack (P), keep deployed (K) | Bulwark, Loom, Caisson | — | One second each way; D and P are separate orders and never toggle. A move order packs first and sends the machine when its legs are free, unless K marked it to stay deployed (off by default) |
| Surge (Z) | Any fully mobile combat machine | 10 pressure each | 3 seconds at +50% speed with weapons and deployment off; 12-second cooldown. In a mixed group only the able machines surge and pay; a capture order given while surging starts on arrival |
| VENT (X) | Headquarters | 100 pressure | Every own combat machine fires 30% faster for ten seconds |
| SOUND (X) | Sounder, Skipper | 20 pressure | Lights nine cells around the machine for five seconds (six before rules 15), so a Sounder can find a Loom line that outranges its line's sight |
| Loom fire | Loom | — | Every shot lights the Loom for the side it fires at, two cells around it for three seconds (`LOOM_FIRE_REVEAL_TICKS`, `LOOM_FIRE_REVEAL_RADIUS`, rules 13); a broken line runs from each incoming shell back to it, and a LOOM FIRE FROM card with a compass letter goes to F3 |
| RECLAIM (R) | Finished headquarters | 100 pressure | 60 salvage at once, no cooldown |
| BOARD (E) / UNLOAD (U) | Transports | — | Four machines; boarding within two cells, unloading onto free ground within three. Cargo is not drawn (the transport carries a rider count), not seen, not hit, takes no orders, and sinks with its transport |
| Formation (F) / Face (R) | Combat machines | — | Compact, Line, Loose destination spacing; Face stops and turns without walking |

A Barge needs water within twelve cells of its Drydock to launch, which makes
where the Assembly puts that building a decision rather than a formality.

### Buildings

| Function | Union | Assembly | Hull | Cost | Build | Rule |
| --- | --- | --- | ---: | --- | ---: | --- |
| Headquarters | Dockhouse | Hearth | 2400 | — | — | Workers and the scout, pressure trickle, doctrines, VENT, RECLAIM, a gun; the defeat target, with the keel rule |
| Basic production | Works | Loomyard | 1000 | 180S | 20s | The three Works roles; Plate, Siege and Temper; +12 crew cap; keeps the keel |
| Drop-off | Salvage yard | Sorting cradle | 600 | 100S | 14s | Shortens hauling; Cranes, Salvage Sonar, Scrap Recovery; +6 crew cap |
| Pressure | Condenser | Condenser | 650 | 140S | 20s | Must sit on a well; Overpressure and Bleed Valves |
| Tier two | Drydock | Drydock | 900 | 250S | 30s | Scout, mender, tide role and transport; Tracks and Refit; +10 crew cap; the scoutable signal that a side is going to tier two |
| Defense | Defense nest | Defense nest | 1000 | 180S 30P | 22s | 18 damage at 8 cells, sees 10; fires on its own |
| Wall | Palisade | Palisade | 400 | 40S | 6s | One cell, no gun; blocks movement like any building |

### The tech tree

Upgrades run one at a time per building, and since rules 15 up to two more
wait behind the running one, paid when queued and refunded whole if cancelled
(`UPGRADE_QUEUE`); the eighth trial's Assembly met "AN UPGRADE IS ALREADY
RUNNING HERE" with 3,000 salvage banked. They complete for the
whole side, update existing machines in place, and refund 75% if cancelled.

| At | Upgrade | Cost | Time | Effect |
| --- | --- | --- | ---: | --- |
| Works | Plate | 120S 40P | 40s | Bulwark, Reedguard and Riveter hull +25%, for machines trained after it too (before rules 15 only the ones already in the field got it; the Riveter since rules 17) |
| Works | Siege | 120S 40P | 40s | Union: Riveter damage to buildings 200%. Assembly: Loom reach +1 cell (eight since rules 21) and deploys in half the time |
| Works | Temper | 300S 40P | 60s | Every combat machine deals 10% more damage (rules 15) |
| Drydock | Tracks | 150S 50P | 40s | Shallow water slows your machines half as much |
| Drydock | Refit | 300S 40P | 60s | Every combat machine carries 15% more hull (rules 15) |
| Yard | Cranes | 100S 20P | 30s | Workers carry 2 more |
| Yard | Salvage Sonar | 80S 40P | 30s | Every wreck shows its salvage on the chart, seen or not |
| Yard | Scrap Recovery | 120S 20P | 30s | A combat machine you destroy leaves a wreck worth its full cost |
| Condenser | Overpressure | 60S 40P | 30s | Pressure cap +100 |
| Condenser | Bleed Valves | 100S 40P | 30s | Pressure income +60 a minute |
| Headquarters | Overhaul I, II, III | 1,000S / 1,500S / 2,000S | 45s each | Each level adds 6% of base hull, rounded, to every machine the side owns, those already built included; buildings unchanged. In order, salvage only, outside any doctrine lock (rules 21) |

The headquarters chooses one doctrine for the match — **Hauling** (workers
carry 3 more) or **Fire Control** (combat cooldowns at 85% of base, rounded
up) — for 120 salvage, 60 pressure and 30 seconds, during which worker
production at that headquarters is paused and its queue preserved. A second
tier opens once the first completes, for 200 salvage, 120 pressure and 45
seconds: Hauling II adds 3 more cargo and doubles repair speed; Fire Control
II adds 50% to nest range. Both doctrine buttons show the second tier's
price and effect before the choice (rules 15).

TEMPER and REFIT are the late sinks the eighth trial asked for: salvage that
needs no crew, for a side whose crew is at the ninety ceiling and whose bank
keeps growing. They are deliberately expensive and deliberately plain.

The Loom and Reedguard pair killed 66 of Union's 69 machines in the eighth
trial, 45 of them by Loom, for 11 losses. Rules 13 made a firing Loom visible
and rules 14 gave the Bulwark the same attack-move stance, so rules 15 trims
reach rather than damage: a sieged Loom now outranges a Union line's sight by
one cell instead of two, a Sounder's SOUND reaches nine cells to find it, and
the Reedguard gives back ten hull because fixing Plate for newly trained
machines raised every later Reedguard by 25%. One match with Union mistakes in
it is thin evidence; the ninth trial is the check.

The ninth trial answered: Reedguards with Plate killed 47 of Union's 74
machines while Looms fell to 18. Plate covered the Assembly's whole main line
but only Union's Bulwark, so rules 17 extends it to the Riveter and takes the
Reedguard to 150 hull (187 with Plate). The Loom numbers stay.

The shape of the tree is deliberate: headquarters → Works → Drydock, with
upgrades spread over the four buildings so that an upgrade is a scoutable
building rather than an invisible percentage.

## 6. The map: The Split Basin

One map, 128×128 simulation cells, mirrored east–west. The art is a 2:1
dimetric projection over an orthogonal simulation grid; the diagonal look does
not change movement mathematics.

| Feature | Where |
| --- | --- |
| Border sea | The outer three cells on every edge |
| The lake | x 52–75, y 10–117, deep and permanent |
| North lane | y 47–51 across the lake, tidal, north side |
| South lane | y 77–81 across the lake, tidal, south side |
| North rim | y 3–9, the path around the top, tidal, north side |
| South rim | y 118–124, the path around the bottom, tidal, south side |
| Sluice island | x 62–66, y 52–76, salt ground in the lake; the station is at (64,64) |
| Silt | Columns x 19, 38, 76, 95 and 114 between y 30 and 95; 95% speed |
| Starts | Headquarters at (10,64) and (117,64), six workers each |
| Home wrecks | (16,57) (16,71) (112,57) (112,71), 1,600 each |
| Side wrecks | (34,40) (39,42) (39,86) (34,88) and their mirrors, 2,400 each |
| Lane wrecks | (54,49) (73,49) (54,79) (73,79), 900 each, at the crossing mouths |
| Deep wrecks | (64,56) (64,72), 1,500 each, on the island |
| Wells | (19,69) (108,69) (44,40) (84,40) (44,88) (84,88) |
| Crossing mouths | (50,49) (77,49) for the north lane, (50,79) (77,79) for the south |

There is no permanent dry causeway any more. Water pass 3 made the rims tidal
so that a full flood closes every route: the map's promise is now "there is
always a legal route unless someone spends 80 pressure to take it away for
forty-five seconds, and the answer to that is a transport."

`Terrain::Rock` exists in the rules and the navigation treats it as the one
thing a Lifter cannot fly over, but this map has none: the two pillars the
generator means to place beside the station at x 61 and x 66 are claimed by
the lake and the island first, so the branch never fires. Nothing depends on
it today; a second map, or a deliberate obstacle beside the station, is what
would make it matter.

Construction is refused on tidal ground, the sluice approach, unexplored
ground, occupied cells, and for a condenser anywhere but a well; the placement
plate names which of those it is, in two words, and colours each footprint
cell for itself.

## 7. Control and interface design

Genre conventions: left-click selection, drag select, right-click context
order, attack-move, stop, hold, shift-queued waypoints, control groups, rally
points, minimap navigation and orders, double-tap group focus. Bindings are
listed in the Guide and in `README.md`. An order click gives immediate
feedback even though execution happens on the next tick, and a command that no
longer validates when its tick arrives says so rather than disappearing.

Selection favors combat machines when workers share a drag box, with Shift to
include workers. Double-click selects matching visible friendly machines.
Point clicks find a machine up to eleven pixels off, and a machine hidden
behind a building keeps a half-transparent ghost and stays targetable on the
ghost's core; elsewhere on the building the building wins. The label over an
own construction site (NO BUILDER, QUEUED) picks the site, so a worker's
right-click there resumes it.

Placement shades the buildable cells around the cursor and keeps a gold ghost
of each site placed in the last five seconds, so a second click on the same
spot asks again before placing a second site. Own machines standing on a new
footprint step off it rather than refusing it. Shift-placing, or placing with a
worker that is already building, queues the new site behind its current one.
F7 selects every worker; Ctrl with a building key (B, C, Y, V, N, P, H)
selects every building of that kind. A rally applies to every selected
producer, the Drydock included; without one, new machines muster three cells
out from the door.

### The window and its regions

The final frame matches the physical client area; there is no fixed canvas and
no downsampling. The world draws at any scale from 1× to 3×, independent
of interface scale; 1×, 2× and 3× are exact blocks and scales between them are
resampled with one-pixel sharp edges (see Display mathematics). The window opens at 1920×1080 subject to the
desktop, with a 960×540 minimum.

The interface scale is 2 at 1280×720 and above, doubled again on a two-to-one
display, never more than the window holds of the 640×360 design, and fixed by
the INTERFACE setting (AUTO, 1X–4X) when it is not AUTO. Every region below is
in interface pixels multiplied by that scale.

| Region | Height | Function |
| --- | --- | --- |
| Header | 32 | Resources with their rates, crew, the clock, the menu |
| Battlefield | the rest | Native sprites at integer 1×, 2× or 3× |
| Notice band | two rows | The order prompt on the first row; alert cards and the control-group bar on the second. Nothing is drawn over the field but the corner controls |
| Command dock | the remainder of 142 | Basin chart, selection panel and roster, production, the command card |

### How the game explains itself

Modelled on the StarCraft command card and the C&C sidebar, because two blind
agent seats had to learn the game from it and did.

- **Command card:** a three-by-three grid in the dock holding every order for
  the selection — builds, trains, upgrades, formation and facing included —
  each button with a name, a cost row and an effect row.
- **Selection panel:** portrait, name, hull, a stats line, a role line and the
  machine's current state (IDLE, GATHERING, BUILDING, REPAIRING, MOVING,
  ATTACKING, HOLDING, CAPTURING, DEPLOYING, PACKING, DEPLOYED), with a roster
  that can keep or drop a type and pages when the selection is diverse.
- **Tooltips:** one structure — name, cost and time, role, STRONG VS, WEAK VS,
  key. They open inside the dock or downward and never cover the field.
- **Cards** in the notice band for what a player cannot be watching: base and
  machines under attack with the cause named, buildings lost, completions
  merged and counted, the sluice captured or threatened, the tide changed,
  ENEMY SEEN with a count and compass letter, crossings held or contested, a
  wreck emptied, a blocked producer door.
- **The route card** names the tide, counts warnings and floods, offers the
  holder DRY N, DRY S and FLOOD, and carries a permanent hold row: an N and an
  S chip coloured by who stands at each lane's mouths and, between them, the
  gauge of whichever count is running, and under it the hold checklist: a
  SLUICE chip, then for each lane a GUN chip (an own gun at a mouth) and a
  FOE chip (an enemy one). A player standing at a mouth without the station
  reads TAKE THE SLUICE TO HOLD A LANE, and a broken count reads YOUR COUNT
  DRAINS or ENEMY COUNT DRAINS. The hold banner over the field is described
  under the tide victory.
- **The chart** is a basin chart: tone contours, hatched soundings, a dithered
  sight rim, every well plated, every map wreck inked, last-seen enemy
  buildings kept as marks, mouth diamonds that turn red for an enemy hold.
- **The Guide** (F1) has eight pages: BASICS, ECONOMY, ORDERS, SLUICE, COMBAT,
  CONTROL, TECH and ROSTER, the last showing one faction at a time.
- **Practice** is seven guided steps with the opponent's attacks off; New
  skirmish is an ordinary match.

PACE (0.5× / 1× / 1.5×) changes field time in a one-player match and is fixed
at 1× in a session. F3 visits the newest unvisited alert, or first the mouth
that stops an enemy count; F4 the Works, I the idle workers, F2 the army
except machines on HOLD at a crossing mouth, so select-all never pulls a hold
apart; selecting never moves the view and Ctrl centres it. Shift-train queues
up to five in a two-player match as it does alone.

## 8. True pixel art and widescreen

### Art direction contract

- Author final sprites and tiles on a native pixel grid in editable Aseprite documents. Preserve semantic layers, named animation tags, original dimensions, and ground anchors.
- No downsampled paintings, high-resolution generated frames, or pixelation filters may be presented as finished native sprite art.
- 2:1 dimetric ground edges and consistent upper-left key light. Restrained ground-contact shadows are permitted for non-building assets. The simulation is 2D; visual height is authored metadata.
- **No shadows beneath buildings.** The owner removed them because they made buildings appear to float. Do not add or restore baked or renderer-generated oval, drop, cast or ground-contact shadow patches beneath any building, including construction/activity states and previews. This also covers fixed sluice structures and wells. Ground buildings through fitted foundations and direct terrain contact; shading on their own surfaces remains allowed. This overrides generic shadow guidance and may change only on explicit owner instruction. Review all building-art and rendering changes against this rule.
- Target workers/light units around 24–32 native pixels across, medium units 32–48, heavy units 48–64, and major structures 80–144. Review actual screen coverage before fixing these budgets.
- Use a project palette with material ramps: the shipped atlas holds 65 colours, and an ordinary sprite uses 8–16 of them, allowing justified separate team-color and effect ramps. Since art v19 every machine's rest and walk use 11–16, the Dredger 17; a muzzle flash or a gathering spark adds one.
- Broad clusters describe planes. Mechanical silhouettes contain useful negative spaces. Reserve high-frequency texture and brightest accents for unit function, not every terrain tile.
- No automatically softened outer edges, interpolated sprite rotations, default bloom, depth of field, or CRT effects. Permitted non-building ground shadows and fog use explicit palette decisions; effects must remain readable with optional flashes disabled.
- Units require eight directional views when their function/facing needs them. Mirroring is acceptable only for geometry whose handedness and lighting can be corrected. A directional set is reviewed before mass animation.

### Native source versus generated concepts

The generated battlefield and faction sheets establish mood, material, construction, and shape. They are not grid-audited pixel assets. Their dense texture and oversized detail must be simplified when translated to native sprites.

The Aseprite study is an original, scripted pixel composition with 14 semantic layers, a project-authored bitmap alphabet, and a 640×360 canvas. It demonstrates silhouette families, palette, HUD footprint, and exact scaling. It is intentionally less detailed than the concept paintings and is not final production art. The gate deck and headquarters in particular still need a stronger art pass. No animation, gameplay, or final art approval is claimed.

### Display mathematics (amended 2026-09-13)

For physical drawable dimensions `W,H`, allocate an exact `W × H` final RGBA
frame and upload it to an identically sized GPU texture. Present at 1:1 across
the surface; do not squeeze the world or the final frame into a fixed canvas.

The world viewport has width `W` and height `V`. For a world scale `s` from 1
to 3, render native artwork into a scene of `ceil(W/s)` by `ceil(V/s)` usable
pixels plus a one-texel margin. No source pixels are skipped to create the
wider view. The existing building foundation correction is applied before this
camera transform.

(Amended 2026-09-23, continuous zoom.) Each window pixel takes the scene texel
under its centre. Only a window pixel that a texel edge crosses is mixed from
the two texels, weighted by how far the edge is from its centre (a sharp
bilinear filter). Every texel is drawn the same width to within a pixel and no
edge is softer than one pixel. At a whole scale `k` the camera's part of a
texel is held to a multiple of `1/k`, so every edge lands on a window pixel,
nothing is mixed and each texel is an exact `k × k` block as before.

The wheel scales continuously: one click multiplies the scale by the cube root
of two, so three clicks double it, eased toward the goal over about a fifth of
a second. When the wheel rests within 8% of a whole scale it settles on it.
The +/- keys and buttons ease to the next whole scale; the readout eases back
to 2×. The scale is saved as a preference.

Rendering and picking share `WorldView`, with the same viewport origin, camera
offset and sub-texel shift as the scene renderer. Zoom keeps the world point
under the pointer still; keyboard zoom uses the center. Panning moves by window
pixels at every scale; the camera keeps its part of a texel for presentation
only. Resize and DPI events use actual physical dimensions. Minimized surfaces
skip drawing.

The scene's canonical camera coordinates remain compatible with prior saves;
no simulation, content, fog authority or replay format changes. Wider windows
and farther zoom show more of the offline map. Online camera fairness remains
a separate future design decision if networking is implemented.

### Art production sequence after approval

1. Redraw the six illustrated unit silhouettes and two missing slice roles at native size. Review blackout silhouettes against both land and water.
2. Make a small material/terrain test: salt, wet silt, shallow water, deep-water border, cliff, ford, dry/flooded gate cues.
3. Complete one representative eight-direction unit, one building, and one simple firing event through export and engine display. Inspect anchors, contacts, and facing before cloning workflow.
4. Block locomotion and attack keys, then add timing. Typical starting budgets: idle 2–4 drawings, movement 6–8, basic attack 3–5, heavy tell/fire/recovery 6–8, destruction 4–6. These are budgets, not mandatory uniform timing.
5. Finish the first slice's four blueprints per faction and essential buildings. Add the rest only after play evidence identifies their purpose.
6. Reconstruct exported animation using tags, durations, trim offsets, and anchors; inspect native, enlarged, and actual-speed display. Technical export success does not replace visual judgment.

### Where the art actually stands (2026-09-22)

The contract above still governs. Three things have moved since it was
written, and a reader should know them before quoting it:

- Art work is reviewed against a broader pixel-art quality checklist (the
  studies credited in `art/README.md`) together with `tools/art_v2/contract.md`.
- The palette bullet used to quote the 2026-09-12 study ("near 32 common
  scene colors", 27 opaque). The shipped atlas (`art/exports/game-assets.png`)
  has held 65 colours since v14; the bullet now says so. The v18 pass brought
  the v16 and v17 machines inside a 12–17 colour budget after an audit of
  v15–v17 found 19–35; v19 did the
  same for the eight original machines, which had kept 19–29.
- Walks are drawn to the engine's own frame rule: one walk frame per quarter
  cell of Manhattan travel, so a planted foot moves back 4 painter units a
  frame on a screen-axis facing and 5.66 on a diagonal, and every wheel,
  track and screw turns a quarter of its symmetry period a frame. The v19
  review plays every walk at engine speed, one frame per 30 Hz tick.

The render path described under **Display mathematics** is the one in use.
The 640×360 figure survives only as the interface design grid and the
offscreen review fixtures; the game itself allocates a frame the size of the
window.

## 9. What 'custom Rust engine' means here

We own the application loop, the deterministic simulation, sprite batching,
pixel-canvas composition, camera and input mapping, navigation, fog, content
definitions, replay, save, networking and tools in Rust. Low-level libraries
provide OS and graphics interfaces. No general game engine is wrapped.

Dependencies, as pinned: `winit` 0.30 for windows and events, `wgpu` 30 for
presentation, `serde`/`serde_json` for content and persistence, `blake3` for
digests and state hashes, `image` for the atlas, `cpal` for audio, `pollster`
and `bytemuck` for the GPU transport. Content is JSON rather than the RON
originally proposed; the map is generated in code rather than authored as a
file, which is a debt to pay when there is a second map.

| Crate | Responsibility | May not do |
| --- | --- | --- |
| `bw_core` | Stable IDs, grid math, fixed-point values, kinds, factions, terrain, camera and viewport transforms | Depend on OS, GPU or audio |
| `bw_content` | Versioned specs, costs, upgrades, doctrines, rule constants, the rules digest and the build fingerprint | Infer rules from decorative image colors |
| `bw_sim` | World state, rules, commands, economy, combat, navigation, fog, the built-in opponent, canonical snapshots, saves and replays | Read wall time, devices, presentation state, or unordered external randomness |
| `bw_desktop` | Event loop, scheduler, CPU pixel compositor, wgpu presentation, input, UI, audio, menus, lockstep session, agent harness, playback | Mutate authoritative state except by submitting commands |
| `bw_tools` | Headless scenarios, skirmish runs, replay diff, save inspect, benchmarks, the depth check | Become a second implementation of the rules |

`bw_sim` depends on core and content, never on desktop; desktop receives
snapshots and submits commands; tools call the same simulation. Canonical
state, replay and save encoding live in `bw_sim`; desktop supplies the file
and UX adapters.

### Platform plan

macOS on Metal is implemented and exercised on the available Mac Studio (M3
Ultra, 96 GB, macOS 15.6). Windows (Direct3D 12 or Vulkan) and Linux (Vulkan)
are untested; listing a backend is not evidence. Unsupported adapters, surface
loss, missing audio and content errors produce actionable messages, and a
silent audio device does not stop a match. The development machine is not a
defensible minimum specification; a lower-end reference machine must be chosen
before any performance or support claim.

## 10. Authoritative simulation and commands

### Tick and numeric contract

Thirty simulation ticks a second (`TICK_HZ`), presentation independent. Time,
resources, cooldowns, damage and positions are integer or fixed-point: `FP` is
256, positions are signed i32 Q24.8, and the world is 128×128 cells, well
inside the validated 512-cell axis bound, so a two-axis squared distance stays
safe in i64 after widening. Per-second rates use a numerator/remainder
accumulator divided by the tick rate with Euclidean quotient and remainder, so
opposite movement directions acquire no truncation bias. Bounds: 1,024
entities, 100,000 commands, 16 queued items, 4,096 path nodes, 128 waypoints,
16 MB saves, 108,000 replay ticks. Values outside declared bounds are
rejected; invariant violations stop a debug scenario with a reproducible dump
rather than wrapping.

`RULES_VERSION` is 17 and `SIM_VERSION` is 2. `rules_digest()` hashes every
spec, upgrade cost and rule constant; saves and replays carry it and are
refused when it differs. A separate build fingerprint, hashed at build time
from the source of `bw_core`, `bw_content` and `bw_sim`, is exchanged in the
lockstep handshake so that two builds whose behaviour differs but whose
constants do not are refused before a match instead of desynchronising a
second into it. It is deliberately outside the digest, so an edit anywhere in
the simulation does not invalidate every save on disk.

Commands carry target tick, player, a monotonic sequence, sorted stable entity
IDs, kind and target. They are validated when they execute, not only when they
are issued, and a command that no longer validates reports the reason instead
of vanishing. A UI click cannot change world state directly.

The command set: Move, AttackMove, Attack, Gather, Build, Train, Cancel,
Rally, Stop, Hold, Deploy, Capture, SwitchGate, SetTide, Flood, Board, Unload,
Repair, SetFormation, Face, Surge, Research, CancelResearch, Upgrade,
CancelUpgrade, Vent, Reclaim, Sound, Surrender.

### System order

`World::step` runs, in this order: scheduled tide changes; the built-in
opponent; pending commands; caps; pressure; construction; production;
research; upgrades; deployment; workers; repair auras; artillery stance;
navigation; spreading; boarding; idle engagement; combat; artillery impacts; gate
capture; the switch guard; Surge timers; the hold gauge; victory; visibility
memory; enemy reports; beacons; then the tick advances.

Consequences that this order is responsible for: a slain machine cannot fire
retroactively outside the simultaneous damage window; a tide change never
depends on render timing; artillery that was committed resolves even if its
Loom packs or dies; and a hold gauge is evaluated before victory on the same
tick. Named seeded RNG streams are used only where a rule needs randomness;
cosmetic particles use a presentation seed. Stable ordering governs entity
iteration, tie-breaks, collision priority, opponent decisions and scheduled
work.

### Time and overload

The desktop scheduler accumulates real time and requests whole ticks, capping
catch-up work per frame to stay responsive; it never drops ticks or changes
delta time. Offline play may visibly slow. In a session a seat advances only
when it holds the other seat's batch for that tick, so a quiet peer stalls the
match visibly — the field says WAITING FOR THE OTHER PLAYER with the seconds —
rather than silently. The harness discards a backlog over 200 ms by design, so
match time in a headless trial advances at a rate that depends on how often
the seats ask for pictures; the match clock does not agree with the wall
clock, which is a known and unfixed limitation.

## 11. Navigation and collision without traps

Deterministic grid A* with group destination slots and a spatial neighbour
grid, fixed neighbour ordering and a stable open-set key. Unit radii, corner
cutting, no-build corridors and reachable destination slots are respected.
Since rules 16 the search takes all eight neighbours: a corner step costs 141
per 100 of a side step and is refused when either cell it passes between is
closed, and two machines standing corner to corner close the gap between
them. A machine moves the same distance per tick in every direction (an
integer square root keeps the seats in step), so on the dimetric screen a
machine now walks straight up, down, left and right instead of zigzagging.
A machine held up behind machines that are standing still plans a way around
them within six cells before it falls back to a one-cell side step. A
click into blocked terrain resolves to the nearest reachable ground within
twenty-four cells, with feedback, never to a teleport.

Lane costs are a movement-class multiple of the dry cost (wheel 182, walker
133, paddle 111, hull and air 100); deep tidal water is impassable to ground
movement. Cache validity includes the lane revision, so a tide change reprices
only affected paths. Path search uses a deterministic work budget per tick
counted in complete node expansions, preserves request progress across ticks,
and reports waiting rather than pretending a machine accepted an unreachable
route.

Three behaviours came out of the trials and are rules, not polish:

- An own deployed Bulwark, Loom or Caisson is not a wall to its own side. Two
  seats lost minutes to groups penned in by their own line.
- Ten seconds of blocked traffic stops the machine with a PathBlocked event,
  which the dock reads as "No way through to that point."
- An attack-move holds a group to its slowest machine's pace, and artillery on
  an attack-move stops at its own reach and deploys there instead of walking
  in packed and unable to fire. Crossing a lane used to cost twenty machines
  in twenty-five seconds; the behaviour was fixed before any number was
  touched, so the next trial measures the behaviour alone. The Bulwark takes
  the same stance: it turns to face the nearest target in reach and deploys.
  Since rules 17 a deployed Loom or Bulwark with an enemy in reach keeps
  firing when a new attack-move arrives, and packs to follow it only once
  nothing is left in reach.
- Machines standing on one another step apart to the nearest free cell, lower
  ids staying put, so a melee does not stack three to a cell. Capturing
  machines stay within two cells of the gate.

Required fixtures: opposing groups in a choke; many machines leaving
production; a tide change mid-crossing; an occupied destination; mixed
footprints; two groups crossing; queued waypoints; placement near exits; and
the same replay at different frame rates. A machine blocked by an enemy must
not disclose hidden enemy identity through an error message.

## 12. Fog, the built-in opponent, and the multiplayer boundary

Topography, the tide and the four crossing mouths are charted from the start.
Dynamic enemy machines, buildings, health, queues and orders require
visibility. `PlayerKnowledge` gives each side its own state, visible enemy
observations, declared public infrastructure and explicit stale observations
with last-seen times; the UI, chart, opponent, audio and effects consume that
view, never the whole world. Unseen machines disappear; seen buildings keep a
stale silhouette without live health or progress. Enemy machines the player
has seen fade from the chart over twenty seconds.

### The built-in opponent

It uses the ordinary command path with no extra resources, no hidden targets
and no privileged rules, and it is honest about what it does. It reads every
decision off the current state and stores no plan of its own, so a replay of
a match reproduces it exactly.

It trains workers to twelve, picks a doctrine after thirty seconds, and puts
up a Works, a salvage yard, a condenser and a Drydock at fixed positions,
holding salvage back from the Works for whatever it is saving for next. From
the Drydock it takes a scout, a mender, two of its faction's water role and a
transport, and it buys TRACKS once those are out and the sluice is its own.
The scout takes station at a crossing mouth and holds there, where its sight
doubles; the menders trail whatever is being shot; the Dredger works the
drowned lane wrecks no worker can reach. It sends a machine to take the
station, garrisons both crossing mouths on its own bank with the Caisson dug
in as the anchor, and then FLOODs, so nothing of the enemy's can walk across
to contest a mouth while the hold gauge fills; if the enemy leaves a side
open instead, it sets the tide to dry the lane its force is waiting at. Its
transport carries a landing party over the basin to the mouth on the far
bank. Spare pressure becomes salvage through RECLAIM. Against a passive
opponent it wins by holding the tide at about tick 12,300 in either faction
matchup.

What it does **not** do, and should: build a defence nest or a palisade, take
a doctrine's second tier or any upgrade but TRACKS, use VENT or SOUND, defend
its own base when something comes at it, or vary its plan — it goes for the
tide every match. It has never been tested against an opponent that contests
a crossing. The scripted controller used for reference runs still has the
whole of the old gap, which is why the reference runs under
(`bw_tools skirmish`) mostly end with no decision at 45,000 ticks and why the
agent trials, not the scripted runs, are the acceptance test.

### Where multiplayer stands

- **Offline:** deterministic local simulation, saves, full diagnostic replays,
  live recording and a fog-lifted observer. Delivered.
- **Trusted two-peer lockstep:** delivered. Newline-delimited JSON over TCP, a
  protocol and rules handshake with the build fingerprint, commands scheduled
  three ticks ahead, one batch per tick per seat, host commands applied before
  guest, state hashes every thirty ticks, a desync report written to
  `saves/desync-<tick>.json`, and a clean disconnect. The guest is canonically
  player 1 and draws a relabelled copy of the world in which its own side is
  player 0; the copy is never stepped or hashed. Both peers hold the whole
  world, so this is explicitly a trusted mode and not public play.
- **The agent harness:** `--headless` runs the loop from wall time without a
  window, and `--agent-port` serves `GET /frame`, `POST /input` and
  `GET /state` so that a player who is not a person can see and act through
  the same entry points a window uses. Seven refereed two-agent matches have
  been played this way; almost every rules change from 5 to 12 came out of
  them. `/state` is served on the playing seat's own port, so a seat's
  blindness is a convention rather than a structure; a separate referee port
  would fix that.
- **Public competitive play:** still deferred, and still requires a
  server-authoritative model that distributes filtered views and validates
  commands. Deterministic peer lockstep does not stop a modified client from
  reading hidden state.

## 13. Asset, map, replay and save pipeline

### Art and content

`.aseprite` files are the editable source, under `art/source/v*/`. Lua modules
under `tools/art_v*/` construct and export them; a verifier proves that every
rectangle an earlier pass owned is still exact and that a re-authored frame
kept its rectangle, so a new art pass cannot silently move the atlas under the
renderer. The runtime loads one atlas, `art/exports/game-assets.png` with its
JSON metadata. Superseded atlases are kept under `art/archive/`.

Content records use stable logical IDs. Rule data and art identity have
separate digests, so a repaint does not invalidate a deterministic replay.
The map owns terrain classes, tidal strips, wells, wreck beds, starts and the
station; decorative pixels carry no authority.

### Replays and recording

A replay records the format version, rules digest, seeds, player slots,
commands, initial state identity and periodic canonical hashes, so a divergence
can be bisected to its first tick and a diff names differing fields and IDs
rather than only a checksum. Canonical serialization uses explicit field order
and sorted records and includes RNG counters and all pending navigation state:
request IDs and queue order, start and goal, group slots, movement class, lane
revision, frontier entries with total tie-break keys, closed costs and
parents, request age, reservation age and the per-request work cursor.
Dropping pending searches on load is not permitted. Hash-map order, allocator
addresses, build timestamps, platform handles and render interpolation are
excluded.

Every match records itself: `saves/live-match.replay.json` is rewritten every
ten seconds of field time on a writer thread, and `last-match.replay.json` is
written when a match is decided and when a headless seat is told to quit, so
an abandoned match is still watchable. In a session both seats write the
canonical world, so their files are byte-identical. `--replay FILE` plays a
recording with the fog lifted through `World::revealed`, a presentation flag
that is never serialised or hashed; `--follow` re-reads a growing file once a
second, so a live match can be watched a few seconds behind. Orders are
refused in playback.

Watching a recording, the top band grows a spectator bar: both sides' hold
gauges, army value, income per minute, crew and salvage, and the sluice owner.
Under it is the replay timeline: the sluice owner over the match, the tide
switches, and every hold count drawn as tall as it got. A finished recording
is scanned once on a background thread for that timeline, keeping a world
every fifteen seconds, so a click on the timeline (or `[` and `]`, thirty
seconds) seeks both ways. The camera follows the biggest fight, or the mouth a
running count depends on, until the watcher pans (`F` toggles it).

Every match ends on a result screen with two pages. SUMMARY charts both sides'
army value and income over the match above the same sluice and count strip,
lists each side's counts, sluice share, losses and kills by kind, names the
fight that turned the match, and says in one line what decided it. DOCK LOG
lists the observed events newest first with the timeline above them. Deaths
have their own bound in the log, so captures, switches and counts are never
pushed out, and a fight's deaths fold into one row. `--spectate-review DIR
--replay FILE` saves both pages and the spectator view from a recording.

A recording plays only under the rules that made it: the replay stores the
rules digest, and a different build refuses it.

### Saves

Saves are written at tick boundaries in a versioned envelope carrying the
schema version, the rules digest, the state hash and a payload digest, written
to a temporary file and atomically replaced. A mismatched version, digest,
checksum or state hash is refused with a reason rather than loaded. Pre-alpha
saves are not migrated; historical files are preserved on disk and rejected.
Save and load must reproduce the uninterrupted run's future hash sequence.
Control groups travel with a saved match in a hash-matched sidecar, as do
chart memory and battle traces.

## 14. Performance and observability

Targets are budgets to test, not achieved capabilities. What has actually been
measured, on an M3 Ultra, is narrower than the budget table:

| Scenario | Budget | Measured so far |
| --- | --- | --- |
| Simulation tick, ordinary match | p95 < 8 ms, p99 < 16 ms at 30 Hz | p95 0.058 ms on an idle-AI benchmark; p95 9.46 ms on a prepared 128-entity engaged field |
| 1080p presentation | Stable 60 fps, CPU and GPU reported separately | CPU compositor p95 6.02 ms at 2× at 1080p |
| Input | Immediate feedback; first legal movement on the scheduled tick | Exercised by hand and in journeys; not instrumented |
| Long match | Bounded path, command and event memory | A 73-minute agent match ran to a result; memory was never instrumented |
| Stretch stress | 1,000 machines on a larger map | Not run. The 90 crew ceiling makes it unreachable in ordinary play |

Debug views show sprite anchors, footprints, viewport bounds, the input
inverse, navigation requests and costs, visibility, stale observations, the
tide schedule, economy rates and authoritative entity IDs. Privileged overlays
are marked. Logs connect commands, ticks, systems and rejected actions. A bug
report should be reproducible from a recording or a named scenario;
`bw_tools depth-check` runs eight scenario groups and every rules change is
expected to keep them passing.

## 15. Where the milestones actually stand

| Milestone | Deliverable | State |
| --- | --- | --- |
| M0 — design packet | Design, concepts, native pixel study, architecture | Done, 2026-09-12 |
| M1 — engine foundation | Native window, pixel canvas, input inverse, deterministic kernel, command log | Done |
| M2 — one useful encounter | Two forces with movement, formation slots, selection, attack, deaths, lane costs, scheduled transitions, fog | Done |
| M3 — first complete game | Both factions, production, two resources, one map, an opponent, victory, restart, save and replay | Done; the opponent is the weak part |
| M4 — visual and feel pass | Native directional assets, grounded animation, terrain, UI, audio | Art through v18 and eighteen interface passes are in; the owner's own taste judgment is not recorded and is the gate |
| M5 — two-human validation | Trusted LAN, repeated real matches | The transport is delivered and seven refereed agent matches are recorded. **No human has played a match.** The milestone is not met |
| M6 — roster and maps | Justified roles toward six per faction, two further maps, tutorial, stronger opponent | Each faction now has seven machines plus a worker; the tutorial exists; the second and third maps and the stronger opponent do not |
| M7 — release hardening | Platform builds, input and accessibility options, error handling, performance evidence, licensed assets | Open. macOS only |

These are dependency milestones, not dates. The honest summary: the game is
complete enough to be played end to end by someone who has never seen it, and
it has been, seven times, by agents. What it lacks is a human's judgment, a
second map, and an opponent that plays the game it has become.

### Human evaluation without inventing evidence

When a complete slice is put in front of people, target five non-authoring
testers and at least ten matches with spawn and faction swaps. Observe whether
players recognize roles, ownership and the tide state, use scouting to change
a decision, make an economy tradeoff, retreat successfully, and explain a
meaningful tide choice. Track match durations, stuck machines, misunderstood
orders, first advantage, expansion payback and composition. Those samples
diagnose large problems; they do not prove balance, and agent trials do not
substitute for them — an agent seat that reads a card is evidence the card is
legible, not that the match was fun.

## 17. Current risks and open questions

| Risk | Present state | Trigger for revision |
| --- | --- | --- |
| Pacing | Agent matches ran 09:47, 14:41, 34:17, 61:47, 73:21, one unfinished at 88:00, and 20:58 in the eighth trial (rules 12, tide victory) against a 15–25 minute target | One match inside the target is not a trend; a ninth trial under rules 13 should show whether a defender who can see the count still loses to it |
| The built-in opponent plays one plan | It now builds a Drydock, fields every tier-two role, takes the sluice, floods the lanes and wins on the tide, but it goes for the tide every match, does not defend its base, and has never met an opponent that contests a crossing | A practice player who beats it the same way twice; any balance question that needs it to run a second plan |
| Balance is unmeasured | Every number is a seed; no human has played | First human matches |
| One victory could crowd out the other | Four decided agent matches ended on a headquarters and one on the tide | A run of matches that all end the same way, or a tide count that nobody contests |
| One map | The Split Basin is generated in code, not authored | A second map, which also forces the map format the pipeline section owes |
| Art quality against the skill | v18 fixed the audit's colour, size, weight and damage findings on the later machines; v19 brought the original eight inside their budget, drew every walk to the engine's ground speed and reviewed the loops at engine speed in a simulated playback. No human has judged them at speed, and the six-cell-a-second walkers still cycle faster than four frames can read | The next art audit |
| Public networking gives hidden-state access | Trusted mode is labelled as such | A request for untrusted competitive play |
| Scope | Seven machines and a worker per faction, one map | A new layer requested without a tested role in the core loop |
| Documentation drift | This plan went stale for five days and five rules versions before this revision | Any rules change that does not update this file in the same pass |

## 19. Record: what changed, and when

The direction was set on 2026-09-12: coastal reclamation fiction, Union
versus Assembly, the sluice, native pixel art, and the custom Rust engine.
The tables below record what changed after that.

### Rules

| Version | What changed |
| --- | --- |
| 1–2 | The first alpha: economy, production, combat, fog, the paired lanes, headquarters victory |
| 3 | Depth and skill: deployed Bulwark facing, Loom deployment and warned ground blasts, formations, Face, Surge, headquarters doctrines |
| 4 | Idle machines and defence nests engage on their own; nests reach eight cells and see ten; enemies carry a red edge |
| 5 | Workers never idle at a base; Riveters and Looms hit buildings harder so a push that reaches a base can finish it |
| 6 | The auto-gather reach is forty cells |
| 7 | Depth pass 2: the Drydock and its roles, the tech tree, doctrine tiers, the crew and pressure caps, the keel rule, the tide victory, a sluice that pays and costs, VENT and SOUND, nests and the Palisade |
| 8 | A destination in deep water resolves within twenty-four cells; a machine with no path stops and says so; a group moves past its own deployed machines |
| 9 | A move packs a deployed specialist first and sends it when its legs are free |
| 10 | Water pass 3: four tidal strips, the neutral start, dry/shallow/deep, the flood, swamping, the Barge and the Lifter |
| 11 | The hold gauge drains instead of resetting; the Caisson counts at a mouth; wrecks from destroyed machines; RECLAIM; attack-move puts artillery in the rear slots; death events name the killer |
| 12 | The tide victory requires the station; artillery on an attack-move stops at its reach and deploys; a group holds its slowest pace; own deployed machines are not walls; blocked machines stop and say so; wells and map wrecks are always charted |
| 13 | From the eighth trial: a firing Loom is lit for the side it fires at; a tide switch is refused while an enemy stands at the station and a cancelled one is refunded; workers hit under fire leave marked wrecks and none pick one on their own. The hold rule itself is unchanged; its count is now a banner with a timer for both sides |
| 15 | Balance from the eighth trial: wrecks at a crossing mouth are worth half; Siege gives a Loom one more cell (nine) instead of 25%; SOUND lights nine cells; the Reedguard's hull is 160; Plate reaches machines trained after it; pressure over the cap becomes salvage at 3 per 10; upgrades queue two deep; TEMPER and REFIT; a falling flood refills the lane wrecks; the scout is trained at the headquarters; the first capture of the neutral station takes sixty seconds; the doctrine buttons preview tier II. The hold rule is unchanged |
| 16 | Machines walk diagonals: the path search takes all eight neighbours (§11) |
| 17 | From the ninth trial: Plate also covers the Riveter and the Reedguard's hull is 150; an enemy within four cells of the station contests a capture (two before), and the ring is drawn while a capture runs; a deployed gun with an enemy in reach fights before an attack-move packs it; F2 skips any machine standing at a crossing mouth, not only one on hold; the hold banner shows the banked count and a gauge while it drains; the post-match line counts breaks; condensers may be placed on any charted well; mouths are named NW, NE, SW and SE on the ground; an off-screen Loom's fire gets an edge arrow; free wells pulse while pressure is short and no condenser stands. The hold rule is unchanged |
| 21 | From the eleventh trial: the Glinter hits a deployed Loom for 5 more a shot, as the Sounder does; the Loom reaches seven cells, eight with Siege; a broken hold count drains three ticks a tick; OVERHAUL, three salvage-only levels at the headquarters, adds 6% hull a level to every machine |
| 14 | From the eighth trial's controls list: the Bulwark stops at its reach and deploys; separate DEPLOY and PACK and a keep-deployed mark; able machines surge in a mixed group and capture chains from Surge; queued build sites keep their builder; own machines step off footprints and stacked machines spread; Drydock rallies and a three-cell muster out of the door |

### Art and interface passes

v2 and v3 redrew the roster after the owner rejected the first quality; v4–v8
added material, coastline, grounded buildings, construction and activity
states, and the six approved directions; v9–v11 added artistry, the chart and
craft pass and a coherence pass; v12 and v13 reworked the coast and removed
every shadow beneath a building; v14 and v15 rebuilt ground light, water,
shadows, idles, damage, weather and the day; v16 drew the depth pass's six
machines and two buildings a side; v17 drew the Barge and the Lifter; v18
re-authored 753 frames to fix an audit's findings on v15–v17. In
parallel, three quality-of-life passes, a UI copy pass and the fixes from
seven agent trials rebuilt the dock, the cards, the chart and the Guide.

The standing rules from those passes that are not negotiable: no shadows
beneath buildings, native pixel grid, integer scaling, and an atlas in which
every earlier rectangle stays exact.

## 20. The third faction and the three-player map

**Designed 2026-09-23 on the owner's request ("a third race and a map that
can support generally the same mechanics but with three teams") and approved
the same day: free-for-all, the Compact without a transport, and the names
as written. It has since been built (rules 18–22).** The detail and the
numbers are in [THIRD-FACTION-AND-CONFLUENCE.md](THIRD-FACTION-AND-CONFLUENCE.md).
The layout sketch and its measurements come from
`tools/maps/confluence_sketch.py`.

**The Saltglass Compact.** Salt-pan crews who fire brine with mirrors and
fuse salt into glass. The Union blocks the water, the Assembly works with it,
and the Compact makes ground over it. It is fast and fragile. Its machines are
tall and thin (stilts, masts, mirror dishes, glass), in a cobalt-violet glaze
with salt-white and copper.

| Slot | Machine | Hull | Damage | Range | Speed | Cooldown | Cost | Move | Role |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| Worker | Raker | 65 | 3 | 1 | 3 | 30 | 50S | Walker | Worker |
| Works | Brander | 110 | 13 | 3 | 4 | 24 | 75S | Walker | Fast line fighter |
| Works | Heliostat | 200 | 2→10 | 6 | 2 | 12 | 150S 40P | Walker | Deploys; its beam gains 1 a shot on one target, to 10; 150% to buildings |
| Works | Glinter | 80 | 8 | 5 | 5 | 26 | 80S 10P | Walker | Harasser; GLINT lights a ray 18 × 3 cells for 5 s (20P); +5 a shot against a deployed Loom (rules 21) |
| HQ, Drydock | Stilt | 60 | — | — | 6 | — | 40S | Walker | Scout |
| Drydock | Glazier | 80 | — | — | 3 | — | 90S 20P | Walker | Mender |
| Drydock | Salter | 180 | — | — | 2 | — | 130S 30P | Walker | LAY: crusts tidal water into a 3-wide causeway, a row a second for 3P, dry for everyone until the tide next changes |
| Drydock | Pan | 120 | — | — | 2 | — | 110S 20P | Walker | Deploys on dry ground for 40 pressure a minute; no well needed |

The Compact has no transport. Its answer to the wall is the causeway, and
anyone may cross one. Its buildings are the shared kinds under its own names:
the Kiln, the Glassworks and the Rake shed. Plate covers the Brander and the
Heliostat. Siege makes the beam gain 2 a shot and halves the Heliostat's
deploy time. Everything else in §5 is shared.

**The Confluence.** A round salt basin 176 cells across, ringed by sea and
cut into three wedges by a Y of water. Each pair of players shares one arm,
and each arm has a tidal lane near the middle and a tidal rim at the coast.
The station stands on a three-pointed island where the arms meet, and each
point touches one lane. The map is mirrored across the grid diagonal, so two
seats are exact mirrors and the third is tuned by measurement. Every path the
sketch measures (to the station, to the mouths, to the beds and wells, to
each neighbour) is within 2% across the three seats. Each seat has the Split
Basin's economy: two home beds, four side beds and three wells. The map adds
six lane wrecks and three deep wrecks.

**The tide with three.** Neutral opens every match. The holder can then DRY
E, DRY W or DRY S, which dries one arm and leaves the other two deep, or
FLOOD all three. A dry arm joins its two players to each other and to the
island and walls the third off. Prices, warnings and locks are as in §3.

**Victory with three.** Free-for-all. You win on the tide by owning the
station and holding both lanes that touch your land for ninety seconds; only
the owner can count, so there is one banner at most. A fallen headquarters
(keel rule intact) or a surrender eliminates that seat, whose machines become
wrecks. The last headquarters standing wins, and the summary ranks the
placings. Each opponent has its own colour (red, violet), and yours stays
jade.

**Engine.** Every per-player `[T; 2]`, `0..=1` loop, `1 - player` and
`^ 1`, the hard-coded map, the north/south tide, the practice AI's fixed
seat and sites, the harness's own/enemy split, and the jade/red interface
has to become per-seat. The design file lists each one with its location.
The "Peer-to-peer multiplayer" thread owns the lockstep session, which it is
designing for N seats. The proposed order: N seats with two-seat replays
unchanged; a map format with the Split Basin ported hash-identical; the
three-way rules and interface; the Compact's rules with stand-in art; its art;
then trials.
