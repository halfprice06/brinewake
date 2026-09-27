# Playing BRINEWAKE as an agent

You are one seat in a two- or three-player match. You see the game only as pictures
and act only through the pointer and keyboard, the way a person does. Your
seat is a headless instance with an agent interface on a local port. This
playbook is controls only: what the machines, buildings and the sluice do,
you learn from the game itself, as a new player would. Read the buttons,
the panel, the tooltips, the cards and the Guide (F1).

## Your seat's folder and helper

Your seat has a work folder of its own, `output/agent-play/SEAT/work`
(SEAT is `host`, `guest` or `third`), and in it a helper, `play-SEAT`, that
drives your seat's port and refuses any other. Use that folder as your
scratchpad for pictures, crops and scripts, and use the helper for every
call. Do not write anywhere shared (`/tmp`, `/usr/local/bin`, the repository)
and do not type a port: the seats of one match share a machine, and in
trial 10 one seat's helper overwrote another's, so a seat's first picture
came from the wrong game.

Below, `PLAY` stands for your helper and `WORK` for your work folder, for
example `output/agent-play/guest/work/play-guest` and
`output/agent-play/guest/work`. (Without a helper,
`python3 tools/agent_play/play.py --port PORT` does the same.)

## How to see

    PLAY frame WORK/frame.png --scale 2

Then read the picture. The frame is 1280x720; with `--scale 2` the picture
you read is 640x360, so **multiply the coordinates you read off the picture
by 2** before you click. Fetch a fresh frame before every decision. A frame
at `--scale 2` can hide one-pixel marks; when you need to be sure, fetch
`--scale 1` and read coordinates directly. Stay at 2X world zoom.

To read something small, cut it out and enlarge it; there is no need to
write a cropper of your own:

    PLAY frame WORK/panel.png --crop 380 590 900 720 --zoom 2

`--crop X0 Y0 X1 Y1` takes full-frame pixels, the same numbers you click
with; a pixel (px, py) of the cropped picture is at full-frame
(X0 + px / zoom, Y0 + py / zoom). `--clean` draws the field without the
corner controls so you can see the ground under them; they are still there
to click.

The match clock starts only when every seat has fetched its first
frame. Read this playbook first; nothing is running yet.

## How to act

    PLAY click X Y            left click
    PLAY click X Y --right    right click (orders)
    PLAY click X Y --shift    add to selection
    PLAY dblclick X Y         select all of a type
    PLAY drag X0 Y0 X1 Y1     box select
    PLAY key KEY              press a key
    PLAY key KEY --control    a Ctrl chord (Ctrl+1 stores a group)
    PLAY key KEY --shift      a Shift chord
    PLAY pan DX DY            scroll the camera in screen pixels

Key names are case-insensitive: `Space`, `Escape`, `F1`, single letters and
digits. A chord may also be spelled in the name: `key Ctrl+1`, `key Shift+I`.
Keys that give an order need no click after them: `G` alone sends a
capture, `D` alone deploys or packs. A click after such a key is an
ordinary click and changes the selection.

Several actions in one call, and a pointer move without a click:

    PLAY events '[{"t":"click","x":300,"y":400},{"t":"key","key":"G"}]'
    PLAY events '[{"t":"move","x":700,"y":300}]'
    PLAY events '[{"t":"click","x":300,"y":400,"button":"right","shift":true}]'

An event takes only its own keys: `move` x, y; `click` and `dblclick` x, y,
`button` (`"left"` or `"right"`), `shift`; `drag` x0, y0, x1, y1, `shift`;
`key` key, `shift`, `control`; `pan` dx, dy. An event with any other key
(`"right": true`, say) or a value of the wrong kind is refused, not guessed
at: it is listed under `errors` and nothing of it is applied.

Every action call answers with how many events it applied, each one as the
game took it (`as_applied`: the key name it matched and whether shift and
control reached it), anything it refused, the tick and match clock it acted
on, and the prompt row under the field. That is confirmation that the action
landed, and nothing you could play from: what is happening you read off the
picture, as a person would.

The reply also carries `heard`: the game's tide sounds of the last ten
seconds as text, because a seat has no speakers. A player with sound hears
a buzz when the enemy starts capturing the sluice, a bell when it changes
hands, a clank when a switch is cancelled, and a bell when either side's
hold count starts, every ten seconds while it runs, every second at the
end, and when it breaks. For example `"18:13 enemy holds both crossings:
you lose in 90s"` or `"20:05 enemy hold broken"`.

