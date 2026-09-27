# Development notes

## Two players and the agent harness

A trusted lockstep session lets two copies of the game play one match: each
runs the same deterministic simulation, commands are exchanged three ticks
ahead over TCP, and state hashes are compared every thirty ticks so a
desync is reported instead of played through. Each peer holds the whole
world, so this is for two people who trust each other, not public play.

    target/release/brinewake --host 0.0.0.0:4790 --play          host, Union by default
    target/release/brinewake --host 0.0.0.0:4790 --assembly       host as Assembly
    target/release/brinewake --join HOST:4790                    guest takes the other faction
    target/release/brinewake --host 0.0.0.0:4790 --map confluence --faction compact
                                                                 three players: starts when two guests join

Pace is fixed at 1x, pause on focus loss is off, and Load is unavailable in
a session. The match cannot be paused: the other player is still playing, so
the menu keeps the match running and says so. Both peers must be the same
build; the handshake checks the rules and a fingerprint of the simulation's
source and refuses a pair that differs. While a peer is quiet the field says
how long it has been waiting.

The same session can run without a window. `--headless` runs the game loop
from wall time and `--agent-port N` serves a small local interface on it:
`GET /frame?scale=2` returns the picture, `POST /input` takes pointer and
key events through the same entry points the window uses and answers with
what it did, and `GET /state` is for referees. `POST /quit` ends the seat and
keeps the recording. `tools/agent_play/match.sh start` launches a host on port
4711 and a guest on 4712 on this machine (`--map confluence` adds a third
seat on 4713: Union, Assembly and Compact in that order), `tools/agent_play/play.py` drives a
seat from the shell, and `tools/agent_play/PLAYBOOK.md` tells an agent how
to play from pictures alone.

## Recording and observing

Every match records itself: each ten seconds of field time the match so far
is written to `saves/live-match.replay.json` in the data directory (the
canonical match in a two-player session), and `last-match.replay.json` is
written when a match is decided and when a headless seat is told to quit, so
a match that is stopped part way through is still watchable. A recording can
be watched with the fog lifted:

    target/release/brinewake --replay PATH/live-match.replay.json            watch a recording
    target/release/brinewake --replay PATH/live-match.replay.json --follow   watch a match still being played

The observer sees both sides everywhere, from the host's seat; pace and
pause work and orders are off. With `--follow` the file is re-read as it
grows, so a live match, including one between two agents, can be watched a
few seconds behind. `tools/agent_play/match.sh start` prints the command
for the match it launches.

## Build and inspect

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check
    cargo build --release --workspace
    ./tools/package_macos.sh

Tests that time against the wall clock are ignored in the full suite, where
the other tests slow them down. Run them alone; CI runs the lossy-network one
this way on every push:

    cargo test --release -p bw_desktop --bin brinewake -- --ignored --exact net::tests::a_bad_connection_keeps_the_match_in_step_and_close_to_real_time
    cargo test --release -p bw_desktop --bin brinewake -- --ignored --exact audio::tests::mixer_is_fast_enough_for_the_output_callback

    target/release/brinewake --ux-review output/ux-review
    target/release/brinewake --qol-review output/qol-v1/journeys
    target/release/brinewake --data-dir output/test-profile
    target/release/brinewake --depth-review output/depth/ui
    target/release/brinewake --artistry-review output/art-v9/engine
    target/release/brinewake --polish-journeys output/art-v9/journeys
    target/release/brinewake --audio-review output/art-v9/audio
    target/release/brinewake --render-review output/art-v9/performance
    target/release/brinewake --depth-practice --data-dir output/depth/native
    target/release/brinewake --depth-practice --assembly --data-dir output/depth/native-assembly

    target/release/bw_tools depth-check --out output/depth/scenarios
    target/release/bw_tools skirmish 45000 --out output/check
    target/release/bw_tools replay output/check/skirmish.replay.json
    target/release/bw_tools save-check output/check/save.json
    target/release/bw_tools benchmark 18000

    target/release/brinewake --soundtrack-review output/audio-review/latest
    python3 tools/audio_review/analyze.py output/audio-review/latest
    target/release/brinewake --music-timeline output/audio-review/timeline

## Releases and updates

Players run a small launcher (`crates/bw_launcher`) rather than the game. On
each start it fetches `latest.json` and its ed25519 signature from
danprice.ai/brinewake (falling back to this repository's latest GitHub
release), checks the signature against the public key compiled into it
(`crates/bw_launcher/src/release_key.txt`), and, when there is a newer game,
downloads it, checks its size and BLAKE3 hash against the signed manifest,
installs it in the player's data folder and starts it. Offline it starts the
game it already has.

    tools/release.sh VERSION NOTES.txt [--no-notarize] [--no-windows]
    tools/publish_release.sh VERSION

`release.sh` builds the Mac disk image (the app and the image Developer ID
signed, notarized and stapled),
the Windows zip, the game-only archives the launcher installs, the
third-party license notices and the signed manifest in `dist/release/VERSION/`.
Signing the manifest needs the release key, which is not in this repository;
a fork makes its own with `brinewake-release keygen` and replaces the public
key. `publish_release.sh` uploads everything as a GitHub release.

Every player in an online match needs a build of the same simulation source:
the lobby refuses a friend whose build differs (the fingerprint in
`crates/bw_content/build.rs`). `tools/net_test/two_seats.py --bin-b "wine
BRINEWAKE.exe"` plays a Linux seat against the Windows build.

## The trailer

    target/release/brinewake --trailer-capture tools/trailer/shots.json output/trailer/clips
    python3 tools/trailer/make_trailer.py [--preview]

The capture draws shots in the engine without the interface (the practice AI
on every seat, or staged fights described in `shots.json`) and logs the
sounds each shot makes; the script cuts them to the soundtrack, draws the
titles in the game's console face and renders the soundtrack through the
game's own mixer (`--trailer-audio`). It needs Python 3 with NumPy and
Pillow, and ffmpeg.

Rust owns the simulation, navigation, pixel composition, input, UI, content contracts and tools. winit and wgpu are low-level interfaces, not an existing game engine.

`--depth-practice` is a labeled development field with prepared armies and
resources for repeatable interaction checks. Use New skirmish for ordinary
starting conditions. Tactical rings, arrows, and timers are code-rendered UI;
the original editable Aseprite artwork is unchanged.
