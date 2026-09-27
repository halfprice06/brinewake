# Contributing

Issues and pull requests are welcome. BRINEWAKE is an alpha, and the most
useful things right now are bug reports with a replay (F6 exports one),
balance notes from real matches, and fixes that keep the simulation
deterministic.

## Before a pull request

    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

Play the change if it affects controls, the interface or the look of the
field: a screenshot proves one instant, not a working loop.

## Determinism

Every copy of the game in an online match runs the same simulation, so the
simulation must give the same result everywhere:

- `bw_sim` uses fixed-point numbers and ordered collections only. No floats,
  no hash-map iteration order, no clocks, no randomness outside the seeded
  generator.
- A change to the rules bumps `RULES_VERSION` in `crates/bw_content`. The
  lobby compares a fingerprint of the simulation source
  (`crates/bw_content/build.rs`), and replays carry the rules digest, so
  mismatched builds refuse each other instead of drifting apart.
- Presentation (drawing, sound, the interface) may read the simulation but
  never changes it.

## Art

See [art/README.md](art/README.md). The one rule that is easiest to break:
**no shadows beneath buildings** — buildings, the sluice and the wells stand
on fitted foundations in contact with the ground.

## Tools you may need

Rust 1.90 or newer for everything. Aseprite 1.3.x to rebuild the art.
Python 3 with NumPy and Pillow, and ffmpeg, for the audio review and the
trailer. MinGW (`x86_64-w64-mingw32-gcc`) to cross-build for Windows.

By contributing you agree that your contribution is licensed under the MIT
license, like the rest of the project.