**Lockstep lag:** an order runs three ticks after you send it. A frame
fetched at once still shows the old salvage; the next frame is right. A
production panel counts a queued unit at once and says ORDER SENT /
STARTING until the building takes it. Read the rest a beat after acting. The match clock runs only while
every seat is serving pictures and orders, so it moves in bursts. Neither
the Guide (F1) nor the menu stops it: the other seat is still playing, and
the menu says THE MATCH RUNS ON rather than PAUSED.

## What is where

- **Header:** your resources and the match clock.
- **Field:** the ground you play on. Your machines have no outline; every
  enemy machine and building has a red edge. Machines behind a building
  show as a tinted ghost and can be clicked there. Labels on dark plates
  name things on the ground.
- **Tide gauge (header, centre):** the sluice in its holder's colour (a
  thin line under it fills while someone captures), your two crossings as
  letter chips (gold when yours, red when the enemy's, framed when two
  sides stand at that lane, the water under each letter), a tide timer
  behind an arrow while a switch or flood is on its way, and the hold bar
  with the seconds to go. While your machines have a capture order, the
  gauge shows how many of the four that count stand in range (`1/4`),
  red when an enemy in the ring halts the claim.
- **Sluice card (under the gauge, open from the start):** a status line,
  the tide line (`TIDE NEUTRAL / BOTH SHALLOW`, `NORTH DRY / SOUTH DEEP`), a
  row of chips (`SLUICE`, then `N` and `S` each with `GUN` and `FOE`,
  filled when true) and, when you hold the station, the `DRY N` / `DRY S`
  / `FLOOD` buttons. During your capture the status line says how it goes
  and why it stands still: `CLAIMING 45% / 2 OF 4 IN RANGE`, `STAND CLOSER`,
  `ENEMY IN RING: CLAIM HALTED`, `SURGING: THE CLAIM WAITS`. The `-` at its
  corner folds it to the status line; clicking the gauge unpins it, and it
  then opens only while the pointer is on the gauge. It stands aside while
  an order or a placement waits for its click.
- **Other corner controls in the field:** top centre, under the card,
  while a hold count runs, a banner with the seconds left and whose count
  it is (`YOUR 52/90 DRAINS`, `ENEMY DRAINING`); click it or press
  F3 to look at the crossing it names. Bottom left, IDLE (next idle
  worker; Shift-click selects all), ARMY and WORKS; top right, zoom. These
  corner controls act when the button is released, a press that turns
  into a drag is a box over the ground under them, and while a placement
  or order is pending a click on them goes to the ground instead.
- **Notice band (under the field):** the first row is the prompt or the
  last message; the second row holds up to two alert cards at the left and
  the control-group bar at the right. Click a card or press F3 to look at
  the place it names.
- **Dock (bottom):** the chart minimap at the left, the selection panel in
  the middle (what is selected, its hull, its stats, what it does, what it
  is doing), and the command card at the right: a grid of every order the
  selection can take, with its key and cost. The headquarters card carries
  RECLAIM (R) and a machine card carries BOARD (E) when an own transport
  with room stands in the field. Hovering a button (an `events`
  move) opens a tooltip beside the card that says what the order does and
  what it is strong and weak against.

On the three-player map the arms of the map are named by letter rather
than north and south, and the sluice card, chips and hold row name your two
lanes by those letters. Every other player is an enemy: a free-for-all.

## General controls

Left click or drag selects; Shift adds. Double-click selects all visible of
a type. A box may start on the header or the band; an empty box clears the
selection. Right click gives the order that fits the target. Space centres
on headquarters. In the selection panel, clicking a type's row keeps only
that type and its `-` button removes the type; send a frame between a
selection change and a row click, because the rows are laid out when the
picture is drawn. Left-click on the chart moves the camera; right-click on the
chart orders a move or sets a rally. `pan` scrolls. Plus and minus zoom. F1
opens the Guide, which has a page for every subject and a roster page.
Escape cancels a placement or closes a menu, but a bare Escape with nothing
active opens the menu and the clicks after it in the same batch are lost:
press Escape only when a prompt says ESC CANCELS. If you do land in the
menu, Escape again returns to the field and the match has run on without
you. Ctrl+I centres on the selection.

## Reporting

Keep your notes and write your report in the folder you were given
(`output/agent-play/reports/host` or `.../guest`); the match script checked
it can be written. If a file tool refuses the write, append with the shell
instead (`cat >> FILE <<'EOF'`), so the report is never lost.

Play until the match ends or you are told to stop. Then report in this
order: what you did, what happened, what you worked out about the game and
what stayed unclear, anything that looked wrong or confusing (a control that
did not do what you expected, a picture you could not read, a rule that
seemed unfair), and one suggestion. Include the tick and the frame file name
for each problem so it can be reproduced.
