# Controls and rules at a glance

| Action | Control |
| --- | --- |
| Menu navigation / activate / back | Tab or arrows / Enter / Esc |
| Focus live HUD and practice controls | Tab; Esc returns to field control |
| Select / group select | Left click / drag; Shift adds and includes workers in mixed boxes |
| Select visible units of one type | Double-click; Shift adds to the current selection |
| Move, gather, attack target, repair or resume construction, rally | Right click in context. A wreck under the fog takes a gather order; a selected producer shows its rally flag, and a rally on a wreck sends new workers to gather. A rally applies to every selected producer, the Drydock included; without one, new machines muster three cells out from the door |
| Queue movement | Shift + right click |
| Queue attack-move | A, then Shift + left click; continues accepting waypoints until a plain click or Esc |
| Minimap camera / orders | Left click or drag / right click Move or Rally; Shift queues Move; A then click accepts attack-move |
| Attack-move / stop / hold | A, then click / S / H. Idle machines and defence nests fire at enemies in weapon range on their own; an idle mobile machine that sees an enemy beyond its range closes on it and stops where it was. Stop halts movement; Hold stands and fires |
| Gather with workers or Dredgers / capture with combat / set the tide at an owned sluice | G / G / DRY N or DRY S on the sluice card (T dries the other side, Shift+T floods). Capture is 40s of work shared by the machines beside the station; drying a side costs 40 pressure, a flood 80, and an enemy at the station cancels either |
| Transports | R at a Drydock trains the Barge (Assembly, water only) or the Lifter (Union, flies). BOARD on the card (E; B is always BUILD), or a right-click on an own transport with machines selected, boards the nearest one with room; U unloads onto ground within three cells. A loaded transport shows its rider count on the field and in the panel |
| Reclaim pressure | RECLAIM on the headquarters card (R): 100 pressure for 60 salvage, as often as you like |
| The tide victory | Own the sluice, and hold each lane with a gun machine or Caisson within four cells of either of its bank mouths and no enemy one at either: the hold gauge on the sluice card fills, drains while the lanes are not held, and wins at 90 seconds. The four mouths are always in sight |
| Deploy / pack Bulwark, Loom or Caisson | D / P, never a toggle. K keeps the selected machines deployed when their group moves (off by default). A deployed Caisson blocks its cell |
| Every worker / every building of a kind | F7 / Ctrl + B, C, Y, V, N, P or H |
| Build Drydock / Palisade | N / P with a worker selected; B, C, Y, V as before |
| Building upgrades | P, L, N for the first, second and third upgrade of the selected Works, Drydock, yard or condenser; K cancels |
| SOUND with a Sounder or Skipper | X: 20 pressure, lights six cells for 5s |
| Train at the Drydock | Q, W, E, as at the Works |
| Cycle Compact / Line / Loose formation | F; applies to subsequent movement orders |
| Stop and face a direction | R, then click; pack specialists first |
| Surge selected combat units | Z; 10 pressure for each machine able to surge, 3s speed boost with weapons off, 12s cooldown; G while surging captures on arrival |
| Headquarters doctrine, its second tier / cancel research / VENT | U: Hauling, J: Fire Control (again for tier II) / K / X: 100 pressure, combat machines fire 30% faster for 10s |
| Next idle worker / all idle workers | I or click IDLE / Shift + I or Shift + click IDLE. Selecting never moves the view; Ctrl + I centres on the selection |
| All combat machines / center army | F2 or click ARMY / Shift + F2 or Ctrl + F2; machines on Hold at a crossing mouth are left where they are |
| Recent field alerts | Click a card in the notice band, or F3: while the enemy's hold count runs, the crossing mouth that stops it first; then the newest red card you have not looked at, then the cycle; cards away from home carry a compass letter; cards expire after 20 seconds of field time |
| Select completed Works | F4 or click WORKS; camera stays in place |
| Game speed | PACE button or Settings: 0.5× / 1× / 1.5×; all field timers and both crews follow this pace |
| Headquarters | Space |
| Pan | Arrow keys, screen edges, or middle-button drag |
| World zoom | Mouse wheel zooms smoothly toward the pointer from 1× to 3×, settling on a whole step when close to one; + / - step to the next whole scale; click the scale readout to reset to 2× |
| Selection roster | Click a type in the dock to keep only it; Shift-click or its minus control removes it; paged when diverse |
| Control group bar | Click a numbered slot to recall its living members; Shift-click centers on them |
| Health bars for everything visible | O, or the Settings entry; default shows selected, damaged and building |
| Store / recall control group | Ctrl or Cmd + number / number |
| Add selection to control group | Ctrl or Cmd + Shift + number |
| Center control group | Double-tap number or Shift + number |
| Train at headquarters | Q: worker |
| Train at Works | Q / W / E: faction's three combat/support roles |
| Batch training | Shift + training key or Shift-click: up to five across selected compatible producers; partial batches explain limits |
| Build with a worker | B: Works, C: condenser, Y: yard, V: defense; click to place. Shift, or a worker already building, queues the site behind its current one; buildable ground is shaded |
| Cancel placement or order mode | Esc or right click |
| Cancel construction or first queued production | Backspace |
| Save / load / export replay | F5 / F9 / F6 |
| Pause / help / fullscreen / mute | Esc / F1 / F11 / M |
| Quit with save decision | Home → Quit, close button, or Ctrl/Cmd + Q |
| Two-player match | `--host ADDR` and `--join ADDR` on the command line; see below |
| Capture canvas / reload art | F12 / F8 |

Workers collect salvage from wrecks and return it to headquarters or a yard. Pressure comes from headquarters and condensers placed on marked wells. The tide starts neutral; the sluice's holder dries one side and deepens the other after a public ten-second warning, or floods every crossing for 45 seconds. Wading slows machines and makes them take more damage; a machine caught by a rising tide makes for the shore. A destroyed combat machine leaves a wreck worth half its cost for whoever holds the ground.

Bulwarks lock their shield and fire to the front when deployed; flank them or
force them to pack and reposition. Looms must deploy to fire, with a two-cell
minimum range. Their ground blasts warn for 0.8 seconds and hit the marked
point, so spread out, dodge, or close the distance. Surge spends pressure to
move 50% faster while giving up fire and deployment for three seconds.

At headquarters, choose one doctrine for the match: **Hauling** raises worker
cargo from five to eight, or **Fire Control** shortens combat attack cooldowns
by 15%, rounded up to whole ticks. Either costs 120 salvage and 60 pressure and
pauses worker production for 30 seconds. Existing queues remain. Cancelling
research refunds 75%; a completed doctrine locks the choice. The tradeoff is an
economic payoff versus an earlier combat advantage, with an exposed investment
window either way. These values still need human balance testing.

Home and Pause preserve the active field. Replacing an unsaved field offers a clear keep/save/replace decision; loading protects the existing saved slot. Practice progress is saved alongside the world in a hash-matched UI sidecar. Missing guide data opens recoverable free practice.

The installed game stores saves and replays under `~/Library/Application Support/Brinewake/saves/` (macOS) or `%APPDATA%\Brinewake\saves` (Windows). Runs from source use the project's `saves/` folder. A decided match exports `last-match.replay.json`; F6 exports earlier.

The depth expansion changes authoritative rules. Historical saves and replays
from the earlier alpha are incompatible with this build and are kept on disk;
start a new match. New sessions serialize doctrines, formations, Surge timers,
and committed artillery shots.

